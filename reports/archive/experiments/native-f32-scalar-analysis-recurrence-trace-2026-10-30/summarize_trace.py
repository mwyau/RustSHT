#!/usr/bin/env python3
"""Summarize the native-f32 paired scalar-analysis kernel trace."""

from collections import defaultdict
from fractions import Fraction
import json
import struct
import sys


def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits))[0]


def bits32(value):
    return struct.unpack("<I", struct.pack("<f", value))[0]


def round_ratio_even(numerator, denominator):
    quotient, remainder = divmod(numerator, denominator)
    twice = remainder * 2
    if twice > denominator or (twice == denominator and quotient & 1):
        quotient += 1
    return quotient


def round_fraction_to_f32(value):
    if not value:
        return 0
    sign = 0
    if value < 0:
        sign = 0x80000000
        value = -value
    numerator, denominator = value.numerator, value.denominator
    exponent = numerator.bit_length() - denominator.bit_length()
    if exponent >= 0:
        if numerator < (denominator << exponent):
            exponent -= 1
    elif (numerator << -exponent) < denominator:
        exponent -= 1

    if exponent < -126:
        significand = round_ratio_even(numerator << 149, denominator)
        if significand == 0:
            return sign
        if significand >= (1 << 23):
            return sign | (1 << 23)
        return sign | significand

    shift = 23 - exponent
    if shift >= 0:
        significand = round_ratio_even(numerator << shift, denominator)
    else:
        significand = round_ratio_even(numerator, denominator << -shift)
    if significand == (1 << 24):
        significand >>= 1
        exponent += 1
    if exponent > 127:
        return sign | 0x7F800000
    return sign | ((exponent + 127) << 23) | (significand - (1 << 23))


def ordered(bits):
    return (~bits & 0xFFFFFFFF) if bits & 0x80000000 else (bits | 0x80000000)


def ulp_distance(left, right):
    return abs(ordered(left) - ordered(right))


def as_fraction(bits):
    return Fraction.from_float(f32(bits))


def read_rows(path):
    with open(path, encoding="ascii") as source:
        header = source.readline().strip().split(",")
        for line in source:
            values = line.strip().split(",")
            row = dict(zip(header, values))
            for name in header:
                if name.endswith("_bits"):
                    row[name] = int(row[name], 16)
            row["m"] = int(row["m"])
            row["ring"] = int(row["ring"])
            row["pair"] = int(row["pair"])
            row["degree"] = int(row["degree"])
            row["exponent"] = int(row["exponent"])
            yield row


