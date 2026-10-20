#!/usr/bin/env python3
"""Summarize matched native transform runs for fixed scalar-analysis scratch."""

from __future__ import annotations

import argparse
import csv
import json
import pathlib
import statistics


FIELDS = (
    "case_id",
    "lmax",
    "mmax",
    "ntheta",
    "nphi",
    "longitude_mode",
    "operation",
    "threads",
    "baseline_rust_ms",
    "baseline_rust_p10_ms",
    "baseline_rust_p90_ms",
    "candidate_rust_ms",
    "candidate_rust_p10_ms",
    "candidate_rust_p90_ms",
    "candidate_over_baseline",
    "baseline_ducc_ms",
    "candidate_ducc_ms",
    "baseline_rust_ducc_ratio",
    "candidate_rust_ducc_ratio",
    "candidate_over_baseline_rust_ducc_ratio",
    "relative_l2",
    "absolute_max",
    "max_relative_above_1e-12_peak",
    "batch_count_per_backend",
)


def load(path: pathlib.Path) -> dict:
    with path.open() as handle:
        return json.load(handle)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("baseline", type=pathlib.Path)
    parser.add_argument("candidate", type=pathlib.Path)
    parser.add_argument("output", type=pathlib.Path)
    args = parser.parse_args()

    baseline = load(args.baseline)
    candidate = load(args.candidate)
    for key in ("rustc", "ducc_source_revision", "ducc_flags", "ducc_dispatch"):
        if baseline["metadata"][key] != candidate["metadata"][key]:
            raise SystemExit(f"baseline and candidate differ in {key}")
    base_rows = {
        (row["case_id"], row["operation"], row["threads"]): row
        for row in baseline["rows"]
    }
    candidate_rows = {
        (row["case_id"], row["operation"], row["threads"]): row
        for row in candidate["rows"]
    }
    if base_rows.keys() != candidate_rows.keys():
        raise SystemExit("baseline and candidate case/operation/thread keys differ")

    rows = []
    for key in sorted(base_rows):
        base = base_rows[key]
        current = candidate_rows[key]
        rows.append(
            {
                "case_id": base["case_id"],
                "lmax": base["lmax"],
                "mmax": base["mmax"],
                "ntheta": base["ntheta"],
                "nphi": base["nphi"],
                "longitude_mode": base["longitude_mode"],
                "operation": base["operation"],
                "threads": base["threads"],
                "baseline_rust_ms": base["rust_warm_ms"],
                "baseline_rust_p10_ms": base["rust_p10_ms"],
                "baseline_rust_p90_ms": base["rust_p90_ms"],
                "candidate_rust_ms": current["rust_warm_ms"],
                "candidate_rust_p10_ms": current["rust_p10_ms"],
                "candidate_rust_p90_ms": current["rust_p90_ms"],
                "candidate_over_baseline": current["rust_warm_ms"]
                / base["rust_warm_ms"],
                "baseline_ducc_ms": base["ducc_warm_ms"],
                "candidate_ducc_ms": current["ducc_warm_ms"],
                "baseline_rust_ducc_ratio": base["rust_ducc_ratio"],
                "candidate_rust_ducc_ratio": current["rust_ducc_ratio"],
                "candidate_over_baseline_rust_ducc_ratio": current[
                    "rust_ducc_ratio"
                ]
                / base["rust_ducc_ratio"],
                "relative_l2": current["relative_l2"],
                "absolute_max": current["absolute_max"],
                "max_relative_above_1e-12_peak": current[
                    "max_relative_above_1e-12_peak"
                ],
                "batch_count_per_backend": current["batch_count"],
            }
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=FIELDS, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)

    for threads in (1, 8):
        selected = [row for row in rows if row["threads"] == threads]
        rust_changes = [row["candidate_over_baseline"] for row in selected]
        ratio_changes = [
            row["candidate_over_baseline_rust_ducc_ratio"] for row in selected
        ]
        print(
            f"q{threads}: {len(selected)} rows; median Rust candidate/baseline="
            f"{statistics.median(rust_changes):.4f}; median change in Rust/DUCC="
            f"{statistics.median(ratio_changes):.4f}; "
            f"Rust wins={sum(value < 1.0 for value in rust_changes)}/"
            f"{len(selected)}"
        )
    print(f"wrote {len(rows)} sanitized rows to {args.output}")


if __name__ == "__main__":
    main()
