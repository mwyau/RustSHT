#!/usr/bin/env python3
"""Count finite, zero, subnormal and nonfinite scalar components in captures."""
import json
from pathlib import Path

import numpy as np


def count(path: Path, dtype: str):
    values = np.fromfile(path, dtype=dtype)
    if values.dtype.kind == "c":
        values = values.view(np.float32 if values.dtype.itemsize == 8 else np.float64)
    tiny = np.finfo(values.dtype).tiny
    finite = np.isfinite(values)
    return {
        "components": int(values.size),
        "finite": int(finite.sum()),
        "nonfinite": int((~finite).sum()),
        "zero": int((values == 0).sum()),
        "subnormal_nonzero": int(((values != 0) & (np.abs(values) < tiny)).sum()),
        "max_abs_finite": float(np.max(np.abs(values[finite]), initial=0.0)),
        "min_abs_nonzero_finite": float(np.min(np.abs(values[finite & (values != 0)]), initial=np.inf)),
    }


for root in map(Path, __import__("sys").argv[1:]):
    names = (
        "scalar-map.f32", "q-map.f32", "u-map.f32",
        "scalar-analysis-spectrum.f32", "scalar-analysis-spectrum.f64",
        "spin-analysis-q-spectrum.f32", "spin-analysis-u-spectrum.f32",
        "spin-analysis-e.f32", "spin-analysis-b.f32",
        "scalar-analysis.f32", "spin-analysis-e.f64", "spin-analysis-b.f64",
        "scalar-analysis.f64",
    )
    result = {}
    for name in names:
        dtype = "<c8" if "spectrum" in name or name in (
            "scalar-analysis.f32", "spin-analysis-e.f32", "spin-analysis-b.f32",
            "scalar-analysis.f64", "spin-analysis-e.f64", "spin-analysis-b.f64") else "<f4"
        if name.endswith(".f64"):
            dtype = "<c16" if "spectrum" in name or name in (
                "scalar-analysis.f64", "spin-analysis-e.f64", "spin-analysis-b.f64") else "<f8"
        if (root / name).exists():
            result[name] = count(root / name, dtype)
    result["metadata"] = (root / "metadata.txt").read_text()
    out = root.parent / f"{root.name}-ieee-counts.json"
    out.write_text(json.dumps(result, indent=2) + "\n")
    print(out)
