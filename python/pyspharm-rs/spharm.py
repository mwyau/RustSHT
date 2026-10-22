# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
"""pyspharm-compatible transforms implemented with the RustSHT extension."""

from __future__ import annotations

import math
import operator

import numpy as np
from rustsht._rustsht import (
    ShtPlan32,
    pyspharm_gaussian_lats_wts,
    pyspharm_getgeodesicpts,
    pyspharm_legendre,
    pyspharm_specintrp,
)

__version__ = "1.0.9"
__all__ = [
    "Spharmt",
    "gaussian_lats_wts",
    "getgeodesicpts",
    "getspecindx",
    "legendre",
    "regrid",
    "specintrp",
]

_READ_ONLY = {"nlon", "nlat", "rsphere", "gridtype", "legfunc"}
_SQRT_2PI = math.sqrt(2.0 * math.pi)


def _index(value: int, name: str) -> int:
    try:
        return operator.index(value)
    except TypeError as error:
        raise ValueError(f"{name} must be an integer") from error


def _triangular_lmax(length: int) -> int:
    if length < 1:
        raise ValueError("spectral arrays must contain at least one coefficient")
    lmax = 0
    while (lmax + 1) * (lmax + 2) // 2 < length:
        lmax += 1
    if (lmax + 1) * (lmax + 2) // 2 != length:
        raise ValueError(f"coefficient count {length} is not triangular")
    return lmax


def _order_start(lmax: int, order: int) -> int:
    return order * (lmax + 1) - order * (order - 1) // 2


def _py_to_core(values: np.ndarray, lmax: int) -> np.ndarray:
    result = np.ascontiguousarray(values, dtype=np.complex64).copy()
    for order in range(lmax + 1):
        start = _order_start(lmax, order)
        stop = start + lmax - order + 1
        factor = np.float32(_SQRT_2PI if order % 2 == 0 else -_SQRT_2PI)
        result[start:stop] *= factor
    result[: lmax + 1].imag = 0.0
    return result


def _core_to_py(values: np.ndarray, lmax: int) -> np.ndarray:
    result = np.ascontiguousarray(values, dtype=np.complex64).copy()
    for order in range(lmax + 1):
        start = _order_start(lmax, order)
        stop = start + lmax - order + 1
        factor = np.float32(1.0 / _SQRT_2PI if order % 2 == 0 else -1.0 / _SQRT_2PI)
        result[start:stop] *= factor
    result[: lmax + 1].imag = 0.0
    return result


def _as_field_series(array: np.ndarray, nlat: int, nlon: int, name: str) -> tuple[np.ndarray, bool]:
    data = np.asarray(array, dtype=np.float32)
    if data.ndim not in (2, 3):
        raise ValueError(f"{name} must have rank two or three; got rank {data.ndim}")
    if data.shape[:2] != (nlat, nlon):
        raise ValueError(
            f"{name} must begin with shape ({nlat}, {nlon}); got {data.shape[:2]}"
        )
    if data.ndim == 3 and data.shape[2] == 0:
        raise ValueError(f"{name} must contain at least one field")
    return data, data.ndim == 2


def _fields(data: np.ndarray, single: bool):
    if single:
        yield np.ascontiguousarray(data)
    else:
        for index in range(data.shape[2]):
            yield np.ascontiguousarray(data[:, :, index])


def _spectral_series(values: list[np.ndarray], single: bool) -> np.ndarray:
    if single:
        return values[0]
    return np.stack(values, axis=1).astype(np.complex64, copy=False)


def _map_series(values: list[np.ndarray], single: bool) -> np.ndarray:
    if single:
        return values[0]
    return np.stack(values, axis=2).astype(np.float32, copy=False)


def _check_spectrum_array(array: np.ndarray, name: str) -> tuple[np.ndarray, bool, int]:
    values = np.asarray(array, dtype=np.complex64)
    if values.ndim not in (1, 2):
        raise ValueError(f"{name} must have rank one or two; got rank {values.ndim}")
    if values.ndim == 2 and values.shape[1] == 0:
        raise ValueError(f"{name} must contain at least one field")
    return values, values.ndim == 1, _triangular_lmax(values.shape[0])


