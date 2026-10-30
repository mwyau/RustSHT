#!/usr/bin/env python3
"""Compare bounded Rust, DUCC and binary128 traces for the archived high band."""
import csv
import decimal
import json
import math
import re
import struct
import sys
from pathlib import Path

decimal.getcontext().prec = 64
D = decimal.Decimal


def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits & 0xFFFFFFFF))[0]


def f32_bits(value):
    return struct.unpack("<I", struct.pack("<f", value))[0]


def ordered(bits):
    bits &= 0xFFFFFFFF
    return (~bits & 0xFFFFFFFF) if bits & 0x80000000 else (bits | 0x80000000)


def ulp(a, b):
    return abs(ordered(a) - ordered(b))


def complex_bits(text):
    re_bits, im_bits = text.strip("()").split(",")
    return int(re_bits, 16), int(im_bits, 16)


def parse_rust(path):
    pairs, rec, outputs, prefix = {}, {}, {}, {}
    for line in Path(path).read_text().splitlines():
        p = line.split(",")
        if p[0] == "PAIR_STATE":
            m, base = int(p[1]), int(p[2])
            pairs[m, base] = tuple(int(x, 16) for x in p[3:7])
        elif p[0] == "REC":
            m, ring, degree = map(int, p[1:4])
            bits = [int(x, 16) & 0xFFFFFFFF for x in p[4:] if x]
            rec.setdefault((m, ring), []).append((degree, bits))
        elif p[0] == "PREFIX":
            m, ring = int(p[1]), int(p[2])
            prefix[m, ring] = {
                "active_lanes": int(p[3]), "scaled_pairs": int(p[4]),
                "ordinary_start_degree": int(p[5]), "reached_full_ieee": p[6] == "true",
            }
        elif p[0] == "OUTPUT":
            m, ring = int(p[1]), int(p[2])
            fields = dict(re.findall(r"(theta|sin|cos|csq)=([0-9a-f]+)", line))
            even = complex_bits(re.search(r"even=(\([^)]*\))", line).group(1))
            odd = complex_bits(re.search(r"odd=(\([^)]*\))", line).group(1))
            north = complex_bits(re.search(r"north=(\([^)]*\))", line).group(1))
            south = complex_bits(re.search(r"south=(\([^)]*\))", line).group(1))
            outputs[m, ring] = {
                **{k: int(v, 16) for k, v in fields.items()},
                "even": even, "odd": odd, "north": north, "south": south,
            }
    return pairs, rec, outputs, prefix


def parse_ducc(path):
    outputs, accum, replay = {}, {}, {}
    for line in Path(path).read_text().splitlines():
        p = line.split(",")
        if p[0] == "OUTPUT":
            m, ring = int(p[1]), int(p[2])
            outputs[m, ring] = tuple(int(x, 16) for x in p[3:7])
        elif p[0] == "DUCC_ACCUM":
            m, ring = int(p[1]), int(p[2])
            accum[m, ring] = tuple(int(x, 16) for x in p[4:15])
        elif p[0] == "REPLAY":
            m, ring = int(p[1]), int(p[2])
            replay[m, ring] = tuple(int(x, 16) for x in p[3:9])
    return outputs, accum, replay


def parse_quad(path):
    outputs, pairs, rec = {}, {}, {}
    for line in Path(path).read_text().splitlines():
        p = line.split(",")
        if p[0] == "PAIR_STATE":
            m, base = int(p[1]), int(p[2])
            pairs[m, base] = {
                "decimal": tuple(D(x) for x in p[3:7]),
                "bits": tuple(int(x, 16) for x in p[7:11]),
            }
        elif p[0] == "QUAD_REC":
            m, ring, degree = int(p[1]), int(p[2]), int(p[3])
            rec.setdefault((m, ring), []).append({
                "degree": degree,
                "decimal": tuple(D(x) for x in p[4:11]),
                "bits": tuple(int(x, 16) for x in p[11:18]),
            })
        elif p[0] == "QUAD_OUT":
            m, ring = int(p[1]), int(p[2])
            fields = dict(re.findall(r"(c|evenr|oddr|nr|sr|eveni|oddi|ni)=([^,]+)", line))
            bit_text = re.search(r"bits=([0-9a-f:]+)", line).group(1)
            outputs[m, ring] = {
                "theta": int(p[3], 16),
                "decimal": {k: D(v) for k, v in fields.items()},
                "bits": tuple(int(x, 16) for x in bit_text.split(":")),
            }
    return outputs, pairs, rec


