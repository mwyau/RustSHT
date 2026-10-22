// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use numpy::ndarray::{ArrayD, IxDyn};
use numpy::{
    Complex32, Element, IntoPyArray, PyArrayDyn, PyReadonlyArrayDyn, PyUntypedArrayMethods,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rustsht::{GridKind, GridSpec, HarmonicSpec, ShtError, ShtPlan};
use std::sync::{Mutex, MutexGuard};

type PythonFloat64ArrayPair<'py> = (Bound<'py, PyArrayDyn<f64>>, Bound<'py, PyArrayDyn<f64>>);

#[pyclass(name = "ShtPlan32", module = "rustsht._rustsht")]
struct PythonShtPlan32 {
    plan: ShtPlan<f32>,
    workspace: Mutex<Option<rustsht::ShtWorkspace<f32>>>,
}

#[pymethods]
impl PythonShtPlan32 {
    /// Build a reusable single-precision plan for GL, CC, or F1 rings.
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
        let kind = grid_kind(kind)?;
        let plan = py
            .detach(move || {
                let grid = GridSpec::new(kind, ntheta, nphi, phi0)?;
                let harmonics = HarmonicSpec::new(lmax, mmax, spin)?;
                ShtPlan::<f32>::new(grid, harmonics)
            })
            .map_err(to_python_error)?;
        Ok(Self {
            plan,
            workspace: Mutex::new(None),
        })
    }

    #[getter]
    fn nalm(&self) -> usize {
        self.plan.harmonics().nalm()
    }

    #[getter]
    fn map_shape(&self) -> (usize, usize, usize) {
        (
            self.component_count(),
            self.plan.grid().ntheta(),
            self.plan.grid().nphi(),
        )
    }

    #[getter]
    fn alm_shape(&self) -> (usize, usize) {
        (self.component_count(), self.plan.harmonics().nalm())
    }

    /// Synthesize scalar or spin maps from float32 and complex64 arrays.
    fn synthesis<'py>(
        &self,
        py: Python<'py>,
        alm: PyReadonlyArrayDyn<'py, Complex32>,
    ) -> PyResult<Bound<'py, PyArrayDyn<f32>>> {
        let (ncomp, nalm) = self.alm_shape();
        let coefficients = read_array(&alm, &[ncomp, nalm], "alm")?;
        let plan = &self.plan;
        let workspaces = &self.workspace;
        let output = py
            .detach(move || {
                let mut workspace = lock_workspace(workspaces, plan)?;
                let workspace = workspace.as_mut().expect("workspace was initialized");
                let sample_count = plan.grid().ntheta() * plan.grid().nphi();
                let map_len = sample_count
                    .checked_mul(ncomp)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "pyspharm map output length",
                    })?;
                let mut map = reserve_filled(map_len, 0.0_f32, "pyspharm map output")?;
                run_synthesis(plan, coefficients, &mut map, workspace)?;
                Ok(map)
            })
            .map_err(to_python_error)?;
        let (ncomp, ntheta, nphi) = self.map_shape();
        output_array(py, &[ncomp, ntheta, nphi], output)
    }

    /// Analyze scalar maps or a two-component spin field.
    fn analysis<'py>(
        &self,
        py: Python<'py>,
        map: PyReadonlyArrayDyn<'py, f32>,
    ) -> PyResult<Bound<'py, PyArrayDyn<Complex32>>> {
        self.analyze(py, map, false)
    }

    /// Apply the grid-quadrature projection without an exact-latitude check.
    fn analysis_projected<'py>(
        &self,
        py: Python<'py>,
        map: PyReadonlyArrayDyn<'py, f32>,
    ) -> PyResult<Bound<'py, PyArrayDyn<Complex32>>> {
        self.analyze(py, map, true)
    }

    /// Synthesize the angular derivatives `(d/dtheta, csc(theta)*d/dphi)`.
    fn synthesis_deriv1<'py>(
        &self,
        py: Python<'py>,
        alm: PyReadonlyArrayDyn<'py, Complex32>,
    ) -> PyResult<Bound<'py, PyArrayDyn<f32>>> {
        if self.plan.harmonics().spin() != 0 {
            return Err(PyValueError::new_err(
                "derivative synthesis requires a scalar plan",
            ));
        }
        let nalm = self.plan.harmonics().nalm();
        let coefficients = read_array(&alm, &[1, nalm], "alm")?;
        let plan = &self.plan;
        let workspaces = &self.workspace;
        let output = py
            .detach(move || {
                let mut workspace = lock_workspace(workspaces, plan)?;
                let workspace = workspace.as_mut().expect("workspace was initialized");
                let sample_count = plan.grid().ntheta() * plan.grid().nphi();
                let output_len = sample_count
                    .checked_mul(2)
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "pyspharm derivative output length",
                    })?;
                let mut derivatives =
                    reserve_filled(output_len, 0.0_f32, "pyspharm derivative output")?;
                let (theta, phi) = derivatives.split_at_mut(sample_count);
                plan.synthesis_deriv1_into(coefficients, theta, phi, workspace)?;
                Ok(derivatives)
            })
            .map_err(to_python_error)?;
        output_array(
            py,
            &[2, self.plan.grid().ntheta(), self.plan.grid().nphi()],
            output,
        )
    }
}

impl PythonShtPlan32 {
    fn component_count(&self) -> usize {
        if self.plan.harmonics().spin() == 0 {
            1
        } else {
            2
        }
    }

