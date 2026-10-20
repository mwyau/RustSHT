#!/usr/bin/env python3
"""Calibrate matched Rust/DUCC scalar-analysis prefix kernel batches."""

from __future__ import annotations

import hashlib
import math
import os
import pathlib
import re
import shlex
import statistics
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import benchmark_scalar_analysis_kernel as common

DUCC_REVISION = common.DUCC_REVISION
RUST_TEST = "legendre::scalar_paired::kernel_bench::benchmark_fused_scaled_scalar_analysis_prefix"
TIMED_CASES = (
    ("synthetic", 64),
    ("synthetic", 128),
    ("synthetic", 256),
    ("gl-sectoral", 64),
    ("gl-sectoral", 128),
)
CONTROL_CASE = ("gl-sectoral", 256)
CALIBRATION_REPETITIONS = 1024
BATCHES_PER_PROCESS = 10
PROCESS_INVOCATIONS = 3
TARGET_BATCH_US = 32_000.0


def pinned_text(command: list[str], env: dict, cpu: int, monitor_cpu: int):
    return common.run_pinned(command, env, cpu, monitor_cpu)


def rust_environment(
    base: pathlib.Path,
    *,
    case=None,
    kernel_reps=None,
    wall_reps=None,
    batches=0,
    calibrate=False,
):
    env = os.environ.copy()
    env.pop("RUSTSHT_PREFIX_CALIBRATE_ONLY", None)
    env.pop("RUSTSHT_PREFIX_ONLY_CASE", None)
    env["RUSTFLAGS"] = common.RUSTFLAGS
    env["RUSTSHT_KERNEL_BENCH_BASE"] = str(base)
    env["RUSTSHT_PREFIX_BATCHES"] = str(batches)
    if calibrate:
        env["RUSTSHT_PREFIX_CALIBRATE_ONLY"] = "1"
    if case is not None:
        env["RUSTSHT_PREFIX_ONLY_CASE"] = f"{case[0]}-{case[1]}"
    case_suffix = f"{case[0].replace('-', '_').upper()}_{case[1]}" if case else ""
    if kernel_reps is not None:
        env[f"RUSTSHT_PREFIX_KERNEL_REPS_{case_suffix}"] = str(kernel_reps)
    if wall_reps is not None:
        env[f"RUSTSHT_PREFIX_WALL_REPS_{case_suffix}"] = str(wall_reps)
    return env


def output_fields(text: str, prefix: str):
    return [
        fields
        for line in text.splitlines()
        if (fields := common.line_fields(line, prefix)) is not None
    ]


