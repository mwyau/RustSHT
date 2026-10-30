#!/usr/bin/env python3
"""Compare all GL T720 Rust analysis FFT spectra with NumPy pocketfft."""
import argparse
import hashlib
import json
from pathlib import Path

import numpy as np


NT = 721
NPHI = 1458
STORED_SPECTRUM_LEN = NPHI // 2 + 1
VALID_ORDER_COUNT = 721


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def comparison(expected, actual):
    diff = actual - expected
    magnitude = np.abs(expected)
    nonzero = magnitude > 0
    rel = np.zeros_like(magnitude)
    rel[nonzero] = np.abs(diff[nonzero]) / magnitude[nonzero]
    by_order = []
    for m in range(VALID_ORDER_COUNT):
        e = expected[:, m]
        d = diff[:, m]
        norm = np.linalg.norm(e)
        by_order.append(
            {
                "m": m,
                "relative_l2": float(np.linalg.norm(d) / norm) if norm else 0.0,
                "max_absolute": float(np.max(np.abs(d))),
            }
        )
    return {
        "relative_l2": float(np.linalg.norm(diff) / np.linalg.norm(expected)),
        "max_absolute": float(np.max(np.abs(diff))),
        "max_relative_nonzero": float(np.max(rel)),
        "worst_order_relative_l2": max(by_order, key=lambda item: item["relative_l2"]),
    }


def read_map(directory, name):
    path = directory / name
    return np.fromfile(path, dtype="<f4").reshape(NT, NPHI), path


def read_spectrum(directory, name, dtype):
    path = directory / name
    values = np.fromfile(path, dtype=dtype).reshape(NT, STORED_SPECTRUM_LEN) / NPHI
    return values[:, :VALID_ORDER_COUNT], path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--q1", type=Path, required=True)
    parser.add_argument("--q8", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--source-sha", default="dc1aa4f65109fc93fef8a0595998df464a70a639")
    args = parser.parse_args()

    outputs = {}
    fixture_hashes = {}
    bitwise = {}
    names = (
        "scalar-map.f32",
        "q-map.f32",
        "u-map.f32",
        "scalar-analysis-spectrum.f32",
        "scalar-analysis-spectrum.f64",
        "spin-analysis-q-spectrum.f32",
        "spin-analysis-q-spectrum.f64",
        "spin-analysis-u-spectrum.f32",
        "spin-analysis-u-spectrum.f64",
        "scalar-analysis.f32",
        "spin-analysis-e.f32",
        "spin-analysis-b.f32",
        "spin-synthesis-e-input.f32",
        "spin-synthesis-b-input.f32",
        "spin-synthesis-q.f32",
        "spin-synthesis-u.f32",
        "spin-synthesis-e-input.f64",
        "spin-synthesis-b-input.f64",
        "spin-synthesis-q.f64",
        "spin-synthesis-u.f64",
    )
    for name in names:
        q1_hash, q8_hash = sha(args.q1 / name), sha(args.q8 / name)
        fixture_hashes[name] = {"q1": q1_hash, "q8": q8_hash}
        bitwise[name] = q1_hash == q8_hash

    for name, stem in (
            ("scalar-map.f32", "scalar"),
            ("q-map.f32", "q"),
            ("u-map.f32", "u"),
        ):
        values, _ = read_map(args.q1, name)
        expected = (np.fft.rfft(values.astype(np.float64), axis=1) / NPHI)[:, :VALID_ORDER_COUNT]
        outputs[stem] = {}
        for precision, dtype in (("f32", "<c8"), ("f64", "<c16")):
            spectrum_name = {
                ("scalar", "f32"): "scalar-analysis-spectrum.f32",
                ("scalar", "f64"): "scalar-analysis-spectrum.f64",
                ("q", "f32"): "spin-analysis-q-spectrum.f32",
                ("q", "f64"): "spin-analysis-q-spectrum.f64",
                ("u", "f32"): "spin-analysis-u-spectrum.f32",
                ("u", "f64"): "spin-analysis-u-spectrum.f64",
            }[(stem, precision)]
            actual, _ = read_spectrum(args.q1, spectrum_name, dtype)
            outputs[stem][precision] = comparison(expected, actual)

    result = {
        "source_sha": args.source_sha,
        "grid": {"kind": "GL", "lmax": 720, "mmax": 720, "ntheta": NT, "nphi": NPHI},
        "reference": "NumPy 2.x pocketfft rfft in float64 of exact f32 map values widened to f64, normalized by nphi; compares valid harmonic orders m=0..720 and excludes stored bins 721..729 outside mmax",
        "numpy_version": np.__version__,
        "q1_q8_bitwise_equal": bitwise,
        "fixture_and_spectrum_sha256": fixture_hashes,
        "q1_fft_stage": outputs,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"out": str(args.out), "q1_q8_bitwise_equal": bitwise}, indent=2))


if __name__ == "__main__":
    main()
