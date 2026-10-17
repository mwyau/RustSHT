#!/usr/bin/env python3
"""Summarize paired P02 scalar-synthesis whole-transform measurements."""

import csv
import random
import re
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
NAME = re.compile(
    r"t(?P<lmax>\d+)-s(?P<spin>\d+)-q1-pair(?P<pair>\d+)-"
    r"(?P<variant>baseline|candidate)\.stdout$"
)
SAMPLES = re.compile(r"(?P<op>synthesis|analysis)_rayon_samples_ms=\[([^]]+)\]")
RSS = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
EXIT = re.compile(r"Exit status:\s*(\d+)")


def read(path):
    match = NAME.fullmatch(path.name)
    assert match, path
    meta = {
        key: int(value) if key != "variant" else value
        for key, value in match.groupdict().items()
    }
    text = path.read_text()
    config = text.splitlines()[0]
    ntheta = meta["lmax"] + 1
    nphi = 2 * ntheta
    assert f"GL {ntheta}x{nphi}" in config
    assert "requested_only=true" in config and "phi0=0.37" in config
    values = {
        item.group("op"): [float(value) for value in item.group(2).split(",")]
        for item in SAMPLES.finditer(text)
    }
    assert set(values) == {"synthesis", "analysis"}
    assert all(len(samples) == 5 for samples in values.values())
    timing = path.with_suffix(".time").read_text()
    rss = RSS.search(timing)
    exit_status = EXIT.search(timing)
    assert rss and exit_status and exit_status.group(1) == "0"
    return {**meta, "values": values, "rss": int(rss.group(1))}


processes = {}
for path in HERE.glob("t*-s*-q1-pair*-*.stdout"):
    process = read(path)
    key = (
        process["lmax"],
        process["spin"],
        process["pair"],
        process["variant"],
    )
    assert key not in processes
    processes[key] = process

cells = sorted({key[:2] for key in processes})
expected_cells = {(511, 0), (511, 1), (1023, 0), (2047, 0)}
assert set(cells) == expected_cells, (cells, expected_cells)
pair_rows = []
summary_rows = []
rng = random.Random(20261017)


def quantile(values, p):
    values = sorted(values)
    position = (len(values) - 1) * p
    lower = int(position)
    upper = min(lower + 1, len(values) - 1)
    fraction = position - lower
    return values[lower] * (1 - fraction) + values[upper] * fraction


for cell in cells:
    pairs = sorted({key[2] for key in processes if key[:2] == cell})
    assert pairs == list(range(1, 7)), (cell, pairs)
    lmax, spin = cell
    for operation in ("synthesis", "analysis"):
        ratios = []
        baseline_medians = []
        candidate_medians = []
        rss_deltas = []
        baseline_samples = []
        candidate_samples = []
        for pair in pairs:
            baseline = processes[(*cell, pair, "baseline")]
            candidate = processes[(*cell, pair, "candidate")]
            baseline_values = baseline["values"][operation]
            candidate_values = candidate["values"][operation]
            baseline_median = statistics.median(baseline_values)
            candidate_median = statistics.median(candidate_values)
            ratio = candidate_median / baseline_median
            ratios.append(ratio)
            baseline_medians.append(baseline_median)
            candidate_medians.append(candidate_median)
            rss_delta = candidate["rss"] - baseline["rss"]
            rss_deltas.append(rss_delta)
            baseline_samples.extend(baseline_values)
            candidate_samples.extend(candidate_values)
            pair_rows.append(
                {
                    "lmax": lmax,
                    "spin": spin,
                    "threads": 1,
                    "operation": operation,
                    "pair": pair,
                    "baseline_first": pair % 2 == 1,
                    "baseline_samples_ms": ";".join(
                        f"{value:.3f}" for value in baseline_values
                    ),
                    "candidate_samples_ms": ";".join(
                        f"{value:.3f}" for value in candidate_values
                    ),
                    "baseline_median_ms": f"{baseline_median:.6f}",
                    "candidate_median_ms": f"{candidate_median:.6f}",
                    "candidate_over_baseline": f"{ratio:.8f}",
                    "baseline_rss_kib": baseline["rss"],
                    "candidate_rss_kib": candidate["rss"],
                    "rss_delta_kib": rss_delta,
                }
            )
        bootstrap = [
            statistics.median(rng.choice(ratios) for _ in ratios)
            for _ in range(50_000)
        ]
        summary_rows.append(
            {
                "lmax": lmax,
                "spin": spin,
                "threads": 1,
                "operation": operation,
                "n_pairs": 6,
                "baseline_median_ms": f"{statistics.median(baseline_medians):.6f}",
                "candidate_median_ms": f"{statistics.median(candidate_medians):.6f}",
                "median_paired_ratio": f"{statistics.median(ratios):.8f}",
                "bootstrap95_low": f"{quantile(bootstrap, 0.025):.8f}",
                "bootstrap95_high": f"{quantile(bootstrap, 0.975):.8f}",
                "paired_ratio_min": f"{min(ratios):.8f}",
                "paired_ratio_max": f"{max(ratios):.8f}",
                "baseline_sample_min_ms": f"{min(baseline_samples):.3f}",
                "baseline_sample_max_ms": f"{max(baseline_samples):.3f}",
                "candidate_sample_min_ms": f"{min(candidate_samples):.3f}",
                "candidate_sample_max_ms": f"{max(candidate_samples):.3f}",
                "median_rss_delta_kib": int(statistics.median(rss_deltas)),
            }
        )

for name, rows in (("process-pairs.csv", pair_rows), ("summary.csv", summary_rows)):
    with (HERE / name).open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
print(f"wrote {len(pair_rows)} operation-pair rows and {len(summary_rows)} summaries")
