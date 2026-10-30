#!/usr/bin/env python3
import csv
import json
import os
import sys
from collections import defaultdict
from decimal import Decimal, getcontext
from pathlib import Path

getcontext().prec = 70
D = Decimal


def complex_value(re, im):
    return D(re), D(im)


def summarize_rows(rows, *, reference_fields, f32_fields, f64_fields, error_fields,
                   scaled_fields, group_field=None):
    groups = defaultdict(list)
    for row in rows:
        try:
            ref = complex_value(*reference_fields(row))
            v32 = complex_value(*f32_fields(row))
            v64 = complex_value(*f64_fields(row))
            e32 = D(error_fields(row)[0])
            e64 = D(error_fields(row)[1])
            s32 = D(scaled_fields(row)[0])
            s64 = D(scaled_fields(row)[1])
        except Exception:
            continue
        if not all(value.is_finite() for pair in (ref, v32, v64) for value in pair):
            continue
        if not all(value.is_finite() for value in (e32, e64, s32, s64)):
            continue
        group = row[group_field] if group_field else "all"
        groups[group].append((row, ref, v32, v64, e32, e64, s32, s64))
    result = {}
    for group, values in groups.items():
        ref_norm = sum((ref[0] * ref[0] + ref[1] * ref[1] for _, ref, *_ in values), D(0))
        err32_norm = sum((e32 * e32 for _, _, _, _, e32, _, _, _ in values), D(0))
        err64_norm = sum((e64 * e64 for _, _, _, _, _, e64, _, _ in values), D(0))
        worst_abs32 = max(values, key=lambda x: x[4])
        worst_scaled32 = max(values, key=lambda x: x[6])
        result[group] = {
            "count": len(values),
            "max_abs_f32": str(max(x[4] for x in values)),
            "max_abs_f64": str(max(x[5] for x in values)),
            "rms_abs_f32": str((err32_norm / len(values)).sqrt()),
            "rms_abs_f64": str((err64_norm / len(values)).sqrt()),
            "relative_l2_f32": str((err32_norm / ref_norm).sqrt()) if ref_norm else None,
            "relative_l2_f64": str((err64_norm / ref_norm).sqrt()) if ref_norm else None,
            "max_envelope_scaled_f32": str(max(x[6] for x in values)),
            "max_envelope_scaled_f64": str(max(x[7] for x in values)),
            "worst_f32_abs_degree_order": {
                "m": worst_abs32[0].get("m"), "l": worst_abs32[0].get("l")
            },
            "worst_f32_scaled_degree_order": {
                "m": worst_scaled32[0].get("m"), "l": worst_scaled32[0].get("l")
            },
        }
    return result


def coefficient(path, components):
    rows = list(csv.DictReader(path.open()))
    selectors = {
        "all": lambda r: True,
        "l600_720": lambda r: 600 <= int(r["l"]) <= 720,
    }
    result = {}
    for name, select in selectors.items():
        result[name] = summarize_rows(
            [row for row in rows if select(row)],
            reference_fields=lambda r: (r["reference_re"], r["reference_im"]),
            f32_fields=lambda r: (r["f32_re"], r["f32_im"]),
            f64_fields=lambda r: (r["f64_re"], r["f64_im"]),
            error_fields=lambda r: (r["f32_abs"], r["f64_abs"]),
            scaled_fields=lambda r: (r["f32_scaled"], r["f64_scaled"]),
            group_field=components,
        )
    return result


def coefficient_by_l_m(path):
    rows = list(csv.DictReader(path.open()))
    selectors = {
        "all": lambda r: True,
        "l0_599": lambda r: int(r["l"]) < 600,
        "l600_720": lambda r: 600 <= int(r["l"]) <= 720,
        "m0": lambda r: int(r["m"]) == 0,
        "m1_4": lambda r: 1 <= int(r["m"]) <= 4,
        "m5_16": lambda r: 5 <= int(r["m"]) <= 16,
    }
    result = {}
    for band, select in selectors.items():
        result[band] = summarize_rows(
            [row for row in rows if select(row)],
            reference_fields=lambda r: (r["reference_re"], r["reference_im"]),
            f32_fields=lambda r: (r["f32_re"], r["f32_im"]),
            f64_fields=lambda r: (r["f64_re"], r["f64_im"]),
            error_fields=lambda r: (r["f32_abs"], r["f64_abs"]),
            scaled_fields=lambda r: (r["f32_scaled"], r["f64_scaled"]),
            group_field="component",
        )
    return result


