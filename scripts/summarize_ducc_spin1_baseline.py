#!/usr/bin/env python3
"""Publish aggregate Plan 15 spin-1 benchmark rows without host/process logs."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path
from typing import Any


GRIDS = ("GL", "CC", "F1")
CSV_FIELDS = (
    "case_id",
    "grid",
    "lmax",
    "mmax",
    "ntheta",
    "nphi",
    "latitude_mode",
    "longitude_mode",
    "operation",
    "threads",
    "rust_median_ms",
    "rust_p10_ms",
    "rust_p90_ms",
    "rust_sample_count",
    "ducc_median_ms",
    "ducc_p10_ms",
    "ducc_p90_ms",
    "ducc_sample_count",
    "paired_ratio_median",
    "excess_median_ms",
    "rust_first_call_median_ms",
    "ducc_first_call_median_ms",
    "rust_setup_median_ms",
    "ducc_setup_median_ms",
    "absolute_max_error",
    "relative_l2_error",
    "max_relative_above_peak",
    "comparison_count",
    "finite_outputs",
    "repeat_consistent",
    "reset_consistent",
)


def median(values: list[float]) -> float | None:
    return statistics.median(values) if values else None


def safe_metadata(raw: dict[str, Any]) -> dict[str, Any]:
    metadata = raw["metadata"]
    result = {
        "rust_source_revision": metadata["rust_source_revision"],
        "rust_toolchain": metadata["rust_toolchain"],
        "rust_flags": metadata["rust_flags"],
        "rust_dependency_features": "Pulp 0.22.3 runtime dispatch",
        "rust_binary_sha256": metadata["rust_binary_sha256"],
        "rust_dispatch_path": (
            "Production spin-1 transform calls pulp::Arch::new(); the native "
            "benchmark worker record does not include a per-call arch label."
        ),
        "ducc_revision": metadata["ducc_revision"],
        "ducc_version": metadata["ducc_version"],
        "ducc_compiler": metadata["ducc_compiler"],
        "ducc_flags": metadata["ducc_flags"],
        "ducc_binary_sha256": metadata["ducc_binary_sha256"],
        "cpu_model": metadata["cpu_model"],
        "affinity_policy": (
            "q1 pinned to one physical core; q8 pinned to eight physical cores "
            "within one shared L3 domain"
        ),
        "measurement": metadata["measurement"],
        "call_boundary": metadata["call_boundary"],
        "case_count": len(raw["cases"]),
        "summary_row_count": len(raw["summaries"]),
        "comparison_count": len(raw["comparisons"]),
        "failure_count": len(raw["failures"]),
        "smoke": bool(metadata.get("smoke", False)),
        "coverage": {
            "lmax": sorted({case["lmax"] for case in raw["cases"]}),
            "latitude_modes": sorted(
                {case["latitude_mode"] for case in raw["cases"]}
            ),
            "longitude_modes": sorted(
                {case["longitude_mode"] for case in raw["cases"]}
            ),
            "mmax_modes": sorted(
                {
                    "full" if case["mmax"] == case["lmax"] else "reduced"
                    for case in raw["cases"]
                }
            ),
        },
    }
    if result["smoke"]:
        raise ValueError("smoke datasets are not valid publication inputs")
    return result


def grid_summary(rows: list[dict[str, Any]]) -> dict[str, Any]:
    groups: dict[tuple[str, int], list[dict[str, Any]]] = {}
    for row in rows:
        key = (row["operation"], row["threads"])
        groups.setdefault(key, []).append(row)

    result: dict[str, Any] = {}
    for (operation, threads), items in sorted(groups.items()):
        worst_excess = max(items, key=lambda row: row["excess_median_ms"])
        worst_ratio = max(items, key=lambda row: row["paired_ratio_median"])
        result[f"{operation}_q{threads}"] = {
            "configuration_count": len(items),
            "median_paired_ratio": statistics.median(
                row["paired_ratio_median"] for row in items
            ),
            "mean_paired_ratio": statistics.mean(
                row["paired_ratio_median"] for row in items
            ),
            "largest_absolute_excess": {
                "case_id": worst_excess["case_id"],
                "excess_median_ms": worst_excess["excess_median_ms"],
                "rust_median_ms": worst_excess["rust_median_ms"],
                "ducc_median_ms": worst_excess["ducc_median_ms"],
            },
            "largest_ratio": {
                "case_id": worst_ratio["case_id"],
                "paired_ratio_median": worst_ratio["paired_ratio_median"],
            },
        }
    return result


def summarize_grid(grid: str, input_path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    raw = json.loads(input_path.read_text())
    if raw["failures"]:
        raise ValueError(f"{grid} contains {len(raw['failures'])} failed cases")

    comparisons: dict[tuple[str, str, int], list[dict[str, Any]]] = {}
    for comparison in raw["comparisons"]:
        key = (
            comparison["case_id"],
            comparison["operation"],
            comparison["threads"],
        )
        comparisons.setdefault(key, []).append(comparison)

    rows: list[dict[str, Any]] = []
    for summary in raw["summaries"]:
        key = (summary["case_id"], summary["operation"], summary["threads"])
        checks = comparisons[key]
        rust = summary["rust_ms"]
        ducc = summary["ducc_ms"]
        row = {
            "case_id": summary["case_id"],
            "grid": grid,
            "lmax": summary["lmax"],
            "mmax": summary["mmax"],
            "ntheta": summary["ntheta"],
            "nphi": summary["nphi"],
            "latitude_mode": summary["latitude_mode"],
            "longitude_mode": summary["longitude_mode"],
            "operation": summary["operation"],
            "threads": summary["threads"],
            "rust_median_ms": rust["median"],
            "rust_p10_ms": rust["p10"],
            "rust_p90_ms": rust["p90"],
            "rust_sample_count": rust["count"],
            "ducc_median_ms": ducc["median"],
            "ducc_p10_ms": ducc["p10"],
            "ducc_p90_ms": ducc["p90"],
            "ducc_sample_count": ducc["count"],
            "paired_ratio_median": summary["paired_ratio_median"],
            "excess_median_ms": rust["median"] - ducc["median"],
            "rust_first_call_median_ms": median(summary["first_call_rust_ms"]),
            "ducc_first_call_median_ms": median(summary["first_call_ducc_ms"]),
            "rust_setup_median_ms": median(summary["setup_rust_ms"]),
            "ducc_setup_median_ms": median(summary["setup_ducc_ms"]),
            "absolute_max_error": max(c["absolute_max"] for c in checks),
            "relative_l2_error": max(c["relative_l2"] for c in checks),
            "max_relative_above_peak": max(
                c["max_relative_above_1e-12_peak"] for c in checks
            ),
            "comparison_count": len(checks),
            "finite_outputs": all(c["finite_outputs"] for c in checks),
            "repeat_consistent": all(
                all(c["repeat_consistent"].values()) for c in checks
            ),
            "reset_consistent": all(
                all(c["reset_consistent"].values()) for c in checks
            ),
        }
        rows.append(row)

    metadata = safe_metadata(raw)
    metadata["max_absolute_error"] = max(c["absolute_max"] for c in raw["comparisons"])
    metadata["max_relative_l2_error"] = max(c["relative_l2"] for c in raw["comparisons"])
    metadata["max_relative_above_peak"] = max(
        c["max_relative_above_1e-12_peak"] for c in raw["comparisons"]
    )
    metadata["all_finite"] = all(c["finite_outputs"] for c in raw["comparisons"])
    metadata["all_repeat_consistent"] = all(
        all(c["repeat_consistent"].values()) for c in raw["comparisons"]
    )
    metadata["all_reset_consistent"] = all(
        all(c["reset_consistent"].values()) for c in raw["comparisons"]
    )
    metadata["aggregate_performance"] = grid_summary(rows)
    return metadata, rows


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)

    all_metadata: dict[str, Any] = {}
    all_rows: list[dict[str, Any]] = []
    for grid in GRIDS:
        metadata, rows = summarize_grid(
            grid, args.input_dir / grid / "results.json"
        )
        all_metadata[grid] = metadata
        all_rows.extend(rows)

    all_rows.sort(
        key=lambda row: (
            row["grid"],
            row["lmax"],
            row["mmax"],
            row["ntheta"],
            row["nphi"],
            row["operation"],
            row["threads"],
        )
    )
    csv_path = args.output_dir / "full-transform-summary.csv"
    with csv_path.open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=CSV_FIELDS)
        writer.writeheader()
        writer.writerows({key: row[key] for key in CSV_FIELDS} for row in all_rows)

    payload = {
        "title": "Plan 15 native spin-1 GL/CC/F1 baseline",
        "rows": all_rows,
        "grid_metadata": all_metadata,
    }
    (args.output_dir / "full-transform-summary.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n"
    )
    print(
        f"Wrote {len(all_rows)} aggregate timing rows across "
        f"{sum(x['case_count'] for x in all_metadata.values())} geometry cases; "
        f"numerical comparisons={sum(x['comparison_count'] for x in all_metadata.values())}; "
        f"max relative L2={max(x['max_relative_l2_error'] for x in all_metadata.values()):.3e}"
    )


if __name__ == "__main__":
    main()
