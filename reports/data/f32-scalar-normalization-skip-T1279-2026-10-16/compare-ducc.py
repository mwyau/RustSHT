#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
"""Compare full T1279 scalar f32 outputs with pinned DUCC 0.41.0."""

import hashlib
import json
from pathlib import Path

import ducc0
import numpy as np
from ducc0 import sht


ROOT = Path(__file__).parent
LMAX = MMAX = 1279
NTHETA = 1280
NPHI = 2560
PHI0 = 0.37
REVISION = "c1ed01b8fda69d2a378190f0915f1a19762dddbf"


def packed_index(l: int, m: int) -> int:
    return m * (2 * LMAX + 1 - m) // 2 + l


def coefficients() -> np.ndarray:
    alm = np.zeros((1, (LMAX + 1) * (LMAX + 2) // 2), dtype=np.complex64)
    for m in range(LMAX + 1):
        for l in range(m, LMAX + 1):
            real = ((l * 7 + m * 11) % 19) - 9
            imag = 0 if m == 0 else ((l * 13 + m * 5) % 23) - 11
            alm[0, packed_index(l, m)] = np.complex64(
                complex(
                    np.float32(real) * np.float32(1.0e-5),
                    np.float32(imag) * np.float32(1.0e-5),
                )
            )
    return alm


def compare(actual: np.ndarray, expected: np.ndarray) -> dict[str, float | int]:
    actual_wide = actual.astype(np.complex128 if np.iscomplexobj(actual) else np.float64)
    expected_wide = expected.astype(
        np.complex128 if np.iscomplexobj(expected) else np.float64
    )
    difference = actual_wide - expected_wide
    magnitude = np.abs(difference)
    return {
        "elements": int(actual.size),
        "relative_l2": float(
            np.sqrt(np.vdot(difference, difference).real)
            / np.sqrt(np.vdot(expected_wide, expected_wide).real)
        ),
        "rms_absolute": float(np.sqrt(np.mean(magnitude * magnitude))),
        "max_absolute": float(np.max(magnitude)),
    }


def main() -> None:
    baseline_map_path = ROOT / "output-baseline/spin0-synthesis-channel0.f32"
    candidate_map_path = ROOT / "output-candidate/spin0-synthesis-channel0.f32"
    candidate_alm_path = ROOT / "output-candidate/spin0-analysis-channel0.c32"
    baseline_map = np.fromfile(baseline_map_path, dtype="<f4").reshape(NTHETA, NPHI)
    candidate_map = np.fromfile(candidate_map_path, dtype="<f4").reshape(NTHETA, NPHI)
    candidate_alm = np.fromfile(candidate_alm_path, dtype="<c8")
    reference_map = sht.synthesis_2d(
        alm=coefficients(),
        spin=0,
        lmax=LMAX,
        mmax=MMAX,
        geometry="GL",
        ntheta=NTHETA,
        nphi=NPHI,
        phi0=PHI0,
        nthreads=1,
    )[0]
    reference_alm = sht.analysis_2d(
        map=baseline_map[None, :, :],
        spin=0,
        lmax=LMAX,
        mmax=MMAX,
        geometry="GL",
        phi0=PHI0,
        nthreads=1,
    )[0]
    output = {
        "ducc0": ducc0.__version__,
        "ducc_source_revision": REVISION,
        "grid": {
            "geometry": "GL",
            "lmax": LMAX,
            "mmax": MMAX,
            "ntheta": NTHETA,
            "nphi": NPHI,
            "phi0": PHI0,
            "dtype": "float32 / complex64",
            "nthreads": 1,
        },
        "analysis_input_sha256": hashlib.sha256(baseline_map_path.read_bytes()).hexdigest(),
        "candidate_vs_ducc": {
            "synthesis": compare(candidate_map, reference_map),
            "analysis": compare(candidate_alm, reference_alm),
        },
    }
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
