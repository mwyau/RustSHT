#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import statistics
import struct
import subprocess
import sys
import time
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from benchmark_ducc_f32_native import error_metrics, make_fixture, run_worker

DUCC_REVISION = "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
PHI0 = 0.23
TARGET_MS = 25.0
PAIRS = 3


def sha(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def widen_f32_file(source: Path, destination: Path) -> None:
    np.fromfile(source, dtype="<f4").astype("<f8").tofile(destination)


def run_f64(binary: Path, case: dict, fixture64: dict[str, Path], outdir: Path,
            cpus: str, target_ms: float) -> dict:
    outdir.mkdir(parents=True, exist_ok=True)
    out = {name: outdir / f"{name}.f64" for name in ("map0", "map1", "alm0", "alm1")}
    argv = [
        str(binary), "--worker", "analysis", "GL", str(case["threads"]),
        str(case["ntheta"]), str(case["nphi"]), str(case["lmax"]), str(case["mmax"]),
        repr(PHI0), str(fixture64["map0"]), str(fixture64["map1"]),
        str(fixture64["alm0"]), str(fixture64["alm1"]),
        str(out["map0"]), str(out["map1"]), str(out["alm0"]), str(out["alm1"]),
        repr(target_ms),
    ]
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = str(case["threads"])
    started = time.monotonic()
    done = subprocess.run(["taskset", "-c", cpus, *argv], text=True, capture_output=True, env=env)
    if done.returncode:
        raise RuntimeError(f"Rust f64 worker failed for {case}:\n{done.stderr}\n{done.stdout}")
    result = json.loads(done.stdout.strip().splitlines()[-1])
    result["process_elapsed_s"] = time.monotonic() - started
    result["outputs"] = [str(out["alm0"]), str(out["alm1"])]
    return result


def fixture_hashes(fixture: dict[str, Path]) -> dict[str, str]:
    return {name: sha(path) for name, path in fixture.items()}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-f32", type=Path, required=True)
    parser.add_argument("--rust-f64", type=Path, required=True)
    parser.add_argument("--ducc-f32", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--source-head", required=True)
    parser.add_argument("--source-patch-sha256", default=None)
    parser.add_argument("--tile-size", type=int, required=True)
    parser.add_argument("--q1-cpu", required=True)
    parser.add_argument("--q8-cpus", required=True)
    args = parser.parse_args()
    for binary in (args.rust_f32, args.rust_f64, args.ducc_f32):
        if not binary.is_file():
            parser.error(f"worker executable does not exist: {binary}")
    args.out.mkdir(parents=True, exist_ok=True)
    fixtures_dir = args.out / "fixtures"
    scratch_dir = args.out / "scratch"
    cases = []
    for mmax, nphi in ((720, 1458), (180, 378)):
        for threads in (1, 8):
            cases.append({
                "grid": "GL", "ntheta": 721, "nphi": nphi,
                "lmax": 720, "mmax": mmax, "longitude": "even",
                "spin": 1, "operation": "analysis", "threads": threads,
            })
    result_cases = []
    for case in cases:
        case_id = f"GL-T720-M{case['mmax']}-even-s1-analysis-q{case['threads']}"
        fixture = make_fixture(case, fixtures_dir / case_id)
        widened = {}
        for name, source in fixture.items():
            destination = fixtures_dir / case_id / f"{name}.f64"
            widen_f32_file(source, destination)
            widened[name] = destination
        cpus = args.q1_cpu if case["threads"] == 1 else args.q8_cpus
        pair_details = []
        for pair in range(1, PAIRS + 1):
            order = (["rust_f32", "ducc_f32", "rust_f64"] if pair == 1 else
                     ["rust_f64", "rust_f32", "ducc_f32"] if pair == 2 else
                     ["ducc_f32", "rust_f64", "rust_f32"])
            results = {}
            for backend in order:
                outdir = scratch_dir / case_id / f"pair-{pair}" / backend
                if backend == "rust_f64":
                    results[backend] = run_f64(args.rust_f64, case, widened, outdir, cpus, TARGET_MS)
                else:
                    binary = args.rust_f32 if backend == "rust_f32" else args.ducc_f32
                    results[backend] = run_worker(binary, backend, case, fixture, outdir, TARGET_MS, cpus)
            f32 = results["rust_f32"]
            f64 = results["rust_f64"]
            ducc = results["ducc_f32"]
            f32_ms = statistics.median(f32["batch_ms"])
            f64_ms = statistics.median(f64["batch_ms"])
            ducc_ms = statistics.median(ducc["batch_ms"])
            error = error_metrics([Path(path) for path in ducc["outputs"]],
                                  [Path(path) for path in f32["outputs"]])
            pair_details.append({
                "pair": pair,
                "execution_order": order,
                "f32_ms": f32_ms,
                "f64_ms": f64_ms,
                "ducc_f32_ms": ducc_ms,
                "f32_over_f64": f32_ms / f64_ms,
                "f32_over_ducc": f32_ms / ducc_ms,
                "f32_vs_ducc": error,
            })
            for item in results.values():
                for path in item.get("outputs", []):
                    Path(path).unlink(missing_ok=True)
        result_cases.append({
            **case,
            "case_id": case_id,
            "fixture_sha256": fixture_hashes(fixture),
            "median_f32_ms": statistics.median(p["f32_ms"] for p in pair_details),
            "median_f64_ms": statistics.median(p["f64_ms"] for p in pair_details),
            "median_ducc_f32_ms": statistics.median(p["ducc_f32_ms"] for p in pair_details),
            "median_f32_over_f64": statistics.median(p["f32_over_f64"] for p in pair_details),
            "median_f32_over_ducc": statistics.median(p["f32_over_ducc"] for p in pair_details),
            "max_relative_l2_vs_ducc": max(p["f32_vs_ducc"]["relative_l2"] for p in pair_details),
            "max_absolute_error_vs_ducc": max(p["f32_vs_ducc"]["max_absolute"] for p in pair_details),
            "pair_details": pair_details,
        })
        print(f"completed {case_id}", flush=True)
    output = {
        "purpose": "matched native-f32 spin-1 analysis tile schedule screen",
        "source_head": args.source_head,
        "source_patch_sha256": args.source_patch_sha256,
        "ducc_revision": DUCC_REVISION,
        "ducc_binary_sha256": sha(args.ducc_f32),
        "rust_f32_binary_sha256": sha(args.rust_f32),
        "rust_f64_binary_sha256": sha(args.rust_f64),
        "rustc_vv": subprocess.check_output(["rustc", "-Vv"], text=True),
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "cpu_model": os.environ.get("RUSTSHT_CPU_DESCRIPTION", "unspecified"),
        "affinity": {"q1": args.q1_cpu, "q8": args.q8_cpus},
        "process_pairs": PAIRS,
        "target_ms": TARGET_MS,
        "input_precision": "f32 fixtures shared by f32 Rust and DUCC; components widened for matched f64 worker",
        "tile_size": args.tile_size,
        "cases": result_cases,
    }
    (args.out / "result.json").write_text(json.dumps(output, indent=2) + "\n")
    print(args.out / "result.json")


if __name__ == "__main__":
    main()
