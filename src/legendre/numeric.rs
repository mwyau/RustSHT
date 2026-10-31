// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use pulp::Simd;
use rustfft::num_traits::{Float, FromPrimitive};

use crate::error::ShtError;

/// Precision-dependent limits shared by the scalar and spin recurrences.
///
/// Implementations stay concrete so these values are resolved at compile
/// time in the f32 and f64 kernels.
pub(crate) trait LegendrePrecision: Float + FromPrimitive {
    const HALF: Self;
    const SPIN_HALF_ANGLE_FLOOR: Self;
    const BIG: Self;
    const SMALL: Self;
    const SCALE_UP: Self;
    const SCALE_DOWN: Self;
    const SCALE_EXPONENT: i64;
}

impl LegendrePrecision for f32 {
    const HALF: Self = 0.5;
    const SPIN_HALF_ANGLE_FLOOR: Self = 1e-15;
    const BIG: Self = f32::from_bits(167_u32 << 23);
    const SMALL: Self = f32::from_bits(87_u32 << 23);
    const SCALE_UP: Self = f32::from_bits(207_u32 << 23);
    const SCALE_DOWN: Self = f32::from_bits(47_u32 << 23);
    const SCALE_EXPONENT: i64 = 80;
}

impl LegendrePrecision for f64 {
    const HALF: Self = 0.5;
    const SPIN_HALF_ANGLE_FLOOR: Self = 1e-15;
    const BIG: Self = f64::from_bits(1423_u64 << 52);
    const SMALL: Self = f64::from_bits(623_u64 << 52);
    const SCALE_UP: Self = f64::from_bits(1823_u64 << 52);
    const SCALE_DOWN: Self = f64::from_bits(223_u64 << 52);
    const SCALE_EXPONENT: i64 = 800;
}

/// Typed SIMD leaves for the shared order-major scalar synthesis kernel.
///
/// The recurrence, scaling state machine, and reflected-ring traversal are
/// generic over this trait. Its implementations only select native vector
/// width, lane operations, and precision-specific range constants.
pub(crate) trait OrderedSimdPrecision: LegendrePrecision {
    type Vector<S: Simd>: Copy;
    type Mask<S: Simd>: Copy;

    const PREPARE_ORDER_CONSTANTS_IN_F64: bool;
    const USE_SHARED_SCALED_BASIS_PREFIX: bool;
    const SHARED_PREFIX_BATCH: bool;
    const SKIP_SCALED_PREFIX: bool;
    const SCALE_FTOL: Self;
    const MAX_FINITE: Self;
    const MIN_POSITIVE: Self;
    /// Largest negative scale count that always restores a value bounded by
    /// `SCALE_FTOL` to signed zero in shared-basis accumulation.
    const SHARED_BASIS_ZERO_SCALE_LIMIT: i64;
    const REFLECTION_TOLERANCE: Self;
    const SYNTHESIS_FACTOR_LIMIT: Self;

    fn lane_count<S: Simd>() -> usize;
    fn as_simd<S: Simd>(values: &[Self]) -> (&[Self::Vector<S>], &[Self]);
    fn as_mut_simd<S: Simd>(values: &mut [Self]) -> (&mut [Self::Vector<S>], &mut [Self]);
    fn splat<S: Simd>(simd: S, value: Self) -> Self::Vector<S>;
    fn vmul<S: Simd>(simd: S, left: Self::Vector<S>, right: Self::Vector<S>) -> Self::Vector<S>;
    fn vmul_add<S: Simd>(
        simd: S,
        left: Self::Vector<S>,
        right: Self::Vector<S>,
        addend: Self::Vector<S>,
    ) -> Self::Vector<S>;
    fn vabs<S: Simd>(simd: S, value: Self::Vector<S>) -> Self::Vector<S>;
    fn less_equal<S: Simd>(simd: S, left: Self::Vector<S>, right: Self::Vector<S>)
    -> Self::Mask<S>;
    fn mask_not<S: Simd>(simd: S, value: Self::Mask<S>) -> Self::Mask<S>;
    fn first_true<S: Simd>(simd: S, mask: Self::Mask<S>) -> usize;
    fn scale_binary(value: Self, exponent: i64) -> Self;
    fn scale_binary_uniform<S: Simd>(
        simd: S,
        values: &[Self],
        output: &mut [Self],
        exponent: i64,
    ) -> bool;
    fn scale_binary_simd<S: Simd>(
        simd: S,
        values: &[Self],
        scales: &[i64],
        output: &mut [Self],
    ) -> bool;
    fn rescale_ordered_batch<
        S: Simd,
        const TRACK_ANY_RESCALED: bool,
        const TRACK_PREFIX_EPOCH: bool,
    >(
        simd: S,
        previous: &mut [Self],
        current: &mut [Self],
        scale: &mut [i64],
    ) -> Option<(bool, bool)>;
    fn rescale_scaled_prefix_initial<S: Simd>(
        simd: S,
        previous: &mut [Self],
        current: &mut [Self],
        scale: &mut [i64],
    ) -> Option<bool>;
    fn rescale_scaled_prefix_step<S: Simd>(
        simd: S,
        previous: &mut [Self],
        current: &mut [Self],
        scale: &mut [i64],
    ) -> Option<bool>;
    fn initialize_sectoral_batch<S: Simd>(
        simd: S,
        sine: &[Self],
        values: &mut [Self],
        scales: &mut [i64],
        exponent: usize,
        lower_limit: Self,
        factor_value: Self,
    ) -> bool;
    #[expect(
        clippy::too_many_arguments,
        reason = "The precision leaf receives the packed recurrence state without allocation."
    )]
    fn skip_scaled_prefix<S: Simd>(
        simd: S,
        m: usize,
        lmax: usize,
        pair_count: usize,
        csq: &[Self],
        coefficient_a: &[Self],
        coefficient_b: &[Self],
        previous: &mut [Self],
        current: &mut [Self],
        scale: &mut [i64],
    ) -> Option<(usize, usize, bool)>;

    #[inline(always)]
    fn mask_all<S: Simd>(simd: S, mask: Self::Mask<S>) -> bool {
        Self::first_true(simd, Self::mask_not(simd, mask)) == Self::lane_count::<S>()
    }
}

