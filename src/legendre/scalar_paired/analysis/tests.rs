use super::*;

#[test]
fn insignificant_prefix_uses_ducc_threshold_at_all_latitude_counts() {
    assert_eq!(ANALYSIS_FTOL, 2.0_f64.powi(-60));
    assert!(!insignificant_prefix_skip_allowed(0));
    for latitude_count in [1, 2047, 2048, 2049, 3073, 4096, 8192] {
        assert!(insignificant_prefix_skip_allowed(latitude_count));
    }
}

struct F64LaneCount;

impl WithSimd for F64LaneCount {
    type Output = usize;

    fn with_simd<S: Simd>(self, _: S) -> Self::Output {
        S::F64_LANES
    }
}

struct ScaledPairAccumulationProbe<'a> {
    cos_theta: &'a [f64],
    current: &'a [f64],
    exponents: &'a [i64],
    first_even: Complex64,
    first_odd: Complex64,
    second_even: Complex64,
    second_odd: Complex64,
    first_sum_re: &'a mut [f64],
    first_sum_im: &'a mut [f64],
    second_sum_re: &'a mut [f64],
    second_sum_im: &'a mut [f64],
}

impl WithSimd for ScaledPairAccumulationProbe<'_> {
    type Output = bool;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        try_accumulate_scaled_pair_vector(
            simd,
            self.cos_theta,
            self.current,
            self.exponents,
            self.first_even,
            self.first_odd,
            self.second_even,
            self.second_odd,
            self.first_sum_re,
            self.first_sum_im,
            self.second_sum_re,
            self.second_sum_im,
        )
    }
}

#[test]
fn scaled_synthesis_prefix_vector_matches_scalar_and_declines_unsafe_products() {
    let lane_count = Arch::new().dispatch(F64LaneCount);
    if lane_count < 2 {
        return;
    }
    let cos_theta = (0..lane_count)
        .map(|lane| -0.4 + 0.3 * lane as f64)
        .collect::<Vec<_>>();
    let current = (0..lane_count)
        .map(|lane| {
            if lane == 0 {
                0.0
            } else {
                0.75 + 0.25 * lane as f64
            }
        })
        .collect::<Vec<_>>();
    let exponents = (0..lane_count)
        .map(|lane| {
            if lane == 0 {
                i64::MAX
            } else {
                [4, -4, 0, 1, -2, 3, 2, -1][lane % 8]
            }
        })
        .collect::<Vec<_>>();
    let first_even = Complex64::new(0.5, -0.75);
    let first_odd = Complex64::new(0.125, 0.25);
    let second_even = Complex64::new(-0.75, 0.625);
    let second_odd = Complex64::new(0.2, -0.15);
    let mut first_sum_re = vec![0.125; lane_count];
    let mut first_sum_im = vec![-0.25; lane_count];
    let mut second_sum_re = vec![-0.375; lane_count];
    let mut second_sum_im = vec![0.5; lane_count];
    let mut expected = [
        first_sum_re.clone(),
        first_sum_im.clone(),
        second_sum_re.clone(),
        second_sum_im.clone(),
    ];
    for lane in 0..lane_count {
        let cosine = cos_theta[lane];
        let first_factor = Complex64::new(
            first_odd.re.mul_add(cosine, first_even.re),
            first_odd.im.mul_add(cosine, first_even.im),
        );
        let second_factor = Complex64::new(
            second_odd.re.mul_add(cosine, second_even.re),
            second_odd.im.mul_add(cosine, second_even.im),
        );
        let first = scalar::multiply_complex_scaled(first_factor, current[lane], exponents[lane]);
        let second = scalar::multiply_complex_scaled(second_factor, current[lane], exponents[lane]);
        expected[0][lane] += first.re;
        expected[1][lane] += first.im;
        expected[2][lane] += second.re;
        expected[3][lane] += second.im;
    }

    assert!(Arch::new().dispatch(ScaledPairAccumulationProbe {
        cos_theta: &cos_theta,
        current: &current,
        exponents: &exponents,
        first_even,
        first_odd,
        second_even,
        second_odd,
        first_sum_re: &mut first_sum_re,
        first_sum_im: &mut first_sum_im,
        second_sum_re: &mut second_sum_re,
        second_sum_im: &mut second_sum_im,
    }));
    for (actual, expected) in [&first_sum_re, &first_sum_im, &second_sum_re, &second_sum_im]
        .into_iter()
        .zip(&expected)
    {
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(actual, expected)| { actual.to_bits() == expected.to_bits() })
        );
    }

    let tiny = 1e-200;
    let unsafe_even = Complex64::new(tiny, tiny);
    let unsafe_odd = Complex64::new(0.0, 0.0);
    let mut unsafe_first_re = vec![1.0; lane_count];
    let mut unsafe_first_im = vec![2.0; lane_count];
    let mut unsafe_second_re = vec![3.0; lane_count];
    let mut unsafe_second_im = vec![4.0; lane_count];
    let before = [
        unsafe_first_re.clone(),
        unsafe_first_im.clone(),
        unsafe_second_re.clone(),
        unsafe_second_im.clone(),
    ];
    assert!(!Arch::new().dispatch(ScaledPairAccumulationProbe {
        cos_theta: &cos_theta,
        current: &vec![tiny; lane_count],
        exponents: &vec![0; lane_count],
        first_even: unsafe_even,
        first_odd: unsafe_odd,
        second_even: unsafe_even,
        second_odd: unsafe_odd,
        first_sum_re: &mut unsafe_first_re,
        first_sum_im: &mut unsafe_first_im,
        second_sum_re: &mut unsafe_second_re,
        second_sum_im: &mut unsafe_second_im,
    }));
    for (actual, before) in [
        &unsafe_first_re,
        &unsafe_first_im,
        &unsafe_second_re,
        &unsafe_second_im,
    ]
    .into_iter()
    .zip(&before)
    {
        assert_eq!(actual, before);
    }

    let mut overflow_first_re = vec![1.0; lane_count];
    let mut overflow_first_im = vec![2.0; lane_count];
    let mut overflow_second_re = vec![3.0; lane_count];
    let mut overflow_second_im = vec![4.0; lane_count];
    let before = [
        overflow_first_re.clone(),
        overflow_first_im.clone(),
        overflow_second_re.clone(),
        overflow_second_im.clone(),
    ];
    assert!(!Arch::new().dispatch(ScaledPairAccumulationProbe {
        cos_theta: &cos_theta,
        current: &vec![1.0; lane_count],
        exponents: &vec![i64::MAX; lane_count],
        first_even,
        first_odd,
        second_even,
        second_odd,
        first_sum_re: &mut overflow_first_re,
        first_sum_im: &mut overflow_first_im,
        second_sum_re: &mut overflow_second_re,
        second_sum_im: &mut overflow_second_im,
    }));
    for (actual, before) in [
        &overflow_first_re,
        &overflow_first_im,
        &overflow_second_re,
        &overflow_second_im,
    ]
    .into_iter()
    .zip(&before)
    {
        assert_eq!(actual, before);
    }
}

struct ScaleEpochBelowProbe<'a>(&'a [f64]);

impl WithSimd for ScaleEpochBelowProbe<'_> {
    type Output = bool;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        analysis_scale_epochs_below_one(simd, self.0)
    }
}

struct AdvanceAnalysisPrefixProbe<'a> {
    cosine_squared: &'a [f64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    scale_epochs: &'a mut [f64],
    corrections: &'a mut [f64],
    skip_insignificant_validation: bool,
}

impl WithSimd for AdvanceAnalysisPrefixProbe<'_> {
    type Output = Option<(bool, bool, bool, bool)>;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            cosine_squared,
            previous,
            current,
            scale_epochs,
            corrections,
            skip_insignificant_validation,
        } = self;
        advance_and_rescale_analysis_pair(
            simd,
            [(0.0, 1.0), (0.0, 1.0)],
            cosine_squared,
            (previous, current),
            scale_epochs,
            corrections,
            skip_insignificant_validation,
        )
        .map(|(below_limit, remains_insignificant_prefix)| {
            (
                below_limit,
                remains_insignificant_prefix,
                analysis_scale_epochs_below_one(simd, scale_epochs),
                scale_epochs.iter().all(|&epoch| epoch < 0.0),
            )
        })
    }
}

struct AnalysisMaskAllProbe;

impl WithSimd for AnalysisMaskAllProbe {
    type Output = (bool, bool, bool);

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let one = simd.splat_f64s(1.0);
        let zero = simd.splat_f64s(0.0);
        let all_true = analysis_mask_all(simd, simd.equal_f64s(one, one));
        let all_false = analysis_mask_all(simd, simd.equal_f64s(one, zero));
        let values: Vec<_> = (0..S::F64_LANES)
            .map(|lane| if lane == 0 { 0.0 } else { 1.0 })
            .collect();
        let (vectors, tail) = S::as_simd_f64s(&values);
        assert_eq!(vectors.len(), 1);
        assert!(tail.is_empty());
        let one_false_lane = analysis_mask_all(simd, simd.equal_f64s(vectors[0], one));
        (all_true, all_false, one_false_lane)
    }
}

#[test]
fn analysis_mask_all_handles_all_and_mixed_lanes() {
    assert_eq!(
        Arch::new().dispatch(AnalysisMaskAllProbe),
        (true, false, false)
    );
}

#[test]
fn prefix_advance_carries_the_epoch_frontier_across_vectors_and_tails() {
    for lane_count in [4, 5, 8, 9] {
        let cosine_squared = vec![0.25; lane_count];
        let mut previous = vec![0.0; lane_count];
        let mut current = vec![ANALYSIS_FTOL * 0.5; lane_count];
        let mut scale_epochs = vec![-1.0; lane_count];
        let mut corrections = vec![0.0; lane_count];

        assert_eq!(
            Arch::new().dispatch(AdvanceAnalysisPrefixProbe {
                cosine_squared: &cosine_squared,
                previous: &mut previous,
                current: &mut current,
                scale_epochs: &mut scale_epochs,
                corrections: &mut corrections,
                skip_insignificant_validation: true,
            }),
            Some((true, true, true, true)),
            "no-crossing lanes={lane_count}"
        );

        previous.fill(0.0);
        current.fill(0.5);
        scale_epochs.fill(-1.0);
        corrections.fill(0.0);
        assert_eq!(
            Arch::new().dispatch(AdvanceAnalysisPrefixProbe {
                cosine_squared: &cosine_squared,
                previous: &mut previous,
                current: &mut current,
                scale_epochs: &mut scale_epochs,
                corrections: &mut corrections,
                skip_insignificant_validation: true,
            }),
            Some((true, false, true, false)),
            "epoch-zero lanes={lane_count}"
        );

        previous.fill(0.0);
        current.fill(0.5);
        scale_epochs.fill(-1.0);
        corrections.fill(0.0);
        scale_epochs[0] = 0.0;
        corrections[0] = 1.0;
        assert_eq!(
            Arch::new().dispatch(AdvanceAnalysisPrefixProbe {
                cosine_squared: &cosine_squared,
                previous: &mut previous,
                current: &mut current,
                scale_epochs: &mut scale_epochs,
                corrections: &mut corrections,
                skip_insignificant_validation: false,
            }),
            Some((false, false, false, false)),
            "crossing-lane lanes={lane_count}"
        );

        if lane_count >= 8 {
            previous.fill(0.0);
            current.fill(ANALYSIS_FTOL * 0.5);
            current[4..8].fill(0.5);
            scale_epochs.fill(-1.0);
            corrections.fill(0.0);
            scale_epochs[4] = 0.0;
            corrections[4] = 1.0;
            assert_eq!(
                Arch::new().dispatch(AdvanceAnalysisPrefixProbe {
                    cosine_squared: &cosine_squared,
                    previous: &mut previous,
                    current: &mut current,
                    scale_epochs: &mut scale_epochs,
                    corrections: &mut corrections,
                    skip_insignificant_validation: false,
                }),
                Some((false, false, false, false)),
                "sparse-crossing-vector lanes={lane_count}"
            );
        }
    }
}

