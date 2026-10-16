# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

import csv
import re
from pathlib import Path


root = Path(__file__).parent / "python-ducc-high"
columns = [
    "lmax",
    "ntheta",
    "nphi",
    "spin",
    "threads",
    "rust_synthesis_ms",
    "ducc_synthesis_ms",
    "synthesis_rust_ducc_ratio",
    "synthesis_relative_l2",
    "rust_analysis_ms",
    "ducc_analysis_ms",
    "analysis_rust_ducc_ratio",
    "analysis_relative_l2",
    "repeat_rust_synthesis_ms",
    "repeat_ducc_synthesis_ms",
    "repeat_synthesis_relative_l2",
    "repeat_rust_analysis_ms",
    "repeat_ducc_analysis_ms",
    "repeat_analysis_relative_l2",
    "peak_rss_kib",
    "wall_seconds",
]


def median(text: str, method: str) -> float:
    match = re.search(
        rf"^{method}_run_medians_ms=.*?median=([\d.]+)", text, re.MULTILINE
    )
    if match is None:
        raise ValueError(f"missing median for {method}")
    return float(match.group(1))


def relative_error(text: str, operation: str) -> float:
    match = re.search(rf"^{operation}_error=.*?rel_l2=([\deE+.-]+)", text, re.MULTILINE)
    if match is None:
        raise ValueError(f"missing {operation} error")
    return float(match.group(1))


rows = []
for log_path in sorted(root.glob("T*.log")):
    match = re.fullmatch(r"T(\d+)-nphi(\d+)-spin(\d+)-q(\d+)", log_path.stem)
    if match is None:
        continue
    lmax, nphi, spin, threads = map(int, match.groups())
    text = log_path.read_text()
    rust_synthesis = median(text, "synthesis_rust_plan")
    ducc_synthesis = median(text, "synthesis_ducc")
    rust_analysis = median(text, "analysis_rust_plan")
    ducc_analysis = median(text, "analysis_ducc")
    time_text = log_path.with_suffix(".time").read_text()
    rss = re.search(r"peak_rss_kib=(\d+)", time_text)
    wall = re.search(r"wall_seconds=([\d.]+)", time_text)
    row = {
        "lmax": lmax,
        "ntheta": lmax + 1,
        "nphi": nphi,
        "spin": spin,
        "threads": threads,
        "rust_synthesis_ms": rust_synthesis,
        "ducc_synthesis_ms": ducc_synthesis,
        "synthesis_rust_ducc_ratio": rust_synthesis / ducc_synthesis,
        "synthesis_relative_l2": relative_error(text, "synthesis"),
        "rust_analysis_ms": rust_analysis,
        "ducc_analysis_ms": ducc_analysis,
        "analysis_rust_ducc_ratio": rust_analysis / ducc_analysis,
        "analysis_relative_l2": relative_error(text, "analysis"),
        "repeat_rust_synthesis_ms": "",
        "repeat_ducc_synthesis_ms": "",
        "repeat_synthesis_relative_l2": "",
        "repeat_rust_analysis_ms": "",
        "repeat_ducc_analysis_ms": "",
        "repeat_analysis_relative_l2": "",
        "peak_rss_kib": rss.group(1) if rss else "",
        "wall_seconds": wall.group(1) if wall else "",
    }
    repeat = root / "repeat-after-build" / log_path.name
    if repeat.exists():
        repeat_text = repeat.read_text()
        row.update(
            {
                "repeat_rust_synthesis_ms": median(repeat_text, "synthesis_rust_plan"),
                "repeat_ducc_synthesis_ms": median(repeat_text, "synthesis_ducc"),
                "repeat_synthesis_relative_l2": relative_error(repeat_text, "synthesis"),
                "repeat_rust_analysis_ms": median(repeat_text, "analysis_rust_plan"),
                "repeat_ducc_analysis_ms": median(repeat_text, "analysis_ducc"),
                "repeat_analysis_relative_l2": relative_error(repeat_text, "analysis"),
            }
        )
    rows.append(row)

with (root / "summary.csv").open("w", newline="") as output:
    writer = csv.DictWriter(output, fieldnames=columns)
    writer.writeheader()
    writer.writerows(rows)
