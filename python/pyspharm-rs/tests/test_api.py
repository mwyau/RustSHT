# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later

import math

import numpy as np
import pytest

import spharm_rs


def test_public_module_functions_and_pyspharm_normalization():
    assert spharm_rs.__version__ == "1.0.9"

    pbar = spharm_rs.legendre(30.0, 2)
    latitude = math.radians(30.0)
    expected = np.array(
        [
            1 / math.sqrt(2),
            math.sqrt(3 / 2) * math.sin(latitude),
            math.sqrt(5 / 8) * (3 * math.sin(latitude) ** 2 - 1),
            math.sqrt(3 / 4) * math.cos(latitude),
            math.sqrt(15 / 4) * math.sin(latitude) * math.cos(latitude),
            math.sqrt(15 / 16) * math.cos(latitude) ** 2,
        ],
        dtype=np.float32,
    )
    assert pbar.dtype == np.float32
    np.testing.assert_allclose(pbar, expected, rtol=2e-7, atol=2e-7)

    coefficients = np.array([2.0 + 7.0j], dtype=np.complex64)
    legendre = np.array([1 / math.sqrt(2)], dtype=np.float32)
    value = spharm_rs.specintrp(123.0, coefficients, legendre)
    assert isinstance(value, np.float32)
    assert value == pytest.approx(math.sqrt(2), rel=2e-7)

    orders, degrees = spharm_rs.getspecindx(2)
    assert orders.dtype == np.int32
    assert degrees.dtype == np.int32
    np.testing.assert_array_equal(orders, [0, 0, 0, 1, 1, 2])
    np.testing.assert_array_equal(degrees, [0, 1, 2, 1, 2, 2])
    zero_order, zero_degree = spharm_rs.getspecindx(0)
    assert zero_order.shape == zero_degree.shape == ()
    assert zero_order.dtype == zero_degree.dtype == np.int32
    assert zero_order == zero_degree == 0

    latitudes, weights = spharm_rs.gaussian_lats_wts(8)
    assert latitudes.dtype == np.float64
    assert weights.dtype == np.float64
    np.testing.assert_allclose(
        latitudes,
        [
            73.79921362856322,
            52.81294318999426,
            31.704091745007943,
            10.569882312576084,
            -10.569882312576084,
            -31.704091745007943,
            -52.81294318999426,
            -73.79921362856322,
        ],
        rtol=0.0,
        atol=4e-10,
    )
    assert weights.sum() == pytest.approx(2.0, abs=2e-14)

    geodesic_lat, geodesic_lon = spharm_rs.getgeodesicpts(2)
    assert geodesic_lat.dtype == geodesic_lon.dtype == np.float32
    assert geodesic_lat.shape == geodesic_lon.shape == (12,)
    np.testing.assert_allclose(
        geodesic_lat,
        [90, -90, *([26.56505] * 5), *([-26.56505] * 5)],
        atol=1e-4,
    )
    np.testing.assert_allclose(
        np.mod(geodesic_lon, 360),
        np.mod([0, 0, 180, 252, 324, 36, 108, 216, 288, 360, 72, 144], 360),
        atol=2e-5,
    )


def _reference_inputs():
    row = np.arange(8, dtype=np.float32)[:, None]
    column = np.arange(16, dtype=np.float32)[None, :]
    field = (
        np.arange(8 * 16, dtype=np.float32).reshape(8, 16) / np.float32(127.0)
        - np.float32(0.5)
    )
    eastward = (np.float32(0.1) * row + np.float32(0.01) * column).astype(np.float32)
    northward = (np.float32(0.04) * row - np.float32(0.02) * column).astype(np.float32)
    return field, eastward, northward