#[test]
fn vector_sectoral_initialization_matches_high_order_reference_and_tail() {
    let m = 720;
    let lane_count = 21;
    let sin_theta: Vec<_> = (0..lane_count)
        .map(|lane| 0.71 + lane as f64 * 0.001)
        .collect();
    let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
    let mut current = vec![0.0; lane_count];

    let initialization = initialize_normal_analysis_sectoral_dispatched(
        Arch::new(),
        m,
        &sin_theta,
        sectoral_norm,
        &mut current,
    )
    .unwrap();
    assert!(initialization.complete);
    assert_eq!(
        initialization.completed_vectors,
        lane_count / Arch::new().dispatch(F64LaneCount)
    );

    for (&sine, &actual) in sin_theta.iter().zip(&current) {
        let (expected, exponent) = scalar::sectoral_value(sine, m, sectoral_norm).unwrap();
        assert_eq!(exponent, 0);
        assert!((actual - expected).abs() <= expected.abs() * 2.0e-14);
    }
}

#[test]
fn scaled_sectoral_initialization_matches_ducc_power_routes_and_tail() {
    let arch = Arch::new();
    let sin_theta = [
        0.0, 1.0e-30, 0.2, 0.5, 0.7, 0.95, 0.999, 1.0, 0.42, 0.83, 0.91,
    ];

    for m in [0, 1, 250, 720] {
        let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
        let mut current = vec![f64::NAN; sin_theta.len()];
        let mut scale_epochs = vec![-1.0; sin_theta.len()];
        assert!(
            arch.dispatch(InitializeScaledAnalysisSectoral {
                m,
                sin_theta: &sin_theta,
                sectoral_norm,
                normal_prefix_vectors: 0,
                current: &mut current,
                scale_epochs: &mut scale_epochs,
            })
            .is_some()
        );

        for (lane, (&sine, &actual)) in sin_theta.iter().zip(&current).enumerate() {
            let (value, exponent) = scalar::sectoral_value(sine, m, sectoral_norm).unwrap();
            let (expected, exponent) = normalize_analysis_seed_scalar(value, exponent).unwrap();
            assert_eq!(
                scale_epochs[lane],
                exponent as f64 / ANALYSIS_SCALE_STEP as f64
            );
            if expected == 0.0 {
                assert_eq!(actual, expected);
            } else {
                assert!((actual - expected).abs() <= expected.abs() * 2.0e-13);
            }
        }
    }
}

#[test]
fn scaled_sectoral_initialization_reuses_normal_vector_prefix() {
    let arch = Arch::new();
    let m = 720;
    let lane_width = arch.dispatch(F64LaneCount);
    let mut sin_theta = vec![0.71; lane_width * 2];
    sin_theta.extend(std::iter::repeat_n(0.5, lane_width));
    sin_theta.push(0.83);
    let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
    let mut current = vec![f64::NAN; sin_theta.len()];
    let initialization = initialize_normal_analysis_sectoral_dispatched(
        arch,
        m,
        &sin_theta,
        sectoral_norm,
        &mut current,
    )
    .unwrap();
    assert!(!initialization.complete);
    assert_eq!(initialization.completed_vectors, 2);

    let prefix_len = initialization.completed_vectors * lane_width;
    for lane in 0..prefix_len {
        let (expected, exponent) =
            scalar::sectoral_value(sin_theta[lane], m, sectoral_norm).unwrap();
        assert_eq!(exponent, 0);
        assert!((current[lane] - expected).abs() <= expected.abs() * 2.0e-14);
    }
    current[..prefix_len].fill(1234.5);
    let mut scale_epochs = vec![-7.0; sin_theta.len()];
    let result = arch.dispatch(InitializeScaledAnalysisSectoral {
        m,
        sin_theta: &sin_theta,
        sectoral_norm,
        normal_prefix_vectors: initialization.completed_vectors,
        current: &mut current,
        scale_epochs: &mut scale_epochs,
    });
    assert_eq!(result, Some(()));

    for lane in 0..prefix_len {
        assert_eq!(current[lane], 1234.5);
        assert_eq!(scale_epochs[lane], 0.0);
    }
    for lane in prefix_len..sin_theta.len() {
        let (value, exponent) = scalar::sectoral_value(sin_theta[lane], m, sectoral_norm).unwrap();
        let (expected, exponent) = normalize_analysis_seed_scalar(value, exponent).unwrap();
        assert_eq!(
            scale_epochs[lane],
            exponent as f64 / ANALYSIS_SCALE_STEP as f64
        );
        if expected == 0.0 {
            assert_eq!(current[lane], expected);
        } else {
            assert!((current[lane] - expected).abs() <= expected.abs() * 2.0e-13);
        }
    }

    let mut previous = vec![f64::NAN; sin_theta.len()];
    current.fill(f64::NAN);
    scale_epochs.fill(-7.0);
    assert_eq!(
        initialize_scaled_analysis_sectoral_lanes(
            arch,
            m,
            m,
            &sin_theta,
            sectoral_norm,
            &mut previous,
            &mut current,
            &mut scale_epochs,
        )
        .unwrap(),
        Some(false)
    );
    assert!(previous.iter().all(|&value| value == 0.0));
    assert!(scale_epochs[..prefix_len].iter().all(|&epoch| epoch == 0.0));
    for (lane, &actual) in current.iter().enumerate() {
        let (value, exponent) = scalar::sectoral_value(sin_theta[lane], m, sectoral_norm).unwrap();
        let (expected, _) = normalize_analysis_seed_scalar(value, exponent).unwrap();
        if expected == 0.0 {
            assert_eq!(actual, expected);
        } else {
            assert!((actual - expected).abs() <= expected.abs() * 2.0e-13);
        }
    }
}

#[test]
fn fused_scalar_analysis_block_matches_paired_reference_with_simd_tails() {
    let m = 2;
    let lmax = 511;
    let lane_count = 21;
    let angles: Vec<_> = (0..lane_count)
        .map(|lane| 0.04 + lane as f64 * 0.075)
        .collect();
    let sin_theta: Vec<_> = angles.iter().map(|angle| angle.sin()).collect();
    let cos_theta: Vec<_> = angles.iter().map(|angle| angle.cos()).collect();
    let same_real: Vec<_> = (0..lane_count)
        .map(|lane| (lane + 1) as f64 * 0.0013)
        .collect();
    let same_imag: Vec<_> = (0..lane_count)
        .map(|lane| -((lane + 2) as f64) * 0.0009)
        .collect();
    let opposite_real: Vec<_> = (0..lane_count)
        .map(|lane| (lane + 3) as f64 * 0.0011)
        .collect();
    let opposite_imag: Vec<_> = (0..lane_count)
        .map(|lane| -((lane + 4) as f64) * 0.0007)
        .collect();
    let mut epsilon = vec![0.0; lmax + 4];
    scalar::fill_epsilon(m, lmax + 3, &mut epsilon);
    let mut alpha = vec![0.0; lmax / 2 + 2];
    let mut coefficient_a = vec![0.0; lmax / 2 + 2];
    let mut coefficient_b = vec![0.0; lmax / 2 + 2];
    fill_factors(
        m,
        lmax,
        &epsilon,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    );
    let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
    let coefficient_count = lmax - m + 1;
    let mut staged = vec![Complex64::new(17.0, -9.0); coefficient_count];
    let mut previous = vec![0.0; lane_count];
    let mut current = vec![0.0; lane_count];
    let mut next = vec![0.0; lane_count];
    let mut exponents = vec![0; lane_count];
    let arch = Arch::new();

    let fused = fused_scalar_analysis_pair_block_dispatched(
        arch,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &same_real,
        &same_imag,
        &opposite_real,
        &opposite_imag,
        &mut staged,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
    )
    .unwrap();
    assert!(fused);

    let mut reference = vec![Complex64::new(0.0, 0.0); coefficient_count];
    for_each_y_lm_paired_status_dispatched(
        arch,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
        |l, ring_start, basis, lane_exponents, normal_range| {
            let lane_end = ring_start + basis.len();
            let (same, opposite) = accumulate_analysis_pair_dispatched(
                arch,
                true,
                normal_range,
                &same_real[ring_start..lane_end],
                &same_imag[ring_start..lane_end],
                &opposite_real[ring_start..lane_end],
                &opposite_imag[ring_start..lane_end],
                basis,
                lane_exponents,
                &cos_theta[ring_start..lane_end],
            );
            let offset = l - m;
            reference[offset] += same;
            if l < lmax {
                reference[offset + 1] += opposite;
            }
            Ok(())
        },
    )
    .unwrap();

    for (actual, expected) in staged.iter().zip(&reference) {
        assert!((*actual - expected).norm() <= 2.0e-14);
    }

    let weighted_opposite_real: Vec<_> = opposite_real
        .iter()
        .zip(&cos_theta)
        .map(|(&spectrum, &cosine)| spectrum * cosine)
        .collect();
    let weighted_opposite_imag: Vec<_> = opposite_imag
        .iter()
        .zip(&cos_theta)
        .map(|(&spectrum, &cosine)| spectrum * cosine)
        .collect();
    let cos_theta_squared: Vec<_> = sin_theta
        .iter()
        .zip(&cos_theta)
        .map(|(&sine, &cosine)| {
            if cosine.abs() > 0.99 {
                (1.0 - sine) * (1.0 + sine)
            } else {
                cosine * cosine
            }
        })
        .collect();
    let mut staged_preweighted = vec![Complex64::new(17.0, -9.0); coefficient_count];
    previous.fill(0.0);
    current.fill(0.0);
    next.fill(0.0);
    let mut scale_epochs = vec![0.0; sin_theta.len()];
    let mut range_check_was_exercised = false;
    exponents.fill(0);
    let fused_preweighted = fused_scalar_analysis_pair_block_preweighted_dispatched(
        arch,
        m,
        lmax,
        false,
        false,
        &mut range_check_was_exercised,
        &sin_theta,
        &cos_theta,
        &cos_theta_squared,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &same_real,
        &same_imag,
        &weighted_opposite_real,
        &weighted_opposite_imag,
        &mut staged_preweighted,
        &mut previous,
        &mut current,
        &mut next,
        &mut scale_epochs,
        &mut [],
        &mut exponents,
    )
    .unwrap();
    assert!(fused_preweighted);
    for (actual, expected) in staged_preweighted.iter().zip(&reference) {
        assert!((*actual - expected).norm() <= 2.0e-14);
    }
}

#[test]
fn analysis_preparation_rejects_subnormal_cosine_weight() {
    let mut opposite_real = [1.0];
    let mut opposite_imag = [0.0];
    assert!(!prepare_analysis_opposite(
        &mut opposite_real,
        &mut opposite_imag,
        &[1.0e-310],
    ));
    let mut zero_real = [0.0];
    let mut zero_imag = [0.0];
    assert!(prepare_analysis_opposite(
        &mut zero_real,
        &mut zero_imag,
        &[1.0e-310],
    ));
    assert_eq!(zero_real, [0.0]);
    assert_eq!(zero_imag, [0.0]);
}

