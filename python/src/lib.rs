// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use numpy::ndarray::{ArrayD, IxDyn};
use numpy::{Complex64, Element, IntoPyArray, PyArrayDyn, PyReadonlyArrayDyn};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rustsht::{GridKind, GridSpec, HarmonicSpec, ShtError, ShtPlan};

/// Reusable spherical transform plan for GL, CC and F1 grids.
///
/// Standard scalar arrays have shapes `(1, nalm)` and `(1, ntheta, nphi)`.
/// Standard spin arrays have shapes `(2, nalm)` and `(2, ntheta, nphi)`,
/// where the coefficient components are E and B. Methods accept C-contiguous
/// `complex128` or `float64` arrays and return newly allocated NumPy arrays.
/// Input storage is borrowed during each numerical call; do not modify it
/// through another alias until the method returns.
/// Map rings run north to south; longitude samples run eastward from `phi0`
/// in radians.
#[pyclass(name = "ShtPlan", module = "rustsht._rustsht")]
struct PythonShtPlan {
    plan: ShtPlan,
}

#[pymethods]
impl PythonShtPlan {
    /// Build a reusable plan for `GL`, `CC`, or `F1` latitude rings.
    ///
    /// Inputs and outputs use component-first NumPy arrays. The plan accepts
    /// contiguous `float64` maps and `complex128` packed coefficients.
    /// `phi0` gives the longitude of the first sample on each ring, in radians.
    #[new]
    #[pyo3(signature = (kind, ntheta, nphi, lmax, mmax, spin=0, phi0=0.0))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        kind: &str,
        ntheta: usize,
        nphi: usize,
        lmax: usize,
        mmax: usize,
        spin: usize,
        phi0: f64,
    ) -> PyResult<Self> {
        let kind = grid_kind(kind, "kind")?;
        let plan = py
            .detach(move || {
                let grid = GridSpec::new(kind, ntheta, nphi, phi0)?;
                let harmonics = HarmonicSpec::new(lmax, mmax, spin)?;
                ShtPlan::new(grid, harmonics)
            })
            .map_err(to_python_error)?;
        Ok(Self { plan })
    }

    /// Number of packed nonnegative-order harmonic coefficients.
    #[getter]
    fn nalm(&self) -> usize {
        self.plan.harmonics().nalm()
    }

    /// Expected standard coefficient shape `(ncomp, nalm)`.
    #[getter]
    fn alm_shape(&self) -> (usize, usize) {
        (self.component_count(), self.plan.harmonics().nalm())
    }

    /// Expected standard map shape `(ncomp, ntheta, nphi)`.
    #[getter]
    fn map_shape(&self) -> (usize, usize, usize) {
        (
            self.component_count(),
            self.plan.grid().ntheta(),
            self.plan.grid().nphi(),
        )
    }

    /// Synthesize a scalar field or a spin E/B pair.
    fn synthesis<'py>(
        &self,
        py: Python<'py>,
        alm: PyReadonlyArrayDyn<'py, Complex64>,
    ) -> PyResult<Bound<'py, PyArrayDyn<f64>>> {
        let alm_shape = self.alm_shape();
        let coefficients = read_array(&alm, &[alm_shape.0, alm_shape.1], "alm")?;
        let plan = &self.plan;
        let output = py
            .detach(move || {
                let mut workspace = plan.workspace()?;
                let sample_count = plan.grid().ntheta() * plan.grid().nphi();
                if plan.harmonics().spin() == 0 {
                    let mut map = vec![0.0; sample_count];
                    plan.synthesis_into(coefficients, &mut map, &mut workspace)?;
                    Ok(map)
                } else {
                    let nalm = plan.harmonics().nalm();
                    let mut first = vec![0.0; sample_count];
                    let mut second = vec![0.0; sample_count];
                    plan.synthesis_spin_into(
                        &coefficients[..nalm],
                        &coefficients[nalm..],
                        &mut first,
                        &mut second,
                        &mut workspace,
                    )?;
                    first.extend(second);
                    Ok(first)
                }
            })
            .map_err(to_python_error)?;
        let (ncomp, ntheta, nphi) = self.map_shape();
        output_array(py, &[ncomp, ntheta, nphi], output)
    }

    /// Analyze a scalar map or a spin map pair into packed coefficients.
    fn analysis<'py>(
        &self,
        py: Python<'py>,
        map: PyReadonlyArrayDyn<'py, f64>,
    ) -> PyResult<Bound<'py, PyArrayDyn<Complex64>>> {
        let map_shape = self.map_shape();
        let samples = read_array(&map, &[map_shape.0, map_shape.1, map_shape.2], "map")?;
        let plan = &self.plan;
        let output = py
            .detach(move || {
                let mut workspace = plan.workspace()?;
                let nalm = plan.harmonics().nalm();
                if plan.harmonics().spin() == 0 {
                    let mut alm = vec![Complex64::new(0.0, 0.0); nalm];
                    plan.analysis_into(samples, &mut alm, &mut workspace)?;
                    Ok(alm)
                } else {
                    let sample_count = plan.grid().ntheta() * plan.grid().nphi();
                    let (first, second) = samples.split_at(sample_count);
                    let mut e_alm = vec![Complex64::new(0.0, 0.0); nalm];
                    let mut b_alm = vec![Complex64::new(0.0, 0.0); nalm];
                    plan.analysis_spin_into(first, second, &mut e_alm, &mut b_alm, &mut workspace)?;
                    e_alm.extend(b_alm);
                    Ok(e_alm)
                }
            })
            .map_err(to_python_error)?;
        output_array(
            py,
            &[self.component_count(), self.plan.harmonics().nalm()],
            output,
        )
    }

    /// Apply the adjoint of standard scalar or spin synthesis.
    fn adjoint_synthesis<'py>(
        &self,
        py: Python<'py>,
        map: PyReadonlyArrayDyn<'py, f64>,
    ) -> PyResult<Bound<'py, PyArrayDyn<Complex64>>> {
        let map_shape = self.map_shape();
        let samples = read_array(&map, &[map_shape.0, map_shape.1, map_shape.2], "map")?;
        let plan = &self.plan;
        let output = py
            .detach(move || {
                let mut workspace = plan.workspace()?;
                let nalm = plan.harmonics().nalm();
                if plan.harmonics().spin() == 0 {
                    let mut alm = vec![Complex64::new(0.0, 0.0); nalm];
                    plan.adjoint_synthesis_into(samples, &mut alm, &mut workspace)?;
                    Ok(alm)
                } else {
                    let sample_count = plan.grid().ntheta() * plan.grid().nphi();
                    let (first, second) = samples.split_at(sample_count);
                    let mut e_alm = vec![Complex64::new(0.0, 0.0); nalm];
                    let mut b_alm = vec![Complex64::new(0.0, 0.0); nalm];
                    plan.adjoint_synthesis_spin_into(
                        first,
                        second,
                        &mut e_alm,
                        &mut b_alm,
                        &mut workspace,
                    )?;
                    e_alm.extend(b_alm);
                    Ok(e_alm)
                }
            })
            .map_err(to_python_error)?;
        output_array(
            py,
            &[self.component_count(), self.plan.harmonics().nalm()],
            output,
        )
    }

    /// Apply the adjoint of exact scalar or spin analysis.
    fn adjoint_analysis<'py>(
        &self,
        py: Python<'py>,
        alm: PyReadonlyArrayDyn<'py, Complex64>,
    ) -> PyResult<Bound<'py, PyArrayDyn<f64>>> {
        let alm_shape = self.alm_shape();
        let coefficients = read_array(&alm, &[alm_shape.0, alm_shape.1], "alm")?;
        let plan = &self.plan;
        let output = py
            .detach(move || {
                let mut workspace = plan.workspace()?;
                let sample_count = plan.grid().ntheta() * plan.grid().nphi();
                if plan.harmonics().spin() == 0 {
                    let mut map = vec![0.0; sample_count];
                    plan.adjoint_analysis_into(coefficients, &mut map, &mut workspace)?;
                    Ok(map)
                } else {
                    let nalm = plan.harmonics().nalm();
                    let mut first = vec![0.0; sample_count];
                    let mut second = vec![0.0; sample_count];
                    plan.adjoint_analysis_spin_into(
                        &coefficients[..nalm],
                        &coefficients[nalm..],
                        &mut first,
                        &mut second,
                        &mut workspace,
                    )?;
                    first.extend(second);
                    Ok(first)
                }
            })
            .map_err(to_python_error)?;
        let (ncomp, ntheta, nphi) = self.map_shape();
        output_array(py, &[ncomp, ntheta, nphi], output)
    }

    /// Synthesize the gradient-only spin field from its E coefficient channel.
    ///
    /// The input shape is `(1, nalm)` and the output shape is
    /// `(2, ntheta, nphi)`.
    fn synthesis_gradient<'py>(
        &self,
        py: Python<'py>,
        e_alm: PyReadonlyArrayDyn<'py, Complex64>,
    ) -> PyResult<Bound<'py, PyArrayDyn<f64>>> {
        self.require_gradient_plan()?;
        let e_alm = read_array(&e_alm, &[1, self.plan.harmonics().nalm()], "e_alm")?;
        let plan = &self.plan;
        let output = py
            .detach(move || {
                let mut workspace = plan.workspace()?;
                let sample_count = plan.grid().ntheta() * plan.grid().nphi();
                let mut first = vec![0.0; sample_count];
                let mut second = vec![0.0; sample_count];
                plan.synthesis_gradient_into(e_alm, &mut first, &mut second, &mut workspace)?;
                first.extend(second);
                Ok(first)
            })
            .map_err(to_python_error)?;
        let (ncomp, ntheta, nphi) = self.map_shape();
        output_array(py, &[ncomp, ntheta, nphi], output)
    }

    /// Apply the adjoint of gradient-only synthesis to two map components.
    ///
    /// The input shape is `(2, ntheta, nphi)` and the output shape is
    /// `(1, nalm)`.
    fn adjoint_synthesis_gradient<'py>(
        &self,
        py: Python<'py>,
        map: PyReadonlyArrayDyn<'py, f64>,
    ) -> PyResult<Bound<'py, PyArrayDyn<Complex64>>> {
        self.require_gradient_plan()?;
        let map_shape = self.map_shape();
        let samples = read_array(&map, &[map_shape.0, map_shape.1, map_shape.2], "map")?;
        let plan = &self.plan;
        let output = py
            .detach(move || {
                let mut workspace = plan.workspace()?;
                let sample_count = plan.grid().ntheta() * plan.grid().nphi();
                let (first, second) = samples.split_at(sample_count);
                let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
                plan.adjoint_synthesis_gradient_into(first, second, &mut e_alm, &mut workspace)?;
                Ok(e_alm)
            })
            .map_err(to_python_error)?;
        output_array(py, &[1, self.plan.harmonics().nalm()], output)
    }

    /// Synthesize two maps containing `(d/dtheta, csc(theta)*d/dphi)`.
    ///
    /// The output shape is `(2, ntheta, nphi)`; the first axis separates the
    /// colatitude and longitude derivatives.
    fn synthesis_deriv1<'py>(
        &self,
        py: Python<'py>,
        alm: PyReadonlyArrayDyn<'py, Complex64>,
    ) -> PyResult<Bound<'py, PyArrayDyn<f64>>> {
        self.require_scalar_plan()?;
        let alm = read_array(&alm, &[1, self.plan.harmonics().nalm()], "alm")?;
        let plan = &self.plan;
        let output = py
            .detach(move || {
                let mut workspace = plan.workspace()?;
                let sample_count = plan.grid().ntheta() * plan.grid().nphi();
                let mut theta = vec![0.0; sample_count];
                let mut phi = vec![0.0; sample_count];
                plan.synthesis_deriv1_into(alm, &mut theta, &mut phi, &mut workspace)?;
                theta.extend(phi);
                Ok(theta)
            })
            .map_err(to_python_error)?;
        output_array(
            py,
            &[2, self.plan.grid().ntheta(), self.plan.grid().nphi()],
            output,
        )
    }

    /// Apply the adjoint of scalar angular-derivative synthesis.
    ///
    /// The input shape is `(2, ntheta, nphi)` and the output shape is
    /// `(1, nalm)`.
    fn adjoint_synthesis_deriv1<'py>(
        &self,
        py: Python<'py>,
        map: PyReadonlyArrayDyn<'py, f64>,
    ) -> PyResult<Bound<'py, PyArrayDyn<Complex64>>> {
        self.require_scalar_plan()?;
        let samples = read_array(
            &map,
            &[2, self.plan.grid().ntheta(), self.plan.grid().nphi()],
            "map",
        )?;
        let plan = &self.plan;
        let output = py
            .detach(move || {
                let mut workspace = plan.workspace()?;
                let sample_count = plan.grid().ntheta() * plan.grid().nphi();
                let (theta, phi) = samples.split_at(sample_count);
                let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
                plan.adjoint_synthesis_deriv1_into(theta, phi, &mut alm, &mut workspace)?;
                Ok(alm)
            })
            .map_err(to_python_error)?;
        output_array(py, &[1, self.plan.harmonics().nalm()], output)
    }
}

