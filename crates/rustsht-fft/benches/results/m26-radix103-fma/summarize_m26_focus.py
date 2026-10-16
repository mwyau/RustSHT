#!/usr/bin/env python3
"""Summarize a six-pair N=1442 C2C screen with exact bootstrap intervals."""

from __future__ import annotations

import csv
import itertools
import json
import math
import statistics
import sys
from pathlib import Path


def interval(values: list[float]) -> tuple[float, float]:
    samples = sorted(statistics.median(sample) for sample in itertools.product(values, repeat=6))

    def percentile(q: float) -> float:
        position = q * (len(samples) - 1)
        lower = math.floor(position)
        upper = math.ceil(position)
        fraction = position - lower
        return samples[lower] + (samples[upper] - samples[lower]) * fraction

    return percentile(0.025), percentile(0.975)


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: summarize_m26_focus.py CONFIG.json INPUT.jsonl", file=sys.stderr)
        return 2
    config = json.loads(Path(sys.argv[1]).read_text())
    path = Path(sys.argv[2])
    records = [json.loads(line) for line in path.read_text().splitlines()]
    if int(records[0]["pairs"]) != 6:
        raise ValueError("expected six pairs")
    labels = {config[side]["label"]: side for side in ("a", "b")}
    values: dict[tuple[str, int], dict[str, dict[int, dict[str, float]]]] = {
        ("c2c_forward", 721): {"a": {}, "b": {}},
        ("c2c_inverse", 721): {"a": {}, "b": {}},
    }
    for record in records[1:]:
        if record["exit_code"] != 0:
            raise ValueError(f"nonzero exit code in pair {record['pair']}")
        side = labels[record["label"]]
        for row in csv.DictReader(record["stdout"].splitlines()):
            key = (row["mode"], int(row["batch_count"]))
            values[key][side][int(record["pair"])] = {
                "operation_ns": float(row["batch_operation_ns"]) / int(row["batch_count"]),
                "checksum": float(row["checksum"]),
            }

    for key, sides in values.items():
        ratios = [sides["b"][pair]["operation_ns"] / sides["a"][pair]["operation_ns"] for pair in range(1, 7)]
        low, high = interval(ratios)
        print(
            f"{key[0]} rows={key[1]} speedup={statistics.median(ratios):.6f} "
            f"95%=[{low:.6f},{high:.6f}] candidate_ns="
            f"{statistics.median(sides['a'][p]['operation_ns'] for p in range(1, 7)):.3f} "
            f"reference_ns={statistics.median(sides['b'][p]['operation_ns'] for p in range(1, 7)):.3f} "
            f"paired=" + ";".join(f"{value:.6g}" for value in ratios)
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