#[test]
fn fused_analysis_input_preparation_matches_separate_preflights() {
    for lane_count in [1, 4, 5, 9] {
        let same_real: Vec<_> = (0..lane_count)
            .map(|lane| 0.2 + lane as f64 * 0.01)
            .collect();
        let same_imag: Vec<_> = (0..lane_count)
            .map(|lane| -0.3 + lane as f64 * 0.008)
            .collect();
        let opposite_real: Vec<_> = (0..lane_count)
            .map(|lane| 0.4 - lane as f64 * 0.007)
            .collect();
        let opposite_imag: Vec<_> = (0..lane_count)
            .map(|lane| -0.1 + lane as f64 * 0.003)
            .collect();
        let cosine: Vec<_> = (0..lane_count)
            .map(|lane| -0.8 + lane as f64 * 1.6 / lane_count.max(1) as f64)
            .collect();
        let products_safe = analysis_products_safe(
            &same_real,
            &same_imag,
            &opposite_real,
            &opposite_imag,
            &cosine,
        );
        let mut separate_real = opposite_real.clone();
        let mut separate_imag = opposite_imag.clone();
        let separate_weighting = products_safe
            && prepare_analysis_opposite(&mut separate_real, &mut separate_imag, &cosine);
        let mut fused_real = opposite_real.clone();
        let mut fused_imag = opposite_imag.clone();
        let fused = prepare_analysis_products_and_opposite(
            &same_real,
            &same_imag,
            &mut fused_real,
            &mut fused_imag,
            &cosine,
        );
        assert_eq!(fused, (products_safe, separate_weighting));
        if separate_weighting {
            assert_eq!(fused_real, separate_real);
            assert_eq!(fused_imag, separate_imag);
        }
    }

    let same_real = [1.0, 1.0];
    let same_imag = [0.0, 0.0];
    let opposite_real = [1.0, 1.0];
    let opposite_imag = [0.0, 0.0];
    let mut weighted_real = opposite_real;
    let mut weighted_imag = opposite_imag;
    assert_eq!(
        prepare_analysis_products_and_opposite(
            &same_real,
            &same_imag,
            &mut weighted_real,
            &mut weighted_imag,
            &[1.0, 1.0e-310],
        ),
        (true, false)
    );
    let mut zero_real = [0.0];
    let mut zero_imag = [0.0];
    assert_eq!(
        prepare_analysis_products_and_opposite(
            &[0.0],
            &[0.0],
            &mut zero_real,
            &mut zero_imag,
            &[1.0e-310],
        ),
        (true, true)
    );
    let mut extreme_real = [0.25, f64::MAX / 16.0];
    let mut extreme_imag = [0.0, 0.0];
    assert_eq!(
        prepare_analysis_products_and_opposite(
            &[0.0, 0.0],
            &[0.0, 0.0],
            &mut extreme_real,
            &mut extreme_imag,
            &[1.0, 1.0],
        ),
        (false, false)
    );
}

#[test]
fn analysis_products_safe_simd_matches_ieee_boundaries() {
    for lane_count in [0, 1, 3, 4, 5, 8, 9, LATITUDE_BLOCK_SIZE] {
        let max_input = f64::MAX / scalar::BIG / (2.0 * lane_count.max(1) as f64);
        let same_real = vec![0.5; lane_count];
        let same_imag = vec![-0.25; lane_count];
        let opposite_real = vec![f64::MIN_POSITIVE; lane_count];
        let opposite_imag = vec![-0.0; lane_count];
        let cosine = vec![1.0; lane_count];
        let check = |same_real: &[f64],
                     same_imag: &[f64],
                     opposite_real: &[f64],
                     opposite_imag: &[f64],
                     cosine: &[f64]| {
            let expected = same_real
                .iter()
                .chain(same_imag)
                .chain(opposite_real)
                .chain(opposite_imag)
                .all(|value| (value.is_normal() || *value == 0.0) && value.abs() <= max_input)
                && cosine
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= 1.0);
            assert_eq!(
                analysis_products_safe(same_real, same_imag, opposite_real, opposite_imag, cosine,),
                expected,
                "lane_count={lane_count}"
            );
        };

        check(
            &same_real,
            &same_imag,
            &opposite_real,
            &opposite_imag,
            &cosine,
        );
        for index in 0..lane_count {
            for invalid in [
                f64::NAN,
                f64::INFINITY,
                -f64::INFINITY,
                f64::MIN_POSITIVE * 0.5,
                f64::from_bits(max_input.to_bits() + 1),
            ] {
                let mut changed = same_real.clone();
                changed[index] = invalid;
                check(
                    &changed,
                    &same_imag,
                    &opposite_real,
                    &opposite_imag,
                    &cosine,
                );
                let mut changed = same_imag.clone();
                changed[index] = invalid;
                check(
                    &same_real,
                    &changed,
                    &opposite_real,
                    &opposite_imag,
                    &cosine,
                );
                let mut changed = opposite_real.clone();
                changed[index] = invalid;
                check(&same_real, &same_imag, &changed, &opposite_imag, &cosine);
                let mut changed = opposite_imag.clone();
                changed[index] = invalid;
                check(&same_real, &same_imag, &opposite_real, &changed, &cosine);
            }

            for invalid in [
                f64::NAN,
                f64::INFINITY,
                -f64::INFINITY,
                1.0 + f64::EPSILON,
                -1.0 - f64::EPSILON,
            ] {
                let mut changed = cosine.clone();
                changed[index] = invalid;
                check(
                    &same_real,
                    &same_imag,
                    &opposite_real,
                    &opposite_imag,
                    &changed,
                );
            }
        }
    }
}

#[test]
fn analysis_scale_epoch_preflight_bounds_all_recurrence_steps() {
    assert!(analysis_scale_epochs_allow_recurrence(&[0.0, -3.0], 4));
    assert!(analysis_scale_epochs_allow_recurrence(
        &[ANALYSIS_MAX_SCALE_EPOCH - 2.0],
        2,
    ));
    assert!(!analysis_scale_epochs_allow_recurrence(
        &[ANALYSIS_MAX_SCALE_EPOCH - 1.0],
        2,
    ));
    assert!(!analysis_scale_epochs_allow_recurrence(&[f64::NAN], 1));
    assert!(!analysis_scale_epochs_allow_recurrence(&[f64::INFINITY], 1));
    assert!(!analysis_scale_epochs_allow_recurrence(
        &[-ANALYSIS_MAX_SCALE_EPOCH - 1.0],
        1,
    ));
    if usize::BITS == 64 {
        assert!(!analysis_scale_epochs_allow_recurrence(&[0.0], usize::MAX));
    }
}

#[test]
fn scale_correction_preparation_carries_epoch_frontiers_across_tails() {
    let arch = Arch::new();
    let lane_width = arch.dispatch(F64LaneCount);
    for lane_count in [
        lane_width.saturating_sub(1),
        lane_width,
        lane_width + 1,
        lane_width * 2 + 3,
    ] {
        let scale_epochs: Vec<_> = (0..lane_count)
            .map(|lane| [-3.0, -1.0, 0.0, 0.5, 1.0, 2.0][lane % 6])
            .collect();
        let basis: Vec<_> = scale_epochs
            .iter()
            .map(|&epoch| if epoch < 1.0 { 0.5 } else { f64::MIN_POSITIVE })
            .collect();
        let mut corrections = vec![f64::NAN; lane_count];
        let prepared =
            prepare_analysis_scale_corrections(&basis, &scale_epochs, &mut corrections).unwrap();

        assert_eq!(
            prepared.0,
            arch.dispatch(ScaleEpochBelowProbe(&scale_epochs)),
            "below-one lane_count={lane_count}"
        );
        assert_eq!(
            prepared.1,
            scale_epochs.iter().all(|&epoch| epoch < 0.0),
            "all-negative lane_count={lane_count}"
        );
        for ((&value, &epoch), &correction) in basis.iter().zip(&scale_epochs).zip(&corrections) {
            assert_eq!(correction, analysis_scale_correction(value, epoch).unwrap());
        }
    }

    for invalid_epoch in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut correction = [0.0];
        assert_eq!(
            prepare_analysis_scale_corrections(&[0.5], &[invalid_epoch], &mut correction),
            None
        );
    }
    let mut correction = [0.0];
    assert_eq!(
        prepare_analysis_scale_corrections(&[f64::MAX], &[2.0], &mut correction),
        None
    );
}

#[test]
fn paired_factor_preflight_rejects_nonfinite_recurrence_coefficients() {
    let lmax = 4;
    let epsilon = vec![0.0; lmax + 4];
    let factor_count = lmax / 2 + 2;
    let mut alpha = vec![0.0; factor_count];
    let mut coefficient_a = vec![0.0; factor_count];
    let mut coefficient_b = vec![0.0; factor_count];

    assert!(!fill_factors(
        0,
        lmax,
        &epsilon,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    ));
}

#[test]
fn normalized_scalar_harmonic_bound_is_below_fast_factor_limit() {
    let max_degree = usize::MAX as f64;
    let addition_theorem_bound = ((2.0 * max_degree + 1.0) / (4.0 * std::f64::consts::PI)).sqrt();
    assert!(addition_theorem_bound < PAIRED_FACTOR_LIMIT);
    const { assert!(PAIRED_FACTOR_LIMIT < scalar::BIG) };
}

#[test]
fn fused_two_channel_analysis_block_matches_paired_reference_with_simd_tails() {
    let m = 2;
    let lmax = 511;
    let lane_count = 17;
    let angles: Vec<_> = (0..lane_count)
        .map(|lane| 0.04 + lane as f64 * 0.075)
        .collect();
    let sin_theta: Vec<_> = angles.iter().map(|angle| angle.sin()).collect();
    let cos_theta: Vec<_> = angles.iter().map(|angle| angle.cos()).collect();
    let same_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
        (0..lane_count)
            .map(|lane| (lane + channel + 1) as f64 * 0.0013)
            .collect()
    });
    let same_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
        (0..lane_count)
            .map(|lane| -((lane + channel + 2) as f64) * 0.0009)
            .collect()
    });
    let opposite_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
        (0..lane_count)
            .map(|lane| (lane + channel + 3) as f64 * 0.0011)
            .collect()
    });
    let opposite_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
        (0..lane_count)
            .map(|lane| -((lane + channel + 4) as f64) * 0.0007)
            .collect()
    });
    let mut epsilon = vec![0.0; lmax + 4];
    scalar::fill_epsilon(m, lmax + 3, &mut epsilon);
    let mut alpha = vec![0.0; lmax / 2 + 2];
    let mut coefficient_a = vec![0.0; lmax / 2 + 2];
    let mut coefficient_b = vec![0.0; lmax / 2 + 2];
    fill_factors(
        m,
        lmax,
        &epsilon,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    );
    let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
    let coefficient_count = lmax - m + 1;
    let mut staged_first = vec![Complex64::new(17.0, -9.0); coefficient_count];
    let mut staged_second = vec![Complex64::new(-13.0, 7.0); coefficient_count];
    let mut previous = vec![0.0; lane_count];
    let mut current = vec![0.0; lane_count];
    let mut next = vec![f64::INFINITY; lane_count];
    let mut exponents = vec![0; lane_count];
    let arch = Arch::new();

    let mut range_check_was_exercised = false;
    let fused = fused_scalar_analysis_pair_channels_block_with_range_proof(
        arch,
        true,
        false,
        false,
        false,
        &mut range_check_was_exercised,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        [&same_real[0], &same_real[1]],
        [&same_imag[0], &same_imag[1]],
        [&opposite_real[0], &opposite_real[1]],
        [&opposite_imag[0], &opposite_imag[1]],
        &mut staged_first,
        &mut staged_second,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
    )
    .unwrap();
    assert!(fused);
    assert!(range_check_was_exercised);
    let checked_first = staged_first.clone();
    let checked_second = staged_second.clone();

    let mut unchecked_first = vec![Complex64::new(17.0, -9.0); coefficient_count];
    let mut unchecked_second = vec![Complex64::new(-13.0, 7.0); coefficient_count];
    range_check_was_exercised = false;
    let unchecked = fused_scalar_analysis_pair_channels_block_with_range_proof(
        arch,
        true,
        false,
        false,
        true,
        &mut range_check_was_exercised,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        [&same_real[0], &same_real[1]],
        [&same_imag[0], &same_imag[1]],
        [&opposite_real[0], &opposite_real[1]],
        [&opposite_imag[0], &opposite_imag[1]],
        &mut unchecked_first,
        &mut unchecked_second,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
    )
    .unwrap();
    assert!(unchecked);
    assert!(!range_check_was_exercised);
    assert_eq!(unchecked_first, checked_first);
    assert_eq!(unchecked_second, checked_second);

    next.fill(0.0);
    let mut reference: [Vec<Complex64>; 2] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); coefficient_count]);
    for_each_y_lm_paired_status_dispatched(
        arch,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
        |l, ring_start, basis, lane_exponents, normal_range| {
            let lane_end = ring_start + basis.len();
            let channel_sums = accumulate_analysis_pair_channels_dispatched(
                arch,
                [true, true],
                normal_range,
                [
                    &same_real[0][ring_start..lane_end],
                    &same_real[1][ring_start..lane_end],
                ],
                [
                    &same_imag[0][ring_start..lane_end],
                    &same_imag[1][ring_start..lane_end],
                ],
                [
                    &opposite_real[0][ring_start..lane_end],
                    &opposite_real[1][ring_start..lane_end],
                ],
                [
                    &opposite_imag[0][ring_start..lane_end],
                    &opposite_imag[1][ring_start..lane_end],
                ],
                basis,
                lane_exponents,
                &cos_theta[ring_start..lane_end],
            );
            let offset = l - m;
            for channel in 0..2 {
                reference[channel][offset] += channel_sums[channel].0;
                if l < lmax {
                    reference[channel][offset + 1] += channel_sums[channel].1;
                }
            }
            Ok(())
        },
    )
    .unwrap();

    for (channel, staged) in [&staged_first, &staged_second].into_iter().enumerate() {
        for (actual, expected) in staged.iter().zip(&reference[channel]) {
            assert!((*actual - *expected).norm() <= 2.0e-14);
        }
    }

    let baseline_first = staged_first.clone();
    let baseline_second = staged_second.clone();
    let mut pairwise_first = vec![Complex64::new(17.0, -9.0); coefficient_count];
    let mut pairwise_second = vec![Complex64::new(-13.0, 7.0); coefficient_count];
    next.fill(0.0);
    let pairwise = fused_scalar_analysis_pair_channels_block_with_range_proof(
        arch,
        false,
        false,
        false,
        false,
        &mut range_check_was_exercised,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        [&same_real[0], &same_real[1]],
        [&same_imag[0], &same_imag[1]],
        [&opposite_real[0], &opposite_real[1]],
        [&opposite_imag[0], &opposite_imag[1]],
        &mut pairwise_first,
        &mut pairwise_second,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
    )
    .unwrap();
    assert!(pairwise);
    for (channel, staged) in [&pairwise_first, &pairwise_second].into_iter().enumerate() {
        for (actual, expected) in staged.iter().zip(&reference[channel]) {
            assert!((*actual - *expected).norm() <= 2.0e-14);
        }
    }

    let mut weighted_first = vec![Complex64::new(17.0, -9.0); coefficient_count];
    let mut weighted_second = vec![Complex64::new(-13.0, 7.0); coefficient_count];
    next.fill(0.0);
    let weighted = fused_scalar_analysis_pair_channels_block_dispatched(
        arch,
        true,
        true,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        [&same_real[0], &same_real[1]],
        [&same_imag[0], &same_imag[1]],
        [&opposite_real[0], &opposite_real[1]],
        [&opposite_imag[0], &opposite_imag[1]],
        &mut weighted_first,
        &mut weighted_second,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
    )
    .unwrap();
    assert!(weighted);
    for (weighted, baseline) in [
        (&weighted_first, &baseline_first),
        (&weighted_second, &baseline_second),
    ] {
        for (actual, expected) in weighted.iter().zip(baseline) {
            assert!((*actual - *expected).norm() <= 5.0e-14);
        }
    }
}