impl PythonShtPlan {
    fn component_count(&self) -> usize {
        if self.plan.harmonics().spin() == 0 {
            1
        } else {
            2
        }
    }

    fn require_gradient_plan(&self) -> PyResult<()> {
        if self.plan.harmonics().spin() == 0 {
            Err(PyValueError::new_err(
                "gradient synthesis requires a plan with spin 1 through 4",
            ))
        } else {
            Ok(())
        }
    }

    fn require_scalar_plan(&self) -> PyResult<()> {
        if self.plan.harmonics().spin() != 0 {
            Err(PyValueError::new_err(
                "derivative synthesis requires a scalar plan",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy)]
enum SynthesisMode {
    Standard,
    GradientOnly,
    Derivative1,
}

impl SynthesisMode {
    fn parse(value: &str, spin: usize) -> PyResult<Self> {
        match value.to_ascii_uppercase().as_str() {
            "STANDARD" => Ok(Self::Standard),
            "GRAD_ONLY" => Ok(Self::GradientOnly),
            "DERIV1" if spin == 1 => Ok(Self::Derivative1),
            "DERIV1" => Err(PyValueError::new_err("DERIV1 mode requires spin=1")),
            _ => Err(PyValueError::new_err(
                "mode must be one of 'STANDARD', 'GRAD_ONLY', or 'DERIV1'",
            )),
        }
    }

    fn plan_spin(self, spin: usize) -> usize {
        if matches!(self, Self::Derivative1) {
            0
        } else {
            spin
        }
    }
}

/// Synthesize a scalar or spin field from packed coefficients.
///
/// `alm` uses RustSHT's order-major layout. STANDARD output has shape
/// `(1, ntheta, nphi)` for scalar plans and `(2, ntheta, nphi)` for spin
/// plans. GRAD_ONLY and DERIV1 output two map components. The input is
/// borrowed while the numerical operation runs; do not mutate aliased input
/// storage until this function returns.
#[pyfunction]
#[pyo3(signature = (alm, spin, lmax, geometry, ntheta, nphi, mmax=None, phi0=0.0, mode="STANDARD"))]
#[allow(clippy::too_many_arguments)]
fn synthesis_2d<'py>(
    py: Python<'py>,
    alm: PyReadonlyArrayDyn<'py, Complex64>,
    spin: usize,
    lmax: usize,
    geometry: &str,
    ntheta: usize,
    nphi: usize,
    mmax: Option<usize>,
    phi0: f64,
    mode: &str,
) -> PyResult<Bound<'py, PyArrayDyn<f64>>> {
    let mode = SynthesisMode::parse(mode, spin)?;
    let plan = PythonShtPlan::new(
        py,
        geometry,
        ntheta,
        nphi,
        lmax,
        mmax.unwrap_or(lmax),
        mode.plan_spin(spin),
        phi0,
    )?;
    match mode {
        SynthesisMode::Standard => plan.synthesis(py, alm),
        SynthesisMode::GradientOnly => plan.synthesis_gradient(py, alm),
        SynthesisMode::Derivative1 => plan.synthesis_deriv1(py, alm),
    }
}

/// Analyze a scalar map or spin map pair into packed coefficients.
///
/// `map` uses component-first order and RustSHT's order-major coefficient
/// layout. `ntheta` and `nphi` are read from the map shape. The input is
/// borrowed while the numerical operation runs; do not mutate aliased input
/// storage until this function returns.
#[pyfunction]
#[pyo3(signature = (map, spin, lmax, geometry, mmax=None, phi0=0.0))]
fn analysis_2d<'py>(
    py: Python<'py>,
    map: PyReadonlyArrayDyn<'py, f64>,
    spin: usize,
    lmax: usize,
    geometry: &str,
    mmax: Option<usize>,
    phi0: f64,
) -> PyResult<Bound<'py, PyArrayDyn<Complex64>>> {
    let (ntheta, nphi) = map_dimensions(&map, "map")?;
    let plan = PythonShtPlan::new(
        py,
        geometry,
        ntheta,
        nphi,
        lmax,
        mmax.unwrap_or(lmax),
        spin,
        phi0,
    )?;
    plan.analysis(py, map)
}

