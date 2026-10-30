#!/usr/bin/env python3
"""Exact-rational replay and one-input-at-a-time sensitivity for B1 traces."""

from collections import defaultdict
from fractions import Fraction
import json
import struct
import sys


def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits))[0]


def f64(bits):
    return struct.unpack("<d", struct.pack("<Q", bits))[0]


def bits32(value):
    return struct.unpack("<I", struct.pack("<f", value))[0]


def round_ratio_even(numerator, denominator):
    quotient, remainder = divmod(numerator, denominator)
    twice = 2 * remainder
    if twice > denominator or (twice == denominator and quotient & 1):
        quotient += 1
    return quotient


def round_fraction_to_f32(value):
    if value == 0:
        return 0
    sign = 0
    if value < 0:
        sign, value = 0x80000000, -value
    numerator, denominator = value.numerator, value.denominator
    exponent = numerator.bit_length() - denominator.bit_length()
    if exponent >= 0:
        if numerator < denominator << exponent:
            exponent -= 1
    elif numerator << -exponent < denominator:
        exponent -= 1
    if exponent < -126:
        significand = round_ratio_even(numerator << 149, denominator)
        if significand == 0:
            return sign
        return sign | ((1 << 23) if significand >= 1 << 23 else significand)
    shift = 23 - exponent
    significand = (
        round_ratio_even(numerator << shift, denominator)
        if shift >= 0
        else round_ratio_even(numerator, denominator << -shift)
    )
    if significand == 1 << 24:
        significand >>= 1
        exponent += 1
    if exponent > 127:
        return sign | 0x7F800000
    return sign | ((exponent + 127) << 23) | (significand - (1 << 23))


def frac32(bits):
    return Fraction.from_float(f32(bits))


def frac64(bits):
    return Fraction.from_float(f64(bits))


def bits64(value):
    return struct.unpack("<Q", struct.pack("<d", value))[0]


def ordered(bits, width):
    sign = 1 << (width - 1)
    mask = (1 << width) - 1
    return (~bits & mask) if bits & sign else (bits | sign)


def ulp_distance(left, right, width=32):
    return abs(ordered(left, width) - ordered(right, width))


def ulp_size32(bits):
    exponent = (bits >> 23) & 0xFF
    if exponent == 0:
        return Fraction(1, 1 << 149)
    return Fraction(1 << max(exponent - 127 - 23, 0), 1) if exponent >= 150 else Fraction(1, 1 << (150 - exponent))


def read_trace(path):
    rows = defaultdict(list)
    with open(path, encoding="ascii") as source:
        header = source.readline().strip().split(",")
        for line in source:
            values = line.strip().split(",")
            row = dict(zip(header, values))
            for name in header:
                if name.endswith("_bits"):
                    row[name] = int(row[name], 16)
            for name in ("m", "ring", "pair", "degree", "exponent"):
                row[name] = int(row[name])
            rows[(row["m"], row["ring"])].append(row)
    for group in rows.values():
        group.sort(key=lambda row: row["pair"])
    return rows


def exact_replay(group, seed, a_name, b_name, cos2_name, a_convert, b_convert, cos2_convert):
    previous = Fraction(0)
    current = seed
    states = []
    for row in group:
        states.append(current)
        if row is group[-1]:
            break
        a = a_convert(row[a_name])
        b = b_convert(row[b_name])
        cos2 = cos2_convert(row[cos2_name])
        following = (a * cos2 + b) * current + previous
        previous, current = current, following
    return states


def state_error(group, states, actual_name, rounding, width):
    max_abs = (Fraction(0), None)
    max_rel = (Fraction(0), None)
    max_scaled = (Fraction(0), None)
    max_ulp = (0, None)
    first_ulp = None
    selected = {}
    previous = Fraction(0)
    targets = {600, 601, 650, 651, 700, 701, 719, 720}
    for row, exact in zip(group, states):
        actual_bits = row[actual_name]
        actual = frac32(actual_bits) if width == 32 else frac64(actual_bits)
        expected_bits = rounding(exact)
        ulps = ulp_distance(actual_bits, expected_bits, width)
        degree = row["degree"]
        if degree != group[0]["degree"] and ulps and first_ulp is None:
            first_ulp = {"degree": degree, "ulp_distance": ulps}
        if ulps > max_ulp[0]:
            max_ulp = (ulps, degree)
        absolute = abs(actual - exact)
        if absolute > max_abs[0]:
            max_abs = (absolute, degree)
        if exact:
            relative = absolute / abs(exact)
            if relative > max_rel[0]:
                max_rel = (relative, degree)
        local_scale = max(abs(previous), abs(exact), Fraction(1, 1 << (126 if width == 32 else 1022)))
        scaled = absolute / local_scale
        if scaled > max_scaled[0]:
            max_scaled = (scaled, degree)
        if degree in targets:
            selected[str(degree)] = {
                "actual_bits": f"{actual_bits:0{width // 4}x}",
                "rounded_exact_bits": f"{expected_bits:0{width // 4}x}",
                "ulp_distance": ulps,
                "absolute_error": float(absolute),
                "relative_error": float(absolute / abs(exact)) if exact else None,
                "state_scaled_error": float(scaled),
            }
        previous = exact
    return {
        "first_state_ulp_divergence": first_ulp,
        "max_ulp_distance": {"value": max_ulp[0], "degree": max_ulp[1]},
        "max_absolute_error": {"value": float(max_abs[0]), "degree": max_abs[1]},
        "max_relative_error": {"value": float(max_rel[0]), "degree": max_rel[1]},
        "max_state_scaled_error": {"value": float(max_scaled[0]), "degree": max_scaled[1]},
        "selected_degrees": selected,
    }