#[test]
fn fused_two_channel_analysis_preserves_opposite_product_range() {
    const LANE_COUNT: usize = 9;
    let basis = vec![2.0_f64.powi(-1022); LANE_COUNT];
    let cosine = vec![2.0_f64.powi(-54); LANE_COUNT];
    let opposite_real: [Vec<f64>; 2] = std::array::from_fn(|_| vec![2.0_f64.powi(500); LANE_COUNT]);
    let zeros: [Vec<f64>; 2] = std::array::from_fn(|_| vec![0.0; LANE_COUNT]);
    let exponents = vec![0; LANE_COUNT];

    let sums = accumulate_analysis_pair_channels_dispatched(
        Arch::new(),
        [true, true],
        true,
        [&zeros[0], &zeros[1]],
        [&zeros[0], &zeros[1]],
        [&opposite_real[0], &opposite_real[1]],
        [&zeros[0], &zeros[1]],
        &basis,
        &exponents,
        &cosine,
    );

    let expected = LANE_COUNT as f64 * 2.0_f64.powi(-576);
    assert_eq!(sums[0].1.re, expected);
    assert_eq!(sums[1].1.re, expected);
}

#[test]
fn fused_two_channel_analysis_block_declines_on_late_range_failure() {
    let m = 0;
    let lmax = 2;
    let lane_count = 17;
    let sin_theta = vec![1.0; lane_count];
    let same_real = [vec![1.0; lane_count], vec![1.0; lane_count]];
    let same_imag = [vec![0.0; lane_count], vec![0.0; lane_count]];
    let opposite_real = [vec![0.0; lane_count], vec![0.0; lane_count]];
    let opposite_imag = [vec![0.0; lane_count], vec![0.0; lane_count]];
    let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
    let mut coefficient_a = vec![0.0; lmax / 2 + 2];
    let coefficient_b = vec![0.0; lmax / 2 + 2];
    coefficient_a[0] = f64::MAX;
    let mut first_vector_fails = vec![0.0; lane_count];
    first_vector_fails[0] = 1.0;
    let mut later_vector_fails = vec![0.0; lane_count];
    later_vector_fails[8] = 1.0;
    let mut scalar_tail_fails = vec![0.0; lane_count];
    scalar_tail_fails[lane_count - 1] = 1.0;

    for cos_theta in [first_vector_fails, later_vector_fails, scalar_tail_fails] {
        let coefficient_count = lmax - m + 1;
        let mut staged_first = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut staged_second = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut previous = vec![0.0; lane_count];
        let mut current = vec![0.0; lane_count];
        let mut next = vec![0.0; lane_count];
        let mut exponents = vec![0; lane_count];

        let fused = fused_scalar_analysis_pair_channels_block_dispatched(
            Arch::new(),
            true,
            false,
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            [&same_real[0], &same_real[1]],
            [&same_imag[0], &same_imag[1]],
            [&opposite_real[0], &opposite_real[1]],
            [&opposite_imag[0], &opposite_imag[1]],
            &mut staged_first,
            &mut staged_second,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
        )
        .unwrap();
        assert!(!fused);
        assert_ne!(staged_first[0], Complex64::new(0.0, 0.0));
        assert_ne!(staged_second[0], Complex64::new(0.0, 0.0));
    }
}

#[test]
fn fused_two_channel_analysis_block_rejects_scaled_sectoral_lanes() {
    let m = 40;
    let lmax = 80;
    let sin_theta = [1.0e-6, 0.4];
    let cos_theta = [1.0, 0.916515138991168];
    let spectra = [0.25, -0.5];
    let mut epsilon = vec![0.0; lmax + 4];
    scalar::fill_epsilon(m, lmax + 3, &mut epsilon);
    let mut alpha = vec![0.0; lmax / 2 + 2];
    let mut coefficient_a = vec![0.0; lmax / 2 + 2];
    let mut coefficient_b = vec![0.0; lmax / 2 + 2];
    fill_factors(
        m,
        lmax,
        &epsilon,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    );
    let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
    let coefficient_count = lmax - m + 1;
    let mut staged_first = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut staged_second = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut previous = [0.0; 2];
    let mut current = [0.0; 2];
    let mut next = [0.0; 2];
    let mut exponents = [0; 2];

    let fused = fused_scalar_analysis_pair_channels_block_dispatched(
        Arch::new(),
        true,
        false,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        [&spectra, &spectra],
        [&spectra, &spectra],
        [&spectra, &spectra],
        [&spectra, &spectra],
        &mut staged_first,
        &mut staged_second,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
    )
    .unwrap();
    assert!(!fused);
}

#[test]
fn shared_two_channel_analysis_handles_scaled_sectoral_lanes() {
    let m = 49;
    let lmax = 721;
    let theta: Vec<_> = (0..32).map(|lane| 0.002 + lane as f64 * 0.035).collect();
    let sin_theta: Vec<_> = theta.iter().map(|theta| theta.sin()).collect();
    let cos_theta: Vec<_> = theta.iter().map(|theta| theta.cos()).collect();
    let make_channel = |offset: usize| {
        (0..sin_theta.len())
            .map(|lane| (((lane + offset) as f64 * 0.731).sin()) * 0.5)
            .collect::<Vec<_>>()
    };
    let same_real = [make_channel(1), make_channel(2)];
    let same_imag = [make_channel(3), make_channel(4)];
    let opposite_real = [make_channel(5), make_channel(6)];
    let opposite_imag = [make_channel(7), make_channel(8)];
    let mut epsilon = vec![0.0; lmax + 4];
    scalar::fill_epsilon(m, lmax + 3, &mut epsilon);
    let mut alpha = vec![0.0; lmax / 2 + 2];
    let mut coefficient_a = vec![0.0; lmax / 2 + 2];
    let mut coefficient_b = vec![0.0; lmax / 2 + 2];
    fill_factors(
        m,
        lmax,
        &epsilon,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    );
    let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
    assert_ne!(
        scalar::sectoral_value(sin_theta[0], m, sectoral_norm)
            .unwrap()
            .1,
        0
    );
    let coefficient_count = lmax - m + 1;
    let mut staged_first = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut staged_second = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut previous = vec![0.0; sin_theta.len()];
    let mut current = vec![0.0; sin_theta.len()];
    let mut next = vec![0.0; sin_theta.len()];
    let mut exponents = vec![0; sin_theta.len()];
    let mut range_check_was_exercised = false;
    let arch = Arch::new();

    let fused = fused_scalar_analysis_pair_channels_block_with_range_proof(
        arch,
        true,
        true,
        true,
        false,
        &mut range_check_was_exercised,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        [&same_real[0], &same_real[1]],
        [&same_imag[0], &same_imag[1]],
        [&opposite_real[0], &opposite_real[1]],
        [&opposite_imag[0], &opposite_imag[1]],
        &mut staged_first,
        &mut staged_second,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
    )
    .unwrap();
    assert!(fused);
    let candidate_previous = previous.clone();
    let candidate_current = current.clone();
    let candidate_exponents = exponents.clone();

    let mut expected_first = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut expected_second = vec![Complex64::new(0.0, 0.0); coefficient_count];
    for_each_y_lm_paired_status_dispatched(
        arch,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
        |l, ring_start, basis, lane_exponents, normal_range| {
            let lane_end = ring_start + basis.len();
            let sums = accumulate_analysis_pair_channels_dispatched(
                arch,
                [true, true],
                normal_range,
                [
                    &same_real[0][ring_start..lane_end],
                    &same_real[1][ring_start..lane_end],
                ],
                [
                    &same_imag[0][ring_start..lane_end],
                    &same_imag[1][ring_start..lane_end],
                ],
                [
                    &opposite_real[0][ring_start..lane_end],
                    &opposite_real[1][ring_start..lane_end],
                ],
                [
                    &opposite_imag[0][ring_start..lane_end],
                    &opposite_imag[1][ring_start..lane_end],
                ],
                basis,
                lane_exponents,
                &cos_theta[ring_start..lane_end],
            );
            let offset = l - m;
            expected_first[offset] += sums[0].0;
            expected_second[offset] += sums[1].0;
            if l < lmax {
                expected_first[offset + 1] += sums[0].1;
                expected_second[offset + 1] += sums[1].1;
            }
            Ok(())
        },
    )
    .unwrap();

    let state_delta = previous
        .iter()
        .zip(&candidate_previous)
        .chain(current.iter().zip(&candidate_current))
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);
    assert_eq!(exponents, candidate_exponents, "state exponent mismatch");
    assert!(state_delta <= 1e-14, "recurrence state delta={state_delta}");

    for (index, ((actual_first, actual_second), (expected_first, expected_second))) in staged_first
        .iter()
        .zip(&staged_second)
        .zip(expected_first.iter().zip(&expected_second))
        .enumerate()
    {
        assert!((*actual_first - *expected_first).norm() <= 5e-13);
        assert!(
            (*actual_second - *expected_second).norm() <= 5e-13,
            "index={index} l={} actual={actual_second:?} expected={expected_second:?}",
            m + index
        );
    }
}

