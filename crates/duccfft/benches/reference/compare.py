#!/usr/bin/env python3
"""Compare deterministic DUCC and RustFFT 1D outputs."""

from __future__ import annotations

import argparse
import csv
import io
import math
import os
import subprocess
from pathlib import Path


def parse_records(output: str) -> dict[tuple[str, int, int], tuple[float, float]]:
    records: dict[tuple[str, int, int], tuple[float, float]] = {}
    for row in csv.reader(io.StringIO(output)):
        if not row or row[0].startswith("#") or row[0] == "mode":
            continue
        mode, n, index, real, imag = row
        records[(mode, int(n), int(index))] = (float(real), float(imag))
    return records


def compare_records(
    label: str,
    expected: dict[tuple[str, int, int], tuple[float, float]],
    actual: dict[tuple[str, int, int], tuple[float, float]],
    *,
    rtol: float,
    atol: float,
) -> None:
    if expected.keys() != actual.keys():
        missing_actual = sorted(expected.keys() - actual.keys())[:8]
        missing_expected = sorted(actual.keys() - expected.keys())[:8]
        raise SystemExit(
            f"{label} output shape mismatch: missing {missing_actual}; "
            f"unexpected {missing_expected}"
        )

    max_abs = 0.0
    max_scaled = 0.0
    worst_abs = None
    worst_scaled = None
    for key, expected_value in expected.items():
        actual_value = actual[key]
        for component, (got, want) in zip(
            ("real", "imag"), zip(actual_value, expected_value, strict=True), strict=True
        ):
            if not math.isfinite(got) or not math.isfinite(want):
                raise SystemExit(
                    f"{label} nonfinite value at {key} {component}: "
                    f"expected={want}, actual={got}"
                )
            error = abs(got - want)
            scaled = error / max(abs(got), abs(want), 1.0)
            if error > max_abs:
                max_abs = error
                worst_abs = (key, component)
            if scaled > max_scaled:
                max_scaled = scaled
                worst_scaled = (key, component)
            if error > atol + rtol * abs(want):
                raise SystemExit(
                    f"{label} mismatch at {key} {component}: "
                    f"expected={want}, actual={got}, abs_error={error:.3e}"
                )

    print(
        f"{label}: compared {len(expected)} output records "
        f"({2 * len(expected)} scalar components); max_abs={max_abs:.3e}; "
        f"worst_abs={worst_abs}; max_scaled={max_scaled:.3e}; "
        f"worst_scaled={worst_scaled}"
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--ducc",
        type=Path,
        default=Path("/tmp/duccfft-reference/duccfft-reference"),
        help="path to the binary built by reference/build.sh",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "Cargo.toml",
    )
    parser.add_argument("--rtol", type=float, default=2e-11)
    parser.add_argument("--atol", type=float, default=2e-11)
    args = parser.parse_args()

    ducc = subprocess.run(
        [str(args.ducc), "dump-all"], check=True, text=True, capture_output=True
    )
    rustfft = subprocess.run(
        [
            "cargo",
            "bench",
            "--manifest-path",
            str(args.manifest),
            "--bench",
            "rustfft",
            "--",
            "dump-all",
        ],
        check=True,
        text=True,
        capture_output=True,
    )
    native = subprocess.run(
        [
            "cargo",
            "bench",
            "--manifest-path",
            str(args.manifest),
            "--bench",
            "rustfft",
            "--",
            "dump-native-all",
        ],
        check=True,
        text=True,
        capture_output=True,
    )
    expected = parse_records(ducc.stdout)
    rustfft_output = parse_records(rustfft.stdout)
    native_output = parse_records(native.stdout)
    compare_records(
        "RustFFT",
        expected,
        rustfft_output,
        rtol=args.rtol,
        atol=args.atol,
    )
    compare_records(
        "DUCCFFT f64 C2C/R2C/C2R",
        expected,
        native_output,
        rtol=args.rtol,
        atol=args.atol,
    )
    compare_records(
        "DUCCFFT vs RustFFT f64",
        rustfft_output,
        native_output,
        rtol=args.rtol,
        atol=args.atol,
    )
    print(
        f"DUCC vectorize={'on' if os.environ.get('DUCCFFT_DUCC_VECTORIZE') == '1' else 'off'}"
    )


if __name__ == "__main__":
    main()