def _coefficient_columns(values: np.ndarray, single: bool):
    if single:
        yield np.ascontiguousarray(values)
    else:
        for index in range(values.shape[1]):
            yield np.ascontiguousarray(values[:, index])


def _laplacian(values: np.ndarray, rsphere: float) -> np.ndarray:
    lmax = _triangular_lmax(values.shape[0])
    result = np.ascontiguousarray(values, dtype=np.complex64).copy()
    for order in range(lmax + 1):
        start = _order_start(lmax, order)
        for degree in range(order, lmax + 1):
            index = start + degree - order
            result[index] *= np.float32(-degree * (degree + 1) / (rsphere * rsphere))
    return result


def _inverse_laplacian(values: np.ndarray, rsphere: float) -> np.ndarray:
    lmax = _triangular_lmax(values.shape[0])
    result = np.ascontiguousarray(values, dtype=np.complex64).copy()
    for order in range(lmax + 1):
        start = _order_start(lmax, order)
        for degree in range(order, lmax + 1):
            index = start + degree - order
            if degree == 0:
                result[index] = 0.0
            else:
                result[index] *= np.float32(-rsphere * rsphere / (degree * (degree + 1)))
    return result


def _check_truncation(ntrunc: int | None, nlat: int, default: int | None = None) -> int:
    value = nlat - 1 if ntrunc is None and default is None else default if ntrunc is None else ntrunc
    value = _index(value, "ntrunc")
    if value < 0 or value >= nlat:
        raise ValueError(f"ntrunc must be between 0 and {nlat - 1}")
    return value


