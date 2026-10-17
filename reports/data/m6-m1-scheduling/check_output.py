import sys

import numpy as np
from ducc0 import sht as ducc_sht
import ducc0
import rustsht


lmax = 2047
mmax = lmax
ntheta = 2048
nphi = 4096
phi0 = 0.37
threads = 8
nalm = sum(lmax - m + 1 for m in range(lmax + 1))

alm = np.zeros((1, nalm), dtype=np.complex128)
alm_ducc = np.zeros((1, (lmax + 1) * (lmax + 2) // 2), dtype=np.complex128)
offset = 0
for m in range(lmax + 1):
    for l in range(m, lmax + 1):
        re = ((l * 7 + m * 11) % 19) - 9
        im = 0 if m == 0 else ((l * 13 + m * 5) % 23) - 11
        alm[0, offset] = (re + 1j * im) * 1e-5
        ducc_index = m * (2 * lmax + 1 - m) // 2 + l
        alm_ducc[0, ducc_index] = alm[0, offset]
        offset += 1

plan = rustsht.ShtPlan("GL", ntheta, nphi, lmax, mmax, spin=0, phi0=phi0)
rust_map = np.empty((1, ntheta, nphi), dtype=np.float64)
ducc_map = np.empty_like(rust_map)
plan.synthesis_into(alm, rust_map)
ducc_sht.synthesis_2d(
    alm=alm_ducc,
    spin=0,
    lmax=lmax,
    mmax=mmax,
    geometry="GL",
    ntheta=ntheta,
    nphi=nphi,
    phi0=phi0,
    mode="STANDARD",
    nthreads=threads,
    map=ducc_map,
)

diff = rust_map - ducc_map
relative_l2 = np.linalg.norm(diff) / max(np.linalg.norm(ducc_map), 1e-300)
print(f"rustsht={rustsht.__file__}")
print(f"ducc0={ducc0.__file__}; version={ducc0.__version__}")
print(f"config=GL STANDARD f64; T={lmax}; ntheta={ntheta}; nphi={nphi}; mmax={mmax}; spin=0; phi0={phi0}; q={threads}")
print(f"ducc_relative_l2={relative_l2:.17e}")
print(f"ducc_max_abs={np.max(np.abs(diff)):.17e}")
np.save(sys.argv[1], rust_map)
