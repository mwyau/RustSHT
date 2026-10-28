// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

mod numeric;
pub mod pointwise;
mod scalar;
pub(crate) mod scalar_ordered;
mod scalar_paired;
mod scalar_table;
mod spin;
mod spin_factors;
mod spin_fast;

pub(crate) use numeric::{
    LegendrePrecision, ScaledValue, define_f64_scaled_arithmetic, define_scale_binary,
    define_scale_complex_normal_binary, scalar_recurrence_step,
};

#[cfg(all(test, rustsht_profile_spin32_normalizer))]
pub(crate) use spin::{normalizer_profile, reset_normalizer_profile};

pub(crate) const SPIN1_FAST_MIN_LMAX: usize = 15;
pub(crate) const SCALAR_ORDER_BATCH_SIZE: usize = 256;

/// Production scalar-analysis tile: 256 active reflected-latitude pairs.
pub(crate) const SCALAR_ANALYSIS_PAIR_TILE_SIZE: usize = 256;

pub(crate) fn recurrence_scratch_len(latitude_count: usize) -> usize {
    latitude_count
        .min(scalar::LATITUDE_BLOCK_SIZE)
        .max(SCALAR_ORDER_BATCH_SIZE)
        .max(SCALAR_ANALYSIS_PAIR_TILE_SIZE)
}

#[cfg(feature = "parallel")]
pub(crate) use scalar_paired::HEMISPHERE_PAIR_DEGREE_TILE;
#[cfg(test)]
pub(crate) use scalar_table::benchmark_strategy_override;
#[cfg(all(test, feature = "parallel"))]
pub(crate) use scalar_table::set_benchmark_strategy_override;
pub(crate) use scalar_table::{
    LegendreOperation, LegendreStrategy, MAX_OVERSAMPLED_TABLE_ANALYSIS_LMAX, ScalarLegendreTable,
    measured_table_break_even_calls, select_scalar_legendre_strategy,
};
pub(crate) use spin_fast::{
    fill_spin2_factors, scalar_to_spin1, scalar_to_spin1_with_epsilon, scalar_to_spin2,
    spin1_to_scalar, spin1_to_scalar_with_visit, spin2_to_scalar,
};

pub(crate) use scalar_paired::prepare_synthesis_coefficient_pair_values;
#[cfg(all(test, feature = "parallel"))]
pub(crate) use scalar_paired::{
    ScalarAnalysisRouteCounts, begin_scalar_analysis_route_profile,
    finish_scalar_analysis_route_profile,
};

use crate::error::ShtError;
use num_complex::Complex;
use pulp::Arch;
use rustfft::FftNum;
use rustfft::num_traits::{Float, FromPrimitive};

struct F64LaneCount;

impl pulp::WithSimd for F64LaneCount {
    type Output = usize;

    #[inline(always)]
    fn with_simd<S: pulp::Simd>(self, _: S) -> Self::Output {
        S::F64_LANES
    }
}

pub(crate) fn longitude_phase<T: Float + FromPrimitive>(
    order: usize,
    phi0: f64,
    inverse: bool,
) -> Complex<T> {
    let angle = order as f64 * phi0;
    let phase = if angle.is_finite() {
        let angle = if inverse { -angle } else { angle };
        let (sin, cos) = angle.sin_cos();
        Complex::<f64>::new(cos, sin)
    } else {
        let (sin, cos) = phi0.sin_cos();
        let mut base = Complex::<f64>::new(cos, sin);
        let mut result = Complex::<f64>::new(1.0, 0.0);
        let mut exponent = order;
        while exponent > 0 {
            if exponent & 1 == 1 {
                result *= base;
            }
            exponent >>= 1;
            if exponent > 0 {
                base *= base;
            }
        }
        if inverse { result.conj() } else { result }
    };
    Complex::new(
        T::from_f64(phase.re).expect("longitude phase is representable"),
        T::from_f64(phase.im).expect("longitude phase is representable"),
    )
}

pub(crate) fn sin_cos_from_f64<T: Float + FromPrimitive>(angle: f64) -> (T, T) {
    let (sin, cos) = if angle == 0.0 {
        (0.0, 1.0)
    } else if angle == std::f64::consts::PI {
        (0.0, -1.0)
    } else {
        angle.sin_cos()
    };
    (
        T::from_f64(sin).expect("latitude sine is representable"),
        T::from_f64(cos).expect("latitude cosine is representable"),
    )
}