def summarize(rows):
    grouped = defaultdict(list)
    for row in rows:
        grouped[(row["m"], row["ring"])].append(row)

    result = []
    for (m, ring), group in sorted(grouped.items()):
        group.sort(key=lambda row: row["pair"])
        hp_previous = Fraction(0)
        hp_current = as_fraction(group[0]["current_bits"])
        first_rounded_recurrence_divergence = None
        first_fma_vs_separate_difference = None
        lead_correct_count = 0
        next_correct_count = 0
        fma_lead_diff_count = 0
        fma_next_diff_count = 0
        maximum_abs_error = (Fraction(0), None)
        maximum_scaled_error = (Fraction(0), None)
        selected = {}

        for row in group:
            pair = row["pair"]
            degree = row["degree"]
            current_bits = row["current_bits"]
            actual_current = as_fraction(current_bits)
            if pair:
                expected_bits = round_fraction_to_f32(hp_current)
                ulps = ulp_distance(current_bits, expected_bits)
                if ulps and first_rounded_recurrence_divergence is None:
                    first_rounded_recurrence_divergence = {
                        "degree": degree,
                        "actual_bits": f"{current_bits:08x}",
                        "rounded_exact_state_bits": f"{expected_bits:08x}",
                        "ulp_distance": ulps,
                    }
                abs_error = abs(actual_current - hp_current)
                if abs_error > maximum_abs_error[0]:
                    maximum_abs_error = (abs_error, degree)
                local_scale = max(abs(hp_previous), abs(hp_current), Fraction(1, 1 << 126))
                scaled_error = abs_error / local_scale
                if scaled_error > maximum_scaled_error[0]:
                    maximum_scaled_error = (scaled_error, degree)
                if degree in [600, 601, 650, 651, 700, 701, 719, 720]:
                    selected[str(degree)] = {
                        "actual": f32(current_bits),
                        "actual_bits": f"{current_bits:08x}",
                        "exact_state": float(hp_current),
                        "exact_state_round_f32_bits": f"{expected_bits:08x}",
                        "ulp_distance": ulps,
                        "absolute_error": float(abs_error),
                        "relative_error": float(abs_error / abs(hp_current)) if hp_current else None,
                        "state_scaled_error": float(scaled_error),
                    }

            if pair + 1 < len(group):
                a = as_fraction(row["a_bits"])
                b = as_fraction(row["b_bits"])
                cos2 = as_fraction(row["cos2_bits"])
                previous = as_fraction(row["previous_bits"])
                current = as_fraction(row["current_bits"])
                leading_bits = row["leading_fma_bits"]
                next_bits = row["next_fma_bits"]
                exact_leading = a * cos2 + b
                expected_leading_bits = round_fraction_to_f32(exact_leading)
                exact_next_from_f32_leading = (
                    as_fraction(leading_bits) * current + previous
                )
                expected_next_bits = round_fraction_to_f32(exact_next_from_f32_leading)
                lead_ok = leading_bits == expected_leading_bits
                next_ok = row["actual_next_bits"] == expected_next_bits
                lead_correct_count += lead_ok
                next_correct_count += next_ok
                if not lead_ok and first_rounded_recurrence_divergence is None:
                    first_rounded_recurrence_divergence = {
                        "degree": degree,
                        "operation": "leading fma",
                        "actual_bits": f"{leading_bits:08x}",
                        "correctly_rounded_bits": f"{expected_leading_bits:08x}",
                    }
                if not next_ok and first_rounded_recurrence_divergence is None:
                    first_rounded_recurrence_divergence = {
                        "degree": degree + 2,
                        "operation": "state fma",
                        "actual_bits": f"{row['actual_next_bits']:08x}",
                        "correctly_rounded_bits": f"{expected_next_bits:08x}",
                    }
                if leading_bits != row["leading_separate_bits"]:
                    fma_lead_diff_count += 1
                    if first_fma_vs_separate_difference is None:
                        first_fma_vs_separate_difference = {
                            "degree": degree,
                            "operation": "leading factor",
                            "fma_bits": f"{leading_bits:08x}",
                            "separate_bits": f"{row['leading_separate_bits']:08x}",
                            "ulp_distance": ulp_distance(leading_bits, row["leading_separate_bits"]),
                        }
                if next_bits != row["next_separate_bits"]:
                    fma_next_diff_count += 1
                    if first_fma_vs_separate_difference is None:
                        first_fma_vs_separate_difference = {
                            "degree": degree,
                            "operation": "state update",
                            "fma_bits": f"{next_bits:08x}",
                            "separate_bits": f"{row['next_separate_bits']:08x}",
                            "ulp_distance": ulp_distance(next_bits, row["next_separate_bits"]),
                        }

                # Exact recurrence with the actual f32 operands and no
                # intermediate rounding. This carries a high-precision state.
                hp_leading = exact_leading
                hp_next = hp_leading * hp_current + hp_previous
                hp_previous, hp_current = hp_current, hp_next

        result.append(
            {
                "m": m,
                "ring": ring,
                "rows": len(group),
                "exponent": group[0]["exponent"],
                "fma_first_stage_correctly_rounded_count": lead_correct_count,
                "fma_second_stage_correctly_rounded_count": next_correct_count,
                "fma_leading_differs_from_separate_count": fma_lead_diff_count,
                "fma_state_differs_from_separate_count": fma_next_diff_count,
                "first_fma_vs_separate_difference": first_fma_vs_separate_difference,
                "first_rounded_recurrence_divergence": first_rounded_recurrence_divergence,
                "max_abs_state_error": {
                    "value": float(maximum_abs_error[0]),
                    "degree": maximum_abs_error[1],
                },
                "max_state_scaled_error": {
                    "value": float(maximum_scaled_error[0]),
                    "degree": maximum_scaled_error[1],
                },
                "selected_degrees": selected,
            }
        )
    return result


if __name__ == "__main__":
    print(json.dumps(summarize(read_rows(sys.argv[1])), indent=2, sort_keys=True))
