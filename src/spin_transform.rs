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
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{ShtError, check_len};
use crate::legendre::{
    LATITUDE_BLOCK_SIZE, LegendreOperation, MAX_OVERSAMPLED_TABLE_ANALYSIS_LMAX,
    SPIN1_FAST_MIN_LMAX, ShtScalar, longitude_phase, scalar_to_spin1, scalar_to_spin2,
    sin_cos_from_f64, spin1_to_scalar, spin2_to_scalar,
};
use crate::meridional::CCResamplingPlan;
#[cfg(feature = "parallel")]
use crate::meridional::CCResamplingWorkspace;
use crate::plan::{ShtPlan, current_execution_thread_count};
use crate::workspace::{RecurrenceWorkspace, ShtWorkspace};

#[cfg(feature = "parallel")]
mod hemisphere;

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
const SPIN1_PARALLEL_MERIDIONAL_RESAMPLING_MIN_ORDER_RING_PAIRS: usize = 65_536;
const SPIN2_FAST_MIN_SIN_THETA: f64 = 0.01;
#[cfg(feature = "parallel")]
const SPIN1_SYNTH_COEFFICIENT_CACHE_MAX_LMAX: usize = 720;
#[cfg(feature = "parallel")]
const SPIN1_SYNTH_COEFFICIENT_CACHE_MAX_BYTES: usize = 17 * 1024 * 1024;