/// Apply the adjoint of STANDARD, GRAD_ONLY, or DERIV1 synthesis.
///
/// `map` uses component-first order. The returned coefficients use
/// RustSHT's order-major layout. The input is borrowed while the numerical
/// operation runs; do not mutate aliased input storage until this function
/// returns.
#[pyfunction]
#[pyo3(signature = (map, spin, lmax, geometry, mmax=None, phi0=0.0, mode="STANDARD"))]
#[allow(clippy::too_many_arguments)]
fn adjoint_synthesis_2d<'py>(
    py: Python<'py>,
    map: PyReadonlyArrayDyn<'py, f64>,
    spin: usize,
    lmax: usize,
    geometry: &str,
    mmax: Option<usize>,
    phi0: f64,
    mode: &str,
) -> PyResult<Bound<'py, PyArrayDyn<Complex64>>> {
    let (ntheta, nphi) = map_dimensions(&map, "map")?;
    let mode = SynthesisMode::parse(mode, spin)?;
    let plan = PythonShtPlan::new(
        py,
        geometry,
        ntheta,
        nphi,
        lmax,
        mmax.unwrap_or(lmax),
        mode.plan_spin(spin),
        phi0,
    )?;
    match mode {
        SynthesisMode::Standard => plan.adjoint_synthesis(py, map),
        SynthesisMode::GradientOnly => plan.adjoint_synthesis_gradient(py, map),
        SynthesisMode::Derivative1 => plan.adjoint_synthesis_deriv1(py, map),
    }
}