macro_rules! impl_ordered_simd_precision {
    (
        $scalar:ty, $vector:ident, $mask:ident, $lanes:ident,
        $as_simd:ident, $as_mut_simd:ident, $splat:ident,
        $mul:ident, $mul_add:ident, $abs:ident, $less_equal:ident,
        $mask_not:ident, $first_true:ident, $scale_binary:ident, $scale_binary_simd:ident,
        $rescale_batch:ident, $prefix_rescale_batch:ident, $initialize_batch:ident,
        $scale_binary_uniform:ident,
        $prepare_order_constants_in_f64:expr, $use_shared_scaled_basis_prefix:expr,
        $shared_prefix_batch:expr, $skip_scaled_prefix:expr,
        $skip_prefix:ident,
        $shared_basis_zero_scale_limit:expr,
        $ftol:expr, $max_finite:expr, $min_positive:expr,
        $reflection_tolerance:expr, $synthesis_factor_limit:expr $(,)?
    ) => {
        impl OrderedSimdPrecision for $scalar {
            type Vector<S: Simd> = S::$vector;
            type Mask<S: Simd> = S::$mask;

            const PREPARE_ORDER_CONSTANTS_IN_F64: bool = $prepare_order_constants_in_f64;
            const USE_SHARED_SCALED_BASIS_PREFIX: bool = $use_shared_scaled_basis_prefix;
            const SHARED_PREFIX_BATCH: bool = $shared_prefix_batch;
            const SKIP_SCALED_PREFIX: bool = $skip_scaled_prefix;
            const SCALE_FTOL: Self = $ftol;
            const MAX_FINITE: Self = $max_finite;
            const MIN_POSITIVE: Self = $min_positive;
            const SHARED_BASIS_ZERO_SCALE_LIMIT: i64 = $shared_basis_zero_scale_limit;
            const REFLECTION_TOLERANCE: Self = $reflection_tolerance;
            const SYNTHESIS_FACTOR_LIMIT: Self = $synthesis_factor_limit;

            #[inline(always)]
            fn lane_count<S: Simd>() -> usize {
                S::$lanes
            }
            #[inline(always)]
            fn as_simd<S: Simd>(values: &[Self]) -> (&[Self::Vector<S>], &[Self]) {
                S::$as_simd(values)
            }
            #[inline(always)]
            fn as_mut_simd<S: Simd>(values: &mut [Self]) -> (&mut [Self::Vector<S>], &mut [Self]) {
                S::$as_mut_simd(values)
            }
            #[inline(always)]
            fn splat<S: Simd>(simd: S, value: Self) -> Self::Vector<S> {
                simd.$splat(value)
            }
            #[inline(always)]
            fn vmul<S: Simd>(
                simd: S,
                left: Self::Vector<S>,
                right: Self::Vector<S>,
            ) -> Self::Vector<S> {
                simd.$mul(left, right)
            }
            #[inline(always)]
            fn vmul_add<S: Simd>(
                simd: S,
                left: Self::Vector<S>,
                right: Self::Vector<S>,
                addend: Self::Vector<S>,
            ) -> Self::Vector<S> {
                simd.$mul_add(left, right, addend)
            }
            #[inline(always)]
            fn vabs<S: Simd>(simd: S, value: Self::Vector<S>) -> Self::Vector<S> {
                simd.$abs(value)
            }
            #[inline(always)]
            fn less_equal<S: Simd>(
                simd: S,
                left: Self::Vector<S>,
                right: Self::Vector<S>,
            ) -> Self::Mask<S> {
                simd.$less_equal(left, right)
            }
            #[inline(always)]
            fn mask_not<S: Simd>(simd: S, value: Self::Mask<S>) -> Self::Mask<S> {
                simd.$mask_not(value)
            }
            #[inline(always)]
            fn first_true<S: Simd>(simd: S, mask: Self::Mask<S>) -> usize {
                simd.$first_true(mask)
            }
            #[inline(always)]
            fn scale_binary(value: Self, exponent: i64) -> Self {
                $scale_binary(value, exponent)
            }
            #[inline(always)]
            fn scale_binary_uniform<S: Simd>(
                simd: S,
                values: &[Self],
                output: &mut [Self],
                exponent: i64,
            ) -> bool {
                $scale_binary_uniform::<S>(simd, values, output, exponent)
            }
            #[inline(always)]
            fn scale_binary_simd<S: Simd>(
                simd: S,
                values: &[Self],
                scales: &[i64],
                output: &mut [Self],
            ) -> bool {
                $scale_binary_simd::<S>(simd, values, scales, output)
            }
            #[inline(always)]
            fn rescale_ordered_batch<
                S: Simd,
                const TRACK_ANY_RESCALED: bool,
                const TRACK_PREFIX_EPOCH: bool,
            >(
                simd: S,
                previous: &mut [Self],
                current: &mut [Self],
                scale: &mut [i64],
            ) -> Option<(bool, bool)> {
                super::scalar_ordered::$rescale_batch::<S, TRACK_ANY_RESCALED, TRACK_PREFIX_EPOCH>(
                    simd, previous, current, scale,
                )
            }
            #[inline(always)]
            fn rescale_scaled_prefix_initial<S: Simd>(
                simd: S,
                previous: &mut [Self],
                current: &mut [Self],
                scale: &mut [i64],
            ) -> Option<bool> {
                super::scalar_ordered::$prefix_rescale_batch::<S, false, true>(
                    simd, previous, current, scale,
                )
                .map(|status| status.1)
            }
            #[inline(always)]
            fn rescale_scaled_prefix_step<S: Simd>(
                simd: S,
                previous: &mut [Self],
                current: &mut [Self],
                scale: &mut [i64],
            ) -> Option<bool> {
                let (any_rescaled, all_below_one) =
                    super::scalar_ordered::$prefix_rescale_batch::<S, true, true>(
                        simd, previous, current, scale,
                    )?;
                Some(!any_rescaled || all_below_one)
            }
            #[inline(always)]
            fn initialize_sectoral_batch<S: Simd>(
                simd: S,
                sine: &[Self],
                values: &mut [Self],
                scales: &mut [i64],
                exponent: usize,
                lower_limit: Self,
                factor_value: Self,
            ) -> bool {
                super::scalar_ordered::$initialize_batch::<S>(
                    simd,
                    sine,
                    values,
                    scales,
                    exponent,
                    lower_limit,
                    factor_value,
                )
            }
            #[inline(always)]
            fn skip_scaled_prefix<S: Simd>(
                simd: S,
                m: usize,
                lmax: usize,
                pair_count: usize,
                csq: &[Self],
                coefficient_a: &[Self],
                coefficient_b: &[Self],
                previous: &mut [Self],
                current: &mut [Self],
                scale: &mut [i64],
            ) -> Option<(usize, usize, bool)> {
                super::scalar_ordered::$skip_prefix::<$scalar, S>(
                    simd,
                    m,
                    lmax,
                    pair_count,
                    csq,
                    coefficient_a,
                    coefficient_b,
                    previous,
                    current,
                    scale,
                )
            }
        }
    };
}

