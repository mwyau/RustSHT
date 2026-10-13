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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--ducc",
        type=Path,
        default=Path(__import__("os").environ.get("RUSTSHT_TMP_DIR", __import__("tempfile").gettempdir())) / 'duccfft-reference/duccfft-reference',
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
    left = parse_records(ducc.stdout)
    right = parse_records(rustfft.stdout)
    if left.keys() != right.keys():
        missing_rustfft = sorted(left.keys() - right.keys())[:8]
        missing_ducc = sorted(right.keys() - left.keys())[:8]
        raise SystemExit(
            f"output shape mismatch: RustFFT missing {missing_rustfft}; "
            f"DUCC missing {missing_ducc}"
        )

    max_abs = 0.0
    max_scaled = 0.0
    worst_abs = None
    worst_scaled = None
    for key, expected in left.items():
        actual = right[key]
        for component, (a, e) in zip(("real", "imag"), zip(actual, expected, strict=True), strict=True):
            if not math.isfinite(a) or not math.isfinite(e):
                raise SystemExit(
                    f"nonfinite value at {key} {component}: DUCC={e}, RustFFT={a}"
                )
            error = abs(a - e)
            scale = max(abs(a), abs(e), 1.0)
            scaled = error / scale
            if error > max_abs:
                max_abs = error
                worst_abs = (key, component)
            if scaled > max_scaled:
                max_scaled = scaled
                worst_scaled = (key, component)
            if error > args.atol + args.rtol * abs(e):
                raise SystemExit(
                    f"mismatch at {key} {component}: DUCC={expected}, RustFFT={actual}, "
                    f"abs_error={error:.3e}"
                )

    print(
        f"DUCC vectorize={'on' if os.environ.get('DUCCFFT_DUCC_VECTORIZE') == '1' else 'off'}; "
        f"compared {len(left)} output values; max_abs={max_abs:.3e}; "
        f"worst_abs={worst_abs}; max_scaled={max_scaled:.3e}; "
        f"worst_scaled={worst_scaled}"
    )


if __name__ == "__main__":
    main()