    fn analyze<'py>(
        &self,
        py: Python<'py>,
        map: PyReadonlyArrayDyn<'py, f32>,
        projected: bool,
    ) -> PyResult<Bound<'py, PyArrayDyn<Complex32>>> {
        let (ncomp, ntheta, nphi) = self.map_shape();
        let samples = read_array(&map, &[ncomp, ntheta, nphi], "map")?;
        let plan = &self.plan;
        let workspaces = &self.workspace;
        let output = py
            .detach(move || {
                let mut workspace = lock_workspace(workspaces, plan)?;
                let workspace = workspace.as_mut().expect("workspace was initialized");
                let nalm = plan.harmonics().nalm();
                let alm_len = ncomp.checked_mul(nalm).ok_or(ShtError::IntegerOverflow {
                    quantity: "pyspharm coefficient output length",
                })?;
                let mut alm =
                    reserve_filled(alm_len, Complex32::new(0.0, 0.0), "pyspharm coefficients")?;
                if plan.harmonics().spin() == 0 {
                    if projected {
                        plan.analysis_projected_into(samples, &mut alm, workspace)?;
                    } else {
                        plan.analysis_into(samples, &mut alm, workspace)?;
                    }
                } else {
                    let sample_count = ntheta * nphi;
                    let (first, second) = samples.split_at(sample_count);
                    let (e_alm, b_alm) = alm.split_at_mut(nalm);
                    if projected {
                        plan.analysis_spin_projected_into(first, second, e_alm, b_alm, workspace)?;
                    } else {
                        plan.analysis_spin_into(first, second, e_alm, b_alm, workspace)?;
                    }
                }
                Ok(alm)
            })
            .map_err(to_python_error)?;
        let (ncomp, nalm) = self.alm_shape();
        output_array(py, &[ncomp, nalm], output)
    }
}

fn run_synthesis(
    plan: &ShtPlan<f32>,
    coefficients: &[Complex32],
    output: &mut [f32],
    workspace: &mut rustsht::ShtWorkspace<f32>,
) -> Result<(), ShtError> {
    if plan.harmonics().spin() == 0 {
        plan.synthesis_into(coefficients, output, workspace)
    } else {
        let nalm = plan.harmonics().nalm();
        let sample_count = plan.grid().ntheta() * plan.grid().nphi();
        let (first, second) = output.split_at_mut(sample_count);
        plan.synthesis_spin_into(
            &coefficients[..nalm],
            &coefficients[nalm..],
            first,
            second,
            workspace,
        )
    }
}

fn lock_workspace<'a>(
    workspaces: &'a Mutex<Option<rustsht::ShtWorkspace<f32>>>,
    plan: &ShtPlan<f32>,
) -> Result<MutexGuard<'a, Option<rustsht::ShtWorkspace<f32>>>, ShtError> {
    let mut workspace = match workspaces.lock() {
        Ok(workspace) => workspace,
        Err(poisoned) => {
            let mut workspace = poisoned.into_inner();
            *workspace = None;
            workspaces.clear_poison();
            workspace
        }
    };
    if workspace.is_none() {
        *workspace = Some(plan.workspace()?);
    }
    Ok(workspace)
}

fn grid_kind(value: &str) -> PyResult<GridKind> {
    match value.to_ascii_uppercase().as_str() {
        "GL" => Ok(GridKind::GaussLegendre),
        "CC" => Ok(GridKind::ClenshawCurtis),
        "F1" => Ok(GridKind::Fejer1),
        _ => Err(PyValueError::new_err(
            "kind must be one of 'GL', 'CC', or 'F1'",
        )),
    }
}

fn read_array<'py, T: Element + Copy>(
    array: &'py PyReadonlyArrayDyn<'py, T>,
    expected_shape: &[usize],
    name: &str,
) -> PyResult<&'py [T]> {
    if array.shape() != expected_shape {
        return Err(PyValueError::new_err(format!(
            "{name} shape must be {expected_shape:?}; got {:?}",
            array.shape()
        )));
    }
    if !array.is_c_contiguous() {
        return Err(PyValueError::new_err(format!(
            "{name} must be C-contiguous"
        )));
    }
    array
        .as_slice()
        .map_err(|_| PyValueError::new_err(format!("{name} must be C-contiguous")))
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

fn reserve_filled<T: Clone>(len: usize, value: T, array: &'static str) -> Result<Vec<T>, ShtError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    result.resize(len, value);
    Ok(result)
}

fn to_python_error(error: ShtError) -> PyErr {
    PyValueError::new_err(error.to_string())
}

#[pyfunction]
#[pyo3(signature = (nlat))]
fn pyspharm_gaussian_lats_wts<'py>(
    py: Python<'py>,
    nlat: usize,
) -> PyResult<PythonFloat64ArrayPair<'py>> {
    let (latitudes, weights) = py
        .detach(move || {
            let grid = GridSpec::new(GridKind::GaussLegendre, nlat, 4, 0.0)?;
            let mut latitudes = Vec::new();
            let mut weights = Vec::new();
            latitudes
                .try_reserve_exact(nlat)
                .map_err(|_| ShtError::AllocationFailed {
                    array: "Gaussian latitudes",
                })?;
            weights
                .try_reserve_exact(nlat)
                .map_err(|_| ShtError::AllocationFailed {
                    array: "Gaussian quadrature weights",
                })?;
            for (theta, &ring_weight) in grid.theta().iter().zip(grid.ring_weights()) {
                latitudes.push(90.0 - theta.to_degrees());
                weights.push(ring_weight / (2.0 * std::f64::consts::PI));
            }
            Ok((latitudes, weights))
        })
        .map_err(to_python_error)?;
    let latitudes = output_array(py, &[latitudes.len()], latitudes)?;
    let weights = output_array(py, &[weights.len()], weights)?;
    Ok((latitudes, weights))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PythonShtPlan32>()?;
    module.add_function(wrap_pyfunction!(pyspharm_gaussian_lats_wts, module)?)?;
    Ok(())
}
