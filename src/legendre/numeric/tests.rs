use super::*;
use pulp::{Arch, Simd, WithSimd};

struct F32VectorScaling<'a> {
    values: &'a [f32],
    scales: &'a [i64],
    output: &'a mut [f32],
}

impl WithSimd for F32VectorScaling<'_> {
    type Output = bool;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        scale_binary_f32_simd(simd, self.values, self.scales, self.output)
    }
}

struct F32UniformVectorScaling<'a> {
    values: &'a [f32],
    output: &'a mut [f32],
}

impl WithSimd for F32UniformVectorScaling<'_> {
    type Output = bool;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        scale_binary_f32_uniform(simd, self.values, self.output, -80)
    }
}

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

#[test]
fn f32_binary_scaling_matches_correctly_rounded_reference() {
    let mut values = vec![
        f32::from_bits(1),
        f32::from_bits(2),
        f32::from_bits(0x007f_ffff),
        f32::MIN_POSITIVE,
        f32::from_bits(f32::MIN_POSITIVE.to_bits() + 1),
        0.5,
        -1.0,
        f32::MAX,
        -f32::MAX,
    ];
    let mut state = 0x9e37_79b9_u32;
    for _ in 0..64 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let candidate = f32::from_bits(state);
        if candidate.is_finite() {
            values.push(candidate);
        }
    }

    for value in values {
        for exponent in -500_i32..=500 {
            let expected = (f64::from(value) * 2.0_f64.powi(exponent)) as f32;
            let actual = scale_binary_f32(value, i64::from(exponent));
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "value={value:?} exponent={exponent}"
            );
        }
    }

    assert_eq!(scale_binary_f32(f32::from_bits(1), 23), f32::MIN_POSITIVE);
    assert_eq!(
        scale_binary_f32(-f32::from_bits(1), i64::MIN).to_bits(),
        0x8000_0000
    );
    assert_eq!(scale_binary_f32(f32::from_bits(1), i64::MAX), f32::INFINITY);
    assert_eq!(
        scale_binary_f32(-0.0, i64::MAX).to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(scale_binary_f32(f32::NAN, 80).to_bits(), f32::NAN.to_bits());
}

#[test]
fn f32_simd_binary_scaling_matches_exact_lane_scaling() {
    let mut values = vec![
        f32::from_bits(1),
        f32::from_bits(2),
        f32::from_bits(0x007f_ffff),
        f32::MIN_POSITIVE,
        f32::from_bits(f32::MIN_POSITIVE.to_bits() + 1),
        0.5,
        -1.0,
        f32::MAX,
        -f32::MAX,
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let mut state = 0x243f_6a88_u32;
    for _ in 0..97 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let candidate = f32::from_bits(state);
        if candidate.is_finite() {
            values.push(candidate);
        }
    }

    let scales: Vec<i64> = (0..values.len())
        .map(|index| match index % 7 {
            0 => -12,
            1 => -8,
            2 => -3,
            3 => -1,
            4 => 0,
            5 => 4,
            _ => 11,
        })
        .collect();
    let expected: Vec<f32> = values
        .iter()
        .zip(&scales)
        .map(|(&value, &scale)| {
            scale_binary_f32(value, scale * <f32 as LegendrePrecision>::SCALE_EXPONENT)
        })
        .collect();

    for arch in [Arch::Scalar, Arch::new()] {
        let mut actual = vec![f32::NAN; values.len()];
        assert!(arch.dispatch(F32VectorScaling {
            values: &values,
            scales: &scales,
            output: &mut actual,
        }));
        assert_eq!(
            actual
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
        );
    }

    let uniform_scales = vec![4_i64; values.len()];
    let uniform_expected: Vec<f32> = values
        .iter()
        .map(|&value| scale_binary_f32(value, 4 * <f32 as LegendrePrecision>::SCALE_EXPONENT))
        .collect();
    for arch in [Arch::Scalar, Arch::new()] {
        let mut actual = vec![f32::NAN; values.len()];
        assert!(arch.dispatch(F32VectorScaling {
            values: &values,
            scales: &uniform_scales,
            output: &mut actual,
        }));
        assert_eq!(
            actual
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            uniform_expected
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
        );
    }
}

#[test]
fn f32_uniform_minus_80_simd_scaling_matches_exact_scalar_scaling_and_tail() {
    let mut values = vec![-0.0_f32, 0.0, f32::from_bits(1), -f32::from_bits(1)];
    for exponent_field in 1..=254_u32 {
        for fraction in [0, 1, 0x003f_ffff, 0x0040_0000, 0x007f_ffff] {
            let bits = (exponent_field << 23) | fraction;
            values.push(f32::from_bits(bits));
            values.push(f32::from_bits(bits | 0x8000_0000));
        }
    }
    let mut state = 0x9e37_79b9_u32;
    for _ in 0..513 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let value = f32::from_bits(state);
        if value.is_finite() {
            values.push(value);
        }
    }
    while values.len().is_multiple_of(8) {
        values.push(1.0);
    }
    assert_ne!(values.len() % 8, 0, "exercise the AVX2 scalar tail");

    for arch in [Arch::Scalar, Arch::new()] {
        let mut output = vec![f32::NAN; values.len()];
        assert!(arch.dispatch(F32UniformVectorScaling {
            values: &values,
            output: &mut output,
        }));
        for (lane, (&value, &actual)) in values.iter().zip(&output).enumerate() {
            assert_eq!(
                actual.to_bits(),
                scale_binary_f32(value, -80).to_bits(),
                "lane {lane}, {value:?}"
            );
        }
    }

    let exceptional = [f32::INFINITY; 8];
    let mut output = [0.0; 8];
    assert!(!Arch::new().dispatch(F32UniformVectorScaling {
        values: &exceptional,
        output: &mut output,
    }));
}

#[test]
fn f32_shared_basis_zero_scale_limit_covers_the_recurrence_bound() {
    assert_eq!(
        <f32 as OrderedSimdPrecision>::SHARED_BASIS_ZERO_SCALE_LIMIT,
        -2
    );
    let bound = <f32 as OrderedSimdPrecision>::SCALE_FTOL;
    for value in [
        bound,
        -bound,
        f32::from_bits(1),
        -f32::from_bits(1),
        0.0,
        -0.0,
    ] {
        assert_eq!(
            scale_binary_f32(value, -2 * <f32 as LegendrePrecision>::SCALE_EXPONENT).to_bits(),
            (0.0_f32.copysign(value)).to_bits(),
            "value={value:?}"
        );
    }
    let next_exponent = -<f32 as LegendrePrecision>::SCALE_EXPONENT;
    assert_eq!(
        scale_binary_f32(bound, next_exponent).to_bits(),
        512,
        "the less negative epoch remains a nonzero subnormal"
    );
}
