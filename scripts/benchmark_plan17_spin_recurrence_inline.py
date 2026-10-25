#!/usr/bin/env python3
"""Matched Plan 17 f32 spin-recurrence baseline/candidate/DUCC screen."""

from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.util
import json
import statistics
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import benchmark_ducc_f32_native as native  # noqa: E402

DUCC_SHA = "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
BASELINE_COMMIT = "b77bdfa2d8f8390d6178e1430c1fea981b05b62d"


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher.hexdigest()


def compact(result: dict) -> dict:
    fields = (
        "first_ms", "warmup_ms", "setup_ms", "pool_setup_ms", "fixture_load_ms",
        "repetitions_per_batch", "batch_ms", "batch_total_ms", "output_checksum",
        "finite_outputs", "repeat_consistent", "reset_consistent", "simd_arch", "f32_lanes",
    )
    return {key: result[key] for key in fields if key in result}


def run_pair(
    binaries: tuple[tuple[str, Path], tuple[str, Path]],
    case: dict,
    fixture: dict,
    pair: int,
    outdir: Path,
    target_ms: float,
    cpus: str,
) -> tuple[dict, dict]:
    order = binaries if pair % 2 else binaries[::-1]
    results = {}
    for name, binary in order:
        result = native.run_worker(
            binary,
            name,
            case,
            fixture,
            outdir / f"pair-{pair}" / name,
            target_ms,
            cpus,
        )
        results[name] = result
    return results[binaries[0][0]], results[binaries[1][0]]


