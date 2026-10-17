#!/usr/bin/env python3
"""Summarize six paired M6 C02 baseline/candidate process comparisons."""
import csv
import random
import re
import statistics
from pathlib import Path

BASE = Path(__file__).resolve().parent
PAIRS = range(1, 7)
SEED = 20261017
BOOTSTRAP_SAMPLES = 20_000
CASES = [
    (label, spin, lmax, threads)
    for label, spin, lmax in (("scalar", 0, 255), ("scalar", 0, 511), ("spin1", 1, 255), ("spin1", 1, 511))
    for threads in (1, 8)
]


def read_process(stem, flavor, label, spin, lmax, threads):
    root = f"{label}-t{lmax}-q{threads}-pair{stem}-{flavor}"
    stdout = (BASE / f"{root}.stdout").read_text()
    time_text = (BASE / f"{root}.time").read_text().strip()
    if "panicked" in stdout.lower() or "error:" in stdout.lower():
        raise RuntimeError(f"error output in {root}")
    header = stdout.splitlines()[0]
    expected = f"spin-{spin} T{lmax} mmax={lmax}; GL {lmax + 1}x{2 * (lmax + 1)}; f64; STANDARD; requested={threads} threads; both; requested_only=true"
    if not header.startswith(expected):
        raise RuntimeError(f"unexpected case header in {root}: {header}")
    fields = {}
    for line in stdout.splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            fields[key] = value
    for op in ("synthesis", "analysis"):
        key = f"{op}_rayon_samples_ms"
        if key not in fields or len(fields[key].strip("[]").split(",")) != 5:
            raise RuntimeError(f"expected five samples for {op} in {root}")
    time_fields = dict(piece.split("=", 1) for piece in time_text.split())
    return {
        "synthesis_ms": float(fields["synthesis_rayon_ms"]),
        "analysis_ms": float(fields["analysis_rayon_ms"]),
        "rss_kib": int(time_fields["peak_rss_kib"]),
        "elapsed_s": float(time_fields["elapsed_s"]),
    }


def quantile(sorted_values, q):
    # Linear interpolation, equivalent to the common type-7 sample quantile.
    position = q * (len(sorted_values) - 1)
    lower = int(position)
    upper = min(lower + 1, len(sorted_values) - 1)
    fraction = position - lower
    return sorted_values[lower] * (1 - fraction) + sorted_values[upper] * fraction


def bootstrap_median_ci(values, seed):
    rng = random.Random(seed)
    boot = []
    for _ in range(BOOTSTRAP_SAMPLES):
        sample = [rng.choice(values) for _ in values]
        boot.append(statistics.median(sample))
    boot.sort()
    return quantile(boot, 0.025), quantile(boot, 0.975)


paired = {}
for label, spin, lmax, threads in CASES:
    cell = f"{label}-t{lmax}-q{threads}"
    paired[cell] = {}
    for pair in PAIRS:
        baseline = read_process(pair, "baseline", label, spin, lmax, threads)
        candidate = read_process(pair, "candidate", label, spin, lmax, threads)
        paired[cell][pair] = {"baseline": baseline, "candidate": candidate}

snapshot_text = (BASE / "preflight-processes.log").read_text()
snapshot_count = len(re.findall(r"^--- run=", snapshot_text, re.M))
contention_flags = len(re.findall(r"^CONTENTION_FLAG", snapshot_text, re.M))
load_values = [
    tuple(float(x) for x in match)
    for match in re.findall(r"load average: ([0-9.]+), ([0-9.]+), ([0-9.]+)", snapshot_text)
]
if snapshot_count != 96:
    raise RuntimeError(f"expected 96 per-process preflights, found {snapshot_count}")
if contention_flags:
    raise RuntimeError(f"runner flagged contention {contention_flags} times; inspect raw preflights")
if len(list(BASE.glob("*.stdout"))) != 96 or len(list(BASE.glob("*.time"))) != 96:
    raise RuntimeError("expected 96 raw stdout and 96 time files")