/// Apply the adjoint of exact STANDARD analysis to packed coefficients.
///
/// `alm` uses RustSHT's order-major layout. The output map uses
/// component-first order. The input is borrowed while the numerical
/// operation runs; do not mutate aliased input storage until this function
/// returns.
#[pyfunction]
#[pyo3(signature = (alm, spin, lmax, geometry, ntheta, nphi, mmax=None, phi0=0.0))]
#[allow(clippy::too_many_arguments)]
fn adjoint_analysis_2d<'py>(
    py: Python<'py>,
    alm: PyReadonlyArrayDyn<'py, Complex64>,
    spin: usize,
    lmax: usize,
    geometry: &str,
    ntheta: usize,
    nphi: usize,
    mmax: Option<usize>,
    phi0: f64,
) -> PyResult<Bound<'py, PyArrayDyn<f64>>> {
    let plan = PythonShtPlan::new(
        py,
        geometry,
        ntheta,
        nphi,
        lmax,
        mmax.unwrap_or(lmax),
        spin,
        phi0,
    )?;
    plan.adjoint_analysis(py, alm)
}

/// Return the `ntheta` ring weights for a GL, CC, or F1 grid.
///
/// The weights sum to `4*pi`; divide by `nphi` for per-pixel quadrature
/// weights on a complete longitude ring.
#[pyfunction]
#[pyo3(signature = (geometry, ntheta))]
fn get_gridweights<'py>(
    py: Python<'py>,
    geometry: &str,
    ntheta: usize,
) -> PyResult<Bound<'py, PyArrayDyn<f64>>> {
    let kind = grid_kind(geometry, "geometry")?;
    let weights = py
        .detach(move || {
            let grid = GridSpec::new(kind, ntheta, 1, 0.0)?;
            Ok(grid.ring_weights().to_vec())
        })
        .map_err(to_python_error)?;
    output_array(py, &[ntheta], weights)
}