/// Numerical operations needed by the f32 and f64 transform implementations.
///
/// This remains crate-private so the public API exposes only the supported
/// scalar types through its generic plan and workspace types.
pub(crate) trait ShtScalar:
    FftNum
    + Float
    + FromPrimitive
    + std::ops::AddAssign
    + std::ops::DivAssign
    + std::ops::MulAssign
    + std::ops::RemAssign
    + std::ops::SubAssign
{
    const FUSED_SCALAR_ANALYSIS_MIN_LMAX: usize = 255;

    fn supports_scalar_ordered() -> bool {
        false
    }

    #[cfg(feature = "parallel")]
    fn validate_scalar_ordered_geometry_dispatched<'a>(
        _sin_theta: &'a [Self],
        _cos_theta: &'a [Self],
        _ring_order_limits: &'a [usize],
    ) -> Option<scalar_ordered::ValidatedReflectedGeometry<'a>> {
        None
    }

    fn supports_paired_scalar() -> bool {
        false
    }

    fn supports_paired_spin() -> bool {
        false
    }

    #[inline(always)]
    fn scalar_paired_analysis_lane_width(_arch: Arch) -> usize {
        1
    }

    fn supports_precomputed_scalar() -> bool {
        false
    }

    #[cfg(feature = "parallel")]
    fn prefers_fine_spin1_synthesis_groups(_group_count: usize, _execution_threads: usize) -> bool {
        false
    }

    #[cfg(feature = "parallel")]
    fn prefers_fine_scalar_synthesis_groups(
        _group_count: usize,
        _execution_threads: usize,
    ) -> bool {
        false
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The table accumulator receives packed spectra, basis storage and reusable order scratch."
    )]
    fn analyze_scalar_table_group(
        order_spectra: &[Complex<Self>],
        table: &ScalarLegendreTable<Self>,
        ntheta: usize,
        lmax: usize,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<Self>],
        _scratch_even: &mut [Complex<Self>],
        _scratch_odd: &mut [Complex<Self>],
    ) {
        scalar_table::analyze_group_scalar(order_spectra, table, ntheta, lmax, m_start, m_end, alm);
    }

    fn supports_spin_analysis_simd() -> bool {
        false
    }

    fn spin_analysis_inputs_vectorizable(
        _m: usize,
        _q_real: &[Self],
        _q_imag: &[Self],
        _u_real: &[Self],
        _u_imag: &[Self],
        _ring_order_limits: &[usize],
    ) -> bool {
        false
    }

    type SpinInitial;

    fn sectoral_normalizations(mmax: usize) -> Result<Vec<Self>, ShtError>;
    fn polar_order_limit(lmax: usize, spin: usize, sin_theta: Self, cos_theta: Self) -> usize;
    fn fill_epsilon(m: usize, lmax: usize, output: &mut [Self]);
    #[expect(
        clippy::too_many_arguments,
        reason = "The recurrence receives grid lanes and caller-owned recurrence scratch explicitly."
    )]
    fn for_each_y_lm_dispatched<F>(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        epsilon: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64]) -> Result<(), ShtError>;
    fn fill_paired_scalar_factors(
        _m: usize,
        _lmax: usize,
        _epsilon: &[Self],
        _alpha: &mut [Self],
        _coefficient_a: &mut [Self],
        _coefficient_b: &mut [Self],
    ) -> bool {
        false
    }
    fn prepare_paired_scalar_synthesis_coefficients(
        _m: usize,
        _lmax: usize,
        _epsilon: &[Self],
        _alpha: &[Self],
        _coefficients: &[Complex<Self>],
        _coefficient_even: &mut [Complex<Self>],
        _coefficient_odd: &mut [Complex<Self>],
    ) -> bool {
        false
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Spin-1 synthesis prepares both converted scalar channels together."
    )]
    fn prepare_paired_scalar_synthesis_coefficient_pair(
        m: usize,
        lmax: usize,
        epsilon: &[Self],
        alpha: &[Self],
        first_coefficients: &[Complex<Self>],
        second_coefficients: &[Complex<Self>],
        first_even: &mut [Complex<Self>],
        first_odd: &mut [Complex<Self>],
        second_even: &mut [Complex<Self>],
        second_odd: &mut [Complex<Self>],
    ) -> bool {
        Self::prepare_paired_scalar_synthesis_coefficients(
            m,
            lmax,
            epsilon,
            alpha,
            first_coefficients,
            first_even,
            first_odd,
        ) && Self::prepare_paired_scalar_synthesis_coefficients(
            m,
            lmax,
            epsilon,
            alpha,
            second_coefficients,
            second_even,
            second_odd,
        )
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The fused scalar block receives operator data and worker scratch explicitly."
    )]
    #[cfg(test)]
    fn synthesize_paired_scalar_block_dispatched(
        _arch: Arch,
        _m: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _epsilon: &[Self],
        _alpha: &[Self],
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _coefficients: &[Complex<Self>],
        _ring_order_limits: &[usize],
        _spectrum_len: usize,
        _spectra: &mut [Complex<Self>],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The prepared scalar block receives operator data and worker scratch explicitly."
    )]
    fn synthesize_paired_scalar_block_prepared_dispatched(
        _arch: Arch,
        _m: usize,
        _output_order: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _coefficient_even: &[Complex<Self>],
        _coefficient_odd: &[Complex<Self>],
        _ring_order_limits: &[usize],
        _spectrum_len: usize,
        _spectra: &mut [Complex<Self>],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The scalar lane split uses existing recurrence and compaction scratch."
    )]
    fn synthesize_paired_scalar_block_partitioned_dispatched(
        _arch: Arch,
        _m: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _coefficient_even: &[Complex<Self>],
        _coefficient_odd: &[Complex<Self>],
        _ring_order_limits: &[usize],
        _spectrum_len: usize,
        _spectra: &mut [Complex<Self>],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _exponents: &mut [i64],
        _compact_theta_sin: &mut [Self],
        _compact_theta_cos: &mut [Self],
        _compact_ring_indices: &mut [usize],
    ) -> Result<bool, ShtError> {
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The private ordered scalar route receives one complete scalar GL synthesis and reusable workspace scratch."
    )]
    fn synthesize_scalar_ordered_dispatched(
        _arch: Arch,
        _alm: &[Complex<Self>],
        _lmax: usize,
        _mmax: usize,
        _nphi: usize,
        _spectrum_len: usize,
        _phi0: f64,
        _sectoral_norms: &[Self],
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _ring_order_limits: &[usize],
        _spectra: &mut [Complex<Self>],
        _scratch: &mut scalar_ordered::ScalarOrderedScratch<'_, Self>,
    ) -> bool {
        false
    }
    fn synthesize_prepared_scalar_component_batch_dispatched(
        _arch: Arch,
        _m: usize,
        _lmax: usize,
        _sectoral_norm: Self,
        _sin_theta: &[Self],
        _vector_lanes: usize,
        _scratch: &mut scalar_ordered::ScalarOrderedScratch<'_, Self>,
    ) -> bool {
        false
    }
    /// Writes one uniquely scheduled contiguous order range directly into a
    /// shared ring-major spectrum destination.
    ///
    /// # Safety
    ///
    /// The order range must be assigned to exactly one live parallel job, no
    /// other job may own an overlapping range, and that job must not write any
    /// ring/order cell more than once. All jobs must join before the destination
    /// is read.
    #[expect(
        clippy::too_many_arguments,
        reason = "The private scalar order route receives validated geometry and worker scratch."
    )]
    #[cfg(feature = "parallel")]
    unsafe fn synthesize_scalar_ordered_output_dispatched(
        _arch: Arch,
        _alm: &[Complex<Self>],
        _lmax: usize,
        _mmax: usize,
        _order_start: usize,
        _order_end: usize,
        _nphi: usize,
        _spectrum_len: usize,
        _phi0: f64,
        _sectoral_norms: &[Self],
        _geometry: &scalar_ordered::ValidatedReflectedGeometry<'_>,
        _output: &scalar_ordered::DisjointSpectrumOutput<'_, Self>,
        _scratch: &mut scalar_ordered::ScalarOrderedScratch<'_, Self>,
    ) -> bool {
        false
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The dual-channel scalar block receives two spectra and worker scratch explicitly."
    )]
    fn synthesize_paired_scalar_pair_block_dispatched(
        _arch: Arch,
        _m: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _epsilon: &[Self],
        _alpha: &[Self],
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _first_coefficients: &[Complex<Self>],
        _second_coefficients: &[Complex<Self>],
        _ring_order_limits: &[usize],
        _spectrum_len: usize,
        _first_spectra: &mut [Complex<Self>],
        _second_spectra: &mut [Complex<Self>],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _exponents: &mut [i64],
        _first_sum_re: &mut [Self],
        _first_sum_im: &mut [Self],
        _second_sum_re: &mut [Self],
        _second_sum_im: &mut [Self],
    ) -> Result<bool, ShtError> {
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The prepared dual-channel block receives both spectra and worker scratch explicitly."
    )]
    fn synthesize_prepared_scalar_pair_block_dispatched(
        _arch: Arch,
        _m: usize,
        _output_order: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _first_even: &[Complex<Self>],
        _first_odd: &[Complex<Self>],
        _second_even: &[Complex<Self>],
        _second_odd: &[Complex<Self>],
        _ring_order_limits: &[usize],
        _spectrum_len: usize,
        _first_spectra: &mut [Complex<Self>],
        _second_spectra: &mut [Complex<Self>],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _exponents: &mut [i64],
        _first_sum_re: &mut [Self],
        _first_sum_im: &mut [Self],
        _second_sum_re: &mut [Self],
        _second_sum_im: &mut [Self],
    ) -> Result<bool, ShtError> {
        Ok(false)
    }
    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "North/south paired synthesis writes both hemispheres for two scalar channels."
    )]
    fn synthesize_hemisphere_pair_block_dispatched(
        _arch: Arch,
        _m: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _first_even: &[Complex<Self>],
        _first_odd: &[Complex<Self>],
        _second_even: &[Complex<Self>],
        _second_odd: &[Complex<Self>],
        _spectrum_len: usize,
        _north_first: &mut [Complex<Self>],
        _south_first: &mut [Complex<Self>],
        _north_second: &mut [Complex<Self>],
        _south_second: &mut [Complex<Self>],
        _current: &mut [Self],
        _basis_tile: &mut [Self],
        _sums: &mut [&mut [Self]; 8],
    ) -> Result<bool, ShtError> {
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The transition kernel receives paired coefficients and caller-owned recurrence and sum scratch."
    )]
    fn try_synthesize_paired_scalar_transition_dispatched(
        _arch: Arch,
        _m: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _epsilon: &[Self],
        _alpha: &[Self],
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _first_coefficients: &[Complex<Self>],
        _second_coefficients: &[Complex<Self>],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _exponents: &mut [i64],
        _first_sum_re: &mut [Self],
        _first_sum_im: &mut [Self],
        _second_sum_re: &mut [Self],
        _second_sum_im: &mut [Self],
    ) -> Result<bool, ShtError> {
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The prepared transition receives four checked coefficient-pair rows and lane scratch."
    )]
    fn try_synthesize_prepared_paired_scalar_transition_dispatched(
        _arch: Arch,
        _m: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _first_even: &[Complex<Self>],
        _first_odd: &[Complex<Self>],
        _second_even: &[Complex<Self>],
        _second_odd: &[Complex<Self>],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _exponents: &mut [i64],
        _first_sum_re: &mut [Self],
        _first_sum_im: &mut [Self],
        _second_sum_re: &mut [Self],
        _second_sum_im: &mut [Self],
    ) -> Result<bool, ShtError> {
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The paired recurrence receives order factors and caller-owned lane scratch explicitly."
    )]
    fn for_each_y_lm_paired_dispatched<F>(
        _arch: Arch,
        _m: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _exponents: &mut [i64],
        _visit: F,
    ) -> Result<bool, ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64]) -> Result<(), ShtError>,
    {
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Analysis needs the paired recurrence state and per-lane exponent scratch."
    )]
    fn for_each_y_lm_paired_status_dispatched<F>(
        _arch: Arch,
        _m: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _exponents: &mut [i64],
        _visit: F,
    ) -> Result<bool, ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64], bool) -> Result<(), ShtError>,
    {
        Ok(false)
    }
    fn multiply_complex_scaled(value: Complex<Self>, factor: Self, exponent: i64) -> Complex<Self>;
    fn scalar_analysis_products_safe(
        _same_real: &[Self],
        _same_imag: &[Self],
        _opposite_real: &[Self],
        _opposite_imag: &[Self],
        _cos_theta: &[Self],
    ) -> bool {
        false
    }
    fn accumulate_scalar_analysis_pair_normal(
        _arch: Arch,
        _same_real: &[Self],
        _same_imag: &[Self],
        _opposite_real: &[Self],
        _opposite_imag: &[Self],
        _basis: &[Self],
        _cos_theta: &[Self],
    ) -> Option<(Complex<Self>, Complex<Self>)> {
        None
    }
    fn prepare_scalar_analysis_pair_inputs(
        _same_real: &[Self],
        _same_imag: &[Self],
        _opposite_real: &mut [Self],
        _opposite_imag: &mut [Self],
        _cos_theta: &[Self],
    ) -> Option<(bool, bool)> {
        None
    }
    fn scalar_analysis_prefix_skip_allowed(_latitude_count: usize) -> bool {
        false
    }
    fn prepare_scalar_analysis_opposite(
        _opposite_real: &mut [Self],
        _opposite_imag: &mut [Self],
        _cos_theta: &[Self],
    ) -> Option<bool> {
        None
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Paired analysis combines two parity spectra with lane recurrence values."
    )]
    fn accumulate_scalar_analysis_pair(
        arch: Arch,
        products_safe: bool,
        normal_range: bool,
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
        basis: &[Self],
        exponents: &[i64],
        cos_theta: &[Self],
    ) -> (Complex<Self>, Complex<Self>) {
        debug_assert_eq!(same_real.len(), basis.len());
        debug_assert_eq!(same_imag.len(), basis.len());
        debug_assert_eq!(opposite_real.len(), basis.len());
        debug_assert_eq!(opposite_imag.len(), basis.len());
        debug_assert_eq!(exponents.len(), basis.len());
        debug_assert_eq!(cos_theta.len(), basis.len());
        if products_safe
            && normal_range
            && exponents.iter().all(|&exponent| exponent == 0)
            && let Some(sums) = Self::accumulate_scalar_analysis_pair_normal(
                arch,
                same_real,
                same_imag,
                opposite_real,
                opposite_imag,
                basis,
                cos_theta,
            )
        {
            return sums;
        }
        let mut same = Complex::new(Self::zero(), Self::zero());
        let mut opposite = Complex::new(Self::zero(), Self::zero());
        for lane in 0..basis.len() {
            same += Self::multiply_complex_scaled(
                Complex::new(same_real[lane], same_imag[lane]),
                basis[lane],
                exponents[lane],
            );
            opposite += Self::multiply_complex_scaled_product(
                Complex::new(opposite_real[lane], opposite_imag[lane]),
                basis[lane],
                cos_theta[lane],
                exponents[lane],
            );
        }
        (same, opposite)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The fused f64 scalar analysis block receives parity spectra and worker-owned recurrence scratch explicitly."
    )]
    fn fused_scalar_analysis_pair_block_preweighted(
        _arch: Arch,
        _m: usize,
        _lmax: usize,
        _allow_insignificant_prefix: bool,
        _range_check_is_proven_safe: bool,
        _range_check_was_exercised: &mut bool,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _cos_theta_squared: &[Self],
        _sectoral_norm: Self,
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _same_real: &[Self],
        _same_imag: &[Self],
        _opposite_real: &[Self],
        _opposite_imag: &[Self],
        _staged_coefficients: &mut [Complex<Self>],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _analysis_scale_epochs: &mut [Self],
        _analysis_scale_corrections: &mut [Self],
        _exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The fused spin-2 analysis block shares recurrence state across two scalar-conversion channels."
    )]
    fn fused_scalar_analysis_pair_channels_block(
        _arch: Arch,
        _sequential_channels: bool,
        _opposite_preweighted: bool,
        _shared_recurrence_sequential_accumulation: bool,
        _range_check_is_proven_safe: bool,
        _range_check_was_exercised: &mut bool,
        _m: usize,
        _lmax: usize,
        _sin_theta: &[Self],
        _cos_theta: &[Self],
        _sectoral_norm: Self,
        _coefficient_a: &[Self],
        _coefficient_b: &[Self],
        _same_real: [&[Self]; 2],
        _same_imag: [&[Self]; 2],
        _opposite_real: [&[Self]; 2],
        _opposite_imag: [&[Self]; 2],
        _staged_first: &mut [Complex<Self>],
        _staged_second: &mut [Complex<Self>],
        _previous: &mut [Self],
        _current: &mut [Self],
        _next: &mut [Self],
        _exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        Ok(false)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Spin-1 scalar conversion accumulates the Q and U parity channels over shared recurrence lanes."
    )]
    fn accumulate_scalar_analysis_pair_channels(
        arch: Arch,
        products_safe: [bool; 2],
        normal_range: bool,
        same_real: [&[Self]; 2],
        same_imag: [&[Self]; 2],
        opposite_real: [&[Self]; 2],
        opposite_imag: [&[Self]; 2],
        basis: &[Self],
        exponents: &[i64],
        cos_theta: &[Self],
    ) -> [(Complex<Self>, Complex<Self>); 2] {
        [
            Self::accumulate_scalar_analysis_pair(
                arch,
                products_safe[0],
                normal_range,
                same_real[0],
                same_imag[0],
                opposite_real[0],
                opposite_imag[0],
                basis,
                exponents,
                cos_theta,
            ),
            Self::accumulate_scalar_analysis_pair(
                arch,
                products_safe[1],
                normal_range,
                same_real[1],
                same_imag[1],
                opposite_real[1],
                opposite_imag[1],
                basis,
                exponents,
                cos_theta,
            ),
        ]
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Spin analysis accumulation receives lane spectra, two scaled bases and cutoff metadata."
    )]
    fn accumulate_spin_analysis_block(
        _arch: Arch,
        _vectorizable_inputs: bool,
        m: usize,
        q_real: &[Self],
        q_imag: &[Self],
        u_real: &[Self],
        u_imag: &[Self],
        plus: &[Self],
        plus_exponents: &[i64],
        minus: &[Self],
        minus_exponents: &[i64],
        alpha_norm: Self,
        ring_order_limits: &[usize],
        include_b: bool,
    ) -> (Complex<Self>, Complex<Self>) {
        accumulate_spin_analysis_scalar_fallback::<Self>(
            m,
            q_real,
            q_imag,
            u_real,
            u_imag,
            plus,
            plus_exponents,
            minus,
            minus_exponents,
            alpha_norm,
            ring_order_limits,
            include_b,
        )
    }
    #[inline(always)]
    fn accumulate_spin_analysis_lane(
        e_sum: &mut Complex<Self>,
        b_sum: &mut Complex<Self>,
        q: Complex<Self>,
        u: Complex<Self>,
        even_basis: Self,
        odd_basis: Self,
        include_b: bool,
    ) {
        *e_sum += q * even_basis + Complex::new(Self::zero(), -Self::one()) * u * odd_basis;
        if include_b {
            *b_sum += Complex::new(Self::zero(), Self::one()) * q * odd_basis + u * even_basis;
        }
    }
    fn multiply_complex_scaled_product(
        value: Complex<Self>,
        factor: Self,
        additional_factor: Self,
        exponent: i64,
    ) -> Complex<Self> {
        Self::multiply_complex_scaled(value * additional_factor, factor, exponent)
    }
    fn fill_spin_factors(
        spin: usize,
        m: usize,
        lmax: usize,
        alpha: &mut [Self],
        coefficient_a: &mut [Self],
        coefficient_b: &mut [Self],
    ) -> Result<Self::SpinInitial, ShtError>;
    #[expect(
        clippy::too_many_arguments,
        reason = "The spin recurrence receives plan factors and caller-owned lane scratch explicitly."
    )]
    fn for_each_spin_lm_dispatched<F>(
        arch: Arch,
        initial: &Self::SpinInitial,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous_plus: &mut [Self],
        current_plus: &mut [Self],
        next_plus: &mut [Self],
        exponent_plus: &mut [i64],
        previous_minus: &mut [Self],
        current_minus: &mut [Self],
        next_minus: &mut [Self],
        exponent_minus: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64], &[Self], &[i64]) -> Result<(), ShtError>;
    fn scaled_basis_value(value: Self, exponent: i64, alpha_norm: Self) -> Self;
    fn spin_normalization(spin: usize, degree: usize, derivative: bool) -> Self;
}

