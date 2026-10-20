#!/usr/bin/env python3
"""Run calibrated, interleaved Rust/DUCC scalar-analysis kernel trials."""

from __future__ import annotations

import math
import hashlib
import os
import pathlib
import re
import shlex
import statistics
import subprocess
import tempfile
import threading
import time
from collections import defaultdict


ROOT = pathlib.Path(__file__).resolve().parents[1]
DUCC_REVISION = "80388904b8cfc32400ba84af8a6a45aabfe60f86"
RUST_TEST = "legendre::scalar_paired::kernel_bench::benchmark_fused_scalar_analysis_pair_kernel"
RUSTFLAGS = "-C target-cpu=x86-64-v3 -Aunexpected_cfgs"
BATCHES = 30
TARGET_FASTEST_BATCH_US = 35_000.0


def output(command: list[str], *, cwd: pathlib.Path = ROOT, env=None) -> str:
    return subprocess.run(
        command, cwd=cwd, env=env, check=True, text=True, capture_output=True
    ).stdout


def cpu_topology(cpu: int) -> tuple[str, str]:
    base = pathlib.Path(f"/sys/devices/system/cpu/cpu{cpu}/topology")
    package = (base / "physical_package_id").read_text().strip()
    core = (base / "core_id").read_text().strip()
    return package, core


def choose_cpus() -> tuple[int, int]:
    if not hasattr(os, "sched_getaffinity"):
        raise RuntimeError("Linux CPU affinity APIs are required")
    allowed = sorted(os.sched_getaffinity(0))
    if not allowed:
        raise RuntimeError("the current process has no allowed CPUs")
    # Pin every subprocess to one allowed logical CPU. The corresponding
    # package/core pair is checked here but deliberately not printed or saved.
    cpu = allowed[0]
    topology = cpu_topology(cpu)
    monitor_cpu = next(
        (candidate for candidate in allowed[1:] if cpu_topology(candidate) != topology),
        allowed[1] if len(allowed) > 1 else cpu,
    )
    return cpu, monitor_cpu


def parse_schedstat(pid: int) -> tuple[int, int, int] | None:
    try:
        fields = pathlib.Path(f"/proc/{pid}/schedstat").read_text().split()
        return int(fields[0]), int(fields[1]), int(fields[2])
    except (OSError, ValueError, IndexError):
        return None


def parse_context_switches(pid: int) -> tuple[int, int] | None:
    try:
        status = pathlib.Path(f"/proc/{pid}/status").read_text()
    except OSError:
        return None
    values = {}
    for key in ("voluntary_ctxt_switches", "nonvoluntary_ctxt_switches"):
        match = re.search(rf"^{key}:\s+(\d+)$", status, re.MULTILINE)
        if match:
            values[key] = int(match.group(1))
    if len(values) != 2:
        return None
    return values["voluntary_ctxt_switches"], values["nonvoluntary_ctxt_switches"]


def frequency_path(cpu: int) -> pathlib.Path | None:
    cpu_path = pathlib.Path(f"/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_cur_freq")
    if cpu_path.exists() and os.access(cpu_path, os.R_OK):
        return cpu_path

    for policy in sorted(pathlib.Path("/sys/devices/system/cpu/cpufreq").glob("policy*")):
        related = policy / "related_cpus"
        if not related.exists():
            related = policy / "affected_cpus"
        if not related.exists():
            continue
        try:
            policy_cpus = {int(value) for value in related.read_text().split()}
        except (OSError, ValueError):
            continue
        path = policy / "scaling_cur_freq"
        if cpu in policy_cpus and path.exists() and os.access(path, os.R_OK):
            return path
    return None


