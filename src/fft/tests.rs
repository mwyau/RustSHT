use num_complex::{Complex32, Complex64};
use rustfft::FftPlanner;

use super::*;

#[test]
fn length_two_extreme_finite_forward() {
    for input in [[1.0e308, 0.0], [0.0, 1.0e308]] {
        let plan = RealFftPlan::new(2).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut spectrum = [Complex64::new(0.0, 0.0); 2];
        plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
        assert_eq!(spectrum[0], Complex64::new(1.0e308, 0.0));
        assert_eq!(spectrum[1], Complex64::new(input[0] - input[1], 0.0));
    }
}

#[test]
fn length_two_extreme_finite_inverse() {
    let plan = RealFftPlan::new(2).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let spectrum = [Complex64::new(1.0e308, 0.0); 2];
    let mut output = [0.0; 2];
    plan.inverse(&spectrum, &mut output, &mut workspace)
        .unwrap();
    assert_eq!(output, [1.0e308, 0.0]);
}

#[test]
fn length_two_extreme_finite_f32() {
    let plan = RealFftPlan::<f32>::new(2).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let input = [2.0e38, 0.0];
    let mut spectrum = [Complex32::new(0.0, 0.0); 2];
    plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
    assert_eq!(spectrum, [Complex32::new(2.0e38, 0.0); 2]);
    let mut output = [0.0; 2];
    plan.inverse(
        &[Complex32::new(2.0e38, 0.0); 2],
        &mut output,
        &mut workspace,
    )
    .unwrap();
    assert_eq!(output, input);
}

fn close_complex(actual: Complex64, expected: Complex64, tolerance: f64) {
    let error = (actual - expected).norm();
    assert!(
        error <= tolerance * expected.norm().max(1.0),
        "actual={actual:?}, expected={expected:?}, error={error:.3e}"
    );
}

fn sample_rows(n: usize) -> Vec<Vec<f64>> {
    let mut rows = Vec::new();
    rows.push((0..n).map(|i| if i == n / 3 { 1.0 } else { 0.0 }).collect());
    rows.push(vec![1.25; n]);
    rows.push(
        (0..n)
            .map(|i| (0.37 * i as f64).sin() + (0.61 * i as f64).cos())
            .collect(),
    );
    for k in 1..=n / 2 {
        rows.push(
            (0..n)
                .map(|i| (2.0 * PI * k as f64 * i as f64 / n as f64).cos())
                .collect(),
        );
    }
    rows
}

fn full_complex_forward(input: &[f64]) -> Vec<Complex64> {
    let n = input.len();
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(n);
    let mut values: Vec<_> = input.iter().map(|&x| Complex64::new(x, 0.0)).collect();
    let mut scratch = vec![Complex64::new(0.0, 0.0); fft.get_inplace_scratch_len()];
    fft.process_with_scratch(&mut values, &mut scratch);
    values
}

#[test]
fn real_fft_modes_match_full_complex_rustfft() {
    for n in [1, 2, 3, 4, 5, 6, 7, 8, 31, 32, 33] {
        let plan = RealFftPlan::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        for input in sample_rows(n) {
            let mut actual = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            plan.forward(&input, &mut actual, &mut workspace).unwrap();
            let expected = full_complex_forward(&input);
            for k in 0..plan.spectrum_len() {
                close_complex(actual[k], expected[k], 4e-13);
            }
            assert_eq!(actual[0].im, 0.0);
            if n % 2 == 0 {
                assert_eq!(actual[n / 2].im, 0.0);
            }
        }
    }
}

