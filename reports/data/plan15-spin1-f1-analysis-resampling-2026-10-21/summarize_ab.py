#!/usr/bin/env python3
"""Build the compact paired summary CSV from the archived raw result JSONs."""

from __future__ import annotations

import csv
import json
from pathlib import Path


HERE = Path(__file__).resolve().parent
baseline = json.loads((HERE / "baseline-results.json").read_text())
candidate = json.loads((HERE / "candidate-results.json").read_text())


def keyed(data: dict) -> dict[tuple[str, str, int], dict]:
    return {
        (row["case_id"], row["operation"], row["threads"]): row
        for row in data["summaries"]
    }


before = keyed(baseline)
after = keyed(candidate)
fields = [
    "case_id",
    "operation",
    "threads",
    "baseline_rust_ms",
    "candidate_rust_ms",
    "baseline_ducc_ms",
    "candidate_ducc_ms",
    "baseline_rust_ducc_ratio",
    "candidate_rust_ducc_ratio",
    "candidate_over_baseline",
    "candidate_max_relative_l2",
]

rows = []
for key in sorted(after):
    old, new = before[key], after[key]
    rows.append(
        {
            "case_id": key[0],
            "operation": key[1],
            "threads": key[2],
            "baseline_rust_ms": old["rust_ms"]["median"],
            "candidate_rust_ms": new["rust_ms"]["median"],
            "baseline_ducc_ms": old["ducc_ms"]["median"],
            "candidate_ducc_ms": new["ducc_ms"]["median"],
            "baseline_rust_ducc_ratio": old["paired_ratio_median"],
            "candidate_rust_ducc_ratio": new["paired_ratio_median"],
            "candidate_over_baseline": (
                new["rust_ms"]["median"] / old["rust_ms"]["median"]
            ),
            "candidate_max_relative_l2": new["max_relative_l2"],
        }
    )

with (HERE / "summary.csv").open("w", newline="") as output:
    writer = csv.DictWriter(output, fieldnames=fields, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
