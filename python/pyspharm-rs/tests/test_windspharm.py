# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

import numpy as np
import pytest

windspharm_standard = pytest.importorskip("windspharm.standard")
VectorWind = windspharm_standard.VectorWind

import spharm


def test_standard_vectorwind_operators_use_the_spharm_compatibility_api():
    transform = spharm.Spharmt(24, 12, gridtype="gaussian", legfunc="computed")
    orders, degrees = spharm.getspecindx(4)
    vorticity_spectrum = np.zeros(len(orders), dtype=np.complex64)
    divergence_spectrum = np.zeros(len(orders), dtype=np.complex64)
    for index, (order, degree) in enumerate(zip(orders, degrees)):
        if degree:
            vorticity_spectrum[index] = np.complex64(
                0.03 / degree + 0.01 * order / degree * 1j
            )
            divergence_spectrum[index] = np.complex64(
                -0.02 / degree + 0.007 * order / degree * 1j
            )
        if order == 0:
            vorticity_spectrum[index] = vorticity_spectrum[index].real
            divergence_spectrum[index] = divergence_spectrum[index].real

    eastward, northward = transform.getuv(vorticity_spectrum, divergence_spectrum)
    wind = VectorWind(eastward, northward, gridtype="gaussian", legfunc="computed")
    scalar_spectrum = np.zeros_like(vorticity_spectrum)
    scalar_spectrum[1] = np.complex64(0.12)
    scalar_spectrum[4] = np.complex64(0.05 - 0.02j)
    scalar = transform.spectogrd(scalar_spectrum)

    scalar_fields = [
        wind.magnitude(),
        wind.vorticity(),
        wind.divergence(),
        wind.absolutevorticity(),
        wind.streamfunction(),
        wind.velocitypotential(),
        wind.truncate(scalar, truncation=3),
    ]
    scalar_fields.extend(wind.sfvp())
    scalar_fields.extend(wind.vrtdiv(truncation=4))
    scalar_fields.extend(wind.gradient(scalar, truncation=4))
    scalar_fields.extend(wind.helmholtz(truncation=4))
    scalar_fields.extend(wind.nondivergentcomponent(truncation=4))
    scalar_fields.extend(wind.irrotationalcomponent(truncation=4))
    for values in scalar_fields:
        assert values.shape == (12, 24)
        assert np.isfinite(values).all()
    np.testing.assert_allclose(
        scalar_fields[0], np.hypot(eastward, northward), rtol=2e-7, atol=2e-7
    )