def make_cases(lmaxes: list[int], threads: list[int], operations: list[str]) -> list[dict]:
    cases = []
    for lmax in lmaxes:
        for operation in operations:
            for thread_count in threads:
                cases.append({
                    "grid": "GL",
                    "ntheta": lmax + 1,
                    "nphi": native.smooth_even_at_least(2 * lmax + 2),
                    "lmax": lmax,
                    "mmax": lmax,
                    "longitude": "even",
                    "spin": 1,
                    "operation": operation,
                    "threads": thread_count,
                })
    return cases


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-rust", type=Path, required=True)
    parser.add_argument("--candidate-rust", type=Path, required=True)
    parser.add_argument("--ducc", type=Path, required=True)
    parser.add_argument("--outdir", type=Path, required=True)
    parser.add_argument("--scratch-dir", type=Path)
    parser.add_argument("--lmax", type=int, nargs="+", default=[255, 720])
    parser.add_argument("--threads", type=int, nargs="+", default=[1, 8])
    parser.add_argument("--operations", nargs="+", choices=["analysis", "synthesis"], default=["analysis", "synthesis"])
    parser.add_argument("--process-pairs", type=int, default=3)
    parser.add_argument("--extra-t255-q8-synthesis-pairs", type=int, default=3)
    parser.add_argument("--target-ms", type=float, default=25.0)
    parser.add_argument("--q1-cpu", default="0")
    parser.add_argument("--q8-cpus", default="0,1,2,3,4,5,6,7")
    parser.add_argument("--baseline-commit", default=BASELINE_COMMIT)
    parser.add_argument("--candidate-description", default="spin_f32::recurrence_step with #[inline(always)]")
    args = parser.parse_args()
    for binary in (args.baseline_rust, args.candidate_rust, args.ducc):
        if not binary.is_file():
            raise SystemExit(f"binary does not exist: {binary}")
    if any(value < 0 for value in args.lmax) or any(value < 1 for value in args.threads):
        raise SystemExit("lmax must be nonnegative and thread counts must be positive")

    args.outdir.mkdir(parents=True, exist_ok=True)
    scratch = args.scratch_dir or args.outdir / "scratch"
    cases = make_cases(args.lmax, args.threads, args.operations)
    full_results = []
    sample_rows = []
    cold_warm_rows = []

    for case in cases:
        case_id = (
            f"{case['grid']}-T{case['lmax']}-M{case['mmax']}-{case['longitude']}"
            f"-s1-{case['operation']}-q{case['threads']}"
        )
        fixture = native.make_fixture(case, scratch / "fixtures" / case_id)
        affinity = args.q1_cpu if case["threads"] == 1 else args.q8_cpus
        ab_pairs = args.process_pairs
        if case == {
            **case,
            "lmax": 255,
            "threads": 8,
            "operation": "synthesis",
        }:
            ab_pairs += args.extra_t255_q8_synthesis_pairs

        ab_results = []
        for pair in range(1, ab_pairs + 1):
            baseline, candidate = run_pair(
                (("baseline", args.baseline_rust), ("candidate", args.candidate_rust)),
                case,
                fixture,
                pair,
                args.outdir / case_id / "rust-ab",
                args.target_ms,
                affinity,
            )
            error = native.error_metrics(
                [Path(path) for path in baseline["outputs"]],
                [Path(path) for path in candidate["outputs"]],
            )
            pair_result = {
                "pair": pair,
                "baseline": compact(baseline),
                "candidate": compact(candidate),
                "candidate_over_baseline": statistics.median(candidate["batch_ms"])
                / statistics.median(baseline["batch_ms"]),
                "output_error": error,
            }
            ab_results.append(pair_result)
            for name, result in (("baseline", baseline), ("candidate", candidate)):
                cold_warm_rows.append({
                    "case_id": case_id,
                    "comparison": "rust-before-after",
                    "variant": name,
                    "pair": pair,
                    "setup_ms": result.get("setup_ms"),
                    "first_ms": result.get("first_ms"),
                    "warmup_ms": result.get("warmup_ms"),
                    "median_warm_ms": statistics.median(result["batch_ms"]),
                })
                for index, value in enumerate(result["batch_ms"], 1):
                    sample_rows.append({
                        "case_id": case_id,
                        "comparison": "rust-before-after",
                        "variant": name,
                        "pair": pair,
                        "sample_index": index,
                        "latency_ms": value,
                    })
            print("Completed Rust baseline/candidate benchmark pair", flush=True)

        ducc_pairs = []
        for pair in range(1, args.process_pairs + 1):
            candidate, ducc = run_pair(
                (("candidate", args.candidate_rust), ("DUCC", args.ducc)),
                case,
                fixture,
                pair,
                args.outdir / case_id / "candidate-ducc",
                args.target_ms,
                affinity,
            )
            error = native.error_metrics(
                [Path(path) for path in ducc["outputs"]],
                [Path(path) for path in candidate["outputs"]],
            )
            pair_result = {
                "pair": pair,
                "candidate": compact(candidate),
                "DUCC": compact(ducc),
                "candidate_over_DUCC": statistics.median(candidate["batch_ms"])
                / statistics.median(ducc["batch_ms"]),
                "output_error": error,
            }
            ducc_pairs.append(pair_result)
            for name, result in (("candidate", candidate), ("DUCC", ducc)):
                for index, value in enumerate(result["batch_ms"], 1):
                    sample_rows.append({
                        "case_id": case_id,
                        "comparison": "candidate-vs-DUCC",
                        "variant": name,
                        "pair": pair,
                        "sample_index": index,
                        "latency_ms": value,
                    })
            print("Completed Rust/DUCC benchmark pair", flush=True)

        full_results.append({"case": case, "case_id": case_id, "rust_ab": ab_results, "candidate_ducc": ducc_pairs})
        (args.outdir / "results.json").write_text(json.dumps({"metadata": metadata(args), "cases": full_results}, indent=2) + "\n")
        write_csv(args.outdir / "samples.csv", sample_rows)
        write_csv(args.outdir / "cold-warm.csv", cold_warm_rows)


def metadata(args: argparse.Namespace) -> dict:
    return {
        "baseline_commit": args.baseline_commit,
        "candidate_description": args.candidate_description,
        "ducc_sha": DUCC_SHA,
        "rustc_vv": subprocess.check_output(["rustc", "-Vv"], text=True),
        "rust_flags": "Cargo release defaults; runtime Pulp dispatch; no target-cpu override",
        "ducc_flags": "-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; strict driver separately compiled",
        "host": subprocess.check_output(["lscpu"], text=True).split("Model name:")[1].splitlines()[0].strip(),
        "affinity_method": "pin q1 to one physical core and q8 to eight cores in one L3 domain; run-specific assignments are in the report",
        "process_pairs": args.process_pairs,
        "extra_t255_q8_synthesis_pairs": args.extra_t255_q8_synthesis_pairs,
        "target_ms": args.target_ms,
        "method": "same deterministic fixtures; fresh interleaved processes; reusable plans/workspaces; per-worker warmup and output reset validation",
        "baseline_binary_sha256": digest(args.baseline_rust),
        "candidate_binary_sha256": digest(args.candidate_rust),
        "ducc_binary_sha256": digest(args.ducc),
    }


def write_csv(path: Path, rows: list[dict]) -> None:
    if not rows:
        return
    with path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


if __name__ == "__main__":
    main()
