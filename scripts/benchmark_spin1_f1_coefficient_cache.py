#!/usr/bin/env python3
"""Matched native A/B benchmark for q8 spin-1 synthesis coefficient reuse.

The driver expects already-built baseline RustSHT, candidate RustSHT, and
pinned DUCC worker binaries. Raw JSON includes host diagnostics and should be
kept as local benchmark evidence; the companion summarizer emits a sanitized
aggregate suitable for version control.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import statistics
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import benchmark_ducc_spin1_native as spin1_bench  # noqa: E402
from benchmark_ducc_full_transform_parity import (  # noqa: E402
    cpu_model,
    error_metrics,
    physical_cpus,
    run_pinned,
)


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=pathlib.Path, required=True)
    parser.add_argument("--candidate", type=pathlib.Path, required=True)
    parser.add_argument("--ducc", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--grid", choices=("CC", "F1"), default="F1")
    parser.add_argument(
        "--baseline-source", default="0b2e1336409ba0bbe9fd6fa4cff70db0e327cd52"
    )
    parser.add_argument(
        "--candidate-source", default="7f47f06f875991a982caf0f4c396323a32c0d72b"
    )
    parser.add_argument("--target-ms", type=float, default=25.0)
    parser.add_argument("--process-pairs", type=int, default=3)
    args = parser.parse_args()

    binaries = {
        "baseline": args.baseline.resolve(),
        "candidate": args.candidate.resolve(),
        "ducc": args.ducc.resolve(),
    }
    for name, path in binaries.items():
        if not path.is_file():
            parser.error(f"{name} binary does not exist: {path}")

    cpus = physical_cpus()
    if len(cpus) < 8:
        parser.error(f"q8 needs eight physical CPUs; found {len(cpus)}")
    monitor_cpu = cpus[8] if len(cpus) > 8 else cpus[0]
    source_cases = spin1_bench.make_cases([720], [args.grid], False)
    cases = [
        case
        for case in source_cases
        if case["latitude_mode"] == "exact"
        and case["longitude_mode"] == "even-smooth"
    ]
    if {case["mmax"] for case in cases} != {180, 720}:
        raise RuntimeError(f"unexpected F1 benchmark cases: {cases!r}")

    raw: list[dict] = []
    comparisons: list[dict] = []
    summaries: list[dict] = []
    with tempfile.TemporaryDirectory(prefix="spin1-f1-cache-fixtures-") as temp_name:
        temp = pathlib.Path(temp_name)
        for case_index, case in enumerate(cases):
            case_dir = temp / f"case-{case_index}"
            case_dir.mkdir()
            fixture = spin1_bench.make_fixture(case, case_dir)
            for threads in (1, 8):
                affinity = cpus[:threads]
                for operation in ("analysis", "synthesis"):
                    runs = {backend: [] for backend in binaries}
                    for pair, order in enumerate(
                        (
                            ("baseline", "ducc", "candidate"),
                            ("candidate", "ducc", "baseline"),
                            ("ducc", "baseline", "candidate"),
                        )[: args.process_pairs]
                    ):
                        output_paths: dict[str, dict[str, pathlib.Path]] = {}
                        results_for_pair: dict[str, dict] = {}
                        for backend in order:
                            outputs = {
                                name: temp
                                / f"{case['case_id']}-{operation}-q{threads}-p{pair}-{backend}-{name}.bin"
                                for name in ("map0", "map1", "e", "b")
                            }
                            command = spin1_bench.worker_command(
                                binaries[backend],
                                case,
                                fixture,
                                operation,
                                threads,
                                outputs,
                                args.target_ms,
                            )
                            result, diagnostics = run_pinned(command, affinity, monitor_cpu)
                            result.update(
                                {
                                    "backend": backend,
                                    **case,
                                    "operation": operation,
                                    "threads": threads,
                                    "process_pair": pair,
                                    "host_diagnostics": diagnostics,
                                    "fixture_sha256": fixture["sha256"],
                                }
                            )
                            raw.append(result)
                            runs[backend].append(result)
                            results_for_pair[backend] = result
                            output_paths[backend] = outputs

                        values = {}
                        for backend in binaries:
                            names, kind = (
                                (("e", "b"), "complex")
                                if operation == "analysis"
                                else (("map0", "map1"), "real")
                            )
                            values[backend] = spin1_bench.np.concatenate(
                                [
                                    spin1_bench.output_values(output_paths[backend][name], kind)
                                    for name in names
                                ]
                            )
                        comparisons.append(
                            {
                                **case,
                                "operation": operation,
                                "threads": threads,
                                "process_pair": pair,
                                "baseline_vs_ducc": error_metrics(values["baseline"], values["ducc"]),
                                "candidate_vs_ducc": error_metrics(values["candidate"], values["ducc"]),
                                "candidate_vs_baseline": error_metrics(values["candidate"], values["baseline"]),
                                "all_finite": all(spin1_bench.np.isfinite(v).all() for v in values.values()),
                                "repeat_consistent": {
                                    backend: record["repeat_consistent"]
                                    for backend, record in results_for_pair.items()
                                },
                                "reset_consistent": {
                                    backend: record["reset_consistent"]
                                    for backend, record in results_for_pair.items()
                                },
                            }
                        )

                    for backend, records in runs.items():
                        batches = [sample for record in records for sample in record["batch_ms"]]
                        summaries.append(
                            {
                                "case_id": case["case_id"],
                                "grid": "F1",
                                "lmax": case["lmax"],
                                "mmax": case["mmax"],
                                "operation": operation,
                                "threads": threads,
                                "backend": backend,
                                "latency": spin1_bench.summarize(batches),
                                "pair_medians_ms": [
                                    statistics.median(record["batch_ms"])
                                    for record in records
                                ],
                                "first_call_ms": [record["first_ms"] for record in records],
                                "setup_ms": [record.get("setup_ms") for record in records],
                                "binary_sha256": sha256(binaries[backend]),
                            }
                        )
                    lookup = {
                        item["backend"]: item
                        for item in summaries
                        if item["case_id"] == case["case_id"]
                        and item["operation"] == operation
                        and item["threads"] == threads
                    }
                    comparisons[-1]["paired_ratios"] = {
                        "baseline_ducc": [
                            a / b
                            for a, b in zip(
                                lookup["baseline"]["pair_medians_ms"],
                                lookup["ducc"]["pair_medians_ms"],
                            )
                        ],
                        "candidate_ducc": [
                            a / b
                            for a, b in zip(
                                lookup["candidate"]["pair_medians_ms"],
                                lookup["ducc"]["pair_medians_ms"],
                            )
                        ],
                        "candidate_baseline": [
                            a / b
                            for a, b in zip(
                                lookup["candidate"]["pair_medians_ms"],
                                lookup["baseline"]["pair_medians_ms"],
                            )
                        ],
                    }
                    print(
                        f"done {case['case_id']} {operation} q{threads}: "
                        + ", ".join(
                            f"{backend}={lookup[backend]['latency']['median']:.3f} ms"
                            for backend in binaries
                        ),
                        flush=True,
                    )

    metadata = {
        "baseline_source": args.baseline_source,
        "candidate_source": args.candidate_source,
        "binaries": {backend: str(path) for backend, path in binaries.items()},
        "binary_sha256": {backend: sha256(path) for backend, path in binaries.items()},
        "ducc_revision": spin1_bench.PINNED_DUCC,
        "rust_toolchain": spin1_bench.common.command_output(["rustc", "-Vv"]),
        "rust_flags": spin1_bench.RUST_FLAGS,
        "ducc_compiler": "GCC 15.2.0",
        "ducc_flags": "-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math",
        "cpu_model": cpu_model(),
        "q1_cpus": cpus[:1],
        "q8_cpus": cpus[:8],
        "monitor_cpu": monitor_cpu,
        "measurement": {
            "process_pairs": args.process_pairs,
            "target_ms": args.target_ms,
            "interleaved_order": [
                "baseline,DUCC,candidate",
                "candidate,DUCC,baseline",
                "DUCC,baseline,candidate",
            ][: args.process_pairs],
        },
    }
    result = {
        "metadata": metadata,
        "cases": cases,
        "raw": raw,
        "comparisons": comparisons,
        "summaries": summaries,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
    print(f"wrote {args.output}: {len(raw)} runs, {len(comparisons)} checks")


if __name__ == "__main__":
    main()
