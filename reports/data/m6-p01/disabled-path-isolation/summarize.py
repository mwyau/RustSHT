#!/usr/bin/env python3
"""Summarize paired baseline/disabled-P01 screens."""
import csv
import random
import re
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
SAMPLES = re.compile(r"(synthesis|analysis)_rayon_samples_ms=\[([^]]+)\]")
RSS = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
EXIT = re.compile(r"Exit status:\s*(\d+)")


def read(stem):
    out = (HERE / f"{stem}.stdout").read_text()
    values = {m.group(1): [float(x) for x in m.group(2).split(",")] for m in SAMPLES.finditer(out)}
    assert set(values) == {"synthesis", "analysis"}
    assert all(len(samples) == 5 for samples in values.values())
    timing = (HERE / f"{stem}.time").read_text()
    rss = RSS.search(timing)
    status = EXIT.search(timing)
    assert rss and status and status.group(1) == "0"
    return values, int(rss.group(1))


def quantile(values, p):
    values = sorted(values)
    pos = (len(values) - 1) * p
    lo = int(pos)
    hi = min(lo + 1, len(values) - 1)
    f = pos - lo
    return values[lo] * (1 - f) + values[hi] * f


pairs = []
for pair in range(1, 7):
    baseline, base_rss = read(f"t511-s1-q1-pair{pair}-baseline")
    disabled, disabled_rss = read(f"t511-s1-q1-pair{pair}-disabled")
    for operation in ("synthesis", "analysis"):
        b = baseline[operation]
        d = disabled[operation]
        bm = statistics.median(b)
        dm = statistics.median(d)
        pairs.append({
            "lmax": 511,
            "spin": 1,
            "threads": 1,
            "operation": operation,
            "pair": pair,
            "baseline_first": pair % 2 == 1,
            "baseline_samples_ms": ";".join(f"{x:.3f}" for x in b),
            "disabled_samples_ms": ";".join(f"{x:.3f}" for x in d),
            "baseline_median_ms": f"{bm:.6f}",
            "disabled_median_ms": f"{dm:.6f}",
            "disabled_over_baseline": f"{dm / bm:.8f}",
            "baseline_rss_kib": base_rss,
            "disabled_rss_kib": disabled_rss,
            "rss_delta_kib": disabled_rss - base_rss,
        })

summary = []
rng = random.Random(20261017)
for operation in ("synthesis", "analysis"):
    selected = [r for r in pairs if r["operation"] == operation]
    ratios = [float(r["disabled_over_baseline"]) for r in selected]
    bootstrap = [statistics.median(rng.choice(ratios) for _ in ratios) for _ in range(50000)]
    summary.append({
        "lmax": 511,
        "spin": 1,
        "threads": 1,
        "operation": operation,
        "n_pairs": 6,
        "baseline_median_ms": f"{statistics.median(float(r['baseline_median_ms']) for r in selected):.6f}",
        "disabled_median_ms": f"{statistics.median(float(r['disabled_median_ms']) for r in selected):.6f}",
        "median_paired_ratio": f"{statistics.median(ratios):.8f}",
        "bootstrap95_low": f"{quantile(bootstrap, .025):.8f}",
        "bootstrap95_high": f"{quantile(bootstrap, .975):.8f}",
        "paired_ratio_min": f"{min(ratios):.8f}",
        "paired_ratio_max": f"{max(ratios):.8f}",
        "median_rss_delta_kib": int(statistics.median(int(r['rss_delta_kib']) for r in selected)),
    })

for filename, rows in (("process-pairs.csv", pairs), ("summary.csv", summary)):
    with (HERE / filename).open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
print(f"wrote {len(pairs)} operation-pair rows and {len(summary)} summaries")
