#!/usr/bin/env python3
"""Interleaved full scalar SHT A/B for the odd-row pairing experiment."""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import importlib.util
import json
import os
import pathlib
import platform
import statistics
import subprocess
import tempfile

import numpy as np


ROOT = pathlib.Path(__file__).resolve().parents[2]
BENCH_PATH = ROOT / "scripts/benchmark_ducc_full_transform_parity.py"
SPEC = importlib.util.spec_from_file_location("ducc_full_transform_parity", BENCH_PATH)
BENCH = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(BENCH)

CASES = [
    (124, 124, "odd_minimum"),
    (127, 127, "odd_minimum"),
    (248, 248, "odd_minimum"),
    (254, 254, "odd_minimum"),
    (255, 255, "odd_minimum"),
    (256, 256, "odd_minimum"),
    (257, 257, "odd_minimum"),
    (720, 180, "odd_minimum"),
    (720, 720, "odd_minimum"),
    (720, 720, "even_smooth"),
]


def cpu_pressure() -> str:
    return pathlib.Path("/proc/pressure/cpu").read_text().strip()


def host_top() -> list[str]:
    result = subprocess.run(
        ["ps", "-eo", "pid,psr,pcpu,comm", "--sort=-pcpu"],
        check=True,
        text=True,
        capture_output=True,
    )
    return result.stdout.splitlines()[:9]


def quantiles(values: list[float]) -> dict[str, float]:
    array = np.asarray(values, dtype=np.float64)
    return {
        "median_ms": float(np.median(array)),
        "p10_ms": float(np.quantile(array, 0.10)),
        "p90_ms": float(np.quantile(array, 0.90)),
        "min_ms": float(np.min(array)),
        "max_ms": float(np.max(array)),
    }


