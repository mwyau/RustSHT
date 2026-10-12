# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

import os
import sys

threads = int(sys.argv[3]) if len(sys.argv) > 3 else 1
rayon_threads = int(os.environ.get("RAYON_NUM_THREADS", "0"))
if rayon_threads != threads:
    raise SystemExit(f"set RAYON_NUM_THREADS={threads} before starting Python")

import time
import numpy as np
from ducc0 import sht as ducc_sht
import ducc0
import rustsht

lmax = int(sys.argv[1]) if len(sys.argv) > 1 else 511
spin = int(sys.argv[2]) if len(sys.argv) > 2 else 0
geometry = sys.argv[4] if len(sys.argv) > 4 else "GL"
runs = int(sys.argv[5]) if len(sys.argv) > 5 else 2
scope = sys.argv[6] if len(sys.argv) > 6 else "full"
if scope not in {"full", "plan"}:
    raise SystemExit("scope must be 'full' or 'plan'")
operation = sys.argv[7] if len(sys.argv) > 7 else "both"
if operation not in {"both", "synthesis", "analysis"}:
    raise SystemExit("operation must be 'both', 'synthesis' or 'analysis'")
backend = sys.argv[8] if len(sys.argv) > 8 else "both"
if backend not in {"both", "rust", "ducc"}:
    raise SystemExit("backend must be 'both', 'rust' or 'ducc'")
phi0 = 0.37
ntheta = lmax + (2 if geometry == "CC" else 1)
nphi = 2 * lmax + 3
ncomp = 1 if spin == 0 else 2
nalm = sum(lmax - m + 1 for m in range(lmax + 1))

alm = np.zeros((ncomp, nalm), dtype=np.complex128)
offset = 0
for m in range(lmax + 1):
    for l in range(m, lmax + 1):
        if l >= spin:
            for c in range(ncomp):
                re = ((l * 7 + m * 11) % 19) - 9 if c == 0 else ((l * 3 + m * 7) % 17) - 8
                im = 0 if m == 0 else (((l * 13 + m * 5) % 23) - 11 if c == 0 else ((l * 5 + m * 17) % 29) - 14)
                alm[c, offset] = (re + 1j * im) * 1e-5
        offset += 1

alm_ducc = np.zeros((ncomp, (lmax + 1) * (lmax + 2) // 2), dtype=np.complex128)
offset = 0
for m in range(lmax + 1):
    for l in range(m, lmax + 1):
        idx = m * (2 * lmax + 1 - m) // 2 + l
        alm_ducc[:, idx] = alm[:, offset]
        offset += 1

rings = np.arange(ntheta, dtype=np.float64)[:, None]
longitudes = np.arange(nphi, dtype=np.float64)[None, :]
base_map = np.sin(0.019 * rings + 0.037 * longitudes) + 0.3 * np.cos(0.071 * rings - 0.013 * longitudes)
map_in = np.ascontiguousarray(np.broadcast_to(base_map, (ncomp, ntheta, nphi)).copy())

plan = rustsht.ShtPlan(geometry, ntheta, nphi, lmax, lmax, spin=spin, phi0=phi0)

def ducc_synth():
    return ducc_sht.synthesis_2d(alm=alm_ducc, spin=spin, lmax=lmax, mmax=lmax, geometry=geometry, ntheta=ntheta, nphi=nphi, phi0=phi0, nthreads=threads)
def rust_plan_synth():
    return plan.synthesis(alm)
def rust_proc_synth():
    return rustsht.synthesis_2d(alm, spin=spin, lmax=lmax, geometry=geometry, ntheta=ntheta, nphi=nphi, mmax=lmax, phi0=phi0)
def ducc_analysis():
    return ducc_sht.analysis_2d(map=map_in, spin=spin, lmax=lmax, mmax=lmax, geometry=geometry, phi0=phi0, nthreads=threads)
def rust_plan_analysis():
    return plan.analysis(map_in)
def rust_proc_analysis():
    return rustsht.analysis_2d(map_in, spin=spin, lmax=lmax, geometry=geometry, mmax=lmax, phi0=phi0)

def time_runs(fn, count=runs):
    run_medians = []
    for _ in range(count):
        fn()
        samples = []
        for _ in range(5):
            start = time.perf_counter()
            result = fn()
            samples.append((time.perf_counter() - start) * 1000.0)
        run_medians.append(float(np.median(samples)))
    return run_medians

def to_rust_layout(ducc):
    out = np.empty((ncomp, nalm), dtype=np.complex128)
    offset = 0
    for m in range(lmax + 1):
        for l in range(m, lmax + 1):
            idx = m * (2 * lmax + 1 - m) // 2 + l
            out[:, offset] = ducc[:, idx]
            offset += 1
    return out

def errors(actual, expected):
    diff = np.asarray(actual) - np.asarray(expected)
    return f"max_abs={np.max(np.abs(diff)):.3e} rms={np.sqrt(np.mean(np.abs(diff) ** 2)):.3e} rel_l2={np.linalg.norm(diff) / max(np.linalg.norm(expected), 1e-300):.3e}"

def report(name, fn):
    ms = time_runs(fn)
    print(f"{name}_run_medians_ms=" + ",".join(f"{x:.3f}" for x in ms) + f"; median={np.median(ms):.3f}; spread={max(ms)-min(ms):.3f}")

print(f"T{lmax} {geometry} {ntheta}x{nphi} spin={spin} threads={threads} runs={runs} scope={scope} operation={operation} backend={backend} python={sys.version.split()[0]} numpy={np.__version__} ducc={ducc0.__version__}")
print("plan_construction_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: rustsht.ShtPlan(geometry, ntheta, nphi, lmax, lmax, spin=spin, phi0=phi0))))
print("numpy_alm_copy_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: np.array(alm, copy=True))))
print("numpy_map_copy_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: np.array(map_in, copy=True))))
print("numpy_output_allocation_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: np.empty((ncomp, ntheta, nphi), dtype=np.float64))))
if operation in {"both", "synthesis"}:
    if backend == "both":
        print("synthesis_error=" + errors(rust_plan_synth(), ducc_synth()))
    if backend in {"both", "rust"}:
        report("synthesis_rust_plan", rust_plan_synth)
        if scope == "full":
            report("synthesis_rust_procedural", rust_proc_synth)
    if backend in {"both", "ducc"}:
        report("synthesis_ducc", ducc_synth)
if operation in {"both", "analysis"}:
    if backend == "both":
        print("analysis_error=" + errors(rust_plan_analysis(), to_rust_layout(ducc_analysis())))
    if backend in {"both", "rust"}:
        report("analysis_rust_plan", rust_plan_analysis)
        if scope == "full":
            report("analysis_rust_procedural", rust_proc_analysis)
    if backend in {"both", "ducc"}:
        report("analysis_ducc", ducc_analysis)
