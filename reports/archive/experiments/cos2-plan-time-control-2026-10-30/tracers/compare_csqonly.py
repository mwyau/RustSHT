#!/usr/bin/env python3
"""Compare isolated plan-time cosine-squared outputs with the archived independent references."""
from __future__ import annotations

import csv
import gzip
import json
import re
import struct
from collections import defaultdict
from decimal import Decimal, getcontext
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
ROOT = Path(__file__).resolve().parents[5]
REFERENCE = ROOT / "reports/archive/experiments/numerical-causality-ducc-0.42.0-2026-10-29/data"
getcontext().prec = 100


def f32(bits: str) -> float:
    return struct.unpack(">f", int(bits, 16).to_bytes(4, "big"))[0]


def decimal_from_hex_float(value: str) -> Decimal:
    sign = 1
    if value.startswith("-"):
        sign, value = -1, value[1:]
    elif value.startswith("+"):
        value = value[1:]
    mantissa, exponent_text = value[2:].split("p")
    whole, _, fractional = mantissa.partition(".")
    exponent = int(exponent_text) - 4 * len(fractional)
    result = Decimal(int(whole + fractional, 16)) * (Decimal(2) ** exponent)
    return result.copy_sign(Decimal(sign))


def exact_f32(bits: str) -> Decimal:
    return Decimal.from_float(f32(bits))


def rms_abs(errors: list[Decimal]) -> float:
    mean_square = sum((error * error for error in errors), Decimal(0)) / len(errors)
    return float(mean_square.sqrt())


def read_mode_reference():
    with gzip.open(REFERENCE / "mode-comparison.csv.gz", "rt", newline="") as stream:
        return {
            (int(row["m"]), int(row["l"]), int(row["ring"]), row["part"]): row
            for row in csv.DictReader(stream)
        }


def read_plan_values(path: Path, compressed: bool) -> dict[tuple[int, int], tuple[str, ...]]:
    pattern = re.compile(
        r"^PLAN,(\d+),(\d+),[^,]+,theta=([^,]+),sin=([^,]+),cos=([^,]+),"
        r"csq=([^,]+),sectoral=([^,]+),pair=([^,]+),alpha=([^,]+),a=([^,]+),b=([^,]+)"
    )
    opener = gzip.open if compressed else open
    values = {}
    with opener(path, "rt") as stream:
        for line in stream:
            match = pattern.match(line)
            if match:
                m, ring, *fields = match.groups()
                values[int(m), int(ring)] = tuple(fields)
    return values


def read_stage_outputs():
    result = {}
    pattern = re.compile(
        r"^OUTPUT,(\d+),(\d+),(re|im)_l(\d+),.*?"
        r"north=\(([^,]+),([^\)]+)\)"
    )
    with gzip.open(PACKAGE / "data/candidate-stage-outputs.csv.gz", "rt") as stream:
        for line in stream:
            match = pattern.match(line)
            if not match:
                raise ValueError(f"bad candidate output row: {line[:160]}")
            m, ring, part, degree = match.group(1, 2, 3, 4)
            real_bits, imag_bits = match.group(5, 6)
            result[int(m), int(degree), int(ring), part] = (
                real_bits if part == "re" else imag_bits
            )
    return result


def read_parent_stage_outputs():
    result = {}
    pattern = re.compile(
        r"^OUTPUT,(\d+),(\d+),(re|im)_l(\d+),.*?"
        r"north=\(([^,]+),([^\)]+)\)"
    )
    with gzip.open(REFERENCE / "rust-modes.csv.gz", "rt") as stream:
        for line in stream:
            match = pattern.match(line)
            if not match:
                continue
            m, ring, part, degree = match.group(1, 2, 3, 4)
            real_bits, imag_bits = match.group(5, 6)
            result[int(m), int(degree), int(ring), part] = (
                real_bits if part == "re" else imag_bits
            )
    return result


def read_public_map(path: Path):
    values = {}
    with gzip.open(path, "rt", newline="") as stream:
        for row in csv.reader(stream):
            if not row or row[0] != "MAP":
                continue
            key = tuple(row[1:5])
            if len(row[8:]) != 34:
                raise ValueError(f"expected 34 map values, got {len(row[8:])}")
            for phi, bits in enumerate(row[8:]):
                values[key + (str(phi),)] = bits
    return values


def read_public_reference():
    values = {}
    with gzip.open(REFERENCE / "longdouble-public-map.csv.gz", "rt", newline="") as stream:
        for row in csv.reader(stream):
            if row and row[0] == "REF_MAP":
                # Preserve both the independent extended-precision value and its
                # correctly rounded binary32 result.
                values[tuple(row[1:6])] = (row[7], decimal_from_hex_float(row[6]))
    return values


