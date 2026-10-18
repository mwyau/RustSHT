import hashlib
import sys
import numpy as np
from ducc0 import sht as ducc_sht
import rustsht

lmax = int(sys.argv[1])
threads = int(sys.argv[2])
out_path = sys.argv[3]
ntheta = lmax + 1
nphi = 2 * (lmax + 1)
phi0 = 0.37
nalm = sum(lmax - m + 1 for m in range(lmax + 1))
alm = np.zeros((2, nalm), dtype=np.complex128)
offset = 0
for m in range(lmax + 1):
    for degree in range(m, lmax + 1):
        if degree >= 1:
            for channel in range(2):
                re = ((degree * 7 + m * 11) % 19) - 9 if channel == 0 else ((degree * 3 + m * 7) % 17) - 8
                im = 0 if m == 0 else (((degree * 13 + m * 5) % 23) - 11 if channel == 0 else ((degree * 5 + m * 17) % 29) - 14)
                alm[channel, offset] = (re + 1j * im) * 1e-5
        offset += 1
alm_ducc = np.zeros((2, (lmax + 1) * (lmax + 2) // 2), dtype=np.complex128)
offset = 0
for m in range(lmax + 1):
    for degree in range(m, lmax + 1):
        alm_ducc[:, m * (2 * lmax + 1 - m) // 2 + degree] = alm[:, offset]
        offset += 1
plan = rustsht.ShtPlan("GL", ntheta, nphi, lmax, lmax, spin=1, phi0=phi0)
actual = np.empty((2, ntheta, nphi), dtype=np.float64)
plan.synthesis_into(alm, actual)
expected = ducc_sht.synthesis_2d(
    alm=alm_ducc,
    spin=1,
    lmax=lmax,
    mmax=lmax,
    geometry="GL",
    ntheta=ntheta,
    nphi=nphi,
    phi0=phi0,
    mode="STANDARD",
    nthreads=threads,
)
difference = actual - expected
relative_l2 = np.linalg.norm(difference) / np.linalg.norm(expected)
max_abs = float(np.max(np.abs(difference)))
if relative_l2 > 1e-12:
    raise SystemExit(f"DUCC relative-L2 gate failed: {relative_l2:.6e}")
np.save(out_path, actual)
print(f"lmax={lmax} spin=1 threads={threads} geometry=GL mode=STANDARD ntheta={ntheta} nphi={nphi} mmax={lmax} phi0={phi0}")
print(f"rust_module={rustsht._rustsht.__file__}")
print(f"rust_map_sha256={hashlib.sha256(actual.tobytes(order='C')).hexdigest()}")
print(f"ducc_relative_l2={relative_l2:.6e} max_abs={max_abs:.6e}")
print(f"map_saved={out_path}")