def write_jsonl(path: pathlib.Path, records: list[dict]) -> None:
    path.write_text("".join(json.dumps(record, sort_keys=True) + "\n" for record in records))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=pathlib.Path, required=True)
    parser.add_argument("--candidate", type=pathlib.Path, required=True)
    parser.add_argument("--ducc", type=pathlib.Path, required=True)
    parser.add_argument("--output-dir", type=pathlib.Path, required=True)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--target-ms", type=float, default=25.0)
    parser.add_argument("--case", action="append", help="case_id filter; may be repeated")
    parser.add_argument(
        "--operation", choices=("analysis", "synthesis"), action="append",
        help="operation filter; may be repeated (default: both)",
    )
    args = parser.parse_args()
    if args.runs < 3:
        parser.error("use at least three independent interleaved runs")

    cases = CASES
    if args.case:
        requested = set(args.case)
        cases = [case for case in CASES if f"T{case[0]}-m{case[1]}-{case[2]}" in requested]
        missing = requested - {f"T{case[0]}-m{case[1]}-{case[2]}" for case in cases}
        if missing:
            parser.error(f"unknown case_id(s): {', '.join(sorted(missing))}")
    operations = args.operation or ["analysis", "synthesis"]

    args.output_dir.mkdir(parents=True, exist_ok=True)
    physical = BENCH.physical_cpus()
    if len(physical) < 9:
        raise RuntimeError("run under taskset with at least nine physical cores allowed")
    cpu_sets = {1: physical[:1], 8: physical[:8]}
    monitor_cpu = physical[8]
    binary_paths = {
        "baseline": args.baseline.resolve(),
        "candidate": args.candidate.resolve(),
        "ducc": args.ducc.resolve(),
    }
    config = {
        "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "rust_source_sha": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, check=True, text=True, capture_output=True
        ).stdout.strip(),
        "candidate_source_diff_sha256": hashlib.sha256(
            subprocess.run(["git", "diff", "--binary"], cwd=ROOT, check=True, capture_output=True).stdout
        ).hexdigest(),
        "ducc_revision": BENCH.DUCC_REVISION,
        "cpu_model": BENCH.cpu_model(),
        "physical_cpus_available": physical,
        "q1_cpus": cpu_sets[1],
        "q8_cpus": cpu_sets[8],
        "monitor_cpu": monitor_cpu,
        "runs_per_route": args.runs,
        "target_batch_ms": args.target_ms,
        "rust_flags": BENCH.RUST_FLAGS,
        "binary_sha256": {name: BENCH.sha256_file(path) for name, path in binary_paths.items()},
        "cases": [],
        "host_before": {"cpu_pressure": cpu_pressure(), "load_average": os.getloadavg(), "top": host_top()},
    }
    process_rows: list[dict] = []
    result_rows: list[dict] = []
    validations: list[dict] = []
    case_total = len(cases) * 2 * len(operations)
    case_progress = 0

    for lmax, mmax, longitude_mode in cases:
        nphi = BENCH.longitude_count(mmax, longitude_mode)
        case = {
            "case_id": f"T{lmax}-m{mmax}-{longitude_mode}",
            "lmax": lmax,
            "mmax": mmax,
            "ntheta": lmax + 1,
            "nphi": nphi,
            "longitude_mode": longitude_mode,
            "grid": "GL",
        }
        config["cases"].append(case.copy())
        for threads in (1, 8):
            for operation in operations:
                case_progress += 1
                print(
                    f"CASE {case_progress}/{case_total} {case['case_id']} {operation} q{threads}",
                    flush=True,
                )
                with tempfile.TemporaryDirectory(prefix="paired-odd-sht-") as temporary:
                    temp = pathlib.Path(temporary)
                    BENCH.make_fixture(case, temp)
                    config["cases"][-1]["input_sha256"] = case["input_sha256"]
                    run_outputs: list[dict[str, pathlib.Path]] = []
                    run_records: list[dict[str, dict]] = []
                    for run_index in range(args.runs):
                        order = ["baseline", "candidate", "ducc"]
                        shift = run_index % len(order)
                        order = order[shift:] + order[:shift]
                        if run_index % 2:
                            order.reverse()
                        outputs: dict[str, pathlib.Path] = {}
                        records: dict[str, dict] = {}
                        for route in order:
                            output_path = temp / f"{route}-{operation}-q{threads}-run{run_index}.bin"
                            backend = "ducc" if route == "ducc" else "rustsht"
                            executable = binary_paths[route]
                            command = BENCH.worker_command(
                                {backend: executable},
                                backend,
                                case,
                                operation,
                                threads,
                                output_path,
                                args.target_ms,
                                validate=(run_index == 0 and operation == "analysis"),
                            )
                            before = {
                                "cpu_pressure": cpu_pressure(),
                                "load_average": os.getloadavg(),
                                "top": host_top(),
                            }
                            record, diagnostics = BENCH.run_pinned(
                                command, cpu_sets[threads], monitor_cpu
                            )
                            after = {
                                "cpu_pressure": cpu_pressure(),
                                "load_average": os.getloadavg(),
                                "top": host_top(),
                            }
                            outputs[route] = output_path
                            records[route] = record
                            process_rows.append({
                                "case_id": case["case_id"],
                                "operation": operation,
                                "threads": threads,
                                "run": run_index + 1,
                                "route": route,
                                "cpu_set": cpu_sets[threads],
                                "batch_ms": record["batch_ms"],
                                "batch_total_ms": record["batch_total_ms"],
                                "timed_batch_count": len(record["batch_ms"]),
                                "repetitions_per_batch": record["repetitions"],
                                "first_ms": record["first_ms"],
                                "warmup_ms": record["warmup_ms"],
                                "timed_call_count": len(record["batch_ms"]) * record["repetitions"],
                                "wait_fraction": diagnostics["wait_fraction"],
                                "involuntary_switches": diagnostics["involuntary_switches"],
                                "frequency_ghz": diagnostics["frequency_ghz"],
                                "host_before": before,
                                "host_after": after,
                            })
                        run_outputs.append(outputs)
                        run_records.append(records)

                        base_values = BENCH.read_result(outputs["baseline"], operation, case, "rustsht")
                        candidate_values = BENCH.read_result(outputs["candidate"], operation, case, "rustsht")
                        ducc_values = BENCH.read_result(outputs["ducc"], operation, case, "ducc")
                        result_rows.append({
                            "case_id": case["case_id"],
                            "operation": operation,
                            "threads": threads,
                            "run": run_index + 1,
                            "baseline_ms": quantiles(run_records[-1]["baseline"]["batch_ms"])["median_ms"],
                            "candidate_ms": quantiles(run_records[-1]["candidate"]["batch_ms"])["median_ms"],
                            "ducc_ms": quantiles(run_records[-1]["ducc"]["batch_ms"])["median_ms"],
                            "candidate_vs_baseline": BENCH.error_metrics(candidate_values, base_values),
                            "candidate_vs_ducc": BENCH.error_metrics(candidate_values, ducc_values),
                        })

                    if operation == "analysis":
                        # The first run of each backend emitted independent roundtrip,
                        # basis-mode, edge-case and adjoint evidence outside timing.
                        for route in ("baseline", "candidate", "ducc"):
                            route_records = run_records[0][route]
                            output = run_outputs[0][route]
                            validation = route_records["validation"]
                            entry = {
                                "case_id": case["case_id"],
                                "threads": threads,
                                "route": route,
                                "validation": validation,
                                "reset_consistent": route_records["reset_consistent"],
                            }
                            validations.append(entry)
                        baseline_path = run_outputs[0]["baseline"]
                        candidate_path = run_outputs[0]["candidate"]
                        ducc_path = run_outputs[0]["ducc"]
                        for route, output_path, backend in (
                            ("baseline", baseline_path, "rustsht"),
                            ("candidate", candidate_path, "rustsht"),
                            ("ducc", ducc_path, "ducc"),
                        ):
                            validation_outputs = {"rustsht": output_path, "ducc": output_path}
                            validation_record = run_records[0][route]
                            if route == "ducc":
                                continue
                            validation_outputs["ducc"] = ducc_path
                            checks = BENCH.compare_validation(
                                case,
                                validation_outputs,
                                {
                                    "rustsht": validation_record,
                                    "ducc": run_records[0]["ducc"],
                                },
                            )
                            validations.append({
                                "case_id": case["case_id"],
                                "threads": threads,
                                "route_vs_ducc": route,
                                "checks": checks,
                            })

                    for outputs in run_outputs:
                        for output_path in outputs.values():
                            output_path.unlink(missing_ok=True)

    result_groups: dict[tuple[str, str, int], dict[str, list[float]]] = {}
    for row in result_rows:
        key = (row["case_id"], row["operation"], row["threads"])
        group = result_groups.setdefault(key, {route: [] for route in ("baseline", "candidate", "ducc")})
        for route in group:
            group[route].append(float(row[f"{route}_ms"]))
    summary_rows = []
    for (case_id, operation, threads), group in sorted(result_groups.items()):
        baseline = statistics.median(group["baseline"])
        candidate = statistics.median(group["candidate"])
        ducc = statistics.median(group["ducc"])
        paired = [
            row for row in result_rows
            if row["case_id"] == case_id and row["operation"] == operation and row["threads"] == threads
        ]
        summary_rows.append({
            "case_id": case_id,
            "operation": operation,
            "threads": threads,
            "baseline_median_ms": baseline,
            "candidate_median_ms": candidate,
            "ducc_median_ms": ducc,
            "candidate_saved_us": (baseline - candidate) * 1000.0,
            "candidate_speedup": baseline / candidate,
            "candidate_vs_ducc": candidate / ducc,
            "paired_candidate_baseline_ratios": [r["baseline_ms"] / r["candidate_ms"] for r in paired],
            "baseline_process_range_ms": [min(group["baseline"]), max(group["baseline"])],
            "candidate_process_range_ms": [min(group["candidate"]), max(group["candidate"])],
            "ducc_process_range_ms": [min(group["ducc"]), max(group["ducc"])],
        })
    config["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    config["host_after"] = {"cpu_pressure": cpu_pressure(), "load_average": os.getloadavg(), "top": host_top()}
    (args.output_dir / "config.json").write_text(json.dumps(config, indent=2) + "\n")
    write_jsonl(args.output_dir / "process-samples.jsonl", process_rows)
    write_jsonl(args.output_dir / "paired-results.jsonl", result_rows)
    write_jsonl(args.output_dir / "validation.jsonl", validations)
    with (args.output_dir / "summary.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(summary_rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(summary_rows)
    print(f"wrote {len(summary_rows)} full-transform A/B summaries to {args.output_dir}", flush=True)


if __name__ == "__main__":
    main()