def main():
    mode_reference = read_mode_reference()
    stage = read_stage_outputs()
    parent_stage = read_parent_stage_outputs()
    if len(stage) != 420:
        raise ValueError(f"expected 420 single-mode stage outputs; found {len(stage)}")
    if len(parent_stage) != 420:
        raise ValueError(f"expected 420 parent single-mode stage outputs; found {len(parent_stage)}")

    parent_plan = read_plan_values(REFERENCE / "rust-native-f32.csv.gz", compressed=True)
    candidate_plan = read_plan_values(PACKAGE / "data/candidate-plan-inputs.csv.gz", compressed=True)
    shared_plan = parent_plan.keys() & candidate_plan.keys()
    changed_csq = sum(parent_plan[key][3] != candidate_plan[key][3] for key in shared_plan)
    changed_other = sum(
        any(parent_plan[key][index] != candidate_plan[key][index]
            for index in range(len(parent_plan[key])) if index != 3)
        for key in shared_plan
    )
    if changed_other:
        raise ValueError(f"cos2 control changed non-cos2 plan fields in {changed_other} rows")

    direct_rows = []
    for key, reference in sorted(mode_reference.items()):
        candidate_bits = stage[key]
        parent_bits = parent_stage[key]
        if parent_bits != reference["rust_stage_bits"]:
            raise ValueError(f"parent trace/reference key disagrees at {key}")
        expected_bits = reference["reference_f32_bits"]
        expected = Decimal(reference["reference_quad_decimal"])
        parent_error = abs(exact_f32(parent_bits) - expected)
        candidate_error = abs(exact_f32(candidate_bits) - expected)
        direct_rows.append(
            {
                "m": key[0], "l": key[1], "ring": key[2], "part": key[3],
                "parent_bits": parent_bits, "candidate_bits": candidate_bits,
                "binary128_rounded_f32_bits": expected_bits,
                "reference_binary128_decimal": reference["reference_quad_decimal"],
                "parent_abs_error": parent_error,
                "candidate_abs_error": candidate_error,
                "change": "improved" if candidate_error < parent_error else (
                    "worsened" if candidate_error > parent_error else "unchanged"
                ),
            }
        )

    relative_errors = [
        (row["parent_abs_error"] / abs(Decimal(row["reference_binary128_decimal"])),
         row["candidate_abs_error"] / abs(Decimal(row["reference_binary128_decimal"])))
        for row in direct_rows if Decimal(row["reference_binary128_decimal"]) != 0
    ]
    direct_summary = {
        "matched_cases": len(direct_rows),
        "nonzero_reference_cases": len(relative_errors),
        "parent_max_relative_error": float(max(pair[0] for pair in relative_errors)),
        "candidate_max_relative_error": float(max(pair[1] for pair in relative_errors)),
        "parent_rms_relative_error": rms_abs([pair[0] for pair in relative_errors]),
        "candidate_rms_relative_error": rms_abs([pair[1] for pair in relative_errors]),
        "parent_correctly_rounded_bits": sum(
            row["parent_bits"] == row["binary128_rounded_f32_bits"] for row in direct_rows
        ),
        "candidate_correctly_rounded_bits": sum(
            row["candidate_bits"] == row["binary128_rounded_f32_bits"] for row in direct_rows
        ),
        "parent_max_abs_error": float(max(row["parent_abs_error"] for row in direct_rows)),
        "candidate_max_abs_error": float(max(row["candidate_abs_error"] for row in direct_rows)),
        "parent_rms_abs_error": rms_abs([row["parent_abs_error"] for row in direct_rows]),
        "candidate_rms_abs_error": rms_abs([row["candidate_abs_error"] for row in direct_rows]),
        "improved_cases": sum(row["change"] == "improved" for row in direct_rows),
        "worsened_cases": sum(row["change"] == "worsened" for row in direct_rows),
        "unchanged_cases": sum(row["change"] == "unchanged" for row in direct_rows),
        "by_order": {},
    }
    for m in sorted({row["m"] for row in direct_rows}):
        subset = [row for row in direct_rows if row["m"] == m]
        direct_summary["by_order"][str(m)] = {
            "count": len(subset),
            "improved": sum(row["change"] == "improved" for row in subset),
            "worsened": sum(row["change"] == "worsened" for row in subset),
            "unchanged": sum(row["change"] == "unchanged" for row in subset),
        }

    public_reference = read_public_reference()
    parent = read_public_map(REFERENCE / "rust-public-map.csv.gz")
    candidate = read_public_map(PACKAGE / "data/candidate-public-map.csv.gz")
    if len(public_reference) != 13090 or parent.keys() != public_reference.keys() or candidate.keys() != public_reference.keys():
        raise ValueError("public map fixture keys do not match the 13,090-cell independent reference")

    cell_acc = defaultdict(lambda: {"old": [], "new": [], "changed": 0, "improved": 0, "worsened": 0})
    changed_pixels = improved_pixels = worsened_pixels = tied_pixels = 0
    for key in sorted(public_reference):
        expected_bits, expected_value = public_reference[key]
        old_bits, new_bits = parent[key], candidate[key]
        old_error = abs(exact_f32(old_bits) - expected_value)
        new_error = abs(exact_f32(new_bits) - expected_value)
        cell_key = key[:4]
        acc = cell_acc[cell_key]
        acc["old"].append(old_error)
        acc["new"].append(new_error)
        if old_bits != new_bits:
            changed_pixels += 1
            acc["changed"] += 1
            if new_error < old_error:
                improved_pixels += 1
                acc["improved"] += 1
            elif new_error > old_error:
                worsened_pixels += 1
                acc["worsened"] += 1
            else:
                tied_pixels += 1
    public_cell_rows = []
    for key, acc in sorted(cell_acc.items()):
        public_cell_rows.append(
            {
                "m": int(key[0]), "l": int(key[1]), "part": key[2], "ring": int(key[3]),
                "changed_pixels": acc["changed"], "improved_pixels": acc["improved"],
                "worsened_pixels": acc["worsened"],
                "parent_max_abs_to_reference": float(max(acc["old"])),
                "candidate_max_abs_to_reference": float(max(acc["new"])),
                "parent_rms_abs_to_reference": rms_abs(acc["old"]),
                "candidate_rms_abs_to_reference": rms_abs(acc["new"]),
            }
        )
    old_public_errors = [
        abs(exact_f32(parent[k]) - public_reference[k][1]) for k in public_reference
    ]
    new_public_errors = [
        abs(exact_f32(candidate[k]) - public_reference[k][1]) for k in public_reference
    ]
    reference_square_sum = sum((value * value for _, value in public_reference.values()), Decimal(0))
    public_summary = {
        "count": len(public_reference),
        "parent_relative_l2_to_reference": float(
            (sum(error * error for error in old_public_errors) / reference_square_sum).sqrt()
        ),
        "candidate_relative_l2_to_reference": float(
            (sum(error * error for error in new_public_errors) / reference_square_sum).sqrt()
        ),
        "parent_correctly_rounded_bits": sum(
            parent[k] == public_reference[k][0] for k in public_reference
        ),
        "candidate_correctly_rounded_bits": sum(
            candidate[k] == public_reference[k][0] for k in public_reference
        ),
        "parent_max_abs_to_reference": float(max(old_public_errors)),
        "candidate_max_abs_to_reference": float(max(new_public_errors)),
        "parent_rms_abs_to_reference": rms_abs(old_public_errors),
        "candidate_rms_abs_to_reference": rms_abs(new_public_errors),
        "improved_cases": sum(new < old for old, new in zip(old_public_errors, new_public_errors)),
        "worsened_cases": sum(new > old for old, new in zip(old_public_errors, new_public_errors)),
        "unchanged_error_cases": sum(new == old for old, new in zip(old_public_errors, new_public_errors)),
    }
    public_summary.update(
        {
            "changed_pixels": changed_pixels,
            "changed_transform_cells": sum(row["changed_pixels"] > 0 for row in public_cell_rows),
            "changed_pixel_error_improved": improved_pixels,
            "changed_pixel_error_worsened": worsened_pixels,
            "changed_pixel_error_tied": tied_pixels,
            "by_order": {},
        }
    )
    for m in sorted({row["m"] for row in public_cell_rows}):
        keys = [k for k in public_reference if int(k[0]) == m]
        old_e = [abs(exact_f32(parent[k]) - public_reference[k][1]) for k in keys]
        new_e = [abs(exact_f32(candidate[k]) - public_reference[k][1]) for k in keys]
        reference_e = [public_reference[k][1] for k in keys]
        ref_square_sum = sum((v * v for v in reference_e), Decimal(0))
        public_summary["by_order"][str(m)] = {
            "parent_relative_l2": float((sum(e * e for e in old_e) / ref_square_sum).sqrt()),
            "candidate_relative_l2": float((sum(e * e for e in new_e) / ref_square_sum).sqrt()),
            "pixels": len(keys),
            "parent_rms_abs": rms_abs(old_e),
            "candidate_rms_abs": rms_abs(new_e),
            "changed_pixels": sum(parent[k] != candidate[k] for k in keys),
        }

    (PACKAGE / "data/direct-comparison.csv").write_text("", encoding="utf-8")
    with (PACKAGE / "data/direct-comparison.csv").open("w", newline="", encoding="utf-8") as stream:
        fields = list(direct_rows[0])
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(direct_rows)
    with (PACKAGE / "data/public-cell-comparison.csv").open("w", newline="", encoding="utf-8") as stream:
        fields = list(public_cell_rows[0])
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(public_cell_rows)
    with gzip.open(PACKAGE / "data/recurrence-csq-comparison.csv.gz", "rt", newline="") as stream:
        recurrence_rows = list(csv.DictReader(stream))
    first_recurrence_change = next(
        row for row in recurrence_rows
        if row["parent_leading_bits"] != row["candidate_leading_bits"]
    )
    recurrence_summary = {
        "m": 5,
        "north_ring": 5,
        "matched_degree_states": len(recurrence_rows),
        "first_differing_degree": int(first_recurrence_change["degree"]),
        "first_parent_leading_bits": first_recurrence_change["parent_leading_bits"],
        "first_candidate_leading_bits": first_recurrence_change["candidate_leading_bits"],
        "first_parent_next_bits": first_recurrence_change["parent_next_bits"],
        "first_candidate_next_bits": first_recurrence_change["candidate_next_bits"],
        "parent_rms_abs_state_error": rms_abs([
            Decimal(row["parent_next_abs_error"]) for row in recurrence_rows
        ]),
        "candidate_rms_abs_state_error": rms_abs([
            Decimal(row["candidate_next_abs_error"]) for row in recurrence_rows
        ]),
        "parent_max_abs_state_error": float(max(
            Decimal(row["parent_next_abs_error"]) for row in recurrence_rows
        )),
        "candidate_max_abs_state_error": float(max(
            Decimal(row["candidate_next_abs_error"]) for row in recurrence_rows
        )),
        "parent_rms_relative_state_error": rms_abs([
            Decimal(row["parent_next_relative_error"]) for row in recurrence_rows
        ]),
        "candidate_rms_relative_state_error": rms_abs([
            Decimal(row["candidate_next_relative_error"]) for row in recurrence_rows
        ]),
        "parent_max_ulp_from_quad_rounded_f32": max(
            int(row["parent_next_ulp_from_quad"]) for row in recurrence_rows
        ),
        "candidate_max_ulp_from_quad_rounded_f32": max(
            int(row["candidate_next_ulp_from_quad"]) for row in recurrence_rows
        ),
    }
    summary = {
        "candidate": "native-f32 generic scalar synthesis, isolated plan-time cosine-squared control",
        "recurrence": recurrence_summary,
        "provenance": {
            "rust_candidate_base_commit": "45380d7e4bd849a33ffef7d91978ee18fd7e4193",
            "rust_candidate_diff_sha256": "ebedf095a6dceac343ee98e0465b3702e23a7d86b37bdcc1f0ebf01fc58f0ee5",
            "rust_trace_test_binary_sha256": "c806026b2bd11e884de229f5874453cd1e4f97beeb8b472cebd83b0dc70ec13f",
            "ducc_source_commit": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
            "theta_fixture_sha256": "1f16212ea7ef50ccaf9a52b552f9a822fb7052da1e8b164efdd858bdfd2a6f03",
            "raw_candidate_stage_trace_sha256": "bd834e729ca198265fa1a6032da995259d30eb4a9fad3a29ef57bed9126cc6fc",
            "raw_candidate_public_map_sha256": "d5e7679872608589ed68869657fdb836613f6a5233dc846583be76b9f897a915",
            "rust_flags": ["-C", "target-cpu=x86-64-v3", "-Aunexpected_cfgs"],
            "rustc": "1.99.1 (58cf8f9c3 2026-10-13)",
            "test_features": ["default", "parallel"],
            "direct_probe_lanes": 8,
            "public_probe_workspace_capacity": 1,
        "plan_input_comparison": {
            "shared_records": len(shared_plan),
            "cos_squared_bit_changes": changed_csq,
            "other_prepared_field_changes": changed_other,
        },
        },
        "geometry": "GL T720, phi0=0, nphi=34, 721 exact archived theta values",
        "direct_stage_reference": "binary128 values and correctly rounded f32 bits from the archived mode-comparison matrix",
        "public_map_reference": "independent long-double values; its correctly rounded f32 bits are also checked",
        "direct_stage": direct_summary,
        "public_map": public_summary,
        "accuracy_target": "UNSET; these are errors and sensitivity measurements, not acceptance PASSes",
    }
    (PACKAGE / "data/comparison.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
