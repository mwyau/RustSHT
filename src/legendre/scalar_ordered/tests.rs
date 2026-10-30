use super::{
    FTOL, OrderedSimdPrecision, SCALE_DOWN, SCALE_UP, WithSimd, all_scale_nonnegative,
    initialize_sectoral_batch, mypow, normalize_batch, normalize_lane, power_lower_limit,
    rescale_batch, rescale_prefix_step, scalar_integer_power, set_correction_factors,
    validate_reflected_geometry,
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

struct RescaleF32Dispatch<'a> {
    previous: &'a mut [f32],
    current: &'a mut [f32],
    scales: &'a mut [i64],
}

impl WithSimd for RescaleF32Dispatch<'_> {
    type Output = Option<(bool, bool)>;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        <f32 as OrderedSimdPrecision>::rescale_ordered_batch::<S, true, false>(
            simd,
            self.previous,
            self.current,
            self.scales,
        )
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
fn f32_vector_rescaling_matches_scalar_underflow_and_tail_rules() {
    let ftol = <f32 as OrderedSimdPrecision>::SCALE_FTOL;
    let scale_down = <f32 as super::LegendrePrecision>::SCALE_DOWN;
    let previous = (0..19)
        .map(|lane| match lane % 8 {
            0 => f32::from_bits(1),
            1 => 0.25,
            2 => 0.0,
            3 => 1.5,
            4 => 0.75,
            5 => -0.5,
            6 => 0.125,
            _ => f32::MAX,
        })
        .collect::<Vec<_>>();
    let current = (0..19)
        .map(|lane| match lane % 8 {
            0 => ftol * 2.0,
            1 => ftol,
            2 => -ftol * 3.0,
            3 => 0.0,
            4 => f32::NAN,
            5 => f32::INFINITY,
            6 => -ftol * 1.5,
            _ => ftol * 0.5,
        })
        .collect::<Vec<_>>();
    let scales = (0..19)
        .map(|lane| match lane % 4 {
            0 => -2,
            1 => -1,
            2 => 0,
            _ => 1,
        })
        .collect::<Vec<_>>();

    let mut expected_previous = previous.clone();
    let mut expected_current = current.clone();
    let mut expected_scales = scales.clone();
    let mut expected_any = false;
    for lane in 0..current.len() {
        if current[lane].abs() > ftol {
            let next_previous = previous[lane] * scale_down;
            let next_current = current[lane] * scale_down;
            if (previous[lane] == 0.0 || next_previous != 0.0)
                && (current[lane] == 0.0 || next_current != 0.0)
            {
                expected_previous[lane] = next_previous;
                expected_current[lane] = next_current;
                expected_scales[lane] += 1;
                expected_any = true;
            }
        }
    }

    for arch in [Arch::Scalar, Arch::new()] {
        let mut actual_previous = previous.clone();
        let mut actual_current = current.clone();
        let mut actual_scales = scales.clone();
        let (actual_any, _) = arch
            .dispatch(RescaleF32Dispatch {
                previous: &mut actual_previous,
                current: &mut actual_current,
                scales: &mut actual_scales,
            })
            .unwrap();
        assert_eq!(actual_any, expected_any);
        assert_eq!(
            actual_previous
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected_previous
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            actual_current
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected_current
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
        );
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

// The diagnostic replay intentionally uses `term + sum` to preserve source order.
#[test]
#[allow(clippy::assign_op_pattern)]
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
        sum32 = term + sum32;
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
        sum64_prepared32 = term + sum64_prepared32;
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
