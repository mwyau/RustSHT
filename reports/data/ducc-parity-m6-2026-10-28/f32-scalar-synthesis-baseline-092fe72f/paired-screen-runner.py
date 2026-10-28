import importlib.util
import json
import statistics
import hashlib
import os
from pathlib import Path

root = Path(__file__).resolve().parent
build_root = Path(os.environ.get("RUSTSHT_M6_BUILD_ROOT", root))
results_root = Path(os.environ.get("RUSTSHT_M6_RESULTS_DIR", root / "results"))
spec = importlib.util.spec_from_file_location(
    "bench", str(Path(__file__).resolve().parents[5] / "scripts/benchmark_ducc_f32_native.py")
)
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)

bins = {
    "parent": build_root / "cargo-parent/release/examples/ducc_f32_full_transform_parity",
    "candidate": build_root / "cargo-candidate/release/examples/ducc_f32_full_transform_parity",
    "ducc": build_root / "ducc-build/ducc_f32_full_transform_parity",
}
results_root.mkdir(parents=True, exist_ok=True)
cases = []
for mmax in (720, 180):
    nphi = bench.smooth_even_at_least(2 * mmax + 2)
    for threads in (1, 8):
        cases.append({
            "grid": "GL", "ntheta": 721, "nphi": nphi, "lmax": 720,
            "mmax": mmax, "longitude": "even", "spin": 0,
            "operation": "synthesis", "threads": threads,
        })
orders = [
    ("parent", "candidate", "ducc"),
    ("candidate", "ducc", "parent"),
    ("ducc", "parent", "candidate"),
]
rows = []
for case_index, case in enumerate(cases):
    case_id = f"GL-T720-M{case['mmax']}-s0-synthesis-q{case['threads']}"
    fixture = bench.make_fixture(case, results_root / "fixtures" / case_id)
    cpus = "3" if case["threads"] == 1 else "8-15"
    cell = []
    for pair in range(1, 4):
        order = orders[(pair + case_index) % len(orders)]
        outputs = {}
        for backend in order:
            run_dir = results_root / case_id / f"pair-{pair:02d}" / backend
            outputs[backend] = bench.run_worker(
                bins[backend], backend, case, fixture, run_dir, 35.0, cpus
            )
        medians = {
            name: statistics.median(outputs[name]["batch_ms"])
            for name in ("parent", "candidate", "ducc")
        }
        pc_error = bench.error_metrics(
            [Path(path) for path in outputs["parent"]["outputs"]],
            [Path(path) for path in outputs["candidate"]["outputs"]],
        )
        cd_error = bench.error_metrics(
            [Path(path) for path in outputs["ducc"]["outputs"]],
            [Path(path) for path in outputs["candidate"]["outputs"]],
        )
        row = {
            "pair": pair, "order": order, "times_ms": medians,
            "candidate_parent_ratio": medians["candidate"] / medians["parent"],
            "parent_ducc_ratio": medians["parent"] / medians["ducc"],
            "candidate_ducc_ratio": medians["candidate"] / medians["ducc"],
            "parent_candidate_error": pc_error, "candidate_ducc_error": cd_error,
            "checks": {
                name: {key: outputs[name].get(key) for key in (
                    "finite_outputs", "repeat_consistent", "reset_consistent",
                    "simd_arch", "f32_lanes",
                )}
                for name in outputs
            },
            "telemetry": {name: outputs[name].get("telemetry") for name in outputs},
        }
        cell.append(row)
        for output in outputs.values():
            for path in output.get("outputs", []):
                try:
                    Path(path).unlink()
                except FileNotFoundError:
                    pass
        print(json.dumps({"cell": case_id, **row}), flush=True)
    summary = {
        "case_id": case_id, "case": case, "pairs": cell,
        "median_ms": {
            name: statistics.median(row["times_ms"][name] for row in cell)
            for name in ("parent", "candidate", "ducc")
        },
        "candidate_parent_ratio_median": statistics.median(
            row["candidate_parent_ratio"] for row in cell
        ),
        "candidate_ducc_ratio_median": statistics.median(
            row["candidate_ducc_ratio"] for row in cell
        ),
        "parent_ducc_ratio_median": statistics.median(
            row["parent_ducc_ratio"] for row in cell
        ),
        "candidate_parent_wins": sum(row["candidate_parent_ratio"] < 1 for row in cell),
        "max_parent_relative_l2": max(
            row["parent_candidate_error"]["relative_l2"] for row in cell
        ),
        "max_parent_absolute": max(
            row["parent_candidate_error"]["max_absolute"] for row in cell
        ),
        "max_ducc_relative_l2": max(
            row["candidate_ducc_error"]["relative_l2"] for row in cell
        ),
        "max_ducc_absolute": max(
            row["candidate_ducc_error"]["max_absolute"] for row in cell
        ),
        "all_checks": all(
            all(output.get("finite_outputs") and output.get("repeat_consistent")
                and output.get("reset_consistent")
                for output in row["checks"].values())
            for row in cell
        ),
        "fixture_sha256": {
            name: hashlib.sha256(path.read_bytes()).hexdigest()
            for name, path in fixture.items()
        },
    }
    rows.append(summary)
    results_root.mkdir(parents=True, exist_ok=True)
    (results_root / f"{case_id}-summary.json").write_text(
        json.dumps(summary, indent=2) + "\n"
    )
(results_root / "summary.json").write_text(json.dumps({
    "parent_source": "092fe72f322bd482dac73b020785b096772bc901",
    "candidate_change": "permit f32 spin-1 to enter existing separate-channel paired-scalar route",
    "ducc_source": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
    "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
    "ducc_flags": "-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math",
    "host": "AMD Ryzen 9 5950X",
    "process_pairs": 3,
    "cases": rows,
}, indent=2) + "\n")