/// Runs the shared native-spin scalar fallback over scaled and inactive lanes.
/// SIMD accumulation and precision-specific lane arithmetic remain outside
/// this loop, so unusual inputs retain each scalar type's established path.
#[expect(
    clippy::too_many_arguments,
    reason = "The shared scalar fallback receives lane arrays and basis scale metadata."
)]
#[inline(always)]
pub(crate) fn accumulate_spin_analysis_scalar_fallback<T: ShtScalar>(
    m: usize,
    q_real: &[T],
    q_imag: &[T],
    u_real: &[T],
    u_imag: &[T],
    plus: &[T],
    plus_exponents: &[i64],
    minus: &[T],
    minus_exponents: &[i64],
    alpha_norm: T,
    ring_order_limits: &[usize],
    include_b: bool,
) -> (Complex<T>, Complex<T>) {
    let mut e_sum = Complex::new(T::zero(), T::zero());
    let mut b_sum = Complex::new(T::zero(), T::zero());
    for lane in 0..plus.len() {
        if m > ring_order_limits[lane] {
            continue;
        }
        let plus_value = T::scaled_basis_value(plus[lane], plus_exponents[lane], alpha_norm);
        let minus_value = T::scaled_basis_value(minus[lane], minus_exponents[lane], alpha_norm);
        let even_basis = plus_value + minus_value;
        let odd_basis = plus_value - minus_value;
        T::accumulate_spin_analysis_lane(
            &mut e_sum,
            &mut b_sum,
            Complex::new(q_real[lane], q_imag[lane]),
            Complex::new(u_real[lane], u_imag[lane]),
            even_basis,
            odd_basis,
            include_b,
        );
    }
    (e_sum, b_sum)
}

