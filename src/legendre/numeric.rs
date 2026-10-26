// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use rustfft::num_traits::{Float, FromPrimitive};

use crate::error::ShtError;

/// Shares scalar degree ordering and the three-buffer rotation while expanding
/// precision-specific advance and normalization blocks at compile time.
macro_rules! scalar_degree_traversal {
    (
        $m:ident, $lmax:ident, $ring_start:ident,
        $previous:ident, $current:ident, $next:ident, $exponents:ident, $visit:ident,
        $degree:ident, $advance:block, $normalize:block
    ) => {
        for $degree in $m..=$lmax {
            $visit($degree, $ring_start, $current, $exponents)?;
            if $degree == $lmax {
                break;
            }
            $advance;
            $normalize;
            std::mem::swap(&mut $previous, &mut $current);
            std::mem::swap(&mut $current, &mut $next);
        }
    };
}

pub(crate) use scalar_degree_traversal;

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

#[cfg(test)]
mod tests {
    use super::*;

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
                for l in m..=mmax {
                    let lf = l as f64;
                    let mf = m as f64;
                    let expected = if l == m {
                        0.0
                    } else {
                        (((lf - mf) * (lf + mf)) / ((2.0 * lf + 1.0) * (2.0 * lf - 1.0))).sqrt()
                    };
                    assert_eq!(factors_f64[l].to_bits(), expected.to_bits());
                }

                let mut factors_f32 = vec![f32::NAN; mmax + 1];
                fill_epsilon(m, mmax, &mut factors_f32);
                for l in m..=mmax {
                    let lf = l as f32;
                    let mf = m as f32;
                    let expected = if l == m {
                        0.0
                    } else {
                        (((lf - mf) * (lf + mf)) / ((2.0 * lf + 1.0) * (2.0 * lf - 1.0))).sqrt()
                    };
                    assert_eq!(factors_f32[l].to_bits(), expected.to_bits());
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
