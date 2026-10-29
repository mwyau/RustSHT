#!/usr/bin/env python3
"""Matched native-f32 versus reusable f32-I/O/f64-internal scalar synthesis."""

import hashlib
import importlib.util
import json
import os
import pathlib
import random
import statistics
import sys

ROOT = pathlib.Path(os.environ["M6_EXPERIMENT_ROOT"])
REPO = pathlib.Path(os.environ["M6_REPO_ROOT"])
PARENT = ROOT / "bin/ducc_f32_native"
CANDIDATE = pathlib.Path(os.environ.get(
    "M6_INTERNAL_BINARY", str(ROOT / "bin/ducc_f32_internal_f64")
))
DUCC = ROOT / "bin/ducc_f32_pinned"
OUT = ROOT / os.environ.get("M6_RESULTS_DIR", "results-scalar-synthesis-15")
SCRATCH = ROOT / "runs-scalar-synthesis-15"
sys.path.insert(0, str(REPO / "scripts"))
spec = importlib.util.spec_from_file_location(
    "bench", REPO / "scripts/benchmark_ducc_f32_native.py"
)
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def worker_record(result):
    fields = (
        "backend",
        "first_ms",
        "warmup_ms",
        "setup_ms",
        "pool_setup_ms",
        "fixture_load_ms",
        "repetitions_per_batch",
        "batch_ms",
        "batch_total_ms",
        "process_elapsed_s",
        "output_checksum",
        "finite_outputs",
        "repeat_consistent",
        "reset_consistent",
        "simd_arch",
        "f32_lanes",
        "f64_lanes",
        "internal_precision",
        "reusable_conversion_buffer_bytes",
        "telemetry",
    )
    return {key: result[key] for key in fields if key in result}


