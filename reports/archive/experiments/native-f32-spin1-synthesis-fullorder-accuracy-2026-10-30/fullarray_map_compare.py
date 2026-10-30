#!/usr/bin/env python3
"""Compare full GL T720 B2 spectra/maps to binary128-derived spectra."""
import argparse
import hashlib
import json
from pathlib import Path

import numpy as np


NT = 721
NPHI = 1458
SPECTRUM_LEN = NPHI // 2 + 1
MMAX = 720


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_map(directory, name, dtype):
    path = directory / name
    values = np.fromfile(path, dtype=dtype)
    if values.size != NT * NPHI:
        raise ValueError(f"unexpected map size: {path}")
    return values.reshape(NT, NPHI)


def read_spectrum(directory, name, dtype):
    path = directory / name
    values = np.fromfile(path, dtype=dtype)
    if values.size != NT * SPECTRUM_LEN:
        raise ValueError(f"unexpected spectrum size: {path}")
    return values.reshape(NT, SPECTRUM_LEN)


def norm_error(actual, reference):
    diff = actual.astype(np.complex128) - reference
    ref_norm = np.linalg.norm(reference)
    abs_error = np.abs(diff)
    return {
        "relative_l2": float(np.linalg.norm(diff) / ref_norm) if ref_norm else 0.0,
        "max_absolute": float(abs_error.max(initial=0.0)),
    }


def inverse_spectrum(spectrum):
    full = np.zeros((NT, SPECTRUM_LEN), dtype=np.complex128)
    full[:, : MMAX + 1] = spectrum
    return np.fft.irfft(full * NPHI, n=NPHI, axis=1)


def map_metrics(actual, reference, envelope):
    error = actual.astype(np.float64) - reference
    absolute = np.abs(error)
    scale = np.maximum(envelope[:, None], np.abs(reference))
    scaled = np.divide(absolute, scale, out=np.zeros_like(absolute), where=scale > 0)
    ref_norm = np.linalg.norm(reference)
    return {
        "relative_l2": float(np.linalg.norm(error) / ref_norm) if ref_norm else 0.0,
        "max_absolute": float(absolute.max(initial=0.0)),
        "max_absolute_ring_longitude": [
            int(x) for x in np.unravel_index(np.argmax(absolute), absolute.shape)
        ],
        "max_envelope_scaled": float(scaled.max(initial=0.0)),
        "max_envelope_scaled_ring_longitude": [
            int(x) for x in np.unravel_index(np.argmax(scaled), scaled.shape)
        ],
    }


def by_order(actual, reference, component):
    result = []
    for m in range(MMAX + 1):
        a = actual[:, m].astype(np.complex128)
        r = reference[:, m]
        diff = np.abs(a - r)
        scaled = np.divide(diff, np.abs(r), out=np.zeros_like(diff), where=np.abs(r) > 0)
        ref_norm = np.linalg.norm(r)
        result.append(
            {
                "m": m,
                "component": component,
                "relative_l2": float(np.linalg.norm(a - r) / ref_norm) if ref_norm else 0.0,
                "max_absolute": float(diff.max(initial=0.0)),
                "max_absolute_ring": int(np.argmax(diff)),
                "max_reference_scaled": float(scaled.max(initial=0.0)),
                "max_reference_scaled_ring": int(np.argmax(scaled)),
            }
        )
    return result


def analyze(directory, q8_directory):
    results = {}
    hashes = {}
    equal_files = (
        "geometry.csv",
        "spin-synthesis-e-input.f32",
        "spin-synthesis-b-input.f32",
        "spin-synthesis-e-input.f64",
        "spin-synthesis-b-input.f64",
        "spin-synthesis-q.f32",
        "spin-synthesis-u.f32",
        "spin-synthesis-q.f64",
        "spin-synthesis-u.f64",
        "spin-synthesis-q-spectrum.f32",
        "spin-synthesis-u-spectrum.f32",
        "spin-synthesis-q-spectrum.f64",
        "spin-synthesis-u-spectrum.f64",
    )
    for name in equal_files:
        hashes[name] = {
            "q1": sha(directory / name),
            "q8": sha(q8_directory / name),
        }

    for component in ("q", "u"):
        reference_path = directory / f"spin-synthesis-{component}-reference.c128"
        reference = np.fromfile(reference_path, dtype="<c16").reshape(NT, MMAX + 1)
        spectrum32 = read_spectrum(directory, f"spin-synthesis-{component}-spectrum.f32", "<c8")
        spectrum64 = read_spectrum(directory, f"spin-synthesis-{component}-spectrum.f64", "<c16")
        map32 = read_map(directory, f"spin-synthesis-{component}.f32", "<f4")
        map64 = read_map(directory, f"spin-synthesis-{component}.f64", "<f8")

        reference_map = inverse_spectrum(reference)
        f32_spectrum_map = inverse_spectrum(spectrum32[:, : MMAX + 1].astype(np.complex128))
        f64_spectrum_map = inverse_spectrum(spectrum64[:, : MMAX + 1])
        envelope = np.abs(reference[:, 0].real) + 2 * np.abs(reference[:, 1:]).sum(axis=1)
        envelope32 = np.abs(spectrum32[:, 0].real) + 2 * np.abs(
            spectrum32[:, 1 : MMAX + 1]
        ).sum(axis=1)
        results[component] = {
            "reference_spectrum_sha256": sha(reference_path),
            "f32_spectrum_vs_binary128": norm_error(spectrum32[:, : MMAX + 1], reference),
            "f64_spectrum_vs_binary128": norm_error(spectrum64[:, : MMAX + 1], reference),
            "f32_vs_f64_spectrum": norm_error(
                spectrum32[:, : MMAX + 1], spectrum64[:, : MMAX + 1]
            ),
            "f32_inverse_fft_residual": map_metrics(map32, f32_spectrum_map, envelope32),
            "f64_inverse_fft_residual": map_metrics(map64, f64_spectrum_map, envelope),
            "f32_map_vs_binary128_reference": map_metrics(map32, reference_map, envelope),
            "f64_map_vs_binary128_reference": map_metrics(map64, reference_map, envelope),
        }
        with (directory / f"spin-synthesis-{component}-spectrum-by-order.csv").open("w") as out:
            out.write(
                "m,relative_l2,max_absolute,max_absolute_ring,max_reference_scaled,"
                "max_reference_scaled_ring\n"
            )
            for row in by_order(spectrum32, reference, component):
                out.write(
                    f"{row['m']},{row['relative_l2']:.17g},{row['max_absolute']:.17g},"
                    f"{row['max_absolute_ring']},{row['max_reference_scaled']:.17g},"
                    f"{row['max_reference_scaled_ring']}\n"
                )

    return {
        "source_sha": "dc1aa4f65109fc93fef8a0595998df464a70a639",
        "grid": {"kind": "GaussLegendre", "lmax": 720, "mmax": MMAX, "ntheta": NT, "nphi": NPHI},
        "reference": "GCC __float128 spin-1-to-scalar channel conversion and normalized scalar-harmonic recurrence; expected spectra rounded once to binary64, then NumPy 2.x pocketfft inverse real FFT",
        "q1_q8_hashes": hashes,
        "q1_q8_bitwise_equal": all(
            value["q1"] == value["q8"] for value in hashes.values()
        ),
        "components": results,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--q1", type=Path, required=True)
    parser.add_argument("--q8", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    result = analyze(args.q1, args.q8)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"out": str(args.out), "q1_q8_bitwise_equal": result["q1_q8_bitwise_equal"]}, indent=2))


if __name__ == "__main__":
    main()
