#!/usr/bin/env python3
"""Collect independent calibrated pack/FFT/extract phase profiles."""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import os
import pathlib
import re
import subprocess
import time


def parse_fields(line: str) -> dict[str, str]:
    return dict(re.findall(r"([a-z0-9_]+)=([^,]+)", line))


def snapshot() -> dict[str, object]:
    result: dict[str, object] = {
        "utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "load_average": os.getloadavg(),
        "cpu_pressure": pathlib.Path("/proc/pressure/cpu").read_text(),
    }
    try:
        result["top_processes"] = subprocess.run(
            ["ps", "-eo", "pid,psr,pcpu,comm", "--sort=-pcpu"],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
        ).stdout.splitlines()[:12]
    except OSError:
        result["top_processes"] = "unavailable"
    return result


def pressure_total(value: dict[str, object]) -> int | None:
    match = re.search(r"^some .*\btotal=(\d+)", str(value["cpu_pressure"]), re.MULTILINE)
    return int(match.group(1)) if match else None


def write_csv(path: pathlib.Path, rows: list[dict[str, object]]) -> None:
    if not rows:
        return
    fields = list(dict.fromkeys(key for row in rows for key in row))
    with path.open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-bin", type=pathlib.Path, required=True)
    parser.add_argument("--output-dir", type=pathlib.Path, required=True)
    parser.add_argument("--lengths", default="361,509,513")
    parser.add_argument("--operations", default="forward,inverse")
    parser.add_argument("--process-runs", type=int, default=3)
    parser.add_argument("--q1-cpus", default="8")
    args = parser.parse_args()
    if args.process_runs < 3:
        parser.error("use at least three independent processes per route and case")

    binary = args.rust_bin.resolve()
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    temp_dir = output_dir / "fixtures"
    temp_dir.mkdir(exist_ok=True)
    lengths = tuple(int(value) for value in args.lengths.split(","))
    operations = tuple(args.operations.split(","))
    cases: list[tuple[int, str, tuple[str, ...]]] = []
    for n in lengths:
        for operation in operations:
            routes = ("rust-real", "rust-pair")
            if n == 361 and operation == "inverse":
                routes = (*routes, "rust-pair-mixed")
            cases.append((n, operation, routes))

    config = {
        "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "binary": str(binary),
        "lengths": lengths,
        "operations": operations,
        "shape": "full",
        "rows": 2,
        "process_runs_per_route": args.process_runs,
        "timed_batches_per_phase": 30,
        "target_batch_ms": 30,
        "q1_cpus": args.q1_cpus,
        "host_before": snapshot(),
    }
    (output_dir / "config.json").write_text(json.dumps(config, indent=2) + "\n")

    process_rows: list[dict[str, object]] = []
    phase_rows: list[dict[str, object]] = []
    total = sum(len(routes) * args.process_runs for _, _, routes in cases)
    completed = 0
    for n, operation, routes in cases:
        fixture = temp_dir / f"n{n}-{operation}.input"
        if not fixture.exists():
            subprocess.run(
                [str(binary), "--write-fixture", operation, str(n), "2", "full", str(fixture)],
                check=True,
            )
        for run in range(args.process_runs):
            order = list(routes)
            shift = run % len(order)
            order = order[shift:] + order[:shift]
            if run % 2:
                order.reverse()
            for route in order:
                log = output_dir / f"n{n}-{operation}-run{run + 1}-{route}.txt"
                before = snapshot()
                started = time.monotonic()
                result = subprocess.run(
                    [
                        "taskset",
                        "-c",
                        args.q1_cpus,
                        str(binary),
                        "--profile",
                        route,
                        operation,
                        str(n),
                        "full",
                        str(fixture),
                    ],
                    check=True,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    env=os.environ.copy(),
                )
                elapsed = time.monotonic() - started
                after = snapshot()
                log.write_text(result.stdout)
                env_line = next(
                    line for line in result.stdout.splitlines() if line.startswith("profile_environment,")
                )
                profile_env = parse_fields(env_line)
                for line in result.stdout.splitlines():
                    if line.startswith("profile,"):
                        fields = parse_fields(line)
                        phase_rows.append(
                            {
                                "n": n,
                                "operation": operation,
                                "shape": "full",
                                "process_run": run + 1,
                                "route": route,
                                **fields,
                            }
                        )
                    elif line.startswith("profile_estimate,"):
                        fields = parse_fields(line)
                        phase_rows.append(
                            {
                                "n": n,
                                "operation": operation,
                                "shape": "full",
                                "process_run": run + 1,
                                "route": route,
                                **fields,
                            }
                        )
                start_total = pressure_total(before)
                end_total = pressure_total(after)
                wait_pct = (
                    100.0 * (end_total - start_total) / (elapsed * 1.0e6)
                    if start_total is not None and end_total is not None and elapsed > 0
                    else None
                )
                process_rows.append(
                    {
                        "n": n,
                        "operation": operation,
                        "shape": "full",
                        "process_run": run + 1,
                        "route": route,
                        "cpu_set": args.q1_cpus,
                        "runtime_avx2": profile_env.get("runtime_avx2", "not-reported"),
                        "runtime_fma": profile_env.get("runtime_fma", "not-reported"),
                        "elapsed_process_s": f"{elapsed:.6f}",
                        "host_cpu_psi_some_wait_percent": "" if wait_pct is None else f"{wait_pct:.3f}",
                        "host_before_cpu_psi": str(before["cpu_pressure"]).splitlines()[0],
                        "host_after_cpu_psi": str(after["cpu_pressure"]).splitlines()[0],
                        "host_before_top_processes": json.dumps(before["top_processes"]),
                        "host_after_top_processes": json.dumps(after["top_processes"]),
                        "log": str(log),
                    }
                )
                completed += 1
                write_csv(output_dir / "process-samples.csv", process_rows)
                write_csv(output_dir / "phase-samples.csv", phase_rows)
                print(f"finished {completed}/{total}: N={n} {operation} {route} run={run + 1}", flush=True)

    config["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    config["host_after"] = snapshot()
    (output_dir / "config.json").write_text(json.dumps(config, indent=2) + "\n")
    write_csv(output_dir / "process-samples.csv", process_rows)
    write_csv(output_dir / "phase-samples.csv", phase_rows)


if __name__ == "__main__":
    main()