def test_scalar_and_vector_methods_match_pyspharm_syl_reference():
    # Reference values were recorded with pyspharm-syl 1.2.1, source revision
    # 9e4446ecec539ef01f77e7b7ed96980b9a9fb262, on Python 3.13.15.
    transform = spharm.Spharmt(16, 8, gridtype="gaussian", legfunc="computed")
    field, eastward, northward = _reference_inputs()

    spectrum = transform.grdtospec(field, ntrunc=4)
    expected_spectrum = np.array(
        [
            0.0 + 0.0j,
            -0.328254223 + 0.0j,
            0.0 + 0.0j,
            -0.0306984391 + 0.0j,
            0.0 + 0.0j,
            -0.00536034117 + 0.0269482546j,
            9.83852888e-10 - 3.54917051e-9j,
            -0.00127080444 + 0.00638876762j,
            -4.55712273e-10 + 2.16409113e-9j,
            -0.00508265477 + 0.0122706145j,
            -2.49636023e-9 + 6.23187413e-9j,
            -0.00176068349 + 0.0042506652j,
            -0.00485046115 + 0.00725922827j,
            1.33023326e-9 - 1.8087587e-9j,
            -0.00465832837 + 0.00465832837j,
        ],
        dtype=np.complex64,
    )
    assert spectrum.dtype == np.complex64
    np.testing.assert_allclose(spectrum, expected_spectrum, rtol=3e-6, atol=2e-8)

    restored = transform.spectogrd(spectrum)
    assert restored.dtype == np.float32
    assert restored.shape == field.shape
    # Samples were measured with pyspharm-syl 1.2.1; the input has energy
    # above T4, so its T4 synthesis is a projection rather than a roundtrip.
    np.testing.assert_allclose(
        restored.ravel()[:12],
        [
            -0.43744874,
            -0.44909325,
            -0.45525315,
            -0.45555934,
            -0.4516445,
            -0.44562474,
            -0.43921438,
            -0.43328342,
            -0.42767882,
            -0.4217479,
            -0.41533753,
            -0.40931776,
        ],
        rtol=2e-6,
        atol=2e-7,
    )

    constant = transform.spectogrd(np.array([math.sqrt(2)], dtype=np.complex64))
    np.testing.assert_allclose(constant, 1.0, rtol=2e-7, atol=2e-7)

    vorticity, divergence = transform.getvrtdivspec(eastward, northward, ntrunc=4)
    assert vorticity.dtype == divergence.dtype == np.complex64
    np.testing.assert_allclose(vorticity[1:5], [
        1.28442494e-7, -9.0192799e-8, 7.45882929e-8, -6.6487523e-8
    ], rtol=4e-6, atol=1e-13)
    np.testing.assert_allclose(divergence[1:5], [
        3.02217629e-9, 3.60771182e-8, 1.75501913e-9, 2.65950124e-8
    ], rtol=4e-6, atol=1e-13)

    eastward_wind, northward_wind = transform.getuv(spectrum * 0.3, spectrum * -0.2)
    assert eastward_wind.dtype == northward_wind.dtype == np.float32
    np.testing.assert_allclose(
        eastward_wind.ravel()[:8],
        [-147078.64, -117812.91, -91863.08, -74213.625, -65987.8, -66097.17, -72701.23, -83921.99],
        rtol=3e-6,
        atol=0.2,
    )

    streamfunction, potential = transform.getpsichi(eastward, northward, ntrunc=4)
    np.testing.assert_allclose(
        streamfunction.ravel()[:4],
        [-2590365.0, -2526363.0, -2440710.25, -2355864.25],
        rtol=3e-6,
        atol=0.5,
    )
    np.testing.assert_allclose(
        potential.ravel()[:4],
        [-368565.88, -348463.25, -355030.44, -381891.22],
        rtol=3e-6,
        atol=0.25,
    )

    gradient_east, gradient_north = transform.getgrad(spectrum)
    np.testing.assert_allclose(
        gradient_east.ravel()[:4],
        [-1.9376778e-8, -1.3185194e-8, -4.4026542e-9, 3.0938292e-9],
        rtol=5e-6,
        atol=2e-14,
    )
    assert gradient_north.shape == field.shape

    smoothed = transform.specsmooth(field, np.linspace(1, 0.2, 8, dtype=np.float32))
    assert smoothed.shape == (8, 16, 1)
    np.testing.assert_allclose(
        smoothed.ravel()[:4], [-0.38256744, -0.3941563, -0.39977258, -0.39988226],
        rtol=3e-6,
        atol=2e-7,
    )

    output_grid = spharm.Spharmt(12, 6, gridtype="gaussian", legfunc="computed")
    remapped = spharm.regrid(
        transform,
        output_grid,
        field,
        ntrunc=4,
        smooth=np.linspace(1, 0.4, 6, dtype=np.float32),
    )
    assert remapped.shape == (6, 12, 1)
    np.testing.assert_allclose(
        remapped.ravel()[:4], [-0.3599232, -0.3740071, -0.37778878, -0.3734724],
        rtol=3e-6,
        atol=2e-7,
    )


