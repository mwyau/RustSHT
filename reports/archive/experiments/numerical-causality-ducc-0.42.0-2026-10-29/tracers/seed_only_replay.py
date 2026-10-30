#!/usr/bin/env python3
"""Seed-only replay with the archived native-f32 recurrence operands."""
from __future__ import annotations

import csv
import ctypes
import gzip
import hashlib
import io
import json
import math
import platform
import re
import sys
from fractions import Fraction
from decimal import Decimal
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PACKAGE / 'tracers'))
import controlled_f32_replay as replay  # noqa: E402

D = Decimal
LIBM = ctypes.CDLL('libm.so.6')
SQRTF = LIBM.sqrtf
SQRTF.argtypes = (ctypes.c_float,)
SQRTF.restype = ctypes.c_float

def replay_seed_with_traced_operands(rustrows, m: int, seed_bits: int):
    """Replay fmaf state updates without requiring the replacement to match trace."""
    states = {m: replay.f32d(seed_bits)}
    previous = 0
    current = seed_bits
    for degree in sorted(d for d in rustrows if m + 2 <= d <= 720):
        row = rustrows[degree]
        csq, a, b = row[0], row[1], row[2]
        leading = replay.fma32(a, csq, b)
        if leading != row[5]:
            raise AssertionError(f"traced leading factor mismatch m={m}, degree={degree}")
        following = replay.fma32(leading, current, previous)
        states[degree] = replay.f32d(following)
        previous, current = current, following
    return states

def f32(value: float) -> float:
    import struct
    return struct.unpack('<f', struct.pack('<f', value))[0]

def round_fraction_to_f32_bits(value: Fraction) -> int:
    """Correctly round an exact rational to IEEE binary32, ties to even."""
    sign = 0
    if value < 0:
        sign = 0x8000_0000
        value = -value
    if value == 0:
        return sign

    numerator, denominator = value.numerator, value.denominator
    exponent = numerator.bit_length() - denominator.bit_length()
    if exponent >= 0:
        if numerator < (denominator << exponent):
            exponent -= 1
    elif (numerator << -exponent) < denominator:
        exponent -= 1

    def nearest_even(quotient: int, remainder: int, divisor: int) -> int:
        twice = remainder << 1
        return quotient + int(twice > divisor or (twice == divisor and quotient & 1 != 0))

    if exponent >= -126:
        shift = 23 - exponent
        if shift >= 0:
            scaled_numerator, scaled_denominator = numerator << shift, denominator
        else:
            scaled_numerator, scaled_denominator = numerator, denominator << -shift
        quotient, remainder = divmod(scaled_numerator, scaled_denominator)
        significand = nearest_even(quotient, remainder, scaled_denominator)
        if significand == 1 << 24:
            significand >>= 1
            exponent += 1
        if exponent > 127:
            return sign | 0x7f80_0000
        return sign | ((exponent + 127) << 23) | (significand - (1 << 23))

    quotient, remainder = divmod(numerator << 149, denominator)
    significand = nearest_even(quotient, remainder, denominator)
    if significand >= 1 << 23:
        return sign | 0x0080_0000
    return sign | significand

def exact_product_seed_bits(sine_bits: int, norm_bits: int, order: int) -> int:
    sine = Fraction.from_float(replay.f32(sine_bits))
    norm = Fraction.from_float(replay.f32(norm_bits))
    sign = -1 if order & 1 else 1
    return round_fraction_to_f32_bits(sign * (sine ** order) * norm)

def f32_sqrt(value: float) -> float:
    return float(SQRTF(ctypes.c_float(value)))

def rust_f32_sectoral_norms(mmax: int) -> list[float]:
    one = f32(1.0)
    two = f32(one + one)
    four = f32(two + two)
    pi = f32(math.pi)
    root = f32_sqrt(f32(four * pi))
    norms = [f32(one / root)]
    for m in range(1, mmax + 1):
        mf = f32(float(m))
        numerator = f32(f32(two * mf) + one)
        denominator = f32(two * mf)
        ratio = f32(numerator / denominator)
        norms.append(f32(norms[-1] * f32_sqrt(ratio)))
    return norms

