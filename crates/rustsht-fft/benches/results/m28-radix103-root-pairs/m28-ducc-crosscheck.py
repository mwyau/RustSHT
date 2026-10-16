from pathlib import Path

import ducc0
import numpy as np

n = 1442
x = np.arange(1, n + 1, dtype=np.float64)
values = np.sin(0.173 * x) + 1j * np.cos(0.113 * (x + 1.0))
results = Path(__file__).parent

for direction in ("forward", "inverse"):
    data = np.loadtxt(results / f"m28-ducc-{direction}.txt")
    actual = data[:, 0] + 1j * data[:, 1]
    expected = ducc0.fft.c2c(
        values,
        forward=direction == "forward",
        inorm=0 if direction == "forward" else 2,
        nthreads=1,
    )
    relative_l2 = np.linalg.norm(actual - expected) / np.linalg.norm(expected)
    max_abs = np.max(np.abs(actual - expected))
    print(
        f"ducc0={ducc0.__version__} direction={direction} "
        f"relative_l2={relative_l2:.3e} max_abs={max_abs:.3e}"
    )