columns = [
    "cell", "spin", "lmax", "ntheta", "nphi", "mmax", "threads", "affinity", "operation", "pairs",
    "baseline_median_ms", "candidate_median_ms", "ratio_median", "ratio_ci95_low", "ratio_ci95_high",
    "ratio_min", "ratio_max", "pairs_ratio_gt_1_02", "median_ratio_gt_1_02", "ci95_low_gt_1_02",
    "repeatable_gt2pct_signal", "pair_ratios", "baseline_peak_rss_median_kib", "candidate_peak_rss_median_kib",
    "rss_delta_median_kib", "rss_delta_min_kib", "rss_delta_max_kib", "pair_rss_deltas_kib",
]
rows = []
for group_index, (label, spin, lmax, threads) in enumerate(CASES):
    cell = f"{label}-t{lmax}-q{threads}"
    ratios_by_op = {"synthesis": [], "analysis": [], "both": []}
    rss_baseline = []
    rss_candidate = []
    rss_deltas = []
    for pair in PAIRS:
        data = paired[cell][pair]
        b = data["baseline"]
        c = data["candidate"]
        for op in ("synthesis", "analysis"):
            ratios_by_op[op].append((c[f"{op}_ms"], b[f"{op}_ms"]))
        ratios_by_op["both"].append((c["synthesis_ms"] + c["analysis_ms"], b["synthesis_ms"] + b["analysis_ms"]))
        rss_baseline.append(b["rss_kib"])
        rss_candidate.append(c["rss_kib"])
        rss_deltas.append(c["rss_kib"] - b["rss_kib"])

    for offset, op in enumerate(("synthesis", "analysis", "both")):
        ratios = [candidate / baseline for candidate, baseline in ratios_by_op[op]]
        low, high = bootstrap_median_ci(ratios, SEED + group_index * 3 + offset)
        median_ratio = statistics.median(ratios)
        count_over = sum(value > 1.02 for value in ratios)
        # Descriptive signal only: median exceeds +2%, with at least five of six pairs also above +2%.
        repeatable_signal = median_ratio > 1.02 and count_over >= 5
        rows.append({
            "cell": cell,
            "spin": spin,
            "lmax": lmax,
            "ntheta": lmax + 1,
            "nphi": 2 * (lmax + 1),
            "mmax": lmax,
            "threads": threads,
            "affinity": "0" if threads == 1 else "8-15",
            "operation": op,
            "pairs": 6,
            "baseline_median_ms": f"{statistics.median([b for _, b in ratios_by_op[op]]):.6f}",
            "candidate_median_ms": f"{statistics.median([c for c, _ in ratios_by_op[op]]):.6f}",
            "ratio_median": f"{median_ratio:.6f}",
            "ratio_ci95_low": f"{low:.6f}",
            "ratio_ci95_high": f"{high:.6f}",
            "ratio_min": f"{min(ratios):.6f}",
            "ratio_max": f"{max(ratios):.6f}",
            "pairs_ratio_gt_1_02": count_over,
            "median_ratio_gt_1_02": str(median_ratio > 1.02).lower(),
            "ci95_low_gt_1_02": str(low > 1.02).lower(),
            "repeatable_gt2pct_signal": str(repeatable_signal).lower(),
            "pair_ratios": ";".join(f"{i}:{ratio:.6f}" for i, ratio in zip(PAIRS, ratios)),
            "baseline_peak_rss_median_kib": statistics.median(rss_baseline),
            "candidate_peak_rss_median_kib": statistics.median(rss_candidate),
            "rss_delta_median_kib": statistics.median(rss_deltas),
            "rss_delta_min_kib": min(rss_deltas),
            "rss_delta_max_kib": max(rss_deltas),
            "pair_rss_deltas_kib": ";".join(f"{i}:{delta}" for i, delta in zip(PAIRS, rss_deltas)),
        })

with (BASE / "summary.csv").open("w", newline="") as f:
    writer = csv.DictWriter(f, fieldnames=columns)
    writer.writeheader()
    writer.writerows(rows)

print(f"rows={len(rows)}; processes=96; preflights={snapshot_count}; contention_flags={contention_flags}")
if load_values:
    print("host_load_1m_range=" + f"{min(x[0] for x in load_values):.2f}..{max(x[0] for x in load_values):.2f}")
    print("host_load_5m_range=" + f"{min(x[1] for x in load_values):.2f}..{max(x[1] for x in load_values):.2f}")
for row in rows:
    if row["operation"] == "both":
        print(f"{row['cell']}: S+A ratio={row['ratio_median']} CI95=[{row['ratio_ci95_low']},{row['ratio_ci95_high']}] range={row['ratio_min']}..{row['ratio_max']} n_gt_1.02={row['pairs_ratio_gt_1_02']} flag={row['repeatable_gt2pct_signal']}; RSS delta median={row['rss_delta_median_kib']} KiB range={row['rss_delta_min_kib']}..{row['rss_delta_max_kib']}")