#[cfg(feature = "parallel")]
pub(crate) fn spin1_hemisphere_worker_count<T: ShtScalar>(
    plan: &ShtPlan<T>,
    execution_capacity: usize,
) -> Option<usize> {
    let lmax = plan.harmonics.lmax();
    let thread_count = execution_capacity;
    if plan.harmonics.spin() != 1
        || !T::supports_paired_scalar()
        || !(319..=511).contains(&lmax)
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
            .filter(|_| ring_weights.is_some())
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
            if T::supports_paired_scalar()
                && recurrence_spin == 1
                && !derivative_norm
                && self.grid.kind() == crate::grid::GridKind::GaussLegendre
                && let Some(ring_weights) = ring_weights
            {
                let longitude_count = T::from_usize(nphi).expect("longitude count converts");
                for (ring, &weight) in ring_weights.iter().enumerate() {
                    let mut scale = T::from_f64(weight).expect("ring weight is representable")
                        / workspace.theta_sin[ring];
                    if normalize_fourier {
                        scale /= longitude_count;
                    }
                    workspace.analysis_ring_scales[ring] = scale;
                }
            }
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
        let analysis_ring_scales = if T::supports_paired_scalar()
            && recurrence_spin == 1
            && !derivative_norm
            && self.grid.kind() == crate::grid::GridKind::GaussLegendre
            && analysis_ring_weights.is_some()
        {
            Some(&workspace.analysis_ring_scales[..analysis_ntheta])
        } else {
            None
        };
        let theta_sin = &workspace.theta_sin[..analysis_ntheta];
        let theta_cos = &workspace.theta_cos[..analysis_ntheta];
        let ring_order_limits = &workspace.ring_order_limits[..analysis_ntheta];
        let arch = pulp::Arch::new();
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
                    >= SPIN1_PARALLEL_MERIDIONAL_RESAMPLING_MIN_ORDER_RING_PAIRS
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

    #[expect(
        clippy::too_many_arguments,
        reason = "The shared STANDARD, GRAD_ONLY and DERIV1 driver receives two channels, outputs and recurrence mode."
    )]
    fn synthesis_spin_core(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        first_component: &mut [T],
        second_component: &mut [T],
        recurrence_spin: usize,
        derivative_norm: bool,
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.synthesis_spin_core_with_hemisphere(
            e_alm,
            b_alm,
            first_component,
            second_component,
            recurrence_spin,
            derivative_norm,
            workspace,
            true,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The shared transform core receives channels, outputs, recurrence mode and strategy control."
    )]
    fn synthesis_spin_core_with_hemisphere(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        first_component: &mut [T],
        second_component: &mut [T],
        recurrence_spin: usize,
        derivative_norm: bool,
        workspace: &mut ShtWorkspace<T>,
        allow_hemisphere: bool,
    ) -> Result<(), ShtError> {
        #[cfg(not(feature = "parallel"))]
        let _ = allow_hemisphere;

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
        self.harmonics.validate_coefficients(e_alm)?;
        if let Some(b_alm) = b_alm {
            self.harmonics.validate_coefficients(b_alm)?;
        }
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
        self.check_synthesis_longitude()?;
        self.check_workspace(workspace)?;

        #[cfg(feature = "parallel")]
        if allow_hemisphere
            && recurrence_spin == 1
            && !derivative_norm
            && let Some(b_alm) = b_alm
            && hemisphere::is_beneficial(
                self,
                current_execution_thread_count().min(workspace.execution_capacity),
            )
        {
            return hemisphere::synthesize(
                self,
                e_alm,
                b_alm,
                first_component,
                second_component,
                workspace,
            );
        }

        #[cfg(all(test, feature = "parallel"))]
        let phase_probe = workspace.transform_phase_probe();
        #[cfg(all(test, feature = "parallel"))]
        let mut phase_started = std::time::Instant::now();

        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        let phi0 = self.grid.phi0();
        let allow_fast_scalar = !derivative_norm
            || (recurrence_spin == 1
                && self.harmonics.spin() == 0
                && T::supports_paired_scalar()
                && self.harmonics.lmax() >= SPIN1_FAST_MIN_LMAX);
        let can_use_spin1_table = spin1_synthesis_table_band_is_beneficial(self.harmonics.lmax())
            && recurrence_spin == 1
            && self.harmonics.spin() == 1
            && b_alm.is_some_and(|b_coefficients| {
                self.spin1_scalar_legendre_table
                    .get()
                    .is_some_and(Option::is_some)
                    || spin1_synthesis_coefficients_safe_for_table(e_alm, b_coefficients)
            })
            && !derivative_norm
            && self.grid.kind() == crate::grid::GridKind::GaussLegendre
            && self.grid.ntheta() == self.harmonics.lmax() + 1
            && single_threaded_execution()
            && self.legendre_reuse_allows_table(
                LegendreOperation::Synthesis,
                self.spin1_scalar_legendre_table
                    .get()
                    .is_some_and(Option::is_some),
            )
            && self
                .grid
                .theta()
                .iter()
                .all(|&theta| theta.sin() >= SPIN1_PAIR_MIN_SIN_THETA);
        if can_use_spin1_table {
            self.spin1_scalar_legendre_table.get_or_init(|| {
                crate::legendre::ScalarLegendreTable::build_if_beneficial(
                    &self.grid,
                    &self.harmonics,
                    &self.sectoral_norms,
                )
            });
        }
        self.prepare_spin_latitudes(workspace, recurrence_spin);
        workspace
            .spectra
            .fill(Complex::<T>::new(T::zero(), T::zero()));
        workspace
            .spin_spectra
            .fill(Complex::<T>::new(T::zero(), T::zero()));
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "latitude and spin-spectrum setup",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        let arch = pulp::Arch::new();

        #[cfg(feature = "parallel")]
        {
            let execution_threads =
                current_execution_thread_count().min(workspace.execution_capacity);
            let f1_synthesis_group_size =
                self.spin1_f1_synthesis_ring_group_size(execution_threads);
            let group_size = if !derivative_norm && b_alm.is_some() {
                self.fine_synthesis_ring_group_size(execution_threads)
                    .or(f1_synthesis_group_size)
                    .map_or(workspace.ring_group_size, |size| {
                        size.min(workspace.ring_group_size)
                    })
            } else {
                workspace.ring_group_size
            };
            let group_ring_count = group_size.min(ntheta);
            let group_count = ntheta.div_ceil(group_ring_count);
            let use_parallel = execution_threads > 1 && group_count > 1;
            if use_parallel {
                let use_spin1_coefficient_cache = recurrence_spin == 1
                    && self.harmonics.spin() == 1
                    && b_alm.is_some()
                    && !derivative_norm
                    && self.grid.kind() == crate::grid::GridKind::GaussLegendre
                    && allow_fast_scalar
                    && self.harmonics.lmax() >= SPIN1_FAST_MIN_LMAX
                    && self.harmonics.lmax() <= SPIN1_SYNTH_COEFFICIENT_CACHE_MAX_LMAX
                    && T::supports_paired_scalar()
                    && execution_threads >= 8
                    && workspace.ring_group_size < ntheta;
                let use_dynamic_spin1_group_queue = use_spin1_coefficient_cache
                    || (recurrence_spin == 1
                        && !derivative_norm
                        && b_alm.is_some()
                        && f1_synthesis_group_size.is_some());
                let required_recurrence_workers = if use_dynamic_spin1_group_queue {
                    group_count.min(execution_threads)
                } else {
                    group_count
                };
                debug_assert!(required_recurrence_workers <= workspace.recurrence_workers.len());
                #[cfg(all(test, feature = "parallel"))]
                let coefficient_prep_started = std::time::Instant::now();
                let spin1_coefficients_ready = if use_spin1_coefficient_cache {
                    self.prepare_spin1_scalar_coefficients(
                        e_alm,
                        b_alm.expect("the cache requires both spin-1 channels"),
                        &mut workspace.spin1_scalar_coefficients,
                        &mut workspace.spin1_scalar_coefficient_finite_orders,
                        execution_threads,
                    )
                } else {
                    false
                };
                let spin1_coefficients = spin1_coefficients_ready
                    .then_some(workspace.spin1_scalar_coefficients.as_slice());
                let spin1_coefficient_finite_orders = spin1_coefficients_ready
                    .then_some(workspace.spin1_scalar_coefficient_finite_orders.as_slice());
                #[cfg(all(test, feature = "parallel"))]
                {
                    crate::workspace::record_transform_phase(
                        &phase_probe,
                        "spin-1 scalar coefficient cache preparation",
                        coefficient_prep_started,
                    );
                    phase_started = std::time::Instant::now();
                }
                let group_spectrum_len = group_ring_count.checked_mul(spectrum_len).ok_or(
                    ShtError::IntegerOverflow {
                        quantity: "spin Fourier spectrum group size",
                    },
                )?;
                if use_dynamic_spin1_group_queue {
                    // Spin-1 synthesis groups have strongly varying work near
                    // polar cutoffs. Queue one group at a time to balance the
                    // work, while limiting active workers to the capacity
                    // used to prepare this workspace.
                    let group_queue = Mutex::new(
                        workspace
                            .spectra
                            .chunks_mut(group_spectrum_len)
                            .zip(workspace.spin_spectra.chunks_mut(group_spectrum_len))
                            .enumerate(),
                    );
                    let worker_count = group_count.min(execution_threads);
                    workspace.recurrence_workers[..worker_count]
                        .par_iter_mut()
                        .try_for_each(|recurrence| {
                            #[cfg(all(test, feature = "parallel"))]
                            if recurrence.spin1_profile_enabled {
                                recurrence.spin1_profile_group_worker_id =
                                    rayon::current_thread_index().unwrap_or(usize::MAX);
                            }
                            loop {
                                let group = group_queue
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                                    .next();
                                let Some((group_index, (q_spectra, u_spectra))) = group else {
                                    break;
                                };
                                let ring_start = group_index * group_ring_count;
                                let ring_end =
                                    (ring_start + q_spectra.len() / spectrum_len).min(ntheta);
                                #[cfg(all(test, feature = "parallel"))]
                                let group_started = recurrence
                                    .spin1_profile_enabled
                                    .then(std::time::Instant::now);
                                let result = self.synthesize_spin_ring_group(
                                    e_alm,
                                    b_alm,
                                    recurrence_spin,
                                    derivative_norm,
                                    allow_fast_scalar,
                                    can_use_spin1_table,
                                    spin1_coefficients,
                                    spin1_coefficient_finite_orders,
                                    nphi,
                                    spectrum_len,
                                    phi0,
                                    ring_start,
                                    ring_end,
                                    q_spectra,
                                    u_spectra,
                                    &workspace.theta_sin[ring_start..ring_end],
                                    &workspace.theta_cos[ring_start..ring_end],
                                    &workspace.ring_order_limits,
                                    arch,
                                    recurrence,
                                );
                                #[cfg(all(test, feature = "parallel"))]
                                if let Some(group_started) = group_started {
                                    recurrence.spin1_profile_group_ns +=
                                        group_started.elapsed().as_nanos();
                                }
                                result?;
                            }
                            Ok(())
                        })?;
                } else {
                    workspace
                        .spectra
                        .par_chunks_mut(group_spectrum_len)
                        .zip(workspace.spin_spectra.par_chunks_mut(group_spectrum_len))
                        .zip(workspace.recurrence_workers[..group_count].par_iter_mut())
                        .enumerate()
                        .with_min_len(group_count.div_ceil(execution_threads))
                        .try_for_each(|(group_index, ((q_spectra, u_spectra), recurrence))| {
                            let ring_start = group_index * group_ring_count;
                            let ring_end =
                                (ring_start + q_spectra.len() / spectrum_len).min(ntheta);
                            self.synthesize_spin_ring_group(
                                e_alm,
                                b_alm,
                                recurrence_spin,
                                derivative_norm,
                                allow_fast_scalar,
                                can_use_spin1_table,
                                spin1_coefficients,
                                spin1_coefficient_finite_orders,
                                nphi,
                                spectrum_len,
                                phi0,
                                ring_start,
                                ring_end,
                                q_spectra,
                                u_spectra,
                                &workspace.theta_sin[ring_start..ring_end],
                                &workspace.theta_cos[ring_start..ring_end],
                                &workspace.ring_order_limits,
                                arch,
                                recurrence,
                            )
                        })?;
                }
            } else {
                self.synthesize_spin_ring_group(
                    e_alm,
                    b_alm,
                    recurrence_spin,
                    derivative_norm,
                    allow_fast_scalar,
                    can_use_spin1_table,
                    None,
                    None,
                    nphi,
                    spectrum_len,
                    phi0,
                    0,
                    ntheta,
                    &mut workspace.spectra,
                    &mut workspace.spin_spectra,
                    &workspace.theta_sin[..ntheta],
                    &workspace.theta_cos[..ntheta],
                    &workspace.ring_order_limits[..ntheta],
                    arch,
                    &mut workspace.recurrence_workers[0],
                )?;
            }
        }
        #[cfg(not(feature = "parallel"))]
        self.synthesize_spin_ring_group(
            e_alm,
            b_alm,
            recurrence_spin,
            derivative_norm,
            allow_fast_scalar,
            can_use_spin1_table,
            None,
            None,
            nphi,
            spectrum_len,
            phi0,
            0,
            ntheta,
            &mut workspace.spectra,
            &mut workspace.spin_spectra,
            &workspace.theta_sin[..ntheta],
            &workspace.theta_cos[..ntheta],
            &workspace.ring_order_limits[..ntheta],
            arch,
            &mut workspace.recurrence_workers[0],
        )?;

        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "order scheduling, Legendre recurrence, and spectral accumulation",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }

        self.inverse_synthesis_rows(
            &workspace.spectra,
            first_component,
            &mut workspace.fft,
            #[cfg(feature = "parallel")]
            &mut workspace.fft_worker_workspaces,
            #[cfg(feature = "parallel")]
            workspace.execution_capacity,
        )?;
        self.inverse_synthesis_rows(
            &workspace.spin_spectra,
            second_component,
            &mut workspace.fft,
            #[cfg(feature = "parallel")]
            &mut workspace.fft_worker_workspaces,
            #[cfg(feature = "parallel")]
            workspace.execution_capacity,
        )?;
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "two inverse longitude FFTs and map writeback",
            phase_started,
        );
        Ok(())
    }

    #[cfg(feature = "parallel")]
    fn prepare_spin1_scalar_coefficients(
        &self,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        coefficients: &mut Vec<Complex<T>>,
        finite_orders: &mut Vec<u8>,
        execution_capacity: usize,
    ) -> bool {
        let row_len = match self.harmonics.lmax().checked_add(2) {
            Some(value) => value,
            None => return false,
        };
        let row_stride = match row_len.checked_mul(2) {
            Some(value) => value,
            None => return false,
        };
        let order_count = match self.harmonics.mmax().checked_add(1) {
            Some(value) => value,
            None => return false,
        };
        let coefficient_count = match row_stride.checked_mul(order_count) {
            Some(value) => value,
            None => return false,
        };
        let Some(byte_count) = coefficient_count.checked_mul(std::mem::size_of::<Complex<T>>())
        else {
            return false;
        };
        if byte_count > SPIN1_SYNTH_COEFFICIENT_CACHE_MAX_BYTES {
            return false;
        }
        if coefficients.capacity() < coefficient_count
            && coefficients
                .try_reserve_exact(coefficient_count.saturating_sub(coefficients.len()))
                .is_err()
        {
            return false;
        }
        if finite_orders.capacity() < order_count
            && finite_orders
                .try_reserve_exact(order_count.saturating_sub(finite_orders.len()))
                .is_err()
        {
            return false;
        }
        coefficients.resize(coefficient_count, Complex::new(T::zero(), T::zero()));
        finite_orders.resize(order_count, 0);

        let worker_count = current_execution_thread_count()
            .min(execution_capacity)
            .min(order_count)
            .max(1);
        coefficients
            .par_chunks_mut(row_stride)
            .zip(finite_orders.par_iter_mut())
            .enumerate()
            .with_min_len(order_count.div_ceil(worker_count))
            .for_each(|(m, (order_coefficients, finite))| {
                let coefficient_start = self
                    .harmonics
                    .index(m, m)
                    .expect("every supported order starts at its diagonal");
                let coefficient_len = self.harmonics.lmax() - m + 1;
                let (first, second) = order_coefficients.split_at_mut(row_len);
                spin1_to_scalar(
                    m,
                    self.harmonics.lmax(),
                    &e_alm[coefficient_start..coefficient_start + coefficient_len],
                    Some(&b_alm[coefficient_start..coefficient_start + coefficient_len]),
                    first,
                    second,
                );
                *finite = u8::from(
                    first
                        .iter()
                        .chain(second.iter())
                        .all(|value| value.re.is_finite() && value.im.is_finite()),
                );
            });
        true
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Each ring group receives plan inputs, optional prepared coefficients and worker scratch."
    )]
    fn synthesize_spin_ring_group(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        spin: usize,
        derivative_norm: bool,
        allow_fast_scalar: bool,
        allow_spin1_table: bool,
        spin1_scalar_coefficients: Option<&[Complex<T>]>,
        spin1_scalar_coefficient_finite_orders: Option<&[u8]>,
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_start: usize,
        ring_end: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        #[cfg(all(test, feature = "parallel"))]
        recurrence.record_active_worker();
        debug_assert_eq!(ring_end - ring_start, theta_sin.len());
        debug_assert_eq!(theta_cos.len(), theta_sin.len());
        debug_assert_eq!(q_spectra.len(), theta_sin.len() * spectrum_len);
        debug_assert_eq!(u_spectra.len(), q_spectra.len());
        let fast_scalar_coefficient_count = match spin {
            1 => self.harmonics.lmax() + 2,
            2 => self.harmonics.lmax() + 5,
            _ => 0,
        };
        let fast_scalar_min_lmax = match spin {
            1 => SPIN1_FAST_MIN_LMAX,
            2 => SPIN2_FAST_MIN_LMAX,
            _ => usize::MAX,
        };
        let fast_scalar_enabled = allow_fast_scalar
            && matches!(spin, 1 | 2)
            && (!derivative_norm || (spin == 1 && self.harmonics.spin() == 0))
            && (spin == 1 || b_alm.is_some())
            && self.harmonics.lmax() >= fast_scalar_min_lmax
            && T::supports_paired_scalar()
            && recurrence.fast_scalar_first.len() >= fast_scalar_coefficient_count
            && (spin != 2 || self.spin2_fast_f1.len() > self.harmonics.lmax());
        let fast_sine_min = match spin {
            1 => SPIN1_FAST_MIN_SIN_THETA,
            2 => SPIN2_FAST_MIN_SIN_THETA,
            _ => 1.0,
        };
        let fast_sine_min =
            T::from_f64(fast_sine_min).expect("spin fast sine limit is representable");
        let mut segment_start = 0;
        while segment_start < theta_sin.len() {
            let segment_fast = fast_scalar_enabled && theta_sin[segment_start] >= fast_sine_min;
            let mut segment_end = segment_start + 1;
            while segment_end < theta_sin.len()
                && (fast_scalar_enabled && theta_sin[segment_end] >= fast_sine_min) == segment_fast
            {
                segment_end += 1;
            }
            let spectrum_start = segment_start * spectrum_len;
            let spectrum_end = segment_end * spectrum_len;
            self.synthesize_spin_ring_group_mode(
                e_alm,
                b_alm,
                spin,
                derivative_norm,
                segment_fast,
                allow_spin1_table,
                spin1_scalar_coefficients,
                spin1_scalar_coefficient_finite_orders,
                nphi,
                spectrum_len,
                phi0,
                ring_start + segment_start,
                ring_start + segment_end,
                &mut q_spectra[spectrum_start..spectrum_end],
                &mut u_spectra[spectrum_start..spectrum_end],
                &theta_sin[segment_start..segment_end],
                &theta_cos[segment_start..segment_end],
                ring_order_limits,
                arch,
                recurrence,
            )?;
            segment_start = segment_end;
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Each latitude segment receives transform inputs, optional coefficients and worker scratch."
    )]
    fn synthesize_spin_ring_group_mode(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        spin: usize,
        derivative_norm: bool,
        fast_scalar: bool,
        allow_spin1_table: bool,
        spin1_scalar_coefficients: Option<&[Complex<T>]>,
        spin1_scalar_coefficient_finite_orders: Option<&[u8]>,
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_start: usize,
        ring_end: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        debug_assert_eq!(ring_end - ring_start, theta_sin.len());
        debug_assert_eq!(q_spectra.len(), (ring_end - ring_start) * spectrum_len);
        debug_assert_eq!(u_spectra.len(), q_spectra.len());

        let nyquist_scale = T::from_f64(2.0).expect("two is representable");
        let maximum_ring_order = (spin == 1).then(|| {
            ring_order_limits[ring_start..ring_end]
                .iter()
                .copied()
                .max()
                .unwrap_or(0)
        });
        for m in 0..=mmax {
            // Synthesis cleared these spectra, and the polar cutoff excludes this
            // order from every ring in the segment.
            if maximum_ring_order.is_some_and(|limit| m > limit) {
                continue;
            }
            let phase = longitude_phase::<T>(m, phi0, false);
            let mut fast_scalar_order = fast_scalar;
            if fast_scalar_order && spin == 1 {
                let coefficient_start = self
                    .harmonics
                    .index(m, m)
                    .expect("every packed order starts at its diagonal degree");
                let coefficient_count = lmax - m + 1;
                let coefficient_slice = if derivative_norm {
                    let scaled = &mut recurrence.fast_spin_first[..coefficient_count];
                    for (offset, coefficient) in scaled.iter_mut().enumerate() {
                        let degree = m + offset;
                        let degree = T::from_usize(degree).expect("degree converts");
                        let scale = if degree == T::zero() {
                            T::zero()
                        } else {
                            (degree * (degree + T::one())).sqrt()
                        };
                        *coefficient = e_alm[coefficient_start + offset] * scale;
                        if !coefficient.re.is_finite() || !coefficient.im.is_finite() {
                            fast_scalar_order = false;
                            break;
                        }
                    }
                    fast_scalar_order.then_some(&scaled[..])
                } else {
                    Some(&e_alm[coefficient_start..coefficient_start + coefficient_count])
                };
                if let Some(finite_orders) = spin1_scalar_coefficient_finite_orders {
                    debug_assert!(spin1_scalar_coefficients.is_some());
                    debug_assert!(m < finite_orders.len());
                    fast_scalar_order = finite_orders[m] != 0;
                } else if let Some(e_coefficients) = coefficient_slice {
                    let (scalar_first, scalar_second): (&[Complex<T>], &[Complex<T>]) =
                        if let Some(coefficients) = spin1_scalar_coefficients {
                            let row_len = lmax + 2;
                            let row_stride = row_len * 2;
                            let row_start = m * row_stride;
                            (
                                &coefficients[row_start..row_start + row_len],
                                &coefficients[row_start + row_len..row_start + row_stride],
                            )
                        } else {
                            spin1_to_scalar(
                                m,
                                lmax,
                                e_coefficients,
                                b_alm.map(|values| {
                                    &values
                                        [coefficient_start..coefficient_start + coefficient_count]
                                }),
                                &mut recurrence.fast_scalar_first[..lmax + 2],
                                &mut recurrence.fast_scalar_second[..lmax + 2],
                            );
                            (
                                &recurrence.fast_scalar_first[..lmax + 2],
                                &recurrence.fast_scalar_second[..lmax + 2],
                            )
                        };
                    // The conversion can overflow for finite E/B coefficients
                    // even when the scaled spin recurrence can represent the
                    // sampled field. Keep the original channels for fallback.
                    #[cfg(all(test, feature = "parallel"))]
                    let safety_started = recurrence
                        .spin1_profile_enabled
                        .then(std::time::Instant::now);
                    fast_scalar_order = scalar_first
                        .iter()
                        .chain(scalar_second)
                        .all(|value| value.re.is_finite() && value.im.is_finite());
                    #[cfg(all(test, feature = "parallel"))]
                    if let Some(safety_started) = safety_started {
                        recurrence.spin1_profile_coefficient_safety_ns +=
                            safety_started.elapsed().as_nanos();
                        recurrence.spin1_profile_coefficient_safety_values +=
                            (scalar_first.len() + scalar_second.len()) as u128;
                    }
                }
            }
            if fast_scalar_order {
                if spin == 2 {
                    self.synthesize_spin2_scalar_order(
                        m,
                        e_alm,
                        b_alm.expect("the scalar conversion requires E and B"),
                        spectrum_len,
                        ring_start,
                        q_spectra,
                        u_spectra,
                        theta_sin,
                        theta_cos,
                        ring_order_limits,
                        arch,
                        recurrence,
                    )?;
                } else {
                    let scalar_lmax = lmax + 1;
                    #[cfg(all(test, feature = "parallel"))]
                    let factor_started = recurrence
                        .spin1_profile_enabled
                        .then(std::time::Instant::now);
                    T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
                    let paired_factors_ready = T::fill_paired_scalar_factors(
                        m,
                        scalar_lmax,
                        &recurrence.epsilon,
                        &mut recurrence.spin_alpha,
                        &mut recurrence.spin_coefficient_a,
                        &mut recurrence.spin_coefficient_b,
                    );
                    debug_assert!(paired_factors_ready);
                    #[cfg(all(test, feature = "parallel"))]
                    if let Some(factor_started) = factor_started {
                        recurrence.spin1_profile_factor_ns += factor_started.elapsed().as_nanos();
                    }
                    for block_start in
                        (0..theta_sin.len()).step_by(crate::legendre::LATITUDE_BLOCK_SIZE)
                    {
                        let block_end = (block_start + crate::legendre::LATITUDE_BLOCK_SIZE)
                            .min(theta_sin.len());
                        let spectrum_start = block_start * spectrum_len;
                        let spectrum_end = block_end * spectrum_len;
                        self.synthesize_spin1_scalar_block(
                            m,
                            scalar_lmax,
                            spin1_scalar_coefficients,
                            spectrum_len,
                            ring_start + block_start,
                            allow_spin1_table,
                            &theta_sin[block_start..block_end],
                            &theta_cos[block_start..block_end],
                            &ring_order_limits[ring_start + block_start..ring_start + block_end],
                            arch,
                            recurrence,
                            &mut q_spectra[spectrum_start..spectrum_end],
                            &mut u_spectra[spectrum_start..spectrum_end],
                        )?;
                    }
                }
            } else {
                self.synthesize_general_spin_order(
                    m,
                    e_alm,
                    b_alm,
                    spin,
                    derivative_norm,
                    spectrum_len,
                    ring_start,
                    q_spectra,
                    u_spectra,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    recurrence,
                )?;
            }
            for group_ring in 0..ring_end - ring_start {
                let spectrum_index = group_ring * spectrum_len + m;
                let phased_q = q_spectra[spectrum_index] * phase;
                let phased_u = u_spectra[spectrum_index] * phase;
                if nphi.is_multiple_of(2) && m == nphi / 2 {
                    q_spectra[spectrum_index] =
                        Complex::new(nyquist_scale * phased_q.re, T::zero());
                    u_spectra[spectrum_index] =
                        Complex::new(nyquist_scale * phased_u.re, T::zero());
                } else {
                    q_spectra[spectrum_index] = phased_q;
                    u_spectra[spectrum_index] = phased_u;
                }
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Original-coefficient fallback uses one order and caller-owned scratch."
    )]
    fn synthesize_general_spin_order(
        &self,
        m: usize,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        spin: usize,
        derivative_norm: bool,
        spectrum_len: usize,
        ring_start: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let initial = T::fill_spin_factors(
            spin,
            m,
            lmax,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        )?;
        T::for_each_spin_lm_dispatched(
            arch,
            &initial,
            lmax,
            theta_sin,
            theta_cos,
            &recurrence.spin_coefficient_a,
            &recurrence.spin_coefficient_b,
            &mut recurrence.spin_previous_plus,
            &mut recurrence.spin_current_plus,
            &mut recurrence.spin_next_plus,
            &mut recurrence.spin_exponents_plus,
            &mut recurrence.spin_previous_minus,
            &mut recurrence.spin_current_minus,
            &mut recurrence.spin_next_minus,
            &mut recurrence.spin_exponents_minus,
            |degree, group_ring_start, plus, plus_exp, minus, minus_exp| {
                let coefficient_index = self
                    .harmonics
                    .index(degree, m)
                    .expect("the recurrence visits degrees in the packed order");
                let e_coefficient = e_alm[coefficient_index];
                let b_coefficient = b_alm
                    .map(|values| values[coefficient_index])
                    .unwrap_or_else(|| Complex::new(T::zero(), T::zero()));
                let alpha_norm = recurrence.spin_alpha[degree]
                    * T::spin_normalization(spin, degree, derivative_norm);
                for lane in 0..plus.len() {
                    let group_ring = group_ring_start + lane;
                    let global_ring = ring_start + group_ring;
                    if m > ring_order_limits[global_ring] {
                        continue;
                    }
                    let plus_value = T::scaled_basis_value(plus[lane], plus_exp[lane], alpha_norm);
                    let minus_value =
                        T::scaled_basis_value(minus[lane], minus_exp[lane], alpha_norm);
                    let even_basis = plus_value + minus_value;
                    let odd_basis = plus_value - minus_value;
                    let q_value = e_coefficient * even_basis
                        + Complex::<T>::new(T::zero(), -T::one()) * b_coefficient * odd_basis;
                    let u_value =
                        Complex::<T>::new(T::zero(), T::one()) * e_coefficient * odd_basis
                            + b_coefficient * even_basis;
                    let spectrum_index = group_ring * spectrum_len + m;
                    q_spectra[spectrum_index] += q_value;
                    u_spectra[spectrum_index] += u_value;
                }
                Ok(())
            },
        )?;
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Spin-1 scalar synthesis uses the plan's recurrence factors and worker scratch."
    )]
    fn synthesize_spin1_scalar_block(
        &self,
        m: usize,
        scalar_lmax: usize,
        cached_scalar_coefficients: Option<&[Complex<T>]>,
        spectrum_len: usize,
        global_ring_start: usize,
        allow_table: bool,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
    ) -> Result<(), ShtError> {
        let block_len = theta_sin.len();
        debug_assert_eq!(theta_cos.len(), block_len);
        debug_assert_eq!(ring_order_limits.len(), block_len);
        debug_assert_eq!(q_spectra.len(), block_len * spectrum_len);
        debug_assert_eq!(u_spectra.len(), q_spectra.len());

        let cached_scalar_row = cached_scalar_coefficients.map(|coefficients| {
            let row_len = self.harmonics.lmax() + 2;
            let row_stride = row_len * 2;
            let row_start = m * row_stride;
            (
                &coefficients[row_start..row_start + row_len],
                &coefficients[row_start + row_len..row_start + row_stride],
            )
        });
        let has_excluded_ring = ring_order_limits.iter().any(|&limit| m > limit);
        if ring_order_limits.iter().all(|&limit| m > limit) {
            return Ok(());
        }

        let pair_min_sine = T::from_f64(SPIN1_PAIR_MIN_SIN_THETA)
            .expect("spin-1 paired scalar sine limit converts");
        if allow_table
            && self.harmonics.lmax() >= SPIN1_TABLE_SYNTH_MIN_LMAX
            && let Some(table) = self
                .spin1_scalar_legendre_table
                .get()
                .and_then(Option::as_ref)
            && theta_sin.iter().all(|&sine| sine >= pair_min_sine)
            && (m..=scalar_lmax).all(|degree| {
                let (scalar_first, scalar_second) = cached_scalar_row.unwrap_or((
                    &recurrence.fast_scalar_first[..scalar_lmax + 1],
                    &recurrence.fast_scalar_second[..scalar_lmax + 1],
                ));
                let first = scalar_first[degree];
                let second = scalar_second[degree];
                table.input_is_safe(first.re)
                    && table.input_is_safe(first.im)
                    && table.input_is_safe(second.re)
                    && table.input_is_safe(second.im)
            })
        {
            let ntheta = self.grid.ntheta();
            for degree in m..=scalar_lmax {
                let (scalar_first, scalar_second) = cached_scalar_row.unwrap_or((
                    &recurrence.fast_scalar_first[..scalar_lmax + 1],
                    &recurrence.fast_scalar_second[..scalar_lmax + 1],
                ));
                let basis_lanes = table.degree_lanes(m, degree - m);
                let first_coefficient = scalar_first[degree];
                let second_coefficient = scalar_second[degree];
                let reflected_odd = !(degree + m).is_multiple_of(2);
                for lane in 0..block_len {
                    if m > ring_order_limits[lane] {
                        continue;
                    }
                    let global_ring = global_ring_start + lane;
                    let mirrored_ring = ntheta - 1 - global_ring;
                    let north_ring = global_ring.min(mirrored_ring);
                    let mut basis = basis_lanes[north_ring];
                    if global_ring > mirrored_ring && reflected_odd {
                        basis = -basis;
                    }
                    let weighted_basis = basis / theta_sin[lane];
                    let spectrum_index = lane * spectrum_len + m;
                    q_spectra[spectrum_index] += first_coefficient * weighted_basis;
                    u_spectra[spectrum_index] += second_coefficient * weighted_basis;
                }
            }
            return Ok(());
        }

        if !has_excluded_ring {
            let paired_channels_eligible = T::supports_paired_scalar()
                && recurrence.fast_spin_first.len() >= (scalar_lmax - m + 2) / 2
                && recurrence.fast_spin_pair_first.len() >= (scalar_lmax - m + 2) / 2
                && theta_sin.iter().all(|&sine| sine >= pair_min_sine);
            #[cfg(all(test, feature = "parallel"))]
            if recurrence.spin1_profile_enabled && paired_channels_eligible {
                recurrence.spin1_profile_pair_eligible_blocks += 1;
            }
            let coefficient_pair_count = (scalar_lmax - m + 2) / 2;
            let prepared_pair = if paired_channels_eligible {
                #[cfg(all(test, feature = "parallel"))]
                let prepare_started = recurrence
                    .spin1_profile_enabled
                    .then(std::time::Instant::now);
                let (scalar_first, scalar_second) = cached_scalar_row.unwrap_or((
                    &recurrence.fast_scalar_first[..scalar_lmax + 1],
                    &recurrence.fast_scalar_second[..scalar_lmax + 1],
                ));
                let prepared_first = T::prepare_paired_scalar_synthesis_coefficients(
                    m,
                    scalar_lmax,
                    &recurrence.epsilon,
                    &recurrence.spin_alpha,
                    &scalar_first[m..=scalar_lmax],
                    &mut recurrence.fast_spin_first[..coefficient_pair_count],
                    &mut recurrence.fast_spin_second[..coefficient_pair_count],
                );
                let prepared_second = T::prepare_paired_scalar_synthesis_coefficients(
                    m,
                    scalar_lmax,
                    &recurrence.epsilon,
                    &recurrence.spin_alpha,
                    &scalar_second[m..=scalar_lmax],
                    &mut recurrence.fast_spin_pair_first[..coefficient_pair_count],
                    &mut recurrence.fast_spin_pair_second[..coefficient_pair_count],
                );
                #[cfg(all(test, feature = "parallel"))]
                if let Some(prepare_started) = prepare_started {
                    recurrence.spin1_profile_pair_prepare_ns +=
                        prepare_started.elapsed().as_nanos();
                }
                prepared_first && prepared_second
            } else {
                false
            };
            if prepared_pair {
                #[cfg(all(test, feature = "parallel"))]
                let recurrence_started = recurrence
                    .spin1_profile_enabled
                    .then(std::time::Instant::now);
                let fused_pair = T::synthesize_prepared_scalar_pair_block_dispatched(
                    arch,
                    m,
                    scalar_lmax,
                    theta_sin,
                    theta_cos,
                    self.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &recurrence.fast_spin_first[..coefficient_pair_count],
                    &recurrence.fast_spin_second[..coefficient_pair_count],
                    &recurrence.fast_spin_pair_first[..coefficient_pair_count],
                    &recurrence.fast_spin_pair_second[..coefficient_pair_count],
                    ring_order_limits,
                    spectrum_len,
                    q_spectra,
                    u_spectra,
                    &mut recurrence.previous[..block_len],
                    &mut recurrence.current[..block_len],
                    &mut recurrence.next[..block_len],
                    &mut recurrence.exponents[..block_len],
                    &mut recurrence.analysis_same_real[..block_len],
                    &mut recurrence.analysis_same_imag[..block_len],
                    &mut recurrence.analysis_opposite_real[..block_len],
                    &mut recurrence.analysis_opposite_imag[..block_len],
                )?;
                #[cfg(all(test, feature = "parallel"))]
                if let Some(recurrence_started) = recurrence_started {
                    recurrence.spin1_profile_recurrence_ns +=
                        recurrence_started.elapsed().as_nanos();
                }
                if fused_pair {
                    #[cfg(all(test, feature = "parallel"))]
                    if recurrence.spin1_profile_enabled {
                        recurrence.spin1_profile_pair_success_blocks += 1;
                    }
                    for (ring, &sin_theta) in theta_sin.iter().enumerate() {
                        let spectrum_index = ring * spectrum_len + m;
                        let inverse_sine = sin_theta.recip();
                        q_spectra[spectrum_index] *= inverse_sine;
                        u_spectra[spectrum_index] *= inverse_sine;
                    }
                    return Ok(());
                }
            }

            // The separate-channel fallback is useful only when shared pair
            // scratch is unavailable or the latitude block is too close to a
            // pole for the dual-channel kernel. Prepare both channels before
            // either consumer writes spectra so a single unsafe channel cannot
            // cause a partial commit.
            let prepared_separate = if !paired_channels_eligible {
                let (scalar_first, scalar_second) = cached_scalar_row.unwrap_or((
                    &recurrence.fast_scalar_first[..scalar_lmax + 1],
                    &recurrence.fast_scalar_second[..scalar_lmax + 1],
                ));
                let scratch_ready = recurrence.fast_spin_first.len() >= coefficient_pair_count
                    && recurrence.fast_spin_second.len() >= coefficient_pair_count
                    && recurrence.fast_spin_pair_first.len() >= coefficient_pair_count
                    && recurrence.fast_spin_pair_second.len() >= coefficient_pair_count;
                scratch_ready
                    && T::prepare_paired_scalar_synthesis_coefficients(
                        m,
                        scalar_lmax,
                        &recurrence.epsilon,
                        &recurrence.spin_alpha,
                        &scalar_first[m..=scalar_lmax],
                        &mut recurrence.fast_spin_first[..coefficient_pair_count],
                        &mut recurrence.fast_spin_second[..coefficient_pair_count],
                    )
                    && T::prepare_paired_scalar_synthesis_coefficients(
                        m,
                        scalar_lmax,
                        &recurrence.epsilon,
                        &recurrence.spin_alpha,
                        &scalar_second[m..=scalar_lmax],
                        &mut recurrence.fast_spin_pair_first[..coefficient_pair_count],
                        &mut recurrence.fast_spin_pair_second[..coefficient_pair_count],
                    )
            } else {
                false
            };
            let fused_first = if prepared_separate {
                T::synthesize_paired_scalar_block_prepared_dispatched(
                    arch,
                    m,
                    scalar_lmax,
                    theta_sin,
                    theta_cos,
                    self.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &recurrence.fast_spin_first[..coefficient_pair_count],
                    &recurrence.fast_spin_second[..coefficient_pair_count],
                    ring_order_limits,
                    spectrum_len,
                    q_spectra,
                    &mut recurrence.previous[..block_len],
                    &mut recurrence.current[..block_len],
                    &mut recurrence.next[..block_len],
                    &mut recurrence.exponents[..block_len],
                )?
            } else {
                false
            };
            let fused_second = if prepared_separate {
                T::synthesize_paired_scalar_block_prepared_dispatched(
                    arch,
                    m,
                    scalar_lmax,
                    theta_sin,
                    theta_cos,
                    self.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &recurrence.fast_spin_pair_first[..coefficient_pair_count],
                    &recurrence.fast_spin_pair_second[..coefficient_pair_count],
                    ring_order_limits,
                    spectrum_len,
                    u_spectra,
                    &mut recurrence.previous[..block_len],
                    &mut recurrence.current[..block_len],
                    &mut recurrence.next[..block_len],
                    &mut recurrence.exponents[..block_len],
                )?
            } else {
                false
            };
            debug_assert_eq!(fused_first, fused_second);
            if fused_first && fused_second {
                for (ring, &sin_theta) in theta_sin.iter().enumerate() {
                    let spectrum_index = ring * spectrum_len + m;
                    let inverse_sine = sin_theta.recip();
                    q_spectra[spectrum_index] *= inverse_sine;
                    u_spectra[spectrum_index] *= inverse_sine;
                }
                return Ok(());
            }
        }

        // The common paired route reads the shared rows directly. The
        // scaled and per-degree fallbacks still consume worker-local rows, so
        // populate those only after the paired route declines.
        if let Some((scalar_first, scalar_second)) = cached_scalar_row {
            recurrence.fast_scalar_first[..scalar_lmax + 1].copy_from_slice(scalar_first);
            recurrence.fast_scalar_second[..scalar_lmax + 1].copy_from_slice(scalar_second);
        }
        #[cfg(all(test, feature = "parallel"))]
        let fallback_started = recurrence
            .spin1_profile_enabled
            .then(std::time::Instant::now);
        #[cfg(all(test, feature = "parallel"))]
        if recurrence.spin1_profile_enabled {
            recurrence.spin1_profile_fallback_blocks += 1;
        }
        let mut split_transition_attempted = false;
        let mut split_polar_count = None;
        if self.harmonics.lmax() >= SPIN1_TRANSITION_MIN_LMAX {
            let transition_min_sine =
                T::from_f64(0.01).expect("paired scalar transition sine limit converts");
            let active_start = ring_order_limits
                .iter()
                .position(|&limit| m <= limit)
                .expect("the block has at least one active ring");
            let active_end = ring_order_limits
                .iter()
                .rposition(|&limit| m <= limit)
                .expect("the block has at least one active ring")
                + 1;
            let has_polar_endpoint = theta_sin[active_start] < transition_min_sine
                || theta_sin[active_end - 1] < transition_min_sine;
            if has_polar_endpoint {
                split_transition_attempted = true;
                if theta_sin[active_start..active_end]
                    .iter()
                    .any(|&sine| sine >= transition_min_sine)
                {
                    split_polar_count = try_synthesize_spin1_transition_safe_lanes(
                        arch,
                        m,
                        scalar_lmax,
                        spectrum_len,
                        self.sectoral_norms[m],
                        theta_sin,
                        theta_cos,
                        ring_order_limits,
                        q_spectra,
                        u_spectra,
                        recurrence,
                    )?;
                }
            }
        }
        if split_polar_count == Some(0) {
            #[cfg(all(test, feature = "parallel"))]
            if let Some(fallback_started) = fallback_started {
                recurrence.spin1_profile_fallback_ns += fallback_started.elapsed().as_nanos();
            }
            return Ok(());
        }
        let (fallback_sin, fallback_cos, compact_ring_indices, fallback_len, split_polar_fallback) =
            if let Some(polar_count) = split_polar_count {
                (
                    &recurrence.spin_previous_minus[..polar_count],
                    &recurrence.spin_current_minus[..polar_count],
                    None,
                    polar_count,
                    true,
                )
            } else if has_excluded_ring {
                let mut compact_len = 0;
                for (ring, &limit) in ring_order_limits.iter().enumerate() {
                    if m <= limit {
                        recurrence.compact_theta_sin[compact_len] = theta_sin[ring];
                        recurrence.compact_theta_cos[compact_len] = theta_cos[ring];
                        recurrence.compact_ring_indices[compact_len] = ring;
                        compact_len += 1;
                    }
                }
                debug_assert!(compact_len > 0);
                (
                    &recurrence.compact_theta_sin[..compact_len],
                    &recurrence.compact_theta_cos[..compact_len],
                    Some(&recurrence.compact_ring_indices[..compact_len]),
                    compact_len,
                    false,
                )
            } else {
                (theta_sin, theta_cos, None, block_len, false)
            };
        let mut staged_pair =
            if self.harmonics.lmax() >= SPIN1_TRANSITION_MIN_LMAX && !split_transition_attempted {
                T::try_synthesize_paired_scalar_transition_dispatched(
                    arch,
                    m,
                    scalar_lmax,
                    fallback_sin,
                    fallback_cos,
                    self.sectoral_norms[m],
                    &recurrence.epsilon,
                    &recurrence.spin_alpha,
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &recurrence.fast_scalar_first[m..=scalar_lmax],
                    &recurrence.fast_scalar_second[m..=scalar_lmax],
                    &mut recurrence.previous[..fallback_len],
                    &mut recurrence.current[..fallback_len],
                    &mut recurrence.next[..fallback_len],
                    &mut recurrence.exponents[..fallback_len],
                    &mut recurrence.analysis_same_real[..fallback_len],
                    &mut recurrence.analysis_same_imag[..fallback_len],
                    &mut recurrence.analysis_opposite_real[..fallback_len],
                    &mut recurrence.analysis_opposite_imag[..fallback_len],
                )?
            } else {
                false
            };
        if staged_pair {
            for (lane, &sin_theta) in fallback_sin.iter().enumerate() {
                let inverse_sine = sin_theta.recip();
                staged_pair = [
                    recurrence.analysis_same_real[lane] * inverse_sine,
                    recurrence.analysis_same_imag[lane] * inverse_sine,
                    recurrence.analysis_opposite_real[lane] * inverse_sine,
                    recurrence.analysis_opposite_imag[lane] * inverse_sine,
                ]
                .into_iter()
                .all(|value| value.is_finite());
                if !staged_pair {
                    break;
                }
            }
        }
        if staged_pair {
            for lane in 0..fallback_len {
                let local_ring = compact_ring_indices.map_or(lane, |indices| indices[lane]);
                let spectrum_index = local_ring * spectrum_len + m;
                let inverse_sine = fallback_sin[lane].recip();
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
            if let Some(fallback_started) = fallback_started {
                recurrence.spin1_profile_fallback_ns += fallback_started.elapsed().as_nanos();
            }
            return Ok(());
        }

        T::for_each_y_lm_dispatched(
            arch,
            m,
            scalar_lmax,
            fallback_sin,
            fallback_cos,
            self.sectoral_norms[m],
            &recurrence.epsilon,
            &mut recurrence.previous[..fallback_len],
            &mut recurrence.current[..fallback_len],
            &mut recurrence.next[..fallback_len],
            &mut recurrence.exponents[..fallback_len],
            |l, block_ring_start, basis, exponents| {
                for lane in 0..basis.len() {
                    let fallback_ring = block_ring_start + lane;
                    let group_ring = if split_polar_fallback {
                        usize::try_from(recurrence.spin_exponents_minus[fallback_ring])
                            .expect("saved ring index is nonnegative")
                    } else {
                        compact_ring_indices.map_or(fallback_ring, |indices| indices[fallback_ring])
                    };
                    let inverse_sine = fallback_sin[fallback_ring].recip();
                    let first = T::multiply_complex_scaled_product(
                        recurrence.fast_scalar_first[l],
                        basis[lane],
                        inverse_sine,
                        exponents[lane],
                    );
                    let second = T::multiply_complex_scaled_product(
                        recurrence.fast_scalar_second[l],
                        basis[lane],
                        inverse_sine,
                        exponents[lane],
                    );
                    let spectrum_index = group_ring * spectrum_len + m;
                    q_spectra[spectrum_index] += first;
                    u_spectra[spectrum_index] += second;
                }
                Ok(())
            },
        )?;
        #[cfg(all(test, feature = "parallel"))]
        if let Some(fallback_started) = fallback_started {
            recurrence.spin1_profile_fallback_ns += fallback_started.elapsed().as_nanos();
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Spin-2 scalar synthesis uses the plan's coefficient factors and worker scratch."
    )]
    fn synthesize_spin2_scalar_order(
        &self,
        m: usize,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        spectrum_len: usize,
        ring_start: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let scalar_lmax = lmax + 2;
        let coefficient_start = self
            .harmonics
            .index(m, m)
            .expect("every packed order starts at its diagonal degree");
        let coefficient_count = lmax - m + 1;
        spin2_to_scalar(
            m,
            lmax,
            &e_alm[coefficient_start..coefficient_start + coefficient_count],
            &b_alm[coefficient_start..coefficient_start + coefficient_count],
            &mut recurrence.fast_scalar_first[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_scalar_second[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_spin_first[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_spin_second[..self.spin_fast_coefficient_count],
            &self.spin2_fast_f1,
            &self.spin2_fast_f2,
        );
        let converted_finite = recurrence.fast_scalar_first[m..=scalar_lmax]
            .iter()
            .chain(&recurrence.fast_scalar_second[m..=scalar_lmax])
            .all(|value| value.re.is_finite() && value.im.is_finite());
        if !converted_finite {
            return self.synthesize_general_spin_order(
                m,
                e_alm,
                Some(b_alm),
                2,
                false,
                spectrum_len,
                ring_start,
                q_spectra,
                u_spectra,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                recurrence,
            );
        }
        T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
        let paired_factors_ready = T::fill_paired_scalar_factors(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        );
        debug_assert!(paired_factors_ready);
        let pair_count = (scalar_lmax - m + 2) / 2;
        let pairs_finite = paired_factors_ready
            && T::prepare_paired_scalar_synthesis_coefficients(
                m,
                scalar_lmax,
                &recurrence.epsilon,
                &recurrence.spin_alpha,
                &recurrence.fast_scalar_first[m..=scalar_lmax],
                &mut recurrence.fast_spin_first[..pair_count],
                &mut recurrence.fast_spin_second[..pair_count],
            )
            && T::prepare_paired_scalar_synthesis_coefficients(
                m,
                scalar_lmax,
                &recurrence.epsilon,
                &recurrence.spin_alpha,
                &recurrence.fast_scalar_second[m..=scalar_lmax],
                &mut recurrence.fast_spin_first[..pair_count],
                &mut recurrence.fast_spin_second[..pair_count],
            );
        if !pairs_finite {
            return self.synthesize_general_spin_order(
                m,
                e_alm,
                Some(b_alm),
                2,
                false,
                spectrum_len,
                ring_start,
                q_spectra,
                u_spectra,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                recurrence,
            );
        }

        for block_start in (0..theta_sin.len()).step_by(crate::legendre::LATITUDE_BLOCK_SIZE) {
            let block_end =
                (block_start + crate::legendre::LATITUDE_BLOCK_SIZE).min(theta_sin.len());
            let block_len = block_end - block_start;
            let block_sine = &theta_sin[block_start..block_end];
            let block_cosine = &theta_cos[block_start..block_end];
            let block_order_limits =
                &ring_order_limits[ring_start + block_start..ring_start + block_end];
            let spectrum_start = block_start * spectrum_len;
            let spectrum_end = block_end * spectrum_len;
            if block_order_limits.iter().all(|&limit| m > limit) {
                continue;
            }
            let fused_pair = T::synthesize_paired_scalar_pair_block_dispatched(
                arch,
                m,
                scalar_lmax,
                block_sine,
                block_cosine,
                self.sectoral_norms[m],
                &recurrence.epsilon,
                &recurrence.spin_alpha,
                &recurrence.spin_coefficient_a,
                &recurrence.spin_coefficient_b,
                &recurrence.fast_scalar_first[m..=scalar_lmax],
                &recurrence.fast_scalar_second[m..=scalar_lmax],
                block_order_limits,
                spectrum_len,
                &mut q_spectra[spectrum_start..spectrum_end],
                &mut u_spectra[spectrum_start..spectrum_end],
                &mut recurrence.previous[..block_len],
                &mut recurrence.current[..block_len],
                &mut recurrence.next[..block_len],
                &mut recurrence.exponents[..block_len],
                &mut recurrence.analysis_same_real[..block_len],
                &mut recurrence.analysis_same_imag[..block_len],
                &mut recurrence.analysis_opposite_real[..block_len],
                &mut recurrence.analysis_opposite_imag[..block_len],
            )?;
            if fused_pair {
                for (local_ring, &sine) in block_sine.iter().enumerate() {
                    let spectrum_index = local_ring * spectrum_len + m;
                    let inverse_sine = sine.recip();
                    let inverse_sine_squared = inverse_sine * inverse_sine;
                    q_spectra[spectrum_start + spectrum_index] *= inverse_sine_squared;
                    u_spectra[spectrum_start + spectrum_index] *= inverse_sine_squared;
                }
                continue;
            }

            let (fallback_sine, fallback_cosine, compact_ring_indices) =
                if block_order_limits.iter().any(|&limit| m > limit) {
                    let mut compact_len = 0;
                    for (local_ring, &limit) in block_order_limits.iter().enumerate() {
                        if m <= limit {
                            recurrence.compact_theta_sin[compact_len] = block_sine[local_ring];
                            recurrence.compact_theta_cos[compact_len] = block_cosine[local_ring];
                            recurrence.compact_ring_indices[compact_len] = local_ring;
                            compact_len += 1;
                        }
                    }
                    debug_assert!(compact_len > 0);
                    (
                        &recurrence.compact_theta_sin[..compact_len],
                        &recurrence.compact_theta_cos[..compact_len],
                        Some(&recurrence.compact_ring_indices[..compact_len]),
                    )
                } else {
                    (block_sine, block_cosine, None)
                };

            let mut staged_pair = T::try_synthesize_paired_scalar_transition_dispatched(
                arch,
                m,
                scalar_lmax,
                fallback_sine,
                fallback_cosine,
                self.sectoral_norms[m],
                &recurrence.epsilon,
                &recurrence.spin_alpha,
                &recurrence.spin_coefficient_a,
                &recurrence.spin_coefficient_b,
                &recurrence.fast_scalar_first[m..=scalar_lmax],
                &recurrence.fast_scalar_second[m..=scalar_lmax],
                &mut recurrence.previous[..fallback_sine.len()],
                &mut recurrence.current[..fallback_sine.len()],
                &mut recurrence.next[..fallback_sine.len()],
                &mut recurrence.exponents[..fallback_sine.len()],
                &mut recurrence.analysis_same_real[..fallback_sine.len()],
                &mut recurrence.analysis_same_imag[..fallback_sine.len()],
                &mut recurrence.analysis_opposite_real[..fallback_sine.len()],
                &mut recurrence.analysis_opposite_imag[..fallback_sine.len()],
            )?;
            if staged_pair {
                for (lane, &sine) in fallback_sine.iter().enumerate() {
                    let inverse_sine = sine.recip();
                    let inverse_sine_squared = inverse_sine * inverse_sine;
                    staged_pair = [
                        recurrence.analysis_same_real[lane] * inverse_sine_squared,
                        recurrence.analysis_same_imag[lane] * inverse_sine_squared,
                        recurrence.analysis_opposite_real[lane] * inverse_sine_squared,
                        recurrence.analysis_opposite_imag[lane] * inverse_sine_squared,
                    ]
                    .into_iter()
                    .all(|value| value.is_finite());
                    if !staged_pair {
                        break;
                    }
                }
            }
            if staged_pair {
                for lane in 0..fallback_sine.len() {
                    let local_ring = compact_ring_indices.map_or(lane, |indices| indices[lane]);
                    let group_ring = block_start + local_ring;
                    let inverse_sine = fallback_sine[lane].recip();
                    let inverse_sine_squared = inverse_sine * inverse_sine;
                    let spectrum_index = group_ring * spectrum_len + m;
                    q_spectra[spectrum_index] += Complex::new(
                        recurrence.analysis_same_real[lane] * inverse_sine_squared,
                        recurrence.analysis_same_imag[lane] * inverse_sine_squared,
                    );
                    u_spectra[spectrum_index] += Complex::new(
                        recurrence.analysis_opposite_real[lane] * inverse_sine_squared,
                        recurrence.analysis_opposite_imag[lane] * inverse_sine_squared,
                    );
                }
                continue;
            }

            T::for_each_y_lm_paired_dispatched(
                arch,
                m,
                scalar_lmax,
                fallback_sine,
                fallback_cosine,
                self.sectoral_norms[m],
                &recurrence.spin_coefficient_a,
                &recurrence.spin_coefficient_b,
                &mut recurrence.previous[..block_len],
                &mut recurrence.current[..block_len],
                &mut recurrence.next[..block_len],
                &mut recurrence.exponents[..block_len],
                |l, block_ring_start, basis, exponents| {
                    let pair_index = (l - m) / 2;
                    let next_even = l + 2 <= scalar_lmax;
                    let next_first = if next_even {
                        recurrence.fast_scalar_first[l + 2]
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    let next_second = if next_even {
                        recurrence.fast_scalar_second[l + 2]
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    let even_first = (recurrence.fast_scalar_first[l] * recurrence.epsilon[l + 1]
                        + next_first * recurrence.epsilon[l + 2])
                        * recurrence.spin_alpha[pair_index];
                    let even_second = (recurrence.fast_scalar_second[l]
                        * recurrence.epsilon[l + 1]
                        + next_second * recurrence.epsilon[l + 2])
                        * recurrence.spin_alpha[pair_index];
                    let odd_first = if l < scalar_lmax {
                        recurrence.fast_scalar_first[l + 1] * recurrence.spin_alpha[pair_index]
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    let odd_second = if l < scalar_lmax {
                        recurrence.fast_scalar_second[l + 1] * recurrence.spin_alpha[pair_index]
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    for lane in 0..basis.len() {
                        let fallback_lane = block_ring_start + lane;
                        let local_ring = compact_ring_indices
                            .map_or(fallback_lane, |indices| indices[fallback_lane]);
                        let group_ring = block_start + local_ring;
                        let global_ring = ring_start + group_ring;
                        if m > ring_order_limits[global_ring] {
                            continue;
                        }
                        let inverse_sine = fallback_sine[fallback_lane].recip();
                        let inverse_sine_squared = inverse_sine * inverse_sine;
                        let cosine_over_sine_squared =
                            fallback_cosine[fallback_lane] * inverse_sine_squared;
                        let first = T::multiply_complex_scaled_product(
                            even_first,
                            basis[lane],
                            inverse_sine_squared,
                            exponents[lane],
                        ) + T::multiply_complex_scaled_product(
                            odd_first,
                            basis[lane],
                            cosine_over_sine_squared,
                            exponents[lane],
                        );
                        let second = T::multiply_complex_scaled_product(
                            even_second,
                            basis[lane],
                            inverse_sine_squared,
                            exponents[lane],
                        ) + T::multiply_complex_scaled_product(
                            odd_second,
                            basis[lane],
                            cosine_over_sine_squared,
                            exponents[lane],
                        );
                        let spectrum_index = group_ring * spectrum_len + m;
                        q_spectra[spectrum_index] += first;
                        u_spectra[spectrum_index] += second;
                    }
                    Ok(())
                },
            )?;
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The analysis group shares spectra and grid metadata and owns two coefficient slices plus worker scratch."
    )]
    fn analyze_spin_order_group(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_weights: Option<&[f64]>,
        normalize_fourier: bool,
        analysis_ring_scales: Option<&[T]>,
        spectra_preweighted: bool,
        recurrence_spin: usize,
        derivative_norm: bool,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        e_alm: &mut [Complex<T>],
        mut b_alm: Option<&mut [Complex<T>]>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        #[cfg(all(test, feature = "parallel"))]
        recurrence.record_active_worker();
        #[cfg(all(test, feature = "parallel"))]
        let profile_started = recurrence
            .analysis_leaf_profile_enabled()
            .then(std::time::Instant::now);
        #[cfg(all(test, feature = "parallel"))]
        recurrence.reset_analysis_leaf_block_counts();
        let lmax = self.harmonics.lmax();
        let longitude_count = T::from_usize(nphi).expect("longitude count converts");
        let fast_scalar_coefficient_count = match recurrence_spin {
            1 => lmax + 2,
            2 => lmax + 5,
            _ => 0,
        };
        let fast_scalar_min_lmax = match recurrence_spin {
            1 => SPIN1_FAST_MIN_LMAX,
            2 => SPIN2_FAST_MIN_LMAX,
            _ => usize::MAX,
        };
        let fast_scalar_domain = match recurrence_spin {
            1 => ring_weights.is_some() || spectra_preweighted,
            2 => self.grid.kind() == crate::grid::GridKind::GaussLegendre && ring_weights.is_some(),
            _ => false,
        };
        let fast_scalar_enabled = fast_scalar_domain
            && self.harmonics.spin() == recurrence_spin
            && !derivative_norm
            && (recurrence_spin == 1 || b_alm.is_some())
            && lmax >= fast_scalar_min_lmax
            && T::supports_paired_scalar()
            && recurrence.fast_scalar_first.len() >= fast_scalar_coefficient_count
            && (recurrence_spin != 2 || self.spin2_fast_f1.len() > lmax)
            && recurrence.fast_spin_first.len() > lmax;
        let fast_range = if fast_scalar_enabled {
            let minimum_sine = match recurrence_spin {
                1 => SPIN1_FAST_MIN_SIN_THETA,
                2 => SPIN2_FAST_MIN_SIN_THETA,
                _ => unreachable!("only the spin-1/2 scalar fast paths are enabled"),
            };
            let minimum_sine =
                T::from_f64(minimum_sine).expect("spin fast sine limit is representable");
            let start = theta_sin
                .iter()
                .position(|&sine| sine >= minimum_sine)
                .unwrap_or(theta_sin.len());
            let end = theta_sin[start..]
                .iter()
                .position(|&sine| sine < minimum_sine)
                .map_or(theta_sin.len(), |offset| start + offset);
            (start < end).then_some((start, end))
        } else {
            None
        };
        let mut coefficient_start = 0;
        for m in m_start..m_end {
            let coefficient_count = lmax - m + 1;
            let e_coefficients =
                &mut e_alm[coefficient_start..coefficient_start + coefficient_count];
            let mut b_coefficients = b_alm.as_deref_mut().map(|values| {
                &mut values[coefficient_start..coefficient_start + coefficient_count]
            });
            let phase = longitude_phase::<T>(m, phi0, true);
            if recurrence_spin == 1 && fast_range == Some((0, theta_sin.len())) {
                if self.allow_spin1_analysis_table()
                    && let Some(table) = self
                        .spin1_scalar_legendre_table
                        .get()
                        .and_then(Option::as_ref)
                {
                    let used_table = self.analyze_spin_scalar_table_order(
                        q_spectra,
                        u_spectra,
                        spectrum_len,
                        longitude_count,
                        phase,
                        ring_weights.expect("the spin scalar table requires exact GL weights"),
                        normalize_fourier,
                        m,
                        theta_sin,
                        ring_order_limits,
                        arch,
                        e_coefficients,
                        b_coefficients.as_deref_mut(),
                        table,
                        recurrence,
                    )?;
                    if used_table {
                        coefficient_start += coefficient_count;
                        continue;
                    }
                }
                let paired_order = self.grid.kind() == crate::grid::GridKind::GaussLegendre
                    && ring_weights.is_some()
                    && self.analyze_spin_scalar_gl_paired_order(
                        q_spectra,
                        u_spectra,
                        spectrum_len,
                        phase,
                        analysis_ring_scales.expect("GL spin analysis has ring scales"),
                        m,
                        theta_sin,
                        theta_cos,
                        ring_order_limits,
                        arch,
                        e_coefficients,
                        b_coefficients.as_deref_mut(),
                        recurrence,
                    )?;
                if paired_order {
                    coefficient_start += coefficient_count;
                    continue;
                }
                self.analyze_spin_scalar_fast_order(
                    q_spectra,
                    u_spectra,
                    spectrum_len,
                    longitude_count,
                    phase,
                    ring_weights,
                    normalize_fourier,
                    m,
                    0,
                    theta_sin.len(),
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    e_coefficients,
                    b_coefficients.as_deref_mut(),
                    recurrence,
                )?;
                coefficient_start += coefficient_count;
                continue;
            }
            let mut initial = T::fill_spin_factors(
                recurrence_spin,
                m,
                lmax,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            )?;
            if let Some((fast_start, fast_end)) = fast_range {
                if recurrence_spin == 2 {
                    self.analyze_spin2_scalar_fast_order(
                        q_spectra,
                        u_spectra,
                        spectrum_len,
                        longitude_count,
                        phase,
                        ring_weights.expect("the fast analysis path uses exact ring weights"),
                        normalize_fourier,
                        m,
                        fast_start,
                        fast_end,
                        theta_sin,
                        theta_cos,
                        ring_order_limits,
                        arch,
                        e_coefficients,
                        b_coefficients.as_deref_mut(),
                        recurrence,
                    )?;
                } else {
                    self.analyze_spin_scalar_fast_order(
                        q_spectra,
                        u_spectra,
                        spectrum_len,
                        longitude_count,
                        phase,
                        ring_weights,
                        normalize_fourier,
                        m,
                        fast_start,
                        fast_end,
                        theta_sin,
                        theta_cos,
                        ring_order_limits,
                        arch,
                        e_coefficients,
                        b_coefficients.as_deref_mut(),
                        recurrence,
                    )?;
                }
                initial = T::fill_spin_factors(
                    recurrence_spin,
                    m,
                    lmax,
                    &mut recurrence.spin_alpha,
                    &mut recurrence.spin_coefficient_a,
                    &mut recurrence.spin_coefficient_b,
                )?;
            }
            let first_normal_end = fast_range.map_or(theta_sin.len(), |(start, _)| start);
            let second_normal_start = fast_range.map_or(theta_sin.len(), |(_, end)| end);
            for (segment_start, segment_end) in [
                (0, first_normal_end),
                (second_normal_start, theta_sin.len()),
            ] {
                for block_start in (segment_start..segment_end).step_by(LATITUDE_BLOCK_SIZE) {
                    let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(segment_end);
                    let block_len = block_end - block_start;
                    for lane in 0..block_len {
                        let ring = block_start + lane;
                        let spectrum_start = ring * spectrum_len;
                        let mut q_weighted = q_spectra[spectrum_start + m] * phase;
                        let mut u_weighted = u_spectra[spectrum_start + m] * phase;
                        let mut scale = ring_weights.map_or(T::one(), |weights| {
                            T::from_f64(weights[ring]).expect("ring weight is representable")
                        });
                        if normalize_fourier {
                            scale /= longitude_count;
                        }
                        q_weighted *= scale;
                        u_weighted *= scale;
                        recurrence.weighted_spin_q_spectrum[lane] = q_weighted;
                        recurrence.weighted_spin_u_spectrum[lane] = u_weighted;
                    }
                    let weighted_q_spectrum = &recurrence.weighted_spin_q_spectrum[..block_len];
                    let weighted_u_spectrum = &recurrence.weighted_spin_u_spectrum[..block_len];
                    if T::supports_spin_analysis_simd() {
                        for lane in 0..block_len {
                            recurrence.analysis_same_real[lane] = weighted_q_spectrum[lane].re;
                            recurrence.analysis_same_imag[lane] = weighted_q_spectrum[lane].im;
                            recurrence.analysis_opposite_real[lane] = weighted_u_spectrum[lane].re;
                            recurrence.analysis_opposite_imag[lane] = weighted_u_spectrum[lane].im;
                        }
                    }
                    let analysis_q_real = &recurrence.analysis_same_real[..block_len];
                    let analysis_q_imag = &recurrence.analysis_same_imag[..block_len];
                    let analysis_u_real = &recurrence.analysis_opposite_real[..block_len];
                    let analysis_u_imag = &recurrence.analysis_opposite_imag[..block_len];
                    let vectorizable_inputs = T::spin_analysis_inputs_vectorizable(
                        m,
                        analysis_q_real,
                        analysis_q_imag,
                        analysis_u_real,
                        analysis_u_imag,
                        &ring_order_limits[block_start..block_end],
                    );
                    T::for_each_spin_lm_dispatched(
                        arch,
                        &initial,
                        lmax,
                        &theta_sin[block_start..block_end],
                        &theta_cos[block_start..block_end],
                        &recurrence.spin_coefficient_a,
                        &recurrence.spin_coefficient_b,
                        &mut recurrence.spin_previous_plus,
                        &mut recurrence.spin_current_plus,
                        &mut recurrence.spin_next_plus,
                        &mut recurrence.spin_exponents_plus,
                        &mut recurrence.spin_previous_minus,
                        &mut recurrence.spin_current_minus,
                        &mut recurrence.spin_next_minus,
                        &mut recurrence.spin_exponents_minus,
                        |degree, ring_start, plus, plus_exp, minus, minus_exp| {
                            let alpha_norm = recurrence.spin_alpha[degree]
                                * T::spin_normalization(recurrence_spin, degree, derivative_norm);
                            let coefficient = &mut e_coefficients[degree - m];
                            let mut b_coefficient = b_coefficients
                                .as_deref_mut()
                                .map(|values| &mut values[degree - m]);
                            if T::supports_spin_analysis_simd() {
                                let lane_end = ring_start + plus.len();
                                let (e_contribution, b_contribution) =
                                    T::accumulate_spin_analysis_block(
                                        arch,
                                        vectorizable_inputs,
                                        m,
                                        &analysis_q_real[ring_start..lane_end],
                                        &analysis_q_imag[ring_start..lane_end],
                                        &analysis_u_real[ring_start..lane_end],
                                        &analysis_u_imag[ring_start..lane_end],
                                        plus,
                                        plus_exp,
                                        minus,
                                        minus_exp,
                                        alpha_norm,
                                        &ring_order_limits
                                            [block_start + ring_start..block_start + lane_end],
                                        b_coefficient.is_some(),
                                    );
                                *coefficient += e_contribution;
                                if let Some(b_coefficient) = b_coefficient.as_mut() {
                                    **b_coefficient += b_contribution;
                                }
                            } else {
                                for lane in 0..plus.len() {
                                    let ring = ring_start + lane;
                                    if m > ring_order_limits[block_start + ring] {
                                        continue;
                                    }
                                    let plus_value = T::scaled_basis_value(
                                        plus[lane],
                                        plus_exp[lane],
                                        alpha_norm,
                                    );
                                    let minus_value = T::scaled_basis_value(
                                        minus[lane],
                                        minus_exp[lane],
                                        alpha_norm,
                                    );
                                    let even_basis = plus_value + minus_value;
                                    let odd_basis = plus_value - minus_value;
                                    let q_fourier = weighted_q_spectrum[ring];
                                    let u_fourier = weighted_u_spectrum[ring];
                                    *coefficient += q_fourier * even_basis
                                        + Complex::<T>::new(T::zero(), -T::one())
                                            * u_fourier
                                            * odd_basis;
                                    if let Some(b_coefficient) = b_coefficient.as_mut() {
                                        **b_coefficient += Complex::<T>::new(T::zero(), T::one())
                                            * q_fourier
                                            * odd_basis
                                            + u_fourier * even_basis;
                                    }
                                }
                            }
                            Ok(())
                        },
                    )?;
                }
            }
            coefficient_start += coefficient_count;
        }
        debug_assert_eq!(coefficient_start, e_alm.len());
        if let Some(b_alm) = b_alm {
            debug_assert_eq!(coefficient_start, b_alm.len());
        }
        #[cfg(all(test, feature = "parallel"))]
        if let Some(started) = profile_started {
            recurrence.record_analysis_leaf(true, lmax, m_start, m_end, ring_order_limits, started);
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The paired scalar reducer receives the parity spectra and recurrence block explicitly."
    )]
    fn accumulate_spin_scalar_analysis_pair_block(
        arch: pulp::Arch,
        m: usize,
        scalar_lmax: usize,
        sectoral_norm: T,
        block_sine: &[T],
        block_cosine: &[T],
        q_products_safe: bool,
        u_products_safe: bool,
        q_even_real: &[T],
        q_even_imag: &[T],
        q_odd_real: &[T],
        q_odd_imag: &[T],
        u_even_real: &[T],
        u_even_imag: &[T],
        u_odd_real: &[T],
        u_odd_imag: &[T],
        coefficient_a: &[T],
        coefficient_b: &[T],
        previous: &mut [T],
        current: &mut [T],
        next: &mut [T],
        exponents: &mut [i64],
        #[cfg(all(test, feature = "parallel"))] profile_range_status: Option<&mut (bool, bool)>,
        scalar_first: &mut [Complex<T>],
        scalar_second: &mut [Complex<T>],
    ) -> Result<bool, ShtError> {
        #[cfg(all(test, feature = "parallel"))]
        let mut profile_range_status = profile_range_status;
        let paired_used = T::for_each_y_lm_paired_status_dispatched(
            arch,
            m,
            scalar_lmax,
            block_sine,
            block_cosine,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            previous,
            current,
            next,
            exponents,
            |l, local_ring_start, basis, exponents, normal_range| {
                #[cfg(all(test, feature = "parallel"))]
                if let Some((normal, scaled)) = profile_range_status.as_deref_mut() {
                    if normal_range {
                        *normal = true;
                    } else {
                        *scaled = true;
                    }
                }
                let mut first_even = scalar_first[l];
                let mut second_even = scalar_second[l];
                let mut first_odd = if l < scalar_lmax {
                    scalar_first[l + 1]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let mut second_odd = if l < scalar_lmax {
                    scalar_second[l + 1]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let lane_end = local_ring_start + basis.len();
                let lanes = local_ring_start..lane_end;
                let cosine = &block_cosine[lanes.clone()];
                let [(q_even, q_odd), (u_even, u_odd)] =
                    T::accumulate_scalar_analysis_pair_channels(
                        arch,
                        [q_products_safe, u_products_safe],
                        normal_range,
                        [&q_even_real[lanes.clone()], &u_even_real[lanes.clone()]],
                        [&q_even_imag[lanes.clone()], &u_even_imag[lanes.clone()]],
                        [&q_odd_real[lanes.clone()], &u_odd_real[lanes.clone()]],
                        [&q_odd_imag[lanes.clone()], &u_odd_imag[lanes]],
                        basis,
                        exponents,
                        cosine,
                    );
                first_even += q_even;
                second_even += u_even;
                if l < scalar_lmax {
                    first_odd += q_odd;
                    second_odd += u_odd;
                }
                scalar_first[l] = first_even;
                scalar_second[l] = second_even;
                if l < scalar_lmax {
                    scalar_first[l + 1] = first_odd;
                    scalar_second[l + 1] = second_odd;
                }
                Ok(())
            },
        )?;
        Ok(paired_used)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The fused spin-1 analysis block shares recurrence lanes across Q and U and stages both coefficient rows."
    )]
    fn try_fused_spin_scalar_analysis_pair_block(
        arch: pulp::Arch,
        m: usize,
        scalar_lmax: usize,
        sectoral_norm: T,
        sin_theta: &[T],
        cos_theta: &[T],
        products_safe: [bool; 2],
        same_real: [&[T]; 2],
        same_imag: [&[T]; 2],
        opposite_real: [&[T]; 2],
        opposite_imag: [&[T]; 2],
        coefficient_a: &[T],
        coefficient_b: &[T],
        scalar_first: &mut [Complex<T>],
        scalar_second: &mut [Complex<T>],
        staged_first: &mut [Complex<T>],
        staged_second: &mut [Complex<T>],
        previous: &mut [T],
        current: &mut [T],
        next: &mut [T],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        if !T::supports_paired_scalar() || !products_safe[0] || !products_safe[1] || m > scalar_lmax
        {
            return Ok(false);
        }

        let staged_count = scalar_lmax - m + 1;
        if scalar_first.len() < scalar_lmax + 1
            || scalar_second.len() < scalar_lmax + 1
            || staged_first.len() < staged_count
            || staged_second.len() < staged_count
            || previous.len() < sin_theta.len()
            || current.len() < sin_theta.len()
            || next.len() < sin_theta.len()
            || exponents.len() < sin_theta.len()
        {
            return Ok(false);
        }

        let fused = T::fused_scalar_analysis_pair_channels_block(
            arch,
            m,
            scalar_lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            &mut staged_first[..staged_count],
            &mut staged_second[..staged_count],
            previous,
            current,
            next,
            exponents,
        )?;
        if fused {
            for offset in 0..staged_count {
                scalar_first[m + offset] += staged_first[offset];
                scalar_second[m + offset] += staged_second[offset];
            }
        }
        Ok(fused)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The GL paired analysis path receives its mirrored Fourier data and worker scratch explicitly."
    )]
    fn analyze_spin_scalar_gl_paired_order(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        spectrum_len: usize,
        phase: Complex<T>,
        ring_scales: &[T],
        m: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        e_coefficients: &mut [Complex<T>],
        mut b_coefficients: Option<&mut [Complex<T>]>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<bool, ShtError> {
        if !T::supports_paired_scalar() {
            return Ok(false);
        }
        debug_assert_eq!(theta_sin.len(), theta_cos.len());
        debug_assert_eq!(theta_sin.len(), ring_order_limits.len());
        debug_assert_eq!(ring_scales.len(), theta_sin.len());

        let scalar_lmax = self.harmonics.lmax() + 1;
        let scalar_coefficient_count = scalar_lmax + 1;
        let fused_analysis_enabled =
            self.harmonics.lmax() >= SPIN1_FUSED_ANALYSIS_MIN_LMAX && single_threaded_execution();
        #[cfg(all(test, feature = "parallel"))]
        let profiling = recurrence.analysis_leaf_profile_enabled();
        recurrence.fast_scalar_first[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        recurrence.fast_scalar_second[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
        let paired_factors_ready = T::fill_paired_scalar_factors(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        );
        debug_assert!(paired_factors_ready);

        let ntheta = theta_sin.len();
        let pair_count = ntheta.div_ceil(2);
        let compaction_enabled = self.harmonics.lmax() >= 127;
        for block_start in (0..pair_count).step_by(LATITUDE_BLOCK_SIZE) {
            let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(pair_count);
            let block_len = block_end - block_start;
            let mut active_lanes = 0;
            for lane in 0..block_len {
                let north_ring = block_start + lane;
                let south_ring = ntheta - 1 - north_ring;
                let north_active = m <= ring_order_limits[north_ring];
                let south_active = south_ring != north_ring && m <= ring_order_limits[south_ring];
                if north_active || south_active {
                    active_lanes += 1;
                }

                let north_spectrum_start = north_ring * spectrum_len + m;
                let south_spectrum_start = south_ring * spectrum_len + m;
                let north_q = if north_active {
                    q_spectra[north_spectrum_start] * phase * ring_scales[north_ring]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let south_q = if south_active {
                    q_spectra[south_spectrum_start] * phase * ring_scales[south_ring]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let north_u = if north_active {
                    u_spectra[north_spectrum_start] * phase * ring_scales[north_ring]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let south_u = if south_active {
                    u_spectra[south_spectrum_start] * phase * ring_scales[south_ring]
                } else {
                    Complex::new(T::zero(), T::zero())
                };

                recurrence.analysis_same_real[lane] = north_q.re;
                recurrence.analysis_same_imag[lane] = north_q.im;
                recurrence.analysis_opposite_real[lane] = south_q.re;
                recurrence.analysis_opposite_imag[lane] = south_q.im;
                recurrence.spin_previous_plus[lane] = north_u.re;
                recurrence.spin_current_plus[lane] = north_u.im;
                recurrence.spin_previous_minus[lane] = south_u.re;
                recurrence.spin_current_minus[lane] = south_u.im;
            }
            if active_lanes == 0 {
                continue;
            }

            // Check each reflected input before forming its sum and difference.
            // This keeps very large finite values on the established recurrence
            // path instead of overflowing an intermediate parity combination.
            let all_cosines = &theta_cos[block_start..block_end];
            let q_inputs_safe = T::scalar_analysis_products_safe(
                &recurrence.analysis_same_real[..block_len],
                &recurrence.analysis_same_imag[..block_len],
                &recurrence.analysis_opposite_real[..block_len],
                &recurrence.analysis_opposite_imag[..block_len],
                all_cosines,
            );
            let u_inputs_safe = T::scalar_analysis_products_safe(
                &recurrence.spin_previous_plus[..block_len],
                &recurrence.spin_current_plus[..block_len],
                &recurrence.spin_previous_minus[..block_len],
                &recurrence.spin_current_minus[..block_len],
                all_cosines,
            );
            if !q_inputs_safe || !u_inputs_safe {
                return Ok(false);
            }

            let compact = compaction_enabled && active_lanes < block_len;
            let mut compact_len = 0;
            for lane in 0..block_len {
                let north_ring = block_start + lane;
                let south_ring = ntheta - 1 - north_ring;
                let active = m <= ring_order_limits[north_ring]
                    || (south_ring != north_ring && m <= ring_order_limits[south_ring]);
                if compact && !active {
                    continue;
                }
                let output_lane = if compact { compact_len } else { lane };
                let north_q_re = recurrence.analysis_same_real[lane];
                let north_q_im = recurrence.analysis_same_imag[lane];
                let south_q_re = recurrence.analysis_opposite_real[lane];
                let south_q_im = recurrence.analysis_opposite_imag[lane];
                let north_u_re = recurrence.spin_previous_plus[lane];
                let north_u_im = recurrence.spin_current_plus[lane];
                let south_u_re = recurrence.spin_previous_minus[lane];
                let south_u_im = recurrence.spin_current_minus[lane];

                recurrence.analysis_same_real[output_lane] = north_q_re + south_q_re;
                recurrence.analysis_same_imag[output_lane] = north_q_im + south_q_im;
                recurrence.analysis_opposite_real[output_lane] = north_q_re - south_q_re;
                recurrence.analysis_opposite_imag[output_lane] = north_q_im - south_q_im;
                recurrence.spin_previous_plus[output_lane] = north_u_re + south_u_re;
                recurrence.spin_current_plus[output_lane] = north_u_im + south_u_im;
                recurrence.spin_previous_minus[output_lane] = north_u_re - south_u_re;
                recurrence.spin_current_minus[output_lane] = north_u_im - south_u_im;
                if compact {
                    recurrence.compact_theta_sin[output_lane] = theta_sin[north_ring];
                    recurrence.compact_theta_cos[output_lane] = theta_cos[north_ring];
                }
                compact_len += 1;
            }

            let lane_count = if compact { compact_len } else { block_len };
            let block_sine = if compact {
                &recurrence.compact_theta_sin[..lane_count]
            } else {
                &theta_sin[block_start..block_end]
            };
            let block_cosine = if compact {
                &recurrence.compact_theta_cos[..lane_count]
            } else {
                &theta_cos[block_start..block_end]
            };
            let q_products_safe = T::scalar_analysis_products_safe(
                &recurrence.analysis_same_real[..lane_count],
                &recurrence.analysis_same_imag[..lane_count],
                &recurrence.analysis_opposite_real[..lane_count],
                &recurrence.analysis_opposite_imag[..lane_count],
                block_cosine,
            );
            let u_products_safe = T::scalar_analysis_products_safe(
                &recurrence.spin_previous_plus[..lane_count],
                &recurrence.spin_current_plus[..lane_count],
                &recurrence.spin_previous_minus[..lane_count],
                &recurrence.spin_current_minus[..lane_count],
                block_cosine,
            );
            if !q_products_safe || !u_products_safe {
                return Ok(false);
            }
            #[cfg(all(test, feature = "parallel"))]
            let mut block_range_status = (false, false);
            let fused_block = fused_analysis_enabled
                && Self::try_fused_spin_scalar_analysis_pair_block(
                    arch,
                    m,
                    scalar_lmax,
                    self.sectoral_norms[m],
                    block_sine,
                    block_cosine,
                    [q_products_safe, u_products_safe],
                    [
                        &recurrence.analysis_same_real[..lane_count],
                        &recurrence.spin_previous_plus[..lane_count],
                    ],
                    [
                        &recurrence.analysis_same_imag[..lane_count],
                        &recurrence.spin_current_plus[..lane_count],
                    ],
                    [
                        &recurrence.analysis_opposite_real[..lane_count],
                        &recurrence.spin_previous_minus[..lane_count],
                    ],
                    [
                        &recurrence.analysis_opposite_imag[..lane_count],
                        &recurrence.spin_current_minus[..lane_count],
                    ],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.fast_scalar_first[..scalar_coefficient_count],
                    &mut recurrence.fast_scalar_second[..scalar_coefficient_count],
                    &mut recurrence.fast_spin_first,
                    &mut recurrence.fast_spin_second,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                )?;
            #[cfg(all(test, feature = "parallel"))]
            if fused_block {
                block_range_status.0 = true;
            }
            if !fused_block {
                #[cfg(all(test, feature = "parallel"))]
                let profile_range_status = profiling.then_some(&mut block_range_status);
                let paired_used = Self::accumulate_spin_scalar_analysis_pair_block(
                    arch,
                    m,
                    scalar_lmax,
                    self.sectoral_norms[m],
                    block_sine,
                    block_cosine,
                    q_products_safe,
                    u_products_safe,
                    &recurrence.analysis_same_real[..lane_count],
                    &recurrence.analysis_same_imag[..lane_count],
                    &recurrence.analysis_opposite_real[..lane_count],
                    &recurrence.analysis_opposite_imag[..lane_count],
                    &recurrence.spin_previous_plus[..lane_count],
                    &recurrence.spin_current_plus[..lane_count],
                    &recurrence.spin_previous_minus[..lane_count],
                    &recurrence.spin_current_minus[..lane_count],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                    #[cfg(all(test, feature = "parallel"))]
                    profile_range_status,
                    &mut recurrence.fast_scalar_first[..scalar_coefficient_count],
                    &mut recurrence.fast_scalar_second[..scalar_coefficient_count],
                )?;
                if !paired_used {
                    #[cfg(all(test, feature = "parallel"))]
                    if profiling {
                        recurrence
                            .record_analysis_leaf_block(block_range_status.0, block_range_status.1);
                    }
                    return Ok(false);
                }
            }
            #[cfg(all(test, feature = "parallel"))]
            if profiling {
                recurrence.record_analysis_leaf_block(block_range_status.0, block_range_status.1);
            }
        }

        let mut previous_even_first = Complex::new(T::zero(), T::zero());
        let mut previous_even_second = Complex::new(T::zero(), T::zero());
        for (pair_index, l) in (m..=scalar_lmax).step_by(2).enumerate() {
            let even_first = recurrence.fast_scalar_first[l];
            let even_second = recurrence.fast_scalar_second[l];
            let odd_first = if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let odd_second = if l < scalar_lmax {
                recurrence.fast_scalar_second[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let previous_factor = if pair_index == 0 {
                T::zero()
            } else {
                recurrence.spin_alpha[pair_index - 1] * recurrence.epsilon[l]
            };
            recurrence.fast_scalar_first[l] = even_first
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_first * previous_factor;
            recurrence.fast_scalar_second[l] = even_second
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_second * previous_factor;
            if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1] = odd_first * recurrence.spin_alpha[pair_index];
                recurrence.fast_scalar_second[l + 1] =
                    odd_second * recurrence.spin_alpha[pair_index];
            }
            previous_even_first = even_first;
            previous_even_second = even_second;
        }

        let coefficient_count = self.harmonics.lmax() - m + 1;
        let fast_e = &mut recurrence.fast_spin_first[..coefficient_count];
        let fast_b = &mut recurrence.fast_spin_second[..coefficient_count];
        scalar_to_spin1(
            m,
            self.harmonics.lmax(),
            &recurrence.fast_scalar_first[..scalar_coefficient_count],
            &recurrence.fast_scalar_second[..scalar_coefficient_count],
            fast_e,
            fast_b,
        );
        for index in 0..coefficient_count {
            e_coefficients[index] += fast_e[index];
            if let Some(b_coefficients) = b_coefficients.as_deref_mut() {
                b_coefficients[index] += fast_b[index];
            }
        }
        Ok(true)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The bounded spin-1 table projection receives order geometry and worker-owned scratch."
    )]
    fn analyze_spin_scalar_table_order(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        spectrum_len: usize,
        longitude_count: T,
        phase: Complex<T>,
        ring_weights: &[f64],
        normalize_fourier: bool,
        m: usize,
        theta_sin: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        e_coefficients: &mut [Complex<T>],
        mut b_coefficients: Option<&mut [Complex<T>]>,
        table: &crate::legendre::ScalarLegendreTable<T>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<bool, ShtError> {
        if !T::supports_precomputed_scalar() || !T::supports_paired_scalar() {
            return Ok(false);
        }
        let lmax = self.harmonics.lmax();
        let scalar_lmax = lmax + 1;
        let scalar_coefficient_count = scalar_lmax + 1;
        recurrence.fast_scalar_first[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        recurrence.fast_scalar_second[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));

        let ntheta = theta_sin.len();
        let north_ring_count = table.north_ring_count();
        let unit_cosine = [T::one(); LATITUDE_BLOCK_SIZE];
        let weighted_north_q = &mut recurrence.weighted_spin_q_spectrum;
        let weighted_south_q = &mut recurrence.weighted_spin_u_spectrum;
        let weighted_north_u_real = &mut recurrence.analysis_same_real;
        let weighted_north_u_imag = &mut recurrence.analysis_same_imag;
        let weighted_south_u_real = &mut recurrence.analysis_opposite_real;
        let weighted_south_u_imag = &mut recurrence.analysis_opposite_imag;
        let q_real = &mut recurrence.previous;
        let q_imag = &mut recurrence.current;
        let u_real = &mut recurrence.spin_previous_plus;
        let u_imag = &mut recurrence.spin_previous_minus;
        let exponents = &mut recurrence.exponents;

        for block_start in (0..north_ring_count).step_by(LATITUDE_BLOCK_SIZE) {
            let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(north_ring_count);
            let block_len = block_end - block_start;
            let mut active = false;
            exponents[..block_len].fill(0);
            for (lane, north_ring) in (block_start..block_end).enumerate() {
                let south_ring = ntheta - 1 - north_ring;
                let north_active = m <= ring_order_limits[north_ring];
                let south_active = south_ring != north_ring && m <= ring_order_limits[south_ring];
                active |= north_active || south_active;

                let weighted = |ring: usize, spectrum: Complex<T>| {
                    let mut scale = T::from_f64(ring_weights[ring])
                        .expect("ring weight is representable")
                        / theta_sin[ring];
                    if normalize_fourier {
                        scale /= longitude_count;
                    }
                    spectrum * phase * scale
                };
                let spectrum_start = north_ring * spectrum_len;
                let north_q = if north_active {
                    weighted(north_ring, q_spectra[spectrum_start + m])
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let north_u = if north_active {
                    weighted(north_ring, u_spectra[spectrum_start + m])
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let south_spectrum_start = south_ring * spectrum_len;
                let south_q = if south_active {
                    weighted(south_ring, q_spectra[south_spectrum_start + m])
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let south_u = if south_active {
                    weighted(south_ring, u_spectra[south_spectrum_start + m])
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                if !table.input_is_safe(north_q.re)
                    || !table.input_is_safe(north_q.im)
                    || !table.input_is_safe(south_q.re)
                    || !table.input_is_safe(south_q.im)
                    || !table.input_is_safe(north_u.re)
                    || !table.input_is_safe(north_u.im)
                    || !table.input_is_safe(south_u.re)
                    || !table.input_is_safe(south_u.im)
                {
                    return Ok(false);
                }
                weighted_north_q[lane] = north_q;
                weighted_south_q[lane] = south_q;
                weighted_north_u_real[lane] = north_u.re;
                weighted_north_u_imag[lane] = north_u.im;
                weighted_south_u_real[lane] = south_u.re;
                weighted_south_u_imag[lane] = south_u.im;
            }
            if !active {
                continue;
            }

            for lane in 0..block_len {
                let north_q = weighted_north_q[lane];
                let south_q = weighted_south_q[lane];
                weighted_north_q[lane] = north_q + south_q;
                weighted_south_q[lane] = north_q - south_q;
                let north_u =
                    Complex::new(weighted_north_u_real[lane], weighted_north_u_imag[lane]);
                let south_u =
                    Complex::new(weighted_south_u_real[lane], weighted_south_u_imag[lane]);
                weighted_north_u_real[lane] = north_u.re + south_u.re;
                weighted_north_u_imag[lane] = north_u.im + south_u.im;
                weighted_south_u_real[lane] = north_u.re - south_u.re;
                weighted_south_u_imag[lane] = north_u.im - south_u.im;
                q_real[lane] = weighted_north_q[lane].re;
                q_imag[lane] = weighted_north_q[lane].im;
                u_real[lane] = weighted_north_u_real[lane];
                u_imag[lane] = weighted_north_u_imag[lane];
            }
            let products_safe = T::scalar_analysis_products_safe(
                &q_real[..block_len],
                &q_imag[..block_len],
                &u_real[..block_len],
                &u_imag[..block_len],
                &unit_cosine[..block_len],
            );
            if !products_safe {
                return Ok(false);
            }

            for degree_offset in 0..(scalar_lmax - m + 1) {
                let (q_sums, u_real_sums, u_imag_sums) = if degree_offset.is_multiple_of(2) {
                    (
                        &weighted_north_q[..],
                        &weighted_north_u_real[..],
                        &weighted_north_u_imag[..],
                    )
                } else {
                    (
                        &weighted_south_q[..],
                        &weighted_south_u_real[..],
                        &weighted_south_u_imag[..],
                    )
                };
                for lane in 0..block_len {
                    q_real[lane] = q_sums[lane].re;
                    q_imag[lane] = q_sums[lane].im;
                    u_real[lane] = u_real_sums[lane];
                    u_imag[lane] = u_imag_sums[lane];
                }
                let basis = &table.degree_lanes(m, degree_offset)[block_start..block_end];
                let (q_sum, u_sum) = T::accumulate_scalar_analysis_pair(
                    arch,
                    true,
                    true,
                    &q_real[..block_len],
                    &q_imag[..block_len],
                    &u_real[..block_len],
                    &u_imag[..block_len],
                    basis,
                    &exponents[..block_len],
                    &unit_cosine[..block_len],
                );
                let degree = m + degree_offset;
                recurrence.fast_scalar_first[degree] += q_sum;
                recurrence.fast_scalar_second[degree] += u_sum;
            }
        }

        let coefficient_count = lmax - m + 1;
        let fast_e = &mut recurrence.fast_spin_first[..coefficient_count];
        let fast_b = &mut recurrence.fast_spin_second[..coefficient_count];
        scalar_to_spin1(
            m,
            lmax,
            &recurrence.fast_scalar_first[..scalar_coefficient_count],
            &recurrence.fast_scalar_second[..scalar_coefficient_count],
            fast_e,
            fast_b,
        );
        for index in 0..coefficient_count {
            e_coefficients[index] += fast_e[index];
            if let Some(b_coefficients) = b_coefficients.as_deref_mut() {
                b_coefficients[index] += fast_b[index];
            }
        }
        Ok(true)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The spin-1 fast analysis segment needs its Fourier weights, latitude lanes, and worker scratch."
    )]
    fn analyze_spin_scalar_fast_order(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        spectrum_len: usize,
        longitude_count: T,
        phase: Complex<T>,
        ring_weights: Option<&[f64]>,
        normalize_fourier: bool,
        m: usize,
        ring_start: usize,
        ring_end: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        e_coefficients: &mut [Complex<T>],
        mut b_coefficients: Option<&mut [Complex<T>]>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let scalar_lmax = self.harmonics.lmax() + 1;
        let scalar_coefficient_count = scalar_lmax + 1;
        let fused_analysis_enabled =
            self.harmonics.lmax() >= SPIN1_FUSED_ANALYSIS_MIN_LMAX && single_threaded_execution();
        recurrence.fast_scalar_first[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        recurrence.fast_scalar_second[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
        let paired_factors_ready = T::fill_paired_scalar_factors(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        );
        debug_assert!(paired_factors_ready);

        for block_start in (ring_start..ring_end).step_by(LATITUDE_BLOCK_SIZE) {
            let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(ring_end);
            if ring_order_limits[block_start..block_end]
                .iter()
                .all(|&limit| m > limit)
            {
                continue;
            }
            let block_len = block_end - block_start;
            let compaction_enabled = self.harmonics.lmax() >= 127;
            let mut active_lanes = 0;
            for lane in 0..block_len {
                let ring = block_start + lane;
                if m <= ring_order_limits[ring] {
                    if compaction_enabled {
                        active_lanes += 1;
                    }
                    let spectrum_start = ring * spectrum_len;
                    let ring_weight = ring_weights.map_or(T::one(), |weights| {
                        T::from_f64(weights[ring]).expect("ring weight is representable")
                    });
                    let mut scale = ring_weight / theta_sin[ring];
                    if normalize_fourier {
                        scale /= longitude_count;
                    }
                    recurrence.weighted_spin_q_spectrum[lane] =
                        q_spectra[spectrum_start + m] * phase * scale;
                    recurrence.weighted_spin_u_spectrum[lane] =
                        u_spectra[spectrum_start + m] * phase * scale;
                } else {
                    recurrence.weighted_spin_q_spectrum[lane] = Complex::new(T::zero(), T::zero());
                    recurrence.weighted_spin_u_spectrum[lane] = Complex::new(T::zero(), T::zero());
                }
            }
            let compact = compaction_enabled && active_lanes < block_len;
            if compact {
                let mut compact_len = 0;
                for lane in 0..block_len {
                    let ring = block_start + lane;
                    if m <= ring_order_limits[ring] {
                        recurrence.compact_theta_sin[compact_len] = theta_sin[ring];
                        recurrence.compact_theta_cos[compact_len] = theta_cos[ring];
                        let weighted_q = recurrence.weighted_spin_q_spectrum[lane];
                        let weighted_u = recurrence.weighted_spin_u_spectrum[lane];
                        recurrence.weighted_spin_q_spectrum[compact_len] = weighted_q;
                        recurrence.weighted_spin_u_spectrum[compact_len] = weighted_u;
                        compact_len += 1;
                    }
                }
                debug_assert_eq!(compact_len, active_lanes);
            }
            let (block_sine, block_cosine, lane_count) = if compact {
                (
                    &recurrence.compact_theta_sin[..active_lanes],
                    &recurrence.compact_theta_cos[..active_lanes],
                    active_lanes,
                )
            } else {
                (
                    &theta_sin[block_start..block_end],
                    &theta_cos[block_start..block_end],
                    block_len,
                )
            };
            for lane in 0..lane_count {
                recurrence.analysis_same_real[lane] = recurrence.weighted_spin_q_spectrum[lane].re;
                recurrence.analysis_same_imag[lane] = recurrence.weighted_spin_q_spectrum[lane].im;
                recurrence.analysis_opposite_real[lane] =
                    recurrence.weighted_spin_u_spectrum[lane].re;
                recurrence.analysis_opposite_imag[lane] =
                    recurrence.weighted_spin_u_spectrum[lane].im;
            }
            let q_real = &recurrence.analysis_same_real[..lane_count];
            let q_imag = &recurrence.analysis_same_imag[..lane_count];
            let u_real = &recurrence.analysis_opposite_real[..lane_count];
            let u_imag = &recurrence.analysis_opposite_imag[..lane_count];
            let q_products_safe =
                T::scalar_analysis_products_safe(q_real, q_imag, q_real, q_imag, block_cosine);
            let u_products_safe =
                T::scalar_analysis_products_safe(u_real, u_imag, u_real, u_imag, block_cosine);
            let fused_block = fused_analysis_enabled
                && Self::try_fused_spin_scalar_analysis_pair_block(
                    arch,
                    m,
                    scalar_lmax,
                    self.sectoral_norms[m],
                    block_sine,
                    block_cosine,
                    [q_products_safe, u_products_safe],
                    [q_real, u_real],
                    [q_imag, u_imag],
                    [q_real, u_real],
                    [q_imag, u_imag],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.fast_scalar_first[..scalar_coefficient_count],
                    &mut recurrence.fast_scalar_second[..scalar_coefficient_count],
                    &mut recurrence.fast_spin_first,
                    &mut recurrence.fast_spin_second,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                )?;
            if !fused_block {
                let paired_used = Self::accumulate_spin_scalar_analysis_pair_block(
                    arch,
                    m,
                    scalar_lmax,
                    self.sectoral_norms[m],
                    block_sine,
                    block_cosine,
                    q_products_safe,
                    u_products_safe,
                    q_real,
                    q_imag,
                    q_real,
                    q_imag,
                    u_real,
                    u_imag,
                    u_real,
                    u_imag,
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                    #[cfg(all(test, feature = "parallel"))]
                    None,
                    &mut recurrence.fast_scalar_first[..scalar_coefficient_count],
                    &mut recurrence.fast_scalar_second[..scalar_coefficient_count],
                )?;
                debug_assert!(paired_used);
            }
        }

        let mut previous_even_first = Complex::new(T::zero(), T::zero());
        let mut previous_even_second = Complex::new(T::zero(), T::zero());
        for (pair_index, l) in (m..=scalar_lmax).step_by(2).enumerate() {
            let even_first = recurrence.fast_scalar_first[l];
            let even_second = recurrence.fast_scalar_second[l];
            let odd_first = if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let odd_second = if l < scalar_lmax {
                recurrence.fast_scalar_second[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let previous_factor = if pair_index == 0 {
                T::zero()
            } else {
                recurrence.spin_alpha[pair_index - 1] * recurrence.epsilon[l]
            };
            recurrence.fast_scalar_first[l] = even_first
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_first * previous_factor;
            recurrence.fast_scalar_second[l] = even_second
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_second * previous_factor;
            if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1] = odd_first * recurrence.spin_alpha[pair_index];
                recurrence.fast_scalar_second[l + 1] =
                    odd_second * recurrence.spin_alpha[pair_index];
            }
            previous_even_first = even_first;
            previous_even_second = even_second;
        }

        let coefficient_count = self.harmonics.lmax() - m + 1;
        let fast_e = &mut recurrence.fast_spin_first[..coefficient_count];
        let fast_b = &mut recurrence.fast_spin_second[..coefficient_count];
        scalar_to_spin1(
            m,
            self.harmonics.lmax(),
            &recurrence.fast_scalar_first[..scalar_coefficient_count],
            &recurrence.fast_scalar_second[..scalar_coefficient_count],
            fast_e,
            fast_b,
        );
        for index in 0..coefficient_count {
            e_coefficients[index] += fast_e[index];
            if let Some(b_coefficients) = b_coefficients.as_deref_mut() {
                b_coefficients[index] += fast_b[index];
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Spin-2 scalar analysis needs the Fourier weights, latitude lanes, and worker scratch."
    )]
    fn analyze_spin2_scalar_fast_order(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        spectrum_len: usize,
        longitude_count: T,
        phase: Complex<T>,
        ring_weights: &[f64],
        normalize_fourier: bool,
        m: usize,
        ring_start: usize,
        ring_end: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        e_coefficients: &mut [Complex<T>],
        mut b_coefficients: Option<&mut [Complex<T>]>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let scalar_lmax = lmax + 2;
        let scalar_coefficient_count = scalar_lmax + 1;
        recurrence.fast_scalar_first[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        recurrence.fast_scalar_second[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
        let paired_factors_ready = T::fill_paired_scalar_factors(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        );
        debug_assert!(paired_factors_ready);

        for block_start in (ring_start..ring_end).step_by(LATITUDE_BLOCK_SIZE) {
            let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(ring_end);
            if ring_order_limits[block_start..block_end]
                .iter()
                .all(|&limit| m > limit)
            {
                continue;
            }
            let block_len = block_end - block_start;
            for lane in 0..block_len {
                let ring = block_start + lane;
                if m <= ring_order_limits[ring] {
                    let spectrum_start = ring * spectrum_len;
                    let sine = theta_sin[ring];
                    let mut scale = T::from_f64(ring_weights[ring])
                        .expect("ring weight is representable")
                        / (sine * sine);
                    if normalize_fourier {
                        scale /= longitude_count;
                    }
                    let weighted_q = q_spectra[spectrum_start + m] * phase * scale;
                    let weighted_u = u_spectra[spectrum_start + m] * phase * scale;
                    recurrence.analysis_same_real[lane] = weighted_q.re;
                    recurrence.analysis_same_imag[lane] = weighted_q.im;
                    recurrence.analysis_opposite_real[lane] = weighted_u.re;
                    recurrence.analysis_opposite_imag[lane] = weighted_u.im;
                } else {
                    recurrence.analysis_same_real[lane] = T::zero();
                    recurrence.analysis_same_imag[lane] = T::zero();
                    recurrence.analysis_opposite_real[lane] = T::zero();
                    recurrence.analysis_opposite_imag[lane] = T::zero();
                }
            }
            let q_real = &recurrence.analysis_same_real[..block_len];
            let q_imag = &recurrence.analysis_same_imag[..block_len];
            let u_real = &recurrence.analysis_opposite_real[..block_len];
            let u_imag = &recurrence.analysis_opposite_imag[..block_len];
            let block_cosine = &theta_cos[block_start..block_end];
            let q_products_safe =
                T::scalar_analysis_products_safe(q_real, q_imag, q_real, q_imag, block_cosine);
            let u_products_safe =
                T::scalar_analysis_products_safe(u_real, u_imag, u_real, u_imag, block_cosine);
            let fused_count = scalar_lmax - m + 1;
            let fused_block = if q_products_safe && u_products_safe {
                T::fused_scalar_analysis_pair_channels_block(
                    arch,
                    m,
                    scalar_lmax,
                    &theta_sin[block_start..block_end],
                    block_cosine,
                    self.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    [q_real, u_real],
                    [q_imag, u_imag],
                    [q_real, u_real],
                    [q_imag, u_imag],
                    &mut recurrence.fast_spin_first[..fused_count],
                    &mut recurrence.fast_spin_second[..fused_count],
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                )?
            } else {
                false
            };
            if fused_block {
                for offset in 0..fused_count {
                    let degree = m + offset;
                    recurrence.fast_scalar_first[degree] += recurrence.fast_spin_first[offset];
                    recurrence.fast_scalar_second[degree] += recurrence.fast_spin_second[offset];
                }
            } else {
                let paired_used = T::for_each_y_lm_paired_status_dispatched(
                    arch,
                    m,
                    scalar_lmax,
                    &theta_sin[block_start..block_end],
                    &theta_cos[block_start..block_end],
                    self.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                    |l, local_ring_start, basis, exponents, normal_range| {
                        let mut first_even = recurrence.fast_scalar_first[l];
                        let mut second_even = recurrence.fast_scalar_second[l];
                        let mut first_odd = if l < scalar_lmax {
                            recurrence.fast_scalar_first[l + 1]
                        } else {
                            Complex::new(T::zero(), T::zero())
                        };
                        let mut second_odd = if l < scalar_lmax {
                            recurrence.fast_scalar_second[l + 1]
                        } else {
                            Complex::new(T::zero(), T::zero())
                        };
                        let lane_end = local_ring_start + basis.len();
                        let lanes = local_ring_start..lane_end;
                        let cosine =
                            &block_cosine[local_ring_start..local_ring_start + basis.len()];
                        let (q_even, q_odd) = T::accumulate_scalar_analysis_pair(
                            arch,
                            q_products_safe,
                            normal_range,
                            &q_real[lanes.clone()],
                            &q_imag[lanes.clone()],
                            &q_real[lanes.clone()],
                            &q_imag[lanes.clone()],
                            basis,
                            exponents,
                            cosine,
                        );
                        let (u_even, u_odd) = T::accumulate_scalar_analysis_pair(
                            arch,
                            u_products_safe,
                            normal_range,
                            &u_real[lanes.clone()],
                            &u_imag[lanes.clone()],
                            &u_real[lanes.clone()],
                            &u_imag[lanes],
                            basis,
                            exponents,
                            cosine,
                        );
                        first_even += q_even;
                        second_even += u_even;
                        if l < scalar_lmax {
                            first_odd += q_odd;
                            second_odd += u_odd;
                        }
                        recurrence.fast_scalar_first[l] = first_even;
                        recurrence.fast_scalar_second[l] = second_even;
                        if l < scalar_lmax {
                            recurrence.fast_scalar_first[l + 1] = first_odd;
                            recurrence.fast_scalar_second[l + 1] = second_odd;
                        }
                        Ok(())
                    },
                )?;
                debug_assert!(paired_used);
            }
        }

        let mut previous_even_first = Complex::new(T::zero(), T::zero());
        let mut previous_even_second = Complex::new(T::zero(), T::zero());
        for (pair_index, l) in (m..=scalar_lmax).step_by(2).enumerate() {
            let even_first = recurrence.fast_scalar_first[l];
            let even_second = recurrence.fast_scalar_second[l];
            let odd_first = if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let odd_second = if l < scalar_lmax {
                recurrence.fast_scalar_second[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let previous_factor = if pair_index == 0 {
                T::zero()
            } else {
                recurrence.spin_alpha[pair_index - 1] * recurrence.epsilon[l]
            };
            recurrence.fast_scalar_first[l] = even_first
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_first * previous_factor;
            recurrence.fast_scalar_second[l] = even_second
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_second * previous_factor;
            if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1] = odd_first * recurrence.spin_alpha[pair_index];
                recurrence.fast_scalar_second[l + 1] =
                    odd_second * recurrence.spin_alpha[pair_index];
            }
            previous_even_first = even_first;
            previous_even_second = even_second;
        }

        scalar_to_spin2(
            m,
            lmax,
            &mut recurrence.fast_scalar_first[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_scalar_second[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_spin_first[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_spin_second[..self.spin_fast_coefficient_count],
            &self.spin2_fast_f1,
            &self.spin2_fast_f2,
        );
        for l in m..=lmax {
            let index = l - m;
            e_coefficients[index] += recurrence.fast_scalar_first[l];
            if let Some(b_coefficients) = b_coefficients.as_deref_mut() {
                b_coefficients[index] += recurrence.fast_scalar_second[l];
            }
        }
        Ok(())
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "Dynamic spin-analysis jobs share immutable geometry and own disjoint E/B coefficient slices."
    )]
    fn analyze_spin_order_jobs_dynamic(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_weights: Option<&[f64]>,
        normalize_fourier: bool,
        analysis_ring_scales: Option<&[T]>,
        spectra_preweighted: bool,
        recurrence_spin: usize,
        derivative_norm: bool,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        e_alm: &mut [Complex<T>],
        b_alm: Option<&mut [Complex<T>]>,
        recurrence: &mut [RecurrenceWorkspace<T>],
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let order_count = self.harmonics.mmax() + 1;
        if order_count <= 1 || recurrence.len() <= 1 {
            return self.analyze_spin_order_group(
                q_spectra,
                u_spectra,
                nphi,
                spectrum_len,
                phi0,
                ring_weights,
                normalize_fourier,
                analysis_ring_scales,
                spectra_preweighted,
                recurrence_spin,
                derivative_norm,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                0,
                order_count,
                e_alm,
                b_alm,
                recurrence
                    .first_mut()
                    .expect("each spin-analysis group has at least one recurrence workspace"),
            );
        }

        let mut order_jobs = VecDeque::new();
        if order_jobs.try_reserve_exact(order_count).is_err() {
            return self.analyze_spin_order_group(
                q_spectra,
                u_spectra,
                nphi,
                spectrum_len,
                phi0,
                ring_weights,
                normalize_fourier,
                analysis_ring_scales,
                spectra_preweighted,
                recurrence_spin,
                derivative_norm,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                0,
                order_count,
                e_alm,
                b_alm,
                &mut recurrence[0],
            );
        }

        let mut remaining_e = e_alm;
        let mut remaining_b = b_alm;
        for m in 0..order_count {
            let coefficient_count = lmax - m + 1;
            let (e_coefficients, e_tail) = remaining_e.split_at_mut(coefficient_count);
            remaining_e = e_tail;
            let b_coefficients = match remaining_b.take() {
                Some(b_remaining) => {
                    let (b_order, b_tail) = b_remaining.split_at_mut(coefficient_count);
                    remaining_b = Some(b_tail);
                    Some(b_order)
                }
                None => None,
            };
            order_jobs.push_back((m, e_coefficients, b_coefficients));
        }
        debug_assert!(remaining_e.is_empty());
        debug_assert!(remaining_b.as_ref().is_none_or(|tail| tail.is_empty()));

        // Match DUCC's dynamic harmonic-order schedule. Each task owns one
        // packed coefficient order and reuses its recurrence workspace.
        let order_jobs = Mutex::new(order_jobs);
        let failed = AtomicBool::new(false);
        let failure = Mutex::new(None::<ShtError>);
        recurrence.par_iter_mut().for_each(|worker| {
            loop {
                if failed.load(Ordering::Relaxed) {
                    break;
                }
                let job = order_jobs
                    .lock()
                    .expect("spin analysis order queue is not poisoned")
                    .pop_front();
                let Some((m, e_coefficients, b_coefficients)) = job else {
                    break;
                };
                if let Err(error) = self.analyze_spin_order_group(
                    q_spectra,
                    u_spectra,
                    nphi,
                    spectrum_len,
                    phi0,
                    ring_weights,
                    normalize_fourier,
                    analysis_ring_scales,
                    spectra_preweighted,
                    recurrence_spin,
                    derivative_norm,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    m,
                    m + 1,
                    e_coefficients,
                    b_coefficients,
                    worker,
                ) {
                    failed.store(true, Ordering::Relaxed);
                    let mut first_error = failure
                        .lock()
                        .expect("spin analysis error slot is not poisoned");
                    first_error.get_or_insert(error);
                    break;
                }
            }
        });

        if failed.load(Ordering::Relaxed) {
            return Err(failure
                .into_inner()
                .expect("spin analysis error slot is not poisoned")
                .expect("a failed spin-analysis job records its error"));
        }
        Ok(())
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
    reason = "The polar-lane split uses one block's geometry, spectra and worker scratch."
)]
#[cold]
#[inline(never)]
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
            return Ok(None);
        }
    }
    for lane in 0..transition_len {
        let ring = usize::try_from(recurrence.spin_exponents_plus[lane])
            .expect("saved ring index is nonnegative");
        let spectrum_index = ring * spectrum_len + m;
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
    Ok(Some(polar_len))
}

#[cfg(test)]
mod tests;
