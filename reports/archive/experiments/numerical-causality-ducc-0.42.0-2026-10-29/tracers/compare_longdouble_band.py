#!/usr/bin/env python3
"""Cross-check binary128 and host long-double high-band reference outputs."""
import csv
import decimal
import json
import math
import re
import sys
from pathlib import Path

from compare_dense_band import f32, parse_ducc, parse_quad, parse_rust

decimal.getcontext().prec = 64
D = decimal.Decimal


def parse_longdouble(path):
    result = {}
    for line in Path(path).read_text().splitlines():
        p = line.split(",")
        if p[0] != "LONGDOUBLE_OUT":
            continue
        m, ring = int(p[1]), int(p[2])
        vals = dict(re.findall(r"(nr|ni|sr|si|evenr|oddr|eveni|oddi)=([^,]+)", line))
        bit_text = re.search(r"bits=([0-9a-f:]+)", line).group(1)
        result[m, ring] = {
            "theta": int(p[3], 16),
            "values": {k: D(v) for k, v in vals.items()},
            "bits": tuple(int(x, 16) for x in bit_text.split(":")),
        }
    return result


def main():
    if len(sys.argv) != 6:
        raise SystemExit("usage: compare_longdouble_band.py quad.csv longdouble.csv ducc.csv rust.csv output_dir")
    quad, _, _ = parse_quad(sys.argv[1])
    longdouble = parse_longdouble(sys.argv[2])
    ducc, _, _ = parse_ducc(sys.argv[3])
    _, _, rust, _ = parse_rust(sys.argv[4])
    out = Path(sys.argv[5])
    out.mkdir(parents=True, exist_ok=True)

    rows = []
    quad_errors, ducc_errors, rust_errors = [], [], []
    quad_rounded_mismatches = 0
    for m in range(1, 17):
        for ring in (0, 1, 5, 360):
            q, ld = quad[m, ring], longdouble[m, ring]
            if q["theta"] != ld["theta"]:
                raise SystemExit(f"theta mismatch m={m} ring={ring}")
            d_bits = ducc[m, ring]
            r_out = rust[m, ring]
            r_bits = (*r_out["north"], *r_out["south"])
            keys = ("nr", "ni", "sr", "si")
            for i, key in enumerate(keys):
                qv = q["decimal"].get(key)
                if qv is None and key == "si":
                    qv = q["decimal"]["eveni"] - q["decimal"]["c"] * q["decimal"]["oddi"]
                lv = ld["values"][key]
                qerr = abs(qv - lv)
                # Sum-of-absolute-contributions scale remains meaningful near zero.
                channel = key[-1]
                contribution_scale = abs(q["decimal"]["even" + channel]) + abs(q["decimal"]["odd" + channel])
                scale = max(contribution_scale, abs(lv), D("1e-100"))
                db, rb = d_bits[i], r_bits[i]
                derr = abs(D.from_float(f32(db)) - lv)
                rerr = abs(D.from_float(f32(rb)) - lv)
                quad_errors.append(float(qerr))
                ducc_errors.append(float(derr))
                rust_errors.append(float(rerr))
                quad_rounded_mismatches += q["bits"][i] != ld["bits"][i]
                rows.append((m, ring, key, f"{q['theta']:016x}", format(qv, ".36g"),
                    format(lv, ".24g"), format(qerr, ".8e"), format(qerr / scale, ".8e"),
                    f"{q['bits'][i]:08x}", f"{ld['bits'][i]:08x}", q["bits"][i] == ld["bits"][i],
                    f"{db:08x}", format(derr, ".8e"), format(derr / scale, ".8e"),
                    f"{rb:08x}", format(rerr, ".8e"), format(rerr / scale, ".8e")))

    with (out / "dense-band-longdouble-crosscheck.csv").open("w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(("order", "ring", "component", "theta_bits", "quad128_value", "longdouble_value",
            "quad_longdouble_abs_error", "quad_longdouble_contribution_scaled_error", "quad_f32_bits",
            "longdouble_f32_bits", "rounded_bits_match", "ducc_f32_bits", "ducc_longdouble_abs_error",
            "ducc_longdouble_contribution_scaled_error", "rust_f32_bits", "rust_longdouble_abs_error",
            "rust_longdouble_contribution_scaled_error"))
        writer.writerows(rows)
    summary = {
        "component_count": len(rows),
        "quad128_vs_longdouble_max_abs": max(quad_errors),
        "quad128_vs_longdouble_rms_abs": math.sqrt(math.fsum(x*x for x in quad_errors) / len(quad_errors)),
        "quad128_vs_longdouble_rounded_f32_bit_mismatches": quad_rounded_mismatches,
        "ducc_f32_vs_longdouble_max_abs": max(ducc_errors),
        "ducc_f32_vs_longdouble_rms_abs": math.sqrt(math.fsum(x*x for x in ducc_errors) / len(ducc_errors)),
        "rust_f32_vs_longdouble_max_abs": max(rust_errors),
        "rust_f32_vs_longdouble_rms_abs": math.sqrt(math.fsum(x*x for x in rust_errors) / len(rust_errors)),
        "reference": "binary128 recurrence compared against independent host x86 extended-precision recurrence",
    }
    (out / "dense-band-longdouble-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
