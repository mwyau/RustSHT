#!/usr/bin/env python3
"""Summarize the selected 12 paired synthesis runs for one matrix cell."""
from __future__ import annotations

import argparse
import csv
import hashlib
import random
import re
import statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PERCENTILE_RESAMPLES = 200_000
BOOTSTRAP_SEED_BASE = 20_261_017


def read_process(stem: str) -> dict[str, object]:
    stdout = (ROOT / f"{stem}.stdout").read_text()
    match = re.search(r"synthesis_rust_plan_raw_samples_ms=([^\n]+)", stdout)
    if not match:
        raise ValueError(f"missing Rust synthesis samples in {stem}.stdout")
    samples = [float(value) for value in match.group(1).split(",")]
    if len(samples) != 5:
        raise ValueError(f"expected 5 samples in {stem}.stdout, got {len(samples)}")

    timing = (ROOT / f"{stem}.time").read_text()
    rss_match = re.search(r"Maximum resident set size \(kbytes\): (\d+)", timing)
    if rss_match is None:
        raise ValueError(f"missing maximum RSS in {stem}.time")

    preflight = (ROOT / f"{stem}.preflight").read_text()
    psi_match = re.search(r"some avg10=([^ ]+)", preflight)
    if psi_match is None or "REFUSED:" in preflight:
        raise ValueError(f"selected process has refused or missing preflight: {stem}")
    psi = float(psi_match.group(1))
    if psi > 0.50:
        raise ValueError(f"selected process exceeds PSI gate: {stem} ({psi})")

    return {
        "samples": samples,
        "median_ms": statistics.median(samples),
        "max_rss_kb": int(rss_match.group(1)),
        "psi_avg10": psi,
    }


