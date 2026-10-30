use super::{
    FTOL, SCALE_DOWN, SCALE_UP, WithSimd, all_scale_nonnegative, initialize_sectoral_batch, mypow,
    normalize_batch, normalize_lane, power_lower_limit, rescale_batch, rescale_prefix_step,
    scalar_integer_power, set_correction_factors, validate_reflected_geometry,
};
#[cfg(feature = "parallel")]
use num_complex::Complex64;
use pulp::{Arch, Simd};

#[cfg(feature = "parallel")]
use super::initialize_inactive_spectrum_tails;

struct InitializeDispatch<'a> {
    sine: &'a [f64],
    values: &'a mut [f64],
    scales: &'a mut [i64],
    exponent: usize,
    lower_limit: f64,
    factor: f64,
}

impl WithSimd for InitializeDispatch<'_> {
    type Output = bool;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        initialize_sectoral_batch(
            simd,
            self.sine,
            self.values,
            self.scales,
            self.exponent,
            self.lower_limit,
            self.factor,
        )
    }
}

struct NormalizeDispatch<'a> {
    values: &'a mut [f64],
    scales: &'a mut [i64],
    maximum: f64,
}

impl WithSimd for NormalizeDispatch<'_> {
    type Output = bool;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        normalize_batch(simd, self.values, self.scales, self.maximum)
    }
}

struct OrderedF32VectorReplay<'a> {
    coefficient_a: &'a [f32],
    coefficient_b: &'a [f32],
    coefficient_even: &'a [f32],
    coefficient_odd: &'a [f32],
    cosine_squared: &'a [f32],
    cosine_squared_low: &'a [f32],
    cosine: &'a [f32],
    cosine_low: &'a [f32],
    initial: &'a [f32],
    initial_low: &'a [f32],
    coefficient_a_split_high: &'a [f32],
    coefficient_b_split_high: &'a [f32],
    coefficient_a_low: &'a [f32],
    coefficient_b_low: &'a [f32],
    output: &'a mut [f32],
    compensated: bool,
    residual: bool,
    setup_split: bool,
}

#[inline(always)]
fn residual_two_sum_f32s<S: Simd>(simd: S, first: S::f32s, second: S::f32s) -> (S::f32s, S::f32s) {
    let sum = simd.add_f32s(first, second);
    let displaced = simd.sub_f32s(sum, first);
    let error = simd.add_f32s(
        simd.sub_f32s(first, simd.sub_f32s(sum, displaced)),
        simd.sub_f32s(second, displaced),
    );
    (sum, error)
}

impl WithSimd for OrderedF32VectorReplay<'_> {
    type Output = bool;

    #[inline(never)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let (csq_vectors, csq_tail) = S::as_simd_f32s(self.cosine_squared);
        let (cosine_vectors, cosine_tail) = S::as_simd_f32s(self.cosine);
        let (initial_vectors, initial_tail) = S::as_simd_f32s(self.initial);
        if csq_vectors.len() != 1
            || !csq_tail.is_empty()
            || cosine_vectors.len() != 1
            || !cosine_tail.is_empty()
            || initial_vectors.len() != 1
            || !initial_tail.is_empty()
            || self.coefficient_a.len() != self.coefficient_b.len()
            || self.coefficient_a.len() != self.coefficient_even.len()
            || self.coefficient_a.len() != self.coefficient_odd.len()
            || self.output.len() != S::F32_LANES
        {
            return false;
        }

        let cosine_squared = csq_vectors[0];
        let cosine = cosine_vectors[0];
        let initial = initial_vectors[0];
        let zero = simd.splat_f32s(0.0);
        if self.compensated {
            let mut previous = super::DoubleSingleF32Vector {
                high: zero,
                low: zero,
            };
            let mut current = super::DoubleSingleF32Vector {
                high: initial,
                low: zero,
            };
            let mut sum_even = previous;
            let mut sum_odd = previous;
            for pair in 0..self.coefficient_a.len() {
                let even = simd.splat_f32s(self.coefficient_even[pair]);
                let odd = simd.splat_f32s(self.coefficient_odd[pair]);
                sum_even = super::ds_vector_add(
                    simd,
                    sum_even,
                    super::ds_vector_mul(
                        simd,
                        current,
                        super::DoubleSingleF32Vector {
                            high: even,
                            low: zero,
                        },
                    ),
                );
                sum_odd = super::ds_vector_add(
                    simd,
                    sum_odd,
                    super::ds_vector_mul(
                        simd,
                        current,
                        super::DoubleSingleF32Vector {
                            high: odd,
                            low: zero,
                        },
                    ),
                );

                let a = super::DoubleSingleF32Vector {
                    high: simd.splat_f32s(self.coefficient_a[pair]),
                    low: zero,
                };
                let b = super::DoubleSingleF32Vector {
                    high: simd.splat_f32s(self.coefficient_b[pair]),
                    low: zero,
                };
                let csq = super::DoubleSingleF32Vector {
                    high: cosine_squared,
                    low: zero,
                };
                let leading = super::ds_vector_add(simd, super::ds_vector_mul(simd, a, csq), b);
                let next = super::ds_vector_add(
                    simd,
                    super::ds_vector_mul(simd, leading, current),
                    previous,
                );
                previous = current;
                current = next;
            }
            let cosine = super::DoubleSingleF32Vector {
                high: cosine,
                low: zero,
            };
            let output =
                super::ds_vector_add(simd, sum_even, super::ds_vector_mul(simd, cosine, sum_odd));
            simd.partial_store_f32s(self.output, simd.add_f32s(output.high, output.low));
        } else if self.residual {
            if self.setup_split {
                let (csq_low_vectors, csq_low_tail) = S::as_simd_f32s(self.cosine_squared_low);
                let (cosine_low_vectors, cosine_low_tail) = S::as_simd_f32s(self.cosine_low);
                let (initial_low_vectors, initial_low_tail) = S::as_simd_f32s(self.initial_low);
                if csq_low_vectors.len() != 1
                    || !csq_low_tail.is_empty()
                    || cosine_low_vectors.len() != 1
                    || !cosine_low_tail.is_empty()
                    || initial_low_vectors.len() != 1
                    || !initial_low_tail.is_empty()
                    || self.coefficient_a_split_high.len() != self.coefficient_a.len()
                    || self.coefficient_b_split_high.len() != self.coefficient_b.len()
                    || self.coefficient_a_low.len() != self.coefficient_a.len()
                    || self.coefficient_b_low.len() != self.coefficient_b.len()
                {
                    return false;
                }
                let csq_low = csq_low_vectors[0];
                let cosine_low = cosine_low_vectors[0];
                let mut previous_high = zero;
                let mut previous_low = zero;
                let mut current_high = initial;
                let mut current_low = initial_low_vectors[0];
                let mut sum_even = zero;
                let mut sum_odd = zero;
                for pair in 0..self.coefficient_a.len() {
                    let even = simd.splat_f32s(self.coefficient_even[pair]);
                    let odd = simd.splat_f32s(self.coefficient_odd[pair]);
                    sum_even = simd.mul_add_f32s(current_high, even, sum_even);
                    sum_even = simd.mul_add_f32s(current_low, even, sum_even);
                    sum_odd = simd.mul_add_f32s(current_high, odd, sum_odd);
                    sum_odd = simd.mul_add_f32s(current_low, odd, sum_odd);

                    let a_high = simd.splat_f32s(self.coefficient_a_split_high[pair]);
                    let a_low = simd.splat_f32s(self.coefficient_a_low[pair]);
                    let b_high = simd.splat_f32s(self.coefficient_b_split_high[pair]);
                    let b_low = simd.splat_f32s(self.coefficient_b_low[pair]);
                    let product = simd.mul_f32s(a_high, cosine_squared);
                    let product_error =
                        simd.mul_add_f32s(a_high, cosine_squared, simd.neg_f32s(product));
                    let (leading_high, leading_sum_error) =
                        residual_two_sum_f32s(simd, product, b_high);
                    let cross = simd.mul_add_f32s(
                        a_high,
                        csq_low,
                        simd.mul_add_f32s(a_low, cosine_squared, simd.mul_f32s(a_low, csq_low)),
                    );
                    let leading_low = simd.add_f32s(
                        simd.add_f32s(product_error, leading_sum_error),
                        simd.add_f32s(cross, b_low),
                    );

                    let state_product = simd.mul_f32s(leading_high, current_high);
                    let state_product_error =
                        simd.mul_add_f32s(leading_high, current_high, simd.neg_f32s(state_product));
                    let (next_high, state_sum_error) =
                        residual_two_sum_f32s(simd, state_product, previous_high);
                    let next_low = simd.mul_add_f32s(
                        leading_high,
                        current_low,
                        simd.add_f32s(state_product_error, state_sum_error),
                    );
                    let next_low = simd.mul_add_f32s(leading_low, current_high, next_low);
                    let next_low = simd.add_f32s(next_low, previous_low);
                    previous_high = current_high;
                    previous_low = current_low;
                    current_high = next_high;
                    current_low = next_low;
                }
                let output = simd.mul_add_f32s(cosine, sum_odd, sum_even);
                let output = simd.mul_add_f32s(cosine_low, sum_odd, output);
                simd.partial_store_f32s(self.output, output);
                return self.output.iter().all(|value| value.is_finite());
            }
            let mut previous_high = zero;
            let mut previous_low = zero;
            let mut current_high = initial;
            let mut current_low = zero;
            let mut sum_even = zero;
            let mut sum_odd = zero;
            for pair in 0..self.coefficient_a.len() {
                let even = simd.splat_f32s(self.coefficient_even[pair]);
                let odd = simd.splat_f32s(self.coefficient_odd[pair]);
                sum_even = simd.mul_add_f32s(current_high, even, sum_even);
                sum_even = simd.mul_add_f32s(current_low, even, sum_even);
                sum_odd = simd.mul_add_f32s(current_high, odd, sum_odd);
                sum_odd = simd.mul_add_f32s(current_low, odd, sum_odd);

                let a = simd.splat_f32s(self.coefficient_a[pair]);
                let b = simd.splat_f32s(self.coefficient_b[pair]);
                let product = simd.mul_f32s(a, cosine_squared);
                let product_error = simd.mul_add_f32s(a, cosine_squared, simd.neg_f32s(product));
                let (leading_high, leading_sum_error) = residual_two_sum_f32s(simd, product, b);
                let leading_low = simd.add_f32s(product_error, leading_sum_error);

                let state_product = simd.mul_f32s(leading_high, current_high);
                let state_product_error =
                    simd.mul_add_f32s(leading_high, current_high, simd.neg_f32s(state_product));
                let (next_high, state_sum_error) =
                    residual_two_sum_f32s(simd, state_product, previous_high);
                let next_low = simd.mul_add_f32s(
                    leading_high,
                    current_low,
                    simd.add_f32s(state_product_error, state_sum_error),
                );
                let next_low = simd.mul_add_f32s(leading_low, current_high, next_low);
                let next_low = simd.add_f32s(next_low, previous_low);
                previous_high = current_high;
                previous_low = current_low;
                current_high = next_high;
                current_low = next_low;
            }
            let output = simd.mul_add_f32s(cosine, sum_odd, sum_even);
            simd.partial_store_f32s(self.output, output);
        } else {
            let mut previous = zero;
            let mut current = initial;
            let mut sum_even = zero;
            let mut sum_odd = zero;
            for pair in 0..self.coefficient_a.len() {
                let even = simd.splat_f32s(self.coefficient_even[pair]);
                let odd = simd.splat_f32s(self.coefficient_odd[pair]);
                sum_even = simd.mul_add_f32s(current, even, sum_even);
                sum_odd = simd.mul_add_f32s(current, odd, sum_odd);
                let a = simd.splat_f32s(self.coefficient_a[pair]);
                let b = simd.splat_f32s(self.coefficient_b[pair]);
                let leading = simd.mul_add_f32s(a, cosine_squared, b);
                let next = simd.mul_add_f32s(leading, current, previous);
                previous = current;
                current = next;
            }
            simd.partial_store_f32s(self.output, simd.mul_add_f32s(cosine, sum_odd, sum_even));
        }
        self.output.iter().all(|value| value.is_finite())
    }
}

