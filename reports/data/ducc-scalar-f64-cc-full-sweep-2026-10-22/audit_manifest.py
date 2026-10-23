#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Independently audit native CC sweep shards without importing the runner."""

import argparse
import csv
import hashlib
import json
import math
import pathlib
from collections import Counter, defaultdict


HIGH_LMAX = (
    257, 258, 300, 320, 384, 511, 512, 513, 640, 720, 721, 768,
    896, 1023, 1024, 1025, 1280, 1536, 1792, 2047, 2048, 2049,
    2560, 3072,
)
ODD_MINIMUM_LMAX = frozenset({
    0, 1, 2, 3, 4, 8, 16, 32, 48, 62, 63, 64, 65, 127, 128, 159,
    191, 223, 248, 254, 255, 256, 257, 511, 512, 513, 720, 721,
    1023, 1024, 1025, 2047, 2048, 2049, 2560, 3072,
})
PROCESS_PAIRS = 3
BATCHES_PER_PROCESS = 10
RANDOM_RELATIVE_L2_LIMIT = 5e-12
VALIDATION_RELATIVE_L2_LIMIT = 5e-11
ADJOINT_RELATIVE_RESIDUAL_LIMIT = 5e-11
NUMERICAL_METRIC_CONTRACT = (
    "complex-magnitude-relative-l2-v2;"
    "zero-reference-subnormal-abs-below-min-normal-v1"
)
SUBNORMAL_ABSOLUTE_ERROR_LIMIT = float.fromhex("0x1.0000000000000p-1022")


def smooth_even_longitude(mmax: int) -> int:
    value = max(2, 2 * mmax + 2)
    while True:
        remainder = value
        for factor in (2, 3, 5, 7):
            while remainder % factor == 0:
                remainder //= factor
        if remainder == 1:
            return value
        value += 2


