"""Matched parent/candidate/pinned-DUCC GL T720 spin-1 analysis screen."""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import statistics
import subprocess
import sys

import numpy as np

REPO = pathlib.Path(os.environ.get(
    "RUSTSHT_REPO", pathlib.Path(__file__).resolve().parents[3]
))
SCRATCH = pathlib.Path(os.environ["RUSTSHT_M6_SCRATCH"])
sys.path.insert(0, str(REPO / "scripts"))
import benchmark_ducc_full_transform_parity as common  # noqa: E402
import benchmark_ducc_spin1_native as bench  # noqa: E402


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def complex_pair(paths: dict[str, pathlib.Path]) -> np.ndarray:
    return np.concatenate((bench.output_values(paths["e"], "complex"),
                           bench.output_values(paths["b"], "complex")))


def bitwise_equal(left: np.ndarray, right: np.ndarray) -> bool:
    return left.shape == right.shape and np.array_equal(
        left.view(np.uint64), right.view(np.uint64)
    )


def summarize_diagnostics(diagnostics: dict) -> dict:
    frequencies = diagnostics["frequency_ghz"]
    return {
        "wait_fraction": diagnostics["wait_fraction"],
        "involuntary_switches": diagnostics["involuntary_switches"],
        "frequency_samples": len(frequencies),
        "frequency_ghz_median": float(np.median(frequencies)) if frequencies else None,
        "frequency_ghz_p10": float(np.quantile(frequencies, 0.1)) if frequencies else None,
        "frequency_ghz_p90": float(np.quantile(frequencies, 0.9)) if frequencies else None,
    }


