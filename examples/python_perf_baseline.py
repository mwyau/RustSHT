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
if scope not in {"full", "plan", "into"}:
    raise SystemExit("scope must be 'full', 'plan' or 'into'")
operation = sys.argv[7] if len(sys.argv) > 7 else "both"
if operation not in {"both", "synthesis", "analysis"}:
    raise SystemExit("operation must be 'both', 'synthesis' or 'analysis'")
backend = sys.argv[8] if len(sys.argv) > 8 else "both"
if backend not in {"both", "rust", "ducc", "shtns", "references", "all"}:
    raise SystemExit("unknown backend")
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
if scope == "into" and mode != "STANDARD":
    raise SystemExit("into-buffer timing supports STANDARD only")
phi0 = float(os.environ.get("RUSTSHT_BENCH_PHI0", "0.37"))
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
rust_map_out = np.empty_like(map_in)
ducc_map_out = np.empty_like(map_in)
rust_alm_out = np.empty_like(alm)
ducc_alm_out = np.empty_like(alm_ducc)


def ducc_synth():
    return ducc_sht.synthesis_2d(alm=alm_ducc, spin=ducc_spin, lmax=lmax, mmax=lmax, geometry=geometry, ntheta=ntheta, nphi=nphi, phi0=phi0, mode=mode, nthreads=threads, map=ducc_map_out if scope == "into" else None)
def rust_plan_synth():
    if scope == "into":
        plan.synthesis_into(alm, rust_map_out)
        return rust_map_out
    if mode == "GRAD_ONLY":
        return plan.synthesis_gradient(alm)
    if mode == "DERIV1":
        return plan.synthesis_deriv1(alm)
    return plan.synthesis(alm)
def rust_proc_synth():
    return rustsht.synthesis_2d(alm, spin=ducc_spin, lmax=lmax, geometry=geometry, ntheta=ntheta, nphi=nphi, mmax=lmax, phi0=phi0, mode=mode)
def ducc_analysis():
    return ducc_sht.analysis_2d(map=map_in, spin=spin, lmax=lmax, mmax=lmax, geometry=geometry, phi0=phi0, nthreads=threads, alm=ducc_alm_out if scope == "into" else None)
def rust_plan_analysis():
    if scope == "into":
        plan.analysis_into(map_in, rust_alm_out)
        return rust_alm_out
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