#[test]
fn paired_odd_fft_matches_independent_rows_and_guards_unsafe_inputs() {
    for n in [129, 257, 361, 509, 513] {
        let plan = RealFftPlan::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let first: Vec<_> = (0..n)
            .map(|j| 8.0 * ((0.37 * j as f64).sin() + (0.61 * j as f64).cos()))
            .collect();
        let second: Vec<_> = (0..n)
            .map(|j| (0.37 * j as f64).sin() + (0.61 * j as f64).cos())
            .collect();
        let mut first_expected = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        let mut second_expected = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        plan.forward(&first, &mut first_expected, &mut workspace)
            .unwrap();
        plan.forward(&second, &mut second_expected, &mut workspace)
            .unwrap();
        let mut first_actual = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        let mut second_actual = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        assert!(
            plan.forward_paired_truncated(
                &first,
                &second,
                &mut first_actual,
                &mut second_actual,
                &mut workspace,
            )
            .unwrap()
        );
        for (actual, expected) in first_actual.iter().zip(&first_expected) {
            close_complex(*actual, *expected, 4e-13);
        }
        for (actual, expected) in second_actual.iter().zip(&second_expected) {
            close_complex(*actual, *expected, 4e-13);
        }

        let mut first_inverse_expected = vec![0.0; n];
        let mut second_inverse_expected = vec![0.0; n];
        plan.inverse_series(&first_expected, &mut first_inverse_expected, &mut workspace)
            .unwrap();
        plan.inverse_series(
            &second_expected,
            &mut second_inverse_expected,
            &mut workspace,
        )
        .unwrap();

        let opposite: Vec<_> = second.iter().map(|value| -*value).collect();
        assert!(
            plan.forward_paired_truncated(
                &second,
                &opposite,
                &mut first_actual,
                &mut second_actual,
                &mut workspace,
            )
            .unwrap()
        );
        for (actual, expected) in first_actual.iter().zip(&second_expected) {
            close_complex(*actual, *expected, 4e-13);
        }
        for (actual, expected) in second_actual.iter().zip(&second_expected) {
            close_complex(*actual, -*expected, 4e-13);
        }

        let mut first_reconstructed = vec![0.0; n];
        let mut second_reconstructed = vec![0.0; n];
        assert!(
            plan.inverse_paired_series(
                &first_expected,
                &second_expected,
                &mut first_reconstructed,
                &mut second_reconstructed,
                &mut workspace,
            )
            .unwrap()
        );
        let relative_l2 = |actual: &[f64], expected: &[f64]| {
            let (error, norm) = actual.iter().zip(expected).fold(
                (0.0, 0.0),
                |(error, norm), (&actual, &expected)| {
                    (error + (actual - expected).powi(2), norm + expected.powi(2))
                },
            );
            (error / norm).sqrt()
        };
        assert!(
            relative_l2(&first_reconstructed, &first_inverse_expected) <= 4e-13,
            "N={n}, first inverse relative L2={:.3e}",
            relative_l2(&first_reconstructed, &first_inverse_expected)
        );
        assert!(
            relative_l2(&second_reconstructed, &second_inverse_expected) <= 4e-13,
            "N={n}, second inverse relative L2={:.3e}",
            relative_l2(&second_reconstructed, &second_inverse_expected)
        );

        let zero = vec![0.0; n];
        assert!(
            plan.forward_paired_truncated(
                &zero,
                &zero,
                &mut first_actual,
                &mut second_actual,
                &mut workspace,
            )
            .unwrap()
        );
        assert!(
            !plan
                .forward_paired_truncated(
                    &first,
                    &zero,
                    &mut first_actual,
                    &mut second_actual,
                    &mut workspace,
                )
                .unwrap()
        );

        let zero_spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        let mut zero_output = vec![0.0; n];
        assert!(
            plan.inverse_paired_series(
                &zero_spectrum,
                &zero_spectrum,
                &mut zero_output,
                &mut first_reconstructed,
                &mut workspace,
            )
            .unwrap()
        );
        assert!(
            !plan
                .inverse_paired_series(
                    &first_expected,
                    &zero_spectrum,
                    &mut first_reconstructed,
                    &mut second_reconstructed,
                    &mut workspace,
                )
                .unwrap()
        );
        let very_unequal_spectrum: Vec<_> = first_expected.iter().map(|v| *v * 1.0e4).collect();
        assert!(
            !plan
                .inverse_paired_series(
                    &very_unequal_spectrum,
                    &second_expected,
                    &mut first_reconstructed,
                    &mut second_reconstructed,
                    &mut workspace,
                )
                .unwrap()
        );
        let mut extreme_spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        extreme_spectrum[1] = Complex64::new(f64::MAX / 2.0, 0.0);
        assert!(
            !plan
                .inverse_paired_series(
                    &extreme_spectrum,
                    &extreme_spectrum,
                    &mut first_reconstructed,
                    &mut second_reconstructed,
                    &mut workspace,
                )
                .unwrap()
        );
        let very_unequal: Vec<_> = second.iter().map(|value| value * 1.0e4).collect();
        assert!(
            !plan
                .forward_paired_truncated(
                    &very_unequal,
                    &second,
                    &mut first_actual,
                    &mut second_actual,
                    &mut workspace,
                )
                .unwrap()
        );
    }

    let plan = RealFftPlan::new(129).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let normal = vec![1.0; 129];
    let mut exceptional = normal.clone();
    exceptional[0] = f64::NAN;
    let mut first_spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
    let mut second_spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
    assert!(
        !plan
            .forward_paired_truncated(
                &exceptional,
                &normal,
                &mut first_spectrum,
                &mut second_spectrum,
                &mut workspace,
            )
            .unwrap()
    );
    exceptional[0] = f64::from_bits(1);
    assert!(
        !plan
            .forward_paired_truncated(
                &exceptional,
                &normal,
                &mut first_spectrum,
                &mut second_spectrum,
                &mut workspace,
            )
            .unwrap()
    );
    exceptional[0] = f64::MAX / 2.0;
    assert!(
        !plan
            .forward_paired_truncated(
                &exceptional,
                &normal,
                &mut first_spectrum,
                &mut second_spectrum,
                &mut workspace,
            )
            .unwrap()
    );
}

