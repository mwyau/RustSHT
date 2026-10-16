# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

import csv
import re
from pathlib import Path


root = Path(__file__).parent / "batch-T720"
columns = [
    "lmax",
    "ntheta",
    "nphi",
    "spin",
    "operation",
    "threads",
    "fields",
    "schedule",
    "workspace",
    "plan_construction_ms",
    "workspace_allocation_ms",
    "workspace_rss_delta_kib",
    "peak_rss_kib",
    "warm_batch_median_ms",
    "warm_per_field_median_ms",
    "fields_per_second",
    "output_checksum",
    "wall_seconds",
]


def extract(pattern: str, text: str) -> str:
    match = re.search(pattern, text, re.MULTILINE)
    if match is None:
        raise ValueError(f"missing value for {pattern}")
    return match.group(1)


rows = []
for log_path in sorted(root.glob("*.log")):
    text = log_path.read_text()
    name = re.search(
        r"spin=(\d+) T(\d+) GL (\d+)x(\d+) mmax=\d+ f64 "
        r"operation=(\w+) threads=(\d+) fields=(\d+) schedule=(\S+) "
        r"workspace=(\S+)",
        text,
    )
    if name is None:
        raise ValueError(f"could not parse parameters in {log_path}")
    spin, lmax, ntheta, nphi, operation, threads, fields, schedule, workspace = name.groups()
    time_text = log_path.with_suffix(".time").read_text()
    rows.append(
        {
            "lmax": lmax,
            "ntheta": ntheta,
            "nphi": nphi,
            "spin": spin,
            "operation": operation,
            "threads": threads,
            "fields": fields,
            "schedule": schedule,
            "workspace": workspace,
            "plan_construction_ms": extract(r"^plan_construction_ms=([\d.]+)", text),
            "workspace_allocation_ms": extract(r"^workspace_allocation_ms=([\d.]+)", text),
            "workspace_rss_delta_kib": extract(r"workspace_rss_delta_kib=(\d+)", text),
            "peak_rss_kib": extract(r"peak_rss_kib=(\d+)", time_text),
            "warm_batch_median_ms": extract(r"^warm_batch_median_ms=([\d.]+)", text),
            "warm_per_field_median_ms": extract(r"^warm_per_field_median_ms=([\d.]+)", text),
            "fields_per_second": extract(r"fields_per_second=([\d.]+)", text),
            "output_checksum": extract(r"^output_checksum=([\deE+.-]+)", text),
            "wall_seconds": extract(r"wall_seconds=([\d.]+)", time_text),
        }
    )

with (root / "summary.csv").open("w", newline="") as output:
    writer = csv.DictWriter(output, fieldnames=columns)
    writer.writeheader()
    writer.writerows(rows)
