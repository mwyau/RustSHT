#!/usr/bin/env python3
"""Focused exact-grid CC cases for the spin-1 resampling A/B."""

from __future__ import annotations

import os
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import benchmark_ducc_spin1_native as benchmark


_make_cases = benchmark.make_cases


def make_cases(lmax_values: list[int], grids: list[str], smoke: bool) -> list[dict]:
    cases = _make_cases(lmax_values, ["CC"], False)
    return [
        case
        for case in cases
        if case["lmax"] in {255, 720, 1023}
        and case["latitude_mode"] == "exact"
        and (case["lmax"] == 720 or case["longitude_mode"] == "even-smooth")
    ]


benchmark.make_cases = make_cases
if root := os.environ.get("RUSTSHT_BENCH_ROOT"):
    benchmark.ROOT = pathlib.Path(root).resolve()
    benchmark.common.ROOT = benchmark.ROOT

if __name__ == "__main__":
    raise SystemExit(benchmark.main())
