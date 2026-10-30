#!/usr/bin/env python3
"""Three-pair GL T720 screen for native-f32 spin-1 analysis."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path


ROOT = next(
    parent for parent in Path(__file__).resolve().parents
    if (parent / "scripts" / "benchmark_ducc_f32_native.py").is_file()
)
sys.path.insert(0, str(ROOT / "scripts"))
import benchmark_ducc_f32_native as bench

BENCH = Path(os.environ["RUSTSHT_B3_SCREEN_DIR"])
RUST = Path(os.environ["RUSTSHT_B3_RUST_F32"])
F64 = Path(os.environ["RUSTSHT_B3_RUST_F64"])
DUCC = Path(os.environ["RUSTSHT_B3_DUCC_F32"])
OUT = BENCH / "t720-3pair.json"
TARGET_MS = 25.0
PHI0 = 0.23
bench.PHI0 = PHI0


def sha(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def run_f64(case: dict, fixture: dict[str, Path], run_dir: Path, cpus: str) -> dict:
    run_dir.mkdir(parents=True, exist_ok=True)
    args = [
        str(F64), "--worker", "analysis", "1", str(case["threads"]),
        str(case["ntheta"]), str(case["nphi"]), str(case["lmax"]),
        str(case["mmax"]), repr(PHI0), str(fixture["map0"]),
        str(fixture["map1"]), str(fixture["alm0"]), str(fixture["alm1"]),
        repr(TARGET_MS),
    ]
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = str(case["threads"])
    started = time.monotonic()
    completed = subprocess.run(
        ["taskset", "-c", cpus, *args], text=True, capture_output=True, env=env
    )
    if completed.returncode:
        raise RuntimeError(f"f64 worker failed: {completed.stderr}\n{completed.stdout}")
    result = json.loads(completed.stdout.strip().splitlines()[-1])
    result["process_elapsed_s"] = time.monotonic() - started
    return result


def timing(result: dict) -> dict:
    return {
        key: result[key]
        for key in (
            "backend", "first_ms", "warmup_ms", "setup_ms", "pool_setup_ms",
            "fixture_load_ms", "repetitions_per_batch", "batch_ms", "batch_total_ms",
            "process_elapsed_s", "finite_outputs", "repeat_consistent", "reset_consistent",
            "simd_arch", "f32_lanes",
        )
        if key in result
    }


def main() -> None:
    cases = [
        {
            "grid": "GL", "ntheta": 721, "nphi": nphi, "lmax": 720,
            "mmax": mmax, "longitude": "even", "spin": 1,
            "operation": "analysis", "threads": threads,
        }
        for mmax, nphi in ((720, 1458), (180, 378))
        for threads in (1, 8)
    ]
    orders = [
        ("rust_f32", "rust_f64", "ducc_f32"),
        ("ducc_f32", "rust_f32", "rust_f64"),
        ("rust_f64", "ducc_f32", "rust_f32"),
    ]
    binaries = {"rust_f32": RUST, "rust_f64": F64, "ducc_f32": DUCC}
    rows = []
    for case in cases:
        case_id = f"GL-T720-M{case['mmax']}-s1-analysis-q{case['threads']}"
        fixture = bench.make_fixture(case, BENCH / "fixtures" / case_id)
        cpus = "0" if case["threads"] == 1 else "0,1,2,3,4,5,6,7"
        pairs = []
        for pair_index, order in enumerate(orders, 1):
            results = {}
            for backend in order:
                run_dir = BENCH / "scratch" / case_id / f"pair-{pair_index}" / backend
                if backend == "rust_f64":
                    results[backend] = run_f64(case, fixture, run_dir, cpus)
                else:
                    results[backend] = bench.run_worker(
                        binaries[backend], backend, case, fixture, run_dir, TARGET_MS, cpus
                    )
            f32_ms = statistics.median(results["rust_f32"]["batch_ms"])
            f64_ms = statistics.median(results["rust_f64"]["batch_ms"])
            ducc_ms = statistics.median(results["ducc_f32"]["batch_ms"])
            error = bench.error_metrics(
                [Path(path) for path in results["ducc_f32"]["outputs"]],
                [Path(path) for path in results["rust_f32"]["outputs"]],
            )
            pairs.append(
                {
                    "pair": pair_index,
                    "execution_order": order,
                    "rust_f32_ms": f32_ms,
                    "rust_f64_ms": f64_ms,
                    "ducc_f32_ms": ducc_ms,
                    "f32_over_f64": f32_ms / f64_ms,
                    "f32_over_ducc": f32_ms / ducc_ms,
                    "error_vs_ducc": error,
                    "rust_f32": timing(results["rust_f32"]),
                    "rust_f64": timing(results["rust_f64"]),
                    "ducc_f32": timing(results["ducc_f32"]),
                }
            )
            for result in results.values():
                for output in result.get("outputs", []):
                    Path(output).unlink(missing_ok=True)
            print(f"completed {case_id} pair {pair_index}", flush=True)

        rows.append(
            {
                "case_id": case_id,
                "fixture_sha256": {name: sha(path) for name, path in fixture.items()},
                "pairs": pairs,
                "median_rust_f32_ms": statistics.median(p["rust_f32_ms"] for p in pairs),
                "median_rust_f64_ms": statistics.median(p["rust_f64_ms"] for p in pairs),
                "median_ducc_f32_ms": statistics.median(p["ducc_f32_ms"] for p in pairs),
                "median_f32_over_f64": statistics.median(p["f32_over_f64"] for p in pairs),
                "median_f32_over_ducc": statistics.median(p["f32_over_ducc"] for p in pairs),
                "f32_over_f64_pair_range": [
                    min(p["f32_over_f64"] for p in pairs),
                    max(p["f32_over_f64"] for p in pairs),
                ],
                "f32_over_ducc_pair_range": [
                    min(p["f32_over_ducc"] for p in pairs),
                    max(p["f32_over_ducc"] for p in pairs),
                ],
            }
        )
    result = {
        "purpose": "native-f32 spin-1 analysis preweighted scaled-channel screen",
        "source_head": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip(),
        "compiler_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "cpu_model": "AMD Ryzen 9 5950X 16-Core Processor",
        "ducc_revision": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "process_pairs": 3,
        "target_ms": TARGET_MS,
        "phi0": PHI0,
        "affinity": {"q1": "0", "q8": "0,1,2,3,4,5,6,7"},
        "workers": {name: {"sha256": sha(path)} for name, path in binaries.items()},
        "cases": rows,
    }
    OUT.write_text(json.dumps(result, indent=2) + "\n")
    print(OUT)


if __name__ == "__main__":
    main()
