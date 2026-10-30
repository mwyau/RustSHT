#!/usr/bin/env python3
"""Three-pair native-f32 scalar-analysis screen for the extended-range candidate."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import pathlib
import statistics
import tempfile
import time


def find_repository_root() -> pathlib.Path:
    for parent in pathlib.Path(__file__).resolve().parents:
        if (parent / "Cargo.toml").is_file() and (
            parent / "scripts/benchmark_ducc_f32_native.py"
        ).is_file():
            return parent
    raise RuntimeError("could not locate the RustSHT repository root")


ROOT = find_repository_root()
SPEC = importlib.util.spec_from_file_location(
    "native_bench", ROOT / "scripts/benchmark_ducc_f32_native.py"
)
BENCH = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(BENCH)

RUST = pathlib.Path(
    os.environ.get(
        "RUSTSHT_B1_EXTENDED_RUST",
        ROOT / "target/release/examples/ducc_f32_full_transform_parity",
    )
)
DUCC_VALUE = os.environ.get("RUSTSHT_B1_EXTENDED_DUCC")
DUCC = pathlib.Path(DUCC_VALUE) if DUCC_VALUE else None
OUT = pathlib.Path(
    os.environ.get(
        "RUSTSHT_B1_EXTENDED_OUT",
        ROOT / "diagnostics/b1-extended-range/t720-3pair.json",
    )
)
PAIRS = 3
TARGET_MS = 30.0
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


def main() -> None:
    if DUCC is None:
        raise RuntimeError("set RUSTSHT_B1_EXTENDED_DUCC to the pinned DUCC worker path")
    if sha(DUCC) != "1e8412966f7bf306a1fea8141969208f661d5a705f7737e7a8f0dc1cc74cc36d":
        raise RuntimeError("DUCC worker binary differs from the recorded pinned worker")
    if not RUST.is_file():
        raise RuntimeError(f"Rust worker not found: {RUST}")

    rows = []
    started = time.time()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="b1-extended-range-") as scratch:
        scratch = pathlib.Path(scratch)
        for case in CASES:
            case_id = (
                f"GL-T720-M{case['mmax']}-s0-analysis-q{case['threads']}"
            )
            fixture = BENCH.make_fixture(case, scratch / case_id / "fixture")
            pairs = []
            for pair in range(1, PAIRS + 1):
                order = ("rust", "ducc") if pair % 2 else ("ducc", "rust")
                results = {}
                for backend in order:
                    binary = RUST if backend == "rust" else DUCC
                    cpus = "0" if case["threads"] == 1 else "0,1,2,3,4,5,6,7"
                    results[backend] = BENCH.run_worker(
                        binary,
                        backend,
                        case,
                        fixture,
                        scratch / case_id / f"pair-{pair}" / backend,
                        TARGET_MS,
                        cpus,
                    )
                reference = [pathlib.Path(p) for p in results["ducc"]["outputs"]]
                candidate = [pathlib.Path(p) for p in results["rust"]["outputs"]]
                error = BENCH.error_metrics(reference, candidate)
                rust_ms = statistics.median(results["rust"]["batch_ms"])
                ducc_ms = statistics.median(results["ducc"]["batch_ms"])
                pairs.append(
                    {
                        "pair": pair,
                        "execution_order": order,
                        "rust_f32_ms": rust_ms,
                        "ducc_f32_ms": ducc_ms,
                        "f32_over_ducc": rust_ms / ducc_ms,
                        "error_vs_ducc": error,
                        "rust": BENCH.timing_details(results["rust"]),
                        "ducc": BENCH.timing_details(results["ducc"]),
                    }
                )
            rust_values = [item["rust_f32_ms"] for item in pairs]
            ducc_values = [item["ducc_f32_ms"] for item in pairs]
            rows.append(
                {
                    **case,
                    "case_id": case_id,
                    "fixture_sha256": {
                        name: sha(path) for name, path in fixture.items()
                    },
                    "rust_f32_median_ms": statistics.median(rust_values),
                    "ducc_f32_median_ms": statistics.median(ducc_values),
                    "f32_over_ducc_median": statistics.median(
                        item["f32_over_ducc"] for item in pairs
                    ),
                    "pair_details": pairs,
                }
            )
            print(
                f"DONE {case_id} rust={statistics.median(rust_values):.6f}ms "
                f"ducc={statistics.median(ducc_values):.6f}ms",
                flush=True,
            )

    result = {
        "purpose": "screen isolated extended normal-range native-f32 scalar-analysis candidate",
        "source_parent": "f122428c24709c53233d0922deeffd6b46dce048",
        "rust_binary_sha256": sha(RUST),
        "ducc_revision": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "ducc_binary_sha256": sha(DUCC),
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "cpu": "AMD Ryzen 9 5950X 16-Core Processor",
        "q1_affinity": "0",
        "q8_affinity": "0,1,2,3,4,5,6,7",
        "target_ms": TARGET_MS,
        "process_pairs": PAIRS,
        "fixture_protocol": "same f32 fixtures as prior matched B1 baseline, widened only for f64 baseline worker",
        "elapsed_seconds": time.time() - started,
        "cases": rows,
    }
    OUT.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
    print(f"RESULT {OUT}", flush=True)


if __name__ == "__main__":
    main()
