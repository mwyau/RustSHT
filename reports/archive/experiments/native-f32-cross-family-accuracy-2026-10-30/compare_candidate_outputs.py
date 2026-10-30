#!/usr/bin/env python3
"""Compare a candidate's raw native-f32 outputs with existing Q reference CSVs."""
import csv
import json
import struct
import sys
from decimal import Decimal, getcontext
from pathlib import Path

getcontext().prec = 70
D = Decimal


def read_f32(path):
    data = path.read_bytes()
    if len(data) % 4:
        raise ValueError(f"{path}: non-f32 byte count {len(data)}")
    return [struct.unpack_from("<f", data, offset)[0]
            for offset in range(0, len(data), 4)]


def exact(v):
    return D.from_float(float(v))


def summarize(errors, refs, envelopes):
    err2 = sum((e * e for e in errors), D(0))
    ref2 = sum((r * r for r in refs), D(0))
    scaled = [e / env for e, env in zip(errors, envelopes) if env]
    return {
        "count": len(errors),
        "max_abs": str(max(errors)),
        "rms_abs": str((err2 / len(errors)).sqrt()),
        "relative_l2": str((err2 / ref2).sqrt()) if ref2 else None,
        "max_reference_envelope_scaled": str(max(scaled)) if scaled else None,
    }


def alm(candidate_dir, reference_dir, stem, filename):
    values = read_f32(candidate_dir / filename)
    if len(values) % 2:
        raise ValueError(f"{filename}: odd complex-component count")
    complex_values = list(zip(values[::2], values[1::2]))
    rows = list(csv.DictReader((reference_dir / f"{stem}-comparison.csv").open()))
    result = {}
    by_group = {"all": rows, "l600_720": [r for r in rows if 600 <= int(r["l"]) <= 720]}
    for name, selected in by_group.items():
        errors, refs, envelopes = [], [], []
        for row in selected:
            l, m = int(row["l"]), int(row["m"])
            index = m * 721 - m * (m + 1) // 2 + l
            real, imag = complex_values[index]
            dr = exact(real) - D(row["reference_re"])
            di = exact(imag) - D(row["reference_im"])
            errors.append((dr * dr + di * di).sqrt())
            refs.append((D(row["reference_re"]) ** 2 + D(row["reference_im"]) ** 2).sqrt())
            base_err = D(row["f32_abs"])
            base_scaled = D(row["f32_scaled"])
            envelopes.append(base_err / base_scaled if base_scaled else D(0))
        result[name] = summarize(errors, refs, envelopes)
    return result


def maps(candidate_dir, reference_dir, nphi=34):
    result = {}
    rows = list(csv.DictReader((reference_dir / "spin-synthesis-comparison.csv").open()))
    for component, filename in (("Q", "spin-synthesis-q.f32"), ("U", "spin-synthesis-u.f32")):
        data = read_f32(candidate_dir / filename)
        errors, refs, envelopes = [], [], []
        for row in rows:
            if row["component"] != component:
                continue
            index = int(row["ring"]) * nphi + int(row["longitude"])
            error = abs(exact(data[index]) - D(row["reference"]))
            errors.append(error)
            refs.append(abs(D(row["reference"])))
            base_err, base_scaled = D(row["f32_abs"]), D(row["f32_scaled"])
            envelopes.append(base_err / base_scaled if base_scaled else D(0))
        result[component] = summarize(errors, refs, envelopes)
    return result


def main():
    candidate_dir, reference_dir = map(Path, sys.argv[1:3])
    summary = {
        "candidate": candidate_dir.name,
        "reference_fixture": reference_dir.name,
        "scalar_analysis": alm(candidate_dir, reference_dir, "scalar-analysis", "scalar-analysis.f32"),
        "spin_analysis_e": alm(candidate_dir, reference_dir, "spin-analysis-e", "spin-analysis-e.f32"),
        "spin_analysis_b": alm(candidate_dir, reference_dir, "spin-analysis-b", "spin-analysis-b.f32"),
        "spin_synthesis": maps(candidate_dir, reference_dir),
    }
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