impl ShtScalar for f64 {
    const FUSED_SCALAR_ANALYSIS_MIN_LMAX: usize = 16;

    type SpinInitial = spin::SpinInitial;

    #[cfg(feature = "parallel")]
    fn prefers_fine_spin1_synthesis_groups(group_count: usize, execution_threads: usize) -> bool {
        group_count.div_ceil(2) < execution_threads
    }

    fn supports_scalar_ordered() -> bool {
        true
    }

    #[cfg(feature = "parallel")]
    fn validate_scalar_ordered_geometry_dispatched<'a>(
        sin_theta: &'a [Self],
        cos_theta: &'a [Self],
        ring_order_limits: &'a [usize],
    ) -> Option<scalar_ordered::ValidatedReflectedGeometry<'a>> {
        scalar_ordered::validate_reflected_geometry(sin_theta, cos_theta, ring_order_limits)
    }

    fn supports_paired_scalar() -> bool {
        true
    }

    fn supports_paired_spin() -> bool {
        true
    }

    #[inline(always)]
    fn scalar_paired_analysis_lane_width(arch: Arch) -> usize {
        arch.dispatch(F64LaneCount).max(1)
    }

    fn supports_precomputed_scalar() -> bool {
        true
    }

    fn analyze_scalar_table_group(
        order_spectra: &[Complex<Self>],
        table: &ScalarLegendreTable<Self>,
        ntheta: usize,
        lmax: usize,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<Self>],
        scratch_even: &mut [Complex<Self>],
        scratch_odd: &mut [Complex<Self>],
    ) {
        scalar_table::analyze_group_f64(
            order_spectra,
            table,
            ntheta,
            lmax,
            m_start,
            m_end,
            alm,
            scratch_even,
            scratch_odd,
        );
    }

    fn supports_spin_analysis_simd() -> bool {
        true
    }

    fn spin_analysis_inputs_vectorizable(
        m: usize,
        q_real: &[Self],
        q_imag: &[Self],
        u_real: &[Self],
        u_imag: &[Self],
        ring_order_limits: &[usize],
    ) -> bool {
        spin::analysis_inputs_vectorizable(m, q_real, q_imag, u_real, u_imag, ring_order_limits)
    }

    fn sectoral_normalizations(mmax: usize) -> Result<Vec<Self>, ShtError> {
        scalar::sectoral_normalizations(mmax)
    }

    fn polar_order_limit(lmax: usize, spin: usize, sin_theta: Self, cos_theta: Self) -> usize {
        crate::grid::polar_order_limit(lmax, spin, sin_theta, cos_theta)
    }

    fn fill_epsilon(m: usize, lmax: usize, output: &mut [Self]) {
        scalar::fill_epsilon(m, lmax, output);
    }

    fn for_each_y_lm_dispatched<F>(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        epsilon: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64]) -> Result<(), ShtError>,
    {
        scalar::for_each_y_lm_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            epsilon,
            previous,
            current,
            next,
            exponents,
            visit,
        )
    }

    fn fill_paired_scalar_factors(
        m: usize,
        lmax: usize,
        epsilon: &[Self],
        alpha: &mut [Self],
        coefficient_a: &mut [Self],
        coefficient_b: &mut [Self],
    ) -> bool {
        scalar_paired::fill_factors(m, lmax, epsilon, alpha, coefficient_a, coefficient_b)
    }

    fn prepare_paired_scalar_synthesis_coefficients(
        m: usize,
        lmax: usize,
        epsilon: &[Self],
        alpha: &[Self],
        coefficients: &[Complex<Self>],
        coefficient_even: &mut [Complex<Self>],
        coefficient_odd: &mut [Complex<Self>],
    ) -> bool {
        scalar_paired::prepare_synthesis_coefficients(
            m,
            lmax,
            epsilon,
            alpha,
            coefficients,
            coefficient_even,
            coefficient_odd,
        )
    }

    fn prepare_paired_scalar_synthesis_coefficient_pair(
        m: usize,
        lmax: usize,
        epsilon: &[Self],
        alpha: &[Self],
        first_coefficients: &[Complex<Self>],
        second_coefficients: &[Complex<Self>],
        first_even: &mut [Complex<Self>],
        first_odd: &mut [Complex<Self>],
        second_even: &mut [Complex<Self>],
        second_odd: &mut [Complex<Self>],
    ) -> bool {
        scalar_paired::prepare_synthesis_coefficient_pair(
            m,
            lmax,
            epsilon,
            alpha,
            first_coefficients,
            second_coefficients,
            first_even,
            first_odd,
            second_even,
            second_odd,
        )
    }

    fn synthesize_scalar_ordered_dispatched(
        arch: Arch,
        alm: &[Complex<Self>],
        lmax: usize,
        mmax: usize,
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        sectoral_norms: &[Self],
        sin_theta: &[Self],
        cos_theta: &[Self],
        ring_order_limits: &[usize],
        spectra: &mut [Complex<Self>],
        scratch: &mut scalar_ordered::ScalarOrderedScratch<'_, Self>,
    ) -> bool {
        scalar_ordered::synthesize_dispatched(
            arch,
            alm,
            lmax,
            mmax,
            nphi,
            spectrum_len,
            phi0,
            sectoral_norms,
            sin_theta,
            cos_theta,
            ring_order_limits,
            spectra,
            scratch,
        )
    }

    fn synthesize_prepared_scalar_component_batch_dispatched(
        arch: Arch,
        m: usize,
        lmax: usize,
        sectoral_norm: Self,
        sin_theta: &[Self],
        vector_lanes: usize,
        scratch: &mut scalar_ordered::ScalarOrderedScratch<'_, Self>,
    ) -> bool {
        scalar_ordered::synthesize_prepared_component_batch_dispatched(
            arch,
            m,
            lmax,
            sectoral_norm,
            sin_theta,
            vector_lanes,
            scratch,
        )
    }

    #[cfg(feature = "parallel")]
    unsafe fn synthesize_scalar_ordered_output_dispatched(
        arch: Arch,
        alm: &[Complex<Self>],
        lmax: usize,
        mmax: usize,
        order_start: usize,
        order_end: usize,
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        sectoral_norms: &[Self],
        geometry: &scalar_ordered::ValidatedReflectedGeometry<'_>,
        output: &scalar_ordered::DisjointSpectrumOutput<'_, Self>,
        scratch: &mut scalar_ordered::ScalarOrderedScratch<'_, Self>,
    ) -> bool {
        // SAFETY: the caller schedules each order range exactly once, and the
        // scalar kernel emits at most one value per physical ring and order.
        unsafe {
            scalar_ordered::synthesize_order_output_dispatched(
                arch,
                alm,
                lmax,
                mmax,
                order_start,
                order_end,
                nphi,
                spectrum_len,
                phi0,
                sectoral_norms,
                geometry,
                output,
                scratch,
            )
        }
    }

    #[cfg(feature = "parallel")]
    fn synthesize_hemisphere_pair_block_dispatched(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        first_even: &[Complex<Self>],
        first_odd: &[Complex<Self>],
        second_even: &[Complex<Self>],
        second_odd: &[Complex<Self>],
        spectrum_len: usize,
        north_first: &mut [Complex<Self>],
        south_first: &mut [Complex<Self>],
        north_second: &mut [Complex<Self>],
        south_second: &mut [Complex<Self>],
        current: &mut [Self],
        basis_tile: &mut [Self],
        sums: &mut [&mut [Self]; 8],
    ) -> Result<bool, ShtError> {
        scalar_paired::synthesize_hemisphere_pair_block_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            first_even,
            first_odd,
            second_even,
            second_odd,
            spectrum_len,
            north_first,
            south_first,
            north_second,
            south_second,
            current,
            basis_tile,
            sums,
        )
    }

    fn for_each_y_lm_paired_dispatched<F>(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        visit: F,
    ) -> Result<bool, ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64]) -> Result<(), ShtError>,
    {
        scalar_paired::for_each_y_lm_paired_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            previous,
            current,
            next,
            exponents,
            visit,
        )?;
        Ok(true)
    }

    fn for_each_y_lm_paired_status_dispatched<F>(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        visit: F,
    ) -> Result<bool, ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64], bool) -> Result<(), ShtError>,
    {
        scalar_paired::for_each_y_lm_paired_status_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            previous,
            current,
            next,
            exponents,
            visit,
        )?;
        Ok(true)
    }

    #[cfg(test)]
    fn synthesize_paired_scalar_block_dispatched(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        epsilon: &[Self],
        alpha: &[Self],
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        coefficients: &[Complex<Self>],
        ring_order_limits: &[usize],
        spectrum_len: usize,
        spectra: &mut [Complex<Self>],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        scalar_paired::synthesize_order_block_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            epsilon,
            alpha,
            coefficient_a,
            coefficient_b,
            coefficients,
            ring_order_limits,
            spectrum_len,
            spectra,
            previous,
            current,
            next,
            exponents,
        )
    }

    fn synthesize_paired_scalar_block_prepared_dispatched(
        arch: Arch,
        m: usize,
        output_order: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        coefficient_even: &[Complex<Self>],
        coefficient_odd: &[Complex<Self>],
        ring_order_limits: &[usize],
        spectrum_len: usize,
        spectra: &mut [Complex<Self>],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        scalar_paired::synthesize_order_block_precomputed_dispatched(
            arch,
            m,
            output_order,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
            ring_order_limits,
            spectrum_len,
            spectra,
            previous,
            current,
            next,
            exponents,
        )
    }

    fn synthesize_paired_scalar_block_partitioned_dispatched(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        coefficient_even: &[Complex<Self>],
        coefficient_odd: &[Complex<Self>],
        ring_order_limits: &[usize],
        spectrum_len: usize,
        spectra: &mut [Complex<Self>],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        compact_theta_sin: &mut [Self],
        compact_theta_cos: &mut [Self],
        compact_ring_indices: &mut [usize],
    ) -> Result<bool, ShtError> {
        scalar_paired::synthesize_order_block_partitioned_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
            ring_order_limits,
            spectrum_len,
            spectra,
            previous,
            current,
            next,
            exponents,
            compact_theta_sin,
            compact_theta_cos,
            compact_ring_indices,
        )
    }

    fn synthesize_paired_scalar_pair_block_dispatched(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        epsilon: &[Self],
        alpha: &[Self],
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        first_coefficients: &[Complex<Self>],
        second_coefficients: &[Complex<Self>],
        ring_order_limits: &[usize],
        spectrum_len: usize,
        first_spectra: &mut [Complex<Self>],
        second_spectra: &mut [Complex<Self>],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        first_sum_re: &mut [Self],
        first_sum_im: &mut [Self],
        second_sum_re: &mut [Self],
        second_sum_im: &mut [Self],
    ) -> Result<bool, ShtError> {
        scalar_paired::synthesize_order_pair_block_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            epsilon,
            alpha,
            coefficient_a,
            coefficient_b,
            first_coefficients,
            second_coefficients,
            ring_order_limits,
            spectrum_len,
            first_spectra,
            second_spectra,
            previous,
            current,
            next,
            exponents,
            first_sum_re,
            first_sum_im,
            second_sum_re,
            second_sum_im,
        )
    }

    fn synthesize_prepared_scalar_pair_block_dispatched(
        arch: Arch,
        m: usize,
        output_order: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        first_even: &[Complex<Self>],
        first_odd: &[Complex<Self>],
        second_even: &[Complex<Self>],
        second_odd: &[Complex<Self>],
        ring_order_limits: &[usize],
        spectrum_len: usize,
        first_spectra: &mut [Complex<Self>],
        second_spectra: &mut [Complex<Self>],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        first_sum_re: &mut [Self],
        first_sum_im: &mut [Self],
        second_sum_re: &mut [Self],
        second_sum_im: &mut [Self],
    ) -> Result<bool, ShtError> {
        scalar_paired::synthesize_prepared_order_pair_block_dispatched(
            arch,
            m,
            output_order,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            first_even,
            first_odd,
            second_even,
            second_odd,
            ring_order_limits,
            spectrum_len,
            first_spectra,
            second_spectra,
            previous,
            current,
            next,
            exponents,
            first_sum_re,
            first_sum_im,
            second_sum_re,
            second_sum_im,
        )
    }

    fn try_synthesize_paired_scalar_transition_dispatched(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        epsilon: &[Self],
        alpha: &[Self],
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        first_coefficients: &[Complex<Self>],
        second_coefficients: &[Complex<Self>],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        first_sum_re: &mut [Self],
        first_sum_im: &mut [Self],
        second_sum_re: &mut [Self],
        second_sum_im: &mut [Self],
    ) -> Result<bool, ShtError> {
        scalar_paired::try_synthesize_order_pair_transition_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            epsilon,
            alpha,
            coefficient_a,
            coefficient_b,
            first_coefficients,
            second_coefficients,
            previous,
            current,
            next,
            exponents,
            first_sum_re,
            first_sum_im,
            second_sum_re,
            second_sum_im,
        )
    }

    fn try_synthesize_prepared_paired_scalar_transition_dispatched(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        first_even: &[Complex<Self>],
        first_odd: &[Complex<Self>],
        second_even: &[Complex<Self>],
        second_odd: &[Complex<Self>],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        first_sum_re: &mut [Self],
        first_sum_im: &mut [Self],
        second_sum_re: &mut [Self],
        second_sum_im: &mut [Self],
    ) -> Result<bool, ShtError> {
        scalar_paired::try_synthesize_prepared_order_pair_transition_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            first_even,
            first_odd,
            second_even,
            second_odd,
            previous,
            current,
            next,
            exponents,
            first_sum_re,
            first_sum_im,
            second_sum_re,
            second_sum_im,
        )
    }

    fn multiply_complex_scaled(value: Complex<Self>, factor: Self, exponent: i64) -> Complex<Self> {
        scalar::multiply_complex_scaled(value, factor, exponent)
    }

    fn scalar_analysis_products_safe(
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
        cos_theta: &[Self],
    ) -> bool {
        scalar_paired::analysis_products_safe(
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            cos_theta,
        )
    }

    fn prepare_scalar_analysis_pair_inputs(
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &mut [Self],
        opposite_imag: &mut [Self],
        cos_theta: &[Self],
    ) -> Option<(bool, bool)> {
        Some(scalar_paired::prepare_analysis_products_and_opposite(
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            cos_theta,
        ))
    }

    fn scalar_analysis_prefix_skip_allowed(latitude_count: usize) -> bool {
        scalar_paired::insignificant_prefix_skip_allowed(latitude_count)
    }

    fn prepare_scalar_analysis_opposite(
        opposite_real: &mut [Self],
        opposite_imag: &mut [Self],
        cos_theta: &[Self],
    ) -> Option<bool> {
        Some(scalar_paired::prepare_analysis_opposite(
            opposite_real,
            opposite_imag,
            cos_theta,
        ))
    }

    fn accumulate_scalar_analysis_pair(
        arch: Arch,
        products_safe: bool,
        normal_range: bool,
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
        basis: &[Self],
        exponents: &[i64],
        cos_theta: &[Self],
    ) -> (Complex<Self>, Complex<Self>) {
        scalar_paired::accumulate_analysis_pair_dispatched(
            arch,
            products_safe,
            normal_range,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            exponents,
            cos_theta,
        )
    }

    fn fused_scalar_analysis_pair_block_preweighted(
        arch: Arch,
        m: usize,
        lmax: usize,
        allow_insignificant_prefix: bool,
        range_check_is_proven_safe: bool,
        range_check_was_exercised: &mut bool,
        sin_theta: &[Self],
        cos_theta: &[Self],
        cos_theta_squared: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
        staged_coefficients: &mut [Complex<Self>],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        analysis_scale_epochs: &mut [Self],
        analysis_scale_corrections: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        scalar_paired::fused_scalar_analysis_pair_block_preweighted_dispatched(
            arch,
            m,
            lmax,
            allow_insignificant_prefix,
            range_check_is_proven_safe,
            range_check_was_exercised,
            sin_theta,
            cos_theta,
            cos_theta_squared,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_coefficients,
            previous,
            current,
            next,
            analysis_scale_epochs,
            analysis_scale_corrections,
            exponents,
        )
    }

    fn fused_scalar_analysis_pair_channels_block(
        arch: Arch,
        sequential_channels: bool,
        opposite_preweighted: bool,
        shared_recurrence_sequential_accumulation: bool,
        range_check_is_proven_safe: bool,
        range_check_was_exercised: &mut bool,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        same_real: [&[Self]; 2],
        same_imag: [&[Self]; 2],
        opposite_real: [&[Self]; 2],
        opposite_imag: [&[Self]; 2],
        staged_first: &mut [Complex<Self>],
        staged_second: &mut [Complex<Self>],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        scalar_paired::fused_scalar_analysis_pair_channels_block_with_range_proof(
            arch,
            sequential_channels,
            opposite_preweighted,
            shared_recurrence_sequential_accumulation,
            range_check_is_proven_safe,
            range_check_was_exercised,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            staged_first,
            staged_second,
            previous,
            current,
            next,
            exponents,
        )
    }

    fn accumulate_scalar_analysis_pair_channels(
        arch: Arch,
        products_safe: [bool; 2],
        normal_range: bool,
        same_real: [&[Self]; 2],
        same_imag: [&[Self]; 2],
        opposite_real: [&[Self]; 2],
        opposite_imag: [&[Self]; 2],
        basis: &[Self],
        exponents: &[i64],
        cos_theta: &[Self],
    ) -> [(Complex<Self>, Complex<Self>); 2] {
        scalar_paired::accumulate_analysis_pair_channels_dispatched(
            arch,
            products_safe,
            normal_range,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            exponents,
            cos_theta,
        )
    }

    fn accumulate_spin_analysis_block(
        arch: Arch,
        vectorizable_inputs: bool,
        m: usize,
        q_real: &[Self],
        q_imag: &[Self],
        u_real: &[Self],
        u_imag: &[Self],
        plus: &[Self],
        plus_exponents: &[i64],
        minus: &[Self],
        minus_exponents: &[i64],
        alpha_norm: Self,
        ring_order_limits: &[usize],
        include_b: bool,
    ) -> (Complex<Self>, Complex<Self>) {
        spin::accumulate_analysis_dispatched(
            arch,
            vectorizable_inputs,
            m,
            q_real,
            q_imag,
            u_real,
            u_imag,
            plus,
            plus_exponents,
            minus,
            minus_exponents,
            alpha_norm,
            ring_order_limits,
            include_b,
        )
    }

    fn multiply_complex_scaled_product(
        value: Complex<Self>,
        factor: Self,
        additional_factor: Self,
        exponent: i64,
    ) -> Complex<Self> {
        scalar::multiply_complex_scaled_product(value, factor, additional_factor, exponent)
    }

    fn fill_spin_factors(
        spin: usize,
        m: usize,
        lmax: usize,
        alpha: &mut [Self],
        coefficient_a: &mut [Self],
        coefficient_b: &mut [Self],
    ) -> Result<Self::SpinInitial, ShtError> {
        spin_factors::fill_spin_factors::<Self>(spin, m, lmax, alpha, coefficient_a, coefficient_b)
    }

    fn for_each_spin_lm_dispatched<F>(
        arch: Arch,
        initial: &Self::SpinInitial,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous_plus: &mut [Self],
        current_plus: &mut [Self],
        next_plus: &mut [Self],
        exponent_plus: &mut [i64],
        previous_minus: &mut [Self],
        current_minus: &mut [Self],
        next_minus: &mut [Self],
        exponent_minus: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64], &[Self], &[i64]) -> Result<(), ShtError>,
    {
        spin::for_each_spin_lm_dispatched(
            arch,
            initial,
            lmax,
            sin_theta,
            cos_theta,
            coefficient_a,
            coefficient_b,
            previous_plus,
            current_plus,
            next_plus,
            exponent_plus,
            previous_minus,
            current_minus,
            next_minus,
            exponent_minus,
            visit,
        )
    }

    fn scaled_basis_value(value: Self, exponent: i64, alpha_norm: Self) -> Self {
        spin::scaled_basis_value(value, exponent, alpha_norm)
    }

    fn spin_normalization(spin: usize, degree: usize, derivative: bool) -> Self {
        spin_factors::spin_normalization::<Self>(spin, degree, derivative)
    }
}

