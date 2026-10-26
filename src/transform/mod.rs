// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(feature = "parallel")]
use std::collections::VecDeque;
#[cfg(any(test, feature = "parallel"))]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{ShtError, check_len};
use crate::grid::GridKind;
use crate::legendre::{
    LATITUDE_BLOCK_SIZE, LegendreOperation, LegendreStrategy, SCALAR_ANALYSIS_PAIR_TILE_SIZE,
    ShtScalar, longitude_phase, recurrence_scratch_len, sin_cos_from_f64,
};
use crate::plan::{ScalarLatitudeGeometry, ShtPlan};
use crate::workspace::{RecurrenceWorkspace, ShtWorkspace};
#[cfg(feature = "parallel")]
use scheduling::{run_dynamic_order_jobs, weighted_order_midpoint};

mod fft_rows;
mod scalar;
#[cfg(feature = "parallel")]
pub(crate) mod scheduling;
pub(crate) mod spin;

const SCALAR_TRANSITION_MIN_LMAX: usize = 1023;
#[cfg(feature = "parallel")]
// Complex64 occupies 16 bytes, so four adjacent orders span 64 bytes and
// reduce cross-worker sharing on 64-byte cache lines in the ring-major output.
const SCALAR_ORDERED_SYNTHESIS_ORDER_BLOCK_SIZE: usize = 4;

#[cfg(test)]
static SCALAR_TABLE_SYNTHESIS_CALLS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static SCALAR_RECURRENCE_SYNTHESIS_CALLS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static SCALAR_TABLE_ANALYSIS_CALLS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static SCALAR_RECURRENCE_ANALYSIS_CALLS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static PAIRED_ODD_FFT_DISABLED: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static PAIRED_FORWARD_SUCCESSES: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static PAIRED_FORWARD_DECLINES: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static PAIRED_INVERSE_SUCCESSES: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static PAIRED_INVERSE_DECLINES: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn set_paired_odd_fft_disabled_for_test(disabled: bool) {
    PAIRED_ODD_FFT_DISABLED.store(disabled, Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn take_paired_odd_fft_row_counts() -> [usize; 4] {
    [
        PAIRED_FORWARD_SUCCESSES.swap(0, Ordering::Relaxed),
        PAIRED_FORWARD_DECLINES.swap(0, Ordering::Relaxed),
        PAIRED_INVERSE_SUCCESSES.swap(0, Ordering::Relaxed),
        PAIRED_INVERSE_DECLINES.swap(0, Ordering::Relaxed),
    ]
}

#[cfg(all(test, feature = "parallel"))]
pub(crate) fn take_scalar_legendre_benchmark_route_counts() -> [usize; 4] {
    [
        SCALAR_TABLE_SYNTHESIS_CALLS.swap(0, Ordering::Relaxed),
        SCALAR_RECURRENCE_SYNTHESIS_CALLS.swap(0, Ordering::Relaxed),
        SCALAR_TABLE_ANALYSIS_CALLS.swap(0, Ordering::Relaxed),
        SCALAR_RECURRENCE_ANALYSIS_CALLS.swap(0, Ordering::Relaxed),
    ]
}

#[cfg(feature = "parallel")]
#[derive(Clone, Copy)]
struct ScalarSynthesisCoefficientCacheLayout {
    coefficient_count: usize,
    row_stride: usize,
    half_stride: usize,
}

#[derive(Clone, Copy)]
struct ScalarSynthesisCoefficientCache<'a, T> {
    values: &'a [Complex<T>],
    row_stride: usize,
    half_stride: usize,
}

fn scalar_ordered_scratch<T: ShtScalar>(
    recurrence: &mut RecurrenceWorkspace<T>,
) -> crate::legendre::scalar_ordered::ScalarOrderedScratch<'_, T> {
    crate::legendre::scalar_ordered::ScalarOrderedScratch {
        epsilon: &mut recurrence.epsilon,
        alpha: &mut recurrence.spin_alpha,
        coefficient_a: &mut recurrence.spin_coefficient_a,
        coefficient_b: &mut recurrence.spin_coefficient_b,
        coefficient_even: &mut recurrence.fast_spin_first,
        coefficient_odd: &mut recurrence.fast_spin_second,
        packed_sin: &mut recurrence.compact_theta_sin,
        packed_csq: &mut recurrence.spin_current_plus,
        previous: &mut recurrence.previous,
        current: &mut recurrence.current,
        scale: &mut recurrence.exponents,
        corfac: &mut recurrence.scalar_order_correction,
        sum_even_re: &mut recurrence.analysis_same_real,
        sum_even_im: &mut recurrence.analysis_same_imag,
        sum_odd_re: &mut recurrence.analysis_opposite_real,
        sum_odd_im: &mut recurrence.analysis_opposite_imag,
    }
}

