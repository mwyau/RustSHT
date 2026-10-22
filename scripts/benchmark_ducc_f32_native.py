#!/usr/bin/env python3
"""Matched native f32 RustSHT/DUCC spin-0 and STANDARD spin-1 screen."""

from __future__ import annotations

import argparse
import csv
import json
import math
import os
import random
import statistics
import struct
import subprocess
import time
from pathlib import Path

DUCC_SHA = "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
PHI0 = 0.23


def smooth_even_at_least(minimum: int) -> int:
    value = max(2, minimum + minimum % 2)
    while True:
        remainder = value
        for factor in (2, 3, 5, 7):
            while remainder % factor == 0:
                remainder //= factor
        if remainder == 1:
            return value
        value += 2


def nalm(lmax: int, mmax: int) -> int:
    return (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) // 2


def write_f32(path: Path, values: list[float]) -> None:
    with path.open("wb") as stream:
        for start in range(0, len(values), 8192):
            stream.write(struct.pack(f"<{len(values[start:start + 8192])}f", *values[start:start + 8192]))


def read_f32(path: Path) -> list[float]:
    raw = path.read_bytes()
    if len(raw) % 4:
        raise ValueError(f"{path} byte count is not divisible by four")
    return list(struct.unpack(f"<{len(raw) // 4}f", raw))


def make_fixture(case: dict, directory: Path) -> dict[str, Path]:
    directory.mkdir(parents=True, exist_ok=True)
    count = case["ntheta"] * case["nphi"]
    coefficients = nalm(case["lmax"], case["mmax"])
    rng = random.Random(0xF32D00D + case["lmax"] * 31 + case["spin"] * 7 + case["nphi"])
    maps = [[rng.uniform(-1.0, 1.0) for _ in range(count)] for _ in range(2)]
    coeff_scale = 1.0 / max(1, case["lmax"] + 1)
    alms = []
    for _ in range(2):
        values = []
        for m in range(case["mmax"] + 1):
            for _l in range(m, case["lmax"] + 1):
                if _l < case["spin"]:
                    values.extend((0.0, 0.0))
                else:
                    values.append(rng.uniform(-1.0, 1.0) * coeff_scale)
                    values.append(0.0 if m == 0 else rng.uniform(-1.0, 1.0) * coeff_scale)
        alms.append(values)
    paths = {}
    for name, values in (("map0", maps[0]), ("map1", maps[1]),
                         ("alm0", alms[0]), ("alm1", alms[1])):
        paths[name] = directory / f"{name}.f32"
        write_f32(paths[name], values)
    return paths


def percentile(values: list[float], q: float) -> float:
    if not values:
        return math.nan
    ordered = sorted(values)
    position = q * (len(ordered) - 1)
    low = math.floor(position)
    high = math.ceil(position)
    return ordered[low] if low == high else ordered[low] * (high - position) + ordered[high] * (position - low)


def error_metrics(reference_paths: list[Path], candidate_paths: list[Path]) -> dict:
    reference = [value for path in reference_paths for value in read_f32(path)]
    candidate = [value for path in candidate_paths for value in read_f32(path)]
    if len(reference) != len(candidate):
        raise ValueError(f"output lengths differ: {len(candidate)} != {len(reference)}")
    diffs = [a - b for a, b in zip(candidate, reference)]
    ref_sq = math.fsum(value * value for value in reference)
    diff_sq = math.fsum(value * value for value in diffs)
    return {
        "output_values": len(reference),
        "relative_l2": math.sqrt(diff_sq / ref_sq) if ref_sq else (0.0 if diff_sq == 0 else math.inf),
        "absolute_l2": math.sqrt(diff_sq),
        "max_absolute": max((abs(value) for value in diffs), default=0.0),
        "reference_rms": math.sqrt(ref_sq / max(1, len(reference))),
        "finite": all(math.isfinite(x) for x in candidate),
    }


def cpu_telemetry(cpus: str) -> dict:
    result = {"affinity_cpus": cpus}
    try:
        result["cpu_pressure"] = Path("/proc/pressure/cpu").read_text().strip()
    except OSError:
        result["cpu_pressure"] = None
    frequencies = {}
    for cpu in cpus.split(","):
        path = Path(f"/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_cur_freq")
        try:
            frequencies[cpu] = int(path.read_text().strip())
        except (OSError, ValueError):
            frequencies[cpu] = None
    result["scaling_cur_freq_khz"] = frequencies
    return result


def timing_details(result: dict) -> dict:
    fields = ("backend", "first_ms", "warmup_ms", "setup_ms", "pool_setup_ms",
              "fixture_load_ms", "repetitions_per_batch", "batch_ms", "batch_total_ms",
              "process_elapsed_s", "output_checksum", "finite_outputs", "repeat_consistent",
              "reset_consistent", "simd_arch", "f32_lanes", "telemetry")
    return {field: result[field] for field in fields if field in result}


