# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

from typing import Literal

import numpy as np
from numpy.typing import NDArray

GridKind = Literal["GL", "CC", "F1"]
SynthesisMode = Literal["STANDARD", "GRAD_ONLY", "DERIV1"]


class ShtPlan:
    def __init__(
        self,
        kind: GridKind,
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
    def synthesis_into(self, alm: NDArray[np.complex128], out: NDArray[np.float64]) -> None: ...
    def analysis(self, map: NDArray[np.float64]) -> NDArray[np.complex128]: ...
    def analysis_into(self, map: NDArray[np.float64], out: NDArray[np.complex128]) -> None: ...
    def adjoint_synthesis(self, map: NDArray[np.float64]) -> NDArray[np.complex128]: ...
    def adjoint_analysis(self, alm: NDArray[np.complex128]) -> NDArray[np.float64]: ...
    def synthesis_gradient(self, alm: NDArray[np.complex128]) -> NDArray[np.float64]: ...
    def adjoint_synthesis_gradient(self, map: NDArray[np.float64]) -> NDArray[np.complex128]: ...
    def synthesis_deriv1(self, alm: NDArray[np.complex128]) -> NDArray[np.float64]: ...
    def adjoint_synthesis_deriv1(self, map: NDArray[np.float64]) -> NDArray[np.complex128]: ...


class ShtPlan32:
    """Internal single-precision transform plan for pyspharm compatibility."""

    def __init__(
        self,
        kind: GridKind,
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

    def synthesis(self, alm: NDArray[np.complex64]) -> NDArray[np.float32]: ...
    def analysis(self, map: NDArray[np.float32]) -> NDArray[np.complex64]: ...
    def analysis_projected(self, map: NDArray[np.float32]) -> NDArray[np.complex64]: ...
    def synthesis_deriv1(self, alm: NDArray[np.complex64]) -> NDArray[np.float32]: ...


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

def pyspharm_legendre(latitude_degrees: float, ntrunc: int) -> NDArray[np.float32]: ...

def pyspharm_specintrp(
    longitude_degrees: float,
    coefficients: NDArray[np.complex64],
    legendre: NDArray[np.float32],
) -> float: ...

def pyspharm_getgeodesicpts(
    subdivision: int,
) -> tuple[NDArray[np.float32], NDArray[np.float32]]: ...

def pyspharm_gaussian_lats_wts(
    nlat: int,
) -> tuple[NDArray[np.float64], NDArray[np.float64]]: ...