struct RescaleDispatch<'a> {
    previous: &'a mut [f64],
    current: &'a mut [f64],
    scales: &'a mut [i64],
}

impl WithSimd for RescaleDispatch<'_> {
    type Output = Option<bool>;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        rescale_batch(simd, self.previous, self.current, self.scales)
    }
}

struct PrefixStepDispatch<'a> {
    previous: &'a mut [f64],
    current: &'a mut [f64],
    scales: &'a mut [i64],
}

impl WithSimd for PrefixStepDispatch<'_> {
    type Output = Option<bool>;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        rescale_prefix_step(simd, self.previous, self.current, self.scales)
    }
}

struct CorrectionDispatch<'a> {
    scales: &'a [i64],
    correction: &'a mut [f64],
}

impl WithSimd for CorrectionDispatch<'_> {
    type Output = bool;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        set_correction_factors(simd, self.scales, self.correction);
        all_scale_nonnegative(simd, self.scales)
    }
}

#[test]
fn reflected_geometry_preflight_checks_symmetry_and_order_limits() {
    let sin_theta = [0.2, 0.8, 0.8, 0.2];
    let cos_theta = [0.98, 0.6, -0.6, -0.98];
    let ring_order_limits = [1, 3, 3, 1];
    assert!(validate_reflected_geometry(&sin_theta, &cos_theta, &ring_order_limits,).is_some());

    let mut asymmetric_sin = sin_theta;
    asymmetric_sin[3] += 1.0e-12;
    assert!(
        validate_reflected_geometry(&asymmetric_sin, &cos_theta, &ring_order_limits,).is_none()
    );

    let nonmonotone_limits = [3, 1, 1, 3];
    assert!(validate_reflected_geometry(&sin_theta, &cos_theta, &nonmonotone_limits,).is_none());
}

#[cfg(feature = "parallel")]
#[test]
fn sparse_spectrum_tails_follow_northern_reflected_order_limits() {
    let sin_theta = [0.2, 0.8, 0.8, 0.2];
    let cos_theta = [0.98, 0.6, -0.6, -0.98];
    let limits = [1, 3, 99, 99];
    let geometry = validate_reflected_geometry(&sin_theta, &cos_theta, &limits).unwrap();
    let mut spectra = vec![Complex64::new(3.0, -2.0); 4 * 5];
    assert!(initialize_inactive_spectrum_tails(
        &mut spectra,
        5,
        3,
        &geometry,
        Complex64::new(0.0, 0.0),
    ));

    for ring in [0, 3] {
        let row = &spectra[ring * 5..(ring + 1) * 5];
        assert_eq!(row[0], Complex64::new(3.0, -2.0));
        assert_eq!(row[1], Complex64::new(3.0, -2.0));
        assert!(
            row[2..]
                .iter()
                .all(|&value| value == Complex64::new(0.0, 0.0))
        );
    }
    for ring in [1, 2] {
        let row = &spectra[ring * 5..(ring + 1) * 5];
        assert!(
            row[..4]
                .iter()
                .all(|&value| value == Complex64::new(3.0, -2.0))
        );
        assert_eq!(row[4], Complex64::new(0.0, 0.0));
    }
}

#[test]
fn sectoral_initializer_matches_checked_lane_route_across_vector_tail() {
    let exponent = 17;
    let lower_limit = power_lower_limit(exponent);
    let mut sine = (0..19)
        .map(|index| 0.2 + index as f64 * 0.03)
        .collect::<Vec<_>>();
    sine[0] = lower_limit;
    sine[1] = lower_limit * 0.5;
    sine[2] = 1.0;
    sine[3] = 0.0;
    sine[4] = f64::from_bits(1);
    sine[18] = lower_limit * 0.25;
    let factor = -0.75;

    for arch in [Arch::Scalar, Arch::new()] {
        let mut values = vec![f64::NAN; sine.len()];
        let mut scales = vec![i64::MIN; sine.len()];
        assert!(arch.dispatch(InitializeDispatch {
            sine: &sine,
            values: &mut values,
            scales: &mut scales,
            exponent,
            lower_limit,
            factor,
        }));

        for lane in 0..sine.len() {
            let (mut expected, mut expected_scale) =
                mypow(sine[lane], exponent, lower_limit).unwrap();
            expected *= factor;
            assert!(normalize_lane(&mut expected, &mut expected_scale, FTOL));
            assert_eq!(values[lane].to_bits(), expected.to_bits(), "lane {lane}");
            assert_eq!(scales[lane], expected_scale, "lane {lane}");
        }
    }
}

#[test]
fn sectoral_initializer_declines_nonfinite_lanes() {
    let sine = [0.5, f64::NAN, 0.75];
    let mut values = [0.0; 3];
    let mut scales = [0; 3];
    assert!(!Arch::new().dispatch(InitializeDispatch {
        sine: &sine,
        values: &mut values,
        scales: &mut scales,
        exponent: 3,
        lower_limit: power_lower_limit(3),
        factor: 1.0,
    }));
}

#[test]
fn sectoral_initializer_declines_scale_exponent_overflow() {
    let mut sine = [0.5; 19];
    sine[1] = f64::from_bits(1);
    let mut values = [0.0; 19];
    let mut scales = [0; 19];
    assert!(!Arch::new().dispatch(InitializeDispatch {
        sine: &sine,
        values: &mut values,
        scales: &mut scales,
        exponent: usize::MAX,
        lower_limit: power_lower_limit(usize::MAX),
        factor: 1.0,
    }));
}

#[test]
fn zero_order_initialization_keeps_exact_unit_power() {
    let sine = [0.0, 0.25, 0.75, 1.0, 0.3];
    let mut values = [f64::NAN; 5];
    let mut scales = [i64::MIN; 5];
    let factor = FTOL * 0.5;
    assert!(Arch::new().dispatch(InitializeDispatch {
        sine: &sine,
        values: &mut values,
        scales: &mut scales,
        exponent: 0,
        lower_limit: power_lower_limit(0),
        factor,
    }));
    assert_eq!(values, [factor; 5]);
    assert_eq!(scales, [0; 5]);
    assert_eq!(scalar_integer_power(0.0, 0), 1.0);
}

#[test]
fn vector_normalization_matches_checked_lane_route_across_vector_tail() {
    let mut values = (0..19)
        .map(|index| 0.05 + index as f64 * 0.2)
        .collect::<Vec<_>>();
    values[0] = 2.0;
    values[1] = f64::MIN_POSITIVE;
    values[2] = f64::from_bits(1);
    values[3] = 0.0;
    values[4] = -2.0;
    let mut expected = values.clone();
    let scales = vec![0_i64; values.len()];
    let mut expected_scales = scales.clone();
    for lane in 0..expected.len() {
        assert!(normalize_lane(
            &mut expected[lane],
            &mut expected_scales[lane],
            FTOL,
        ));
    }

    for arch in [Arch::Scalar, Arch::new()] {
        let mut actual = values.clone();
        let mut actual_scales = scales.clone();
        assert!(arch.dispatch(NormalizeDispatch {
            values: &mut actual,
            scales: &mut actual_scales,
            maximum: FTOL,
        }));
        assert_eq!(actual, expected);
        assert_eq!(actual_scales, expected_scales);
    }
}

#[test]
fn vector_rescaling_matches_lane_epoch_and_tail_rules() {
    let previous = (0..19).map(|lane| 0.25 + lane as f64).collect::<Vec<_>>();
    let current = (0..19)
        .map(|lane| match lane % 5 {
            0 => FTOL * 0.5,
            1 => FTOL,
            2 => FTOL * 2.0,
            3 => -FTOL * 3.0,
            _ => 0.0,
        })
        .collect::<Vec<_>>();
    let scales = (0..19)
        .map(|lane| match lane % 5 {
            0 => -2,
            1 => -1,
            2 => 0,
            3 => 1,
            _ => i64::MAX,
        })
        .collect::<Vec<_>>();
    let mut expected_previous = previous.clone();
    let mut expected_current = current.clone();
    let mut expected_scales = scales.clone();
    let mut expected_any = false;
    for lane in 0..current.len() {
        if current[lane].abs() > FTOL {
            expected_previous[lane] *= SCALE_DOWN;
            expected_current[lane] *= SCALE_DOWN;
            expected_scales[lane] = expected_scales[lane].checked_add(1).unwrap();
            expected_any = true;
        }
    }

    for arch in [Arch::Scalar, Arch::new()] {
        let mut actual_previous = previous.clone();
        let mut actual_current = current.clone();
        let mut actual_scales = scales.clone();
        let actual_any = arch
            .dispatch(RescaleDispatch {
                previous: &mut actual_previous,
                current: &mut actual_current,
                scales: &mut actual_scales,
            })
            .unwrap();
        assert_eq!(actual_any, expected_any);
        assert_eq!(actual_previous, expected_previous);
        assert_eq!(actual_current, expected_current);
        assert_eq!(actual_scales, expected_scales);
    }
}

#[test]
fn insignificant_prefix_step_skips_scale_reduction_without_rescaling() {
    let previous = vec![0.25; 19];
    let current = vec![FTOL; 19];
    let scales = vec![i64::MAX; 19];

    for arch in [Arch::Scalar, Arch::new()] {
        let mut actual_previous = previous.clone();
        let mut actual_current = current.clone();
        let mut actual_scales = scales.clone();
        let below_limit = arch
            .dispatch(PrefixStepDispatch {
                previous: &mut actual_previous,
                current: &mut actual_current,
                scales: &mut actual_scales,
            })
            .unwrap();

        assert!(below_limit);
        assert_eq!(actual_previous, previous);
        assert_eq!(actual_current, current);
        assert_eq!(actual_scales, scales);
    }
}

