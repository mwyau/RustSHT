#!/usr/bin/env python3
from __future__ import annotations
import hashlib
import json
import os
import platform
import random
import statistics
import struct
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
from benchmark_ducc_f32_native import PHI0, error_metrics, make_fixture, run_worker

DUCC_SHA = "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
TARGET_MS = 30.0
PAIRS = 3
TARGET_DIR = Path(os.environ.get("RUSTSHT_TARGET_DIR", str(ROOT / "target")))
F32_BIN = TARGET_DIR / "release/examples/ducc_f32_full_transform_parity"
F64_BIN = TARGET_DIR / "release/examples/ducc_full_transform_parity"
DUCC_WORKER = os.environ.get("RUSTSHT_DUCC_F32_WORKER")
DUCC_BIN = Path(DUCC_WORKER) if DUCC_WORKER else Path()
Q1_CPUSET = os.environ.get("RUSTSHT_Q1_CPUSET")
Q8_CPUSET = os.environ.get("RUSTSHT_Q8_CPUSET")
CPU_DESCRIPTION = os.environ.get("RUSTSHT_CPU_DESCRIPTION", platform.processor() or platform.machine())
OUT = Path(os.environ.get("RUSTSHT_RUN_DIR", str(Path(__file__).resolve().parent / "rerun-baseline"))).resolve()
FIXTURE_DIR = OUT / "fixtures"
SCRATCH = OUT / "scratch"