def run_pinned(
    command: list[str], env: dict[str, str], cpu: int, monitor_cpu: int
) -> tuple[str, dict]:
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
    sched_before = parse_schedstat(process.pid)
    switches_before = parse_context_switches(process.pid)
    freq_file = frequency_path(cpu)
    frequencies: list[int] = []
    sched_records = []
    switch_records = []

    def sample_frequency() -> None:
        os.sched_setaffinity(threading.get_native_id(), {monitor_cpu})
        while process.poll() is None:
            if freq_file is not None:
                try:
                    frequencies.append(int(freq_file.read_text().strip()))
                except (OSError, ValueError):
                    pass
            sched = parse_schedstat(process.pid)
            switches = parse_context_switches(process.pid)
            if sched is not None:
                sched_records.append(sched)
            if switches is not None:
                switch_records.append(switches)
            time.sleep(0.05)

    sampler = threading.Thread(target=sample_frequency, daemon=True)
    sampler.start()
    stdout, stderr = process.communicate()
    sampler.join()
    sched_after = sched_records[-1] if sched_records else parse_schedstat(process.pid)
    switches_after = switch_records[-1] if switch_records else parse_context_switches(process.pid)
    if process.returncode:
        raise RuntimeError(
            f"command failed ({process.returncode}): {shlex.join(command)}\n{stderr}\n{stdout}"
        )

    diagnostics = {
        "frequencies_khz": frequencies,
        "sched_before": sched_before,
        "sched_after": sched_after,
        "switches_before": switches_before,
        "switches_after": switches_after,
    }
    return stdout + "\n" + stderr, diagnostics


def test_environment(base: pathlib.Path, lane_count: int, mode: str, reps=0) -> dict[str, str]:
    env = os.environ.copy()
    env["RUSTSHT_KERNEL_BENCH_BASE"] = str(base)
    env["RUSTSHT_KERNEL_BENCH_LANES"] = str(lane_count)
    env["RUSTSHT_KERNEL_BENCH_MODE"] = mode
    env["RUSTFLAGS"] = RUSTFLAGS
    if mode == "run":
        env["RUSTSHT_KERNEL_BENCH_REPETITIONS"] = str(reps)
        env["RUSTSHT_KERNEL_BENCH_BATCHES"] = str(BATCHES)
    elif mode == "calibrate":
        env["RUSTSHT_KERNEL_BENCH_CALIBRATION_REPETITIONS"] = str(reps)
    return env


def line_fields(line: str, prefix: str) -> dict[str, str] | None:
    if not line.startswith(prefix + " "):
        return None
    return dict(re.findall(r"([A-Za-z0-9_]+)=([^\s]+)", line))


def metric_line(text: str, prefix: str) -> dict[str, str]:
    for line in text.splitlines():
        found = line_fields(line, prefix)
        if found is not None:
            return found
    raise RuntimeError(f"missing {prefix} output:\n{text}")


def summarize(values: list[float]) -> tuple[float, float, float, float, float, float, float]:
    ordered = sorted(values)
    median = statistics.median(ordered)
    deciles = statistics.quantiles(ordered, n=10, method="inclusive")
    q1, _, q3 = statistics.quantiles(ordered, n=4, method="inclusive")
    return median, deciles[0], deciles[8], q1, q3, ordered[0], ordered[-1]


def read_cpu_summary() -> tuple[str, int, int, float]:
    cpuinfo = pathlib.Path("/proc/cpuinfo").read_text(errors="replace")
    model_match = re.search(r"^model name\s*:\s*(.+)$", cpuinfo, re.MULTILINE)
    model = model_match.group(1).strip() if model_match else "unavailable"
    pairs = set()
    for block in cpuinfo.strip().split("\n\n"):
        package = re.search(r"^physical id\s*:\s*(\d+)$", block, re.MULTILINE)
        core = re.search(r"^core id\s*:\s*(\d+)$", block, re.MULTILINE)
        if package and core:
            pairs.add((package.group(1), core.group(1)))
    logical = os.cpu_count() or 0
    physical = len(pairs) or logical
    mem_match = re.search(r"^MemTotal:\s+(\d+) kB$", pathlib.Path("/proc/meminfo").read_text(), re.MULTILINE)
    memory_gib = int(mem_match.group(1)) / 1024 / 1024 if mem_match else 0.0
    return model, physical, logical, memory_gib


