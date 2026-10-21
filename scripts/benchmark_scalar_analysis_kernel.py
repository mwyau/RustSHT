#!/usr/bin/env python3
"""Run calibrated, interleaved Rust/DUCC scalar-analysis kernel trials."""

from __future__ import annotations

import hashlib
import math
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
DUCC_REVISION = os.environ.get(
    "RUSTSHT_DUCC_REVISION", "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
)
RUST_TEST = "legendre::scalar_paired::kernel_bench::benchmark_fused_scalar_analysis_pair_kernel"
RUSTFLAGS = os.environ.get(
    "RUSTSHT_KERNEL_BENCH_RUSTFLAGS",
    # The default must measure the checked production route. Experimental
    # range-check variants are selected only through the explicit override.
    "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
)
DUCC_KERNEL_CXXFLAGS = shlex.split(
    os.environ.get(
        "RUSTSHT_KERNEL_DUCC_KERNEL_CXXFLAGS",
        "-O3 -DNDEBUG -ffast-math -ffp-contract=fast -march=x86-64-v3",
    )
)
DUCC_DRIVER_CXXFLAGS = shlex.split(
    os.environ.get(
        "RUSTSHT_KERNEL_DUCC_DRIVER_CXXFLAGS",
        "-O3 -DNDEBUG -fno-fast-math -ffp-contract=fast -march=x86-64-v3",
    )
)
# Spread samples across more independent processes while keeping several
# calibrated batches in each process. This makes process-level timing regimes
# visible without relying on one long invocation per backend.
BATCHES = 6
INVOCATIONS = 10
TARGET_FASTEST_BATCH_US = 50_000.0
COMMIT_CALIBRATION_REPETITIONS = 100_000
DUCC_TIMED_WARMUPS = 3
MAX_PROCESS_RANGE_OVER_MEDIAN = 0.15


def validate_ducc_flag_contract(compiler: list[str]) -> None:
    """Reject builds that do not preserve the measured strict/fast split."""
    kernel_flags = [*compiler[1:], *DUCC_KERNEL_CXXFLAGS]
    driver_flags = [*compiler[1:], *DUCC_DRIVER_CXXFLAGS]
    required_kernel = {
        "-O3",
        "-ffast-math",
        "-ffp-contract=fast",
        "-march=x86-64-v3",
    }
    required_driver = {
        "-O3",
        "-fno-fast-math",
        "-ffp-contract=fast",
        "-march=x86-64-v3",
    }
    if not required_kernel.issubset(kernel_flags):
        raise RuntimeError(
            "DUCC kernel flags must include -O3 -ffast-math "
            "-ffp-contract=fast -march=x86-64-v3"
        )
    if not required_driver.issubset(driver_flags):
        raise RuntimeError(
            "DUCC validation driver flags must include -O3 -fno-fast-math "
            "-ffp-contract=fast -march=x86-64-v3"
        )
    for role, flags in (("kernel", kernel_flags), ("driver", driver_flags)):
        optimization_flags = [flag for flag in flags if flag.startswith("-O")]
        if optimization_flags != ["-O3"]:
            raise RuntimeError(
                f"unexpected optimization flags for DUCC {role}: {optimization_flags}"
            )
        march_flags = [flag for flag in flags if flag.startswith("-march=")]
        if march_flags != ["-march=x86-64-v3"]:
            raise RuntimeError(f"unexpected ISA flags for DUCC {role}: {march_flags}")
        contract_flags = [flag for flag in flags if flag.startswith("-ffp-contract=")]
        if contract_flags != ["-ffp-contract=fast"]:
            raise RuntimeError(f"unexpected contraction flags for DUCC {role}: {contract_flags}")
    if "-fno-fast-math" in kernel_flags:
        raise RuntimeError("DUCC numerical kernel must retain -ffast-math")
    if "-ffast-math" in driver_flags or "-Ofast" in driver_flags:
        raise RuntimeError("DUCC validation driver must retain strict floating-point semantics")

if BATCHES * INVOCATIONS < 30:
    raise RuntimeError("the benchmark requires at least 30 batches per backend")


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
        task_paths = pathlib.Path(f"/proc/{pid}/task").glob("*/schedstat")
        totals = [0, 0, 0]
        found = False
        for task_path in task_paths:
            fields = task_path.read_text().split()
            for index in range(3):
                totals[index] += int(fields[index])
            found = True
        if found:
            return tuple(totals)
    except (OSError, ValueError, IndexError):
        pass
    try:
        fields = pathlib.Path(f"/proc/{pid}/schedstat").read_text().split()
        return int(fields[0]), int(fields[1]), int(fields[2])
    except (OSError, ValueError, IndexError):
        return None


