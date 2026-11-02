#!/usr/bin/env python3
"""Run the existing spin-1 parity runner for only B3 f64 GL T720 analysis."""

from __future__ import annotations

import os
import pathlib
import subprocess
import sys

PACKAGE = pathlib.Path(__file__).resolve().parents[1]
REPO = pathlib.Path(os.environ["RUSTSHT_ROOT"]).resolve()
EXPECTED_SOURCE = "85f74d6d91863278f06891205d22d30207e2649d"
actual = subprocess.check_output(
    ["git", "rev-parse", "HEAD"], cwd=REPO, text=True
).strip()
if actual != EXPECTED_SOURCE:
    raise SystemExit(f"expected Rust source {EXPECTED_SOURCE}, got {actual}")

runner_path = REPO / "scripts/benchmark_ducc_spin1_native.py"
source = runner_path.read_text()
old_cases = "cases = make_cases(lmax_values, grids, args.smoke)"
new_cases = (
    'cases = [case for case in make_cases(lmax_values, grids, args.smoke) '
    'if case["latitude_mode"] == "exact" and '
    'case["longitude_mode"] == "even-smooth"]'
)
old_operations = 'for operation in ("analysis", "synthesis"):'
new_operations = 'for operation in ("analysis",):'
if source.count(old_cases) != 1 or source.count(old_operations) != 1:
    raise SystemExit("existing runner shape changed; review the narrow filter")
source = source.replace(old_cases, new_cases).replace(old_operations, new_operations)

sys.path.insert(0, str(runner_path.parent))
sys.argv = [
    str(runner_path),
    "--ducc-src",
    os.environ["DUCC_SRC"],
    "--build-dir",
    os.environ["DUCC_BUILD_DIR"],
    "--results-dir",
    os.environ.get("RESULTS_DIR", str(PACKAGE / "raw/reproduced")),
    "--lmax",
    "720",
    "--grid",
    "GL",
    "--process-pairs",
    "3",
    "--target-ms",
    "25",
    "--skip-build",
]
namespace = {"__file__": str(runner_path), "__name__": "__main__"}
exec(compile(source, str(runner_path), "exec"), namespace)
