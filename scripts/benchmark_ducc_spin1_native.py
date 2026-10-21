#!/usr/bin/env python3
"""Matched native RustSHT/DUCC spin-1 STANDARD full-transform benchmark."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import pathlib
import shlex
import statistics
import subprocess
import sys
import tempfile
from collections import defaultdict

import numpy as np

import benchmark_ducc_full_transform_parity as common

ROOT = pathlib.Path(__file__).resolve().parents[1]
PINNED_DUCC = "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
DATA = ROOT / "reports/data/plan15-spin1-native-baseline-2026-10-21"
RUST_FLAGS = os.environ.get("RUSTSHT_SPIN1_RUSTFLAGS", "-C target-cpu=x86-64-v3 -Aunexpected_cfgs")
DEFAULT_LMAX = [1, 4, 16, 32, 63, 64, 65, 127, 128, 159, 255, 256, 511, 720, 1023]
ODD_LMAX = {16, 32, 63, 64, 65, 127, 255, 256, 720}


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def smooth_even(minimum: int) -> int:
    value = max(2, minimum + minimum % 2)
    while True:
        rest = value
        for factor in (2, 3, 5, 7):
            while rest % factor == 0:
                rest //= factor
        if rest == 1:
            return value
        value += 2


def make_cases(lmax_values: list[int], grids: list[str], smoke: bool) -> list[dict]:
    cases = []
    for lmax in lmax_values:
        if lmax < 1:
            continue
        mmax_values = [lmax]
        if lmax >= 16:
            mmax_values.append(max(1, lmax // 4))
        for grid in grids:
            ntheta_exact = lmax + (2 if grid == "CC" else 1)
            ntheta_values = [(ntheta_exact, "exact")]
            if not smoke and lmax in {255, 720}:
                if grid == "CC":
                    ntheta_values.extend([
                        (2 * lmax + 1, "cc-resample-below-skip"),
                        (2 * lmax + 2, "cc-skip-threshold"),
                    ])
                else:
                    ntheta_values.append((2 * lmax + 1, "overresolved"))
            for mmax in sorted(set(mmax_values)):
                longitude_modes = ["even-smooth"]
                if not smoke and lmax in ODD_LMAX:
                    longitude_modes.append("odd-minimum")
                if smoke:
                    longitude_modes.append("odd-minimum")
                for ntheta, latitude_mode in ntheta_values:
                    for longitude_mode in longitude_modes:
                        nphi = 2 * mmax + 1 if longitude_mode == "odd-minimum" else smooth_even(2 * mmax + 2)
                        cases.append({
                            "case_id": f"{grid}-T{lmax}-m{mmax}-n{ntheta}-{latitude_mode}-p{nphi}-{longitude_mode}",
                            "grid": grid,
                            "lmax": lmax,
                            "mmax": mmax,
                            "ntheta": ntheta,
                            "nphi": nphi,
                            "latitude_mode": latitude_mode,
                            "longitude_mode": longitude_mode,
                        })
    return cases


def make_fixture(case: dict, directory: pathlib.Path) -> dict:
    seed = (0x51A1_2026 + case["lmax"] * 65537 + case["mmax"] * 97
            + case["nphi"] * 131 + case["ntheta"] * 17 + ("GL", "CC", "F1").index(case["grid"]))
    rng = np.random.Generator(np.random.PCG64(seed))
    lmax, mmax = case["lmax"], case["mmax"]
    nalm = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) // 2
    e = (rng.standard_normal(nalm) + 1j * rng.standard_normal(nalm)).astype("<c16")
    b = (rng.standard_normal(nalm) + 1j * rng.standard_normal(nalm)).astype("<c16")
    offset = 0
    for m in range(mmax + 1):
        length = lmax - m + 1
        if m == 0:
            e[offset:offset + length] = e[offset:offset + length].real
            b[offset:offset + length] = b[offset:offset + length].real
        if m < 1:
            e[offset] = 0.0
            b[offset] = 0.0
        offset += length
    map0 = rng.standard_normal((case["ntheta"], case["nphi"])).astype("<f8")
    map1 = rng.standard_normal((case["ntheta"], case["nphi"])).astype("<f8")
    paths = {}
    for name, values in (("map0", map0), ("map1", map1), ("e", e), ("b", b)):
        path = directory / f"{name}.bin"
        values.tofile(path)
        paths[name] = path
    return {"seed": seed, "paths": paths,
            "sha256": {name: sha256_file(path) for name, path in paths.items()}}


def build(ducc_src: pathlib.Path, build_dir: pathlib.Path) -> dict[str, pathlib.Path]:
    actual_ducc = common.command_output(["git", "rev-parse", "HEAD"], ducc_src)
    if actual_ducc != PINNED_DUCC:
        raise RuntimeError(f"DUCC checkout is {actual_ducc}; expected pinned {PINNED_DUCC}")
    env = os.environ.copy()
    env["RUSTFLAGS"] = RUST_FLAGS
    subprocess.run(["cargo", "build", "--release", "--example", "ducc_spin1_full_transform_parity", "--features", "parallel"], cwd=ROOT, env=env, check=True)
    cxx = shlex.split(os.environ.get("CXX", "g++"))
    subprocess.run([
        "cmake", "-S", str(DATA), "-B", str(build_dir), f"-DDUCC_SRC={ducc_src}",
        "-DCMAKE_BUILD_TYPE=Release", "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
        f"-DCMAKE_CXX_COMPILER={cxx[0]}",
    ], cwd=ROOT, check=True)
    subprocess.run(["cmake", "--build", str(build_dir), "--parallel", "4"], cwd=ROOT, check=True)
    commands = json.loads((build_dir / "compile_commands.json").read_text())
    sht = next(entry for entry in commands if pathlib.Path(entry["file"]).name == "sht.cc")
    driver = next(entry for entry in commands if pathlib.Path(entry["file"]).name == "ducc_spin1_full_transform_parity.cpp")
    command_text = lambda entry: entry.get("command", " ".join(entry.get("arguments", [])))
    sht_command, driver_command = command_text(sht), command_text(driver)
    for flag in ("-O3", "-ffast-math", "-ffp-contract=fast", "-march=x86-64-v3"):
        if flag not in sht_command:
            raise RuntimeError(f"DUCC SHT compile command lacks {flag}: {sht_command}")
    if "-fno-fast-math" not in driver_command or driver_command.rfind("-fno-fast-math") < driver_command.rfind("-ffast-math"):
        raise RuntimeError(f"benchmark driver must override fast math: {driver_command}")
    return {
        "rustsht": ROOT / "target/release/examples/ducc_spin1_full_transform_parity",
        "ducc": build_dir / "ducc_spin1_full_transform_parity",
        "sht_command": sht_command,
        "driver_command": driver_command,
    }


def worker_command(binary: pathlib.Path, case: dict, fixture: dict, operation: str,
                   threads: int, outputs: dict[str, pathlib.Path], target_ms: float) -> list[str]:
    p = fixture["paths"]
    return [str(binary), "--worker", operation, case["grid"], str(threads),
            str(case["ntheta"]), str(case["nphi"]), str(case["lmax"]), str(case["mmax"]),
            "0.23", str(p["map0"]), str(p["map1"]), str(p["e"]), str(p["b"]),
            str(outputs["map0"]), str(outputs["map1"]), str(outputs["e"]), str(outputs["b"]),
            f"{target_ms:.6g}"]


def output_values(path: pathlib.Path, kind: str) -> np.ndarray:
    return np.fromfile(path, dtype="<f8" if kind == "real" else "<c16")


def summarize(values: list[float]) -> dict:
    if not values:
        return {"median": None, "p10": None, "p90": None, "min": None, "max": None, "count": 0}
    arr = np.asarray(values, dtype=np.float64)
    return {"median": float(np.median(arr)), "p10": float(np.quantile(arr, .1)),
            "p90": float(np.quantile(arr, .9)), "min": float(arr.min()), "max": float(arr.max()),
            "count": len(values)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ducc-src", type=pathlib.Path, default=os.environ.get("DUCC_SRC"))
    parser.add_argument("--ducc-revision", default=PINNED_DUCC)
    parser.add_argument("--results-dir", type=pathlib.Path, default=DATA)
    parser.add_argument("--build-dir", type=pathlib.Path)
    parser.add_argument("--lmax", type=int, action="append")
    parser.add_argument("--grid", choices=["GL", "CC", "F1"], action="append")
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--process-pairs", type=int, default=3)
    parser.add_argument("--target-ms", type=float, default=25.0)
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()
    if args.ducc_src is None:
        parser.error("--ducc-src or DUCC_SRC must name the pinned DUCC checkout")
    ducc_src = args.ducc_src.resolve()
    if args.ducc_revision != PINNED_DUCC:
        parser.error(f"this baseline is pinned to {PINNED_DUCC}")
    result_dir = args.results_dir.resolve()
    result_dir.mkdir(parents=True, exist_ok=True)
    build_dir = args.build_dir.resolve() if args.build_dir else pathlib.Path(tempfile.mkdtemp(prefix="rsh-plan15-build-")).resolve()
    binaries = build(ducc_src, build_dir) if not args.skip_build else {
        "rustsht": ROOT / "target/release/examples/ducc_spin1_full_transform_parity",
        "ducc": build_dir / "ducc_spin1_full_transform_parity",
    }
    for name in ("rustsht", "ducc"):
        if not binaries[name].is_file():
            raise FileNotFoundError(f"missing {name} executable: {binaries[name]}")
    lmax_values = [4] if args.smoke else (args.lmax or DEFAULT_LMAX)
    grids = args.grid or (["GL", "CC", "F1"] if not args.smoke else ["GL", "CC", "F1"])
    cases = make_cases(lmax_values, grids, args.smoke)
    physical = common.physical_cpus()
    monitor_cpu = physical[8] if len(physical) > 8 else physical[0]
    metadata = {
        "rust_source_revision": common.command_output(["git", "rev-parse", "HEAD"]),
        "rust_toolchain": common.command_output(["rustc", "-Vv"]),
        "rust_flags": RUST_FLAGS,
        "rust_binary_sha256": sha256_file(binaries["rustsht"]),
        "ducc_revision": common.command_output(["git", "rev-parse", "HEAD"], ducc_src),
        "ducc_version": "0.42.0",
        "ducc_compiler": common.command_output(shlex.split(os.environ.get("CXX", "g++")) + ["--version"]).splitlines()[0],
        "ducc_flags": "SHT numerical sources: -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver adds -fno-fast-math",
        "ducc_binary_sha256": sha256_file(binaries["ducc"]),
        "cpu_model": common.cpu_model(),
        "q1_cpus": physical[:1],
        "q8_cpus": physical[:8],
        "q8_topology": [common.topology(cpu) for cpu in physical[:8]],
        "q8_l3_ids": [common.l3_cache_id(cpu) for cpu in physical[:8]],
        "affinity_policy": "q1 first allowed physical core; q8 first eight allowed physical cores",
        "measurement": {"process_pairs": args.process_pairs, "batches_per_short_process": 10,
                         "target_batch_ms": args.target_ms, "long_call_threshold_ms": 100,
                         "interleaved_backend_order": "Rust then DUCC for even pair; DUCC then Rust for odd pair"},
        "call_boundary": "reused Rust ShtPlan/ShtWorkspace per process; direct native DUCC analysis_2d/synthesis_2d calls; output checksum outside timed batch",
        "benchmark_commands": {
            "rust": "RUSTFLAGS='" + RUST_FLAGS + "' cargo build --release --example ducc_spin1_full_transform_parity --features parallel",
            "ducc": f"cmake -S {DATA} -B <build> -DDUCC_SRC={ducc_src} -DCMAKE_BUILD_TYPE=Release -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DCMAKE_CXX_COMPILER={shlex.split(os.environ.get('CXX','g++'))[0]} && cmake --build <build> --parallel 4",
        },
        "case_count": len(cases),
        "smoke": args.smoke,
    }
    if not args.skip_build:
        metadata["ducc_sht_compile_command"] = binaries["sht_command"]
        metadata["ducc_driver_compile_command"] = binaries["driver_command"]
    raw, comparisons, failures, fixtures = [], [], [], []
    with tempfile.TemporaryDirectory(prefix="rsh-plan15-fixtures-") as tmp_value:
        tmp = pathlib.Path(tmp_value)
        total = len(cases) * 4
        progress = 0
        for case_index, case in enumerate(cases):
            case_tmp = tmp / f"case-{case_index}"
            case_tmp.mkdir()
            fixture = make_fixture(case, case_tmp)
            fixtures.append({**case, "seed": fixture["seed"], "input_sha256": fixture["sha256"]})
            nalm = (case["mmax"] + 1) * (case["lmax"] + 1) - case["mmax"] * (case["mmax"] + 1) // 2
            for threads in (1, 8):
                cpus = physical[:threads]
                for operation in ("analysis", "synthesis"):
                    progress += 1
                    print(f"CASE {progress}/{total} {case['case_id']} {operation} q{threads}", flush=True)
                    pair_workers = []
                    for pair in range(args.process_pairs):
                        records = {}
                        output_paths = {}
                        order = ["rustsht", "ducc"] if pair % 2 == 0 else ["ducc", "rustsht"]
                        for backend in order:
                            output = {name: tmp / f"{backend}-{operation}-q{threads}-p{pair}-{name}.bin"
                                      for name in ("map0", "map1", "e", "b")}
                            command = worker_command(binaries[backend], case, fixture, operation, threads, output, args.target_ms)
                            try:
                                result, diagnostics = common.run_pinned(command, cpus, monitor_cpu)
                            except Exception as error:
                                failures.append({**case, "operation": operation, "threads": threads,
                                                 "process_pair": pair, "backend": backend,
                                                 "failure_class": "worker_error", "reason": str(error)[:12000]})
                                print(f"FAIL {case['case_id']} {operation} q{threads} p{pair} {backend}: {error}", flush=True)
                                continue
                            result.update({**case, "operation": operation, "threads": threads,
                                           "process_pair": pair, "host_diagnostics": diagnostics,
                                           "fixture_sha256": fixture["sha256"]})
                            raw.append(result)
                            records[backend] = result
                            output_paths[backend] = output
                        if len(output_paths) != 2:
                            continue
                        names, kind = (("e", "b"), "complex") if operation == "analysis" else (("map0", "map1"), "real")
                        rust_values = np.concatenate([output_values(output_paths["rustsht"][name], kind) for name in names])
                        ducc_values = np.concatenate([output_values(output_paths["ducc"][name], kind) for name in names])
                        metrics = common.error_metrics(rust_values, ducc_values)
                        finite = bool(np.isfinite(rust_values).all() and np.isfinite(ducc_values).all())
                        record = {**case, "operation": operation, "threads": threads,
                                  "process_pair": pair, "finite_outputs": finite, **metrics,
                                  "repeat_consistent": {key: bool(records[key]["repeat_consistent"]) for key in records},
                                  "reset_consistent": {key: bool(records[key]["reset_consistent"]) for key in records}}
                        comparisons.append(record)
                        if not finite or not all(record["repeat_consistent"].values()) or not all(record["reset_consistent"].values()):
                            failures.append({**record, "failure_class": "numerical_or_reset_validation"})
                        pair_workers.append((records.get("rustsht"), records.get("ducc")))

    # Aggregate per-worker batches, preserving process pairs as the paired unit.
    groups = defaultdict(lambda: {"rustsht": [], "ducc": [], "ratios": [], "errors": []})
    for result in raw:
        key = (result["case_id"], result["operation"], result["threads"])
        groups[key][result["backend"]].append(result)
    for error in comparisons:
        groups[(error["case_id"], error["operation"], error["threads"])]["errors"].append(error)
    summaries = []
    for (case_id, operation, threads), group in groups.items():
        rust_by_pair = {item["process_pair"]: item for item in group["rustsht"]}
        ducc_by_pair = {item["process_pair"]: item for item in group["ducc"]}
        pair_ratios = []
        for pair in sorted(set(rust_by_pair) & set(ducc_by_pair)):
            pair_ratios.append(float(np.median(rust_by_pair[pair]["batch_ms"])) /
                                float(np.median(ducc_by_pair[pair]["batch_ms"])))
        rust_batches = [sample for item in group["rustsht"] for sample in item["batch_ms"]]
        ducc_batches = [sample for item in group["ducc"] for sample in item["batch_ms"]]
        case = next(case for case in cases if case["case_id"] == case_id)
        summaries.append({**case, "operation": operation, "threads": threads,
                          "rust_ms": summarize(rust_batches), "ducc_ms": summarize(ducc_batches),
                          "paired_ratio_median": float(np.median(pair_ratios)) if pair_ratios else None,
                          "paired_ratio_by_process": pair_ratios,
                          "max_relative_l2": max((item["relative_l2"] for item in group["errors"]), default=None),
                          "comparison_pairs": len(group["errors"]),
                          "first_call_rust_ms": [item["first_ms"] for item in group["rustsht"]],
                          "first_call_ducc_ms": [item["first_ms"] for item in group["ducc"]],
                          "setup_rust_ms": [item.get("setup_ms") for item in group["rustsht"]],
                          "setup_ducc_ms": [item.get("setup_ms") for item in group["ducc"]]})

    result = {"metadata": metadata, "cases": fixtures, "raw_workers": raw,
              "comparisons": comparisons, "summaries": summaries, "failures": failures}
    (result_dir / "results.json").write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
    fields = ["case_id", "grid", "lmax", "mmax", "ntheta", "nphi", "latitude_mode", "longitude_mode",
              "operation", "threads", "backend", "process_pair", "batch_index", "latency_ms",
              "first_ms", "warmup_ms", "setup_ms", "repetitions_per_batch", "runqueue_wait_fraction",
              "frequency_ghz_median", "fixture_sha256"]
    with (result_dir / "batch-samples.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        for item in raw:
            diag = item.get("host_diagnostics", {})
            freq = diag.get("frequency_ghz", [])
            for index, latency in enumerate(item.get("batch_ms", [])):
                writer.writerow({"case_id": item["case_id"], "grid": item["grid"], "lmax": item["lmax"],
                                 "mmax": item["mmax"], "ntheta": item["ntheta"], "nphi": item["nphi"],
                                 "latitude_mode": item["latitude_mode"], "longitude_mode": item["longitude_mode"],
                                 "operation": item["operation"], "threads": item["threads"], "backend": item["backend"],
                                 "process_pair": item["process_pair"], "batch_index": index, "latency_ms": latency,
                                 "first_ms": item["first_ms"], "warmup_ms": item["warmup_ms"],
                                 "setup_ms": item.get("setup_ms"), "repetitions_per_batch": item["repetitions_per_batch"],
                                 "runqueue_wait_fraction": diag.get("wait_fraction"),
                                 "frequency_ghz_median": float(np.median(freq)) if freq else None,
                                 "fixture_sha256": item["fixture_sha256"]})
    (result_dir / "failures.json").write_text(json.dumps(failures, indent=2) + "\n")
    print(f"Finished {len(cases)} configurations; {len(raw)} worker records; {len(comparisons)} numerical comparisons; {len(failures)} failures. Results: {result_dir}", flush=True)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
