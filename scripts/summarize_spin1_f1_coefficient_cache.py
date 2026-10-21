#!/usr/bin/env python3
"""Remove raw process telemetry and emit reviewable spin-1 cache A/B summaries."""

from __future__ import annotations

import argparse
import csv
import json
import pathlib
import statistics


def quantile(values: list[float], fraction: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def median(values: list[float]) -> float | None:
    return statistics.median(values) if values else None


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw_json", type=pathlib.Path)
    parser.add_argument("output_dir", type=pathlib.Path)
    args = parser.parse_args()

    raw = json.loads(args.raw_json.read_text())
    summaries = raw["summaries"]
    rows: list[dict] = []
    paired: list[dict] = []
    for summary in summaries:
        key = (summary["case_id"], summary["operation"], summary["threads"])
        lookup = {
            item["backend"]: item
            for item in summaries
            if (item["case_id"], item["operation"], item["threads"]) == key
        }
        baseline_pairs = lookup["baseline"]["pair_medians_ms"]
        candidate_pairs = lookup["candidate"]["pair_medians_ms"]
        ducc_pairs = lookup["ducc"]["pair_medians_ms"]
        row = {
            "case_id": summary["case_id"],
            "grid": summary["grid"],
            "lmax": summary["lmax"],
            "mmax": summary["mmax"],
            "operation": summary["operation"],
            "threads": summary["threads"],
            "backend": summary["backend"],
            "median_ms": summary["latency"]["median"],
            "p10_ms": summary["latency"]["p10"],
            "p90_ms": summary["latency"]["p90"],
            "min_ms": summary["latency"]["min"],
            "max_ms": summary["latency"]["max"],
            "batch_count": summary["latency"]["count"],
            "process_pair_medians_ms": summary["pair_medians_ms"],
            "first_call_median_ms": median(summary["first_call_ms"]),
            "first_call_values_ms": summary["first_call_ms"],
            "setup_median_ms": median(summary["setup_ms"]),
            "setup_values_ms": summary["setup_ms"],
            "binary_sha256": summary["binary_sha256"],
            "paired_baseline_ducc_median": median(
                [a / b for a, b in zip(baseline_pairs, ducc_pairs)]
            ),
            "paired_candidate_ducc_median": median(
                [a / b for a, b in zip(candidate_pairs, ducc_pairs)]
            ),
            "paired_candidate_baseline_median": median(
                [a / b for a, b in zip(candidate_pairs, baseline_pairs)]
            ),
        }
        rows.append(row)
        if summary["backend"] == "baseline":
            comparisons = [
                item
                for item in raw["comparisons"]
                if (item["case_id"], item["operation"], item["threads"]) == key
            ]
            candidate_over_baseline = [a / b for a, b in zip(candidate_pairs, baseline_pairs)]
            candidate_over_ducc = [a / b for a, b in zip(candidate_pairs, ducc_pairs)]
            baseline_over_ducc = [a / b for a, b in zip(baseline_pairs, ducc_pairs)]
            paired.append(
                {
                    "case_id": summary["case_id"],
                    "grid": summary["grid"],
                    "lmax": summary["lmax"],
                    "mmax": summary["mmax"],
                    "operation": summary["operation"],
                    "threads": summary["threads"],
                    "baseline_ducc_pair_ratios": baseline_over_ducc,
                    "candidate_ducc_pair_ratios": candidate_over_ducc,
                    "candidate_baseline_pair_ratios": candidate_over_baseline,
                    "baseline_ducc_ratio_median": median(baseline_over_ducc),
                    "candidate_ducc_ratio_median": median(candidate_over_ducc),
                    "candidate_baseline_ratio_median": median(candidate_over_baseline),
                    "candidate_baseline_ratio_min": min(candidate_over_baseline),
                    "candidate_baseline_ratio_max": max(candidate_over_baseline),
                    "candidate_improvement_percent_median": 100.0
                    * (1.0 - median(candidate_over_baseline)),
                    "baseline_ducc_relative_l2_max": max(
                        item["baseline_vs_ducc"]["relative_l2"] for item in comparisons
                    ),
                    "candidate_ducc_relative_l2_max": max(
                        item["candidate_vs_ducc"]["relative_l2"] for item in comparisons
                    ),
                    "candidate_baseline_absolute_error_max": max(
                        item["candidate_vs_baseline"]["absolute_max"] for item in comparisons
                    ),
                    "candidate_baseline_relative_l2_max": max(
                        item["candidate_vs_baseline"]["relative_l2"] for item in comparisons
                    ),
                    "all_finite": all(
                        item.get("all_finite", item.get("finite", False))
                        for item in comparisons
                    ),
                    "reset_repeat_consistent": all(
                        all(values.values())
                        for item in comparisons
                        for values in (item["repeat_consistent"], item["reset_consistent"])
                    ),
                }
            )

    metadata = raw["metadata"]
    binary_hashes = metadata.get("binary_sha256") or {
        "baseline": metadata["baseline_binary_sha256"],
        "candidate": metadata["candidate_binary_sha256"],
        "ducc": metadata["ducc_binary_sha256"],
    }
    host_summaries = []
    host_keys = sorted(
        {
            (item["backend"], item["operation"], item["threads"])
            for item in raw["raw"]
        }
    )
    for backend, operation, threads in host_keys:
        records = [
            item
            for item in raw["raw"]
            if (item["backend"], item["operation"], item["threads"])
            == (backend, operation, threads)
        ]
        waits = [item["host_diagnostics"]["wait_fraction"] for item in records]
        switches = [item["host_diagnostics"]["involuntary_switches"] for item in records]
        frequencies = [
            value
            for item in records
            for value in item["host_diagnostics"].get("frequency_ghz", [])
        ]
        host_summaries.append(
            {
                "backend": backend,
                "operation": operation,
                "threads": threads,
                "wait_fraction": {
                    "median": median(waits),
                    "p10": quantile(waits, 0.1),
                    "p90": quantile(waits, 0.9),
                    "min": min(waits),
                    "max": max(waits),
                },
                "involuntary_switches": {
                    "median": median(switches),
                    "min": min(switches),
                    "max": max(switches),
                },
                "observed_frequency_ghz": {
                    "median": median(frequencies),
                    "min": min(frequencies),
                    "max": max(frequencies),
                },
            }
        )
    sanitized_metadata = {
        "baseline_source": metadata.get(
            "baseline_source", "0b2e1336409ba0bbe9fd6fa4cff70db0e327cd52"
        ),
        "candidate_source": metadata.get(
            "candidate_source", "7f47f06f875991a982caf0f4c396323a32c0d72b"
        ),
        "binary_sha256": binary_hashes,
        "ducc_revision": metadata["ducc_revision"],
        "rust_toolchain": metadata["rust_toolchain"],
        "rust_flags": metadata["rust_flags"],
        "ducc_compiler": metadata["ducc_compiler"],
        "ducc_flags": metadata["ducc_flags"],
        "cpu_model": metadata["cpu_model"],
        "q1_cpus": metadata["q1_cpus"],
        "q8_cpus": metadata["q8_cpus"],
        "monitor_cpu": metadata.get("monitor_cpu"),
        "measurement": metadata["measurement"],
        "fixture_sha256_by_case": [
            {
                "case_id": case_id,
                "sha256": next(
                    item["fixture_sha256"]
                    for item in raw["raw"]
                    if item["case_id"] == case_id
                ),
            }
            for case_id in sorted({item["case_id"] for item in raw["raw"]})
        ],
        "host_diagnostics_summary": host_summaries,
    }
    output = {
        "schema_version": 1,
        "metadata": sanitized_metadata,
        "cases": raw["cases"],
        "backend_summaries": rows,
        "paired_comparisons": paired,
    }
    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "summary.json").write_text(
        json.dumps(output, indent=2, allow_nan=False) + "\n"
    )
    columns = [
        "case_id",
        "grid",
        "lmax",
        "mmax",
        "operation",
        "threads",
        "backend",
        "median_ms",
        "p10_ms",
        "p90_ms",
        "min_ms",
        "max_ms",
        "batch_count",
        "process_pair_medians_ms",
        "first_call_median_ms",
        "first_call_values_ms",
        "setup_median_ms",
        "setup_values_ms",
        "paired_baseline_ducc_median",
        "paired_candidate_ducc_median",
        "paired_candidate_baseline_median",
        "binary_sha256",
    ]
    with (args.output_dir / "summary.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=columns, lineterminator="\n")
        writer.writeheader()
        for row in rows:
            writer.writerow(
                {
                    **row,
                    "process_pair_medians_ms": json.dumps(row["process_pair_medians_ms"]),
                    "first_call_values_ms": json.dumps(row["first_call_values_ms"]),
                    "setup_values_ms": json.dumps(row["setup_values_ms"]),
                }
            )
    print(f"wrote sanitized summaries to {args.output_dir}")


if __name__ == "__main__":
    main()
