# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

from typing import Literal

import numpy as np
from numpy.typing import NDArray

__version__: str

GridKind = Literal["GL", "CC", "F1"]
SynthesisMode = Literal["STANDARD", "GRAD_ONLY", "DERIV1"]

class ShtPlan:
    """Reusable GL, CC, or F1 transform plan.

    Map rings run north to south and longitudes eastward from ``phi0``.
    Calls on one plan serialize while reusing its workspace.
    """

    def __init__(
        self,
        kind: Literal["GL", "CC", "F1"],
        ntheta: int,
        nphi: int,
        lmax: int,
        mmax: int,
        spin: int = 0,
        phi0: float = 0.0,
    ) -> None: ...

    @property
    def nalm(self) -> int: ...

    @property
    def alm_shape(self) -> tuple[int, int]: ...

    @property
    def map_shape(self) -> tuple[int, int, int]: ...

    def synthesis(self, alm: NDArray[np.complex128]) -> NDArray[np.float64]: ...
    def analysis(self, map: NDArray[np.float64]) -> NDArray[np.complex128]: ...
    def adjoint_synthesis(self, map: NDArray[np.float64]) -> NDArray[np.complex128]: ...
    def adjoint_analysis(self, alm: NDArray[np.complex128]) -> NDArray[np.float64]: ...
    def synthesis_gradient(self, e_alm: NDArray[np.complex128]) -> NDArray[np.float64]: ...
    def adjoint_synthesis_gradient(self, map: NDArray[np.float64]) -> NDArray[np.complex128]: ...
    def synthesis_deriv1(self, alm: NDArray[np.complex128]) -> NDArray[np.float64]: ...
    def adjoint_synthesis_deriv1(self, map: NDArray[np.float64]) -> NDArray[np.complex128]: ...

def synthesis_2d(
    alm: NDArray[np.complex128],
    spin: int,
    lmax: int,
    geometry: GridKind,
    ntheta: int,
    nphi: int,
    mmax: int | None = None,
    phi0: float = 0.0,
    mode: SynthesisMode = "STANDARD",
) -> NDArray[np.float64]: ...

def analysis_2d(
    map: NDArray[np.float64],
    spin: int,
    lmax: int,
    geometry: GridKind,
    mmax: int | None = None,
    phi0: float = 0.0,
) -> NDArray[np.complex128]: ...

def adjoint_synthesis_2d(
    map: NDArray[np.float64],
    spin: int,
    lmax: int,
    geometry: GridKind,
    mmax: int | None = None,
    phi0: float = 0.0,
    mode: SynthesisMode = "STANDARD",
) -> NDArray[np.complex128]: ...

def adjoint_analysis_2d(
    alm: NDArray[np.complex128],
    spin: int,
    lmax: int,
    geometry: GridKind,
    ntheta: int,
    nphi: int,
    mmax: int | None = None,
    phi0: float = 0.0,
) -> NDArray[np.float64]: ...

def get_gridweights(geometry: GridKind, ntheta: int) -> NDArray[np.float64]: ...
