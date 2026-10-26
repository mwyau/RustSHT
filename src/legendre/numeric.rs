// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use rustfft::num_traits::{Float, FromPrimitive};

use crate::error::ShtError;

/// Precision-dependent limits shared by the scalar and spin recurrences.
///
/// Implementations stay concrete so these values are resolved at compile
/// time in the f32 and f64 kernels.
pub(crate) trait LegendrePrecision: Float + FromPrimitive {
    const BIG: Self;
    const SMALL: Self;
    const SCALE_UP: Self;
    const SCALE_DOWN: Self;
    const SCALE_EXPONENT: i64;
}

impl LegendrePrecision for f32 {
    const BIG: Self = f32::from_bits(167_u32 << 23);
    const SMALL: Self = f32::from_bits(87_u32 << 23);
    const SCALE_UP: Self = f32::from_bits(207_u32 << 23);
    const SCALE_DOWN: Self = f32::from_bits(47_u32 << 23);
    const SCALE_EXPONENT: i64 = 80;
}

impl LegendrePrecision for f64 {
    const BIG: Self = f64::from_bits(1423_u64 << 52);
    const SMALL: Self = f64::from_bits(623_u64 << 52);
    const SCALE_UP: Self = f64::from_bits(1823_u64 << 52);
    const SCALE_DOWN: Self = f64::from_bits(223_u64 << 52);
    const SCALE_EXPONENT: i64 = 800;
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn shared_scaled_arithmetic_preserves_precision<T: LegendrePrecision + std::fmt::Debug>() {
        let two = T::one() + T::one();
        let low = T::SMALL / two;
        let normalized = normalize_scaled_value(low, 0, "test recurrence scale").unwrap();
        assert_eq!(normalized.exponent, -T::SCALE_EXPONENT);
        assert_eq!(normalized.mantissa, low * T::SCALE_UP);

        let powered = scaled_power(T::from_f64(0.75).unwrap(), 3, "test recurrence scale").unwrap();
        assert_eq!(powered.exponent, 0);
        assert_eq!(powered.mantissa, T::from_f64(0.421875).unwrap());

        let overflow = multiply_scaled(
            ScaledValue {
                mantissa: T::one(),
                exponent: i64::MAX,
            },
            ScaledValue {
                mantissa: T::one(),
                exponent: 1,
            },
            "test recurrence scale",
        );
        assert!(matches!(
            overflow,
            Err(ShtError::IntegerOverflow {
                quantity: "test recurrence scale"
            })
        ));
    }

    #[test]
    fn shared_scaled_arithmetic_uses_native_precision_and_checked_exponents() {
        shared_scaled_arithmetic_preserves_precision::<f32>();
        shared_scaled_arithmetic_preserves_precision::<f64>();
    }

    #[test]
    fn shared_scalar_factors_match_precision_specific_formulas_bitwise() {
        for mmax in [0, 1, 7, 255] {
            let actual_f64 = sectoral_normalizations::<f64>(mmax).unwrap();
            let mut expected_f64 = vec![1.0 / (4.0 * std::f64::consts::PI).sqrt()];
            for m in 1..=mmax {
                let mf = m as f64;
                expected_f64.push(expected_f64[m - 1] * ((2.0 * mf + 1.0) / (2.0 * mf)).sqrt());
            }
            assert_eq!(
                actual_f64
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
                expected_f64
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
            );

            let actual_f32 = sectoral_normalizations::<f32>(mmax).unwrap();
            let mut expected_f32 = vec![1.0 / (4.0 * std::f32::consts::PI).sqrt()];
            for m in 1..=mmax {
                let mf = m as f32;
                expected_f32.push(expected_f32[m - 1] * ((2.0 * mf + 1.0) / (2.0 * mf)).sqrt());
            }
            assert_eq!(
                actual_f32
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
                expected_f32
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
            );

            for m in 0..=mmax {
                let mut factors_f64 = vec![f64::NAN; mmax + 1];
                fill_epsilon(m, mmax, &mut factors_f64);
                for (l, &factor) in factors_f64.iter().enumerate().skip(m) {
                    let lf = l as f64;
                    let mf = m as f64;
                    let expected = if l == m {
                        0.0
                    } else {
                        (((lf - mf) * (lf + mf)) / ((2.0 * lf + 1.0) * (2.0 * lf - 1.0))).sqrt()
                    };
                    assert_eq!(factor.to_bits(), expected.to_bits());
                }

                let mut factors_f32 = vec![f32::NAN; mmax + 1];
                fill_epsilon(m, mmax, &mut factors_f32);
                for (l, &factor) in factors_f32.iter().enumerate().skip(m) {
                    let lf = l as f32;
                    let mf = m as f32;
                    let expected = if l == m {
                        0.0
                    } else {
                        (((lf - mf) * (lf + mf)) / ((2.0 * lf + 1.0) * (2.0 * lf - 1.0))).sqrt()
                    };
                    assert_eq!(factor.to_bits(), expected.to_bits());
                }
            }
        }
    }

    #[test]
    fn normal_range_policy_preserves_zero_finite_and_exceptional_boundaries() {
        for value in [
            0.0_f32,
            -0.0,
            <f32 as LegendrePrecision>::SMALL,
            -<f32 as LegendrePrecision>::BIG,
        ] {
            assert!(value_in_normal_range(value));
        }
        for value in [
            f32::from_bits(<f32 as LegendrePrecision>::SMALL.to_bits() - 1),
            f32::from_bits(<f32 as LegendrePrecision>::BIG.to_bits() + 1),
            f32::INFINITY,
            f32::NAN,
        ] {
            assert!(!value_in_normal_range(value));
        }

        for value in [
            0.0_f64,
            -0.0,
            <f64 as LegendrePrecision>::SMALL,
            -<f64 as LegendrePrecision>::BIG,
        ] {
            assert!(value_in_normal_range(value));
        }
        for value in [
            f64::from_bits(<f64 as LegendrePrecision>::SMALL.to_bits() - 1),
            f64::from_bits(<f64 as LegendrePrecision>::BIG.to_bits() + 1),
            f64::INFINITY,
            f64::NAN,
        ] {
            assert!(!value_in_normal_range(value));
        }
    }
}