def default_cases(smoke: bool) -> list[dict]:
    degrees = [7] if smoke else [127, 255, 511, 720, 1023]
    cases = []
    for lmax in degrees:
        mmaxes = [lmax] if smoke else ([lmax, max(1, lmax // 4)] if lmax in (127, 255, 720) else [lmax])
        for mmax in mmaxes:
            shapes = ["even"] if smoke else (["even", "odd"] if lmax in (127, 255) else ["even"])
            for shape in shapes:
                nphi = smooth_even_at_least(2 * mmax + 2) if shape == "even" else 2 * mmax + 1
                for spin in (0, 1):
                    for operation in ("analysis", "synthesis"):
                        for threads in (1, 8):
                            cases.append({
                                "grid": "GL", "ntheta": lmax + 1, "nphi": nphi,
                                "lmax": lmax, "mmax": mmax, "longitude": shape,
                                "spin": spin, "operation": operation, "threads": threads,
                            })
    if not smoke:
        # Limited exact CC/F1 controls at T255, without repeating the whole GL matrix.
        for grid, ntheta in (("CC", 257), ("F1", 256)):
            for spin in (0, 1):
                for operation in ("analysis", "synthesis"):
                    for threads in (1, 8):
                        cases.append({
                            "grid": grid, "ntheta": ntheta, "nphi": smooth_even_at_least(512),
                            "lmax": 255, "mmax": 255, "longitude": "even",
                            "spin": spin, "operation": operation, "threads": threads,
                        })
    return cases


def run_worker(binary: Path, backend: str, case: dict, fixture: dict[str, Path],
               run_dir: Path, target_ms: float, cpus: str) -> dict:
    run_dir.mkdir(parents=True, exist_ok=True)
    outputs = [run_dir / "out0.f32", run_dir / "out1.f32"]
    args = [
        str(binary), "--worker", case["operation"], str(case["spin"]), case["grid"],
        str(case["threads"]), str(case["ntheta"]), str(case["nphi"]), str(case["lmax"]),
        str(case["mmax"]), repr(PHI0), str(fixture["map0"]), str(fixture["map1"]),
        str(fixture["alm0"]), str(fixture["alm1"]), str(outputs[0]), str(outputs[1]), repr(target_ms),
    ]
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = str(case["threads"])
    started = time.monotonic()
    command = ["taskset", "-c", cpus, *args]
    completed = subprocess.run(command, text=True, capture_output=True, env=env)
    elapsed = time.monotonic() - started
    if completed.returncode != 0:
        raise RuntimeError(f"{backend} worker failed for {case}: {completed.stderr}\n{completed.stdout}")
    output = json.loads(completed.stdout.strip().splitlines()[-1])
    output["process_elapsed_s"] = elapsed
    output["outputs"] = [str(path) for path in outputs if path.exists()]
    output["telemetry"] = cpu_telemetry(cpus)
    return output


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-binary", type=Path, default=Path("target/release/examples/ducc_f32_full_transform_parity"))
    parser.add_argument("--ducc-binary", type=Path, default=Path("build/ducc-f32-plan17/ducc_f32_full_transform_parity"))
    parser.add_argument("--outdir", type=Path, required=True)
    parser.add_argument("--scratch-dir", type=Path)
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--process-pairs", type=int, default=3)
    parser.add_argument("--target-ms", type=float, default=25.0)
    parser.add_argument("--q1-cpu", default="0")
    parser.add_argument("--q8-cpus", default="0,1,2,3,4,5,6,7")
    args = parser.parse_args()
    if not args.rust_binary.is_file() or not args.ducc_binary.is_file():
        raise SystemExit("build both native workers before running the benchmark")
    cases = default_cases(args.smoke)
    args.outdir.mkdir(parents=True, exist_ok=True)
    scratch_dir = args.scratch_dir or Path("benchmark-scratch") / f"plan17-f32-{time.time_ns()}"
    scratch_dir.mkdir(parents=True, exist_ok=True)
    all_rows = []
    for index, case in enumerate(cases, 1):
        case_id = f"{case['grid']}-T{case['lmax']}-M{case['mmax']}-{case['longitude']}-s{case['spin']}-{case['operation']}-q{case['threads']}"
        case_dir = args.outdir / case_id
        case_dir.mkdir(parents=True, exist_ok=True)
        fixture = make_fixture(case, scratch_dir / case_id)
        process_rows = []
        for pair in range(args.process_pairs):
            order = ("rust", "ducc") if pair % 2 == 0 else ("ducc", "rust")
            outputs = {}
            for backend in order:
                binary = args.rust_binary if backend == "rust" else args.ducc_binary
                cpus = args.q1_cpu if case["threads"] == 1 else args.q8_cpus
                result = run_worker(binary, backend, case, fixture,
                                    scratch_dir / case_id / f"pair-{pair + 1}" / backend,
                                    args.target_ms, cpus)
                result.update({"backend": backend, "pair": pair + 1, **case})
                (case_dir / f"pair-{pair + 1}-{backend}.json").write_text(json.dumps(result, indent=2) + "\n")
                outputs[backend] = result
            reference_paths = [Path(path) for path in outputs["ducc"]["outputs"]]
            candidate_paths = [Path(path) for path in outputs["rust"]["outputs"]]
            error = error_metrics(reference_paths, candidate_paths)
            process_rows.append({
                "pair": pair + 1,
                "rust_median_ms": statistics.median(outputs["rust"]["batch_ms"]),
                "ducc_median_ms": statistics.median(outputs["ducc"]["batch_ms"]),
                "ratio": statistics.median(outputs["rust"]["batch_ms"]) / statistics.median(outputs["ducc"]["batch_ms"]),
                "error": error,
                "rust": timing_details(outputs["rust"]),
                "ducc": timing_details(outputs["ducc"]),
            })
            print(json.dumps({"case": case_id, **process_rows[-1]}), flush=True)
        rust_samples = [v for pair in range(1, args.process_pairs + 1)
                        for v in json.loads((case_dir / f"pair-{pair}-rust.json").read_text())["batch_ms"]]
        ducc_samples = [v for pair in range(1, args.process_pairs + 1)
                        for v in json.loads((case_dir / f"pair-{pair}-ducc.json").read_text())["batch_ms"]]
        ratios = [item["ratio"] for item in process_rows]
        row = {
            **case, "case_id": case_id, "status": "complete",
            "rust_median_ms": statistics.median(rust_samples), "rust_p10_ms": percentile(rust_samples, 0.1),
            "rust_p90_ms": percentile(rust_samples, 0.9), "ducc_median_ms": statistics.median(ducc_samples),
            "ducc_p10_ms": percentile(ducc_samples, 0.1), "ducc_p90_ms": percentile(ducc_samples, 0.9),
            "paired_ratio_median": statistics.median(ratios),
            "paired_ratio_p10": percentile(ratios, 0.1), "paired_ratio_p90": percentile(ratios, 0.9),
            "batch_count_rust": len(rust_samples), "batch_count_ducc": len(ducc_samples),
            "process_pairs": args.process_pairs,
            "error": {
                "output_values": process_rows[-1]["error"]["output_values"],
                "relative_l2_max_across_pairs": max(p["error"]["relative_l2"] for p in process_rows),
                "absolute_l2_max_across_pairs": max(p["error"]["absolute_l2"] for p in process_rows),
                "max_absolute_max_across_pairs": max(p["error"]["max_absolute"] for p in process_rows),
                "reference_rms": process_rows[-1]["error"]["reference_rms"],
                "finite": all(p["error"]["finite"] for p in process_rows),
            },
            "process_pair_details": process_rows,
        }
        all_rows.append(row)
        print(f"[{index}/{len(cases)}] {case_id} done", flush=True)
    (args.outdir / "results.json").write_text(json.dumps({
        "metadata": {
            "ducc_sha": DUCC_SHA, "rust_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
            "rustc_vv": subprocess.check_output(["rustc", "-Vv"], text=True),
            "ducc_cxx_flags": "-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3",
            "driver_flags": "-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3",
            "rust_flags": os.environ.get("RUSTFLAGS", "<unset; Cargo release defaults>"),
            "rust_release_profile": "Cargo release profile, no target-cpu override; Pulp runtime dispatch",
            "target_ms": args.target_ms, "process_pairs": args.process_pairs,
            "scratch_dir": str(scratch_dir),
            "cpu_model": subprocess.check_output(["lscpu"], text=True).split("Model name:")[1].splitlines()[0].strip(),
            "q1_affinity": args.q1_cpu,
            "q8_affinity": args.q8_cpus,
            "method": "same fixtures, reusable Rust plan/workspace, public DUCC API, alternating fresh processes; 10 calibrated batches under 100 ms/call, 5 individual batches otherwise",
        }, "cases": all_rows,
    }, indent=2) + "\n")
    with (args.outdir / "results.csv").open("w", newline="") as stream:
        fields = ["case_id", "grid", "lmax", "mmax", "longitude", "spin", "operation", "threads",
                  "ntheta", "nphi",
                  "rust_median_ms", "rust_p10_ms", "rust_p90_ms", "ducc_median_ms", "ducc_p10_ms",
                  "ducc_p90_ms", "paired_ratio_median", "paired_ratio_p10", "paired_ratio_p90",
                  "batch_count_rust", "batch_count_ducc", "error_relative_l2", "error_max_absolute"]
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        for row in all_rows:
            csv_row = {field: row.get(field) for field in fields}
            csv_row["error_relative_l2"] = row["error"]["relative_l2_max_across_pairs"]
            csv_row["error_max_absolute"] = row["error"]["max_absolute_max_across_pairs"]
            writer.writerow(csv_row)


if __name__ == "__main__":
    main()