#[test]
#[allow(unexpected_cfgs)]
fn fused_scalar_analysis_block_transitions_scaled_lanes_to_ordinary_simd() {
    let m = 250;
    let lmax = 720;
    let lane_count = 9;
    let sin_theta: Vec<_> = (0..lane_count)
        .map(|lane| {
            if lane + 1 == lane_count {
                0.31
            } else {
                0.31 + lane as f64 * 0.01
            }
        })
        .collect();
    let cos_theta: Vec<_> = sin_theta
        .iter()
        .map(|sine| (1.0 - sine * sine).sqrt())
        .collect();
    let same_real: Vec<_> = (0..lane_count)
        .map(|lane| 0.13 + lane as f64 * 0.007)
        .collect();
    let same_imag: Vec<_> = (0..lane_count)
        .map(|lane| -0.08 + lane as f64 * 0.003)
        .collect();
    let opposite_real: Vec<_> = (0..lane_count)
        .map(|lane| -0.17 + lane as f64 * 0.005)
        .collect();
    let opposite_imag: Vec<_> = (0..lane_count)
        .map(|lane| 0.09 - lane as f64 * 0.004)
        .collect();
    let mut epsilon = vec![0.0; lmax + 4];
    scalar::fill_epsilon(m, lmax + 3, &mut epsilon);
    let mut alpha = vec![0.0; lmax / 2 + 2];
    let mut coefficient_a = vec![0.0; lmax / 2 + 2];
    let mut coefficient_b = vec![0.0; lmax / 2 + 2];
    fill_factors(
        m,
        lmax,
        &epsilon,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    );
    let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
    let coefficient_count = lmax - m + 1;
    let mut staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut previous = vec![0.0; lane_count];
    let mut current = vec![0.0; lane_count];
    let mut next = vec![0.0; lane_count];
    let mut scale_epochs = vec![0.0; lane_count];
    let mut exponents = vec![0; lane_count];

    assert!(
        sin_theta
            .iter()
            .any(|&sine| scalar::sectoral_value(sine, m, sectoral_norm).unwrap().1 != 0)
    );

    let fused = fused_scalar_analysis_pair_block_dispatched(
        Arch::new(),
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &same_real,
        &same_imag,
        &opposite_real,
        &opposite_imag,
        &mut staged,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
    )
    .unwrap();
    assert!(fused);

    let mut reference = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut first_normal_degree = None;
    for_each_y_lm_paired_status_dispatched(
        Arch::new(),
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
        |l, ring_start, basis, lane_exponents, normal_range| {
            if normal_range && first_normal_degree.is_none() {
                first_normal_degree = Some(l);
            }
            let lane_end = ring_start + basis.len();
            let (same, opposite) = accumulate_analysis_pair_dispatched(
                Arch::new(),
                true,
                normal_range,
                &same_real[ring_start..lane_end],
                &same_imag[ring_start..lane_end],
                &opposite_real[ring_start..lane_end],
                &opposite_imag[ring_start..lane_end],
                basis,
                lane_exponents,
                &cos_theta[ring_start..lane_end],
            );
            let offset = l - m;
            reference[offset] += same;
            if l < lmax {
                reference[offset + 1] += opposite;
            }
            Ok(())
        },
    )
    .unwrap();
    assert!(first_normal_degree.is_some_and(|degree| degree > m));
    let reference_scale = reference
        .iter()
        .map(|value| value.norm())
        .fold(0.0, f64::max);
    for (actual, expected) in staged.iter().zip(&reference) {
        assert!(
            (*actual - *expected).norm() <= reference_scale * 2.0e-12 + 1.0e-300,
            "scaled transition differs: actual={actual:?}, expected={expected:?}"
        );
    }

    let weighted_opposite_real: Vec<_> = opposite_real
        .iter()
        .zip(&cos_theta)
        .map(|(&spectrum, &cosine)| spectrum * cosine)
        .collect();
    let weighted_opposite_imag: Vec<_> = opposite_imag
        .iter()
        .zip(&cos_theta)
        .map(|(&spectrum, &cosine)| spectrum * cosine)
        .collect();
    let cos_theta_squared: Vec<_> = sin_theta
        .iter()
        .zip(&cos_theta)
        .map(|(&sine, &cosine)| {
            if cosine.abs() > 0.99 {
                (1.0 - sine) * (1.0 + sine)
            } else {
                cosine * cosine
            }
        })
        .collect();
    assert!(insignificant_prefix_skip_allowed(lane_count));
    let input_scale = same_real
        .iter()
        .zip(&same_imag)
        .chain(weighted_opposite_real.iter().zip(&weighted_opposite_imag))
        .map(|(&real, &imag)| real.hypot(imag))
        .fold(0.0_f64, f64::max);
    let omission_bound = 2.0 * lane_count as f64 * ANALYSIS_FTOL * input_scale;
    let mut checked_staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut checked_previous = vec![0.0; lane_count];
    let mut checked_current = vec![0.0; lane_count];
    let mut checked_next = vec![0.0; lane_count];
    let mut checked_scale_epochs = vec![0.0; lane_count];
    let mut checked_exponents = vec![0; lane_count];
    let mut checked_range_was_exercised = false;
    assert!(
        fused_scalar_analysis_pair_block_preweighted_dispatched(
            Arch::new(),
            m,
            lmax,
            false,
            false,
            &mut checked_range_was_exercised,
            &sin_theta,
            &cos_theta,
            &cos_theta_squared,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &same_real,
            &same_imag,
            &weighted_opposite_real,
            &weighted_opposite_imag,
            &mut checked_staged,
            &mut checked_previous,
            &mut checked_current,
            &mut checked_next,
            &mut checked_scale_epochs,
            &mut [],
            &mut checked_exponents,
        )
        .unwrap()
    );
    staged.fill(Complex64::new(0.0, 0.0));
    previous.fill(0.0);
    current.fill(0.0);
    next.fill(0.0);
    scale_epochs.fill(0.0);
    exponents.fill(0);
    let mut scale_corrections = vec![0.0; lane_count];
    let mut range_check_was_exercised = false;
    let fused_preweighted = fused_scalar_analysis_pair_block_preweighted_dispatched(
        Arch::new(),
        m,
        lmax,
        true,
        false,
        &mut range_check_was_exercised,
        &sin_theta,
        &cos_theta,
        &cos_theta_squared,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &same_real,
        &same_imag,
        &weighted_opposite_real,
        &weighted_opposite_imag,
        &mut staged,
        &mut previous,
        &mut current,
        &mut next,
        &mut scale_epochs,
        &mut scale_corrections,
        &mut exponents,
    )
    .unwrap();
    assert!(fused_preweighted);
    assert_eq!(staged[0], Complex64::new(0.0, 0.0));
    let prefix_len = staged
        .iter()
        .take_while(|&&value| value == Complex64::new(0.0, 0.0))
        .count();
    assert!(prefix_len > 0);
    for (degree_offset, expected) in reference.iter().take(prefix_len).enumerate() {
        assert!(
            expected.norm() <= omission_bound,
            "skipped pair {degree_offset} error {:.3e} exceeds {:.3e}",
            expected.norm(),
            omission_bound,
        );
    }
    for (degree, (actual, checked)) in staged.iter().zip(&checked_staged).enumerate() {
        assert!(
            (*actual - *checked).norm() <= omission_bound,
            "insignificant terms at offset {degree} exceed {omission_bound:e}"
        );
    }
    for (actual, expected) in staged.iter().zip(&reference) {
        assert!(
            (*actual - *expected).norm() <= reference_scale * 2.0e-12 + 1.0e-300,
            "preweighted scaled transition differs: actual={actual:?}, expected={expected:?}"
        );
    }
}

#[test]
fn fused_scalar_analysis_block_declines_on_late_recurrence_range_failure() {
    let m = 0;
    let lmax = 2;
    let sectoral_norm = scalar::sectoral_normalizations(m).unwrap()[m];
    let coefficient_a = vec![0.0; lmax / 2 + 2];
    let mut coefficient_b = vec![0.0; lmax / 2 + 2];
    coefficient_b[0] = 1.0e-308;
    let same_real = [1.0; 5];
    let same_imag = [0.0; 5];
    let opposite_real = [0.0; 5];
    let opposite_imag = [0.0; 5];

    for (sin_theta, cos_theta) in [
        ([0.0; 5], [1.0; 5]),
        ([1.0, 1.0, 1.0, 1.0, 0.0], [0.0, 0.0, 0.0, 0.0, 1.0]),
    ] {
        let mut staged = vec![Complex64::new(0.0, 0.0); lmax - m + 1];
        let mut previous = [0.0; 5];
        let mut current = [0.0; 5];
        let mut next = [0.0; 5];
        let mut exponents = [0; 5];
        let fused = fused_scalar_analysis_pair_block_dispatched(
            Arch::new(),
            m,
            lmax,
            &sin_theta,
            &cos_theta,
            sectoral_norm,
            &coefficient_a,
            &coefficient_b,
            &same_real,
            &same_imag,
            &opposite_real,
            &opposite_imag,
            &mut staged,
            &mut previous,
            &mut current,
            &mut next,
            &mut exponents,
        )
        .unwrap();
        assert!(!fused);
        assert_ne!(staged[0], Complex64::new(0.0, 0.0));
    }
}

#[test]
fn dual_analysis_accumulation_matches_independent_channels_and_scaled_fallback() {
    for lane_count in [1, 3, 4, 5, 7, 8, 9, 31, 32, 33] {
        let basis: Vec<_> = (0..lane_count)
            .map(|lane| 0.2 + 0.01 * lane as f64)
            .collect();
        let cosine: Vec<_> = (0..lane_count)
            .map(|lane| -0.8 + 0.02 * lane as f64)
            .collect();
        let exponents = vec![0; lane_count];
        let same_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| (lane + channel + 1) as f64 * 1.0e-3)
                .collect::<Vec<_>>()
        });
        let same_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| -((lane + channel + 2) as f64) * 1.1e-3)
                .collect::<Vec<_>>()
        });
        let opposite_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| (lane + channel + 3) as f64 * 0.9e-3)
                .collect::<Vec<_>>()
        });
        let opposite_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| -((lane + channel + 4) as f64) * 0.7e-3)
                .collect::<Vec<_>>()
        });
        let actual = accumulate_analysis_pair_channels_dispatched(
            Arch::new(),
            [true, true],
            true,
            [&same_real[0], &same_real[1]],
            [&same_imag[0], &same_imag[1]],
            [&opposite_real[0], &opposite_real[1]],
            [&opposite_imag[0], &opposite_imag[1]],
            &basis,
            &exponents,
            &cosine,
        );
        for channel in 0..2 {
            let expected = accumulate_analysis_pair_dispatched(
                Arch::new(),
                true,
                true,
                &same_real[channel],
                &same_imag[channel],
                &opposite_real[channel],
                &opposite_imag[channel],
                &basis,
                &exponents,
                &cosine,
            );
            for (actual, expected) in [
                (actual[channel].0, expected.0),
                (actual[channel].1, expected.1),
            ] {
                let error = (actual - expected).norm();
                assert!(error <= 2.0e-14, "lanes={lane_count}, error={error:.3e}");
            }
        }
    }

    let basis = [0.5, 0.25, 0.125, 0.75, 0.4];
    let cosine = [-0.7, -0.2, 0.1, 0.5, 0.9];
    let exponents = [0, -100, 0, -20, 0];
    let same_real = [[0.25; 5], [f64::MAX / 8.0; 5]];
    let same_imag = [[-0.5; 5], [0.0; 5]];
    let opposite_real = [[0.75; 5], [f64::MAX / 8.0; 5]];
    let opposite_imag = [[-0.125; 5], [0.0; 5]];
    let products_safe = [
        analysis_products_safe(
            &same_real[0],
            &same_imag[0],
            &opposite_real[0],
            &opposite_imag[0],
            &cosine,
        ),
        analysis_products_safe(
            &same_real[1],
            &same_imag[1],
            &opposite_real[1],
            &opposite_imag[1],
            &cosine,
        ),
    ];
    assert!(products_safe[0]);
    assert!(!products_safe[1]);
    let actual = accumulate_analysis_pair_channels_dispatched(
        Arch::new(),
        products_safe,
        false,
        [&same_real[0], &same_real[1]],
        [&same_imag[0], &same_imag[1]],
        [&opposite_real[0], &opposite_real[1]],
        [&opposite_imag[0], &opposite_imag[1]],
        &basis,
        &exponents,
        &cosine,
    );
    for channel in 0..2 {
        let expected = accumulate_analysis_pair_dispatched(
            Arch::new(),
            products_safe[channel],
            false,
            &same_real[channel],
            &same_imag[channel],
            &opposite_real[channel],
            &opposite_imag[channel],
            &basis,
            &exponents,
            &cosine,
        );
        assert_eq!(actual[channel], expected);
    }
}

