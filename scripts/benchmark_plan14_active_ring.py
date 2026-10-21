#!/usr/bin/env python3
"""Matched T127 q1 active-ring scalar-analysis kernel comparison."""

from __future__ import annotations

import argparse
import csv
import hashlib
import os
import pathlib
import re
import shlex
import statistics
import subprocess
import threading
import time


ROOT = pathlib.Path(__file__).resolve().parents[1]
DUCC_REVISION = "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
RUST_TEST_CAPTURE = "transform::tests::capture_plan14_t127_active_ring_kernel_input"
RUST_TEST_BENCH = "legendre::scalar_paired::kernel_bench::benchmark_plan14_t127_active_ring_route"
SOURCE_FILES = (
    "src/ducc0/sht/sht_inner_loop.h",
    "src/ducc0/sht/sht.h",
    "src/ducc0/infra/simd.h",
    "src/ducc0/math/constants.h",
)
DRIVER_SOURCE = ROOT / "reports/data/scalar-analysis-pair-grouping-2026-10-19/ducc-ordinary-kernel.cpp"
KERNEL_SOURCE = ROOT / "reports/data/scalar-analysis-pair-grouping-2026-10-19/ducc-kernel-fast.cpp"
RUSTFLAGS = "-C target-cpu=x86-64-v3 -Aunexpected_cfgs"
DUCC_DRIVER_FLAGS = [
    "-O3",
    "-DNDEBUG",
    "-fno-fast-math",
    "-ffp-contract=fast",
    "-march=x86-64-v3",
]
DUCC_KERNEL_FLAGS = [
    "-O3",
    "-DNDEBUG",
    "-ffast-math",
    "-ffp-contract=fast",
    "-march=x86-64-v3",
]
DUCC_REPETITIONS = 35_000
BATCHES = 30
PROCESS_PAIRS = 3


def command_output(command: list[str], **kwargs) -> str:
    return subprocess.run(command, check=True, text=True, capture_output=True, **kwargs).stdout


def choose_cpus() -> tuple[int, int]:
    allowed = sorted(os.sched_getaffinity(0))
    if not allowed:
        raise RuntimeError("no allowed CPUs")

    def topology(cpu: int) -> tuple[str, str]:
        directory = pathlib.Path(f"/sys/devices/system/cpu/cpu{cpu}/topology")
        return (
            (directory / "physical_package_id").read_text().strip(),
            (directory / "core_id").read_text().strip(),
        )

    cpu = allowed[0]
    other_core = next((candidate for candidate in allowed[1:] if topology(candidate) != topology(cpu)), None)
    if other_core is None:
        raise RuntimeError("a second physical core is required for frequency sampling")
    return cpu, other_core


def frequency_path(cpu: int) -> pathlib.Path | None:
    direct = pathlib.Path(f"/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_cur_freq")
    if direct.exists() and os.access(direct, os.R_OK):
        return direct
    for policy in sorted(pathlib.Path("/sys/devices/system/cpu/cpufreq").glob("policy*")):
        related = policy / "related_cpus"
        if not related.exists():
            related = policy / "affected_cpus"
        if not related.exists():
            continue
        try:
            cpus = {int(value) for value in related.read_text().split()}
        except (OSError, ValueError):
            continue
        candidate = policy / "scaling_cur_freq"
        if cpu in cpus and candidate.exists() and os.access(candidate, os.R_OK):
            return candidate
    return None


def cpu_pressure_avg10() -> str:
    try:
        line = pathlib.Path("/proc/pressure/cpu").read_text().splitlines()[0]
        match = re.search(r"avg10=([0-9.]+)", line)
        return match.group(1) if match else "unavailable"
    except (OSError, IndexError):
        return "unavailable"


def proc_stat(pid: int) -> tuple[int, int, int] | None:
    try:
        values = pathlib.Path(f"/proc/{pid}/schedstat").read_text().split()
        return tuple(map(int, values[:3]))  # type: ignore[return-value]
    except (OSError, ValueError):
        return None


def proc_switches(pid: int) -> tuple[int, int] | None:
    try:
        status = pathlib.Path(f"/proc/{pid}/status").read_text()
    except OSError:
        return None
    values = [
        re.search(rf"^{name}:\s+(\d+)$", status, re.MULTILINE)
        for name in ("voluntary_ctxt_switches", "nonvoluntary_ctxt_switches")
    ]
    if not all(values):
        return None
    return int(values[0].group(1)), int(values[1].group(1))