def expected_cases() -> dict[str, dict]:
    cases = {}
    for lmax in (*range(257), *HIGH_LMAX):
        for mmax in {lmax, lmax // 4}:
            modes = {"even_smooth"}
            if lmax in ODD_MINIMUM_LMAX:
                modes.add("odd_minimum")
            for mode in modes:
                nphi = (smooth_even_longitude(mmax) if mode == "even_smooth"
                        else 2 * mmax + 1)
                case_id = f"CC-T{lmax}-m{mmax}-{mode}"
                cases[case_id] = {
                    "case_id": case_id,
                    "grid": "CC",
                    "lmax": lmax,
                    "mmax": mmax,
                    "ntheta": lmax + 2,
                    "nphi": nphi,
                    "longitude_mode": mode,
                }
    return cases


def finite_number(value) -> bool:
    return not isinstance(value, bool) and isinstance(value, (int, float)) and math.isfinite(value)


def relative_values(tree, field: str) -> list[float]:
    if isinstance(tree, dict):
        result = []
        for key, value in tree.items():
            if key == field and finite_number(value):
                result.append(float(value))
            else:
                result.extend(relative_values(value, field))
        return result
    if isinstance(tree, list):
        return [number for value in tree for number in relative_values(value, field)]
    return []


def zero_reference_subnormal_metric_acceptable(error: dict, expected_count: int) -> bool:
    absolute_max = error.get("absolute_max")
    return (
        error.get("relative_l2") is None
        and error.get("reference_peak") == 0.0
        and error.get("near_zero_reference_count") == expected_count
        and finite_number(absolute_max)
        and absolute_max <= SUBNORMAL_ABSOLUTE_ERROR_LIMIT
    )


def zero_reference_exact_metric_acceptable(error: dict, expected_count: int) -> bool:
    return (
        error.get("reference_peak") == 0.0
        and error.get("absolute_max") == 0.0
        and error.get("relative_l2") == 0.0
        and error.get("near_zero_reference_count") == expected_count
        and error.get("metrics_finite") is True
    )


def _metric_reasons(metric, label: str, limit: float,
                    expected_count: int, allow_zero_reference_subnormal: bool = False,
                    require_exact_zero: bool = False) -> list[str]:
    if not isinstance(metric, dict):
        return [f"{label}_metrics_missing"]
    reasons = []
    for field in ("absolute_max", "reference_peak", "max_relative_above_1e-12_peak"):
        if not finite_number(metric.get(field)) or metric[field] < 0.0:
            reasons.append(f"{label}_{field}_missing_or_nonfinite")
    near_zero_count = metric.get("near_zero_reference_count")
    if (not isinstance(near_zero_count, int) or isinstance(near_zero_count, bool) or near_zero_count < 0
            or near_zero_count > expected_count):
        reasons.append(f"{label}_near_zero_reference_count_invalid")
    relative_l2 = metric.get("relative_l2")
    if require_exact_zero:
        if (metric.get("absolute_max") != 0.0 or not finite_number(relative_l2)
                or relative_l2 != 0.0):
            reasons.append(f"{label}_not_exact_zero")
        if (metric.get("reference_peak") != 0.0
                or near_zero_count != expected_count):
            reasons.append(f"{label}_zero_reference_record_invalid")
        if metric.get("metrics_finite") is not True:
            reasons.append(f"{label}_metrics_not_finite")
    elif (allow_zero_reference_subnormal
          and zero_reference_subnormal_metric_acceptable(metric, expected_count)):
        # The relative norm is undefined only for an exactly zero DUCC reference.
        # The finite absolute error still has the strict subnormal-sized gate above.
        if metric.get("metrics_finite") is not False:
            reasons.append(f"{label}_metrics_finite_flag_inconsistent")
    elif not finite_number(relative_l2):
        reasons.append(f"{label}_relative_l2_missing_or_nonfinite")
    elif metric.get("metrics_finite") is not True:
        reasons.append(f"{label}_metrics_not_finite")
    if (not require_exact_zero and finite_number(metric.get("reference_peak"))
            and metric["reference_peak"] == 0.0
            and not (allow_zero_reference_subnormal
                     and zero_reference_subnormal_metric_acceptable(metric, expected_count))
            and not (allow_zero_reference_subnormal
                     and zero_reference_exact_metric_acceptable(metric, expected_count))):
        reasons.append(f"{label}_unexpected_zero_reference")
    if finite_number(relative_l2) and relative_l2 < 0.0:
        reasons.append(f"{label}_relative_l2_invalid")
    if finite_number(relative_l2) and relative_l2 > limit:
        reasons.append(f"{label}_relative_l2_exceeded")
    return reasons


def random_numeric_reasons(record: dict | None) -> list[str]:
    if not isinstance(record, dict):
        return ["random_numeric_record_missing"]
    if record.get("finite_outputs") is not True:
        return ["random_output_nonfinite"]
    geometry = (record.get("lmax"), record.get("mmax"),
                record.get("ntheta"), record.get("nphi"))
    if not all(isinstance(value, int) and not isinstance(value, bool) for value in geometry):
        return ["random_numeric_geometry_missing"]
    lmax, mmax, ntheta, nphi = geometry
    if lmax < 0 or mmax < 0 or mmax > lmax or ntheta <= 0 or nphi <= 0:
        return ["random_numeric_geometry_invalid"]
    expected_count = (
        ntheta * nphi if record.get("operation") == "synthesis"
        else (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) // 2
    )
    return _metric_reasons(
        record, "random_output", RANDOM_RELATIVE_L2_LIMIT, expected_count
    )


def validation_reasons(validation: dict | None, expected_case: dict | None = None) -> list[str]:
    if validation is None:
        return ["missing_validation"]
    if not isinstance(validation, dict):
        return ["malformed_validation"]
    reasons = []
    if validation.get("finite_outputs") is not True:
        reasons.append("nonfinite_validation_output")
    lmax, mmax = validation.get("lmax"), validation.get("mmax")
    ntheta, nphi = validation.get("ntheta"), validation.get("nphi")
    if not all(isinstance(value, int) and not isinstance(value, bool)
               for value in (lmax, mmax, ntheta, nphi)):
        return reasons + ["validation_geometry_missing"]
    if (lmax < 0 or mmax < 0 or mmax > lmax or ntheta <= 0 or nphi <= 0):
        reasons.append("validation_geometry_invalid")
    if expected_case is not None and any(
        validation.get(field) != expected_case.get(field)
        for field in ("lmax", "mmax", "ntheta", "nphi")
    ):
        reasons.append("validation_geometry_mismatch")
    nmap = ntheta * nphi
    nalm = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) // 2

    expected_finite_flags = {"zero_finite", "extreme_finite", "subnormal_finite"}
    finite_flags = validation.get("finite_flags")
    reset_flags = validation.get("reset_consistent")
    if not isinstance(finite_flags, dict) or set(finite_flags) != {"rustsht", "ducc"}:
        reasons.append("finite_flags_backend_set_incomplete")
    if not isinstance(reset_flags, dict) or set(reset_flags) != {"rustsht", "ducc"}:
        reasons.append("reset_consistent_backend_set_incomplete")
    for backend in ("rustsht", "ducc"):
        flags = finite_flags.get(backend) if isinstance(finite_flags, dict) else None
        if not isinstance(flags, dict) or set(flags) != expected_finite_flags:
            reasons.append(f"{backend}_finite_flags_incomplete")
        elif any(value is not True for value in flags.values()):
            reasons.append(f"{backend}_finite_flags_failed")
        reset = reset_flags.get(backend) if isinstance(reset_flags, dict) else None
        if reset is not True:
            reasons.append(f"{backend}_validation_reset_failed")

    roundtrip = validation.get("roundtrip")
    expected_roundtrip = {"rustsht", "ducc", "rust_ducc"}
    if not isinstance(roundtrip, dict) or set(roundtrip) != expected_roundtrip:
        reasons.append("roundtrip_backend_set_incomplete")
    for backend in expected_roundtrip:
        reasons.extend(_metric_reasons(
            roundtrip.get(backend) if isinstance(roundtrip, dict) else None,
            f"roundtrip_{backend}", VALIDATION_RELATIVE_L2_LIMIT, nalm,
        ))

    expected_basis_modes = {
        (0, 0),
        (lmax, 0),
        (lmax // 2, min(mmax, lmax // 2)),
        (lmax, mmax),
    }
    basis_records = validation.get("basis")
    if not isinstance(basis_records, list):
        basis_records = []
        reasons.append("basis_records_missing")
    observed_basis_modes = {
        (item.get("l"), item.get("m"))
        for item in basis_records if isinstance(item, dict)
    }
    if (observed_basis_modes != expected_basis_modes
            or len(basis_records) != len(expected_basis_modes)):
        reasons.append("basis_record_set_incomplete")
    for item in basis_records:
        if not isinstance(item, dict):
            reasons.append("basis_record_malformed")
            continue
        label = f"basis_l{item.get('l')}_m{item.get('m')}"
        for field, count in (
            ("map_rust_ducc", nmap),
            ("analysis_of_synthesis_rust", nalm),
            ("analysis_of_synthesis_ducc", nalm),
        ):
            reasons.extend(_metric_reasons(
                item.get(field), f"{label}_{field}",
                VALIDATION_RELATIVE_L2_LIMIT, count,
            ))

    adjoints = validation.get("adjoints")
    if not isinstance(adjoints, dict) or set(adjoints) != {"rustsht", "ducc"}:
        reasons.append("adjoint_backend_set_incomplete")
    for backend in ("rustsht", "ducc"):
        backend_adjoints = adjoints.get(backend) if isinstance(adjoints, dict) else None
        for operation in ("synthesis", "analysis"):
            value = (backend_adjoints.get(operation + "_relative_residual")
                     if isinstance(backend_adjoints, dict) else None)
            if not finite_number(value) or value < 0.0:
                reasons.append(f"{backend}_{operation}_adjoint_missing_or_nonfinite")
            elif value > ADJOINT_RELATIVE_RESIDUAL_LIMIT:
                reasons.append(f"{backend}_{operation}_adjoint_exceeded")

    edges = validation.get("edge_cases")
    expected_edges = {"zero", "extreme", "subnormal"}
    if not isinstance(edges, dict) or set(edges) != expected_edges:
        reasons.append("edge_case_record_set_incomplete")
    for name in ("zero", "extreme", "subnormal"):
        edge = edges.get(name) if isinstance(edges, dict) else None
        if not isinstance(edge, dict):
            reasons.append(f"{name}_edge_case_missing")
            continue
        if name == "zero" and edge.get("exact_zero") is not True:
            reasons.append("exact_zero_failed")
        for field, count in (("map_rust_ducc", nmap), ("alm_rust_ducc", nalm)):
            reasons.extend(_metric_reasons(
                edge.get(field), f"{name}_{field}",
                VALIDATION_RELATIVE_L2_LIMIT, count,
                allow_zero_reference_subnormal=(name == "subnormal"),
                require_exact_zero=(name == "zero"),
            ))
    return reasons


def audited_numerical_status(complete: bool, numeric_reasons: list[str],
                             validation: dict | None, reset_status: str,
                             expected_case: dict | None = None) -> tuple[str, list[str]]:
    check_reasons = validation_reasons(validation, expected_case)
    if numeric_reasons or reset_status == "rejected" or (validation is not None and check_reasons):
        return "rejected", check_reasons
    if not complete or reset_status != "accepted" or validation is None:
        return "incomplete", check_reasons
    return "accepted", check_reasons


def independent_full_sweep_pass(expected_groups: list[dict], missing_cases: list[str],
                                geometry_mismatches: list[dict], missing_groups: list[str],
                                timing_incomplete: list[str], duration_incomplete: list[str],
                                failures: list[dict], duplicates: dict,
                                identities: dict, metric_contract_pass: bool) -> bool:
    return (
        not missing_cases and not geometry_mismatches and not missing_groups
        and not timing_incomplete and not duration_incomplete and not failures
        and not any(duplicates.values())
        and bool(expected_groups)
        and all(row.get("status") == "complete" for row in expected_groups)
        and all(row.get("numerical_status") == "accepted" for row in expected_groups)
        and all(row.get("timing_quality_status") == "qualified" for row in expected_groups)
        and all(row.get("reset_consistency_status") == "accepted" for row in expected_groups)
        and all(len(values) == 1 for values in identities.values())
        and metric_contract_pass
    )


def audit(shard_paths: list[pathlib.Path], output_dir: pathlib.Path) -> dict:
    docs = [json.loads(path.read_text()) for path in shard_paths]
    expected = expected_cases()
    cases: dict[str, dict] = {}
    rows: dict[tuple, dict] = {}
    samples: dict[tuple, dict] = {}
    numerics: dict[tuple, dict] = {}
    validations: dict[tuple, dict] = {}
    failures = []
    duplicates = Counter()
    identity_fields = (
        "production_source_revision", "binary_source_revision", "ducc_source_revision", "rust_binary_sha256",
        "ducc_binary_sha256", "rustc", "cargo", "cxx", "rust_flags", "ducc_flags",
        "numerical_metric_contract",
    )
    identities = defaultdict(set)
    harnesses = set()
    build_records = {}
    verified_record_shards = 0

    for path, doc in zip(shard_paths, docs):
        metadata = doc.get("metadata", {})
        for field in identity_fields:
            value = metadata.get(field)
            if field == "binary_source_revision" and value is None:
                value = metadata.get("production_source_revision")
            identities[field].add(value)
        harnesses.add((metadata.get("benchmark_harness_revision", metadata.get("rust_source_revision")),
                       metadata.get("benchmark_harness_sha256")))
        record = metadata.get("binary_build_record")
        if record:
            build_records[record.get("sha256")] = record
        verified_record_shards += metadata.get("binary_provenance_verified") is True
        for case in doc.get("cases", []):
            case_id = case["case_id"]
            if case_id in cases:
                duplicates["cases"] += 1
            cases[case_id] = case
        for row in doc.get("rows", []):
            key = (row["case_id"], row["operation"], row["threads"])
            if key in rows:
                duplicates["rows"] += 1
            rows[key] = row
        for sample in doc.get("samples", []):
            key = (sample["case_id"], sample["operation"], sample["threads"],
                   sample["backend"], sample["process_pair"])
            if key in samples:
                duplicates["samples"] += 1
            samples[key] = sample
        for numeric in doc.get("numerics", []):
            key = (numeric["case_id"], numeric["operation"], numeric["threads"],
                   numeric["process_pair"])
            if key in numerics:
                duplicates["numerics"] += 1
            numerics[key] = numeric
        for validation in doc.get("validation", []):
            key = (validation["case_id"], validation["threads"])
            if key in validations:
                duplicates["validation"] += 1
            validations[key] = validation
        failures.extend(doc.get("failures", []))

    actual_groups = {
        (case_id, operation, threads)
        for case_id, operation, threads in rows
    }
    expected_groups = {
        (case_id, operation, threads)
        for case_id in expected
        for operation in ("analysis", "synthesis")
        for threads in (1, 8)
    }
    all_groups = sorted(expected_groups | actual_groups)
    group_records = []
    max_random = (None, None)
    max_validation = (None, None)
    geometry_mismatches = []
    for case_id in sorted(set(cases) & set(expected)):
        actual = cases[case_id]
        target = expected[case_id]
        differing = [
            field for field in ("grid", "lmax", "mmax", "ntheta", "nphi", "longitude_mode")
            if actual.get(field) != target[field]
        ]
        if differing:
            geometry_mismatches.append({"case_id": case_id, "fields": differing})
    for case_id, operation, threads in all_groups:
        case = cases.get(case_id) or expected.get(case_id, {})
        expected_group = (case_id, operation, threads) in expected_groups
        group_samples = [
            value for key, value in samples.items()
            if key[:3] == (case_id, operation, threads)
        ]
        group_numerics = [
            value for key, value in numerics.items()
            if key[:3] == (case_id, operation, threads)
        ]
        pair_sets = {
            backend: {sample["process_pair"] for sample in group_samples
                      if sample["backend"] == backend}
            for backend in ("rustsht", "ducc")
        }
        numeric_pairs = {item["process_pair"] for item in group_numerics}
        common_pairs = pair_sets["rustsht"] & pair_sets["ducc"] & numeric_pairs
        complete = (
            common_pairs == set(range(PROCESS_PAIRS))
            and pair_sets["rustsht"] == set(range(PROCESS_PAIRS))
            and pair_sets["ducc"] == set(range(PROCESS_PAIRS))
            and numeric_pairs == set(range(PROCESS_PAIRS))
        )
        reset_status = (
            "rejected" if any(value.get("reset_consistent") is not True for value in group_samples)
            else "accepted" if len(group_samples) == 2 * PROCESS_PAIRS
            else "incomplete"
        )
        numeric_reasons = []
        for item in group_numerics:
            numeric_reasons.extend(random_numeric_reasons(item))
            value = item.get("relative_l2")
            if finite_number(value) and (max_random[0] is None or value > max_random[0]):
                max_random = (value, item["case_id"])
        validation = validations.get((case_id, threads))
        expected_case = expected.get(case_id)
        numerical_status, check_reasons = audited_numerical_status(
            complete, numeric_reasons, validation, reset_status, expected_case
        )
        validation_values = relative_values(validation or {}, "relative_l2")
        if validation_values:
            value = max(validation_values)
            if max_validation[0] is None or value > max_validation[0]:
                max_validation = (value, case_id)
        validation_status = (
            "incomplete" if validation is None
            else "accepted" if not check_reasons
            else "rejected"
        )
        timing_qualified = complete and all(
            len(sample.get("batch_ms", [])) >= (
                5 if float(sample.get("warmup_ms", 0.0)) >= 100.0
                else BATCHES_PER_PROCESS
            )
            for sample in group_samples
        )
        timing_status = "qualified" if timing_qualified else "insufficient"
        target_batches = [
            duration for sample in group_samples
            if float(sample.get("warmup_ms", 0.0)) < 100.0
            for duration in sample.get("batch_total_ms", [])
        ]
        if target_batches:
            target_status = (
                "within_target" if all(20.0 <= value <= 100.0 for value in target_batches)
                else "partially_outside_target"
            )
        elif group_samples and all(float(sample.get("warmup_ms", 0.0)) >= 100.0
                                   for sample in group_samples):
            target_status = "not_applicable"
        else:
            target_status = "mixed_or_unavailable"
        group_status = "complete" if complete else "missing"
        if expected_group and case_id in {item["case_id"] for item in geometry_mismatches}:
            group_status = "geometry_mismatch"
        if (case_id, operation, threads) not in rows and group_samples:
            group_status = "missing_summary_row"
        group_records.append({
            "case_id": case_id,
            "grid": case.get("grid", "CC"),
            "lmax": case.get("lmax"),
            "mmax": case.get("mmax"),
            "ntheta": case.get("ntheta"),
            "nphi": case.get("nphi"),
            "longitude_mode": case.get("longitude_mode"),
            "expected": expected_group,
            "operation": operation,
            "threads": threads,
            "status": group_status,
            "numerical_status": numerical_status,
            "validation_status": validation_status,
            "timing_quality_status": timing_status,
            "duration_window_status": target_status,
            "reset_consistency_status": reset_status,
            "rust_process_pairs": ";".join(map(str, sorted(pair_sets["rustsht"]))),
            "ducc_process_pairs": ";".join(map(str, sorted(pair_sets["ducc"]))),
            "numeric_process_pairs": ";".join(map(str, sorted(numeric_pairs))),
            "rust_batch_count": sum(
                len(sample.get("batch_ms", [])) for sample in group_samples
                if sample["backend"] == "rustsht"
            ),
            "ducc_batch_count": sum(
                len(sample.get("batch_ms", [])) for sample in group_samples
                if sample["backend"] == "ducc"
            ),
            "random_relative_l2_max": max(
                (item["relative_l2"] for item in group_numerics
                 if finite_number(item.get("relative_l2"))), default=None
            ),
            "validation_failures": ";".join(sorted(set(check_reasons))),
        })

    output_dir.mkdir(parents=True, exist_ok=True)
    fields = list(group_records[0]) if group_records else []
    with (output_dir / "independent-coverage-manifest.csv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(group_records)

    expected_actual = set(cases) & set(expected)
    extras = sorted(set(cases) - set(expected))
    missing_cases = sorted(set(expected) - set(cases))
    missing_groups = [
        row["case_id"] + "|" + row["operation"] + "|q" + str(row["threads"])
        for row in group_records if row["expected"] and row["status"] != "complete"
    ]
    rejected = [
        row["case_id"] + "|" + row["operation"] + "|q" + str(row["threads"])
        for row in group_records if row["expected"] and row["numerical_status"] == "rejected"
    ]
    numerical_incomplete = [
        row["case_id"] + "|" + row["operation"] + "|q" + str(row["threads"])
        for row in group_records if row["expected"] and row["numerical_status"] != "accepted"
    ]
    timing_incomplete = [
        row["case_id"] + "|" + row["operation"] + "|q" + str(row["threads"])
        for row in group_records if row["expected"] and row["timing_quality_status"] != "qualified"
    ]
    duration_incomplete = [
        row["case_id"] + "|" + row["operation"] + "|q" + str(row["threads"])
        for row in group_records if row["expected"]
        and row["duration_window_status"] not in {"within_target", "not_applicable"}
    ]
    expected_group_records = [row for row in group_records if row["expected"]]
    counts = {
        "measurement_status": dict(Counter(row["status"] for row in expected_group_records)),
        "numerical_status": dict(Counter(row["numerical_status"] for row in expected_group_records)),
        "validation_status": dict(Counter(row["validation_status"] for row in expected_group_records)),
        "timing_quality_status": dict(Counter(row["timing_quality_status"] for row in expected_group_records)),
        "duration_window_status": dict(Counter(row["duration_window_status"] for row in expected_group_records)),
        "reset_consistency_status": dict(Counter(row["reset_consistency_status"] for row in expected_group_records)),
    }
    metric_contract_pass = identities["numerical_metric_contract"] == {
        NUMERICAL_METRIC_CONTRACT
    }
    audit = {
        "audit_method": "independent stdlib manifest and raw-record audit; does not import benchmark runner",
        "input_shards": [
            {"shard": path.parent.name, "path": path.parent.name + "/results.json",
             "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
            for path in shard_paths
        ],
        "source_and_build_identity": {
            field: sorted(values, key=lambda value: "" if value is None else str(value))
            for field, values in identities.items()
        },
        "harness_variants": [
            {"revision": revision, "sha256": digest}
            for revision, digest in sorted(harnesses, key=lambda pair: (str(pair[0]), str(pair[1])))
        ],
        "validated_binary_build_records": [
            {"sha256": digest, "binary_source_revision": record.get("binary_source_revision"),
             "rust_binary_sha256": record.get("rust_binary_sha256"),
             "ducc_binary_sha256": record.get("ducc_binary_sha256")}
            for digest, record in sorted(build_records.items())
        ],
        "shards_with_build_record_verified_at_capture": verified_record_shards,
        "source_identity_mismatch_fields": sorted(
            field for field, values in identities.items() if len(values) != 1
        ),
        "numerical_metric_contract": {
            "expected": NUMERICAL_METRIC_CONTRACT,
            "observed": sorted(str(value) for value in identities["numerical_metric_contract"]),
            "pass": metric_contract_pass,
        },
        "shard_count": len(shard_paths),
        "expected_geometry_count": len(expected),
        "observed_expected_geometry_count": len(expected_actual),
        "missing_geometry_count": len(missing_cases),
        "missing_geometry_ids": missing_cases,
        "geometry_mismatch_count": len(geometry_mismatches),
        "geometry_mismatches": geometry_mismatches,
        "extra_geometry_count": len(extras),
        "extra_geometry_ids": extras,
        "expected_group_count": len(expected_groups),
        "observed_expected_group_count": sum(
            row["expected"] and row["status"] == "complete" for row in group_records
        ),
        "missing_group_count": len(missing_groups),
        "missing_group_ids": missing_groups,
        "rejected_group_count": len(rejected),
        "rejected_group_ids": rejected,
        "numerical_incomplete_group_count": len(numerical_incomplete),
        "numerical_incomplete_group_ids": numerical_incomplete,
        "timing_incomplete_group_count": len(timing_incomplete),
        "timing_incomplete_group_ids": timing_incomplete,
        "duration_window_incomplete_group_count": len(duration_incomplete),
        "duration_window_incomplete_group_ids": duration_incomplete,
        "status_counts": counts,
        "duplicate_record_counts": dict(duplicates),
        "failure_record_count": len(failures),
        "maximum_random_relative_l2": {"value": max_random[0], "case_id": max_random[1]},
        "maximum_validation_relative_l2": {"value": max_validation[0], "case_id": max_validation[1]},
        "tolerances": {
            "random_relative_l2": RANDOM_RELATIVE_L2_LIMIT,
            "roundtrip_and_basis_relative_l2": VALIDATION_RELATIVE_L2_LIMIT,
            "adjoint_relative_residual": ADJOINT_RELATIVE_RESIDUAL_LIMIT,
            "required_process_pairs_per_backend": PROCESS_PAIRS,
            "short_batches_per_process": BATCHES_PER_PROCESS,
            "long_individual_samples_per_process": 5,
        },
        "full_sweep_pass": independent_full_sweep_pass(
            expected_group_records, missing_cases, geometry_mismatches, missing_groups,
            timing_incomplete, duration_incomplete, failures, duplicates,
            identities, metric_contract_pass,
        ),
    }
    (output_dir / "independent-audit.json").write_text(
        json.dumps(audit, indent=2, allow_nan=False) + "\n"
    )
    return audit


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--shards", type=pathlib.Path, nargs="+", required=True)
    parser.add_argument("--output-dir", type=pathlib.Path, required=True)
    args = parser.parse_args()
    files = []
    for path in args.shards:
        files.extend(sorted(path.rglob("results.json")) if path.is_dir() else [path])
    result = audit(files, args.output_dir)
    print(json.dumps({key: value for key, value in result.items()
                      if not key.endswith("_ids") and key != "input_shards"}, indent=2))


if __name__ == "__main__":
    main()
