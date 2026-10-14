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
mode = sys.argv[9] if len(sys.argv) > 9 else "STANDARD"
nphi = int(sys.argv[10]) if len(sys.argv) > 10 else 2 * lmax + 3
ntheta = int(sys.argv[11]) if len(sys.argv) > 11 else lmax + (2 if geometry == "CC" else 1)
if mode not in {"STANDARD", "GRAD_ONLY", "DERIV1"}:
    raise SystemExit("mode must be STANDARD, GRAD_ONLY or DERIV1")
if mode == "GRAD_ONLY" and spin == 0:
    raise SystemExit("GRAD_ONLY requires a nonzero spin")
if mode == "DERIV1" and spin != 1:
    raise SystemExit("DERIV1 uses spin=1 in DUCC's synthesis API")
if mode != "STANDARD" and operation != "synthesis":
    raise SystemExit("GRAD_ONLY and DERIV1 support synthesis benchmarking only")
phi0 = 0.37
ncomp = 1 if spin == 0 or mode != "STANDARD" else 2
map_components = 1 if mode == "STANDARD" and spin == 0 else 2
plan_spin = 0 if mode == "DERIV1" else spin
ducc_spin = 1 if mode == "DERIV1" else spin
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
map_in = np.ascontiguousarray(np.broadcast_to(base_map, (map_components, ntheta, nphi)).copy())

plan = rustsht.ShtPlan(geometry, ntheta, nphi, lmax, lmax, spin=plan_spin, phi0=phi0)

def ducc_synth():
    return ducc_sht.synthesis_2d(alm=alm_ducc, spin=ducc_spin, lmax=lmax, mmax=lmax, geometry=geometry, ntheta=ntheta, nphi=nphi, phi0=phi0, mode=mode, nthreads=threads)
def rust_plan_synth():
    if mode == "GRAD_ONLY":
        return plan.synthesis_gradient(alm)
    if mode == "DERIV1":
        return plan.synthesis_deriv1(alm)
    return plan.synthesis(alm)
def rust_proc_synth():
    return rustsht.synthesis_2d(alm, spin=ducc_spin, lmax=lmax, geometry=geometry, ntheta=ntheta, nphi=nphi, mmax=lmax, phi0=phi0, mode=mode)
def ducc_analysis():
    return ducc_sht.analysis_2d(map=map_in, spin=spin, lmax=lmax, mmax=lmax, geometry=geometry, phi0=phi0, nthreads=threads)
def rust_plan_analysis():
    return plan.analysis(map_in)
def rust_proc_analysis():
    return rustsht.analysis_2d(map_in, spin=spin, lmax=lmax, geometry=geometry, mmax=lmax, phi0=phi0)

def time_runs(fn, count=runs, include_samples=False):
    run_medians = []
    run_samples = []
    for _ in range(count):
        fn()
        samples = []
        for _ in range(5):
            start = time.perf_counter()
            result = fn()
            samples.append((time.perf_counter() - start) * 1000.0)
        run_medians.append(float(np.median(samples)))
        run_samples.append(samples)
    if include_samples:
        return run_medians, run_samples
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
    if os.environ.get("RUSTSHT_PERF_RAW") == "1":
        ms, samples = time_runs(fn, include_samples=True)
        raw_text = ";".join(",".join(f"{x:.3f}" for x in group) for group in samples)
        print(f"{name}_raw_samples_ms=" + raw_text)
    else:
        ms = time_runs(fn)
    print(f"{name}_run_medians_ms=" + ",".join(f"{x:.3f}" for x in ms) + f"; median={np.median(ms):.3f}; spread={max(ms)-min(ms):.3f}")

print(f"T{lmax} {geometry} {ntheta}x{nphi} spin={spin} mode={mode} threads={threads} runs={runs} scope={scope} operation={operation} backend={backend} python={sys.version.split()[0]} numpy={np.__version__} ducc={ducc0.__version__}")
print("plan_construction_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: rustsht.ShtPlan(geometry, ntheta, nphi, lmax, lmax, spin=plan_spin, phi0=phi0))))
print("numpy_alm_copy_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: np.array(alm, copy=True))))
print("numpy_map_copy_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: np.array(map_in, copy=True))))
print("numpy_output_allocation_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: np.empty((map_components, ntheta, nphi), dtype=np.float64))))
if operation in {"both", "synthesis"}:
    if backend == "both":
        print("synthesis_error=" + errors(rust_plan_synth(), ducc_synth()))
    if backend in {"both", "rust"}:
        report("synthesis_rust_plan", rust_plan_synth)
        if scope == "full":
            report("synthesis_rust_procedural", rust_proc_synth)
    if backend in {"both", "ducc"}:
        report("synthesis_ducc", ducc_synth)
if operation in {"both", "analysis"} and mode == "STANDARD":
    if backend == "both":
        print("analysis_error=" + errors(rust_plan_analysis(), to_rust_layout(ducc_analysis())))
    if backend in {"both", "rust"}:
        report("analysis_rust_plan", rust_plan_analysis)
        if scope == "full":
            report("analysis_rust_procedural", rust_proc_analysis)
    if backend in {"both", "ducc"}:
        report("analysis_ducc", ducc_analysis)
