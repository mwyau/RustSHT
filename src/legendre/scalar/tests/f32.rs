use super::super::super::numeric::scale_binary_f32 as scale_binary;
use super::super::super::scalar_paired::synthesize_paired_scalar_order_prepared_f32;
use super::super::*;
use super::super::{
    BIG_F32 as BIG, SCALE_DOWN_F32 as SCALE_DOWN, SCALE_UP_F32 as SCALE_UP, SMALL_F32 as SMALL,
    multiply_complex_scaled_f32 as multiply_complex_scaled,
    multiply_component_scaled_f32 as multiply_component_scaled,
    normalize_pair_lanes_f32 as normalize_pair_lanes,
    normalize_pair_lanes_with_state_f32 as normalize_pair_lanes_with_state,
    pair_is_in_normal_range_f32 as pair_is_in_normal_range,
    pair_lanes_need_scaling_f32 as pair_lanes_need_scaling,
    scale_normal_binary_f32 as scale_normal_binary, split_binary_f32 as split_binary,
};
use std::f32::consts::PI;

fn sectoral_normalizations(mmax: usize) -> Result<Vec<f32>, ShtError> {
    super::super::super::numeric::sectoral_normalizations(mmax)
}

fn fill_epsilon(m: usize, lmax: usize, output: &mut [f32]) {
    super::super::super::numeric::fill_epsilon(m, lmax, output);
}

fn fill_paired_scalar_factors(
    m: usize,
    lmax: usize,
    epsilon: &[f32],
    alpha: &mut [f32],
    coefficient_a: &mut [f32],
    coefficient_b: &mut [f32],
) -> bool {
    super::super::super::scalar_paired::fill_factors_typed(
        m,
        lmax,
        epsilon,
        alpha,
        coefficient_a,
        coefficient_b,
        super::super::super::scalar_paired::PAIRED_FACTOR_LIMIT_F32,
    )
}

struct PairRangeCheck<'a> {
    previous: &'a [f32],
    current: &'a [f32],
}

impl WithSimd for PairRangeCheck<'_> {
    type Output = bool;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        pair_lanes_need_scaling(simd, self.previous, self.current)
    }
}

fn reference_multiply_component_scaled(value: f32, factor: f32, exponent: i64) -> f32 {
    if value == 0.0 || factor == 0.0 || !value.is_finite() || !factor.is_finite() {
        return scale_binary(value * factor, exponent);
    }
    let (value_mantissa, value_exponent) = split_binary(value);
    let (factor_mantissa, factor_exponent) = split_binary(factor);
    scale_binary(
        value_mantissa * factor_mantissa,
        exponent
            .saturating_add(value_exponent)
            .saturating_add(factor_exponent),
    )
}

fn reference_scale_binary(mut value: f32, mut exponent: i64) -> f32 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    if exponent > 4096 {
        return value.signum() * f32::INFINITY;
    }
    if exponent < -4096 {
        return 0.0_f32.copysign(value);
    }
    while exponent > 127 {
        value *= 2.0_f32.powi(127);
        exponent -= 127;
        if !value.is_finite() {
            return value;
        }
    }
    while exponent < -126 {
        value *= 2.0_f32.powi(-126);
        exponent += 126;
        if value == 0.0 {
            return value;
        }
    }
    value * 2.0_f32.powi(exponent as i32)
}

fn reference_normalize_pair_lanes(
    previous: &mut [f32],
    current: &mut [f32],
    exponents: &mut [i64],
) {
    for lane in 0..previous.len() {
        let mut maximum = previous[lane].abs().max(current[lane].abs());
        while maximum > BIG {
            previous[lane] *= SCALE_DOWN;
            current[lane] *= SCALE_DOWN;
            exponents[lane] += 80;
            maximum *= SCALE_DOWN;
        }
        while maximum != 0.0 && maximum < SMALL {
            previous[lane] *= SCALE_UP;
            current[lane] *= SCALE_UP;
            exponents[lane] -= 80;
            maximum *= SCALE_UP;
        }
    }
}

