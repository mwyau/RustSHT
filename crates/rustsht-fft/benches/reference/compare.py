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
        default=Path("/tmp/rustsht-fft-reference/rustsht-fft-reference"),
        help="path to the binary built by reference/build.sh",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "Cargo.toml",
    )
    parser.add_argument("--rtol", type=float, default=2e-11)
    parser.add_argument("--atol", type=float, default=2e-11)
    parser.add_argument(
        "--external",
        action="append",
        default=[],
        metavar="LABEL=PATH",
        help="compare output with an external_fft.cpp binary exposing dump MODE N",
    )
    parser.add_argument(
        "--lengths",
        type=int,
        nargs="+",
        help="compare f64 C2C/R2C/C2R results for these lengths",
    )
    args = parser.parse_args()

    if args.lengths:
        modes = ("c2c_forward", "c2c_inverse", "r2c", "c2r")
        expected_parts = []
        for n in args.lengths:
            for mode in modes:
                expected_parts.append(
                    subprocess.run(
                        [str(args.ducc), "dump", mode, str(n)],
                        check=True,
                        text=True,
                        capture_output=True,
                    ).stdout
                )
        actual_output = subprocess.run(
            [
                "cargo",
                "bench",
                "--manifest-path",
                str(args.manifest),
                "--bench",
                "rustfft",
                "--",
                "dump-native-lengths",
                *(str(n) for n in args.lengths),
            ],
            check=True,
            text=True,
            capture_output=True,
        )
        compare_records(
            "RustSHT-FFT f64 C2C/R2C/C2R",
            parse_records("".join(expected_parts)),
            parse_records(actual_output.stdout),
            rtol=args.rtol,
            atol=args.atol,
        )
        native_records = parse_records(actual_output.stdout)
        for specification in args.external:
            label, separator, executable = specification.partition("=")
            if not separator or not label or not executable:
                raise SystemExit("--external must use LABEL=PATH")
            external_parts = []
            for n in args.lengths:
                for mode in modes:
                    external_parts.append(
                        subprocess.run(
                            [executable, "dump", mode, str(n)],
                            check=True,
                            text=True,
                            capture_output=True,
                        ).stdout
                    )
            compare_records(
                f"RustSHT-FFT versus {label} f64 C2C/R2C/C2R",
                native_records,
                parse_records("".join(external_parts)),
                rtol=args.rtol,
                atol=args.atol,
            )
        print(
            f"DUCC vectorize={'on' if os.environ.get('RUSTSHT_FFT_DUCC_VECTORIZE') == '1' else 'off'}"
        )
        return

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
        "RustSHT-FFT f64 C2C/R2C/C2R",
        expected,
        native_output,
        rtol=args.rtol,
        atol=args.atol,
    )
    compare_records(
        "RustSHT-FFT vs RustFFT f64",
        rustfft_output,
        native_output,
        rtol=args.rtol,
        atol=args.atol,
    )
    print(
        f"DUCC vectorize={'on' if os.environ.get('RUSTSHT_FFT_DUCC_VECTORIZE') == '1' else 'off'}"
    )


if __name__ == "__main__":
    main()
