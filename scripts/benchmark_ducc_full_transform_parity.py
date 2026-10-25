#!/usr/bin/env python3
"""Matched native RustSHT/DUCC scalar GL, CC, and focused F1 benchmark."""

from __future__ import annotations

import argparse
import csv
import ctypes
import ctypes.util
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
F1_DATA_DIR = ROOT / "reports/data/ducc-scalar-f64-f1-performance-2026-10-23"
F1_REPORT_PATH = ROOT / "reports/ducc-scalar-f64-f1-performance-2026-10-23.md"
RUST_FLAGS = os.environ.get(
    "RUSTSHT_FULL_TRANSFORM_RUSTFLAGS",
    "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
)
PHI0 = 0.23
TARGET_BATCH_MS = 25.0
PROCESS_PAIRS = 3
BATCHES_PER_PROCESS = 10
# The random-map gate is stricter than the scalar analysis/synthesis tests in
# src/transform/tests.rs. Validation uses that existing 5e-11 tolerance; the
# adjoint gate also covers the cancellation-conditioned T214 dot-product case
# audited in reports/f64-ducc-independent-review-2026-10-22.md.
RANDOM_OUTPUT_RELATIVE_L2_TOLERANCE = 5e-12
VALIDATION_RELATIVE_L2_TOLERANCE = 5e-11
ADJOINT_RELATIVE_RESIDUAL_TOLERANCE = 5e-11
ERROR_METRICS_CONTRACT = (
    "complex-magnitude-relative-l2-v2;"
    "zero-reference-subnormal-abs-below-min-normal-v1"
)
LEGACY_STABLE_NORM_METRIC_CONTRACT = "complex-magnitude-stable-norm-v1"
LEGACY_STABLE_NORM_HARNESS_REVISION = "cd8beaa3d554b9d45725eee73bb55697767d9a37"
LEGACY_STABLE_NORM_HARNESS_SHA256 = "2d2181a16a16fe47c97e1c361ff129990e93d33b0d926ecd42ec8caf7bc5ccf2"
SUPPORTED_METRIC_CONTRACTS = frozenset({
    ERROR_METRICS_CONTRACT,
    LEGACY_STABLE_NORM_METRIC_CONTRACT,
})
ACTIVE_METRIC_CONTRACT = ERROR_METRICS_CONTRACT
SUBNORMAL_ABSOLUTE_ERROR_LIMIT = float(np.finfo(np.float64).tiny)
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


