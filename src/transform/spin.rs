// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(feature = "parallel")]
use std::collections::VecDeque;
#[cfg(feature = "parallel")]
use std::sync::Mutex;
#[cfg(feature = "parallel")]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{ShtError, check_len};
use crate::grid::resampling::CCResamplingPlan;
#[cfg(feature = "parallel")]
use crate::grid::resampling::CCResamplingWorkspace;
use crate::legendre::{
    LATITUDE_BLOCK_SIZE, LegendreOperation, MAX_OVERSAMPLED_TABLE_ANALYSIS_LMAX,
    SPIN1_FAST_MIN_LMAX, ShtScalar, longitude_phase, scalar_to_spin1, scalar_to_spin1_with_epsilon,
    scalar_to_spin2, sin_cos_from_f64, spin1_to_scalar, spin1_to_scalar_with_visit,
    spin2_to_scalar,
};
use crate::plan::{ShtPlan, current_execution_thread_count};
#[cfg(feature = "parallel")]
use crate::transform::scheduling::run_dynamic_order_jobs;
use crate::workspace::{RecurrenceWorkspace, ShtWorkspace};

mod analysis;
#[cfg(feature = "parallel")]
mod hemisphere;
mod synthesis;

const MAX_SUPPORTED_SPIN: usize = 4;
const SPIN2_FAST_MIN_LMAX: usize = 500;
const SPIN1_FUSED_ANALYSIS_MIN_LMAX: usize = 255;
const SPIN1_TRANSITION_MIN_LMAX: usize = 191;
const SPIN1_TABLE_SYNTH_MIN_LMAX: usize = 127;
const SPIN1_TABLE_SYNTH_MID_BAND_LMAX: usize = 159;
// Current q1 synthesis measurements retain T127, T159, and T175–T191. T144
// remains a measured regression, so the unmeasured gap stays on recurrence.
const SPIN1_TABLE_SYNTH_HIGH_BAND_MIN_LMAX: usize = 175;
const SPIN1_TABLE_SYNTH_MAX_LMAX: usize = 191;
const SPIN1_TABLE_ANALYSIS_SINGLE_THREAD_MIN_LMAX: usize = 63;
const SPIN1_TABLE_ANALYSIS_SINGLE_THREAD_MAX_LMAX: usize = 224;
const SPIN1_FAST_MIN_SIN_THETA: f64 = 0.0001;
const SPIN1_PAIR_MIN_SIN_THETA: f64 = 0.01;
#[cfg(feature = "parallel")]
const SPIN1_ORDER_MAJOR_TILE: usize = 1;
#[cfg(feature = "parallel")]
const SPIN1_ORDER_MAJOR_ELIGIBILITY_TILE: usize = 8;
const SPIN2_FAST_MIN_SIN_THETA: f64 = 0.01;
#[cfg(feature = "parallel")]
const SPIN1_SYNTH_COEFFICIENT_CACHE_MAX_LMAX: usize = 1023;
#[cfg(feature = "parallel")]
const SPIN1_SYNTH_COEFFICIENT_CACHE_MAX_BYTES: usize = 34 * 1024 * 1024;
#[cfg(feature = "parallel")]
const SPIN1_ORDER_LOCAL_CONVERSION_MIN_BYTES: usize = 4_000_000;
const SPIN1_ORDER_FUSED_LOCAL_PREPARATION_MAX_BYTES: usize = 16_000_000;

#[cfg(all(test, feature = "parallel"))]
static SPIN1_PREPARED_COMPONENT_ORDER_SUCCESSES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "parallel")]
pub(crate) fn spin1_hemisphere_worker_count<T: ShtScalar>(
    plan: &ShtPlan<T>,
    execution_capacity: usize,
) -> Option<usize> {
    let lmax = plan.harmonics.lmax();
    let thread_count = execution_capacity;
    if plan.harmonics.spin() != 1
        || !T::supports_paired_spin()
        || !(319..=511).contains(&lmax)
        || (lmax == 511 && thread_count >= 8)
        || plan.harmonics.mmax() != lmax
        || plan.grid.kind() != crate::grid::GridKind::GaussLegendre
        || plan.grid.ntheta() != lmax + 1
        || !(4..=8).contains(&thread_count)
    {
        return None;
    }
    let hemisphere_ring_count = plan.grid.ntheta() / 2;
    let rings_per_group = hemisphere_ring_count
        .div_ceil(thread_count.saturating_mul(2))
        .max(1);
    let full_group_count = hemisphere_ring_count / rings_per_group;
    let first_group_count = usize::from(!hemisphere_ring_count.is_multiple_of(rings_per_group));
    Some(full_group_count + first_group_count)
}

#[inline]
pub(crate) fn single_threaded_execution() -> bool {
    current_execution_thread_count() == 1
}

#[inline]
fn spin1_synthesis_table_band_is_beneficial(lmax: usize) -> bool {
    lmax == SPIN1_TABLE_SYNTH_MIN_LMAX
        || lmax == SPIN1_TABLE_SYNTH_MID_BAND_LMAX
        || (SPIN1_TABLE_SYNTH_HIGH_BAND_MIN_LMAX..=SPIN1_TABLE_SYNTH_MAX_LMAX).contains(&lmax)
}