impl ShtScalar for f32 {
    type SpinInitial = spin_factors::SpinInitial<f32>;

    fn supports_paired_scalar() -> bool {
        true
    }

    #[cfg(feature = "parallel")]
    fn prefers_fine_scalar_synthesis_groups(group_count: usize, execution_threads: usize) -> bool {
        group_count > execution_threads
    }

    fn supports_spin_analysis_simd() -> bool {
        true
    }

    #[cfg(feature = "parallel")]
    fn prefers_fine_spin1_synthesis_groups(_group_count: usize, _execution_threads: usize) -> bool {
        true
    }

    fn spin_analysis_inputs_vectorizable(
        m: usize,
        q_real: &[Self],
        q_imag: &[Self],
        u_real: &[Self],
        u_imag: &[Self],
        ring_order_limits: &[usize],
    ) -> bool {
        spin::analysis_inputs_vectorizable(m, q_real, q_imag, u_real, u_imag, ring_order_limits)
    }

    fn accumulate_spin_analysis_block(
        arch: Arch,
        vectorizable_inputs: bool,
        m: usize,
        q_real: &[Self],
        q_imag: &[Self],
        u_real: &[Self],
        u_imag: &[Self],
        plus: &[Self],
        plus_exponents: &[i64],
        minus: &[Self],
        minus_exponents: &[i64],
        alpha_norm: Self,
        ring_order_limits: &[usize],
        include_b: bool,
    ) -> (Complex<Self>, Complex<Self>) {
        spin::accumulate_analysis_dispatched(
            arch,
            vectorizable_inputs,
            m,
            q_real,
            q_imag,
            u_real,
            u_imag,
            plus,
            plus_exponents,
            minus,
            minus_exponents,
            alpha_norm,
            ring_order_limits,
            include_b,
        )
    }