impl_ordered_simd_precision!(
    f32,
    f32s,
    m32s,
    F32_LANES,
    as_simd_f32s,
    as_mut_simd_f32s,
    splat_f32s,
    mul_f32s,
    mul_add_f32s,
    abs_f32s,
    less_than_or_equal_f32s,
    not_m32s,
    first_true_m32s,
    scale_binary_f32,
    scale_binary_f32_simd,
    rescale_ordered_batch_f32,
    rescale_ordered_batch_f32,
    initialize_sectoral_batch_f32,
    scale_binary_f32_uniform,
    true,
    true,
    false,
    true,
    skip_negligible_scaled_prefix,
    -2,
    f32::from_bits(67_u32 << 23),
    f32::MAX,
    f32::MIN_POSITIVE,
    2.0e-6_f32,
    f32::from_bits(143_u32 << 23),
);

impl_ordered_simd_precision!(
    f64,
    f64s,
    m64s,
    F64_LANES,
    as_simd_f64s,
    as_mut_simd_f64s,
    splat_f64s,
    mul_f64s,
    mul_add_f64s,
    abs_f64s,
    less_than_or_equal_f64s,
    not_m64s,
    first_true_m64s,
    scale_binary_f64,
    scale_binary_f64_simd,
    rescale_ordered_batch_f64,
    rescale_prefix_batch_f64,
    initialize_sectoral_batch_f64,
    scale_binary_f64_uniform,
    false,
    false,
    true,
    true,
    skip_negligible_scaled_prefix,
    -2,
    f64::from_bits(963_u64 << 52),
    f64::MAX,
    f64::MIN_POSITIVE,
    5.0e-15_f64,
    f64::from_bits(1151_u64 << 52),
);

/// A normalized recurrence value with its separately tracked binary scale.
#[derive(Clone, Copy)]
pub(crate) struct ScaledValue<T> {
    pub(crate) mantissa: T,
    pub(crate) exponent: i64,
}

pub(crate) fn sectoral_normalizations<T: LegendrePrecision>(
    mmax: usize,
) -> Result<Vec<T>, ShtError> {
    let count = mmax.checked_add(1).ok_or(ShtError::IntegerOverflow {
        quantity: "scalar sectoral normalization count",
    })?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(count)
        .map_err(|_| ShtError::AllocationFailed {
            array: "scalar sectoral normalization",
        })?;

    let one = T::one();
    let two = one + one;
    let four = two + two;
    let pi = T::from_f64(std::f64::consts::PI).expect("PI is representable in f32 and f64");
    result.push(one / (four * pi).sqrt());
    for m in 1..=mmax {
        let mf = T::from_usize(m).expect("usize is representable in f32 and f64");
        result.push(result[m - 1] * ((two * mf + one) / (two * mf)).sqrt());
    }
    Ok(result)
}

pub(crate) fn fill_epsilon<T: LegendrePrecision>(m: usize, lmax: usize, output: &mut [T]) {
    debug_assert!(m <= lmax);
    debug_assert!(output.len() > lmax);
    for (l, value) in output.iter_mut().enumerate().skip(m) {
        *value = epsilon(l, m);
    }
}

pub(crate) fn epsilon<T: LegendrePrecision>(l: usize, m: usize) -> T {
    if l == m {
        return T::zero();
    }
    let lf = T::from_usize(l).expect("usize is representable in f32 and f64");
    let mf = T::from_usize(m).expect("usize is representable in f32 and f64");
    let one = T::one();
    let two = one + one;
    (((lf - mf) * (lf + mf)) / ((two * lf + one) * (two * lf - one))).sqrt()
}

#[inline]
pub(crate) fn value_in_normal_range<T: LegendrePrecision>(value: T) -> bool {
    let magnitude = value.abs();
    value == T::zero() || (value.is_finite() && magnitude <= T::BIG && magnitude >= T::SMALL)
}