class Spharmt:
    """Spherical harmonic transforms on regular or Gaussian latitude grids."""

    def __repr__(self) -> str:
        return "Spharmt({:d}, {:d}, {:e}, {:s}, {:s})".format(
            self.nlon, self.nlat, self.rsphere, self.gridtype, self.legfunc
        )

    def __setattr__(self, name, value):
        if name in _READ_ONLY and name in self.__dict__:
            raise AttributeError(f"Attempt to rebind read-only instance variable {name}")
        self.__dict__[name] = value

    def __delattr__(self, name):
        if name in _READ_ONLY and name in self.__dict__:
            raise AttributeError(f"Attempt to unbind read-only instance variable {name}")
        del self.__dict__[name]

    def __init__(
        self,
        nlon: int,
        nlat: int,
        rsphere: float = 6.3712e6,
        gridtype: str = "regular",
        legfunc: str = "stored",
    ):
        nlon = _index(nlon, "nlon")
        nlat = _index(nlat, "nlat")
        if not rsphere > 0.0:
            raise ValueError(f"Spharmt.__init__ illegal value of rsphere ({rsphere}) - must be positive")
        if nlon < 4:
            raise ValueError(f"Spharmt.__init__ illegal value of nlon ({nlon}) - must be at least 4")
        if nlat < 3:
            raise ValueError(f"Spharmt.__init__ illegal value of nlat ({nlat}) - must be at least 3")
        if gridtype not in ("regular", "gaussian"):
            raise ValueError(
                f'Spharmt.__init__ illegal value of gridtype ({gridtype}) - '
                'must be either "gaussian" or "regular"'
            )
        if legfunc not in ("computed", "stored"):
            raise ValueError(
                f'Spharmt.__init__ illegal value of legfunc ({legfunc}) - '
                'must be either "computed" or "stored"'
            )
        self.nlon = nlon
        self.nlat = nlat
        self.rsphere = float(rsphere)
        self.gridtype = gridtype
        self.legfunc = legfunc
        self._geometry = "GL" if gridtype == "gaussian" else "CC"
        self._plan_cache: dict[tuple[int, int], ShtPlan32] = {}

    def _plan(self, lmax: int, spin: int = 0) -> ShtPlan32:
        key = (lmax, spin)
        plan = self._plan_cache.get(key)
        if plan is None:
            plan = ShtPlan32(self._geometry, self.nlat, self.nlon, lmax, lmax, spin)
            self._plan_cache[key] = plan
        return plan

    def _analyze_scalar(self, field: np.ndarray, ntrunc: int) -> np.ndarray:
        plan = self._plan(ntrunc)
        map_array = np.ascontiguousarray(field[None, :, :], dtype=np.float32)
        if self._geometry == "CC" and ntrunc == self.nlat - 1:
            coefficients = plan.analysis_projected(map_array)[0]
        else:
            coefficients = plan.analysis(map_array)[0]
        return _core_to_py(coefficients, ntrunc)

    def grdtospec(self, datagrid: np.ndarray, ntrunc: int | None = None) -> np.ndarray:
        """Analyze one map or a stack of maps into complex64 coefficients."""
        data, single = _as_field_series(datagrid, self.nlat, self.nlon, "grdtospec input")
        ntrunc = _check_truncation(ntrunc, self.nlat)
        spectra = [self._analyze_scalar(field, ntrunc) for field in _fields(data, single)]
        return _spectral_series(spectra, single)

    def spectogrd(self, dataspec: np.ndarray) -> np.ndarray:
        """Synthesize one packed spectrum or a column stack of spectra."""
        values, single, lmax = _check_spectrum_array(dataspec, "spectogrd input")
        if lmax > self.nlat - 1:
            raise ValueError(f"ntrunc too large - can be max of {self.nlat - 1}, got {lmax}")
        plan = self._plan(lmax)
        maps = []
        for spectrum in _coefficient_columns(values, single):
            core = _py_to_core(spectrum, lmax)
            maps.append(plan.synthesis(core[None, :])[0])
        return _map_series(maps, single)

    def getvrtdivspec(
        self, ugrid: np.ndarray, vgrid: np.ndarray, ntrunc: int | None = None
    ) -> tuple[np.ndarray, np.ndarray]:
        """Analyze eastward and northward winds into relative vorticity and divergence."""
        u, single = _as_field_series(ugrid, self.nlat, self.nlon, "ugrid")
        v, v_single = _as_field_series(vgrid, self.nlat, self.nlon, "vgrid")
        if u.shape != v.shape or single != v_single:
            raise ValueError("getvrtdivspec input arrays must be same shape!")
        ntrunc = _check_truncation(ntrunc, self.nlat)
        nalm = (ntrunc + 1) * (ntrunc + 2) // 2
        if ntrunc == 0:
            empty = [np.zeros(1, dtype=np.complex64) for _ in _fields(u, single)]
            return _spectral_series(empty, single), _spectral_series(empty, single).copy()
        plan = self._plan(ntrunc, 1)
        vorticity: list[np.ndarray] = []
        divergence: list[np.ndarray] = []
        for eastward, northward in zip(_fields(u, single), _fields(v, single)):
            components = np.ascontiguousarray(
                np.stack((-self.rsphere * northward, self.rsphere * eastward)),
                dtype=np.float32,
            )
            if self._geometry == "CC" and ntrunc == self.nlat - 1:
                e_b = plan.analysis_projected(components)
            else:
                e_b = plan.analysis(components)
            vrt = np.zeros(nalm, dtype=np.complex64)
            div = np.zeros(nalm, dtype=np.complex64)
            for order in range(ntrunc + 1):
                start = _order_start(ntrunc, order)
                phase = _SQRT_2PI if order % 2 == 0 else -_SQRT_2PI
                for degree in range(max(1, order), ntrunc + 1):
                    index = start + degree - order
                    factor = np.float32(-math.sqrt(degree * (degree + 1)) / (self.rsphere**2 * phase))
                    vrt[index] = factor * e_b[1, index]
                    div[index] = factor * e_b[0, index]
            vorticity.append(vrt)
            divergence.append(div)
        return _spectral_series(vorticity, single), _spectral_series(divergence, single)

    def getuv(self, vrtspec: np.ndarray, divspec: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        """Synthesize eastward and northward winds from vorticity and divergence."""
        vrt, single, lmax = _check_spectrum_array(vrtspec, "vrtspec")
        div, div_single, div_lmax = _check_spectrum_array(divspec, "divspec")
        if vrt.shape != div.shape or single != div_single or lmax != div_lmax:
            raise ValueError("vrtspec, divspec must be same size in getuv!")
        maps_u: list[np.ndarray] = []
        maps_v: list[np.ndarray] = []
        if lmax == 0:
            for _ in _coefficient_columns(vrt, single):
                maps_u.append(np.zeros((self.nlat, self.nlon), dtype=np.float32))
                maps_v.append(np.zeros((self.nlat, self.nlon), dtype=np.float32))
            return _map_series(maps_u, single), _map_series(maps_v, single)
        plan = self._plan(lmax, 1)
        for vrt_column, div_column in zip(
            _coefficient_columns(vrt, single), _coefficient_columns(div, single)
        ):
            e_alm = np.zeros_like(vrt_column)
            b_alm = np.zeros_like(vrt_column)
            for order in range(lmax + 1):
                start = _order_start(lmax, order)
                phase = np.float32(_SQRT_2PI if order % 2 == 0 else -_SQRT_2PI)
                for degree in range(max(1, order), lmax + 1):
                    index = start + degree - order
                    factor = np.float32(-self.rsphere**2 / math.sqrt(degree * (degree + 1))) * phase
                    e_alm[index] = factor * div_column[index]
                    b_alm[index] = factor * vrt_column[index]
            core_maps = plan.synthesis(np.stack((e_alm, b_alm)))
            maps_u.append(core_maps[1] / np.float32(self.rsphere))
            maps_v.append(-core_maps[0] / np.float32(self.rsphere))
        return _map_series(maps_u, single), _map_series(maps_v, single)

    def getpsichi(
        self, ugrid: np.ndarray, vgrid: np.ndarray, ntrunc: int | None = None
    ) -> tuple[np.ndarray, np.ndarray]:
        """Return streamfunction and velocity-potential maps from winds."""
        vrt, div = self.getvrtdivspec(ugrid, vgrid, ntrunc=ntrunc)
        vrt_values, single, _ = _check_spectrum_array(vrt, "vorticity spectrum")
        div_values, _, _ = _check_spectrum_array(div, "divergence spectrum")
        psi_spectrum = _spectral_series(
            [_inverse_laplacian(c, self.rsphere) for c in _coefficient_columns(vrt_values, single)],
            single,
        )
        chi_spectrum = _spectral_series(
            [_inverse_laplacian(c, self.rsphere) for c in _coefficient_columns(div_values, single)],
            single,
        )
        psi = self.spectogrd(psi_spectrum)
        chi = self.spectogrd(chi_spectrum)
        return psi, chi

    def getgrad(self, chispec: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        """Synthesize eastward and northward gradients from scalar coefficients."""
        values, single, _ = _check_spectrum_array(chispec, "chispec")
        divergence = _spectral_series(
            [_laplacian(c, self.rsphere) for c in _coefficient_columns(values, single)], single
        )
        zeros = np.zeros_like(divergence)
        return self.getuv(zeros, divergence)

    def specsmooth(self, datagrid: np.ndarray, smooth: np.ndarray) -> np.ndarray:
        """Apply an isotropic degree-dependent smoothing factor to map fields."""
        data, single = _as_field_series(datagrid, self.nlat, self.nlon, "specsmooth input")
        factors = np.asarray(smooth, dtype=np.float32)
        if factors.ndim != 1 or factors.shape[0] != self.nlat:
            raise ValueError("smooth must be rank 1 and same size as datagrid.shape[0] in specsmooth!")
        spectra = self.grdtospec(data, ntrunc=self.nlat - 1)
        values, spec_single, lmax = _check_spectrum_array(spectra, "spectrum")
        smoothed = []
        for column in _coefficient_columns(values, spec_single):
            result = column.copy()
            for order in range(lmax + 1):
                start = _order_start(lmax, order)
                for degree in range(order, lmax + 1):
                    result[start + degree - order] *= factors[degree]
            smoothed.append(result)
        result = self.spectogrd(_spectral_series(smoothed, single))
        return result[:, :, None] if single else result


def regrid(
    grdin: Spharmt,
    grdout: Spharmt,
    datagrid: np.ndarray,
    ntrunc: int | None = None,
    smooth: np.ndarray | None = None,
) -> np.ndarray:
    """Interpolate map fields between pyspharm latitude-longitude grids."""
    data, single = _as_field_series(datagrid, grdin.nlat, grdin.nlon, "regrid input")
    if ntrunc is None:
        ntrunc = min(grdin.nlat - 1, grdout.nlat - 1)
    ntrunc = _check_truncation(ntrunc, min(grdin.nlat, grdout.nlat))
    factors = None if smooth is None else np.asarray(smooth, dtype=np.float32)
    if factors is not None and (factors.ndim != 1 or factors.shape[0] != grdout.nlat):
        raise ValueError("smooth must be rank 1 size grdout.nlat in regrid!")
    spectra = grdin.grdtospec(data, ntrunc=ntrunc)
    if factors is not None:
        values, spec_single, lmax = _check_spectrum_array(spectra, "spectrum")
        smoothed = []
        for column in _coefficient_columns(values, spec_single):
            result = column.copy()
            for order in range(lmax + 1):
                start = _order_start(lmax, order)
                for degree in range(order, lmax + 1):
                    result[start + degree - order] *= factors[degree]
            smoothed.append(result)
        spectra = _spectral_series(smoothed, single)
    result = grdout.spectogrd(spectra)
    return result[:, :, None] if factors is not None and single else result


def gaussian_lats_wts(nlat: int) -> tuple[np.ndarray, np.ndarray]:
    """Return Gaussian latitudes in degrees north and weights summing to two."""
    nlat = _index(nlat, "nlat")
    if nlat < 1:
        raise ValueError("nlat must be positive")
    return pyspharm_gaussian_lats_wts(nlat)


def getspecindx(ntrunc: int) -> tuple[np.ndarray, np.ndarray]:
    """Return int32 order and degree arrays in pyspharm packed order."""
    ntrunc = _index(ntrunc, "ntrunc")
    if ntrunc < 0:
        raise ValueError("ntrunc must be nonnegative")
    orders = np.repeat(np.arange(ntrunc + 1, dtype=np.int32), np.arange(ntrunc + 1, 0, -1))
    degrees = np.concatenate(
        [np.arange(order, ntrunc + 1, dtype=np.int32) for order in range(ntrunc + 1)]
    )
    return np.squeeze(orders), np.squeeze(degrees)


def getgeodesicpts(m: int) -> tuple[np.ndarray, np.ndarray]:
    """Return geodesic-point latitudes and longitudes in float32 degrees."""
    m = _index(m, "m")
    if m < 1:
        raise ValueError("m must be at least one")
    return pyspharm_getgeodesicpts(m)


def legendre(lat: float, ntrunc: int) -> np.ndarray:
    """Return pyspharm-normalized associated Legendre values at latitude `lat`."""
    ntrunc = _index(ntrunc, "ntrunc")
    if ntrunc < 0:
        raise ValueError("ntrunc must be nonnegative")
    return pyspharm_legendre(np.float32(lat), ntrunc)


def specintrp(lon: float, dataspec: np.ndarray, legfuncs: np.ndarray) -> np.float32:
    """Evaluate a packed scalar spectrum at one longitude and latitude."""
    spectrum = np.ascontiguousarray(dataspec, dtype=np.complex64)
    legendre_values = np.ascontiguousarray(legfuncs, dtype=np.float32)
    return np.float32(pyspharm_specintrp(np.float32(lon), spectrum, legendre_values))