    #[inline(always)]
    fn accumulate_spin_analysis_lane(
        e_sum: &mut Complex<Self>,
        b_sum: &mut Complex<Self>,
        q: Complex<Self>,
        u: Complex<Self>,
        even_basis: Self,
        odd_basis: Self,
        include_b: bool,
    ) {
        e_sum.re += q.re * even_basis + u.im * odd_basis;
        e_sum.im += q.im * even_basis - u.re * odd_basis;
        if include_b {
            b_sum.re += u.re * even_basis - q.im * odd_basis;
            b_sum.im += u.im * even_basis + q.re * odd_basis;
        }
    }

    fn sectoral_normalizations(mmax: usize) -> Result<Vec<Self>, ShtError> {
        numeric::sectoral_normalizations(mmax)
    }

    fn polar_order_limit(lmax: usize, spin: usize, sin_theta: Self, cos_theta: Self) -> usize {
        crate::grid::polar_order_limit(lmax, spin, sin_theta, cos_theta)
    }

    fn fill_epsilon(m: usize, lmax: usize, output: &mut [Self]) {
        numeric::fill_epsilon(m, lmax, output);
    }

    fn fill_paired_scalar_factors(
        m: usize,
        lmax: usize,
        epsilon: &[Self],
        alpha: &mut [Self],
        coefficient_a: &mut [Self],
        coefficient_b: &mut [Self],
    ) -> bool {
        scalar_paired::fill_factors_typed(
            m,
            lmax,
            epsilon,
            alpha,
            coefficient_a,
            coefficient_b,
            scalar_paired::PAIRED_FACTOR_LIMIT_F32,
        )
    }