/// Expands one normalized scalar-recurrence degree step with concrete Pulp
/// operations selected by the f32/f64 caller.
macro_rules! scalar_recurrence_step {
    (
        $simd:ident, $simd_type:ident,
        $epsilon:ident, $degree:ident,
        $current:ident, $previous:ident, $cosine:ident, $next:ident,
        $splat:ident, $as_simd:ident, $as_mut_simd:ident,
        $multiply:ident, $subtract:ident, $divide:ident
    ) => {{
        let epsilon_l = $simd.$splat($epsilon[$degree]);
        let epsilon_next = $simd.$splat($epsilon[$degree + 1]);
        let (current_vectors, current_tail) = $simd_type::$as_simd($current);
        let (previous_vectors, previous_tail) = $simd_type::$as_simd($previous);
        let (cos_vectors, cos_tail) = $simd_type::$as_simd($cosine);
        let (next_vectors, next_tail) = $simd_type::$as_mut_simd($next);
        debug_assert_eq!(current_vectors.len(), previous_vectors.len());
        debug_assert_eq!(current_vectors.len(), cos_vectors.len());
        debug_assert_eq!(current_vectors.len(), next_vectors.len());
        for block in 0..current_vectors.len() {
            let degree_term = $simd.$multiply(epsilon_l, previous_vectors[block]);
            let latitude_term = $simd.$multiply(cos_vectors[block], current_vectors[block]);
            next_vectors[block] =
                $simd.$divide($simd.$subtract(latitude_term, degree_term), epsilon_next);
        }
        debug_assert_eq!(current_tail.len(), previous_tail.len());
        debug_assert_eq!(current_tail.len(), cos_tail.len());
        debug_assert_eq!(current_tail.len(), next_tail.len());
        for index in 0..current_tail.len() {
            next_tail[index] = (cos_tail[index] * current_tail[index]
                - $epsilon[$degree] * previous_tail[index])
                / $epsilon[$degree + 1];
        }
    }};
}

pub(crate) use scalar_recurrence_step;

/// Defines the shared typed binary-scale fallback used by the scalar kernels.
/// The normal-value paths remain local because their precision-specific return
/// contracts differ and are part of the measured hot leaves.
macro_rules! define_scale_binary {
    (
        $visibility:vis fn $name:ident {
            float: $float:ty,
            bits: $bits:ty,
            mantissa_bits: $mantissa_bits:expr,
            exponent_field_mask: $exponent_field_mask:expr,
            maximum_exponent: $maximum_exponent:expr,
            upper_chunk: $upper_chunk:expr,
            lower_chunk: $lower_chunk:expr $(,)?
        }
    ) => {
        $visibility fn $name(mut value: $float, mut exponent: i64) -> $float {
            if value == 0.0 || !value.is_finite() || exponent == 0 {
                return value;
            }
            if exponent > 4096 {
                return value.signum() * <$float>::INFINITY;
            }
            if exponent < -4096 {
                return (0.0 as $float).copysign(value);
            }

            let bits = value.to_bits();
            let raw_exponent =
                ((bits >> $mantissa_bits) & ($exponent_field_mask as $bits)) as i64;
            let scaled_exponent = raw_exponent + exponent;
            // Preserve exact normal scaling by changing only the exponent bits.
            if raw_exponent > 0
                && raw_exponent < ($maximum_exponent as i64)
                && scaled_exponent > 0
                && scaled_exponent < ($maximum_exponent as i64)
            {
                let exponent_mask = ($exponent_field_mask as $bits) << $mantissa_bits;
                return <$float>::from_bits(
                    (bits & !exponent_mask) | ((scaled_exponent as $bits) << $mantissa_bits),
                );
            }

            while exponent > $upper_chunk {
                value *= (2.0 as $float).powi($upper_chunk);
                exponent -= $upper_chunk;
                if !value.is_finite() {
                    return value;
                }
            }
            while exponent < $lower_chunk {
                value *= (2.0 as $float).powi($lower_chunk);
                exponent += -($lower_chunk);
                if value == 0.0 {
                    return value;
                }
            }
            value * (2.0 as $float).powi(exponent as i32)
        }
    };
}

pub(crate) use define_scale_binary;

/// Applies an exact binary exponent to one f32, including correctly rounded
/// normal-to-subnormal transitions, without calling `powi` in the scaled
/// recurrence prefix.
pub(crate) fn scale_binary_f32(value: f32, exponent: i64) -> f32 {
    if value == 0.0 || !value.is_finite() || exponent == 0 {
        return value;
    }

    let bits = value.to_bits();
    let sign = bits & 0x8000_0000;
    let exponent_field = ((bits >> 23) & 0xff) as i64;
    let fraction = bits & 0x007f_ffff;
    let (significand, unbiased_exponent) = if exponent_field == 0 {
        debug_assert_ne!(fraction, 0);
        let bit_len = 32 - fraction.leading_zeros();
        let shift = 24 - bit_len;
        (fraction << shift, -126 - i64::from(shift))
    } else {
        ((1 << 23) | fraction, exponent_field - 127)
    };

    let Some(target_exponent) = unbiased_exponent.checked_add(exponent) else {
        return if exponent.is_positive() {
            f32::INFINITY.copysign(value)
        } else {
            0.0_f32.copysign(value)
        };
    };
    if target_exponent > 127 {
        return f32::INFINITY.copysign(value);
    }
    if target_exponent >= -126 {
        let target_field = (target_exponent + 127) as u32;
        return f32::from_bits(sign | (target_field << 23) | (significand & 0x007f_ffff));
    }

    let shift = -126 - target_exponent;
    if shift > 24 {
        return f32::from_bits(sign);
    }
    let shift = shift as u32;
    let quotient = significand >> shift;
    let remainder_mask = (1_u32 << shift) - 1;
    let remainder = significand & remainder_mask;
    let halfway = 1_u32 << (shift - 1);
    let round_up = remainder > halfway || (remainder == halfway && quotient & 1 != 0);
    let rounded = quotient + u32::from(round_up);
    if rounded >= 1 << 23 {
        f32::from_bits(sign | (1 << 23))
    } else {
        f32::from_bits(sign | rounded)
    }
}