#[test]
fn paired_odd_fft_matches_equal_peak_rows_with_distinct_spectral_support() {
    for n in [129, 257, 361, 509] {
        let plan = RealFftPlan::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let first: Vec<_> = (0..n)
            .map(|j| (2.0 * PI * 3.0 * j as f64 / n as f64).sin())
            .collect();
        let high_frequency = n / 2 - 2;
        let second: Vec<_> = (0..n)
            .map(|j| 0.8 * (2.0 * PI * high_frequency as f64 * j as f64 / n as f64 + 0.17).cos())
            .collect();
        let mut first_expected = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        let mut second_expected = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        plan.forward(&first, &mut first_expected, &mut workspace)
            .unwrap();
        plan.forward(&second, &mut second_expected, &mut workspace)
            .unwrap();

        let mut first_actual = vec![Complex64::new(f64::NAN, f64::INFINITY); plan.spectrum_len()];
        let mut second_actual = first_actual.clone();
        assert!(
            plan.forward_paired_truncated(
                &first,
                &second,
                &mut first_actual,
                &mut second_actual,
                &mut workspace,
            )
            .unwrap(),
            "equal peak input rows should pass the empirical amplitude guard at N={n}"
        );
        for (actual, expected) in first_actual.iter().zip(&first_expected) {
            close_complex(*actual, *expected, 4e-13);
        }
        for (actual, expected) in second_actual.iter().zip(&second_expected) {
            close_complex(*actual, *expected, 4e-13);
        }

        let mut first_inverse_expected = vec![0.0; n];
        let mut second_inverse_expected = vec![0.0; n];
        plan.inverse_series(&first_expected, &mut first_inverse_expected, &mut workspace)
            .unwrap();
        plan.inverse_series(
            &second_expected,
            &mut second_inverse_expected,
            &mut workspace,
        )
        .unwrap();
        let mut first_inverse_actual = vec![f64::NAN; n];
        let mut second_inverse_actual = vec![f64::NAN; n];
        assert!(
            plan.inverse_paired_series(
                &first_expected,
                &second_expected,
                &mut first_inverse_actual,
                &mut second_inverse_actual,
                &mut workspace,
            )
            .unwrap()
        );
        for (&actual, &expected) in first_inverse_actual.iter().zip(&first_inverse_expected) {
            assert!((actual - expected).abs() <= 4e-13 * expected.abs().max(1.0));
        }
        for (&actual, &expected) in second_inverse_actual.iter().zip(&second_inverse_expected) {
            assert!((actual - expected).abs() <= 4e-13 * expected.abs().max(1.0));
        }
    }
}