def write_csv(path, header, rows):
    with open(path, "w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(header)
        writer.writerows(rows)


def main():
    if len(sys.argv) != 5:
        raise SystemExit("usage: compare_dense_band.py rust.csv ducc.csv quad.csv output_dir")
    rust_pairs, rust_rec, rust_outputs, prefix = parse_rust(sys.argv[1])
    ducc_outputs, ducc_accum, ducc_replay = parse_ducc(sys.argv[2])
    quad_outputs, quad_pairs, quad_rec = parse_quad(sys.argv[3])
    output_dir = Path(sys.argv[4])
    output_dir.mkdir(parents=True, exist_ok=True)
    keys = sorted((m, r) for m in range(1, 17) for r in (0, 1, 5, 360))
    if not set(keys).issubset(rust_outputs) or set(keys) != set(ducc_outputs) or not set(keys).issubset(quad_outputs):
        raise SystemExit("trace case counts/keys disagree")
    if len(quad_pairs) != len(rust_pairs):
        raise SystemExit(f"prepared-pair counts disagree: Rust={len(rust_pairs)} quad={len(quad_pairs)}")

    output_rows = []
    accum_rows = []
    pair_rows = []
    recurrence_rows = []
    recurrence_first_rows = []
    prefix_rows = []
    controlled_rows = []
    candidates, duccs, rust_vs_ducc = [], [], []
    accumulation_effects, nonaccum_errors = [], []
    for m, ring in keys:
        ro = rust_outputs[m, ring]
        qo = quad_outputs[m, ring]
        dbits = ducc_outputs[m, ring]
        if ro["theta"] != qo["theta"]:
            raise SystemExit(f"theta fixture mismatch m={m} ring={ring}")
        qd = qo["decimal"]
        q_values = (qd["nr"], qd["ni"], qd["sr"], qd["eveni"] - qd["c"] * qd["oddi"])
        q_bits = qo["bits"]
        scale_for = (
            abs(qd["evenr"]) + abs(qd["c"] * qd["oddr"]),
            abs(qd["eveni"]) + abs(qd["c"] * qd["oddi"]),
        )

        # Replay the native-f32 recurrence/coefficient terms with binary64 summation.
        # Each f32*f32 product is exactly representable in f64. Reconstruct the
        # uninstrumented one-pair tail from the last traced recurrence state.
        terms = {"even_re": [], "even_im": [], "odd_re": [], "odd_im": []}
        for _, bits in rust_rec[m, ring]:
            current = f32(bits[4])
            for name, index in (("even_re", 8), ("odd_re", 9), ("even_im", 10), ("odd_im", 11)):
                terms[name].append(current * f32(bits[index]))
        pair_count = (720 - m + 2) // 2
        trace_count = len(rust_rec[m, ring])
        if trace_count < pair_count:
            if trace_count != pair_count - 1:
                raise SystemExit(f"unexpected trace tail m={m} ring={ring}: {trace_count}/{pair_count}")
            base = m + 2 * (pair_count - 1)
            tail_state = f32(rust_rec[m, ring][-1][1][7])
            tail_pair = rust_pairs[m, base]
            for name, index in (("even_re", 0), ("even_im", 1), ("odd_re", 2), ("odd_im", 3)):
                terms[name].append(tail_state * f32(tail_pair[index]))
        elif trace_count != pair_count:
            raise SystemExit(f"unexpected trace length m={m} ring={ring}: {trace_count}/{pair_count}")
        exact_sums = {name: math.fsum(values) for name, values in terms.items()}
        mixed_terms = {
            path: {name: [] for name in ("even_re", "even_im", "odd_re", "odd_im")}
            for path in ("quad_recurrence_rust_pairs", "rust_recurrence_quad_pairs", "quad_recurrence_quad_pairs")
        }
        qrec_by_degree_for_mix = {x["degree"]: x for x in quad_rec.get((m, ring), [])}
        for degree, bits in rust_rec[m, ring]:
            if degree in qrec_by_degree_for_mix:
                qstate = qrec_by_degree_for_mix[degree]["decimal"][1]
            else:
                qstate = qrec_by_degree_for_mix[degree - 2]["decimal"][6]
            rstate = D.from_float(f32(bits[4]))
            base = degree - 2
            qpair = quad_pairs.get((m, base))
            qcoeffs = qpair["decimal"] if qpair else (D(0), D(0), D(0), D(0))
            rcoeffs = tuple(D.from_float(f32(bits[i])) for i in (8, 10, 9, 11))
            for name, idx in (("even_re", 0), ("even_im", 1), ("odd_re", 2), ("odd_im", 3)):
                mixed_terms["quad_recurrence_rust_pairs"][name].append(qstate * rcoeffs[idx])
                mixed_terms["rust_recurrence_quad_pairs"][name].append(rstate * qcoeffs[idx])
                mixed_terms["quad_recurrence_quad_pairs"][name].append(qstate * qcoeffs[idx])
        pair_count = (720 - m + 2) // 2
        if len(rust_rec[m, ring]) < pair_count:
            base = m + 2 * (pair_count - 1)
            tail_state_rust = D.from_float(f32(rust_rec[m, ring][-1][1][7]))
            tail_state_quad = qrec_by_degree_for_mix[base]["decimal"][6]
            tail_pair_rust = rust_pairs[m, base]
            tail_pair_quad = quad_pairs[m, base]["decimal"]
            for name, idx in (("even_re", 0), ("even_im", 1), ("odd_re", 2), ("odd_im", 3)):
                mixed_terms["quad_recurrence_rust_pairs"][name].append(
                    tail_state_quad * D.from_float(f32(tail_pair_rust[idx]))
                )
                mixed_terms["rust_recurrence_quad_pairs"][name].append(
                    tail_state_rust * tail_pair_quad[idx]
                )
                mixed_terms["quad_recurrence_quad_pairs"][name].append(
                    tail_state_quad * tail_pair_quad[idx]
                )
        mixed_sums = {
            path: {name: sum(values, D(0)) for name, values in channels.items()}
            for path, channels in mixed_terms.items()
        }
        mixed_sums["rust_recurrence_rust_pairs"] = {
            name: D.from_float(value) for name, value in exact_sums.items()
        }
        ideal_sums = {name: f32_bits(value) for name, value in exact_sums.items()}
        cosine = f32(ro["cos"])
        cosine_d = D.from_float(cosine)
        cosine_q = qd["c"]
        ideal_maps = {}
        for channel in ("re", "im"):
            even32 = f32(ideal_sums[f"even_{channel}"])
            odd32 = f32(ideal_sums[f"odd_{channel}"])
            t2 = f32_bits(cosine * odd32)
            ideal_maps[f"north_{channel}"] = f32_bits(even32 + f32(t2))
            ideal_maps[f"south_{channel}"] = f32_bits(even32 - f32(t2))

        prefix_info = prefix.get((m, ring), {})
        prefix_rows.append((m, ring, prefix_info.get("active_lanes", ""),
            prefix_info.get("scaled_pairs", ""), prefix_info.get("ordinary_start_degree", ""),
            prefix_info.get("reached_full_ieee", "")))

        qrec_by_degree = {x["degree"]: x for x in quad_rec.get((m, ring), [])}
        rust_by_degree = {degree: bits for degree, bits in rust_rec[m, ring]}
        common_degrees = sorted(set(qrec_by_degree) & set(rust_by_degree))
        if not common_degrees:
            raise SystemExit(f"no matched recurrence states m={m} ring={ring}")
        first = {name: "" for name in ("csq", "coefficient", "state", "leading", "next")}
        highband_current_ulp = []
        highband_current_abs = []
        for degree in common_degrees:
            qr = qrec_by_degree[degree]
            qb = qr["bits"]
            qdrec = qr["decimal"]
            rb = rust_by_degree[degree]
            actual = {
                "csq": rb[0], "a": rb[1], "b": rb[2], "prev": rb[3],
                "current": rb[4], "leading": rb[5], "next": rb[6],
                "actual_next": rb[7],
            }
            expected = {
                "prev": qb[0], "current": qb[1], "a": qb[2], "b": qb[3],
                "csq": qb[4], "leading": qb[5], "next": qb[6],
                "actual_next": qb[6],
            }
            for stage, names in (("csq", ("csq",)), ("coefficient", ("a", "b")),
                                 ("state", ("prev", "current")),
                                 ("leading", ("leading",)), ("next", ("next", "actual_next"))):
                if first[stage] == "" and any(actual[name] != expected[name] for name in names):
                    first[stage] = degree
            current_ulp = ulp(actual["current"], expected["current"])
            if degree >= 600:
                highband_current_ulp.append(current_ulp)
                highband_current_abs.append(abs(f32(actual["current"]) - float(qdrec[1])))
            recurrence_rows.append((
                m, ring, degree,
                f"{actual['csq']:08x}", f"{expected['csq']:08x}", ulp(actual["csq"], expected["csq"]),
                f"{actual['a']:08x}", f"{expected['a']:08x}", ulp(actual["a"], expected["a"]),
                f"{actual['b']:08x}", f"{expected['b']:08x}", ulp(actual["b"], expected["b"]),
                f"{actual['prev']:08x}", f"{expected['prev']:08x}", ulp(actual["prev"], expected["prev"]),
                f"{actual['current']:08x}", f"{expected['current']:08x}", current_ulp,
                f"{actual['leading']:08x}", f"{expected['leading']:08x}", ulp(actual["leading"], expected["leading"]),
                f"{actual['next']:08x}", f"{expected['next']:08x}", ulp(actual["next"], expected["next"]),
                format(qdrec[0], ".30g"), format(qdrec[1], ".30g"),
                format(qdrec[2], ".30g"), format(qdrec[3], ".30g"),
                format(qdrec[4], ".30g"), format(qdrec[5], ".30g"), format(qdrec[6], ".30g"),
            ))
        first_kind, first_degree = "", ""
        for stage in ("csq", "coefficient", "state", "leading", "next"):
            if first[stage] != "":
                first_kind, first_degree = stage, first[stage]
                break
        recurrence_first_rows.append((
            m, ring, prefix_info.get("scaled_pairs", ""),
            prefix_info.get("ordinary_start_degree", ""),
            first_kind, first_degree, first["csq"], first["coefficient"],
            first["state"], first["leading"], first["next"],
            max(highband_current_ulp, default=0), max(highband_current_abs, default=0.0),
            len(common_degrees),
        ))
        component_specs = [
            ("north_re", ro["north"][0], dbits[0], q_values[0], q_bits[0], scale_for[0]),
            ("north_im", ro["north"][1], dbits[1], q_values[1], q_bits[1], scale_for[1]),
            ("south_re", ro["south"][0], dbits[2], q_values[2], q_bits[2], scale_for[0]),
            ("south_im", ro["south"][1], dbits[3], q_values[3], q_bits[3], scale_for[1]),
        ]
        for name, rb, db, q, qb, scale in component_specs:
            rv, dv = f32(rb), f32(db)
            ra, da = abs(D.from_float(rv) - q), abs(D.from_float(dv) - q)
            denom = max(abs(q), scale, D("1e-100"))
            r_scaled, d_scaled = float(ra / denom), float(da / denom)
            ideal_bits = ideal_maps[name]
            ideal_value = f32(ideal_bits)
            accumulation_delta = rv - ideal_value
            nonaccum_error = ideal_value - float(q)
            accumulation_effects.append(abs(accumulation_delta))
            nonaccum_errors.append(abs(nonaccum_error))
            output_rows.append((
                m, ring, name, f"{ro['theta']:016x}", f"{rb:08x}", f"{db:08x}",
                f"{qb:08x}", format(q, ".36g"), float(ra),
                float(ra / abs(q)) if q else "", r_scaled, ulp(rb, qb), float(da),
                float(da / abs(q)) if q else "", d_scaled, ulp(db, qb),
                ulp(rb, db), f"{ideal_bits:08x}", abs(accumulation_delta),
                nonaccum_error, abs(nonaccum_error),
            ))
            candidates.append((float(ra), r_scaled, m, ring, name, float(q)))
            duccs.append((float(da), d_scaled, m, ring, name, float(q)))
            rust_vs_ducc.append(ulp(rb, db))

        for name, idx in (("even_re", 0), ("even_im", 1), ("odd_re", 0), ("odd_im", 1)):
            actual_bits = ro["even"][idx] if name.startswith("even") else ro["odd"][idx]
            exact_sum = exact_sums[name]
            actual = f32(actual_bits)
            abs_error = abs(actual - exact_sum)
            magnitude = math.fsum(abs(x) for x in terms[name])
            accum_rows.append((
                m, ring, name, f"{actual_bits:08x}", exact_sum, actual,
                abs_error, abs_error / max(abs(exact_sum), magnitude, 1e-100),
                magnitude / max(abs(exact_sum), 1e-100), len(terms[name]),
            ))

        q_output_values = {"north_re": qd["nr"], "north_im": qd["ni"],
                           "south_re": qd["sr"], "south_im": qd["eveni"] - cosine_q * qd["oddi"]}
        for path, sums in mixed_sums.items():
            for channel in ("re", "im"):
                even = sums[f"even_{channel}"]
                odd = sums[f"odd_{channel}"]
                for side, sign in (("north", D(1)), ("south", D(-1))):
                    component = f"{side}_{channel}"
                    value_rust_cos = even + sign * cosine_d * odd
                    error = value_rust_cos - q_output_values[component]
                    denominator = max(abs(q_output_values[component]), scale_for[0 if channel == "re" else 1], D("1e-100"))
                    controlled_rows.append((
                        m, ring, path, component, format(value_rust_cos, ".36g"),
                        format(q_output_values[component], ".36g"), format(error, ".36g"),
                        format(abs(error), ".36g"), float(abs(error) / denominator),
                    ))
        for channel in ("re", "im"):
            even = mixed_sums["quad_recurrence_quad_pairs"][f"even_{channel}"]
            odd = mixed_sums["quad_recurrence_quad_pairs"][f"odd_{channel}"]
            for side, sign in (("north", D(1)), ("south", D(-1))):
                component = f"{side}_{channel}"
                value = even + sign * cosine_q * odd
                error = value - q_output_values[component]
                denominator = max(abs(q_output_values[component]), scale_for[0 if channel == "re" else 1], D("1e-100"))
                controlled_rows.append((m, ring, "quad_recurrence_quad_pairs_quad_cos", component,
                    format(value, ".36g"), format(q_output_values[component], ".36g"),
                    format(error, ".36g"), format(abs(error), ".36g"), float(abs(error) / denominator)))

        # Keep only the upper-band prepared pair states. Compare each native coefficient with
        # the high-precision formula and its correctly rounded f32 value.
    for m, base in sorted(rust_pairs):
        rb = rust_pairs[m, base]
        qp = quad_pairs[m, base]
        for i, part in enumerate(("even_re", "even_im", "odd_re", "odd_im")):
            actual, expected = rb[i], qp["bits"][i]
            q = qp["decimal"][i]
            error = abs(D.from_float(f32(actual)) - q)
            pair_rows.append((
                m, base, part, f"{actual:08x}", f"{expected:08x}",
                format(q, ".36g"), float(error), ulp(actual, expected),
            ))

    write_csv(output_dir / "dense-band-output-comparison.csv", (
        "order", "ring", "component", "theta_bits", "rust_f32_bits", "ducc_f32_bits",
        "quad128_rounded_f32_bits", "quad128_value", "rust_abs_error", "rust_relative_error",
        "rust_cancellation_scaled_error", "rust_ulp_to_quad32", "ducc_abs_error",
        "ducc_relative_error", "ducc_cancellation_scaled_error", "ducc_ulp_to_quad32",
        "rust_ducc_ulp", "same_f32_terms_correctly_rounded_output_bits",
        "accumulation_only_absolute_delta", "same_f32_terms_minus_quad_signed",
        "same_f32_terms_minus_quad_absolute",
    ), output_rows)
    write_csv(output_dir / "dense-band-accumulation.csv", (
        "order", "ring", "sum", "rust_f32_bits", "sum_of_same_f32_terms_binary64",
        "rust_value", "absolute_accumulation_error", "scaled_accumulation_error",
        "term_cancellation_condition", "trace_terms",
    ), accum_rows)
    write_csv(output_dir / "dense-band-pairs.csv", (
        "order", "base_degree", "coefficient", "rust_f32_bits", "quad_rounded_f32_bits",
        "quad128_value", "absolute_error", "ulp_to_quad32",
    ), pair_rows)
    write_csv(output_dir / "dense-band-recurrence.csv", (
        "order", "ring", "degree",
        "rust_csq_bits", "quad_rounded_f32_csq_bits", "csq_ulp",
        "rust_a_bits", "quad_rounded_f32_a_bits", "a_ulp",
        "rust_b_bits", "quad_rounded_f32_b_bits", "b_ulp",
        "rust_previous_bits", "quad_rounded_f32_previous_bits", "previous_ulp",
        "rust_current_bits", "quad_rounded_f32_current_bits", "current_ulp",
        "rust_leading_bits", "quad_rounded_f32_leading_bits", "leading_ulp",
        "rust_next_bits", "quad_rounded_f32_next_bits", "next_ulp",
        "quad128_previous", "quad128_current", "quad128_a", "quad128_b",
        "quad128_cos_squared", "quad128_leading", "quad128_next",
    ), recurrence_rows)
    write_csv(output_dir / "dense-band-recurrence-first.csv", (
        "order", "ring", "scaled_prefix_pairs", "ordinary_start_degree",
        "first_differing_stage", "first_differing_degree", "first_csq_difference_degree",
        "first_a_or_b_difference_degree", "first_state_difference_degree",
        "first_leading_fma_difference_degree", "first_state_update_difference_degree",
        "max_highband_current_ulp", "max_highband_current_absolute_error", "matched_recurrence_steps",
    ), recurrence_first_rows)
    recurrence_summary_rows = []
    for m, ring in keys:
        group = [row for row in recurrence_rows if row[0] == m and row[1] == ring and row[2] >= 600]
        for target in (600, 650, 700, 720):
            row = min(group, key=lambda x: (abs(x[2] - target), x[2]))
            actual_current = f32(int(row[15], 16))
            q_current = D(row[25])
            recurrence_summary_rows.append((
                m, ring, f"nearest_{target}", row[2], row[15], row[16], row[17],
                abs(D.from_float(actual_current) - q_current),
                float(abs(D.from_float(actual_current) - q_current) / abs(q_current)) if q_current else "",
                row[21], row[22], row[23],
            ))
        worst = max(group, key=lambda x: float(abs(D.from_float(f32(int(x[15], 16))) - D(x[25]))))
        actual_current = f32(int(worst[15], 16))
        q_current = D(worst[25])
        recurrence_summary_rows.append((
            m, ring, "max_abs_current_error", worst[2], worst[15], worst[16], worst[17],
            abs(D.from_float(actual_current) - q_current),
            float(abs(D.from_float(actual_current) - q_current) / abs(q_current)) if q_current else "",
            worst[21], worst[22], worst[23],
        ))
    write_csv(output_dir / "dense-band-recurrence-summary.csv", (
        "order", "ring", "sample", "degree", "rust_current_bits",
        "quad_rounded_f32_current_bits", "current_ulp", "current_absolute_error",
        "current_relative_error", "rust_next_bits", "quad_rounded_f32_next_bits", "next_ulp",
    ), recurrence_summary_rows)
    write_csv(output_dir / "dense-band-prefix.csv", (
        "order", "ring", "active_lanes", "scaled_prefix_pairs",
        "ordinary_start_degree", "reached_full_ieee",
    ), prefix_rows)
    write_csv(output_dir / "dense-band-controlled-precision.csv", (
        "order", "ring", "controlled_path", "component", "controlled_value",
        "quad128_reference", "signed_error", "absolute_error", "cancellation_scaled_error",
    ), controlled_rows)

    def stats(items):
        errors = [x[0] for x in items]
        scaled = [x[1] for x in items]
        worst = max(items)
        return {
            "count": len(items),
            "max_absolute_error": max(errors),
            "rms_absolute_error": math.sqrt(math.fsum(x*x for x in errors) / len(errors)),
            "max_cancellation_scaled_error": max(scaled),
            "rms_cancellation_scaled_error": math.sqrt(math.fsum(x*x for x in scaled) / len(scaled)),
            "worst_case": {"order": worst[2], "ring": worst[3], "component": worst[4], "reference": worst[5]},
        }

    sum_acc = [row[6] for row in accum_rows]
    controlled_summary = {}
    for path in sorted({row[2] for row in controlled_rows}):
        group = [row for row in controlled_rows if row[2] == path]
        errors = [float(D(row[7])) for row in group]
        scaled = [float(row[8]) for row in group]
        worst = max(group, key=lambda row: float(D(row[7])))
        controlled_summary[path] = {
            "count": len(group),
            "max_absolute_error": max(errors),
            "rms_absolute_error": math.sqrt(math.fsum(x*x for x in errors) / len(errors)),
            "max_cancellation_scaled_error": max(scaled),
            "rms_cancellation_scaled_error": math.sqrt(math.fsum(x*x for x in scaled) / len(scaled)),
            "worst_case": {"order": worst[0], "ring": worst[1], "component": worst[3]},
        }
    summary = {
        "cases": {"orders": 16, "north_representative_rings": [0, 1, 5, 360], "paired_south_rings": [720, 719, 715, 360], "outputs": len(output_rows)},
        "rust_vs_quad128": stats(candidates),
        "ducc_vs_quad128": stats(duccs),
        "rust_ducc_ulp": {"max": max(rust_vs_ducc), "nonzero": sum(x != 0 for x in rust_vs_ducc)},
        "same_f32_terms_accumulation_attribution": {
            "max_accumulation_only_output_delta": max(accumulation_effects),
            "rms_accumulation_only_output_delta": math.sqrt(math.fsum(x*x for x in accumulation_effects) / len(accumulation_effects)),
            "max_remaining_recurrence_coefficient_geometry_output_error": max(nonaccum_errors),
            "rms_remaining_recurrence_coefficient_geometry_output_error": math.sqrt(math.fsum(x*x for x in nonaccum_errors) / len(nonaccum_errors)),
        },
        "controlled_precision_substitutions": controlled_summary,
        "native_f32_prefix": {
            "cases": len(prefix_rows),
            "max_scaled_prefix_pairs": max(int(row[3]) for row in prefix_rows),
            "all_reach_full_ieee": all(row[5] is True for row in prefix_rows),
            "ordinary_start_degrees": sorted({int(row[4]) for row in prefix_rows}),
        },
        "recurrence": {
            "matched_steps": len(recurrence_rows),
            "first_differing_stage_counts": {
                stage: sum(row[4] == stage for row in recurrence_first_rows)
                for stage in ("csq", "coefficient", "state", "leading", "next", "")
            },
            "max_highband_current_ulp": max(row[11] for row in recurrence_first_rows),
            "max_highband_current_absolute_error": max(row[12] for row in recurrence_first_rows),
        },
        "prepared_pairs": {
            "count": len(pair_rows),
            "nonzero_ulp_to_quad_f32": sum(row[7] != 0 for row in pair_rows),
            "max_ulp_to_quad_f32": max(row[7] for row in pair_rows),
            "max_absolute_error": max(row[6] for row in pair_rows),
        },
        "f32_accumulation_only": {
            "count": len(accum_rows),
            "max_absolute_error": max(sum_acc),
            "rms_absolute_error": math.sqrt(math.fsum(x*x for x in sum_acc) / len(sum_acc)),
            "max_condition": max(row[8] for row in accum_rows),
        },
    }
    (output_dir / "dense-band-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