fn scale_binary_f32_simd<S: Simd>(
    simd: S,
    values: &[f32],
    scales: &[i64],
    output: &mut [f32],
) -> bool {
    if values.len() != scales.len() || values.len() != output.len() {
        return false;
    }
    let Some(&common_scale) = scales.first() else {
        return true;
    };
    let Some(common_exponent) =
        common_scale.checked_mul(<f32 as LegendrePrecision>::SCALE_EXPONENT)
    else {
        return false;
    };
    let common_scale = scales.iter().all(|&scale| scale == common_scale);

    let lane_count = S::F32_LANES;
    let common_exponent_i32 = i32::try_from(common_exponent);
    let vectorizable = common_scale
        && lane_count == S::I32_LANES
        && lane_count <= 16
        && common_exponent_i32.is_ok_and(|exponent| (-1_000_000..=1_000_000).contains(&exponent));
    if !vectorizable {
        for ((&value, &scale), out) in values.iter().zip(scales).zip(output) {
            let Some(exponent) = scale.checked_mul(<f32 as LegendrePrecision>::SCALE_EXPONENT)
            else {
                return false;
            };
            *out = scale_binary_f32(value, exponent);
        }
        return true;
    }

    let (value_vectors, value_tail) = S::as_simd_f32s(values);
    let (output_vectors, output_tail) = S::as_mut_simd_f32s(output);
    let vector_values = value_vectors.len();

    for vector_index in 0..vector_values {
        let offset = vector_index * lane_count;
        let value_bits = simd.transmute_u32s_f32s(value_vectors[vector_index]);
        let field_bits = simd.and_u32s(
            simd.wrapping_dyn_shr_u32s(value_bits, simd.splat_u32s(23)),
            simd.splat_u32s(0xff),
        );
        let field = simd.transmute_i32s_u32s(field_bits);
        let exponent_vector = simd.splat_i32s(common_exponent_i32.unwrap());
        let target_field = simd.add_i32s(field, exponent_vector);
        let normal_input = simd.and_m32s(
            simd.greater_than_i32s(field, simd.splat_i32s(0)),
            simd.less_than_i32s(field, simd.splat_i32s(255)),
        );
        let normal_output = simd.and_m32s(
            simd.greater_than_i32s(target_field, simd.splat_i32s(0)),
            simd.less_than_i32s(target_field, simd.splat_i32s(255)),
        );
        let normal = simd.and_m32s(normal_input, normal_output);
        let target_bits = simd.mul_u32s(
            simd.transmute_u32s_i32s(target_field),
            simd.splat_u32s(1 << 23),
        );
        let sign_and_fraction = simd.and_u32s(value_bits, simd.splat_u32s(0x807f_ffff));
        let adjusted_bits = simd.or_u32s(sign_and_fraction, target_bits);
        let selected_bits = simd.select_u32s(normal, adjusted_bits, value_bits);
        if simd.first_true_m32s(simd.not_m32s(normal)) == lane_count {
            output_vectors[vector_index] = simd.transmute_f32s_u32s(selected_bits);
        } else {
            let mut output_lanes = [0.0_f32; 16];
            simd.partial_store_f32s(
                &mut output_lanes[..lane_count],
                simd.transmute_f32s_u32s(selected_bits),
            );
            for lane in 0..lane_count {
                let input_field = (values[offset + lane].to_bits() >> 23) & 0xff;
                let target_field = input_field as i64 + common_exponent;
                if input_field == 0 || input_field == 0xff || !(1..=254).contains(&target_field) {
                    output_lanes[lane] = scale_binary_f32(values[offset + lane], common_exponent);
                }
            }
            output_vectors[vector_index] = simd.partial_load_f32s(&output_lanes[..lane_count]);
        }
    }

    let tail_offset = vector_values * lane_count;
    for ((&value, &scale), out) in value_tail
        .iter()
        .zip(&scales[tail_offset..])
        .zip(output_tail)
    {
        let Some(exponent) = scale.checked_mul(<f32 as LegendrePrecision>::SCALE_EXPONENT) else {
            return false;
        };
        *out = scale_binary_f32(value, exponent);
    }
    true
}

/// Applies the common f32 scale epoch -80 directly to vector exponent fields.
/// Subnormal outputs use integer round-to-nearest, ties-to-even; non-finite
/// lanes decline so the caller can use the checked scalar path.
#[inline(always)]
fn scale_binary_f32_vector_m80<S: Simd>(simd: S, values: S::f32s) -> Option<S::f32s> {
    let bits = simd.transmute_u32s_f32s(values);
    let exponent = simd.and_u32s(
        simd.wrapping_dyn_shr_u32s(bits, simd.splat_u32s(23)),
        simd.splat_u32s(0xff),
    );
    let non_finite = simd.equal_u32s(exponent, simd.splat_u32s(0xff));
    if simd.first_true_m32s(non_finite) != S::F32_LANES {
        return None;
    }

    let sign = simd.and_u32s(bits, simd.splat_u32s(0x8000_0000));
    let normal_output = simd.greater_than_u32s(exponent, simd.splat_u32s(80));
    let subnormal_input = simd.equal_u32s(exponent, simd.splat_u32s(0));
    let subnormal_field = simd.select_u32s(normal_output, simd.splat_u32s(80), exponent);
    let unbounded_shift = simd.sub_u32s(simd.splat_u32s(81), subnormal_field);
    let rounds_to_zero = simd.greater_than_u32s(unbounded_shift, simd.splat_u32s(24));
    let shift = simd.min_u32s(unbounded_shift, simd.splat_u32s(24));
    let significand = simd.or_u32s(
        simd.and_u32s(bits, simd.splat_u32s(0x007f_ffff)),
        simd.splat_u32s(0x0080_0000),
    );
    let quotient = simd.wrapping_dyn_shr_u32s(significand, shift);
    let divisor = simd.wrapping_dyn_shl_u32s(simd.splat_u32s(1), shift);
    let remainder = simd.and_u32s(significand, simd.sub_u32s(divisor, simd.splat_u32s(1)));
    let halfway = simd.wrapping_dyn_shr_u32s(divisor, simd.splat_u32s(1));
    let above_halfway = simd.greater_than_u32s(remainder, halfway);
    let tie_with_odd_quotient = simd.and_m32s(
        simd.equal_u32s(remainder, halfway),
        simd.not_m32s(simd.equal_u32s(
            simd.and_u32s(quotient, simd.splat_u32s(1)),
            simd.splat_u32s(0),
        )),
    );
    let round_up = simd.or_m32s(above_halfway, tie_with_odd_quotient);
    let rounded = simd.add_u32s(
        quotient,
        simd.select_u32s(round_up, simd.splat_u32s(1), simd.splat_u32s(0)),
    );
    let rounded = simd.select_u32s(rounds_to_zero, simd.splat_u32s(0), rounded);
    let subnormal_bits = simd.select_u32s(subnormal_input, sign, simd.or_u32s(sign, rounded));
    let normal_bits = simd.sub_u32s(bits, simd.splat_u32s(80 << 23));
    Some(simd.transmute_f32s_u32s(simd.select_u32s(normal_output, normal_bits, subnormal_bits)))
}