#[test]
fn paired_odd_fft_reuses_poisoned_workspace() {
    for n in [129, 361, 509] {
        let plan = RealFftPlan::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut reference_workspace = plan.workspace().unwrap();
        let first: Vec<_> = (0..n)
            .map(|j| (0.31 * j as f64).sin() + 0.5 * (0.17 * j as f64).cos())
            .collect();
        let second: Vec<_> = (0..n)
            .map(|j| 0.75 * ((0.23 * j as f64).cos() - 0.2 * (0.41 * j as f64).sin()))
            .collect();
        let mut first_spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        let mut second_spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        plan.forward(&first, &mut first_spectrum, &mut reference_workspace)
            .unwrap();
        plan.forward(&second, &mut second_spectrum, &mut reference_workspace)
            .unwrap();
        let mut first_expected = vec![0.0; n];
        let mut second_expected = vec![0.0; n];
        plan.inverse_series(
            &first_spectrum,
            &mut first_expected,
            &mut reference_workspace,
        )
        .unwrap();
        plan.inverse_series(
            &second_spectrum,
            &mut second_expected,
            &mut reference_workspace,
        )
        .unwrap();

        for _ in 0..3 {
            workspace
                .transform
                .fill(Complex64::new(f64::NAN, f64::INFINITY));
            workspace
                .scratch
                .fill(Complex64::new(f64::NAN, f64::NEG_INFINITY));
            let mut first_actual = vec![Complex64::new(17.0, -29.0); plan.spectrum_len()];
            let mut second_actual = vec![Complex64::new(17.0, -29.0); plan.spectrum_len()];
            assert!(
                plan.forward_paired_truncated(
                    &first,
                    &second,
                    &mut first_actual,
                    &mut second_actual,
                    &mut workspace,
                )
                .unwrap()
            );
            for (actual, expected) in first_actual.iter().zip(&first_spectrum) {
                close_complex(*actual, *expected, 4e-13);
            }
            for (actual, expected) in second_actual.iter().zip(&second_spectrum) {
                close_complex(*actual, *expected, 4e-13);
            }

            workspace
                .transform
                .fill(Complex64::new(f64::NAN, f64::INFINITY));
            workspace
                .scratch
                .fill(Complex64::new(f64::NAN, f64::NEG_INFINITY));
            let mut first_output = vec![f64::NAN; n];
            let mut second_output = vec![f64::NAN; n];
            assert!(
                plan.inverse_paired_series(
                    &first_spectrum,
                    &second_spectrum,
                    &mut first_output,
                    &mut second_output,
                    &mut workspace,
                )
                .unwrap()
            );
            for (actual, expected) in first_output.iter().zip(&first_expected) {
                assert!((actual - expected).abs() <= 4e-13 * expected.abs().max(1.0));
            }
            for (actual, expected) in second_output.iter().zip(&second_expected) {
                assert!((actual - expected).abs() <= 4e-13 * expected.abs().max(1.0));
            }
        }
    }
}

#[test]
fn truncated_real_fft_matches_full_spectrum_prefix() {
    for n in [1, 2, 3, 4, 5, 6, 7, 8, 31, 32, 33] {
        let plan = RealFftPlan::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut inputs = sample_rows(n);
        inputs.push(vec![f64::from_bits(1); n]);
        inputs.push(
            (0..n)
                .map(|index| {
                    if index.is_multiple_of(2) {
                        1.0e307
                    } else {
                        -1.0e307
                    }
                })
                .collect(),
        );

        for input in inputs {
            let mut full = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            plan.forward(&input, &mut full, &mut workspace).unwrap();

            let spectrum_len = plan.spectrum_len();
            let mut mode_counts = vec![1, spectrum_len.div_ceil(2), spectrum_len];
            mode_counts.sort_unstable();
            mode_counts.dedup();
            for mode_count in mode_counts {
                let sentinel = Complex64::new(17.0, -29.0);
                let mut truncated = vec![sentinel; mode_count];
                plan.forward_truncated(&input, &mut truncated, &mut workspace)
                    .unwrap();
                assert_eq!(truncated, full[..mode_count], "n={n}, modes={mode_count}");
            }
        }
    }
}

#[test]
fn truncated_real_fft_rejects_invalid_mode_counts() {
    let plan = RealFftPlan::new(8).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let input = [0.0; 8];
    let mut empty = [];
    assert!(
        plan.forward_truncated(&input, &mut empty, &mut workspace)
            .is_err()
    );
    let mut too_many = vec![Complex64::new(0.0, 0.0); plan.spectrum_len() + 1];
    assert!(
        plan.forward_truncated(&input, &mut too_many, &mut workspace)
            .is_err()
    );
}