    fn prepare_paired_scalar_synthesis_coefficients(
        m: usize,
        lmax: usize,
        epsilon: &[Self],
        alpha: &[Self],
        coefficients: &[Complex<Self>],
        coefficient_even: &mut [Complex<Self>],
        coefficient_odd: &mut [Complex<Self>],
    ) -> bool {
        scalar_paired::prepare_synthesis_coefficients(
            m,
            lmax,
            epsilon,
            alpha,
            coefficients,
            coefficient_even,
            coefficient_odd,
        )
    }

    fn synthesize_paired_scalar_block_partitioned_dispatched(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        coefficient_even: &[Complex<Self>],
        coefficient_odd: &[Complex<Self>],
        ring_order_limits: &[usize],
        spectrum_len: usize,
        spectra: &mut [Complex<Self>],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        compact_theta_sin: &mut [Self],
        compact_theta_cos: &mut [Self],
        compact_ring_indices: &mut [usize],
    ) -> Result<bool, ShtError> {
        scalar_paired::synthesize_paired_scalar_block_partitioned_f32(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            coefficient_even,
            coefficient_odd,
            ring_order_limits,
            spectrum_len,
            spectra,
            previous,
            current,
            next,
            exponents,
            compact_theta_sin,
            compact_theta_cos,
            compact_ring_indices,
        )
    }