#[inline(always)]
fn scale_binary_f32_uniform<S: Simd>(
    simd: S,
    values: &[f32],
    output: &mut [f32],
    exponent: i64,
) -> bool {
    if values.len() != output.len() || exponent != -80 {
        return false;
    }
    let (value_vectors, value_tail) = S::as_simd_f32s(values);
    let (output_vectors, output_tail) = S::as_mut_simd_f32s(output);
    if value_vectors.len() != output_vectors.len() || value_tail.len() != output_tail.len() {
        return false;
    }
    for (source, destination) in value_vectors.iter().zip(output_vectors.iter_mut()) {
        let Some(scaled) = scale_binary_f32_vector_m80(simd, *source) else {
            return false;
        };
        *destination = scaled;
    }
    for (&source, destination) in value_tail.iter().zip(output_tail.iter_mut()) {
        *destination = scale_binary_f32(source, exponent);
    }
    true
}

#[inline(always)]
fn scale_binary_f64_uniform<S: Simd>(
    _simd: S,
    _values: &[f64],
    _output: &mut [f64],
    _exponent: i64,
) -> bool {
    false
}

fn scale_binary_f64_simd<S: Simd>(
    _simd: S,
    values: &[f64],
    scales: &[i64],
    output: &mut [f64],
) -> bool {
    if values.len() != scales.len() || values.len() != output.len() {
        return false;
    }
    for ((&value, &scale), out) in values.iter().zip(scales).zip(output) {
        let Some(exponent) = scale.checked_mul(<f64 as LegendrePrecision>::SCALE_EXPONENT) else {
            return false;
        };
        *out = scale_binary_f64(value, exponent);
    }
    true
}

define_scale_binary! {
    pub(crate) fn scale_binary_f64 {
        float: f64,
        bits: u64,
        mantissa_bits: 52,
        exponent_field_mask: 0x7ff,
        maximum_exponent: 0x7ff,
        upper_chunk: 1023,
        lower_chunk: -1022,
    }
}

/// Generates exponent-bit restoration for each native complex precision,
/// keeping the mantissa width and finite exponent range typed.
macro_rules! define_scale_complex_normal_binary {
    (
        $name:ident,
        $complex:ty,
        $float:ty,
        $bits:ty,
        $mantissa_bits:expr,
        $exponent_mask:expr,
        $exponent_limit:literal $(,)?
    ) => {
        fn $name(value: $complex, exponent: i64) -> Option<$complex> {
            if exponent == 0 {
                return Some(value);
            }
            if !(-$exponent_limit..=$exponent_limit).contains(&exponent) {
                return None;
            }

            let real_bits = value.re.to_bits();
            let imag_bits = value.im.to_bits();
            let real_exponent =
                ((real_bits >> $mantissa_bits) & ($exponent_mask as $bits)) as i64 + exponent;
            let imag_exponent =
                ((imag_bits >> $mantissa_bits) & ($exponent_mask as $bits)) as i64 + exponent;
            if !(1..($exponent_mask as i64)).contains(&real_exponent)
                || !(1..($exponent_mask as i64)).contains(&imag_exponent)
            {
                return None;
            }

            let exponent_mask = ($exponent_mask as $bits) << $mantissa_bits;
            Some(<$complex>::new(
                <$float>::from_bits(
                    (real_bits & !exponent_mask) | ((real_exponent as $bits) << $mantissa_bits),
                ),
                <$float>::from_bits(
                    (imag_bits & !exponent_mask) | ((imag_exponent as $bits) << $mantissa_bits),
                ),
            ))
        }
    };
}

pub(crate) use define_scale_complex_normal_binary;

