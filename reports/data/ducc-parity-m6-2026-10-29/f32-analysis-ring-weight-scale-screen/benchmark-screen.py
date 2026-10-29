#!/usr/bin/env python3
"""Interleaved public f32 scalar-analysis comparison for M6."""

import hashlib
import importlib.util
import json
import pathlib
import random
import statistics
import sys

if len(sys.argv) < 3:
    raise SystemExit("usage: benchmark-analysis.py EXPERIMENT_ROOT REPOSITORY_ROOT [PAIRS]")

ROOT = pathlib.Path(sys.argv[1]).resolve()
REPO = pathlib.Path(sys.argv[2]).resolve()
PARENT = ROOT / "target-parent/release/examples/ducc_f32_full_transform_parity"
CANDIDATE = ROOT / "target-candidate/release/examples/ducc_f32_full_transform_parity"
DUCC = ROOT / "ducc-build-pinned/ducc_f32_full_transform_parity"
OUT = ROOT / "results-15"
SCRATCH = ROOT / "runs-15"
sys.path.insert(0, str(REPO / "scripts"))
spec = importlib.util.spec_from_file_location("bench", REPO / "scripts/benchmark_ducc_f32_native.py")
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)

ORDERS = [
    ("parent", "candidate", "ducc"),
    ("candidate", "ducc", "parent"),
    ("ducc", "parent", "candidate"),
    ("ducc", "candidate", "parent"),
    ("candidate", "parent", "ducc"),
    ("parent", "ducc", "candidate"),
]


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def paired_ci(values, seed):
    rng = random.Random(seed)
    medians = []
    for _ in range(20000):
        sample = [rng.choice(values) for _ in values]
        medians.append(statistics.median(sample))
    medians.sort()
    return [medians[500], medians[19499]]


def main():
    pairs = int(sys.argv[3]) if len(sys.argv) > 3 else 3
    OUT.mkdir(parents=True, exist_ok=True)
    all_cells = []
    for threads in (1, 8):
        for mmax, nphi in ((720, 1458), (180, 378)):
            case = {
                "grid": "GL", "ntheta": 721, "nphi": nphi, "lmax": 720,
                "mmax": mmax, "longitude": "even", "spin": 0,
                "operation": "analysis", "threads": threads,
            }
            case_id = f"GL-T720-M{mmax}-s0-analysis-q{threads}"
            fixture = bench.make_fixture(case, SCRATCH / "fixtures" / case_id)
            fixture_hashes = {name: sha256(path) for name, path in fixture.items()}
            case_out = OUT / case_id
            case_out.mkdir(parents=True, exist_ok=True)
            paired_rows = []
            for pair in range(pairs):
                order = ORDERS[pair % len(ORDERS)]
                outputs = {}
                for backend in order:
                    binary = {"parent": PARENT, "candidate": CANDIDATE, "ducc": DUCC}[backend]
                    cpus = "0" if threads == 1 else "0,2,4,6,8,10,12,14"
                    result = bench.run_worker(
                        binary, backend, case, fixture,
                        SCRATCH / case_id / f"pair-{pair + 1}" / backend,
                        25.0, cpus,
                    )
                    outputs[backend] = result
                paths = {
                    backend: [pathlib.Path(path) for path in result["outputs"]]
                    for backend, result in outputs.items()
                }
                errors = {
                    "candidate_vs_parent": bench.error_metrics(paths["parent"], paths["candidate"]),
                    "parent_vs_ducc": bench.error_metrics(paths["ducc"], paths["parent"]),
                    "candidate_vs_ducc": bench.error_metrics(paths["ducc"], paths["candidate"]),
                }
                parent_ms = statistics.median(outputs["parent"]["batch_ms"])
                candidate_ms = statistics.median(outputs["candidate"]["batch_ms"])
                ducc_ms = statistics.median(outputs["ducc"]["batch_ms"])
                row = {
                    "pair": pair + 1,
                    "order": order,
                    "parent_ms": parent_ms,
                    "candidate_ms": candidate_ms,
                    "ducc_ms": ducc_ms,
                    "candidate_parent": candidate_ms / parent_ms,
                    "candidate_ducc": candidate_ms / ducc_ms,
                    "parent_ducc": parent_ms / ducc_ms,
                    "errors": errors,
                    "workers": {
                        backend: {key: value for key, value in result.items() if key != "outputs"}
                        for backend, result in outputs.items()
                    },
                }
                for backend_paths in paths.values():
                    for path in backend_paths:
                        path.unlink(missing_ok=True)
                paired_rows.append(row)
                print(f"completed {case_id} pair {pair + 1}", flush=True)
                (case_out / f"pair-{pair + 1:02}.json").write_text(json.dumps(row, indent=2) + "\n")

            cp = [row["candidate_parent"] for row in paired_rows]
            cd = [row["candidate_ducc"] for row in paired_rows]
            cell = {
                "case": case,
                "case_id": case_id,
                "pairs": paired_rows,
                "fixture_sha256": fixture_hashes,
                "parent_median_ms": statistics.median(row["parent_ms"] for row in paired_rows),
                "candidate_median_ms": statistics.median(row["candidate_ms"] for row in paired_rows),
                "ducc_median_ms": statistics.median(row["ducc_ms"] for row in paired_rows),
                "candidate_parent_median": statistics.median(cp),
                "candidate_parent_bootstrap_95": paired_ci(cp, 0xFA32 + threads + mmax),
                "candidate_ducc_median": statistics.median(cd),
                "candidate_ducc_bootstrap_95": paired_ci(cd, 0xFA320 + threads + mmax),
            }
            all_cells.append(cell)
            (case_out / "summary.json").write_text(json.dumps(cell, indent=2) + "\n")
            print(
                f"{case_id}: parent/candidate/DUCC="
                f"{cell['parent_median_ms']:.3f}/{cell['candidate_median_ms']:.3f}/"
                f"{cell['ducc_median_ms']:.3f} ms, C/D={cell['candidate_ducc_median']:.4f}",
                flush=True,
            )
    manifest = {
        "parent_sha": "1f90f71f7be0effa04455076b3243f92444c3bf5",
        "candidate_parent_sha": "1f90f71f7be0effa04455076b3243f92444c3bf5",
        "ducc_sha": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "ducc_version": "0.42.0",
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "ducc_flags": "-O3 -ffast-math -march=x86-64-v3 -ffp-contract=fast",
        "warm_target_ms": 25.0,
        "paired_comparisons_per_cell": pairs,
        "candidate_patch_sha256": sha256(ROOT / "candidate.patch"),
        "binaries_sha256": {name: sha256(path) for name, path in (
            ("parent", PARENT), ("candidate", CANDIDATE), ("ducc", DUCC),
        )},
        "cells": all_cells,
    }
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
