#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
"""Compare full T1279 scalar f32 outputs from baseline and candidate runs."""

import json
from pathlib import Path

import numpy as np


ROOT = Path(__file__).parent
BASELINE = ROOT / "output-baseline"
CANDIDATE = ROOT / "output-candidate"
FILES = ("spin0-synthesis-channel0.f32", "spin0-analysis-channel0.c32")


def main() -> None:
    results = {}
    for name in FILES:
        baseline_bytes = (BASELINE / name).read_bytes()
        candidate_bytes = (CANDIDATE / name).read_bytes()
        dtype = "<f4" if name.endswith(".f32") else "<c8"
        baseline = np.frombuffer(baseline_bytes, dtype=dtype).astype(
            np.complex128 if dtype == "<c8" else np.float64
        )
        candidate = np.frombuffer(candidate_bytes, dtype=dtype).astype(
            np.complex128 if dtype == "<c8" else np.float64
        )
        if baseline.shape != candidate.shape:
            raise ValueError(f"shape mismatch for {name}")
        difference = candidate - baseline
        results[name] = {
            "elements": int(baseline.size),
            "bitwise_identical": baseline_bytes == candidate_bytes,
            "relative_l2_difference": float(
                np.sqrt(np.vdot(difference, difference).real)
                / np.sqrt(np.vdot(baseline, baseline).real)
            ),
            "max_absolute_difference": float(np.max(np.abs(difference))),
        }
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
