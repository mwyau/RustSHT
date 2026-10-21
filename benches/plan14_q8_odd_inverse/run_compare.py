#!/usr/bin/env python3
"""Run calibrated matched inverse-row comparisons against pinned DUCC."""

import argparse
import csv
import datetime as dt
import json
import math
import os
import pathlib
import re
import struct
import subprocess
import tempfile
import time


def parse_summary(output: str) -> dict[str, float]:
    line = next(line for line in output.splitlines() if line.startswith("summary,"))
    return {
        key: float(value)
        for key, value in re.findall(r"([a-z0-9_]+)=([0-9.eE+-]+)", line)
    }


def parse_first_call_ms(output: str) -> float:
    line = next(line for line in output.splitlines() if line.startswith("environment,"))
    match = re.search(r"first_ms=([0-9.eE+-]+)", line)
    if match is None:
        raise RuntimeError("benchmark output omitted first-call latency")
    return float(match.group(1))


def run(args: list[str], cpus: str, env: dict[str, str]) -> tuple[str, float]:
    start = time.monotonic()
    result = subprocess.run(
        ["taskset", "-c", cpus, *args],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        env=env,
    )
    return result.stdout, time.monotonic() - start


def compare_output(rust_path: pathlib.Path, ducc_path: pathlib.Path) -> tuple[float, float]:
    rust_data = rust_path.read_bytes()
    ducc_data = ducc_path.read_bytes()
    if len(rust_data) != len(ducc_data) or len(rust_data) % 8:
        raise RuntimeError("backend output lengths differ or are not f64 arrays")
    count = len(rust_data) // 8
    rust_values = struct.unpack("<" + "d" * count, rust_data)
    ducc_values = struct.unpack("<" + "d" * count, ducc_data)
    error2 = math.fsum((left - right) ** 2 for left, right in zip(rust_values, ducc_values))
    norm2 = math.fsum(value * value for value in rust_values)
    maximum = max(abs(left - right) for left, right in zip(rust_values, ducc_values))
    return math.sqrt(error2 / norm2), maximum