    fn for_each_y_lm_dispatched<F>(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        epsilon: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64]) -> Result<(), ShtError>,
    {
        scalar::for_each_y_lm_dispatched(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            epsilon,
            previous,
            current,
            next,
            exponents,
            visit,
        )
    }

    fn for_each_y_lm_paired_dispatched<F>(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        mut visit: F,
    ) -> Result<bool, ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64]) -> Result<(), ShtError>,
    {
        scalar_paired::for_each_y_lm_paired_status_dispatched_typed::<f32, _>(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            previous,
            current,
            next,
            exponents,
            |degree: usize,
             ring_start: usize,
             basis: &[f32],
             lane_exponents: &[i64],
             _normal_range: bool| { visit(degree, ring_start, basis, lane_exponents) },
        )?;
        Ok(true)
    }

    fn for_each_y_lm_paired_status_dispatched<F>(
        arch: Arch,
        m: usize,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        sectoral_norm: Self,
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        next: &mut [Self],
        exponents: &mut [i64],
        visit: F,
    ) -> Result<bool, ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64], bool) -> Result<(), ShtError>,
    {
        scalar_paired::for_each_y_lm_paired_status_dispatched_typed::<f32, _>(
            arch,
            m,
            lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            previous,
            current,
            next,
            exponents,
            visit,
        )?;
        Ok(true)
    }

    fn multiply_complex_scaled(value: Complex<Self>, factor: Self, exponent: i64) -> Complex<Self> {
        scalar::multiply_complex_scaled_f32(value, factor, exponent)
    }

    fn scalar_analysis_products_safe(
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
        cos_theta: &[Self],
    ) -> bool {
        scalar::analysis_products_safe_f32(
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            cos_theta,
        )
    }

    fn accumulate_scalar_analysis_pair_normal(
        arch: Arch,
        same_real: &[Self],
        same_imag: &[Self],
        opposite_real: &[Self],
        opposite_imag: &[Self],
        basis: &[Self],
        cos_theta: &[Self],
    ) -> Option<(Complex<Self>, Complex<Self>)> {
        Some(scalar::accumulate_analysis_pair_normal_f32(
            arch,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            cos_theta,
        ))
    }

    fn fill_spin_factors(
        spin: usize,
        m: usize,
        lmax: usize,
        alpha: &mut [Self],
        coefficient_a: &mut [Self],
        coefficient_b: &mut [Self],
    ) -> Result<Self::SpinInitial, ShtError> {
        spin_factors::fill_spin_factors::<Self>(spin, m, lmax, alpha, coefficient_a, coefficient_b)
    }

    fn for_each_spin_lm_dispatched<F>(
        arch: Arch,
        initial: &Self::SpinInitial,
        lmax: usize,
        sin_theta: &[Self],
        cos_theta: &[Self],
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous_plus: &mut [Self],
        current_plus: &mut [Self],
        next_plus: &mut [Self],
        exponent_plus: &mut [i64],
        previous_minus: &mut [Self],
        current_minus: &mut [Self],
        next_minus: &mut [Self],
        exponent_minus: &mut [i64],
        visit: F,
    ) -> Result<(), ShtError>
    where
        F: FnMut(usize, usize, &[Self], &[i64], &[Self], &[i64]) -> Result<(), ShtError>,
    {
        spin::for_each_spin_lm_dispatched_typed::<f32, F>(
            arch,
            initial,
            lmax,
            sin_theta,
            cos_theta,
            coefficient_a,
            coefficient_b,
            previous_plus,
            current_plus,
            next_plus,
            exponent_plus,
            previous_minus,
            current_minus,
            next_minus,
            exponent_minus,
            visit,
        )
    }

    fn scaled_basis_value(value: Self, exponent: i64, alpha_norm: Self) -> Self {
        spin::scaled_basis_value_f32(value, exponent, alpha_norm)
    }

    fn spin_normalization(spin: usize, degree: usize, derivative: bool) -> Self {
        spin_factors::spin_normalization::<Self>(spin, degree, derivative)
    }
}

pub(crate) use scalar::{LATITUDE_BLOCK_SIZE, scale_binary};

#[cfg(test)]
mod tests;