/// Generates the same concrete f64 scaled arithmetic in scalar and native-spin
/// owners while keeping their local function names and overflow descriptions.
macro_rules! define_f64_scaled_arithmetic {
    ($power:ident, $multiply:ident, $normalize:ident, $overflow_quantity:literal $(,)?) => {
        fn $power(
            base: f64,
            mut exponent: usize,
        ) -> Result<$crate::legendre::ScaledValue<f64>, $crate::error::ShtError> {
            let mut result = $crate::legendre::ScaledValue {
                mantissa: 1.0,
                exponent: 0,
            };
            let mut factor = $normalize(base, 0)?;
            while exponent != 0 {
                if !exponent.is_multiple_of(2) {
                    result = $multiply(result, factor)?;
                }
                exponent /= 2;
                if exponent != 0 {
                    factor = $multiply(factor, factor)?;
                }
            }
            Ok(result)
        }

        fn $multiply(
            left: $crate::legendre::ScaledValue<f64>,
            right: $crate::legendre::ScaledValue<f64>,
        ) -> Result<$crate::legendre::ScaledValue<f64>, $crate::error::ShtError> {
            let exponent = left.exponent.checked_add(right.exponent).ok_or(
                $crate::error::ShtError::IntegerOverflow {
                    quantity: $overflow_quantity,
                },
            )?;
            $normalize(left.mantissa * right.mantissa, exponent)
        }

        fn $normalize(
            mut value: f64,
            mut exponent: i64,
        ) -> Result<$crate::legendre::ScaledValue<f64>, $crate::error::ShtError> {
            while value.abs() > <f64 as $crate::legendre::LegendrePrecision>::BIG {
                value *= <f64 as $crate::legendre::LegendrePrecision>::SCALE_DOWN;
                exponent = exponent
                    .checked_add(<f64 as $crate::legendre::LegendrePrecision>::SCALE_EXPONENT)
                    .ok_or($crate::error::ShtError::IntegerOverflow {
                        quantity: $overflow_quantity,
                    })?;
            }
            while value != 0.0 && value.abs() < <f64 as $crate::legendre::LegendrePrecision>::SMALL
            {
                value *= <f64 as $crate::legendre::LegendrePrecision>::SCALE_UP;
                exponent = exponent
                    .checked_sub(<f64 as $crate::legendre::LegendrePrecision>::SCALE_EXPONENT)
                    .ok_or($crate::error::ShtError::IntegerOverflow {
                        quantity: $overflow_quantity,
                    })?;
            }
            Ok($crate::legendre::ScaledValue {
                mantissa: value,
                exponent,
            })
        }
    };
}

pub(crate) use define_f64_scaled_arithmetic;

/// Restores one recurrence value to the precision's safe exponent interval.
/// The shared power/product helpers below serve native-f32 scalar and spin;
/// f64 transform kernels keep their measured local leaves.
#[inline(always)]
pub(crate) fn normalize_scaled_value<T: LegendrePrecision>(
    mut value: T,
    mut exponent: i64,
    overflow_quantity: &'static str,
) -> Result<ScaledValue<T>, ShtError> {
    while value.abs() > T::BIG {
        value = value * T::SCALE_DOWN;
        exponent = exponent
            .checked_add(T::SCALE_EXPONENT)
            .ok_or(ShtError::IntegerOverflow {
                quantity: overflow_quantity,
            })?;
    }
    while value != T::zero() && value.abs() < T::SMALL {
        value = value * T::SCALE_UP;
        exponent = exponent
            .checked_sub(T::SCALE_EXPONENT)
            .ok_or(ShtError::IntegerOverflow {
                quantity: overflow_quantity,
            })?;
    }
    Ok(ScaledValue {
        mantissa: value,
        exponent,
    })
}

/// Normalizes one native-f32 recurrence pair shared by scalar and spin paths.
/// Returns the final absolute-value bits and number of scaling steps so callers
/// can preserve their fast-range and test-profile bookkeeping.
#[inline(always)]
pub(crate) fn normalize_pair_lane_f32(
    previous: &mut f32,
    current: &mut f32,
    exponent: &mut i64,
    overflow_quantity: &'static str,
) -> Result<(u32, usize), ShtError> {
    let previous_bits = previous.to_bits() & 0x7fff_ffff;
    let current_bits = current.to_bits() & 0x7fff_ffff;
    debug_assert!(previous_bits < 0x7f80_0000 && current_bits < 0x7f80_0000);
    let mut maximum_bits = previous_bits.max(current_bits);
    let mut maximum = f32::from_bits(maximum_bits);
    let mut scale_steps = 0;

    while maximum_bits > <f32 as LegendrePrecision>::BIG.to_bits() {
        *previous *= <f32 as LegendrePrecision>::SCALE_DOWN;
        *current *= <f32 as LegendrePrecision>::SCALE_DOWN;
        *exponent = exponent
            .checked_add(<f32 as LegendrePrecision>::SCALE_EXPONENT)
            .ok_or(ShtError::IntegerOverflow {
                quantity: overflow_quantity,
            })?;
        maximum *= <f32 as LegendrePrecision>::SCALE_DOWN;
        maximum_bits = maximum.to_bits() & 0x7fff_ffff;
        scale_steps += 1;
    }
    while maximum_bits != 0 && maximum_bits < <f32 as LegendrePrecision>::SMALL.to_bits() {
        *previous *= <f32 as LegendrePrecision>::SCALE_UP;
        *current *= <f32 as LegendrePrecision>::SCALE_UP;
        *exponent = exponent
            .checked_sub(<f32 as LegendrePrecision>::SCALE_EXPONENT)
            .ok_or(ShtError::IntegerOverflow {
                quantity: overflow_quantity,
            })?;
        maximum *= <f32 as LegendrePrecision>::SCALE_UP;
        maximum_bits = maximum.to_bits() & 0x7fff_ffff;
        scale_steps += 1;
    }

    Ok((maximum_bits, scale_steps))
}

/// Normalizes every lane in a native-f32 recurrence pair. The optional
/// counters support the spin normalizer profile without duplicating the lane
/// traversal in the scalar and spin owners.
#[inline(always)]
pub(crate) fn normalize_pair_lanes_f32<const TRACK_SCALE_COUNTS: bool>(
    previous: &mut [f32],
    current: &mut [f32],
    exponents: &mut [i64],
    overflow_quantity: &'static str,
) -> Result<(usize, usize), ShtError> {
    let mut scaled_lanes = 0;
    let mut scale_steps = 0;
    for lane in 0..previous.len() {
        let (_, lane_scale_steps) = normalize_pair_lane_f32(
            &mut previous[lane],
            &mut current[lane],
            &mut exponents[lane],
            overflow_quantity,
        )?;
        if TRACK_SCALE_COUNTS && lane_scale_steps != 0 {
            scaled_lanes += 1;
            scale_steps += lane_scale_steps;
        }
    }
    Ok((scaled_lanes, scale_steps))
}