def validate_seed_inputs(norms, rust_rec, rust_outputs):
    plan_rows = 0
    actual_seed_rows = 0
    sparse_trace = PACKAGE / 'data' / 'rust-native-f32.csv.gz'
    with gzip.open(sparse_trace, 'rt') as stream:
        for line in stream:
            if not line.startswith('INIT,'):
                continue
            fields = line.rstrip().split(',')
            m, ring = int(fields[1]), int(fields[2])
            norm_bits = int(re.search(r'norm=([0-9a-f]+)', line).group(1), 16)
            restored_bits = int(re.search(r'restored=([0-9a-f]+)', line).group(1), 16)
            if norm_bits != replay.f32bits(norms[m]):
                raise AssertionError(f'plan normalization mismatch m={m}, ring={ring}')
            plan_rows += 1
            key = (m, ring)
            if m in range(1, 17) and ring in (0, 1, 5, 360) and key in rust_rec:
                actual_bits = rust_rec[key][m + 2][4]
                if restored_bits != actual_bits:
                    raise AssertionError(f'actual seed trace mismatch m={m}, ring={ring}')
                actual_seed_rows += 1
    if plan_rows != 48 or actual_seed_rows != 40:
        raise AssertionError(f'unexpected input validation counts: {plan_rows}, {actual_seed_rows}')
    sparse_seed_rows = 0
    sparse_path = PACKAGE / 'data' / 'first-divergence-selected.csv.gz'
    with gzip.open(sparse_path, 'rt', newline='') as stream:
        for row in csv.DictReader(stream):
            expected = int(row['rust_exact_f32_input_seed_bits'], 16)
            calculated = exact_product_seed_bits(
                int(row['rust_sin_bits'], 16),
                int(row['rust_mfac_bits'], 16),
                int(row['m']),
            )
            if calculated != expected:
                raise AssertionError(
                    f"exact seed formula disagrees with archived sparse row "
                    f"m={row['m']}, l={row['l']}, ring={row['ring']}"
                )
            sparse_seed_rows += 1
    if sparse_seed_rows != 357:
        raise AssertionError(f'unexpected sparse exact-seed validation count: {sparse_seed_rows}')
    return plan_rows, actual_seed_rows, sparse_seed_rows


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b''):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    source = PACKAGE / 'data' / 'dense-band'
    out_dir = Path(sys.argv[1]) if len(sys.argv) == 2 else source
    out_dir.mkdir(parents=True, exist_ok=True)
    if len(sys.argv) > 2:
        raise SystemExit('usage: seed_only_replay.py [output-directory]')
    rust_rec, rust_pairs_bits, rust_outputs = replay.load_rust(source / 'rust-f32-trace.csv.gz')
    quad_rec, quad_pairs, quad_outputs = replay.load_quad(source / 'quad128-reference.csv.gz')
    norms = rust_f32_sectoral_norms(16)
    plan_validation_rows, actual_seed_validation_rows, sparse_seed_validation_rows = validate_seed_inputs(
        norms, rust_rec, rust_outputs
    )
    f32_pairs = {
        key: tuple(replay.f32d(bits) for bits in values)
        for key, values in rust_pairs_bits.items()
    }
    rows = []
    seed_rows = []
    recurrence_rows = []
    for m in range(1, 17):
        for ring in (0, 1, 5, 360):
            key = (m, ring)
            rrows = rust_rec[key]
            qrows = quad_rec[key]
            sine_bits = rust_outputs[key]['sin']
            norm_bits = replay.f32bits(norms[m])
            corrected_seed_bits = exact_product_seed_bits(sine_bits, norm_bits, m)
            actual_seed_bits = rrows[m + 2][4]
            quad_seed_bits = qrows[m + 2]['bits'][1]
            actual_states, actual_ops = replay.rust_operands_replay(rrows, m, actual_seed_bits)
            traced_states = replay.state_from_trace(qrows, m, rustrows=rrows)
            if actual_states != traced_states:
                raise AssertionError(f'actual native f32 replay mismatch m={m} ring={ring}')
            corrected_states = replay_seed_with_traced_operands(
                rrows, m, corrected_seed_bits
            )
            reference = replay.state_from_trace(qrows, m)
            if corrected_seed_bits != actual_seed_bits:
                first_diff = min(
                    degree for degree in sorted(reference)
                    if actual_states.get(degree) != corrected_states.get(degree)
                )
            else:
                first_diff = None
            for degree in sorted(reference):
                actual = actual_states[degree]
                corrected = corrected_states[degree]
                target = reference[degree]
                recurrence_rows.append({
                    'm': m, 'ring': ring, 'degree': degree,
                    'actual_f32_state': format(actual, '.40g'),
                    'corrected_seed_f32_state': format(corrected, '.40g'),
                    'binary128_state': format(target, '.40g'),
                    'actual_abs_error': format(abs(actual - target), '.17g'),
                    'corrected_abs_error': format(abs(corrected - target), '.17g'),
                    'state_delta_from_seed': format(corrected - actual, '.17g'),
                })
            cos = replay.f32d(rust_outputs[key]['cos'])
            actual_out = replay.accumulate(actual_states, f32_pairs, m, cos)
            corrected_out = replay.accumulate(corrected_states, f32_pairs, m, cos)
            qout = quad_outputs[key]['decimal']
            ref_out = {
                'north_re': qout['nr'], 'north_im': qout['ni'],
                'south_re': qout['sr'],
                'south_im': qout['eveni'] - qout['c'] * qout['oddi'],
            }
            envelope = {
                'north_re': abs(qout['evenr']) + abs(qout['c'] * qout['oddr']),
                'south_re': abs(qout['evenr']) + abs(qout['c'] * qout['oddr']),
                'north_im': abs(qout['eveni']) + abs(qout['c'] * qout['oddi']),
                'south_im': abs(qout['eveni']) + abs(qout['c'] * qout['oddi']),
            }
            for component in ('north_re', 'north_im', 'south_re', 'south_im'):
                target = ref_out[component]
                a = actual_out[component]
                c = corrected_out[component]
                scale = max(abs(target), envelope[component], D('1e-100'))
                rows.append({
                    'm': m, 'ring': ring, 'component': component,
                    'actual_seed_bits': f'{actual_seed_bits:08x}',
                    'corrected_seed_bits': f'{corrected_seed_bits:08x}',
                    'binary128_seed_bits': f'{quad_seed_bits:08x}',
                    'rust_sin_bits': f'{sine_bits:08x}',
                    'rust_sectoral_norm_bits': f'{replay.f32bits(norms[m]):08x}',
                    'first_state_divergence_degree': '' if first_diff is None else first_diff,
                    'actual_prefft_value_high_precision_sum': format(a, '.40g'),
                    'corrected_prefft_value_high_precision_sum': format(c, '.40g'),
                    'binary128_prefft_reference': format(target, '.40g'),
                    'actual_abs_error': format(abs(a-target), '.20g'),
                    'corrected_abs_error': format(abs(c-target), '.20g'),
                    'absolute_output_delta_from_seed': format(c-a, '.20g'),
                    'actual_envelope_scaled_error': format(abs(a-target)/scale, '.20g'),
                    'corrected_envelope_scaled_error': format(abs(c-target)/scale, '.20g'),
                    'ulp_seed_actual_to_corrected': replay.ulp(actual_seed_bits, corrected_seed_bits),
                })
            seed_rows.append({
                'm': m, 'ring': ring,
                'actual_seed_bits': f'{actual_seed_bits:08x}',
                'corrected_seed_bits': f'{corrected_seed_bits:08x}',
                'binary128_seed_rounded_f32_bits': f'{quad_seed_bits:08x}',
                'corrected_seed_value': format(replay.f32d(corrected_seed_bits), '.40g'),
                'actual_seed_value': format(replay.f32d(actual_seed_bits), '.40g'),
                'corrected_seed_abs_error_vs_bq': format(abs(replay.f32d(corrected_seed_bits)-qrows[m+2]['decimal'][1]), '.20g'),
                'actual_seed_abs_error_vs_bq': format(abs(replay.f32d(actual_seed_bits)-qrows[m+2]['decimal'][1]), '.20g'),
                'seed_ulp_actual_to_corrected': replay.ulp(actual_seed_bits, corrected_seed_bits),
                'first_state_divergence_degree': '' if first_diff is None else first_diff,
                'last_common_state_degree': max(set(corrected_states) & set(actual_states)),
                'last_common_state_delta': format(
                    corrected_states[max(set(corrected_states) & set(actual_states))]
                    - actual_states[max(set(corrected_states) & set(actual_states))],
                    '.20g',
                ),
            })

    def write_csv_gz(path: Path, records: list[dict]) -> None:
        with path.open('wb') as raw_stream:
            with gzip.GzipFile(fileobj=raw_stream, mode='wb', filename='', mtime=0, compresslevel=9) as compressed:
                with io.TextIOWrapper(compressed, newline='') as stream:
                    writer = csv.DictWriter(stream, fieldnames=list(records[0]))
                    writer.writeheader()
                    writer.writerows(records)

    write_csv_gz(out_dir / 'seed-only-prefft-comparison.csv.gz', rows)
    write_csv_gz(out_dir / 'seed-only-seed-and-state-summary.csv.gz', seed_rows)
    write_csv_gz(out_dir / 'seed-only-recurrence-growth.csv.gz', recurrence_rows)

    summary = {}
    for component in ('north_re','north_im','south_re','south_im'):
        group=[r for r in rows if r['component']==component]
        for key in ('actual_abs_error','corrected_abs_error'):
            vals=[D(r[key]) for r in group]
            summary.setdefault(component,{})[key]={
                'max':format(max(vals),'.20g'),
                'rms':format((sum(x*x for x in vals)/len(vals)).sqrt(),'.20g')
            }
        for key in ('actual_envelope_scaled_error','corrected_envelope_scaled_error'):
            vals=[D(r[key]) for r in group]
            summary.setdefault(component,{})[key]={
                'max':format(max(vals),'.20g'),
                'rms':format((sum(x*x for x in vals)/len(vals)).sqrt(),'.20g')
            }
        a=[D(r['actual_abs_error']) for r in group]
        c=[D(r['corrected_abs_error']) for r in group]
        summary.setdefault(component,{})['per_component_outcomes']={
            'improved':sum(v<u for v,u in zip(c,a)),
            'worsened':sum(v>u for v,u in zip(c,a)),
            'unchanged':sum(v==u for v,u in zip(c,a)),
        }
    for label, key in [('actual','actual_abs_error'),('corrected','corrected_abs_error')]:
        vals=[D(r[key]) for r in rows]
        refs=[abs(D(r['binary128_prefft_reference'])) for r in rows]
        summary.setdefault('all_256_outputs',{})[label]={
            'relative_l2':format((sum(v*v for v in vals)/sum(v*v for v in refs)).sqrt(),'.20g'),
            'max_abs':format(max(vals),'.20g'),
            'rms_abs':format((sum(v*v for v in vals)/len(vals)).sqrt(),'.20g'),
        }
    summary['fixture']={
        'source_trace_commit':'1d1986faf9444438879b34904b6538aeecfeb456',
        'candidate_source_commit':'45380d7e4bd849a33ffef7d91978ee18fd7e4193',
        'ducc_reference_commit':'42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b',
        'orders':'m=1..16','rings':'0,1,5,360','outputs':len(rows),
        'seed_correction':'correctly round the exact rational product of the actual f32 sine and f32 sectoral normalization, including (-1)^m; then replay the unchanged native-f32 recurrence operands and FMA state updates; keep Rust f32 coefficient pairs/cosine and accumulate/reconstruct in 100-digit Decimal',
        'scope':'pre-FFT Fourier coefficients; diagnostic-only; no production source changes',
        'replayed_recurrence_updates':len(recurrence_rows) - len(seed_rows),
        'sectoral_norm_plan_rows_validated':plan_validation_rows,
        'actual_seed_rows_validated_against_sparse_trace':actual_seed_validation_rows,
        'correctly_rounded_seed_formula_rows_validated_against_archived_sparse_table':sparse_seed_validation_rows,
        'candidate_trace_sha256':sha256(source / 'rust-f32-trace.csv.gz'),
        'binary128_trace_sha256':sha256(source / 'quad128-reference.csv.gz'),
        'sparse_validation_trace_sha256':sha256(PACKAGE / 'data' / 'rust-native-f32.csv.gz'),
        'python':platform.python_version(),
    }
    summary['seed_change_counts'] = {
        'cases': len(seed_rows),
        'changed_from_actual': sum(row['actual_seed_bits'] != row['corrected_seed_bits'] for row in seed_rows),
        'equal_to_binary128_rounded_f32': sum(
            row['corrected_seed_bits'] == row['binary128_seed_rounded_f32_bits']
            for row in seed_rows
        ),
        'max_ulp_actual_to_corrected': max(
            int(row['seed_ulp_actual_to_corrected']) for row in seed_rows
        ),
    }
    summary['all_256_outputs']['paired_component_outcomes'] = {
        label: sum(int(summary[component]['per_component_outcomes'][label])
                   for component in ('north_re', 'north_im', 'south_re', 'south_im'))
        for label in ('improved', 'worsened', 'unchanged')
    }
    actual_all = summary['all_256_outputs']['actual']
    corrected_all = summary['all_256_outputs']['corrected']
    summary['all_256_outputs']['relative_changes_percent'] = {
        key: format(
            (D(corrected_all[key]) / D(actual_all[key]) - 1) * 100,
            '.12g',
        )
        for key in ('relative_l2', 'max_abs', 'rms_abs')
    }
    m8_ring1 = next(
        row for row in rows
        if row['m'] == 8 and row['ring'] == 1 and row['component'] == 'north_re'
    )
    m8_actual, m8_corrected = D(m8_ring1['actual_abs_error']), D(m8_ring1['corrected_abs_error'])
    summary['selected_m8_ring1_north_real'] = {
        'actual_seed_bits': m8_ring1['actual_seed_bits'],
        'corrected_seed_bits': m8_ring1['corrected_seed_bits'],
        'binary128_rounded_seed_bits': m8_ring1['binary128_seed_bits'],
        'actual_absolute_prefft_error': format(m8_actual, '.20g'),
        'corrected_absolute_prefft_error': format(m8_corrected, '.20g'),
        'relative_error_reduction_percent': format((D(1) - m8_corrected / m8_actual) * 100, '.12g'),
    }
    (out_dir/'seed-only-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(summary['all_256_outputs'],indent=2))

if __name__=='__main__':
    import csv
    main()