#[test]
fn f32_pair_normalization_matches_float_magnitude_reference() {
    let initial_previous = [
        0.0,
        f32::from_bits(1),
        f32::from_bits(SMALL.to_bits() - 1),
        -f32::from_bits(BIG.to_bits() + 1),
        f32::MAX,
        f32::MIN_POSITIVE,
        -0.5,
        0.0,
    ];
    let initial_current = [
        -0.0,
        0.0,
        0.0,
        BIG,
        f32::MIN_POSITIVE,
        -f32::from_bits(1),
        0.25,
        SMALL,
    ];
    let mut actual_previous = initial_previous;
    let mut actual_current = initial_current;
    let mut actual_exponents = [0; 8];
    let mut expected_previous = initial_previous;
    let mut expected_current = initial_current;
    let mut expected_exponents = [0; 8];

    normalize_pair_lanes(
        &mut actual_previous,
        &mut actual_current,
        &mut actual_exponents,
    )
    .unwrap();
    reference_normalize_pair_lanes(
        &mut expected_previous,
        &mut expected_current,
        &mut expected_exponents,
    );

    for lane in 0..initial_previous.len() {
        assert_eq!(
            actual_previous[lane].to_bits(),
            expected_previous[lane].to_bits()
        );
        assert_eq!(
            actual_current[lane].to_bits(),
            expected_current[lane].to_bits()
        );
        assert_eq!(actual_exponents[lane], expected_exponents[lane]);
    }
}

struct NormalizeF32PairWithSimd<'a> {
    previous: &'a mut [f32],
    current: &'a mut [f32],
    exponents: &'a mut [i64],
}

impl pulp::WithSimd for NormalizeF32PairWithSimd<'_> {
    type Output = Result<bool, ShtError>;

    #[inline(always)]
    fn with_simd<S: pulp::Simd>(self, simd: S) -> Self::Output {
        super::super::normalize_pair_lanes_with_state_f32_simd(
            simd,
            self.previous,
            self.current,
            self.exponents,
        )
    }
}

#[test]
fn f32_simd_pair_normalization_matches_scalar_epochs_and_bits() {
    let initial_previous = [
        0.0,
        f32::from_bits(1),
        f32::from_bits(SMALL.to_bits() - 1),
        -f32::from_bits(BIG.to_bits() + 1),
        f32::MAX,
        f32::MIN_POSITIVE,
        -0.5,
        0.0,
        1.0,
        -f32::MAX,
        0.0,
        f32::from_bits(1),
        SMALL,
        BIG,
        -3.0,
        0.0,
        f32::from_bits(1),
    ];
    let initial_current = [
        -0.0,
        0.0,
        0.0,
        BIG,
        f32::MIN_POSITIVE,
        -f32::from_bits(1),
        0.25,
        SMALL,
        -2.0,
        0.0,
        f32::MAX,
        -f32::MAX,
        f32::from_bits(1),
        SMALL,
        0.0,
        0.0,
        0.0,
    ];

    for arch in [Arch::Scalar, Arch::new()] {
        let mut actual_previous = initial_previous;
        let mut actual_current = initial_current;
        let mut actual_exponents = [0; 17];
        let mut expected_previous = initial_previous;
        let mut expected_current = initial_current;
        let mut expected_exponents = [0; 17];

        let actual_normal = arch
            .dispatch(NormalizeF32PairWithSimd {
                previous: &mut actual_previous,
                current: &mut actual_current,
                exponents: &mut actual_exponents,
            })
            .unwrap();
        let expected_normal = normalize_pair_lanes_with_state(
            &mut expected_previous,
            &mut expected_current,
            &mut expected_exponents,
        )
        .unwrap();

        assert_eq!(actual_normal, expected_normal);
        assert_eq!(actual_exponents, expected_exponents);
        assert_eq!(
            actual_previous.map(f32::to_bits),
            expected_previous.map(f32::to_bits)
        );
        assert_eq!(
            actual_current.map(f32::to_bits),
            expected_current.map(f32::to_bits)
        );
    }
}