#[test]
fn mixed_dual_analysis_accumulation_matches_channel_kernels() {
    for lane_count in [6, LATITUDE_BLOCK_SIZE] {
        let basis: Vec<_> = (0..lane_count)
            .map(|lane| 0.2 + 0.003 * lane as f64)
            .collect();
        let cosine: Vec<_> = (0..lane_count)
            .map(|lane| -0.9 + 1.8 * lane as f64 / lane_count as f64)
            .collect();
        let exponents: Vec<_> = (0..lane_count)
            .map(|lane| match lane % 5 {
                1 => 800,
                3 => -800,
                _ => 0,
            })
            .collect();
        let same_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| (lane + channel + 1) as f64 * 1.0e-4)
                .collect()
        });
        let same_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| -((lane + channel + 2) as f64) * 1.1e-4)
                .collect()
        });
        let opposite_real: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| (lane + channel + 3) as f64 * 0.9e-4)
                .collect()
        });
        let opposite_imag: [Vec<f64>; 2] = std::array::from_fn(|channel| {
            (0..lane_count)
                .map(|lane| -((lane + channel + 4) as f64) * 0.7e-4)
                .collect()
        });
        for channel in 0..2 {
            assert!(analysis_products_safe(
                &same_real[channel],
                &same_imag[channel],
                &opposite_real[channel],
                &opposite_imag[channel],
                &cosine,
            ));
        }

        let actual = accumulate_analysis_pair_channels_dispatched(
            Arch::new(),
            [true, true],
            false,
            [&same_real[0], &same_real[1]],
            [&same_imag[0], &same_imag[1]],
            [&opposite_real[0], &opposite_real[1]],
            [&opposite_imag[0], &opposite_imag[1]],
            &basis,
            &exponents,
            &cosine,
        );
        for channel in 0..2 {
            let expected = accumulate_analysis_pair_dispatched(
                Arch::new(),
                true,
                false,
                &same_real[channel],
                &same_imag[channel],
                &opposite_real[channel],
                &opposite_imag[channel],
                &basis,
                &exponents,
                &cosine,
            );
            assert_eq!(actual[channel], expected, "lanes={lane_count}");
        }
    }
}

#[test]
fn synthesis_coefficient_preparation_matches_degree_transform_at_boundaries() {
    for m in 0..=4 {
        for lmax in m..=m + 9 {
            let mut epsilon = vec![0.0; lmax + 4];
            for (degree, value) in epsilon.iter_mut().enumerate().skip(m + 1) {
                let degree_f = degree as f64;
                let order_f = m as f64;
                *value = (((degree_f - order_f) * (degree_f + order_f))
                    / ((2.0 * degree_f + 1.0) * (2.0 * degree_f - 1.0)))
                    .sqrt();
            }
            let factor_count = lmax / 2 + 2;
            let mut alpha = vec![0.0; factor_count];
            let mut coefficient_a = vec![0.0; factor_count];
            let mut coefficient_b = vec![0.0; factor_count];
            fill_factors(
                m,
                lmax,
                &epsilon,
                &mut alpha,
                &mut coefficient_a,
                &mut coefficient_b,
            );
            let coefficients: Vec<_> = (m..=lmax)
                .map(|degree| {
                    Complex64::new(
                        (0.31 * degree as f64 + 0.73 * m as f64).sin(),
                        (0.19 * degree as f64 - 0.41 * m as f64).cos(),
                    )
                })
                .collect();
            let pair_count = (lmax - m + 2) / 2;
            let mut coefficient_even = vec![Complex64::new(0.0, 0.0); pair_count];
            let mut coefficient_odd = vec![Complex64::new(0.0, 0.0); pair_count];

            assert!(prepare_synthesis_coefficients(
                m,
                lmax,
                &epsilon,
                &alpha,
                &coefficients,
                &mut coefficient_even,
                &mut coefficient_odd,
            ));

            for pair in 0..pair_count {
                let degree = m + 2 * pair;
                let offset = degree - m;
                let next_even = if degree + 2 <= lmax {
                    coefficients[offset + 2]
                } else {
                    Complex64::new(0.0, 0.0)
                };
                let expected_even = (coefficients[offset] * epsilon[degree + 1]
                    + next_even * epsilon[degree + 2])
                    * alpha[pair];
                let expected_odd = if degree < lmax {
                    coefficients[offset + 1] * alpha[pair]
                } else {
                    Complex64::new(0.0, 0.0)
                };
                assert_eq!(coefficient_even[pair], expected_even);
                assert_eq!(coefficient_odd[pair], expected_odd);
            }
        }
    }
}

#[test]
fn paired_synthesis_setup_uses_native_f32_and_rejects_overflow() {
    use num_complex::Complex32;

    for m in 0..=4 {
        for lmax in m..=m + 9 {
            let mut epsilon = vec![0.0_f32; lmax + 4];
            crate::legendre::numeric::fill_epsilon(m, lmax, &mut epsilon);
            let factor_count = lmax / 2 + 2;
            let mut alpha = vec![0.0_f32; factor_count];
            let mut coefficient_a = vec![0.0_f32; factor_count];
            let mut coefficient_b = vec![0.0_f32; factor_count];
            assert!(fill_factors_typed(
                m,
                lmax,
                &epsilon,
                &mut alpha,
                &mut coefficient_a,
                &mut coefficient_b,
                f32::MAX,
            ));

            let mut expected_alpha = vec![0.0_f32; factor_count];
            expected_alpha[0] = 1.0_f32 / epsilon[m + 1];
            expected_alpha[1] = epsilon[m + 1] / (epsilon[m + 2] * epsilon[m + 3]);
            for (index, degree) in (m + 2..lmax + 1).step_by(2).enumerate() {
                let alpha_index = index + 1;
                let sign = if alpha_index.is_multiple_of(2) {
                    1.0_f32
                } else {
                    -1.0_f32
                };
                expected_alpha[alpha_index + 1] = sign
                    / (epsilon[degree + 2] * epsilon[degree + 3] * expected_alpha[alpha_index]);
            }
            let mut expected_coefficient_a = vec![0.0_f32; factor_count];
            let mut expected_coefficient_b = vec![0.0_f32; factor_count];
            for (index, degree) in (m..lmax + 2).step_by(2).enumerate() {
                let sign = if index.is_multiple_of(2) {
                    1.0_f32
                } else {
                    -1.0_f32
                };
                expected_coefficient_a[index] =
                    sign * expected_alpha[index] * expected_alpha[index];
                expected_coefficient_b[index] = -expected_coefficient_a[index]
                    * (epsilon[degree + 2] * epsilon[degree + 2]
                        + epsilon[degree + 1] * epsilon[degree + 1]);
            }
            assert_eq!(alpha, expected_alpha);
            assert_eq!(coefficient_a, expected_coefficient_a);
            assert_eq!(coefficient_b, expected_coefficient_b);

            let first: Vec<_> = (m..=lmax)
                .map(|degree| {
                    Complex32::new(
                        (0.31_f32 * degree as f32 + 0.73_f32 * m as f32).sin(),
                        (0.19_f32 * degree as f32 - 0.41_f32 * m as f32).cos(),
                    )
                })
                .collect();
            let second: Vec<_> = (m..=lmax)
                .map(|degree| {
                    Complex32::new(
                        (0.47_f32 * degree as f32 - 0.29_f32 * m as f32).cos(),
                        (0.13_f32 * degree as f32 + 0.67_f32 * m as f32).sin(),
                    )
                })
                .collect();
            let pair_count = (lmax - m + 2) / 2;
            let mut first_even = vec![Complex32::new(0.0, 0.0); pair_count];
            let mut first_odd = first_even.clone();
            let mut second_even = first_even.clone();
            let mut second_odd = first_even.clone();
            let mut expected_first_even = first_even.clone();
            let mut expected_first_odd = first_even.clone();
            let mut expected_second_even = first_even.clone();
            let mut expected_second_odd = first_even.clone();

            assert!(prepare_synthesis_coefficient_pair(
                m,
                lmax,
                &epsilon,
                &alpha,
                &first,
                &second,
                &mut first_even,
                &mut first_odd,
                &mut second_even,
                &mut second_odd,
            ));
            assert!(prepare_synthesis_coefficients(
                m,
                lmax,
                &epsilon,
                &alpha,
                &first,
                &mut expected_first_even,
                &mut expected_first_odd,
            ));
            assert!(prepare_synthesis_coefficients(
                m,
                lmax,
                &epsilon,
                &alpha,
                &second,
                &mut expected_second_even,
                &mut expected_second_odd,
            ));
            assert_eq!(first_even, expected_first_even);
            assert_eq!(first_odd, expected_first_odd);
            assert_eq!(second_even, expected_second_even);
            assert_eq!(second_odd, expected_second_odd);

            for pair in 0..pair_count {
                let degree = m + 2 * pair;
                let offset = degree - m;
                let next_even = if degree + 2 <= lmax {
                    first[offset + 2]
                } else {
                    Complex32::new(0.0, 0.0)
                };
                let expected_even = (first[offset] * epsilon[degree + 1]
                    + next_even * epsilon[degree + 2])
                    * alpha[pair];
                let expected_odd = if degree < lmax {
                    first[offset + 1] * alpha[pair]
                } else {
                    Complex32::new(0.0, 0.0)
                };
                assert_eq!(first_even[pair], expected_even);
                assert_eq!(first_odd[pair], expected_odd);
            }
        }
    }

    let (m, lmax) = (0, 1);
    let mut epsilon = vec![0.0_f32; lmax + 4];
    crate::legendre::numeric::fill_epsilon(m, lmax, &mut epsilon);
    let mut alpha = [0.0_f32; 2];
    let mut coefficient_a = [0.0_f32; 2];
    let mut coefficient_b = [0.0_f32; 2];
    assert!(fill_factors_typed(
        m,
        lmax,
        &epsilon,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
        f32::MAX,
    ));
    let coefficients = [Complex32::new(0.0, 0.0), Complex32::new(f32::MAX, 0.0)];
    let mut even = [Complex32::new(0.0, 0.0)];
    let mut odd = [Complex32::new(0.0, 0.0)];
    assert!(!prepare_synthesis_coefficients(
        m,
        lmax,
        &epsilon,
        &alpha,
        &coefficients,
        &mut even,
        &mut odd,
    ));
}

#[test]
fn paired_spin_coefficient_preparation_matches_independent_rows() {
    for m in 0..=4 {
        for lmax in m..=m + 9 {
            let mut epsilon = vec![0.0; lmax + 4];
            scalar::fill_epsilon(m, lmax, &mut epsilon);
            let factor_count = lmax / 2 + 2;
            let mut alpha = vec![0.0; factor_count];
            let mut coefficient_a = vec![0.0; factor_count];
            let mut coefficient_b = vec![0.0; factor_count];
            assert!(fill_factors(
                m,
                lmax,
                &epsilon,
                &mut alpha,
                &mut coefficient_a,
                &mut coefficient_b,
            ));
            let first: Vec<_> = (m..=lmax)
                .map(|degree| {
                    Complex64::new(
                        (0.31 * degree as f64 + 0.73 * m as f64).sin(),
                        (0.19 * degree as f64 - 0.41 * m as f64).cos(),
                    )
                })
                .collect();
            let second: Vec<_> = (m..=lmax)
                .map(|degree| {
                    Complex64::new(
                        (0.47 * degree as f64 - 0.29 * m as f64).cos(),
                        (0.13 * degree as f64 + 0.67 * m as f64).sin(),
                    )
                })
                .collect();
            let pair_count = (lmax - m + 2) / 2;
            let mut first_even = vec![Complex64::new(0.0, 0.0); pair_count];
            let mut first_odd = vec![Complex64::new(0.0, 0.0); pair_count];
            let mut second_even = vec![Complex64::new(0.0, 0.0); pair_count];
            let mut second_odd = vec![Complex64::new(0.0, 0.0); pair_count];
            let mut expected_first_even = first_even.clone();
            let mut expected_first_odd = first_odd.clone();
            let mut expected_second_even = second_even.clone();
            let mut expected_second_odd = second_odd.clone();

            assert!(prepare_synthesis_coefficient_pair(
                m,
                lmax,
                &epsilon,
                &alpha,
                &first,
                &second,
                &mut first_even,
                &mut first_odd,
                &mut second_even,
                &mut second_odd,
            ));
            assert!(prepare_synthesis_coefficients(
                m,
                lmax,
                &epsilon,
                &alpha,
                &first,
                &mut expected_first_even,
                &mut expected_first_odd,
            ));
            assert!(prepare_synthesis_coefficients(
                m,
                lmax,
                &epsilon,
                &alpha,
                &second,
                &mut expected_second_even,
                &mut expected_second_odd,
            ));
            assert_eq!(first_even, expected_first_even);
            assert_eq!(first_odd, expected_first_odd);
            assert_eq!(second_even, expected_second_even);
            assert_eq!(second_odd, expected_second_odd);
        }
    }
}

