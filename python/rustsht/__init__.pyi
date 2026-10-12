# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

from typing import Literal

import numpy as np
from numpy.typing import NDArray

__version__: str

class ShtPlan:
    """Reusable GL, CC, or F1 plan; map rings run north to south and longitudes eastward from phi0."""

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