def run_pinned(command: list[str], env: dict[str, str], cpu: int, monitor_cpu: int) -> tuple[str, dict]:
    def pin_child() -> None:
        os.sched_setaffinity(0, {cpu})

    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        preexec_fn=pin_child,
    )
    frequency_file = frequency_path(cpu)
    frequencies: list[int] = []
    sched_samples: list[tuple[int, int, int]] = []
    switch_samples: list[tuple[int, int]] = []
    sampler_errors: list[str] = []

    def sample() -> None:
        try:
            os.sched_setaffinity(threading.get_native_id(), {monitor_cpu})
            while process.poll() is None:
                if frequency_file is not None:
                    try:
                        frequencies.append(int(frequency_file.read_text().strip()))
                    except (OSError, ValueError):
                        pass
                scheduler = proc_stat(process.pid)
                switches = proc_switches(process.pid)
                if scheduler is not None:
                    sched_samples.append(scheduler)
                if switches is not None:
                    switch_samples.append(switches)
                time.sleep(0.05)
        except OSError as error:
            sampler_errors.append(str(error))

    before_sched = proc_stat(process.pid)
    before_switches = proc_switches(process.pid)
    before_pressure = cpu_pressure_avg10()
    sampler = threading.Thread(target=sample, daemon=True)
    sampler.start()
    stdout, stderr = process.communicate()
    sampler.join()
    after_pressure = cpu_pressure_avg10()
    if process.returncode:
        raise RuntimeError(
            f"command failed ({process.returncode}): {shlex.join(command)}\n{stderr}\n{stdout}"
        )
    diagnostics = {
        "frequency_samples_khz": frequencies,
        "sched_before": before_sched,
        "sched_after": sched_samples[-1] if sched_samples else None,
        "switches_before": before_switches,
        "switches_after": switch_samples[-1] if switch_samples else None,
        "pressure_before": before_pressure,
        "pressure_after": after_pressure,
        "sampler_errors": sampler_errors,
    }
    return stdout + "\n" + stderr, diagnostics


def fields(line: str, prefix: str) -> dict[str, str] | None:
    if not line.startswith(prefix + " "):
        return None
    return dict(re.findall(r"([A-Za-z0-9_]+)=([^\s]+)", line))


def p10_median_p90(values: list[float]) -> tuple[float, float, float]:
    ordered = sorted(values)
    quantiles = statistics.quantiles(ordered, n=10, method="inclusive")
    return quantiles[0], statistics.median(ordered), quantiles[8]


def source_sha(ducc_src: pathlib.Path, ducc_git: pathlib.Path, relative: str) -> str:
    source_bytes = (ducc_src / relative).read_bytes()
    pinned_bytes = subprocess.run(
        ["git", "-C", str(ducc_git), "show", f"{DUCC_REVISION}:{relative}"],
        check=True,
        capture_output=True,
    ).stdout
    source_hash = hashlib.sha256(source_bytes).hexdigest()
    pinned_hash = hashlib.sha256(pinned_bytes).hexdigest()
    if source_hash != pinned_hash:
        raise RuntimeError(f"DUCC file differs from pinned revision: {relative}")
    return source_hash


def build_rust_test(rustflags: str) -> pathlib.Path:
    env = os.environ.copy()
    env["RUSTFLAGS"] = rustflags
    result = subprocess.run(
        ["cargo", "test", "-p", "rustsht", "--release", "--lib", RUST_TEST_BENCH, "--no-run"],
        cwd=ROOT,
        env=env,
        check=True,
        text=True,
        capture_output=True,
    )
    match = re.search(r"Executable .* \(([^)]+)\)", result.stdout + result.stderr)
    if not match:
        raise RuntimeError(f"could not find the release test binary:\n{result.stdout}\n{result.stderr}")
    binary = pathlib.Path(match.group(1))
    return binary if binary.is_absolute() else ROOT / binary