def percentile(sorted_values: list[float], probability: float) -> float:
    position = probability * (len(sorted_values) - 1)
    lower = int(position)
    upper = min(lower + 1, len(sorted_values) - 1)
    weight = position - lower
    return sorted_values[lower] + (sorted_values[upper] - sorted_values[lower]) * weight


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("cell", help="matrix cell label, e.g. spin0-t511-q1")
    args = parser.parse_args()
    cell = args.cell

    selected_attempts: dict[int, int] = {}
    for line in (ROOT / "runner.log").read_text().splitlines():
        match = re.search(r"pair_complete cell=(\S+) pair=(\d+) attempt=(\d+)", line)
        if match and match.group(1) == cell:
            selected_attempts[int(match.group(2))] = int(match.group(3))
    if set(selected_attempts) != set(range(1, 13)):
        raise ValueError(f"expected 12 completed pairs for {cell}, found {sorted(selected_attempts)}")

    rows: list[dict[str, object]] = []
    ratios: list[float] = []
    for pair in range(1, 13):
        attempt = selected_attempts[pair]
        baseline = read_process(f"{cell}-pair{pair:02d}-attempt{attempt:02d}-baseline")
        candidate = read_process(f"{cell}-pair{pair:02d}-attempt{attempt:02d}-candidate")
        ratio = float(candidate["median_ms"]) / float(baseline["median_ms"])
        ratios.append(ratio)
        rows.append(
            {
                "cell": cell,
                "pair": pair,
                "attempt": attempt,
                "baseline_median_ms": f"{float(baseline['median_ms']):.6f}",
                "candidate_median_ms": f"{float(candidate['median_ms']):.6f}",
                "candidate_baseline_ratio": f"{ratio:.9f}",
                "baseline_rss_kb": baseline["max_rss_kb"],
                "candidate_rss_kb": candidate["max_rss_kb"],
                "rss_delta_kb": int(candidate["max_rss_kb"]) - int(baseline["max_rss_kb"]),
                "baseline_psi_avg10": f"{float(baseline['psi_avg10']):.2f}",
                "candidate_psi_avg10": f"{float(candidate['psi_avg10']):.2f}",
                "baseline_raw_ms": ",".join(f"{value:.3f}" for value in baseline["samples"]),
                "candidate_raw_ms": ",".join(f"{value:.3f}" for value in candidate["samples"]),
            }
        )

    seed = BOOTSTRAP_SEED_BASE + int.from_bytes(hashlib.sha256(cell.encode()).digest()[:4], "big")
    rng = random.Random(seed)
    bootstrap_medians = [statistics.median(rng.choices(ratios, k=12)) for _ in range(PERCENTILE_RESAMPLES)]
    bootstrap_medians.sort()
    ci_lower = percentile(bootstrap_medians, 0.025)
    ci_upper = percentile(bootstrap_medians, 0.975)
    median_ratio = statistics.median(ratios)
    strict_target = cell == "spin1-t2047-q8"
    if strict_target:
        gate_pass = median_ratio <= 0.98 and ci_upper < 1.0
        gate_text = "target: median <= 0.98 and 95% upper < 1.00"
    else:
        gate_pass = ci_upper <= 1.02
        gate_text = "control: 95% upper <= 1.02"

    pair_path = ROOT / f"{cell}-pairs.csv"
    with pair_path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)

    summary = {
        "cell": cell,
        "pairs": 12,
        "median_ratio": f"{median_ratio:.9f}",
        "ci95_lower": f"{ci_lower:.9f}",
        "ci95_upper": f"{ci_upper:.9f}",
        "min_pair_ratio": f"{min(ratios):.9f}",
        "max_pair_ratio": f"{max(ratios):.9f}",
        "pairs_over_1_02": sum(ratio > 1.02 for ratio in ratios),
        "bootstrap_seed": seed,
        "bootstrap_resamples": PERCENTILE_RESAMPLES,
        "gate": "pass" if gate_pass else "fail",
    }
    summary_path = ROOT / f"{cell}-summary.csv"
    with summary_path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(summary))
        writer.writeheader()
        writer.writerow(summary)

    summary_md = ROOT / f"{cell}-summary.md"
    summary_md.write_text(
        f"# {cell}\n\n"
        f"Selected {len(ratios)} completed paired synthesis runs. Per-process times are medians of five retained raw samples. "
        f"Bootstrap resamples pair ratios with replacement, uses the median, {PERCENTILE_RESAMPLES} draws, and seed {seed}.\n\n"
        f"- Candidate/baseline median ratio: {median_ratio:.6f}\n"
        f"- Paired bootstrap 95% interval: [{ci_lower:.6f}, {ci_upper:.6f}]\n"
        f"- Pair ratio range: [{min(ratios):.6f}, {max(ratios):.6f}]\n"
        f"- Pairs above 1.02: {summary['pairs_over_1_02']} / 12\n"
        f"- Gate: **{summary['gate']}** ({gate_text})\n\n"
        f"Per-pair raw values and selected attempt IDs: [{cell}-pairs.csv]({cell}-pairs.csv). "
        "All selected stdout, GNU time and preflight records remain alongside the runner artifacts.\n"
    )

    central = ROOT / "cell-gates.csv"
    existing: dict[str, dict[str, str]] = {}
    if central.exists():
        with central.open(newline="") as stream:
            existing = {row["cell"]: row for row in csv.DictReader(stream)}
    existing[cell] = {key: str(value) for key, value in summary.items()}
    with central.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(summary))
        writer.writeheader()
        for key in sorted(existing):
            writer.writerow(existing[key])

    print(f"cell={cell} pairs=12 median={median_ratio:.6f} ci95=[{ci_lower:.6f},{ci_upper:.6f}] "
          f"range=[{min(ratios):.6f},{max(ratios):.6f}] gate={summary['gate']} seed={seed}")
    for row in rows:
        print(f"pair={row['pair']:02d} attempt={row['attempt']:02d} "
              f"C/B={row['candidate_baseline_ratio']} RSS_delta_kb={row['rss_delta_kb']} "
              f"PSI={row['baseline_psi_avg10']}/{row['candidate_psi_avg10']}")


if __name__ == "__main__":
    main()