def round64(value):
    return bits64(float(value))


def summarize_local_rounding(group):
    leading_count = 0
    next_count = 0
    separate_leading_count = 0
    separate_next_count = 0
    first_leading_rounding = None
    first_next_rounding = None
    first_fma_vs_separate = None
    worst_leading_condition = (Fraction(0), None)
    worst_update_condition = (Fraction(0), None)
    max_leading_roundoff_ulp = (Fraction(0), None)
    max_update_roundoff_ulp = (Fraction(0), None)

    for row in group[:-1]:
        a, b, cos2 = map(frac32, (row["a_bits"], row["b_bits"], row["cos2_bits"]))
        previous, current = map(frac32, (row["previous_bits"], row["current_bits"]))
        term_a = a * cos2
        exact_leading = term_a + b
        actual_leading = row["leading_fma_bits"]
        expected_leading = round_fraction_to_f32(exact_leading)
        lead_ok = actual_leading == expected_leading
        leading_count += lead_ok
        if not lead_ok and first_leading_rounding is None:
            first_leading_rounding = {"degree": row["degree"], "actual": f"{actual_leading:08x}", "expected": f"{expected_leading:08x}"}
        lead_error_ulp = abs(exact_leading - frac32(actual_leading)) / ulp_size32(actual_leading)
        if lead_error_ulp > max_leading_roundoff_ulp[0]:
            max_leading_roundoff_ulp = (lead_error_ulp, row["degree"])
        lead_denominator = abs(term_a) + abs(b)
        if exact_leading:
            condition = lead_denominator / abs(exact_leading)
            if condition > worst_leading_condition[0]:
                worst_leading_condition = (condition, row["degree"])
        else:
            worst_leading_condition = (Fraction(10**100), row["degree"])

        exact_next_from_rounded_leading = frac32(actual_leading) * current + previous
        actual_next = row["actual_next_bits"]
        expected_next = round_fraction_to_f32(exact_next_from_rounded_leading)
        next_ok = actual_next == expected_next
        next_count += next_ok
        if not next_ok and first_next_rounding is None:
            first_next_rounding = {"degree": row["degree"] + 2, "actual": f"{actual_next:08x}", "expected": f"{expected_next:08x}"}
        update_error_ulp = abs(exact_next_from_rounded_leading - frac32(actual_next)) / ulp_size32(actual_next)
        if update_error_ulp > max_update_roundoff_ulp[0]:
            max_update_roundoff_ulp = (update_error_ulp, row["degree"] + 2)
        update_denominator = abs(frac32(actual_leading) * current) + abs(previous)
        if exact_next_from_rounded_leading:
            condition = update_denominator / abs(exact_next_from_rounded_leading)
            if condition > worst_update_condition[0]:
                worst_update_condition = (condition, row["degree"] + 2)
        else:
            worst_update_condition = (Fraction(10**100), row["degree"] + 2)

        if actual_leading != row["leading_separate_bits"]:
            separate_leading_count += 1
            if first_fma_vs_separate is None:
                first_fma_vs_separate = {
                    "degree": row["degree"],
                    "operation": "a*cos2+b",
                    "fma_bits": f"{actual_leading:08x}",
                    "separate_bits": f"{row['leading_separate_bits']:08x}",
                    "ulp_distance": ulp_distance(actual_leading, row["leading_separate_bits"]),
                }
        if row["next_fma_bits"] != row["next_separate_bits"]:
            separate_next_count += 1
            if first_fma_vs_separate is None:
                first_fma_vs_separate = {
                    "degree": row["degree"],
                    "operation": "leading*current+previous",
                    "fma_bits": f"{row['next_fma_bits']:08x}",
                    "separate_bits": f"{row['next_separate_bits']:08x}",
                    "ulp_distance": ulp_distance(row["next_fma_bits"], row["next_separate_bits"]),
                }

    return {
        "leading_fma_correctly_rounded_steps": leading_count,
        "state_fma_correctly_rounded_steps": next_count,
        "fma_leading_differs_from_separate_steps": separate_leading_count,
        "fma_state_differs_from_separate_steps": separate_next_count,
        "first_fma_vs_separate_difference": first_fma_vs_separate,
        "first_leading_fma_rounding_mismatch": first_leading_rounding,
        "first_state_fma_rounding_mismatch": first_next_rounding,
        "max_leading_fma_roundoff_ulp": {"value": float(max_leading_roundoff_ulp[0]), "degree": max_leading_roundoff_ulp[1]},
        "max_state_fma_roundoff_ulp": {"value": float(max_update_roundoff_ulp[0]), "degree": max_update_roundoff_ulp[1]},
        "worst_leading_cancellation_condition": {"condition_number": float(worst_leading_condition[0]), "degree": worst_leading_condition[1]},
        "worst_state_update_cancellation_condition": {"condition_number": float(worst_update_condition[0]), "degree": worst_update_condition[1]},
    }