def synthesis(path):
    rows = list(csv.DictReader(path.open()))
    result = {}
    for component in ("Q", "U"):
        selected = [r for r in rows if r["component"] == component]
        ref_norm = sum((D(r["reference"]) ** 2 for r in selected), D(0))
        err32 = sum((D(r["f32_abs"]) ** 2 for r in selected), D(0))
        err64 = sum((D(r["f64_abs"]) ** 2 for r in selected), D(0))
        fft_err = sum((D(r["f32_fft_abs"]) ** 2 for r in selected), D(0))
        result[component] = {
            "count": len(selected),
            "max_abs_f32": str(max(D(r["f32_abs"]) for r in selected)),
            "max_abs_f64": str(max(D(r["f64_abs"]) for r in selected)),
            "rms_abs_f32": str((err32 / len(selected)).sqrt()),
            "rms_abs_f64": str((err64 / len(selected)).sqrt()),
            "relative_l2_f32": str((err32 / ref_norm).sqrt()),
            "relative_l2_f64": str((err64 / ref_norm).sqrt()),
            "inverse_fft_residual_relative_l2": str((fft_err / ref_norm).sqrt()),
            "max_envelope_scaled_f32": str(max(D(r["f32_scaled"]) for r in selected)),
            "max_envelope_scaled_f64": str(max(D(r["f64_scaled"]) for r in selected)),
            "worst_f32_location": max(
                selected, key=lambda r: D(r["f32_abs"])
            )["ring"] + ":" + max(selected, key=lambda r: D(r["f32_abs"]))["longitude"],
            "worst_f32_scaled_location": max(
                selected, key=lambda r: D(r["f32_scaled"])
            )["ring"] + ":" + max(selected, key=lambda r: D(r["f32_scaled"]))["longitude"],
        }
    return result


def synthesis_by_ring(path):
    rows = list(csv.DictReader(path.open()))
    selected_rings = (0, 1, 5, 360, 715, 719, 720)
    result = {}
    for ring in selected_rings:
        result[str(ring)] = {}
        for component in ("Q", "U"):
            selected = [r for r in rows if int(r["ring"]) == ring and r["component"] == component]
            ref_norm = sum((D(r["reference"]) ** 2 for r in selected), D(0))
            err32 = sum((D(r["f32_abs"]) ** 2 for r in selected), D(0))
            result[str(ring)][component] = {
                "count": len(selected),
                "max_abs_f32": str(max(D(r["f32_abs"]) for r in selected)),
                "relative_l2_f32": str((err32 / ref_norm).sqrt()) if ref_norm else None,
                "max_envelope_scaled_f32": str(max(D(r["f32_scaled"]) for r in selected)),
                "worst_longitude": max(selected, key=lambda r: D(r["f32_abs"]))["longitude"],
            }
    return result


def spectral(path):
    rows = list(csv.DictReader(path.open()))
    result = {}
    for component in sorted({row["component"] for row in rows}):
        selected = [row for row in rows if row["component"] == component]
        ref_norm = sum(
            (D(row["reference_re"]) ** 2 + D(row["reference_im"]) ** 2 for row in selected), D(0)
        )
        err_norm = sum((D(row["abs_error"]) ** 2 for row in selected), D(0))
        result[component] = {
            "count": len(selected),
            "max_abs": str(max(D(row["abs_error"]) for row in selected)),
            "relative_l2": str((err_norm / ref_norm).sqrt()) if ref_norm else None,
        }
    return result


def forward_fft(path):
    rows = list(csv.DictReader(path.open()))
    norm_ref = D(0)
    error_norm = D(0)
    for row in rows:
        norm_ref += D(row["exact_re"]) ** 2 + D(row["exact_im"]) ** 2
        error_norm += D(row["abs_error"]) ** 2
    return {
        "count": len(rows),
        "max_abs": str(max(D(row["abs_error"]) for row in rows)),
        "relative_l2": str((error_norm / norm_ref).sqrt()) if norm_ref else None,
    }


def main():
    directory = Path(sys.argv[1])
    summary = {
        "source": os.environ.get("PHASEC_SOURCE_SHA", "unspecified"),
        "geometry": "GL T720, mmax=16, nphi=34, phi0=0",
        "reference": "independent binary128 scalar-harmonic recurrence, direct Fourier sums and spin conversion transpose",
        "scalar_analysis": coefficient(directory / "scalar-analysis-comparison.csv", "component"),
        "scalar_analysis_by_l_m": coefficient_by_l_m(directory / "scalar-analysis-comparison.csv"),
        "scalar_analysis_after_f32_fft_input": coefficient(
            directory / "scalar-analysis-fft-input-comparison.csv", "component"
        ),
        "scalar_forward_fft": forward_fft(directory / "scalar-forward-fft.csv"),
        "spin_analysis_e": coefficient(directory / "spin-analysis-e-comparison.csv", "component"),
        "spin_analysis_e_by_l_m": coefficient_by_l_m(directory / "spin-analysis-e-comparison.csv"),
        "spin_analysis_b": coefficient(directory / "spin-analysis-b-comparison.csv", "component"),
        "spin_analysis_b_by_l_m": coefficient_by_l_m(directory / "spin-analysis-b-comparison.csv"),
        "spin_analysis_e_after_f32_fft_input": coefficient(
            directory / "spin-analysis-e-fft-input-comparison.csv", "component"
        ),
        "spin_analysis_b_after_f32_fft_input": coefficient(
            directory / "spin-analysis-b-fft-input-comparison.csv", "component"
        ),
        "spin_forward_fft_q": forward_fft(directory / "spin-analysis-q-forward-fft.csv"),
        "spin_forward_fft_u": forward_fft(directory / "spin-analysis-u-forward-fft.csv"),
        "spin_synthesis": synthesis(directory / "spin-synthesis-comparison.csv"),
        "spin_synthesis_by_ring": synthesis_by_ring(directory / "spin-synthesis-comparison.csv"),
        "spin_synthesis_spectrum": spectral(
            directory / "spin-synthesis-spectrum-comparison.csv"
        ),
    }
    destination = directory / "summary.json"
    destination.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