#[test]
fn f32_normal_range_check_covers_simd_lanes_and_tail() {
    for (previous_value, current_value) in [
        (0.0, 0.0),
        (0.0, SMALL / 2.0),
        (BIG, SMALL / 2.0),
        (BIG * 2.0, 0.0),
        (SMALL, BIG),
        (f32::NAN, BIG),
        (BIG, f32::NAN),
        (f32::NAN, SMALL / 2.0),
        (f32::NAN, f32::NAN),
        (f32::INFINITY, f32::NAN),
    ] {
        let previous = [previous_value; 17];
        let current = [current_value; 17];
        let expected = !pair_is_in_normal_range(previous_value, current_value);
        for arch in [Arch::Scalar, Arch::new()] {
            let actual = arch.dispatch(PairRangeCheck {
                previous: &previous,
                current: &current,
            });
            assert_eq!(actual, expected, "{previous_value:?}, {current_value:?}");
        }
    }

    let mut previous = vec![0.25_f32; 17];
    let mut current = vec![0.5_f32; 17];
    let mut exponents = vec![0_i64; 17];
    previous[0] = 0.0;
    current[0] = -0.0;
    previous[1] = SMALL;
    current[1] = 0.0;
    previous[2] = BIG;

    for arch in [Arch::Scalar, Arch::new()] {
        let needs_scaling =
            |previous: &[f32], current: &[f32]| arch.dispatch(PairRangeCheck { previous, current });
        assert!(!needs_scaling(&previous, &current));

        let mut normal_previous = previous.clone();
        let mut normal_current = current.clone();
        let mut normal_exponents = exponents.clone();
        assert!(
            normalize_pair_lanes_with_state(
                &mut normal_previous,
                &mut normal_current,
                &mut normal_exponents,
            )
            .unwrap()
        );
        assert_eq!(normal_previous, previous);
        assert_eq!(normal_current, current);

        let mut below = current.clone();
        below[3] = f32::from_bits(SMALL.to_bits() - 1);
        let mut below_previous = previous.clone();
        below_previous[3] = 0.0;
        assert!(needs_scaling(&below_previous, &below));

        let mut above = current.clone();
        above[16] = f32::from_bits(BIG.to_bits() + 1);
        assert!(needs_scaling(&previous, &above));
    }

    exponents[8] = 1;
    assert!(!normalize_pair_lanes_with_state(&mut previous, &mut current, &mut exponents).unwrap());
}