#[test]
fn insignificant_prefix_step_checks_scale_epoch_after_rescaling() {
    let previous = vec![0.25; 19];
    let mut current = vec![FTOL; 19];
    current[0] = FTOL * 2.0;
    let mut scales = vec![-1; 19];

    for arch in [Arch::Scalar, Arch::new()] {
        let mut actual_previous = previous.clone();
        let mut actual_current = current.clone();
        let mut actual_scales = scales.clone();
        let below_limit = arch
            .dispatch(PrefixStepDispatch {
                previous: &mut actual_previous,
                current: &mut actual_current,
                scales: &mut actual_scales,
            })
            .unwrap();

        assert!(below_limit);
        assert_eq!(actual_scales[0], 0);

        scales[0] = 0;
        let mut actual_previous = previous.clone();
        let mut actual_current = current.clone();
        let mut actual_scales = scales.clone();
        let below_limit = arch
            .dispatch(PrefixStepDispatch {
                previous: &mut actual_previous,
                current: &mut actual_current,
                scales: &mut actual_scales,
            })
            .unwrap();
        assert!(!below_limit);
        assert_eq!(actual_scales[0], 1);
        scales[0] = -1;
    }
}

#[test]
fn vector_correction_factors_preserve_scale_epoch_semantics() {
    let scales = (0..19)
        .map(|lane| match lane % 4 {
            0 => -2,
            1 => 0,
            2 => 1,
            _ => 3,
        })
        .collect::<Vec<_>>();
    let mut correction = vec![f64::NAN; scales.len()];
    let all_nonnegative = Arch::new().dispatch(CorrectionDispatch {
        scales: &scales,
        correction: &mut correction,
    });
    assert!(!all_nonnegative);
    for (scale, actual) in scales.iter().zip(correction) {
        let expected = if *scale < 0 {
            0.0
        } else if *scale < 1 {
            1.0
        } else {
            SCALE_UP
        };
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

#[test]
fn normalization_and_rescaling_reject_exponent_overflow() {
    let mut values = vec![2.0; 19];
    let mut scales = vec![0_i64; values.len()];
    scales[5] = i64::MAX;
    assert!(!Arch::new().dispatch(NormalizeDispatch {
        values: &mut values,
        scales: &mut scales,
        maximum: FTOL,
    }));

    let mut previous = vec![0.5; 19];
    let mut current = vec![FTOL * 0.5; 19];
    let mut scales = vec![0_i64; current.len()];
    current[7] = FTOL * 2.0;
    scales[7] = i64::MAX;
    assert!(
        Arch::new()
            .dispatch(RescaleDispatch {
                previous: &mut previous,
                current: &mut current,
                scales: &mut scales,
            })
            .is_none()
    );
}

#[test]
fn native_f32_ordered_sectoral_keeps_representable_scaled_contributions() {
    use num_complex::Complex32;

    let lmax = 40;
    let mmax = 40;
    let ntheta = 8;
    let spectrum_len = mmax + 1;
    let sin_theta = [0.05_f32, 0.25, 0.5, 0.9, 0.9, 0.5, 0.25, 0.05];
    let north_cosine = sin_theta[..4]
        .iter()
        .map(|&sine| (1.0 - sine * sine).sqrt())
        .collect::<Vec<_>>();
    let cos_theta = [
        north_cosine[0],
        north_cosine[1],
        north_cosine[2],
        north_cosine[3],
        -north_cosine[3],
        -north_cosine[2],
        -north_cosine[1],
        -north_cosine[0],
    ];
    let ring_order_limits = vec![mmax; ntheta];
    let sectoral_norms = crate::legendre::numeric::sectoral_normalizations::<f32>(mmax).unwrap();
    let alm_len = (lmax + 1) * (lmax + 2) / 2;
    let mut alm = vec![Complex32::new(0.0, 0.0); alm_len];
    let packed_index = super::packed_order_offset(mmax, lmax).unwrap();
    let coefficient = 1.0e30_f32;
    alm[packed_index] = Complex32::new(coefficient, 0.0);
    let m1_offset = super::packed_order_offset(1, lmax).unwrap();
    alm[m1_offset] = Complex32::new(1.25, -0.4);
    alm[m1_offset + 2] = Complex32::new(-0.3, 0.7);
    let mut spectra = vec![Complex32::new(0.0, 0.0); ntheta * spectrum_len];

    let mut epsilon = vec![0.0_f32; lmax + 4];
    let mut alpha = vec![0.0_f32; lmax + 4];
    let mut coefficient_a = vec![0.0_f32; lmax + 4];
    let mut coefficient_b = vec![0.0_f32; lmax + 4];
    let mut coefficient_even = vec![num_complex::Complex::new(0.0_f32, 0.0); lmax + 4];
    let mut coefficient_odd = vec![num_complex::Complex::new(0.0_f32, 0.0); lmax + 4];
    let mut packed_sin = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut packed_csq = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut previous = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut current = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut scale = vec![0_i64; super::SCALAR_ORDER_BATCH_SIZE];
    let mut corfac = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_even_re = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_even_im = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_odd_re = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_odd_im = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut scratch = super::ScalarOrderedScratch {
        epsilon: &mut epsilon,
        alpha: &mut alpha,
        coefficient_a: &mut coefficient_a,
        coefficient_b: &mut coefficient_b,
        coefficient_even: &mut coefficient_even,
        coefficient_odd: &mut coefficient_odd,
        packed_sin: &mut packed_sin,
        packed_csq: &mut packed_csq,
        previous: &mut previous,
        current: &mut current,
        scale: &mut scale,
        corfac: &mut corfac,
        sum_even_re: &mut sum_even_re,
        sum_even_im: &mut sum_even_im,
        sum_odd_re: &mut sum_odd_re,
        sum_odd_im: &mut sum_odd_im,
    };

    assert!(super::synthesize_dispatched::<f32>(
        Arch::new(),
        &alm,
        lmax,
        mmax,
        None,
        128,
        spectrum_len,
        0.0,
        &sectoral_norms,
        &sin_theta,
        &cos_theta,
        &ring_order_limits,
        &mut spectra,
        &mut scratch,
    ));

    let reference_norms = crate::legendre::numeric::sectoral_normalizations::<f64>(mmax).unwrap();
    for (ring, &sine) in sin_theta.iter().enumerate() {
        let expected =
            coefficient as f64 * reference_norms[mmax] * f64::from(sine).powi(mmax as i32);
        let actual = spectra[ring * spectrum_len + mmax].re;
        let relative_error = (f64::from(actual) - expected).abs() / expected.abs();
        assert!(
            relative_error < 2.0e-5,
            "ring {ring}: got {actual:e}, expected {expected:e}, relative error {relative_error:e}"
        );
        assert_eq!(spectra[ring * spectrum_len + mmax].im, 0.0);
    }

    for ring in 0..ntheta {
        let sine = f64::from(sin_theta[ring]);
        let cosine = f64::from(cos_theta[ring]);
        let mut previous = 0.0_f64;
        let mut current = -reference_norms[1] * sine;
        let mut expected = num_complex::Complex::new(0.0_f64, 0.0);
        for degree in 1..=lmax {
            let basis = if degree == 1 {
                current
            } else {
                let next = (cosine * current
                    - crate::legendre::numeric::epsilon::<f64>(degree - 1, 1) * previous)
                    / crate::legendre::numeric::epsilon::<f64>(degree, 1);
                previous = current;
                current = next;
                next
            };
            if degree == 1 {
                expected += num_complex::Complex::new(1.25_f64, -0.4) * basis;
            } else if degree == 3 {
                expected += num_complex::Complex::new(-0.3_f64, 0.7) * basis;
            }
        }
        let actual = spectra[ring * spectrum_len + 1];
        let error = (num_complex::Complex::new(f64::from(actual.re), f64::from(actual.im))
            - expected)
            .norm();
        assert!(
            error < 2.0e-6,
            "ring {ring}: f32 order-one error {error:e}, actual {actual:?}, expected {expected:?}"
        );
    }
}

#[test]
fn native_f32_ordered_high_degree_zonal_probe_reports_near_pole_error() {
    use num_complex::{Complex, Complex32};

    let lmax = 720;
    let ntheta = 2;
    let sine = 0.003_333_085_2_f32;
    let cosine = 0.999_994_46_f32;
    let sin_theta = [sine, sine];
    let cos_theta = [cosine, -cosine];
    let ring_order_limits = [0, 0];
    let sectoral_norms = crate::legendre::numeric::sectoral_normalizations::<f32>(0).unwrap();
    let mut alm = vec![Complex32::new(0.0, 0.0); (lmax + 1) * (lmax + 2) / 2];
    alm[lmax] = Complex32::new(1.0, 0.0);

    let run = |arch| {
        let mut spectra = vec![Complex32::new(0.0, 0.0); ntheta * 2];
        let mut epsilon = vec![0.0_f32; lmax + 4];
        let mut alpha = vec![0.0_f32; lmax + 4];
        let mut coefficient_a = vec![0.0_f32; lmax + 4];
        let mut coefficient_b = vec![0.0_f32; lmax + 4];
        let mut coefficient_even = vec![Complex::new(0.0_f32, 0.0); lmax + 4];
        let mut coefficient_odd = vec![Complex::new(0.0_f32, 0.0); lmax + 4];
        let mut packed_sin = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
        let mut packed_csq = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
        let mut previous = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
        let mut current = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
        let mut scale = vec![0_i64; super::SCALAR_ORDER_BATCH_SIZE];
        let mut corfac = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
        let mut sum_even_re = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
        let mut sum_even_im = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
        let mut sum_odd_re = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
        let mut sum_odd_im = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
        let mut scratch = super::ScalarOrderedScratch {
            epsilon: &mut epsilon,
            alpha: &mut alpha,
            coefficient_a: &mut coefficient_a,
            coefficient_b: &mut coefficient_b,
            coefficient_even: &mut coefficient_even,
            coefficient_odd: &mut coefficient_odd,
            packed_sin: &mut packed_sin,
            packed_csq: &mut packed_csq,
            previous: &mut previous,
            current: &mut current,
            scale: &mut scale,
            corfac: &mut corfac,
            sum_even_re: &mut sum_even_re,
            sum_even_im: &mut sum_even_im,
            sum_odd_re: &mut sum_odd_re,
            sum_odd_im: &mut sum_odd_im,
        };
        assert!(super::synthesize_dispatched::<f32>(
            arch,
            &alm,
            lmax,
            0,
            None,
            2,
            2,
            0.0,
            &sectoral_norms,
            &sin_theta,
            &cos_theta,
            &ring_order_limits,
            &mut spectra,
            &mut scratch,
        ));
        spectra
    };

    let reference_norm = crate::legendre::numeric::sectoral_normalizations::<f64>(0).unwrap()[0];
    let mut previous = 0.0_f64;
    let mut current = reference_norm;
    for degree in 1..=lmax {
        let next = (f64::from(cosine) * current
            - crate::legendre::numeric::epsilon::<f64>(degree - 1, 0) * previous)
            / crate::legendre::numeric::epsilon::<f64>(degree, 0);
        previous = current;
        current = next;
    }
    let expected = current;
    let scalar = run(Arch::Scalar)[0].re;
    let vector = run(Arch::new())[0].re;
    let pairs = lmax / 2 + 1;
    let mut epsilon32 = vec![0.0_f32; lmax + 4];
    let mut alpha32 = vec![0.0_f32; lmax + 4];
    let mut a32 = vec![0.0_f32; lmax + 4];
    let mut b32 = vec![0.0_f32; lmax + 4];
    crate::legendre::numeric::fill_epsilon(0, lmax, &mut epsilon32);
    assert!(crate::legendre::scalar_paired::fill_factors_typed(
        0,
        lmax,
        &epsilon32,
        &mut alpha32,
        &mut a32,
        &mut b32,
        f32::MAX,
    ));
    let csq32 = (1.0_f32 - sine) * (1.0_f32 + sine);
    let mut alm32 = vec![Complex32::new(0.0, 0.0); lmax + 1];
    alm32[lmax] = Complex32::new(1.0, 0.0);
    let mut even32 = vec![Complex32::new(0.0, 0.0); lmax + 4];
    let mut odd32 = vec![Complex32::new(0.0, 0.0); lmax + 4];
    assert!(
        crate::legendre::scalar_paired::prepare_synthesis_coefficients(
            0,
            lmax,
            &epsilon32,
            &alpha32,
            &alm32,
            &mut even32,
            &mut odd32,
        )
    );
    let mut recurrence32_prev = 0.0_f32;
    let mut recurrence32 = crate::legendre::numeric::sectoral_normalizations::<f32>(0).unwrap()[0];
    let mut sum32 = 0.0_f32;
    let mut pair359_term32 = 0.0_f32;
    let mut pair360_term32 = 0.0_f32;
    for pair in 0..pairs {
        let term = recurrence32 * even32[pair].re;
        sum32 += term;
        if pair == 359 {
            pair359_term32 = term;
        }
        if pair == 360 {
            pair360_term32 = term;
        }
        let leading32 = a32[pair].mul_add(csq32, b32[pair]);
        let next = leading32.mul_add(recurrence32, recurrence32_prev);
        recurrence32_prev = recurrence32;
        recurrence32 = next;
    }
    let mut epsilon64 = vec![0.0_f64; lmax + 4];
    let mut alpha64 = vec![0.0_f64; lmax + 4];
    let mut a64 = vec![0.0_f64; lmax + 4];
    let mut b64 = vec![0.0_f64; lmax + 4];
    crate::legendre::numeric::fill_epsilon(0, lmax, &mut epsilon64);
    assert!(crate::legendre::scalar_paired::fill_factors_typed(
        0,
        lmax,
        &epsilon64,
        &mut alpha64,
        &mut a64,
        &mut b64,
        f64::MAX,
    ));
    let mut alm64 = vec![Complex::new(0.0_f64, 0.0); lmax + 1];
    alm64[lmax] = Complex::new(1.0, 0.0);
    let mut even64 = vec![Complex::new(0.0_f64, 0.0); lmax + 4];
    let mut odd64 = vec![Complex::new(0.0_f64, 0.0); lmax + 4];
    assert!(
        crate::legendre::scalar_paired::prepare_synthesis_coefficients(
            0,
            lmax,
            &epsilon64,
            &alpha64,
            &alm64,
            &mut even64,
            &mut odd64,
        )
    );
    let mut prev64_cast_coeff = 0.0_f32;
    let mut curr64_cast_coeff =
        crate::legendre::numeric::sectoral_normalizations::<f32>(0).unwrap()[0];
    let mut sum64_prepared32 = 0.0_f32;
    let mut prev64 = 0.0_f64;
    let mut curr64 = crate::legendre::numeric::sectoral_normalizations::<f64>(0).unwrap()[0];
    let mut sum64 = 0.0_f64;
    let csq64 = (1.0 - f64::from(sine)) * (1.0 + f64::from(sine));
    for pair in 0..pairs {
        sum64 = curr64.mul_add(even64[pair].re, sum64);
        let leading64 = a64[pair].mul_add(csq64, b64[pair]);
        let next64 = leading64.mul_add(curr64, prev64);
        prev64 = curr64;
        curr64 = next64;

        let term = curr64_cast_coeff * (even64[pair].re as f32);
        sum64_prepared32 += term;
        let leading64_coeff32 = (a64[pair] as f32).mul_add(csq32, b64[pair] as f32);
        let next = leading64_coeff32.mul_add(curr64_cast_coeff, prev64_cast_coeff);
        prev64_cast_coeff = curr64_cast_coeff;
        curr64_cast_coeff = next;
    }
    let output64_coeff_cast = curr64_cast_coeff * ((epsilon64[lmax + 1] * alpha64[pairs]) as f32);
    let cosine_from_sine = csq64.sqrt();
    let mut direct_previous = 0.0_f64;
    let mut direct_current =
        crate::legendre::numeric::sectoral_normalizations::<f64>(0).unwrap()[0];
    for degree in 1..=lmax {
        let next = (cosine_from_sine * direct_current
            - crate::legendre::numeric::epsilon::<f64>(degree - 1, 0) * direct_previous)
            / crate::legendre::numeric::epsilon::<f64>(degree, 0);
        direct_previous = direct_current;
        direct_current = next;
    }
    eprintln!(
        "zonal probe: sine={sine:.9}, cosine={cosine:.9}, expected={expected:.9}, expected_sine={direct_current:.9}, scalar={scalar:.9}, vector={vector:.9}, manual_sum32={sum32:.9}, pair359term32={pair359_term32:.9}, pair360term32={pair360_term32:.9}, manual_sum64prep32={sum64_prepared32:.9}, manual_sum64={sum64:.9}, pair64coef32={curr64_cast_coeff:.9}, output64coef32={output64_coeff_cast:.9}, paircount={pairs}, a360_32={:.9e}, a360_64={:.9e}, b360_32={:.9e}, b360_64={:.9e}, alpha360_32={:.9e}, alpha360_64={:.9e}",
        a32[360], a64[360], b32[360], b64[360], alpha32[pairs], alpha64[pairs]
    );
}

#[test]
#[ignore = "test-only recurrence versus accumulation decomposition for high-degree low orders"]
fn native_f32_ordered_recurrence_vs_accumulation_error_probe() {
    use num_complex::{Complex, Complex32};

    struct LaneCount;
    impl WithSimd for LaneCount {
        type Output = usize;

        fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
            <f32 as crate::legendre::OrderedSimdPrecision>::lane_count::<S>()
        }
    }

    struct Initialize<'a> {
        sine: &'a [f32],
        values: &'a mut [f32],
        scales: &'a mut [i64],
        exponent: usize,
        lower_limit: f32,
        factor: f32,
    }
    impl WithSimd for Initialize<'_> {
        type Output = bool;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            super::kernel::initialize_sectoral_batch::<f32, S>(
                simd,
                self.sine,
                self.values,
                self.scales,
                self.exponent,
                self.lower_limit,
                self.factor,
            )
        }
    }

    let lmax = 720;
    let mmax = 16;
    let grid = crate::grid::GridSpec::new(
        crate::grid::GridKind::GaussLegendre,
        lmax + 1,
        2 * mmax + 2,
        0.23,
    )
    .unwrap();
    let target_rings = [0, 1, 2, 3, 4, 5, 360, 715, 716, 717, 718, 719, 720];
    let arch = Arch::new();
    let lanes = arch.dispatch(LaneCount);
    assert!((1..=super::SCALAR_ORDER_BATCH_SIZE).contains(&lanes));
    let sectoral_norms = crate::legendre::numeric::sectoral_normalizations::<f32>(mmax).unwrap();
    let sectoral_norms64 = crate::legendre::numeric::sectoral_normalizations::<f64>(mmax).unwrap();
    let replay_f64 = |initial: f64, csq: f64, a: &[f64], b: &[f64], even: &[f64], odd: &[f64]| {
        let mut previous = 0.0_f64;
        let mut current = initial;
        let mut sum_even = 0.0_f64;
        let mut sum_odd = 0.0_f64;
        for pair in 0..a.len() {
            sum_even = current.mul_add(even[pair], sum_even);
            sum_odd = current.mul_add(odd[pair], sum_odd);
            let leading = a[pair].mul_add(csq, b[pair]);
            let next = leading.mul_add(current, previous);
            previous = current;
            current = next;
        }
        (sum_even, sum_odd)
    };
    let split_f64 = |value: f64| {
        let high = value as f32;
        let low = (value - f64::from(high)) as f32;
        (high, low)
    };
    let two_sum_f32 = |first: f32, second: f32| {
        let sum = first + second;
        let displaced = sum - first;
        let error = (first - (sum - displaced)) + (second - displaced);
        (sum, error)
    };
    let residual_leading_setup = |a: (f32, f32), csq: (f32, f32), b: (f32, f32)| {
        let product = a.0 * csq.0;
        let product_error = a.0.mul_add(csq.0, -product);
        let (leading, sum_error) = two_sum_f32(product, b.0);
        let cross = a.0.mul_add(csq.1, a.1 * csq.0) + a.1 * csq.1;
        (leading, product_error + sum_error + cross + b.1)
    };
    let residual_step_setup = |leading: (f32, f32), current: (f32, f32), previous: (f32, f32)| {
        let product = leading.0 * current.0;
        let product_error = leading.0.mul_add(current.0, -product);
        let (high, sum_error) = two_sum_f32(product, previous.0);
        let low = leading.0.mul_add(current.1, product_error + sum_error);
        let low = leading.1.mul_add(current.0, low) + previous.1;
        (high, low)
    };

    let mut epsilon = vec![0.0_f32; lmax + 4];
    let mut alpha = vec![0.0_f32; lmax + 4];
    let mut coefficient_a = vec![0.0_f32; lmax + 4];
    let mut coefficient_b = vec![0.0_f32; lmax + 4];
    let mut coefficient_even = vec![Complex::new(0.0_f32, 0.0); lmax + 4];
    let mut coefficient_odd = vec![Complex::new(0.0_f32, 0.0); lmax + 4];
    let mut packed_sin = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut packed_csq = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut previous = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut current = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut scale = vec![0_i64; super::SCALAR_ORDER_BATCH_SIZE];
    let mut corfac = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_even_re = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_even_im = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_odd_re = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_odd_im = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut scratch = super::ScalarOrderedScratch {
        epsilon: &mut epsilon,
        alpha: &mut alpha,
        coefficient_a: &mut coefficient_a,
        coefficient_b: &mut coefficient_b,
        coefficient_even: &mut coefficient_even,
        coefficient_odd: &mut coefficient_odd,
        packed_sin: &mut packed_sin,
        packed_csq: &mut packed_csq,
        previous: &mut previous,
        current: &mut current,
        scale: &mut scale,
        corfac: &mut corfac,
        sum_even_re: &mut sum_even_re,
        sum_even_im: &mut sum_even_im,
        sum_odd_re: &mut sum_odd_re,
        sum_odd_im: &mut sum_odd_im,
    };

    let mut sum_sq_actual = 0.0_f64;
    let mut sum_sq_accumulation = 0.0_f64;
    let mut sum_sq_recurrence = 0.0_f64;
    let mut sum_sq_total = 0.0_f64;
    let mut reference_sq = 0.0_f64;
    let mut sum_sq_factor_setup = 0.0_f64;
    let mut sum_sq_geometry_setup = 0.0_f64;
    let mut sum_sq_sectoral_setup = 0.0_f64;
    let mut sum_sq_coefficient_setup = 0.0_f64;
    let mut sum_sq_high_low_setup = 0.0_f64;
    let mut high_low_setup_reference_sq = 0.0_f64;
    let mut sum_sq_high_only_setup = 0.0_f64;
    let mut high_only_setup_reference_sq = 0.0_f64;
    let mut max_high_low_setup_error = 0.0_f64;
    let mut max_actual_vs_replay = 0.0_f64;
    let mut max_accumulation = 0.0_f64;
    let mut max_recurrence = 0.0_f64;
    let mut max_total = 0.0_f64;
    let mut observation_count = 0usize;
    let mut ring_stats = vec![[0.0_f64; 5]; target_rings.len()];
    let mut order_stats = vec![[0.0_f64; 5]; mmax + 1];
    let mut setup_ring_stats = vec![[0.0_f64; 5]; target_rings.len()];

    for m in 1..=mmax {
        let coefficients = (m..=lmax)
            .map(|degree| {
                let value = if degree >= 600 {
                    let seed = ((degree * 37 + m * 13) % 997) as f32 / 997.0;
                    (seed - 0.5) / 721.0
                } else {
                    0.0
                };
                Complex32::new(value, 0.0)
            })
            .collect::<Vec<_>>();
        assert!(super::prepare_order(&mut scratch, m, lmax, &coefficients));
        let pair_count = (lmax - m + 2) / 2;
        let even32_wide = scratch.coefficient_even[..pair_count]
            .iter()
            .map(|value| f64::from(value.re))
            .collect::<Vec<_>>();
        let odd32_wide = scratch.coefficient_odd[..pair_count]
            .iter()
            .map(|value| f64::from(value.re))
            .collect::<Vec<_>>();

        let mut epsilon64 = vec![0.0_f64; lmax + 4];
        let mut alpha64 = vec![0.0_f64; lmax + 4];
        let mut coefficient_a64 = vec![0.0_f64; lmax + 4];
        let mut coefficient_b64 = vec![0.0_f64; lmax + 4];
        let mut coefficient_even64 = vec![Complex::new(0.0_f64, 0.0); lmax + 4];
        let mut coefficient_odd64 = vec![Complex::new(0.0_f64, 0.0); lmax + 4];
        let coefficients64 = coefficients
            .iter()
            .map(|value| Complex::new(f64::from(value.re), f64::from(value.im)))
            .collect::<Vec<_>>();
        crate::legendre::numeric::fill_epsilon(m, lmax, &mut epsilon64);
        assert!(crate::legendre::scalar_paired::fill_factors_typed(
            m,
            lmax,
            &epsilon64,
            &mut alpha64,
            &mut coefficient_a64,
            &mut coefficient_b64,
            f64::MAX,
        ));
        assert!(
            crate::legendre::scalar_paired::prepare_synthesis_coefficients(
                m,
                lmax,
                &epsilon64,
                &alpha64,
                &coefficients64,
                &mut coefficient_even64,
                &mut coefficient_odd64,
            )
        );
        let coefficient_a64 = &coefficient_a64[..pair_count];
        let coefficient_b64 = &coefficient_b64[..pair_count];
        let coefficient_a_high_only = coefficient_a64
            .iter()
            .map(|&value| f64::from(value as f32))
            .collect::<Vec<_>>();
        let coefficient_b_high_only = coefficient_b64
            .iter()
            .map(|&value| f64::from(value as f32))
            .collect::<Vec<_>>();
        let coefficient_even64 = coefficient_even64[..pair_count]
            .iter()
            .map(|value| value.re)
            .collect::<Vec<_>>();
        let coefficient_odd64 = coefficient_odd64[..pair_count]
            .iter()
            .map(|value| value.re)
            .collect::<Vec<_>>();
        for (ring_slot, &ring) in target_rings.iter().enumerate() {
            let angle = grid.theta()[ring];
            let (sine, cosine) = crate::legendre::sin_cos_from_f64::<f32>(angle);
            let (sine64, cosine64) = crate::legendre::sin_cos_from_f64::<f64>(angle);
            let csq = if cosine.abs() > 0.99 {
                (1.0_f32 - sine) * (1.0_f32 + sine)
            } else {
                cosine * cosine
            };
            let csq64 = if cosine64.abs() > 0.99 {
                (1.0_f64 - sine64) * (1.0_f64 + sine64)
            } else {
                cosine64 * cosine64
            };
            scratch.packed_csq[..lanes].fill(csq);
            let sine_batch = vec![sine; lanes];
            let mut initial = vec![0.0_f32; lanes];
            let mut initial_scale = vec![0_i64; lanes];
            let sign = if m.is_multiple_of(2) { 1.0 } else { -1.0 };
            assert!(arch.dispatch(Initialize {
                sine: &sine_batch,
                values: &mut initial,
                scales: &mut initial_scale,
                exponent: m,
                lower_limit: super::kernel::power_lower_limit::<f32>(m),
                factor: sign * sectoral_norms[m],
            }));
            for lane in 0..lanes {
                if initial[lane].abs() > <f32 as crate::legendre::OrderedSimdPrecision>::SCALE_FTOL
                {
                    let next =
                        initial[lane] * <f32 as crate::legendre::LegendrePrecision>::SCALE_DOWN;
                    if initial[lane] == 0.0 || next != 0.0 {
                        initial[lane] = next;
                        initial_scale[lane] += 1;
                    }
                }
            }
            assert!(
                initial_scale.iter().all(|&value| value >= 0),
                "the test geometry entered the scaled pair loop: m={m}, ring={ring}, scale={initial_scale:?}"
            );
            for lane in 0..lanes {
                let exponent = initial_scale[lane]
                    * <f32 as crate::legendre::LegendrePrecision>::SCALE_EXPONENT;
                initial[lane] = <f32 as crate::legendre::OrderedSimdPrecision>::scale_binary(
                    initial[lane],
                    exponent,
                );
                initial_scale[lane] = 0;
            }

            let mut previous32 = 0.0_f32;
            let mut current32 = initial[0];
            let mut even32 = 0.0_f32;
            let mut odd32 = 0.0_f32;
            let mut previous64 = 0.0_f64;
            let mut current64 = f64::from(initial[0]);
            let mut even64_from_f32_state = 0.0_f64;
            let mut odd64_from_f32_state = 0.0_f64;
            let mut even64_from_f64_state = 0.0_f64;
            let mut odd64_from_f64_state = 0.0_f64;
            let sign64 = if m.is_multiple_of(2) { 1.0 } else { -1.0 };
            let initial64 =
                sign64 * sectoral_norms64[m] * super::kernel::scalar_integer_power(sine64, m);
            let mut high_low_previous = (0.0_f32, 0.0_f32);
            let mut high_low_current = split_f64(initial64);
            let mut high_low_even = 0.0_f64;
            let mut high_low_odd = 0.0_f64;

            for pair in 0..pair_count {
                let even = scratch.coefficient_even[pair].re;
                let odd = scratch.coefficient_odd[pair].re;
                even32 = current32.mul_add(even, even32);
                odd32 = current32.mul_add(odd, odd32);
                even64_from_f32_state =
                    f64::from(current32).mul_add(f64::from(even), even64_from_f32_state);
                odd64_from_f32_state =
                    f64::from(current32).mul_add(f64::from(odd), odd64_from_f32_state);
                even64_from_f64_state = current64.mul_add(f64::from(even), even64_from_f64_state);
                odd64_from_f64_state = current64.mul_add(f64::from(odd), odd64_from_f64_state);
                let high_low_wide = f64::from(high_low_current.0) + f64::from(high_low_current.1);
                high_low_even = high_low_wide.mul_add(coefficient_even64[pair], high_low_even);
                high_low_odd = high_low_wide.mul_add(coefficient_odd64[pair], high_low_odd);
                let high_low_leading = residual_leading_setup(
                    split_f64(coefficient_a64[pair]),
                    split_f64(csq64),
                    split_f64(coefficient_b64[pair]),
                );
                let high_low_next =
                    residual_step_setup(high_low_leading, high_low_current, high_low_previous);
                high_low_previous = high_low_current;
                high_low_current = high_low_next;

                let leading32 =
                    scratch.coefficient_a[pair].mul_add(csq, scratch.coefficient_b[pair]);
                let next32 = leading32.mul_add(current32, previous32);
                previous32 = current32;
                current32 = next32;

                let leading64 = f64::from(scratch.coefficient_a[pair])
                    .mul_add(f64::from(csq), f64::from(scratch.coefficient_b[pair]));
                let next64 = leading64.mul_add(current64, previous64);
                previous64 = current64;
                current64 = next64;
            }

            assert!(
                super::kernel::synthesize_prepared_component_batch_dispatched::<f32>(
                    arch,
                    m,
                    lmax,
                    sectoral_norms[m],
                    &sine_batch,
                    lanes,
                    &mut scratch,
                )
            );
            let actual = scratch.sum_even_re[0] + scratch.sum_odd_re[0] * cosine;
            let replay = even32 + odd32 * cosine;
            assert!(
                actual.is_finite(),
                "nonfinite generic result at m={m}, ring={ring}"
            );
            let accumulation_reference =
                even64_from_f32_state + odd64_from_f32_state * f64::from(cosine);
            let wide_recurrence_reference =
                even64_from_f64_state + odd64_from_f64_state * f64::from(cosine);
            let (factor_even, factor_odd) = replay_f64(
                f64::from(initial[0]),
                f64::from(csq),
                coefficient_a64,
                coefficient_b64,
                &even32_wide,
                &odd32_wide,
            );
            let factor_reference = factor_even + factor_odd * f64::from(cosine);
            let (geometry_even, geometry_odd) = replay_f64(
                f64::from(initial[0]),
                csq64,
                coefficient_a64,
                coefficient_b64,
                &even32_wide,
                &odd32_wide,
            );
            let geometry_reference = geometry_even + geometry_odd * cosine64;
            let (sectoral_even, sectoral_odd) = replay_f64(
                initial64,
                csq64,
                coefficient_a64,
                coefficient_b64,
                &even32_wide,
                &odd32_wide,
            );
            let sectoral_reference = sectoral_even + sectoral_odd * cosine64;
            let (coefficient_even, coefficient_odd) = replay_f64(
                initial64,
                csq64,
                coefficient_a64,
                coefficient_b64,
                &coefficient_even64,
                &coefficient_odd64,
            );
            let full_wide_reference = coefficient_even + coefficient_odd * cosine64;
            let high_low_setup_value = high_low_even + high_low_odd * cosine64;
            let high_low_setup_error = high_low_setup_value - full_wide_reference;
            let (high_only_even, high_only_odd) = replay_f64(
                f64::from(initial64 as f32),
                f64::from(csq64 as f32),
                &coefficient_a_high_only,
                &coefficient_b_high_only,
                &even32_wide,
                &odd32_wide,
            );
            let high_only_setup_value = high_only_even + high_only_odd * f64::from(cosine64 as f32);
            let high_only_setup_error = high_only_setup_value - full_wide_reference;
            let accumulation_error = f64::from(actual) - accumulation_reference;
            let recurrence_error = accumulation_reference - wide_recurrence_reference;
            let total_error = f64::from(actual) - wide_recurrence_reference;
            let factor_setup_error = factor_reference - wide_recurrence_reference;
            let geometry_setup_error = geometry_reference - factor_reference;
            let sectoral_setup_error = sectoral_reference - geometry_reference;
            let coefficient_setup_error = full_wide_reference - sectoral_reference;

            sum_sq_actual += f64::from(actual).powi(2);
            sum_sq_accumulation += accumulation_error.powi(2);
            sum_sq_recurrence += recurrence_error.powi(2);
            sum_sq_total += total_error.powi(2);
            reference_sq += wide_recurrence_reference.powi(2);
            sum_sq_factor_setup += factor_setup_error.powi(2);
            sum_sq_geometry_setup += geometry_setup_error.powi(2);
            sum_sq_sectoral_setup += sectoral_setup_error.powi(2);
            sum_sq_coefficient_setup += coefficient_setup_error.powi(2);
            sum_sq_high_low_setup += high_low_setup_error.powi(2);
            high_low_setup_reference_sq += full_wide_reference.powi(2);
            sum_sq_high_only_setup += high_only_setup_error.powi(2);
            high_only_setup_reference_sq += full_wide_reference.powi(2);
            max_high_low_setup_error = max_high_low_setup_error.max(high_low_setup_error.abs());
            let setup_stats = &mut setup_ring_stats[ring_slot];
            setup_stats[0] += factor_setup_error.powi(2);
            setup_stats[1] += geometry_setup_error.powi(2);
            setup_stats[2] += sectoral_setup_error.powi(2);
            setup_stats[3] += coefficient_setup_error.powi(2);
            setup_stats[4] += wide_recurrence_reference.powi(2);
            for stats in [&mut ring_stats[ring_slot], &mut order_stats[m]] {
                stats[0] += accumulation_error.powi(2);
                stats[1] += recurrence_error.powi(2);
                stats[2] += total_error.powi(2);
                stats[3] += wide_recurrence_reference.powi(2);
                stats[4] += 1.0;
            }
            max_actual_vs_replay =
                max_actual_vs_replay.max((f64::from(actual) - f64::from(replay)).abs());
            max_accumulation = max_accumulation.max(accumulation_error.abs());
            max_recurrence = max_recurrence.max(recurrence_error.abs());
            max_total = max_total.max(total_error.abs());
            observation_count += 1;
        }
    }

    let rms = |sum_sq: f64| (sum_sq / observation_count as f64).sqrt();
    eprintln!(
        "ordinary recurrence/accumulation probe: observations={observation_count}, lanes={lanes}, actual_rms={:.9e}, reference_rms={:.9e}, accumulation_rms={:.9e}, recurrence_rms={:.9e}, total_rms={:.9e}, relative_accumulation={:.9e}, relative_recurrence={:.9e}, factor_setup_rms={:.9e}, geometry_setup_rms={:.9e}, sectoral_setup_rms={:.9e}, coefficient_setup_rms={:.9e}, relative_factor_setup={:.9e}, relative_geometry_setup={:.9e}, relative_sectoral_setup={:.9e}, relative_coefficient_setup={:.9e}, high_only_setup_rel_rms={:.9e}, high_low_setup_rel_rms={:.9e}, max_high_low_setup_error={max_high_low_setup_error:.9e}, max_replay_diff={max_actual_vs_replay:.9e}, max_accumulation={max_accumulation:.9e}, max_recurrence={max_recurrence:.9e}, max_total={max_total:.9e}",
        rms(sum_sq_actual),
        rms(reference_sq),
        rms(sum_sq_accumulation),
        rms(sum_sq_recurrence),
        rms(sum_sq_total),
        rms(sum_sq_accumulation) / rms(reference_sq),
        rms(sum_sq_recurrence) / rms(reference_sq),
        rms(sum_sq_factor_setup),
        rms(sum_sq_geometry_setup),
        rms(sum_sq_sectoral_setup),
        rms(sum_sq_coefficient_setup),
        rms(sum_sq_factor_setup) / rms(reference_sq),
        rms(sum_sq_geometry_setup) / rms(reference_sq),
        rms(sum_sq_sectoral_setup) / rms(reference_sq),
        rms(sum_sq_coefficient_setup) / rms(reference_sq),
        rms(sum_sq_high_only_setup) / rms(high_only_setup_reference_sq),
        rms(sum_sq_high_low_setup) / rms(high_low_setup_reference_sq),
    );
    for (ring, stats) in target_rings.iter().zip(&ring_stats) {
        eprintln!(
            "  ring={ring}: accumulation_rel={:.6e}, recurrence_rel={:.6e}, total_rel={:.6e}",
            (stats[0] / stats[3]).sqrt(),
            (stats[1] / stats[3]).sqrt(),
            (stats[2] / stats[3]).sqrt(),
        );
    }
    for (ring, stats) in target_rings.iter().zip(&setup_ring_stats) {
        eprintln!(
            "  setup_ring={ring}: factors_rel={:.6e}, geometry_rel={:.6e}, sectoral_rel={:.6e}, coefficient_rel={:.6e}",
            (stats[0] / stats[4]).sqrt(),
            (stats[1] / stats[4]).sqrt(),
            (stats[2] / stats[4]).sqrt(),
            (stats[3] / stats[4]).sqrt(),
        );
    }
    for (m, stats) in order_stats.iter().enumerate().skip(1) {
        eprintln!(
            "  order={m}: accumulation_rel={:.6e}, recurrence_rel={:.6e}, total_rel={:.6e}",
            (stats[0] / stats[3]).sqrt(),
            (stats[1] / stats[3]).sqrt(),
            (stats[2] / stats[3]).sqrt(),
        );
    }
}

