#!/usr/bin/env python3
"""Matched native RustSHT/DUCC scalar GL and CC benchmark."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import pathlib
import platform
import re
import shlex
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from collections import Counter, defaultdict

import numpy as np


ROOT = pathlib.Path(__file__).resolve().parents[1]
DUCC_REVISION = os.environ.get(
    "RUSTSHT_DUCC_REVISION", "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
)
HISTORICAL_DUCC_REVISION = "80388904b8cfc32400ba84af8a6a45aabfe60f86"
DATA_DIR = ROOT / "reports/data/ducc-full-transform-parity-2026-10-20"
REPORT_PATH = ROOT / "reports/ducc-full-transform-parity-2026-10-20.md"
CC_DATA_DIR = ROOT / "reports/data/ducc-scalar-f64-cc-full-sweep-2026-10-22"
CC_REPORT_PATH = ROOT / "reports/ducc-scalar-f64-cc-full-sweep-2026-10-22.md"
RUST_FLAGS = os.environ.get(
    "RUSTSHT_FULL_TRANSFORM_RUSTFLAGS",
    "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
)
PHI0 = 0.23
TARGET_BATCH_MS = 25.0
PROCESS_PAIRS = 3
BATCHES_PER_PROCESS = 10
DENSE_SWEEP_HIGH_LMAX = (
    257, 258, 300, 320, 384, 511, 512, 513, 640, 720, 721, 768, 896,
    1023, 1024, 1025, 1280, 1536, 1792, 2047, 2048, 2049, 2560, 3072,
)
DENSE_SWEEP_ODD_LMAX = frozenset({
    0, 1, 2, 3, 4, 8, 16, 32, 48, 62, 63, 64, 65, 127, 128, 159, 191,
    223, 248, 254, 255, 256, 257, 511, 512, 513, 720, 721, 1023, 1024,
    1025, 2047, 2048, 2049, 2560, 3072,
})


def command_output(command: list[str], cwd: pathlib.Path = ROOT, env=None) -> str:
    return subprocess.run(
        command, cwd=cwd, env=env, check=True, text=True, capture_output=True
    ).stdout.strip()


def cpu_model() -> str:
    text = pathlib.Path("/proc/cpuinfo").read_text()
    match = re.search(r"^model name\s*:\s*(.+)$", text, re.MULTILINE)
    return match.group(1).strip() if match else platform.processor() or "unknown"


def cpu_flags() -> set[str]:
    text = pathlib.Path("/proc/cpuinfo").read_text()
    rows = re.findall(r"^flags\s*:\s*(.+)$", text, re.MULTILINE)
    if not rows:
        return set()
    return set(rows[0].split())


def topology(cpu: int) -> tuple[str, str]:
    base = pathlib.Path(f"/sys/devices/system/cpu/cpu{cpu}/topology")
    return (base / "physical_package_id").read_text().strip(), (base / "core_id").read_text().strip()


def l3_cache_id(cpu: int) -> str | None:
    path = pathlib.Path(f"/sys/devices/system/cpu/cpu{cpu}/cache/index3/id")
    try:
        return path.read_text().strip()
    except OSError:
        return None


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def portable_evidence(value):
    """Remove machine-local roots while preserving reproducible flags/data."""
    if isinstance(value, dict):
        return {key: portable_evidence(item) for key, item in value.items()}
    if isinstance(value, list):
        return [portable_evidence(item) for item in value]
    if isinstance(value, str):
        value = value.replace(str(ROOT), "<repo>")
        value = re.sub(r"/tmp/[^\s;'\"]+", "<temporary-path>", value)
        return value
    return value


def physical_cpus() -> list[int]:
    allowed = sorted(os.sched_getaffinity(0))
    seen = set()
    result = []
    for cpu in allowed:
        key = topology(cpu)
        if key not in seen:
            seen.add(key)
            result.append(cpu)
    if len(result) < 8:
        raise RuntimeError(f"q8 needs eight allowed physical cores; found {len(result)}")
    return result


def frequency_path(cpu: int) -> pathlib.Path | None:
    direct = pathlib.Path(f"/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_cur_freq")
    if direct.exists() and os.access(direct, os.R_OK):
        return direct
    for policy in pathlib.Path("/sys/devices/system/cpu/cpufreq").glob("policy*"):
        cpulist = policy / "related_cpus"
        if not cpulist.exists():
            cpulist = policy / "affected_cpus"
        try:
            related = {int(item) for item in cpulist.read_text().split()}
        except (OSError, ValueError):
            continue
        path = policy / "scaling_cur_freq"
        if cpu in related and path.exists() and os.access(path, os.R_OK):
            return path
    return None


def scheduler_snapshot(pid: int) -> dict[str, int] | None:
    task_dir = pathlib.Path(f"/proc/{pid}/task")
    try:
        tasks = list(task_dir.iterdir())
    except OSError:
        return None
    runtime_ns = wait_ns = slices = 0
    voluntary = involuntary = 0
    found = False
    for task in tasks:
        try:
            fields = (task / "schedstat").read_text().split()
            runtime_ns += int(fields[0])
            wait_ns += int(fields[1])
            slices += int(fields[2])
            status = (task / "status").read_text()
            for key, target in (("voluntary_ctxt_switches", "voluntary"),
                                ("nonvoluntary_ctxt_switches", "involuntary")):
                match = re.search(rf"^{key}:\s+(\d+)$", status, re.MULTILINE)
                if match:
                    if target == "voluntary":
                        voluntary += int(match.group(1))
                    else:
                        involuntary += int(match.group(1))
            found = True
        except (OSError, ValueError, IndexError):
            continue
    if not found:
        return None
    return {
        "runtime_ns": runtime_ns,
        "wait_ns": wait_ns,
        "slices": slices,
        "voluntary": voluntary,
        "involuntary": involuntary,
    }


def worker_command(binary: pathlib.Path, backend: str, case: dict, operation: str,
                   threads: int, output: pathlib.Path, target_ms: float,
                   validate: bool = False) -> list[str]:
    executable = binary[backend]
    args = [
        "--worker", operation, str(threads), str(case["ntheta"]), str(case["nphi"]),
        str(case["lmax"]), str(case["mmax"]), f"{PHI0:.17g}",
        str(case["map_path"]), str(case["alm_path"]), str(output), f"{target_ms:.6g}",
        "1" if validate else "0", case.get("grid", "GL"),
    ]
    return [str(executable), *args]


def run_pinned(command: list[str], cpus: list[int], monitor_cpu: int) -> tuple[dict, dict]:
    def pin_child() -> None:
        os.sched_setaffinity(0, set(cpus))

    environment = os.environ.copy()
    environment["RAYON_NUM_THREADS"] = str(len(cpus))
    environment["OMP_NUM_THREADS"] = "1"
    environment["DUCC0_NUM_THREADS"] = str(len(cpus))
    environment["OPENBLAS_NUM_THREADS"] = "1"
    process = subprocess.Popen(
        command, cwd=ROOT, env=environment, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, preexec_fn=pin_child,
    )
    freq_files = [frequency_path(cpu) for cpu in cpus]
    frequencies: list[float] = []
    snapshots: list[dict[str, int]] = []

    def monitor() -> None:
        os.sched_setaffinity(threading.get_native_id(), {monitor_cpu})
        while process.poll() is None:
            for path in freq_files:
                if path is not None:
                    try:
                        frequencies.append(int(path.read_text().strip()) / 1e6)
                    except (OSError, ValueError):
                        pass
            snap = scheduler_snapshot(process.pid)
            if snap is not None:
                snapshots.append(snap)
            time.sleep(0.05)

    sampler = threading.Thread(target=monitor, daemon=True)
    sampler.start()
    stdout, stderr = process.communicate()
    sampler.join()
    if process.returncode:
        raise RuntimeError(
            f"worker failed: {shlex.join(command)}\n{stderr}\n{stdout}"
        )
    lines = [line for line in stdout.splitlines() if line.startswith("{")]
    if not lines:
        raise RuntimeError(f"worker returned no JSON record:\n{stdout}\n{stderr}")
    record = json.loads(lines[-1])
    first = snapshots[0] if snapshots else None
    last = snapshots[-1] if snapshots else None
    wait_fraction = None
    if first and last:
        runtime = max(0, last["runtime_ns"] - first["runtime_ns"])
        wait = max(0, last["wait_ns"] - first["wait_ns"])
        wait_fraction = wait / max(runtime + wait, 1)
    diagnostics = {
        "frequency_ghz": frequencies,
        "wait_fraction": wait_fraction,
        "involuntary_switches": (
            max(0, last["involuntary"] - first["involuntary"])
            if first and last else None
        ),
    }
    return record, diagnostics


def smooth_even_at_least(minimum: int) -> int:
    value = max(2, minimum + minimum % 2)
    while True:
        remainder = value
        for factor in (2, 3, 5, 7):
            while remainder % factor == 0:
                remainder //= factor
        if remainder == 1:
            return value
        value += 2


def longitude_count(mmax: int, mode: str) -> int:
    minimum = 2 * mmax + 1
    if mode == "odd_minimum":
        return minimum
    if mode == "even_smooth":
        return smooth_even_at_least(2 * mmax + 2)
    raise ValueError(mode)


def make_cases(
    smoke: bool,
    fixed_mmax: int | None = None,
    fixed_nphi: int | None = None,
    fixed_ntheta: int | None = None,
    selected_lmax: set[int] | None = None,
    max_lmax: int | None = None,
    dense_sweep: bool = False,
    selected_longitude_modes: set[str] | None = None,
    grid: str = "GL",
) -> list[dict]:
    if grid not in {"GL", "CC"}:
        raise ValueError(f"unsupported grid {grid}; expected GL or CC")
    if smoke and not dense_sweep:
        lmax_values = [0, 4, 16, 64]
    elif dense_sweep:
        lmax_values = sorted(set(range(257)) | set(DENSE_SWEEP_HIGH_LMAX))
    else:
        lmax_values = sorted(
            set(range(0, 33))
            | set(range(40, 257, 16))
            | {
                48, 62, 63, 64, 65, 126, 127, 128, 129, 159, 191, 223, 254,
                255, 256, 300, 320, 384, 448, 511, 512, 513, 640, 720, 721,
                768, 896, 1024, 1280, 1536, 1792, 2047, 2048, 3072,
            }
        )
    if selected_lmax is not None:
        # `--lmax` is an explicit case selector, so include requested degrees
        # outside the standard sweep instead of silently dropping them.
        lmax_values = sorted(set(lmax_values) | selected_lmax)
    key_sizes = {
        48, 62, 63, 64, 65, 126, 127, 128, 129, 159, 191, 223, 248, 254,
        255, 256, 300, 511, 512, 513, 720, 721, 1024, 1536, 2048,
    }
    cases: list[dict] = []
    for lmax in lmax_values:
        if selected_lmax is not None and lmax not in selected_lmax:
            continue
        if max_lmax is not None and lmax > max_lmax:
            continue
        if fixed_mmax is None:
            mmax_values = {lmax}
            if dense_sweep and lmax > 0:
                reduced_mmax = max(1, lmax // 4)
                if reduced_mmax <= lmax:
                    mmax_values.add(reduced_mmax)
            elif lmax >= 4:
                mmax_values.add(max(1, lmax // 4))
            if not dense_sweep and lmax in {255, 720, 1024}:
                mmax_values.add(1)
        elif fixed_mmax <= lmax:
            mmax_values = {fixed_mmax}
        else:
            continue
        for mmax in sorted(mmax_values):
            if fixed_nphi is not None:
                if fixed_nphi < 2 * mmax + 1:
                    raise ValueError(
                        f"fixed nphi={fixed_nphi} cannot represent mmax={mmax}"
                    )
                remainder = fixed_nphi
                for factor in (2, 3, 5, 7):
                    while remainder % factor == 0:
                        remainder //= factor
                longitude_modes = [
                    "even_smooth"
                    if fixed_nphi % 2 == 0 and remainder == 1
                    else "fixed_nphi"
                ]
            elif selected_lmax is not None and lmax in selected_lmax and selected_longitude_modes:
                longitude_modes = sorted(selected_longitude_modes)
            elif dense_sweep:
                longitude_modes = ["even_smooth"]
                if lmax in DENSE_SWEEP_ODD_LMAX:
                    longitude_modes.append("odd_minimum")
            elif lmax <= 32 or (lmax in key_sizes and lmax < 2048):
                longitude_modes = ["odd_minimum", "even_smooth"]
            elif smoke:
                longitude_modes = ["odd_minimum"]
            else:
                longitude_modes = ["even_smooth" if lmax % 2 == 0 else "odd_minimum"]
            for longitude_mode in longitude_modes:
                nphi = (
                    fixed_nphi
                    if fixed_nphi is not None
                    else longitude_count(mmax, longitude_mode)
                )
                minimum_ntheta = lmax + 1 + int(grid == "CC")
                ntheta = fixed_ntheta if fixed_ntheta is not None else minimum_ntheta
                if ntheta < minimum_ntheta:
                    raise ValueError(
                        f"fixed ntheta={ntheta} cannot exactly analyze lmax={lmax} on {grid}"
                    )
                ntheta_tag = "" if ntheta == minimum_ntheta else f"-ntheta{ntheta}"
                grid_tag = "" if grid == "GL" else f"{grid}-"
                case_id = (
                    f"{grid_tag}T{lmax}-m{mmax}{ntheta_tag}-{longitude_mode}"
                    if fixed_nphi is None
                    else f"{grid_tag}T{lmax}-m{mmax}{ntheta_tag}-nphi{fixed_nphi}"
                )
                cases.append({
                    "case_id": case_id,
                    "lmax": lmax,
                    "mmax": mmax,
                    "ntheta": ntheta,
                    "nphi": nphi,
                    "longitude_mode": longitude_mode,
                    "grid": grid,
                })
    return cases


def make_fixture(case: dict, directory: pathlib.Path) -> None:
    grid_salt = 0 if case.get("grid", "GL") == "GL" else 0xCC20261022
    rng = np.random.Generator(np.random.PCG64(
        0xD00C20261020 + grid_salt + case["lmax"] * 65537
        + case["mmax"] * 97 + case["nphi"]
    ))
    nalm = (case["mmax"] + 1) * (case["lmax"] + 1) - case["mmax"] * (case["mmax"] + 1) // 2
    alm = (rng.standard_normal(nalm) + 1j * rng.standard_normal(nalm)).astype("<c16")
    alm[: case["lmax"] + 1] = alm[: case["lmax"] + 1].real
    map_values = rng.standard_normal((case["ntheta"], case["nphi"])).astype("<f8")
    case["alm_path"] = directory / "alm.bin"
    case["map_path"] = directory / "map.bin"
    alm.tofile(case["alm_path"])
    map_values.tofile(case["map_path"])
    case["input_sha256"] = {
        "alm": sha256_file(case["alm_path"]),
        "map": sha256_file(case["map_path"]),
    }


def error_metrics(actual: np.ndarray, reference: np.ndarray) -> dict[str, float]:
    actual = np.asarray(actual)
    reference = np.asarray(reference)
    delta = np.abs(actual - reference)
    reference_abs = np.abs(reference)

    def stable_norm(values: np.ndarray) -> float:
        flat = np.abs(values.ravel())
        peak = float(flat.max(initial=0.0))
        if peak == 0.0:
            return 0.0
        return peak * math.sqrt(float(np.square(flat / peak).sum()))

    scale = max(stable_norm(reference), np.finfo(np.float64).tiny)
    maximum = float(reference_abs.max(initial=0.0))
    near_zero = reference_abs <= max(maximum * 1e-12, np.finfo(np.float64).tiny)
    valid = ~near_zero
    max_relative = float((delta[valid] / reference_abs[valid]).max(initial=0.0)) if valid.any() else 0.0
    return {
        "absolute_max": float(delta.max(initial=0.0)),
        "relative_l2": float(stable_norm(delta) / scale),
        "max_relative_above_1e-12_peak": max_relative,
        "near_zero_reference_count": int(near_zero.sum()),
    }


def validation_sidecar(output: pathlib.Path, name: str, dtype: str) -> np.ndarray:
    return np.fromfile(pathlib.Path(f"{output}.{name}"), dtype=dtype)


def validation_alm(output: pathlib.Path, name: str, backend: str, case: dict) -> np.ndarray:
    values = validation_sidecar(output, name, "<c16")
    if backend == "ducc":
        nalm = (case["mmax"] + 1) * (case["lmax"] + 1) - case["mmax"] * (case["mmax"] + 1) // 2
        values = values[case["mmax"] : case["mmax"] + nalm]
    return values


def selected_modes(lmax: int, mmax: int) -> list[tuple[int, int]]:
    return sorted({(0, 0), (lmax, 0), (lmax // 2, min(mmax, lmax // 2)), (lmax, mmax)})


def compact_index(lmax: int, degree: int, order: int) -> int:
    order_start = order * (lmax + 1) - order * (order - 1) // 2
    return order_start + degree - order


def compare_validation(case: dict, outputs: dict[str, pathlib.Path], records: dict[str, dict]) -> dict:
    nmap = case["ntheta"] * case["nphi"]
    nalm = (case["mmax"] + 1) * (case["lmax"] + 1) - case["mmax"] * (case["mmax"] + 1) // 2
    original_alm = np.fromfile(case["alm_path"], dtype="<c16")
    finite = all(records[backend]["validation"] for backend in ("rustsht", "ducc"))
    checks: dict[str, object] = {"finite_flags": {}}
    for backend in ("rustsht", "ducc"):
        summary = records[backend]["validation"]
        checks["finite_flags"][backend] = {
            name: bool(summary[name]) for name in ("zero_finite", "extreme_finite", "subnormal_finite")
        }
        finite &= all(checks["finite_flags"][backend].values())

    checks["roundtrip"] = {}
    for backend in ("rustsht", "ducc"):
        values = validation_alm(outputs[backend], "roundtrip.alm", backend, case)
        checks["roundtrip"][backend] = error_metrics(values, original_alm)
    checks["roundtrip"]["rust_ducc"] = error_metrics(
        validation_alm(outputs["rustsht"], "roundtrip.alm", "rustsht", case),
        validation_alm(outputs["ducc"], "roundtrip.alm", "ducc", case),
    )

    checks["basis"] = []
    for mode_index, (degree, order) in enumerate(selected_modes(case["lmax"], case["mmax"])):
        rust_map = validation_sidecar(outputs["rustsht"], f"basis-{mode_index}.map", "<f8")
        ducc_map = validation_sidecar(outputs["ducc"], f"basis-{mode_index}.map", "<f8")
        rust_alm = validation_alm(outputs["rustsht"], f"basis-{mode_index}.alm", "rustsht", case)
        ducc_alm = validation_alm(outputs["ducc"], f"basis-{mode_index}.alm", "ducc", case)
        expected = np.zeros(nalm, dtype="<c16")
        expected[compact_index(case["lmax"], degree, order)] = 1.0
        checks["basis"].append({
            "l": degree,
            "m": order,
            "map_rust_ducc": error_metrics(rust_map, ducc_map),
            "analysis_of_synthesis_rust": error_metrics(rust_alm, expected),
            "analysis_of_synthesis_ducc": error_metrics(ducc_alm, expected),
        })
        finite &= all(np.isfinite(values).all() for values in (rust_map, ducc_map, rust_alm, ducc_alm))

    checks["edge_cases"] = {}
    for case_name in ("zero", "extreme", "subnormal"):
        map_values = {
            backend: validation_sidecar(outputs[backend], f"{case_name}.map", "<f8")
            for backend in ("rustsht", "ducc")
        }
        alm_values = {
            backend: validation_alm(outputs[backend], f"{case_name}.alm", backend, case)
            for backend in ("rustsht", "ducc")
        }
        checks["edge_cases"][case_name] = {
            "map_rust_ducc": error_metrics(map_values["rustsht"], map_values["ducc"]),
            "alm_rust_ducc": error_metrics(alm_values["rustsht"], alm_values["ducc"]),
        }
        finite &= all(np.isfinite(values).all() for values in (*map_values.values(), *alm_values.values()))
        if case_name == "zero":
            checks["edge_cases"][case_name]["exact_zero"] = all(
                np.count_nonzero(values) == 0 for values in (*map_values.values(), *alm_values.values())
            )

    checks["adjoints"] = {
        backend: {
            "synthesis_relative_residual": records[backend]["validation"]["synthesis_adjoint_relative_residual"],
            "analysis_relative_residual": records[backend]["validation"]["analysis_adjoint_relative_residual"],
        }
        for backend in ("rustsht", "ducc")
    }
    checks["finite_outputs"] = bool(finite)
    checks["reset_consistent"] = {
        backend: bool(records[backend]["reset_consistent"])
        for backend in ("rustsht", "ducc")
    }
    return checks


def read_result(path: pathlib.Path, operation: str, case: dict, backend: str) -> np.ndarray:
    if operation == "synthesis":
        return np.fromfile(path, dtype="<f8")
    values = np.fromfile(path, dtype="<c16")
    if backend == "ducc":
        values = values[case["mmax"] : case["mmax"] + (case["mmax"] + 1) * (case["lmax"] + 1) - case["mmax"] * (case["mmax"] + 1) // 2]
    return values


def summarize(values: list[float]) -> dict[str, float]:
    array = np.asarray(values, dtype=np.float64)
    return {
        "median": float(np.median(array)),
        "p10": float(np.quantile(array, 0.10)),
        "p90": float(np.quantile(array, 0.90)),
        "min": float(array.min()),
        "max": float(array.max()),
    }


def run_matrix(
    cases: list[dict],
    binaries: dict[str, pathlib.Path],
    smoke: bool,
    threads_to_run: list[int] | None = None,
    operations_to_run: list[str] | None = None,
) -> tuple[list[dict], dict]:
    physical = physical_cpus()
    monitor_cpu = physical[8]
    threads_to_run = threads_to_run or [1, 8]
    operations_to_run = operations_to_run or ["analysis", "synthesis"]
    pairs = 1 if smoke else PROCESS_PAIRS
    target_ms = 5.0 if smoke else TARGET_BATCH_MS
    raw: list[dict] = []
    errors: list[dict] = []
    failures: list[dict] = []
    validations: list[dict] = []
    host_samples: list[dict] = []
    total = len(cases) * len(threads_to_run) * len(operations_to_run)
    progress = 0
    for case in cases:
        with tempfile.TemporaryDirectory(prefix="rustsht-ducc-parity-") as temporary:
            temp = pathlib.Path(temporary)
            try:
                make_fixture(case, temp)
            except Exception as error:
                for threads in threads_to_run:
                    for operation in operations_to_run:
                        failures.append({
                            "case_id": case["case_id"],
                            "lmax": case["lmax"],
                            "mmax": case["mmax"],
                            "ntheta": case["ntheta"],
                            "nphi": case["nphi"],
                            "longitude_mode": case["longitude_mode"],
                            "operation": operation,
                            "threads": threads,
                            "process_pair": -1,
                            "backend": "fixture",
                            "failure_class": "fixture_error",
                            "reason": str(error)[:16000],
                        })
                print(f"FAIL fixture {case['case_id']}: {error}", flush=True)
                continue
            for threads in threads_to_run:
                cpus = physical[:threads]
                for operation in operations_to_run:
                    progress += 1
                    print(
                        f"CASE {progress}/{total} {case['case_id']} {operation} q{threads}",
                        flush=True,
                    )
                    for pair in range(pairs):
                        order = ["rustsht", "ducc"] if pair % 2 == 0 else ["ducc", "rustsht"]
                        outputs = {}
                        pair_records = {}
                        validate = operation == "analysis" and pair == 0
                        for backend in order:
                            output_path = temp / f"{backend}-{operation}-q{threads}-p{pair}.bin"
                            command = worker_command(
                                binaries, backend, case, operation, threads, output_path, target_ms,
                                validate=validate,
                            )
                            try:
                                result, diagnostics = run_pinned(command, cpus, monitor_cpu)
                            except Exception as error:
                                reason = str(error)
                                lowered = reason.lower()
                                failure_class = (
                                    "unsupported" if any(term in lowered for term in
                                                         ("unsupported", "not supported", "invalid geometry"))
                                    else "worker_error"
                                )
                                failures.append({
                                    "case_id": case["case_id"],
                                    "lmax": case["lmax"],
                                    "mmax": case["mmax"],
                                    "ntheta": case["ntheta"],
                                    "nphi": case["nphi"],
                                    "longitude_mode": case["longitude_mode"],
                                    "operation": operation,
                                    "threads": threads,
                                    "process_pair": pair,
                                    "backend": backend,
                                    "failure_class": failure_class,
                                    "reason": reason[:16000],
                                })
                                print(
                                    f"FAIL {case['case_id']} {operation} q{threads} "
                                    f"pair={pair} backend={backend}: {reason[:500]}",
                                    flush=True,
                                )
                                continue
                            result.update({
                                "case_id": case["case_id"],
                                "lmax": case["lmax"],
                                "mmax": case["mmax"],
                                "ntheta": case["ntheta"],
                                "nphi": case["nphi"],
                                "longitude_mode": case["longitude_mode"],
                                "operation": operation,
                                "threads": threads,
                                "process_pair": pair,
                                "host_diagnostics": diagnostics,
                            })
                            raw.append(result)
                            pair_records[backend] = result
                            outputs[backend] = output_path
                            host_samples.append(diagnostics)
                        if len(outputs) != 2:
                            continue
                        try:
                            rust_values = read_result(outputs["rustsht"], operation, case, "rustsht")
                            ducc_values = read_result(outputs["ducc"], operation, case, "ducc")
                        except Exception as error:
                            failures.append({
                                "case_id": case["case_id"],
                                "lmax": case["lmax"],
                                "mmax": case["mmax"],
                                "ntheta": case["ntheta"],
                                "nphi": case["nphi"],
                                "longitude_mode": case["longitude_mode"],
                                "operation": operation,
                                "threads": threads,
                                "process_pair": pair,
                                "backend": "comparison",
                                "failure_class": "output_read_error",
                                "reason": str(error)[:16000],
                            })
                            continue
                        all_finite = bool(np.isfinite(rust_values).all() and np.isfinite(ducc_values).all())
                        if all_finite:
                            metrics = error_metrics(rust_values, ducc_values)
                        else:
                            metrics = {
                                "absolute_max": None,
                                "relative_l2": None,
                                "max_relative_above_1e-12_peak": None,
                                "nonfinite_rust_values": int((~np.isfinite(rust_values)).sum()),
                                "nonfinite_ducc_values": int((~np.isfinite(ducc_values)).sum()),
                            }
                            failures.append({
                                "case_id": case["case_id"],
                                "lmax": case["lmax"],
                                "mmax": case["mmax"],
                                "ntheta": case["ntheta"],
                                "nphi": case["nphi"],
                                "longitude_mode": case["longitude_mode"],
                                "operation": operation,
                                "threads": threads,
                                "process_pair": pair,
                                "backend": "comparison",
                                "failure_class": "numerical_nonfinite_output",
                                **metrics,
                            })
                        errors.append({
                            "case_id": case["case_id"],
                            "lmax": case["lmax"],
                            "mmax": case["mmax"],
                            "ntheta": case["ntheta"],
                            "nphi": case["nphi"],
                            "longitude_mode": case["longitude_mode"],
                            "operation": operation,
                            "threads": threads,
                            "process_pair": pair,
                            "finite_outputs": all_finite,
                            **metrics,
                        })
                        if validate:
                            try:
                                validation = compare_validation(case, outputs, pair_records)
                            except Exception as error:
                                failures.append({
                                    "case_id": case["case_id"],
                                    "lmax": case["lmax"],
                                    "mmax": case["mmax"],
                                    "ntheta": case["ntheta"],
                                    "nphi": case["nphi"],
                                    "longitude_mode": case["longitude_mode"],
                                    "operation": operation,
                                    "threads": threads,
                                    "process_pair": pair,
                                    "backend": "validation",
                                    "failure_class": "validation_error",
                                    "reason": str(error)[:16000],
                                })
                                continue
                            validation.update({
                                "case_id": case["case_id"],
                                "lmax": case["lmax"],
                                "mmax": case["mmax"],
                                "ntheta": case["ntheta"],
                                "nphi": case["nphi"],
                                "threads": threads,
                            })
                            validations.append(validation)
                            if not validation["finite_outputs"] or not all(validation["reset_consistent"].values()):
                                failures.append({
                                    "case_id": case["case_id"],
                                    "lmax": case["lmax"],
                                    "mmax": case["mmax"],
                                    "ntheta": case["ntheta"],
                                    "nphi": case["nphi"],
                                    "longitude_mode": case["longitude_mode"],
                                    "operation": operation,
                                    "threads": threads,
                                    "process_pair": pair,
                                    "backend": "validation",
                                    "failure_class": "validation_failed",
                                    "finite_outputs": validation["finite_outputs"],
                                    "reset_consistent": validation["reset_consistent"],
                                })
                                print(
                                    f"VALIDATION WARNING {case['case_id']} finite={validation['finite_outputs']} reset={validation['reset_consistent']}",
                                    flush=True,
                                )
            case["map_path"] = str(case["map_path"])
            case["alm_path"] = str(case["alm_path"])
    host = summarize_host(host_samples)
    return raw, {"errors": errors, "validations": validations, "host": host,
                 "failures": failures}


def summarize_host(samples: list[dict]) -> dict:
    frequencies = [value for sample in samples for value in sample["frequency_ghz"]]
    waits = [sample["wait_fraction"] for sample in samples if sample["wait_fraction"] is not None]
    involuntary = [sample["involuntary_switches"] for sample in samples if sample["involuntary_switches"] is not None]
    return {
        "frequency_ghz_median": float(np.median(frequencies)) if frequencies else None,
        "frequency_ghz_p10": float(np.quantile(frequencies, 0.1)) if frequencies else None,
        "frequency_ghz_p90": float(np.quantile(frequencies, 0.9)) if frequencies else None,
        "runqueue_wait_fraction_median": float(np.median(waits)) if waits else None,
        "runqueue_wait_fraction_p90": float(np.quantile(waits, 0.9)) if waits else None,
        "involuntary_switches_median": float(np.median(involuntary)) if involuntary else None,
        "involuntary_switches_max": int(max(involuntary)) if involuntary else None,
        "sampled_worker_processes": len(samples),
    }


def hardware_metadata(ducc_src: pathlib.Path) -> dict:
    flags = cpu_flags()
    meminfo = pathlib.Path("/proc/meminfo").read_text()
    memory_kib = int(re.search(r"^MemTotal:\s+(\d+)", meminfo, re.MULTILINE).group(1))
    cargo_lock = (ROOT / "Cargo.lock").read_text()
    pulp_match = re.search(r'\[\[package\]\]\nname = "pulp"\nversion = "([^"]+)', cargo_lock)
    ducc_project = (ducc_src / "pyproject.toml").read_text()
    ducc_version_match = re.search(r'^version\s*=\s*"([^"]+)"', ducc_project, re.MULTILINE)
    cxx = shlex.split(os.environ.get("CXX", "g++"))
    selected_cpus = physical_cpus()
    source_status = command_output(
        ["git", "status", "--porcelain", "--", "src", "Cargo.toml", "Cargo.lock"]
    )
    return {
        "cpu_model": cpu_model(),
        "architecture": platform.machine(),
        "physical_cores_visible": len(physical_cpus()),
        "selected_q1_cpu": selected_cpus[0],
        "selected_q8_cpus": selected_cpus[:8],
        "selected_q8_topology": [topology(cpu) for cpu in selected_cpus[:8]],
        "selected_q8_l3_cache_ids": [l3_cache_id(cpu) for cpu in selected_cpus[:8]],
        "logical_cpus_visible": len(os.sched_getaffinity(0)),
        "installed_ram_gib": round(memory_kib / 1024 / 1024, 2),
        "cpu_features": [name for name in ("avx2", "fma", "avx512f") if name in flags],
        "os": platform.platform(),
        "rustc": command_output(["rustc", "--version"]),
        "rustc_verbose": command_output(["rustc", "-Vv"]),
        "cargo": command_output(["cargo", "--version"]),
        "rust_source_revision": command_output(["git", "rev-parse", "HEAD"]),
        "rust_source_worktree_clean": not bool(source_status),
        "repository_worktree_clean": not bool(command_output(["git", "status", "--porcelain"])),
        "cxx": command_output(cxx + ["--version"]).splitlines()[0],
        "pulp": pulp_match.group(1) if pulp_match else "unknown",
        "ducc_source_revision": command_output(["git", "rev-parse", "HEAD"], ducc_src),
        "ducc_version": ducc_version_match.group(1) if ducc_version_match else "unknown",
        "rust_flags": RUST_FLAGS,
        "ducc_flags": "DUCC sources: -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; benchmark driver: -O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3",
        "rust_dispatch": "runtime Pulp Arch::new; worker records actual specialization",
        "rust_call_boundary": "direct ShtPlan calls on a process-global Rayon pool; no per-call ThreadPool::install wrapper",
        "ducc_dispatch": "fixed x86-64-v3 C++ build (AVX2/FMA, four f64 lanes)",
        "affinity_policy": "q1: one allowed physical core; q8: eight allowed physical cores",
    }


def build(ducc_src: pathlib.Path, build_dir: pathlib.Path, skip_build: bool) -> dict[str, pathlib.Path]:
    cargo_target_dir = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    rust_binary = cargo_target_dir / "release/examples/ducc_full_transform_parity"
    ducc_binary = build_dir / "ducc_full_transform_parity"
    if not skip_build:
        env = os.environ.copy()
        env["RUSTFLAGS"] = RUST_FLAGS
        subprocess.run(
            ["cargo", "build", "--release", "--example", "ducc_full_transform_parity", "--features", "parallel"],
            cwd=ROOT, env=env, check=True,
        )
        subprocess.run(
            ["cmake", "-S", str(DATA_DIR), "-B", str(build_dir),
             f"-DDUCC_SRC={ducc_src}", "-DCMAKE_BUILD_TYPE=Release",
             "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
             f"-DCMAKE_CXX_COMPILER={shlex.split(os.environ.get('CXX', 'g++'))[0]}"],
            cwd=ROOT, check=True,
        )
        subprocess.run(["cmake", "--build", str(build_dir), "--parallel", "4"], cwd=ROOT, check=True)
    if not rust_binary.exists() or not ducc_binary.exists():
        raise FileNotFoundError("benchmark binaries missing; rerun without --skip-build")
    compile_commands = build_dir / "compile_commands.json"
    if not compile_commands.is_file():
        raise FileNotFoundError(f"missing DUCC compile_commands.json: {compile_commands}")
    commands = json.loads(compile_commands.read_text())
    sht = next((entry for entry in commands if pathlib.Path(entry["file"]).name == "sht.cc"), None)
    driver = next((entry for entry in commands
                   if pathlib.Path(entry["file"]).name == "ducc_full_transform_parity.cpp"), None)
    if sht is None or driver is None:
        raise RuntimeError("DUCC compile database lacks the SHT TU or native benchmark driver")
    sht_command = sht.get("command", " ".join(sht.get("arguments", [])))
    driver_command = driver.get("command", " ".join(driver.get("arguments", [])))
    required_sht_flags = ("-O3", "-ffast-math", "-ffp-contract=fast", "-march=x86-64-v3")
    if any(flag not in sht_command for flag in required_sht_flags):
        raise RuntimeError(f"DUCC SHT compile flags do not match the benchmark contract: {sht_command}")
    if "-fno-fast-math" not in driver_command or "-ffast-math" not in driver_command:
        raise RuntimeError(f"native driver must compile with target fast-math overridden by -fno-fast-math: {driver_command}")
    if driver_command.rfind("-fno-fast-math") < driver_command.rfind("-ffast-math"):
        raise RuntimeError(f"driver -fno-fast-math does not override target -ffast-math: {driver_command}")
    return {"rustsht": rust_binary, "ducc": ducc_binary}


def percentile(values: list[float], p: float) -> float:
    return float(np.quantile(np.asarray(values, dtype=np.float64), p))


def aggregate(raw: list[dict], errors: list[dict]) -> list[dict]:
    groups: dict[tuple, dict[str, list[dict]]] = defaultdict(lambda: {"rustsht": [], "ducc": []})
    for entry in raw:
        key = (entry["case_id"], entry["operation"], entry["threads"])
        groups[key][entry["backend"]].append(entry)
    error_groups: dict[tuple, list[dict]] = defaultdict(list)
    for entry in errors:
        error_groups[(entry["case_id"], entry["operation"], entry["threads"])].append(entry)
    rows = []
    for (case_id, operation, threads), backends in groups.items():
        rust_by_pair = {proc["process_pair"]: proc for proc in backends["rustsht"]}
        ducc_by_pair = {proc["process_pair"]: proc for proc in backends["ducc"]}
        valid_error_pairs = {
            item["process_pair"] for item in error_groups[(case_id, operation, threads)]
            if item.get("relative_l2") is not None
        }
        common_pairs = sorted(set(rust_by_pair) & set(ducc_by_pair) & valid_error_pairs)
        if not common_pairs:
            continue
        matched_rust = [rust_by_pair[pair] for pair in common_pairs]
        matched_ducc = [ducc_by_pair[pair] for pair in common_pairs]
        rust_values = [value for proc in matched_rust for value in proc["batch_ms"]]
        ducc_values = [value for proc in matched_ducc for value in proc["batch_ms"]]
        paired_ratios = []
        for rust_proc, ducc_proc in zip(matched_rust, matched_ducc):
            for rust_time, ducc_time in zip(rust_proc["batch_ms"], ducc_proc["batch_ms"]):
                paired_ratios.append(rust_time / ducc_time)
        rs = summarize(rust_values)
        ds = summarize(ducc_values)
        rust_batch_totals = [value for proc in matched_rust for value in proc["batch_total_ms"]]
        ducc_batch_totals = [value for proc in matched_ducc for value in proc["batch_total_ms"]]
        batch_totals = rust_batch_totals + ducc_batch_totals
        short_operation_entries = [
            entry for backend in (matched_rust, matched_ducc) for entry in backend
            if entry["warmup_ms"] < 100.0
        ]
        target_window_values = [
            elapsed for entry in short_operation_entries for elapsed in entry["batch_total_ms"]
        ]
        target_window_fraction = (
            sum(20.0 <= elapsed <= 100.0 for elapsed in target_window_values) / len(target_window_values)
            if target_window_values else None
        )
        pair_summary = summarize(paired_ratios)
        error_values = [
            item for item in error_groups[(case_id, operation, threads)]
            if item["process_pair"] in common_pairs and item.get("relative_l2") is not None
        ]
        if not error_values:
            continue
        errors_summary = {
            key: max(item[key] for item in error_values)
            for key in ("absolute_max", "relative_l2", "max_relative_above_1e-12_peak")
        }
        template = matched_rust[0]
        rows.append({
            "case_id": case_id,
            "grid": template.get("grid", "GL"),
            "lmax": template["lmax"],
            "mmax": template["mmax"],
            "ntheta": template["ntheta"],
            "nphi": template["nphi"],
            "longitude_mode": template["longitude_mode"],
            "operation": operation,
            "threads": threads,
            "rust_warm_ms": rs["median"],
            "rust_p10_ms": rs["p10"],
            "rust_p90_ms": rs["p90"],
            "ducc_warm_ms": ds["median"],
            "ducc_p10_ms": ds["p10"],
            "ducc_p90_ms": ds["p90"],
            "rust_ducc_ratio": rs["median"] / ds["median"],
            "paired_ratio_p10": pair_summary["p10"],
            "paired_ratio_p90": pair_summary["p90"],
            "excess_ms": rs["median"] - ds["median"],
            **errors_summary,
            "rust_cold_first_ms": statistics.median(proc["first_ms"] for proc in matched_rust),
            "ducc_cold_first_ms": statistics.median(proc["first_ms"] for proc in matched_ducc),
            "rust_setup_ms": statistics.median(proc["setup_ms"] for proc in matched_rust),
            "ducc_setup_ms": statistics.median(proc["setup_ms"] for proc in matched_ducc),
            "rust_fixture_load_ms": statistics.median(proc["fixture_load_ms"] for proc in matched_rust),
            "ducc_fixture_load_ms": statistics.median(proc["fixture_load_ms"] for proc in matched_ducc),
            "rust_arch": matched_rust[0]["arch"],
            "ducc_arch": matched_ducc[0]["arch"],
            "batch_count": len(rust_values),
            "batch_elapsed_median_ms": statistics.median(batch_totals),
            "batch_elapsed_p10_ms": percentile(batch_totals, 0.10),
            "batch_elapsed_p90_ms": percentile(batch_totals, 0.90),
            "20_100ms_batch_fraction_for_short_ops": target_window_fraction,
            "process_invocations_per_backend": len(matched_rust),
            "completed_process_pairs": len(common_pairs),
            "measurement_status": "complete" if len(common_pairs) == PROCESS_PAIRS else "partial",
        })
    return sorted(rows, key=lambda row: (row["lmax"], row["mmax"], row["nphi"], row["operation"], row["threads"]))


def plot_svg(path: pathlib.Path, title: str, series: list[dict], xkey: str, ykey: str, y_label: str) -> None:
    width, height = 960, 540
    left, right, top, bottom = 78, 28, 56, 76
    plot_w, plot_h = width - left - right, height - top - bottom
    xs = [float(point[xkey]) for item in series for point in item["points"]]
    ys = [float(point[ykey]) for item in series for point in item["points"] if math.isfinite(float(point[ykey]))]
    if not xs or not ys:
        return
    xmin, xmax = min(xs), max(xs)
    ymin, ymax = min(ys), max(ys)
    if xmax == xmin:
        xmax = xmin + 1
    if ymax == ymin:
        ymax = ymin + 1
    if ykey == "ratio":
        ymin = min(0.5, ymin)
        ymax = max(2.0, ymax)
    else:
        ymin = min(0.0, ymin)
    pad = (ymax - ymin) * 0.06
    ymin -= pad
    ymax += pad

    def xy(x: float, y: float) -> tuple[float, float]:
        return left + (x - xmin) / (xmax - xmin) * plot_w, top + (ymax - y) / (ymax - ymin) * plot_h

    colors = ["#174a7e", "#d1495b", "#2a9d8f", "#f4a261", "#6a4c93", "#577590"]
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
             '<rect width="100%" height="100%" fill="white"/>',
             f'<text x="{left}" y="30" font-family="sans-serif" font-size="21" font-weight="bold">{title}</text>']
    for tick in range(6):
        y = ymin + (ymax - ymin) * tick / 5
        px, py = xy(xmin, y)
        parts.append(f'<line x1="{left}" y1="{py:.1f}" x2="{width-right}" y2="{py:.1f}" stroke="#ddd"/>')
        parts.append(f'<text x="{left-10}" y="{py+4:.1f}" text-anchor="end" font-family="sans-serif" font-size="12">{y:.3g}</text>')
    parts.append(f'<line x1="{left}" y1="{top}" x2="{left}" y2="{height-bottom}" stroke="#333"/>')
    parts.append(f'<line x1="{left}" y1="{height-bottom}" x2="{width-right}" y2="{height-bottom}" stroke="#333"/>')
    if ykey == "ratio" and ymin < 1 < ymax:
        _, y1 = xy(xmin, 1)
        parts.append(f'<line x1="{left}" y1="{y1:.1f}" x2="{width-right}" y2="{y1:.1f}" stroke="#555" stroke-dasharray="5,5"/>')
    for index, item in enumerate(series):
        points = sorted(item["points"], key=lambda point: float(point[xkey]))
        color = colors[index % len(colors)]
        coords = [xy(float(point[xkey]), float(point[ykey])) for point in points]
        if len(coords) > 1:
            points_text = " ".join(f"{x:.1f},{y:.1f}" for x, y in coords)
            parts.append(f'<polyline fill="none" stroke="{color}" stroke-width="2" points="{points_text}"/>')
        for x, y in coords:
            parts.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="3" fill="{color}"/>')
        legend_x = left + index * 175
        parts.append(f'<line x1="{legend_x}" y1="{height-28}" x2="{legend_x+24}" y2="{height-28}" stroke="{color}" stroke-width="3"/>')
        parts.append(f'<text x="{legend_x+30}" y="{height-24}" font-family="sans-serif" font-size="12">{item["name"]}</text>')
    parts.append(f'<text x="{left+plot_w/2}" y="{height-4}" text-anchor="middle" font-family="sans-serif" font-size="14">lmax</text>')
    parts.append(f'<text transform="translate(18 {top+plot_h/2}) rotate(-90)" text-anchor="middle" font-family="sans-serif" font-size="14">{y_label}</text>')
    parts.append('</svg>')
    path.write_text("\n".join(parts))



def issue_lmax_ticks(xmax: int) -> list[int]:
    """Keep the dense low-degree data while labeling a readable subset."""
    candidates = [0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 320, 512,
                  720, 1024, 1536, 2048, 2560, 3072]
    return [degree for degree in candidates if degree <= xmax]


def grid_display_name(grid: str) -> str:
    return "Clenshaw–Curtis" if grid == "CC" else "Gauss–Legendre"


def plot_issue_ratio_svg(path: pathlib.Path, operation: str, order_label: str,
                         points_by_threads: dict[int, list[dict]],
                         grid: str = "GL") -> None:
    """Issue-ready full-transform ratio chart: sampled lmax, q1/q8, log axes.

    Display even FFT-smooth longitude counts only; odd-minimum FFTs are a
    separate workload. No averaging or smoothing between observed lmax points.
    """
    width, height = 1260, 680
    left, right, top, bottom = 98, 40, 94, 139
    plot_w, plot_h = width - left - right, height - top - bottom
    observed = [
        (int(point["lmax"]), float(point["ratio"]))
        for points in points_by_threads.values() for point in points
    ]
    if not observed:
        path.write_text(
            f'<svg xmlns="http://www.w3.org/2000/svg" width="1260" height="680" viewBox="0 0 1260 680">'
            f'<rect width="100%" height="100%" fill="#fff"/>'
            f'<text x="98" y="35" font-family="sans-serif" font-size="25" font-weight="bold">RustSHT / DUCC — {operation.capitalize()} ({order_label})</text>'
            f'<text x="98" y="61" font-family="sans-serif" font-size="15" fill="#555">f64 · {grid_display_name(grid)} · even-smooth longitude sampling · warmed full transforms</text>'
            '<text x="630" y="340" text-anchor="middle" font-family="sans-serif" font-size="18" fill="#555">No completed measurements for this selection</text>'
            '<text x="960" y="77" font-family="sans-serif" font-size="14">q1 / q8</text></svg>\n'
        )
        return
    ymin = min(0.25, min(value for _, value in observed) * 0.9)
    ymax = max(16.0, max(value for _, value in observed) * 1.1)
    if ymin <= 0:
        raise ValueError("ratio plot requires positive runtimes")
    xmax = max(1, max(degree for degree, _ in observed))
    xlogmax = math.log1p(xmax)
    ylogmin, ylogmax = math.log(ymin), math.log(ymax)

    def xcoord(degree: float) -> float:
        return left + math.log1p(degree) / xlogmax * plot_w

    def ycoord(ratio: float) -> float:
        return top + (ylogmax - math.log(ratio)) / (ylogmax - ylogmin) * plot_h

    def fmt_number(value: float) -> str:
        return f"{value:g}"

    colors = {1: "#155e99", 8: "#bf3b42"}
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#fff"/>',
        f'<text x="{left}" y="35" font-family="sans-serif" font-size="25" font-weight="bold">RustSHT / DUCC — {operation.capitalize()} ({order_label})</text>',
        f'<text x="{left}" y="61" font-family="sans-serif" font-size="15" fill="#555">f64 · {grid_display_name(grid)} · even-smooth longitude sampling · warmed full transforms · lower is faster</text>',
    ]
    tick_values = [0.25, 0.5, 1, 2, 3, 4, 5, 6, 8, 10, 12, 16]
    tick_values = [v for v in tick_values if ymin <= v <= ymax]
    if all(abs(v - 1) > 1e-12 for v in tick_values):
        tick_values.append(1)
    for ratio in sorted(tick_values):
        y = ycoord(ratio)
        strong = ratio == 1
        stroke = "#53616c" if strong else "#e3e8ed"
        dash = ' stroke-dasharray="6 4"' if strong else ""
        parts.append(f'<line x1="{left}" y1="{y:.1f}" x2="{width-right}" y2="{y:.1f}" stroke="{stroke}"{dash}/>')
        suffix = "× parity" if strong else "×"
        parts.append(f'<text x="{left-11}" y="{y+4:.1f}" text-anchor="end" font-family="sans-serif" font-size="13" font-weight="{"bold" if strong else "normal"}">{fmt_number(ratio)}{suffix}</text>')
    xticks = issue_lmax_ticks(xmax)
    for i, degree in enumerate(xticks):
        x = xcoord(degree)
        parts.append(f'<line x1="{x:.1f}" y1="{height-bottom}" x2="{x:.1f}" y2="{height-bottom+6}" stroke="#77818b"/>')
        # Rotate long tick labels and alternate placement at close transitions.
        label_y = height - bottom + 27 + (18 if i >= 7 and i % 2 else 0)
        parts.append(f'<text transform="translate({x:.1f} {label_y}) rotate(-48)" text-anchor="end" font-family="sans-serif" font-size="12" fill="#394651">{degree}</text>')
    for boundary_degree, label in ((64, "T64"), (256, "T256")):
        boundary = xcoord(boundary_degree)
        parts.append(f'<line x1="{boundary:.1f}" y1="{top}" x2="{boundary:.1f}" y2="{height-bottom}" stroke="#7b8794" stroke-dasharray="4 4"/>')
        parts.append(f'<text x="{boundary+4:.1f}" y="{top+39}" font-family="sans-serif" font-size="12" fill="#4c5a67">{label}</text>')
    parts.append(f'<text x="{left+9}" y="{top+20}" font-family="sans-serif" font-size="13" fill="#4c5a67">Dense: every lmax = 0…256</text>')
    if xmax >= 2048:
        onset = xcoord(2048)
        parts.append(f'<line x1="{onset:.1f}" y1="{top}" x2="{onset:.1f}" y2="{height-bottom}" stroke="#9d752c" stroke-dasharray="4 4"/>')
        parts.append(f'<text x="{onset-8:.1f}" y="{top+20}" text-anchor="end" font-family="sans-serif" font-size="13" fill="#825a18">T2048</text>')
    parts.append(f'<line x1="{left}" y1="{height-bottom}" x2="{width-right}" y2="{height-bottom}" stroke="#34404a" stroke-width="1.5"/>')
    parts.append(f'<line x1="{left}" y1="{top}" x2="{left}" y2="{height-bottom}" stroke="#34404a" stroke-width="1.5"/>')
    for threads in (1, 8):
        points = sorted(points_by_threads.get(threads, []), key=lambda pt: pt["lmax"])
        if not points:
            continue
        coords = [(xcoord(p["lmax"]), ycoord(p["ratio"])) for p in points]
        color = colors[threads]
        poly = " ".join(f"{x:.1f},{y:.1f}" for x, y in coords)
        parts.append(f'<polyline fill="none" stroke="{color}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" points="{poly}"/>')
        for x, y in coords:
            parts.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="2.4" fill="{color}"/>')
    for index, threads in enumerate((1, 8)):
        x = width - right - 207 + 105 * index
        y = 77
        parts.append(f'<line x1="{x}" y1="{y}" x2="{x+24}" y2="{y}" stroke="{colors[threads]}" stroke-width="3"/>')
        parts.append(f'<text x="{x+30}" y="{y+5}" font-family="sans-serif" font-size="14" fill="#24333f">q{threads}</text>')
    parts += [
        f'<text x="{left+plot_w/2:.1f}" y="{height-12}" text-anchor="middle" font-family="sans-serif" font-size="15">lmax (log(1 + lmax) spacing; sampled degrees, not evenly spaced)</text>',
        f'<text transform="translate(20 {top+plot_h/2:.1f}) rotate(-90)" text-anchor="middle" font-family="sans-serif" font-size="15">RustSHT / DUCC runtime (log scale)</text>',
        '</svg>',
    ]
    path.write_text("\n".join(parts) + "\n")



def plot_issue_runtime_svg(path: pathlib.Path, operation: str,
                           series: list[dict], grid: str = "GL") -> None:
    """Issue-ready log-log absolute timings, matched to the ratio charts.

    The plotted points are the observed warmed full-order, even-smooth
    configurations only. Color represents backend; solid/dashed represent q1/q8.
    """
    width, height = 1260, 680
    left, right, top, bottom = 98, 40, 94, 139
    plot_w, plot_h = width - left - right, height - top - bottom
    observed = [
        (int(point["lmax"]), float(point["ms"]))
        for item in series for point in item["points"]
    ]
    if not observed:
        return
    if any(runtime <= 0 or not math.isfinite(runtime) for _, runtime in observed):
        raise ValueError("runtime log plot requires positive finite timings")
    xmax = max(1, max(degree for degree, _ in observed))
    xmin_runtime = min(value for _, value in observed)
    xmax_runtime = max(value for _, value in observed)
    # Fixed engineering-decade ticks keep analysis and synthesis comparable.
    runtime_ticks = [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 1, 3,
                     10, 30, 100, 300, 1000, 3000, 10000]
    ymin = min(0.0007, xmin_runtime / 1.15)
    ymax = max(10000.0, xmax_runtime * 1.15)
    xlogmax = math.log1p(xmax)
    logmin, logmax = math.log(ymin), math.log(ymax)

    def xcoord(degree: float) -> float:
        return left + math.log1p(degree) / xlogmax * plot_w

    def ycoord(runtime: float) -> float:
        return top + (logmax - math.log(runtime)) / (logmax - logmin) * plot_h

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#fff"/>',
        f'<text x="{left}" y="35" font-family="sans-serif" font-size="25" font-weight="bold">RustSHT vs DUCC — {operation.capitalize()} runtime (full mmax)</text>',
        f'<text x="{left}" y="61" font-family="sans-serif" font-size="15" fill="#555">f64 · {grid_display_name(grid)} · even-smooth longitude sampling · warmed full transforms · lower is faster</text>',
    ]
    for runtime in runtime_ticks:
        if not ymin <= runtime <= ymax:
            continue
        y = ycoord(runtime)
        parts.append(f'<line x1="{left}" y1="{y:.1f}" x2="{width-right}" y2="{y:.1f}" stroke="#e3e8ed"/>')
        parts.append(f'<text x="{left-11}" y="{y+4:.1f}" text-anchor="end" font-family="sans-serif" font-size="13">{runtime:g}</text>')
    xticks = issue_lmax_ticks(xmax)
    for i, degree in enumerate(xticks):
        x = xcoord(degree)
        label_y = height - bottom + 27 + (18 if i >= 7 and i % 2 else 0)
        parts.append(f'<line x1="{x:.1f}" y1="{height-bottom}" x2="{x:.1f}" y2="{height-bottom+6}" stroke="#77818b"/>')
        parts.append(f'<text transform="translate({x:.1f} {label_y}) rotate(-48)" text-anchor="end" font-family="sans-serif" font-size="12" fill="#394651">{degree}</text>')
    for boundary_degree, label in ((64, "T64"), (256, "T256")):
        boundary = xcoord(boundary_degree)
        parts.append(f'<line x1="{boundary:.1f}" y1="{top}" x2="{boundary:.1f}" y2="{height-bottom}" stroke="#7b8794" stroke-dasharray="4 4"/>')
        parts.append(f'<text x="{boundary+4:.1f}" y="{top+39}" font-family="sans-serif" font-size="12" fill="#4c5a67">{label}</text>')
    parts.append(f'<text x="{left+9}" y="{top+20}" font-family="sans-serif" font-size="13" fill="#4c5a67">Dense: every lmax = 0…256</text>')
    if xmax >= 2048:
        onset = xcoord(2048)
        parts.append(f'<line x1="{onset:.1f}" y1="{top}" x2="{onset:.1f}" y2="{height-bottom}" stroke="#9d752c" stroke-dasharray="4 4"/>')
        parts.append(f'<text x="{onset-8:.1f}" y="{top+20}" text-anchor="end" font-family="sans-serif" font-size="13" fill="#825a18">T2048</text>')
    parts.append(f'<line x1="{left}" y1="{height-bottom}" x2="{width-right}" y2="{height-bottom}" stroke="#34404a" stroke-width="1.5"/>')
    parts.append(f'<line x1="{left}" y1="{top}" x2="{left}" y2="{height-bottom}" stroke="#34404a" stroke-width="1.5"/>')

    palette = {"RustSHT": "#155e99", "DUCC": "#bf3b42"}
    for item in series:
        backend, threads = item["backend"], item["threads"]
        color = palette[backend]
        dash = ' stroke-dasharray="7 5"' if threads == 8 else ""
        points = sorted(item["points"], key=lambda point: point["lmax"])
        coords = [(xcoord(p["lmax"]), ycoord(p["ms"])) for p in points]
        if len(coords) > 1:
            poly = " ".join(f"{x:.1f},{y:.1f}" for x, y in coords)
            parts.append(f'<polyline fill="none" stroke="{color}" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"{dash} points="{poly}"/>')
        for x, y in coords:
            parts.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="2.2" fill="{color}"/>')
    for index, (backend, threads) in enumerate(
        (("RustSHT", 1), ("DUCC", 1), ("RustSHT", 8), ("DUCC", 8))
    ):
        x = left + index * 193
        y = 77
        color = palette[backend]
        dash = ' stroke-dasharray="7 5"' if threads == 8 else ""
        parts.append(f'<line x1="{x}" y1="{y}" x2="{x+24}" y2="{y}" stroke="{color}" stroke-width="3"{dash}/>')
        parts.append(f'<text x="{x+30}" y="{y+5}" font-family="sans-serif" font-size="14" fill="#24333f">{backend} q{threads}</text>')
    parts += [
        f'<text x="{left+plot_w/2:.1f}" y="{height-12}" text-anchor="middle" font-family="sans-serif" font-size="15">lmax (log(1 + lmax) spacing; sampled degrees, not evenly spaced)</text>',
        f'<text transform="translate(20 {top+plot_h/2:.1f}) rotate(-90)" text-anchor="middle" font-family="sans-serif" font-size="15">Warmed transform time (ms, log scale)</text>',
        '</svg>',
    ]
    path.write_text("\n".join(parts) + "\n")


def write_issue_runtime_plots(rows: list[dict], output_dir: pathlib.Path,
                              grid: str = "GL") -> None:
    """Create issue-ready full-mmax actual runtimes without rerunning timings."""
    output_dir.mkdir(parents=True, exist_ok=True)
    for operation in ("analysis", "synthesis"):
        series = []
        for backend, field in (("RustSHT", "rust_warm_ms"), ("DUCC", "ducc_warm_ms")):
            for threads in (1, 8):
                points = [
                    {"lmax": int(row["lmax"]), "ms": float(row[field])}
                    for row in rows
                    if row["operation"] == operation
                    and int(row["threads"]) == threads
                    and int(row["mmax"]) == int(row["lmax"])
                    and row["longitude_mode"] == "even_smooth"
                ]
                series.append({"backend": backend, "threads": threads, "points": points})
        plot_issue_runtime_svg(
            output_dir / f"runtime-{operation}.svg", operation, series, grid,
        )


def write_issue_ratio_plots(rows: list[dict], output_dir: pathlib.Path,
                            grid: str = "GL") -> None:
    """Regenerate consistent GitHub-issue figures without rerunning timings."""
    output_dir.mkdir(parents=True, exist_ok=True)
    for operation in ("analysis", "synthesis"):
        for label, full in (("full mmax", True), ("reduced mmax", False)):
            grouped = {}
            for threads in (1, 8):
                grouped[threads] = [
                    {"lmax": int(row["lmax"]), "ratio": float(row["rust_ducc_ratio"])}
                    for row in rows
                    if row["operation"] == operation
                    and int(row["threads"]) == threads
                    and (int(row["mmax"]) == int(row["lmax"])) == full
                    and row["longitude_mode"] == "even_smooth"
                ]
            plot_issue_ratio_svg(
                output_dir / f"ratio-{operation}-{label.replace(' ', '-')}.svg",
                operation, label, grouped, grid,
            )


def plot_log_degree_series(path: pathlib.Path, title: str, subtitle: str,
                           series: list[dict], y_label: str) -> None:
    """Plot observed positive series with log(1+lmax) x and log y axes."""
    points = [point for item in series for point in item["points"]]
    if not points:
        return
    width, height = 1260, 690
    left, right, top, bottom = 100, 38, 100, 136
    plot_w, plot_h = width - left - right, height - top - bottom
    x_max = max(1, max(int(point["lmax"]) for point in points))
    y_values = [float(value) for point in points
                for value in (point["value"], point.get("low", point["value"]),
                              point.get("high", point["value"]))]
    y_min = min(y_values) * 0.8
    y_max = max(y_values) * 1.2
    if y_max / y_min < 2:
        y_min /= 1.5
        y_max *= 1.5
    log_x_max = math.log1p(x_max)
    log_y_min, log_y_max = math.log(y_min), math.log(y_max)

    def xcoord(degree: float) -> float:
        return left + math.log1p(degree) / log_x_max * plot_w

    def ycoord(value: float) -> float:
        return top + (log_y_max - math.log(value)) / (log_y_max - log_y_min) * plot_h

    colors = ["#155e99", "#bf3b42", "#2a806c", "#9467bd", "#d17a22", "#477ca8"]
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#fff"/>',
        f'<text x="{left}" y="34" font-family="sans-serif" font-size="24" font-weight="bold">{title}</text>',
        f'<text x="{left}" y="60" font-family="sans-serif" font-size="14" fill="#555">{subtitle}</text>',
    ]
    for item in series:
        for start, end, label in item.get("regions", []):
            x0, x1 = xcoord(start), xcoord(end)
            parts.append(
                f'<rect x="{x0:.1f}" y="{top}" width="{max(1.0, x1-x0):.1f}" '
                f'height="{plot_h}" fill="#9ab6c8" opacity="0.20"/>'
            )
            parts.append(
                f'<text x="{(x0+x1)/2:.1f}" y="{top+14}" text-anchor="middle" '
                f'font-family="sans-serif" font-size="10" fill="#405a6b">{label}</text>'
            )
    y_ticks = [0.125, 0.25, 0.5, 1, 2, 4, 8, 16]
    for value in y_ticks:
        if not y_min <= value <= y_max:
            continue
        y = ycoord(value)
        strong = abs(value - 1) < 1e-12
        parts.append(f'<line x1="{left}" y1="{y:.1f}" x2="{width-right}" y2="{y:.1f}" stroke="{"#586675" if strong else "#e3e8ed"}"{" stroke-dasharray=\"6 4\"" if strong else ""}/>')
        parts.append(f'<text x="{left-10}" y="{y+4:.1f}" text-anchor="end" font-family="sans-serif" font-size="13">{value:g}{"× parity" if strong else "×"}</text>')
    xticks = issue_lmax_ticks(x_max)
    for index, degree in enumerate(xticks):
        x = xcoord(degree)
        label_y = height - bottom + 27 + (17 if index >= 8 and index % 2 else 0)
        parts.append(f'<line x1="{x:.1f}" y1="{height-bottom}" x2="{x:.1f}" y2="{height-bottom+6}" stroke="#77818b"/>')
        parts.append(f'<text transform="translate({x:.1f} {label_y}) rotate(-48)" text-anchor="end" font-family="sans-serif" font-size="11" fill="#394651">{degree}</text>')
    for degree, label in ((64, "T64"), (256, "T256")):
        x = xcoord(degree)
        parts.append(f'<line x1="{x:.1f}" y1="{top}" x2="{x:.1f}" y2="{height-bottom}" stroke="#7b8794" stroke-dasharray="4 4"/>')
        parts.append(f'<text x="{x+3:.1f}" y="{top+20}" font-family="sans-serif" font-size="11" fill="#4c5a67">{label}</text>')
    parts.append(f'<line x1="{left}" y1="{top}" x2="{left}" y2="{height-bottom}" stroke="#34404a" stroke-width="1.5"/>')
    parts.append(f'<line x1="{left}" y1="{height-bottom}" x2="{width-right}" y2="{height-bottom}" stroke="#34404a" stroke-width="1.5"/>')
    for index, item in enumerate(series):
        color = colors[index % len(colors)]
        observed = sorted(item["points"], key=lambda point: int(point["lmax"]))
        coords = [(xcoord(int(point["lmax"])), ycoord(float(point["value"]))) for point in observed]
        dash = ' stroke-dasharray="7 5"' if item.get("dashed") else ""
        if len(coords) > 1:
            path_data = " ".join(f"{x:.1f},{y:.1f}" for x, y in coords)
            parts.append(f'<polyline fill="none" stroke="{color}" stroke-width="2.3"{dash} points="{path_data}"/>')
        for point, (x, y) in zip(observed, coords):
            low = float(point.get("low", point["value"]))
            high = float(point.get("high", point["value"]))
            if low < high:
                low_y, high_y = ycoord(low), ycoord(high)
                parts.append(f'<line x1="{x:.1f}" y1="{high_y:.1f}" x2="{x:.1f}" y2="{low_y:.1f}" stroke="{color}" stroke-width="1.2"/>')
                parts.append(f'<line x1="{x-3:.1f}" y1="{high_y:.1f}" x2="{x+3:.1f}" y2="{high_y:.1f}" stroke="{color}" stroke-width="1.2"/>')
                parts.append(f'<line x1="{x-3:.1f}" y1="{low_y:.1f}" x2="{x+3:.1f}" y2="{low_y:.1f}" stroke="{color}" stroke-width="1.2"/>')
            parts.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="2.8" fill="{color}"/>')
        legend_x = left + index * min(280, (width-left-right)//max(1, len(series)))
        legend_y = 82
        parts.append(f'<line x1="{legend_x}" y1="{legend_y}" x2="{legend_x+23}" y2="{legend_y}" stroke="{color}" stroke-width="3"{dash}/>')
        parts.append(f'<text x="{legend_x+29}" y="{legend_y+5}" font-family="sans-serif" font-size="12" fill="#24333f">{item["name"]}</text>')
    parts += [
        f'<text x="{left+plot_w/2:.1f}" y="{height-12}" text-anchor="middle" font-family="sans-serif" font-size="15">lmax (log(1 + lmax) spacing)</text>',
        f'<text transform="translate(20 {top+plot_h/2:.1f}) rotate(-90)" text-anchor="middle" font-family="sans-serif" font-size="14">{y_label} (log scale)</text>',
        '</svg>',
    ]
    path.write_text("\n".join(parts) + "\n")


def write_dispatch_and_pairing_plots(output_dir: pathlib.Path) -> list[str]:
    """Add the independent Legendre-route and paired-FFT evidence figures."""
    generated = []
    table_csv = ROOT / "reports/data/legendre-policy-2026-10-22/table-recurrence-screen.csv"
    if table_csv.is_file():
        table_series = []
        with table_csv.open(newline="") as handle:
            table_rows = list(csv.DictReader(handle))
        for operation in ("Analysis", "Synthesis"):
            for threads in (1, 8):
                selected = [row for row in table_rows
                            if row["operation"] == operation
                            and int(row["threads"]) == threads
                            and int(row["mmax"]) == int(row["lmax"])
                            and row["longitude"] == "even_smooth"
                            and row["status"] == "complete"
                            and row["table_over_recurrence"]]
                plot_points = []
                for row in selected:
                    table_samples = [float(value) for value in row["table_samples_us"].split(";") if value]
                    recurrence_samples = [float(value) for value in row["recurrence_samples_us"].split(";") if value]
                    ratios = [table / recurrence for table, recurrence
                              in zip(table_samples, recurrence_samples) if recurrence > 0]
                    point = {"lmax": int(row["lmax"]),
                             "value": float(row["table_over_recurrence"])}
                    if ratios:
                        point["low"], point["high"] = min(ratios), max(ratios)
                    plot_points.append(point)
                regions = []
                if operation == "Analysis" and threads == 1:
                    table_degrees = sorted(int(row["lmax"]) for row in selected
                                            if row["production_strategy"] == "CompactTable")
                    if table_degrees:
                        start = previous = table_degrees[0]
                        for degree in table_degrees[1:]:
                            if degree != previous + 1:
                                regions.append((start, previous, "selected q1 analysis table policy"))
                                start = degree
                            previous = degree
                        regions.append((start, previous, "selected q1 analysis table policy"))
                table_series.append({
                    "name": f"{operation.lower()} q{threads}",
                    "points": plot_points,
                    "dashed": threads == 8,
                    "regions": regions,
                })
        plot_log_degree_series(
            output_dir / "legendre-table-vs-recurrence.svg",
            "Compact Legendre table / on-the-fly recurrence",
            "Forced-route screen; whiskers show three-sample min–max; shaded band marks selected q1 analysis table degrees (separate source sweep)",
            table_series, "table / recurrence runtime",
        )
        generated.append("legendre-table-vs-recurrence.svg")

    paired_csv = ROOT / "reports/data/paired-fft-validation-2026-10-22/matched-boundary-ab.csv"
    if paired_csv.is_file():
        paired_series = []
        with paired_csv.open(newline="") as handle:
            paired_rows = list(csv.DictReader(handle))
        for operation in ("analysis", "synthesis"):
            for threads in (1, 8):
                selected = [row for row in paired_rows
                            if row["operation"] == operation
                            and int(row["threads"]) == threads
                            and int(row["mmax"]) == int(row["lmax"])]
                paired_series.append({
                    "name": f"{operation} q{threads}",
                    "points": [{"lmax": int(row["lmax"]),
                                "value": float(row["enabled_over_disabled"])}
                               for row in selected],
                    "dashed": threads == 8,
                })
        plot_log_degree_series(
            output_dir / "paired-fft-benefit.svg",
            "Paired odd-row FFT / independent-row FFT",
            "Same-source full-transform boundary A/B, odd-minimum longitude; below 1 favors paired rows",
            paired_series, "paired / independent latency",
        )
        generated.append("paired-fft-benefit.svg")
    return generated


def write_cc_gl_comparison_plot(output_dir: pathlib.Path,
                                gl_results: pathlib.Path) -> pathlib.Path | None:
    """Compare matched Rust/DUCC ratios while retaining CC's extra pole row."""
    cc_path = output_dir / "results.csv"
    if not cc_path.is_file() or not gl_results.is_file():
        return None
    with cc_path.open(newline="") as handle:
        cc_rows = list(csv.DictReader(handle))
    with gl_results.open(newline="") as handle:
        gl_rows = list(csv.DictReader(handle))

    def key(row: dict) -> tuple:
        return (int(row["lmax"]), int(row["mmax"]), row["longitude_mode"],
                row["operation"], int(row["threads"]))

    gl_by_key = {key(row): row for row in gl_rows}
    series = []
    for operation in ("analysis", "synthesis"):
        for order_label, full in (("full mmax", True), ("quarter mmax", False)):
            for threads in (1, 8):
                points = []
                for cc in cc_rows:
                    if (cc["longitude_mode"] != "even_smooth"
                            or cc["operation"] != operation
                            or int(cc["threads"]) != threads
                            or ((int(cc["mmax"]) == int(cc["lmax"])) != full)):
                        continue
                    gl = gl_by_key.get(key(cc))
                    if gl is None:
                        continue
                    points.append({
                        "lmax": int(cc["lmax"]),
                        "value": float(cc["rust_ducc_ratio"]) / float(gl["rust_ducc_ratio"]),
                    })
                series.append({
                    "name": f"{operation} {order_label} q{threads}",
                    "points": points,
                    "dashed": threads == 8,
                })
    output = output_dir / "cc-versus-gl-matched-ratio.svg"
    plot_log_degree_series(
        output,
        "CC versus GL matched RustSHT/DUCC ratio",
        "Ratio of Rust/DUCC full-transform ratios; matching lmax, mmax and nphi; CC uses ntheta=lmax+2, GL uses lmax+1",
        series,
        "CC ratio / GL ratio",
    )
    return output


