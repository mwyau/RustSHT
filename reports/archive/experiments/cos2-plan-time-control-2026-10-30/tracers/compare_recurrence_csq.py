#!/usr/bin/env python3
"""Build the focused m=5, north-ring-5 recurrence comparison."""
from __future__ import annotations

import csv
import gzip
import struct
from decimal import Decimal, getcontext
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
ROOT = Path(__file__).resolve().parents[5]
REFERENCE = ROOT / "reports/archive/experiments/numerical-causality-ducc-0.42.0-2026-10-29/data"
getcontext().prec = 100


def f32(bits: str) -> float:
    return struct.unpack(">f", int(bits[-8:], 16).to_bytes(4, "big"))[0]


def ordered(bits: str) -> int:
    value = int(bits[-8:], 16)
    return 0x80000000 - value if value & 0x80000000 else value + 0x80000000


def read_trace(path: Path, *, compressed: bool, part: str) -> dict[int, list[str]]:
    opener = gzip.open if compressed else open
    rows = {}
    with opener(path, "rt") as stream:
        for line in stream:
            if not line.startswith("REC,5,5,"):
                continue
            row = line.rstrip().split(",")
            if row[3] == part:
                rows[int(row[4])] = row
    return rows


def main() -> None:
    parent = read_trace(REFERENCE / "rust-native-f32.csv.gz", compressed=True, part="re")
    candidate = read_trace(PACKAGE / "data/candidate-recurrence-raw.csv.gz", compressed=True, part="re_l720")
    with gzip.open(REFERENCE / "recurrence-growth.csv.gz", "rt", newline="") as stream:
        reference = {
            int(row["degree"]): row
            for row in csv.DictReader(stream)
            if int(row["m"]) == 5 and int(row["ring"]) == 5
        }

    common = sorted(parent.keys() & candidate.keys() & reference.keys())
    if len(common) != 357:
        raise ValueError(f"expected 357 matched recurrence degrees, got {len(common)}")
    rows = []
    for degree in common:
        old, new, wide = parent[degree], candidate[degree], reference[degree]
        if old[6:8] != new[6:8]:
            raise ValueError(f"recurrence coefficients changed at degree {degree}")
        previous_old, previous_new = old[8][-8:], new[8][-8:]
        current_old, current_new = old[9][-8:], new[9][-8:]
        leading_old, leading_new = old[10][-8:], new[10][-8:]
        next_old, next_new = old[11][-8:], new[11][-8:]
        quad_next = Decimal(wide["quad_next"])
        exact_old = Decimal.from_float(f32(next_old))
        exact_new = Decimal.from_float(f32(next_new))
        expected_bits = wide["ducc_next_rounded_f32_bits"]
        rows.append({
            "degree": degree,
            "parent_csq_bits": old[5][-8:],
            "candidate_csq_bits": new[5][-8:],
            "coefficient_a_bits": old[6][-8:],
            "coefficient_b_bits": old[7][-8:],
            "parent_previous_bits": previous_old,
            "candidate_previous_bits": previous_new,
            "parent_current_bits": current_old,
            "candidate_current_bits": current_new,
            "parent_leading_bits": leading_old,
            "candidate_leading_bits": leading_new,
            "parent_next_bits": next_old,
            "candidate_next_bits": next_new,
            "quad_csq": wide["quad_csq"],
            "quad_next": wide["quad_next"],
            "rounded_quad_next_f32_bits": expected_bits,
            "parent_next_ulp_from_quad": abs(ordered(next_old) - ordered(expected_bits)),
            "candidate_next_ulp_from_quad": abs(ordered(next_new) - ordered(expected_bits)),
            "parent_next_abs_error": format(abs(exact_old - quad_next), "E"),
            "candidate_next_abs_error": format(abs(exact_new - quad_next), "E"),
            "parent_next_relative_error": format(abs(exact_old - quad_next) / abs(quad_next), "E"),
            "candidate_next_relative_error": format(abs(exact_new - quad_next) / abs(quad_next), "E"),
        })
    output = PACKAGE / "data/recurrence-csq-comparison.csv.gz"
    with output.open("wb") as raw:
        with gzip.GzipFile(fileobj=raw, mode="wb", mtime=0, filename="") as zipped:
            import io
            text = io.TextIOWrapper(zipped, encoding="utf-8", newline="")
            writer = csv.DictWriter(text, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)
            text.flush()
            text.detach()
    print(f"wrote {len(rows)} degree rows to {output}")


if __name__ == "__main__":
    main()
