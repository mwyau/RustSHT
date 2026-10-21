#!/usr/bin/env python3
"""Interleaved T720 source A/B for scalar-analysis output initialization."""

from __future__ import annotations

import argparse
import csv
import json
import pathlib
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from scripts import benchmark_ducc_full_transform_parity as parity


DUCC_REVISION = "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
PROCESS_ORDER = (
    ("baseline", "ducc", "candidate"),
    ("candidate", "ducc", "baseline"),
    ("ducc", "baseline", "candidate"),
)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline-binary", type=pathlib.Path, required=True)
    parser.add_argument("--candidate-binary", type=pathlib.Path, required=True)
    parser.add_argument("--ducc-binary", type=pathlib.Path, required=True)
    parser.add_argument("--ducc-src", type=pathlib.Path, required=True)
    parser.add_argument("--baseline-revision", required=True)
    parser.add_argument("--candidate-revision", required=True)
    parser.add_argument("--results-dir", type=pathlib.Path, required=True)
    parser.add_argument("--batch-ms", type=float, default=25.0)
    args = parser.parse_args()
    if not 20.0 <= args.batch_ms <= 100.0:
        parser.error("--batch-ms must be in the 20–100 ms calibration window")
    actual_ducc_revision = parity.command_output(
        ["git", "rev-parse", "HEAD"], cwd=args.ducc_src
    )
    if actual_ducc_revision != DUCC_REVISION:
        parser.error(f"DUCC checkout {actual_ducc_revision} is not pinned {DUCC_REVISION}")

    args.results_dir.mkdir(parents=True, exist_ok=True)
    physical = parity.physical_cpus()
    monitor_cpu = physical[8]
    binaries = {
        "baseline": args.baseline_binary.resolve(),
        "candidate": args.candidate_binary.resolve(),
        "ducc": args.ducc_binary.resolve(),
    }
    hardware = parity.hardware_metadata(args.ducc_src)
    raw = {
        "metadata": {
            "cpu_model": parity.cpu_model(),
            "rustc": parity.command_output(["rustc", "--version", "--verbose"]),
            "baseline_revision": args.baseline_revision,
            "candidate_revision": args.candidate_revision,
            "ducc_revision": actual_ducc_revision,
            "pulp": hardware["pulp"],
            "ducc_compiler": hardware["cxx"],
            "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs; runtime Pulp V3",
            "ducc_sht_flags": "-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3",
            "runtime_dispatch": "Each Rust worker record stores selected arch and f64 SIMD lane width; this host selected V3, four f64 lanes.",
            "target_batch_ms": args.batch_ms,
            "measured_batches_per_process": parity.BATCHES_PER_PROCESS,
            "independent_process_pairs": len(PROCESS_ORDER),
            "interleaving": [list(order) for order in PROCESS_ORDER],
            "timed_scope": "full native ShtPlan scalar GL analysis; setup and first call recorded separately",
            "fixtures": "deterministic PCG64 from the shared parity harness; SHA-256 recorded per geometry",
        },
        "cases": [],
    }
    summary = []

    for mmax in (720, 180):
        case = {
            "case_id": f"T720-m{mmax}-nphi1458",
            "lmax": 720,
            "mmax": mmax,
            "ntheta": 721,
            "nphi": 1458,
            "longitude_mode": "even_smooth",
            "grid": "GL",
        }
        case_record = {
            "case": {key: value for key, value in case.items() if key != "case_id"},
            "process_pairs": [],
        }
        with tempfile.TemporaryDirectory(prefix="plan14-analysis-init-") as temporary:
            temp = pathlib.Path(temporary)
            parity.make_fixture(case, temp)
            case_record["input_sha256"] = case["input_sha256"]

            for threads in (1, 8):
                cpus = physical[:threads]
                process_pairs = []
                for pair, order in enumerate(PROCESS_ORDER):
                    records = {}
                    hosts = {}
                    outputs = {}
                    for backend in order:
                        output = temp / f"{backend}-q{threads}-p{pair}.bin"
                        command = parity.worker_command(
                            binaries,
                            backend,
                            case,
                            "analysis",
                            threads,
                            output,
                            args.batch_ms,
                            validate=False,
                        )
                        record, host = parity.run_pinned(command, cpus, monitor_cpu)
                        record.update(
                            backend=backend,
                            process_pair=pair,
                            threads=threads,
                            case_id=case["case_id"],
                        )
                        records[backend] = record
                        hosts[backend] = {
                            "wait_fraction": host["wait_fraction"],
                            "involuntary_switches": host["involuntary_switches"],
                            "frequency_ghz_p10_median_p90": (
                                [
                                    parity.percentile(host["frequency_ghz"], p)
                                    for p in (0.10, 0.50, 0.90)
                                ]
                                if host["frequency_ghz"]
                                else None
                            ),
                        }
                        outputs[backend] = output

                    for backend in ("baseline", "candidate"):
                        records[backend]["error_vs_ducc"] = parity.error_metrics(
                            parity.read_result(outputs[backend], "analysis", case, backend),
                            parity.read_result(outputs["ducc"], "analysis", case, "ducc"),
                        )
                    records["candidate"]["error_vs_baseline"] = parity.error_metrics(
                        parity.read_result(outputs["candidate"], "analysis", case, "candidate"),
                        parity.read_result(outputs["baseline"], "analysis", case, "baseline"),
                    )
                    process_pairs.append(
                        {"pair": pair, "threads": threads, "records": records, "host": hosts}
                    )

                case_record["process_pairs"].extend(process_pairs)
                for backend in ("baseline", "candidate", "ducc"):
                    samples = [
                        value
                        for pair in process_pairs
                        for value in pair["records"][backend]["batch_ms"]
                    ]
                    totals = [
                        value
                        for pair in process_pairs
                        for value in pair["records"][backend]["batch_total_ms"]
                    ]
                    summary.append(
                        {
                            "case_id": case["case_id"],
                            "mmax": mmax,
                            "threads": threads,
                            "backend": backend,
                            "median_ms": parity.percentile(samples, 0.50),
                            "p10_ms": parity.percentile(samples, 0.10),
                            "p90_ms": parity.percentile(samples, 0.90),
                            "batch_count": len(samples),
                            "batch_duration_p10_ms": parity.percentile(totals, 0.10),
                            "batch_duration_p90_ms": parity.percentile(totals, 0.90),
                        }
                    )
                for numerator, denominator in (
                    ("baseline", "ducc"),
                    ("candidate", "ducc"),
                    ("candidate", "baseline"),
                ):
                    ratios = [
                        top / bottom
                        for pair in process_pairs
                        for top, bottom in zip(
                            pair["records"][numerator]["batch_ms"],
                            pair["records"][denominator]["batch_ms"],
                        )
                    ]
                    summary.append(
                        {
                            "case_id": case["case_id"],
                            "mmax": mmax,
                            "threads": threads,
                            "backend": f"{numerator}/{denominator}",
                            "median_ms": parity.percentile(ratios, 0.50),
                            "p10_ms": parity.percentile(ratios, 0.10),
                            "p90_ms": parity.percentile(ratios, 0.90),
                            "batch_count": len(ratios),
                            "batch_duration_p10_ms": "",
                            "batch_duration_p90_ms": "",
                        }
                    )
                print(f"DONE {case['case_id']} q{threads}", flush=True)
        raw["cases"].append(case_record)

    with (args.results_dir / "paired-batches.json").open("w") as handle:
        json.dump(raw, handle, indent=2)
    with (args.results_dir / "summary.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(summary[0]))
        writer.writeheader()
        writer.writerows(summary)
    print(f"RESULTS {args.results_dir}")


if __name__ == "__main__":
    main()
