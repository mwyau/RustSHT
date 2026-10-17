#!/usr/bin/env python3
"""Summarize the three-process C02 FFT guard isolation screen."""
import csv
import statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CASES = (
    ("scalar-t255-q1", 0, 255),
    ("spin1-t511-q1", 1, 511),
)
FLAVORS = ("baseline", "candidate", "no-guard")
OPS = ("synthesis", "analysis", "both")


def read_process(cell, triple, flavor, spin, lmax):
    stem = f"{cell}-triple{triple}-{flavor}"
    text = (ROOT / f"{stem}.stdout").read_text()
    time = (ROOT / f"{stem}.time").read_text().split()
    if "panic" in text.lower() or "error:" in text.lower():
        raise RuntimeError(f"error output in {stem}")
    lines = dict(line.split("=", 1) for line in text.splitlines() if "=" in line)
    expected = f"spin-{spin} T{lmax} mmax={lmax}; GL {lmax + 1}x{2 * (lmax + 1)}; f64; STANDARD; requested=1 threads; both; requested_only=true"
    if not text.splitlines()[0].startswith(expected):
        raise RuntimeError(f"unexpected case header in {stem}")
    for operation in ("synthesis", "analysis"):
        if len(lines[f"{operation}_rayon_samples_ms"].strip("[]").split(",")) != 5:
            raise RuntimeError(f"expected five {operation} samples in {stem}")
    fields = dict(part.split("=", 1) for part in time)
    return {
        "synthesis": float(lines["synthesis_rayon_ms"]),
        "analysis": float(lines["analysis_rayon_ms"]),
        "both": float(lines["synthesis_rayon_ms"]) + float(lines["analysis_rayon_ms"]),
        "rss": int(fields["peak_rss_kib"]),
        "elapsed_s": float(fields["elapsed_s"]),
    }


rows = []
for cell, spin, lmax in CASES:
    per_flavor = {flavor: [] for flavor in FLAVORS}
    for triple in range(1, 4):
        for flavor in FLAVORS:
            per_flavor[flavor].append(read_process(cell, triple, flavor, spin, lmax))

    for operation in OPS:
        baseline = [run[operation] for run in per_flavor["baseline"]]
        candidate = [run[operation] for run in per_flavor["candidate"]]
        no_guard = [run[operation] for run in per_flavor["no-guard"]]
        ratios = {
            "candidate_over_baseline": [c / b for b, c in zip(baseline, candidate)],
            "no_guard_over_candidate": [n / c for c, n in zip(candidate, no_guard)],
            "no_guard_over_baseline": [n / b for b, n in zip(baseline, no_guard)],
        }
        row = {
            "cell": cell,
            "operation": operation,
            "baseline_median_ms": statistics.median(baseline),
            "candidate_median_ms": statistics.median(candidate),
            "no_guard_median_ms": statistics.median(no_guard),
        }
        for name, values in ratios.items():
            row[f"{name}_median"] = statistics.median(values)
            row[f"{name}_min"] = min(values)
            row[f"{name}_max"] = max(values)
            row[f"{name}_triples"] = ";".join(f"{index}:{value:.6f}" for index, value in enumerate(values, 1))
        rss = {flavor: [run["rss"] for run in per_flavor[flavor]] for flavor in FLAVORS}
        row["candidate_minus_baseline_rss_median_kib"] = statistics.median(
            c - b for b, c in zip(rss["baseline"], rss["candidate"])
        )
        row["no_guard_minus_candidate_rss_median_kib"] = statistics.median(
            n - c for c, n in zip(rss["candidate"], rss["no-guard"])
        )
        rows.append(row)

columns = list(rows[0])
with (ROOT / "isolation-summary.csv").open("w", newline="") as output:
    writer = csv.DictWriter(output, fieldnames=columns)
    writer.writeheader()
    writer.writerows(rows)

print(f"rows={len(rows)} raw_processes=18")
for row in rows:
    print(
        row["cell"], row["operation"],
        f"baseline/candidate/no_guard={row['baseline_median_ms']:.3f}/{row['candidate_median_ms']:.3f}/{row['no_guard_median_ms']:.3f} ms",
        f"candidate/base={row['candidate_over_baseline_median']:.4f}",
        f"no_guard/candidate={row['no_guard_over_candidate_median']:.4f}",
        f"no_guard/base={row['no_guard_over_baseline_median']:.4f}",
    )
