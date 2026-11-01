#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import math
import os
import platform
import statistics
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from benchmark_ducc_f32_native import DUCC_SHA, error_metrics, make_fixture, run_worker


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def paired_ratio_summary(values: list[float]) -> dict[str, float]:
    logs = [math.log(value) for value in values]
    mean = statistics.mean(logs)
    margin = 2.1447866879169273 * statistics.stdev(logs) / math.sqrt(len(logs))
    return {
        "geometric_mean": math.exp(mean),
        "ci95_low": math.exp(mean - margin),
        "ci95_high": math.exp(mean + margin),
    }


def main() -> None:
    parent = Path(os.environ["RUSTSHT_PARENT_F32"])
    candidate = Path(os.environ["RUSTSHT_CANDIDATE_F32"])
    ducc = Path(os.environ["RUSTSHT_DUCC_F32"])
    out = Path(os.environ["RUSTSHT_RUN_DIR"])
    q1_cpus = os.environ["RUSTSHT_Q1_CPUSET"]
    q8_cpus = os.environ["RUSTSHT_Q8_CPUSET"]
    target_ms = float(os.environ.get("RUSTSHT_TARGET_MS", "25"))
    pair_count = int(os.environ.get("RUSTSHT_PAIRS", "3"))
    scratch = out / "scratch"
    results = []
    base_orders = [
        ("parent", "ducc", "candidate"),
        ("candidate", "parent", "ducc"),
        ("ducc", "candidate", "parent"),
    ]
    pairs = [base_orders[index % len(base_orders)] for index in range(pair_count)]
    binaries = {"parent": parent, "candidate": candidate, "ducc": ducc}
    for mmax in (720, 180):
        nphi = 1458 if mmax == 720 else 378
        for threads in (1, 8):
            case = {
                "grid": "GL", "ntheta": 721, "nphi": nphi, "lmax": 720,
                "mmax": mmax, "longitude": "even", "spin": 0,
                "operation": "analysis", "threads": threads,
            }
            case_id = f"GL-T720-M{mmax}-analysis-q{threads}"
            fixture_dir = out / "fixtures" / case_id
            fixture = make_fixture(case, fixture_dir)
            fixture_hashes = {key: sha(path) for key, path in fixture.items()}
            case_pairs = []
            for pair_index, order in enumerate(pairs, 1):
                outputs = {}
                for backend in order:
                    cpus = q1_cpus if threads == 1 else q8_cpus
                    run_dir = scratch / case_id / f"pair-{pair_index}" / backend
                    outputs[backend] = run_worker(
                        binaries[backend], backend, case, fixture, run_dir,
                        target_ms, cpus,
                    )
                medians = {
                    key: statistics.median(outputs[key]["batch_ms"])
                    for key in order
                }
                candidate_parent_error = error_metrics(
                    [Path(p) for p in outputs["parent"]["outputs"]],
                    [Path(p) for p in outputs["candidate"]["outputs"]],
                )
                candidate_ducc_error = error_metrics(
                    [Path(p) for p in outputs["ducc"]["outputs"]],
                    [Path(p) for p in outputs["candidate"]["outputs"]],
                )
                case_pairs.append({
                    "pair": pair_index,
                    "order": order,
                    "median_ms": medians,
                    "candidate_parent_ratio": medians["candidate"] / medians["parent"],
                    "candidate_ducc_ratio": medians["candidate"] / medians["ducc"],
                    "parent_ducc_ratio": medians["parent"] / medians["ducc"],
                    "candidate_parent_error": candidate_parent_error,
                    "candidate_ducc_error": candidate_ducc_error,
                    "simd": outputs["candidate"].get("simd_arch"),
                    "f32_lanes": outputs["candidate"].get("f32_lanes"),
                    "reset_consistent": outputs["candidate"].get("reset_consistent"),
                    "repeat_consistent": outputs["candidate"].get("repeat_consistent"),
                })
                for worker in outputs.values():
                    for path in worker.get("outputs", []):
                        Path(path).unlink(missing_ok=True)
            results.append({
                **case,
                "case_id": case_id,
                "fixture_sha256": fixture_hashes,
                "pairs": case_pairs,
                "median_candidate_parent_ratio": statistics.median(
                    p["candidate_parent_ratio"] for p in case_pairs
                ),
                "median_candidate_ducc_ratio": statistics.median(
                    p["candidate_ducc_ratio"] for p in case_pairs
                ),
                "paired_candidate_parent": paired_ratio_summary(
                    [p["candidate_parent_ratio"] for p in case_pairs]
                ),
                "paired_candidate_ducc": paired_ratio_summary(
                    [p["candidate_ducc_ratio"] for p in case_pairs]
                ),
                "paired_parent_ducc": paired_ratio_summary(
                    [p["parent_ducc_ratio"] for p in case_pairs]
                ),
            })
            print(f"completed {case_id}", flush=True)
    patch = subprocess.check_output(["git", "-C", str(ROOT), "diff", "--binary"])
    data = {
        "purpose": "three-pair native-f32 B1 precision-specific tile screen",
        "parent_source_sha": subprocess.check_output(
            ["git", "-C", str(ROOT), "rev-parse", "HEAD"], text=True
        ).strip(),
        "candidate_patch_sha256": hashlib.sha256(patch).hexdigest(),
        "ducc_source_sha": DUCC_SHA,
        "binary_sha256": {
            "parent": sha(parent), "candidate": sha(candidate), "ducc_f32": sha(ducc),
        },
        "rustc_vv": subprocess.check_output(["rustc", "-Vv"], text=True),
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "cpu_model": platform.processor() or platform.machine(),
        "affinity": {
            "q1": "one physical core (logical id local only)",
            "q8": "eight distinct physical cores (logical ids local only)",
        },
        "target_ms": target_ms,
        "process_pairs": pair_count,
        "input_precision": "deterministic f32 map and harmonic fixtures",
        "results": results,
    }
    out.mkdir(parents=True, exist_ok=True)
    (out / "screen3.json").write_text(json.dumps(data, indent=2) + "\n")
    print(out / "screen3.json")


if __name__ == "__main__":
    main()