#[test]
#[ignore = "test-only double-single recurrence-state accuracy replay"]
fn native_f32_compensated_recurrence_state_probe() {
    use num_complex::{Complex, Complex32};

    struct LaneCount;
    impl WithSimd for LaneCount {
        type Output = usize;

        fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
            <f32 as crate::legendre::OrderedSimdPrecision>::lane_count::<S>()
        }
    }

    struct Initialize<'a> {
        sine: &'a [f32],
        values: &'a mut [f32],
        scales: &'a mut [i64],
        exponent: usize,
        lower_limit: f32,
        factor: f32,
    }
    impl WithSimd for Initialize<'_> {
        type Output = bool;

        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            super::kernel::initialize_sectoral_batch::<f32, S>(
                simd,
                self.sine,
                self.values,
                self.scales,
                self.exponent,
                self.lower_limit,
                self.factor,
            )
        }
    }

    fn two_sum(first: f32, second: f32) -> (f32, f32) {
        let sum = first + second;
        let displaced = sum - first;
        let error = (first - (sum - displaced)) + (second - displaced);
        (sum, error)
    }

    fn quick_two_sum(first: f32, second: f32) -> (f32, f32) {
        let sum = first + second;
        (sum, second - (sum - first))
    }

    fn double_single_add(first: (f32, f32), second: (f32, f32)) -> (f32, f32) {
        let (head, head_error) = two_sum(first.0, second.0);
        let (tail, tail_error) = two_sum(first.1, second.1);
        let (middle, middle_error) = two_sum(head_error, tail);
        let (renormalized, renormalized_error) = quick_two_sum(head, middle);
        let residual = tail_error + middle_error + renormalized_error;
        let (high, low) = two_sum(renormalized, residual);
        quick_two_sum(high, low)
    }

    fn double_single_mul(first: (f32, f32), second: (f32, f32)) -> (f32, f32) {
        let product = first.0 * second.0;
        let mut error = first.0.mul_add(second.0, -product);
        error = first.0.mul_add(second.1, error);
        error = first.1.mul_add(second.0, error);
        quick_two_sum(product, error)
    }

    // A first-order residual expansion. The high state follows one ordinary
    // rounded recurrence; the low state tracks its residual and propagates
    // previous low parts using fused operations.
    fn residual_leading(a: f32, cosine_squared: f32, b: f32) -> (f32, f32) {
        let product = a * cosine_squared;
        let product_error = a.mul_add(cosine_squared, -product);
        let (high, sum_error) = two_sum(product, b);
        (high, product_error + sum_error)
    }

    fn residual_recurrence(
        leading: (f32, f32),
        current: (f32, f32),
        previous: (f32, f32),
    ) -> (f32, f32) {
        let product = leading.0 * current.0;
        let product_error = leading.0.mul_add(current.0, -product);
        let (high, sum_error) = two_sum(product, previous.0);
        let low = leading.0.mul_add(current.1, product_error + sum_error);
        let low = leading.1.mul_add(current.0, low) + previous.1;
        (high, low)
    }

    let lmax = 720;
    let mmax = 16;
    let grid = crate::grid::GridSpec::new(
        crate::grid::GridKind::GaussLegendre,
        lmax + 1,
        2 * mmax + 2,
        0.23,
    )
    .unwrap();
    let target_rings = [0, 1, 2, 3, 4, 5, 360, 715, 716, 717, 718, 719, 720];
    let arch = Arch::new();
    let lanes = arch.dispatch(LaneCount);
    assert!((1..=super::SCALAR_ORDER_BATCH_SIZE).contains(&lanes));
    let sectoral_norms = crate::legendre::numeric::sectoral_normalizations::<f32>(mmax).unwrap();

    let mut epsilon = vec![0.0_f32; lmax + 4];
    let mut alpha = vec![0.0_f32; lmax + 4];
    let mut coefficient_a = vec![0.0_f32; lmax + 4];
    let mut coefficient_b = vec![0.0_f32; lmax + 4];
    let mut coefficient_even = vec![Complex::new(0.0_f32, 0.0); lmax + 4];
    let mut coefficient_odd = vec![Complex::new(0.0_f32, 0.0); lmax + 4];
    let mut packed_sin = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut packed_csq = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut previous = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut current = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut scale = vec![0_i64; super::SCALAR_ORDER_BATCH_SIZE];
    let mut corfac = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_even_re = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_even_im = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_odd_re = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_odd_im = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut scratch = super::ScalarOrderedScratch {
        epsilon: &mut epsilon,
        alpha: &mut alpha,
        coefficient_a: &mut coefficient_a,
        coefficient_b: &mut coefficient_b,
        coefficient_even: &mut coefficient_even,
        coefficient_odd: &mut coefficient_odd,
        packed_sin: &mut packed_sin,
        packed_csq: &mut packed_csq,
        previous: &mut previous,
        current: &mut current,
        scale: &mut scale,
        corfac: &mut corfac,
        sum_even_re: &mut sum_even_re,
        sum_even_im: &mut sum_even_im,
        sum_odd_re: &mut sum_odd_re,
        sum_odd_im: &mut sum_odd_im,
    };

    let mut baseline_error_sq = 0.0_f64;
    let mut state_only_baseline_error_sq = 0.0_f64;
    let mut state_only_compensated_error_sq = 0.0_f64;
    let mut state_compensated_full_error_sq = 0.0_f64;
    let mut compensated_error_sq = 0.0_f64;
    let mut residual_error_sq = 0.0_f64;
    let mut reference_sq = 0.0_f64;
    let mut max_baseline_error = 0.0_f64;
    let mut max_compensated_error = 0.0_f64;
    let mut max_residual_error = 0.0_f64;
    let mut observations = 0usize;
    let mut ring_stats = vec![[0.0_f64; 4]; target_rings.len()];

    for (m, &sectoral_norm) in sectoral_norms.iter().enumerate().take(mmax + 1).skip(1) {
        let coefficients = (m..=lmax)
            .map(|degree| {
                let value = if degree >= 600 {
                    let seed = ((degree * 37 + m * 13) % 997) as f32 / 997.0;
                    (seed - 0.5) / 721.0
                } else {
                    0.0
                };
                Complex32::new(value, 0.0)
            })
            .collect::<Vec<_>>();
        assert!(super::prepare_order(&mut scratch, m, lmax, &coefficients));
        let pair_count = (lmax - m + 2) / 2;

        for (ring_slot, &ring) in target_rings.iter().enumerate() {
            let angle = grid.theta()[ring];
            let (sine, cosine) = crate::legendre::sin_cos_from_f64::<f32>(angle);
            let csq = if cosine.abs() > 0.99 {
                (1.0_f32 - sine) * (1.0_f32 + sine)
            } else {
                cosine * cosine
            };
            let sine_batch = vec![sine; lanes];
            let mut initial = vec![0.0_f32; lanes];
            let mut initial_scale = vec![0_i64; lanes];
            let sign = if m.is_multiple_of(2) { 1.0 } else { -1.0 };
            assert!(arch.dispatch(Initialize {
                sine: &sine_batch,
                values: &mut initial,
                scales: &mut initial_scale,
                exponent: m,
                lower_limit: super::kernel::power_lower_limit::<f32>(m),
                factor: sign * sectoral_norm,
            }));
            for lane in 0..lanes {
                if initial[lane].abs() > <f32 as crate::legendre::OrderedSimdPrecision>::SCALE_FTOL
                {
                    let next =
                        initial[lane] * <f32 as crate::legendre::LegendrePrecision>::SCALE_DOWN;
                    if initial[lane] == 0.0 || next != 0.0 {
                        initial[lane] = next;
                        initial_scale[lane] += 1;
                    }
                }
            }
            assert!(initial_scale.iter().all(|&value| value >= 0));
            for lane in 0..lanes {
                let exponent = initial_scale[lane]
                    * <f32 as crate::legendre::LegendrePrecision>::SCALE_EXPONENT;
                initial[lane] = <f32 as crate::legendre::OrderedSimdPrecision>::scale_binary(
                    initial[lane],
                    exponent,
                );
                initial_scale[lane] = 0;
            }

            let mut baseline_previous = 0.0_f32;
            let mut baseline_current = initial[0];
            let mut reference_previous = 0.0_f64;
            let mut reference_current = f64::from(initial[0]);
            let mut state_reference_previous = 0.0_f64;
            let mut state_reference_current = f64::from(initial[0]);
            let mut compensated_previous = (0.0_f32, 0.0_f32);
            let mut compensated_current = (initial[0], 0.0_f32);
            let mut state_compensated_previous = (0.0_f32, 0.0_f32);
            let mut state_compensated_current = (initial[0], 0.0_f32);
            let mut residual_previous = (0.0_f32, 0.0_f32);
            let mut residual_current = (initial[0], 0.0_f32);
            let mut baseline_even = 0.0_f64;
            let mut baseline_odd = 0.0_f64;
            let mut reference_even = 0.0_f64;
            let mut reference_odd = 0.0_f64;
            let mut state_reference_even = 0.0_f64;
            let mut state_reference_odd = 0.0_f64;
            let mut compensated_even = 0.0_f64;
            let mut compensated_odd = 0.0_f64;
            let mut state_compensated_even = 0.0_f64;
            let mut state_compensated_odd = 0.0_f64;
            let mut residual_even = 0.0_f64;
            let mut residual_odd = 0.0_f64;

            for pair in 0..pair_count {
                let even = scratch.coefficient_even[pair].re;
                let odd = scratch.coefficient_odd[pair].re;
                let even64 = f64::from(even);
                let odd64 = f64::from(odd);
                baseline_even = f64::from(baseline_current).mul_add(even64, baseline_even);
                baseline_odd = f64::from(baseline_current).mul_add(odd64, baseline_odd);
                reference_even = reference_current.mul_add(even64, reference_even);
                reference_odd = reference_current.mul_add(odd64, reference_odd);
                state_reference_even =
                    state_reference_current.mul_add(even64, state_reference_even);
                state_reference_odd = state_reference_current.mul_add(odd64, state_reference_odd);
                let compensated_wide =
                    f64::from(compensated_current.0) + f64::from(compensated_current.1);
                compensated_even = compensated_wide.mul_add(even64, compensated_even);
                compensated_odd = compensated_wide.mul_add(odd64, compensated_odd);
                let state_compensated_wide =
                    f64::from(state_compensated_current.0) + f64::from(state_compensated_current.1);
                state_compensated_even =
                    state_compensated_wide.mul_add(even64, state_compensated_even);
                state_compensated_odd =
                    state_compensated_wide.mul_add(odd64, state_compensated_odd);
                let residual_wide = f64::from(residual_current.0) + f64::from(residual_current.1);
                residual_even = residual_wide.mul_add(even64, residual_even);
                residual_odd = residual_wide.mul_add(odd64, residual_odd);

                let a = scratch.coefficient_a[pair];
                let b = scratch.coefficient_b[pair];
                let leading = a.mul_add(csq, b);
                let baseline_next = leading.mul_add(baseline_current, baseline_previous);
                baseline_previous = baseline_current;
                baseline_current = baseline_next;

                let state_reference_next =
                    f64::from(leading).mul_add(state_reference_current, state_reference_previous);
                state_reference_previous = state_reference_current;
                state_reference_current = state_reference_next;
                let state_compensated_next = double_single_add(
                    double_single_mul((leading, 0.0), state_compensated_current),
                    state_compensated_previous,
                );
                state_compensated_previous = state_compensated_current;
                state_compensated_current = state_compensated_next;

                let leading64 = f64::from(a).mul_add(f64::from(csq), f64::from(b));
                let reference_next = leading64.mul_add(reference_current, reference_previous);
                reference_previous = reference_current;
                reference_current = reference_next;

                let leading_compensated =
                    double_single_add(double_single_mul((a, 0.0), (csq, 0.0)), (b, 0.0));
                let compensated_next = double_single_add(
                    double_single_mul(leading_compensated, compensated_current),
                    compensated_previous,
                );
                compensated_previous = compensated_current;
                compensated_current = compensated_next;

                let residual_next = residual_recurrence(
                    residual_leading(a, csq, b),
                    residual_current,
                    residual_previous,
                );
                residual_previous = residual_current;
                residual_current = residual_next;
            }

            let cosine64 = f64::from(cosine);
            let baseline_value = baseline_even + baseline_odd * cosine64;
            let reference_value = reference_even + reference_odd * cosine64;
            let compensated_value = compensated_even + compensated_odd * cosine64;
            let residual_value = residual_even + residual_odd * cosine64;
            let state_reference_value = state_reference_even + state_reference_odd * cosine64;
            let state_compensated_value = state_compensated_even + state_compensated_odd * cosine64;
            let baseline_error = baseline_value - reference_value;
            let state_only_baseline_error = baseline_value - state_reference_value;
            let state_only_compensated_error = state_compensated_value - state_reference_value;
            let state_compensated_full_error = state_compensated_value - reference_value;
            let compensated_error = compensated_value - reference_value;
            let residual_error = residual_value - reference_value;
            baseline_error_sq += baseline_error.powi(2);
            state_only_baseline_error_sq += state_only_baseline_error.powi(2);
            state_only_compensated_error_sq += state_only_compensated_error.powi(2);
            state_compensated_full_error_sq += state_compensated_full_error.powi(2);
            compensated_error_sq += compensated_error.powi(2);
            residual_error_sq += residual_error.powi(2);
            reference_sq += reference_value.powi(2);
            ring_stats[ring_slot][0] += baseline_error.powi(2);
            ring_stats[ring_slot][1] += compensated_error.powi(2);
            ring_stats[ring_slot][2] += reference_value.powi(2);
            ring_stats[ring_slot][3] += residual_error.powi(2);
            max_baseline_error = max_baseline_error.max(baseline_error.abs());
            max_compensated_error = max_compensated_error.max(compensated_error.abs());
            max_residual_error = max_residual_error.max(residual_error.abs());
            observations += 1;
        }
    }

    let reference_rms = (reference_sq / observations as f64).sqrt();
    let baseline_rms = (baseline_error_sq / observations as f64).sqrt();
    let state_only_baseline_rms = (state_only_baseline_error_sq / observations as f64).sqrt();
    let state_only_compensated_rms = (state_only_compensated_error_sq / observations as f64).sqrt();
    let state_compensated_full_rms = (state_compensated_full_error_sq / observations as f64).sqrt();
    let compensated_rms = (compensated_error_sq / observations as f64).sqrt();
    let residual_rms = (residual_error_sq / observations as f64).sqrt();
    eprintln!(
        "compensated recurrence-state probe: observations={observations}, lanes={lanes}, reference_rms={reference_rms:.9e}, baseline_full_rel_rms={:.9e}, baseline_fixed_lead_state_rel_rms={:.9e}, compensated_fixed_lead_state_rel_rms={:.9e}, fixed_lead_compensated_vs_full_rel_rms={:.9e}, compensated_full_rel_rms={:.9e}, residual_recurrence_rel_rms={:.9e}, improvement={:.6}, max_baseline_error={max_baseline_error:.9e}, max_compensated_error={max_compensated_error:.9e}, max_residual_error={max_residual_error:.9e}",
        baseline_rms / reference_rms,
        state_only_baseline_rms / reference_rms,
        state_only_compensated_rms / reference_rms,
        state_compensated_full_rms / reference_rms,
        compensated_rms / reference_rms,
        residual_rms / reference_rms,
        baseline_rms / compensated_rms,
    );
    for (ring, stats) in target_rings.iter().zip(&ring_stats) {
        let denom = (stats[2] / mmax as f64).sqrt();
        eprintln!(
            "  ring={ring}: baseline_rel={:.6e}, compensated_rel={:.6e}, residual_rel={:.6e}",
            (stats[0] / mmax as f64).sqrt() / denom,
            (stats[1] / mmax as f64).sqrt() / denom,
            (stats[3] / mmax as f64).sqrt() / denom,
        );
    }
    assert!(reference_rms.is_finite() && reference_rms > 0.0);
    assert!(baseline_rms.is_finite() && compensated_rms.is_finite() && residual_rms.is_finite());
}