def compile_ducc(ducc_src: pathlib.Path, cxx: list[str], output: pathlib.Path) -> tuple[str, str]:
    driver_object = output / "ducc-driver.o"
    kernel_object = output / "ducc-kernel.o"
    binary = output / "ducc-analysis-kernel-bench"
    include = f"-I{ducc_src / 'src'}"
    driver_command = cxx + [
        "-std=c++17",
        *DUCC_DRIVER_FLAGS,
        include,
        "-c",
        str(DRIVER_SOURCE),
        "-o",
        str(driver_object),
    ]
    kernel_command = cxx + [
        "-std=c++17",
        *DUCC_KERNEL_FLAGS,
        include,
        "-c",
        str(KERNEL_SOURCE),
        "-o",
        str(kernel_object),
    ]
    link_command = cxx + [str(driver_object), str(kernel_object), "-o", str(binary)]
    for command in (driver_command, kernel_command, link_command):
        subprocess.run(command, cwd=ROOT, check=True)
    return str(binary), command_output(cxx + ["--version"]).splitlines()[0]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--ducc-src", type=pathlib.Path, required=True)
    parser.add_argument("--ducc-git", type=pathlib.Path, required=True)
    parser.add_argument("--output-dir", type=pathlib.Path, required=True)
    parser.add_argument("--cxx", default=os.environ.get("CXX", "g++"))
    parser.add_argument("--rustflags", default=RUSTFLAGS)
    parser.add_argument("--route", choices=("callback", "fused"), default="callback")
    parser.add_argument("--rust-repetitions", type=int)
    args = parser.parse_args()
    rust_repetitions = args.rust_repetitions or (6_000 if args.route == "callback" else 24_000)
    ducc_src = args.ducc_src.resolve()
    ducc_git = args.ducc_git.resolve()
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    hashes = {relative: source_sha(ducc_src, ducc_git, relative) for relative in SOURCE_FILES}
    cpu, monitor_cpu = choose_cpus()
    cxx = shlex.split(args.cxx)
    rust_test = build_rust_test(args.rustflags)
    ducc_binary, cxx_version = compile_ducc(ducc_src, cxx, output_dir)
    fixture = output_dir / "t127-m31-active.in"
    capture_env = os.environ.copy()
    capture_env["RUSTSHT_PLAN14_KERNEL_INPUT"] = str(fixture)
    capture_env["RUSTFLAGS"] = args.rustflags
    capture_output, _ = run_pinned(
        [str(rust_test), "--ignored", "--nocapture", "--exact", RUST_TEST_CAPTURE, "--test-threads=1"],
        capture_env,
        cpu,
        monitor_cpu,
    )
    fixture_match = re.search(r"PLAN14_KERNEL_FIXTURE .*active_lanes=(\d+)", capture_output)
    if (
        fixture_match is None
        or int(fixture_match.group(1)) != 64
        or "previous_production_route=paired_callback" not in capture_output
    ):
        raise RuntimeError(f"unexpected T127 active-ring packing:\n{capture_output}")

    rust_env = os.environ.copy()
    rust_env.update(
        {
            "RUSTSHT_PLAN14_KERNEL_INPUT": str(fixture),
            "RUSTSHT_PLAN14_CALLBACK_REPETITIONS": str(rust_repetitions),
            "RUSTSHT_PLAN14_CALLBACK_BATCHES": str(BATCHES),
            "RUSTSHT_PLAN14_ROUTE": args.route,
        }
    )
    ducc_env = os.environ.copy()
    process_records: list[dict] = []
    batch_records: list[dict] = []
    run_sequence = ["rust", "ducc", "ducc", "rust", "rust", "ducc"]
    for invocation, backend in enumerate(run_sequence, start=1):
        if backend == "rust":
            command = [
                str(rust_test),
                "--ignored",
                "--nocapture",
                "--exact",
                RUST_TEST_BENCH,
                "--test-threads=1",
            ]
        else:
            command = [
                str(ducc_binary),
                str(fixture),
                str(DUCC_REPETITIONS),
                str(BATCHES),
                "--calc-map2alm",
            ]
        stdout, diagnostics = run_pinned(command, rust_env if backend == "rust" else ducc_env, cpu, monitor_cpu)
        log_path = output_dir / f"{backend}-{invocation}.log"
        log_path.write_text(stdout)
        if backend == "rust":
            if "PLAN14_RUST_ARCH V3(" not in stdout or f"route={args.route}" not in stdout:
                raise RuntimeError(f"Rust did not use the expected Pulp V3 {args.route} route in run {invocation}")
            if "lanes=64" not in stdout:
                raise RuntimeError(f"Rust active block validation failed in run {invocation}")
            records = [
                fields(line, "PLAN14_RUST_BATCH")
                for line in stdout.splitlines()
                if line.startswith("PLAN14_RUST_BATCH ")
            ]
            records = [record for record in records if record is not None]
            if len(records) != BATCHES:
                raise RuntimeError(f"Rust run {invocation} emitted {len(records)} batches")
            kernel_us = [float(record["kernel_us"]) / rust_repetitions for record in records]
            reset_inclusive_us = [float(record["reset_inclusive_us"]) / rust_repetitions for record in records]
            batch_durations_ms = [float(record["reset_inclusive_us"]) / 1000.0 for record in records]
            if not all(20.0 <= duration <= 100.0 for duration in batch_durations_ms):
                raise RuntimeError(f"Rust batch outside 20-100ms in run {invocation}: {batch_durations_ms}")
            for index, record in enumerate(records):
                batch_records.append(
                    {
                        "backend": backend,
                        "process": invocation,
                        "batch": index,
                        "kernel_stage_us_per_call": kernel_us[index],
                        "reset_inclusive_us_per_call": reset_inclusive_us[index],
                        "reset_us_per_call": float(record["reset_us"]) / rust_repetitions,
                        "batch_ms": batch_durations_ms[index],
                    }
                )
        else:
            if "DUCC_WARMUP lanes=64" not in stdout or "DUCC_KERNEL m=31 lmax=127 lanes=64" not in stdout or "calc_map2alm=1" not in stdout:
                raise RuntimeError(f"DUCC did not execute the expected AVX2 kernel in run {invocation}")
            records_kernel: list[dict[str, str]] = []
            records_reset: list[dict[str, str]] = []
            for line in stdout.splitlines():
                kernel_record = fields(line, "DUCC_BATCH")
                if kernel_record is None:
                    continue
                if kernel_record["mode"] == "calc_map2alm":
                    records_kernel.append(kernel_record)
                elif kernel_record["mode"] == "reset_inclusive":
                    records_reset.append(kernel_record)
            if len(records_kernel) != BATCHES or len(records_reset) != BATCHES:
                raise RuntimeError(f"DUCC run {invocation} emitted wrong batch count")
            kernel_us = [float(record["kernel_us"]) / DUCC_REPETITIONS for record in records_kernel]
            reset_inclusive_us = [float(record["batch_us"]) / DUCC_REPETITIONS for record in records_reset]
            batch_durations_ms = [float(record["batch_us"]) / 1000.0 for record in records_reset]
            if not all(20.0 <= duration <= 100.0 for duration in batch_durations_ms):
                raise RuntimeError(f"DUCC batch outside 20-100ms in run {invocation}: {batch_durations_ms}")
            numeric = next(
                fields(line, "DUCC_KERNEL")
                for line in stdout.splitlines()
                if line.startswith("DUCC_KERNEL ")
            )
            if float(numeric["coefficient_max_scaled_diff"]) > 2e-15 or float(numeric["output_relative_l2"]) > 5e-12:
                raise RuntimeError(f"Rust/DUCC numerical comparison failed: {numeric}")
            for index, record in enumerate(records_kernel):
                batch_records.append(
                    {
                        "backend": backend,
                        "process": invocation,
                        "batch": index,
                        "kernel_stage_us_per_call": kernel_us[index],
                        "reset_inclusive_us_per_call": reset_inclusive_us[index],
                        "reset_us_per_call": float(records_reset[index]["reset_us"]) / DUCC_REPETITIONS,
                        "batch_ms": batch_durations_ms[index],
                    }
                )
        all_kernel = [row["kernel_stage_us_per_call"] for row in batch_records if row["process"] == invocation]
        all_reset = [row["reset_inclusive_us_per_call"] for row in batch_records if row["process"] == invocation]
        k_p10, k_median, k_p90 = p10_median_p90(all_kernel)
        r_p10, r_median, r_p90 = p10_median_p90(all_reset)
        frequencies = diagnostics["frequency_samples_khz"]
        process_records.append(
            {
                "invocation": invocation,
                "backend": backend,
                "order": "-".join(run_sequence[max(0, invocation - 2) : invocation]),
                "cpu": cpu,
                "monitor_cpu": monitor_cpu,
                "kernel_p10_us": k_p10,
                "kernel_median_us": k_median,
                "kernel_p90_us": k_p90,
                "reset_inclusive_p10_us": r_p10,
                "reset_inclusive_median_us": r_median,
                "reset_inclusive_p90_us": r_p90,
                "batch_ms_min": min(row["batch_ms"] for row in batch_records if row["process"] == invocation),
                "batch_ms_max": max(row["batch_ms"] for row in batch_records if row["process"] == invocation),
                "frequency_p10_khz": statistics.quantiles(frequencies, n=10, method="inclusive")[0] if frequencies else "",
                "frequency_median_khz": statistics.median(frequencies) if frequencies else "",
                "frequency_p90_khz": statistics.quantiles(frequencies, n=10, method="inclusive")[8] if frequencies else "",
                "cpu_psi_avg10_before": diagnostics["pressure_before"],
                "cpu_psi_avg10_after": diagnostics["pressure_after"],
                "schedstat_before": diagnostics["sched_before"],
                "schedstat_after": diagnostics["sched_after"],
                "context_switches_before": diagnostics["switches_before"],
                "context_switches_after": diagnostics["switches_after"],
            }
        )

    with (output_dir / "batches.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(batch_records[0]))
        writer.writeheader()
        writer.writerows(batch_records)
    with (output_dir / "processes.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(process_records[0]))
        writer.writeheader()
        writer.writerows(process_records)

    source_commit = command_output(["git", "-C", str(ROOT), "rev-parse", "HEAD"]).strip()
    rustc = command_output(["rustc", "-Vv"]).strip()
    metadata = [
        f"RustSHT source HEAD: {source_commit}",
        f"DUCC revision: {DUCC_REVISION}",
        f"DUCC C++ compiler: {cxx_version}",
        f"DUCC driver flags: {' '.join(DUCC_DRIVER_FLAGS)}",
        f"DUCC kernel flags: {' '.join(DUCC_KERNEL_FLAGS)}",
        f"Rust flags: {args.rustflags}",
        f"Rust test binary: {rust_test}",
        f"Rust toolchain:\n{rustc}",
        f"Pinned DUCC source SHA256:\n" + "\n".join(f"{name}: {value}" for name, value in hashes.items()),
        "Geometry: f64 scalar GL analysis, lmax=127, mmax=31, ntheta=128, nphi=63, phi0=0.23, order=31.",
        "Active block: 64 reflected pairs, 16 four-lane AVX2 vectors, 49 paired coefficient slots / 97 degree outputs.",
        "Input: deterministic map FFT, GL ring weights, analysis phase, north/south packing, cosine-weighted opposite spectrum, and sine/cosine geometry. Each timed path initializes its recurrence state.",
        f"Timed operation: Rust T127 {args.route} route with paired recurrence, accumulation and coefficient finalization; DUCC native init_lambda plus calc_map2alm, including iter_to_ieee, scaled-prefix advancement, map2alm_kernel and its coefficient adjustment. Both reset output coefficients per operation.",
        f"Protocol: {PROCESS_PAIRS} interleaved process pairs, {BATCHES} batches per process, {rust_repetitions} Rust and {DUCC_REPETITIONS} DUCC operations per batch, CPU {cpu} pinned; monitor CPU {monitor_cpu}.",
        "Run order: Rust, DUCC, DUCC, Rust, Rust, DUCC.",
        f"Rust route: {args.route}",
        f"Fixture capture: {capture_output.strip()}",
    ]
    (output_dir / "metadata.txt").write_text("\n".join(metadata) + "\n")
    print("Results:", output_dir)
    print("Process summaries:")
    for row in process_records:
        print(
            f"{row['backend']} process {row['invocation']}: kernel stage "
            f"{row['kernel_p10_us']:.3f}/{row['kernel_median_us']:.3f}/{row['kernel_p90_us']:.3f} us; "
            f"reset-inclusive median {row['reset_inclusive_median_us']:.3f} us"
        )
    rust_medians = [row["kernel_median_us"] for row in process_records if row["backend"] == "rust"]
    ducc_medians = [row["kernel_median_us"] for row in process_records if row["backend"] == "ducc"]
    ratio = statistics.median(rust_medians) / statistics.median(ducc_medians)
    print(f"Paired-process median Rust/DUCC kernel ratio: {ratio:.4f}x")


if __name__ == "__main__":
    main()