#[test]
fn paired_spin_coefficient_preparation_rejects_nonfinite_second_row() {
    let (m, lmax) = (0, 1);
    let mut epsilon = vec![0.0; lmax + 4];
    scalar::fill_epsilon(m, lmax, &mut epsilon);
    let alpha = [2.0];
    let first = [Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)];
    let second = [Complex64::new(0.0, 0.0), Complex64::new(f64::MAX, 0.0)];
    let mut first_even = [Complex64::new(0.0, 0.0)];
    let mut first_odd = [Complex64::new(0.0, 0.0)];
    let mut second_even = [Complex64::new(0.0, 0.0)];
    let mut second_odd = [Complex64::new(0.0, 0.0)];

    assert!(!prepare_synthesis_coefficient_pair(
        m,
        lmax,
        &epsilon,
        &alpha,
        &first,
        &second,
        &mut first_even,
        &mut first_odd,
        &mut second_even,
        &mut second_odd,
    ));
}

#[test]
fn synthesis_coefficient_preparation_rejects_nonfinite_transformed_pairs() {
    let (m, lmax) = (0, 1);
    let mut epsilon = vec![0.0; lmax + 4];
    scalar::fill_epsilon(m, lmax, &mut epsilon);
    let factor_count = lmax / 2 + 2;
    let mut alpha = vec![0.0; factor_count];
    let mut coefficient_a = vec![0.0; factor_count];
    let mut coefficient_b = vec![0.0; factor_count];
    fill_factors(
        m,
        lmax,
        &epsilon,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    );

    for input in [1.0, f64::from_bits(1)] {
        let coefficients = [Complex64::new(0.0, 0.0), Complex64::new(input, 0.0)];
        let mut even = [Complex64::new(0.0, 0.0)];
        let mut odd = [Complex64::new(0.0, 0.0)];
        assert!(prepare_synthesis_coefficients(
            m,
            lmax,
            &epsilon,
            &alpha,
            &coefficients,
            &mut even,
            &mut odd,
        ));
        assert!(even[0].re.is_finite() && odd[0].re.is_finite());
    }

    let coefficients = [Complex64::new(0.0, 0.0), Complex64::new(1.1e308, 0.0)];
    let mut even = [Complex64::new(0.0, 0.0)];
    let mut odd = [Complex64::new(0.0, 0.0)];
    assert!(!prepare_synthesis_coefficients(
        m,
        lmax,
        &epsilon,
        &alpha,
        &coefficients,
        &mut even,
        &mut odd,
    ));
    assert_eq!(coefficients[1], Complex64::new(1.1e308, 0.0));
}

#[test]
fn partitioned_synthesis_matches_scaled_reference_for_mixed_initial_lanes() {
    let m = 32;
    let lmax = 63;
    let sine: [f64; 7] = [0.99, 0.95, 0.90, 0.85, 0.80, 1.0e-10, 3.0e-10];
    let sin_theta = sine.to_vec();
    let cos_theta: Vec<_> = sine
        .iter()
        .map(|value| (1.0 - value * value).sqrt())
        .collect();
    let ring_order_limits = vec![lmax; sine.len()];
    let sectoral_norm = scalar::sectoral_normalizations(lmax).unwrap()[m];
    assert_eq!(
        scalar::sectoral_value(sine[0], m, sectoral_norm).unwrap().1,
        0
    );
    assert_ne!(
        scalar::sectoral_value(sine[5], m, sectoral_norm).unwrap().1,
        0
    );
    let mut epsilon = vec![0.0; lmax + 4];
    scalar::fill_epsilon(m, lmax, &mut epsilon);
    let factor_count = lmax / 2 + 2;
    let mut alpha = vec![0.0; factor_count];
    let mut coefficient_a = vec![0.0; factor_count];
    let mut coefficient_b = vec![0.0; factor_count];
    fill_factors(
        m,
        lmax,
        &epsilon,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    );
    let coefficients: Vec<_> = (m..=lmax)
        .map(|degree| Complex64::new((0.31 * degree as f64).sin(), (0.17 * degree as f64).cos()))
        .collect();
    let pair_count = (lmax - m + 2) / 2;
    let mut coefficient_even = vec![Complex64::new(0.0, 0.0); pair_count];
    let mut coefficient_odd = vec![Complex64::new(0.0, 0.0); pair_count];
    assert!(prepare_synthesis_coefficients(
        m,
        lmax,
        &epsilon,
        &alpha,
        &coefficients,
        &mut coefficient_even,
        &mut coefficient_odd,
    ));

    let mut actual = vec![Complex64::new(0.0, 0.0); sine.len() * (lmax + 1)];
    let mut previous = vec![0.0; sine.len()];
    let mut current = vec![0.0; sine.len()];
    let mut next = vec![0.0; sine.len()];
    let mut exponents = vec![0; sine.len()];
    let mut compact_theta_sin = vec![0.0; sine.len()];
    let mut compact_theta_cos = vec![0.0; sine.len()];
    let mut compact_ring_indices = vec![0; sine.len()];
    let used = synthesize_order_block_partitioned_dispatched(
        Arch::new(),
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
        lmax + 1,
        &mut actual,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
        &mut compact_theta_sin,
        &mut compact_theta_cos,
        &mut compact_ring_indices,
    )
    .unwrap();
    assert!(used);

    let mut expected = vec![Complex64::new(0.0, 0.0); actual.len()];
    for_each_y_lm_paired_dispatched(
        Arch::new(),
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
        |l, local_ring_start, basis, lane_exponents| {
            let pair = (l - m) / 2;
            for (lane, &value) in basis.iter().enumerate() {
                let ring = local_ring_start + lane;
                expected[ring * (lmax + 1) + m] += scalar::multiply_complex_scaled(
                    coefficient_even[pair],
                    value,
                    lane_exponents[lane],
                ) + scalar::multiply_complex_scaled_product(
                    coefficient_odd[pair],
                    value,
                    cos_theta[ring],
                    lane_exponents[lane],
                );
            }
            Ok(())
        },
    )
    .unwrap();

    for ring in 0..sine.len() {
        let difference = (actual[ring * (lmax + 1) + m] - expected[ring * (lmax + 1) + m]).norm();
        assert!(difference < 3.0e-11, "ring={ring} error={difference:.3e}");
    }
}

#[test]
fn fused_synthesis_continuation_declines_on_late_tail_range_failure() {
    let lane_count = 5;
    let cosine = vec![0.5; lane_count];
    let epsilon = vec![1.0; 8];
    let alpha = vec![1.0; 4];
    let coefficient_a = [12.0, 0.0, 0.0, 0.0];
    let coefficient_b = [0.0; 4];
    let first_coefficients = [Complex64::new(1.0, 0.5); 4];
    let second_coefficients = [Complex64::new(-0.25, 0.75); 4];
    let previous = [0.0; 5];
    let current = [1.0, 1.0, 1.0, 1.0, scalar::BIG * 0.75];
    let mut first_sum_re = [10.0; 5];
    let mut first_sum_im = [20.0; 5];
    let mut second_sum_re = [30.0; 5];
    let mut second_sum_im = [40.0; 5];

    let completed = Arch::new().dispatch(FusedSynthesisPairBlock {
        m: 0,
        lmax: 3,
        pair_start: 0,
        cos_theta: &cosine,
        coefficient_a: &coefficient_a,
        coefficient_b: &coefficient_b,
        coefficients: OriginalSynthesisPairCoefficients {
            epsilon: &epsilon,
            alpha: &alpha,
            first_coefficients: &first_coefficients,
            second_coefficients: &second_coefficients,
        },
        previous: &previous,
        current: &current,
        first_sum_re: &mut first_sum_re,
        first_sum_im: &mut first_sum_im,
        second_sum_re: &mut second_sum_re,
        second_sum_im: &mut second_sum_im,
        initialize_sums: false,
        check_normal_range: true,
    });

    assert!(!completed);
    assert_ne!(first_sum_re[0], 10.0);
    assert_ne!(second_sum_im[3], 40.0);
    assert_eq!(first_sum_re[4], 10.0);
    assert_eq!(first_sum_im[4], 20.0);
    assert_eq!(second_sum_re[4], 30.0);
    assert_eq!(second_sum_im[4], 40.0);
}

#[test]
fn prepared_order_pair_transition_matches_original_coefficients() {
    let m = 0;
    let lmax = 3;
    let pair_count = (lmax - m + 2) / 2;
    let sin_theta = [0.2, 0.7, 0.98];
    let cos_theta = sin_theta.map(|sine| (1.0 - sine * sine).sqrt());
    let sectoral_norm = 0.75;
    let epsilon = [1.0, 0.9, 1.1, 0.8, 1.2, 1.0];
    let alpha = [0.95, 1.05];
    let coefficient_a = [0.15, 0.1];
    let coefficient_b = [0.2, -0.05];
    let first_coefficients = [
        Complex64::new(0.25, -0.125),
        Complex64::new(0.5, 0.375),
        Complex64::new(-0.75, 0.25),
        Complex64::new(0.125, 0.625),
    ];
    let second_coefficients = [
        Complex64::new(-0.125, 0.5),
        Complex64::new(0.75, -0.25),
        Complex64::new(0.375, 0.125),
        Complex64::new(-0.5, 0.25),
    ];
    let mut first_even = [Complex64::new(0.0, 0.0); 2];
    let mut first_odd = [Complex64::new(0.0, 0.0); 2];
    let mut second_even = [Complex64::new(0.0, 0.0); 2];
    let mut second_odd = [Complex64::new(0.0, 0.0); 2];
    assert!(prepare_synthesis_coefficient_pair(
        m,
        lmax,
        &epsilon,
        &alpha,
        &first_coefficients,
        &second_coefficients,
        &mut first_even,
        &mut first_odd,
        &mut second_even,
        &mut second_odd,
    ));

    let mut original_first_sum_re = [0.0; 3];
    let mut original_first_sum_im = [0.0; 3];
    let mut original_second_sum_re = [0.0; 3];
    let mut original_second_sum_im = [0.0; 3];
    let mut original_previous = [0.0; 3];
    let mut original_current = [0.0; 3];
    let mut original_next = [0.0; 3];
    let mut original_exponents = [0; 3];
    let original_completed = try_synthesize_order_pair_transition_dispatched(
        Arch::new(),
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &epsilon,
        &alpha,
        &coefficient_a,
        &coefficient_b,
        &first_coefficients,
        &second_coefficients,
        &mut original_previous,
        &mut original_current,
        &mut original_next,
        &mut original_exponents,
        &mut original_first_sum_re,
        &mut original_first_sum_im,
        &mut original_second_sum_re,
        &mut original_second_sum_im,
    )
    .unwrap();

    let mut prepared_first_sum_re = [0.0; 3];
    let mut prepared_first_sum_im = [0.0; 3];
    let mut prepared_second_sum_re = [0.0; 3];
    let mut prepared_second_sum_im = [0.0; 3];
    let mut prepared_previous = [0.0; 3];
    let mut prepared_current = [0.0; 3];
    let mut prepared_next = [0.0; 3];
    let mut prepared_exponents = [0; 3];
    let prepared_completed = try_synthesize_prepared_order_pair_transition_dispatched(
        Arch::new(),
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &first_even[..pair_count],
        &first_odd[..pair_count],
        &second_even[..pair_count],
        &second_odd[..pair_count],
        &mut prepared_previous,
        &mut prepared_current,
        &mut prepared_next,
        &mut prepared_exponents,
        &mut prepared_first_sum_re,
        &mut prepared_first_sum_im,
        &mut prepared_second_sum_re,
        &mut prepared_second_sum_im,
    )
    .unwrap();

    assert!(original_completed);
    assert_eq!(prepared_completed, original_completed);
    assert_eq!(prepared_first_sum_re, original_first_sum_re);
    assert_eq!(prepared_first_sum_im, original_first_sum_im);
    assert_eq!(prepared_second_sum_re, original_second_sum_re);
    assert_eq!(prepared_second_sum_im, original_second_sum_im);
}

