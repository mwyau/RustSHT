#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Reproduce the three numerical transition controls; report no timings."""
import argparse
import json
import os
import pathlib
import subprocess

import numpy as np


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rust-test', type=pathlib.Path)
    parser.add_argument('--ducc-clean', type=pathlib.Path)
    parser.add_argument('--ducc-counter', type=pathlib.Path)
    parser.add_argument('--directory', type=pathlib.Path, required=True)
    parser.add_argument('--read-existing', action='store_true')
    parser.add_argument('--cpu', type=int, default=8)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if not args.read_existing and not all((args.rust_test, args.ducc_clean, args.ducc_counter)):
        parser.error('provide all three binaries or use --read-existing')
    args.directory.mkdir(exist_ok=True, parents=True)
    rows = []
    for degree in (499, 500, 511):
        rust_path = args.directory/f'rust-spin-T{degree}.bin'
        ducc_path = args.directory/f'clean-spin-T{degree}.bin'
        counter_path = args.directory/f'spin-T{degree}-m{degree}-CC-q1.bin'
        if not args.read_existing:
            env = os.environ | {'RUSTSHT_PROFILE_GRID':'CC', 'RUSTSHT_PROFILE_LMAX':str(degree),
                'RUSTSHT_PROFILE_MMAX':str(degree), 'RUSTSHT_PROFILE_NPHI':str(2*(degree+1)),
                'RUSTSHT_PROFILE_THREADS':'1', 'RUSTSHT_PROFILE_REPETITIONS':'0',
                'RUSTSHT_REVIEW_SPIN_MAP_OUT':str(rust_path)}
            subprocess.run(['taskset','-c',str(args.cpu),str(args.rust_test),'--exact',
                'spin_transform::tests::profile_spin1_standard_synthesis_phases',
                '--ignored','--nocapture'],env=env,capture_output=True,check=True)
            for binary, path, counter in ((args.ducc_clean,ducc_path,False),
                                           (args.ducc_counter,counter_path,True)):
                env = os.environ | {'DUCC0_NUM_THREADS':'1'}
                if counter:
                    env['RUSTSHT_REVIEW_SPIN_ROUTE'] = '1'
                subprocess.run(['taskset','-c',str(args.cpu),str(binary),str(degree),str(degree),
                    'CC','1',str(path)],env=env,capture_output=True,check=True)
        rust = np.fromfile(rust_path,dtype='<f8')
        ducc = np.fromfile(ducc_path,dtype='<f8')
        counter = np.fromfile(counter_path,dtype='<f8')
        assert len(rust)==len(ducc)==len(counter)==2*(degree+2)*2*(degree+1)
        assert np.isfinite(rust).all() and np.isfinite(ducc).all() and np.isfinite(counter).all()
        error = float(np.linalg.norm(rust-ducc)/np.linalg.norm(ducc))
        assert error <= 5e-12, (degree,error)
        rows.append({'lmax':degree,'mmax':degree,'grid':'CC','ntheta':degree+2,
            'nphi':2*(degree+1),'threads':1,'relative_l2_rust_vs_pinned_ducc':error,
            'max_absolute_error':float(np.max(np.abs(rust-ducc))),
            'instrumented_ducc_output_exact':bool(np.array_equal(counter,ducc)),
            'instrumented_ducc_relative_l2':float(np.linalg.norm(counter-ducc)/np.linalg.norm(ducc))})
    args.output.write_text(json.dumps(rows,indent=2)+'\n')
    print('T499/T500/T511 full CC spin-1 numerical controls passed')


if __name__ == '__main__':
    main()