def main() -> None:
    ducc_src_value = os.environ.get("DUCC_SRC")
    ducc_git_value = os.environ.get("DUCC_GIT")
    if not ducc_src_value or not ducc_git_value:
        raise SystemExit("set DUCC_SRC and DUCC_GIT to the DUCC source snapshot and revision repository")
    ducc_src = pathlib.Path(ducc_src_value).resolve()
    ducc_git = pathlib.Path(ducc_git_value).resolve()
    for relative in (
        "src/ducc0/sht/sht_inner_loop.h",
        "src/ducc0/sht/sht.h",
        "src/ducc0/infra/simd.h",
        "src/ducc0/math/constants.h",
    ):
        source = (ducc_src / relative).read_bytes()
        pinned = subprocess.run(
            ["git", "-C", str(ducc_git), "show", f"{DUCC_REVISION}:{relative}"],
            check=True,
            capture_output=True,
        ).stdout
        if hashlib.sha256(source).digest() != hashlib.sha256(pinned).digest():
            raise SystemExit(f"DUCC source differs from pinned {DUCC_REVISION}: {relative}")

    cpu, monitor_cpu = common.choose_cpus()
    model, physical, logical, memory_gib = common.read_cpu_summary()
    print(
        f"ENV cpu={model}; physical_cores={physical}; logical_cpus={logical}; "
        f"installed_ram_gib={memory_gib:.1f}; cpu_features={common.read_cpu_features()}; "
        "single-thread trials pinned to one allowed physical core"
    )
    print(
        f"VERSIONS rustc={common.output(['rustc', '--version']).strip()} "
        f"cargo={common.output(['cargo', '--version']).strip()} "
        f"cxx={common.output(shlex.split(os.environ.get('CXX', 'g++')) + ['--version']).splitlines()[0]} "
        f"ducc={DUCC_REVISION} target=x86-64-v3"
    )

    build_env = os.environ.copy()
    build_env["RUSTFLAGS"] = common.RUSTFLAGS
    build = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "rustsht",
            "--release",
            "--lib",
            "benchmark_fused_scaled_scalar_analysis_prefix",
            "--no-run",
        ],
        cwd=ROOT,
        env=build_env,
        check=True,
        text=True,
        capture_output=True,
    )
    match = re.search(r"Executable .* \(([^)]+)\)", build.stdout + build.stderr)
    if not match:
        raise RuntimeError(f"could not locate release test binary:\n{build.stdout}\n{build.stderr}")
    rust_binary = pathlib.Path(match.group(1))
    if not rust_binary.is_absolute():
        rust_binary = ROOT / rust_binary

    cxx = shlex.split(os.environ.get("CXX", "g++"))
    with tempfile.TemporaryDirectory(prefix="rustsht-prefix-batches-") as temporary:
        base = pathlib.Path(temporary) / "prefix"
        ducc_binary = pathlib.Path(temporary) / "ducc-prefix-bench"
        subprocess.run(
            cxx
            + [
                "-std=c++17",
                "-O3",
                "-DNDEBUG",
                "-march=x86-64-v3",
                f"-I{ducc_src / 'src'}",
                str(ROOT / "reports/data/scalar-analysis-pair-grouping-2026-10-19/ducc-scaled-prefix.cpp"),
                "-o",
                str(ducc_binary),
            ],
            cwd=ROOT,
            check=True,
        )

        rust_command = [
            str(rust_binary),
            "--ignored",
            "--nocapture",
            "--exact",
            RUST_TEST,
            "--test-threads=1",
        ]
        prepare, _ = pinned_text(
            rust_command,
            rust_environment(base, calibrate=True),
            cpu,
            monitor_cpu,
        )
        if "RUST_PREFIX_ARCH V3(" not in prepare:
            raise RuntimeError(f"Rust did not dispatch Pulp V3:\n{prepare}")
        rust_calibration = {
            (fields["kind"], int(fields["lanes"])): fields
            for fields in output_fields(prepare, "RUST_PREFIX_CALIBRATION")
        }
        if len(rust_calibration) != 6:
            raise RuntimeError(f"expected six Rust calibration cases, got {len(rust_calibration)}")

        calibration = {}
        for case in (*TIMED_CASES, CONTROL_CASE):
            fixture = pathlib.Path(f"{base}-{case[0]}-{case[1]}.in")
            ducc_text, _ = pinned_text(
                [str(ducc_binary), str(fixture), "--calibrate", str(CALIBRATION_REPETITIONS)],
                os.environ.copy(),
                cpu,
                monitor_cpu,
            )
            fields = common.metric_line(ducc_text, "DUCC_PREFIX_CALIBRATION")
            state = common.metric_line(ducc_text, "DUCC_PREFIX")
            rust_fields = rust_calibration[case]
            if (
                int(state["scale_mismatches"]) != 0
                or float(state["state_relative_l2"]) > 1e-12
                or int(state["prefix_pair"]) != int(rust_fields["pair_start"])
            ):
                raise RuntimeError(f"Rust/DUCC calibration states differ for {case}: {state}")
            if int(fields["vlen"]) != 4:
                raise RuntimeError(f"DUCC did not use four f64 AVX2 lanes: {fields}")
            print(
                f"PREPARATION kind={case[0]} lanes={case[1]} "
                f"rust_scale_and_correction_preflight_us={float(rust_fields['preflight_us']):.3f} "
                f"rust_timer_overhead_ns={float(rust_fields['timer_us_per_call']) * 1000.0:.1f} "
                f"ducc_timer_overhead_ns={float(fields['timer_us_per_call']) * 1000.0:.1f} "
                f"rust_kernel_us={float(rust_fields['kernel_us_per_call']):.6f} "
                f"ducc_kernel_us={float(fields['kernel_us_per_call']):.6f} "
                f"rust_wall_us={float(rust_fields['wall_us_per_call']):.6f} "
                f"ducc_wall_us={float(fields['wall_us_per_call']):.6f} "
                f"state_relative_l2={float(state['state_relative_l2']):.3e} "
                f"prefix_pair={state['prefix_pair']}"
            )
            if case == CONTROL_CASE:
                print(
                    f"CONTROL case={case[0]} lanes={case[1]} prefix_pair="
                    f"{rust_fields['pair_start']} rust_prep_us={rust_fields['preflight_us']} "
                    "timed=no"
                )
                continue
            kernel_per_call = [
                float(fields_for_backend["kernel_us_per_call"])
                for fields_for_backend in (rust_fields, fields)
            ]
            wall_per_call = [
                float(fields_for_backend["wall_us_per_call"])
                for fields_for_backend in (rust_fields, fields)
            ]
            if min(kernel_per_call) <= 0.0 or min(wall_per_call) <= 0.0:
                raise RuntimeError(f"timer overhead exceeds measured work for {case}")

            def choose_repetitions(per_call_values: list[float], phase: str) -> int:
                fastest, slowest = min(per_call_values), max(per_call_values)
                lower = math.ceil(20_000.0 / fastest)
                upper = math.floor(80_000.0 / slowest)
                if lower > upper:
                    raise RuntimeError(f"cannot fit paired {phase} batches in 20-100 ms for {case}")
                return min(max(round(TARGET_BATCH_US / fastest), lower), upper)

            kernel_reps = choose_repetitions(kernel_per_call, "kernel-only")
            wall_reps = choose_repetitions(wall_per_call, "reset-inclusive")
            calibration[case] = {
                "kernel_repetitions": kernel_reps,
                "wall_repetitions": wall_reps,
            }
            print(
                f"CALIBRATION_CASE kind={case[0]} lanes={case[1]} "
                f"kernel_repetitions={kernel_reps} wall_repetitions={wall_reps}"
            )
        print(
            f"CALIBRATION batches={PROCESS_INVOCATIONS * BATCHES_PER_PROCESS} "
            "kernel_and_reset_inclusive_target_ms=32; accepted=20..100"
        )

        observations = {
            (case, backend, phase): []
            for case in TIMED_CASES
            for backend in ("rust", "ducc")
            for phase in ("kernel", "wall")
        }
        process_observations = {}
        diagnostics = []
        for case in TIMED_CASES:
            kernel_repetitions = calibration[case]["kernel_repetitions"]
            wall_repetitions = calibration[case]["wall_repetitions"]
            for invocation in range(PROCESS_INVOCATIONS):
                order = ("rust", "ducc") if invocation % 2 == 0 else ("ducc", "rust")
                for backend in order:
                    if backend == "rust":
                        env = rust_environment(
                            base,
                            case=case,
                            kernel_reps=kernel_repetitions,
                            wall_reps=wall_repetitions,
                            batches=BATCHES_PER_PROCESS,
                        )
                        output_text, diag = pinned_text(
                            rust_command, env, cpu, monitor_cpu
                        )
                        if "RUST_PREFIX_ARCH V3(" not in output_text:
                            raise RuntimeError(f"Rust did not dispatch Pulp V3:\n{output_text}")
                        kernel_rows = output_fields(output_text, "RUST_PREFIX_KERNEL_BATCH")
                        wall_rows = output_fields(output_text, "RUST_PREFIX_WALL_BATCH")
                        timer_us_per_call = float(kernel_rows[0]["timer_us_per_call"])
                    else:
                        fixture = pathlib.Path(f"{base}-{case[0]}-{case[1]}.in")
                        output_text, diag = pinned_text(
                            [
                                str(ducc_binary),
                                str(fixture),
                                str(kernel_repetitions),
                                str(wall_repetitions),
                                str(BATCHES_PER_PROCESS),
                            ],
                            os.environ.copy(),
                            cpu,
                            monitor_cpu,
                        )
                        kernel_rows = output_fields(output_text, "DUCC_PREFIX_KERNEL_BATCH")
                        wall_rows = output_fields(output_text, "DUCC_PREFIX_WALL_BATCH")
                        timer_us_per_call = float(kernel_rows[0]["timer_us_per_call"])
                        state = common.metric_line(output_text, "DUCC_PREFIX")
                        if int(state["scale_mismatches"]) != 0 or float(state["state_relative_l2"]) > 1e-12:
                            raise RuntimeError(f"Rust/DUCC prefix states differ: {state}")
                    if len(kernel_rows) != BATCHES_PER_PROCESS or len(wall_rows) != BATCHES_PER_PROCESS:
                        raise RuntimeError(
                            f"expected {BATCHES_PER_PROCESS} kernel and reset-inclusive "
                            f"batches for {case}/{backend}, got {len(kernel_rows)}/{len(wall_rows)}\n{output_text}"
                        )
                    process_values = {"kernel": [], "wall": []}
                    for kernel_row, wall_row in zip(kernel_rows, wall_rows, strict=True):
                        if int(kernel_row["repetitions"]) != kernel_repetitions:
                            raise RuntimeError(f"{backend} used a mismatched kernel repetition count")
                        if int(wall_row["repetitions"]) != wall_repetitions:
                            raise RuntimeError(f"{backend} used a mismatched wall repetition count")
                        row_timer_us = float(kernel_row["timer_us_per_call"])
                        if not math.isclose(row_timer_us, timer_us_per_call, rel_tol=0.0, abs_tol=1e-12):
                            raise RuntimeError(f"{backend} changed timer calibration between batches")
                        wall_us = float(wall_row["wall_us"])
                        kernel_us = float(kernel_row["kernel_us"])
                        corrected_kernel_us = kernel_us - row_timer_us * kernel_repetitions
                        if not 20_000.0 <= wall_us <= 100_000.0:
                            raise RuntimeError(
                                f"reset-inclusive batch outside 20-100ms "
                                f"for {case}/{backend}: {wall_us} us"
                            )
                        if not 20_000.0 <= corrected_kernel_us <= 100_000.0:
                            raise RuntimeError(
                                f"kernel corrected batch outside 20-100ms "
                                f"for {case}/{backend}: {corrected_kernel_us} us; "
                                f"raw={kernel_us} us repetitions={kernel_repetitions} "
                                f"timer_us_per_call={row_timer_us}"
                            )
                        wall_per_call = wall_us / wall_repetitions
                        kernel_per_call = corrected_kernel_us / kernel_repetitions
                        observations[(case, backend, "kernel")].append(kernel_per_call)
                        observations[(case, backend, "wall")].append(wall_per_call)
                        process_values["kernel"].append(kernel_per_call)
                        process_values["wall"].append(wall_per_call)
                    for phase in ("kernel", "wall"):
                        process_observations[(case, backend, invocation + 1, phase)] = statistics.median(
                            process_values[phase]
                        )
                    diagnostics.append((case, backend, invocation + 1, diag))

        print(
            "RESULTS stage=iter_to_ieee_or_Rust_prefix_loop; times=median_us_per_operation; "
            "kernel subtracts measured per-call clock overhead; wall includes reset"
        )
        for case in TIMED_CASES:
            values = {}
            for backend in ("rust", "ducc"):
                values[backend] = {}
                for phase in ("kernel", "wall"):
                    summary = common.summarize(observations[(case, backend, phase)])
                    values[backend][phase] = summary[0]
                    print(
                        f"CASE kind={case[0]} lanes={case[1]} backend={backend} phase={phase} "
                        f"median={summary[0]:.6f} p10={summary[1]:.6f} p90={summary[2]:.6f} "
                        f"range={summary[5]:.6f}..{summary[6]:.6f} samples={len(observations[(case, backend, phase)])}"
                    )
                    process_medians = [
                        process_observations[(case, backend, invocation, phase)]
                        for invocation in range(1, PROCESS_INVOCATIONS + 1)
                    ]
                    print(
                        f"PROCESS_MEDIANS kind={case[0]} lanes={case[1]} backend={backend} phase={phase} "
                        f"values_us={','.join(f'{value:.6f}' for value in process_medians)}"
                    )
            print(
                f"RATIO kind={case[0]} lanes={case[1]} kernel={values['rust']['kernel'] / values['ducc']['kernel']:.3f}x "
                f"wall={values['rust']['wall'] / values['ducc']['wall']:.3f}x"
            )

        for case, backend, invocation, diag in diagnostics:
            frequencies = diag["frequencies_khz"]
            frequency = (
                f"frequency_median_mhz={statistics.median(frequencies) / 1000.0:.0f} "
                f"frequency_range_mhz={min(frequencies) / 1000.0:.0f}..{max(frequencies) / 1000.0:.0f}"
                if frequencies
                else "frequency=unavailable"
            )
            before, after = diag["sched_before"], diag["sched_after"]
            if before and after:
                runtime = after[0] - before[0]
                wait = after[1] - before[1]
                wait_fraction = 100.0 * wait / max(runtime + wait, 1)
            else:
                wait_fraction = float("nan")
            switches_before, switches_after = diag["switches_before"], diag["switches_after"]
            if switches_before and switches_after:
                voluntary = switches_after[0] - switches_before[0]
                involuntary = switches_after[1] - switches_before[1]
                switches = f"voluntary_switches={voluntary} involuntary_switches={involuntary}"
            else:
                switches = "context_switches=unavailable"
            print(
                f"HOST case={case[0]} lanes={case[1]} backend={backend} invocation={invocation} {frequency} "
                f"runqueue_wait_fraction={wait_fraction:.3f}% {switches}"
            )


if __name__ == "__main__":
    main()