/// Reports whether any native-f32 recurrence pair is outside its ordinary
/// range. Both paths share the float range scan; the const policy preserves
/// scalar's existing single-NaN behavior while spin treats any NaN as unsafe.
#[inline(always)]
pub(crate) fn f32_pair_lanes_need_scaling<S: Simd, const IGNORE_SINGLE_NAN: bool>(
    simd: S,
    previous: &[f32],
    current: &[f32],
) -> bool {
    debug_assert_eq!(previous.len(), current.len());
    let (previous_vectors, previous_tail) = S::as_simd_f32s(previous);
    let (current_vectors, current_tail) = S::as_simd_f32s(current);
    debug_assert_eq!(previous_vectors.len(), current_vectors.len());
    debug_assert_eq!(previous_tail.len(), current_tail.len());

    let big = simd.splat_f32s(<f32 as LegendrePrecision>::BIG);
    let zeros = simd.splat_f32s(0.0);
    for index in 0..previous_vectors.len() {
        let previous = simd.abs_f32s(previous_vectors[index]);
        let current = simd.abs_f32s(current_vectors[index]);
        let previous_ordered = simd.equal_f32s(previous, previous);
        let current_ordered = simd.equal_f32s(current, current);
        let maximum = simd.max_f32s(previous, current);
        let maximum = simd.select_f32s(current_ordered, maximum, previous);
        let maximum = simd.select_f32s(previous_ordered, maximum, current);
        let below_upper_bound = simd.less_than_or_equal_f32s(maximum, big);
        let is_zero = simd.equal_f32s(maximum, zeros);
        let lower_check = simd.select_f32s(is_zero, big, maximum);
        let in_range = simd.select_f32s(below_upper_bound, lower_check, zeros);
        if simd.reduce_min_f32s(in_range) < <f32 as LegendrePrecision>::SMALL {
            return true;
        }
        if !IGNORE_SINGLE_NAN {
            let nonfinite = simd.or_m32s(
                simd.not_m32s(previous_ordered),
                simd.not_m32s(current_ordered),
            );
            if simd.first_true_m32s(nonfinite) != S::U32_LANES {
                return true;
            }
        }
    }

    previous_tail
        .iter()
        .zip(current_tail)
        .any(|(&previous, &current)| {
            let maximum = previous.abs().max(current.abs());
            let in_range = maximum <= <f32 as LegendrePrecision>::BIG
                && (maximum == 0.0 || maximum >= <f32 as LegendrePrecision>::SMALL);
            !in_range || (!IGNORE_SINGLE_NAN && (previous.is_nan() || current.is_nan()))
        })
}

/// Multiplies two binary-scaled values and checks their combined exponent.
#[inline(always)]
pub(crate) fn multiply_scaled<T: LegendrePrecision>(
    left: ScaledValue<T>,
    right: ScaledValue<T>,
    overflow_quantity: &'static str,
) -> Result<ScaledValue<T>, ShtError> {
    let exponent = left
        .exponent
        .checked_add(right.exponent)
        .ok_or(ShtError::IntegerOverflow {
            quantity: overflow_quantity,
        })?;
    normalize_scaled_value(left.mantissa * right.mantissa, exponent, overflow_quantity)
}

/// Raises a scalar base to an integer power without leaving the safe interval.
#[inline(always)]
pub(crate) fn scaled_power<T: LegendrePrecision>(
    base: T,
    mut exponent: usize,
    overflow_quantity: &'static str,
) -> Result<ScaledValue<T>, ShtError> {
    let mut result = ScaledValue {
        mantissa: T::one(),
        exponent: 0,
    };
    let mut factor = normalize_scaled_value(base, 0, overflow_quantity)?;
    while exponent != 0 {
        if !exponent.is_multiple_of(2) {
            result = multiply_scaled(result, factor, overflow_quantity)?;
        }
        exponent /= 2;
        if exponent != 0 {
            factor = multiply_scaled(factor, factor, overflow_quantity)?;
        }
    }
    Ok(result)
}

/// Mutable lane state written by scalar-recurrence sectoral initialization.
pub(crate) struct ScalarRecurrenceBuffers<'a, T> {
    pub(crate) previous: &'a mut [T],
    pub(crate) current: &'a mut [T],
    pub(crate) exponents: &'a mut [i64],
}

/// Initializes one scalar-recurrence latitude block from the shared sectoral
/// formula while retaining each precision's local scaled-power leaves.
#[inline]
pub(crate) fn initialize_scalar_recurrence_lanes<T, P, N>(
    m: usize,
    sin_theta: &[T],
    sectoral_norm: T,
    buffers: ScalarRecurrenceBuffers<'_, T>,
    power: P,
    normalize: N,
) -> Result<(), ShtError>
where
    T: LegendrePrecision,
    P: Fn(T, usize) -> Result<ScaledValue<T>, ShtError>,
    N: Fn(T, i64) -> Result<ScaledValue<T>, ShtError>,
{
    let ScalarRecurrenceBuffers {
        previous,
        current,
        exponents,
    } = buffers;
    debug_assert_eq!(sin_theta.len(), previous.len());
    debug_assert_eq!(sin_theta.len(), current.len());
    debug_assert_eq!(sin_theta.len(), exponents.len());

    let sign = if m.is_multiple_of(2) {
        T::one()
    } else {
        -T::one()
    };
    for lane in 0..sin_theta.len() {
        let sectoral = power(sin_theta[lane], m)?;
        let value = normalize(sectoral.mantissa * sectoral_norm * sign, sectoral.exponent)?;
        previous[lane] = T::zero();
        current[lane] = value.mantissa;
        exponents[lane] = value.exponent;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
