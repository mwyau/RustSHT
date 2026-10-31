#!/usr/bin/env python3
"""Recompute paired medians and deterministic percentile-bootstrap intervals."""

from __future__ import annotations

import json
import gzip
import random
import statistics
from pathlib import Path


ROOT = Path(__file__).resolve().parent
RESAMPLES = 50_000
BASE_SEED = 20261030


def median_ci(values: list[float], seed: int) -> tuple[float, float, float]:
    rng = random.Random(seed)
    med = statistics.median
    sampled = sorted(
        med([values[rng.randrange(len(values))] for _ in values])
        for _ in range(RESAMPLES)
    )
    return med(values), sampled[int(0.025 * RESAMPLES)], sampled[int(0.975 * RESAMPLES)]


def report(folder: str, metrics: tuple[str, ...]) -> None:
    with gzip.open(ROOT / folder / "metadata.json.gz", "rt") as source:
        manifest = json.load(source)
    for case_index, entry in enumerate(manifest["cases"]):
        summary = entry["summary"]
        print(summary["case_id"])
        for metric_index, metric in enumerate(metrics):
            values = summary["paired_ratios"].get(metric)
            if values is None:
                continue
            med, lower, upper = median_ci(values, BASE_SEED + 100 * case_index + metric_index)
            print(f"  {metric}: {med:.6f} [{lower:.6f}, {upper:.6f}]")
        rows = summary["pair_workers"]
        if "candidate_vs_parent_f32" in rows[0]["output_rel_l2"]:
            errors = [row["output_rel_l2"]["candidate_vs_parent_f32"] for row in rows]
            print(f"  candidate_vs_parent_f32_max_rel_l2: {max(errors):.3e}")
        if "candidate_f64_vs_parent_f64" in rows[0]["output_rel_l2"]:
            errors = [row["output_rel_l2"]["candidate_f64_vs_parent_f64"] for row in rows]
            print(f"  candidate_f64_vs_parent_f64_max_rel_l2: {max(errors):.3e}")


if __name__ == "__main__":
    report(
        "b3-15pair",
        (
            "ratio_candidate_over_parent",
            "ratio_f32_over_f64",
            "ratio_rust_f32_over_ducc_f32",
            "ratio_rust_f64_over_ducc_f64",
        ),
    )
    report("b1-full-q8-control-15pair", ("ratio_candidate_over_parent",))
    report("b3-f64-control-3pair", ("ratio_f64_candidate_over_parent_f64",))