def finite_json_value(value):
    """Encode undefined/nonfinite metrics as JSON null, never invalid JSON."""
    if isinstance(value, dict):
        return {key: finite_json_value(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [finite_json_value(item) for item in value]
    if isinstance(value, (float, np.floating)):
        return float(value) if math.isfinite(float(value)) else None
    if isinstance(value, np.integer):
        return int(value)
    return value


def write_json_sidecar(path: pathlib.Path, value, *, indent: int | None = None) -> None:
    """Write valid JSON while retaining records with undefined numeric fields."""
    path.write_text(
        json.dumps(finite_json_value(value), indent=indent, allow_nan=False) + "\n"
    )


def strip_local_host_telemetry(value):
    """Remove local host-only diagnostics from merged historical attempts."""
    if isinstance(value, dict):
        return {
            key: strip_local_host_telemetry(item)
            for key, item in value.items() if key not in {"host_diagnostics", "host_samples"}
        }
    if isinstance(value, list):
        return [strip_local_host_telemetry(item) for item in value]
    return value


def samples_for_result_export(samples: list[dict], metadata: dict) -> list[dict]:
    """Keep raw shard telemetry local while exporting aggregate sample timings."""
    retain_local = metadata.get("retain_local_host_telemetry") is True
    merged = bool(metadata.get("merged_shards"))
    if retain_local and not merged:
        return samples
    return [
        {key: value for key, value in sample.items() if key != "host_diagnostics"}
        for sample in samples
    ]


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
    if grid not in {"GL", "CC", "F1"}:
        raise ValueError(f"unsupported grid {grid}; expected GL, CC, or F1")
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
                # The active CC manifest defines reduced order exactly as
                # floor(lmax / 4), including mmax=0 at T1--T3.  Preserve the
                # historical GL contract, whose low-degree series used
                # max(1, floor(lmax / 4)).
                reduced_mmax = lmax // 4 if grid == "CC" else max(1, lmax // 4)
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
    grid_salt = {
        "GL": 0,
        "CC": 0xCC20261022,
        "F1": 0xF120261023,
    }.get(case.get("grid", "GL"), 0)
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


def error_metrics(actual: np.ndarray, reference: np.ndarray) -> dict[str, float | int | None | bool]:
    actual = np.asarray(actual)
    reference = np.asarray(reference)
    if actual.shape != reference.shape:
        raise ValueError(f"metric shape mismatch: {actual.shape} != {reference.shape}")
    if not np.isfinite(actual).all() or not np.isfinite(reference).all():
        return {
            "absolute_max": None,
            "reference_peak": None,
            "relative_l2": None,
            "max_relative_above_1e-12_peak": None,
            "near_zero_reference_count": None,
            "metrics_finite": False,
        }
    delta = np.abs(actual - reference)
    reference_abs = np.abs(reference)
    error_peak = float(delta.max(initial=0.0))
    reference_peak = float(reference_abs.max(initial=0.0))
    if error_peak == 0.0:
        relative_l2 = 0.0
    elif reference_peak == 0.0:
        relative_l2 = math.inf
    else:
        scaled_error_norm = math.sqrt(float(np.square(delta / error_peak).sum()))
        scaled_reference_norm = math.sqrt(
            float(np.square(reference_abs / reference_peak).sum())
        )
        relative_l2 = (error_peak / reference_peak) * (scaled_error_norm / scaled_reference_norm)
    maximum = float(reference_abs.max(initial=0.0))
    near_zero = reference_abs <= max(maximum * 1e-12, np.finfo(np.float64).tiny)
    valid = ~near_zero
    max_relative = float((delta[valid] / reference_abs[valid]).max(initial=0.0)) if valid.any() else 0.0
    absolute_max: float | None = error_peak if math.isfinite(error_peak) else None
    relative_l2_value: float | None = relative_l2 if math.isfinite(relative_l2) else None
    max_relative_value: float | None = max_relative if math.isfinite(max_relative) else None
    return {
        "absolute_max": absolute_max,
        "reference_peak": reference_peak,
        "relative_l2": relative_l2_value,
        "max_relative_above_1e-12_peak": max_relative_value,
        "near_zero_reference_count": int(near_zero.sum()),
        "metrics_finite": (
            absolute_max is not None
            and relative_l2_value is not None
            and max_relative_value is not None
        ),
    }


def metric_contract_from_metadata(metadata: dict) -> str | None:
    """Resolve a declared metric contract or the one pinned pre-contract harness."""
    declared = metadata.get("numerical_metric_contract")
    harness_revision = metadata.get(
        "benchmark_harness_revision", metadata.get("rust_source_revision")
    )
    harness_sha256 = metadata.get("benchmark_harness_sha256")
    if declared == LEGACY_STABLE_NORM_METRIC_CONTRACT:
        if (harness_revision == LEGACY_STABLE_NORM_HARNESS_REVISION
                and harness_sha256 in {None, LEGACY_STABLE_NORM_HARNESS_SHA256}):
            return declared
        return None
    if declared is not None:
        return declared
    if (harness_revision == LEGACY_STABLE_NORM_HARNESS_REVISION
            and harness_sha256 in {None, LEGACY_STABLE_NORM_HARNESS_SHA256}):
        return LEGACY_STABLE_NORM_METRIC_CONTRACT
    return None


def compatible_metric_protocols(protocols: list[dict]) -> bool:
    """Allow only known metric-version differences over an otherwise exact protocol."""
    if not protocols:
        return False
    contracts = {protocol.get("numerical_metric_contract") for protocol in protocols}
    if None in contracts or not contracts.issubset(SUPPORTED_METRIC_CONTRACTS):
        return False
    common = [
        {key: value for key, value in protocol.items()
         if key != "numerical_metric_contract"}
        for protocol in protocols
    ]
    return all(protocol == common[0] for protocol in common[1:])


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


def zero_reference_subnormal_comparison_acceptable(
    metric: dict, expected_count: int
) -> bool:
    """Accept only finite underflow-sized error when the reference is exactly zero."""
    absolute_max = metric.get("absolute_max")
    return (
        metric.get("relative_l2") is None
        and metric.get("reference_peak") == 0.0
        and metric.get("near_zero_reference_count") == expected_count
        and isinstance(absolute_max, (int, float))
        and math.isfinite(absolute_max)
        and absolute_max <= SUBNORMAL_ABSOLUTE_ERROR_LIMIT
    )


def zero_reference_exact_comparison_acceptable(metric: dict, expected_count: int) -> bool:
    return (
        metric.get("reference_peak") == 0.0
        and metric.get("absolute_max") == 0.0
        and metric.get("relative_l2") == 0.0
        and metric.get("near_zero_reference_count") == expected_count
        and metric.get("metrics_finite") is True
    )


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
    checks.update({
        "lmax": case["lmax"],
        "mmax": case["mmax"],
        "ntheta": case["ntheta"],
        "nphi": case["nphi"],
    })
    checks["acceptance_status"], checks["acceptance_failures"] = saved_validation_acceptance(
        checks, case
    )
    return checks


def _validation_metric_failures(metric, label: str, limit: float, expected_count: int,
                                allow_zero_reference_subnormal: bool = False,
                                require_exact_zero: bool = False,
                                metric_contract: str | None = None) -> list[str]:
    if not isinstance(metric, dict):
        return [f"{label}_metrics_missing"]
    metric_contract = metric_contract or metric.get("numerical_metric_contract") or ACTIVE_METRIC_CONTRACT
    if metric_contract == LEGACY_STABLE_NORM_METRIC_CONTRACT:
        failures = []
        for field in ("absolute_max", "max_relative_above_1e-12_peak"):
            value = metric.get(field)
            if (isinstance(value, bool) or not isinstance(value, (int, float))
                    or not math.isfinite(value) or value < 0.0):
                failures.append(f"{label}_{field}_missing_or_nonfinite")
        near_zero_count = metric.get("near_zero_reference_count")
        if (not isinstance(near_zero_count, int) or isinstance(near_zero_count, bool)
                or near_zero_count < 0 or near_zero_count > expected_count):
            failures.append(f"{label}_near_zero_reference_count_invalid")
        relative_l2 = metric.get("relative_l2")
        if (isinstance(relative_l2, bool) or not isinstance(relative_l2, (int, float))
                or not math.isfinite(relative_l2) or relative_l2 < 0.0):
            failures.append(f"{label}_relative_l2_missing_or_nonfinite")
        if metric.get("metrics_finite") is False:
            failures.append(f"{label}_metrics_not_finite")
        if require_exact_zero:
            if (metric.get("absolute_max") != 0.0 or relative_l2 != 0.0
                    or metric.get("max_relative_above_1e-12_peak") != 0.0):
                failures.append(f"{label}_not_exact_zero")
            if near_zero_count != expected_count:
                failures.append(f"{label}_zero_reference_count_invalid")
        elif allow_zero_reference_subnormal:
            if near_zero_count != expected_count:
                failures.append(f"{label}_subnormal_reference_count_invalid")
            absolute_max = metric.get("absolute_max")
            if (isinstance(absolute_max, (int, float))
                    and not isinstance(absolute_max, bool)
                    and absolute_max > SUBNORMAL_ABSOLUTE_ERROR_LIMIT):
                failures.append(f"{label}_legacy_subnormal_absolute_error_exceeded")
        if (isinstance(relative_l2, (int, float)) and not isinstance(relative_l2, bool)
                and math.isfinite(relative_l2) and relative_l2 > limit):
            failures.append(f"{label}_relative_l2_exceeded")
        return failures
    if metric_contract != ERROR_METRICS_CONTRACT:
        return [f"{label}_numerical_metric_contract_unrecognized"]
    failures = []
    for field in ("absolute_max", "reference_peak", "max_relative_above_1e-12_peak"):
        value = metric.get(field)
        if (isinstance(value, bool) or not isinstance(value, (int, float))
                or not math.isfinite(value)
                or value < 0.0):
            failures.append(f"{label}_{field}_missing_or_nonfinite")
    near_zero_count = metric.get("near_zero_reference_count")
    if (not isinstance(near_zero_count, int) or isinstance(near_zero_count, bool) or near_zero_count < 0
            or near_zero_count > expected_count):
        failures.append(f"{label}_near_zero_reference_count_invalid")
    relative_l2 = metric.get("relative_l2")
    if require_exact_zero:
        if (metric.get("absolute_max") != 0.0 or isinstance(relative_l2, bool)
                or not isinstance(relative_l2, (int, float))
                or not math.isfinite(relative_l2) or relative_l2 != 0.0):
            failures.append(f"{label}_not_exact_zero")
        if (metric.get("reference_peak") != 0.0
                or near_zero_count != expected_count):
            failures.append(f"{label}_zero_reference_record_invalid")
        if metric.get("metrics_finite") is not True:
            failures.append(f"{label}_metrics_not_finite")
    elif (allow_zero_reference_subnormal
          and zero_reference_subnormal_comparison_acceptable(metric, expected_count)):
        if metric.get("metrics_finite") is not False:
            failures.append(f"{label}_metrics_finite_flag_inconsistent")
    elif (isinstance(relative_l2, bool) or not isinstance(relative_l2, (int, float))
          or not math.isfinite(relative_l2)):
        failures.append(f"{label}_relative_l2_missing_or_nonfinite")
    elif metric.get("metrics_finite") is not True:
        failures.append(f"{label}_metrics_not_finite")
    if (not require_exact_zero and isinstance(metric.get("reference_peak"), (int, float))
            and not isinstance(metric.get("reference_peak"), bool)
            and metric["reference_peak"] == 0.0
            and not (allow_zero_reference_subnormal
                     and zero_reference_subnormal_comparison_acceptable(metric, expected_count))
            and not (allow_zero_reference_subnormal
                     and zero_reference_exact_comparison_acceptable(metric, expected_count))):
        failures.append(f"{label}_unexpected_zero_reference")
    if (not isinstance(relative_l2, bool) and isinstance(relative_l2, (int, float))
            and math.isfinite(relative_l2)
            and relative_l2 < 0.0):
        failures.append(f"{label}_relative_l2_invalid")
    if (isinstance(relative_l2, (int, float)) and math.isfinite(relative_l2)
            and relative_l2 > limit):
        failures.append(f"{label}_relative_l2_exceeded")
    return failures


def random_comparison_failures(record: dict | None,
                               metric_contract: str | None = None) -> list[str]:
    if not isinstance(record, dict):
        return ["random_numeric_record_missing"]
    if record.get("finite_outputs") is not True:
        return ["random_output_nonfinite"]
    geometry = (record.get("lmax"), record.get("mmax"),
                record.get("ntheta"), record.get("nphi"))
    if not all(isinstance(value, int) and not isinstance(value, bool) for value in geometry):
        return ["random_numeric_geometry_missing"]
    lmax, mmax, ntheta, nphi = geometry
    if lmax < 0 or mmax < 0 or mmax > lmax or ntheta <= 0 or nphi <= 0:
        return ["random_numeric_geometry_invalid"]
    expected_count = (
        ntheta * nphi if record.get("operation") == "synthesis"
        else (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) // 2
    )
    return _validation_metric_failures(
        record, "random_output", RANDOM_OUTPUT_RELATIVE_L2_TOLERANCE, expected_count,
        metric_contract=metric_contract or record.get("numerical_metric_contract"),
    )


def saved_validation_acceptance(validation: dict | None,
                                expected_case: dict | None = None,
                                metric_contract: str | None = None) -> tuple[str, list[str]]:
    """Apply strict per-field acceptance to one saved analytic validation record."""
    if validation is None:
        return "incomplete", ["missing_validation"]
    if not isinstance(validation, dict):
        return "rejected", ["malformed_validation"]
    failures = []
    metric_contract = (
        metric_contract or validation.get("numerical_metric_contract") or ACTIVE_METRIC_CONTRACT
    )
    if validation.get("finite_outputs") is not True:
        failures.append("nonfinite_validation_output")

    lmax, mmax = validation.get("lmax"), validation.get("mmax")
    ntheta, nphi = validation.get("ntheta"), validation.get("nphi")
    if expected_case is not None:
        lmax = expected_case.get("lmax")
        mmax = expected_case.get("mmax")
        ntheta = expected_case.get("ntheta")
        nphi = expected_case.get("nphi")
    if not all(isinstance(value, int) and not isinstance(value, bool)
               for value in (lmax, mmax, ntheta, nphi)):
        return "rejected", failures + ["validation_geometry_missing"]
    if lmax < 0 or mmax < 0 or mmax > lmax or ntheta <= 0 or nphi <= 0:
        failures.append("validation_geometry_invalid")
    if expected_case is not None and any(
        validation.get(field) != expected_case.get(field)
        for field in ("lmax", "mmax", "ntheta", "nphi")
    ):
        failures.append("validation_geometry_mismatch")
    nmap = ntheta * nphi
    nalm = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) // 2

    reset_flags = validation.get("reset_consistent")
    if not isinstance(reset_flags, dict) or set(reset_flags) != {"rustsht", "ducc"}:
        failures.append("reset_backend_set_incomplete")
    elif any(reset_flags[backend] is not True for backend in ("rustsht", "ducc")):
        failures.append("reused_output_reset_mismatch")

    expected_finite_flags = {"zero_finite", "extreme_finite", "subnormal_finite"}
    finite_flags = validation.get("finite_flags")
    if not isinstance(finite_flags, dict) or set(finite_flags) != {"rustsht", "ducc"}:
        failures.append("finite_flags_backend_set_incomplete")
    for backend in ("rustsht", "ducc"):
        flags = finite_flags.get(backend) if isinstance(finite_flags, dict) else None
        if not isinstance(flags, dict) or set(flags) != expected_finite_flags:
            failures.append(f"{backend}_finite_flags_incomplete")
        elif any(value is not True for value in flags.values()):
            failures.append(f"{backend}_finite_flags_failed")

    roundtrip = validation.get("roundtrip")
    expected_roundtrip = {"rustsht", "ducc", "rust_ducc"}
    if not isinstance(roundtrip, dict) or set(roundtrip) != expected_roundtrip:
        failures.append("roundtrip_backend_set_incomplete")
    for backend in expected_roundtrip:
        failures.extend(_validation_metric_failures(
            roundtrip.get(backend) if isinstance(roundtrip, dict) else None,
            f"roundtrip_{backend}", VALIDATION_RELATIVE_L2_TOLERANCE, nalm,
            metric_contract=metric_contract,
        ))

    basis = validation.get("basis")
    if not isinstance(basis, list):
        basis = []
        failures.append("basis_records_missing")
    expected_modes = set(selected_modes(lmax, mmax))
    observed_modes = {
        (item.get("l"), item.get("m")) for item in basis if isinstance(item, dict)
    }
    if observed_modes != expected_modes or len(basis) != len(expected_modes):
        failures.append("basis_record_set_mismatch")
    for item in basis:
        if not isinstance(item, dict):
            failures.append("basis_record_malformed")
            continue
        label = f"basis_l{item.get('l')}_m{item.get('m')}"
        for field, count in (
            ("map_rust_ducc", nmap),
            ("analysis_of_synthesis_rust", nalm),
            ("analysis_of_synthesis_ducc", nalm),
        ):
            failures.extend(_validation_metric_failures(
                item.get(field), f"{label}_{field}",
                VALIDATION_RELATIVE_L2_TOLERANCE, count,
                metric_contract=metric_contract,
            ))

    adjoints = validation.get("adjoints")
    if not isinstance(adjoints, dict) or set(adjoints) != {"rustsht", "ducc"}:
        failures.append("adjoint_backend_set_incomplete")
    for backend in ("rustsht", "ducc"):
        backend_adjoints = adjoints.get(backend) if isinstance(adjoints, dict) else None
        for operation in ("synthesis", "analysis"):
            residual = (backend_adjoints.get(f"{operation}_relative_residual")
                        if isinstance(backend_adjoints, dict) else None)
            if (isinstance(residual, bool) or not isinstance(residual, (int, float))
                    or not math.isfinite(residual)
                    or residual < 0.0 or residual > ADJOINT_RELATIVE_RESIDUAL_TOLERANCE):
                failures.append(f"{backend}_{operation}_adjoint_residual")

    edges = validation.get("edge_cases")
    expected_edges = {"zero", "extreme", "subnormal"}
    if not isinstance(edges, dict) or set(edges) != expected_edges:
        failures.append("edge_case_record_set_mismatch")
    for edge_name in ("zero", "extreme", "subnormal"):
        edge = edges.get(edge_name) if isinstance(edges, dict) else None
        if not isinstance(edge, dict):
            failures.append(f"{edge_name}_edge_case_missing")
            continue
        if edge_name == "zero" and edge.get("exact_zero") is not True:
            failures.append("zero_input_was_not_exact_zero")
        for field, count in (("map_rust_ducc", nmap), ("alm_rust_ducc", nalm)):
            failures.extend(_validation_metric_failures(
                edge.get(field), f"{edge_name}_{field}",
                VALIDATION_RELATIVE_L2_TOLERANCE, count,
                allow_zero_reference_subnormal=(edge_name == "subnormal"),
                require_exact_zero=(edge_name == "zero"),
                metric_contract=metric_contract,
            ))
    return ("rejected" if failures else "accepted"), sorted(set(failures))


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


def format_scientific(value) -> str:
    return f"{value:.3e}" if isinstance(value, (int, float)) and math.isfinite(value) else "unavailable"


def convert_svg_to_png(svg_path: pathlib.Path) -> pathlib.Path:
    """Rasterize an SVG through the system librsvg and Cairo libraries."""
    svg = svg_path.read_text()
    dimensions = re.search(r'<svg\b[^>]*\bwidth="(\d+)"[^>]*\bheight="(\d+)"', svg)
    if not dimensions:
        raise ValueError(f"SVG dimensions are not explicit: {svg_path}")
    width, height = map(int, dimensions.groups())
    rsvg_path = ctypes.util.find_library("rsvg-2")
    cairo_path = ctypes.util.find_library("cairo")
    gobject_path = ctypes.util.find_library("gobject-2.0")
    if not rsvg_path or not cairo_path or not gobject_path:
        raise RuntimeError("librsvg, Cairo, and GObject are required to emit PNG figure companions")
    rsvg = ctypes.CDLL(rsvg_path)
    cairo = ctypes.CDLL(cairo_path)
    gobject = ctypes.CDLL(gobject_path)

    class RsvgRectangle(ctypes.Structure):
        _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double),
                    ("width", ctypes.c_double), ("height", ctypes.c_double)]

    error = ctypes.c_void_p()
    rsvg.rsvg_handle_new_from_file.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_void_p)]
    rsvg.rsvg_handle_new_from_file.restype = ctypes.c_void_p
    handle = rsvg.rsvg_handle_new_from_file(os.fsencode(svg_path.resolve()), ctypes.byref(error))
    if not handle:
        message = "unable to load SVG with librsvg"
        if error.value:
            glib = ctypes.CDLL(ctypes.util.find_library("glib-2.0"))
            glib.g_error_free.argtypes = [ctypes.c_void_p]
            glib.g_error_free(error)
        raise RuntimeError(f"{message}: {svg_path}")
    cairo.cairo_image_surface_create.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_int]
    cairo.cairo_image_surface_create.restype = ctypes.c_void_p
    cairo.cairo_create.argtypes = [ctypes.c_void_p]
    cairo.cairo_create.restype = ctypes.c_void_p
    cairo.cairo_surface_write_to_png.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    cairo.cairo_surface_write_to_png.restype = ctypes.c_int
    cairo.cairo_destroy.argtypes = [ctypes.c_void_p]
    cairo.cairo_surface_destroy.argtypes = [ctypes.c_void_p]
    rsvg.rsvg_handle_render_document.argtypes = [
        ctypes.c_void_p, ctypes.c_void_p, ctypes.POINTER(RsvgRectangle),
        ctypes.POINTER(ctypes.c_void_p),
    ]
    rsvg.rsvg_handle_render_document.restype = ctypes.c_int
    gobject.g_object_unref.argtypes = [ctypes.c_void_p]
    surface = cairo.cairo_image_surface_create(0, width, height)  # CAIRO_FORMAT_ARGB32
    if not surface:
        gobject.g_object_unref(handle)
        raise RuntimeError("Cairo could not allocate the PNG surface")
    context = cairo.cairo_create(surface)
    if not context:
        cairo.cairo_surface_destroy(surface)
        gobject.g_object_unref(handle)
        raise RuntimeError("Cairo could not create a drawing context")
    viewport = RsvgRectangle(0.0, 0.0, float(width), float(height))
    rendered = rsvg.rsvg_handle_render_document(handle, context, ctypes.byref(viewport), ctypes.byref(error))
    png_path = svg_path.with_suffix(".png")
    write_status = cairo.cairo_surface_write_to_png(surface, os.fsencode(png_path)) if rendered else -1
    cairo.cairo_destroy(context)
    cairo.cairo_surface_destroy(surface)
    gobject.g_object_unref(handle)
    if error.value:
        glib = ctypes.CDLL(ctypes.util.find_library("glib-2.0"))
        glib.g_error_free.argtypes = [ctypes.c_void_p]
        glib.g_error_free(error)
    if not rendered or write_status != 0:
        raise RuntimeError(f"librsvg/Cairo failed to render {svg_path} to PNG")
    if not png_path.is_file() or not png_path.read_bytes().startswith(b"\x89PNG\r\n\x1a\n"):
        raise RuntimeError(f"Chromium did not produce a valid PNG for {svg_path}")
    return png_path


def convert_svg_directory_to_png(output_dir: pathlib.Path) -> list[pathlib.Path]:
    return [convert_svg_to_png(path) for path in sorted(output_dir.glob("*.svg"))]


def png_renderer_version() -> str:
    rsvg_path = ctypes.util.find_library("rsvg-2")
    cairo_path = ctypes.util.find_library("cairo")
    if not rsvg_path or not cairo_path:
        return "unavailable"
    rsvg = ctypes.CDLL(rsvg_path)
    cairo = ctypes.CDLL(cairo_path)
    cairo.cairo_version_string.restype = ctypes.c_char_p
    rsvg_version = ".".join(
        str(ctypes.c_uint.in_dll(rsvg, symbol).value)
        for symbol in ("rsvg_major_version", "rsvg_minor_version", "rsvg_micro_version")
    )
    return f"librsvg {rsvg_version} / Cairo {cairo.cairo_version_string().decode()}"


