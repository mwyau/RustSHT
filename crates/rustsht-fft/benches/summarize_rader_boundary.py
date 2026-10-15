#!/usr/bin/env python3
"""Summarize the M4 Rader target gate from paired-process CSV rows."""

from __future__ import annotations

import argparse
import csv
import random
import statistics
import zlib
from pathlib import Path


def summarize(path: Path) -> dict[str, str | int | float]:
    with path.open(newline="") as stream:
        rows = list(csv.DictReader(stream))
    primary = []
    controls = []
    for row in rows:
        mode = row["mode"]
        length = int(row["n"])
        class_name = mode.rsplit("_", maxsplit=1)[-1]
        is_primary = (
            length in (257, 723)
            and mode.startswith(("r2c_", "c2r_"))
            and class_name in ("ordinary", "sparse", "cancellation")
        )
        (primary if is_primary else controls).append(row)

    if len(primary) != 24:
        raise SystemExit(f"expected 24 target cells, found {len(primary)}")
    pair_count = int(primary[0]["pairs"])
    if any(int(row["pairs"]) != pair_count for row in rows):
        raise SystemExit("pair count differs between cells")

    paired = [
        [float(value) for value in row["paired_speedups"].split(";")]
        for row in primary
    ]
    pair_level = [
        statistics.median(cell[pair] for cell in paired)
        for pair in range(pair_count)
    ]
    aggregate = statistics.median(pair_level)
    rng = random.Random(zlib.crc32(path.name.encode()))
    bootstrap = sorted(
        statistics.median(rng.choices(pair_level, k=pair_count))
        for _ in range(5000)
    )
    low, high = bootstrap[124], bootstrap[4874]

    target_min = min(float(row["speedup_median_reference_over_candidate"]) for row in primary)
    control_min = min(
        float(row["speedup_median_reference_over_candidate"]) for row in controls
    )
    target_slowdown = max(0.0, (1.0 / target_min - 1.0) * 100.0)
    control_slowdown = max(0.0, (1.0 / control_min - 1.0) * 100.0)
    workspace_match = all(
        float(row["candidate_workspace_bytes"])
        == float(row["reference_workspace_bytes"])
        for row in rows
    )
    passed = (
        aggregate > 1.0
        and low > 1.0
        and target_slowdown <= 5.0
        and control_slowdown <= 5.0
        and workspace_match
    )
    return {
        "source_summary": path.name,
        "primary_cells": len(primary),
        "pair_count": pair_count,
        "pair_level_primary_speedups": ";".join(f"{value:.6f}" for value in pair_level),
        "primary_aggregate_median": aggregate,
        "bootstrap_95_low": low,
        "bootstrap_95_high": high,
        "max_target_cell_slowdown_pct": target_slowdown,
        "max_control_or_fallback_slowdown_pct": control_slowdown,
        "workspace_bytes_match": workspace_match,
        "gate_pass": passed,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("summary", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    result = summarize(args.summary)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(result), lineterminator="\n")
        writer.writeheader()
        writer.writerow(result)
    print(",".join(str(value) for value in result.values()))


if __name__ == "__main__":
    main()
