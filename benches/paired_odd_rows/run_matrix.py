#!/usr/bin/env python3
"""Run interleaved, calibrated RustSHT/RustFFT/DUCC row-pair timings."""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import json
import math
import os
import pathlib
import re
import subprocess
import tempfile
import time


DUCC_REVISION = "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
ODD_LENGTHS = (129, 255, 257, 361, 497, 509, 511, 513, 515)
EVEN_LENGTHS = (256, 378, 500, 512, 540)
ROWS_BY_LENGTH = {
    129: 65,
    255: 128,
    257: 129,
    361: 721,
    497: 249,
    509: 255,
    511: 256,
    513: 257,
    515: 258,
    256: 128,
    378: 721,
    500: 250,
    512: 256,
    540: 270,
}
ROUTES = ("rust-real", "rust-c2c", "rust-pair", "ducc-real")


def parse_pairs(line: str) -> dict[str, str]:
    return dict(re.findall(r"([a-z0-9_]+)=([^,]+)", line))


def read_values(path: pathlib.Path) -> list[float]:
    import struct

    data = path.read_bytes()
    if len(data) % 8:
        raise RuntimeError(f"output {path} is not an f64 array")
    return list(struct.unpack("<" + "d" * (len(data) // 8), data))


def compare(reference: list[float], actual: list[float]) -> tuple[float, float]:
    if len(reference) != len(actual):
        raise RuntimeError(f"output sizes differ: {len(reference)} != {len(actual)}")
    error2 = math.fsum((x - y) ** 2 for x, y in zip(reference, actual))
    norm2 = math.fsum(value * value for value in reference)
    max_abs = max((abs(x - y) for x, y in zip(reference, actual)), default=0.0)
    scale = max(1.0, max((abs(value) for value in reference), default=0.0))
    return (math.sqrt(error2 / norm2) if norm2 else math.sqrt(error2), max_abs / scale)


def host_snapshot() -> dict[str, object]:
    result: dict[str, object] = {
        "utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "load_average": os.getloadavg(),
        "cpu_pressure": pathlib.Path("/proc/pressure/cpu").read_text().strip(),
    }
    for field in ("scaling_governor", "scaling_cur_freq", "scaling_max_freq"):
        path = pathlib.Path(f"/sys/devices/system/cpu/cpu0/cpufreq/{field}")
        try:
            result[f"cpu0_{field}"] = path.read_text().strip()
        except OSError:
            result[f"cpu0_{field}"] = "unavailable"
    try:
        result["top_processes"] = subprocess.run(
            ["ps", "-eo", "pid,psr,pcpu,comm", "--sort=-pcpu"],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
        ).stdout.splitlines()[:14]
    except OSError:
        result["top_processes"] = "unavailable"
    return result


def harness_sha256(repo: pathlib.Path) -> str:
    root = repo / "benches" / "paired_odd_rows"
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        if not path.is_file() or "target" in path.parts or "__pycache__" in path.parts:
            continue
        digest.update(path.relative_to(root).as_posix().encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def pressure_total(snapshot: dict[str, object], section: str) -> int | None:
    raw = str(snapshot.get("cpu_pressure", ""))
    for line in raw.splitlines():
        if line.startswith(section + " "):
            match = re.search(r"\btotal=(\d+)", line)
            if match:
                return int(match.group(1))
    return None


def source_sha(repo: pathlib.Path) -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout.strip()


def run_process(
    command: list[str], cpus: str, env: dict[str, str], log: pathlib.Path
) -> tuple[str, float, float | None, dict[str, object], dict[str, object]]:
    before = host_snapshot()
    start = time.monotonic()
    result = subprocess.run(
        ["taskset", "-c", cpus, *command],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        env=env,
    )
    elapsed = time.monotonic() - start
    after = host_snapshot()
    log.write_text(result.stdout)
    before_total = pressure_total(before, "some")
    after_total = pressure_total(after, "some")
    wait_percent = None
    if before_total is not None and after_total is not None and elapsed > 0:
        wait_percent = 100.0 * (after_total - before_total) / (elapsed * 1.0e6)
    return result.stdout, elapsed, wait_percent, before, after


def parse_benchmark(output: str) -> tuple[dict[str, str], dict[str, str], list[float]]:
    environment = parse_pairs(next(line for line in output.splitlines() if line.startswith("environment,")))
    summary = parse_pairs(next(line for line in output.splitlines() if line.startswith("summary,")))
    batches = [
        float(parse_pairs(line)["ms"])
        for line in output.splitlines()
        if line.startswith("batch,")
    ]
    if int(environment["batches"]) != 30 or len(batches) != 30:
        raise RuntimeError("benchmark did not return 30 calibrated samples")
    return environment, summary, batches


def write_rows(path: pathlib.Path, rows: list[dict[str, object]]) -> None:
    if not rows:
        return
    fields = list(dict.fromkeys(key for row in rows for key in row))
    with path.open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def case_list(lengths: tuple[int, ...], shapes: tuple[str, ...], operations: tuple[str, ...], threads: tuple[int, ...]):
    cases = []
    for n in lengths:
        for operation in operations:
            case_shapes = shapes if operation == "forward" else (*shapes, "polar")
            for shape in case_shapes:
                if shape == "polar" and n not in (361, 497, 509, 511, 513):
                    continue
                for count in threads:
                    cases.append((n, ROWS_BY_LENGTH[n], operation, shape, count))
    return cases


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=pathlib.Path, default=pathlib.Path(__file__).resolve().parents[2])
    parser.add_argument("--rust-bin", type=pathlib.Path, required=True)
    parser.add_argument("--ducc-bin", type=pathlib.Path, required=True)
    parser.add_argument("--output-dir", type=pathlib.Path, required=True)
    parser.add_argument("--lengths", default=",".join(map(str, (*ODD_LENGTHS, *EVEN_LENGTHS))))
    parser.add_argument("--threads", default="1,8")
    parser.add_argument("--operations", default="forward,inverse")
    parser.add_argument("--shapes", default="full,reduced")
    parser.add_argument("--pairs", type=int, default=3, help="independent process runs per route/case")
    parser.add_argument("--q1-cpus", default="0")
    parser.add_argument("--q8-cpus", default="0-7")
    parser.add_argument("--seed", type=int, default=0x51A7)
    args = parser.parse_args()
    if args.pairs < 3:
        parser.error("use at least three independent process runs for final row-stage claims")
    lengths = tuple(int(value) for value in args.lengths.split(","))
    thread_counts = tuple(int(value) for value in args.threads.split(","))
    operations = tuple(args.operations.split(","))
    shapes = tuple(args.shapes.split(","))
    if any(n not in ROWS_BY_LENGTH for n in lengths):
        parser.error("length is outside the declared odd/even set")
    if any(thread not in (1, 8) for thread in thread_counts):
        parser.error("thread counts must be 1 or 8")
    if any(op not in ("forward", "inverse") for op in operations):
        parser.error("operations must be forward and/or inverse")
    if any(shape not in ("full", "reduced") for shape in shapes):
        parser.error("shapes must be full and/or reduced")

    args.output_dir.mkdir(parents=True, exist_ok=True)
    temp_dir = pathlib.Path(tempfile.mkdtemp(prefix="paired-odd-row-", dir=args.output_dir))
    rust_bin = args.rust_bin.resolve()
    ducc_bin = args.ducc_bin.resolve()
    src_sha = source_sha(args.repo.resolve())
    config = {
        "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "rust_source_sha": src_sha,
        "harness_sha256": harness_sha256(args.repo.resolve()),
        "ducc_revision": DUCC_REVISION,
        "lengths": lengths,
        "rows_by_length": {str(n): ROWS_BY_LENGTH[n] for n in lengths},
        "operations": operations,
        "shapes": shapes,
        "threads": thread_counts,
        "process_runs_per_route": args.pairs,
        "timed_batches_per_process": 30,
        "target_batch_ms": 30,
        "q1_cpus": args.q1_cpus,
        "q8_cpus": args.q8_cpus,
        "host_before": host_snapshot(),
    }
    (args.output_dir / "config.json").write_text(json.dumps(config, indent=2) + "\n")
    (args.output_dir / "host-before.json").write_text(json.dumps(config["host_before"], indent=2) + "\n")

    cases = case_list(lengths, shapes, operations, thread_counts)
    # The deterministic shuffle spreads small and large transforms through the
    # run while preserving adjacency within each interleaved route comparison.
    import random

    random.Random(args.seed).shuffle(cases)
    process_rows: list[dict[str, object]] = []
    validation_rows: list[dict[str, object]] = []
    total_runs = len(cases) * args.pairs
    completed_runs = 0

    for n, rows, operation, shape, threads in cases:
        case_key = (n, rows, operation, shape, threads)
        for pair in range(1, args.pairs + 1):
            route_names = list(ROUTES)
            if n == 361 and operation == "inverse":
                route_names.append("rust-pair-mixed")
            shift = (pair - 1) % len(route_names)
            if pair % 2 == 0:
                route_names.reverse()
            order = route_names[shift:] + route_names[:shift]
            fixture = temp_dir / f"n{n}-{operation}-{shape}.input"
            if not fixture.exists():
                subprocess.run(
                    [
                        str(rust_bin),
                        "--write-fixture",
                        operation,
                        str(n),
                        str(rows),
                        shape,
                        str(fixture),
                    ],
                    check=True,
                    stdout=subprocess.DEVNULL,
                )
            output_paths: dict[str, pathlib.Path] = {}
            summaries: dict[str, tuple[dict[str, str], dict[str, str], list[float]]] = {}
            for route in order:
                output_path = temp_dir / f"n{n}-{operation}-{shape}-q{threads}-run{pair}-{route}.output"
                log_path = args.output_dir / f"n{n}-{operation}-{shape}-q{threads}-run{pair}-{route}.txt"
                command = (
                    [str(ducc_bin), operation, str(n), str(rows), str(threads), shape, str(fixture), str(output_path)]
                    if route == "ducc-real"
                    else [str(rust_bin), route, operation, str(n), str(rows), str(threads), shape, str(fixture), str(output_path)]
                )
                env = os.environ.copy()
                if route == "ducc-real":
                    env["DUCC0_NUM_THREADS"] = str(threads)
                cpus = args.q1_cpus if threads == 1 else args.q8_cpus
                output, elapsed, wait_percent, host_before, host_after = run_process(
                    command, cpus, env, log_path
                )
                environment, summary, batches = parse_benchmark(output)
                output_paths[route] = output_path
                summaries[route] = environment, summary, batches
                process_rows.append(
                    {
                        "n": n,
                        "rows": rows,
                        "operation": operation,
                        "shape": shape,
                        "threads": threads,
                        "process_pair": pair,
                        "route": route,
                        "cpu_set": cpus,
                        "first_ms": environment["first_ms"],
                        "calibration_ms": environment["calibration_ms"],
                        "batch_calls": environment["batch_calls"],
                        "batches": environment["batches"],
                        "p10_ms": summary["p10_ms"],
                        "median_ms": summary["median_ms"],
                        "p90_ms": summary["p90_ms"],
                        "us_per_row_pair": summary["us_per_row_pair"],
                        "scratch_bytes_per_worker": environment["scratch_bytes_per_worker"],
                        "warm_allocations": summary.get("warm_allocations", "not-reported"),
                        "runtime_avx2": environment.get("runtime_avx2", "not-reported"),
                        "runtime_fma": environment.get("runtime_fma", "not-reported"),
                        "elapsed_process_s": f"{elapsed:.6f}",
                        "host_cpu_psi_some_wait_percent": "" if wait_percent is None else f"{wait_percent:.3f}",
                        "host_before_cpu_psi": str(host_before.get("cpu_pressure", "")).splitlines()[0],
                        "host_after_cpu_psi": str(host_after.get("cpu_pressure", "")).splitlines()[0],
                        "host_before_load_average": json.dumps(host_before.get("load_average", [])),
                        "host_after_load_average": json.dumps(host_after.get("load_average", [])),
                        "host_before_top_processes": json.dumps(host_before.get("top_processes", [])),
                        "host_after_top_processes": json.dumps(host_after.get("top_processes", [])),
                        "log": str(log_path),
                    }
                )
            reference = read_values(output_paths["rust-real"])
            for route, path in output_paths.items():
                relative_l2, scaled_max = compare(reference, read_values(path))
                if relative_l2 > 2.0e-12 or scaled_max > 2.0e-12:
                    raise RuntimeError(
                        f"{case_key} run {pair} {route} differs from independent Rust rows: "
                        f"relative_l2={relative_l2:.4e}, scaled_max={scaled_max:.4e}"
                    )
                validation_rows.append(
                    {
                        "n": n,
                        "rows": rows,
                        "operation": operation,
                        "shape": shape,
                        "threads": threads,
                        "process_pair": pair,
                        "route": route,
                        "relative_l2_vs_rust_real": f"{relative_l2:.12e}",
                        "scaled_max_vs_rust_real": f"{scaled_max:.12e}",
                    }
                )
                path.unlink(missing_ok=True)

            baseline = float(summaries["rust-real"][1]["median_ms"])
            for route, (_, summary, _) in summaries.items():
                candidate = float(summary["median_ms"])
                process_rows[-len(summaries) + list(summaries).index(route)]["speedup_vs_rust_real"] = f"{baseline / candidate:.8f}"
                process_rows[-len(summaries) + list(summaries).index(route)]["saved_us_per_row_pair_vs_rust_real"] = (
                    f"{(baseline - candidate) * 1000.0 / (rows / 2.0):.8f}"
                )
            for path in output_paths.values():
                path.unlink(missing_ok=True)
            completed_runs += 1
            write_rows(args.output_dir / "process-samples.csv", process_rows)
            write_rows(args.output_dir / "validation.csv", validation_rows)
            print(
                f"finished {completed_runs}/{total_runs}: N={n} rows={rows} "
                f"{operation}/{shape} q{threads} process-pair={pair}",
                flush=True,
            )

    write_rows(args.output_dir / "process-samples.csv", process_rows)
    write_rows(args.output_dir / "validation.csv", validation_rows)

    summary_rows: list[dict[str, object]] = []
    grouped: dict[tuple[int, int, str, str, int, str], list[dict[str, object]]] = {}
    for row in process_rows:
        key = (row["n"], row["rows"], row["operation"], row["shape"], row["threads"], row["route"])
        grouped.setdefault(key, []).append(row)
    for key, rows_for_route in grouped.items():
        medians = sorted(float(row["median_ms"]) for row in rows_for_route)
        sorted_pairs = sorted(rows_for_route, key=lambda row: int(row["process_pair"]))
        n, row_count, operation, shape, threads, route = key
        baselines = {
            int(row["process_pair"]): float(row["median_ms"])
            for row in process_rows
            if row["n"] == n
            and row["rows"] == row_count
            and row["operation"] == operation
            and row["shape"] == shape
            and row["threads"] == threads
            and row["route"] == "rust-real"
        }
        ratios = [float(row["median_ms"]) / baselines[int(row["process_pair"])] for row in sorted_pairs]
        summary_rows.append(
            {
                "n": n,
                "rows": row_count,
                "operation": operation,
                "shape": shape,
                "threads": threads,
                "route": route,
                "process_runs": len(medians),
                "median_of_process_medians_ms": medians[len(medians) // 2],
                "process_median_min_ms": medians[0],
                "process_median_max_ms": medians[-1],
                "candidate_over_rust_real_ratio_median": sorted(ratios)[len(ratios) // 2],
                "rust_real_minus_candidate_us_per_pair_median": (
                    sorted(
                        (baselines[int(row["process_pair"])] - float(row["median_ms"]))
                        * 1000.0
                        / (row_count / 2.0)
                        for row in sorted_pairs
                    )[len(sorted_pairs) // 2]
                ),
            }
        )
    summary_fields = list(summary_rows[0])
    with (args.output_dir / "summary.csv").open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=summary_fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(summary_rows)

    config["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    config["host_after"] = host_snapshot()
    config["temporary_output_directory"] = str(temp_dir)
    (args.output_dir / "config.json").write_text(json.dumps(config, indent=2) + "\n")
    (args.output_dir / "host-after.json").write_text(json.dumps(config["host_after"], indent=2) + "\n")


if __name__ == "__main__":
    main()