def read_cpu_features() -> str:
    cpuinfo = pathlib.Path("/proc/cpuinfo").read_text(errors="replace")
    match = re.search(r"^flags\s*:\s*(.+)$", cpuinfo, re.MULTILINE)
    if not match:
        return "unavailable"
    flags = set(match.group(1).split())
    return ",".join(feature for feature in ("avx2", "fma", "avx512f") if feature in flags) or "none"


def main() -> None:
    ducc_src_value = os.environ.get("DUCC_SRC")
    ducc_git_value = os.environ.get("DUCC_GIT")
    if not ducc_src_value or not ducc_git_value:
        raise SystemExit("set DUCC_SRC and DUCC_GIT to the DUCC source snapshot and revision repository")
    ducc_src = pathlib.Path(ducc_src_value).resolve()
    ducc_git = pathlib.Path(ducc_git_value).resolve()
    verified_ducc_files = (
        "src/ducc0/sht/sht_inner_loop.h",
        "src/ducc0/sht/sht.h",
        "src/ducc0/infra/simd.h",
        "src/ducc0/math/constants.h",
    )
    for relative in verified_ducc_files:
        source_bytes = (ducc_src / relative).read_bytes()
        pinned_bytes = subprocess.run(
            ["git", "-C", str(ducc_git), "show", f"{DUCC_REVISION}:{relative}"],
            check=True,
            capture_output=True,
        ).stdout
        if hashlib.sha256(source_bytes).digest() != hashlib.sha256(pinned_bytes).digest():
            raise SystemExit(f"DUCC source file differs from pinned {DUCC_REVISION}: {relative}")

    cpu, monitor_cpu = choose_cpus()
    model, physical, logical, memory_gib = read_cpu_summary()
    cpu_features = read_cpu_features()
    rustc = output(["rustc", "--version"]).strip()
    cargo = output(["cargo", "--version"]).strip()
    cxx = shlex.split(os.environ.get("CXX", "g++"))
    cxx_version = output(cxx + ["--version"]).splitlines()[0]
    lanes = (64, 128, 256)

    env = os.environ.copy()
    env["RUSTFLAGS"] = RUSTFLAGS
    build = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "rustsht",
            "--release",
            "--lib",
            "benchmark_fused_scalar_analysis_pair_kernel",
            "--no-run",
        ],
        cwd=ROOT,
        env=env,
        check=True,
        text=True,
        capture_output=True,
    )
    executable_match = re.search(r"Executable .* \(([^)]+)\)", build.stdout + build.stderr)
    if not executable_match:
        raise RuntimeError(f"could not locate the release unit-test binary:\n{build.stdout}\n{build.stderr}")
    rust_test_binary = pathlib.Path(executable_match.group(1))
    if not rust_test_binary.is_absolute():
        rust_test_binary = ROOT / rust_test_binary

    with tempfile.TemporaryDirectory(prefix="rustsht-ducc-kernel-batches-") as temporary:
        base = pathlib.Path(temporary) / "analysis-kernel"
        ducc_binary = pathlib.Path(temporary) / "ducc-analysis-kernel-bench"
        source = ROOT / "reports/data/scalar-analysis-pair-grouping-2026-10-19/ducc-ordinary-kernel.cpp"
        subprocess.run(
            cxx
            + [
                "-std=c++17",
                "-O3",
                "-DNDEBUG",
                "-march=x86-64-v3",
                f"-I{ducc_src / 'src'}",
                str(source),
                "-o",
                str(ducc_binary),
            ],
            cwd=ROOT,
            check=True,
        )

        calibration = {}
        calibration_repetitions = {64: 5000, 128: 3000, 256: 1400}
        fixture_paths = {}
        diagnostics = []
        rust_command = [
            str(rust_test_binary),
            "--ignored",
            "--nocapture",
            "--exact",
            RUST_TEST,
            "--test-threads=1",
        ]
        for lane_count in lanes:
            fixture_paths[lane_count] = pathlib.Path(f"{base}-{lane_count}.in")
            stdout, diag = run_pinned(
                rust_command, test_environment(base, lane_count, "prepare"), cpu, monitor_cpu
            )
            if "output_equal=true" not in stdout:
                raise RuntimeError(f"Rust fixture preparation failed:\n{stdout}")
            if "RUST_KERNEL_ARCH V3(" not in stdout:
                raise RuntimeError(f"Rust kernel did not dispatch Pulp V3:\n{stdout}")
            diagnostics.append(("rust-prepare", lane_count, diag))

            rust_stdout, rust_diag = run_pinned(
                rust_command,
                test_environment(
                    base, lane_count, "calibrate", calibration_repetitions[lane_count]
                ),
                cpu,
                monitor_cpu,
            )
            rust_cal = metric_line(rust_stdout, "RUST_CALIBRATION")
            ducc_stdout, ducc_diag = run_pinned(
                [
                    str(ducc_binary),
                    str(fixture_paths[lane_count]),
                    "--calibrate",
                    str(calibration_repetitions[lane_count]),
                ],
                env,
                cpu,
                monitor_cpu,
            )
            ducc_cal = metric_line(ducc_stdout, "DUCC_CALIBRATION")
            if int(ducc_cal["vlen"]) != 4:
                raise RuntimeError(f"DUCC did not execute four-lane f64 AVX2: {ducc_cal}")
            if int(rust_cal["repetitions"]) != calibration_repetitions[lane_count]:
                raise RuntimeError("Rust calibration used an unexpected batch size")
            if int(ducc_cal["repetitions"]) != calibration_repetitions[lane_count]:
                raise RuntimeError("DUCC calibration used an unexpected batch size")
            calibration[lane_count] = {
                "rust_checked": float(rust_cal["checked_us"]),
                "rust_unchecked": float(rust_cal["unchecked_us"]),
                "ducc": float(ducc_cal["reset_inclusive_us"]),
            }
            diagnostics.extend(
                [("rust-calibrate", lane_count, rust_diag), ("ducc-calibrate", lane_count, ducc_diag)]
            )

        repetitions = {}
        for lane_count, samples in calibration.items():
            calibration_ms = {
                key: value * calibration_repetitions[lane_count] / 1000.0
                for key, value in samples.items()
            }
            if any(not (20.0 <= value <= 100.0) for value in calibration_ms.values()):
                raise RuntimeError(
                    f"calibration batch outside 20-100 ms for lane={lane_count}: {calibration_ms}"
                )
            fastest = min(samples.values())
            slowest = max(samples.values())
            lower = math.ceil(20_000.0 / fastest)
            upper = math.floor(95_000.0 / slowest)
            if lower > upper:
                raise RuntimeError(
                    f"cannot fit all lane={lane_count} variants into 20-100 ms batches: "
                    f"calibrated range {fastest:.3f}..{slowest:.3f} us/call"
                )
            chosen = min(max(round(TARGET_FASTEST_BATCH_US / fastest), lower), upper)
            repetitions[lane_count] = chosen

        observations = defaultdict(list)
        numeric_errors = []
        batch_durations = []
        for lane_count in lanes:
            for invocation in range(3):
                order = ("rust", "ducc") if invocation % 2 == 0 else ("ducc", "rust")
                for backend in order:
                    if backend == "rust":
                        run_env = test_environment(
                            base, lane_count, "run", repetitions[lane_count]
                        )
                        command = rust_command
                    else:
                        run_env = env
                        command = [
                            str(ducc_binary),
                            str(fixture_paths[lane_count]),
                            str(repetitions[lane_count]),
                            str(BATCHES),
                        ]
                    stdout, diag = run_pinned(command, run_env, cpu, monitor_cpu)
                    diagnostics.append((f"{backend}-run-{invocation + 1}", lane_count, diag))
                    expected_prefix = "RUST_BATCH" if backend == "rust" else "DUCC_BATCH"
                    parsed_count = 0
                    for line in stdout.splitlines():
                        fields = line_fields(line, expected_prefix)
                        if fields is None:
                            continue
                        parsed_count += 1
                        mode = fields["mode"]
                        variant = fields.get("variant", "ducc")
                        reps = int(fields["repetitions"])
                        if reps != repetitions[lane_count]:
                            raise RuntimeError("backend used a different repetition count")
                        if mode == "kernel_only":
                            batch_us = float(fields["kernel_us"])
                        else:
                            batch_us = float(fields["batch_us"])
                        batch_durations.append(batch_us / 1000.0)
                        per_call_us = batch_us / reps
                        observations[(lane_count, mode, variant, backend)].append(
                            (per_call_us, invocation + 1, batch_us / 1000.0)
                        )
                    expected_lines = BATCHES * (4 if backend == "rust" else 2)
                    if parsed_count != expected_lines:
                        raise RuntimeError(
                            f"expected {expected_lines} batch records from {backend}, got {parsed_count}\n{stdout}"
                        )
                    if backend == "ducc":
                        error = metric_line(stdout, "DUCC_KERNEL")
                        numeric_errors.append(
                            (
                                lane_count,
                                float(error["coefficient_max_scaled_diff"]),
                                float(error["output_relative_l2"]),
                                float(error["output_max_abs"]),
                            )
                        )
                    elif "output_equal=true" not in stdout:
                        raise RuntimeError(f"checked/unchecked outputs differ:\n{stdout}")

        print(
            f"ENV cpu={model}; physical_cores={physical}; logical_cpus={logical}; "
            f"installed_ram_gib={memory_gib:.1f}; cpu_features={cpu_features}; "
            f"pinned=one allowed logical CPU on one physical core"
        )
        print(
            f"BUILD rust={rustc}; {cargo}; RUSTFLAGS='{RUSTFLAGS}'; "
            f"DUCC={DUCC_REVISION}; cxx={cxx_version}; CXXFLAGS='-O3 -DNDEBUG -march=x86-64-v3'"
        )
        print("DISPATCH Rust=Pulp V3 AVX2/FMA; DUCC=compiled x86-64-v3; both use 4 f64 lanes")
        print("TRIAL_ORDER per lane: Rust/DUCC, DUCC/Rust, Rust/DUCC")
        for lane_count in lanes:
            calibration_batch_ms = {
                key: value * calibration_repetitions[lane_count] / 1000.0
                for key, value in calibration[lane_count].items()
            }
            print(
                f"CALIBRATION lanes={lane_count} calibration_repetitions={calibration_repetitions[lane_count]} "
                f"run_repetitions={repetitions[lane_count]} "
                + " ".join(f"{key}={value:.3f}us" for key, value in calibration[lane_count].items())
            )
            print(
                "CALIBRATION_BATCH_MS "
                + " ".join(f"{key}={value:.2f}" for key, value in calibration_batch_ms.items())
            )

        print("TIMINGS are per kernel call; values summarize 90 batches (3 processes x 30).")
        for lane_count in lanes:
            for mode in ("kernel_only", "reset_inclusive"):
                print(f"{mode} lanes={lane_count}")
                variants = ("checked", "unchecked", "ducc")
                medians = {}
                for variant in variants:
                    backend = "ducc" if variant == "ducc" else "rust"
                    samples = observations[(lane_count, mode, variant, backend)]
                    per_call = [sample[0] for sample in samples]
                    median, p10, p90, q1, q3, low, high = summarize(per_call)
                    medians[variant] = median
                    process_medians = []
                    for invocation in (1, 2, 3):
                        process_medians.append(
                            statistics.median(
                                sample[0] for sample in samples if sample[1] == invocation
                            )
                        )
                    print(
                        f"  {variant}: {median:.3f} us/op; p10/p90={p10:.3f}..{p90:.3f}; "
                        f"IQR={q1:.3f}..{q3:.3f}; "
                        f"range={low:.3f}..{high:.3f}; process_medians="
                        + ",".join(f"{value:.3f}" for value in process_medians)
                    )
                print(
                    f"  ratios checked/DUCC={medians['checked'] / medians['ducc']:.3f} "
                    f"unchecked/DUCC={medians['unchecked'] / medians['ducc']:.3f}"
                )

        min_batch = min(batch_durations)
        max_batch = max(batch_durations)
        outside = sum(not (20.0 <= sample <= 100.0) for sample in batch_durations)
        print(
            f"BATCH_DURATIONS measured_kernel_or_reset_ms range={min_batch:.2f}..{max_batch:.2f}; "
            f"outside_20_100ms={outside}/{len(batch_durations)}"
        )
        for lane_count, coefficient_error, relative_l2, max_abs in numeric_errors:
            print(
                f"NUMERICS lanes={lane_count} coefficient_max_scaled_diff={coefficient_error:.3e} "
                f"output_relative_l2={relative_l2:.3e} output_max_abs={max_abs:.3e}"
            )

        frequencies = [
            value
            for _, _, diag in diagnostics
            for value in diag["frequencies_khz"]
        ]
        if frequencies:
            ordered = sorted(frequencies)
            print(
                f"HOST_FREQUENCY available_samples={len(ordered)} "
                f"median_ghz={statistics.median(ordered) / 1e6:.3f} "
                f"range_ghz={ordered[0] / 1e6:.3f}..{ordered[-1] / 1e6:.3f}"
            )
        else:
            print("HOST_FREQUENCY unavailable from readable Linux cpufreq sysfs")

        switch_deltas = []
        wait_fractions = []
        for _, _, diag in diagnostics:
            before, after = diag["switches_before"], diag["switches_after"]
            if before and after:
                switch_deltas.append((after[0] - before[0], after[1] - before[1]))
            before_sched, after_sched = diag["sched_before"], diag["sched_after"]
            if before_sched and after_sched:
                runtime = after_sched[0] - before_sched[0]
                wait = after_sched[1] - before_sched[1]
                if runtime + wait > 0:
                    wait_fractions.append(100.0 * wait / (runtime + wait))
        if switch_deltas:
            involuntary = [item[1] for item in switch_deltas]
            print(
                f"HOST_SCHED available_process_samples={len(switch_deltas)} "
                f"median_involuntary_switches={statistics.median(involuntary):.1f} "
                f"max_involuntary_switches={max(involuntary)}"
            )
        else:
            print("HOST_SCHED context-switch counters unavailable")
        if wait_fractions:
            print(
                f"HOST_RUNQUEUE median_wait_fraction={statistics.median(wait_fractions):.3f}% "
                f"max_wait_fraction={max(wait_fractions):.3f}%"
            )
        else:
            print("HOST_RUNQUEUE schedstat counters unavailable")

        for label, lane_count, diag in diagnostics:
            if "-run-" not in label:
                continue
            backend, _, invocation = label.partition("-run-")
            process_frequencies = diag["frequencies_khz"]
            frequency_summary = (
                f"freq_ghz={statistics.median(process_frequencies) / 1e6:.3f} "
                f"range={min(process_frequencies) / 1e6:.3f}..{max(process_frequencies) / 1e6:.3f}"
                if process_frequencies
                else "freq_ghz=unavailable"
            )
            before_switches, after_switches = diag["switches_before"], diag["switches_after"]
            context_summary = (
                f"involuntary_switches={after_switches[1] - before_switches[1]}"
                if before_switches and after_switches
                else "context_switches=unavailable"
            )
            before_sched, after_sched = diag["sched_before"], diag["sched_after"]
            if before_sched and after_sched:
                runtime = after_sched[0] - before_sched[0]
                wait = after_sched[1] - before_sched[1]
                wait_summary = (
                    f"runqueue_wait_fraction={100.0 * wait / (runtime + wait):.3f}%"
                    if runtime + wait > 0
                    else "runqueue_wait_fraction=unavailable"
                )
            else:
                wait_summary = "runqueue_wait_fraction=unavailable"
            print(
                f"HOST_PROCESS backend={backend} lanes={lane_count} invocation={invocation} "
                f"{frequency_summary} {context_summary} {wait_summary}"
            )


if __name__ == "__main__":
    main()
