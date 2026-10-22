#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Small independent analytic controls using the frozen native worker ABI."""
import argparse
import hashlib
import json
import math
import os
import pathlib
import subprocess
import tempfile

import numpy as np


def coefficient_index(lmax, degree, order):
    return order * (lmax + 1) - order * (order - 1) // 2 + degree - order


def analytic_fixture(grid, lmax, mmax, nphi, phi0):
    ntheta = lmax + (2 if grid == 'CC' else 1)
    if grid == 'CC':
        theta = math.pi * np.arange(ntheta) / (ntheta - 1)
    else:
        theta = np.arccos(np.polynomial.legendre.leggauss(ntheta)[0][::-1])
    phi = phi0 + 2 * math.pi * np.arange(nphi) / nphi
    alm = np.zeros((mmax + 1) * (lmax + 1) - mmax * (mmax + 1) // 2, dtype='<c16')
    alm[0] = 0.7
    expected = np.full((ntheta, nphi), 0.7 / math.sqrt(4 * math.pi))
    if lmax >= 1:
        alm[1] = -0.2
        expected += (-0.2 * math.sqrt(3 / (4 * math.pi)) * np.cos(theta))[:, None]
    if mmax >= 1:
        alm[coefficient_index(lmax, 1, 1)] = 0.3 + 0.4j
        expected += -math.sqrt(3 / (2 * math.pi)) * np.sin(theta)[:, None] * (
            0.3 * np.cos(phi) - 0.4 * np.sin(phi))
    if lmax >= 2:
        coefficient = 0.125
        if mmax == 0:
            alm[lmax] = coefficient
            legendre_coeffs = np.zeros(lmax + 1)
            legendre_coeffs[-1] = 1
            expected += (coefficient * math.sqrt((2 * lmax + 1) / (4 * math.pi))
                         * np.polynomial.legendre.legval(np.cos(theta), legendre_coeffs))[:, None]
        elif mmax == lmax:
            alm[coefficient_index(lmax, lmax, lmax)] = coefficient * (1 - 0.25j)
            norm = 1 / math.sqrt(4 * math.pi)
            for degree in range(1, lmax + 1):
                norm *= math.sqrt((2 * degree + 1) / (2 * degree))
            expected += (2 * coefficient * (-1)**lmax * norm * np.sin(theta)**lmax)[:, None] * (
                np.cos(lmax * phi) + 0.25 * np.sin(lmax * phi))
    return ntheta, alm, expected.astype('<f8')


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--rust', type=pathlib.Path, required=True)
    ap.add_argument('--ducc', type=pathlib.Path, required=True)
    ap.add_argument('--cpu', type=int, default=8)
    ap.add_argument('--output', type=pathlib.Path, required=True)
    a = ap.parse_args()
    records = []
    cases = [('CC', 0, 0, 1, .23, 1), ('CC', 0, 0, 2, -.41, 1),
             ('CC', 1, 0, 1, -.41, 1), ('CC', 2, 0, 2, .37, 1),
             ('CC', 3, 0, 1, .23, 1), ('CC', 4, 4, 9, -.41, 1),
             ('CC', 4, 4, 10, .37, 1), ('CC', 255, 255, 513, .37, 1),
             ('CC', 255, 255, 512, -.41, 8), ('GL', 4, 4, 9, -.41, 1)]
    with tempfile.TemporaryDirectory(prefix='cc-independent-analytic-') as td:
        folder = pathlib.Path(td)
        for grid, lmax, mmax, nphi, phi0, q in cases:
            ntheta, alm, map_values = analytic_fixture(grid, lmax, mmax, nphi, phi0)
            mp, al = folder/'input.map', folder/'input.alm'
            map_values.tofile(mp)
            alm.tofile(al)
            results = {}
            for op in ('analysis', 'synthesis'):
                for backend, binary in (('rustsht', a.rust), ('ducc', a.ducc)):
                    out = folder/f'{backend}-{op}.bin'
                    command = [str(binary), '--worker', op, str(q), str(ntheta), str(nphi),
                               str(lmax), str(mmax), str(phi0), str(mp), str(al), str(out), '25', '0', grid]
                    env = os.environ | {'DUCC0_NUM_THREADS':str(q), 'RAYON_NUM_THREADS':str(q)}
                    cpus = str(a.cpu) if q == 1 else ','.join(map(str, range(a.cpu, a.cpu+q)))
                    p = subprocess.run(['taskset', '-c', cpus, *command], env=env,
                                       text=True, capture_output=True, check=True)
                    worker = json.loads(p.stdout.strip().splitlines()[-1])
                    values = np.fromfile(out, dtype='<c16' if op == 'analysis' else '<f8')
                    if backend == 'ducc' and op == 'analysis':
                        values = values[mmax:mmax+len(alm)]
                    expected = alm if op == 'analysis' else map_values.ravel()
                    error = float(np.abs(values-expected).max())
                    tolerance = (2e-9 if op == 'analysis' else 4e-12) if lmax == 255 else 3e-12
                    assert np.isfinite(values).all() and worker['reset_consistent']
                    assert error <= tolerance, (grid,lmax,mmax,nphi,q,backend,op,error,tolerance)
                    legacy_exact = None
                    if grid == 'GL':
                        # Same fixtures, same native executable, omitted optional GRID.
                        subprocess.run(['taskset','-c',cpus,*command[:-1]],env=env,
                                       text=True,capture_output=True,check=True)
                        legacy = np.fromfile(out,dtype='<c16' if op=='analysis' else '<f8')
                        if backend=='ducc' and op=='analysis':
                            legacy=legacy[mmax:mmax+len(alm)]
                        legacy_exact = bool(np.array_equal(values,legacy))
                        assert legacy_exact
                    results[(backend,op)] = values
                    records.append({'grid':grid,'lmax':lmax,'mmax':mmax,'ntheta':ntheta,'nphi':nphi,
                        'phi0':phi0,'threads':q,'backend':backend,'operation':op,
                        'analytic_absolute_max':error,'tolerance':tolerance,
                        'reset_consistent':worker['reset_consistent'],'legacy_grid_output_exact':legacy_exact})
            for op in ('analysis','synthesis'):
                assert np.max(np.abs(results[('rustsht',op)]-results[('ducc',op)])) <= (2e-9 if lmax==255 else 3e-12)
    out={'binary_sha256':{name:hashlib.sha256(path.read_bytes()).hexdigest()
                          for name,path in (('rust',a.rust),('ducc',a.ducc))},
         'q1_cpu':a.cpu,'q8_cpus':list(range(a.cpu,a.cpu+8)),
         'note':'Correctness controls only; worker timing samples are deliberately not interpreted.',
         'configuration_count':len(cases),'worker_records':records}
    a.output.write_text(json.dumps(out,indent=2)+'\n')
    print(f'{len(cases)} analytic configurations / {len(records)} worker records passed')


if __name__ == '__main__':
    main()
