#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
"""Compare complete f32 RustSHT output arrays before and after the skip path."""

import json
from pathlib import Path
import sys

import numpy as np


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit(
            f"usage: {Path(sys.argv[0]).name} BASELINE_OUTPUT_DIRECTORY CANDIDATE_OUTPUT_DIRECTORY"
        )
    baseline_dir = Path(sys.argv[1])
    candidate_dir = Path(sys.argv[2])
    results = {}
    for baseline_path in sorted(baseline_dir.iterdir()):
        candidate_path = candidate_dir / baseline_path.name
        baseline_bytes = baseline_path.read_bytes()
        candidate_bytes = candidate_path.read_bytes()
        dtype = "<f4" if baseline_path.suffix == ".f32" else "<c8"
        baseline = np.frombuffer(baseline_bytes, dtype=dtype).astype(
            np.complex128 if dtype == "<c8" else np.float64
        )
        candidate = np.frombuffer(candidate_bytes, dtype=dtype).astype(
            np.complex128 if dtype == "<c8" else np.float64
        )
        if baseline.shape != candidate.shape:
            raise ValueError(f"shape mismatch for {baseline_path.name}")
        difference = candidate - baseline
        magnitude = np.abs(difference)
        baseline_norm = np.sqrt(np.vdot(baseline, baseline).real)
        results[baseline_path.name] = {
            "elements": int(baseline.size),
            "bitwise_identical": baseline_bytes == candidate_bytes,
            "relative_l2_difference": float(
                np.sqrt(np.vdot(difference, difference).real) / baseline_norm
            ),
            "max_absolute_difference": float(np.max(magnitude)),
        }
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