def dense_sweep_coverage(cases: list[dict], raw: list[dict], errors: list[dict],
                         failures: list[dict], grid: str = "GL") -> dict:
    expected = make_cases(False, dense_sweep=True, grid=grid)
    expected_by_id = {case["case_id"]: case for case in expected}
    actual_by_id = {case["case_id"]: case for case in cases}
    expected_ids = set(expected_by_id)
    actual_ids = set(actual_by_id)
    measurement_groups = {}
    for case in expected:
        for operation in ("analysis", "synthesis"):
            for threads in (1, 8):
                key = (case["case_id"], operation, threads)
                backend_pairs = {
                    backend: {
                        entry["process_pair"] for entry in raw
                        if entry["case_id"] == key[0]
                        and entry["operation"] == operation
                        and entry["threads"] == threads
                        and entry["backend"] == backend
                    }
                    for backend in ("rustsht", "ducc")
                }
                comparison_pairs = {
                    entry["process_pair"] for entry in errors
                    if entry["case_id"] == key[0]
                    and entry["operation"] == operation
                    and entry["threads"] == threads
                    and entry.get("relative_l2") is not None
                }
                failed_pairs = {
                    entry["process_pair"] for entry in failures
                    if entry.get("case_id") == key[0]
                    and entry.get("operation") == operation
                    and entry.get("threads") == threads
                }
                complete = backend_pairs["rustsht"] & backend_pairs["ducc"] & comparison_pairs
                measurement_groups[f"{key[0]}|{operation}|q{threads}"] = {
                    "rust_process_pairs": sorted(backend_pairs["rustsht"]),
                    "ducc_process_pairs": sorted(backend_pairs["ducc"]),
                    "compared_process_pairs": sorted(comparison_pairs),
                    "complete_process_pairs": sorted(complete),
                    "failed_process_pairs": sorted(failed_pairs),
                    "status": (
                        "complete" if len(complete) == PROCESS_PAIRS
                        else "partial" if complete
                        else "failed" if failed_pairs
                        else "missing"
                    ),
                }
    low_degrees = sorted({case["lmax"] for case in cases if case["lmax"] <= 256})
    required_high = sorted(value for value in DENSE_SWEEP_HIGH_LMAX if value > 256)
    low_even_missing = []
    for lmax in range(257):
        mmax_values = {lmax}
        if lmax > 0:
            mmax_values.add(max(1, lmax // 4))
        for mmax in mmax_values:
            grid_tag = "" if grid == "GL" else f"{grid}-"
            case_id = f"{grid_tag}T{lmax}-m{mmax}-even_smooth"
            if case_id not in actual_ids:
                low_even_missing.append(case_id)
    return {
        "profile": f"dense-{grid}-0-256-plus-selected-high-degrees",
        "grid": grid,
        "expected_lmax_0_through_256": list(range(257)),
        "observed_lmax_0_through_256": low_degrees,
        "required_high_lmax": required_high,
        "observed_high_lmax": sorted({case["lmax"] for case in cases if case["lmax"] > 256}),
        "expected_case_count": len(expected),
        "observed_case_count": len(cases),
        "missing_case_ids": sorted(expected_ids - actual_ids),
        "unexpected_case_ids": sorted(actual_ids - expected_ids),
        "low_degree_even_smooth_missing": low_even_missing,
        "expected_measurement_group_count": len(expected) * 4,
        "measurement_groups": measurement_groups,
        "measurement_status_counts": {
            status: sum(group["status"] == status for group in measurement_groups.values())
            for status in ("complete", "partial", "failed", "missing")
        },
        "failure_count": len(failures),
        "all_requested_measurements_attempted": all(
            group["status"] != "missing" for group in measurement_groups.values()
        ),
        "all_measurements_complete": all(
            group["status"] == "complete" for group in measurement_groups.values()
        ),
        "coverage_pass": (
            actual_ids == expected_ids and not low_even_missing
            and not (expected_ids - actual_ids)
        ),
    }


def write_artifacts(output_dir: pathlib.Path, metadata: dict, cases: list[dict], raw: list[dict], analysis: dict) -> None:
    rows = aggregate(raw, analysis["errors"])
    output_dir.mkdir(parents=True, exist_ok=True)
    metadata["runtime_rust_arches"] = sorted({entry["arch"] for entry in raw if entry["backend"] == "rustsht"})
    metadata["runtime_ducc_arches"] = sorted({entry["arch"] for entry in raw if entry["backend"] == "ducc"})
    metadata["runtime_rust_f64_lane_widths"] = sorted({entry["lane_width_f64"] for entry in raw if entry["backend"] == "rustsht"})
    sanitized_cases = [
        {key: value for key, value in case.items() if key not in {"map_path", "alm_path"}}
        for case in cases
    ]
    public_samples = raw if metadata.get("retain_local_host_telemetry") else [
        {key: value for key, value in sample.items() if key != "host_diagnostics"}
        for sample in raw
    ]
    machine = {
        "metadata": portable_evidence(metadata),
        "protocol": {
            "source_revision": metadata["ducc_source_revision"],
            "target_batch_ms": TARGET_BATCH_MS,
            "batches_per_process": BATCHES_PER_PROCESS,
            "independent_process_pairs": PROCESS_PAIRS,
            "interleaving": "Rust then DUCC for even process-pair indexes; DUCC then Rust for odd indexes",
            "q_values": sorted({entry["threads"] for entry in raw}),
            "grid": metadata.get("grid", "GL"),
            "thread_control": "Rust process-global Rayon pool configured before plan creation; DUCC DUCC0_NUM_THREADS; both workers report active pool capacity",
            "operations": sorted({entry["operation"] for entry in raw}),
            "input_layout": "identical compact m-major coefficient body; DUCC mstart adds a leading offset and maps without timed conversion",
            "fixture_generation": "deterministic PCG64 seed derived from grid, lmax, mmax, and nphi; SHA-256 of each alm/map fixture is recorded per case",
            "output_buffers": "reused across calls; transform outputs read and compared outside timed intervals",
            "timed_region": "direct native ShtPlan transform calls with a process-global Rayon pool reused; pool/workspace setup and first call reported separately",
            "rust_call_boundary": "no per-call ThreadPool::install wrapper; RAYON_NUM_THREADS is set before process startup",
            "short_operation_calibration": "up to three untimed repeated-call batches; adjust repetitions until 20-100 ms, then collect 10 measured batches",
            "long_transform_rule": "operations whose warmup exceeds 100 ms are timed individually (five per process invocation)",
            "output_reset_check": "after timing, exact comparison of first-call output and last repeated-call output",
        },
        "cases": sanitized_cases,
        "rows": rows,
        "samples": portable_evidence(public_samples),
        "numerics": portable_evidence(analysis["errors"]),
        "validation": portable_evidence(analysis["validations"]),
        "failures": portable_evidence(analysis.get("failures", [])),
        "host_summary": analysis["host"],
    }
    (output_dir / "results.json").write_text(
        json.dumps(machine, separators=(",", ":"), allow_nan=False) + "\n"
    )
    result_fields = list(rows[0]) if rows else [
        "case_id", "grid", "lmax", "mmax", "ntheta", "nphi", "longitude_mode",
        "operation", "threads", "rust_warm_ms", "rust_p10_ms", "rust_p90_ms",
        "ducc_warm_ms", "ducc_p10_ms", "ducc_p90_ms", "rust_ducc_ratio",
        "paired_ratio_p10", "paired_ratio_p90", "excess_ms", "absolute_max",
        "relative_l2", "max_relative_above_1e-12_peak", "batch_count",
        "completed_process_pairs", "measurement_status",
    ]
    with (output_dir / "results.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=result_fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    failures = analysis.get("failures", [])
    failure_fields = [
        "case_id", "lmax", "mmax", "ntheta", "nphi", "longitude_mode",
        "operation", "threads", "process_pair", "backend", "failure_class", "reason",
    ]
    failure_fields = list(dict.fromkeys(failure_fields + [key for failure in failures for key in failure]))
    with (output_dir / "failures.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=failure_fields, lineterminator="\n")
        writer.writeheader()
        for failure in failures:
            writer.writerow({
                key: json.dumps(value, sort_keys=True) if isinstance(value, (dict, list)) else value
                for key, value in failure.items()
            })
    if metadata.get("dense_sweep"):
        coverage = dense_sweep_coverage(
            cases, raw, analysis["errors"], failures, metadata.get("grid", "GL")
        )
        (output_dir / "coverage-audit.json").write_text(
            json.dumps(coverage, indent=2, allow_nan=False) + "\n"
        )
    grid = metadata.get("grid", "GL")
    write_issue_ratio_plots(rows, output_dir, grid)
    write_issue_runtime_plots(rows, output_dir, grid)
    if metadata.get("dense_sweep") and grid == "GL":
        write_dispatch_and_pairing_plots(output_dir)
    for operation in ("analysis", "synthesis"):
        speed_series = []
        for backend, key in (("RustSHT", "rust_warm_ms"), ("DUCC", "ducc_warm_ms")):
            q1 = {(row["case_id"]): row[key] for row in rows if row["operation"] == operation and row["threads"] == 1}
            pts = [{"lmax": row["lmax"], "speedup": q1[row["case_id"]] / row[key]}
                   for row in rows if row["operation"] == operation and row["threads"] == 8 and row["case_id"] in q1]
            speed_series.append({"name": backend, "points": pts})
        plot_svg(output_dir / f"thread-speedup-{operation}.svg", f"One-thread/eight-thread speedup: {operation}",
                 speed_series, "lmax", "speedup", "q1 time / q8 time")
        reduced_series = []
        for backend, key in (("RustSHT", "rust_warm_ms"), ("DUCC", "ducc_warm_ms")):
            for threads in (1, 8):
                matched = []
                for reduced in rows:
                    if (reduced["operation"] != operation or reduced["threads"] != threads
                            or reduced["mmax"] != max(1, reduced["lmax"] // 4)):
                        continue
                    full = next((candidate for candidate in rows
                                 if candidate["operation"] == operation
                                 and candidate["threads"] == threads
                                 and candidate["lmax"] == reduced["lmax"]
                                 and candidate["longitude_mode"] == reduced["longitude_mode"]
                                 and candidate["mmax"] == candidate["lmax"]), None)
                    if full is not None:
                        matched.append({
                            "lmax": reduced["lmax"],
                            "full_reduced": full[key] / reduced[key],
                        })
                reduced_series.append({"name": f"{backend} q{threads}", "points": matched})
        plot_svg(output_dir / f"full-reduced-mmax-{operation}.svg",
                 f"Full/reduced mmax runtime: {operation}",
                 reduced_series, "lmax", "full_reduced", "full mmax / reduced mmax")
    analysis_rows = [row for row in rows if row["operation"] == "analysis" and row["threads"] == 1 and row["mmax"] == row["lmax"]]
    cold = [{"lmax": row["lmax"], "ms": row["rust_cold_first_ms"]} for row in analysis_rows]
    warm = [{"lmax": row["lmax"], "ms": row["rust_warm_ms"]} for row in analysis_rows]
    plot_svg(output_dir / "cold-warm-analysis-rust-q1.svg", "RustSHT first-call vs warmed analysis q1",
             [{"name": "cold first call", "points": cold}, {"name": "warm batches", "points": warm}],
             "lmax", "ms", "milliseconds")
    return rows


def write_report(output_dir: pathlib.Path, metadata: dict, rows: list[dict], cases: list[dict],
                 host: dict, validations: list[dict], report: pathlib.Path = REPORT_PATH) -> None:
    report.parent.mkdir(parents=True, exist_ok=True)
    grid = metadata.get("grid", "GL")
    grid_name = grid_display_name(grid)
    ntheta_rule = "lmax+2 (including both poles)" if grid == "CC" else "lmax+1"
    results_prefix = pathlib.Path(os.path.relpath(output_dir, report.parent)).as_posix()
    results_from_root = pathlib.Path(os.path.relpath(output_dir, ROOT)).as_posix()
    report_from_root = pathlib.Path(os.path.relpath(report, ROOT)).as_posix()
    target_lmax = {0, 4, 8, 16, 32, 64, 128, 255, 256, 511, 512, 513, 720, 721, 1024, 1536, 2048, 3072}
    chosen = [row for row in rows if row["lmax"] in target_lmax and (row["mmax"] == row["lmax"] or row["lmax"] in {720, 2048})]
    lines = [
        "# Full-transform RustSHT/DUCC parity benchmark (2026-10-20)",
        "",
        f"Pinned DUCC revision: `{metadata['ducc_source_revision']}` (DUCC {metadata['ducc_version']}).",
        f"Hardware: {metadata['cpu_model']}; {metadata['physical_cores_visible']} physical cores and {metadata['logical_cpus_visible']} logical CPUs visible; {metadata['installed_ram_gib']} GiB RAM; x86-64 features `{', '.join(metadata['cpu_features'])}`.",
        f"Builds: Rust `{metadata['rustc']}` with `{metadata['rust_flags']}`, Pulp {metadata['pulp']}; DUCC `{metadata.get('cxx', metadata.get('gxx', 'unknown'))}` with `{metadata['ducc_flags']}`.",
        f"Runtime SIMD: Rust worker reports `{rows[0]['rust_arch'] if rows else 'not recorded'}`; DUCC is compiled for x86-64-v3 (AVX2/FMA, four f64 lanes).",
        "",
        "## Scope and method",
        "",
        f"Measured {len(cases)} {grid_name} scalar configurations using RustSHT's native `ShtPlan` and pinned DUCC's native `analysis_2d`/`synthesis_2d` C++ API. Each transform used all coefficients in its permitted `l=0..lmax, m=0..min(l,mmax)` triangle. Maps and coefficients were generated once per configuration and reused by both workers. `phi0=0.23`; `ntheta={ntheta_rule}`; longitude counts cover odd minimum sampling and even FFT-smooth sampling.",
        "",
        f"Analysis and synthesis were each timed at q1 and q8. Single-thread processes were pinned to one allowed physical core; q8 used eight physical cores. Rust reused its plan and caller-owned workspace. DUCC's public 2D functions have no persistent plan/workspace object, so the adapter reuses all input/output buffers and reports its internal per-call preparation as part of transform latency. DUCC `mstart` mapped its indexing into a leading-offset view of the same compact coefficient body; no per-call coefficient conversion was timed.",
        "",
        f"Each backend/case/operation/thread-count collected {PROCESS_PAIRS} interleaved independent process pairs with up to three out-of-sample calibration batches and 10 measured repeated-operation batches per process when each operation was below 100 ms. Target was {TARGET_BATCH_MS:g} ms and each measured short-operation batch was checked against the requested 20–100 ms range. Operations above 100 ms were measured individually five times per process. First-call and setup costs are separate from warmed samples. The runner records median, p10/p90, range, paired per-batch ratios and output error. See `{results_prefix}/results.json` and `.csv` for all cases and samples.",
        "",
        "## Results",
        "",
        "The table reports warmed native calls; ratios below one indicate lower RustSHT latency. Relative L2 uses the DUCC output norm, with near-zero values excluded only from the separately reported max-relative metric. No fixed parity threshold was applied.",
        "",
        "| lmax | mmax | grid | operation | threads | Rust ms [p10,p90] | DUCC ms [p10,p90] | Rust/DUCC | paired ratio [p10,p90] | excess ms | relative L2 |",
        "|---:|---:|---|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row in chosen:
        lines.append(
            f"| {row['lmax']} | {row['mmax']} | {row.get('grid', grid)} {row['ntheta']}×{row['nphi']} {row['longitude_mode']} | {row['operation']} | {row['threads']} | {row['rust_warm_ms']:.6g} [{row['rust_p10_ms']:.6g},{row['rust_p90_ms']:.6g}] | {row['ducc_warm_ms']:.6g} [{row['ducc_p10_ms']:.6g},{row['ducc_p90_ms']:.6g}] | {row['rust_ducc_ratio']:.4f} | {row['paired_ratio_p10']:.4f}–{row['paired_ratio_p90']:.4f} | {row['excess_ms']:.6g} | {row['relative_l2']:.3e} |"
        )
    def selected(lmax: int, mmax: int, longitude_mode: str, operation: str,
                 threads: int) -> dict | None:
        return next((row for row in rows if row["lmax"] == lmax and row["mmax"] == mmax
                     and row["longitude_mode"] == longitude_mode
                     and row["operation"] == operation and row["threads"] == threads), None)

    t1536_a = selected(1536, 1536, "even_smooth", "analysis", 1)
    t1536_s = selected(1536, 1536, "even_smooth", "synthesis", 1)
    t2048_a1 = selected(2048, 2048, "even_smooth", "analysis", 1)
    t2048_a8 = selected(2048, 2048, "even_smooth", "analysis", 8)
    t2048_s1 = selected(2048, 2048, "even_smooth", "synthesis", 1)
    t2048_s8 = selected(2048, 2048, "even_smooth", "synthesis", 8)
    t3072_a1 = selected(3072, 3072, "even_smooth", "analysis", 1)
    t3072_a8 = selected(3072, 3072, "even_smooth", "analysis", 8)
    t3072_s1 = selected(3072, 3072, "even_smooth", "synthesis", 1)
    t3072_s8 = selected(3072, 3072, "even_smooth", "synthesis", 8)
    faster = sum(row["paired_ratio_p90"] < 1.0 for row in rows)
    slower = sum(row["paired_ratio_p10"] > 1.0 for row in rows)
    unresolved = len(rows) - faster - slower
    window_fractions = [row["20_100ms_batch_fraction_for_short_ops"] for row in rows
                        if row["20_100ms_batch_fraction_for_short_ops"] is not None]
    lines += [
        "",
        f"Across {len(rows)} warm rows, RustSHT was faster in {faster} where the paired p10–p90 ratio interval stayed below 1, slower in {slower} where it stayed above 1, and inconclusive in {unresolved} where the interval crossed 1. This classification follows observed variation rather than a fixed parity band.",
        "### Cross-bandlimit findings",
        "",
    ]
    if grid == "GL":
        lines += [
            f"For even-smooth full-mmax GL, q1 RustSHT was near DUCC at T1536 (analysis {t1536_a['rust_ducc_ratio']:.2f}×; synthesis {t1536_s['rust_ducc_ratio']:.2f}×). At T2048, analysis rose to {t2048_a1['rust_ducc_ratio']:.2f}× q1 and {t2048_a8['rust_ducc_ratio']:.2f}× q8, while synthesis was {t2048_s1['rust_ducc_ratio']:.2f}× q1 and {t2048_s8['rust_ducc_ratio']:.2f}× q8. At T3072, analysis was {t3072_a1['rust_ducc_ratio']:.2f}× q1 ({t3072_a1['excess_ms']:.1f} ms excess) and {t3072_a8['rust_ducc_ratio']:.2f}× q8 ({t3072_a8['excess_ms']:.1f} ms excess); synthesis was {t3072_s1['rust_ducc_ratio']:.2f}× q1 and {t3072_s8['rust_ducc_ratio']:.2f}× q8.",
            "The large high-bandlimit excess is specific to the complete analysis call in these cases: synthesis remains near DUCC at q1, while q8 synthesis is about 1.5× slower. T1536 q1 is near parity, so the measured analysis gap grows sharply in the next sampled size range. End-to-end timings include FFT and preparation; they do not isolate recurrence as the cause. The next comparison should separate analysis recurrence/accumulation from FFT and output work at the high-bandlimit cases.",
        ]
    else:
        high_full = [row for row in rows if row["lmax"] == 3072 and row["mmax"] == row["lmax"]
                     and row["longitude_mode"] == "even_smooth"]
        high_summary = "; ".join(
            f"{row['operation']} q{row['threads']} {row['rust_ducc_ratio']:.3f}× ({row['excess_ms']:.3g} ms excess)"
            for row in sorted(high_full, key=lambda item: (item["operation"], item["threads"]))
        )
        lines.append(
            f"At T3072 full mmax, the measured CC Rust/DUCC ratios are: {high_summary or 'no complete comparison row'}. These end-to-end CC timings include its meridional resampling and do not isolate recurrence; stage attribution is recorded separately when available."
        )
    lines += [
        f"For operations below 100 ms, the median per-row fraction of measured batches in the requested 20–100 ms window was {statistics.median(window_fractions):.1%}; the lowest row fraction was {min(window_fractions):.1%}. {sum(row['batch_count'] == PROCESS_PAIRS * 10 for row in rows)} short-operation rows collected 30 batches per backend. One row median was just below the 20-ms lower bound; its timing distribution is retained without adjustment. Calibration attempts are recorded separately in each process sample.",
        f"Maximum measured random-output relative L2 error was {max(row['relative_l2'] for row in rows):.3e}; maximum absolute error was {max(row['absolute_max'] for row in rows):.3e}.",
        f"The out-of-timing checks covered {len(validations)} configurations. Exact-zero/reset consistency held for {sum(all(item['reset_consistent'].values()) for item in validations)}/{len(validations)}; all edge-case outputs were finite for {sum(item['finite_outputs'] for item in validations)}/{len(validations)}. Maximum Rust/DUCC roundtrip relative L2 was {max(max(item['roundtrip'][backend]['relative_l2'] for backend in ('rustsht', 'ducc')) for item in validations):.3e}; maximum selected-basis recovery relative L2 was {max(max(entry[key]['relative_l2'] for entry in item['basis'] for key in ('analysis_of_synthesis_rust', 'analysis_of_synthesis_ducc')) for item in validations):.3e}; maximum adjoint identity relative residual was {max(item['adjoints'][backend][key] for item in validations for backend in ('rustsht', 'ducc') for key in ('synthesis_relative_residual', 'analysis_relative_residual')):.3e}.",
        f"Available host-state summary: frequency median/p10/p90 {fmt(host['frequency_ghz_median'])}/{fmt(host['frequency_ghz_p10'])}/{fmt(host['frequency_ghz_p90'])} GHz; scheduler wait median/p90 {fmt(host['runqueue_wait_fraction_median'])}/{fmt(host['runqueue_wait_fraction_p90'])}; involuntary-switch median/max {fmt(host['involuntary_switches_median'])}/{host['involuntary_switches_max']}. This indicates intermittent scheduling/frequency interference despite low median scheduler wait. No samples were dropped; paired distributions retain the resulting uncertainty.",
        "",
        "### Setup and cold-start costs",
        "",
        "The machine-readable rows retain per-process fixture loading, output-buffer preparation, Rust plan/workspace setup, first-call latency, warmed batches, actual batch wall times and the fraction of short-operation batches within 20–100 ms. Rust creates its plan and workspace before timing; first-call latency reflects the native cold numerical route and runtime initialization. DUCC has no reusable 2D plan at this API boundary, so its internal allocation and transform preparation remain in each call. Each worker verifies that repeated calls overwrite outputs consistently with the first call.",
        "",
        "### Plots",
        "",
        f"- [Analysis ratio by lmax]({results_prefix}/ratio-analysis-full-mmax.svg)",
        f"- [Synthesis ratio by lmax]({results_prefix}/ratio-synthesis-full-mmax.svg)",
        f"- [Analysis absolute runtime]({results_prefix}/runtime-analysis.svg)",
        f"- [Synthesis absolute runtime]({results_prefix}/runtime-synthesis.svg)",
        f"- [Analysis q1/q8 speedup]({results_prefix}/thread-speedup-analysis.svg)",
        f"- [Synthesis q1/q8 speedup]({results_prefix}/thread-speedup-synthesis.svg)",
        f"- [Full/reduced mmax analysis]({results_prefix}/full-reduced-mmax-analysis.svg)",
        f"- [Full/reduced mmax synthesis]({results_prefix}/full-reduced-mmax-synthesis.svg)",
        f"- [Rust analysis first/warmed latency]({results_prefix}/cold-warm-analysis-rust-q1.svg)",
        "",
        "## Measured scope and limitations",
        "",
        f"The largest completed degree is T{max(case['lmax'] for case in cases)}. This report covers f64 scalar {grid_name}; the run times complete native calls including FFT, Legendre work, packing/conversion and output writes.",
        "",
        f"The Rust/DUCC output comparison validates identical deterministic random maps and complete-triangle coefficient arrays in every timed configuration at q1 and q8. For each configuration and thread count, the first q1/q8 analysis process also runs out-of-timing roundtrip, zonal/intermediate/sectoral basis, zero, subnormal, extreme-finite and analysis/synthesis adjoint checks. Their per-case results are in `{results_prefix}/results.json`; neither set of checks is inferred from roundtrip alone.",
        "",
        "## Reproduction",
        "",
        "```sh",
        "RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo build --release --example ducc_full_transform_parity --features parallel",
        "cmake -S reports/data/ducc-full-transform-parity-2026-10-20 -B \"${DUCC_BUILD_DIR:-ducc-build}\" -DDUCC_SRC=\"${DUCC_SRC:?set DUCC_SRC to the pinned checkout}\" -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=g++",
        "cmake --build \"${DUCC_BUILD_DIR:-ducc-build}\" --parallel 4",
        f"python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src \"${{DUCC_SRC:?set DUCC_SRC to the pinned checkout}}\" --ducc-revision {metadata['ducc_source_revision']} --grid {grid} --results-dir {results_from_root} --report-path {report_from_root} --skip-build",
        "```",
        "",
    ]
    report.write_text("\n".join(lines))


def write_dense_sweep_report(output_dir: pathlib.Path, report: pathlib.Path,
                             metadata: dict, rows: list[dict], cases: list[dict],
                             coverage: dict, validations: list[dict],
                             failures: list[dict], host: dict,
                             invocations: list[str], samples: list[dict]) -> None:
    report.parent.mkdir(parents=True, exist_ok=True)
    result_prefix = pathlib.Path(os.path.relpath(output_dir, report.parent)).as_posix()
    grid = metadata.get("grid", "GL")
    grid_name = grid_display_name(grid)
    ntheta_rule = "lmax+2 (including both poles)" if grid == "CC" else "lmax+1"
    degree_bands = ((1, 32), (33, 63), (64, 128), (129, 256), (257, 720),
                    (721, 1536), (1537, 3072))
    timing_count_histogram = Counter(int(row["batch_count"]) for row in rows)
    lines = [
        f"# Current-source native RustSHT/DUCC scalar {grid} sweep (2026-10-22)",
        "",
        f"Production Rust source: `{metadata.get('production_source_revision', metadata['rust_source_revision'])}`; benchmark harness commit: `{metadata['rust_source_revision']}`.",
        f"Pinned DUCC 0.42.0 source: `{metadata['ducc_source_revision']}`.",
        f"Host: {metadata['cpu_model']}; selected q1 CPU `{metadata['selected_q1_cpu']}`, q8 CPUs `{','.join(map(str, metadata['selected_q8_cpus']))}`; q8 L3 cache IDs `{metadata['selected_q8_l3_cache_ids']}` (one CCD on this host).",
        f"Toolchains: Rust `{metadata['rustc']}` / {metadata['pulp']} with `{metadata['rust_flags']}`; DUCC `{metadata['cxx']}` with `{metadata['ducc_flags']}`.",
        f"The production Rust source matches the frozen baseline: `{metadata.get('production_code_matches_baseline')}`; changed production paths since it: `{metadata.get('production_code_changes_since_baseline')}`. Executed Rust Pulp architectures: `{metadata.get('runtime_rust_arches')}`, f64 lane widths `{metadata.get('runtime_rust_f64_lane_widths')}`; DUCC runtime label: `{metadata.get('runtime_ducc_arches')}`.",
        "The exact `rustc -Vv`, DUCC SHT compile command, and strict benchmark-driver compile command are preserved in the result JSON metadata.",
        "",
        "## Scope and method",
        "",
        f"The target dense manifest contains {coverage['expected_case_count']} distinct {grid_name} geometries covering every `lmax=0…256` plus {len([value for value in DENSE_SWEEP_HIGH_LMAX if value > 256])} selected higher degrees through T3072. This dataset contains {len(cases)} geometries. The target includes full and quarter-order `mmax` with even FFT-smooth longitude where those truncations differ; identical low-degree cases are deduplicated. Selected small, dispatch-boundary and high-resolution cases also include odd-minimum longitude. Each transform uses `{ntheta_rule}`, `phi0=0.23`, the full permitted degree/order triangle and deterministic PCG64 fixtures shared by both native libraries.",
        f"Both analysis and synthesis were run at q1/q8. Short calls use {PROCESS_PAIRS} interleaved process pairs and {BATCHES_PER_PROCESS} calibrated batches per backend process, targeting {TARGET_BATCH_MS:g} ms; long calls use the existing individually timed rule. Rust plan/workspace setup, first-call and warmed timings are separate. The JSON retains per-process batch distributions, numerical comparisons and validation; aggregate CPU-frequency and scheduler diagnostics are reported below. Raw per-process telemetry stays in local shard captures. Failure records are retained in `failures.csv` and JSON.",
        f"Timing-count audit: {timing_count_histogram.get(3 * BATCHES_PER_PROCESS, 0)} rows contain 30 calibrated batches (10 in each process pair), {timing_count_histogram.get(15, 0)} contain 15 individually timed long-call observations (five in each pair), and {timing_count_histogram.get(25, 0)} mix two 10-batch pairs with one five-call long pair. The raw JSON records each process's actual timing route; no row is padded to a nominal sample count.",
        f"The q1/q8 CPU affinity was pinned to the physical core sets shown above. q8 uses the first eight physical cores visible to the process (one CCD on this host). RustSHT and DUCC receive the same fixtures and reusable caller buffers; DUCC's public API performs its own internal setup on each call.",
        "",
        "## Coverage audit",
        "",
        f"The dataset contains {len(coverage['observed_lmax_0_through_256'])}/257 integer degrees in the low range ({257 - len(coverage['observed_lmax_0_through_256'])} absent). The required high-resolution list has {len(coverage['required_high_lmax'])} values; complete case-selection check: `{coverage['coverage_pass']}`. Low-degree even-smooth full/reduced coverage omissions: {len(coverage['low_degree_even_smooth_missing'])}. Across {coverage['expected_measurement_group_count']} operation/thread groups, statuses are `{coverage['measurement_status_counts']}`. Failed or partial groups are listed explicitly rather than silently removed.",
        "",
        "## Warmed whole-transform summary",
        "",
        "Ratios below 1 mean RustSHT was faster. Each table cell reports median ratio, geometric mean ratio, and completed paired case count. Odd-minimum cases are excluded from this even-smooth summary.",
        "",
        "| lmax band | operation | mmax | threads | median ratio | geomean ratio | cases |",
        "|---|---|---|---:|---:|---:|---:|",
    ]
    for lo, hi in degree_bands:
        for operation in ("analysis", "synthesis"):
            for order_label, is_full in (("full", True), ("reduced", False)):
                for threads in (1, 8):
                    selected = [
                        row for row in rows
                        if lo <= row["lmax"] <= hi
                        and row["operation"] == operation
                        and row["threads"] == threads
                        and ((row["mmax"] == row["lmax"]) == is_full)
                        and row["longitude_mode"] == "even_smooth"
                    ]
                    if not selected:
                        continue
                    ratios = [float(row["rust_ducc_ratio"]) for row in selected]
                    geomean = math.exp(sum(math.log(value) for value in ratios) / len(ratios))
                    lines.append(
                        f"| {lo}–{hi} | {operation} | {order_label} | q{threads} | "
                        f"{statistics.median(ratios):.3f} | {geomean:.3f} | {len(selected)} |"
                    )
    lines += ["", "### Fast and slow measured cases", ""]
    if rows:
        throughput_rows = [row for row in rows if row["lmax"] > 0]
        worst = sorted(throughput_rows, key=lambda row: row["rust_ducc_ratio"], reverse=True)[:8]
        best = sorted(throughput_rows, key=lambda row: row["rust_ducc_ratio"])[:8]
        lines += [
            "| case | operation | q | Rust ms | DUCC ms | ratio | relative L2 |",
            "|---|---|---:|---:|---:|---:|---:|",
        ]
        for label, group in (("Largest ratios", worst), ("Smallest ratios", best)):
            lines.append(f"| **{label}** |  |  |  |  |  |  |")
            for row in group:
                lines.append(
                    f"| T{row['lmax']} m{row['mmax']} {row['longitude_mode']} | {row['operation']} | q{row['threads']} | "
                    f"{row['rust_warm_ms']:.6g} | {row['ducc_warm_ms']:.6g} | {row['rust_ducc_ratio']:.3f} | {row['relative_l2']:.3e} |"
                )
    lines += ["", "### Longitude-shape sensitivity", ""]
    shape_summary = []
    for operation in ("analysis", "synthesis"):
        for threads in (1, 8):
            by_key = {
                (row["lmax"], row["mmax"], row["operation"], row["threads"], row["longitude_mode"]): row
                for row in rows
            }
            pairs = []
            for row in rows:
                if row["operation"] != operation or row["threads"] != threads or row["longitude_mode"] != "odd_minimum":
                    continue
                even = by_key.get((row["lmax"], row["mmax"], operation, threads, "even_smooth"))
                if even is not None:
                    pairs.append((row, even))
            if pairs:
                rust_factors = [odd["rust_warm_ms"] / even["rust_warm_ms"] for odd, even in pairs]
                ducc_factors = [odd["ducc_warm_ms"] / even["ducc_warm_ms"] for odd, even in pairs]
                shape_summary.append(
                    f"- {operation} q{threads}: odd/even median runtime factor Rust {statistics.median(rust_factors):.3f}×, DUCC {statistics.median(ducc_factors):.3f}× across {len(pairs)} matched cases."
                )
    lines += shape_summary or ["No odd/even matched geometries completed."]
    if rows:
        lines += [
            "",
            "### Crossover and numerical results",
            "",
        ]
        for operation in ("analysis", "synthesis"):
            for is_full, order_label in ((True, "full"), (False, "reduced")):
                for threads in (1, 8):
                    selected = sorted(
                        [row for row in rows if row["lmax"] > 0
                         and row["operation"] == operation
                         and row["threads"] == threads
                         and ((row["mmax"] == row["lmax"]) == is_full)
                         and row["longitude_mode"] == "even_smooth"],
                        key=lambda row: row["lmax"],
                    )
                    if not selected:
                        lines.append(f"- {operation} {order_label} q{threads}: no completed even-smooth comparisons.")
                        continue
                    below = [row["lmax"] for row in selected if row["rust_ducc_ratio"] < 1]
                    above = [row["lmax"] for row in selected if row["rust_ducc_ratio"] > 1]
                    crossings = [
                        f"T{left['lmax']}→T{right['lmax']}"
                        for left, right in zip(selected, selected[1:])
                        if (left["rust_ducc_ratio"] < 1) != (right["rust_ducc_ratio"] < 1)
                    ]
                    lines.append(
                        f"- {operation} {order_label} q{threads}: {len(below)} observed points below parity and {len(above)} above; "
                        f"lowest-ratio degree {min(selected, key=lambda row: row['rust_ducc_ratio'])['lmax']}, "
                        f"highest-ratio degree {max(selected, key=lambda row: row['rust_ducc_ratio'])['lmax']}; "
                        f"observed parity crossings {', '.join(crossings) if crossings else 'none in degree order'} (sampled points only)."
                    )
        max_l2 = max(row["relative_l2"] for row in rows)
        max_abs = max(row["absolute_max"] for row in rows)
        lines.append(f"Maximum random-output relative L2 error: `{max_l2:.3e}`; maximum absolute error: `{max_abs:.3e}`.")
        lines.append("T0 remains in the raw results, coverage audit, correctness checks and plots; it is excluded from the degree-band and fastest/slowest throughput summaries.")
    finite_validations = sum(item["finite_outputs"] for item in validations)
    reset_validations = sum(all(item["reset_consistent"].values()) for item in validations)
    roundtrip_max = max((item["roundtrip"][backend]["relative_l2"]
                         for item in validations for backend in ("rustsht", "ducc")), default=0.0)
    basis_max = max((entry[key]["relative_l2"] for item in validations for entry in item["basis"]
                     for key in ("analysis_of_synthesis_rust", "analysis_of_synthesis_ducc")), default=0.0)
    adjoint_max = max((item["adjoints"][backend][key] for item in validations
                       for backend in ("rustsht", "ducc")
                       for key in ("synthesis_relative_residual", "analysis_relative_residual")), default=0.0)
    process_medians: dict[tuple[str, str, int, str], list[tuple[int, float]]] = defaultdict(list)
    for sample in samples:
        timings = sample.get("batch_ms", [])
        if timings:
            process_medians[(sample["case_id"], sample["operation"],
                             sample["threads"], sample["backend"])].append(
                (sample["process_pair"], statistics.median(timings))
            )
    process_spreads = []
    for (case_id, operation, threads, backend), observations in process_medians.items():
        if threads != 8:
            continue
        case = next(item for item in cases if item["case_id"] == case_id)
        if case["lmax"] < 1024 or len(observations) < 3:
            continue
        values = [value for _, value in sorted(observations)]
        if min(values) > 0:
            process_spreads.append((max(values) / min(values), case, operation, backend, values))
    process_spreads.sort(key=lambda item: item[0], reverse=True)
    if process_spreads:
        lines += [
            "",
            "Largest q8 high-degree process-pair spreads (triplets are per-process medians in ms; all observations are retained):",
        ]
        for spread, case, operation, backend, values in process_spreads[:2]:
            lines.append(
                f"- T{case['lmax']} m{case['mmax']} {case['longitude_mode']} {operation} {backend}: "
                f"{spread:.2f}× across process pairs ({', '.join(f'{value:.4g}' for value in values)} ms)."
            )
    lines += [
        f"Validation ran for {len(validations)} case/thread configurations: finite edge outputs {finite_validations}/{len(validations)}, reused-output reset checks {reset_validations}/{len(validations)}; maximum roundtrip relative L2 `{roundtrip_max:.3e}`, selected-basis recovery `{basis_max:.3e}`, and adjoint residual `{adjoint_max:.3e}`. Sidecar results include zero, subnormal and extreme-finite checks.",
        f"Observed CPU frequency median/p10/p90 `{fmt(host['frequency_ghz_median'])}/{fmt(host['frequency_ghz_p10'])}/{fmt(host['frequency_ghz_p90'])}` GHz; scheduler wait median/p90 `{fmt(host['runqueue_wait_fraction_median'])}/{fmt(host['runqueue_wait_fraction_p90'])}`; involuntary-switch median/max `{fmt(host['involuntary_switches_median'])}/{host['involuntary_switches_max']}`. No batch samples are removed from the saved distributions.",
        f"There were {len(failures)} recorded failure/warning events. Inspect `{result_prefix}/failures.csv` and the JSON `failures` array; unsupported cases are labeled separately from worker and numerical-validation errors.",
        "",
        "## Principal figures",
        "",
        f"- [Analysis, full mmax](./{result_prefix}/ratio-analysis-full-mmax.svg)",
        f"- [Analysis, reduced mmax](./{result_prefix}/ratio-analysis-reduced-mmax.svg)",
        f"- [Synthesis, full mmax](./{result_prefix}/ratio-synthesis-full-mmax.svg)",
        f"- [Synthesis, reduced mmax](./{result_prefix}/ratio-synthesis-reduced-mmax.svg)",
        f"- [Absolute analysis latency](./{result_prefix}/runtime-analysis.svg)",
        f"- [Absolute synthesis latency](./{result_prefix}/runtime-synthesis.svg)",
        *( [f"- [Precomputed table versus recurrence](./{result_prefix}/legendre-table-vs-recurrence.svg)",
            f"- [Paired FFT versus independent rows](./{result_prefix}/paired-fft-benefit.svg)"]
           if grid == "GL" else
           [f"- [Matched CC/GL comparison](./{result_prefix}/cc-versus-gl-matched-ratio.svg)"] ),
        "",
        f"The four principal figures plot measured warmed RustSHT/DUCC ratios for even-smooth longitude cases only; q1 and q8 are separate curves and measured points are not smoothed. Odd-minimum cases remain in the data and shape summary. This is a {grid_name} dataset; it is not merged with GL timing rows. The exact CC fixtures use `ntheta=lmax+2` to include both poles, while the GL controls use `ntheta=lmax+1`.",
        "",
        "## Reproduction and attribution",
        "",
        f"The machine-readable artifacts are [`results.csv`](./{result_prefix}/results.csv), [`results.json`](./{result_prefix}/results.json), and [`coverage-audit.json`](./{result_prefix}/coverage-audit.json). Rust plan setup/first-call fields, calibrated warm batch distributions, process pairing, per-case numerical errors, host scheduling/frequency diagnostics, and every requested geometry are retained.",
        "",
        "```sh",
        f"git checkout {metadata.get('benchmark_harness_revision', metadata['rust_source_revision'])}",
        f"git -C \"$DUCC_SRC\" checkout --detach {metadata['ducc_source_revision']}",
        "# Build once with the checked-in native parity CMake project and the recorded toolchain/flags.",
    ]
    lines.extend(invocations)
    lines += [
        "```",
        "",
        "The October 20 sweep is retained as a historical illustration only. It used a different Rust source/toolchain combination; its points are not merged into this dataset or used to compute the ratios above.",
        "",
    ]
    report.write_text("\n".join(lines))


def merge_shards(shard_paths: list[pathlib.Path], output_dir: pathlib.Path,
                 report_path: pathlib.Path, dense_sweep: bool) -> None:
    shards = [json.loads(path.read_text()) for path in shard_paths]
    if not shards:
        raise ValueError("no shard results supplied")
    metadata = dict(shards[0]["metadata"])
    invariant_fields = (
        "rust_source_revision", "production_source_revision", "ducc_source_revision", "grid",
        "ducc_version", "rustc", "cargo", "cxx", "rust_flags", "ducc_flags",
        "cpu_model", "selected_q1_cpu", "selected_q8_cpus", "selected_q8_topology",
        "selected_q8_l3_cache_ids", "process_pairs", "batches_per_process",
        "target_batch_ms", "phi0", "dense_sweep", "sweep_id",
        "ducc_sht_compile_command", "ducc_driver_compile_command",
        "production_code_changes_since_baseline", "production_code_matches_baseline",
        "rust_binary_sha256", "ducc_binary_sha256", "rust_build_command",
        "ducc_cmake_build_command",
        "retain_local_host_telemetry",
    )
    for index, shard in enumerate(shards[1:], 2):
        for field in invariant_fields:
            if shard["metadata"].get(field) != metadata.get(field):
                raise ValueError(f"shard {index} metadata mismatch for {field}")
    cases: list[dict] = []
    raw: list[dict] = []
    errors: list[dict] = []
    validations: list[dict] = []
    failures: list[dict] = []
    invocations = []
    seen_case_ids = set()
    seen_samples = set()
    for path, shard in zip(shard_paths, shards):
        shard_cases = shard["cases"]
        shard_ids = {case["case_id"] for case in shard_cases}
        overlap = seen_case_ids & shard_ids
        if overlap:
            raise ValueError(f"duplicate case IDs across shards: {sorted(overlap)[:8]}")
        seen_case_ids |= shard_ids
        cases.extend(shard_cases)
        for sample in shard["samples"]:
            key = (sample["case_id"], sample["operation"], sample["threads"],
                   sample["backend"], sample["process_pair"])
            if key in seen_samples:
                raise ValueError(f"duplicate native sample key: {key}")
            seen_samples.add(key)
            raw.append(sample)
        errors.extend(shard["numerics"])
        validations.extend(shard["validation"])
        failures.extend(shard.get("failures", []))
        invocation = shard["metadata"].get("invocation")
        if invocation:
            invocations.append(invocation)
    cases.sort(key=lambda case: (case["lmax"], case["mmax"], case["nphi"], case.get("grid", "GL")))
    raw.sort(key=lambda item: (item["lmax"], item["case_id"], item["operation"],
                               item["threads"], item["process_pair"], item["backend"]))
    errors.sort(key=lambda item: (item["lmax"], item["case_id"], item["operation"],
                                  item["threads"], item["process_pair"]))
    validations.sort(key=lambda item: (item["lmax"], item["case_id"], item["threads"]))
    metadata["dense_sweep"] = dense_sweep
    metadata["merged_shards"] = [
        {
            "name": path.name,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "case_count": len(shard["cases"]),
        }
        for path, shard in zip(shard_paths, shards)
    ]
    metadata["invocation"] = "merged shard results"
    diagnostics = [sample["host_diagnostics"] for sample in raw if "host_diagnostics" in sample]
    analysis = {
        "errors": errors,
        "validations": validations,
        "failures": failures,
        "host": summarize_host(diagnostics),
    }
    metadata.pop("retain_local_host_telemetry", None)
    rows = write_artifacts(output_dir, metadata, cases, raw, analysis)
    if metadata.get("grid", "GL") == "CC":
        write_cc_gl_comparison_plot(
            output_dir,
            ROOT / "reports/data/ducc-full-transform-parity-current-2026-10-22/results.csv",
        )
    if dense_sweep:
        coverage = dense_sweep_coverage(cases, raw, errors, failures, metadata.get("grid", "GL"))
        write_dense_sweep_report(output_dir, report_path, metadata, rows, cases,
                                 coverage, validations, failures, analysis["host"],
                                 invocations, raw)


def fmt(value):
    return "unavailable" if value is None else f"{value:.4g}"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--ducc-src", type=pathlib.Path, default=os.environ.get("DUCC_SRC"))
    parser.add_argument("--ducc-revision", default=DUCC_REVISION,
                        help="required immutable DUCC commit SHA (default: current active pin)")
    parser.add_argument("--grid", choices=("GL", "CC"), default="GL",
                        help="native scalar grid; exact CC fixtures use ntheta=lmax+2")
    parser.add_argument("--plot-only", action="store_true", help="regenerate ratio and runtime SVGs from existing results.csv; no benchmark or DUCC checkout needed")
    parser.add_argument("--merge-shards", type=pathlib.Path, nargs="+",
                        help="merge completed native benchmark shard results.json files")
    parser.add_argument("--dense-sweep", action="store_true",
                        help="select every lmax 0..256 plus representative higher degrees")
    parser.add_argument("--production-source-revision",
                        help="frozen Rust production source commit represented by this benchmark")
    parser.add_argument("--sweep-id", default="",
                        help="label recorded in machine-readable metadata")
    parser.add_argument("--smoke", action="store_true", help="small harness check; not a parity result")
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--case-limit", type=int, help="run only the first N configurations")
    parser.add_argument("--lmax", type=int, action="append", help="run all cases at this lmax; may be repeated")
    parser.add_argument("--max-lmax", type=int,
                        help="run every defined case with lmax at or below this value")
    parser.add_argument("--mmax", type=int,
                        help="use one fixed mmax for every selected lmax")
    parser.add_argument("--fixed-nphi", type=int,
                        help="use one fixed longitude count for every selected case")
    parser.add_argument("--fixed-ntheta", type=int,
                        help="use one latitude count for every selected case; must meet the grid's minimum")
    parser.add_argument("--full-mmax-only", action="store_true",
                        help="omit reduced-order configurations from the selected degrees")
    parser.add_argument("--longitude-mode", choices=("odd_minimum", "even_smooth"), action="append",
                        help="retain only this longitude shape; may be repeated")
    parser.add_argument("--operation", choices=("analysis", "synthesis"), action="append",
                        help="run only these operations; may be repeated (default: both)")
    parser.add_argument("--threads", choices=(1, 8), type=int, action="append",
                        help="run only these thread counts; may be repeated (default: 1 and 8)")
    parser.add_argument("--results-dir", type=pathlib.Path,
                        help="directory for machine-readable outputs and plots")
    parser.add_argument("--report-path", type=pathlib.Path,
                        help="report path; ignored with --no-report")
    parser.add_argument("--no-report", action="store_true",
                        help="write machine-readable artifacts without replacing a report")
    parser.add_argument("--retain-local-host-telemetry", action="store_true",
                        help="retain raw per-process CPU-frequency/scheduling samples in this local shard (do not publish them)")
    args = parser.parse_args()
    default_results_dir = CC_DATA_DIR if args.grid == "CC" else DATA_DIR
    default_report_path = CC_REPORT_PATH if args.grid == "CC" else REPORT_PATH
    args.results_dir = (args.results_dir or default_results_dir).resolve()
    args.report_path = (args.report_path or default_report_path).resolve()
    if args.merge_shards:
        merge_shards(
            [path.resolve() for path in args.merge_shards], args.results_dir,
            args.report_path, args.dense_sweep,
        )
        if args.dense_sweep:
            with (args.results_dir / "coverage-audit.json").open() as handle:
                coverage = json.load(handle)
            print(
                f"MERGED {len(args.merge_shards)} shards; case selection pass="
                f"{coverage['coverage_pass']}; measurements="
                f"{coverage['measurement_status_counts']}",
                flush=True,
            )
        else:
            print(f"MERGED {len(args.merge_shards)} benchmark shards", flush=True)
        return
    if args.plot_only:
        with (args.results_dir / "results.csv").open(newline="") as handle:
            rows = list(csv.DictReader(handle))
        if not rows:
            parser.error("results.csv has no benchmark rows")
        with (args.results_dir / "results.json").open() as handle:
            saved_metadata = json.load(handle).get("metadata", {})
        grid = saved_metadata.get("grid", args.grid)
        write_issue_ratio_plots(rows, args.results_dir, grid)
        write_issue_runtime_plots(rows, args.results_dir, grid)
        auxiliary = write_dispatch_and_pairing_plots(args.results_dir) if grid == "GL" else []
        if grid == "CC":
            comparison = write_cc_gl_comparison_plot(
                args.results_dir,
                ROOT / "reports/data/ducc-full-transform-parity-current-2026-10-22/results.csv",
            )
            if comparison is not None:
                auxiliary.append(comparison.name)
        print(
            f"Regenerated four ratio, two runtime and {len(auxiliary)} auxiliary figures "
            f"from {len(rows)} committed rows"
        )
        return
    if args.ducc_src is None:
        parser.error("--ducc-src or DUCC_SRC must name the pinned DUCC checkout")
    ducc_src = pathlib.Path(args.ducc_src).resolve()
    revision = command_output(["git", "rev-parse", "HEAD"], ducc_src)
    if revision != args.ducc_revision:
        raise RuntimeError(f"DUCC source revision {revision} != required {args.ducc_revision}")
    if (not args.no_report and args.results_dir.resolve() == DATA_DIR.resolve()
            and revision != HISTORICAL_DUCC_REVISION):
        raise RuntimeError("refusing to overwrite the historical 80388904 benchmark artifacts; choose a new --results-dir and --report-path or pass --no-report")
    compiler = pathlib.Path(shlex.split(os.environ.get("CXX", "g++"))[0]).name
    build_dir = pathlib.Path(tempfile.gettempdir()) / f"rustsht-ducc-full-parity-{revision[:12]}-{compiler}"
    binaries = build(ducc_src, build_dir, args.skip_build)
    if args.mmax is not None and args.mmax < 0:
        parser.error("--mmax must be nonnegative")
    if args.fixed_nphi is not None and args.fixed_nphi < 1:
        parser.error("--fixed-nphi must be positive")
    if args.fixed_ntheta is not None and args.fixed_ntheta < 1:
        parser.error("--fixed-ntheta must be positive")
    if args.max_lmax is not None and args.max_lmax < 0:
        parser.error("--max-lmax must be nonnegative")
    try:
        cases = make_cases(
            args.smoke,
            args.mmax,
            args.fixed_nphi,
            args.fixed_ntheta,
            set(args.lmax) if args.lmax else None,
            args.max_lmax,
            args.dense_sweep,
            set(args.longitude_mode) if args.longitude_mode else None,
            args.grid,
        )
    except ValueError as error:
        parser.error(str(error))
    if args.full_mmax_only:
        cases = [case for case in cases if case["mmax"] == case["lmax"]]
    if args.longitude_mode:
        selected_modes = set(args.longitude_mode)
        cases = [case for case in cases if case["longitude_mode"] in selected_modes]
    if not cases:
        parser.error("case filters selected no configurations")
    if args.case_limit:
        cases = cases[: args.case_limit]
    if args.smoke:
        global PROCESS_PAIRS, TARGET_BATCH_MS
        PROCESS_PAIRS = 1
        TARGET_BATCH_MS = 5.0
    metadata = hardware_metadata(ducc_src)
    metadata.update({
        "production_source_revision": args.production_source_revision or metadata["rust_source_revision"],
        "benchmark_harness_revision": metadata["rust_source_revision"],
        "benchmark_invocation": shlex.join([sys.executable, *sys.argv]),
        "invocation": shlex.join([sys.executable, *sys.argv]),
        "dense_sweep": args.dense_sweep,
        "grid": args.grid,
        "sweep_id": args.sweep_id,
        "retain_local_host_telemetry": args.retain_local_host_telemetry,
        "process_pairs": PROCESS_PAIRS,
        "batches_per_process": BATCHES_PER_PROCESS,
        "target_batch_ms": TARGET_BATCH_MS,
        "phi0": PHI0,
        "rust_binary_sha256": sha256_file(binaries["rustsht"]),
        "ducc_binary_sha256": sha256_file(binaries["ducc"]),
        "rust_build_command": "RUSTFLAGS='" + RUST_FLAGS + "' cargo build --release --example ducc_full_transform_parity --features parallel",
        "ducc_cmake_build_command": (
            f"cmake -S {DATA_DIR} -B {build_dir} -DDUCC_SRC={ducc_src} "
            f"-DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER={shlex.split(os.environ.get('CXX', 'g++'))[0]} "
            "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON; cmake --build " + str(build_dir) + " --parallel 4"
        ),
    })
    baseline = metadata["production_source_revision"]
    if re.fullmatch(r"[0-9a-fA-F]{40}", baseline):
        delta = subprocess.run(
            ["git", "diff", "--name-only", f"{baseline}..HEAD", "--", "src", "Cargo.toml", "Cargo.lock"],
            cwd=ROOT, text=True, capture_output=True, check=True,
        ).stdout.splitlines()
        metadata["production_code_changes_since_baseline"] = delta
        metadata["production_code_matches_baseline"] = not delta and metadata["rust_source_worktree_clean"]
    else:
        metadata["production_code_changes_since_baseline"] = None
        metadata["production_code_matches_baseline"] = False
    ducc_commands = json.loads((build_dir / "compile_commands.json").read_text())
    metadata["ducc_sht_compile_command"] = next(
        entry.get("command", " ".join(entry.get("arguments", [])))
        for entry in ducc_commands if pathlib.Path(entry["file"]).name == "sht.cc"
    )
    metadata["ducc_driver_compile_command"] = next(
        entry.get("command", " ".join(entry.get("arguments", [])))
        for entry in ducc_commands
        if pathlib.Path(entry["file"]).name == "ducc_full_transform_parity.cpp"
    )
    print(f"BUILD rust={binaries['rustsht']} DUCC={binaries['ducc']}", flush=True)
    print(f"ENV cpu={metadata['cpu_model']} x86={metadata['cpu_features']} affinity=q1/q8 physical cores", flush=True)
    print(f"DISPATCH Rust=Pulp V3 selected by worker; DUCC=fixed x86-64-v3", flush=True)
    raw, aux = run_matrix(
        cases, binaries, args.smoke, args.threads, args.operation,
    )
    if args.smoke:
        rows = aggregate(raw, aux["errors"])
        finite_count = sum(item["finite_outputs"] for item in aux["validations"])
        reset_count = sum(all(item["reset_consistent"].values()) for item in aux["validations"])
        print(
            f"SMOKE complete: {len(cases)} configs, {len(raw)} worker invocations, "
            f"{len(aux['errors'])} random-output comparisons, max relative L2="
            f"{max((row['relative_l2'] for row in rows), default=float('nan')):.3e}; "
            f"finite edge checks {finite_count}/{len(aux['validations'])}, "
            f"repeat reset checks {reset_count}/{len(aux['validations'])}",
            flush=True,
        )
        for item in aux["validations"]:
            print(
                f"VALIDATION {item['case_id']} roundtrip Rust/DUCC="
                f"{item['roundtrip']['rustsht']['relative_l2']:.3e}/"
                f"{item['roundtrip']['ducc']['relative_l2']:.3e} adjoint="
                f"{item['adjoints']['rustsht']['synthesis_relative_residual']:.3e},"
                f"{item['adjoints']['rustsht']['analysis_relative_residual']:.3e}/"
                f"{item['adjoints']['ducc']['synthesis_relative_residual']:.3e},"
                f"{item['adjoints']['ducc']['analysis_relative_residual']:.3e}",
                flush=True,
            )
        return
    output_dir = args.results_dir
    rows = aggregate(raw, aux["errors"])
    write_artifacts(output_dir, metadata, cases, raw, aux)
    if not args.no_report:
        if args.dense_sweep:
            coverage = dense_sweep_coverage(
                cases, raw, aux["errors"], aux.get("failures", []), args.grid
            )
            write_dense_sweep_report(
                output_dir, args.report_path, metadata, rows, cases, coverage,
                aux["validations"], aux.get("failures", []), aux["host"],
                [shlex.join([sys.executable, *sys.argv])], raw,
            )
            if args.grid == "CC":
                write_cc_gl_comparison_plot(
                    output_dir,
                    ROOT / "reports/data/ducc-full-transform-parity-current-2026-10-22/results.csv",
                )
        else:
            write_report(output_dir, metadata, rows, cases, aux["host"], aux["validations"], args.report_path)
    max_relative_l2 = max((row["relative_l2"] for row in rows), default=float("nan"))
    print(
        f"RESULTS {len(rows)} rows, {len(aux['errors'])} numerical comparisons, "
        f"{len(aux.get('failures', []))} failure records; max relative L2={max_relative_l2:.3e}",
        flush=True,
    )


if __name__ == "__main__":
    main()
