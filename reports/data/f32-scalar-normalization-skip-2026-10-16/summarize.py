#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
"""Summarize paired T1023 f32 scalar normalization-screen processes."""

import json
from pathlib import Path
import re
import statistics
import sys


METRICS = (
    "synthesis_q1_median_ms",
    "synthesis_qn_median_ms",
    "analysis_q1_median_ms",
    "analysis_qn_median_ms",
)


def read_values(path: Path) -> dict[str, float]:
    values: dict[str, float] = {}
    for line in path.read_text().splitlines():
        for match in re.finditer(r"([a-zA-Z0-9_]+)=([-+0-9.eE]+)", line):
            values[match.group(1)] = float(match.group(2))
    return values


def main() -> None:
    directory = Path(sys.argv[1]) if len(sys.argv) == 2 else Path(__file__).with_name("paired")
    indices = sorted(
        int(path.name.split("-")[1])
        for path in directory.glob("pair-*-baseline.log")
    )
    if not indices:
        raise ValueError(f"no paired process logs found in {directory}")
    pairs = []
    for index in indices:
        baseline = read_values(directory / f"pair-{index}-baseline.log")
        candidate = read_values(directory / f"pair-{index}-candidate.log")
        pairs.append(
            {
                "pair": index,
                "baseline": baseline,
                "candidate": candidate,
                "candidate_change_percent": {
                    metric: 100.0 * (candidate[metric] / baseline[metric] - 1.0)
                    for metric in METRICS
                },
            }
        )

    summary = {}
    for metric in METRICS:
        paired_changes = [pair["candidate_change_percent"][metric] for pair in pairs]
        baseline_values = [pair["baseline"][metric] for pair in pairs]
        candidate_values = [pair["candidate"][metric] for pair in pairs]
        summary[metric] = {
            "baseline_process_median_ms": statistics.median(baseline_values),
            "candidate_process_median_ms": statistics.median(candidate_values),
            "paired_change_median_percent": statistics.median(paired_changes),
            "paired_change_range_percent": [min(paired_changes), max(paired_changes)],
            "paired_changes_percent": paired_changes,
        }

    rss = {
        strategy: [
            read_values(directory / f"pair-{index}-{strategy}.time")["max_rss_kib"]
            for index in indices
        ]
        for strategy in ("baseline", "candidate")
    }
    print(
        json.dumps(
            {"pair_count": len(pairs), "metrics": summary, "process_max_rss_kib": rss},
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