#[test]
fn f32_real_fft_matches_full_complex_rustfft() {
    for n in [1, 2, 3, 4, 5, 8, 31, 32, 33] {
        let plan = RealFftPlan::<f32>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        for input in sample_rows(n) {
            let input: Vec<f32> = input.into_iter().map(|value| value as f32).collect();
            let mut actual = vec![Complex32::new(0.0, 0.0); plan.spectrum_len()];
            plan.forward(&input, &mut actual, &mut workspace).unwrap();

            let mut planner = FftPlanner::<f32>::new();
            let fft = planner.plan_fft_forward(n);
            let mut expected: Vec<_> = input
                .iter()
                .map(|&value| Complex32::new(value, 0.0))
                .collect();
            let mut scratch = vec![Complex32::new(0.0, 0.0); fft.get_inplace_scratch_len()];
            fft.process_with_scratch(&mut expected, &mut scratch);
            for k in 0..plan.spectrum_len() {
                let error = (actual[k] - expected[k]).norm();
                assert!(
                    error <= 8e-5 * expected[k].norm().max(1.0),
                    "n={n}, k={k}, actual={:?}, expected={:?}, error={error:.3e}",
                    actual[k],
                    expected[k]
                );
            }
            assert_eq!(actual[0].im, 0.0);
            if n.is_multiple_of(2) {
                assert_eq!(actual[n / 2].im, 0.0);
            }

            let mut reconstructed = vec![0.0_f32; n];
            plan.inverse(&actual, &mut reconstructed, &mut workspace)
                .unwrap();
            for (&value, &reference) in reconstructed.iter().zip(&input) {
                assert!((value - reference).abs() <= 4e-6 * reference.abs().max(1.0));
            }
        }
    }
}

#[test]
fn real_fft_preserves_phase_for_positive_modes() {
    let phase = 0.73_f64;
    for n in [3, 4, 5, 8, 31, 32, 33] {
        let plan = RealFftPlan::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let input: Vec<_> = (0..n)
            .map(|j| (2.0 * PI * j as f64 / n as f64 + phase).cos())
            .collect();
        let mut spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
        plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
        close_complex(
            spectrum[1],
            Complex64::from_polar(n as f64 / 2.0, phase),
            4e-13,
        );
    }
}

#[test]
fn inverse_reconstruction_matches_real_samples_and_fft_scaling() {
    for n in [1, 2, 3, 4, 5, 6, 7, 8, 31, 32, 33] {
        let plan = RealFftPlan::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        for input in sample_rows(n) {
            let mut spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
            let mut actual = vec![0.0; n];
            plan.inverse(&spectrum, &mut actual, &mut workspace)
                .unwrap();
            for (&value, &expected) in actual.iter().zip(&input) {
                assert!((value - expected).abs() <= 3e-13 * expected.abs().max(1.0));
            }
        }
    }
}

#[test]
fn bounded_inverse_matches_robust_inverse_for_ordinary_spectra() {
    for n in [1, 2, 3, 4, 5, 8, 31, 32, 33] {
        let plan = RealFftPlan::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        for row in sample_rows(n) {
            let mut spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            plan.forward(&row, &mut spectrum, &mut workspace).unwrap();

            let mut robust = vec![0.0; n];
            let mut bounded = vec![0.0; n];
            plan.inverse(&spectrum, &mut robust, &mut workspace)
                .unwrap();
            plan.inverse_bounded(&spectrum, &mut bounded, &mut workspace)
                .unwrap();
            assert_eq!(bounded, robust, "n={n}");
        }
    }
}