#[test]
fn prepared_order_pair_transition_promotes_scaled_simd_vector() {
    let m = 100;
    let lmax = 106;
    let pair_count = (lmax - m + 2) / 2;
    let lanes = 9;
    let sin_theta = [0.01; 9];
    let cos_theta = sin_theta.map(|sine| (1.0 - sine * sine).sqrt());
    let sectoral_norm = 0.75;
    let epsilon = vec![1.0; lmax + 3];
    let alpha = vec![1.0; pair_count];
    let coefficient_a = vec![0.0; pair_count];
    let coefficient_b = vec![2.0_f64.powi(300); pair_count];
    let large_coefficient = 2.0_f64.powi(264);
    let first_coefficients = [Complex64::new(large_coefficient, large_coefficient); 7];
    let second_coefficients = [Complex64::new(-large_coefficient, large_coefficient * 0.5); 7];
    let mut first_even = vec![Complex64::new(0.0, 0.0); pair_count];
    let mut first_odd = vec![Complex64::new(0.0, 0.0); pair_count];
    let mut second_even = vec![Complex64::new(0.0, 0.0); pair_count];
    let mut second_odd = vec![Complex64::new(0.0, 0.0); pair_count];
    assert!(prepare_synthesis_coefficient_pair(
        m,
        lmax,
        &epsilon,
        &alpha,
        &first_coefficients,
        &second_coefficients,
        &mut first_even,
        &mut first_odd,
        &mut second_even,
        &mut second_odd,
    ));

    let mut original_first_sum_re = [0.0; 9];
    let mut original_first_sum_im = [0.0; 9];
    let mut original_second_sum_re = [0.0; 9];
    let mut original_second_sum_im = [0.0; 9];
    let mut original_previous = [0.0; 9];
    let mut original_current = [0.0; 9];
    let mut original_next = [0.0; 9];
    let mut original_exponents = [0; 9];
    let original_completed = try_synthesize_order_pair_transition_dispatched(
        Arch::new(),
        m,
        lmax,
        &sin_theta[..lanes],
        &cos_theta[..lanes],
        sectoral_norm,
        &epsilon,
        &alpha,
        &coefficient_a,
        &coefficient_b,
        &first_coefficients,
        &second_coefficients,
        &mut original_previous,
        &mut original_current,
        &mut original_next,
        &mut original_exponents,
        &mut original_first_sum_re,
        &mut original_first_sum_im,
        &mut original_second_sum_re,
        &mut original_second_sum_im,
    )
    .unwrap();

    let mut prepared_first_sum_re = [0.0; 9];
    let mut prepared_first_sum_im = [0.0; 9];
    let mut prepared_second_sum_re = [0.0; 9];
    let mut prepared_second_sum_im = [0.0; 9];
    let mut prepared_previous = [0.0; 9];
    let mut prepared_current = [0.0; 9];
    let mut prepared_next = [0.0; 9];
    let mut prepared_exponents = [0; 9];
    let prepared_completed = try_synthesize_prepared_order_pair_transition_dispatched(
        Arch::new(),
        m,
        lmax,
        &sin_theta[..lanes],
        &cos_theta[..lanes],
        sectoral_norm,
        &coefficient_a,
        &coefficient_b,
        &first_even,
        &first_odd,
        &second_even,
        &second_odd,
        &mut prepared_previous,
        &mut prepared_current,
        &mut prepared_next,
        &mut prepared_exponents,
        &mut prepared_first_sum_re,
        &mut prepared_first_sum_im,
        &mut prepared_second_sum_re,
        &mut prepared_second_sum_im,
    )
    .unwrap();

    assert!(
        original_completed,
        "fixture should reach the ordinary suffix"
    );
    assert_eq!(prepared_completed, original_completed);
    assert_eq!(prepared_exponents, original_exponents);
    assert_eq!(prepared_first_sum_re, original_first_sum_re);
    assert_eq!(prepared_first_sum_im, original_first_sum_im);
    assert_eq!(prepared_second_sum_re, original_second_sum_re);
    assert_eq!(prepared_second_sum_im, original_second_sum_im);
}

mod kernel_bench;

#[test]
fn materialize_scaled_native_analysis_seed_preserves_normal_bits_and_rejects_range_loss() {
    let positive = <f32 as FusedAnalysisPrecision>::materialize_scaled_seed(0.75, 4).unwrap();
    let negative = <f32 as FusedAnalysisPrecision>::materialize_scaled_seed(-0.75, 4).unwrap();
    assert_eq!(positive.to_bits(), 12.0_f32.to_bits());
    assert_eq!(negative.to_bits(), (-12.0_f32).to_bits());
    assert_eq!(
        <f32 as FusedAnalysisPrecision>::materialize_scaled_seed(0.0, i64::MAX)
            .unwrap()
            .to_bits(),
        0.0_f32.to_bits()
    );
    assert!(
        <f32 as FusedAnalysisPrecision>::materialize_scaled_seed(f32::MIN_POSITIVE, -1).is_none()
    );
    assert!(<f32 as FusedAnalysisPrecision>::materialize_scaled_seed(f32::MAX, 1).is_none());
}

#[test]
fn native_f32_spin_analysis_fused_scaled_seed_matches_checked_recurrence() {
    let m = 40;
    let lmax = 48;
    let pair_count = (lmax - m + 2) / 2;
    const LANE_COUNT: usize = 9;
    let sin_theta = [
        0.050, 0.061, 0.073, 0.084, 0.092, 0.104, 0.113, 0.121, 0.130,
    ];
    let cos_theta = sin_theta.map(|sine| (1.0_f32 - sine * sine).sqrt());
    let coefficient_a = vec![0.07_f32; pair_count];
    let coefficient_b = vec![0.35_f32; pair_count];
    let mut seed_previous = [0.0_f32; LANE_COUNT];
    let mut seed_current = [0.0_f32; LANE_COUNT];
    let mut seed_exponents = [0_i64; LANE_COUNT];
    <f32 as PairedScalarRecurrencePrecision>::initialize_pair_state(
        m,
        &sin_theta,
        0.75,
        &mut seed_previous,
        &mut seed_current,
        &mut seed_exponents,
    )
    .unwrap();
    assert!(seed_exponents.iter().any(|&exponent| exponent != 0));
    let same_real = [
        [
            0.25_f32, -0.5, 0.75, 0.125, -0.375, 0.625, -0.875, 0.45, -0.2,
        ],
        [
            -0.125_f32, 0.375, -0.625, 0.875, 0.25, -0.5, 0.75, -0.3, 0.55,
        ],
    ];
    let same_imag = [
        [-0.45_f32, 0.2, -0.35, 0.6, -0.15, 0.4, -0.7, 0.25, 0.5],
        [0.6_f32, -0.25, 0.5, -0.15, 0.35, -0.45, 0.2, -0.55, 0.3],
    ];
    let opposite_real = [
        [0.4_f32, -0.15, 0.3, -0.55, 0.7, -0.25, 0.5, -0.65, 0.1],
        [-0.35_f32, 0.65, -0.2, 0.45, -0.6, 0.15, -0.5, 0.75, -0.1],
    ];
    let opposite_imag = [
        [0.15_f32, -0.4, 0.55, -0.2, 0.35, -0.6, 0.25, -0.5, 0.7],
        [-0.55_f32, 0.3, -0.1, 0.65, -0.45, 0.2, -0.75, 0.4, -0.15],
    ];
    let mut staged_first = vec![Complex::new(0.0_f32, 0.0); lmax - m + 1];
    let mut staged_second = vec![Complex::new(0.0_f32, 0.0); lmax - m + 1];
    let mut previous = [0.0_f32; LANE_COUNT];
    let mut current = [0.0_f32; LANE_COUNT];
    let mut next = [0.0_f32; LANE_COUNT];
    let mut exponents = [0_i64; LANE_COUNT];
    let mut range_check_was_exercised = false;
    let success = fused_scalar_analysis_pair_channels_block_native::<f32>(
        Arch::new(),
        false,
        &mut range_check_was_exercised,
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        0.75,
        &coefficient_a,
        &coefficient_b,
        [&same_real[0], &same_real[1]],
        [&same_imag[0], &same_imag[1]],
        [&opposite_real[0], &opposite_real[1]],
        [&opposite_imag[0], &opposite_imag[1]],
        &mut staged_first,
        &mut staged_second,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
    )
    .unwrap();
    assert!(
        success,
        "scaled seed fixture should use the native fused route"
    );

    let mut expected_first = vec![Complex::new(0.0_f32, 0.0); lmax - m + 1];
    let mut expected_second = vec![Complex::new(0.0_f32, 0.0); lmax - m + 1];
    crate::legendre::scalar_paired::for_each_y_lm_paired_status_dispatched_typed::<f32, _>(
        Arch::new(),
        m,
        lmax,
        &sin_theta,
        &cos_theta,
        0.75,
        &coefficient_a,
        &coefficient_b,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
        |degree: usize, _: usize, basis: &[f32], exponents: &[i64], _: bool| {
            let offset = degree - m;
            for lane in 0..LANE_COUNT {
                let odd_basis = basis[lane] * cos_theta[lane];
                let channels = [
                    (
                        &same_real[0],
                        &same_imag[0],
                        &opposite_real[0],
                        &opposite_imag[0],
                    ),
                    (
                        &same_real[1],
                        &same_imag[1],
                        &opposite_real[1],
                        &opposite_imag[1],
                    ),
                ];
                for (channel, (same_re, same_im, opposite_re, opposite_im)) in
                    channels.into_iter().enumerate()
                {
                    let even = Complex::new(
                        scalar::multiply_complex_scaled_f32(
                            Complex::new(same_re[lane], 0.0),
                            basis[lane],
                            exponents[lane],
                        )
                        .re,
                        scalar::multiply_complex_scaled_f32(
                            Complex::new(same_im[lane], 0.0),
                            basis[lane],
                            exponents[lane],
                        )
                        .re,
                    );
                    let odd = Complex::new(
                        scalar::multiply_complex_scaled_f32(
                            Complex::new(opposite_re[lane], 0.0),
                            odd_basis,
                            exponents[lane],
                        )
                        .re,
                        scalar::multiply_complex_scaled_f32(
                            Complex::new(opposite_im[lane], 0.0),
                            odd_basis,
                            exponents[lane],
                        )
                        .re,
                    );
                    let target = if channel == 0 {
                        &mut expected_first
                    } else {
                        &mut expected_second
                    };
                    target[offset] += even;
                    if degree < lmax {
                        target[offset + 1] += odd;
                    }
                }
            }
            Ok(())
        },
    )
    .unwrap();

    for (actual, expected) in staged_first.iter().zip(expected_first) {
        assert!((actual.re - expected.re).abs() <= 3.0e-6);
        assert!((actual.im - expected.im).abs() <= 3.0e-6);
    }
    for (actual, expected) in staged_second.iter().zip(expected_second) {
        assert!((actual.re - expected.re).abs() <= 3.0e-6);
        assert!((actual.im - expected.im).abs() <= 3.0e-6);
    }
}
