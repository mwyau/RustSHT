#!/usr/bin/env python3
"""Summarize alternating fresh-process longitude-size comparisons."""

# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

import csv
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parent / "native" / "paired-longitudes"
OUT = ROOT / "summary.csv"
pattern = re.compile(r"(.+)-pair(\d+)-T(\d+)-nphi(\d+)-spin([01])\.log")
metric_pattern = re.compile(r"([a-z0-9_]+)=([0-9.]+)")
fields = [
    "comparison",
    "pair",
    "lmax",
    "nphi",
    "spin",
    "threads",
    "operation",
    "median_ms",
    "first_call_ms",
    "plan_construction_ms",
    "workspace_allocation_ms",
    "peak_rss_kib",
    "process_wall_seconds",
    "quality",
]
rows = []

for log in sorted(ROOT.glob("*.log")):
    match = pattern.fullmatch(log.name)
    if not match:
        continue
    comparison, pair, lmax, nphi, spin = match.groups()
    metrics = dict((key, float(value)) for key, value in metric_pattern.findall(log.read_text()))
    timing_path = log.with_suffix(".time")
    timing = dict(
        (key, float(value))
        for key, value in metric_pattern.findall(timing_path.read_text())
    )
    quality = "screen-paired"
    if (comparison, pair, lmax, nphi, spin) == ("odd-vs-even", "1", "2047", "4096", "1"):
        quality = "possible placement overlap"
    if pair in {"3", "4"} and lmax == "2047" and spin == "1":
        quality = "worker-sized placement rerun"
    for operation in ("synthesis", "analysis"):
        for threads, suffix in ((1, "one_thread"), (8, "rayon")):
            median_key = f"{operation}_{suffix}_ms"
            first_key = f"{operation}_first_call_{suffix}_ms"
            if median_key not in metrics:
                raise SystemExit(f"missing {median_key} in {log}")
            rows.append(
                {
                    "comparison": comparison,
                    "pair": int(pair),
                    "lmax": int(lmax),
                    "nphi": int(nphi),
                    "spin": int(spin),
                    "threads": threads,
                    "operation": operation,
                    "median_ms": metrics[median_key],
                    "first_call_ms": metrics.get(first_key, ""),
                    "plan_construction_ms": metrics.get("plan_construction_ms", ""),
                    "workspace_allocation_ms": metrics.get("workspace_allocation_ms", ""),
                    "peak_rss_kib": timing.get("peak_rss_kib", ""),
                    "process_wall_seconds": timing.get("wall_seconds", ""),
                    "quality": quality,
                }
            )

with OUT.open("w", newline="") as destination:
    writer = csv.DictWriter(destination, fieldnames=fields)
    writer.writeheader()
    writer.writerows(rows)

print(f"wrote {len(rows)} operation/thread rows to {OUT}")
