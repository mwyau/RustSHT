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
    groups = {
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
        })

    result = {
        "method": "single native-f32 GL T720 full-M720 synthesis fixture filtered to independent degree or order bands; q1; accuracy-only; target batch 2 ms; timings intentionally omitted",
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


if __name__ == "__main__":
    main()
