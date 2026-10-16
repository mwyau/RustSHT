#!/usr/bin/env python3
"""Summarize M26 six-pair data with exhaustive paired bootstrap intervals."""

from __future__ import annotations

import csv
import itertools
import io
import json
import math
import statistics
import sys
from pathlib import Path
from typing import Any


def rows_from_stdout(output: str) -> dict[tuple[str, int, int], dict[str, float]]:
    rows: dict[tuple[str, int, int], dict[str, float]] = {}
    for record in csv.DictReader(io.StringIO(output)):
        if not record.get("mode") or not record.get("n"):
            continue
        mode = record["mode"]
        length = int(record["n"])
        count = int(record.get("batch_count") or 1)
        if count > 1 and record.get("batch_operation_ns"):
            elapsed_ns = float(record["batch_operation_ns"]) / count
        else:
            elapsed_ns = float(record["public_operation_ns"])
        workspace_ns = record.get("workspace_ns") or record.get("scratch_ns") or "0"
        workspace_bytes = (
            record.get("workspace_bytes")
            or record.get("scratch_bytes")
            or record.get("kernel_scratch_bytes")
            or "0"
        )
        rows[(mode, length, count)] = {
            "operation_ns": elapsed_ns,
            "plan_ns": float(record.get("plan_ns") or 0),
            "workspace_ns": float(workspace_ns),
            "workspace_bytes": float(workspace_bytes),
        }
    return rows


def exact_interval(values: list[float]) -> tuple[float, float]:
    if len(values) != 6:
        raise ValueError("exact intervals require six paired speedups")
    samples = sorted(statistics.median(sample) for sample in itertools.product(values, repeat=6))

    def percentile(q: float) -> float:
        position = q * (len(samples) - 1)
        lower = math.floor(position)
        upper = math.ceil(position)
        fraction = position - lower
        return samples[lower] + (samples[upper] - samples[lower]) * fraction

    return percentile(0.025), percentile(0.975)


def summarize(config_path: Path, input_path: Path) -> list[dict[str, Any]]:
    config = json.loads(config_path.read_text())
    candidate_label = str(config["a"]["label"])
    reference_label = str(config["b"]["label"])
    records = [json.loads(line) for line in input_path.read_text().splitlines()]
    pair_count = int(records[0]["pairs"])
    if pair_count != 6:
        raise ValueError(f"expected six pairs, found {pair_count}")
    sides: dict[str, dict[int, dict[tuple[str, int, int], dict[str, float]]]] = {
        "a": {},
        "b": {},
    }
    for process in records[1:]:
        label = process["label"]
        if label == candidate_label:
            side = "a"
        elif label == reference_label:
            side = "b"
        else:
            raise ValueError(f"unexpected comparison label: {label}")
        if process["exit_code"] != 0:
            raise ValueError(f"nonzero exit code for {label}, pair {process['pair']}")
        sides[side][int(process["pair"])] = rows_from_stdout(process["stdout"])
    if any(len(sides[side]) != pair_count for side in ("a", "b")):
        raise ValueError("incomplete process pairs")

    common_keys = set.intersection(
        *(set(rows) for side in sides.values() for rows in side.values())
    )
    if len(common_keys) != 34:
        raise ValueError(f"expected all 34 benchmark cells, found {len(common_keys)}")

    result = []
    for key in sorted(common_keys):
        ratios = [
            sides["b"][pair][key]["operation_ns"]
            / sides["a"][pair][key]["operation_ns"]
            for pair in range(1, pair_count + 1)
        ]
        low, high = exact_interval(ratios)
        candidate_records = [sides["a"][pair][key] for pair in range(1, pair_count + 1)]
        reference_records = [sides["b"][pair][key] for pair in range(1, pair_count + 1)]
        result.append({
            "comparison_file": input_path.name,
            "candidate": candidate_label,
            "reference": reference_label,
            "mode": key[0],
            "n": key[1],
            "rows": key[2],
            "pairs": pair_count,
            "speedup_median_reference_over_candidate": statistics.median(ratios),
            "paired_speedup_min": min(ratios),
            "paired_speedup_max": max(ratios),
            "bootstrap_95_low": low,
            "bootstrap_95_high": high,
            "candidate_ns_median": statistics.median(r["operation_ns"] for r in candidate_records),
            "reference_ns_median": statistics.median(r["operation_ns"] for r in reference_records),
            "candidate_plan_ns_median": statistics.median(r["plan_ns"] for r in candidate_records),
            "reference_plan_ns_median": statistics.median(r["plan_ns"] for r in reference_records),
            "candidate_workspace_ns_median": statistics.median(r["workspace_ns"] for r in candidate_records),
            "reference_workspace_ns_median": statistics.median(r["workspace_ns"] for r in reference_records),
            "candidate_workspace_bytes": statistics.median(r["workspace_bytes"] for r in candidate_records),
            "reference_workspace_bytes": statistics.median(r["workspace_bytes"] for r in reference_records),
            "paired_speedups": ";".join(f"{ratio:.6g}" for ratio in ratios),
        })
    return result


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: summarize_m25_exact_pairs.py CONFIG.json INPUT.jsonl OUTPUT.csv", file=sys.stderr)
        return 2
    config_path, input_path, output_path = map(Path, sys.argv[1:])
    rows = summarize(config_path, input_path)
    if output_path.exists():
        raise FileExistsError(f"refusing to overwrite {output_path}")
    with output_path.open("x", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    print(f"wrote {len(rows)} benchmark cells to {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