fn spin1_synthesis_coefficients_safe_for_table<T: ShtScalar>(
    e_alm: &[Complex<T>],
    b_alm: &[Complex<T>],
) -> bool {
    if !T::supports_precomputed_scalar() {
        return false;
    }

    // Each converted scalar component sums at most three E/B components, with
    // factors whose magnitudes are at most one. Leave a factor-four margin
    // before building a direct-multiply table; the order kernel still checks
    // the converted values before using the table.
    let safe_input_limit = crate::legendre::ScalarLegendreTable::<T>::input_scale_limit()
        / T::from_f64(4.0).expect("four is representable");
    e_alm.iter().chain(b_alm).all(|value| {
        crate::legendre::ScalarLegendreTable::<T>::input_value_is_safe(value.re, safe_input_limit)
            && crate::legendre::ScalarLegendreTable::<T>::input_value_is_safe(
                value.im,
                safe_input_limit,
            )
    })
}

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
    fn allow_spin1_analysis_table(&self) -> bool {
        let lmax = self.harmonics.lmax();
        let thread_count = current_execution_thread_count();
        let oversampled = self.grid.ntheta() > lmax + 1;
        // At over-resolved T159, the one-thread table run has no stable warm
        // advantage and its first use retains roughly 16 MiB of basis values.
        if oversampled
            && (lmax > MAX_OVERSAMPLED_TABLE_ANALYSIS_LMAX
                || self.harmonics.mmax() != lmax
                || (lmax == 159 && thread_count == 1))
        {
            return false;
        }
        // Same-source x86-64 CPU measurements show no repeatable warm
        // advantage for the 34 MiB table at full-mmax T255 with two or more
        // workers; keep the paired recurrence for that memory-heavy case.
        let high_band_full_mmax_multicore_regression =
            self.harmonics.lmax() == 255 && self.harmonics.mmax() == 255 && thread_count >= 2;
        !high_band_full_mmax_multicore_regression
            && (thread_count != 1
                || (SPIN1_TABLE_ANALYSIS_SINGLE_THREAD_MIN_LMAX
                    ..=SPIN1_TABLE_ANALYSIS_SINGLE_THREAD_MAX_LMAX)
                    .contains(&self.harmonics.lmax()))
            && self.legendre_reuse_allows_table(
                LegendreOperation::Analysis,
                self.spin1_scalar_legendre_table
                    .get()
                    .is_some_and(Option::is_some),
            )
    }

    /// Synthesizes two real map components for a spin-1 through spin-4 field.
    ///
    /// `e_alm` and `b_alm` are packed coefficient channels. The output slices
    /// are the two map components in DUCC order, each stored north-to-south
    /// with eastward longitude samples. Both coefficient channels must have
    /// the plan's packed length; their `m=0` values must be real and their
    /// degrees below the spin must be zero.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, an input
    /// coefficient is invalid, a buffer length differs from the plan,
    /// synthesis longitude sampling is insufficient, or the workspace is
    /// incompatible.
    pub fn synthesis_spin_into(
        &self,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        first_component: &mut [T],
        second_component: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.synthesis_spin_core(
            e_alm,
            Some(b_alm),
            first_component,
            second_component,
            spin,
            false,
            workspace,
        )
    }

    /// Synthesizes a spin field with only its first coefficient channel set.
    ///
    /// This is the DUCC `GRAD_ONLY` operation. It uses the plan's spin and
    /// writes both real map components.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, an input
    /// coefficient is invalid, a buffer length differs from the plan,
    /// synthesis longitude sampling is insufficient, or the workspace is
    /// incompatible.
    pub fn synthesis_gradient_into(
        &self,
        e_alm: &[Complex<T>],
        first_component: &mut [T],
        second_component: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.synthesis_spin_core(
            e_alm,
            None,
            first_component,
            second_component,
            spin,
            false,
            workspace,
        )
    }

    /// Synthesizes the angular derivatives of a scalar field.
    ///
    /// The first output is `d/dtheta`; the second is
    /// `(1/sin(theta))*d/dphi`. This is the DUCC `DERIV1` operation and uses
    /// the scalar plan's coefficients with a spin-1 recurrence.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan has nonzero spin, an input coefficient
    /// is invalid, a buffer length differs from the plan, synthesis longitude
    /// sampling is insufficient, or the workspace is incompatible.
    pub fn synthesis_deriv1_into(
        &self,
        alm: &[Complex<T>],
        theta_derivative: &mut [T],
        phi_derivative: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        if self.harmonics.spin() != 0 {
            return Err(ShtError::UnsupportedSpin {
                spin: self.harmonics.spin(),
            });
        }
        if self.harmonics.lmax() == 0 {
            self.harmonics.validate_coefficients(alm)?;
            check_len("scalar coefficient input", self.harmonics.nalm(), alm.len())?;
            check_len(
                "theta derivative output",
                self.sample_count,
                theta_derivative.len(),
            )?;
            check_len(
                "phi derivative output",
                self.sample_count,
                phi_derivative.len(),
            )?;
            self.check_synthesis_longitude()?;
            self.check_workspace(workspace)?;
            theta_derivative.fill(T::zero());
            phi_derivative.fill(T::zero());
            return Ok(());
        }
        self.synthesis_spin_core(
            alm,
            None,
            theta_derivative,
            phi_derivative,
            1,
            true,
            workspace,
        )
    }

    /// Analyzes two real map components into spin-1 through spin-4 E/B
    /// coefficients.
    ///
    /// Exact GL and F1 analysis require `ntheta >= lmax + 1`; exact CC analysis
    /// requires `ntheta >= lmax + 2`. All require
    /// `mmax <= (nphi - 1) / 2`. Each output coefficient slice has the plan's
    /// packed length; degrees below the spin are returned as zero.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, exact-analysis
    /// sampling is insufficient, a buffer length differs from the plan, or
    /// the workspace is incompatible.
    pub fn analysis_spin_into(
        &self,
        first_component: &[T],
        second_component: &[T],
        e_alm: &mut [Complex<T>],
        b_alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.check_analysis_sampling()?;
        self.map_to_spin_coefficients(
            first_component,
            second_component,
            e_alm,
            Some(b_alm),
            spin,
            false,
            Some(self.grid.ring_weights()),
            true,
            true,
            workspace,
        )
    }

    /// Projects two map components onto spin E/B harmonics using grid
    /// quadrature without requiring exact latitude sampling.
    ///
    /// Under-resolved latitude grids produce quadrature and sampling error.
    /// Longitude sampling must still support every requested order. Degrees
    /// below the plan spin are returned as zero.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, longitude
    /// sampling is insufficient, a buffer length differs from the plan, or
    /// the workspace is incompatible.
    pub fn analysis_spin_projected_into(
        &self,
        first_component: &[T],
        second_component: &[T],
        e_alm: &mut [Complex<T>],
        b_alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.check_analysis_longitude()?;
        self.map_to_spin_coefficients(
            first_component,
            second_component,
            e_alm,
            Some(b_alm),
            spin,
            false,
            Some(self.grid.ring_weights()),
            true,
            false,
            workspace,
        )
    }

    /// Applies the adjoint of spin-1 through spin-4 STANDARD synthesis.
    ///
    /// The map dot product is an unweighted pixel sum. The returned E/B
    /// coefficients use the real harmonic inner product, including weight
    /// two for `m > 0`. This operation accepts the even longitude Nyquist
    /// order and has no latitude sampling floor.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, longitude
    /// sampling is insufficient, a buffer length differs from the plan, or
    /// the workspace is incompatible.
    pub fn adjoint_synthesis_spin_into(
        &self,
        first_component: &[T],
        second_component: &[T],
        e_alm: &mut [Complex<T>],
        b_alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.check_synthesis_longitude()?;
        self.map_to_spin_coefficients(
            first_component,
            second_component,
            e_alm,
            Some(b_alm),
            spin,
            false,
            None,
            false,
            false,
            workspace,
        )
    }

    /// Applies the adjoint of spin-1 through spin-4 `GRAD_ONLY` synthesis.
    ///
    /// The two input maps produce only the first (gradient) coefficient
    /// channel. The map dot product is an unweighted pixel sum; the real
    /// harmonic inner product weights `m > 0` by two.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, longitude
    /// sampling is insufficient, a buffer length differs from the plan, or
    /// the workspace is incompatible.
    pub fn adjoint_synthesis_gradient_into(
        &self,
        first_component: &[T],
        second_component: &[T],
        e_alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        self.check_synthesis_longitude()?;
        self.map_to_spin_coefficients(
            first_component,
            second_component,
            e_alm,
            None,
            spin,
            false,
            None,
            false,
            false,
            workspace,
        )
    }

    /// Applies the adjoint of scalar `DERIV1` synthesis.
    ///
    /// The input slices contain the theta and phi derivative map components.
    /// The output is a scalar coefficient channel, with degree zero equal to
    /// zero. The map dot product is an unweighted pixel sum.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan is not scalar, longitude sampling is
    /// insufficient, a buffer length differs from the plan, or the workspace
    /// is incompatible.
    pub fn adjoint_synthesis_deriv1_into(
        &self,
        theta_derivative: &[T],
        phi_derivative: &[T],
        alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        if self.harmonics.spin() != 0 {
            return Err(ShtError::UnsupportedSpin {
                spin: self.harmonics.spin(),
            });
        }
        self.check_synthesis_longitude()?;
        check_len(
            "theta derivative input",
            self.sample_count,
            theta_derivative.len(),
        )?;
        check_len(
            "phi derivative input",
            self.sample_count,
            phi_derivative.len(),
        )?;
        check_len(
            "scalar coefficient output",
            self.harmonics.nalm(),
            alm.len(),
        )?;
        self.check_workspace(workspace)?;
        if self.harmonics.lmax() == 0 {
            alm.fill(Complex::<T>::new(T::zero(), T::zero()));
            return Ok(());
        }
        self.map_to_spin_coefficients(
            theta_derivative,
            phi_derivative,
            alm,
            None,
            1,
            true,
            None,
            false,
            false,
            workspace,
        )
    }

    /// Applies the adjoint of exact spin-1 through spin-4 analysis.
    ///
    /// The output maps are STANDARD spin synthesis. GL analysis scales each
    /// ring by `ring_weight / nphi`; CC analysis uses the adjoint of its
    /// prepared-grid resampling path.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan spin is outside `1..=4`, exact-analysis
    /// sampling is insufficient, a coefficient or output buffer is invalid,
    /// or the workspace is incompatible.
    pub fn adjoint_analysis_spin_into(
        &self,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        first_component: &mut [T],
        second_component: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.supported_spin()?;
        self.check_analysis_sampling()?;
        self.harmonics.validate_coefficients(e_alm)?;
        self.harmonics.validate_coefficients(b_alm)?;
        check_len("E coefficient input", self.harmonics.nalm(), e_alm.len())?;
        check_len("B coefficient input", self.harmonics.nalm(), b_alm.len())?;
        check_len(
            "first spin map output",
            self.sample_count,
            first_component.len(),
        )?;
        check_len(
            "second spin map output",
            self.sample_count,
            second_component.len(),
        )?;
        self.check_workspace(workspace)?;
        if self.cc_resampling.is_some() {
            self.adjoint_analysis_spin_cc_into(
                e_alm,
                b_alm,
                first_component,
                second_component,
                workspace,
            )
        } else {
            self.synthesis_spin_into(e_alm, b_alm, first_component, second_component, workspace)?;
            self.apply_analysis_pixel_weights(first_component);
            self.apply_analysis_pixel_weights(second_component);
            Ok(())
        }
    }

    fn adjoint_analysis_spin_cc_into(
        &self,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        first_component: &mut [T],
        second_component: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let spin = self.supported_spin()?;
        let resampling = self
            .cc_resampling
            .as_ref()
            .expect("CC adjoint analysis requires the prepared CC operator");
        let prepared_ntheta = resampling.prepared_ntheta();
        for (ring, &theta) in resampling.prepared_theta().iter().enumerate() {
            (workspace.theta_sin[ring], workspace.theta_cos[ring]) = sin_cos_from_f64::<T>(theta);
            workspace.ring_order_limits[ring] = T::polar_order_limit(
                self.harmonics.lmax(),
                spin,
                workspace.theta_sin[ring],
                workspace.theta_cos[ring],
            );
        }
        workspace
            .prepared_spectra
            .fill(Complex::<T>::new(T::zero(), T::zero()));
        workspace
            .prepared_spin_spectra
            .fill(Complex::<T>::new(T::zero(), T::zero()));
        self.synthesize_spin_ring_group(
            e_alm,
            Some(b_alm),
            spin,
            false,
            false,
            false,
            None,
            None,
            1,
            self.fft.spectrum_len(),
            self.grid.phi0(),
            0,
            prepared_ntheta,
            &mut workspace.prepared_spectra,
            &mut workspace.prepared_spin_spectra,
            &workspace.theta_sin[..prepared_ntheta],
            &workspace.theta_cos[..prepared_ntheta],
            &workspace.ring_order_limits[..prepared_ntheta],
            pulp::Arch::new(),
            &mut workspace.recurrence_workers[0],
        )?;
        resampling.restore_original(
            &workspace.prepared_spectra,
            &mut workspace.spectra,
            self.fft.spectrum_len(),
            self.harmonics.mmax(),
            spin,
            &mut workspace.cc_profile,
            &mut workspace.cc_fft_scratch,
        )?;
        resampling.restore_original(
            &workspace.prepared_spin_spectra,
            &mut workspace.spin_spectra,
            self.fft.spectrum_len(),
            self.harmonics.mmax(),
            spin,
            &mut workspace.cc_profile,
            &mut workspace.cc_fft_scratch,
        )?;
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        for ring in 0..self.grid.ntheta() {
            let start = ring * spectrum_len;
            workspace.spectra[start].im = T::zero();
            workspace.spin_spectra[start].im = T::zero();
            self.fft.inverse(
                &workspace.spectra[start..start + spectrum_len],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            let output_start = ring * nphi;
            first_component[output_start..output_start + nphi].copy_from_slice(&workspace.row);
            self.fft.inverse(
                &workspace.spin_spectra[start..start + spectrum_len],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            second_component[output_start..output_start + nphi].copy_from_slice(&workspace.row);
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The map-to-coefficient driver covers weighted analysis and the four synthesis adjoints."
    )]
    fn map_to_spin_coefficients(
        &self,
        first_component: &[T],
        second_component: &[T],
        e_alm: &mut [Complex<T>],
        mut b_alm: Option<&mut [Complex<T>]>,
        recurrence_spin: usize,
        derivative_norm: bool,
        ring_weights: Option<&[f64]>,
        normalize_fourier: bool,
        resample_cc: bool,
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        if !matches!(recurrence_spin, 1..=MAX_SUPPORTED_SPIN) {
            return Err(ShtError::UnsupportedSpin {
                spin: recurrence_spin,
            });
        }
        if !derivative_norm && self.harmonics.spin() != recurrence_spin {
            return Err(ShtError::UnsupportedSpin {
                spin: self.harmonics.spin(),
            });
        }
        check_len(
            "first spin map input",
            self.sample_count,
            first_component.len(),
        )?;
        check_len(
            "second spin map input",
            self.sample_count,
            second_component.len(),
        )?;
        check_len("E coefficient output", self.harmonics.nalm(), e_alm.len())?;
        if let Some(b_alm) = b_alm.as_deref() {
            check_len("B coefficient output", self.harmonics.nalm(), b_alm.len())?;
        }
        self.check_workspace(workspace)?;
        #[cfg(all(test, feature = "parallel"))]
        let mut phase_started = std::time::Instant::now();
        #[cfg(all(test, feature = "parallel"))]
        let phase_probe = workspace.transform_phase_probe();
        if recurrence_spin == 1
            && !derivative_norm
            && ring_weights.is_some()
            && self.grid.kind() == crate::grid::GridKind::GaussLegendre
            && self.allow_spin1_analysis_table()
            && (self.spin1_scalar_legendre_table.get().is_some()
                || (crate::legendre::ScalarLegendreTable::<T>::real_input_values_are_safe(
                    first_component,
                ) && crate::legendre::ScalarLegendreTable::<T>::real_input_values_are_safe(
                    second_component,
                )))
        {
            self.spin1_scalar_legendre_table.get_or_init(|| {
                crate::legendre::ScalarLegendreTable::build_if_beneficial(
                    &self.grid,
                    &self.harmonics,
                    &self.sectoral_norms,
                )
            });
        }
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "strategy selection and table setup",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }

        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        #[cfg(feature = "parallel")]
        let parallel_fft = self.row_fft_parallelism_supported()
            && workspace
                .execution_capacity
                .min(rayon::current_num_threads())
                > 1;
        #[cfg(not(feature = "parallel"))]
        let parallel_fft = false;
        if parallel_fft {
            self.forward_analysis_rows(
                first_component,
                &mut workspace.spectra,
                spectrum_len,
                &mut workspace.fft,
                #[cfg(feature = "parallel")]
                &mut workspace.fft_worker_workspaces,
                #[cfg(feature = "parallel")]
                workspace.execution_capacity,
            )?;
            self.forward_analysis_rows(
                second_component,
                &mut workspace.spin_spectra,
                spectrum_len,
                &mut workspace.fft,
                #[cfg(feature = "parallel")]
                &mut workspace.fft_worker_workspaces,
                #[cfg(feature = "parallel")]
                workspace.execution_capacity,
            )?;
        } else {
            for ring in 0..ntheta {
                let input_start = ring * nphi;
                let spectrum_start = ring * spectrum_len;
                let end = spectrum_start + spectrum_len;
                self.fft.forward(
                    &first_component[input_start..input_start + nphi],
                    &mut workspace.spectra[spectrum_start..end],
                    &mut workspace.fft,
                )?;
                self.fft.forward(
                    &second_component[input_start..input_start + nphi],
                    &mut workspace.spin_spectra[spectrum_start..end],
                    &mut workspace.fft,
                )?;
            }
        }
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(&phase_probe, "forward FFT", phase_started);
            phase_started = std::time::Instant::now();
        }

        e_alm.fill(Complex::<T>::new(T::zero(), T::zero()));
        if let Some(b_alm) = b_alm.as_deref_mut() {
            b_alm.fill(Complex::<T>::new(T::zero(), T::zero()));
        }
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "coefficient initialization",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        let (
            q_spectra,
            u_spectra,
            analysis_ntheta,
            analysis_ring_weights,
            analysis_normalize_fourier,
            analysis_spectra_preweighted,
        ) = if let Some(resampling) = self
            .cc_resampling
            .as_ref()
            .filter(|_| ring_weights.is_some() && resample_cc)
        {
            #[cfg(all(test, feature = "parallel"))]
            let meridional_started = std::time::Instant::now();
            for value in &mut workspace.spectra {
                *value /= T::from_usize(nphi).expect("longitude count converts");
            }
            for value in &mut workspace.spin_spectra {
                *value /= T::from_usize(nphi).expect("longitude count converts");
            }
            let _first_resample_parallel = self.resample_spin_analysis_spectrum(
                resampling,
                &workspace.spectra,
                &mut workspace.prepared_spectra,
                spectrum_len,
                recurrence_spin,
                &mut workspace.cc_profile,
                &mut workspace.cc_fft_scratch,
                #[cfg(feature = "parallel")]
                &mut workspace.cc_resampling_order_tile,
                #[cfg(feature = "parallel")]
                &mut workspace.cc_resampling_workers,
                #[cfg(feature = "parallel")]
                workspace.execution_capacity,
            )?;
            let _second_resample_parallel = self.resample_spin_analysis_spectrum(
                resampling,
                &workspace.spin_spectra,
                &mut workspace.prepared_spin_spectra,
                spectrum_len,
                recurrence_spin,
                &mut workspace.cc_profile,
                &mut workspace.cc_fft_scratch,
                #[cfg(feature = "parallel")]
                &mut workspace.cc_resampling_order_tile,
                #[cfg(feature = "parallel")]
                &mut workspace.cc_resampling_workers,
                #[cfg(feature = "parallel")]
                workspace.execution_capacity,
            )?;
            #[cfg(all(test, feature = "parallel"))]
            {
                crate::workspace::record_transform_phase(
                    &phase_probe,
                    if _first_resample_parallel || _second_resample_parallel {
                        "meridional resampling (parallel orders)"
                    } else {
                        "meridional resampling (serial orders)"
                    },
                    meridional_started,
                );
                phase_started = std::time::Instant::now();
            }
            for (ring, &theta) in resampling.prepared_theta().iter().enumerate() {
                (workspace.theta_sin[ring], workspace.theta_cos[ring]) =
                    sin_cos_from_f64::<T>(theta);
                workspace.ring_order_limits[ring] = T::polar_order_limit(
                    self.harmonics.lmax(),
                    recurrence_spin,
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
            }
            (
                workspace.prepared_spectra.as_slice(),
                workspace.prepared_spin_spectra.as_slice(),
                resampling.prepared_ntheta(),
                None,
                false,
                true,
            )
        } else {
            self.prepare_spin_latitudes(workspace, recurrence_spin);
            (
                workspace.spectra.as_slice(),
                workspace.spin_spectra.as_slice(),
                ntheta,
                ring_weights,
                normalize_fourier,
                false,
            )
        };
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "latitude geometry setup",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        let theta_sin = &workspace.theta_sin[..analysis_ntheta];
        let theta_cos = &workspace.theta_cos[..analysis_ntheta];
        let ring_order_limits = &workspace.ring_order_limits[..analysis_ntheta];
        let analysis_ring_scales = if T::supports_paired_spin()
            && recurrence_spin == 1
            && !derivative_norm
            && (analysis_ring_weights.is_some() || analysis_spectra_preweighted)
        {
            let longitude_count = T::from_usize(nphi).expect("longitude count converts");
            for ring in 0..analysis_ntheta {
                let sine = theta_sin[ring];
                let mut scale = if sine == T::zero() {
                    T::zero()
                } else {
                    let weight = analysis_ring_weights.map_or(T::one(), |weights| {
                        T::from_f64(weights[ring]).expect("ring weight is representable")
                    });
                    weight / sine
                };
                if analysis_normalize_fourier {
                    scale /= longitude_count;
                }
                workspace.analysis_ring_scales[ring] = scale;
            }
            Some(&workspace.analysis_ring_scales[..analysis_ntheta])
        } else {
            None
        };
        let arch = pulp::Arch::new();
        let spin1_range_geometry = if analysis_ring_scales.is_some() {
            let minimum_sine =
                T::from_f64(SPIN1_FAST_MIN_SIN_THETA).expect("spin fast sine limit converts");
            let start = theta_sin
                .iter()
                .position(|&sine| sine >= minimum_sine)
                .unwrap_or(theta_sin.len());
            let end = theta_sin[start..]
                .iter()
                .position(|&sine| sine < minimum_sine)
                .map_or(theta_sin.len(), |offset| start + offset);
            (start < end).then_some((analysis_ntheta, start, end))
        } else {
            None
        };
        if let Some(geometry) = spin1_range_geometry {
            let same_plan = workspace
                .spin1_analysis_pair_range_cache_id
                .as_ref()
                .is_some_and(|cache_id| {
                    std::sync::Arc::ptr_eq(cache_id, &self.analysis_pair_range_cache_id)
                });
            if !same_plan
                || workspace.spin1_analysis_pair_range_arch != Some(std::mem::discriminant(&arch))
                || workspace.spin1_analysis_pair_range_geometry != Some(geometry)
            {
                for cached in &workspace.spin1_analysis_pair_range_safe {
                    cached.store(false, Ordering::Relaxed);
                }
                workspace.spin1_analysis_pair_range_cache_id =
                    Some(std::sync::Arc::clone(&self.analysis_pair_range_cache_id));
                workspace.spin1_analysis_pair_range_arch = Some(std::mem::discriminant(&arch));
                workspace.spin1_analysis_pair_range_geometry = Some(geometry);
            }
        }
        let order_count = self.harmonics.mmax() + 1;

        #[cfg(feature = "parallel")]
        {
            let worker_count = rayon::current_num_threads()
                .min(workspace.execution_capacity)
                .min(workspace.recurrence_workers.len())
                .min(order_count);
            let use_parallel = rayon::current_num_threads() > 1 && worker_count > 1;
            if use_parallel {
                self.analyze_spin_order_jobs_dynamic(
                    q_spectra,
                    u_spectra,
                    nphi,
                    spectrum_len,
                    self.grid.phi0(),
                    analysis_ring_weights,
                    analysis_normalize_fourier,
                    analysis_ring_scales,
                    analysis_spectra_preweighted,
                    recurrence_spin,
                    derivative_norm,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    &workspace.spin1_analysis_pair_range_safe,
                    arch,
                    e_alm,
                    b_alm.as_deref_mut(),
                    &mut workspace.recurrence_workers[..worker_count],
                )?;
                #[cfg(all(test, feature = "parallel"))]
                {
                    crate::workspace::record_transform_phase(
                        &phase_probe,
                        "Legendre recurrence and accumulation",
                        phase_started,
                    );
                    phase_started = std::time::Instant::now();
                }
            } else {
                #[cfg(all(test, feature = "parallel"))]
                {
                    phase_started = std::time::Instant::now();
                }
                self.analyze_spin_order_group(
                    q_spectra,
                    u_spectra,
                    nphi,
                    spectrum_len,
                    self.grid.phi0(),
                    analysis_ring_weights,
                    analysis_normalize_fourier,
                    analysis_ring_scales,
                    analysis_spectra_preweighted,
                    recurrence_spin,
                    derivative_norm,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    &workspace.spin1_analysis_pair_range_safe,
                    arch,
                    0,
                    order_count,
                    e_alm,
                    b_alm.as_deref_mut(),
                    &mut workspace.recurrence_workers[0],
                )?;
                #[cfg(all(test, feature = "parallel"))]
                {
                    crate::workspace::record_transform_phase(
                        &phase_probe,
                        "Legendre recurrence and accumulation",
                        phase_started,
                    );
                    phase_started = std::time::Instant::now();
                }
            }
        }
        #[cfg(not(feature = "parallel"))]
        self.analyze_spin_order_group(
            q_spectra,
            u_spectra,
            nphi,
            spectrum_len,
            self.grid.phi0(),
            analysis_ring_weights,
            analysis_normalize_fourier,
            analysis_ring_scales,
            analysis_spectra_preweighted,
            recurrence_spin,
            derivative_norm,
            theta_sin,
            theta_cos,
            ring_order_limits,
            &workspace.spin1_analysis_pair_range_safe,
            arch,
            0,
            order_count,
            e_alm,
            b_alm.as_deref_mut(),
            &mut workspace.recurrence_workers[0],
        )?;

        for degree in recurrence_spin..=self.harmonics.lmax() {
            let index = self
                .harmonics
                .index(degree, 0)
                .expect("order zero contains every degree");
            e_alm[index].im = T::zero();
            if let Some(b_alm) = b_alm.as_deref_mut() {
                b_alm[index].im = T::zero();
            }
        }
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "coefficient cleanup",
            phase_started,
        );
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "CC preparation receives both legacy serial scratch and bounded parallel order scratch."
    )]
    fn resample_spin_analysis_spectrum(
        &self,
        resampling: &CCResamplingPlan<T>,
        input: &[Complex<T>],
        output: &mut [Complex<T>],
        spectrum_len: usize,
        spin: usize,
        profile: &mut [Complex<T>],
        fft_scratch: &mut [Complex<T>],
        #[cfg(feature = "parallel")] order_tile: &mut [Complex<T>],
        #[cfg(feature = "parallel")] worker_workspaces: &mut [CCResamplingWorkspace<T>],
        #[cfg(feature = "parallel")] execution_capacity: usize,
    ) -> Result<bool, ShtError> {
        #[cfg(feature = "parallel")]
        {
            let order_count = self.harmonics.mmax() + 1;
            let active_workers = current_execution_thread_count()
                .min(execution_capacity)
                .min(worker_workspaces.len())
                .min(order_count);
            let estimated_active_pairs = order_count.saturating_mul(resampling.prepared_ntheta());
            if self.harmonics.spin() == 1
                && spin == 1
                && T::supports_precomputed_scalar()
                && matches!(
                    self.grid.kind(),
                    crate::grid::GridKind::ClenshawCurtis | crate::grid::GridKind::Fejer1
                )
                && resampling.needs_resampling()
                && estimated_active_pairs
                    >= crate::grid::resampling::CC_RESAMPLING_PARALLEL_MIN_ORDER_RING_PAIRS
                && active_workers > 1
            {
                resampling.to_prepared_parallel(
                    input,
                    output,
                    spectrum_len,
                    self.harmonics.mmax(),
                    spin,
                    order_tile,
                    worker_workspaces,
                    active_workers,
                )?;
                return Ok(true);
            }
        }

        resampling.to_prepared(
            input,
            output,
            spectrum_len,
            self.harmonics.mmax(),
            spin,
            profile,
            fft_scratch,
        )?;
        Ok(false)
    }

    fn prepare_spin_latitudes(&self, workspace: &mut ShtWorkspace<T>, spin: usize) {
        for (ring, &theta) in self.grid.theta().iter().enumerate() {
            (workspace.theta_sin[ring], workspace.theta_cos[ring]) = sin_cos_from_f64::<T>(theta);
            workspace.ring_order_limits[ring] = T::polar_order_limit(
                self.harmonics.lmax(),
                spin,
                workspace.theta_sin[ring],
                workspace.theta_cos[ring],
            );
        }
        if spin == 1 {
            // Grid constructors form north/south colatitudes as exact
            // reflections. Evaluate each reflected pair from its northern
            // member so independent libm rounding cannot split its order
            // cutoff or recurrence geometry.
            for north in 0..self.grid.ntheta() / 2 {
                let south = self.grid.ntheta() - 1 - north;
                workspace.theta_sin[south] = workspace.theta_sin[north];
                workspace.theta_cos[south] = -workspace.theta_cos[north];
                workspace.ring_order_limits[south] = workspace.ring_order_limits[north];
            }
        }
    }

    fn supported_spin(&self) -> Result<usize, ShtError> {
        match self.harmonics.spin() {
            spin @ 1..=MAX_SUPPORTED_SPIN => Ok(spin),
            spin => Err(ShtError::UnsupportedSpin { spin }),
        }
    }

    fn check_synthesis_longitude(&self) -> Result<(), ShtError> {
        if self.harmonics.mmax() <= self.grid.nphi() / 2 {
            Ok(())
        } else {
            Err(ShtError::SynthesisLongitudeUndersampled {
                mmax: self.harmonics.mmax(),
                nphi: self.grid.nphi(),
            })
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Each prepared component reuses the same reflected batch and worker recurrence scratch."
)]
fn synthesize_spin1_prepared_scalar_component_batch<T: ShtScalar>(
    arch: pulp::Arch,
    m: usize,
    scalar_lmax: usize,
    sectoral_norm: T,
    active_lanes: usize,
    vector_lanes: usize,
    second_component: bool,
    recurrence: &mut RecurrenceWorkspace<T>,
) -> bool {
    let RecurrenceWorkspace {
        previous,
        current,
        epsilon,
        exponents,
        spin_alpha,
        spin_coefficient_a,
        spin_coefficient_b,
        spin_previous_plus,
        scalar_order_correction,
        compact_theta_sin,
        compact_theta_cos_squared,
        analysis_same_real,
        analysis_same_imag,
        analysis_opposite_real,
        analysis_opposite_imag,
        fast_spin_first,
        fast_spin_second,
        fast_spin_pair_first,
        fast_spin_pair_second,
        ..
    } = recurrence;
    if active_lanes > spin_previous_plus.len() {
        return false;
    }
    let sin_theta = &spin_previous_plus[..active_lanes];
    let (coefficient_even, coefficient_odd) = if second_component {
        (
            &mut fast_spin_pair_first[..],
            &mut fast_spin_pair_second[..],
        )
    } else {
        (&mut fast_spin_first[..], &mut fast_spin_second[..])
    };
    let mut scratch = crate::legendre::scalar_ordered::ScalarOrderedScratch {
        epsilon: &mut epsilon[..],
        alpha: &mut spin_alpha[..],
        coefficient_a: &mut spin_coefficient_a[..],
        coefficient_b: &mut spin_coefficient_b[..],
        coefficient_even,
        coefficient_odd,
        packed_sin: &mut compact_theta_sin[..],
        packed_csq: &mut compact_theta_cos_squared[..],
        previous: &mut previous[..],
        current: &mut current[..],
        scale: &mut exponents[..],
        corfac: &mut scalar_order_correction[..],
        sum_even_re: &mut analysis_same_real[..],
        sum_even_im: &mut analysis_same_imag[..],
        sum_odd_re: &mut analysis_opposite_real[..],
        sum_odd_im: &mut analysis_opposite_imag[..],
    };
    T::synthesize_prepared_scalar_component_batch_dispatched(
        arch,
        m,
        scalar_lmax,
        sectoral_norm,
        sin_theta,
        vector_lanes,
        &mut scratch,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The polar-lane split uses one block's geometry, spectra and worker scratch."
)]
#[cold]
#[inline(never)]
#[cfg(test)]
fn try_synthesize_spin1_transition_safe_lanes<T: ShtScalar>(
    arch: pulp::Arch,
    m: usize,
    scalar_lmax: usize,
    spectrum_len: usize,
    sectoral_norm: T,
    theta_sin: &[T],
    theta_cos: &[T],
    ring_order_limits: &[usize],
    q_spectra: &mut [Complex<T>],
    u_spectra: &mut [Complex<T>],
    recurrence: &mut RecurrenceWorkspace<T>,
) -> Result<Option<usize>, ShtError> {
    try_synthesize_spin1_transition_safe_lanes_to_order(
        arch,
        m,
        scalar_lmax,
        spectrum_len,
        m,
        sectoral_norm,
        theta_sin,
        theta_cos,
        ring_order_limits,
        q_spectra,
        u_spectra,
        recurrence,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "The order-major polar split targets a compact worker-local order spectrum."
)]
#[cold]
#[inline(never)]
fn try_synthesize_spin1_transition_safe_lanes_to_order<T: ShtScalar>(
    arch: pulp::Arch,
    m: usize,
    scalar_lmax: usize,
    spectrum_len: usize,
    output_order: usize,
    sectoral_norm: T,
    theta_sin: &[T],
    theta_cos: &[T],
    ring_order_limits: &[usize],
    q_spectra: &mut [Complex<T>],
    u_spectra: &mut [Complex<T>],
    recurrence: &mut RecurrenceWorkspace<T>,
) -> Result<Option<usize>, ShtError> {
    debug_assert_eq!(theta_sin.len(), theta_cos.len());
    debug_assert_eq!(theta_sin.len(), ring_order_limits.len());
    debug_assert_eq!(q_spectra.len(), theta_sin.len() * spectrum_len);
    debug_assert_eq!(u_spectra.len(), q_spectra.len());

    let transition_min_sine = T::from_f64(0.01).expect("paired transition sine limit converts");
    let mut transition_len = 0;
    let mut polar_len = 0;
    for ring in 0..theta_sin.len() {
        if m > ring_order_limits[ring] {
            continue;
        }
        if theta_sin[ring] >= transition_min_sine {
            recurrence.spin_previous_plus[transition_len] = theta_sin[ring];
            recurrence.spin_current_plus[transition_len] = theta_cos[ring];
            recurrence.spin_exponents_plus[transition_len] =
                i64::try_from(ring).expect("local ring index fits i64");
            transition_len += 1;
        } else {
            recurrence.spin_previous_minus[polar_len] = theta_sin[ring];
            recurrence.spin_current_minus[polar_len] = theta_cos[ring];
            recurrence.spin_exponents_minus[polar_len] =
                i64::try_from(ring).expect("local ring index fits i64");
            polar_len += 1;
        }
    }
    if transition_len == 0 {
        return Ok(None);
    }

    #[cfg(all(test, feature = "parallel"))]
    if recurrence.spin1_profile_enabled {
        let profile_bin = if m <= 255 {
            0
        } else if m <= 767 {
            1
        } else {
            2
        };
        let profile = &mut recurrence.spin1_profile_route_bins[profile_bin];
        profile.polar_split_attempts += 1;
        profile.polar_split_safe_lane_pair_work +=
            (transition_len * (scalar_lmax - m + 2) / 2) as u128;
    }

    let transitioned = T::try_synthesize_paired_scalar_transition_dispatched(
        arch,
        m,
        scalar_lmax,
        &recurrence.spin_previous_plus[..transition_len],
        &recurrence.spin_current_plus[..transition_len],
        sectoral_norm,
        &recurrence.epsilon,
        &recurrence.spin_alpha,
        &recurrence.spin_coefficient_a,
        &recurrence.spin_coefficient_b,
        &recurrence.fast_scalar_first[m..=scalar_lmax],
        &recurrence.fast_scalar_second[m..=scalar_lmax],
        &mut recurrence.previous[..transition_len],
        &mut recurrence.current[..transition_len],
        &mut recurrence.next[..transition_len],
        &mut recurrence.exponents[..transition_len],
        &mut recurrence.analysis_same_real[..transition_len],
        &mut recurrence.analysis_same_imag[..transition_len],
        &mut recurrence.analysis_opposite_real[..transition_len],
        &mut recurrence.analysis_opposite_imag[..transition_len],
    )?;
    if !transitioned {
        #[cfg(all(test, feature = "parallel"))]
        if recurrence.spin1_profile_enabled {
            let profile_bin = if m <= 255 {
                0
            } else if m <= 767 {
                1
            } else {
                2
            };
            recurrence.spin1_profile_route_bins[profile_bin].polar_split_declines += 1;
        }
        return Ok(None);
    }
    for lane in 0..transition_len {
        let inverse_sine = recurrence.spin_previous_plus[lane].recip();
        if ![
            recurrence.analysis_same_real[lane] * inverse_sine,
            recurrence.analysis_same_imag[lane] * inverse_sine,
            recurrence.analysis_opposite_real[lane] * inverse_sine,
            recurrence.analysis_opposite_imag[lane] * inverse_sine,
        ]
        .into_iter()
        .all(|value| value.is_finite())
        {
            #[cfg(all(test, feature = "parallel"))]
            if recurrence.spin1_profile_enabled {
                let profile_bin = if m <= 255 {
                    0
                } else if m <= 767 {
                    1
                } else {
                    2
                };
                recurrence.spin1_profile_route_bins[profile_bin].polar_split_declines += 1;
            }
            return Ok(None);
        }
    }
    for lane in 0..transition_len {
        let ring = usize::try_from(recurrence.spin_exponents_plus[lane])
            .expect("saved ring index is nonnegative");
        let spectrum_index = ring * spectrum_len + output_order;
        let inverse_sine = recurrence.spin_previous_plus[lane].recip();
        q_spectra[spectrum_index] += Complex::new(
            recurrence.analysis_same_real[lane] * inverse_sine,
            recurrence.analysis_same_imag[lane] * inverse_sine,
        );
        u_spectra[spectrum_index] += Complex::new(
            recurrence.analysis_opposite_real[lane] * inverse_sine,
            recurrence.analysis_opposite_imag[lane] * inverse_sine,
        );
    }
    #[cfg(all(test, feature = "parallel"))]
    if recurrence.spin1_profile_enabled {
        let profile_bin = if m <= 255 {
            0
        } else if m <= 767 {
            1
        } else {
            2
        };
        recurrence.spin1_profile_route_bins[profile_bin].polar_split_successes += 1;
    }
    Ok(Some(polar_len))
}

#[cfg(test)]
mod tests;