#[test]
fn inverse_matches_direct_hermitian_sum_for_arbitrary_spectra() {
    for n in [1, 2, 3, 4, 5, 6, 7, 8, 31, 32, 33] {
        let plan = RealFftPlan::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut spectrum: Vec<_> = (0..plan.spectrum_len())
            .map(|k| Complex64::new(0.21 * (k + 1) as f64, 0.17 * (k as f64 - 2.0)))
            .collect();
        spectrum[0].im = 0.0;
        if n.is_multiple_of(2) {
            spectrum[n / 2].im = 0.0;
        }

        let mut actual = vec![0.0; n];
        plan.inverse(&spectrum, &mut actual, &mut workspace)
            .unwrap();
        for (j, value) in actual.iter().enumerate() {
            let mut expected = spectrum[0].re;
            let last_positive = if n.is_multiple_of(2) {
                spectrum.len() - 2
            } else {
                spectrum.len() - 1
            };
            for (k, mode) in spectrum.iter().enumerate().take(last_positive + 1).skip(1) {
                let angle = 2.0 * PI * (k * j) as f64 / n as f64;
                expected += 2.0 * (mode.re * angle.cos() - mode.im * angle.sin());
            }
            if n.is_multiple_of(2) {
                expected += spectrum[n / 2].re * (PI * j as f64).cos();
            }
            expected /= n as f64;
            assert!(
                (value - expected).abs() <= 4e-13 * expected.abs().max(1.0),
                "n={n}, j={j}, actual={value:.16e}, expected={expected:.16e}"
            );
        }
    }
}

#[test]
fn inverse_series_matches_direct_sum_and_preserves_tiny_dc() {
    for n in [1, 2, 3, 4, 5, 8, 31, 32, 33] {
        let plan = RealFftPlan::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut spectrum: Vec<_> = (0..plan.spectrum_len())
            .map(|k| Complex64::new(0.21 * (k + 1) as f64, 0.17 * (k as f64 - 2.0)))
            .collect();
        spectrum[0].im = 0.0;
        if n.is_multiple_of(2) {
            spectrum[n / 2].im = 0.0;
        }
        let mut output = vec![0.0; n];
        plan.inverse_series(&spectrum, &mut output, &mut workspace)
            .unwrap();
        for (j, &actual) in output.iter().enumerate() {
            let mut expected = spectrum[0].re;
            for (k, &mode) in spectrum.iter().enumerate().skip(1) {
                let angle = 2.0 * PI * (k * j) as f64 / n as f64;
                let weight = if 2 * k == n { 1.0 } else { 2.0 };
                expected += weight * (mode.re * angle.cos() - mode.im * angle.sin());
            }
            assert!((actual - expected).abs() <= 4e-13 * expected.abs().max(1.0));
        }
        for dc in [f64::from_bits(1), -f64::from_bits(1), 1.0e308] {
            spectrum.fill(Complex64::new(0.0, 0.0));
            spectrum[0].re = dc;
            plan.inverse_series(&spectrum, &mut output, &mut workspace)
                .unwrap();
            assert!(
                output.iter().all(|&value| value == dc),
                "n={n}, dc={dc}, {output:?}"
            );
        }
    }
}

#[test]
fn mixed_radix_odd_inverse_series_matches_full_complex_and_keeps_dc_edges() {
    let n = 361;
    let plan = RealFftPlan::<f64>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut spectrum: Vec<_> = (0..plan.spectrum_len())
        .map(|k| Complex64::new(0.21 * (k + 1) as f64, 0.17 * (k as f64 - 2.0)))
        .collect();
    spectrum[0].im = 0.0;
    let mut actual = vec![0.0; n];
    plan.inverse_series(&spectrum, &mut actual, &mut workspace)
        .unwrap();

    let mut reference: Vec<_> = (0..n).map(|_| Complex64::new(0.0, 0.0)).collect();
    reference[0] = spectrum[0];
    for k in 1..=n / 2 {
        reference[k] = spectrum[k];
        reference[n - k] = spectrum[k].conj();
    }
    let mut planner = FftPlanner::<f64>::new();
    let inverse = planner.plan_fft_inverse(n);
    let mut scratch = vec![Complex64::new(0.0, 0.0); inverse.get_inplace_scratch_len()];
    inverse.process_with_scratch(&mut reference, &mut scratch);
    for (&value, transformed) in actual.iter().zip(&reference) {
        let error = (value - transformed.re).abs();
        assert!(
            error <= 5e-14 * transformed.re.abs().max(1.0),
            "actual={value:.16e}, reference={:.16e}, error={error:.3e}",
            transformed.re
        );
    }

    // The checked inverse deliberately keeps RustFFT's original full-size
    // plan. Its normalized output must agree with the candidate series
    // after applying the same 1/N scaling.
    let mut robust = vec![0.0; n];
    plan.inverse(&spectrum, &mut robust, &mut workspace)
        .unwrap();
    for (&series_value, &robust_value) in actual.iter().zip(&robust) {
        let normalized = series_value / n as f64;
        assert!(
            (normalized - robust_value).abs() <= 5e-14 * robust_value.abs().max(1.0),
            "series={normalized:.16e}, robust={robust_value:.16e}"
        );
    }

    for dc in [f64::from_bits(1), -f64::from_bits(1), f64::MAX] {
        spectrum.fill(Complex64::new(0.0, 0.0));
        spectrum[0].re = dc;
        plan.inverse_series(&spectrum, &mut actual, &mut workspace)
            .unwrap();
        assert!(actual.iter().all(|&value| value == dc));
    }
}

