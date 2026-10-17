#!/usr/bin/env python3
"""Summarize retained P01 paired-process benchmark outputs."""

from __future__ import annotations

import csv
import random
import re
import statistics
from pathlib import Path


HERE = Path(__file__).resolve().parent
NAME = re.compile(
    r"t(?P<lmax>\d+)-s(?P<spin>\d+)-q(?P<threads>\d+)-"
    r"pair(?P<pair>\d+)-(?P<variant>baseline|candidate)\.stdout$"
)
SAMPLES = re.compile(r"(?P<op>synthesis|analysis)_rayon_samples_ms=\[([^]]+)\]")
RSS = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
EXIT = re.compile(r"Exit status:\s*(\d+)")
BOOTSTRAP_REPLICATES = 50_000
BOOTSTRAP_SEED = 2_026_101_7


def linear_quantile(values: list[float], probability: float) -> float:
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    low = int(position)
    high = min(low + 1, len(ordered) - 1)
    fraction = position - low
    return ordered[low] * (1 - fraction) + ordered[high] * fraction


def read_process(path: Path) -> dict:
    match = NAME.fullmatch(path.name)
    assert match, path
    metadata = {key: int(value) if key != "variant" else value for key, value in match.groupdict().items()}
    output = path.read_text()
    config = output.splitlines()[0]
    expected_geometry = f"GL {metadata['lmax'] + 1}x{2 * (metadata['lmax'] + 1)}"
    assert expected_geometry in config and "requested_only=true" in config and "phi0=0.37" in config, (path, config)
    sample_values: dict[str, list[float]] = {}
    for sample_match in SAMPLES.finditer(output):
        sample_values[sample_match.group("op")] = [
            float(value.strip()) for value in sample_match.group(2).split(",")
        ]
    assert set(sample_values) == {"synthesis", "analysis"}, (path, sample_values)
    assert all(len(values) == 5 for values in sample_values.values()), (path, sample_values)
    timing = path.with_suffix(".time").read_text()
    rss_match = RSS.search(timing)
    exit_match = EXIT.search(timing)
    assert rss_match and exit_match and exit_match.group(1) == "0", (path, timing)
    return {
        **metadata,
        "samples": sample_values,
        "rss_kib": int(rss_match.group(1)),
        "config": config,
    }


processes = {}
for stdout in HERE.glob("t*-s*-q*-pair*-*.stdout"):
    process = read_process(stdout)
    key = (process["lmax"], process["spin"], process["threads"], process["pair"], process["variant"])
    assert key not in processes, key
    processes[key] = process

cells = sorted({key[:3] for key in processes})
expected_cells = {(511, 1, 1)}
assert set(cells) == expected_cells, cells

process_rows: list[dict] = []
summary_rows: list[dict] = []
rng = random.Random(BOOTSTRAP_SEED)

for cell in cells:
    pair_ids = sorted({key[3] for key in processes if key[:3] == cell})
    assert pair_ids == [1, 2, 3, 4, 5, 6], (cell, pair_ids)
    lmax, spin, threads = cell
    for operation in ("synthesis", "analysis"):
        pair_ratios = []
        baseline_medians = []
        candidate_medians = []
        baseline_all_samples = []
        candidate_all_samples = []
        rss_deltas = []
        for pair in pair_ids:
            baseline = processes[(*cell, pair, "baseline")]
            candidate = processes[(*cell, pair, "candidate")]
            base_values = baseline["samples"][operation]
            cand_values = candidate["samples"][operation]
            base_median = statistics.median(base_values)
            cand_median = statistics.median(cand_values)
            ratio = cand_median / base_median
            pair_ratios.append(ratio)
            baseline_medians.append(base_median)
            candidate_medians.append(cand_median)
            baseline_all_samples.extend(base_values)
            candidate_all_samples.extend(cand_values)
            rss_delta = candidate["rss_kib"] - baseline["rss_kib"]
            rss_deltas.append(rss_delta)
            process_rows.append(
                {
                    "lmax": lmax,
                    "spin": spin,
                    "threads": threads,
                    "operation": operation,
                    "pair": pair,
                    "baseline_first": "true" if pair % 2 == 1 else "false",
                    "baseline_samples_ms": ";".join(f"{v:.3f}" for v in base_values),
                    "candidate_samples_ms": ";".join(f"{v:.3f}" for v in cand_values),
                    "baseline_median_ms": f"{base_median:.6f}",
                    "candidate_median_ms": f"{cand_median:.6f}",
                    "candidate_over_baseline": f"{ratio:.8f}",
                    "baseline_rss_kib": baseline["rss_kib"],
                    "candidate_rss_kib": candidate["rss_kib"],
                    "rss_delta_kib": rss_delta,
                }
            )

        bootstrap_medians = []
        for _ in range(BOOTSTRAP_REPLICATES):
            resample = [rng.choice(pair_ratios) for _ in pair_ratios]
            bootstrap_medians.append(statistics.median(resample))
        summary_rows.append(
            {
                "lmax": lmax,
                "spin": spin,
                "threads": threads,
                "operation": operation,
                "n_pairs": len(pair_ratios),
                "baseline_median_ms": f"{statistics.median(baseline_medians):.6f}",
                "candidate_median_ms": f"{statistics.median(candidate_medians):.6f}",
                "median_paired_ratio": f"{statistics.median(pair_ratios):.8f}",
                "bootstrap95_low": f"{linear_quantile(bootstrap_medians, 0.025):.8f}",
                "bootstrap95_high": f"{linear_quantile(bootstrap_medians, 0.975):.8f}",
                "paired_ratio_min": f"{min(pair_ratios):.8f}",
                "paired_ratio_max": f"{max(pair_ratios):.8f}",
                "baseline_sample_min_ms": f"{min(baseline_all_samples):.3f}",
                "baseline_sample_max_ms": f"{max(baseline_all_samples):.3f}",
                "candidate_sample_min_ms": f"{min(candidate_all_samples):.3f}",
                "candidate_sample_max_ms": f"{max(candidate_all_samples):.3f}",
                "median_baseline_rss_kib": int(statistics.median(
                    processes[(*cell, pair, "baseline")]["rss_kib"] for pair in pair_ids
                )),
                "median_candidate_rss_kib": int(statistics.median(
                    processes[(*cell, pair, "candidate")]["rss_kib"] for pair in pair_ids
                )),
                "median_paired_rss_delta_kib": int(statistics.median(rss_deltas)),
            }
        )


def write_csv(path: Path, rows: list[dict]) -> None:
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


write_csv(HERE / "process-pairs.csv", process_rows)
write_csv(HERE / "summary.csv", summary_rows)
print(f"wrote {len(process_rows)} operation-pair rows and {len(summary_rows)} summary rows")
