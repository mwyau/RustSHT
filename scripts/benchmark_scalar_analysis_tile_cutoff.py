#!/usr/bin/env python3
"""Matched T2047/T2048 scalar-analysis cutoff and pair-tile comparison.

Build one DUCC worker and three Rust workers with pair tiles 64/128/256,
then interleave complete analysis calls at q1/q8. Raw process diagnostics are
written only to the caller-selected local JSON path; the compact CSV contains
timing/error summaries suitable for reports.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import pathlib
import sys
import tempfile
import time

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from scripts import benchmark_ducc_full_transform_parity as bench

DUCC_REVISION = "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
CASES = [
    {
        "case_id": "T2047-m2047-ntheta2048-nphi4096",
        "lmax": 2047,
        "mmax": 2047,
        "ntheta": 2048,
        "nphi": 4096,
        "longitude_mode": "even_smooth",
        "grid": "GL",
    },
    {
        "case_id": "T2047-m2047-ntheta2049-nphi4096",
        "lmax": 2047,
        "mmax": 2047,
        "ntheta": 2049,
        "nphi": 4096,
        "longitude_mode": "even_smooth",
        "grid": "GL",
    },
    {
        "case_id": "T2048-m2047-ntheta2049-nphi4096",
        "lmax": 2048,
        "mmax": 2047,
        "ntheta": 2049,
        "nphi": 4096,
        "longitude_mode": "even_smooth",
        "grid": "GL",
    },
]
SCHEDULES = [
    [("ducc", None), ("rustsht", 256), ("rustsht", 64), ("rustsht", 128)],
    [("rustsht", 64), ("rustsht", 128), ("rustsht", 256), ("ducc", None)],
    [("ducc", None), ("rustsht", 128), ("rustsht", 256), ("rustsht", 64)],
]


def summarize(raw: dict) -> list[dict]:
    rows, errors = raw["rows"], raw["errors"]
    result = []
    for case in CASES:
        for threads in (1, 8):
            selected_case = [
                row for row in rows
                if row["case_id"] == case["case_id"] and row["threads"] == threads
            ]
            ducc_records = {
                row["round"]: row for row in selected_case if row["variant"] == "ducc"
            }
            ducc_samples = [
                value for row in ducc_records.values() for value in row["batch_ms"]
            ]
            ducc_median = float(np.median(ducc_samples))
            for variant in ("ducc", "tile256", "tile128", "tile64"):
                selected = [row for row in selected_case if row["variant"] == variant]
                samples = [value for row in selected for value in row["batch_ms"]]
                matching_errors = [
                    item for item in errors
                    if item["case_id"] == case["case_id"]
                    and item["threads"] == threads and item["variant"] == variant
                ]
                paired_ratios = []
                if variant != "ducc":
                    for row in selected:
                        reference = ducc_records[row["round"]]
                        paired_ratios.extend(
                            rust_time / ducc_time
                            for rust_time, ducc_time in zip(row["batch_ms"], reference["batch_ms"])
                        )
                waits = [
                    row["diagnostics"]["wait_fraction"] for row in selected
                    if row["diagnostics"]["wait_fraction"] is not None
                ]
                frequencies = [
                    value for row in selected for value in row["diagnostics"]["frequency_ghz"]
                ]
                result.append({
                    "case_id": case["case_id"],
                    "lmax": case["lmax"],
                    "mmax": case["mmax"],
                    "ntheta": case["ntheta"],
                    "nphi": case["nphi"],
                    "threads": threads,
                    "variant": variant,
                    "process_count": len(selected),
                    "sample_count": len(samples),
                    "rust_median_ms": float(np.median(samples)),
                    "rust_p10_ms": float(np.quantile(samples, .1)),
                    "rust_p90_ms": float(np.quantile(samples, .9)),
                    "ducc_median_ms": ducc_median,
                    "rust_ducc_median_ratio": float(np.median(samples)) / ducc_median,
                    "paired_ratio_median": float(np.median(paired_ratios)) if paired_ratios else "",
                    "paired_ratio_p10": float(np.quantile(paired_ratios, .1)) if paired_ratios else "",
                    "paired_ratio_p90": float(np.quantile(paired_ratios, .9)) if paired_ratios else "",
                    "delta_vs_tile256_pct": (
                        (float(np.median(samples)) / float(np.median([
                            value for row in selected_case if row["variant"] == "tile256"
                            for value in row["batch_ms"]
                        ])) - 1.0) * 100.0
                        if variant.startswith("tile") else ""
                    ),
                    "relative_l2_max": max((item["relative_l2"] for item in matching_errors), default=""),
                    "absolute_max_error": max((item["absolute_max"] for item in matching_errors), default=""),
                    "cold_first_median_ms": float(np.median([row["first_ms"] for row in selected])),
                    "setup_median_ms": float(np.median([row["setup_ms"] for row in selected])),
                    "wait_fraction_median": float(np.median(waits)) if waits else "",
                    "wait_fraction_p90": float(np.quantile(waits, .9)) if waits else "",
                    "frequency_ghz_median": float(np.median(frequencies)) if frequencies else "",
                    "reset_consistent_all": all(row["reset_consistent"] for row in selected),
                    "arch": (
                        "DUCC fixed V3" if variant == "ducc" else "Pulp V3"
                    ) if selected else "",
                })
    return result


def write_summary(rows: list[dict], path: pathlib.Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ducc-src", type=pathlib.Path, default=os.environ.get("DUCC_SRC"))
    parser.add_argument("--ducc-binary", type=pathlib.Path)
    parser.add_argument("--tile256-binary", type=pathlib.Path)
    parser.add_argument("--tile128-binary", type=pathlib.Path)
    parser.add_argument("--tile64-binary", type=pathlib.Path)
    parser.add_argument("--raw-json", type=pathlib.Path,
                        help="local path for the untracked per-process and host capture")
    parser.add_argument("--summary-csv", type=pathlib.Path)
    parser.add_argument("--summarize-raw", type=pathlib.Path,
                        help="summarize an existing local raw JSON capture without running workers")
    args = parser.parse_args()
    if args.summarize_raw:
        raw = json.loads(args.summarize_raw.read_text())
        summary = summarize(raw)
        if args.summary_csv:
            write_summary(summary, args.summary_csv)
        for row in summary:
            print(json.dumps(row, sort_keys=True))
        return

    binaries = {
        256: args.tile256_binary,
        128: args.tile128_binary,
        64: args.tile64_binary,
    }
    if args.ducc_src is None or args.ducc_binary is None or any(value is None for value in binaries.values()):
        parser.error("run mode requires --ducc-src, --ducc-binary and all three --tile*-binary arguments")
    if args.raw_json is None:
        parser.error("run mode requires --raw-json outside the repository's committed data")
    ducc_src = args.ducc_src.resolve()
    if bench.command_output(["git", "rev-parse", "HEAD"], ducc_src) != DUCC_REVISION:
        parser.error(f"DUCC source must be pinned at {DUCC_REVISION}")
    if not args.ducc_binary.is_file() or any(not path.is_file() for path in binaries.values()):
        parser.error("worker binary does not exist")

    physical = bench.physical_cpus()
    monitor_cpu = physical[8]
    rows, errors = [], []
    start_psi = pathlib.Path("/proc/pressure/cpu").read_text()
    started = time.time()
    for case in CASES:
        with tempfile.TemporaryDirectory(prefix="rustsht-analysis-tile-") as temporary:
            temp = pathlib.Path(temporary)
            bench.make_fixture(case, temp)
            for threads in (1, 8):
                invocations = []
                for round_index, schedule in enumerate(SCHEDULES):
                    for sequence, (backend, tile) in enumerate(schedule):
                        variant = "ducc" if backend == "ducc" else f"tile{tile}"
                        selected_binaries = {"ducc": args.ducc_binary, "rustsht": binaries[256 if tile is None else tile]}
                        output = temp / f"{case['case_id']}-q{threads}-r{round_index}-{sequence}-{variant}.bin"
                        command = bench.worker_command(
                            selected_binaries, backend, case, "analysis", threads, output, 25.0, validate=False
                        )
                        record, diagnostics = bench.run_pinned(command, physical[:threads], monitor_cpu)
                        record.update({
                            "case_id": case["case_id"], "lmax": case["lmax"], "mmax": case["mmax"],
                            "ntheta": case["ntheta"], "nphi": case["nphi"], "threads": threads,
                            "round": round_index, "sequence": sequence, "variant": variant,
                            "output_path": str(output), "diagnostics": diagnostics,
                        })
                        invocations.append(record)
                ducc_records = [row for row in invocations if row["variant"] == "ducc"]
                for row in invocations:
                    if row["variant"] == "ducc":
                        continue
                    reference = min(ducc_records, key=lambda item: abs(item["round"] - row["round"]))
                    actual = bench.read_result(pathlib.Path(row["output_path"]), "analysis", case, "rustsht")
                    expected = bench.read_result(pathlib.Path(reference["output_path"]), "analysis", case, "ducc")
                    errors.append({
                        "case_id": case["case_id"], "threads": threads, "variant": row["variant"],
                        "round": row["round"], **bench.error_metrics(actual, expected),
                    })
                for row in invocations:
                    row.pop("output_path")
                    rows.append(row)
                print(f"DONE {case['case_id']} q{threads}", flush=True)

    raw = {
        "metadata": {
            "source_revision": bench.command_output(["git", "rev-parse", "HEAD"]),
            "ducc_revision": DUCC_REVISION,
            "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs; tile selector cfg varies by binary",
            "ducc_flags": "-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3 for SHT sources; strict driver",
            "cpu": bench.cpu_model(), "physical_cpus": physical,
            "start_psi": start_psi, "end_psi": pathlib.Path("/proc/pressure/cpu").read_text(),
            "elapsed_seconds": time.time() - started,
            "timing_protocol": "three interleaved rounds; tile order rotated around DUCC; worker uses five whole-call samples for operations >=100ms, otherwise calibrated 25ms batches",
        },
        "rows": rows,
        "errors": errors,
    }
    args.raw_json.parent.mkdir(parents=True, exist_ok=True)
    args.raw_json.write_text(json.dumps(raw, indent=2))
    summary = summarize(raw)
    if args.summary_csv:
        write_summary(summary, args.summary_csv)
    for row in summary:
        print(json.dumps(row, sort_keys=True))
    print(f"RAW={args.raw_json}")


if __name__ == "__main__":
    main()
