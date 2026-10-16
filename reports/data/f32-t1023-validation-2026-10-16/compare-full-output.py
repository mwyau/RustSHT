#!/usr/bin/env python3
"""Compare dumped RustSHT T1023 f32 outputs with pinned DUCC references."""

import importlib.util
import json
from pathlib import Path
import sys

import ducc0
import numpy as np
from ducc0 import sht


def load_reference_generator():
    generator_path = Path(__file__).with_name("generate-ducc-reference.py")
    spec = importlib.util.spec_from_file_location("f32_t1023_reference", generator_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load reference generator at {generator_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def compare(actual: np.ndarray, reference: np.ndarray) -> dict[str, float | int]:
    actual_wide = actual.astype(np.complex128 if np.iscomplexobj(actual) else np.float64)
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


def read_map(path: Path, shape: tuple[int, ...]) -> np.ndarray:
    values = np.fromfile(path, dtype="<f4")
    if values.size != int(np.prod(shape)):
        raise ValueError(f"{path}: expected {np.prod(shape)} float32 values, got {values.size}")
    return values.reshape(shape)


def read_alm(path: Path, shape: tuple[int, ...]) -> np.ndarray:
    values = np.fromfile(path, dtype="<c8")
    if values.size != int(np.prod(shape)):
        raise ValueError(f"{path}: expected {np.prod(shape)} complex64 values, got {values.size}")
    return values.reshape(shape)


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {Path(sys.argv[0]).name} RUSTSHT_F32_DUMP_DIR")
    dump_dir = Path(sys.argv[1])
    reference = load_reference_generator()

    results: dict[str, object] = {
        "ducc0": ducc0.__version__,
        "ducc_source_revision": "c1ed01b8fda69d2a378190f0915f1a19762dddbf",
        "grid": {
            "geometry": "GL",
            "lmax": reference.LMAX,
            "mmax": reference.MMAX,
            "ntheta": reference.NTHETA,
            "nphi": reference.NPHI,
            "phi0": reference.PHI0,
            "dtype": "float32 / complex64",
            "nthreads": 1,
        },
        "comparison": {},
    }

    for spin in (0, 1):
        expected_synthesis = sht.synthesis_2d(
            alm=reference.coefficients(spin),
            spin=spin,
            lmax=reference.LMAX,
            mmax=reference.MMAX,
            geometry="GL",
            ntheta=reference.NTHETA,
            nphi=reference.NPHI,
            phi0=reference.PHI0,
            nthreads=1,
        )
        expected_analysis = sht.analysis_2d(
            map=reference.maps(spin),
            spin=spin,
            lmax=reference.LMAX,
            mmax=reference.MMAX,
            geometry="GL",
            phi0=reference.PHI0,
            nthreads=1,
        )
        synthesis_results = []
        analysis_results = []
        for channel in range(expected_synthesis.shape[0]):
            actual = read_map(
                dump_dir / f"spin{spin}-synthesis-channel{channel}.f32",
                (reference.NTHETA, reference.NPHI),
            )
            synthesis_results.append(compare(actual, expected_synthesis[channel]))
        for channel in range(expected_analysis.shape[0]):
            actual = read_alm(
                dump_dir / f"spin{spin}-analysis-channel{channel}.c32",
                ((reference.LMAX + 1) * (reference.LMAX + 2) // 2,),
            )
            analysis_results.append(compare(actual, expected_analysis[channel]))
        results["comparison"][str(spin)] = {
            "synthesis": synthesis_results,
            "analysis": analysis_results,
        }

    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