#[test]
fn only_measured_odd_square_selects_mixed_inverse() {
    for n in [169, 289, 529, 841, 961, 255, 359, 363] {
        let plan = RealFftPlan::<f64>::new(n).unwrap();
        let Engine::Odd {
            inverse_mixed_radix,
            ..
        } = &plan.engine
        else {
            panic!("odd transform should use the odd engine");
        };
        assert!(inverse_mixed_radix.is_none(), "n={n}");
    }

    let plan = RealFftPlan::<f64>::new(361).unwrap();
    let Engine::Odd {
        inverse_mixed_radix,
        ..
    } = &plan.engine
    else {
        panic!("odd transform should use the odd engine");
    };
    assert!(inverse_mixed_radix.is_some());
}

#[test]
fn f32_odd_square_keeps_existing_inverse_planner() {
    let n = 361;
    let plan = RealFftPlan::<f32>::new(n).unwrap();
    let Engine::Odd {
        inverse_mixed_radix,
        ..
    } = &plan.engine
    else {
        panic!("odd transform should use the odd engine");
    };
    assert!(inverse_mixed_radix.is_none());
    let mut workspace = plan.workspace().unwrap();
    let mut spectrum: Vec<_> = (0..plan.spectrum_len())
        .map(|k| Complex32::new(0.21 * (k + 1) as f32, 0.17 * (k as f32 - 2.0)))
        .collect();
    spectrum[0].im = 0.0;
    let mut output = vec![0.0_f32; n];
    plan.inverse_series(&spectrum, &mut output, &mut workspace)
        .unwrap();

    let mut reference: Vec<_> = (0..n).map(|_| Complex32::new(0.0, 0.0)).collect();
    reference[0] = spectrum[0];
    for k in 1..=n / 2 {
        reference[k] = spectrum[k];
        reference[n - k] = spectrum[k].conj();
    }
    let mut planner = FftPlanner::<f32>::new();
    let inverse = planner.plan_fft_inverse(n);
    let mut scratch = vec![Complex32::new(0.0, 0.0); inverse.get_inplace_scratch_len()];
    inverse.process_with_scratch(&mut reference, &mut scratch);
    for (&actual, transformed) in output.iter().zip(&reference) {
        assert!(
            (actual - transformed.re).abs() <= 5e-6 * transformed.re.abs().max(1.0),
            "actual={actual:.8e}, reference={:.8e}",
            transformed.re
        );
    }
}

#[test]
fn half_recombination_preserves_subnormals_and_cancellation() {
    for a in [f64::from_bits(1), f64::MIN_POSITIVE, 1.0e308, -1.0e308] {
        assert_eq!(half_sum(a, a), a);
        assert_eq!(half_sum(a, -a), 0.0);
        assert_eq!(half_sum(a, 0.0), a * 0.5);
    }
    for a in [f32::from_bits(1), f32::MIN_POSITIVE, 2.0e38, -2.0e38] {
        assert_eq!(half_sum(a, a), a);
        assert_eq!(half_sum(a, -a), 0.0);
        assert_eq!(half_sum(a, 0.0), a * 0.5);
    }
}

#[test]
fn real_fft_validates_shapes_and_spectrum_edges() {
    let plan = RealFftPlan::new(4).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
    assert_eq!(
        plan.forward(&[0.0; 3], &mut spectrum, &mut workspace),
        Err(ShtError::BufferLength {
            buffer: "real FFT input",
            expected: 4,
            actual: 3
        })
    );

    spectrum[0].im = 1.0;
    assert_eq!(
        plan.inverse(&spectrum, &mut [0.0; 4], &mut workspace),
        Err(ShtError::NonRealFftEdge { index: 0 })
    );
}
