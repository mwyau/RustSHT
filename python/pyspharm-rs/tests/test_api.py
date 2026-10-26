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


@pytest.mark.parametrize("nlat", [7, 8])
def test_regular_grid_uses_source_colatitude_nodes_and_preserves_batch_shapes(nlat):
    nlon = 16
    transform = spharm_rs.Spharmt(nlon, nlat, gridtype="regular", legfunc="stored")
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
    transform = spharm_rs.Spharmt(16, nlat, gridtype="regular", legfunc="computed")
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
    transform = spharm_rs.Spharmt(16, 8, gridtype="gaussian", legfunc="stored")
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
        spharm_rs.Spharmt(3, 8)
    with pytest.raises(ValueError):
        spharm_rs.Spharmt(16, 8, rsphere=float("nan"))
    with pytest.raises(ValueError):
        spharm_rs.getgeodesicpts(0)
    with pytest.raises(ValueError):
        spharm_rs.legendre(0.0, -1)

    transform = spharm_rs.Spharmt(16, 8, gridtype="gaussian")
    with pytest.raises(ValueError):
        transform.grdtospec(np.ones((8, 16), dtype=np.float32), ntrunc=-1)
    with pytest.raises(ValueError):
        spharm_rs.specintrp(
            0.0,
            np.zeros(6, dtype=np.complex64),
            np.zeros(3, dtype=np.float32),
        )
