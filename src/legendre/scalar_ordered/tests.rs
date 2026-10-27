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