def host_snapshot() -> dict[str, str]:
    result = {"cpu_pressure": pathlib.Path("/proc/pressure/cpu").read_text().strip()}
    for name in ("scaling_governor", "scaling_cur_freq", "scaling_max_freq", "cpuinfo_max_freq"):
        path = pathlib.Path(f"/sys/devices/system/cpu/cpu0/cpufreq/{name}")
        try:
            result[f"cpu0_{name}"] = path.read_text().strip()
        except OSError:
            result[f"cpu0_{name}"] = "unavailable"
    return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-bin", type=pathlib.Path, required=True)
    parser.add_argument("--ducc-bin", type=pathlib.Path, required=True)
    parser.add_argument("--output-dir", type=pathlib.Path, required=True)
    parser.add_argument("--pairs", type=int, default=3)
    parser.add_argument("--rows", type=int, default=721)
    parser.add_argument(
        "--case",
        action="append",
        choices=("q1-odd", "q8-odd", "q1-even", "q8-even"),
        help="limit the run to one or more named cases",
    )
    parser.add_argument("--q1-cpus", default="0")
    parser.add_argument("--q8-cpus", default="0-7")
    args = parser.parse_args()
    if args.pairs < 1 or args.rows < 1:
        parser.error("pairs and rows must be positive")
    args.output_dir.mkdir(parents=True, exist_ok=True)

    cases = [(361, 1, "q1-odd"), (361, 8, "q8-odd"), (378, 1, "q1-even"), (378, 8, "q8-even")]
    if args.case:
        cases = [case for case in cases if case[2] in args.case]
    config = {
        "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "rows": args.rows,
        "mmax": 180,
        "cases": [{"nphi": n, "threads": threads, "name": name} for n, threads, name in cases],
        "pairs": args.pairs,
        "q1_cpus": args.q1_cpus,
        "q8_cpus": args.q8_cpus,
        "batch_target_ms": 30,
        "batches_per_process": 30,
        "host_before": host_snapshot(),
    }
    (args.output_dir / "config.json").write_text(json.dumps(config, indent=2) + "\n")

    summary_rows = []
    validation_rows = []
    with tempfile.TemporaryDirectory(prefix="plan14-ducc-rows-") as scratch:
        scratch_path = pathlib.Path(scratch)
        for n, threads, name in cases:
            fixture = scratch_path / f"{name}.spectra"
            subprocess.run(
                [str(args.rust_bin), "--write-fixture", str(n), str(args.rows), str(fixture)],
                check=True,
            )
            cpus = args.q1_cpus if threads == 1 else args.q8_cpus
            for pair in range(1, args.pairs + 1):
                order = ("rust", "ducc") if pair % 2 else ("ducc", "rust")
                outputs = {}
                parsed = {}
                for backend in order:
                    output_path = scratch_path / f"{name}-{backend}-{pair}.f64"
                    if backend == "rust":
                        command = [
                            str(args.rust_bin), str(fixture), str(n), str(args.rows),
                            str(threads), str(output_path),
                        ]
                    else:
                        command = [
                            str(args.ducc_bin), str(fixture), str(n), str(args.rows),
                            str(threads), str(output_path),
                        ]
                    env = os.environ.copy()
                    if backend == "ducc":
                        env["DUCC0_NUM_THREADS"] = str(threads)
                    output, elapsed_s = run(command, cpus, env)
                    (args.output_dir / f"{name}-pair{pair}-{backend}.txt").write_text(output)
                    parsed[backend] = parse_summary(output)
                    outputs[backend] = output_path
                    print(f"{name} pair={pair} backend={backend} elapsed={elapsed_s:.2f}s")

                relative_l2, max_abs = compare_output(outputs["rust"], outputs["ducc"])
                validation_rows.append({
                    "case": name,
                    "process_pair": pair,
                    "relative_l2": relative_l2,
                    "max_abs": max_abs,
                })
                rust_summary = parsed["rust"]
                ducc_summary = parsed["ducc"]
                summary_rows.append({
                    "case": name,
                    "nphi": n,
                    "rows": args.rows,
                    "mmax": 180,
                    "threads": threads,
                    "process_pair": pair,
                    "rust_first_ms": parse_first_call_ms(
                        (args.output_dir / f"{name}-pair{pair}-rust.txt").read_text()
                    ),
                    "ducc_first_ms": parse_first_call_ms(
                        (args.output_dir / f"{name}-pair{pair}-ducc.txt").read_text()
                    ),
                    "rust_p10_ms": rust_summary["p10_ms"],
                    "rust_median_ms": rust_summary["median_ms"],
                    "rust_p90_ms": rust_summary["p90_ms"],
                    "ducc_p10_ms": ducc_summary["p10_ms"],
                    "ducc_median_ms": ducc_summary["median_ms"],
                    "ducc_p90_ms": ducc_summary["p90_ms"],
                    "rust_ducc_ratio": rust_summary["median_ms"] / ducc_summary["median_ms"],
                    "excess_ms": rust_summary["median_ms"] - ducc_summary["median_ms"],
                    "relative_l2": relative_l2,
                    "max_abs": max_abs,
                })
                print(f"{name} pair={pair} relative_l2={relative_l2:.4e} max_abs={max_abs:.4e}")

    for filename, rows in (("summary.csv", summary_rows), ("validation.csv", validation_rows)):
        with (args.output_dir / filename).open("w", newline="") as output_file:
            writer = csv.DictWriter(
                output_file, fieldnames=list(rows[0]), lineterminator="\n"
            )
            writer.writeheader()
            writer.writerows(rows)
    config["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    config["host_after"] = host_snapshot()
    (args.output_dir / "config.json").write_text(json.dumps(config, indent=2) + "\n")


if __name__ == "__main__":
    main()