#[inline]
fn finalize_paired_scalar_analysis_coefficients<T: ShtScalar>(
    coefficients: &mut [Complex<T>],
    m: usize,
    lmax: usize,
    epsilon: &[T],
    alpha: &[T],
) {
    debug_assert_eq!(coefficients.len(), lmax - m + 1);
    let mut previous_even = Complex::new(T::zero(), T::zero());
    let mut previous_alpha = T::zero();
    for (pair_index, l) in (m..=lmax).step_by(2).enumerate() {
        let offset = l - m;
        let even = coefficients[offset];
        let odd = if l < lmax {
            coefficients[offset + 1]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let previous = previous_even * (previous_alpha * epsilon[l]);
        let pair_alpha = alpha[pair_index];
        coefficients[offset] = even * (pair_alpha * epsilon[l + 1]) + previous;
        if l < lmax {
            coefficients[offset + 1] = odd * pair_alpha;
        }
        previous_even = even;
        previous_alpha = pair_alpha;
    }
}

struct ScalarAnalysisOrderPreparation<'a, T> {
    analysis_spectra: &'a [Complex<T>],
    spectrum_len: usize,
    phi0: f64,
    ring_order_limits: &'a [usize],
    scalar_table_selected: bool,
    ring_weights: Option<&'a [f64]>,
    longitude_count: T,
    scalar_table_input_limit: Option<T>,
}

impl<T: ShtScalar> ScalarAnalysisOrderPreparation<'_, T> {
    #[inline]
    fn prepare_order(&self, order: usize, order_spectra: &mut [Complex<T>]) -> bool {
        debug_assert_eq!(order_spectra.len(), self.ring_order_limits.len());
        if let Some(ring_weights) = self.ring_weights {
            debug_assert_eq!(ring_weights.len(), order_spectra.len());
        }

        let phase = if order == 0 {
            Complex::new(T::one(), T::zero())
        } else {
            longitude_phase::<T>(order, self.phi0, true)
        };
        let mut inputs_safe = true;
        for (ring, output) in order_spectra.iter_mut().enumerate() {
            let mut weighted = if self.scalar_table_selected && order > self.ring_order_limits[ring]
            {
                Complex::new(T::zero(), T::zero())
            } else {
                self.analysis_spectra[ring * self.spectrum_len + order] * phase
            };
            if let Some(ring_weights) = self.ring_weights {
                let weight = T::from_f64(ring_weights[ring]).expect("ring weight is representable");
                weighted *= weight / self.longitude_count;
            }
            if let Some(input_limit) = self.scalar_table_input_limit {
                inputs_safe &= crate::legendre::ScalarLegendreTable::<T>::input_value_is_safe(
                    weighted.re,
                    input_limit,
                ) && crate::legendre::ScalarLegendreTable::<T>::input_value_is_safe(
                    weighted.im,
                    input_limit,
                );
            }
            *output = weighted;
        }
        inputs_safe
    }

    fn prepare_serial(&self, order_spectra: &mut [Complex<T>]) -> bool {
        let latitude_count = self.ring_order_limits.len();
        debug_assert!(latitude_count > 0);
        debug_assert_eq!(order_spectra.len() % latitude_count, 0);
        let mut inputs_safe = true;
        for (order, order_row) in order_spectra.chunks_mut(latitude_count).enumerate() {
            let order_safe = self.prepare_order(order, order_row);
            inputs_safe &= order_safe;
        }
        inputs_safe
    }
}

#[derive(Clone, Copy)]
struct ScalarAnalysisDirectSpectra<'a, T> {
    values: &'a [Complex<T>],
    spectrum_len: usize,
    phi0: f64,
    ring_weights: Option<&'a [f64]>,
    longitude_count: T,
}

#[derive(Clone, Copy)]
enum ScalarAnalysisSpectra<'a, T> {
    OrderMajor {
        values: &'a [Complex<T>],
        latitude_count: usize,
    },
    Direct(ScalarAnalysisDirectSpectra<'a, T>),
}

