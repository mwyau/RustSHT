#!/usr/bin/env python3
"""One-change-at-a-time recurrence replays for the archived dense T720 trace."""
from __future__ import annotations

import csv
import ctypes
import gzip
import math
import re
import struct
import sys
from decimal import Decimal, getcontext
from pathlib import Path

getcontext().prec = 100
D = Decimal
LIBM = ctypes.CDLL("libm.so.6")
FMAF = LIBM.fmaf
FMAF.argtypes = (ctypes.c_float, ctypes.c_float, ctypes.c_float)
FMAF.restype = ctypes.c_float
LIBC = ctypes.CDLL(None)
STRTOF = LIBC.strtof
STRTOF.argtypes = (ctypes.c_char_p, ctypes.c_void_p)
STRTOF.restype = ctypes.c_float


def read_gzip_lines(path: Path):
    with gzip.open(path, "rt", newline="") as stream:
        yield from stream


def f32(bits: int) -> float:
    return struct.unpack("<f", struct.pack("<I", bits & 0xFFFFFFFF))[0]


def f32d(bits: int) -> Decimal:
    return D.from_float(f32(bits))


def f32bits(value: Decimal | float) -> int:
    text = str(value).encode("ascii")
    return struct.unpack("<I", struct.pack("<f", STRTOF(text, None)))[0]


def ordered(bits: int) -> int:
    bits &= 0xFFFFFFFF
    return (~bits & 0xFFFFFFFF) if bits & 0x80000000 else (bits | 0x80000000)


def ulp(a: int, b: int) -> int:
    return abs(ordered(a) - ordered(b))


def fma32(a: int, b: int, c: int) -> int:
    return struct.unpack(
        "<I", struct.pack("<f", FMAF(f32(a), f32(b), f32(c)))
    )[0]


def mul32(a: int, b: int) -> int:
    return f32bits(f32(a) * f32(b))


def add32(a: int, b: int) -> int:
    return f32bits(f32(a) + f32(b))


def write_csv_gz(path: Path, header, rows):
    with gzip.open(path, "wt", newline="", compresslevel=9) as stream:
        writer = csv.writer(stream)
        writer.writerow(header)
        writer.writerows(rows)


