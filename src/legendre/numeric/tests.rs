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