#[derive(Clone, Copy)]
enum ScalarAnalysisOrderSpectra<'a, T> {
    Weighted(&'a [Complex<T>]),
    Direct {
        source: ScalarAnalysisDirectSpectra<'a, T>,
        order: usize,
        phase: Complex<T>,
    },
}

impl<'a, T: ShtScalar> ScalarAnalysisSpectra<'a, T> {
    #[inline]
    fn for_order(self, order: usize) -> ScalarAnalysisOrderSpectra<'a, T> {
        match self {
            Self::OrderMajor {
                values,
                latitude_count,
            } => ScalarAnalysisOrderSpectra::Weighted(
                &values[order * latitude_count..(order + 1) * latitude_count],
            ),
            Self::Direct(source) => ScalarAnalysisOrderSpectra::Direct {
                source,
                order,
                phase: if order == 0 {
                    Complex::new(T::one(), T::zero())
                } else {
                    longitude_phase::<T>(order, source.phi0, true)
                },
            },
        }
    }
}

impl<T: ShtScalar> ScalarAnalysisOrderSpectra<'_, T> {
    #[inline]
    fn value(self, ring: usize) -> Complex<T> {
        match self {
            Self::Weighted(values) => values[ring],
            Self::Direct {
                source,
                order,
                phase,
            } => {
                let mut value = source.values[ring * source.spectrum_len + order] * phase;
                if let Some(ring_weights) = source.ring_weights {
                    let weight =
                        T::from_f64(ring_weights[ring]).expect("ring weight is representable");
                    value *= weight / source.longitude_count;
                }
                value
            }
        }
    }
}

fn accumulate_precomputed_synthesis_order<T: ShtScalar>(
    plan: &ShtPlan<T>,
    table: &crate::legendre::ScalarLegendreTable<T>,
    alm: &[Complex<T>],
    ring_order_limits: &[usize],
    m: usize,
    order_row: &mut [Complex<T>],
) -> bool {
    let ntheta = order_row.len();
    let nphi = plan.grid.nphi();
    let lmax = plan.harmonics.lmax();
    for degree_offset in 0..=(lmax - m) {
        let l = m + degree_offset;
        let coefficient = alm[plan
            .harmonics
            .index(l, m)
            .expect("precomputed scalar order is within the harmonic layout")];
        if !table.input_is_safe(coefficient.re) || !table.input_is_safe(coefficient.im) {
            return false;
        }
        let parity = if (l + m).is_multiple_of(2) {
            T::one()
        } else {
            -T::one()
        };
        for (north_ring, &basis) in table.degree_lanes(m, degree_offset).iter().enumerate() {
            let south_ring = ntheta - 1 - north_ring;
            if m <= ring_order_limits[north_ring] {
                order_row[north_ring] += coefficient * basis;
            }
            if south_ring != north_ring && m <= ring_order_limits[south_ring] {
                order_row[south_ring] += coefficient * (parity * basis);
            }
        }
    }

    let phase = if m == 0 {
        Complex::new(T::one(), T::zero())
    } else {
        longitude_phase::<T>(m, plan.grid.phi0(), false)
    };
    let scale = phase * T::from_usize(nphi).expect("longitude count converts");
    for value in order_row {
        *value *= scale;
    }
    true
}