fn map_dimensions<T: Element>(
    map: &PyReadonlyArrayDyn<'_, T>,
    name: &str,
) -> PyResult<(usize, usize)> {
    let array = map.as_array();
    let shape = array.shape();
    if shape.len() != 3 {
        return Err(PyValueError::new_err(format!(
            "{name} must have shape (ncomp, ntheta, nphi); got {shape:?}"
        )));
    }
    Ok((shape[1], shape[2]))
}

fn grid_kind(value: &str, name: &str) -> PyResult<GridKind> {
    match value.to_ascii_uppercase().as_str() {
        "GL" => Ok(GridKind::GaussLegendre),
        "CC" => Ok(GridKind::ClenshawCurtis),
        "F1" => Ok(GridKind::Fejer1),
        _ => Err(PyValueError::new_err(format!(
            "{name} must be one of 'GL', 'CC', or 'F1'"
        ))),
    }
}

fn read_array<'py, T: Element + Copy>(
    array: &'py PyReadonlyArrayDyn<'py, T>,
    expected_shape: &[usize],
    name: &str,
) -> PyResult<&'py [T]> {
    let actual_shape = array.as_array().shape().to_vec();
    if actual_shape != expected_shape {
        return Err(PyValueError::new_err(format!(
            "{name} shape must be {expected_shape:?}; got {actual_shape:?}"
        )));
    }
    if !array.as_array().is_standard_layout() {
        return Err(PyValueError::new_err(format!(
            "{name} must be C-contiguous; noncontiguous inputs are not supported"
        )));
    }
    array.as_slice().map_err(|_| {
        PyValueError::new_err(format!(
            "{name} must be C-contiguous; noncontiguous inputs are not supported"
        ))
    })
}

fn output_array<'py, T: Element>(
    py: Python<'py>,
    shape: &[usize],
    values: Vec<T>,
) -> PyResult<Bound<'py, PyArrayDyn<T>>> {
    let array = ArrayD::from_shape_vec(IxDyn(shape), values)
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(array.into_pyarray(py))
}

fn to_python_error(error: ShtError) -> PyErr {
    PyValueError::new_err(error.to_string())
}

#[pymodule]
fn _rustsht(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PythonShtPlan>()?;
    module.add_function(wrap_pyfunction!(synthesis_2d, module)?)?;
    module.add_function(wrap_pyfunction!(analysis_2d, module)?)?;
    module.add_function(wrap_pyfunction!(adjoint_synthesis_2d, module)?)?;
    module.add_function(wrap_pyfunction!(adjoint_analysis_2d, module)?)?;
    module.add_function(wrap_pyfunction!(get_gridweights, module)?)?;
    Ok(())
}