def replay_sensitivity(group, seed, a_field, b_field, cos2_field, a_convert, b_convert, cos2_convert):
    return exact_replay(
        group, seed, a_field, b_field, cos2_field, a_convert, b_convert, cos2_convert
    )


def summarize(path):
    output = []
    groups = read_trace(path)
    for (m, ring), group in sorted(groups.items()):
        seed32 = frac32(group[0]["current_bits"])
        seed64 = frac64(group[0]["current64_bits"])
        f32_replay = exact_replay(
            group, seed32, "a_bits", "b_bits", "cos2_bits", frac32, frac32, frac32
        )
        f64_replay = exact_replay(
            group, seed64, "a64_bits", "b64_bits", "cos2_64_bits", frac64, frac64, frac64
        )
        f32_error = state_error(group, f32_replay, "current_bits", round_fraction_to_f32, 32)
        f64_error = state_error(group, f64_replay, "current64_bits", round64, 64)

        variants = {
            "f32_prepared_seed_factors_geometry": f32_replay,
            "f64_seed_only": replay_sensitivity(
                group, seed64, "a_bits", "b_bits", "cos2_bits", frac32, frac32, frac32
            ),
            "f64_factors_only": replay_sensitivity(
                group, seed32, "a64_bits", "b64_bits", "cos2_bits", frac64, frac64, frac32
            ),
            "f64_cos2_only": replay_sensitivity(
                group, seed32, "a_bits", "b_bits", "cos2_64_bits", frac32, frac32, frac64
            ),
            "all_f64_prepared": f64_replay,
        }
        final_index = len(group) - 1
        base_final = variants["f32_prepared_seed_factors_geometry"][final_index]
        base_states = variants["f32_prepared_seed_factors_geometry"]
        base_envelope = max(abs(value) for value in base_states)
        sensitivity = {}
        for name, states in variants.items():
            final = states[final_index]
            delta = final - base_final
            max_delta, max_delta_degree = max(
                ((abs(candidate - reference), row["degree"]) for candidate, reference, row in zip(states, base_states, group)),
                key=lambda pair: pair[0],
            )
            sensitivity[name] = {
                "final_degree": group[-1]["degree"],
                "final_value": float(final),
                "final_absolute_delta_from_f32_prepared_replay": float(abs(delta)),
                "final_delta_over_final_local_state_scale": float(
                    abs(delta) / max(abs(base_final), abs(base_states[-2]), Fraction(1, 1 << 126))
                ),
                "max_trajectory_absolute_delta_from_f32_replay": float(max_delta),
                "max_trajectory_delta_over_f32_envelope": float(max_delta / base_envelope) if base_envelope else None,
                "max_trajectory_delta_degree": max_delta_degree,
            }

        output.append(
            {
                "m": m,
                "ring": ring,
                "degrees": [group[0]["degree"], group[-1]["degree"]],
                "f32_simd_vs_exact_same_f32_operands": f32_error,
                "f64_simd_vs_exact_same_f64_operands": f64_error,
                "local_f32_rounding_and_conditioning": summarize_local_rounding(group),
                "one_change_at_a_time_preparation_sensitivity": sensitivity,
            }
        )
    return output


if __name__ == "__main__":
    print(json.dumps(summarize(sys.argv[1]), indent=2, sort_keys=True))
