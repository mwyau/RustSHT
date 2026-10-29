#!/usr/bin/env python3
"""Compare native-f32 synthesis accuracy for isolated degree/order bands.

This is an accuracy diagnostic, not a timing benchmark. It uses the archived
Plan 17 fixture generator and writes only filtered spectra and one JSON result.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import pathlib
import struct
import tempfile


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=pathlib.Path, required=True)
    parser.add_argument("--candidate-bin", type=pathlib.Path, required=True)
    parser.add_argument("--ducc-bin", type=pathlib.Path, required=True)
    parser.add_argument("--scratch-root", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument(
        "--subset",
        choices=("base", "intersections", "ring-target", "low-order", "all"),
        default="base",
    )
    parser.add_argument("--ringwise", action="store_true")
    args = parser.parse_args()

    helper_path = args.source_root / "scripts/benchmark_ducc_f32_native.py"
    spec = importlib.util.spec_from_file_location("benchmark_ducc_f32_native", helper_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot import fixture helper: {helper_path}")
    bench = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(bench)

    case = {
        "grid": "GL", "ntheta": 721, "nphi": 1458, "lmax": 720,
        "mmax": 720, "longitude": "even", "spin": 0,
        "operation": "synthesis", "threads": 1,
    }
    fixture_dir = args.scratch_root / "fixture"
    fixture = bench.make_fixture(case, fixture_dir)
    original_hashes = {name: sha256(path) for name, path in fixture.items()}
    coefficients = [
        list(struct.unpack(f"<{path.stat().st_size // 4}f", path.read_bytes()))
        for path in (fixture["alm0"], fixture["alm1"])
    ]
    base_groups = {
        "degree_0_239": lambda ell, order: 0 <= ell <= 239,
        "degree_240_479": lambda ell, order: 240 <= ell <= 479,
        "degree_480_599": lambda ell, order: 480 <= ell <= 599,
        "degree_600_719": lambda ell, order: 600 <= ell <= 719,
        "degree_720": lambda ell, order: ell == 720,
        "order_0": lambda ell, order: order == 0,
        "order_1_32": lambda ell, order: 1 <= order <= 32,
        "order_33_180": lambda ell, order: 33 <= order <= 180,
        "order_181_360": lambda ell, order: 181 <= order <= 360,
        "order_361_720": lambda ell, order: 361 <= order <= 720,
    }
    intersection_groups = {
        "order_1_32_degree_480_599": lambda ell, order: 1 <= order <= 32 and 480 <= ell <= 599,
        "order_1_32_degree_600_719": lambda ell, order: 1 <= order <= 32 and 600 <= ell <= 719,
        "order_1_32_degree_720": lambda ell, order: 1 <= order <= 32 and ell == 720,
        "order_33_180_degree_600_719": lambda ell, order: 33 <= order <= 180 and 600 <= ell <= 719,
        "order_181_360_degree_600_719": lambda ell, order: 181 <= order <= 360 and 600 <= ell <= 719,
        "order_361_720_degree_600_719": lambda ell, order: 361 <= order <= 720 and 600 <= ell <= 719,
    }
    ring_target_groups = {
        "order_1_32_degree_600_719": intersection_groups[
            "order_1_32_degree_600_719"
        ],
        "order_1_32_degree_720": intersection_groups[
            "order_1_32_degree_720"
        ],
    }
    low_order_groups = {
        "order_1_degree_600_719": lambda ell, order: order == 1 and 600 <= ell <= 719,
        "orders_2_4_degree_600_719": lambda ell, order: 2 <= order <= 4 and 600 <= ell <= 719,
        "orders_5_8_degree_600_719": lambda ell, order: 5 <= order <= 8 and 600 <= ell <= 719,
        "orders_9_16_degree_600_719": lambda ell, order: 9 <= order <= 16 and 600 <= ell <= 719,
        "orders_17_32_degree_600_719": lambda ell, order: 17 <= order <= 32 and 600 <= ell <= 719,
    }
    if args.subset == "base":
        groups = base_groups
    elif args.subset == "intersections":
        groups = intersection_groups
    elif args.subset == "ring-target":
        groups = ring_target_groups
    elif args.subset == "low-order":
        groups = low_order_groups
    else:
        groups = base_groups | intersection_groups | low_order_groups

    records = []
    args.scratch_root.mkdir(parents=True, exist_ok=True)
    for group_name, selected in groups.items():
        filtered = [values.copy() for values in coefficients]
        index = 0
        selected_coefficients = 0
        for order in range(case["mmax"] + 1):
            for ell in range(order, case["lmax"] + 1):
                if selected(ell, order):
                    selected_coefficients += 1
                else:
                    for values in filtered:
                        values[index] = values[index + 1] = 0.0
                index += 2

        alms = {}
        for channel, name in enumerate(("alm0", "alm1")):
            path = args.scratch_root / f"{group_name}-{name}.f32"
            path.write_bytes(struct.pack(f"<{len(filtered[channel])}f", *filtered[channel]))
            alms[name] = path
        filtered_fixture = {"map0": fixture["map0"], "map1": fixture["map1"], **alms}
        with tempfile.TemporaryDirectory(prefix=f"{group_name}-", dir=args.scratch_root) as temp:
            temp = pathlib.Path(temp)
            rust = bench.run_worker(
                args.candidate_bin, "candidate", case, filtered_fixture,
                temp / "candidate", 2.0, "0",
            )
            ducc = bench.run_worker(
                args.ducc_bin, "ducc", case, filtered_fixture,
                temp / "ducc", 2.0, "0",
            )
            error = bench.error_metrics(
                [pathlib.Path(path) for path in ducc["outputs"]],
                [pathlib.Path(path) for path in rust["outputs"]],
            )
            ringwise = (
                ring_error_metrics(
                    [pathlib.Path(path) for path in ducc["outputs"]],
                    [pathlib.Path(path) for path in rust["outputs"]],
                    case["ntheta"],
                    case["nphi"],
                    bench.read_f32,
                )
                if args.ringwise
                else None
            )
        records.append({
            "group": group_name,
            "selected_complex_coefficients_per_channel": selected_coefficients,
            "filtered_alm_sha256": {name: sha256(path) for name, path in alms.items()},
            "candidate_simd_arch": rust.get("simd_arch"),
            "candidate_f32_lanes": rust.get("f32_lanes"),
            "candidate_repeat_consistent": rust.get("repeat_consistent"),
            "candidate_reset_consistent": rust.get("reset_consistent"),
            "candidate_all_finite": rust.get("finite_outputs"),
            "error_vs_ducc": error,
            "ringwise_error_vs_ducc": ringwise,
        })

    result = {
        "method": f"single native-f32 GL T720 full-M720 synthesis fixture filtered to {args.subset} degree/order bands; q1; accuracy-only; target batch 2 ms; timings intentionally omitted",
        "source_root_commit": "b6a96aa7161a64f678b614d73478496f50f0551e",
        "candidate_patch_sha256": "7af69b3fe0af09754d09eac67ed10006eea1a14fc468483ed24d50ce771fc1b9",
        "candidate_binary_sha256": sha256(args.candidate_bin),
        "ducc_commit": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "ducc_binary_sha256": sha256(args.ducc_bin),
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "cpu": "AMD Ryzen 9 5950X",
        "original_fixture_sha256": original_hashes,
        "groups": records,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"result": str(args.output), "groups": len(records)}, indent=2))


def ring_error_metrics(reference_paths, candidate_paths, ntheta, nphi, read_f32):
    reference_maps = [read_f32(path) for path in reference_paths]
    candidate_maps = [read_f32(path) for path in candidate_paths]
    records = []
    for ring in range(ntheta):
        start = ring * nphi
        end = start + nphi
        differences = [
            candidate[index] - reference[index]
            for reference, candidate in zip(reference_maps, candidate_maps)
            for index in range(start, end)
        ]
        reference_values = [
            reference[index]
            for reference in reference_maps
            for index in range(start, end)
        ]
        reference_sq = math.fsum(value * value for value in reference_values)
        difference_sq = math.fsum(value * value for value in differences)
        records.append({
            "ring_north_to_south": ring,
            "relative_l2": (
                math.sqrt(difference_sq / reference_sq)
                if reference_sq > 0.0
                else (0.0 if difference_sq == 0.0 else None)
            ),
            "max_absolute": max((abs(value) for value in differences), default=0.0),
            "reference_rms": math.sqrt(reference_sq / max(1, 2 * nphi)),
        })
    return {
        "ring_order": "north to south",
        "ring_count": ntheta,
        "by_ring": records,
        "largest_relative_l2_rings": sorted(
            records,
            key=lambda row: row["relative_l2"] if row["relative_l2"] is not None else -1.0,
            reverse=True,
        )[:10],
        "largest_absolute_error_rings": sorted(
            records, key=lambda row: row["max_absolute"], reverse=True
        )[:10],
    }


if __name__ == "__main__":
    main()