#[inline]
#[expect(
    clippy::too_many_arguments,
    reason = "The block preparation maps mirrored inputs into existing worker scratch."
)]
fn prepare_symmetric_analysis_pair_block<T: ShtScalar>(
    m: usize,
    latitude_count: usize,
    block_start: usize,
    block_len: usize,
    compact: bool,
    all_active: bool,
    order_spectra: ScalarAnalysisOrderSpectra<'_, T>,
    ring_order_limits: &[usize],
    theta_sin: &[T],
    theta_cos: &[T],
    theta_cos_squared: &[T],
    same_real: &mut [T],
    same_imag: &mut [T],
    opposite_real: &mut [T],
    opposite_imag: &mut [T],
    packed_north_ring: &mut [usize],
    compact_theta_sin: &mut [T],
    compact_theta_cos: &mut [T],
    compact_theta_cos_squared: &mut [T],
) -> usize {
    debug_assert_eq!(latitude_count, theta_cos_squared.len());
    debug_assert!(!compact || !all_active);
    if all_active {
        debug_assert!(block_start + block_len <= latitude_count.div_ceil(2));
        for lane in 0..block_len {
            let north_ring = block_start + lane;
            let south_ring = latitude_count - 1 - north_ring;
            let north = order_spectra.value(north_ring);
            let south = if south_ring != north_ring {
                order_spectra.value(south_ring)
            } else {
                Complex::new(T::zero(), T::zero())
            };
            same_real[lane] = north.re + south.re;
            same_imag[lane] = north.im + south.im;
            opposite_real[lane] = north.re - south.re;
            opposite_imag[lane] = north.im - south.im;
        }
        return block_len;
    }
    let mut output_lane = 0;
    for lane in 0..block_len {
        let north_ring = block_start + lane;
        let south_ring = latitude_count - 1 - north_ring;
        let north_active = m <= ring_order_limits[north_ring];
        let south_active = south_ring != north_ring && m <= ring_order_limits[south_ring];
        if compact && !north_active && !south_active {
            continue;
        }
        let north = if north_active {
            order_spectra.value(north_ring)
        } else {
            Complex::new(T::zero(), T::zero())
        };
        // The equatorial singleton has no reflected sample and contributes once to both parities.
        let south = if south_active {
            order_spectra.value(south_ring)
        } else {
            Complex::new(T::zero(), T::zero())
        };
        same_real[output_lane] = north.re + south.re;
        same_imag[output_lane] = north.im + south.im;
        opposite_real[output_lane] = north.re - south.re;
        opposite_imag[output_lane] = north.im - south.im;
        if compact {
            packed_north_ring[output_lane] = north_ring;
            compact_theta_sin[output_lane] = theta_sin[north_ring];
            compact_theta_cos[output_lane] = theta_cos[north_ring];
            compact_theta_cos_squared[output_lane] = theta_cos_squared[north_ring];
        }
        output_lane += 1;
    }

    if compact { output_lane } else { block_len }
}

