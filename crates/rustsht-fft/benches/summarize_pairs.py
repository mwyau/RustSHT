#!/usr/bin/env python3
"""Summarize alternating process-pair FFT benchmark output."""

from __future__ import annotations

import argparse
import csv
import io
import json
import random
import statistics
import zlib
from pathlib import Path


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
        elif record.get("public_operation_ns"):
            elapsed_ns = float(record["public_operation_ns"])
        else:
            elapsed_ns = float(record["call_ns"])
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


def bootstrap_interval(values: list[float], seed: int) -> tuple[float, float]:
    rng = random.Random(seed)
    estimates = sorted(
        statistics.median(rng.choices(values, k=len(values)))
        for _ in range(5000)
    )
    return estimates[124], estimates[4874]


def summarize(path: Path) -> list[dict[str, str | int | float]]:
    records = [json.loads(line) for line in path.read_text().splitlines()]
    environment = records[0]
    pair_count = int(environment["pairs"])
    processes = records[1:]
    if any(process["exit_code"] != 0 for process in processes):
        raise SystemExit(f"nonzero process status in {path}")

    sides: dict[
        str,
        dict[int, dict[tuple[str, int, int], dict[str, float]]],
    ] = {
        "a": {},
        "b": {},
    }
    labels = {"a": "", "b": ""}
    for process in processes:
        label = process["label"]
        if process["command"] == []:
            raise SystemExit(f"empty process command in {path}")
        if label == labels["a"]:
            side = "a"
        elif label == labels["b"]:
            side = "b"
        elif not labels["a"]:
            labels["a"] = label
            side = "a"
        elif not labels["b"]:
            labels["b"] = label
            side = "b"
        else:
            raise SystemExit(f"unexpected label in {path}: {label}")
        sides[side][int(process["pair"])] = rows_from_stdout(process["stdout"])

    if not labels["a"] or not labels["b"]:
        raise SystemExit(f"expected two labels in {path}")

    if len(sides["a"]) != pair_count or len(sides["b"]) != pair_count:
        raise SystemExit(f"expected {pair_count} complete pairs in {path}")
    reference_label = labels["b"]
    keys = set.intersection(*(set(rows) for rows in [*sides["a"].values(), *sides["b"].values()]))
    summaries = []
    for key in sorted(keys):
        ratios = [
            sides["b"][pair][key]["operation_ns"]
            / sides["a"][pair][key]["operation_ns"]
            for pair in range(1, pair_count + 1)
        ]
        median_ratio = statistics.median(ratios)
        seed = zlib.crc32(f"{path.name}:{key}".encode())
        low, high = bootstrap_interval(ratios, seed)
        candidate_records = [sides["a"][pair][key] for pair in range(1, pair_count + 1)]
        reference_records = [sides["b"][pair][key] for pair in range(1, pair_count + 1)]
        candidate_times = [record["operation_ns"] for record in candidate_records]
        reference_times = [record["operation_ns"] for record in reference_records]
        summaries.append(
            {
                "comparison_file": path.name,
                "candidate": labels["a"],
                "reference": reference_label,
                "mode": key[0],
                "n": key[1],
                "rows": key[2],
                "pairs": pair_count,
                "speedup_median_reference_over_candidate": median_ratio,
                "paired_speedup_min": min(ratios),
                "paired_speedup_max": max(ratios),
                "bootstrap_95_low": low,
                "bootstrap_95_high": high,
                "candidate_ns_median": statistics.median(candidate_times),
                "reference_ns_median": statistics.median(reference_times),
                "candidate_plan_ns_median": statistics.median(
                    record["plan_ns"] for record in candidate_records
                ),
                "reference_plan_ns_median": statistics.median(
                    record["plan_ns"] for record in reference_records
                ),
                "candidate_workspace_ns_median": statistics.median(
                    record["workspace_ns"] for record in candidate_records
                ),
                "reference_workspace_ns_median": statistics.median(
                    record["workspace_ns"] for record in reference_records
                ),
                "candidate_workspace_bytes": statistics.median(
                    record["workspace_bytes"] for record in candidate_records
                ),
                "reference_workspace_bytes": statistics.median(
                    record["workspace_bytes"] for record in reference_records
                ),
                "paired_speedups": ";".join(f"{ratio:.6g}" for ratio in ratios),
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
