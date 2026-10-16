#!/usr/bin/env python3
"""Generate compact T1023 f32 STANDARD references from ducc0 0.41.0."""

import json

import ducc0
import numpy as np
from ducc0 import sht

LMAX = MMAX = 1023
NTHETA = 1024
NPHI = 2048
PHI0 = 0.37
MAP_SAMPLES = [
    (0, 0),
    (0, NPHI // 2),
    (1, 17),
    (256, 512),
    (511, 1023),
    (512, NPHI - 1),
    (768, 1536),
    (1023, NPHI - 1),
]
MODES = [
    (0, 0),
    (1, 0),
    (10, 3),
    (127, 31),
    (255, 128),
    (511, 511),
    (768, 256),
    (1023, 0),
    (1023, 1023),
]


def packed_index(l: int, m: int) -> int:
    return m * (2 * LMAX + 1 - m) // 2 + l


def coefficients(spin: int) -> np.ndarray:
    count = 1 if spin == 0 else 2
    alm = np.zeros((count, (LMAX + 1) * (LMAX + 2) // 2), dtype=np.complex64)
    for m in range(LMAX + 1):
        for l in range(max(m, spin), LMAX + 1):
            index = packed_index(l, m)
            for channel in range(count):
                if channel == 0:
                    real = (l * 7 + m * 11) % 19 - 9
                    imag = 0 if m == 0 else (l * 13 + m * 5) % 23 - 11
                else:
                    real = (l * 3 + m * 7) % 17 - 8
                    imag = 0 if m == 0 else (l * 5 + m * 17) % 29 - 14
                alm[channel, index] = np.complex64(
                    complex(np.float32(real) * np.float32(1.0e-5),
                            np.float32(imag) * np.float32(1.0e-5))
                )
    return alm


def maps(spin: int) -> np.ndarray:
    count = 1 if spin == 0 else 2
    result = np.empty((count, NTHETA, NPHI), dtype=np.float32)
    for ring in range(NTHETA):
        for longitude in range(NPHI):
            result[0, ring, longitude] = np.float32(
                np.float32((ring * 31 + longitude * 17) % 101 - 50) / np.float32(101.0)
            )
            if spin == 1:
                result[1, ring, longitude] = np.float32(
                    np.float32((ring * 19 + longitude * 23 + 7) % 97 - 48)
                    / np.float32(97.0)
                )
    return result


def summary(values: np.ndarray) -> dict[str, float]:
    if np.iscomplexobj(values):
        wide = values.astype(np.complex128)
        return {
            "sum_real": float(wide.real.sum()),
            "sum_imag": float(wide.imag.sum()),
            "l2_norm": float(np.sqrt((wide.real**2 + wide.imag**2).sum())),
            "max_abs": float(np.max(np.abs(wide))),
        }
    wide = values.astype(np.float64)
    return {
        "sum": float(wide.sum()),
        "l2_norm": float(np.sqrt((wide * wide).sum())),
        "max_abs": float(np.max(np.abs(wide))),
    }


def main() -> None:
    output: dict[str, object] = {
        "ducc0": ducc0.__version__,
        "ducc_source_revision": "c1ed01b8fda69d2a378190f0915f1a19762dddbf",
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
        "map_samples_ring_longitude": MAP_SAMPLES,
        "alm_samples_degree_order": MODES,
        "operations": {},
    }

    operations: dict[str, object] = {}
    for spin in (0, 1):
        alm = coefficients(spin)
        synthesized = sht.synthesis_2d(
            alm=alm,
            spin=spin,
            lmax=LMAX,
            mmax=MMAX,
            geometry="GL",
            ntheta=NTHETA,
            nphi=NPHI,
            phi0=PHI0,
            nthreads=1,
        )
        analyzed = sht.analysis_2d(
            map=maps(spin),
            spin=spin,
            lmax=LMAX,
            mmax=MMAX,
            geometry="GL",
            phi0=PHI0,
            nthreads=1,
        )
        operations[str(spin)] = {
            "synthesis": [
                {
                    "summary": summary(synthesized[channel]),
                    "samples": [
                        float(synthesized[channel, ring, longitude])
                        for ring, longitude in MAP_SAMPLES
                    ],
                }
                for channel in range(synthesized.shape[0])
            ],
            "analysis": [
                {
                    "summary": summary(analyzed[channel]),
                    "samples": [
                        [
                            float(analyzed[channel, packed_index(l, m)].real),
                            float(analyzed[channel, packed_index(l, m)].imag),
                        ]
                        for l, m in MODES
                    ],
                }
                for channel in range(analyzed.shape[0])
            ],
        }
    output["operations"] = operations
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
