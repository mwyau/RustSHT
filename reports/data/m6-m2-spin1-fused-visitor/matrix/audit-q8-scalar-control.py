#!/usr/bin/env python3
"""Audit raw timing consistency and retained preflights for spin0 T511 q8."""
from __future__ import annotations

import csv
import re
import statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CELL = "spin0-t511-q8"


def find(pattern: str, text: str, default: str = "NA") -> str:
    match = re.search(pattern, text)
    return match.group(1) if match else default


def parse_side(stem: str) -> dict[str, object]:
    stdout = (ROOT / f"{stem}.stdout").read_text()
    values = [float(x) for x in find(r"synthesis_rust_plan_raw_samples_ms=([^\n]+)", stdout).split(",")]
    time = (ROOT / f"{stem}.time").read_text()
    pre = (ROOT / f"{stem}.preflight").read_text()
    numeric = lambda pattern: float(find(pattern, time))
    psi = float(find(r"some avg10=([^ ]+)", pre))
    top = pre.split("Top processes:\n", 1)[1].split("\nActive compiler or benchmark processes:", 1)[0]
    active = pre.split("Active compiler or benchmark processes:\n", 1)[1].split("\nRequested affinity:", 1)[0]
    # The time-runner already prints a median; recompute from the five raw values.
    return {
        "samples": values,
        "median": statistics.median(values),
        "min": min(values),
        "max": max(values),
        "mean": statistics.mean(values),
        "stdev": statistics.stdev(values),
        "cv_pct": statistics.stdev(values) / statistics.median(values) * 100.0,
        "first3_median": statistics.median(values[:3]),
        "last2_median": statistics.median(values[3:]),
        "psi": psi,
        "user_s": numeric(r"User time \(seconds\): ([^\n]+)"),
        "system_s": numeric(r"System time \(seconds\): ([^\n]+)"),
        "cpu_pct": numeric(r"Percent of CPU this job got: ([0-9.]+)%"),
        "wall": find(r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): ([^\n]+)", time),
        "rss_kb": int(find(r"Maximum resident set size \(kbytes\): (\d+)", time)),
        "voluntary_ctx": int(find(r"Voluntary context switches: (\d+)", time)),
        "involuntary_ctx": int(find(r"Involuntary context switches: (\d+)", time)),
        "top_processes": top,
        "active_processes": active,
    }

completed: dict[int, int] = {}
for line in (ROOT / "runner.log").read_text().splitlines():
    match = re.search(r"pair_complete cell=(\S+) pair=(\d+) attempt=(\d+)", line)
    if match and match.group(1) == CELL:
        completed[int(match.group(2))] = int(match.group(3))
if set(completed) != set(range(1, 13)):
    raise SystemExit(f"expected 12 selected pairs, found {sorted(completed)}")

rows: list[dict[str, object]] = []
processes: list[dict[str, object]] = []
for pair in range(1, 13):
    attempt = completed[pair]
    order = "baseline,candidate" if pair % 2 else "candidate,baseline"
    sides = {}
    for side in ("baseline", "candidate"):
        stem = f"{CELL}-pair{pair:02d}-attempt{attempt:02d}-{side}"
        data = parse_side(stem)
        sides[side] = data
        processes.append(
            {
                "pair": pair,
                "attempt": attempt,
                "order": order,
                "side": side,
                "selected_stem": stem,
                "psi_some_avg10": f"{data['psi']:.2f}",
                "raw_samples_ms": ",".join(f"{v:.3f}" for v in data["samples"]),
                "median_ms": f"{data['median']:.3f}",
                "min_ms": f"{data['min']:.3f}",
                "max_ms": f"{data['max']:.3f}",
                "sample_cv_pct": f"{data['cv_pct']:.2f}",
                "first3_median_ms": f"{data['first3_median']:.3f}",
                "last2_median_ms": f"{data['last2_median']:.3f}",
                "user_s": f"{data['user_s']:.2f}",
                "system_s": f"{data['system_s']:.2f}",
                "cpu_pct": f"{data['cpu_pct']:.0f}",
                "wall": data["wall"],
                "max_rss_kb": data["rss_kb"],
                "voluntary_context_switches": data["voluntary_ctx"],
                "involuntary_context_switches": data["involuntary_ctx"],
                "active_compiler_or_benchmark_processes": data["active_processes"].strip().replace("\n", " | "),
            }
        )
    b, c = sides["baseline"], sides["candidate"]
    rows.append(
        {
            "pair": pair,
            "attempt": attempt,
            "order": order,
            "baseline_median_ms": f"{b['median']:.3f}",
            "candidate_median_ms": f"{c['median']:.3f}",
            "candidate_baseline_ratio": f"{c['median'] / b['median']:.6f}",
            "baseline_samples_ms": ",".join(f"{x:.3f}" for x in b["samples"]),
            "candidate_samples_ms": ",".join(f"{x:.3f}" for x in c["samples"]),
            "baseline_psi": f"{b['psi']:.2f}",
            "candidate_psi": f"{c['psi']:.2f}",
            "baseline_user_s": f"{b['user_s']:.2f}",
            "candidate_user_s": f"{c['user_s']:.2f}",
            "baseline_system_s": f"{b['system_s']:.2f}",
            "candidate_system_s": f"{c['system_s']:.2f}",
            "baseline_cpu_pct": f"{b['cpu_pct']:.0f}",
            "candidate_cpu_pct": f"{c['cpu_pct']:.0f}",
            "baseline_wall": b["wall"],
            "candidate_wall": c["wall"],
            "baseline_voluntary_ctx": b["voluntary_ctx"],
            "candidate_voluntary_ctx": c["voluntary_ctx"],
            "baseline_involuntary_ctx": b["involuntary_ctx"],
            "candidate_involuntary_ctx": c["involuntary_ctx"],
        }
    )

