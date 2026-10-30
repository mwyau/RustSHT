#!/usr/bin/env python3
"""Three-backend screen for the isolated scalar-analysis scale leaf."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import pathlib
import statistics
import subprocess
import tempfile
import time


def find_root() -> pathlib.Path:
    for parent in pathlib.Path(__file__).resolve().parents:
        if (parent / "Cargo.toml").is_file() and (
            parent / "scripts/benchmark_ducc_f32_native.py"
        ).is_file():
            return parent
    raise RuntimeError("RustSHT root not found")


ROOT = find_root()
SPEC = importlib.util.spec_from_file_location(
    "native_bench", ROOT / "scripts/benchmark_ducc_f32_native.py"
)
BENCH = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(BENCH)

PARENT = pathlib.Path(os.environ["B1_PARENT_RUST"])
CANDIDATE = pathlib.Path(os.environ["B1_CANDIDATE_RUST"])
F64 = pathlib.Path(os.environ["B1_F64_RUST"])
DUCC = pathlib.Path(os.environ["B1_DUCC"])
OUT = pathlib.Path(os.environ["B1_OUT"])
TARGET_MS = 30.0
PAIRS = 3
CASES = []
for mmax in (720, 180):
    nphi = 1458 if mmax == 720 else 378
    for threads in (1, 8):
        CASES.append(
            {
                "grid": "GL",
                "ntheta": 721,
                "nphi": nphi,
                "lmax": 720,
                "mmax": mmax,
                "longitude": "even",
                "spin": 0,
                "operation": "analysis",
                "threads": threads,
            }
        )


def sha(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run_f64_worker(case: dict, fixture: dict[str, pathlib.Path], run_dir: pathlib.Path) -> dict:
    run_dir.mkdir(parents=True, exist_ok=True)
    args = [
        str(F64),
        "--worker",
        case["operation"],
        str(case["spin"]),
        str(case["threads"]),
        str(case["ntheta"]),
        str(case["nphi"]),
        str(case["lmax"]),
        str(case["mmax"]),
        repr(BENCH.PHI0),
        str(fixture["map0"]),
        str(fixture["map1"]),
        str(fixture["alm0"]),
        str(fixture["alm1"]),
        repr(TARGET_MS),
    ]
    cpus = "0" if case["threads"] == 1 else "0,1,2,3,4,5,6,7"
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = str(case["threads"])
    completed = subprocess.run(
        ["taskset", "-c", cpus, *args], text=True, capture_output=True, env=env
    )
    if completed.returncode != 0:
        raise RuntimeError(f"f64 worker failed for {case}: {completed.stderr}\n{completed.stdout}")
    return json.loads(completed.stdout.strip().splitlines()[-1])


def main() -> None:
    for binary in (PARENT, CANDIDATE, F64, DUCC):
        if not binary.is_file():
            raise RuntimeError(f"worker not found: {binary}")
    ducc_sha = sha(DUCC)
    if ducc_sha != "1e8412966f7bf306a1fea8141969208f661d5a705f7737e7a8f0dc1cc74cc36d":
        raise RuntimeError(f"unexpected pinned DUCC worker hash: {ducc_sha}")

    rows = []
    started = time.time()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(
        prefix="b1-scale-fast-", dir=OUT.parent
    ) as scratch_text:
        scratch = pathlib.Path(scratch_text)
        for case in CASES:
            case_id = f"GL-T720-M{case['mmax']}-s0-analysis-q{case['threads']}"
            fixture = BENCH.make_fixture(case, scratch / case_id / "fixture")
            case_pairs = []
            for pair in range(1, PAIRS + 1):
                order = (
                    ("parent", "candidate", "ducc", "f64"),
                    ("ducc", "f64", "parent", "candidate"),
                    ("candidate", "f64", "ducc", "parent"),
                )[pair - 1]
                binaries = {"parent": PARENT, "candidate": CANDIDATE, "ducc": DUCC}
                results = {}
                for backend in order:
                    if backend == "f64":
                        results[backend] = run_f64_worker(
                            case, fixture, scratch / case_id / f"pair-{pair}" / backend
                        )
                    else:
                        cpus = "0" if case["threads"] == 1 else "0,1,2,3,4,5,6,7"
                        results[backend] = BENCH.run_worker(
                            binaries[backend],
                            backend,
                            case,
                            fixture,
                            scratch / case_id / f"pair-{pair}" / backend,
                            TARGET_MS,
                            cpus,
                        )

                parent_outputs = [pathlib.Path(path) for path in results["parent"]["outputs"]]
                candidate_outputs = [
                    pathlib.Path(path) for path in results["candidate"]["outputs"]
                ]
                ducc_outputs = [pathlib.Path(path) for path in results["ducc"]["outputs"]]
                parent_candidate_bytes_equal = all(
                    left.read_bytes() == right.read_bytes()
                    for left, right in zip(parent_outputs, candidate_outputs)
                )
                rust_ducc_error = BENCH.error_metrics(ducc_outputs, candidate_outputs)
                parent_ms = statistics.median(results["parent"]["batch_ms"])
                candidate_ms = statistics.median(results["candidate"]["batch_ms"])
                ducc_ms = statistics.median(results["ducc"]["batch_ms"])
                f64_ms = statistics.median(results["f64"]["batch_ms"])
                case_pairs.append(
                    {
                        "pair": pair,
                        "execution_order": order,
                        "parent_f32_ms": parent_ms,
                        "candidate_f32_ms": candidate_ms,
                        "f64_ms": f64_ms,
                        "ducc_f32_ms": ducc_ms,
                        "candidate_over_parent": candidate_ms / parent_ms,
                        "candidate_over_f64": candidate_ms / f64_ms,
                        "candidate_over_ducc": candidate_ms / ducc_ms,
                        "parent_candidate_bytes_equal": parent_candidate_bytes_equal,
                        "candidate_vs_ducc_error": rust_ducc_error,
                        "workers": {
                            name: (
                                {
                                    key: results[name][key]
                                    for key in (
                                        "first_ms",
                                        "repetitions",
                                        "batch_ms",
                                        "finite",
                                        "stable",
                                        "checksum",
                                        "arch",
                                    )
                                    if key in results[name]
                                }
                                if name == "f64"
                                else BENCH.timing_details(results[name])
                            )
                            for name in ("parent", "candidate", "f64", "ducc")
                        },
                    }
                )

            medians = {
                name: statistics.median(
                    pair[f"{name}_ms"] if name == "f64" else pair[f"{name}_f32_ms"]
                    for pair in case_pairs
                )
                for name in ("parent", "candidate", "f64", "ducc")
            }
            rows.append(
                {
                    **case,
                    "case_id": case_id,
                    "f64_baseline_ms": medians["f64"],
                    "fixture_sha256": {name: sha(path) for name, path in fixture.items()},
                    "pair_medians_ms": medians,
                    "candidate_over_parent": medians["candidate"] / medians["parent"],
                    "candidate_over_f64": statistics.median(
                        pair["candidate_over_f64"] for pair in case_pairs
                    ),
                    "candidate_over_ducc": medians["candidate"] / medians["ducc"],
                    "pairs": case_pairs,
                }
            )
            print(
                f"DONE {case_id} parent={medians['parent']:.6f}ms "
                f"candidate={medians['candidate']:.6f}ms ducc={medians['ducc']:.6f}ms",
                flush=True,
            )

    result = {
        "purpose": "screen exact-normal-power scaling in native-f32 scalar analysis",
        "parent_source": "a53e17877440de962567fc21a5226f786b721d1e",
        "parent_patch_sha256": os.environ.get("B1_PARENT_PATCH_SHA256", "none"),
        "candidate_source_base": "a53e17877440de962567fc21a5226f786b721d1e",
        "candidate_patch_sha256": os.environ.get("B1_PATCH_SHA256", "not-recorded"),
        "runner_sha256": sha(pathlib.Path(__file__)),
        "parent_binary_sha256": sha(PARENT),
        "candidate_binary_sha256": sha(CANDIDATE),
        "f64_binary_sha256": sha(F64),
        "ducc_revision": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "ducc_binary_sha256": ducc_sha,
        "rustc_vv": subprocess.check_output(["rustc", "-Vv"], text=True),
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "cpu": subprocess.check_output(["lscpu"], text=True)
        .split("Model name:")[1]
        .splitlines()[0]
        .strip(),
        "q1_affinity": "0",
        "q8_affinity": "0,1,2,3,4,5,6,7",
        "target_ms": TARGET_MS,
        "pairs": PAIRS,
        "f64_baseline_artifact": "reports/archive/experiments/native-f32-scalar-analysis-adaptive-preflight-2026-10-30/fresh-current-source-f64-baseline.json",
        "f64_source_tree_matches_current_dev_source": True,
        "fixture_protocol": "same deterministic native-f32 fixtures for parent/candidate/DUCC; the f64 baseline received component-wise widened versions",
        "elapsed_seconds": time.time() - started,
        "cases": rows,
    }
    OUT.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
    print(f"RESULT {OUT}", flush=True)


if __name__ == "__main__":
    main()