def write_csv(path: Path, header, rows):
    with path.open("w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(header)
        writer.writerows(rows)


def load_rust(path: Path):
    rec, pairs, outputs = {}, {}, {}
    for line in read_gzip_lines(path):
        p = line.rstrip("\n").split(",")
        if p[0] == "PAIR_STATE":
            key = (int(p[1]), int(p[2]))
            val = tuple(int(x, 16) & 0xFFFFFFFF for x in p[3:7])
            if key in pairs and pairs[key] != val:
                raise ValueError(f"inconsistent Rust pair state {key}")
            pairs[key] = val
        elif p[0] == "REC":
            key, degree = (int(p[1]), int(p[2])), int(p[3])
            vals = tuple(int(x, 16) & 0xFFFFFFFF for x in p[4:] if x)
            if len(vals) != 16:
                raise ValueError(f"bad Rust REC row: {line[:160]}")
            rec.setdefault(key, {})[degree] = vals
        elif p[0] == "OUTPUT":
            m, ring = int(p[1]), int(p[2])
            fields = dict(re.findall(r"(theta|sin|cos|csq)=([0-9a-f]+)", line))
            complex_fields = {}
            for name in ("even", "odd", "north", "south"):
                match = re.search(rf"{name}=\(([^)]*)\)", line)
                complex_fields[name] = tuple(int(x, 16) for x in match.group(1).split(","))
            outputs[m, ring] = {
                **{k: int(v, 16) for k, v in fields.items()},
                **complex_fields,
            }
    return rec, pairs, outputs


def load_quad(path: Path):
    rec, pairs, outputs = {}, {}, {}
    for line in read_gzip_lines(path):
        p = line.rstrip("\n").split(",")
        if p[0] == "PAIR_STATE":
            key = int(p[1]), int(p[2])
            value = tuple(D(x) for x in p[3:7])
            bits = tuple(int(x, 16) for x in p[7:11])
            pairs[key] = {"decimal": value, "bits": bits}
        elif p[0] == "QUAD_REC":
            key, degree = (int(p[1]), int(p[2])), int(p[3])
            rec.setdefault(key, {})[degree] = {
                "decimal": tuple(D(x) for x in p[4:11]),
                "bits": tuple(int(x, 16) for x in p[11:18]),
            }
        elif p[0] == "QUAD_OUT":
            m, ring = int(p[1]), int(p[2])
            dec = dict(re.findall(r"(c|evenr|oddr|nr|sr|eveni|oddi|ni)=([^,]+)", line))
            bit_text = re.search(r"bits=([0-9a-f:]+)", line).group(1)
            outputs[m, ring] = {
                "theta": int(p[3], 16),
                "decimal": {k: D(v) for k, v in dec.items()},
                "bits": tuple(int(x, 16) for x in bit_text.split(":")),
            }
    return rec, pairs, outputs


def exact_state_replay(qrows, m, *, seed_mode, factor_mode, arithmetic, quantize_state,
                       rust_seed_bits=None):
    first_degree = m + 2
    first = qrows[first_degree]
    if seed_mode == "q":
        previous, current = D(0), first["decimal"][1]
    elif seed_mode == "q32":
        previous, current = D(0), f32d(first["bits"][1])
    elif seed_mode == "rust":
        if rust_seed_bits is None:
            raise ValueError("Rust seed bits not supplied")
        previous, current = D(0), f32d(rust_seed_bits)
    else:
        raise ValueError(seed_mode)
    states = {m: current}
    ops = {}
    for degree in sorted(d for d in qrows if first_degree <= d <= 720):
        q = qrows[degree]
        _, _, qa, qb, qcsq, qlead, qnext = q["decimal"]
        if factor_mode == "q":
            a, b, csq = qa, qb, qcsq
            abits, bbits, cbits = q["bits"][2:5]
        elif factor_mode == "ab32":
            abits, bbits, cbits = q["bits"][2], q["bits"][3], None
            a, b, csq = f32d(abits), f32d(bbits), qcsq
        elif factor_mode == "csq32":
            abits, bbits, cbits = None, None, q["bits"][4]
            a, b, csq = qa, qb, f32d(cbits)
        elif factor_mode == "all32":
            abits, bbits, cbits = q["bits"][2:5]
            a, b, csq = f32d(abits), f32d(bbits), f32d(cbits)
        else:
            raise ValueError(factor_mode)

        if arithmetic == "exact":
            leading = a * csq + b
            nxt = leading * current + previous
            lead_bits = f32bits(leading)
            next_bits = f32bits(nxt)
        elif arithmetic == "fma32":
            if abits is None:
                abits, bbits = f32bits(a), f32bits(b)
            if cbits is None:
                cbits = f32bits(csq)
            lead_bits = fma32(abits, cbits, bbits)
            current_bits, previous_bits = f32bits(current), f32bits(previous)
            next_bits = fma32(lead_bits, current_bits, previous_bits)
            leading, nxt = f32d(lead_bits), f32d(next_bits)
        elif arithmetic == "separate32":
            if abits is None:
                abits, bbits = f32bits(a), f32bits(b)
            if cbits is None:
                cbits = f32bits(csq)
            lead_bits = add32(mul32(abits, cbits), bbits)
            current_bits, previous_bits = f32bits(current), f32bits(previous)
            next_bits = add32(mul32(lead_bits, current_bits), previous_bits)
            leading, nxt = f32d(lead_bits), f32d(next_bits)
        else:
            raise ValueError(arithmetic)

        if quantize_state and arithmetic == "exact":
            next_bits = f32bits(nxt)
            nxt = f32d(next_bits)
        ops[degree] = {
            "previous": previous, "current": current,
            "a": a, "b": b, "csq": csq,
            "leading": leading, "next": nxt,
            "leading_bits": lead_bits, "next_bits": next_bits,
        }
        states[degree] = nxt
        previous, current = current, nxt
    return states, ops


def rust_operands_replay(rustrows, m, seed_bits):
    states = {m: f32d(seed_bits)}
    previous = 0
    current = seed_bits
    ops = {}
    for degree in sorted(d for d in rustrows if m + 2 <= d <= 720):
        x = rustrows[degree]
        csq, a, b = x[0], x[1], x[2]
        leading = fma32(a, csq, b)
        nxt = fma32(leading, current, previous)
        if leading != x[5] or nxt != x[7] or nxt != x[6]:
            raise AssertionError(f"Rust f32 replay mismatch m={m}, degree={degree}")
        ops[degree] = {
            "previous": f32d(previous), "current": f32d(current),
            "a": f32d(a), "b": f32d(b), "csq": f32d(csq),
            "leading": f32d(leading), "next": f32d(nxt),
            "leading_bits": leading, "next_bits": nxt,
        }
        states[degree] = f32d(nxt)
        previous, current = current, nxt
    return states, ops


def state_from_trace(qrows, m, *, rustrows=None):
    first = qrows[m + 2]
    states = {m: first["decimal"][1]}
    for d, row in qrows.items():
        if m + 2 <= d <= 720:
            states[d] = row["decimal"][6]
    if rustrows is not None:
        states = {m: f32d(rustrows[m + 2][4])}
        for d, row in rustrows.items():
            if m + 2 <= d <= 720:
                states[d] = f32d(row[7])
    return states


def state_bits_from_quad_trace(qrows, m):
    bits = {m: qrows[m + 2]["bits"][1]}
    for degree, row in qrows.items():
        if m + 2 <= degree <= 720:
            bits[degree] = row["bits"][6]
    return bits


def accumulate(states, pairs, m, cosine):
    even_re = even_im = odd_re = odd_im = D(0)
    for (order, base), coeffs in pairs.items():
        if order != m:
            continue
        state = states.get(base)
        if state is None:
            raise AssertionError(f"missing state m={m}, l={base}")
        even_re += state * coeffs[0]
        even_im += state * coeffs[1]
        odd_re += state * coeffs[2]
        odd_im += state * coeffs[3]
    return {
        "even_re": even_re, "even_im": even_im,
        "odd_re": odd_re, "odd_im": odd_im,
        "north_re": even_re + cosine * odd_re,
        "south_re": even_re - cosine * odd_re,
        "north_im": even_im + cosine * odd_im,
        "south_im": even_im - cosine * odd_im,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: controlled_f32_replay.py dense-band-dir output-dir")
    source = Path(sys.argv[1])
    dest = Path(sys.argv[2])
    dest.mkdir(parents=True, exist_ok=True)
    rust_rec, rust_pairs_bits, rust_outputs = load_rust(source / "rust-f32-trace.csv.gz")
    quad_rec, quad_pairs, quad_outputs = load_quad(source / "quad128-reference.csv.gz")
    cases = [(m, r) for m in range(1, 17) for r in (0, 1, 5, 360)]
    if len(cases) != 64 or not set(cases).issubset(quad_outputs) or not set(cases).issubset(rust_outputs):
        raise AssertionError(f"selected north-ring matrix missing: Q={len(quad_outputs)} Rust={len(rust_outputs)}")
    if set(quad_pairs) != set(rust_pairs_bits):
        raise AssertionError("prepared coefficient pair sets differ")

    variants = {
        "q_inputs_100digit_recurrence": ("q", "q", "exact", False),
        "seed_rounded_f32_only": ("q32", "q", "exact", False),
        "rust_seed_q_factors_high_precision": ("rust", "q", "exact", False),
        "a_b_rounded_f32_only": ("q", "ab32", "exact", False),
        "csq_rounded_f32_only": ("q", "csq32", "exact", False),
        "a_b_csq_rounded_f32_exact_state": ("q", "all32", "exact", False),
        "state_rounded_f32_each_update_only": ("q", "q", "exact", True),
        "seed_and_state_rounded_f32_q_factors": ("q32", "q", "exact", True),
        "rust_seed_and_state_q_factors": ("rust", "q", "exact", True),
        "q_rounded_operands_native_f32_fma": ("q32", "all32", "fma32", False),
        "rust_seed_q32_factors_native_f32_fma": ("rust", "all32", "fma32", False),
        "q_rounded_operands_f32_separate_ops": ("q32", "all32", "separate32", False),
    }

    all_states = {}
    all_ops = {}
    actual_replay_checks = []
    for m, ring in cases:
        qrows = quad_rec[m, ring]
        rrows = rust_rec[m, ring]
        first = rrows[m + 2]
        rust_seed = first[4]
        actual_states = state_from_trace(qrows, m, rustrows=rrows)
        rust_replay_states, rust_replay_ops = rust_operands_replay(rrows, m, rust_seed)
        if actual_states != rust_replay_states:
            raise AssertionError(f"Rust FMA state replay mismatch m={m}, ring={ring}")
        actual_replay_checks.append((m, ring, len(rust_replay_ops), "PASS"))

        case_states = {"quad128_trace": state_from_trace(qrows, m), "rust_native_trace": actual_states}
        case_ops = {"rust_native_trace": rust_replay_ops}
        for name, (seed_mode, factor_mode, arithmetic, quantize) in variants.items():
            states, ops = exact_state_replay(
                qrows, m, seed_mode=seed_mode, factor_mode=factor_mode,
                arithmetic=arithmetic, quantize_state=quantize,
                rust_seed_bits=rust_seed,
            )
            case_states[name] = states
            case_ops[name] = ops
        all_states[m, ring] = case_states
        all_ops[m, ring] = case_ops

    pair_sets = {
        "quad128_pairs": {k: v["decimal"] for k, v in quad_pairs.items()},
        "rust_f32_pairs": {
            k: tuple(f32d(x) for x in values) for k, values in rust_pairs_bits.items()
        },
    }
    state_growth, first_divergence, recurrence_metrics = [], [], []
    checkpoints = (600, 612, 650, 700, 720)
    for (m, ring), path_states in sorted(all_states.items()):
        qstate = state_from_trace(quad_rec[m, ring], m)
        qstate_bits = state_bits_from_quad_trace(quad_rec[m, ring], m)
        case_degrees = sorted(qstate)
        rqrows = rust_rec[m, ring]
        selected_case = m in (1, 5, 8, 12, 16)
        for path, states in path_states.items():
            diffs = []
            for degree in case_degrees:
                if degree not in states:
                    continue
                value = states[degree]
                reference = qstate[degree]
                error = value - reference
                abs_error = abs(error)
                ref32 = qstate_bits[degree]
                val32 = f32bits(value)
                rel = abs_error / abs(reference) if reference else D(0)
                diffs.append((degree, abs_error, rel, ulp(val32, ref32)))
                if degree in (m, m + 2, *checkpoints):
                    state_growth.append((
                        m, ring, path, degree, format(value, ".40g"),
                        format(reference, ".40g"), format(error, ".12g"),
                        format(abs_error, ".12g"), format(rel, ".12g"),
                        f"{val32:08x}", f"{ref32:08x}", ulp(val32, ref32),
                    ))
            unequal = [item for item in diffs if item[1] != 0]
            if unequal:
                d, abs_error, rel, distance = unequal[0]
                value, reference = states[d], qstate[d]
                first_divergence.append((
                    m, ring, path, d, format(value, ".40g"), format(reference, ".40g"),
                    format(value - reference, ".12g"), format(abs_error, ".12g"),
                    format(rel, ".12g"), f"{f32bits(value):08x}",
                    f"{f32bits(reference):08x}", distance,
                ))
            else:
                first_divergence.append((m, ring, path, "", "", "", "0", "0", "0", "", "", 0))
            max_abs = max((x[1] for x in diffs), default=D(0))
            max_rel = max((x[2] for x in diffs), default=D(0))
            max_ulp = max((x[3] for x in diffs), default=0)
            at720 = next((x for x in diffs if x[0] == 720), None)
            recurrence_metrics.append((
                m, ring, path, len(diffs), format(max_abs, ".12g"),
                format(max_rel, ".12g"), max_ulp,
                format(at720[1], ".12g") if at720 else "",
                format(at720[2], ".12g") if at720 else "",
                at720[3] if at720 else "",
            ))

    write_csv_gz(dest / "controlled-f32-state-growth.csv.gz", (
        "order", "ring", "path", "degree", "state", "quad128_state",
        "signed_error", "absolute_error", "relative_error", "state_f32_bits",
        "quad128_rounded_f32_bits", "ulp_to_quad128_f32",
    ), state_growth)
    write_csv_gz(dest / "controlled-f32-first-divergence.csv.gz", (
        "order", "ring", "path", "first_degree", "state", "quad128_state",
        "signed_error", "absolute_error", "relative_error", "state_f32_bits",
        "quad128_rounded_f32_bits", "ulp_to_quad128_f32",
    ), first_divergence)
    write_csv_gz(dest / "controlled-f32-recurrence-summary.csv.gz", (
        "order", "ring", "path", "state_count", "max_abs_state_error",
        "max_relative_state_error", "max_ulp_to_quad128_f32", "degree720_abs_error",
        "degree720_relative_error", "degree720_ulp_to_quad128_f32",
    ), recurrence_metrics)
    write_csv(dest / "controlled-f32-native-replay-checks.csv", (
        "order", "ring", "recurrence_updates", "result",
    ), actual_replay_checks)

    # Verify and summarize leading/next operation changes on the same 20 selected cases.
    op_rows = []
    for (m, ring), paths in sorted(all_ops.items()):
        if m not in (1, 5, 8, 12, 16):
            continue
        qrows = quad_rec[m, ring]
        for path in ("a_b_rounded_f32_only", "csq_rounded_f32_only",
                     "a_b_csq_rounded_f32_exact_state",
                     "state_rounded_f32_each_update_only",
                     "seed_and_state_rounded_f32_q_factors",
                     "q_rounded_operands_native_f32_fma",
                     "q_rounded_operands_f32_separate_ops",
                     "rust_native_trace"):
            ops = paths[path]
            for degree, op in sorted(ops.items()):
                if degree not in (m + 2, 600, 612, 650, 700, 720):
                    continue
                q = qrows[degree]
                qd = q["decimal"]
                qbits = q["bits"]
                op_rows.append((
                    m, ring, path, degree,
                    format(op["a"], ".36g"), format(qd[2], ".36g"),
                    format(op["b"], ".36g"), format(qd[3], ".36g"),
                    format(op["csq"], ".36g"), format(qd[4], ".36g"),
                    format(op["current"], ".36g"), format(qd[1], ".36g"),
                    format(op["leading"], ".36g"), format(qd[5], ".36g"),
                    format(op["next"], ".36g"), format(qd[6], ".36g"),
                    f"{op['leading_bits']:08x}", f"{qbits[5]:08x}",
                    f"{op['next_bits']:08x}", f"{qbits[6]:08x}",
                ))
    write_csv_gz(dest / "controlled-f32-operation-checkpoints.csv.gz", (
        "order", "ring", "path", "degree", "used_a", "quad128_a", "used_b",
        "quad128_b", "used_csq", "quad128_csq", "current", "quad128_current",
        "leading", "quad128_leading", "next", "quad128_next", "leading_f32_bits",
        "quad128_leading_rounded_f32_bits", "next_f32_bits", "quad128_next_rounded_f32_bits",
    ), op_rows)

    # Wide-precision accumulation isolates recurrence, prepared pairs, and cosine.
    output_rows = []
    output_specs = (
        ("quad128_trace", "quad128_pairs", "quad128_cosine"),
        ("quad128_trace", "quad128_pairs", "rust_f32_cosine"),
        ("quad128_trace", "rust_f32_pairs", "quad128_cosine"),
        ("quad128_trace", "rust_f32_pairs", "rust_f32_cosine"),
        ("q_inputs_100digit_recurrence", "quad128_pairs", "quad128_cosine"),
        ("rust_native_trace", "quad128_pairs", "quad128_cosine"),
        ("rust_native_trace", "rust_f32_pairs", "quad128_cosine"),
        ("rust_native_trace", "quad128_pairs", "rust_f32_cosine"),
        ("rust_native_trace", "rust_f32_pairs", "rust_f32_cosine"),
        ("seed_rounded_f32_only", "quad128_pairs", "quad128_cosine"),
        ("rust_seed_q_factors_high_precision", "quad128_pairs", "quad128_cosine"),
        ("a_b_rounded_f32_only", "quad128_pairs", "quad128_cosine"),
        ("csq_rounded_f32_only", "quad128_pairs", "quad128_cosine"),
        ("a_b_csq_rounded_f32_exact_state", "quad128_pairs", "quad128_cosine"),
        ("state_rounded_f32_each_update_only", "quad128_pairs", "quad128_cosine"),
        ("seed_and_state_rounded_f32_q_factors", "quad128_pairs", "quad128_cosine"),
        ("rust_seed_and_state_q_factors", "quad128_pairs", "quad128_cosine"),
        ("q_rounded_operands_native_f32_fma", "quad128_pairs", "quad128_cosine"),
        ("rust_seed_q32_factors_native_f32_fma", "quad128_pairs", "quad128_cosine"),
        ("q_rounded_operands_f32_separate_ops", "quad128_pairs", "quad128_cosine"),
    )
    for (m, ring), paths in sorted(all_states.items()):
        qout = quad_outputs[m, ring]["decimal"]
        reference = {
            "north_re": qout["nr"], "north_im": qout["ni"],
            "south_re": qout["sr"],
            "south_im": qout["eveni"] - qout["c"] * qout["oddi"],
        }
        rustcos = f32d(rust_outputs[m, ring]["cos"])
        cosines = {"quad128_cosine": qout["c"], "rust_f32_cosine": rustcos}
        scale_for = {
            "re": abs(qout["evenr"]) + abs(qout["c"] * qout["oddr"]),
            "im": abs(qout["eveni"]) + abs(qout["c"] * qout["oddi"]),
        }
        for state_name, pair_name, cosine_name in output_specs:
            values = accumulate(
                paths[state_name], pair_sets[pair_name], m, cosines[cosine_name]
            )
            for component, value in values.items():
                if component.startswith("even_") or component.startswith("odd_"):
                    continue
                q = reference[component]
                error = value - q
                scale = scale_for["re" if component.endswith("re") else "im"]
                normalized = abs(error) / max(abs(q), scale, D("1e-100"))
                output_rows.append((
                    m, ring, state_name, pair_name, cosine_name, component,
                    format(value, ".40g"), format(q, ".40g"), format(error, ".16g"),
                    format(abs(error), ".16g"), format(normalized, ".12g"),
                ))
        # Actual saved Rust output, including its native accumulation and final rounding.
        ro = rust_outputs[m, ring]
        for component, bits in (
            ("north_re", ro["north"][0]), ("north_im", ro["north"][1]),
            ("south_re", ro["south"][0]), ("south_im", ro["south"][1]),
        ):
            value = f32d(bits)
            q = reference[component]
            error = value - q
            scale = scale_for["re" if component.endswith("re") else "im"]
            normalized = abs(error) / max(abs(q), scale, D("1e-100"))
            output_rows.append((
                m, ring, "rust_native_actual_output", "rust_f32_pairs",
                "rust_f32_cosine_and_native_accumulation", component,
                format(value, ".40g"), format(q, ".40g"), format(error, ".16g"),
                format(abs(error), ".16g"), format(normalized, ".12g"),
            ))
    write_csv_gz(dest / "controlled-f32-output-breakdown.csv.gz", (
        "order", "ring", "recurrence_path", "pair_path", "cosine_path",
        "component", "controlled_value", "quad128_reference", "signed_error",
        "absolute_error", "cancellation_scaled_error",
    ), output_rows)

    # At each common state, compare fused and separately rounded operations with
    # identical operands. The Rust rows use the actual traced recurrence state.
    fma_rows = []
    for (m, ring), qrows in sorted(quad_rec.items()):
        if (m, ring) not in all_states:
            continue
        rrows = rust_rec[m, ring]
        for degree in sorted(d for d in qrows if m + 2 <= d <= 720):
            q = qrows[degree]
            r = rrows[degree]
            qfactor = q["bits"]
            qprev, qcurr = qfactor[0], qfactor[1]
            rcsq, ra, rb, rprev, rcurr = r[0], r[1], r[2], r[3], r[4]
            for operand_source, a, b, csq, prev, curr in (
                ("quad128_values_rounded_to_f32", qfactor[2], qfactor[3], qfactor[4], qprev, qcurr),
                ("rust_actual_f32_operands_and_state", ra, rb, rcsq, rprev, rcurr),
            ):
                leading_fma = fma32(a, csq, b)
                leading_sep = add32(mul32(a, csq), b)
                next_fma_fma = fma32(leading_fma, curr, prev)
                next_sep_same_lead = add32(mul32(leading_fma, curr), prev)
                next_fma_sep_lead = fma32(leading_sep, curr, prev)
                next_sep_sep = add32(mul32(leading_sep, curr), prev)
                if operand_source == "rust_actual_f32_operands_and_state":
                    if leading_fma != r[5] or next_fma_fma != r[7]:
                        raise AssertionError(f"local FMA inputs do not reproduce Rust m={m} r={ring} d={degree}")
                fma_rows.append((
                    m, ring, degree, operand_source,
                    f"{leading_fma:08x}", f"{leading_sep:08x}",
                    format(f32d(leading_fma), ".12g"), format(f32d(leading_sep), ".12g"),
                    ulp(leading_fma, leading_sep),
                    f"{next_fma_fma:08x}", f"{next_sep_same_lead:08x}",
                    ulp(next_fma_fma, next_sep_same_lead),
                    f"{next_fma_sep_lead:08x}", f"{next_sep_sep:08x}",
                    ulp(next_fma_fma, next_fma_sep_lead),
                    ulp(next_fma_fma, next_sep_sep),
                ))
    write_csv_gz(dest / "controlled-f32-fma-local-effects.csv.gz", (
        "order", "ring", "degree", "common_operand_source", "leading_fma_bits",
        "leading_separate_bits", "leading_fma", "leading_separate", "leading_ulp_delta",
        "next_fma_fma_bits", "next_separate_same_leading_bits",
        "state_update_contraction_ulp_delta", "next_fma_separate_leading_bits",
        "next_separate_separate_bits", "leading_contraction_state_ulp_delta",
        "combined_contraction_ulp_delta",
    ), fma_rows)

    # Compact aggregate metrics, with the same per-cell control values preserved above.
    groups = {}
    for row in output_rows:
        key = "+".join((row[2], row[3], row[4]))
        groups.setdefault(key, []).append((D(row[9]), D(row[10])))
    aggregate_rows = []
    for path, vals in sorted(groups.items()):
        errors = [x[0] for x in vals]
        norms = [x[1] for x in vals]
        aggregate_rows.append((
            path, len(vals), format(max(errors), ".16g"),
            format((sum((e * e for e in errors), D(0)) / len(errors)).sqrt(), ".16g"),
            format(max(norms), ".12g"),
            format((sum((e * e for e in norms), D(0)) / len(norms)).sqrt(), ".12g"),
        ))
    write_csv(dest / "controlled-f32-output-summary.csv", (
        "path", "components", "max_absolute_error", "rms_absolute_error",
        "max_cancellation_scaled_error", "rms_cancellation_scaled_error",
    ), aggregate_rows)

    # Existing v2 controls are an independent implementation of the recurrence / pair /
    # cosine substitutions. Require the shared controls to reproduce their values.
    previous_controls = {}
    for parsed in csv.DictReader(read_gzip_lines(source / "dense-band-controlled-precision.csv.gz")):
        previous_controls[(int(parsed["order"]), int(parsed["ring"]), parsed["controlled_path"], parsed["component"])] = D(parsed["controlled_value"])
    control_map = (
        ("quad_recurrence_quad_pairs_quad_cos", "quad128_trace", "quad128_pairs", "quad128_cosine"),
        ("quad_recurrence_quad_pairs", "quad128_trace", "quad128_pairs", "rust_f32_cosine"),
        ("quad_recurrence_rust_pairs", "quad128_trace", "rust_f32_pairs", "rust_f32_cosine"),
        ("rust_recurrence_quad_pairs", "rust_native_trace", "quad128_pairs", "rust_f32_cosine"),
        ("rust_recurrence_rust_pairs", "rust_native_trace", "rust_f32_pairs", "rust_f32_cosine"),
    )
    current_controls = {
        (int(row[0]), int(row[1]), row[2], row[3], row[4], row[5]): D(row[6])
        for row in output_rows
    }
    control_checks = []
    for old_name, rec_name, pair_name, cos_name in control_map:
        errors = []
        for m, ring in cases:
            for component in ("north_re", "south_re", "north_im", "south_im"):
                old = previous_controls[(m, ring, old_name, component)]
                new = current_controls[(m, ring, rec_name, pair_name, cos_name, component)]
                errors.append(abs(new - old))
        maximum = max(errors)
        tolerance = D("2e-14") if old_name == "rust_recurrence_rust_pairs" else D("1e-30")
        if maximum > tolerance:
            raise AssertionError(f"previous control mismatch {old_name}: {maximum}")
        control_checks.append((old_name, len(errors), format(maximum, ".12g"), format(tolerance, ".1g"), "PASS"))
    write_csv(dest / "controlled-f32-previous-control-checks.csv", (
        "previous_path", "components", "maximum_absolute_difference", "tolerance", "result",
    ), control_checks)

    replay_updates = sum(x[2] for x in actual_replay_checks)
    # The published 22,768-row recurrence table also includes the terminal m=1,
    # degree-721 update once per ring. That state is outside lmax and has no
    # coefficient contribution; these replays count only updates through l=720.
    if replay_updates != 22752:
        raise AssertionError(f"expected 22752 contributing native f32 updates, got {replay_updates}")
    print(f"cases={len(cases)} native-f32-replay-updates={replay_updates} PASS")
    print(dest)


if __name__ == "__main__":
    main()