def sha(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def widen_f32_file(src: Path, dst: Path) -> None:
    raw = src.read_bytes()
    if len(raw) % 4:
        raise ValueError(f"invalid f32 fixture length: {src}")
    vals = struct.unpack(f"<{len(raw)//4}f", raw)
    with dst.open("wb") as f:
        for start in range(0, len(vals), 8192):
            chunk = vals[start:start+8192]
            f.write(struct.pack(f"<{len(chunk)}d", *chunk))


def run_f64(case: dict, fixture64: dict[str, Path], run_dir: Path) -> dict:
    run_dir.mkdir(parents=True, exist_ok=True)
    output = run_dir / "out.f64"
    cpus = Q1_CPUSET if case["threads"] == 1 else Q8_CPUSET
    argv = [
        str(F64_BIN), "--worker", "analysis", str(case["threads"]),
        str(case["ntheta"]), str(case["nphi"]), str(case["lmax"]),
        str(case["mmax"]), repr(PHI0), str(fixture64["map0"]),
        str(fixture64["alm0"]), str(output), repr(TARGET_MS), "0", "GL",
    ]
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = str(case["threads"])
    start = time.monotonic()
    done = subprocess.run(["taskset", "-c", cpus, *argv], text=True, capture_output=True, env=env)
    elapsed = time.monotonic() - start
    if done.returncode:
        raise RuntimeError(f"Rust f64 worker failed ({case}):\n{done.stderr}\n{done.stdout}")
    result = json.loads(done.stdout.strip().splitlines()[-1])
    result["process_elapsed_s"] = elapsed
    result["output"] = output
    result["output_sha256"] = sha(output)
    return result


def case_metrics(result: dict) -> dict:
    return {
        "median_ms": statistics.median(result["batch_ms"]),
        "samples_ms": result["batch_ms"],
        "first_ms": result.get("first_ms"),
        "warmup_ms": result.get("warmup_ms"),
        "repetitions": result.get("repetitions_per_batch", result.get("repetitions")),
        "checksum": result.get("output_checksum"),
        "repeat_consistent": result.get("repeat_consistent"),
        "reset_consistent": result.get("reset_consistent"),
        "binary_output_sha256": result.get("output_sha256"),
    }


def main() -> None:
    if not all((F32_BIN.is_file(), F64_BIN.is_file(), DUCC_BIN.is_file(), Q1_CPUSET, Q8_CPUSET)):
        raise SystemExit("missing one of the Rust f32, Rust f64, or pinned DUCC workers")
    cases = []
    for mmax in (720, 180):
        nphi = 1458 if mmax == 720 else 378
        for threads in (1, 8):
            cases.append({"grid": "GL", "ntheta": 721, "nphi": nphi,
                          "lmax": 720, "mmax": mmax, "longitude": "even",
                          "spin": 0, "operation": "analysis", "threads": threads})
    all_results = []
    for case in cases:
        cid = f"GL-T720-M{case['mmax']}-s0-analysis-q{case['threads']}"
        fixture = make_fixture(case, FIXTURE_DIR / cid)
        fixture64 = {}
        for key in ("map0", "alm0"):
            dst = FIXTURE_DIR / cid / f"{key}.f64"
            widen_f32_file(fixture[key], dst)
            fixture64[key] = dst
        fixture_hashes = {key: sha(path) for key, path in fixture.items()}
        case_pairs = []
        for pair in range(1, PAIRS + 1):
            sequence = ["f32", "ducc", "f64"] if pair == 1 else (["f64", "f32", "ducc"] if pair == 2 else ["ducc", "f64", "f32"])
            results = {}
            for backend in sequence:
                run_dir = SCRATCH / cid / f"pair-{pair}" / backend
                if backend == "f64":
                    results[backend] = run_f64(case, fixture64, run_dir)
                else:
                    binary = F32_BIN if backend == "f32" else DUCC_BIN
                    results[backend] = run_worker(binary, backend, case, fixture, run_dir, TARGET_MS,
                                                  Q1_CPUSET if case["threads"] == 1 else Q8_CPUSET)
            f32, f64, ducc = results["f32"], results["f64"], results["ducc"]
            f32_med = statistics.median(f32["batch_ms"])
            f64_med = statistics.median(f64["batch_ms"])
            ducc_med = statistics.median(ducc["batch_ms"])
            err = error_metrics([Path(p) for p in ducc["outputs"]], [Path(p) for p in f32["outputs"]])
            summary = {
                "pair": pair,
                "execution_order": sequence,
                "f32_ms": f32_med,
                "f64_ms": f64_med,
                "ducc_f32_ms": ducc_med,
                "f32_over_f64": f32_med / f64_med,
                "f32_over_ducc": f32_med / ducc_med,
                "f32_vs_ducc": err,
                "f32": case_metrics(f32),
                "f64": case_metrics(f64),
                "ducc": case_metrics(ducc),
            }
            case_pairs.append(summary)
            for result in results.values():
                for path in result.get("outputs", []):
                    Path(path).unlink(missing_ok=True)
                if result.get("output"):
                    Path(result["output"]).unlink(missing_ok=True)
        all_results.append({
            **case,
            "case_id": cid,
            "fixture_sha256": fixture_hashes,
            "pair_details": case_pairs,
            "median_f32_over_f64": statistics.median(p["f32_over_f64"] for p in case_pairs),
            "median_f32_over_ducc": statistics.median(p["f32_over_ducc"] for p in case_pairs),
            "median_f32_ms": statistics.median(p["f32_ms"] for p in case_pairs),
            "median_f64_ms": statistics.median(p["f64_ms"] for p in case_pairs),
            "median_ducc_f32_ms": statistics.median(p["ducc_f32_ms"] for p in case_pairs),
        })
        print(f"completed {cid}", flush=True)
    output = {
        "purpose": "source-current matched native-f32 scalar-analysis baseline",
        "source_head": subprocess.check_output(["git", "-C", str(ROOT), "rev-parse", "HEAD"], text=True).strip(),
        "ducc_revision": DUCC_SHA,
        "ducc_binary_sha256": sha(DUCC_BIN),
        "rustc_vv": subprocess.check_output(["rustc", "-Vv"], text=True),
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "cpu": f"{CPU_DESCRIPTION}; q1 affinity {Q1_CPUSET}; q8 affinity {Q8_CPUSET}",
        "target_ms": TARGET_MS,
        "process_pairs": PAIRS,
        "input_precision": "fixture generated as f32; exact f32 values widened component-wise for f64 worker",
        "simd": "Pulp runtime dispatch; x86-64-v3 target, AVX2/FMA expected",
        "cases": all_results,
    }
    (OUT / "baseline.json").write_text(json.dumps(output, indent=2) + "\n")
    print(OUT / "baseline.json")

if __name__ == "__main__":
    main()