#[test]
#[ignore = "test-only AVX2 first-order residual recurrence cost screen"]
fn native_f32_vector_residual_recurrence_cost_probe() {
    struct LaneCount;
    impl WithSimd for LaneCount {
        type Output = usize;

        fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
            <f32 as crate::legendre::OrderedSimdPrecision>::lane_count::<S>()
        }
    }

    struct PreparedOrder {
        coefficient_a: Vec<f32>,
        coefficient_b: Vec<f32>,
        coefficient_a_low: Vec<f32>,
        coefficient_b_low: Vec<f32>,
        coefficient_even: Vec<f32>,
        coefficient_odd: Vec<f32>,
        cosine_squared: Vec<f32>,
        cosine_squared_low: Vec<f32>,
        cosine: Vec<f32>,
        cosine_low: Vec<f32>,
        initial: Vec<f32>,
        initial_low: Vec<f32>,
    }

    let lmax = 720;
    let mmax = 16;
    let grid = crate::grid::GridSpec::new(
        crate::grid::GridKind::GaussLegendre,
        lmax + 1,
        2 * mmax + 2,
        0.23,
    )
    .unwrap();
    let arch = Arch::new();
    let lanes = arch.dispatch(LaneCount);
    assert_eq!(lanes, 8, "the cost screen requires AVX2 f32 width");
    let sectoral_norms = crate::legendre::numeric::sectoral_normalizations::<f32>(mmax).unwrap();

    let mut epsilon = vec![0.0_f32; lmax + 4];
    let mut alpha = vec![0.0_f32; lmax + 4];
    let mut coefficient_a = vec![0.0_f32; lmax + 4];
    let mut coefficient_b = vec![0.0_f32; lmax + 4];
    let mut coefficient_even = vec![num_complex::Complex::new(0.0_f32, 0.0); lmax + 4];
    let mut coefficient_odd = vec![num_complex::Complex::new(0.0_f32, 0.0); lmax + 4];
    let mut packed_sin = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut packed_csq = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut previous = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut current = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut scale = vec![0_i64; super::SCALAR_ORDER_BATCH_SIZE];
    let mut corfac = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_even_re = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_even_im = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_odd_re = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut sum_odd_im = vec![0.0_f32; super::SCALAR_ORDER_BATCH_SIZE];
    let mut scratch = super::ScalarOrderedScratch {
        epsilon: &mut epsilon,
        alpha: &mut alpha,
        coefficient_a: &mut coefficient_a,
        coefficient_b: &mut coefficient_b,
        coefficient_even: &mut coefficient_even,
        coefficient_odd: &mut coefficient_odd,
        packed_sin: &mut packed_sin,
        packed_csq: &mut packed_csq,
        previous: &mut previous,
        current: &mut current,
        scale: &mut scale,
        corfac: &mut corfac,
        sum_even_re: &mut sum_even_re,
        sum_even_im: &mut sum_even_im,
        sum_odd_re: &mut sum_odd_re,
        sum_odd_im: &mut sum_odd_im,
    };

    let ring = 360;
    let angle = grid.theta()[ring];
    let (sine, cosine) = crate::legendre::sin_cos_from_f64::<f32>(angle);
    let cosine_squared = if cosine.abs() > 0.99 {
        (1.0_f32 - sine) * (1.0_f32 + sine)
    } else {
        cosine * cosine
    };
    let (sine64, cosine64) = crate::legendre::sin_cos_from_f64::<f64>(angle);
    let cosine_squared64 = if cosine64.abs() > 0.99 {
        (1.0_f64 - sine64) * (1.0_f64 + sine64)
    } else {
        cosine64 * cosine64
    };
    let split_low = |value: f64, high: f32| (value - f64::from(high)) as f32;
    let sectoral_norms64 = crate::legendre::numeric::sectoral_normalizations::<f64>(mmax).unwrap();
    let mut prepared = Vec::with_capacity(mmax);
    for (m, &sectoral_norm) in sectoral_norms.iter().enumerate().take(mmax + 1).skip(1) {
        let coefficients = (m..=lmax)
            .map(|degree| {
                let value = if degree >= 600 {
                    let seed = ((degree * 37 + m * 13) % 997) as f32 / 997.0;
                    (seed - 0.5) / 721.0
                } else {
                    0.0
                };
                num_complex::Complex::new(value, 0.0)
            })
            .collect::<Vec<_>>();
        assert!(super::prepare_order(&mut scratch, m, lmax, &coefficients));
        let pair_count = (lmax - m + 2) / 2;
        let sign = if m.is_multiple_of(2) { 1.0 } else { -1.0 };
        let initial = sign * sectoral_norm * super::scalar_integer_power(sine, m);
        let mut epsilon64 = vec![0.0_f64; lmax + 4];
        let mut alpha64 = vec![0.0_f64; lmax + 4];
        let mut coefficient_a64 = vec![0.0_f64; lmax + 4];
        let mut coefficient_b64 = vec![0.0_f64; lmax + 4];
        crate::legendre::numeric::fill_epsilon(m, lmax, &mut epsilon64);
        assert!(crate::legendre::scalar_paired::fill_factors_typed(
            m,
            lmax,
            &epsilon64,
            &mut alpha64,
            &mut coefficient_a64,
            &mut coefficient_b64,
            f64::MAX,
        ));
        let sign64 = if m.is_multiple_of(2) {
            1.0_f64
        } else {
            -1.0_f64
        };
        let initial64 = sign64 * sectoral_norms64[m] * super::scalar_integer_power(sine64, m);
        prepared.push(PreparedOrder {
            coefficient_a: scratch.coefficient_a[..pair_count].to_vec(),
            coefficient_b: scratch.coefficient_b[..pair_count].to_vec(),
            coefficient_a_low: scratch.coefficient_a[..pair_count]
                .iter()
                .zip(&coefficient_a64[..pair_count])
                .map(|(&high, &wide)| split_low(wide, high))
                .collect(),
            coefficient_b_low: scratch.coefficient_b[..pair_count]
                .iter()
                .zip(&coefficient_b64[..pair_count])
                .map(|(&high, &wide)| split_low(wide, high))
                .collect(),
            coefficient_even: scratch.coefficient_even[..pair_count]
                .iter()
                .map(|value| value.re)
                .collect(),
            coefficient_odd: scratch.coefficient_odd[..pair_count]
                .iter()
                .map(|value| value.re)
                .collect(),
            cosine_squared: vec![cosine_squared; lanes],
            cosine_squared_low: vec![split_low(cosine_squared64, cosine_squared); lanes],
            cosine: vec![cosine; lanes],
            cosine_low: vec![split_low(cosine64, cosine); lanes],
            initial: vec![initial; lanes],
            initial_low: vec![split_low(initial64, initial); lanes],
        });
    }

    let mut output = vec![0.0_f32; lanes];
    let run_sweep = |mode: u8, output: &mut [f32]| {
        for order in &prepared {
            assert!(arch.dispatch(OrderedF32VectorReplay {
                coefficient_a: &order.coefficient_a,
                coefficient_b: &order.coefficient_b,
                coefficient_even: &order.coefficient_even,
                coefficient_odd: &order.coefficient_odd,
                cosine_squared: &order.cosine_squared,
                cosine_squared_low: &order.cosine_squared_low,
                cosine: &order.cosine,
                cosine_low: &order.cosine_low,
                initial: &order.initial,
                initial_low: &order.initial_low,
                coefficient_a_split_high: &order.coefficient_a,
                coefficient_b_split_high: &order.coefficient_b,
                coefficient_a_low: &order.coefficient_a_low,
                coefficient_b_low: &order.coefficient_b_low,
                output,
                compensated: mode == 2,
                residual: mode == 1 || mode == 3,
                setup_split: mode == 3,
            }));
            std::hint::black_box(&*output);
        }
    };

    run_sweep(0, &mut output);
    run_sweep(1, &mut output);
    run_sweep(3, &mut output);
    let repetitions = 16;
    let mut residual_ratios = Vec::with_capacity(15);
    let mut split_ratios = Vec::with_capacity(15);
    let mut baseline_ns = Vec::with_capacity(15);
    let mut residual_ns = Vec::with_capacity(15);
    let mut split_ns = Vec::with_capacity(15);
    for pair in 0..15 {
        let mut run = |mode: u8| {
            let start = std::time::Instant::now();
            for _ in 0..repetitions {
                run_sweep(mode, &mut output);
            }
            std::hint::black_box(&output);
            start.elapsed().as_nanos()
        };
        let mut timings = [0_u128; 3];
        let start_mode = pair % 3;
        for offset in 0..3 {
            let mode = (start_mode + offset) % 3;
            timings[mode] = run(match mode {
                0 => 0,
                1 => 1,
                _ => 3,
            });
        }
        baseline_ns.push(timings[0]);
        residual_ns.push(timings[1]);
        split_ns.push(timings[2]);
        residual_ratios.push(timings[1] as f64 / timings[0] as f64);
        split_ratios.push(timings[2] as f64 / timings[0] as f64);
    }
    residual_ratios.sort_by(f64::total_cmp);
    split_ratios.sort_by(f64::total_cmp);
    baseline_ns.sort_unstable();
    residual_ns.sort_unstable();
    split_ns.sort_unstable();
    eprintln!(
        "AVX2 f32 first-order residual setup screen: lanes={lanes}, orders={mmax}, degree_pairs_per_order=353..360, rings=1 (equator), paired_runs=15, repetitions={repetitions}, baseline_median_ms={:.3}, residual_f32_setup_median_ms={:.3}, residual_split_setup_median_ms={:.3}, residual_over_baseline_median={:.3}, split_over_baseline_median={:.3}, residual_ratios={residual_ratios:?}, split_ratios={split_ratios:?}",
        baseline_ns[7] as f64 / 1.0e6,
        residual_ns[7] as f64 / 1.0e6,
        split_ns[7] as f64 / 1.0e6,
        residual_ratios[7],
        split_ratios[7],
    );
}
