#!/usr/bin/env python3
"""Summarize the retained m1b_bench atmospheric-resolution screen."""

# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

import csv
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parent
NATIVE = ROOT / "native"
OUT = NATIVE / "summary.csv"


def values(path: Path) -> dict[str, float]:
    found = {}
    for line in path.read_text().splitlines():
        match = re.fullmatch(r"([a-z0-9_]+)=([0-9.]+)", line.strip())
        if match:
            found[match.group(1)] = float(match.group(2))
    return found


with (ROOT / "grid-matrix.csv").open(newline="") as source:
    grids = list(csv.DictReader(source))

fields = [
    "lmax",
    "ntheta",
    "mmax",
    "nphi",
    "sampling",
    "scope",
    "spin",
    "threads",
    "operation",
    "median_ms",
    "first_call_ms",
    "plan_construction_ms",
    "workspace_allocation_ms",
    "peak_rss_kib",
    "process_wall_seconds",
]
rows = []
missing = []

for grid in grids:
    for spin in (0, 1):
        label = f"T{grid['lmax']}-nphi{grid['nphi']}-spin{spin}-q1q8"
        log = NATIVE / f"{label}.log"
        timing = NATIVE / f"{label}.time"
        if not log.is_file() or not timing.is_file():
            missing.append(label)
            continue
        metrics = values(log)
        wall = values(timing)
        for operation in ("synthesis", "analysis"):
            for threads, suffix in ((1, "one_thread"), (8, "rayon")):
                median_key = f"{operation}_{suffix}_ms"
                first_key = f"{operation}_first_call_{suffix}_ms"
                if median_key not in metrics:
                    missing.append(f"{label}:{median_key}")
                    continue
                rows.append(
                    {
                        "lmax": grid["lmax"],
                        "ntheta": grid["ntheta"],
                        "mmax": grid["mmax"],
                        "nphi": grid["nphi"],
                        "sampling": grid["sampling"],
                        "scope": grid["scope"],
                        "spin": spin,
                        "threads": threads,
                        "operation": operation,
                        "median_ms": metrics[median_key],
                        "first_call_ms": metrics.get(first_key, ""),
                        "plan_construction_ms": metrics.get("plan_construction_ms", ""),
                        "workspace_allocation_ms": metrics.get("workspace_allocation_ms", ""),
                        "peak_rss_kib": wall.get("peak_rss_kib", ""),
                        "process_wall_seconds": wall.get("wall_seconds", ""),
                    }
                )

if missing:
    raise SystemExit("missing benchmark records: " + ", ".join(missing))

with OUT.open("w", newline="") as destination:
    writer = csv.DictWriter(destination, fieldnames=fields)
    writer.writeheader()
    writer.writerows(rows)

print(f"wrote {len(rows)} operation/thread rows from {len(grids)} grids to {OUT}")