def parse_context_switches(pid: int) -> tuple[int, int] | None:
    try:
        task_paths = pathlib.Path(f"/proc/{pid}/task").glob("*/status")
    except OSError:
        return None
    values = {}
    for key in ("voluntary_ctxt_switches", "nonvoluntary_ctxt_switches"):
        values[key] = 0
    found = False
    try:
        for task_path in task_paths:
            status = task_path.read_text()
            found = True
            for key in values:
                match = re.search(rf"^{key}:\s+(\d+)$", status, re.MULTILINE)
                if match:
                    values[key] += int(match.group(1))
    except (OSError, ValueError):
        return None
    if not found:
        try:
            status = pathlib.Path(f"/proc/{pid}/status").read_text()
            for key in values:
                match = re.search(rf"^{key}:\s+(\d+)$", status, re.MULTILINE)
                if match:
                    values[key] = int(match.group(1))
        except OSError:
            return None
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


def test_environment(
    base: pathlib.Path,
    lane_count: int,
    mode: str,
    reps=0,
    commit_reps=0,
) -> dict[str, str]:
    env = os.environ.copy()
    env["RUSTSHT_KERNEL_BENCH_BASE"] = str(base)
    env["RUSTSHT_KERNEL_BENCH_LANES"] = str(lane_count)
    env["RUSTSHT_KERNEL_BENCH_MODE"] = mode
    env["RUSTFLAGS"] = RUSTFLAGS
    if mode == "run":
        env["RUSTSHT_KERNEL_BENCH_REPETITIONS"] = str(reps)
        env["RUSTSHT_KERNEL_BENCH_COMMIT_REPETITIONS"] = str(commit_reps)
        env["RUSTSHT_KERNEL_BENCH_BATCHES"] = str(BATCHES)
    elif mode == "calibrate":
        env["RUSTSHT_KERNEL_BENCH_CALIBRATION_REPETITIONS"] = str(reps)
        env["RUSTSHT_KERNEL_BENCH_COMMIT_CALIBRATION_REPETITIONS"] = str(
            COMMIT_CALIBRATION_REPETITIONS
        )
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
    if BATCHES * INVOCATIONS < 30:
        raise RuntimeError("the comparison must collect at least 30 batches per backend")
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
    validate_ducc_flag_contract(cxx)
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
        source_root = ROOT / "reports/data/scalar-analysis-pair-grouping-2026-10-19"
        driver_source = source_root / "ducc-ordinary-kernel.cpp"
        kernel_source = source_root / "ducc-kernel-fast.cpp"
        driver_object = pathlib.Path(temporary) / "ducc-driver.o"
        kernel_object = pathlib.Path(temporary) / "ducc-kernel.o"
        include_flag = f"-I{ducc_src / 'src'}"
        driver_compile = cxx + [
            "-std=c++17",
            *DUCC_DRIVER_CXXFLAGS,
            include_flag,
            "-c",
            str(driver_source),
            "-o",
            str(driver_object),
        ]
        kernel_compile = cxx + [
            "-std=c++17",
            *DUCC_KERNEL_CXXFLAGS,
            include_flag,
            "-c",
            str(kernel_source),
            "-o",
            str(kernel_object),
        ]
        link_command = cxx + [
            str(driver_object),
            str(kernel_object),
            "-o",
            str(ducc_binary),
        ]
        for command in (driver_compile, kernel_compile, link_command):
            subprocess.run(command, cwd=ROOT, check=True)

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
                    "--map2alm-kernel",
                ],
                env,
                cpu,
                monitor_cpu,
            )
            ducc_cal = metric_line(ducc_stdout, "DUCC_CALIBRATION")
            if int(ducc_cal["vlen"]) != 4:
                raise RuntimeError(f"DUCC did not execute four-lane f64 AVX2: {ducc_cal}")
            if int(ducc_cal["map2alm_kernel"]) != 1:
                raise RuntimeError(f"DUCC calibration skipped map2alm_kernel: {ducc_cal}")
            if int(rust_cal["repetitions"]) != calibration_repetitions[lane_count]:
                raise RuntimeError("Rust calibration used an unexpected batch size")
            if int(rust_cal["commit_repetitions"]) != COMMIT_CALIBRATION_REPETITIONS:
                raise RuntimeError("Rust commit calibration used an unexpected batch size")
            if int(ducc_cal["repetitions"]) != calibration_repetitions[lane_count]:
                raise RuntimeError("DUCC calibration used an unexpected batch size")
            calibration[lane_count] = {
                "rust_checked": float(rust_cal["checked_us"]),
                "rust_unchecked": float(rust_cal["unchecked_us"]),
                "ducc": float(ducc_cal["reset_inclusive_us"]),
                "commit_only": float(rust_cal["commit_us"]),
            }
            diagnostics.extend(
                [("rust-calibrate", lane_count, rust_diag), ("ducc-calibrate", lane_count, ducc_diag)]
            )

        repetitions = {}
        commit_repetitions = {}
        for lane_count, samples in calibration.items():
            calibration_ms = {
                key: value * calibration_repetitions[lane_count] / 1000.0
                for key, value in samples.items()
                if key != "commit_only"
            }
            if any(not (20.0 <= value <= 100.0) for value in calibration_ms.values()):
                raise RuntimeError(
                    f"calibration batch outside 20-100 ms for lane={lane_count}: {calibration_ms}"
                )
            kernel_samples = {
                key: value for key, value in samples.items() if key != "commit_only"
            }
            fastest = min(kernel_samples.values())
            slowest = max(kernel_samples.values())
            lower = math.ceil(20_000.0 / fastest)
            upper = math.floor(95_000.0 / slowest)
            if lower > upper:
                raise RuntimeError(
                    f"cannot fit all lane={lane_count} variants into 20-100 ms batches: "
                    f"calibrated range {fastest:.3f}..{slowest:.3f} us/call"
                )
            chosen = min(max(round(TARGET_FASTEST_BATCH_US / fastest), lower), upper)
            repetitions[lane_count] = chosen
            commit_us = samples["commit_only"]
            commit_lower = math.ceil(20_000.0 / commit_us)
            commit_upper = math.floor(95_000.0 / commit_us)
            if commit_lower > commit_upper:
                raise RuntimeError(
                    f"cannot fit commit-only lane={lane_count} into 20-100 ms batches: "
                    f"calibrated at {commit_us:.6f} us/call"
                )
            commit_repetitions[lane_count] = min(
                max(round(TARGET_FASTEST_BATCH_US / commit_us), commit_lower),
                commit_upper,
            )

        observations = defaultdict(list)
        component_observations = defaultdict(list)
        numeric_errors = {}
        batch_durations = []
        component_batch_durations = []
        wall_durations = []
        reset_observations = defaultdict(list)
        stability_checks = []
        for lane_count in lanes:
            for invocation in range(INVOCATIONS):
                order = ("rust", "ducc") if invocation % 2 == 0 else ("ducc", "rust")
                for backend in order:
                    if backend == "rust":
                        run_env = test_environment(
                            base,
                            lane_count,
                            "run",
                            repetitions[lane_count],
                            commit_repetitions[lane_count],
                        )
                        command = rust_command
                    else:
                        run_env = env
                        command = [
                            str(ducc_binary),
                            str(fixture_paths[lane_count]),
                            str(repetitions[lane_count]),
                            str(BATCHES),
                            "--map2alm-kernel",
                        ]
                    stdout, diag = run_pinned(command, run_env, cpu, monitor_cpu)
                    diagnostics.append((f"{backend}-run-{invocation + 1}", lane_count, diag))
                    if backend == "ducc":
                        warmup = metric_line(stdout, "DUCC_WARMUP")
                        if (
                            int(warmup["lanes"]) != lane_count
                            or int(warmup["repetitions"]) != DUCC_TIMED_WARMUPS
                            or not math.isfinite(float(warmup["checksum"]))
                        ):
                            raise RuntimeError(
                                f"DUCC timed-process warmup mismatch: {warmup}"
                            )
                    expected_prefix = "RUST_BATCH" if backend == "rust" else "DUCC_BATCH"
                    parsed_count = 0
                    component_count = 0
                    for line in stdout.splitlines():
                        component = line_fields(line, "COMPONENT_BATCH")
                        if component is not None:
                            if backend != "rust":
                                raise RuntimeError("DUCC emitted a Rust-only component record")
                            if int(component["lanes"]) != lane_count:
                                raise RuntimeError(f"component lane mismatch: {component}")
                            if int(component["repetitions"]) != repetitions[lane_count]:
                                raise RuntimeError(f"component repetition mismatch: {component}")
                            batch_index = int(component["batch"])
                            if batch_index != component_count:
                                raise RuntimeError(f"component batch order mismatch: {component}")
                            for variant in ("full", "recurrence"):
                                kernel_us = float(component[f"{variant}_kernel_us"])
                                wall_us = float(component[f"{variant}_wall_us"])
                                reset_us = float(component[f"{variant}_reset_us"])
                                per_call_us = kernel_us / repetitions[lane_count]
                                wall_ms = wall_us / 1000.0
                                component_observations[(lane_count, variant)].append(
                                    (per_call_us, invocation + 1, batch_index, wall_ms, reset_us / repetitions[lane_count])
                                )
                                component_batch_durations.append(wall_ms)
                            component_count += 1
                            continue
                        fields = line_fields(line, expected_prefix)
                        if fields is None:
                            continue
                        parsed_count += 1
                        mode = fields["mode"]
                        if backend == "ducc" and mode == "map2alm_kernel":
                            mode = "kernel_plus_commit"
                        variant = fields.get("variant", "ducc")
                        reps = int(fields["repetitions"])
                        expected_repetitions = (
                            commit_repetitions[lane_count]
                            if mode == "commit_only"
                            else repetitions[lane_count]
                        )
                        if reps != expected_repetitions:
                            raise RuntimeError("backend used a different repetition count")
                        if mode in ("kernel_only", "kernel_plus_commit"):
                            batch_us = float(fields["kernel_us"])
                        elif mode == "commit_only":
                            batch_us = float(fields["commit_us"])
                        else:
                            batch_us = float(fields["batch_us"])
                        batch_durations.append(batch_us / 1000.0)
                        per_call_us = batch_us / reps
                        observations[(lane_count, mode, variant, backend)].append(
                            (per_call_us, invocation + 1, batch_us / 1000.0)
                        )
                        if "wall_us" in fields:
                            wall_durations.append(float(fields["wall_us"]) / 1000.0)
                        if "reset_us" in fields:
                            reset_observations[(lane_count, mode, variant, backend)].append(
                                (float(fields["reset_us"]) / reps, invocation + 1)
                            )
                    expected_lines = BATCHES * (8 if backend == "rust" else 2)
                    if parsed_count != expected_lines:
                        raise RuntimeError(
                            f"expected {expected_lines} batch records from {backend}, got {parsed_count}\n{stdout}"
                        )
                    if backend == "rust":
                        if component_count != BATCHES:
                            raise RuntimeError(
                                f"expected {BATCHES} component batches from Rust, got {component_count}\n{stdout}"
                            )
                        if "COMPONENT_VALIDATION lanes=" not in stdout:
                            raise RuntimeError(f"component numerical validation missing:\n{stdout}")
                    if backend == "ducc":
                        error = metric_line(stdout, "DUCC_KERNEL")
                        if int(error["map2alm_kernel"]) != 1:
                            raise RuntimeError(
                                f"DUCC did not execute map2alm_kernel: {error}"
                            )
                        numeric = (
                            lane_count,
                            float(error["coefficient_max_scaled_diff"]),
                            float(error["output_relative_l2"]),
                            float(error["output_max_abs"]),
                        )
                        if not all(math.isfinite(value) for value in numeric[1:]):
                            raise RuntimeError(f"non-finite numerical comparison: {error}")
                        if numeric[1] > 2e-15 or numeric[2] > 5e-12:
                            raise RuntimeError(
                                f"Rust/DUCC output comparison failed at lanes={lane_count}: {error}"
                            )
                        previous_numeric = numeric_errors.get(lane_count)
                        if previous_numeric is None:
                            numeric_errors[lane_count] = numeric
                        else:
                            numeric_errors[lane_count] = (
                                lane_count,
                                max(previous_numeric[1], numeric[1]),
                                max(previous_numeric[2], numeric[2]),
                                max(previous_numeric[3], numeric[3]),
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
            f"DUCC={DUCC_REVISION}; cxx={cxx_version}; Rust fast-math=disabled"
        )
        print(f"DUCC_DRIVER_COMPILE {shlex.join(driver_compile)}")
        print(f"DUCC_KERNEL_COMPILE {shlex.join(kernel_compile)}")
        print(f"DUCC_LINK {shlex.join(link_command)}")
        print("DISPATCH Rust=Pulp V3 AVX2/FMA; DUCC=compiled x86-64-v3; both use 4 f64 lanes")
        trial_order = ", ".join(
            "Rust/DUCC" if invocation % 2 == 0 else "DUCC/Rust"
            for invocation in range(INVOCATIONS)
        )
        print(
            f"TRIAL_ORDER per lane across {INVOCATIONS} independent process pairs: {trial_order}"
        )
        for lane_count in lanes:
            calibration_batch_ms = {
                key: value * calibration_repetitions[lane_count] / 1000.0
                for key, value in calibration[lane_count].items()
                if key != "commit_only"
            }
            commit_calibration_batch_ms = (
                calibration[lane_count]["commit_only"]
                * COMMIT_CALIBRATION_REPETITIONS
                / 1000.0
            )
            print(
                f"CALIBRATION lanes={lane_count} calibration_repetitions={calibration_repetitions[lane_count]} "
                f"run_repetitions={repetitions[lane_count]} "
                f"commit_calibration_repetitions={COMMIT_CALIBRATION_REPETITIONS} "
                f"commit_run_repetitions={commit_repetitions[lane_count]} "
                + " ".join(
                    f"{key}={value:.3f}us"
                    for key, value in calibration[lane_count].items()
                )
            )
            print(
                "CALIBRATION_BATCH_MS "
                + " ".join(f"{key}={value:.2f}" for key, value in calibration_batch_ms.items())
                + f" commit_only={commit_calibration_batch_ms:.2f}"
            )

        print(
            "Batch timers bracket repeated operations; reset-excluded kernel estimates subtract "
            "a separately timed reset-only batch. Rust kernel_only measures recurrence plus SIMD "
            "staging; commit_only measures final coefficient addition; kernel_plus_commit includes "
            "both. DUCC times map2alm_kernel from the same preinitialized recurrence state and "
            "writes raw paired coefficients; sectoral initialization and transform-level paired "
            "coefficient correction are excluded on both sides. "
            "DUCC accumulates directly into its output. reset_inclusive reports the unadjusted "
            "reset plus operation batch."
        )
        print(
            "Timed-process warmups: Rust checked and unchecked routes each run three times; "
            "DUCC runs three reset/kernel/checksum passes before timing."
        )
        print(
            f"TIMINGS are per operation; aggregate values summarize "
            f"{INVOCATIONS * BATCHES} batches ({INVOCATIONS} independent processes x "
            f"{BATCHES} batches per process and backend)."
        )
        for lane_count in lanes:
            for mode in ("kernel_only", "commit_only", "kernel_plus_commit", "reset_inclusive"):
                print(f"{mode} lanes={lane_count}")
                variants = (
                    ("checked", "unchecked", "ducc")
                    if mode in ("kernel_plus_commit", "reset_inclusive")
                    else ("checked", "unchecked")
                )
                medians = {}
                for variant in variants:
                    backend = "ducc" if variant == "ducc" else "rust"
                    samples = observations[(lane_count, mode, variant, backend)]
                    per_call = [sample[0] for sample in samples]
                    median, p10, p90, q1, q3, low, high = summarize(per_call)
                    medians[variant] = median
                    process_medians = []
                    for invocation in range(1, INVOCATIONS + 1):
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
                    process_spread = (max(process_medians) - min(process_medians)) / max(
                        statistics.median(process_medians), 1e-12
                    )
                    process_state = (
                        "STABLE"
                        if process_spread <= MAX_PROCESS_RANGE_OVER_MEDIAN
                        else "UNSTABLE"
                    )
                    stability_checks.append(
                        (lane_count, mode, variant, backend, process_spread, process_state)
                    )
                    print(
                        f"  PROCESS_STABILITY backend={backend} variant={variant} "
                        f"range_over_median={process_spread:.1%} state={process_state}"
                    )
                    resets = reset_observations[(lane_count, mode, variant, backend)]
                    if resets:
                        reset_per_call = [value for value, _ in resets]
                        print(
                            f"  reset_cost median={statistics.median(reset_per_call):.3f} "
                            f"p10/p90={summarize(reset_per_call)[1]:.3f}.."
                            f"{summarize(reset_per_call)[2]:.3f} us/op"
                        )
                    for invocation in range(1, INVOCATIONS + 1):
                        process_values = [
                            sample[0] for sample in samples if sample[1] == invocation
                        ]
                        process_median, process_p10, process_p90, _, _, _, _ = summarize(
                            process_values
                        )
                        print(
                            f"  PROCESS backend={backend} variant={variant} "
                            f"lanes={lane_count} mode={mode} invocation={invocation} "
                            f"median={process_median:.3f}us "
                            f"p10/p90={process_p10:.3f}..{process_p90:.3f}us"
                        )
                if mode in ("kernel_plus_commit", "reset_inclusive"):
                    for rust_variant in ("checked", "unchecked"):
                        paired_ratios = []
                        paired_excess = []
                        for invocation in range(1, INVOCATIONS + 1):
                            rust_process = [
                                sample[0]
                                for sample in observations[
                                    (lane_count, mode, rust_variant, "rust")
                                ]
                                if sample[1] == invocation
                            ]
                            ducc_process = [
                                sample[0]
                                for sample in observations[(lane_count, mode, "ducc", "ducc")]
                                if sample[1] == invocation
                            ]
                            if len(rust_process) != BATCHES or len(ducc_process) != BATCHES:
                                raise RuntimeError(
                                    f"incomplete paired process samples for lanes={lane_count} "
                                    f"mode={mode} invocation={invocation}"
                                )
                            rust_process_median = statistics.median(rust_process)
                            ducc_process_median = statistics.median(ducc_process)
                            paired_ratios.append(rust_process_median / ducc_process_median)
                            paired_excess.append(rust_process_median - ducc_process_median)
                        ratio_summary = summarize(paired_ratios)
                        excess_summary = summarize(paired_excess)
                        ratio_spread = (max(paired_ratios) - min(paired_ratios)) / max(
                            statistics.median(paired_ratios), 1e-12
                        )
                        ratio_state = (
                            "STABLE"
                            if ratio_spread <= MAX_PROCESS_RANGE_OVER_MEDIAN
                            else "UNSTABLE"
                        )
                        stability_checks.append(
                            (
                                lane_count,
                                mode,
                                f"{rust_variant}/DUCC",
                                "paired",
                                ratio_spread,
                                ratio_state,
                            )
                        )
                        print(
                            f"  paired_process {rust_variant}/DUCC: "
                            f"ratio median={ratio_summary[0]:.3f} "
                            f"p10/p90={ratio_summary[1]:.3f}..{ratio_summary[2]:.3f} "
                            f"range={ratio_summary[5]:.3f}..{ratio_summary[6]:.3f}; "
                            f"stability_spread={ratio_spread:.1%} state={ratio_state}; "
                            f"excess_us median={excess_summary[0]:.3f} "
                            f"p10/p90={excess_summary[1]:.3f}..{excess_summary[2]:.3f} "
                            f"range={excess_summary[5]:.3f}..{excess_summary[6]:.3f}"
                        )
                    print(
                        f"  ratios checked/DUCC={medians['checked'] / medians['ducc']:.3f} "
                        f"unchecked/DUCC={medians['unchecked'] / medians['ducc']:.3f}"
                    )

        print(
            "COMPONENT_METHOD full uses the ordinary fused production loop; recurrence-only compiles out spectrum products and horizontal reductions while retaining recurrence, safety checks, state updates, dispatch, and zeroed coefficient stores."
        )
        for lane_count in lanes:
            process_medians = {}
            for variant in ("full", "recurrence"):
                samples = component_observations[(lane_count, variant)]
                per_call = [sample[0] for sample in samples]
                median, p10, p90, _, _, _, _ = summarize(per_call)
                process_medians[variant] = {}
                for invocation in range(1, INVOCATIONS + 1):
                    values = [sample[0] for sample in samples if sample[1] == invocation]
                    process_medians[variant][invocation] = statistics.median(values)
                process_values = list(process_medians[variant].values())
                process_spread = (max(process_values) - min(process_values)) / max(
                    statistics.median(process_values), 1e-12
                )
                state = "STABLE" if process_spread <= MAX_PROCESS_RANGE_OVER_MEDIAN else "UNSTABLE"
                stability_checks.append((lane_count, "components", variant, "rust", process_spread, state))
                resets = [sample[4] for sample in samples]
                print(
                    f"COMPONENT_TIMING lanes={lane_count} variant={variant} median={median:.3f}us/op "
                    f"p10/p90={p10:.3f}..{p90:.3f} process_spread={process_spread:.1%} state={state} "
                    f"reset_median={statistics.median(resets):.3f}us/op"
                )
            paired_excess = [
                process_medians["full"][invocation] - process_medians["recurrence"][invocation]
                for invocation in range(1, INVOCATIONS + 1)
            ]
            paired_ratio = [
                process_medians["full"][invocation] / process_medians["recurrence"][invocation]
                for invocation in range(1, INVOCATIONS + 1)
            ]
            excess_summary = summarize(paired_excess)
            ratio_summary = summarize(paired_ratio)
            print(
                f"COMPONENT_DELTA lanes={lane_count} spectrum_accumulation_plus_reduction="
                f"{excess_summary[0]:.3f}us/op p10/p90={excess_summary[1]:.3f}..{excess_summary[2]:.3f} "
                f"full/recurrence={ratio_summary[0]:.3f} p10/p90={ratio_summary[1]:.3f}..{ratio_summary[2]:.3f}"
            )

        min_component_batch = min(component_batch_durations)
        max_component_batch = max(component_batch_durations)
        component_outside = [
            value for value in component_batch_durations if not (20.0 <= value <= 100.0)
        ]
        print(
            f"COMPONENT_BATCH_DURATIONS ms range={min_component_batch:.2f}..{max_component_batch:.2f}; "
            f"outside_20_100ms={len(component_outside)}/{len(component_batch_durations)}"
        )

        min_batch = min(batch_durations)
        max_batch = max(batch_durations)
        outside_durations = [
            sample for sample in batch_durations if not (20.0 <= sample <= 100.0)
        ]
        print(
            f"BATCH_DURATIONS measured_kernel_or_reset_ms range={min_batch:.2f}..{max_batch:.2f}; "
            f"outside_20_100ms={len(outside_durations)}/{len(batch_durations)}"
        )
        if outside_durations:
            print(
                f"BATCH_WARNING out_of_window_ms="
                f"{min(outside_durations):.3f}..{max(outside_durations):.3f}; "
                "retained without filtering; inspect host scheduling diagnostics"
            )
        if wall_durations:
            wall_min = min(wall_durations)
            wall_max = max(wall_durations)
            wall_outside = [
                sample for sample in wall_durations if not (20.0 <= sample <= 100.0)
            ]
            print(
                f"BATCH_WALL_DURATIONS operation_ms range={wall_min:.2f}..{wall_max:.2f}; "
                f"outside_20_100ms={len(wall_outside)}/{len(wall_durations)}"
            )
            if wall_outside:
                print(
                    f"BATCH_WALL_WARNING out_of_window_ms="
                    f"{min(wall_outside):.3f}..{max(wall_outside):.3f}; "
                    "retained without filtering; inspect host scheduling diagnostics"
                )
        unstable = [item for item in stability_checks if item[-1] == "UNSTABLE"]
        print(
            f"TIMING_STABILITY threshold_process_range_over_median="
            f"{MAX_PROCESS_RANGE_OVER_MEDIAN:.0%} status="
            f"{'UNSTABLE' if unstable else 'STABLE'}"
        )
        if unstable:
            for lane_count, mode, variant, backend, spread, _ in unstable:
                print(
                    f"  unexplained_regime_candidate lanes={lane_count} mode={mode} "
                    f"variant={variant} backend={backend} range_over_median={spread:.1%}"
                )
            print(
                "INTERPRETATION unstable process-level results require correlation with "
                "HOST_PROCESS frequency and scheduling records before parity claims"
            )
        for lane_count, coefficient_error, relative_l2, max_abs in numeric_errors.values():
            print(
                f"NUMERICS lanes={lane_count} coefficient_max_scaled_diff={coefficient_error:.3e} "
                f"output_relative_l2={relative_l2:.3e} output_max_abs={max_abs:.3e}"
            )

        timed_diagnostics = [
            diag for label, _, diag in diagnostics if "-run-" in label
        ]
        frequencies = [
            value
            for diag in timed_diagnostics
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
        for diag in timed_diagnostics:
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
