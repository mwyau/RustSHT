#!/usr/bin/env python3
"""Generate paired latency/error CSV from results.json."""

from __future__ import annotations

import csv
import json
from collections import defaultdict
from pathlib import Path


HERE = Path(__file__).resolve().parent
result = json.loads((HERE / "results.json").read_text())
errors = defaultdict(list)
for row in result["comparisons"]:
    key = (row["case_id"], row["operation"], row["threads"])
    errors[key].append(row)

fields = [
    "grid", "case_id", "lmax", "mmax", "operation", "threads",
    "rust_median_ms", "rust_p10_ms", "rust_p90_ms", "ducc_median_ms",
    "ducc_p10_ms", "ducc_p90_ms", "paired_ratio", "absolute_excess_ms",
    "max_absolute_error", "max_relative_l2", "comparison_pairs",
]
rows = []
for summary in sorted(
    result["summaries"],
    key=lambda row: (row["grid"], row["mmax"], row["operation"], row["threads"]),
):
    key = (summary["case_id"], summary["operation"], summary["threads"])
    comparisons = errors[key]
    rows.append(
        {
            "grid": summary["grid"],
            "case_id": summary["case_id"],
            "lmax": summary["lmax"],
            "mmax": summary["mmax"],
            "operation": summary["operation"],
            "threads": summary["threads"],
            "rust_median_ms": summary["rust_ms"]["median"],
            "rust_p10_ms": summary["rust_ms"]["p10"],
            "rust_p90_ms": summary["rust_ms"]["p90"],
            "ducc_median_ms": summary["ducc_ms"]["median"],
            "ducc_p10_ms": summary["ducc_ms"]["p10"],
            "ducc_p90_ms": summary["ducc_ms"]["p90"],
            "paired_ratio": summary["paired_ratio_median"],
            "absolute_excess_ms": (
                summary["rust_ms"]["median"] - summary["ducc_ms"]["median"]
            ),
            "max_absolute_error": max(row["absolute_max"] for row in comparisons),
            "max_relative_l2": max(row["relative_l2"] for row in comparisons),
            "comparison_pairs": summary["comparison_pairs"],
        }
    )

with (HERE / "summary.csv").open("w", newline="") as stream:
    writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)

metadata_fields = (
    "rust_source_revision", "rust_toolchain", "rust_flags", "rust_binary_sha256",
    "ducc_revision", "ducc_version", "ducc_compiler", "ducc_flags",
    "ducc_binary_sha256", "cpu_model", "q1_cpus", "q8_cpus", "q8_topology",
    "q8_l3_ids", "affinity_policy", "measurement", "call_boundary",
    "case_count", "smoke",
)
metadata = {
    key: result["metadata"][key]
    for key in metadata_fields
    if key in result["metadata"]
}
metadata["source_commit_note"] = (
    "Production code is at 32bb0a2f; recorded HEAD 1033d499 contains "
    "documentation-only changes."
)
summary_fields = (
    "case_id", "grid", "lmax", "mmax", "ntheta", "nphi", "latitude_mode",
    "longitude_mode", "operation", "threads", "rust_ms", "ducc_ms",
    "paired_ratio_median", "max_relative_l2", "comparison_pairs",
)
clean_summaries = [
    {key: row[key] for key in summary_fields if key in row}
    for row in result["summaries"]
]
clean = {
    "metadata": metadata,
    "cases": [
        {
            key: case[key]
            for key in (
                "case_id", "grid", "lmax", "mmax", "ntheta", "nphi",
                "latitude_mode", "longitude_mode", "seed", "input_sha256",
            )
        }
        for case in result["cases"]
    ],
    "summaries": clean_summaries,
    "comparison_count": len(result["comparisons"]),
    "failure_count": len(result["failures"]),
}
(HERE / "summary.json").write_text(json.dumps(clean, indent=2, allow_nan=False) + "\n")