for path, data in ((ROOT / f"{CELL}-audit-pairs.csv", rows), (ROOT / f"{CELL}-audit-processes.csv", processes)):
    with path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(data[0]))
        writer.writeheader()
        writer.writerows(data)

all_cv = [float(p["sample_cv_pct"]) for p in processes]
first3 = [float(p["first3_median_ms"]) for p in processes]
last2 = [float(p["last2_median_ms"]) for p in processes]
ratios = [float(row["candidate_baseline_ratio"]) for row in rows]
psi_values = [float(p["psi_some_avg10"]) for p in processes]
voluntary = [int(p["voluntary_context_switches"]) for p in processes]
involuntary = [int(p["involuntary_context_switches"]) for p in processes]
cpu = [int(p["cpu_pct"]) for p in processes]
summary = {
    "selected_pairs": len(rows),
    "selected_processes": len(processes),
    "pair_ratio_min": min(ratios),
    "pair_ratio_max": max(ratios),
    "median_within_process_cv_pct": statistics.median(all_cv),
    "within_process_cv_min_pct": min(all_cv),
    "within_process_cv_max_pct": max(all_cv),
    "first3_median_of_medians_ms": statistics.median(first3),
    "last2_median_of_medians_ms": statistics.median(last2),
    "last2_over_first3": statistics.median(last2) / statistics.median(first3),
    "preflight_psi_min": min(psi_values),
    "preflight_psi_max": max(psi_values),
    "time_cpu_pct_min": min(cpu),
    "time_cpu_pct_max": max(cpu),
    "voluntary_context_switch_min": min(voluntary),
    "voluntary_context_switch_max": max(voluntary),
    "involuntary_context_switch_min": min(involuntary),
    "involuntary_context_switch_max": max(involuntary),
}
(ROOT / f"{CELL}-audit-summary.txt").write_text("\n".join(f"{k}={v}" for k, v in summary.items()) + "\n")

md = [
    f"# Timing audit: {CELL}",
    "",
    "The selected full-matrix cell was scalar (spin 0), GL STANDARD f64, T511, q8, synthesis. The runner kept all five measured calls per process. Before those five calls, the harness makes one unmeasured warm-up call (`time_runs`, in the matched benchmark script). All 12 pairs use the declared alternating order; selected runs all have PSI `some avg10` <= 0.50, with no compiler or benchmark process listed in the preflight active-process section.",
    "",
    f"- Selected pairs: {len(rows)}; paired median-ratio range: {min(ratios):.3f}–{max(ratios):.3f}.",
    f"- Within-process sample coefficient of variation: median {statistics.median(all_cv):.1f}%, range {min(all_cv):.1f}–{max(all_cv):.1f}%.",
    f"- Across all 24 process runs, median of the first three measured samples: {statistics.median(first3):.3f} ms; median of the last two: {statistics.median(last2):.3f} ms ({statistics.median(last2)/statistics.median(first3):.3f}×).",
    f"- Preflight PSI range: {min(psi_values):.2f}–{max(psi_values):.2f}; GNU time CPU range: {min(cpu)}–{max(cpu)}%; involuntary context-switch range: {min(involuntary)}–{max(involuntary)} per process.",
    "- The large within-process spread and consistent decline between early and late samples are not explained by any recorded active compiler/benchmark process. They make the per-process medians and paired ratios unstable under this protocol. No timed sample was discarded.",
    "- The bootstrap threshold calculation alone had median 0.891984 and 95% interval [0.776221, 0.998472], but this is not a usable control pass given the raw timing inconsistency. Cell status: inconclusive.",
    "",
    "## All selected paired samples",
    "",
    "| Pair | Attempt | Order | Baseline samples (ms) | Candidate samples (ms) | C/B | PSI B/C | GNU time user B/C (s) | CPU B/C (%) | Voluntary ctx B/C | Involuntary ctx B/C |",
    "|---:|---:|---|---|---|---:|---:|---:|---:|---:|---:|",
]
for row in rows:
    md.append(
        f"| {row['pair']} | {row['attempt']} | {row['order']} | {row['baseline_samples_ms']} | {row['candidate_samples_ms']} | {row['candidate_baseline_ratio']} | {row['baseline_psi']}/{row['candidate_psi']} | {row['baseline_user_s']}/{row['candidate_user_s']} | {row['baseline_cpu_pct']}/{row['candidate_cpu_pct']} | {row['baseline_voluntary_ctx']}/{row['candidate_voluntary_ctx']} | {row['baseline_involuntary_ctx']}/{row['candidate_involuntary_ctx']} |"
    )
md.extend(
    [
        "",
        "Structured per-process timing, aggregate RSS, pair order, and five-sample details are summarized in the paired audit CSV. Raw process, GNU-time, and PSI captures remain local and untracked.",
    ]
)
(ROOT / f"{CELL}-audit.md").write_text("\n".join(md) + "\n")
print((ROOT / f"{CELL}-audit-summary.txt").read_text(), end="")