#[test]
fn f32_binary_scaling_matches_staged_products() {
    let values = [
        0.0,
        -0.0,
        0.125,
        -0.75,
        1.0,
        -1.5,
        f32::MIN_POSITIVE,
        f32::from_bits(0x00ff_ffff),
        f32::from_bits(1),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let exponents = [
        -4097, -512, -254, -127, -126, -81, -1, 0, 1, 81, 126, 127, 512, 4097,
    ];
    for value in values {
        for exponent in exponents {
            let actual = scale_binary(value, exponent);
            let expected = reference_scale_binary(value, exponent);
            assert_eq!(actual.to_bits(), expected.to_bits());
            if value.is_normal() {
                let scaled = scale_normal_binary(value, exponent);
                let raw_exponent = i64::from((value.to_bits() >> 23) & 0xff);
                let scaled_exponent = raw_exponent + exponent;
                let should_use_fast_path = (1..0xff).contains(&scaled_exponent);
                assert_eq!(scaled.is_some(), should_use_fast_path);
                if let Some(scaled) = scaled {
                    assert_eq!(scaled.to_bits(), expected.to_bits());
                }
            }
        }
    }
}

#[test]
fn f32_scaled_complex_multiply_matches_split_reference() {
    let values = [
        0.0,
        -0.0,
        0.125,
        -0.75,
        1.0,
        -1.5,
        f32::MIN_POSITIVE,
        -f32::MAX,
        f32::from_bits(0x017f_ffff),
        f32::from_bits(1),
        f32::from_bits(0x6e46_ed13),
    ];
    let factors = [
        0.5,
        -0.75,
        1.125,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        0.0,
        f32::INFINITY,
        f32::from_bits(0x21d3_bb30),
    ];
    let exponents = [-4097, -512, -255, -127, -126, -1, 0, 1, 126, 127, 512, 4097];

    let boundary_value = f32::from_bits(0x017f_ffff);
    assert_eq!((boundary_value * 0.5).to_bits(), 0x00ff_ffff);
    let boundary_actual =
        multiply_complex_scaled(Complex32::new(boundary_value, -boundary_value), 0.5, -1);
    let boundary_expected = Complex32::new(
        reference_multiply_component_scaled(boundary_value, 0.5, -1),
        reference_multiply_component_scaled(-boundary_value, 0.5, -1),
    );
    assert_eq!(boundary_actual.re.to_bits(), boundary_expected.re.to_bits());
    assert_eq!(boundary_actual.im.to_bits(), boundary_expected.im.to_bits());

    for value in values {
        for factor in factors {
            for exponent in exponents {
                let actual = multiply_component_scaled(value, factor, exponent);
                let expected = reference_multiply_component_scaled(value, factor, exponent);
                assert_eq!(
                    actual.to_bits(),
                    expected.to_bits(),
                    "value={value:e}, factor={factor:e}, exponent={exponent}"
                );
                let complex = Complex32::new(value, -value);
                let actual = multiply_complex_scaled(complex, factor, exponent);
                let expected = Complex32::new(
                    reference_multiply_component_scaled(value, factor, exponent),
                    reference_multiply_component_scaled(-value, factor, exponent),
                );
                assert_eq!(actual.re.to_bits(), expected.re.to_bits());
                assert_eq!(actual.im.to_bits(), expected.im.to_bits());
            }
        }
    }

    for (complex, factor, exponent) in [
        (Complex32::new(1.5, 0.25), 0.75, 2),
        (Complex32::new(f32::MIN_POSITIVE, -1.5), 0.5, -1),
        (Complex32::new(f32::MAX, 0.25), 2.0, 0),
    ] {
        let actual = multiply_complex_scaled(complex, factor, exponent);
        let expected = Complex32::new(
            reference_multiply_component_scaled(complex.re, factor, exponent),
            reference_multiply_component_scaled(complex.im, factor, exponent),
        );
        assert_eq!(actual.re.to_bits(), expected.re.to_bits());
        assert_eq!(actual.im.to_bits(), expected.im.to_bits());
    }
}

fn values_over_rings(arch: Arch, m: usize, lmax: usize, theta: &[f32]) -> Vec<Vec<f32>> {
    let sin_theta = theta.iter().map(|value| value.sin()).collect::<Vec<_>>();
    let cos_theta = theta.iter().map(|value| value.cos()).collect::<Vec<_>>();
    values_over_geometry(arch, m, lmax, &sin_theta, &cos_theta)
}

fn values_over_geometry(
    arch: Arch,
    m: usize,
    lmax: usize,
    sin_theta: &[f32],
    cos_theta: &[f32],
) -> Vec<Vec<f32>> {
    let mut epsilon = vec![0.0; lmax + 1];
    fill_epsilon(m, lmax, &mut epsilon);
    let scratch_len = sin_theta.len().min(LATITUDE_BLOCK_SIZE);
    let mut previous = vec![0.0; scratch_len];
    let mut current = vec![0.0; scratch_len];
    let mut next = vec![0.0; scratch_len];
    let mut exponents = vec![0; scratch_len];
    let mut result = vec![vec![0.0; sin_theta.len()]; lmax - m + 1];
    let sectoral_norm = sectoral_normalizations(m).unwrap()[m];
    crate::legendre::scalar::for_each_y_lm_dispatched(
        arch,
        m,
        lmax,
        sin_theta,
        cos_theta,
        sectoral_norm,
        &epsilon,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
        |degree, ring_start, values, exponents| {
            for lane in 0..values.len() {
                result[degree - m][ring_start + lane] = scale_binary(values[lane], exponents[lane]);
            }
            Ok(())
        },
    )
    .unwrap();
    result
}

#[test]
fn f32_paired_accumulator_matches_direct_synthesis_and_declines_polar_orders() {
    assert!(<f32 as crate::legendre::ShtScalar>::supports_paired_scalar());
    assert!(!<f32 as crate::legendre::ShtScalar>::supports_paired_spin());

    let sine_samples = [
        0.0_f32,
        f32::from_bits(1),
        1.0e-12,
        1.0e-5,
        0.01,
        0.1,
        0.3,
        0.7,
        1.0,
    ];
    let mut sin_theta = Vec::with_capacity(81);
    let mut cos_theta = Vec::with_capacity(81);
    for lane in 0..81 {
        let sample = lane % sine_samples.len();
        let sine = sine_samples[sample];
        let cosine = (1.0 - sine * sine).sqrt();
        sin_theta.push(sine);
        cos_theta.push(if sample.is_multiple_of(3) {
            -cosine
        } else {
            cosine
        });
    }

    for m in [0, 1, 2, 4, 15, 31] {
        for degree_span in [8, 9] {
            let lmax = m + degree_span;
            let expected = values_over_geometry(Arch::Scalar, m, lmax, &sin_theta, &cos_theta);
            let coefficients: Vec<_> = (m..=lmax)
                .map(|degree| {
                    Complex32::new(
                        (0.31_f32 * degree as f32 + 0.73_f32 * m as f32).sin(),
                        (0.19_f32 * degree as f32 - 0.41_f32 * m as f32).cos(),
                    )
                })
                .collect();
            let mut epsilon = vec![0.0; lmax + 4];
            fill_epsilon(m, lmax, &mut epsilon);
            let factor_count = lmax / 2 + 2;
            let mut alpha = vec![0.0; factor_count];
            let mut coefficient_a = vec![0.0; factor_count];
            let mut coefficient_b = vec![0.0; factor_count];
            assert!(fill_paired_scalar_factors(
                m,
                lmax,
                &epsilon,
                &mut alpha,
                &mut coefficient_a,
                &mut coefficient_b,
            ));
            let pair_count = (lmax - m + 2) / 2;
            let mut coefficient_even = vec![Complex32::new(0.0, 0.0); pair_count];
            let mut coefficient_odd = coefficient_even.clone();
            assert!(
                super::super::super::scalar_paired::prepare_synthesis_coefficients(
                    m,
                    lmax,
                    &epsilon,
                    &alpha,
                    &coefficients,
                    &mut coefficient_even,
                    &mut coefficient_odd,
                )
            );

            let spectrum_len = m + 1;
            let sectoral_norm = sectoral_normalizations(m).unwrap()[m];
            for arch in [Arch::Scalar, Arch::new()] {
                let seed = Complex32::new(0.125, -0.25);
                let mut spectra = vec![seed; sin_theta.len() * spectrum_len];
                let before = spectra.clone();
                let ring_order_limits = vec![m; sin_theta.len()];
                let scratch_len = sin_theta.len().min(LATITUDE_BLOCK_SIZE);
                let mut previous = vec![0.0; scratch_len];
                let mut current = vec![0.0; scratch_len];
                let mut next = vec![0.0; scratch_len];
                let mut exponents = vec![0; scratch_len];
                let mut staged_real = vec![0.0; sin_theta.len()];
                let mut staged_imag = vec![0.0; sin_theta.len()];
                assert!(
                    synthesize_paired_scalar_order_prepared_f32(
                        arch,
                        m,
                        m,
                        lmax,
                        &sin_theta,
                        &cos_theta,
                        sectoral_norm,
                        &coefficient_a,
                        &coefficient_b,
                        &coefficient_even,
                        &coefficient_odd,
                        &ring_order_limits,
                        spectrum_len,
                        &mut spectra,
                        &mut previous,
                        &mut current,
                        &mut next,
                        &mut exponents,
                        &mut staged_real,
                        &mut staged_imag,
                    )
                    .unwrap()
                );

                for ring in 0..sin_theta.len() {
                    let mut direct_sum = Complex32::new(0.0, 0.0);
                    for (offset, basis) in expected.iter().enumerate() {
                        direct_sum += coefficients[offset] * basis[ring];
                    }
                    let paired_sum = spectra[ring * spectrum_len + m];
                    let expected_sum = before[ring * spectrum_len + m] + direct_sum;
                    let tolerance = 5.0e-5_f32 * (1.0 + direct_sum.norm());
                    assert!(
                        paired_sum.re.is_finite()
                            && paired_sum.im.is_finite()
                            && (paired_sum - expected_sum).norm() <= tolerance,
                        "arch={arch:?}, m={m}, lmax={lmax}, ring={ring}: paired={paired_sum:?}, direct={expected_sum:?}"
                    );
                }

                if m > 0 {
                    let mut declined = vec![seed; sin_theta.len() * spectrum_len];
                    let original = declined.clone();
                    let mut polar_limits = vec![m; sin_theta.len()];
                    polar_limits[0] = m - 1;
                    assert!(
                        !synthesize_paired_scalar_order_prepared_f32(
                            arch,
                            m,
                            m,
                            lmax,
                            &sin_theta,
                            &cos_theta,
                            sectoral_norm,
                            &coefficient_a,
                            &coefficient_b,
                            &coefficient_even,
                            &coefficient_odd,
                            &polar_limits,
                            spectrum_len,
                            &mut declined,
                            &mut previous,
                            &mut current,
                            &mut next,
                            &mut exponents,
                            &mut staged_real,
                            &mut staged_imag,
                        )
                        .unwrap()
                    );
                    assert_eq!(declined, original);
                }

                if m == 0 && degree_span == 8 {
                    let too_large_even = vec![Complex32::new(f32::MAX, 0.0); pair_count];
                    let too_large_odd = too_large_even.clone();
                    let mut declined = vec![seed; sin_theta.len() * spectrum_len];
                    let original = declined.clone();
                    assert!(
                        !synthesize_paired_scalar_order_prepared_f32(
                            arch,
                            m,
                            m,
                            lmax,
                            &sin_theta,
                            &cos_theta,
                            sectoral_norm,
                            &coefficient_a,
                            &coefficient_b,
                            &too_large_even,
                            &too_large_odd,
                            &ring_order_limits,
                            spectrum_len,
                            &mut declined,
                            &mut previous,
                            &mut current,
                            &mut next,
                            &mut exponents,
                            &mut staged_real,
                            &mut staged_imag,
                        )
                        .unwrap()
                    );
                    assert_eq!(declined, original);
                }
            }
        }
    }
}

#[test]
fn f32_fused_synthesis_matches_reference_and_declines_overflowing_sums() {
    let m = 0;
    let lmax = 8;
    let sin_theta = (0..17)
        .map(|ring| 0.25_f32 + 0.1 * (ring % 6) as f32)
        .collect::<Vec<_>>();
    let cos_theta = sin_theta
        .iter()
        .map(|sine| (1.0 - sine * sine).sqrt())
        .collect::<Vec<_>>();
    let sectoral_norm = sectoral_normalizations(m).unwrap()[m];
    let mut seed_previous = vec![0.0_f32; sin_theta.len()];
    let mut seed_current = vec![0.0_f32; sin_theta.len()];
    let mut seed_exponents = vec![0_i64; sin_theta.len()];
    <f32 as crate::legendre::scalar_paired::PairedScalarRecurrencePrecision>::initialize_pair_state(
        m,
        &sin_theta,
        sectoral_norm,
        &mut seed_previous,
        &mut seed_current,
        &mut seed_exponents,
    )
    .unwrap();
    assert!(
        <f32 as crate::legendre::scalar_paired::PairedScalarRecurrencePrecision>::normalize_pair_state(
            &mut seed_previous,
            &mut seed_current,
            &mut seed_exponents,
        )
        .unwrap()
    );

    let expected_basis = values_over_geometry(Arch::Scalar, m, lmax, &sin_theta, &cos_theta);
    let coefficients = (0..=lmax)
        .map(|degree| Complex32::new(0.17 * (degree + 1) as f32, 0.0))
        .collect::<Vec<_>>();
    let mut epsilon = vec![0.0_f32; lmax + 4];
    fill_epsilon(m, lmax, &mut epsilon);
    let factor_count = lmax / 2 + 2;
    let mut alpha = vec![0.0_f32; factor_count];
    let mut coefficient_a = vec![0.0_f32; factor_count];
    let mut coefficient_b = vec![0.0_f32; factor_count];
    assert!(fill_paired_scalar_factors(
        m,
        lmax,
        &epsilon,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    ));
    let pair_count = (lmax - m + 2) / 2;
    let mut coefficient_even = vec![Complex32::new(0.0, 0.0); pair_count];
    let mut coefficient_odd = coefficient_even.clone();
    assert!(
        super::super::super::scalar_paired::prepare_synthesis_coefficients(
            m,
            lmax,
            &epsilon,
            &alpha,
            &coefficients,
            &mut coefficient_even,
            &mut coefficient_odd,
        )
    );

    let spectrum_len = m + 1;
    let ring_order_limits = vec![m; sin_theta.len()];
    let seed = Complex32::new(0.125, -0.25);
    let original_spectra = vec![seed; sin_theta.len() * spectrum_len];
    for arch in [Arch::Scalar, Arch::new()] {
        let mut spectra = original_spectra.clone();
        let mut previous = vec![0.0_f32; sin_theta.len()];
        let mut current = vec![0.0_f32; sin_theta.len()];
        let mut next = vec![0.0_f32; sin_theta.len()];
        let mut exponents = vec![0_i64; sin_theta.len()];
        let mut staged_real = vec![0.0_f32; sin_theta.len()];
        let mut staged_imag = vec![0.0_f32; sin_theta.len()];
        assert!(
            synthesize_paired_scalar_order_prepared_f32(
                arch,
                m,
                m,
                lmax,
                &sin_theta,
                &cos_theta,
                sectoral_norm,
                &coefficient_a,
                &coefficient_b,
                &coefficient_even,
                &coefficient_odd,
                &ring_order_limits,
                spectrum_len,
                &mut spectra,
                &mut previous,
                &mut current,
                &mut next,
                &mut exponents,
                &mut staged_real,
                &mut staged_imag,
            )
            .unwrap()
        );
        for ring in 0..sin_theta.len() {
            let direct = coefficients
                .iter()
                .zip(&expected_basis)
                .fold(Complex32::new(0.0, 0.0), |sum, (&coefficient, basis)| {
                    sum + coefficient * basis[ring]
                });
            let expected = original_spectra[ring * spectrum_len + m] + direct;
            let actual = spectra[ring * spectrum_len + m];
            assert!(
                (actual - expected).norm() <= 2.0e-5 * (1.0 + expected.norm()),
                "arch={arch:?}, ring={ring}, actual={actual:?}, expected={expected:?}"
            );
        }

        let too_large = vec![Complex32::new(f32::MAX, 0.0); pair_count];
        let mut declined = original_spectra.clone();
        assert!(
            !synthesize_paired_scalar_order_prepared_f32(
                arch,
                m,
                m,
                lmax,
                &sin_theta,
                &cos_theta,
                sectoral_norm,
                &coefficient_a,
                &coefficient_b,
                &too_large,
                &too_large,
                &ring_order_limits,
                spectrum_len,
                &mut declined,
                &mut previous,
                &mut current,
                &mut next,
                &mut exponents,
                &mut staged_real,
                &mut staged_imag,
            )
            .unwrap()
        );
        assert_eq!(declined, original_spectra);
    }
}

#[test]
fn f32_paired_factor_limit_covers_supported_lmax_for_every_order() {
    for lmax in [255, 511, 720, 1023] {
        let mut epsilon = vec![0.0; lmax + 4];
        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];

        for m in 0..=lmax {
            fill_epsilon(m, lmax, &mut epsilon);
            assert!(
                fill_paired_scalar_factors(
                    m,
                    lmax,
                    &epsilon,
                    &mut alpha,
                    &mut coefficient_a,
                    &mut coefficient_b,
                ),
                "m={m}, lmax={lmax}"
            );
        }
    }
}

#[test]
fn runtime_simd_recurrence_matches_scalar_fallback_across_blocks_and_tails() {
    let mut theta = (0..129)
        .map(|ring| PI * (ring as f32 + 0.5) / 129.0)
        .collect::<Vec<_>>();
    theta[0] = 1.0e-6;
    theta[1] = 1.0e-3;
    theta[127] = PI - 1.0e-3;
    theta[128] = PI - 1.0e-6;

    for m in [0, 1, 2, 31, 96] {
        let expected = values_over_rings(Arch::Scalar, m, 127, &theta);
        let actual = values_over_rings(Arch::new(), m, 127, &theta);
        for (degree_offset, (expected_values, actual_values)) in
            expected.iter().zip(&actual).enumerate()
        {
            for (ring, (&expected, &actual)) in
                expected_values.iter().zip(actual_values).enumerate()
            {
                assert!(
                    actual.is_finite(),
                    "m={m}, l={}, ring={ring}: non-finite result {actual}",
                    m + degree_offset
                );
                assert!(
                    (actual - expected).abs() <= 5.0e-6,
                    "m={m}, l={}, ring={ring}: actual {actual:.9e}, scalar {expected:.9e}",
                    m + degree_offset
                );
            }
        }
    }
}
