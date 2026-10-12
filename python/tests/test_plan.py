# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

import numpy as np
import pytest
from ducc0 import sht as ducc_sht

from rustsht import ShtPlan


def to_ducc_layout(
    alm: np.ndarray, lmax: int, mmax: int
) -> np.ndarray:
    """Copy order-major RustSHT coefficients into DUCC's default alm layout."""
    ducc = np.zeros((alm.shape[0], (lmax + 1) * (lmax + 2) // 2), dtype=np.complex128)
    rust_offset = 0
    for m in range(mmax + 1):
        for l in range(m, lmax + 1):
            ducc_index = m * (2 * lmax + 1 - m) // 2 + l
            ducc[:, ducc_index] = alm[:, rust_offset]
            rust_offset += 1
    return ducc


def from_ducc_layout(
    alm: np.ndarray, lmax: int, mmax: int
) -> np.ndarray:
    """Copy DUCC's default alm layout into order-major RustSHT coefficients."""
    packed = np.empty((alm.shape[0], sum(lmax - m + 1 for m in range(mmax + 1))), dtype=np.complex128)
    rust_offset = 0
    for m in range(mmax + 1):
        for l in range(m, lmax + 1):
            ducc_index = m * (2 * lmax + 1 - m) // 2 + l
            packed[:, rust_offset] = alm[:, ducc_index]
            rust_offset += 1
    return packed


def random_alm(ncomp: int, lmax: int, mmax: int, spin: int, seed: int) -> np.ndarray:
    rng = np.random.default_rng(seed)
    alm = rng.normal(size=(ncomp, sum(lmax - m + 1 for m in range(mmax + 1))))
    alm = alm + 1j * rng.normal(size=alm.shape)
    alm[:, : lmax + 1] = alm[:, : lmax + 1].real
    for m in range(mmax + 1):
        for l in range(m, min(spin, lmax + 1)):
            alm[:, sum(lmax - order + 1 for order in range(m)) + l - m] = 0.0
    return np.ascontiguousarray(alm, dtype=np.complex128)


@pytest.mark.parametrize("kind", ["GL", "CC", "F1"])
def test_scalar_plan_matches_ducc(kind: str) -> None:
    lmax = mmax = 3
    ntheta, nphi = 5, 9
    phi0 = 0.17
    plan = ShtPlan(kind, ntheta, nphi, lmax, mmax, phi0=phi0)
    alm = random_alm(1, lmax, mmax, 0, seed=13)
    expected_map = ducc_sht.synthesis_2d(
        alm=to_ducc_layout(alm, lmax, mmax),
        spin=0,
        lmax=lmax,
        mmax=mmax,
        geometry=kind,
        ntheta=ntheta,
        nphi=nphi,
        phi0=phi0,
        nthreads=1,
    )

    actual_map = plan.synthesis(alm)
    assert actual_map.shape == (1, ntheta, nphi)
    np.testing.assert_allclose(actual_map, expected_map, rtol=2e-13, atol=2e-13)

    expected_alm = ducc_sht.analysis_2d(
        map=expected_map,
        spin=0,
        lmax=lmax,
        mmax=mmax,
        geometry=kind,
        phi0=phi0,
        nthreads=1,
    )
    np.testing.assert_allclose(
        plan.analysis(actual_map), from_ducc_layout(expected_alm, lmax, mmax), rtol=2e-12, atol=2e-12
    )

    arbitrary_map = np.random.default_rng(19).normal(size=plan.map_shape)
    arbitrary_alm = ducc_sht.analysis_2d(
        map=arbitrary_map,
        spin=0,
        lmax=lmax,
        mmax=mmax,
        geometry=kind,
        phi0=phi0,
        nthreads=1,
    )
    np.testing.assert_allclose(
        plan.analysis(arbitrary_map),
        from_ducc_layout(arbitrary_alm, lmax, mmax),
        rtol=2e-12,
        atol=2e-12,
    )

    expected_adjoint_map = ducc_sht.adjoint_analysis_2d(
        alm=to_ducc_layout(alm, lmax, mmax),
        spin=0,
        lmax=lmax,
        mmax=mmax,
        geometry=kind,
        ntheta=ntheta,
        nphi=nphi,
        phi0=phi0,
        nthreads=1,
    )
    np.testing.assert_allclose(plan.adjoint_analysis(alm), expected_adjoint_map, rtol=2e-12, atol=2e-12)

    expected_adjoint_alm = ducc_sht.adjoint_synthesis_2d(
        map=expected_map,
        spin=0,
        lmax=lmax,
        mmax=mmax,
        geometry=kind,
        phi0=phi0,
        nthreads=1,
    )
    np.testing.assert_allclose(
        plan.adjoint_synthesis(expected_map),
        from_ducc_layout(expected_adjoint_alm, lmax, mmax),
        rtol=2e-12,
        atol=2e-12,
    )


def test_spin_plan_matches_ducc_and_derivative_mode() -> None:
    lmax = mmax = 4
    ntheta, nphi = 6, 11
    spin = 2
    phi0 = -0.23
    plan = ShtPlan("GL", ntheta, nphi, lmax, mmax, spin=spin, phi0=phi0)
    alm = random_alm(2, lmax, mmax, spin, seed=29)
    expected_map = ducc_sht.synthesis_2d(
        alm=to_ducc_layout(alm, lmax, mmax),
        spin=spin,
        lmax=lmax,
        mmax=mmax,
        geometry="GL",
        ntheta=ntheta,
        nphi=nphi,
        phi0=phi0,
        nthreads=1,
    )
    actual_map = plan.synthesis(alm)
    assert actual_map.shape == (2, ntheta, nphi)
    np.testing.assert_allclose(actual_map, expected_map, rtol=2e-12, atol=2e-12)

    expected_alm = ducc_sht.analysis_2d(
        map=actual_map,
        spin=spin,
        lmax=lmax,
        mmax=mmax,
        geometry="GL",
        phi0=phi0,
        nthreads=1,
    )
    np.testing.assert_allclose(
        plan.analysis(actual_map), from_ducc_layout(expected_alm, lmax, mmax), rtol=2e-12, atol=2e-12
    )

    expected_adjoint_alm = ducc_sht.adjoint_synthesis_2d(
        map=actual_map,
        spin=spin,
        lmax=lmax,
        mmax=mmax,
        geometry="GL",
        phi0=phi0,
        nthreads=1,
    )
    np.testing.assert_allclose(
        plan.adjoint_synthesis(actual_map),
        from_ducc_layout(expected_adjoint_alm, lmax, mmax),
        rtol=2e-12,
        atol=2e-12,
    )

    expected_adjoint_map = ducc_sht.adjoint_analysis_2d(
        alm=to_ducc_layout(alm, lmax, mmax),
        spin=spin,
        lmax=lmax,
        mmax=mmax,
        geometry="GL",
        ntheta=ntheta,
        nphi=nphi,
        phi0=phi0,
        nthreads=1,
    )
    np.testing.assert_allclose(
        plan.adjoint_analysis(alm), expected_adjoint_map, rtol=2e-12, atol=2e-12
    )

    gradient = plan.synthesis_gradient(alm[:1])
    expected_gradient = ducc_sht.synthesis_2d(
        alm=to_ducc_layout(alm[:1], lmax, mmax),
        spin=spin,
        lmax=lmax,
        mmax=mmax,
        geometry="GL",
        ntheta=ntheta,
        nphi=nphi,
        phi0=phi0,
        mode="GRAD_ONLY",
        nthreads=1,
    )
    np.testing.assert_allclose(gradient, expected_gradient, rtol=2e-12, atol=2e-12)

    expected_gradient_adjoint = ducc_sht.adjoint_synthesis_2d(
        map=expected_gradient,
        spin=spin,
        lmax=lmax,
        mmax=mmax,
        geometry="GL",
        phi0=phi0,
        mode="GRAD_ONLY",
        nthreads=1,
    )
    np.testing.assert_allclose(
        plan.adjoint_synthesis_gradient(expected_gradient),
        from_ducc_layout(expected_gradient_adjoint, lmax, mmax),
        rtol=2e-12,
        atol=2e-12,
    )

    scalar_plan = ShtPlan("GL", ntheta, nphi, lmax, mmax, phi0=phi0)
    scalar_alm = random_alm(1, lmax, mmax, 0, seed=31)
    derivative = scalar_plan.synthesis_deriv1(scalar_alm)
    expected_derivative = ducc_sht.synthesis_2d(
        alm=to_ducc_layout(scalar_alm, lmax, mmax),
        spin=1,
        lmax=lmax,
        mmax=mmax,
        geometry="GL",
        ntheta=ntheta,
        nphi=nphi,
        phi0=phi0,
        mode="DERIV1",
        nthreads=1,
    )
    assert derivative.shape == (2, ntheta, nphi)
    np.testing.assert_allclose(derivative, expected_derivative, rtol=2e-12, atol=2e-12)
    expected_derivative_adjoint = ducc_sht.adjoint_synthesis_2d(
        map=expected_derivative,
        spin=1,
        lmax=lmax,
        mmax=mmax,
        geometry="GL",
        phi0=phi0,
        mode="DERIV1",
        nthreads=1,
    )
    np.testing.assert_allclose(
        scalar_plan.adjoint_synthesis_deriv1(expected_derivative),
        from_ducc_layout(expected_derivative_adjoint, lmax, mmax),
        rtol=2e-12,
        atol=2e-12,
    )


def test_adjoint_pairings_and_input_validation() -> None:
    plan = ShtPlan("GL", 5, 9, 3, 3)
    alm = random_alm(1, 3, 3, 0, seed=37)
    map_values = np.random.default_rng(41).normal(size=plan.map_shape)
    synthesized = plan.synthesis(alm)
    adjoint = plan.adjoint_synthesis(map_values)
    # The real harmonic inner product doubles the contribution from m > 0.
    packed_weights = np.ones(plan.nalm)
    packed_weights[4:] = 2.0
    left = np.sum(synthesized * map_values)
    right = np.sum(packed_weights * (alm.conj() * adjoint).real)
    assert left == pytest.approx(right, rel=2e-12, abs=2e-12)

    with pytest.raises(ValueError, match="shape"):
        plan.synthesis(alm[:, :-1])
    with pytest.raises(TypeError):
        plan.synthesis(alm.astype(np.complex64))
    with pytest.raises(TypeError):
        plan.adjoint_synthesis(map_values.astype(np.float32))
    spin_plan = ShtPlan("GL", 5, 9, 3, 3, spin=1)
    spin_map = np.zeros(spin_plan.map_shape)
    with pytest.raises(ValueError, match="C-contiguous"):
        spin_plan.adjoint_synthesis(np.asfortranarray(spin_map))