@pytest.mark.parametrize("nlat", [7, 8])
def test_regular_grid_uses_source_colatitude_nodes_and_preserves_batch_shapes(nlat):
    nlon = 16
    transform = spharm.Spharmt(nlon, nlat, gridtype="regular", legfunc="stored")
    assert transform.nlat == nlat
    assert transform.nlon == nlon
    with pytest.raises(AttributeError):
        transform.nlat = nlat + 1
    with pytest.raises(AttributeError):
        del transform.gridtype

    field = np.ones((nlat, nlon, 2), dtype=np.float32)
    spectrum = transform.grdtospec(field, ntrunc=nlat - 2)
    assert spectrum.shape == ((nlat - 1) * nlat // 2, 2)
    assert spectrum.dtype == np.complex64
    restored = transform.spectogrd(spectrum)
    assert restored.shape == (nlat, nlon, 2)
    assert restored.dtype == np.float32


@pytest.mark.parametrize("nlat", [7, 8])
def test_regular_grid_default_maximum_truncation_is_supported(nlat):
    transform = spharm.Spharmt(16, nlat, gridtype="regular", legfunc="computed")
    latitude_index = np.arange(nlat, dtype=np.float32)[:, None]
    longitude_index = np.arange(16, dtype=np.float32)[None, :]
    eastward = (0.1 * latitude_index + 0.01 * longitude_index).astype(np.float32)
    northward = (0.04 * latitude_index - 0.02 * longitude_index).astype(np.float32)

    spectrum = transform.grdtospec(eastward)
    assert spectrum.shape == (nlat * (nlat + 1) // 2,)
    assert np.isfinite(spectrum).all()
    vorticity, divergence = transform.getvrtdivspec(eastward, northward)
    assert vorticity.shape == divergence.shape == spectrum.shape
    assert np.isfinite(vorticity).all() and np.isfinite(divergence).all()

    streamfunction, potential = transform.getpsichi(eastward, northward)
    assert streamfunction.shape == potential.shape == (nlat, 16)
    assert np.isfinite(streamfunction).all() and np.isfinite(potential).all()


def test_array_layout_conversion_and_legacy_repr():
    transform = spharm.Spharmt(16, 8, gridtype="gaussian", legfunc="stored")
    assert repr(transform) == "Spharmt(16, 8, 6.371200e+06, gaussian, stored)"

    backing = np.arange(16 * 32, dtype=np.float64).reshape(16, 32)
    strided = backing[::2, ::2]
    assert strided.shape == (8, 16)
    assert not strided.flags.c_contiguous
    spectrum = transform.grdtospec(strided, ntrunc=4)
    expected = transform.grdtospec(np.ascontiguousarray(strided, dtype=np.float32), ntrunc=4)
    np.testing.assert_array_equal(spectrum, expected)

    fortran = np.asfortranarray(strided, dtype=np.float32)
    fortran.setflags(write=False)
    np.testing.assert_array_equal(transform.grdtospec(fortran, ntrunc=4), expected)


def test_invalid_shapes_truncations_and_module_arguments_raise_value_error():
    with pytest.raises(ValueError):
        spharm.Spharmt(3, 8)
    with pytest.raises(ValueError):
        spharm.Spharmt(16, 8, rsphere=float("nan"))
    with pytest.raises(ValueError):
        spharm.getgeodesicpts(0)
    with pytest.raises(ValueError):
        spharm.legendre(0.0, -1)

    transform = spharm.Spharmt(16, 8, gridtype="gaussian")
    with pytest.raises(ValueError):
        transform.grdtospec(np.ones((8, 16), dtype=np.float32), ntrunc=-1)
    with pytest.raises(ValueError):
        spharm.specintrp(
            0.0,
            np.zeros(6, dtype=np.complex64),
            np.zeros(3, dtype=np.float32),
        )
