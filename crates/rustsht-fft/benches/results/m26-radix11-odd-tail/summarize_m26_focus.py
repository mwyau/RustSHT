#!/usr/bin/env python3
"""Summarize a six-pair focused C2C capture with exact bootstrap intervals."""

from __future__ import annotations

import csv
import itertools
import json
import math
import statistics
import sys
from pathlib import Path


def exact_interval(values: list[float]) -> tuple[float, float]:
    samples = sorted(statistics.median(sample) for sample in itertools.product(values, repeat=6))

    def percentile(q: float) -> float:
        position = q * (len(samples) - 1)
        low = math.floor(position)
        high = math.ceil(position)
        return samples[low] + (samples[high] - samples[low]) * (position - low)

    return percentile(0.025), percentile(0.975)


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: summarize_m26_focus.py CONFIG.json INPUT.jsonl", file=sys.stderr)
        return 2
    config = json.loads(Path(sys.argv[1]).read_text())
    records = [json.loads(line) for line in Path(sys.argv[2]).read_text().splitlines()]
    if int(records[0]["pairs"]) != 6:
        raise ValueError("expected six pairs")
    labels = {config[side]["label"]: side for side in ("a", "b")}
    values: dict[tuple[str, int, int], dict[str, dict[int, float]]] = {}
    for process in records[1:]:
        if process["exit_code"] != 0:
            raise ValueError(f"nonzero exit code in pair {process['pair']}")
        side = labels[process["label"]]
        for row in csv.DictReader(process["stdout"].splitlines()):
            key = (row["mode"], int(row["n"]), int(row["batch_count"]))
            values.setdefault(key, {"a": {}, "b": {}})[side][int(process["pair"])] = (
                float(row["batch_operation_ns"]) / int(row["batch_count"])
            )

    for key, sides in sorted(values.items()):
        ratios = [sides["b"][pair] / sides["a"][pair] for pair in range(1, 7)]
        low, high = exact_interval(ratios)
        print(
            f"{key[0]} n={key[1]} rows={key[2]} speedup={statistics.median(ratios):.6f} "
            f"95%=[{low:.6f},{high:.6f}] candidate_ns="
            f"{statistics.median(sides['a'][pair] for pair in range(1, 7)):.3f} "
            f"reference_ns={statistics.median(sides['b'][pair] for pair in range(1, 7)):.3f} "
            f"paired=" + ";".join(f"{value:.6g}" for value in ratios)
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
