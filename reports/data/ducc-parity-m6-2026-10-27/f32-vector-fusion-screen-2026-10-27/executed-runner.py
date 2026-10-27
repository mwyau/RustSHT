from __future__ import annotations

import hashlib
import json
import os
import statistics
import sys
from pathlib import Path

REPO = Path(os.environ["RUSTSHT_REPO"]).resolve()
ROOT = Path(os.environ["RUSTSHT_M6_SCRATCH"]).resolve()
sys.path.insert(0, str(REPO / "scripts"))
from benchmark_ducc_f32_native import error_metrics, make_fixture, run_worker, smooth_even_at_least

BINARIES = {
    "parent": ROOT / "targets/parent/release/examples/ducc_f32_full_transform_parity",
    "candidate": ROOT / "targets/candidate/release/examples/ducc_f32_full_transform_parity",
    "ducc": ROOT / "ducc-build/ducc_f32_full_transform_parity",
}
OUT = ROOT / "results/triage"
OUT.mkdir(parents=True, exist_ok=True)
cases = []
for mmax in (720, 180):
    for threads in (1, 8):
        cases.append({
            "grid": "GL", "ntheta": 721,
            "nphi": smooth_even_at_least(2 * mmax + 2),
            "lmax": 720, "mmax": mmax, "longitude": "even",
            "spin": 0, "operation": "synthesis", "threads": threads,
        })
orders = (
    ("parent", "candidate", "ducc"),
    ("candidate", "ducc", "parent"),
    ("ducc", "parent", "candidate"),
)
rows = []
for case in cases:
    case_id = f"GL-T720-M{case['mmax']}-even-s0-synthesis-q{case['threads']}"
    fixture = make_fixture(case, ROOT / "results/fixtures" / case_id)
    fixture_hashes = {name: hashlib.sha256(path.read_bytes()).hexdigest()
                      for name, path in fixture.items()}
    cpus = "0" if case["threads"] == 1 else "0,1,2,3,4,5,6,7"
    for triplet, order in enumerate(orders, 1):
        workers = {}
        for backend in order:
            run_dir = OUT / case_id / f"triplet-{triplet}" / backend
            result = run_worker(
                BINARIES[backend], backend, case, fixture, run_dir, 10.0, cpus
            )
            if not result.get("finite_outputs", False):
                raise RuntimeError(f"non-finite output: {case_id} triplet {triplet} {backend}")
            if not result.get("repeat_consistent", False):
                raise RuntimeError(f"repeat check failed: {case_id} triplet {triplet} {backend}")
            if not result.get("reset_consistent", False):
                raise RuntimeError(f"reset check failed: {case_id} triplet {triplet} {backend}")
            result["output_sha256"] = {
                Path(path).name: hashlib.sha256(Path(path).read_bytes()).hexdigest()
                for path in result["outputs"]
            }
            workers[backend] = result

        parent_paths = [Path(path) for path in workers["parent"]["outputs"]]
        candidate_paths = [Path(path) for path in workers["candidate"]["outputs"]]
        ducc_paths = [Path(path) for path in workers["ducc"]["outputs"]]
        row = {
            **case,
            "case_id": case_id,
            "triplet": triplet,
            "order": list(order),
            "fixture_sha256": fixture_hashes,
            "median_ms": {
                backend: statistics.median(workers[backend]["batch_ms"])
                for backend in BINARIES
            },
            "candidate_over_parent": (
                statistics.median(workers["candidate"]["batch_ms"])
                / statistics.median(workers["parent"]["batch_ms"])
            ),
            "parent_over_ducc": (
                statistics.median(workers["parent"]["batch_ms"])
                / statistics.median(workers["ducc"]["batch_ms"])
            ),
            "candidate_over_ducc": (
                statistics.median(workers["candidate"]["batch_ms"])
                / statistics.median(workers["ducc"]["batch_ms"])
            ),
            "candidate_parent_bitwise_equal": all(
                a.read_bytes() == b.read_bytes()
                for a, b in zip(parent_paths, candidate_paths)
            ),
            "error_candidate_vs_parent": error_metrics(parent_paths, candidate_paths),
            "error_parent_vs_ducc": error_metrics(ducc_paths, parent_paths),
            "error_candidate_vs_ducc": error_metrics(ducc_paths, candidate_paths),
            "workers": workers,
        }
        rows.append(row)
        (OUT / "triplets.json").write_text(json.dumps(rows, indent=2) + "\n")
        print(json.dumps({
            "case": case_id,
            "triplet": triplet,
            "order": order,
            "parent_ms": row["median_ms"]["parent"],
            "candidate_ms": row["median_ms"]["candidate"],
            "ducc_ms": row["median_ms"]["ducc"],
            "candidate_over_parent": row["candidate_over_parent"],
            "candidate_parent_bitwise_equal": row["candidate_parent_bitwise_equal"],
            "candidate_ducc_relative_l2": row["error_candidate_vs_ducc"]["relative_l2"],
        }), flush=True)

print(f"completed {len(rows)} triplets across {len(cases)} cells")
