#!/usr/bin/env python3
"""Focused current-source T720 spin-1 GL/CC/F1 integration screen."""

from __future__ import annotations

import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))

import benchmark_ducc_spin1_native as benchmark  # noqa: E402

make_cases = benchmark.make_cases


def exact_smooth_t720(lmax_values: list[int], grids: list[str], smoke: bool) -> list[dict]:
    cases = make_cases(lmax_values, grids, False)
    return [
        case
        for case in cases
        if case["lmax"] == 720
        and case["latitude_mode"] == "exact"
        and case["longitude_mode"] == "even-smooth"
    ]


benchmark.make_cases = exact_smooth_t720

if __name__ == "__main__":
    raise SystemExit(benchmark.main())
