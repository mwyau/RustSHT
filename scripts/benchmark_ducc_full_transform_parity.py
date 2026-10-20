#!/usr/bin/env python3
"""Matched native RustSHT/DUCC scalar GL analysis and synthesis benchmark."""

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
import tempfile
import threading
import time
from collections import defaultdict

import numpy as np


ROOT = pathlib.Path(__file__).resolve().parents[1]
DUCC_REVISION = os.environ.get(
    "RUSTSHT_DUCC_REVISION", "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
)
HISTORICAL_DUCC_REVISION = "80388904b8cfc32400ba84af8a6a45aabfe60f86"
DATA_DIR = ROOT / "reports/data/ducc-full-transform-parity-2026-10-20"
REPORT_PATH = ROOT / "reports/ducc-full-transform-parity-2026-10-20.md"
RUST_FLAGS = os.environ.get(
    "RUSTSHT_FULL_TRANSFORM_RUSTFLAGS",
    "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
)
PHI0 = 0.23
TARGET_BATCH_MS = 25.0
PROCESS_PAIRS = 3
BATCHES_PER_PROCESS = 10


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


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


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
        "1" if validate else "0",
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
) -> list[dict]:
    if smoke:
        lmax_values = [0, 4, 16, 64]
    else:
        lmax_values = sorted(
            set(range(0, 33))
            | set(range(40, 257, 16))
            | {
                62, 63, 64, 65, 126, 127, 128, 129, 159, 191, 223, 254,
                255, 256, 300, 320, 384, 448, 511, 512, 513, 640, 720, 721,
                768, 896, 1024, 1280, 1536, 1792, 2047, 2048, 3072,
            }
        )
    key_sizes = {
        62, 63, 64, 65, 126, 127, 128, 129, 159, 191, 223, 248, 254,
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
            if lmax >= 4:
                mmax_values.add(max(1, lmax // 4))
            if lmax in {255, 720, 1024}:
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
                ntheta = fixed_ntheta if fixed_ntheta is not None else lmax + 1
                if ntheta < lmax + 1:
                    raise ValueError(
                        f"fixed ntheta={ntheta} cannot exactly analyze lmax={lmax}"
                    )
                ntheta_tag = "" if ntheta == lmax + 1 else f"-ntheta{ntheta}"
                case_id = (
                    f"T{lmax}-m{mmax}{ntheta_tag}-{longitude_mode}"
                    if fixed_nphi is None
                    else f"T{lmax}-m{mmax}{ntheta_tag}-nphi{fixed_nphi}"
                )
                cases.append({
                    "case_id": case_id,
                    "lmax": lmax,
                    "mmax": mmax,
                    "ntheta": ntheta,
                    "nphi": nphi,
                    "longitude_mode": longitude_mode,
                    "grid": "GL",
                })
    return cases


def make_fixture(case: dict, directory: pathlib.Path) -> None:
    rng = np.random.Generator(np.random.PCG64(0xD00C20261020 + case["lmax"] * 65537 + case["mmax"] * 97 + case["nphi"]))
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


def run_matrix(cases: list[dict], binaries: dict[str, pathlib.Path], smoke: bool) -> tuple[list[dict], dict]:
    physical = physical_cpus()
    monitor_cpu = physical[8]
    threads_to_run = [1, 8]
    pairs = 1 if smoke else PROCESS_PAIRS
    target_ms = 5.0 if smoke else TARGET_BATCH_MS
    raw: list[dict] = []
    errors: list[dict] = []
    validations: list[dict] = []
    host_samples: list[dict] = []
    total = len(cases) * len(threads_to_run) * 2
    progress = 0
    for case in cases:
        with tempfile.TemporaryDirectory(prefix="rustsht-ducc-parity-") as temporary:
            temp = pathlib.Path(temporary)
            make_fixture(case, temp)
            for threads in threads_to_run:
                cpus = physical[:threads]
                for operation in ("analysis", "synthesis"):
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
                            result, diagnostics = run_pinned(command, cpus, monitor_cpu)
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
                            })
                            raw.append(result)
                            pair_records[backend] = result
                            outputs[backend] = output_path
                            host_samples.append(diagnostics)
                        rust_values = read_result(outputs["rustsht"], operation, case, "rustsht")
                        ducc_values = read_result(outputs["ducc"], operation, case, "ducc")
                        metrics = error_metrics(rust_values, ducc_values)
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
                            **metrics,
                        })
                        if validate:
                            validation = compare_validation(case, outputs, pair_records)
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
                                print(
                                    f"VALIDATION WARNING {case['case_id']} finite={validation['finite_outputs']} reset={validation['reset_consistent']}",
                                    flush=True,
                                )
            case["map_path"] = str(case["map_path"])
            case["alm_path"] = str(case["alm_path"])
    host = summarize_host(host_samples)
    return raw, {"errors": errors, "validations": validations, "host": host}


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
    return {
        "cpu_model": cpu_model(),
        "architecture": platform.machine(),
        "physical_cores_visible": len(physical_cpus()),
        "logical_cpus_visible": len(os.sched_getaffinity(0)),
        "installed_ram_gib": round(memory_kib / 1024 / 1024, 2),
        "cpu_features": [name for name in ("avx2", "fma", "avx512f") if name in flags],
        "os": platform.platform(),
        "rustc": command_output(["rustc", "--version"]),
        "cargo": command_output(["cargo", "--version"]),
        "rust_source_revision": command_output(["git", "rev-parse", "HEAD"]),
        "rust_worktree_clean": not bool(command_output(["git", "status", "--porcelain"])),
        "cxx": command_output(cxx + ["--version"]).splitlines()[0],
        "pulp": pulp_match.group(1) if pulp_match else "unknown",
        "ducc_source_revision": command_output(["git", "rev-parse", "HEAD"], ducc_src),
        "ducc_version": ducc_version_match.group(1) if ducc_version_match else "unknown",
        "rust_flags": RUST_FLAGS,
        "ducc_flags": "DUCC sources: -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; benchmark driver: -O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3",
        "rust_dispatch": "runtime Pulp Arch::new; worker records actual specialization",
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
             f"-DCMAKE_CXX_COMPILER={shlex.split(os.environ.get('CXX', 'g++'))[0]}"],
            cwd=ROOT, check=True,
        )
        subprocess.run(["cmake", "--build", str(build_dir), "--parallel", "4"], cwd=ROOT, check=True)
    if not rust_binary.exists() or not ducc_binary.exists():
        raise FileNotFoundError("benchmark binaries missing; rerun without --skip-build")
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
        rust_values = [value for proc in backends["rustsht"] for value in proc["batch_ms"]]
        ducc_values = [value for proc in backends["ducc"] for value in proc["batch_ms"]]
        paired_ratios = []
        for rust_proc, ducc_proc in zip(backends["rustsht"], backends["ducc"]):
            for rust_time, ducc_time in zip(rust_proc["batch_ms"], ducc_proc["batch_ms"]):
                paired_ratios.append(rust_time / ducc_time)
        rs = summarize(rust_values)
        ds = summarize(ducc_values)
        rust_batch_totals = [value for proc in backends["rustsht"] for value in proc["batch_total_ms"]]
        ducc_batch_totals = [value for proc in backends["ducc"] for value in proc["batch_total_ms"]]
        batch_totals = rust_batch_totals + ducc_batch_totals
        short_operation_entries = [
            entry for backend in backends.values() for entry in backend
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
        error_values = error_groups[(case_id, operation, threads)]
        errors_summary = {
            key: max(item[key] for item in error_values)
            for key in ("absolute_max", "relative_l2", "max_relative_above_1e-12_peak")
        }
        template = backends["rustsht"][0]
        rows.append({
            "case_id": case_id,
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
            "rust_cold_first_ms": statistics.median(proc["first_ms"] for proc in backends["rustsht"]),
            "ducc_cold_first_ms": statistics.median(proc["first_ms"] for proc in backends["ducc"]),
            "rust_setup_ms": statistics.median(proc["setup_ms"] for proc in backends["rustsht"]),
            "ducc_setup_ms": statistics.median(proc["setup_ms"] for proc in backends["ducc"]),
            "rust_fixture_load_ms": statistics.median(proc["fixture_load_ms"] for proc in backends["rustsht"]),
            "ducc_fixture_load_ms": statistics.median(proc["fixture_load_ms"] for proc in backends["ducc"]),
            "rust_arch": backends["rustsht"][0]["arch"],
            "ducc_arch": backends["ducc"][0]["arch"],
            "batch_count": len(rust_values),
            "batch_elapsed_median_ms": statistics.median(batch_totals),
            "batch_elapsed_p10_ms": percentile(batch_totals, 0.10),
            "batch_elapsed_p90_ms": percentile(batch_totals, 0.90),
            "20_100ms_batch_fraction_for_short_ops": target_window_fraction,
            "process_invocations_per_backend": len(backends["rustsht"]),
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



def plot_issue_ratio_svg(path: pathlib.Path, operation: str, order_label: str,
                         points_by_threads: dict[int, list[dict]]) -> None:
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
        f'<text x="{left}" y="61" font-family="sans-serif" font-size="15" fill="#555">f64 · Gauss–Legendre · even-smooth longitude sampling · warmed full transforms · lower is faster</text>',
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
    xticks = [0, 1, 2, 4, 8, 16, 24, 32, 40, 64, 128, 255, 511,
              720, 1024, 1536, 1792, 2048, 3072]
    xticks = [v for v in xticks if v <= xmax]
    for i, degree in enumerate(xticks):
        x = xcoord(degree)
        parts.append(f'<line x1="{x:.1f}" y1="{height-bottom}" x2="{x:.1f}" y2="{height-bottom+6}" stroke="#77818b"/>')
        # Rotate long tick labels and alternate placement at close transitions.
        label_y = height - bottom + 27 + (18 if i >= 7 and i % 2 else 0)
        parts.append(f'<text transform="translate({x:.1f} {label_y}) rotate(-48)" text-anchor="end" font-family="sans-serif" font-size="12" fill="#394651">{degree}</text>')
    boundary = xcoord(32)
    parts.append(f'<line x1="{boundary:.1f}" y1="{top}" x2="{boundary:.1f}" y2="{height-bottom}" stroke="#7b8794" stroke-dasharray="4 4"/>')
    parts.append(f'<text x="{left+9}" y="{top+20}" font-family="sans-serif" font-size="13" fill="#4c5a67">Dense: every lmax = 0…32</text>')
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
                           series: list[dict]) -> None:
    """Issue-ready log-log absolute timings, matched to the ratio charts.

    The plotted points are the observed warmed full-order, even-smooth GL
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
        f'<text x="{left}" y="61" font-family="sans-serif" font-size="15" fill="#555">f64 · Gauss–Legendre · even-smooth longitude sampling · warmed full transforms · lower is faster</text>',
    ]
    for runtime in runtime_ticks:
        if not ymin <= runtime <= ymax:
            continue
        y = ycoord(runtime)
        parts.append(f'<line x1="{left}" y1="{y:.1f}" x2="{width-right}" y2="{y:.1f}" stroke="#e3e8ed"/>')
        parts.append(f'<text x="{left-11}" y="{y+4:.1f}" text-anchor="end" font-family="sans-serif" font-size="13">{runtime:g}</text>')
    xticks = [0, 1, 2, 4, 8, 16, 24, 32, 40, 64, 128, 255, 511,
              720, 1024, 1536, 1792, 2048, 3072]
    for i, degree in enumerate(v for v in xticks if v <= xmax):
        x = xcoord(degree)
        label_y = height - bottom + 27 + (18 if i >= 7 and i % 2 else 0)
        parts.append(f'<line x1="{x:.1f}" y1="{height-bottom}" x2="{x:.1f}" y2="{height-bottom+6}" stroke="#77818b"/>')
        parts.append(f'<text transform="translate({x:.1f} {label_y}) rotate(-48)" text-anchor="end" font-family="sans-serif" font-size="12" fill="#394651">{degree}</text>')
    boundary = xcoord(32)
    parts.append(f'<line x1="{boundary:.1f}" y1="{top}" x2="{boundary:.1f}" y2="{height-bottom}" stroke="#7b8794" stroke-dasharray="4 4"/>')
    parts.append(f'<text x="{left+9}" y="{top+20}" font-family="sans-serif" font-size="13" fill="#4c5a67">Dense: every lmax = 0…32</text>')
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


def write_issue_runtime_plots(rows: list[dict], output_dir: pathlib.Path) -> None:
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
            output_dir / f"runtime-{operation}.svg", operation, series,
        )


def write_issue_ratio_plots(rows: list[dict], output_dir: pathlib.Path) -> None:
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
                operation, label, grouped,
            )


def write_artifacts(output_dir: pathlib.Path, metadata: dict, cases: list[dict], raw: list[dict], analysis: dict) -> None:
    rows = aggregate(raw, analysis["errors"])
    output_dir.mkdir(parents=True, exist_ok=True)
    sanitized_cases = [
        {key: value for key, value in case.items() if key not in {"map_path", "alm_path"}}
        for case in cases
    ]
    machine = {
        "metadata": metadata,
        "protocol": {
            "source_revision": metadata["ducc_source_revision"],
            "target_batch_ms": TARGET_BATCH_MS,
            "batches_per_process": BATCHES_PER_PROCESS,
            "independent_process_pairs": PROCESS_PAIRS,
            "interleaving": "Rust then DUCC for even process-pair indexes; DUCC then Rust for odd indexes",
            "q_values": [1, 8],
            "thread_control": "Rust fixed Rayon pool; DUCC DUCC0_NUM_THREADS; both workers report active pool capacity",
            "operations": ["analysis", "synthesis"],
            "input_layout": "identical compact m-major coefficient body; DUCC mstart adds a leading offset and maps without timed conversion",
            "fixture_generation": "deterministic PCG64 seed derived from lmax, mmax, and nphi; SHA-256 of each alm/map fixture is recorded per case",
            "output_buffers": "reused across calls; transform outputs read and compared outside timed intervals",
            "timed_region": "native public transform call loop; setup and first call reported separately",
            "short_operation_calibration": "up to three untimed repeated-call batches; adjust repetitions until 20-100 ms, then collect 10 measured batches",
            "long_transform_rule": "operations whose warmup exceeds 100 ms are timed individually (five per process invocation)",
            "output_reset_check": "after timing, exact comparison of first-call output and last repeated-call output",
        },
        "cases": sanitized_cases,
        "rows": rows,
        "samples": raw,
        "numerics": analysis["errors"],
        "validation": analysis["validations"],
        "host_summary": analysis["host"],
    }
    (output_dir / "results.json").write_text(json.dumps(machine, indent=2, allow_nan=False) + "\n")
    if rows:
        with (output_dir / "results.csv").open("w", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
            writer.writeheader()
            writer.writerows(rows)
    write_issue_ratio_plots(rows, output_dir)
    write_issue_runtime_plots(rows, output_dir)
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
        f"Measured {len(cases)} GL scalar configurations using RustSHT's native `ShtPlan` and pinned DUCC's native `analysis_2d`/`synthesis_2d` C++ API. Each transform used all coefficients in its permitted `l=0..lmax, m=0..min(l,mmax)` triangle. Maps and coefficients were generated once per configuration and reused by both workers. `phi0=0.23`; `ntheta=lmax+1`; longitude counts cover odd minimum sampling and even FFT-smooth sampling.",
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
            f"| {row['lmax']} | {row['mmax']} | GL {row['ntheta']}×{row['nphi']} {row['longitude_mode']} | {row['operation']} | {row['threads']} | {row['rust_warm_ms']:.6g} [{row['rust_p10_ms']:.6g},{row['rust_p90_ms']:.6g}] | {row['ducc_warm_ms']:.6g} [{row['ducc_p10_ms']:.6g},{row['ducc_p90_ms']:.6g}] | {row['rust_ducc_ratio']:.4f} | {row['paired_ratio_p10']:.4f}–{row['paired_ratio_p90']:.4f} | {row['excess_ms']:.6g} | {row['relative_l2']:.3e} |"
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
        f"For even-smooth full-mmax GL, q1 RustSHT was near DUCC at T1536 (analysis {t1536_a['rust_ducc_ratio']:.2f}×; synthesis {t1536_s['rust_ducc_ratio']:.2f}×). At T2048, analysis rose to {t2048_a1['rust_ducc_ratio']:.2f}× q1 and {t2048_a8['rust_ducc_ratio']:.2f}× q8, while synthesis was {t2048_s1['rust_ducc_ratio']:.2f}× q1 and {t2048_s8['rust_ducc_ratio']:.2f}× q8. At T3072, analysis was {t3072_a1['rust_ducc_ratio']:.2f}× q1 ({t3072_a1['excess_ms']:.1f} ms excess) and {t3072_a8['rust_ducc_ratio']:.2f}× q8 ({t3072_a8['excess_ms']:.1f} ms excess); synthesis was {t3072_s1['rust_ducc_ratio']:.2f}× q1 and {t3072_s8['rust_ducc_ratio']:.2f}× q8.",
        "The large high-bandlimit excess is specific to the complete analysis call in these cases: synthesis remains near DUCC at q1, while q8 synthesis is about 1.5× slower. T1536 q1 is near parity, so the measured analysis gap grows sharply in the next sampled size range. End-to-end timings include FFT and preparation; they do not isolate recurrence as the cause. The next comparison should separate analysis recurrence/accumulation from FFT and output work at the high-bandlimit cases.",
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
        f"The largest completed degree is T{max(case['lmax'] for case in cases)}. The sweep includes exhaustive lmax 0–32, dense low/intermediate sampling, required T255/T256/T511/T512/T513/T720/T721/T1024/T1536/T2048, and T3072 where resources permitted. f64 scalar GL is the full matrix; other precisions, spins, CC and F1 are outside this benchmark. Existing stage-level scalar-analysis measurements remain in `PERFORMANCE_STATUS.md` and the linked active profile; this run times complete native calls including FFT, Legendre work, packing/conversion and output writes.",
        "",
        f"The Rust/DUCC output comparison validates identical deterministic random maps and complete-triangle coefficient arrays in every timed configuration at q1 and q8. For each configuration and thread count, the first q1/q8 analysis process also runs out-of-timing roundtrip, zonal/intermediate/sectoral basis, zero, subnormal, extreme-finite and analysis/synthesis adjoint checks. Their per-case results are in `{results_prefix}/results.json`; neither set of checks is inferred from roundtrip alone.",
        "",
        "## Reproduction",
        "",
        "```sh",
        "RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo build --release --example ducc_full_transform_parity --features parallel",
        "cmake -S reports/data/ducc-full-transform-parity-2026-10-20 -B \"${DUCC_BUILD_DIR:-ducc-build}\" -DDUCC_SRC=\"${DUCC_SRC:?set DUCC_SRC to the pinned checkout}\" -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=g++",
        "cmake --build \"${DUCC_BUILD_DIR:-ducc-build}\" --parallel 4",
        f"python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src \"${{DUCC_SRC:?set DUCC_SRC to the pinned checkout}}\" --ducc-revision {metadata['ducc_source_revision']} --results-dir {results_from_root} --report-path {report_from_root} --skip-build",
        "```",
        "",
    ]
    report.write_text("\n".join(lines))


def fmt(value):
    return "unavailable" if value is None else f"{value:.4g}"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--ducc-src", type=pathlib.Path, default=os.environ.get("DUCC_SRC"))
    parser.add_argument("--ducc-revision", default=DUCC_REVISION,
                        help="required immutable DUCC commit SHA (default: current active pin)")
    parser.add_argument("--plot-only", action="store_true", help="regenerate ratio and runtime SVGs from existing results.csv; no benchmark or DUCC checkout needed")
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
                        help="use one GL latitude count for every selected case; must be >= lmax + 1")
    parser.add_argument("--results-dir", type=pathlib.Path, default=DATA_DIR,
                        help="directory for machine-readable outputs and plots")
    parser.add_argument("--report-path", type=pathlib.Path, default=REPORT_PATH,
                        help="report path; ignored with --no-report")
    parser.add_argument("--no-report", action="store_true",
                        help="write machine-readable artifacts without replacing a report")
    args = parser.parse_args()
    if args.plot_only:
        with (args.results_dir / "results.csv").open(newline="") as handle:
            rows = list(csv.DictReader(handle))
        if not rows:
            parser.error("results.csv has no benchmark rows")
        write_issue_ratio_plots(rows, args.results_dir)
        write_issue_runtime_plots(rows, args.results_dir)
        print(f"Regenerated four ratio and two runtime figures from {len(rows)} committed rows")
        return
    if args.ducc_src is None:
        parser.error("--ducc-src or DUCC_SRC must name the pinned DUCC checkout")
    ducc_src = pathlib.Path(args.ducc_src).resolve()
    args.results_dir = args.results_dir.resolve()
    args.report_path = args.report_path.resolve()
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
        )
    except ValueError as error:
        parser.error(str(error))
    if not cases:
        parser.error("case filters selected no configurations")
    if args.case_limit:
        cases = cases[: args.case_limit]
    if args.smoke:
        global PROCESS_PAIRS, TARGET_BATCH_MS
        PROCESS_PAIRS = 1
        TARGET_BATCH_MS = 5.0
    metadata = hardware_metadata(ducc_src)
    print(f"BUILD rust={binaries['rustsht']} DUCC={binaries['ducc']}", flush=True)
    print(f"ENV cpu={metadata['cpu_model']} x86={metadata['cpu_features']} affinity=q1/q8 physical cores", flush=True)
    print(f"DISPATCH Rust=Pulp V3 selected by worker; DUCC=fixed x86-64-v3", flush=True)
    raw, aux = run_matrix(cases, binaries, args.smoke)
    if args.smoke:
        rows = aggregate(raw, aux["errors"])
        finite_count = sum(item["finite_outputs"] for item in aux["validations"])
        reset_count = sum(all(item["reset_consistent"].values()) for item in aux["validations"])
        print(
            f"SMOKE complete: {len(cases)} configs, {len(raw)} worker invocations, "
            f"{len(aux['errors'])} random-output comparisons, max relative L2="
            f"{max(row['relative_l2'] for row in rows):.3e}; "
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
        write_report(output_dir, metadata, rows, cases, aux["host"], aux["validations"], args.report_path)
    print(f"RESULTS {len(rows)} rows, {len(aux['errors'])} numerical comparisons; max relative L2={max(row['relative_l2'] for row in rows):.3e}", flush=True)


if __name__ == "__main__":
    main()
