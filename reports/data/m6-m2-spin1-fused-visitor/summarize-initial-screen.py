#!/usr/bin/env python3
"""Recompute the initial 6-pair primary/control screen from retained raw runs."""
from pathlib import Path
import csv
import random
import re
import statistics

ROOT = Path(__file__).resolve().parent
N_BOOTSTRAP = 200_000
SEED = 20_261_017
CELLS = {
    "T2047 spin1 q8 synthesis primary": [
        ("primary-t2047-q8-pair01-baseline", "primary-t2047-q8-pair01-candidate"),
        ("primary-t2047-q8-pair02-retry1-pair02-baseline", "primary-t2047-q8-pair02-retry1-pair02-candidate"),
        ("primary-t2047-q8-pair03-baseline", "primary-t2047-q8-pair03-candidate"),
        ("primary-t2047-q8-pair04-baseline", "primary-t2047-q8-pair04-candidate"),
        ("primary-t2047-q8-pair05-baseline", "primary-t2047-q8-pair05-candidate"),
        ("primary-t2047-q8-pair06-baseline", "primary-t2047-q8-pair06-candidate"),
    ],
    "T511 spin1 q1 synthesis control": [
        ("control-t511-q1-pair01-baseline", "control-t511-q1-pair01-candidate"),
        ("control-t511-q1-pair02-retry1-pair02-candidate-pair02-baseline", "control-t511-q1-pair02-retry1-pair02-candidate"),
        ("control-t511-q1-pair03-retry1-baseline", "control-t511-q1-pair03-retry1-candidate"),
        ("control-t511-q1-pair04-retry1-baseline", "control-t511-q1-pair04-retry1-candidate"),
        ("control-t511-q1-pair05-retry1-baseline", "control-t511-q1-pair05-retry1-candidate"),
        ("control-t511-q1-pair06-retry2-baseline", "control-t511-q1-pair06-retry2-candidate"),
    ],
}


def process(stem: str) -> tuple[float, int, float, list[float]]:
    stdout = (ROOT / f"{stem}.stdout").read_text()
    m = re.search(r"synthesis_rust_plan_raw_samples_ms=([^\n]+)", stdout)
    if not m:
        raise ValueError(f"missing raw Rust synthesis samples in {stem}")
    samples = [float(x) for x in m.group(1).split(",")]
    if len(samples) != 5:
        raise ValueError(f"expected five samples in {stem}, got {len(samples)}")
    tm = (ROOT / f"{stem}.time").read_text()
    m = re.search(r"Maximum resident set size \(kbytes\): (\d+)", tm)
    if not m:
        raise ValueError(f"missing RSS in {stem}")
    pre = (ROOT / f"{stem}.preflight").read_text()
    p = re.search(r"some avg10=([^ ]+)", pre)
    if p is None or float(p.group(1)) > 0.50 or "REFUSED:" in pre:
        raise ValueError(f"selected preflight not valid: {stem}")
    return statistics.median(samples), int(m.group(1)), float(p.group(1)), samples


def percentile(values: list[float], p: float) -> float:
    index = p * (len(values) - 1)
    lo = int(index); hi = min(lo + 1, len(values) - 1)
    return values[lo] + (values[hi] - values[lo]) * (index - lo)


pair_rows = []
summary_rows = []
markdown = [
    "# Initial six-pair spin-1 fused visitor screen",
    "",
    "The candidate and baseline are the exact isolated binaries identified in `source-and-binary-sha256.txt` and `build-binary-sha256.txt`. Each process used GL STANDARD f64, full mmax, `phi0=0.37`, synthesis_into, and five retained Rust timing samples. q8 used 8 pinned CPU cores; q1 used one CPU core. The fixed preflight gate was CPU PSI `some avg10` <= 0.50. Bootstrap resampling used the six paired ratios, 200,000 draws, seed 20261017, and the median statistic.",
    "",
]
for label, pairs in CELLS.items():
    ratios = []
    medians = []
    markdown.extend([f"## {label}", "", "| Pair | Baseline median ms | Candidate median ms | C/B | RSS change kB | PSI B/C |", "|---:|---:|---:|---:|---:|---:|"])
    for index, (base_stem, candidate_stem) in enumerate(pairs, 1):
        bmed, brss, bpsi, braw = process(base_stem)
        cmed, crss, cpsi, craw = process(candidate_stem)
        ratio = cmed / bmed
        ratios.append(ratio)
        medians.append((bmed, cmed))
        pair_rows.append({
            "cell": label,
            "pair": index,
            "baseline_stem": base_stem,
            "candidate_stem": candidate_stem,
            "baseline_median_ms": f"{bmed:.6f}",
            "candidate_median_ms": f"{cmed:.6f}",
            "candidate_baseline_ratio": f"{ratio:.9f}",
            "baseline_raw_ms": ",".join(f"{v:.3f}" for v in braw),
            "candidate_raw_ms": ",".join(f"{v:.3f}" for v in craw),
            "baseline_rss_kb": brss,
            "candidate_rss_kb": crss,
            "rss_delta_kb": crss - brss,
            "baseline_psi_avg10": f"{bpsi:.2f}",
            "candidate_psi_avg10": f"{cpsi:.2f}",
        })
        markdown.append(f"| {index} | {bmed:.3f} | {cmed:.3f} | {ratio:.6f} | {crss-brss} | {bpsi:.2f}/{cpsi:.2f} |")
    rng = random.Random(SEED)
    draws = sorted(statistics.median(rng.choices(ratios, k=6)) for _ in range(N_BOOTSTRAP))
    lo, hi = percentile(draws, .025), percentile(draws, .975)
    med = statistics.median(ratios)
    is_primary = "primary" in label
    gate = med <= .98 and hi < 1.0 if is_primary else hi <= 1.02
    summary_rows.append({
        "cell": label,
        "pairs": 6,
        "median_ratio": f"{med:.9f}",
        "ci95_lower": f"{lo:.9f}",
        "ci95_upper": f"{hi:.9f}",
        "min_pair_ratio": f"{min(ratios):.9f}",
        "max_pair_ratio": f"{max(ratios):.9f}",
        "pairs_over_1_02": sum(r > 1.02 for r in ratios),
        "bootstrap_seed": SEED,
        "bootstrap_resamples": N_BOOTSTRAP,
        "bootstrap_gate": "pass" if gate else "fail",
    })
    markdown.append(f"\n- Median C/B: {med:.6f}; paired bootstrap 95% interval: [{lo:.6f}, {hi:.6f}].")
    markdown.append(f"- Pair ratio range: [{min(ratios):.6f}, {max(ratios):.6f}]; pairs above 1.02: {sum(r > 1.02 for r in ratios)}/6.")
    markdown.append(f"- Gate: **{'pass' if gate else 'fail'}** ({'primary median <= 0.98 and upper < 1.00' if is_primary else 'control upper <= 1.02'}).")
    markdown.append("- Initial refused attempts and retained orphans are excluded from these selected pairs and remain in the raw directory with their original preflights.")
    markdown.append("")

for filename, rows in (("initial-screen-pairs.csv", pair_rows), ("initial-screen-summary.csv", summary_rows)):
    with (ROOT / filename).open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
(ROOT / "initial-screen-summary.md").write_text("\n".join(markdown) + "\n")
print((ROOT / "initial-screen-summary.md").read_text())