def main() -> int:
    binaries = {
        "parent": pathlib.Path(os.environ["M6_PARENT_BIN"]),
        "candidate": pathlib.Path(os.environ["M6_CANDIDATE_BIN"]),
        "ducc": pathlib.Path(os.environ["M6_DUCC_BIN"]),
    }
    for name, binary in binaries.items():
        if not binary.is_file():
            raise FileNotFoundError(f"missing {name} binary: {binary}")

    case = {
        "case_id": "GL-T720-m720-n721-exact-p1458-even-smooth",
        "grid": "GL",
        "lmax": 720,
        "mmax": 720,
        "ntheta": 721,
        "nphi": 1458,
        "latitude_mode": "exact",
        "longitude_mode": "even-smooth",
    }
    fixture_dir = SCRATCH / "fixtures"
    fixture_dir.mkdir(parents=True, exist_ok=True)
    fixture = bench.make_fixture(case, fixture_dir)
    physical = common.physical_cpus()
    monitor_cpu = physical[8] if len(physical) > 8 else physical[0]
    base_orders = [
        ["parent", "candidate", "ducc"],
        ["candidate", "ducc", "parent"],
        ["ducc", "parent", "candidate"],
    ]
    process_orders = base_orders * 5
    workers: list[dict] = []
    triplets: list[dict] = []
    results_dir = SCRATCH / "results"

    for threads in (1, 8):
        cpus = physical[:threads]
        for pair_index, order in enumerate(process_orders):
            records: dict[str, dict] = {}
            output_paths: dict[str, dict[str, pathlib.Path]] = {}
            for position, backend in enumerate(order):
                paths = {
                    name: results_dir / f"{backend}-q{threads}-pair{pair_index}-{name}.bin"
                    for name in ("map0", "map1", "e", "b")
                }
                command = bench.worker_command(
                    binaries[backend], case, fixture, "analysis", threads,
                    paths, 25.0,
                )
                record, diagnostics = common.run_pinned(command, cpus, monitor_cpu)
                record.update({
                    "backend": backend,
                    "threads": threads,
                    "process_pair": pair_index,
                    "position": position,
                    "fixture_sha256": fixture["sha256"],
                    "output_sha256": {key: sha256(paths[key]) for key in ("e", "b")},
                    "host_diagnostics": summarize_diagnostics(diagnostics),
                })
                records[backend] = record
                output_paths[backend] = paths
                workers.append(record)

            parent = complex_pair(output_paths["parent"])
            candidate = complex_pair(output_paths["candidate"])
            ducc = complex_pair(output_paths["ducc"])
            timing_medians = {
                key: float(statistics.median(records[key]["batch_ms"]))
                for key in ("parent", "candidate", "ducc")
            }
            triplets.append({
                "threads": threads,
                "process_pair": pair_index,
                "process_order": order,
                "candidate_parent_bitwise_equal": bitwise_equal(parent, candidate),
                "finite_outputs": bool(
                    np.isfinite(parent).all()
                    and np.isfinite(candidate).all()
                    and np.isfinite(ducc).all()
                ),
                "repeat_consistent": {
                    key: bool(records[key]["repeat_consistent"])
                    for key in records
                },
                "reset_consistent": {
                    key: bool(records[key]["reset_consistent"])
                    for key in records
                },
                "timing_median_ms": timing_medians,
                "candidate_parent_ratio": timing_medians["candidate"] / timing_medians["parent"],
                "parent_ducc_ratio": timing_medians["parent"] / timing_medians["ducc"],
                "candidate_ducc_ratio": timing_medians["candidate"] / timing_medians["ducc"],
                "parent_vs_ducc_error": common.error_metrics(parent, ducc),
                "candidate_vs_ducc_error": common.error_metrics(candidate, ducc),
                "batch_samples_ms": {
                    key: records[key]["batch_ms"] for key in records
                },
                "host_diagnostics": {
                    key: records[key]["host_diagnostics"] for key in records
                },
                "output_sha256": {
                    key: records[key]["output_sha256"] for key in records
                },
            })
            print(
                f"DONE q={threads} pair={pair_index} order={','.join(order)} "
                f"C/P={triplets[-1]['candidate_parent_ratio']:.6f} "
                f"P/D={triplets[-1]['parent_ducc_ratio']:.6f} "
                f"bitwise={triplets[-1]['candidate_parent_bitwise_equal']}",
                flush=True,
            )

    metadata = {
        "parent_source_sha": subprocess.check_output(
            ["git", "-C", str(SCRATCH / "parent"), "rev-parse", "HEAD"], text=True
        ).strip(),
        "candidate_base_source_sha": subprocess.check_output(
            ["git", "-C", str(REPO), "rev-parse", "HEAD"], text=True
        ).strip(),
        "candidate_patch_sha256": sha256(SCRATCH / "indexed-loop.patch"),
        "candidate_source_file_sha256": sha256(REPO / "src/legendre/scalar_paired/analysis.rs"),
        "parent_binary_sha256": sha256(binaries["parent"]),
        "candidate_binary_sha256": sha256(binaries["candidate"]),
        "ducc_binary_sha256": sha256(binaries["ducc"]),
        "ducc_revision": subprocess.check_output(
            ["git", "-C", str(SCRATCH / "ducc-source"), "rev-parse", "HEAD"], text=True
        ).strip(),
        "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
        "gxx": subprocess.check_output(["g++", "--version"], text=True).splitlines()[0],
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "ducc_flags": "SHT -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver overrides with -fno-fast-math -ffp-contract=fast",
        "cpu_model": common.cpu_model(),
        "physical_cpus_q1": physical[:1],
        "physical_cpus_q8": physical[:8],
        "monitor_cpu": monitor_cpu,
        "case": case,
        "operation": "spin-1 analysis",
        "phi0": 0.23,
        "target_batch_ms": 25.0,
        "triplets_per_thread_count": 15,
        "fixture_seed": fixture["seed"],
        "fixture_sha256": fixture["sha256"],
    }
    result = {"metadata": metadata, "workers": workers, "triplets": triplets}
    path = results_dir / "triplets.json"
    path.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
    print(f"WROTE {path}", flush=True)
    return 0 if all(
        item["candidate_parent_bitwise_equal"]
        and item["finite_outputs"]
        and all(item["repeat_consistent"].values())
        and all(item["reset_consistent"].values())
        for item in triplets
    ) else 2


if __name__ == "__main__":
    raise SystemExit(main())
