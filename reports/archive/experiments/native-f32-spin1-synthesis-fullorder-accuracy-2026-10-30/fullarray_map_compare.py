#!/usr/bin/env python3
"""Compare full GL T720 B2 spectra/maps to binary128-derived spectra."""
import argparse
import hashlib
import json
from pathlib import Path

import numpy as np


NPHI = 1458
SPECTRUM_LEN = NPHI // 2 + 1
MMAX = 720


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_map(directory, name, dtype, nt):
    path = directory / name
    values = np.fromfile(path, dtype=dtype)
    if values.size != nt * NPHI:
        raise ValueError(f"unexpected map size: {path}")
    return values.reshape(nt, NPHI)


def read_spectrum(directory, name, dtype, nt):
    path = directory / name
    values = np.fromfile(path, dtype=dtype)
    if values.size != nt * SPECTRUM_LEN:
        raise ValueError(f"unexpected spectrum size: {path}")
    return values.reshape(nt, SPECTRUM_LEN)


def norm_error(actual, reference):
    actual = actual.astype(np.complex128)
    valid = (
        np.isfinite(actual.real)
        & np.isfinite(actual.imag)
        & np.isfinite(reference.real)
        & np.isfinite(reference.imag)
    )
    diff = actual[valid] - reference[valid]
    ref_norm = np.linalg.norm(reference[valid])
    abs_error = np.abs(diff)
    return {
        "relative_l2": float(np.linalg.norm(diff) / ref_norm) if ref_norm else 0.0,
        "max_absolute": float(abs_error.max(initial=0.0)),
        "valid_count": int(valid.sum()),
        "excluded_nonfinite_count": int(valid.size - valid.sum()),
    }


def inverse_spectrum(spectrum, nt):
    full = np.zeros((nt, SPECTRUM_LEN), dtype=np.complex128)
    full[:, : MMAX + 1] = spectrum
    return np.fft.irfft(full * NPHI, n=NPHI, axis=1)


def map_metrics(actual, reference, envelope):
    error = actual.astype(np.float64) - reference
    absolute = np.abs(error)
    scale = np.maximum(envelope[:, None], np.abs(reference))
    valid = np.isfinite(actual) & np.isfinite(reference) & np.isfinite(scale)
    scaled = np.divide(
        absolute,
        scale,
        out=np.zeros_like(absolute),
        where=valid & (scale > 0),
    )
    ref_norm = np.linalg.norm(reference[valid])
    finite_absolute = np.where(valid, absolute, -np.inf)
    max_abs_index = int(np.argmax(finite_absolute))
    max_scaled_index = int(np.argmax(np.where(valid, scaled, -np.inf)))
    return {
        "relative_l2": float(np.linalg.norm(error[valid]) / ref_norm) if ref_norm else 0.0,
        "max_absolute": float(absolute.reshape(-1)[max_abs_index]) if valid.any() else 0.0,
        "max_absolute_ring_longitude": [
            int(x) for x in np.unravel_index(max_abs_index, absolute.shape)
        ],
        "max_envelope_scaled": float(scaled.reshape(-1)[max_scaled_index]) if valid.any() else 0.0,
        "max_envelope_scaled_ring_longitude": [
            int(x) for x in np.unravel_index(max_scaled_index, scaled.shape)
        ],
        "valid_count": int(valid.sum()),
        "excluded_nonfinite_count": int(valid.size - valid.sum()),
    }


def by_order(actual, reference, component, nt):
    result = []
    for m in range(MMAX + 1):
        a = actual[:, m].astype(np.complex128)
        r = reference[:, m]
        valid = np.isfinite(a.real) & np.isfinite(a.imag) & np.isfinite(r.real) & np.isfinite(r.imag)
        diff = np.abs(a[valid] - r[valid])
        valid_r = r[valid]
        scaled = np.divide(
            diff,
            np.abs(valid_r),
            out=np.zeros_like(diff),
            where=np.abs(valid_r) > 0,
        )
        ref_norm = np.linalg.norm(valid_r)
        ring_ids = np.flatnonzero(valid)
        result.append(
            {
                "m": m,
                "component": component,
                "relative_l2": float(np.linalg.norm(diff) / ref_norm) if ref_norm else 0.0,
                "max_absolute": float(diff.max(initial=0.0)),
                "max_absolute_ring": int(ring_ids[np.argmax(diff)]) if diff.size else -1,
                "max_reference_scaled": float(scaled.max(initial=0.0)),
                "max_reference_scaled_ring": int(ring_ids[np.argmax(scaled)]) if scaled.size else -1,
                "valid_count": int(valid.sum()),
                "excluded_nonfinite_count": int(valid.size - valid.sum()),
            }
        )
    return result


def analyze(directory, q8_directory):
    results = {}
    hashes = {}
    nt = sum(1 for _ in (directory / "geometry.csv").open()) - 1
    nt_q8 = sum(1 for _ in (q8_directory / "geometry.csv").open()) - 1
    if nt != nt_q8 or sha(directory / "geometry.csv") != sha(q8_directory / "geometry.csv"):
        raise ValueError("q1/q8 geometry mismatch")
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
    equal_by_precision = {
        precision: all(
            hashes[name]["q1"] == hashes[name]["q8"]
            for name in equal_files
            if name.endswith(f".{precision}")
        )
        for precision in ("f32", "f64")
    }

    for component in ("q", "u"):
        reference_path = directory / f"spin-synthesis-{component}-reference.c128"
        reference = np.fromfile(reference_path, dtype="<c16").reshape(nt, MMAX + 1)
        spectrum32 = read_spectrum(directory, f"spin-synthesis-{component}-spectrum.f32", "<c8", nt)
        spectrum64 = read_spectrum(directory, f"spin-synthesis-{component}-spectrum.f64", "<c16", nt)
        map32 = read_map(directory, f"spin-synthesis-{component}.f32", "<f4", nt)
        map64 = read_map(directory, f"spin-synthesis-{component}.f64", "<f8", nt)

        reference_map = inverse_spectrum(reference, nt)
        f32_spectrum_map = inverse_spectrum(spectrum32[:, : MMAX + 1].astype(np.complex128), nt)
        f64_spectrum_map = inverse_spectrum(spectrum64[:, : MMAX + 1], nt)
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
            for row in by_order(spectrum32, reference, component, nt):
                out.write(
                    f"{row['m']},{row['relative_l2']:.17g},{row['max_absolute']:.17g},"
                    f"{row['max_absolute_ring']},{row['max_reference_scaled']:.17g},"
                    f"{row['max_reference_scaled_ring']}\n"
                )

    return {
        "source_sha": "dc1aa4f65109fc93fef8a0595998df464a70a639",
        "grid": {"kind": (directory / "metadata.txt").read_text().split("grid_kind=")[1].splitlines()[0], "lmax": 720, "mmax": MMAX, "ntheta": nt, "nphi": NPHI},
        "reference": "GCC __float128 spin-1-to-scalar channel conversion and normalized scalar-harmonic recurrence; expected spectra rounded once to binary64, then NumPy 2.x pocketfft inverse real FFT",
        "q1_q8_hashes": hashes,
        "q1_q8_bitwise_equal_by_precision": equal_by_precision,
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
