#!/usr/bin/env python3
"""Compare M6 f32 bridge dumps with DUCC 0.41.0 float and double transforms."""

from __future__ import annotations

import json
import statistics
import sys
import time
from pathlib import Path

import ducc0
import numpy as np
from ducc0 import sht

DUCC_REVISION = "c1ed01b8fda69d2a378190f0915f1a19762dddbf"
PHI0 = 0.37


def packed_index(lmax: int, l: int, m: int) -> int:
    return m * (2 * lmax + 1 - m) // 2 + l


def coefficients(lmax: int, spin: int) -> np.ndarray:
    count = 1 if spin == 0 else 2
    result = np.zeros(
        (count, (lmax + 1) * (lmax + 2) // 2), dtype=np.complex64
    )
    for m in range(lmax + 1):
        for l in range(max(m, spin), lmax + 1):
            index = packed_index(lmax, l, m)
            for channel in range(count):
                if channel == 0:
                    real = (l * 7 + m * 11) % 19 - 9
                    imag = 0 if m == 0 else (l * 13 + m * 5) % 23 - 11
                else:
                    real = (l * 3 + m * 7) % 17 - 8
                    imag = 0 if m == 0 else (l * 5 + m * 17) % 29 - 14
                result[channel, index] = np.complex64(
                    complex(np.float32(real) * np.float32(1.0e-5),
                            np.float32(imag) * np.float32(1.0e-5))
                )
    return result


def maps(lmax: int, nphi: int, spin: int) -> np.ndarray:
    count = 1 if spin == 0 else 2
    result = np.empty((count, lmax + 1, nphi), dtype=np.float32)
    for ring in range(lmax + 1):
        for longitude in range(nphi):
            result[0, ring, longitude] = np.float32(
                np.float32((ring * 31 + longitude * 17) % 101 - 50)
                / np.float32(101.0)
            )
            if spin == 1:
                result[1, ring, longitude] = np.float32(
                    np.float32((ring * 19 + longitude * 23 + 7) % 97 - 48)
                    / np.float32(97.0)
                )
    return result


def compare(actual: np.ndarray, reference: np.ndarray) -> dict[str, float | int]:
    actual_wide = actual.astype(
        np.complex128 if np.iscomplexobj(actual) else np.float64
    )
    reference_wide = reference.astype(
        np.complex128 if np.iscomplexobj(reference) else np.float64
    )
    difference = actual_wide - reference_wide
    magnitude = np.abs(difference)
    return {
        "elements": int(actual.size),
        "relative_l2": float(
            np.sqrt(np.vdot(difference, difference).real)
            / np.sqrt(np.vdot(reference_wide, reference_wide).real)
        ),
        "rms_absolute": float(np.sqrt(np.mean(magnitude * magnitude))),
        "max_absolute": float(np.max(magnitude)),
    }


def timed(operation) -> tuple[np.ndarray, list[float]]:
    operation()
    samples: list[float] = []
    result = None
    for _ in range(5):
        start = time.perf_counter()
        result = operation()
        samples.append((time.perf_counter() - start) * 1000.0)
    assert result is not None
    return result, samples


def main() -> None:
    if len(sys.argv) not in (2, 3):
        raise SystemExit(
            f"usage: {Path(sys.argv[0]).name} DUMP_DIRECTORY [THREADS]"
        )
    dump_dir = Path(sys.argv[1])
    threads_filter = int(sys.argv[2]) if len(sys.argv) == 3 else None
    files = sorted(dump_dir.glob("[lt]*-s*-q*-*-*"))
    results: dict[str, object] = {
        "ducc0": ducc0.__version__,
        "ducc_source_revision": DUCC_REVISION,
        "comparison": {},
    }
    cells = sorted(
        {
            (int(parts[0][1:]), int(parts[1][1:]), int(parts[2][1:]))
            for parts in (path.name.split("-")[:3] for path in files)
            if threads_filter is None or int(parts[2][1:]) == threads_filter
        }
    )
    for lmax, spin, threads in cells:
        ntheta = lmax + 1
        nphi = 2 * ntheta
        alm32 = coefficients(lmax, spin)
        map32 = maps(lmax, nphi, spin)
        alm64 = alm32.astype(np.complex128)
        map64 = map32.astype(np.float64)
        reference32_synthesis, ducc32_synthesis_ms = timed(
            lambda: sht.synthesis_2d(
                alm=alm32,
                spin=spin,
                lmax=lmax,
                mmax=lmax,
                geometry="GL",
                ntheta=ntheta,
                nphi=nphi,
                phi0=PHI0,
                nthreads=threads,
            )
        )
        reference32_analysis, ducc32_analysis_ms = timed(
            lambda: sht.analysis_2d(
                map=map32,
                spin=spin,
                lmax=lmax,
                mmax=lmax,
                geometry="GL",
                phi0=PHI0,
                nthreads=threads,
            )
        )
        reference64_synthesis, ducc64_synthesis_ms = timed(
            lambda: sht.synthesis_2d(
                alm=alm64,
                spin=spin,
                lmax=lmax,
                mmax=lmax,
                geometry="GL",
                ntheta=ntheta,
                nphi=nphi,
                phi0=PHI0,
                nthreads=threads,
            )
        )
        reference64_analysis, ducc64_analysis_ms = timed(
            lambda: sht.analysis_2d(
                map=map64,
                spin=spin,
                lmax=lmax,
                mmax=lmax,
                geometry="GL",
                phi0=PHI0,
                nthreads=threads,
            )
        )
        reference32 = {
            "synthesis": reference32_synthesis,
            "analysis": reference32_analysis,
        }
        reference64 = {
            "synthesis": reference64_synthesis,
            "analysis": reference64_analysis,
        }
        cell: dict[str, object] = {
            "geometry": "GL",
            "ntheta": ntheta,
            "nphi": nphi,
            "phi0": PHI0,
            "threads": threads,
            "precision": "f32/complex64 inputs and outputs; f64/complex128 bridge",
            "operations": {},
            "ducc_timing": {
                "synthesis": {
                    "f32_samples_ms": ducc32_synthesis_ms,
                    "f32_median_ms": statistics.median(ducc32_synthesis_ms),
                    "f64_samples_ms": ducc64_synthesis_ms,
                    "f64_median_ms": statistics.median(ducc64_synthesis_ms),
                },
                "analysis": {
                    "f32_samples_ms": ducc32_analysis_ms,
                    "f32_median_ms": statistics.median(ducc32_analysis_ms),
                    "f64_samples_ms": ducc64_analysis_ms,
                    "f64_median_ms": statistics.median(ducc64_analysis_ms),
                },
            },
        }
        for operation in ("synthesis", "analysis"):
            suffix = "f32" if operation == "synthesis" else "c32"
            wide_suffix = "f64" if operation == "synthesis" else "c128"
            channels = 1 if spin == 0 else 2
            legacy_prefix = f"l{lmax}-s{spin}-q{threads}"
            reusable_prefix = f"t{lmax}-s{spin}-q{threads}-{operation}"
            reusable_native_path = dump_dir / f"{reusable_prefix}-native_f32-0.{suffix}"
            if reusable_native_path.exists():
                variant_paths = {
                    "native_f32": [
                        dump_dir / f"{reusable_prefix}-native_f32-{channel}.{suffix}"
                        for channel in range(channels)
                    ],
                    "allocating_bridge": [
                        dump_dir / f"{reusable_prefix}-allocating_bridge-{channel}.{suffix}"
                        for channel in range(channels)
                    ],
                    "reusable_bridge": [
                        dump_dir / f"{reusable_prefix}-reusable_bridge-{channel}.{suffix}"
                        for channel in range(channels)
                    ],
                    "native_f64": [
                        dump_dir
                        / f"{reusable_prefix}-native_f64_rounded_inputs-{channel}.{wide_suffix}"
                        for channel in range(channels)
                    ],
                }
            else:
                legacy_native_path = (
                    dump_dir / f"{legacy_prefix}-native-{operation}-0.{suffix}"
                )
                if not legacy_native_path.exists():
                    continue
                variant_paths = {
                    "native_f32": [
                        dump_dir
                        / f"{legacy_prefix}-native-{operation}-{channel}.{suffix}"
                        for channel in range(channels)
                    ],
                    "allocating_bridge": [
                        dump_dir
                        / f"{legacy_prefix}-bridge-{operation}-{channel}.{suffix}"
                        for channel in range(channels)
                    ],
                    "native_f64": [
                        dump_dir / f"{legacy_prefix}-f64-{operation}-{channel}.{wide_suffix}"
                        for channel in range(channels)
                    ],
                }

            expected_shape = (
                (ntheta, nphi)
                if operation == "synthesis"
                else ((lmax + 1) * (lmax + 2) // 2,)
            )
            actual_values = {}
            for variant, paths in variant_paths.items():
                value_suffix = wide_suffix if variant == "native_f64" else suffix
                dtype = (
                    "<f4"
                    if value_suffix == "f32"
                    else "<c8"
                    if value_suffix == "c32"
                    else "<f8"
                    if value_suffix == "f64"
                    else "<c16"
                )
                actual_values[variant] = [
                    np.fromfile(path, dtype=dtype).reshape(expected_shape)
                    for path in paths
                ]

            operation_comparisons = []
            for channel in range(channels):
                record = {
                    "channel": channel,
                    "native_f32_vs_ducc_f32": compare(
                        actual_values["native_f32"][channel], reference32[operation][channel]
                    ),
                    "native_f64_vs_ducc_f64": compare(
                        actual_values["native_f64"][channel], reference64[operation][channel]
                    ),
                }
                for variant in ("allocating_bridge", "reusable_bridge"):
                    if variant in actual_values:
                        record[f"{variant}_f32_vs_ducc_f32"] = compare(
                            actual_values[variant][channel], reference32[operation][channel]
                        )
                        record[f"{variant}_f32_vs_ducc_f64_cast_to_f32"] = compare(
                            actual_values[variant][channel],
                            reference64[operation][channel].astype(
                                np.complex64
                                if np.iscomplexobj(reference64[operation][channel])
                                else np.float32
                            ),
                        )
                        record[f"{variant}_f32_vs_native_f32"] = compare(
                            actual_values[variant][channel], actual_values["native_f32"][channel]
                        )
                primary_bridge = (
                    "reusable_bridge"
                    if "reusable_bridge" in actual_values
                    else "allocating_bridge"
                )
                record["bridge_f32_vs_ducc_f32"] = record[
                    f"{primary_bridge}_f32_vs_ducc_f32"
                ]
                record["bridge_f32_vs_ducc_f64_cast_to_f32"] = record[
                    f"{primary_bridge}_f32_vs_ducc_f64_cast_to_f32"
                ]
                record["bridge_f32_vs_native_f32"] = record[
                    f"{primary_bridge}_f32_vs_native_f32"
                ]
                operation_comparisons.append(record)
            cell["operations"][operation] = operation_comparisons
        results["comparison"][f"T{lmax}-s{spin}-q{threads}"] = cell
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