def bootstrap_median_ci(values, seed):
    rng = random.Random(seed)
    samples = []
    for _ in range(20000):
        samples.append(statistics.median([values[rng.randrange(len(values))] for _ in values]))
    return [bench.percentile(samples, 0.025), bench.percentile(samples, 0.975)]


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    SCRATCH.mkdir(parents=True, exist_ok=True)
    pairs_per_cell = int(os.environ.get("M6_PAIRS", "15"))
    rows = []
    rng = random.Random(0xF64A720)
    selected = set(os.environ.get("M6_CASES", "").split(","))
    process_orders = (
        ("parent", "internal_f64", "ducc"),
        ("ducc", "parent", "internal_f64"),
        ("internal_f64", "ducc", "parent"),
    )
    binaries = {"parent": PARENT, "internal_f64": CANDIDATE, "ducc": DUCC}

    for threads in (1, 8):
        cpus = "0" if threads == 1 else "0,2,4,6,8,10,12,14"
        for mmax, nphi in ((720, 1458), (180, 378)):
            if selected and selected != {""} and f"GL-T720-M{mmax}-s0-synthesis-q{threads}" not in selected:
                continue
            case = {
                "grid": "GL",
                "ntheta": 721,
                "nphi": nphi,
                "lmax": 720,
                "mmax": mmax,
                "longitude": "even",
                "spin": 0,
                "operation": "synthesis",
                "threads": threads,
            }
            case_id = f"GL-T720-M{mmax}-s0-synthesis-q{threads}"
            fixture_dir = SCRATCH / "fixtures" / case_id
            fixture = bench.make_fixture(case, fixture_dir)
            pairs = []
            for pair in range(1, pairs_per_cell + 1):
                order = list(process_orders[(pair - 1) % len(process_orders)])
                if rng.random() < 0.5:
                    order.reverse()
                workers = {}
                for backend in order:
                    workers[backend] = bench.run_worker(
                        binaries[backend],
                        backend,
                        case,
                        fixture,
                        SCRATCH / case_id / f"pair-{pair:02}" / backend,
                        25.0,
                        cpus,
                    )

                times = {
                    name: statistics.median(workers[name]["batch_ms"])
                    for name in binaries
                }
                paths = {
                    name: [pathlib.Path(path) for path in workers[name]["outputs"]]
                    for name in binaries
                }
                errors = {
                    "internal_vs_parent": bench.error_metrics(paths["parent"], paths["internal_f64"]),
                    "internal_vs_ducc": bench.error_metrics(paths["ducc"], paths["internal_f64"]),
                    "parent_vs_ducc": bench.error_metrics(paths["ducc"], paths["parent"]),
                }
                pair_row = {
                    "pair": pair,
                    "order": order,
                    "times_ms": times,
                    "internal_parent_ratio": times["internal_f64"] / times["parent"],
                    "internal_ducc_ratio": times["internal_f64"] / times["ducc"],
                    "parent_ducc_ratio": times["parent"] / times["ducc"],
                    "errors": errors,
                    "workers": {name: worker_record(workers[name]) for name in binaries},
                }
                for group in paths.values():
                    for path in group:
                        path.unlink(missing_ok=True)
                pairs.append(pair_row)
                (OUT / f"{case_id}-pair-{pair:02}.json").write_text(
                    json.dumps(pair_row, indent=2) + "\n"
                )
                print(f"completed {case_id} pair {pair}", flush=True)

            internal_parent = [row["internal_parent_ratio"] for row in pairs]
            internal_ducc = [row["internal_ducc_ratio"] for row in pairs]
            summary = {
                "case_id": case_id,
                "case": case,
                "pairs": pairs_per_cell,
                "parent_median_ms": statistics.median(row["times_ms"]["parent"] for row in pairs),
                "internal_f64_median_ms": statistics.median(row["times_ms"]["internal_f64"] for row in pairs),
                "ducc_median_ms": statistics.median(row["times_ms"]["ducc"] for row in pairs),
                "internal_parent_paired_median": statistics.median(internal_parent),
                "internal_parent_bootstrap_95": bootstrap_median_ci(
                    internal_parent, 0xF64A + threads + mmax
                ),
                "internal_parent_faster": sum(value < 1.0 for value in internal_parent),
                "internal_ducc_paired_median": statistics.median(internal_ducc),
                "internal_ducc_bootstrap_95": bootstrap_median_ci(
                    internal_ducc, 0xD0CC + threads + mmax
                ),
                "internal_ducc_faster": sum(value < 1.0 for value in internal_ducc),
                "max_internal_parent_relative_l2": max(
                    row["errors"]["internal_vs_parent"]["relative_l2"] for row in pairs
                ),
                "max_internal_parent_absolute": max(
                    row["errors"]["internal_vs_parent"]["max_absolute"] for row in pairs
                ),
                "max_internal_ducc_relative_l2": max(
                    row["errors"]["internal_vs_ducc"]["relative_l2"] for row in pairs
                ),
                "max_internal_ducc_absolute": max(
                    row["errors"]["internal_vs_ducc"]["max_absolute"] for row in pairs
                ),
                "all_worker_checks": all(
                    all(
                        worker.get(key) is True
                        for key in ("finite_outputs", "repeat_consistent", "reset_consistent")
                    )
                    for row in pairs
                    for worker in row["workers"].values()
                ),
                "fixture_sha256": {name: sha256(path) for name, path in fixture.items()},
                "pairs_data": pairs,
            }
            rows.append(summary)
            (OUT / f"{case_id}-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
            print(json.dumps({key: summary[key] for key in (
                "case_id", "parent_median_ms", "internal_f64_median_ms", "ducc_median_ms",
                "internal_parent_paired_median", "internal_ducc_paired_median", "all_worker_checks"
            )}), flush=True)

    manifest = {
        "parent_source_sha": "1b2041491e621fffe94464667619baf788d31835",
        "candidate_source_parent_sha": "fdae803a01273ac7e4e941936f022fa11e770df5",
        "candidate_example_sha256": sha256(
            pathlib.Path(os.environ.get(
                "M6_INTERNAL_SOURCE", str(ROOT / "source/examples/ducc_f32_io_f64_internal.rs")
            ))
        ),
        "ducc_source_sha": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "ducc_flags": "-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math",
        "q1_affinity": "0",
        "q8_affinity": "0,2,4,6,8,10,12,14",
        "warm_target_ms": 25.0,
        "process_order_balanced": True,
        "pairs_per_cell": pairs_per_cell,
        "fixture": "scripts/benchmark_ducc_f32_native.py make_fixture, phi0=0.23",
        "parent_binary_sha256": sha256(PARENT),
        "internal_f64_binary_sha256": sha256(CANDIDATE),
        "ducc_f32_binary_sha256": sha256(DUCC),
        "results": rows,
    }
    (OUT / "results.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
