#!/usr/bin/env python3
"""Summarize paired first-call and warmed-call FFT benchmark output."""

from __future__ import annotations

import argparse
import csv
import io
import json
import random
import statistics
import zlib
from pathlib import Path


def parse_output(output: str) -> dict[tuple[str, int], dict[str, float]]:
    parsed = {}
    for row in csv.DictReader(io.StringIO(output)):
        if not row.get("mode") or not row.get("n"):
            continue
        parsed[(row["mode"], int(row["n"]))] = {
            "first_call_ns": float(row["first_call_ns"]),
            "warm_call_median_ns": float(row["warm_call_median_ns"]),
            "workspace_bytes": float(row["workspace_bytes"]),
            "checksum": float(row["checksum"]),
        }
    return parsed


def interval(values: list[float], seed: int) -> tuple[float, float]:
    rng = random.Random(seed)
    estimates = sorted(
        statistics.median(rng.choices(values, k=len(values))) for _ in range(5000)
    )
    return estimates[124], estimates[4874]


def summarize(path: Path) -> list[dict[str, int | float | str]]:
    records = [json.loads(line) for line in path.read_text().splitlines()]
    pair_count = int(records[0]["pairs"])
    labels = list(dict.fromkeys(record["label"] for record in records[1:]))
    if len(labels) != 2:
        raise SystemExit(f"expected two process labels in {path}")
    candidate, baseline = labels
    sides = {candidate: {}, baseline: {}}
    for record in records[1:]:
        if record["exit_code"] != 0:
            raise SystemExit(f"nonzero process status in {path}")
        sides[record["label"]][int(record["pair"])] = parse_output(record["stdout"])
    if any(len(sides[label]) != pair_count for label in labels):
        raise SystemExit(f"expected {pair_count} complete pairs in {path}")

    keys = set.intersection(
        *(set(rows) for label in labels for rows in sides[label].values())
    )
    summaries = []
    for key in sorted(keys):
        values = {}
        for metric in ("first_call_ns", "warm_call_median_ns"):
            ratios = [
                sides[baseline][pair][key][metric]
                / sides[candidate][pair][key][metric]
                for pair in range(1, pair_count + 1)
            ]
            seed = zlib.crc32(f"{path.name}:{key}:{metric}".encode())
            low, high = interval(ratios, seed)
            values[metric] = (statistics.median(ratios), low, high)

        first_speedup, first_low, first_high = values["first_call_ns"]
        warm_speedup, warm_low, warm_high = values["warm_call_median_ns"]
        candidate_rows = [sides[candidate][pair][key] for pair in range(1, pair_count + 1)]
        baseline_rows = [sides[baseline][pair][key] for pair in range(1, pair_count + 1)]
        summaries.append(
            {
                "comparison_file": path.name,
                "candidate": candidate,
                "reference": baseline,
                "mode": key[0],
                "n": key[1],
                "pairs": pair_count,
                "candidate_first_ns_median": statistics.median(
                    row["first_call_ns"] for row in candidate_rows
                ),
                "baseline_first_ns_median": statistics.median(
                    row["first_call_ns"] for row in baseline_rows
                ),
                "first_speedup_median": first_speedup,
                "first_bootstrap_95_low": first_low,
                "first_bootstrap_95_high": first_high,
                "candidate_warm_ns_median": statistics.median(
                    row["warm_call_median_ns"] for row in candidate_rows
                ),
                "baseline_warm_ns_median": statistics.median(
                    row["warm_call_median_ns"] for row in baseline_rows
                ),
                "warm_speedup_median": warm_speedup,
                "warm_bootstrap_95_low": warm_low,
                "warm_bootstrap_95_high": warm_high,
                "candidate_workspace_bytes": statistics.median(
                    row["workspace_bytes"] for row in candidate_rows
                ),
                "baseline_workspace_bytes": statistics.median(
                    row["workspace_bytes"] for row in baseline_rows
                ),
                "max_checksum_abs_difference": max(
                    abs(candidate_rows[index]["checksum"] - baseline_rows[index]["checksum"])
                    for index in range(pair_count)
                ),
            }
        )
    return summaries


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("inputs", nargs="+", type=Path)
    args = parser.parse_args()
    summaries = [row for path in args.inputs for row in summarize(path)]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="") as stream:
        writer = csv.DictWriter(
            stream, fieldnames=list(summaries[0]), lineterminator="\n"
        )
        writer.writeheader()
        writer.writerows(summaries)


if __name__ == "__main__":
    main()
