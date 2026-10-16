#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
import json
import re
import statistics
from pathlib import Path


ROOT = Path(__file__).resolve().parent
PAIRED = ROOT / "paired"
LABEL = re.compile(r"pair-(\d+)-(baseline|candidate)-spin(\d+)")


def sample_values(lines: list[str], name: str) -> list[float]:
    prefix = f"{name}_samples_ms="
    for line in lines:
        if line.startswith(prefix):
            return [float(value) for value in line[len(prefix) :].split(",")]
    return []


processes = {}
for log_path in sorted(PAIRED.glob("*.log")):
    match = LABEL.fullmatch(log_path.stem)
    if match is None:
        continue
    pair, strategy, spin = match.groups()
    lines = log_path.read_text().splitlines()
    if not lines:
        continue
    process_key = (int(pair), int(spin), strategy)
    processes[process_key] = {
        "log": log_path.name,
        "measurements": {},
    }
    for operation in ("synthesis", "analysis"):
        for label, threads in (("q1", 1), ("qn", 8)):
            name = f"{operation}_{label}"
            values = sample_values(lines, name)
            if values:
                processes[process_key]["measurements"][f"{operation}_q{threads}"] = {
                    "samples_ms": values,
                    "median_ms": statistics.median(values),
                }

rows = []
for pair, spin, strategy in sorted(processes):
    if strategy != "baseline":
        continue
    candidate = processes.get((pair, spin, "candidate"))
    baseline = processes[(pair, spin, "baseline")]
    if candidate is None:
        continue
    for operation in ("synthesis", "analysis"):
        for threads in (1, 8):
            name = f"{operation}_q{threads}"
            old = baseline["measurements"].get(name)
            new = candidate["measurements"].get(name)
            if old is None or new is None:
                continue
            rows.append(
                {
                    "pair": pair,
                    "spin": spin,
                    "operation": operation,
                    "threads": threads,
                    "baseline_ms": old["median_ms"],
                    "candidate_ms": new["median_ms"],
                    "change_percent": 100.0
                    * (new["median_ms"] / old["median_ms"] - 1.0),
                    "baseline_samples_ms": old["samples_ms"],
                    "candidate_samples_ms": new["samples_ms"],
                }
            )

summary = {
    "method": "arithmetic median of raw warm samples; change is candidate / baseline - 1",
    "interpretation": {
        "spin_0": "single-pair diagnostic observations; candidate scalar process overlapped an unrelated benchmark with unknown CPU placement",
        "spin_1": "unaffected-path control variation; the selector changes only the scalar f32 recurrence",
    },
    "processes": [
        {
            "pair": pair,
            "spin": spin,
            "strategy": strategy,
            **process,
        }
        for (pair, spin, strategy), process in sorted(processes.items())
    ],
    "paired_rows": rows,
}
(ROOT / "paired-summary.json").write_text(json.dumps(summary, indent=2) + "\n")

print("pair spin operation threads baseline_ms candidate_ms change_percent")
for row in rows:
    print(
        f"{row['pair']} {row['spin']} {row['operation']} {row['threads']} "
        f"{row['baseline_ms']:.3f} {row['candidate_ms']:.3f} "
        f"{row['change_percent']:+.2f}%"
    )