class ShtnsReference:
    """SHTns 3.7.5 public C ABI; no numerical implementation is copied.

    Timed calls include preserving the immutable inputs and converting E/B
    to/from spheroidal/toroidal potentials. Setup and output allocation are
    excluded. The current adapter supports exact rectangular GL grids.
    """

    def __init__(self):
        import ctypes as ct

        if geometry != "GL" or spin not in (0, 1) or mode != "STANDARD":
            raise SystemExit("SHTns reference supports GL spin-0/1 STANDARD")
        class Info(ct.Structure):
            _fields_ = [("nlm", ct.c_uint), ("lmax", ct.c_ushort),
                        ("mmax", ct.c_ushort), ("mres", ct.c_ushort),
                        ("nlat_2", ct.c_ushort), ("nlat", ct.c_uint),
                        ("nphi", ct.c_uint), ("nspat", ct.c_uint)]

        path = os.environ["RUSTSHT_SHTNS_LIB"]
        lib = ct.CDLL(path)
        lib.shtns_use_threads.argtypes = [ct.c_int]
        lib.shtns_use_threads.restype = ct.c_int
        lib.shtns_create.argtypes = [ct.c_int] * 4
        lib.shtns_create.restype = ct.c_void_p
        lib.shtns_set_grid.argtypes = [ct.c_void_p, ct.c_int, ct.c_double, ct.c_int, ct.c_int]
        lib.shtns_set_grid.restype = ct.c_int
        for name, arrays in [("SH_to_spat", 2), ("spat_to_SH", 2),
                             ("SHsphtor_to_spat", 4), ("spat_to_SHsphtor", 4)]:
            function = getattr(lib, name)
            function.argtypes = [ct.c_void_p] * (arrays + 1)
            function.restype = None
        lib.shtns_use_threads(threads)
        self.cfg = lib.shtns_create(lmax, lmax, 1, 0)  # orthonormal, Condon–Shortley
        if not self.cfg:
            raise RuntimeError("SHTns plan allocation failed")
        # sht_gauss | SHT_PHI_CONTIGUOUS; no padding or persisted tuning cache.
        allocated = lib.shtns_set_grid(self.cfg, 512, 1e-30, ntheta, nphi)
        info = ct.cast(self.cfg, ct.POINTER(Info)).contents
        if (info.nlm, info.nlat, info.nphi, info.nspat) != (nalm, ntheta, nphi, ntheta * nphi):
            raise RuntimeError("SHTns changed the requested grid or requires padded storage")
        if allocated <= 0:
            raise RuntimeError("SHTns grid allocation failed")
        self.lib = lib
        self.ptr = lambda array: ct.c_void_p(array.ctypes.data)
        self.coefficients = np.empty_like(alm)
        self.map = np.empty_like(map_in)
        self.map_work = np.empty_like(map_in)
        degrees = np.concatenate([np.arange(m, lmax + 1) for m in range(lmax + 1)])
        orders = np.concatenate([np.full(lmax - m + 1, m) for m in range(lmax + 1)])
        phase = np.exp(1j * orders * phi0)
        if spin == 0:
            self.synth_scale = phase[None, :]
            self.analysis_scale = phase.conj()[None, :]
        else:
            norm = np.sqrt(degrees.astype(float) * (degrees + 1))
            inverse = np.divide(1.0, norm, out=np.zeros_like(norm), where=norm != 0)
            self.synth_scale = np.stack((phase * inverse, -phase * inverse))
            self.analysis_scale = np.stack((phase.conj() * norm, -phase.conj() * norm))
        print(f"shtns_library={path}; eps=1e-30; flags=512; wrapper=preserve-inputs-and-convert")

    def synthesis(self):
        np.multiply(alm, self.synth_scale, out=self.coefficients)
        if spin == 0:
            self.lib.SH_to_spat(self.cfg, self.ptr(self.coefficients[0]), self.ptr(self.map[0]))
        else:
            self.lib.SHsphtor_to_spat(self.cfg, self.ptr(self.coefficients[0]),
                                     self.ptr(self.coefficients[1]), self.ptr(self.map[0]), self.ptr(self.map[1]))
        return self.map

    def analysis(self):
        np.copyto(self.map_work, map_in)
        if spin == 0:
            self.lib.spat_to_SH(self.cfg, self.ptr(self.map_work[0]), self.ptr(self.coefficients[0]))
        else:
            self.lib.spat_to_SHsphtor(self.cfg, self.ptr(self.map_work[0]), self.ptr(self.map_work[1]),
                                     self.ptr(self.coefficients[0]), self.ptr(self.coefficients[1]))
        np.multiply(self.coefficients, self.analysis_scale, out=self.coefficients)
        return self.coefficients

shtns_reference = ShtnsReference() if backend in {"shtns", "references", "all"} else None

print(f"T{lmax} {geometry} {ntheta}x{nphi} spin={spin} mode={mode} threads={threads} runs={runs} scope={scope} operation={operation} backend={backend} phi0={phi0} python={sys.version.split()[0]} numpy={np.__version__} ducc={ducc0.__version__}")
print("plan_construction_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: rustsht.ShtPlan(geometry, ntheta, nphi, lmax, lmax, spin=plan_spin, phi0=phi0))))
print("numpy_alm_copy_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: np.array(alm, copy=True))))
print("numpy_map_copy_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: np.array(map_in, copy=True))))
print("numpy_output_allocation_ms=" + ",".join(f"{x:.3f}" for x in time_runs(lambda: np.empty((map_components, ntheta, nphi), dtype=np.float64))))
if operation in {"both", "synthesis"}:
    if backend in {"both", "all"}:
        print("synthesis_error=" + errors(rust_plan_synth(), ducc_synth()))
    if backend in {"both", "rust", "all"}:
        report("synthesis_rust_plan", rust_plan_synth)
        if scope == "full":
            report("synthesis_rust_procedural", rust_proc_synth)
    if backend in {"both", "ducc", "references", "all"}:
        report("synthesis_ducc", ducc_synth)
    if shtns_reference is not None:
        print("synthesis_shtns_ducc_error=" + errors(shtns_reference.synthesis(), ducc_synth()))
        report("synthesis_shtns_adapter", shtns_reference.synthesis)
if operation in {"both", "analysis"} and mode == "STANDARD":
    if backend in {"both", "all"}:
        print("analysis_error=" + errors(rust_plan_analysis(), to_rust_layout(ducc_analysis())))
    if backend in {"both", "rust", "all"}:
        report("analysis_rust_plan", rust_plan_analysis)
        if scope == "full":
            report("analysis_rust_procedural", rust_proc_analysis)
    if backend in {"both", "ducc", "references", "all"}:
        report("analysis_ducc", ducc_analysis)
    if shtns_reference is not None:
        print("analysis_shtns_ducc_error=" + errors(shtns_reference.analysis(), to_rust_layout(ducc_analysis())))
        report("analysis_shtns_adapter", shtns_reference.analysis)
