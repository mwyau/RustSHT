#!/usr/bin/env python3
"""Summarize paired ordinary-route C03 controls without filtering samples."""
import csv
import random
import re
import statistics
from pathlib import Path

BASE = Path(__file__).resolve().parent
PAIRS = range(1, 7)
SEED = 20261017
BOOTSTRAP_SAMPLES = 20_000
CASES = [("spin1", 1, 511), ("spin2", 2, 511)]


def read_process(cell, pair, flavor, label, spin, lmax):
    stem = f"{cell}-pair{pair}-{flavor}"
    stdout = (BASE / f"{stem}.stdout").read_text()
    time_text = (BASE / f"{stem}.time").read_text().strip()
    if "panicked" in stdout.lower() or "error:" in stdout.lower():
        raise RuntimeError(f"error output in {stem}")
    expected = (
        f"spin-{spin} T{lmax} mmax={lmax}; GL {lmax + 1}x{2 * (lmax + 1)}; "
        "f64; STANDARD; requested=1 threads; both; requested_only=true"
    )
    header = stdout.splitlines()[0]
    if not header.startswith(expected):
        raise RuntimeError(f"unexpected case header in {stem}: {header}")
    fields = {}
    for line in stdout.splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            fields[key] = value
    for op in ("synthesis", "analysis"):
        samples = fields.get(f"{op}_rayon_samples_ms", "").strip("[]").split(",")
        if len(samples) != 5:
            raise RuntimeError(f"expected five {op} samples in {stem}")
    time_fields = dict(piece.split("=", 1) for piece in time_text.split())
    return {
        "synthesis_ms": float(fields["synthesis_rayon_ms"]),
        "analysis_ms": float(fields["analysis_rayon_ms"]),
        "rss_kib": int(time_fields["peak_rss_kib"]),
        "elapsed_s": float(time_fields["elapsed_s"]),
    }


def quantile(values, q):
    position = q * (len(values) - 1)
    lower = int(position)
    upper = min(lower + 1, len(values) - 1)
    fraction = position - lower
    return values[lower] * (1 - fraction) + values[upper] * fraction


def bootstrap_median_ci(values, seed):
    rng = random.Random(seed)
    boot = []
    for _ in range(BOOTSTRAP_SAMPLES):
        boot.append(statistics.median(rng.choice(values) for _ in values))
    boot.sort()
    return quantile(boot, 0.025), quantile(boot, 0.975)


preflight = (BASE / "preflight-processes.log").read_text()
snapshots = len(re.findall(r"^--- run=", preflight, re.M))
refusals = len(re.findall(r"^PREFLIGHT_REFUSED", preflight, re.M))
commands = len(re.findall(r"^command:", preflight, re.M))
if commands != 24 or snapshots != 24 or refusals:
    raise RuntimeError(
        f"expected 24 commands/snapshots and zero refusals, got {commands}/{snapshots}/{refusals}"
    )
if re.search(r"^(CONTENTION_FLAG|INCOMPLETE_ARTIFACT)", preflight, re.M):
    raise RuntimeError("runner recorded a contaminated or incomplete process; inspect preflights")
if len(list(BASE.glob("*.stdout"))) != 24 or len(list(BASE.glob("*.time"))) != 24:
    raise RuntimeError("expected 24 raw stdout and 24 time files")

rows = []
for cell_index, (label, spin, lmax) in enumerate(CASES):
    cell = f"{label}-t{lmax}-q1"
    paired = {}
    for pair in PAIRS:
        paired[pair] = {
            flavor: read_process(cell, pair, flavor, label, spin, lmax)
            for flavor in ("baseline", "candidate")
        }
    rss_baseline = [paired[p]["baseline"]["rss_kib"] for p in PAIRS]
    rss_candidate = [paired[p]["candidate"]["rss_kib"] for p in PAIRS]
    rss_delta = [c - b for c, b in zip(rss_candidate, rss_baseline)]
    for op_index, op in enumerate(("synthesis", "analysis")):
        ratios = [
            paired[p]["candidate"][f"{op}_ms"] / paired[p]["baseline"][f"{op}_ms"]
            for p in PAIRS
        ]
        low, high = bootstrap_median_ci(ratios, SEED + cell_index * 3 + op_index)
        med = statistics.median(ratios)
        rows.append({
            "cell": cell,
            "spin": spin,
            "lmax": lmax,
            "ntheta": lmax + 1,
            "nphi": 2 * (lmax + 1),
            "mmax": lmax,
            "threads": 1,
            "affinity": "0",
            "operation": op,
            "pairs": 6,
            "baseline_median_ms": f"{statistics.median(paired[p]['baseline'][f'{op}_ms'] for p in PAIRS):.6f}",
            "candidate_median_ms": f"{statistics.median(paired[p]['candidate'][f'{op}_ms'] for p in PAIRS):.6f}",
            "ratio_median": f"{med:.6f}",
            "ratio_ci95_low": f"{low:.6f}",
            "ratio_ci95_high": f"{high:.6f}",
            "ratio_min": f"{min(ratios):.6f}",
            "ratio_max": f"{max(ratios):.6f}",
            "pairs_ratio_gt_1_02": sum(x > 1.02 for x in ratios),
            "pair_ratios": ";".join(f"{p}:{x:.6f}" for p, x in zip(PAIRS, ratios)),
            "baseline_peak_rss_median_kib": statistics.median(rss_baseline),
            "candidate_peak_rss_median_kib": statistics.median(rss_candidate),
            "rss_delta_median_kib": statistics.median(rss_delta),
            "rss_delta_min_kib": min(rss_delta),
            "rss_delta_max_kib": max(rss_delta),
            "pair_rss_deltas_kib": ";".join(f"{p}:{d}" for p, d in zip(PAIRS, rss_delta)),
        })

columns = list(rows[0])
with (BASE / "summary.csv").open("w", newline="") as f:
    writer = csv.DictWriter(f, fieldnames=columns)
    writer.writeheader()
    writer.writerows(rows)

print(f"rows={len(rows)}; processes={commands}; preflight_snapshots={snapshots}; refused_preflights={refusals}")
loads = [tuple(float(x) for x in m) for m in re.findall(r"load average: ([0-9.]+), ([0-9.]+), ([0-9.]+)", preflight)]
if loads:
    print(f"host_load_1m_range={min(x[0] for x in loads):.2f}..{max(x[0] for x in loads):.2f}")
    print(f"host_load_5m_range={min(x[1] for x in loads):.2f}..{max(x[1] for x in loads):.2f}")
for row in rows:
    print(
        f"{row['cell']} {row['operation']}: ratio={row['ratio_median']} "
        f"CI95=[{row['ratio_ci95_low']},{row['ratio_ci95_high']}] "
        f"range={row['ratio_min']}..{row['ratio_max']} "
        f"pairs_gt_1.02={row['pairs_ratio_gt_1_02']}; "
        f"RSS delta median={row['rss_delta_median_kib']} KiB"
    )