#[inline]
fn restore_symmetric_analysis_pair_opposite<T: ShtScalar>(
    m: usize,
    latitude_count: usize,
    order_spectra: ScalarAnalysisOrderSpectra<'_, T>,
    ring_order_limits: &[usize],
    packed_north_ring: &[usize],
    opposite_real: &mut [T],
    opposite_imag: &mut [T],
) {
    debug_assert_eq!(ring_order_limits.len(), latitude_count);
    debug_assert_eq!(packed_north_ring.len(), opposite_real.len());
    debug_assert_eq!(opposite_real.len(), opposite_imag.len());
    for (lane, &north_ring) in packed_north_ring.iter().enumerate() {
        let south_ring = latitude_count - 1 - north_ring;
        let north_active = m <= ring_order_limits[north_ring];
        let south_active = south_ring != north_ring && m <= ring_order_limits[south_ring];
        let north = if north_active {
            order_spectra.value(north_ring)
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let south = if south_active {
            order_spectra.value(south_ring)
        } else {
            Complex::new(T::zero(), T::zero())
        };
        opposite_real[lane] = north.re - south.re;
        opposite_imag[lane] = north.im - south.im;
    }
}

#[inline]
fn stable_analysis_cosine_squared<T: ShtScalar>(sine: T, cosine: T) -> T {
    if cosine.abs() > T::from_f64(0.99).expect("0.99 is representable") {
        (T::one() - sine) * (T::one() + sine)
    } else {
        cosine * cosine
    }
}

fn first_active_symmetric_pair(
    m: usize,
    latitude_count: usize,
    ring_order_limits: &[usize],
) -> usize {
    // On supported symmetric grids, the scalar cutoff rises toward the equator;
    // the maximum limit of each reflected pair is therefore monotone as well.
    let pair_lane_count = latitude_count.div_ceil(2);
    let pair_limit =
        |pair: usize| ring_order_limits[pair].max(ring_order_limits[latitude_count - 1 - pair]);
    let mut lower = 0;
    let mut upper = pair_lane_count;
    while lower < upper {
        let midpoint = lower + (upper - lower) / 2;
        if pair_limit(midpoint) < m {
            lower = midpoint + 1;
        } else {
            upper = midpoint;
        }
    }
    lower
}

fn symmetric_monotone_analysis_pair_limits(
    latitude_count: usize,
    ring_order_limits: &[usize],
) -> bool {
    // With equal mirrored limits and a monotone pole-to-equator cutoff,
    // first_active_symmetric_pair starts at an all-active suffix.
    debug_assert_eq!(ring_order_limits.len(), latitude_count);
    let pair_lane_count = latitude_count.div_ceil(2);
    let mut previous_limit = 0;
    for north_ring in 0..pair_lane_count {
        let south_ring = latitude_count - 1 - north_ring;
        let north_limit = ring_order_limits[north_ring];
        if north_limit != ring_order_limits[south_ring] || north_limit < previous_limit {
            return false;
        }
        previous_limit = north_limit;
    }
    true
}

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
    pub(crate) fn check_analysis_sampling(&self) -> Result<(), ShtError> {
        let minimum_ntheta = match self.grid.kind() {
            GridKind::GaussLegendre => self.harmonics.lmax().checked_add(1),
            GridKind::Fejer1 => self.harmonics.lmax().checked_add(1),
            GridKind::ClenshawCurtis => self.harmonics.lmax().checked_add(2),
        }
        .ok_or(ShtError::IntegerOverflow {
            quantity: "exact-analysis latitude count",
        })?;
        if self.grid.ntheta() < minimum_ntheta {
            return Err(ShtError::AnalysisLatitudeUndersampled {
                lmax: self.harmonics.lmax(),
                ntheta: self.grid.ntheta(),
            });
        }
        self.check_analysis_longitude()
    }

    pub(crate) fn check_analysis_longitude(&self) -> Result<(), ShtError> {
        if self.harmonics.mmax() > (self.grid.nphi() - 1) / 2 {
            return Err(ShtError::AnalysisLongitudeUndersampled {
                mmax: self.harmonics.mmax(),
                nphi: self.grid.nphi(),
            });
        }
        Ok(())
    }

    pub(crate) fn apply_analysis_pixel_weights(&self, map: &mut [T]) {
        let nphi = self.grid.nphi();
        for (ring, &ring_weight) in self.grid.ring_weights().iter().enumerate() {
            let scale = T::from_f64(ring_weight / nphi as f64)
                .expect("analysis pixel weight is representable");
            let start = ring * nphi;
            for sample in &mut map[start..start + nphi] {
                *sample *= scale;
            }
        }
    }

    pub(crate) fn check_workspace(&self, workspace: &ShtWorkspace<T>) -> Result<(), ShtError> {
        let prepared_ntheta = self
            .cc_resampling
            .as_ref()
            .map_or(0, |resampling| resampling.prepared_ntheta());
        let latitude_count = self.grid.ntheta().max(prepared_ntheta);
        let recurrence_scratch_len = recurrence_scratch_len(latitude_count);
        if workspace.ring_group_size == 0 {
            return Err(ShtError::BufferLength {
                buffer: "workspace latitude group size",
                expected: 1,
                actual: 0,
            });
        }
        check_len(
            "workspace Fourier spectra",
            self.spectrum_count,
            workspace.spectra.len(),
        )?;
        check_len(
            "workspace spin Fourier spectra",
            self.spectrum_count,
            workspace.spin_spectra.len(),
        )?;
        let prepared_spectrum_count = prepared_ntheta.checked_mul(self.fft.spectrum_len()).ok_or(
            ShtError::IntegerOverflow {
                quantity: "prepared CC Fourier spectrum count",
            },
        )?;
        check_len(
            "workspace prepared CC Fourier spectra",
            prepared_spectrum_count,
            workspace.prepared_spectra.len(),
        )?;
        check_len(
            "workspace prepared spin CC Fourier spectra",
            prepared_spectrum_count,
            workspace.prepared_spin_spectra.len(),
        )?;
        check_len(
            "workspace CC meridional profile",
            self.cc_resampling
                .as_ref()
                .map_or(0, |resampling| resampling.workspace_profile_len()),
            workspace.cc_profile.len(),
        )?;
        check_len(
            "workspace CC meridional FFT scratch",
            self.cc_resampling
                .as_ref()
                .map_or(0, |resampling| resampling.workspace_fft_scratch_len()),
            workspace.cc_fft_scratch.len(),
        )?;
        check_len(
            "workspace longitude row",
            self.grid.nphi(),
            workspace.row.len(),
        )?;
        for (buffer, actual) in [
            ("workspace latitude sine values", workspace.theta_sin.len()),
            (
                "workspace latitude cosine values",
                workspace.theta_cos.len(),
            ),
            (
                "workspace latitude cosine-square values",
                workspace.theta_cos_squared.len(),
            ),
            (
                "workspace analysis ring scales",
                workspace.analysis_ring_scales.len(),
            ),
            (
                "workspace latitude order limits",
                workspace.ring_order_limits.len(),
            ),
        ] {
            check_len(buffer, latitude_count, actual)?;
        }
        let expected_worker_count = self
            .grid
            .ntheta()
            .div_ceil(workspace.ring_group_size.min(self.grid.ntheta()));
        if workspace.recurrence_workers.len() < expected_worker_count {
            return Err(ShtError::BufferLength {
                buffer: "workspace recurrence worker groups",
                expected: expected_worker_count,
                actual: workspace.recurrence_workers.len(),
            });
        }
        for recurrence in &workspace.recurrence_workers {
            for (buffer, expected, actual) in [
                (
                    "workspace scalar recurrence factors",
                    self.recurrence_factor_count,
                    recurrence.epsilon.len(),
                ),
                (
                    "workspace previous recurrence values",
                    recurrence_scratch_len,
                    recurrence.previous.len(),
                ),
                (
                    "workspace current recurrence values",
                    recurrence_scratch_len,
                    recurrence.current.len(),
                ),
                (
                    "workspace next recurrence values",
                    recurrence_scratch_len,
                    recurrence.next.len(),
                ),
                (
                    "workspace recurrence exponents",
                    recurrence_scratch_len,
                    recurrence.exponents.len(),
                ),
                (
                    "workspace scalar-analysis scale epochs",
                    recurrence_scratch_len,
                    recurrence.analysis_scale_epochs.len(),
                ),
                (
                    "workspace scalar-analysis scale corrections",
                    recurrence_scratch_len,
                    recurrence.analysis_scale_corrections.len(),
                ),
                (
                    "workspace compact latitude cosine-square values",
                    recurrence_scratch_len,
                    recurrence.compact_theta_cos_squared.len(),
                ),
                (
                    "workspace spin recurrence alpha factors",
                    self.spin_recurrence_factor_count,
                    recurrence.spin_alpha.len(),
                ),
                (
                    "workspace spin recurrence A factors",
                    self.spin_recurrence_factor_count,
                    recurrence.spin_coefficient_a.len(),
                ),
                (
                    "workspace spin recurrence B factors",
                    self.spin_recurrence_factor_count,
                    recurrence.spin_coefficient_b.len(),
                ),
                (
                    "workspace same-parity analysis real spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_same_real.len(),
                ),
                (
                    "workspace same-parity analysis imaginary spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_same_imag.len(),
                ),
                (
                    "workspace opposite-parity analysis real spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_opposite_real.len(),
                ),
                (
                    "workspace opposite-parity analysis imaginary spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_opposite_imag.len(),
                ),
                (
                    "workspace ordered scalar scale correction",
                    recurrence_scratch_len,
                    recurrence.scalar_order_correction.len(),
                ),
                (
                    "workspace spin-1 fast scalar first coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_scalar_first.len(),
                ),
                (
                    "workspace spin-1 fast scalar second coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_scalar_second.len(),
                ),
                (
                    "workspace spin-1 fast E coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_spin_first.len(),
                ),
                (
                    "workspace spin-1 fast B coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_spin_second.len(),
                ),
                (
                    "workspace positive-spin previous values",
                    recurrence_scratch_len,
                    recurrence.spin_previous_plus.len(),
                ),
                (
                    "workspace positive-spin current values",
                    recurrence_scratch_len,
                    recurrence.spin_current_plus.len(),
                ),
                (
                    "workspace positive-spin next values",
                    recurrence_scratch_len,
                    recurrence.spin_next_plus.len(),
                ),
                (
                    "workspace positive-spin exponents",
                    recurrence_scratch_len,
                    recurrence.spin_exponents_plus.len(),
                ),
                (
                    "workspace negative-spin previous values",
                    recurrence_scratch_len,
                    recurrence.spin_previous_minus.len(),
                ),
                (
                    "workspace negative-spin current values",
                    recurrence_scratch_len,
                    recurrence.spin_current_minus.len(),
                ),
                (
                    "workspace negative-spin next values",
                    recurrence_scratch_len,
                    recurrence.spin_next_minus.len(),
                ),
                (
                    "workspace negative-spin exponents",
                    recurrence_scratch_len,
                    recurrence.spin_exponents_minus.len(),
                ),
            ] {
                check_len(buffer, expected, actual)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
