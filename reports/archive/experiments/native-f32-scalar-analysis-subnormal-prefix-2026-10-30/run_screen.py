#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import os
import pathlib
import statistics
import subprocess
import sys
import time

ROOT = pathlib.Path(os.environ["RUSTSHT_B1_WORKTREE"])
BUILD = pathlib.Path(os.environ["RUSTSHT_B1_BUILD"])
sys.path.insert(0, str(ROOT / "scripts"))
import benchmark_ducc_f32_native as bench

RUST = pathlib.Path(os.environ.get(
    "RUSTSHT_B1_SCREEN_RUST",
    BUILD / "f32-target/release/examples/ducc_f32_full_transform_parity",
))
F64 = BUILD / "f64-target/release/m6_b2_f64_worker"
DUCC = pathlib.Path(os.environ["RUSTSHT_B1_DUCC"])
RUN_ROOT = pathlib.Path(os.environ.get("RUSTSHT_B1_SCREEN_OUTDIR", BUILD / "out"))
OUT = RUN_ROOT / "three-pair-screen.json"
SCRATCH = BUILD / "scratch"
TARGET_MS = 30.0
PAIRS = 3


def sha(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def run_f64(case: dict, fixture: dict[str, pathlib.Path], run_dir: pathlib.Path) -> dict:
    run_dir.mkdir(parents=True, exist_ok=True)
    cpus = "0" if case["threads"] == 1 else "0-7"
    args = [
        str(F64), "--worker", case["operation"], str(case["spin"]),
        str(case["threads"]), str(case["ntheta"]), str(case["nphi"]),
        str(case["lmax"]), str(case["mmax"]), repr(bench.PHI0),
        str(fixture["map0"]), str(fixture["map1"]), str(fixture["alm0"]),
        str(fixture["alm1"]), str(TARGET_MS), "-",
    ]
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = str(case["threads"])
    started = time.monotonic()
    result = subprocess.run(["taskset", "-c", cpus, *args], text=True, capture_output=True, env=env, check=True)
    data = json.loads(result.stdout.strip().splitlines()[-1])
    data["median_ms"] = statistics.median(data["batch_ms"])
    data["binary_sha256"] = sha(F64)
    data["process_elapsed_s"] = time.monotonic() - started
    (run_dir / "result.json").write_text(json.dumps(data, indent=2) + "\n")
    return data


def main() -> None:
    if sha(DUCC) != "1e8412966f7bf306a1fea8141969208f661d5a705f7737e7a8f0dc1cc74cc36d":
        raise RuntimeError("DUCC worker hash is not the pinned recorded worker")
    cases = [
        case for case in bench.default_cases(False)
        if case["grid"] == "GL" and case["lmax"] == 720
        and case["spin"] == 0 and case["operation"] == "analysis"
    ]
    if os.environ.get("RUSTSHT_B1_SCREEN_FULL_Q1") == "1":
        cases = [case for case in cases if case["mmax"] == 720 and case["threads"] == 1]
    rows = []
    started = time.time()
    for case in cases:
        case_id = f"GL-T720-M{case['mmax']}-analysis-q{case['threads']}"
        fixture = bench.make_fixture(case, SCRATCH / case_id)
        pair_rows = []
        for pair in range(1, PAIRS + 1):
            order = ("rust", "f64", "ducc") if pair % 3 == 1 else (
                ("ducc", "rust", "f64") if pair % 3 == 2 else ("f64", "ducc", "rust")
            )
            results = {}
            for backend in order:
                run_dir = RUN_ROOT / case_id / f"pair-{pair}" / backend
                if backend == "f64":
                    data = run_f64(case, fixture, run_dir)
                else:
                    binary = RUST if backend == "rust" else DUCC
                    cpus = "0" if case["threads"] == 1 else "0,1,2,3,4,5,6,7"
                    data = bench.run_worker(binary, backend, case, fixture, run_dir, TARGET_MS, cpus)
                    data["median_ms"] = statistics.median(data["batch_ms"])
                    data["binary_sha256"] = sha(binary)
                    if backend == "rust":
                        candidate_outputs = [pathlib.Path(p) for p in data["outputs"]]
                    else:
                        ducc_outputs = [pathlib.Path(p) for p in data["outputs"]]
                results[backend] = data
            error = bench.error_metrics(ducc_outputs, candidate_outputs)
            pair_rows.append({
                "pair": pair,
                "order": order,
                "f32_ms": results["rust"]["median_ms"],
                "f64_ms": results["f64"]["median_ms"],
                "ducc_f32_ms": results["ducc"]["median_ms"],
                "f32_over_f64": results["rust"]["median_ms"] / results["f64"]["median_ms"],
                "f32_over_ducc": results["rust"]["median_ms"] / results["ducc"]["median_ms"],
                "error_vs_ducc": error,
                "workers": results,
            })
            for path in (*candidate_outputs, *ducc_outputs):
                path.unlink(missing_ok=True)
            print(case_id, pair, pair_rows[-1]["f32_ms"], pair_rows[-1]["f64_ms"], pair_rows[-1]["ducc_f32_ms"], flush=True)
        rows.append({
            **case,
            "case_id": case_id,
            "fixture_sha256": {name: sha(path) for name, path in fixture.items()},
            "median_f32_ms": statistics.median(row["f32_ms"] for row in pair_rows),
            "median_f64_ms": statistics.median(row["f64_ms"] for row in pair_rows),
            "median_ducc_f32_ms": statistics.median(row["ducc_f32_ms"] for row in pair_rows),
            "median_f32_over_f64": statistics.median(row["f32_over_f64"] for row in pair_rows),
            "median_f32_over_ducc": statistics.median(row["f32_over_ducc"] for row in pair_rows),
            "pairs": pair_rows,
        })
    metadata = {
        "purpose": "isolated native-f32 scalar-analysis subnormal-prefix three-pair screen",
        "source_head": subprocess.check_output(["git", "-C", str(ROOT), "rev-parse", "HEAD"], text=True).strip(),
        "candidate_patch_sha256": hashlib.sha256(subprocess.check_output(["git", "-C", str(ROOT), "diff", "--binary", "HEAD"])).hexdigest(),
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "ducc_revision": bench.DUCC_SHA,
        "ducc_binary_sha256": sha(DUCC),
        "rust_binary_sha256": sha(RUST),
        "f64_binary_sha256": sha(F64),
        "process_pairs": PAIRS,
        "target_ms": TARGET_MS,
        "q1_affinity": "0",
        "q8_affinity": "0-7",
        "elapsed_seconds": time.time() - started,
        "cases": rows,
    }
    OUT.write_text(json.dumps(metadata, indent=2) + "\n")
    print(f"SAVED {OUT}", flush=True)


if __name__ == "__main__":
    main()