def atomic_write_json(path: pathlib.Path, value: dict) -> None:
    """Replace one checkpoint atomically and sync its directory entry."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    with temporary.open("w") as handle:
        json.dump(finite_json_value(value), handle, separators=(",", ":"), allow_nan=False)
        handle.write("\n")
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temporary, path)
    directory_fd = os.open(path.parent, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)


def checkpoint_identity(metadata: dict, cases: list[dict], operations: list[str],
                        threads: list[int]) -> dict:
    """Bind resumable measurements to their exact inputs and timing contract."""
    case_fields = ("case_id", "grid", "lmax", "mmax", "ntheta", "nphi", "longitude_mode")
    return {
        "schema_version": 1,
        "selected_cases": [
            {field: case.get(field) for field in case_fields} for case in cases
        ],
        "operations": list(operations),
        "threads": list(threads),
        "production_source_revision": metadata["production_source_revision"],
        "binary_source_revision": metadata.get("binary_source_revision"),
        "binary_build_record_sha256": (metadata.get("binary_build_record") or {}).get("sha256"),
        "benchmark_harness_sha256": metadata["benchmark_harness_sha256"],
        "ducc_source_revision": metadata["ducc_source_revision"],
        "rust_binary_sha256": metadata["rust_binary_sha256"],
        "ducc_binary_sha256": metadata["ducc_binary_sha256"],
        "rustc": metadata["rustc"],
        "cxx": metadata["cxx"],
        "rust_flags": metadata["rust_flags"],
        "ducc_flags": metadata["ducc_flags"],
        "ducc_sht_compile_command": metadata.get("ducc_sht_compile_command"),
        "ducc_driver_compile_command": metadata.get("ducc_driver_compile_command"),
        "cpu_model": metadata["cpu_model"],
        "selected_q1_cpu": metadata["selected_q1_cpu"],
        "selected_q8_cpus": metadata["selected_q8_cpus"],
        "selected_q8_topology": metadata["selected_q8_topology"],
        "selected_q8_l3_cache_ids": metadata["selected_q8_l3_cache_ids"],
        "retain_local_host_telemetry": metadata["retain_local_host_telemetry"],
        "process_pairs": PROCESS_PAIRS,
        "batches_per_process": BATCHES_PER_PROCESS,
        "target_batch_ms": TARGET_BATCH_MS,
        "short_operation_upper_bound_ms": 100.0,
        "qualified_batch_window_ms": [20.0, 100.0],
        "long_operation_samples_per_process": 5,
        "phi0": PHI0,
    }


def execution_protocol(metadata: dict) -> dict:
    """Fields that must agree before measurements from shards can be merged."""
    keys = (
        "production_source_revision", "ducc_source_revision",
        "numerical_metric_contract",
        "grid", "ducc_version", "rustc", "cargo", "cxx", "rust_flags", "ducc_flags",
        "cpu_model", "selected_q1_cpu", "selected_q8_cpus", "selected_q8_topology",
        "selected_q8_l3_cache_ids", "process_pairs", "batches_per_process",
        "target_batch_ms", "phi0", "rust_binary_sha256", "ducc_binary_sha256",
        "ducc_sht_compile_command", "ducc_driver_compile_command",
    )
    protocol = {key: metadata.get(key) for key in keys}
    protocol["numerical_metric_contract"] = metric_contract_from_metadata(metadata)
    # Older shards predate this field. Their production_source_revision was
    # explicitly supplied as the source represented by the frozen binary.
    protocol["binary_source_revision"] = metadata.get(
        "binary_source_revision", metadata.get("production_source_revision")
    )
    protocol.update({
        "short_operation_upper_bound_ms": 100.0,
        "qualified_batch_window_ms": [20.0, 100.0],
        "long_operation_samples_per_process": 5,
        "pairing": "interleaved process-pair median ratios; independent backend batches",
    })
    return protocol


def validate_binary_build_record(path: pathlib.Path, metadata: dict,
                                 declared_source_revision: str | None) -> dict:
    """Verify skipped-build executables against a saved native build record."""
    payload = json.loads(path.read_text())
    recorded = payload.get("metadata", payload)
    source_revision = recorded.get(
        "binary_source_revision", recorded.get("production_source_revision")
    )
    if not source_revision:
        raise ValueError(f"binary build record has no source revision: {path}")
    if declared_source_revision and declared_source_revision != source_revision:
        raise ValueError(
            f"declared production source {declared_source_revision} does not match "
            f"binary build record source {source_revision}"
        )
    fields = (
        "rust_binary_sha256", "ducc_binary_sha256", "ducc_source_revision",
        "rustc", "cxx", "rust_flags", "ducc_flags",
    )
    mismatches = [
        field for field in fields
        if recorded.get(field) != metadata.get(field)
    ]
    if mismatches:
        raise ValueError(
            "skipped-build executable does not match binary build record fields: "
            + ", ".join(mismatches)
        )
    return {
        "path": str(path.resolve()),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "binary_source_revision": source_revision,
        "rust_binary_sha256": recorded["rust_binary_sha256"],
        "ducc_binary_sha256": recorded["ducc_binary_sha256"],
        "verified_fields": list(fields),
    }


def checkpoint_group_path(directory: pathlib.Path, case_id: str,
                          operation: str, threads: int) -> pathlib.Path:
    key = f"{case_id}|{operation}|q{threads}".encode()
    return directory / f"{hashlib.sha256(key).hexdigest()}.json"


def checkpoint_pair_is_complete(pair_state: dict, validation_required: bool) -> bool:
    if pair_state.get("complete") is not True:
        return False
    if not validation_required:
        return pair_state.get("validation_status", "not_required") == "not_required"
    validation_status = pair_state.get("validation_status")
    payload = pair_state.get("payload")
    validations = payload.get("validations") if isinstance(payload, dict) else None
    return (
        validation_status in {"accepted", "rejected"}
        and isinstance(validations, list)
        and len(validations) == 1
        and isinstance(validations[0], dict)
        and validations[0].get("acceptance_status") == validation_status
    )


def checkpoint_history_records(checkpoint: dict, case_id: str, operation: str,
                               threads: int) -> list[dict]:
    """Return incomplete and superseded checkpoint attempts without merging their timings."""
    records = []
    for pair_key, state in checkpoint.get("pairs", {}).items():
        try:
            pair = int(pair_key)
        except (TypeError, ValueError):
            pair = pair_key
        validation_required = operation == "analysis" and pair == 0
        attempt_status = (
            "recovered" if checkpoint_pair_is_complete(state, validation_required)
            else "incomplete"
        )
        for attempt in state.get("attempt_history", []):
            if isinstance(attempt, dict) and isinstance(attempt.get("payload"), dict):
                payload = attempt["payload"]
                validation_status = attempt.get("validation_status", "unknown")
            else:  # Accept legacy checkpoints that stored payloads directly.
                payload = attempt if isinstance(attempt, dict) else {}
                validation_status = "unknown"
            if payload:
                records.append({
                    "case_id": case_id, "operation": operation, "threads": threads,
                    "process_pair": pair, "attempt_status": "superseded",
                    "validation_status": validation_status, "payload": payload,
                })
        if not checkpoint_pair_is_complete(state, validation_required):
            payload = state.get("payload", {})
            if payload:
                records.append({
                    "case_id": case_id, "operation": operation, "threads": threads,
                    "process_pair": pair, "attempt_status": attempt_status,
                    "validation_status": state.get("validation_status", "pending"),
                    "payload": payload,
                })
    return records


def checkpoint_pair_state(previous_pair: dict | None, payload: dict, complete: bool,
                          validation_status: str, validation_required: bool) -> dict:
    attempt_history = list(previous_pair.get("attempt_history", []) if previous_pair else [])
    if previous_pair and not checkpoint_pair_is_complete(previous_pair, validation_required):
        previous_payload = previous_pair.get("payload", {})
        if previous_payload:
            attempt_history.append({
                "validation_status": previous_pair.get("validation_status", "pending"),
                "payload": previous_payload,
            })
    return {
        "complete": complete,
        "validation_status": validation_status,
        "payload": payload,
        "attempt_history": attempt_history,
    }


def validate_checkpoint_directory(directory: pathlib.Path, identity: dict,
                                  resume: bool) -> None:
    identity_path = directory / "run-identity.json"
    if resume:
        if not identity_path.exists():
            raise RuntimeError(f"--resume requested but no checkpoint identity exists in {directory}")
        saved = json.loads(identity_path.read_text())
        if saved != identity:
            raise RuntimeError(
                "checkpoint identity mismatch; selected cases, binaries, source, host, "
                "toolchain, or timing protocol changed"
            )
        return
    if directory.exists() and any(directory.iterdir()):
        raise RuntimeError(
            f"checkpoint directory {directory} already contains data; pass --resume "
            "with the identical run identity or select a new directory"
        )
    atomic_write_json(identity_path, identity)


def run_matrix(
    cases: list[dict],
    binaries: dict[str, pathlib.Path],
    smoke: bool,
    threads_to_run: list[int] | None = None,
    operations_to_run: list[str] | None = None,
    checkpoint_dir: pathlib.Path | None = None,
    checkpoint_identity: dict | None = None,
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
    historical_attempts: list[dict] = []
    checkpoint_identity_hash = (
        hashlib.sha256(json.dumps(checkpoint_identity, sort_keys=True).encode()).hexdigest()
        if checkpoint_identity is not None else None
    )
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
                            "reason": type(error).__name__,
                        })
                print("FAIL: benchmark fixture generation", flush=True)
                continue
            for threads in threads_to_run:
                cpus = physical[:threads]
                for operation in operations_to_run:
                    checkpoint_path = (
                        checkpoint_group_path(checkpoint_dir, case["case_id"], operation, threads)
                        if checkpoint_dir is not None else None
                    )
                    if checkpoint_path is not None and checkpoint_path.exists():
                        checkpoint = json.loads(checkpoint_path.read_text())
                        if checkpoint.get("identity_sha256") != checkpoint_identity_hash:
                            raise RuntimeError(f"checkpoint identity mismatch in {checkpoint_path}")
                        if (checkpoint.get("case_id"), checkpoint.get("operation"),
                                checkpoint.get("threads")) != (case["case_id"], operation, threads):
                            raise RuntimeError(f"checkpoint group mismatch in {checkpoint_path}")
                    else:
                        checkpoint = {
                            "identity_sha256": checkpoint_identity_hash,
                            "case_id": case["case_id"],
                            "operation": operation,
                            "threads": threads,
                            "pairs": {},
                        }
                    historical_attempts.extend(
                        checkpoint_history_records(
                            checkpoint, case["case_id"], operation, threads
                        )
                    )
                    completed_pairs = set()
                    for pair_key, pair_state in checkpoint["pairs"].items():
                        pair_index = int(pair_key)
                        validation_required = operation == "analysis" and pair_index == 0
                        if not checkpoint_pair_is_complete(pair_state, validation_required):
                            continue
                        pair_payload = pair_state["payload"]
                        raw.extend(pair_payload.get("samples", []))
                        errors.extend(pair_payload.get("comparisons", []))
                        validations.extend(pair_payload.get("validations", []))
                        failures.extend(pair_payload.get("failures", []))
                        host_samples.extend(pair_payload.get("host_samples", []))
                        completed_pairs.add(pair_index)
                    progress += 1
                    print(
                        "CASE: starting benchmark",
                        flush=True,
                    )
                    for pair in range(pairs):
                        validate = operation == "analysis" and pair == 0
                        if pair in completed_pairs:
                            print("RESUME: completed benchmark pair", flush=True)
                            continue
                        pair_raw_start = len(raw)
                        pair_errors_start = len(errors)
                        pair_validations_start = len(validations)
                        pair_failures_start = len(failures)
                        pair_host_start = len(host_samples)
                        previous_pair = checkpoint["pairs"].get(str(pair))

                        def save_pair_checkpoint(complete: bool,
                                                 validation_status: str | None = None) -> None:
                            if checkpoint_path is None:
                                return
                            payload = {
                                "samples": raw[pair_raw_start:],
                                "comparisons": errors[pair_errors_start:],
                                "validations": validations[pair_validations_start:],
                                "failures": failures[pair_failures_start:],
                                "host_samples": host_samples[pair_host_start:],
                            }
                            validation_status = validation_status or (
                                "not_required" if not validate else "pending"
                            )
                            pair_state = checkpoint_pair_state(
                                previous_pair, payload, complete, validation_status, validate
                            )
                            checkpoint["pairs"][str(pair)] = pair_state
                            atomic_write_json(checkpoint_path, checkpoint)

                        order = ["rustsht", "ducc"] if pair % 2 == 0 else ["ducc", "rustsht"]
                        outputs = {}
                        pair_records = {}
                        validation_status = "pending" if validate else "not_required"
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
                                    "reason": failure_class,
                                })
                                print(
                                    "FAIL: benchmark worker",
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
                            if not result.get("reset_consistent", False):
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
                                    "failure_class": "output_reset_failed",
                                })
                        if len(outputs) != 2:
                            save_pair_checkpoint(False, validation_status)
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
                                "reason": type(error).__name__,
                            })
                            save_pair_checkpoint(False, validation_status)
                            continue
                        all_finite = bool(np.isfinite(rust_values).all() and np.isfinite(ducc_values).all())
                        if all_finite:
                            metrics = error_metrics(rust_values, ducc_values)
                            numerical_status = (
                                "accepted"
                                if metrics["relative_l2"] is not None
                                and metrics["relative_l2"] <= RANDOM_OUTPUT_RELATIVE_L2_TOLERANCE
                                else "rejected"
                            )
                        else:
                            metrics = {
                                "absolute_max": None,
                                "relative_l2": None,
                                "max_relative_above_1e-12_peak": None,
                                "near_zero_reference_count": None,
                                "metrics_finite": False,
                                "nonfinite_rust_values": int((~np.isfinite(rust_values)).sum()),
                                "nonfinite_ducc_values": int((~np.isfinite(ducc_values)).sum()),
                            }
                            numerical_status = "rejected"
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
                        if numerical_status == "rejected" and all_finite:
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
                                "failure_class": "numerical_tolerance_failed",
                                "metric": "relative_l2",
                                "value": metrics["relative_l2"],
                                "tolerance": RANDOM_OUTPUT_RELATIVE_L2_TOLERANCE,
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
                            "numerical_status": numerical_status,
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
                                    "reason": type(error).__name__,
                                })
                                save_pair_checkpoint(False, "error")
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
                            validation_status = validation["acceptance_status"]
                            if validation["acceptance_status"] != "accepted":
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
                                    "acceptance_failures": validation["acceptance_failures"],
                                })
                                print(
                                    "VALIDATION WARNING: benchmark validation failed",
                                    flush=True,
                                )
                        save_pair_checkpoint(True, validation_status)
            case["map_path"] = str(case["map_path"])
            case["alm_path"] = str(case["alm_path"])
    host = summarize_host(host_samples)
    return raw, {"errors": errors, "validations": validations, "host": host,
                 "failures": failures, "historical_attempts": historical_attempts}


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
        "png_renderer": png_renderer_version(),
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


def aggregate(raw: list[dict], errors: list[dict],
              validations: list[dict] | None = None) -> list[dict]:
    validations = validations or []
    groups: dict[tuple, dict[str, list[dict]]] = defaultdict(lambda: {"rustsht": [], "ducc": []})
    for entry in raw:
        key = (entry["case_id"], entry["operation"], entry["threads"])
        groups[key][entry["backend"]].append(entry)
    error_groups: dict[tuple, list[dict]] = defaultdict(list)
    for entry in errors:
        error_groups[(entry["case_id"], entry["operation"], entry["threads"])].append(entry)
    validation_by_group = {
        (item["case_id"], item["threads"]): item for item in validations
    }
    rows = []
    for (case_id, operation, threads), backends in groups.items():
        rust_by_pair = {proc["process_pair"]: proc for proc in backends["rustsht"]}
        ducc_by_pair = {proc["process_pair"]: proc for proc in backends["ducc"]}
        errors_by_pair = {
            item["process_pair"]: item
            for item in error_groups[(case_id, operation, threads)]
        }
        common_pairs = sorted(
            set(rust_by_pair) & set(ducc_by_pair) & set(errors_by_pair)
        )
        if not common_pairs:
            continue
        expected_pairs = set(range(PROCESS_PAIRS))
        pair_coverage_complete = (
            set(rust_by_pair) == expected_pairs
            and set(ducc_by_pair) == expected_pairs
            and set(errors_by_pair) == expected_pairs
        )
        matched_rust = [rust_by_pair[pair] for pair in common_pairs]
        matched_ducc = [ducc_by_pair[pair] for pair in common_pairs]
        rust_measurements = backends["rustsht"]
        ducc_measurements = backends["ducc"]
        rust_values = [value for proc in rust_measurements for value in proc["batch_ms"]]
        ducc_values = [value for proc in ducc_measurements for value in proc["batch_ms"]]
        process_pair_ratios = [
            statistics.median(rust_by_pair[pair]["batch_ms"])
            / statistics.median(ducc_by_pair[pair]["batch_ms"])
            for pair in common_pairs
            if rust_by_pair[pair]["batch_ms"] and ducc_by_pair[pair]["batch_ms"]
        ]
        if not rust_values or not ducc_values or not process_pair_ratios:
            continue
        rs = summarize(rust_values)
        ds = summarize(ducc_values)
        rust_batch_totals = [value for proc in rust_measurements for value in proc["batch_total_ms"]]
        ducc_batch_totals = [value for proc in ducc_measurements for value in proc["batch_total_ms"]]
        batch_totals = rust_batch_totals + ducc_batch_totals
        short_operation_entries = [
            entry for backend in (rust_measurements, ducc_measurements) for entry in backend
            if entry["warmup_ms"] < 100.0
        ]
        target_window_values = [
            elapsed for entry in short_operation_entries for elapsed in entry["batch_total_ms"]
        ]
        target_window_fraction = (
            sum(20.0 <= elapsed <= 100.0 for elapsed in target_window_values) / len(target_window_values)
            if target_window_values else None
        )
        pair_summary = summarize(process_pair_ratios)
        error_values = [errors_by_pair[pair] for pair in common_pairs]
        errors_summary = {}
        for metric in ("absolute_max", "relative_l2", "max_relative_above_1e-12_peak"):
            values = [item[metric] for item in error_values if item.get(metric) is not None]
            errors_summary[metric] = max(values) if values else None
        comparison_accepted = all(
            not random_comparison_failures(item) for item in error_values
        )
        validation = validation_by_group.get((case_id, threads))
        reset_consistent = all(
            proc.get("reset_consistent") is True
            for proc in (*rust_measurements, *ducc_measurements)
        )
        validation_status = saved_validation_acceptance(validation, matched_rust[0])[0]
        if not reset_consistent or not comparison_accepted:
            numerical_status = "rejected"
        elif not pair_coverage_complete:
            numerical_status = "incomplete"
        elif validation_status == "accepted":
            numerical_status = "accepted"
        elif validation_status == "incomplete":
            numerical_status = "incomplete"
        else:
            numerical_status = "rejected"
        coverage_status = (
            "complete"
            if pair_coverage_complete
            else "partial"
        )
        modes = {
            "long_individual" if proc["warmup_ms"] >= 100.0 else "short_calibrated"
            for proc in (*rust_measurements, *ducc_measurements)
        }
        timing_mode = next(iter(modes)) if len(modes) == 1 else "mixed"
        process_counts_qualified = (
            pair_coverage_complete
            and len(rust_measurements) == PROCESS_PAIRS
            and len(ducc_measurements) == PROCESS_PAIRS
            and all(
                len(proc["batch_ms"]) >= (5 if proc["warmup_ms"] >= 100.0 else BATCHES_PER_PROCESS)
                for proc in (*rust_measurements, *ducc_measurements)
            )
        )
        timing_quality_status = (
            "qualified"
            if pair_coverage_complete and process_counts_qualified
            else "insufficient"
        )
        if target_window_fraction is not None:
            duration_window_status = (
                "within_target" if target_window_fraction == 1.0 else "partially_outside_target"
            )
        elif timing_mode == "long_individual":
            duration_window_status = "not_applicable"
        else:
            duration_window_status = "mixed_or_unavailable"
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
            "rust_ducc_ratio": pair_summary["median"],
            "process_pair_ratio_p10": pair_summary["p10"],
            "process_pair_ratio_p90": pair_summary["p90"],
            "excess_ms": rs["median"] - ds["median"],
            **errors_summary,
            "rust_cold_first_ms": statistics.median(proc["first_ms"] for proc in rust_measurements),
            "ducc_cold_first_ms": statistics.median(proc["first_ms"] for proc in ducc_measurements),
            "rust_setup_ms": statistics.median(proc["setup_ms"] for proc in rust_measurements),
            "ducc_setup_ms": statistics.median(proc["setup_ms"] for proc in ducc_measurements),
            "rust_fixture_load_ms": statistics.median(proc["fixture_load_ms"] for proc in rust_measurements),
            "ducc_fixture_load_ms": statistics.median(proc["fixture_load_ms"] for proc in ducc_measurements),
            "rust_arch": rust_measurements[0]["arch"],
            "ducc_arch": ducc_measurements[0]["arch"],
            "rust_batch_count": len(rust_values),
            "ducc_batch_count": len(ducc_values),
            "batch_elapsed_median_ms": statistics.median(batch_totals),
            "batch_elapsed_p10_ms": percentile(batch_totals, 0.10),
            "batch_elapsed_p90_ms": percentile(batch_totals, 0.90),
            "20_100ms_batch_fraction_for_short_ops": target_window_fraction,
            "rust_process_pairs": len(rust_by_pair),
            "ducc_process_pairs": len(ducc_by_pair),
            "completed_process_pairs": len(common_pairs),
            "timing_mode": timing_mode,
            "timing_quality_status": timing_quality_status,
            "duration_window_status": duration_window_status,
            "coverage_status": coverage_status,
            "numerical_status": numerical_status,
            "reset_consistency_status": "accepted" if reset_consistent else "rejected",
            "measurement_status": coverage_status,
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
    return {
        "GL": "Gauss–Legendre",
        "CC": "Clenshaw–Curtis",
        "F1": "Fejér first-rule",
    }.get(grid, grid)


def row_is_plot_eligible(row: dict, grid: str) -> bool:
    if grid != "CC":
        return True
    return (
        row.get("coverage_status") == "complete"
        and row.get("numerical_status") == "accepted"
        and row.get("timing_quality_status") == "qualified"
        and row.get("reset_consistency_status", "accepted") == "accepted"
    )


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
            f'<text x="98" y="61" font-family="sans-serif" font-size="15" fill="#555">f64 · {grid_display_name(grid)} · even-smooth sampling · warmed full transforms{ " · accepted complete cells" if grid == "CC" else ""}</text>'
            f'<text x="630" y="340" text-anchor="middle" font-family="sans-serif" font-size="18" fill="#555">No accepted complete measurements for this selection</text>'
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
        f'<text x="{left}" y="61" font-family="sans-serif" font-size="15" fill="#555">f64 · {grid_display_name(grid)} · even-smooth sampling · warmed full transforms · lower is faster{" · accepted complete cells" if grid == "CC" else ""}</text>',
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
        f'<text x="{left}" y="61" font-family="sans-serif" font-size="15" fill="#555">f64 · {grid_display_name(grid)} · even-smooth sampling · warmed full transforms{" · accepted complete cells" if grid == "CC" else ""}</text>',
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
                    and row_is_plot_eligible(row, grid)
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
                def is_selected_order(row: dict) -> bool:
                    lmax = int(row["lmax"])
                    mmax = int(row["mmax"])
                    if full:
                        return mmax == lmax
                    quarter_mmax = lmax // 4 if grid == "CC" else max(1, lmax // 4)
                    return mmax == quarter_mmax

                grouped[threads] = [
                    {"lmax": int(row["lmax"]), "ratio": float(row["rust_ducc_ratio"])}
                    for row in rows
                    if row["operation"] == operation
                    and int(row["threads"]) == threads
                    and is_selected_order(row)
                    and row["longitude_mode"] == "even_smooth"
                    and row_is_plot_eligible(row, grid)
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
    """Compare accepted, exactly matched CC/GL cells and report every missing cell."""
    cc_path = output_dir / "results.csv"
    if not cc_path.is_file() or not gl_results.is_file():
        return None
    with cc_path.open(newline="") as handle:
        cc_rows = list(csv.DictReader(handle))
    with gl_results.open(newline="") as handle:
        gl_rows = list(csv.DictReader(handle))

    def key(row: dict) -> tuple:
        return (int(row["lmax"]), int(row["mmax"]), int(row["nphi"]),
                row["longitude_mode"], row["operation"], int(row["threads"]))

    def unique_rows(rows: list[dict], label: str) -> dict[tuple, dict]:
        result = {}
        for row in rows:
            row_key = key(row)
            if row_key in result:
                raise ValueError(f"duplicate {label} CC/GL comparison key: {row_key}")
            result[row_key] = row
        return result

    cc_by_key = unique_rows(cc_rows, "CC")
    gl_by_key = unique_rows(gl_rows, "GL")

    def load_result_json(csv_path: pathlib.Path) -> dict:
        json_path = csv_path.with_name("results.json")
        if not json_path.is_file():
            return {}
        return json.loads(json_path.read_text())

    cc_machine = load_result_json(cc_path)
    gl_machine = load_result_json(gl_results)

    def derived_status(row: dict | None, machine: dict) -> tuple[str, str, str, str, str]:
        if row is None:
            return "missing", "missing", "missing", "missing", "missing"
        coverage = row.get("coverage_status", row.get("measurement_status", "missing"))
        if coverage != "complete":
            return coverage, "unknown", "insufficient", "mixed_or_unavailable", "incomplete"
        numeric = row.get("numerical_status")
        group_samples = [
            sample for sample in machine.get("samples", [])
            if sample.get("case_id") == row.get("case_id")
            and sample.get("operation") == row.get("operation")
            and int(sample.get("threads", -1)) == int(row["threads"])
        ]
        expected_pairs = set(range(PROCESS_PAIRS))
        sample_keys = Counter(
            (sample.get("backend"), sample.get("process_pair"))
            for sample in group_samples
        )
        one_sample_per_pair = (
            len(group_samples) == 2 * PROCESS_PAIRS
            and all(sample_keys[(backend, pair)] == 1
                    for backend in ("rustsht", "ducc") for pair in expected_pairs)
        )
        if numeric is None:
            try:
                error = float(row["relative_l2"])
            except (KeyError, TypeError, ValueError):
                error = math.inf
            validation_by_group = {
                (item.get("case_id"), int(item.get("threads", -1))): item
                for item in machine.get("validation", [])
            }
            validation = validation_by_group.get((row.get("case_id"), int(row["threads"])))
            validation_status = saved_validation_acceptance(validation, row)[0]
            if not math.isfinite(error) or error > RANDOM_OUTPUT_RELATIVE_L2_TOLERANCE:
                numeric = "rejected"
            elif validation_status == "accepted":
                numeric = "accepted"
            elif validation_status == "incomplete":
                numeric = "incomplete"
            else:
                numeric = "rejected"
        timing = row.get("timing_quality_status")
        if timing is None:
            pairs = {backend: set() for backend in ("rustsht", "ducc")}
            for sample in group_samples:
                backend = sample.get("backend")
                if backend not in pairs:
                    continue
                pairs[backend].add(sample.get("process_pair"))
            timing = (
                "qualified" if one_sample_per_pair
                and all(pairs[backend] == expected_pairs
                        for backend in ("rustsht", "ducc"))
                and all(len(sample.get("batch_ms", [])) >= (
                    5 if sample.get("warmup_ms", 0.0) >= 100.0 else BATCHES_PER_PROCESS
                ) for sample in group_samples
                    if sample.get("backend") in {"rustsht", "ducc"})
                else "insufficient"
            )
        duration = row.get("duration_window_status")
        if duration is None:
            target_batches = [
                elapsed for sample in group_samples
                if sample.get("warmup_ms", 0.0) < 100.0
                for elapsed in sample.get("batch_total_ms", [])
            ]
            if target_batches:
                duration = (
                    "within_target" if all(20.0 <= value <= 100.0 for value in target_batches)
                    else "partially_outside_target"
                )
            elif group_samples and all(sample.get("warmup_ms", 0.0) >= 100.0
                                       for sample in group_samples):
                duration = "not_applicable"
            else:
                duration = "mixed_or_unavailable"
        reset = row.get("reset_consistency_status")
        if reset is None:
            reset = (
                "accepted" if one_sample_per_pair
                and all(sample.get("reset_consistent") is True for sample in group_samples)
                else "rejected" if any(sample.get("reset_consistent") is False
                                        for sample in group_samples)
                else "incomplete"
            )
        return coverage, numeric, timing, duration, reset

    def comparison_status(cc_row: dict | None, gl_row: dict | None,
                          cc_state: tuple[str, str, str],
                          gl_state: tuple[str, str, str]) -> str:
        if cc_row is None:
            return "missing_cc_measurement"
        if gl_row is None:
            same_geometry = [candidate for candidate in gl_rows
                             if int(candidate["lmax"]) == int(cc_row["lmax"])
                             and int(candidate["mmax"]) == int(cc_row["mmax"])
                             and candidate["longitude_mode"] == cc_row["longitude_mode"]
                             and candidate["operation"] == cc_row["operation"]
                             and int(candidate["threads"]) == int(cc_row["threads"])]
            return "nphi_mismatch" if same_geometry else "missing_gl_measurement"
        for state in (cc_state, gl_state):
            if state[0] != "complete":
                return "incomplete_coverage"
            if state[1] != "accepted":
                return "incomplete_numerics" if state[1] == "incomplete" else "rejected_numerics"
            if state[2] != "qualified":
                return "timing_insufficient"
            if state[3] not in {"within_target", "not_applicable"}:
                return "timing_uncalibrated"
            if state[4] != "accepted":
                return "reset_inconsistent"
        return "accepted"

    def metadata_source(machine: dict, field: str) -> str | None:
        return machine.get("metadata", {}).get(field)

    comparison_records = {}
    series = []
    expected_cc = make_cases(False, dense_sweep=True, grid="CC")
    for operation in ("analysis", "synthesis"):
        for order_label, full in (("full mmax", True), ("quarter mmax", False)):
            for threads in (1, 8):
                points = []
                for case in expected_cc:
                    if case["longitude_mode"] != "even_smooth":
                        continue
                    if full and case["mmax"] != case["lmax"]:
                        continue
                    if not full and case["mmax"] != case["lmax"] // 4:
                        continue
                    comparison_key = (
                        case["lmax"], case["mmax"], case["nphi"],
                        case["longitude_mode"], operation, threads,
                    )
                    cc = cc_by_key.get(comparison_key)
                    gl = gl_by_key.get(comparison_key)
                    cc_state = derived_status(cc, cc_machine)
                    gl_state = derived_status(gl, gl_machine)
                    status = comparison_status(cc, gl, cc_state, gl_state)
                    ratio = None
                    if status == "accepted":
                        ratio = float(cc["rust_ducc_ratio"]) / float(gl["rust_ducc_ratio"])
                        points.append({"lmax": case["lmax"], "value": ratio})
                    record = {
                        "lmax": case["lmax"],
                        "mmax": case["mmax"],
                        "nphi": case["nphi"],
                        "longitude_mode": case["longitude_mode"],
                        "operation": operation,
                        "threads": threads,
                        "principal_orders": order_label,
                        "cc_case_id": cc.get("case_id") if cc else case["case_id"],
                        "gl_case_id": gl.get("case_id") if gl else f"T{case['lmax']}-m{case['mmax']}-even_smooth",
                        "cc_ntheta": int(cc["ntheta"]) if cc else case["ntheta"],
                        "gl_ntheta": int(gl["ntheta"]) if gl else case["lmax"] + 1,
                        "cc_status": "/".join(cc_state),
                        "gl_status": "/".join(gl_state),
                        "status": status,
                        "cc_rust_ducc_ratio": cc.get("rust_ducc_ratio") if cc else None,
                        "gl_rust_ducc_ratio": gl.get("rust_ducc_ratio") if gl else None,
                        "cc_over_gl_ratio": ratio,
                    }
                    if comparison_key in comparison_records:
                        labels = set(comparison_records[comparison_key]["principal_orders"].split("+"))
                        labels.add(order_label)
                        comparison_records[comparison_key]["principal_orders"] = "+".join(sorted(labels))
                    else:
                        comparison_records[comparison_key] = record
                series.append({
                    "name": f"{operation} {order_label} q{threads}",
                    "points": points,
                    "dashed": threads == 8,
                })
    comparisons = [comparison_records[key] for key in sorted(comparison_records)]
    output = output_dir / "cc-versus-gl-matched-ratio.svg"
    plot_log_degree_series(
        output,
        "CC versus GL matched RustSHT/DUCC ratio",
        "Accepted cells only; match lmax, mmax, nphi, operation and q; CC ntheta=lmax+2, GL ntheta=lmax+1",
        series,
        "CC ratio / GL ratio",
    )
    if not output.is_file():
        width, height = 1260, 690
        output.write_text(
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">'
            '<rect width="100%" height="100%" fill="#fff"/>'
            '<text x="100" y="34" font-family="sans-serif" font-size="24" font-weight="bold">CC versus GL matched RustSHT/DUCC ratio</text>'
            '<text x="100" y="380" font-family="sans-serif" font-size="20" fill="#555">No accepted matched cells; see the comparison CSV for missing or rejected statuses.</text>'
            '</svg>\n'
        )
    with (output_dir / "cc-versus-gl-matched-ratio.csv").open("w", newline="") as handle:
        fields = list(comparisons[0]) if comparisons else [
            "lmax", "mmax", "nphi", "longitude_mode", "operation", "threads",
            "principal_orders", "cc_case_id", "gl_case_id", "cc_ntheta", "gl_ntheta", "cc_status",
            "gl_status", "status", "cc_rust_ducc_ratio", "gl_rust_ducc_ratio",
            "cc_over_gl_ratio",
        ]
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(comparisons)
    metadata = {
        "comparison_key": ["lmax", "mmax", "nphi", "longitude_mode", "operation", "threads"],
        "cc_production_source_revision": metadata_source(cc_machine, "production_source_revision"),
        "gl_production_source_revision": metadata_source(gl_machine, "production_source_revision"),
        "cc_rust_binary_sha256": metadata_source(cc_machine, "rust_binary_sha256"),
        "gl_rust_binary_sha256": metadata_source(gl_machine, "rust_binary_sha256"),
        "ducc_source_revision": metadata_source(cc_machine, "ducc_source_revision"),
        "status_counts": dict(Counter(row["status"] for row in comparisons)),
        "note": "Plot contains accepted complete cells only; all expected principal even-smooth cells and statuses are in the CSV.",
    }
    (output_dir / "cc-versus-gl-matched-ratio.json").write_text(
        json.dumps(metadata, indent=2, allow_nan=False) + "\n"
    )
    convert_svg_to_png(output)
    return output


def dense_sweep_measurement_pass(expected_ids: set[str], actual_ids: set[str],
                                 low_even_missing: list[str],
                                 measurement_groups: dict) -> bool:
    """Return manifest, numerical, timing and reset acceptance independent of profiling."""
    expected_groups = {
        f"{case_id}|{operation}|q{threads}"
        for case_id in expected_ids
        for operation in ("analysis", "synthesis")
        for threads in (1, 8)
    }
    return (
        bool(expected_ids)
        and bool(measurement_groups)
        and expected_ids <= actual_ids
        and not low_even_missing
        and set(measurement_groups) == expected_groups
        and all(group["status"] == "complete" for group in measurement_groups.values())
        and all(group["numerical_status"] == "accepted" for group in measurement_groups.values())
        and all(group["timing_quality_status"] == "qualified" for group in measurement_groups.values())
        and all(group["duration_window_status"] in {"within_target", "not_applicable"}
                for group in measurement_groups.values())
        and all(group["reset_consistency_status"] == "accepted" for group in measurement_groups.values())
    )


def dense_sweep_coverage(cases: list[dict], raw: list[dict], errors: list[dict],
                         failures: list[dict], grid: str = "GL",
                         validations: list[dict] | None = None,
                         recurrence_stage_attribution_status: str = "not_measured") -> dict:
    validations = validations or []
    expected = make_cases(False, dense_sweep=True, grid=grid)
    expected_by_id = {case["case_id"]: case for case in expected}
    actual_by_id = {case["case_id"]: case for case in cases}
    expected_ids = set(expected_by_id)
    actual_ids = set(actual_by_id)
    measurement_groups = {}
    validation_by_group = {
        (item["case_id"], item["threads"]): item for item in validations
    }
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
                group_errors = [
                    entry for entry in errors
                    if entry["case_id"] == key[0]
                    and entry["operation"] == operation
                    and entry["threads"] == threads
                ]
                comparison_pairs = {entry["process_pair"] for entry in group_errors}
                expected_pairs = set(range(PROCESS_PAIRS))
                failed_pairs = {
                    entry["process_pair"] for entry in failures
                    if entry.get("case_id") == key[0]
                    and entry.get("operation") == operation
                    and entry.get("threads") == threads
                    and entry.get("failure_class") not in {
                        "numerical_tolerance_failed", "validation_failed",
                        "output_reset_failed",
                    }
                    and entry.get("attempt_status") not in {"recovered", "superseded"}
                }
                complete = (
                    backend_pairs["rustsht"] & backend_pairs["ducc"]
                    & comparison_pairs & expected_pairs
                )
                pair_coverage_complete = (
                    backend_pairs["rustsht"] == expected_pairs
                    and backend_pairs["ducc"] == expected_pairs
                    and comparison_pairs == expected_pairs
                    and complete == expected_pairs
                )
                group_samples = [
                    entry for entry in raw
                    if entry["case_id"] == key[0]
                    and entry["operation"] == operation
                    and entry["threads"] == threads
                ]
                backend_batch_counts = {
                    backend: sum(len(entry["batch_ms"]) for entry in group_samples
                                 if entry["backend"] == backend)
                    for backend in ("rustsht", "ducc")
                }
                reset_flags = [entry.get("reset_consistent") for entry in group_samples]
                if reset_flags and any(flag is not True for flag in reset_flags):
                    reset_consistency_status = "rejected"
                elif (len(group_samples) == 2 * PROCESS_PAIRS
                      and all(flag is True for flag in reset_flags)):
                    reset_consistency_status = "accepted"
                else:
                    reset_consistency_status = "incomplete"
                timing_modes = {
                    "long_individual" if entry["warmup_ms"] >= 100.0 else "short_calibrated"
                    for entry in group_samples
                }
                timing_mode = next(iter(timing_modes)) if len(timing_modes) == 1 else (
                    "mixed" if timing_modes else "missing"
                )
                process_counts_qualified = (
                    pair_coverage_complete
                    and all(
                        len(entry["batch_ms"]) >= (
                            5 if entry["warmup_ms"] >= 100.0 else BATCHES_PER_PROCESS
                        )
                        for entry in group_samples
                    )
                )
                timing_quality_status = (
                    "qualified"
                    if pair_coverage_complete and process_counts_qualified
                    else "insufficient"
                )
                short_batches = [
                    total_ms for entry in group_samples if entry["warmup_ms"] < 100.0
                    for total_ms in entry["batch_total_ms"]
                ]
                if short_batches:
                    in_window = sum(20.0 <= value <= 100.0 for value in short_batches)
                    duration_window_status = (
                        "within_target" if in_window == len(short_batches)
                        else "partially_outside_target"
                    )
                elif group_samples and all(entry["warmup_ms"] >= 100.0 for entry in group_samples):
                    duration_window_status = "not_applicable"
                else:
                    duration_window_status = "mixed_or_unavailable"
                numerical_statuses = [
                    "accepted" if not random_comparison_failures(entry) else "rejected"
                    for entry in group_errors
                ]
                validation = validation_by_group.get((key[0], threads))
                validation_status = saved_validation_acceptance(validation, case)[0]
                if reset_consistency_status == "rejected":
                    numerical_status = "rejected"
                elif reset_consistency_status != "accepted":
                    numerical_status = "incomplete"
                elif not all(status == "accepted" for status in numerical_statuses):
                    numerical_status = "rejected"
                elif (comparison_pairs != expected_pairs
                      or len(group_errors) != PROCESS_PAIRS):
                    numerical_status = "incomplete"
                elif validation_status == "accepted":
                    numerical_status = "accepted"
                elif validation_status == "incomplete":
                    numerical_status = "incomplete"
                else:
                    numerical_status = "rejected"
                measurement_groups[f"{key[0]}|{operation}|q{threads}"] = {
                    "rust_process_pairs": sorted(backend_pairs["rustsht"]),
                    "ducc_process_pairs": sorted(backend_pairs["ducc"]),
                    "compared_process_pairs": sorted(comparison_pairs),
                    "complete_process_pairs": sorted(complete),
                    "failed_process_pairs": sorted(failed_pairs),
                    "rust_batch_count": backend_batch_counts["rustsht"],
                    "ducc_batch_count": backend_batch_counts["ducc"],
                    "timing_mode": timing_mode,
                    "timing_quality_status": timing_quality_status,
                    "duration_window_status": duration_window_status,
                    "reset_consistency_status": reset_consistency_status,
                    "validation_status": validation_status,
                    "numerical_status": numerical_status,
                    "status": (
                        "complete" if pair_coverage_complete
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
        if grid == "CC":
            mmax_values.add(lmax // 4)
        elif lmax > 0:
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
        "unexpected_geometry_count": len(actual_ids - expected_ids),
        "unexpected_measurement_group_count": len(actual_ids - expected_ids) * 4,
        "low_degree_even_smooth_missing": low_even_missing,
        "expected_measurement_group_count": len(expected) * 4,
        "measurement_groups": measurement_groups,
        "measurement_status_counts": {
            status: sum(group["status"] == status for group in measurement_groups.values())
            for status in ("complete", "partial", "failed", "missing")
        },
        "numerical_status_counts": {
            status: sum(group["numerical_status"] == status for group in measurement_groups.values())
            for status in ("accepted", "rejected", "incomplete")
        },
        "validation_status_counts": {
            status: sum(group["validation_status"] == status
                        for group in measurement_groups.values())
            for status in ("accepted", "rejected", "incomplete")
        },
        "timing_quality_status_counts": {
            status: sum(group["timing_quality_status"] == status for group in measurement_groups.values())
            for status in ("qualified", "insufficient")
        },
        "duration_window_status_counts": {
            status: sum(group["duration_window_status"] == status
                        for group in measurement_groups.values())
            for status in ("within_target", "partially_outside_target",
                           "not_applicable", "mixed_or_unavailable")
        },
        "reset_consistency_status_counts": {
            status: sum(group["reset_consistency_status"] == status
                        for group in measurement_groups.values())
            for status in ("accepted", "rejected", "incomplete")
        },
        "failure_count": len(failures),
        "all_requested_measurements_attempted": all(
            group["status"] != "missing" for group in measurement_groups.values()
        ),
        "all_measurements_complete": all(
            group["status"] == "complete" for group in measurement_groups.values()
        ),
        "all_numerics_accepted": all(
            group["numerical_status"] == "accepted" for group in measurement_groups.values()
        ),
        "all_timing_quality_qualified": all(
            group["timing_quality_status"] == "qualified" for group in measurement_groups.values()
        ),
        "whole_transform_pass": (
            expected_ids <= actual_ids
            and not low_even_missing
            and all(group["status"] == "complete" for group in measurement_groups.values())
            and all(group["numerical_status"] == "accepted" for group in measurement_groups.values())
            and all(group["timing_quality_status"] == "qualified" for group in measurement_groups.values())
            and all(group["duration_window_status"] in {"within_target", "not_applicable"}
                    for group in measurement_groups.values())
            and all(group["reset_consistency_status"] == "accepted" for group in measurement_groups.values())
        ),
        "recurrence_stage_attribution_status": recurrence_stage_attribution_status,
        "full_sweep_pass": dense_sweep_measurement_pass(
            expected_ids, actual_ids, low_even_missing, measurement_groups
        ),
        "recurrence_stage_attribution_required_for_pass": False,
        "coverage_pass": (
            expected_ids <= actual_ids and not low_even_missing
        ),
    }


def write_artifacts(output_dir: pathlib.Path, metadata: dict, cases: list[dict], raw: list[dict], analysis: dict) -> None:
    failures = list(analysis.get("failures", []))
    recorded_reset_failures = {
        (item.get("case_id"), item.get("operation"), item.get("threads"),
         item.get("process_pair"), item.get("backend"))
        for item in failures if item.get("failure_class") == "output_reset_failed"
    }
    for sample in raw:
        if sample.get("reset_consistent") is True:
            continue
        key = (sample.get("case_id"), sample.get("operation"), sample.get("threads"),
               sample.get("process_pair"), sample.get("backend"))
        if key in recorded_reset_failures:
            continue
        failures.append({
            "case_id": sample.get("case_id"),
            "lmax": sample.get("lmax"),
            "mmax": sample.get("mmax"),
            "ntheta": sample.get("ntheta"),
            "nphi": sample.get("nphi"),
            "longitude_mode": sample.get("longitude_mode"),
            "operation": sample.get("operation"),
            "threads": sample.get("threads"),
            "process_pair": sample.get("process_pair"),
            "backend": sample.get("backend"),
            "failure_class": (
                "output_reset_failed" if sample.get("reset_consistent") is False
                else "output_reset_flag_missing"
            ),
        })
        recorded_reset_failures.add(key)
    analysis["failures"] = failures
    rows = aggregate(raw, analysis["errors"], analysis.get("validations", []))
    output_dir.mkdir(parents=True, exist_ok=True)
    metadata["runtime_rust_arches"] = sorted({entry["arch"] for entry in raw if entry["backend"] == "rustsht"})
    metadata["runtime_ducc_arches"] = sorted({entry["arch"] for entry in raw if entry["backend"] == "ducc"})
    metadata["runtime_rust_f64_lane_widths"] = sorted({entry["lane_width_f64"] for entry in raw if entry["backend"] == "rustsht"})
    sanitized_cases = [
        {key: value for key, value in case.items() if key not in {"map_path", "alm_path"}}
        for case in cases
    ]
    public_samples = samples_for_result_export(raw, metadata)
    if metadata.get("merged_shards"):
        metadata["aggregate_sample_host_telemetry_exported"] = False
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
        "historical_attempts": portable_evidence(
            strip_local_host_telemetry(analysis.get("historical_attempts", []))
            if metadata.get("merged_shards")
            else analysis.get("historical_attempts", [])
        ),
        "host_summary": analysis["host"],
    }
    write_json_sidecar(output_dir / "results.json", machine)
    result_fields = list(rows[0]) if rows else [
        "case_id", "grid", "lmax", "mmax", "ntheta", "nphi", "longitude_mode",
        "operation", "threads", "rust_warm_ms", "rust_p10_ms", "rust_p90_ms",
        "ducc_warm_ms", "ducc_p10_ms", "ducc_p90_ms", "rust_ducc_ratio",
        "process_pair_ratio_p10", "process_pair_ratio_p90", "excess_ms", "absolute_max",
        "relative_l2", "max_relative_above_1e-12_peak", "rust_batch_count",
        "ducc_batch_count", "completed_process_pairs", "coverage_status",
        "numerical_status", "timing_mode", "timing_quality_status",
        "duration_window_status", "reset_consistency_status",
    ]
    with (output_dir / "results.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=result_fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
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
            cases, raw, analysis["errors"], failures, metadata.get("grid", "GL"),
            analysis.get("validations", []),
            metadata.get("recurrence_stage_attribution_status", "not_measured"),
        )
        (output_dir / "coverage-audit.json").write_text(
            json.dumps(finite_json_value(coverage), indent=2, allow_nan=False) + "\n"
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
                    quarter_mmax = (
                        reduced["lmax"] // 4
                        if grid == "CC"
                        else max(1, reduced["lmax"] // 4)
                    )
                    if (reduced["operation"] != operation or reduced["threads"] != threads
                            or reduced["mmax"] != quarter_mmax):
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
    convert_svg_directory_to_png(output_dir)
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
        (
            "# Focused scalar F1 RustSHT/DUCC performance comparison (2026-10-23)"
            if grid == "F1"
            else "# Full-transform RustSHT/DUCC parity benchmark (2026-10-20)"
        ),
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
        f"Each backend/case/operation/thread-count collected {PROCESS_PAIRS} interleaved independent process pairs with up to three out-of-sample calibration batches and 10 measured repeated-operation batches per process when each operation was below 100 ms. Target was {TARGET_BATCH_MS:g} ms and each measured short-operation batch was checked against the requested 20–100 ms range. Operations above 100 ms were measured individually five times per process. First-call and setup costs are separate from warmed samples. The runner reports independent RustSHT and DUCC batch counts and process-pair median ratios; it does not zip call batches from the independent backends. See `{results_prefix}/results.json` and `.csv` for all cases and samples.",
        "",
        "## Results",
        "",
        "The table reports warmed native calls; ratios below one indicate lower RustSHT latency. Relative L2 uses the DUCC output norm, with near-zero values excluded only from the separately reported max-relative metric. A random-output relative-L2 gate of 5e-12 and separate finite, exact-zero, output-reset, roundtrip, basis and adjoint gates determine numerical acceptance; timing quality and coverage remain separate statuses.",
        "",
        "| lmax | mmax | grid | operation | threads | Rust ms [p10,p90] | DUCC ms [p10,p90] | Rust/DUCC | process-pair median ratio [p10,p90] | excess ms | relative L2 |",
        "|---:|---:|---|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row in chosen:
        lines.append(
            f"| {row['lmax']} | {row['mmax']} | {row.get('grid', grid)} {row['ntheta']}×{row['nphi']} {row['longitude_mode']} | {row['operation']} | {row['threads']} | {row['rust_warm_ms']:.6g} [{row['rust_p10_ms']:.6g},{row['rust_p90_ms']:.6g}] | {row['ducc_warm_ms']:.6g} [{row['ducc_p10_ms']:.6g},{row['ducc_p90_ms']:.6g}] | {row['rust_ducc_ratio']:.4f} | {row['process_pair_ratio_p10']:.4f}–{row['process_pair_ratio_p90']:.4f} | {row['excess_ms']:.6g} | {format_scientific(row['relative_l2'])} |"
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
    faster = sum(row["process_pair_ratio_p90"] < 1.0 for row in rows)
    slower = sum(row["process_pair_ratio_p10"] > 1.0 for row in rows)
    unresolved = len(rows) - faster - slower
    window_fractions = [row["20_100ms_batch_fraction_for_short_ops"] for row in rows
                        if row["20_100ms_batch_fraction_for_short_ops"] is not None]
    short_rows = [row for row in rows if row["timing_mode"] == "short_calibrated"]
    short_batch_counts = [count for row in short_rows
                          for count in (row["rust_batch_count"], row["ducc_batch_count"])]
    relative_errors = [row["relative_l2"] for row in rows if row["relative_l2"] is not None]
    absolute_errors = [row["absolute_max"] for row in rows if row["absolute_max"] is not None]
    lines += [
        "",
        f"Across {len(rows)} warm rows, RustSHT was faster in {faster} where the p10–p90 interval of three process-pair median ratios stayed below 1, slower in {slower} where it stayed above 1, and inconclusive in {unresolved} where the interval crossed 1. The ratio interval is process-level; backend call batches remain independent.",
        "### Cross-bandlimit findings",
        "",
    ]
    if grid == "GL":
        lines += [
            f"For even-smooth full-mmax GL, q1 RustSHT was near DUCC at T1536 (analysis {t1536_a['rust_ducc_ratio']:.2f}×; synthesis {t1536_s['rust_ducc_ratio']:.2f}×). At T2048, analysis rose to {t2048_a1['rust_ducc_ratio']:.2f}× q1 and {t2048_a8['rust_ducc_ratio']:.2f}× q8, while synthesis was {t2048_s1['rust_ducc_ratio']:.2f}× q1 and {t2048_s8['rust_ducc_ratio']:.2f}× q8. At T3072, analysis was {t3072_a1['rust_ducc_ratio']:.2f}× q1 ({t3072_a1['excess_ms']:.1f} ms excess) and {t3072_a8['rust_ducc_ratio']:.2f}× q8 ({t3072_a8['excess_ms']:.1f} ms excess); synthesis was {t3072_s1['rust_ducc_ratio']:.2f}× q1 and {t3072_s8['rust_ducc_ratio']:.2f}× q8.",
            "The large high-bandlimit excess is specific to the complete analysis call in these cases: synthesis remains near DUCC at q1, while q8 synthesis is about 1.5× slower. T1536 q1 is near parity, so the measured analysis gap grows sharply in the next sampled size range. End-to-end timings include FFT and preparation; they do not isolate recurrence as the cause. The next comparison should separate analysis recurrence/accumulation from FFT and output work at the high-bandlimit cases.",
        ]
    elif grid == "CC":
        high_full = [row for row in rows if row["lmax"] == 3072 and row["mmax"] == row["lmax"]
                     and row["longitude_mode"] == "even_smooth"]
        high_summary = "; ".join(
            f"{row['operation']} q{row['threads']} {row['rust_ducc_ratio']:.3f}× ({row['excess_ms']:.3g} ms excess)"
            for row in sorted(high_full, key=lambda item: (item["operation"], item["threads"]))
        )
        lines.append(
            f"At T3072 full mmax, the measured CC Rust/DUCC ratios are: {high_summary or 'no complete comparison row'}. These end-to-end CC timings include its meridional resampling and do not isolate recurrence; stage attribution is recorded separately when available."
        )
    else:
        lines.append(
            "F1 excludes both poles and exact analysis uses the mandatory half-grid meridional phase/resampling. This report covers only the selected F1 cases; it does not imply scalar CC or GL performance."
        )
    lines += [
        f"Short-operation batches were within the requested 20–100 ms window for a median {statistics.median(window_fractions):.1%} of each row's combined independent backend batches; the lowest row fraction was {min(window_fractions):.1%}. There are {len(short_rows)} short-operation rows; per-backend row counts range from {min(short_batch_counts) if short_batch_counts else 0} to {max(short_batch_counts) if short_batch_counts else 0}, with {PROCESS_PAIRS * BATCHES_PER_PROCESS} required for a fully sampled backend. Calibration attempts are recorded separately in each process sample.",
        f"Maximum measured random-output relative L2 error was {format_scientific(max(relative_errors, default=None))}; maximum absolute error was {format_scientific(max(absolute_errors, default=None))}.",
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
    timing_count_histogram = Counter(
        (int(row["rust_batch_count"]), int(row["ducc_batch_count"])) for row in rows
    )
    timing_count_text = "; ".join(
        f"{count} rows have Rust/DUCC batch counts {rust_count}/{ducc_count}"
        for (rust_count, ducc_count), count in sorted(timing_count_histogram.items())
    ) or "no rows have timing batches"
    eligible_rows = [
        row for row in rows
        if row["coverage_status"] == "complete"
        and row["timing_quality_status"] == "qualified"
        and row["numerical_status"] == "accepted"
    ]
    relative_errors = [row["relative_l2"] for row in rows if row["relative_l2"] is not None]
    absolute_errors = [row["absolute_max"] for row in rows if row["absolute_max"] is not None]
    lines = [
        f"# Current-source native RustSHT/DUCC scalar {grid} sweep (2026-10-22)",
        "",
        f"Production Rust source: `{metadata.get('production_source_revision', metadata['rust_source_revision'])}`; benchmark harness variants: `{metadata.get('benchmark_harnesses', [{'revision': metadata.get('benchmark_harness_revision', metadata['rust_source_revision']), 'sha256': metadata.get('benchmark_harness_sha256')}])}`.",
        f"Pinned DUCC 0.42.0 source: `{metadata['ducc_source_revision']}`.",
        f"Host: {metadata['cpu_model']}; selected q1 CPU `{metadata['selected_q1_cpu']}`, q8 CPUs `{','.join(map(str, metadata['selected_q8_cpus']))}`; q8 L3 cache IDs `{metadata['selected_q8_l3_cache_ids']}` (one CCD on this host).",
        f"Toolchains: Rust `{metadata['rustc']}` / {metadata['pulp']} with `{metadata['rust_flags']}`; DUCC `{metadata['cxx']}` with `{metadata['ducc_flags']}`.",
        f"The measured Rust executable source is `{metadata.get('binary_source_revision', metadata.get('production_source_revision'))}` and its binary/build identity is verified: `{metadata.get('binary_provenance_verified')}`. Runner checkout variants: `{metadata.get('source_checkout_variants')}`. Paths changed in a checkout relative to the frozen executable source were not built into the measured binary. Executed Rust Pulp architectures: `{metadata.get('runtime_rust_arches')}`, f64 lane widths `{metadata.get('runtime_rust_f64_lane_widths')}`; DUCC runtime label: `{metadata.get('runtime_ducc_arches')}`.",
        "The exact `rustc -Vv`, DUCC SHT compile command, and strict benchmark-driver compile command are preserved in the result JSON metadata.",
        "",
        "## Scope and method",
        "",
        f"The target dense manifest contains {coverage['expected_case_count']} distinct {grid_name} geometries covering every `lmax=0…256` plus {len([value for value in DENSE_SWEEP_HIGH_LMAX if value > 256])} selected higher degrees through T3072. This dataset contains {len(cases)} geometries, including {coverage['unexpected_geometry_count']} supplementary historical geometries ({coverage['unexpected_measurement_group_count']} extra operation/thread groups): `{coverage['unexpected_case_ids']}`. Required full and floor-quarter `mmax` geometries use even FFT-smooth longitude where they differ; T2/T3 `mmax=1` extras are excluded from the principal reduced-order figures. Selected small, dispatch-boundary and high-resolution cases also include odd-minimum longitude. Each transform uses `{ntheta_rule}`, `phi0=0.23`, the full permitted degree/order triangle and deterministic PCG64 fixtures shared by both native libraries.",
        f"Analysis and synthesis were requested at q1/q8. Short calls target {TARGET_BATCH_MS:g} ms per measured batch, with a 20–100 ms duration window and {PROCESS_PAIRS} interleaved process pairs; long calls use five individually timed observations per process. Rust plan/workspace setup, first-call and warmed timings are separate. The JSON retains independent per-backend batch distributions, numerical comparisons and validation; aggregate CPU-frequency and scheduler diagnostics are reported below. Raw per-process telemetry stays in local shard captures. Failure records are retained in `failures.csv` and JSON.",
        f"Timing-count audit by independent backend: {timing_count_text}. A short backend row requires {PROCESS_PAIRS * BATCHES_PER_PROCESS} measured batches; a long backend row requires {PROCESS_PAIRS * 5} individual calls. Each backend chooses its short-batch or long-call route independently for each process, so the two backends may have different routes for one transform. Actual counts are reported separately for RustSHT and DUCC, with no cross-backend batch zipping or padding.",
        f"The q1/q8 CPU affinity was pinned to the physical core sets shown above. q8 uses the first eight physical cores visible to the process (one CCD on this host). RustSHT and DUCC receive the same fixtures and reusable caller buffers; DUCC's public API performs its own internal setup on each call.",
        "",
        "## Coverage audit",
        "",
        f"The dataset contains {len(coverage['observed_lmax_0_through_256'])}/257 integer degrees in the low range ({257 - len(coverage['observed_lmax_0_through_256'])} absent). The required high-resolution list has {len(coverage['required_high_lmax'])} values; required case-selection check: `{coverage['coverage_pass']}`; missing required cases: {len(coverage['missing_case_ids'])}. Low-degree even-smooth full/reduced omissions: {len(coverage['low_degree_even_smooth_missing'])}. Across {coverage['expected_measurement_group_count']} required operation/thread groups, coverage statuses are `{coverage['measurement_status_counts']}`, numerical statuses are `{coverage['numerical_status_counts']}`, analytic-validation statuses are `{coverage['validation_status_counts']}`, timing-quality statuses are `{coverage['timing_quality_status_counts']}`, target-window statuses are `{coverage['duration_window_status_counts']}`, and reset statuses are `{coverage['reset_consistency_status_counts']}`. Whole-transform pass: `{coverage['whole_transform_pass']}`; recurrence-stage attribution: `{coverage['recurrence_stage_attribution_status']}`; full-sweep status: `{coverage['full_sweep_pass']}` (manifest, complete numerical acceptance, qualified timing windows and resets; stage profiling is reported independently). Incomplete, rejected and timing-insufficient groups remain visible in the coverage audit.",
        "",
        "## Warmed whole-transform summary",
        "",
        "Ratios below 1 mean RustSHT was faster. Each table cell reports median ratio, geometric mean ratio, and case count using only rows with complete process-pair coverage, qualified timing counts, and accepted numerical validation. Odd-minimum cases are excluded from this even-smooth summary; all omitted status counts remain in the coverage audit.",
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
                        if row in eligible_rows
                        and lo <= row["lmax"] <= hi
                        and row["operation"] == operation
                        and row["threads"] == threads
                        and (
                            row["mmax"] == row["lmax"]
                            if is_full
                            else row["mmax"] == (
                                row["lmax"] // 4
                                if grid == "CC"
                                else max(1, row["lmax"] // 4)
                            )
                        )
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
    if eligible_rows:
        throughput_rows = [row for row in eligible_rows if row["lmax"] > 0]
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
                    f"{row['rust_warm_ms']:.6g} | {row['ducc_warm_ms']:.6g} | {row['rust_ducc_ratio']:.3f} | {format_scientific(row['relative_l2'])} |"
                )
    lines += ["", "### Longitude-shape sensitivity", ""]
    shape_summary = []
    for operation in ("analysis", "synthesis"):
        for threads in (1, 8):
            by_key = {
                (row["lmax"], row["mmax"], row["operation"], row["threads"], row["longitude_mode"]): row
                for row in eligible_rows
            }
            pairs = []
            for row in eligible_rows:
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
                        [row for row in eligible_rows if row["lmax"] > 0
                         and row["operation"] == operation
                         and row["threads"] == threads
                         and (
                             row["mmax"] == row["lmax"]
                             if is_full
                             else row["mmax"] == (
                                 row["lmax"] // 4
                                 if grid == "CC"
                                 else max(1, row["lmax"] // 4)
                             )
                         )
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
        max_l2 = max(relative_errors, default=None)
        max_abs = max(absolute_errors, default=None)
        lines.append(
            f"Maximum random-output relative L2 error: `{format_scientific(max_l2)}`; "
            f"maximum absolute error: `{format_scientific(max_abs)}`."
        )
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
        f"There were {len(failures)} recorded failure/warning events, including recovered or superseded attempts when applicable. Inspect `{result_prefix}/failures.csv` and the JSON `failures` array; unsupported cases are labeled separately from worker and numerical-validation errors.",
        "",
        "## Principal figures",
        "",
        f"- [Analysis, full mmax](./{result_prefix}/ratio-analysis-full-mmax.svg)",
        f"- [Analysis, full mmax PNG](./{result_prefix}/ratio-analysis-full-mmax.png)",
        f"- [Analysis, reduced mmax](./{result_prefix}/ratio-analysis-reduced-mmax.svg)",
        f"- [Analysis, reduced mmax PNG](./{result_prefix}/ratio-analysis-reduced-mmax.png)",
        f"- [Synthesis, full mmax](./{result_prefix}/ratio-synthesis-full-mmax.svg)",
        f"- [Synthesis, full mmax PNG](./{result_prefix}/ratio-synthesis-full-mmax.png)",
        f"- [Synthesis, reduced mmax](./{result_prefix}/ratio-synthesis-reduced-mmax.svg)",
        f"- [Synthesis, reduced mmax PNG](./{result_prefix}/ratio-synthesis-reduced-mmax.png)",
        f"- [Absolute analysis latency](./{result_prefix}/runtime-analysis.svg)",
        f"- [Absolute analysis latency PNG](./{result_prefix}/runtime-analysis.png)",
        f"- [Absolute synthesis latency](./{result_prefix}/runtime-synthesis.svg)",
        f"- [Absolute synthesis latency PNG](./{result_prefix}/runtime-synthesis.png)",
        f"- [Matched CC/GL ratio](./{result_prefix}/cc-versus-gl-matched-ratio.svg)",
        f"- [Matched CC/GL ratio PNG](./{result_prefix}/cc-versus-gl-matched-ratio.png)",
        f"- [CC/GL matched-cell statuses](./{result_prefix}/cc-versus-gl-matched-ratio.csv)",
        *( [f"- [Precomputed table versus recurrence](./{result_prefix}/legendre-table-vs-recurrence.svg)",
            f"- [Paired FFT versus independent rows](./{result_prefix}/paired-fft-benefit.svg)"]
           if grid == "GL" else
           [f"- [Matched CC/GL comparison](./{result_prefix}/cc-versus-gl-matched-ratio.svg)"] ),
        "",
        f"The four principal ratio figures and two absolute-latency figures contain accepted measured even-smooth cells; q1 and q8 are separate curves and points are not smoothed. Incomplete, numerically rejected, timing-insufficient and unmatched CC/GL cells are listed in `coverage-audit.json` and `cc-versus-gl-matched-ratio.csv`; the ratio chart omits them. Odd-minimum cases remain in the data and shape summary. The CC and GL datasets retain separate source provenance. The exact CC fixtures use `ntheta=lmax+2` to include both poles, while the GL controls use `ntheta=lmax+1`.",
        "",
        "## Reproduction and attribution",
        "",
        f"The machine-readable artifacts are [`results.csv`](./{result_prefix}/results.csv), [`results.json`](./{result_prefix}/results.json), and [`coverage-audit.json`](./{result_prefix}/coverage-audit.json). Rust plan setup/first-call fields, independent backend batch distributions, process-pair ratios, per-case numerical errors, host scheduling/frequency diagnostics, and every requested geometry are retained.",
        "",
        "```sh",
        f"# Compatible harness revisions: {metadata.get('benchmark_harnesses', [{'revision': metadata.get('benchmark_harness_revision', metadata['rust_source_revision']), 'sha256': metadata.get('benchmark_harness_sha256')}])}",
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
                 report_path: pathlib.Path, dense_sweep: bool,
                 recurrence_stage_attribution: pathlib.Path | None = None) -> None:
    shards = [json.loads(path.read_text()) for path in shard_paths]
    if not shards:
        raise ValueError("no shard results supplied")
    metadata = dict(shards[0]["metadata"])
    invariant_fields = (
        "production_source_revision", "ducc_source_revision", "grid",
        "ducc_version", "rustc", "cargo", "cxx", "rust_flags", "ducc_flags",
        "cpu_model", "selected_q1_cpu", "selected_q8_cpus", "selected_q8_topology",
        "selected_q8_l3_cache_ids", "process_pairs", "batches_per_process",
        "target_batch_ms", "phi0", "dense_sweep", "sweep_id",
        "ducc_sht_compile_command", "ducc_driver_compile_command",
        "rust_binary_sha256", "ducc_binary_sha256", "rust_build_command",
        "ducc_cmake_build_command",
        "retain_local_host_telemetry",
    )
    def shard_protocol(shard_metadata: dict) -> dict:
        saved = shard_metadata.get("benchmark_execution_protocol")
        if saved is None:
            return execution_protocol(shard_metadata)
        saved_hash = shard_metadata.get("benchmark_execution_protocol_sha256")
        actual_hash = hashlib.sha256(json.dumps(saved, sort_keys=True).encode()).hexdigest()
        if saved_hash is not None and saved_hash != actual_hash:
            raise ValueError("shard benchmark execution protocol hash is invalid")
        return saved

    base_protocol = shard_protocol(metadata)
    base_binary_source = metadata.get(
        "binary_source_revision", metadata.get("production_source_revision")
    )
    protocol_variants = [base_protocol]
    for index, shard in enumerate(shards[1:], 2):
        for field in invariant_fields:
            if shard["metadata"].get(field) != metadata.get(field):
                raise ValueError(f"shard {index} metadata mismatch for {field}")
        shard_binary_source = shard["metadata"].get(
            "binary_source_revision", shard["metadata"].get("production_source_revision")
        )
        if shard_binary_source != base_binary_source:
            raise ValueError(f"shard {index} metadata mismatch for binary_source_revision")
        protocol = shard_protocol(shard["metadata"])
        protocol_variants.append(protocol)
        if not compatible_metric_protocols([base_protocol, protocol]):
            raise ValueError(f"shard {index} benchmark execution protocol mismatch")
    metric_contracts = sorted({
        protocol.get("numerical_metric_contract") for protocol in protocol_variants
    })
    if not metric_contracts or None in metric_contracts:
        raise ValueError("shards must declare a recognized numerical metric contract")
    cases: list[dict] = []
    raw: list[dict] = []
    errors: list[dict] = []
    validations: list[dict] = []
    failures: list[dict] = []
    historical_attempts: list[dict] = []
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
        shard_contract = shard_protocol(shard["metadata"])["numerical_metric_contract"]
        harness_revision = shard["metadata"].get(
            "benchmark_harness_revision", shard["metadata"].get("rust_source_revision")
        )
        errors.extend({
            **item,
            "numerical_metric_contract": shard_contract,
            "source_harness_revision": harness_revision,
        } for item in shard["numerics"])
        validations.extend({
            **item,
            "numerical_metric_contract": shard_contract,
            "source_harness_revision": harness_revision,
        } for item in shard["validation"])
        failures.extend(shard.get("failures", []))
        historical_attempts.extend(
            {"source_shard": path.parent.name, **attempt}
            for attempt in shard.get("historical_attempts", [])
        )
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
    harnesses = {
        (shard["metadata"].get("benchmark_harness_revision", shard["metadata"].get("rust_source_revision")),
         shard["metadata"].get("benchmark_harness_sha256"))
        for shard in shards
    }
    metadata["benchmark_harnesses"] = [
        {"revision": revision, "sha256": digest}
        for revision, digest in sorted(harnesses, key=lambda item: (str(item[0]), str(item[1])))
    ]
    metadata["rust_source_revisions"] = sorted({
        shard["metadata"].get("rust_source_revision") for shard in shards
    })
    source_checkouts = {
        (
            shard["metadata"].get("rust_source_revision"),
            shard["metadata"].get("rust_source_worktree_clean"),
            shard["metadata"].get("benchmark_harness_revision"),
            shard["metadata"].get("benchmark_harness_sha256"),
            json.dumps(
                shard["metadata"].get("source_checkout_changes_since_binary_source_revision"),
                sort_keys=True,
            ),
        )
        for shard in shards
    }
    metadata["source_checkout_variants"] = [
        {
            "rust_source_revision": revision,
            "rust_source_worktree_clean": worktree_clean,
            "benchmark_harness_revision": harness_revision,
            "benchmark_harness_sha256": harness_sha256,
            "source_checkout_changes_since_binary_source_revision": json.loads(source_delta),
        }
        for revision, worktree_clean, harness_revision, harness_sha256, source_delta
        in sorted(source_checkouts, key=lambda item: (str(item[0]), str(item[2]), str(item[3])))
    ]
    build_records = {
        json.dumps(shard["metadata"].get("binary_build_record"), sort_keys=True)
        for shard in shards if shard["metadata"].get("binary_build_record")
    }
    metadata["binary_build_records"] = [json.loads(record) for record in sorted(build_records)]
    metadata["binary_source_revision"] = base_binary_source
    metadata["numerical_metric_contracts"] = metric_contracts
    metadata["numerical_metric_contract"] = (
        metric_contracts[0] if len(metric_contracts) == 1 else None
    )
    metadata["binary_provenance_verified"] = bool(build_records) and all(
        shard["metadata"].get("rust_binary_sha256") == metadata["rust_binary_sha256"]
        and shard["metadata"].get("ducc_binary_sha256") == metadata["ducc_binary_sha256"]
        and shard["metadata"].get(
            "binary_source_revision", shard["metadata"].get("production_source_revision")
        ) == base_binary_source
        for shard in shards
    )
    canonical_protocols = {
        json.dumps(protocol, sort_keys=True): protocol for protocol in protocol_variants
    }
    metadata["benchmark_execution_protocol_variants"] = [
        {
            "protocol": protocol,
            "sha256": hashlib.sha256(encoded.encode()).hexdigest(),
        }
        for encoded, protocol in sorted(canonical_protocols.items())
    ]
    if len(canonical_protocols) == 1:
        encoded, protocol = next(iter(canonical_protocols.items()))
        metadata["benchmark_execution_protocol"] = protocol
        metadata["benchmark_execution_protocol_sha256"] = hashlib.sha256(
            encoded.encode()
        ).hexdigest()
    else:
        metadata.pop("benchmark_execution_protocol", None)
        metadata.pop("benchmark_execution_protocol_sha256", None)
    metadata["recurrence_stage_attribution_status"] = "not_measured"
    if recurrence_stage_attribution is not None:
        attribution = json.loads(recurrence_stage_attribution.read_text())
        attribution_status = attribution.get("status", "incomplete")
        if attribution_status not in {"measured", "incomplete", "failed"}:
            raise ValueError(f"unsupported recurrence-stage attribution status: {attribution_status}")
        metadata["recurrence_stage_attribution_status"] = attribution_status
        output_dir.mkdir(parents=True, exist_ok=True)
        attribution_destination = output_dir / "recurrence-stage-attribution.json"
        attribution_destination.write_text(
            json.dumps(finite_json_value(attribution), indent=2, allow_nan=False) + "\n"
        )
        metadata["recurrence_stage_attribution_sha256"] = hashlib.sha256(
            attribution_destination.read_bytes()
        ).hexdigest()
    else:
        metadata["recurrence_stage_attribution_sha256"] = None
    metadata["invocation"] = "merged shard results"
    diagnostics = [sample["host_diagnostics"] for sample in raw if "host_diagnostics" in sample]
    analysis = {
        "errors": errors,
        "validations": validations,
        "failures": failures,
        "historical_attempts": historical_attempts,
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
        coverage = dense_sweep_coverage(
            cases, raw, errors, failures, metadata.get("grid", "GL"), validations,
            metadata.get("recurrence_stage_attribution_status", "not_measured"),
        )
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
    parser.add_argument("--grid", choices=("GL", "CC", "F1"), default="GL",
                        help="native scalar grid; exact CC uses lmax+2 latitudes; GL/F1 use lmax+1")
    parser.add_argument("--plot-only", action="store_true", help="regenerate ratio and runtime SVGs from existing results.csv; no benchmark or DUCC checkout needed")
    parser.add_argument("--merge-shards", type=pathlib.Path, nargs="+",
                        help="merge completed native benchmark shard results.json files")
    parser.add_argument("--recurrence-stage-attribution", type=pathlib.Path,
                        help="attach a separately measured CC recurrence-stage attribution JSON during shard merge")
    parser.add_argument("--dense-sweep", action="store_true",
                        help="select every lmax 0..256 plus representative higher degrees")
    parser.add_argument("--production-source-revision",
                        help="frozen Rust production source commit represented by this benchmark")
    parser.add_argument("--binary-build-record", type=pathlib.Path,
                        help="saved native results.json whose source/build/binary identity must match when --skip-build is used")
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
    parser.add_argument("--checkpoint-dir", type=pathlib.Path,
                        help="atomic per-process-pair checkpoint directory (default: RESULTS_DIR/.checkpoints)")
    parser.add_argument("--resume", action="store_true",
                        help="resume incomplete process pairs after exact checkpoint-identity validation")
    parser.add_argument("--report-path", type=pathlib.Path,
                        help="report path; ignored with --no-report")
    parser.add_argument("--no-report", action="store_true",
                        help="write machine-readable artifacts without replacing a report")
    parser.add_argument("--retain-local-host-telemetry", action="store_true",
                        help="retain raw per-process CPU-frequency/scheduling samples in this local shard (do not publish them)")
    args = parser.parse_args()
    default_results_dir = {
        "GL": DATA_DIR,
        "CC": CC_DATA_DIR,
        "F1": F1_DATA_DIR,
    }[args.grid]
    default_report_path = {
        "GL": REPORT_PATH,
        "CC": CC_REPORT_PATH,
        "F1": F1_REPORT_PATH,
    }[args.grid]
    args.results_dir = (args.results_dir or default_results_dir).resolve()
    args.report_path = (args.report_path or default_report_path).resolve()
    if args.merge_shards:
        if args.grid == "F1":
            parser.error("F1 focused results do not use the GL/CC shard-merge contract")
        merge_shards(
            [path.resolve() for path in args.merge_shards], args.results_dir,
            args.report_path, args.dense_sweep,
            args.recurrence_stage_attribution.resolve()
            if args.recurrence_stage_attribution else None,
        )
        if args.dense_sweep:
            with (args.results_dir / "coverage-audit.json").open() as handle:
                coverage = json.load(handle)
            print(
                f"MERGED {len(args.merge_shards)} shards; case selection pass="
                f"{coverage['coverage_pass']}; whole-transform pass="
                f"{coverage['whole_transform_pass']}; stage attribution="
                f"{coverage['recurrence_stage_attribution_status']}; full sweep="
                f"{coverage['full_sweep_pass']}; measurements="
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
        convert_svg_directory_to_png(args.results_dir)
        print(
            f"Regenerated four ratio, two runtime and {len(auxiliary)} auxiliary figures "
            f"from {len(rows)} committed rows"
        )
        return
    if args.grid == "F1" and args.dense_sweep:
        parser.error("the dense-sweep manifest and report are defined for GL/CC, not F1")
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
    if not args.skip_build and args.production_source_revision \
            and args.production_source_revision != metadata["rust_source_revision"]:
        raise ValueError(
            "a newly built Rust worker must use the checked-out Rust source revision "
            "as --production-source-revision"
        )
    metadata.update({
        "production_source_revision": args.production_source_revision or (
            None if args.skip_build else metadata["rust_source_revision"]
        ),
        "numerical_metric_contract": ERROR_METRICS_CONTRACT,
        "benchmark_harness_revision": metadata["rust_source_revision"],
        "benchmark_harness_sha256": sha256_file(pathlib.Path(__file__).resolve()),
        "benchmark_invocation": shlex.join([sys.executable, *sys.argv]),
        "invocation": shlex.join([sys.executable, *sys.argv]),
        "dense_sweep": args.dense_sweep,
        "recurrence_stage_attribution_status": "not_measured",
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
    if args.skip_build and args.binary_build_record:
        build_record = validate_binary_build_record(
            args.binary_build_record.resolve(), metadata, args.production_source_revision
        )
        metadata["binary_build_record"] = build_record
        metadata["binary_source_revision"] = build_record["binary_source_revision"]
        metadata["production_source_revision"] = (
            args.production_source_revision or build_record["binary_source_revision"]
        )
        metadata["binary_provenance_verified"] = True
    elif args.skip_build:
        if args.binary_build_record:
            raise ValueError("--binary-build-record requires --skip-build")
        metadata["binary_build_record"] = None
        metadata["binary_source_revision"] = None
        metadata["binary_provenance_verified"] = False
    else:
        if args.binary_build_record:
            raise ValueError("--binary-build-record is only valid with --skip-build")
        metadata["binary_build_record"] = None
        metadata["binary_source_revision"] = metadata["rust_source_revision"]
        metadata["binary_provenance_verified"] = True
    production_revision = metadata["production_source_revision"]
    binary_revision = metadata["binary_source_revision"]
    if (isinstance(production_revision, str) and isinstance(binary_revision, str)
            and re.fullmatch(r"[0-9a-fA-F]{40}", production_revision)
            and re.fullmatch(r"[0-9a-fA-F]{40}", binary_revision)):
        metadata["production_code_changes_since_baseline"] = subprocess.run(
            ["git", "diff", "--name-only", f"{production_revision}..{binary_revision}",
             "--", "src", "Cargo.toml", "Cargo.lock"],
            cwd=ROOT, text=True, capture_output=True, check=True,
        ).stdout.splitlines()
        metadata["source_checkout_changes_since_binary_source_revision"] = subprocess.run(
            ["git", "diff", "--name-only", f"{binary_revision}..{metadata['rust_source_revision']}",
             "--", "src", "Cargo.toml", "Cargo.lock"],
            cwd=ROOT, text=True, capture_output=True, check=True,
        ).stdout.splitlines()
        metadata["production_code_matches_baseline"] = (
            binary_revision == production_revision and metadata["binary_provenance_verified"]
        )
    else:
        metadata["production_code_changes_since_baseline"] = None
        metadata["source_checkout_changes_since_binary_source_revision"] = None
        metadata["production_code_matches_baseline"] = False
    metadata["benchmark_execution_protocol"] = portable_evidence(execution_protocol(metadata))
    metadata["benchmark_execution_protocol_sha256"] = hashlib.sha256(
        json.dumps(metadata["benchmark_execution_protocol"], sort_keys=True).encode()
    ).hexdigest()
    print(f"BUILD rust={binaries['rustsht']} DUCC={binaries['ducc']}", flush=True)
    print(f"ENV cpu={metadata['cpu_model']} x86={metadata['cpu_features']} affinity=q1/q8 physical cores", flush=True)
    print(f"DISPATCH Rust=Pulp V3 selected by worker; DUCC=fixed x86-64-v3", flush=True)
    operations_to_run = args.operation or ["analysis", "synthesis"]
    threads_to_run = args.threads or [1, 8]
    if args.smoke and (args.resume or args.checkpoint_dir):
        parser.error("checkpointing is disabled for smoke runs")
    checkpoint_dir = None
    checkpoint_run_identity = None
    if not args.smoke:
        checkpoint_dir = (args.checkpoint_dir or (args.results_dir / ".checkpoints")).resolve()
        checkpoint_run_identity = checkpoint_identity(
            metadata, cases, operations_to_run, threads_to_run
        )
        validate_checkpoint_directory(checkpoint_dir, checkpoint_run_identity, args.resume)
        print(f"CHECKPOINTS {checkpoint_dir} resume={args.resume}", flush=True)
    raw, aux = run_matrix(
        cases, binaries, args.smoke, args.threads, args.operation,
        checkpoint_dir=checkpoint_dir,
        checkpoint_identity=checkpoint_run_identity,
    )
    if args.smoke:
        rows = aggregate(raw, aux["errors"], aux.get("validations", []))
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
    rows = aggregate(raw, aux["errors"], aux.get("validations", []))
    write_artifacts(output_dir, metadata, cases, raw, aux)
    if not args.no_report:
        if args.dense_sweep:
            coverage = dense_sweep_coverage(
                cases, raw, aux["errors"], aux.get("failures", []), args.grid,
                aux.get("validations", []),
                metadata.get("recurrence_stage_attribution_status", "not_measured"),
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
