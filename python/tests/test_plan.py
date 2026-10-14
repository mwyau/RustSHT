# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

from concurrent.futures import ThreadPoolExecutor

import numpy as np
import pytest
from ducc0 import sht as ducc_sht

import rustsht
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
    np.testing.assert_allclose(
        rustsht.synthesis_2d(
            alm,
            spin=0,
            lmax=lmax,
            geometry=kind,
            ntheta=ntheta,
            nphi=nphi,
            mmax=mmax,
            phi0=phi0,
        ),
        expected_map,
        rtol=2e-13,
        atol=2e-13,
    )

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
    np.testing.assert_allclose(
        rustsht.analysis_2d(
            arbitrary_map,
            spin=0,
            lmax=lmax,
            geometry=kind,
            mmax=mmax,
            phi0=phi0,
        ),
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
    np.testing.assert_allclose(
        rustsht.adjoint_analysis_2d(
            alm,
            spin=0,
            lmax=lmax,
            geometry=kind,
            ntheta=ntheta,
            nphi=nphi,
            mmax=mmax,
            phi0=phi0,
        ),
        expected_adjoint_map,
        rtol=2e-12,
        atol=2e-12,
    )

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
    np.testing.assert_allclose(
        rustsht.adjoint_synthesis_2d(
            expected_map,
            spin=0,
            lmax=lmax,
            geometry=kind,
            mmax=mmax,
            phi0=phi0,
        ),
        from_ducc_layout(expected_adjoint_alm, lmax, mmax),
        rtol=2e-12,
        atol=2e-12,
    )

    np.testing.assert_allclose(
        rustsht.get_gridweights(kind, ntheta),
        ducc_sht.get_gridweights(kind, ntheta),
        rtol=2e-14,
        atol=2e-14,
    )


@pytest.mark.parametrize("spin", [0, 1])
def test_overresolved_gl_plan_matches_ducc(spin: int) -> None:
    lmax = mmax = 63
    ntheta, nphi = 96, 192
    phi0 = 0.37
    components = 1 if spin == 0 else 2
    plan = ShtPlan("GL", ntheta, nphi, lmax, mmax, spin=spin, phi0=phi0)
    alm = random_alm(components, lmax, mmax, spin, seed=163 + spin)
    ducc_alm = to_ducc_layout(alm, lmax, mmax)
    expected_map = ducc_sht.synthesis_2d(
        alm=ducc_alm,
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
    np.testing.assert_allclose(actual_map, expected_map, rtol=2e-12, atol=2e-12)

    actual_alm = plan.analysis(actual_map)
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
        actual_alm, from_ducc_layout(expected_alm, lmax, mmax), rtol=2e-12, atol=2e-12
    )

    input_map = np.random.default_rng(263 + spin).normal(size=actual_map.shape)
    expected_analysis_adjoint = ducc_sht.adjoint_analysis_2d(
        alm=ducc_alm,
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
        plan.adjoint_analysis(alm),
        expected_analysis_adjoint,
        rtol=2e-12,
        atol=2e-12,
    )
    expected_synthesis_adjoint = ducc_sht.adjoint_synthesis_2d(
        map=input_map,
        spin=spin,
        lmax=lmax,
        mmax=mmax,
        geometry="GL",
        phi0=phi0,
        nthreads=1,
    )
    np.testing.assert_allclose(
        plan.adjoint_synthesis(input_map),
        from_ducc_layout(expected_synthesis_adjoint, lmax, mmax),
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
    np.testing.assert_allclose(
        rustsht.synthesis_2d(
            alm,
            spin=spin,
            lmax=lmax,
            geometry="GL",
            ntheta=ntheta,
            nphi=nphi,
            mmax=mmax,
            phi0=phi0,
        ),
        expected_map,
        rtol=2e-12,
        atol=2e-12,
    )

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
    np.testing.assert_allclose(
        rustsht.analysis_2d(
            actual_map,
            spin=spin,
            lmax=lmax,
            geometry="GL",
            mmax=mmax,
            phi0=phi0,
        ),
        from_ducc_layout(expected_alm, lmax, mmax),
        rtol=2e-12,
        atol=2e-12,
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
    np.testing.assert_allclose(
        rustsht.adjoint_synthesis_2d(
            actual_map,
            spin=spin,
            lmax=lmax,
            geometry="GL",
            mmax=mmax,
            phi0=phi0,
        ),
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
    np.testing.assert_allclose(
        rustsht.adjoint_analysis_2d(
            alm,
            spin=spin,
            lmax=lmax,
            geometry="GL",
            ntheta=ntheta,
            nphi=nphi,
            mmax=mmax,
            phi0=phi0,
        ),
        expected_adjoint_map,
        rtol=2e-12,
        atol=2e-12,
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
    np.testing.assert_allclose(
        rustsht.synthesis_2d(
            alm[:1],
            spin=spin,
            lmax=lmax,
            geometry="GL",
            ntheta=ntheta,
            nphi=nphi,
            mmax=mmax,
            phi0=phi0,
            mode="GRAD_ONLY",
        ),
        expected_gradient,
        rtol=2e-12,
        atol=2e-12,
    )

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
    np.testing.assert_allclose(
        rustsht.adjoint_synthesis_2d(
            expected_gradient,
            spin=spin,
            lmax=lmax,
            geometry="GL",
            mmax=mmax,
            phi0=phi0,
            mode="GRAD_ONLY",
        ),
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
    np.testing.assert_allclose(
        rustsht.synthesis_2d(
            scalar_alm,
            spin=1,
            lmax=lmax,
            geometry="GL",
            ntheta=ntheta,
            nphi=nphi,
            mmax=mmax,
            phi0=phi0,
            mode="DERIV1",
        ),
        expected_derivative,
        rtol=2e-12,
        atol=2e-12,
    )
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
    np.testing.assert_allclose(
        rustsht.adjoint_synthesis_2d(
            expected_derivative,
            spin=1,
            lmax=lmax,
            geometry="GL",
            mmax=mmax,
            phi0=phi0,
            mode="DERIV1",
        ),
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

    readonly_alm = alm.copy()
    readonly_alm.setflags(write=False)
    readonly_map = map_values.copy()
    readonly_map.setflags(write=False)
    np.testing.assert_array_equal(plan.synthesis(readonly_alm), synthesized)
    np.testing.assert_array_equal(plan.analysis(readonly_map), plan.analysis(map_values))

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


def test_readonly_input_can_be_shared_by_concurrent_calls() -> None:
    plan = ShtPlan("GL", 5, 9, 3, 3)
    alm = random_alm(1, 3, 3, 0, seed=59)
    alm.setflags(write=False)
    expected = plan.synthesis(alm)

    with ThreadPoolExecutor(max_workers=2) as workers:
        outputs = list(workers.map(plan.synthesis, [alm, alm]))

    for output in outputs:
        np.testing.assert_array_equal(output, expected)


@pytest.mark.parametrize("kind", ["GL", "CC", "F1"])
@pytest.mark.parametrize("spin", [0, 1])
def test_plan_preallocated_standard_outputs(kind: str, spin: int) -> None:
    lmax = mmax = 3
    ntheta = 5 if kind != "CC" else 6
    nphi = 9
    plan = ShtPlan(kind, ntheta, nphi, lmax, mmax, spin=spin, phi0=0.23)
    alm = random_alm(1 if spin == 0 else 2, lmax, mmax, spin, seed=73 + spin)
    expected_map = plan.synthesis(alm)
    output_map = np.full(plan.map_shape, np.nan, dtype=np.float64)
    assert plan.synthesis_into(alm, output_map) is None
    np.testing.assert_array_equal(output_map, expected_map)

    expected_alm = plan.analysis(expected_map)
    output_alm = np.full(plan.alm_shape, np.nan + 1j * np.nan, dtype=np.complex128)
    assert plan.analysis_into(expected_map, output_alm) is None
    np.testing.assert_array_equal(output_alm, expected_alm)


def test_plan_preallocated_output_validation_and_overlap() -> None:
    plan = ShtPlan("GL", 5, 9, 3, 3)
    alm = random_alm(1, 3, 3, 0, seed=79)
    map_values = np.random.default_rng(83).normal(size=plan.map_shape)

    with pytest.raises(ValueError, match="out shape"):
        plan.synthesis_into(alm, np.empty((1, 5, 8), dtype=np.float64))
    with pytest.raises(ValueError, match="C-contiguous"):
        plan.synthesis_into(alm, np.empty((1, 5, 18), dtype=np.float64)[:, :, ::2])
    readonly_map = np.empty(plan.map_shape, dtype=np.float64)
    readonly_map.setflags(write=False)
    with pytest.raises(ValueError, match="writable"):
        plan.synthesis_into(alm, readonly_map)
    with pytest.raises(TypeError):
        plan.synthesis_into(alm, np.empty(plan.map_shape, dtype=np.float32))

    with pytest.raises(ValueError, match="out shape"):
        plan.analysis_into(map_values, np.empty((1, plan.nalm - 1), dtype=np.complex128))
    with pytest.raises(ValueError, match="C-contiguous"):
        plan.analysis_into(
            map_values, np.empty((1, plan.nalm * 2), dtype=np.complex128)[:, ::2]
        )
    readonly_alm = np.empty(plan.alm_shape, dtype=np.complex128)
    readonly_alm.setflags(write=False)
    with pytest.raises(ValueError, match="writable"):
        plan.analysis_into(map_values, readonly_alm)
    with pytest.raises(TypeError):
        plan.analysis_into(map_values, np.empty(plan.alm_shape, dtype=np.complex64))

    byte_count = max(alm.nbytes, map_values.nbytes) + 32
    shared = np.zeros(byte_count, dtype=np.uint8)
    aliased_alm = np.ndarray(alm.shape, dtype=np.complex128, buffer=shared, offset=0)
    aliased_alm[:] = alm
    overlapping_map = np.ndarray(
        plan.map_shape, dtype=np.float64, buffer=shared, offset=np.dtype(np.float64).itemsize
    )
    with pytest.raises(ValueError, match="must not overlap"):
        plan.synthesis_into(aliased_alm, overlapping_map)

    shared = np.zeros(byte_count, dtype=np.uint8)
    aliased_map = np.ndarray(plan.map_shape, dtype=np.float64, buffer=shared, offset=0)
    aliased_map[:] = map_values
    overlapping_alm = np.ndarray(
        plan.alm_shape,
        dtype=np.complex128,
        buffer=shared,
        offset=np.dtype(np.float64).itemsize,
    )
    with pytest.raises(ValueError, match="must not overlap"):
        plan.analysis_into(aliased_map, overlapping_alm)

