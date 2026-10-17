#!/usr/bin/env python3
"""Summarize the six-pair C02 isolated diagnostic screen."""
import csv
import random
import re
import statistics
from pathlib import Path

BASE = Path(__file__).resolve().parent / "isolated-screen"
PAIRS = range(1, 7)
SEED = 20261017
BOOTSTRAP_SAMPLES = 20_000
CASES = [("scalar-t255-q1", 0, 255), ("spin1-t511-q1", 1, 511)]


def read_process(cell, pair, flavor, spin, lmax):
    stem = f"{cell}-pair{pair}-{flavor}"
    stdout = (BASE / f"{stem}.stdout").read_text()
    time_text = (BASE / f"{stem}.time").read_text().strip()
    if "panicked" in stdout.lower() or "error:" in stdout.lower():
        raise RuntimeError(f"error output in {stem}")
    expected = (
        f"spin-{spin} T{lmax} mmax={lmax}; GL {lmax + 1}x{2 * (lmax + 1)}; "
        "f64; STANDARD; requested=1 threads; both; requested_only=true"
    )
    if not stdout.splitlines()[0].startswith(expected):
        raise RuntimeError(f"unexpected case header in {stem}: {stdout.splitlines()[0]}")
    fields = {}
    for line in stdout.splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            fields[key] = value
    for operation in ("synthesis", "analysis"):
        samples_key = f"{operation}_rayon_samples_ms"
        samples = fields.get(samples_key, "").strip("[]").split(",")
        if len(samples) != 5:
            raise RuntimeError(f"expected all five warmed {operation} samples in {stem}")
    time_fields = dict(piece.split("=", 1) for piece in time_text.split())
    return {
        "synthesis_ms": float(fields["synthesis_rayon_ms"]),
        "analysis_ms": float(fields["analysis_rayon_ms"]),
        "rss_kib": int(time_fields["peak_rss_kib"]),
        "elapsed_s": float(time_fields["elapsed_s"]),
    }


def quantile(sorted_values, q):
    position = q * (len(sorted_values) - 1)
    lower = int(position)
    upper = min(lower + 1, len(sorted_values) - 1)
    fraction = position - lower
    return sorted_values[lower] * (1 - fraction) + sorted_values[upper] * fraction


def bootstrap_median_ci(values, seed):
    rng = random.Random(seed)
    medians = []
    for _ in range(BOOTSTRAP_SAMPLES):
        medians.append(statistics.median(rng.choice(values) for _ in values))
    medians.sort()
    return quantile(medians, 0.025), quantile(medians, 0.975)


snapshot_text = (BASE / "preflight-processes.log").read_text()
snapshot_count = len(re.findall(r"^--- run=", snapshot_text, re.M))
contention_flags = len(re.findall(r"^CONTENTION_FLAG", snapshot_text, re.M))
if snapshot_count != 24:
    raise RuntimeError(f"expected 24 process preflights, found {snapshot_count}")

columns = [
    "cell", "spin", "lmax", "ntheta", "nphi", "mmax", "threads", "affinity",
    "operation", "pairs", "baseline_median_ms", "isolated_median_ms", "ratio_median",
    "ratio_ci95_low", "ratio_ci95_high", "ratio_min", "ratio_max", "pair_ratios",
    "baseline_peak_rss_median_kib", "isolated_peak_rss_median_kib",
    "rss_delta_median_kib", "rss_delta_min_kib", "rss_delta_max_kib", "pair_rss_deltas_kib",
]
rows = []
for cell_index, (cell, spin, lmax) in enumerate(CASES):
    paired = {}
    for pair in PAIRS:
        paired[pair] = {
            flavor: read_process(cell, pair, flavor, spin, lmax)
            for flavor in ("baseline", "isolated")
        }
    for operation_index, operation in enumerate(("synthesis", "analysis")):
        ratios = [
            paired[pair]["isolated"][f"{operation}_ms"]
            / paired[pair]["baseline"][f"{operation}_ms"]
            for pair in PAIRS
        ]
        rss_deltas = [
            paired[pair]["isolated"]["rss_kib"] - paired[pair]["baseline"]["rss_kib"]
            for pair in PAIRS
        ]
        baseline_times = [paired[pair]["baseline"][f"{operation}_ms"] for pair in PAIRS]
        candidate_times = [paired[pair]["isolated"][f"{operation}_ms"] for pair in PAIRS]
        baseline_rss = [paired[pair]["baseline"]["rss_kib"] for pair in PAIRS]
        candidate_rss = [paired[pair]["isolated"]["rss_kib"] for pair in PAIRS]
        low, high = bootstrap_median_ci(ratios, SEED + cell_index * 2 + operation_index)
        rows.append({
            "cell": cell,
            "spin": spin,
            "lmax": lmax,
            "ntheta": lmax + 1,
            "nphi": 2 * (lmax + 1),
            "mmax": lmax,
            "threads": 1,
            "affinity": "0",
            "operation": operation,
            "pairs": 6,
            "baseline_median_ms": f"{statistics.median(baseline_times):.6f}",
            "isolated_median_ms": f"{statistics.median(candidate_times):.6f}",
            "ratio_median": f"{statistics.median(ratios):.6f}",
            "ratio_ci95_low": f"{low:.6f}",
            "ratio_ci95_high": f"{high:.6f}",
            "ratio_min": f"{min(ratios):.6f}",
            "ratio_max": f"{max(ratios):.6f}",
            "pair_ratios": ";".join(f"{pair}:{ratio:.6f}" for pair, ratio in zip(PAIRS, ratios)),
            "baseline_peak_rss_median_kib": statistics.median(baseline_rss),
            "isolated_peak_rss_median_kib": statistics.median(candidate_rss),
            "rss_delta_median_kib": statistics.median(rss_deltas),
            "rss_delta_min_kib": min(rss_deltas),
            "rss_delta_max_kib": max(rss_deltas),
            "pair_rss_deltas_kib": ";".join(f"{pair}:{delta}" for pair, delta in zip(PAIRS, rss_deltas)),
        })

with (BASE / "summary.csv").open("w", newline="") as output:
    writer = csv.DictWriter(output, fieldnames=columns)
    writer.writeheader()
    writer.writerows(rows)

print(f"rows={len(rows)}; processes=24; preflights={snapshot_count}; contention_flags={contention_flags}")
for row in rows:
    print(
        f"{row['cell']} {row['operation']}: baseline={row['baseline_median_ms']} ms; "
        f"isolated={row['isolated_median_ms']} ms; ratio={row['ratio_median']} "
        f"CI95=[{row['ratio_ci95_low']},{row['ratio_ci95_high']}]; "
        f"range={row['ratio_min']}..{row['ratio_max']}; "
        f"RSS delta median={row['rss_delta_median_kib']} KiB "
        f"range={row['rss_delta_min_kib']}..{row['rss_delta_max_kib']} KiB"
    )
