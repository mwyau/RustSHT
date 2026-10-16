#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: BSD-3-Clause
"""Summarize paired f32 real-FFT runs from f32_real_simd.rs."""

from __future__ import annotations

import argparse
import csv
import io
import json
import random
import statistics
import zlib
from pathlib import Path


def interval(values: list[float], seed: int) -> tuple[float, float]:
    rng = random.Random(seed)
    estimates = sorted(
        statistics.median(rng.choices(values, k=len(values))) for _ in range(5000)
    )
    return estimates[124], estimates[4874]


def process_rows(stdout: str) -> dict[tuple[str, int], dict[str, float]]:
    grouped: dict[tuple[str, int], list[dict[str, float]]] = {}
    for row in csv.DictReader(io.StringIO(stdout)):
        if not row.get("mode") or not row.get("n"):
            continue
        key = row["mode"], int(row["n"])
        grouped.setdefault(key, []).append(
            {
                "rustsht_ns": float(row["rustsht_fft_ns"]),
                "rustfft_ns": float(row["rustfft_ns"]),
                "plan_ns": float(row["rustsht_fft_plan_ns"]),
                "workspace_ns": float(row["rustsht_fft_workspace_ns"]),
                "scratch_bytes": float(row["rustsht_fft_scratch_bytes"]),
            }
        )
    return {
        key: {
            field: statistics.median(row[field] for row in samples)
            for field in samples[0]
        }
        for key, samples in grouped.items()
    }


def summarize(path: Path) -> list[dict[str, str | int | float]]:
    records = [json.loads(line) for line in path.read_text().splitlines()]
    pair_count = int(records[0]["pairs"])
    sides: dict[str, dict[int, dict[tuple[str, int], dict[str, float]]]] = {
        "a": {},
        "b": {},
    }
    labels: dict[str, str] = {}
    for process in records[1:]:
        if process["exit_code"] != 0:
            raise SystemExit(f"nonzero process status in {path}")
        label = process["label"]
        if label not in labels.values():
            side = "a" if "a" not in labels else "b"
            labels[side] = label
        else:
            side = next(key for key, value in labels.items() if value == label)
        sides[side][int(process["pair"])] = process_rows(process["stdout"])

    if len(labels) != 2 or any(len(side) != pair_count for side in sides.values()):
        raise SystemExit(f"expected two complete sides with {pair_count} pairs in {path}")

    keys = set.intersection(
        *(set(rows) for side in sides.values() for rows in side.values())
    )
    output = []
    for key in sorted(keys):
        m20 = [sides["a"][pair][key] for pair in range(1, pair_count + 1)]
        m21 = [sides["b"][pair][key] for pair in range(1, pair_count + 1)]
        paired_speedups = [
            before["rustsht_ns"] / after["rustsht_ns"]
            for before, after in zip(m20, m21, strict=True)
        ]
        m21_rustfft_ratios = [
            after["rustfft_ns"] / after["rustsht_ns"] for after in m21
        ]
        speedup_low, speedup_high = interval(
            paired_speedups, zlib.crc32(f"{path.name}:{key}:m20-m21".encode())
        )
        rustfft_low, rustfft_high = interval(
            m21_rustfft_ratios, zlib.crc32(f"{path.name}:{key}:rustfft".encode())
        )
        output.append(
            {
                "comparison_file": path.name,
                "mode": key[0],
                "n": key[1],
                "pairs": pair_count,
                "m20_ns_median": statistics.median(
                    row["rustsht_ns"] for row in m20
                ),
                "m21_ns_median": statistics.median(
                    row["rustsht_ns"] for row in m21
                ),
                "m20_over_m21_speedup": statistics.median(paired_speedups),
                "m20_over_m21_95_low": speedup_low,
                "m20_over_m21_95_high": speedup_high,
                "m21_rustfft_ns_median": statistics.median(
                    row["rustfft_ns"] for row in m21
                ),
                "m21_rustfft_over_m21": statistics.median(m21_rustfft_ratios),
                "m21_rustfft_95_low": rustfft_low,
                "m21_rustfft_95_high": rustfft_high,
                "m20_plan_ns_median": statistics.median(
                    row["plan_ns"] for row in m20
                ),
                "m21_plan_ns_median": statistics.median(
                    row["plan_ns"] for row in m21
                ),
                "m20_workspace_ns_median": statistics.median(
                    row["workspace_ns"] for row in m20
                ),
                "m21_workspace_ns_median": statistics.median(
                    row["workspace_ns"] for row in m21
                ),
                "m20_scratch_bytes": statistics.median(
                    row["scratch_bytes"] for row in m20
                ),
                "m21_scratch_bytes": statistics.median(
                    row["scratch_bytes"] for row in m21
                ),
                "paired_speedups": ";".join(f"{value:.6g}" for value in paired_speedups),
            }
        )
    return output


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("input", type=Path)
    args = parser.parse_args()
    rows = summarize(args.input)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


if __name__ == "__main__":
    main()
