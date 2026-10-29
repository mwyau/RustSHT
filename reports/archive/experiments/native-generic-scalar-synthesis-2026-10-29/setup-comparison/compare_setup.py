#!/usr/bin/env python3
import csv
import gzip
import json
import math
import struct
from pathlib import Path

root = Path(__file__).resolve().parent
data_dir = root / "data"

def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]

def bits(value):
    return struct.unpack("<I", struct.pack("<f", value))[0]

def csv_gz(name):
    with gzip.open(data_dir / name, "rt", newline="") as stream:
        yield from csv.reader(stream)

def rounded_cast_stats(candidate, reference):
    return {
        "rows": len(candidate),
        "not_nearest_f32_count": sum(bits(candidate[k]) != bits(f32(reference[k])) for k in candidate),
        "max_ulp_from_nearest_f32": max(abs(bits(candidate[k]) - bits(f32(reference[k]))) for k in candidate),
        "max_relative_error_vs_ducc_double": max(
            abs((candidate[k] - reference[k]) / reference[k])
            for k in candidate if reference[k] != 0),
    }

dg, df, dp, rg, rp = {}, {}, {}, {}, {}
for row in csv_gz("ducc-setup.csv.gz"):
    values = list(map(float, row[2:]))
    key = int(row[1])
    if row[0] == "G":
        dg[key] = values
    elif row[0] == "F":
        df[(key, int(row[2]))] = list(map(float, row[3:]))
    elif row[0] == "P":
        dp[(key, int(row[2]))] = list(map(float, row[3:]))
for row in csv_gz("rust-setup.csv.gz"):
    if row[0] == "G":
        rg[int(row[1])] = list(map(float, row[2:]))
for row in csv_gz("actual-rust-pair-preparation.csv.gz"):
    rp[(int(row[1]), int(row[2]))] = list(map(float, row[3:]))
assert len(dg) == len(rg) == 720
assert len(df) == len(dp) == len(rp) == 968
assert set(dg) == set(rg) and set(df) == set(dp) == set(rp)

exact_double = {}
for name, rust_idx, ducc_idx in (("theta", 0, 0), ("sin", 1, 1), ("cos", 2, 2), ("cos_squared", 3, 3)):
    exact_double[name] = {
        "bitwise_equal": sum(rg[i][rust_idx] == dg[i][ducc_idx] for i in dg),
        "count": 720,
        "max_absolute_difference": max(abs(rg[i][rust_idx] - dg[i][ducc_idx]) for i in dg),
    }
geometry_f32 = {}
for name, rust_idx, ducc_idx in (("sin", 4, 1), ("cos", 5, 2), ("cos_squared", 6, 3)):
    geometry_f32[name] = rounded_cast_stats(
        {i: rg[i][rust_idx] for i in rg}, {i: dg[i][ducc_idx] for i in dg})
csq_rel = [(rg[i][6] - dg[i][3]) / dg[i][3] for i in dg]
csq_abs = [rg[i][6] - dg[i][3] for i in dg]
max_csq_ring = max(dg, key=lambda i: abs(csq_rel[i]))
geometry_summary = {
    "candidate_f32_vs_ducc_f64_rms_relative": math.sqrt(sum(x*x for x in csq_rel) / 720),
    "candidate_f32_vs_ducc_f64_max_relative": csq_rel[max_csq_ring],
    "max_relative_ring": max_csq_ring,
    "candidate_f32_vs_ducc_f64_rms_absolute": math.sqrt(sum(x*x for x in csq_abs) / 720),
    "candidate_f32_vs_ducc_f64_max_absolute": max(abs(x) for x in csq_abs),
}
factor_fields = (("alpha", 0, 0), ("recurrence_a", 1, 1), ("recurrence_b", 2, 2),
                 ("epsilon_l_plus_1", 3, 3), ("epsilon_l_plus_2", 4, 4))
factor_casts = {
    name: rounded_cast_stats({k: rp[k][rust_idx] for k in rp}, {k: df[k][ducc_idx] for k in df})
    for name, rust_idx, ducc_idx in factor_fields
}

pair_coefficients = {}
for name, rust_idx, ducc_idx in (("even", 5, 0), ("odd", 6, 1)):
    errors = [rp[k][rust_idx] - dp[k][ducc_idx] for k in dp]
    refs = [dp[k][ducc_idx] for k in dp]
    per_order = {}
    for order in range(1, 17):
        keys = [k for k in dp if k[0] == order]
        e = [rp[k][rust_idx] - dp[k][ducc_idx] for k in keys]
        ref = [dp[k][ducc_idx] for k in keys]
        per_order[str(order)] = math.sqrt(sum(x*x for x in e) / sum(x*x for x in ref))
    pair_coefficients[name] = {
        "rows": len(errors),
        "not_nearest_f32_count": sum(bits(rp[k][rust_idx]) != bits(f32(dp[k][ducc_idx])) for k in dp),
        "relative_l2_vs_ducc_double": math.sqrt(sum(x*x for x in errors) / sum(x*x for x in refs)),
        "rms_absolute_difference": math.sqrt(sum(x*x for x in errors) / len(errors)),
        "max_absolute_difference": max(abs(x) for x in errors),
        "relative_l2_by_order": per_order,
    }

summary = {
    "reference": {"name": "DUCC 0.42.0 f32 transform API", "commit": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"},
    "candidate": {"base_commit": "b6a96aa7161a64f678b614d73478496f50f0551e",
                  "source_diff_sha256": "cc72cbc20048affbdbd6cdbce619af79edc65896a9a0ed9730006179ea3ce7a8"},
    "probe": {"geometry_rings": 720, "orders": [1, 16], "degree_pairs": [600, 720], "pair_rows": 968},
    "geometry": {"double_setup": exact_double, "native_f32_rounding": geometry_f32, "cos_squared_error": geometry_summary},
    "recurrence_factors": factor_casts,
    "prepared_coefficients": pair_coefficients,
}
output = root / "results" / "setup-comparison-summary.json"
output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
print(json.dumps({"geometry": summary["geometry"], "recurrence_factors": factor_casts,
                  "prepared_coefficients": {k: {f:v for f,v in row.items() if f != "relative_l2_by_order"}
                                             for k,row in pair_coefficients.items()}}, indent=2))
