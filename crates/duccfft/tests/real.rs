use num_complex::Complex;

use duccfft::{FftError, RealFftPlan};

fn real_signal(index: usize, len: usize) -> f64 {
    let x = (index + 1) as f64;
    (0.173 * x).sin() + 0.2 * (0.37 * x + len as f64 * 0.019).cos()
}

fn direct_r2c_f64(input: &[f64]) -> Vec<Complex<f64>> {
    let n = input.len();
    (0..=n / 2)
        .map(|frequency| {
            input
                .iter()
                .enumerate()
                .fold(Complex::new(0.0, 0.0), |sum, (index, &value)| {
                    let phase = std::f64::consts::TAU * (index * frequency) as f64 / n as f64;
                    sum + value * Complex::new(phase.cos(), -phase.sin())
                })
        })
        .collect()
}

fn direct_c2r_f64(spectrum: &[Complex<f64>], n: usize) -> Vec<f64> {
    (0..n)
        .map(|sample| {
            let sum = (0..n).fold(Complex::new(0.0, 0.0), |sum, frequency| {
                let value = if frequency <= n / 2 {
                    let mut value = spectrum[frequency];
                    if frequency == 0 || (n.is_multiple_of(2) && frequency == n / 2) {
                        value.im = 0.0;
                    }
                    value
                } else {
                    spectrum[n - frequency].conj()
                };
                let phase = std::f64::consts::TAU * (sample * frequency) as f64 / n as f64;
                sum + value * Complex::new(phase.cos(), phase.sin())
            });
            sum.re / n as f64
        })
        .collect()
}

fn assert_f64_close(actual: &[f64], expected: &[f64], n: usize) {
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        let tolerance = 3.0e-12 * (n as f64).max(expected.abs()).max(1.0);
        assert!(
            (actual - expected).abs() <= tolerance,
            "mismatch at {index}: actual={actual}, expected={expected}, tolerance={tolerance}"
        );
    }
}

fn assert_complex_f64_close(actual: &[Complex<f64>], expected: &[Complex<f64>], n: usize) {
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        let tolerance = 3.0e-12
            * (n as f64)
                .max(expected.re.abs())
                .max(expected.im.abs())
                .max(1.0);
        assert!(
            (actual - expected).norm() <= tolerance,
            "mismatch at {index}: actual={actual}, expected={expected}, tolerance={tolerance}"
        );
    }
}

fn assert_f32_close(actual: &[f32], expected: &[f64], n: usize) {
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        let tolerance = 1.0e-5 * (n as f64).max(expected.abs()).max(1.0);
        assert!(
            (actual as f64 - expected).abs() <= tolerance,
            "mismatch at {index}: actual={actual}, expected={expected}, tolerance={tolerance}"
        );
    }
}

fn report_f32_real_metrics(label: &str, actual: &[f32], expected: &[f64]) {
    let mut max_absolute = 0.0_f64;
    let mut max_expected = 0.0_f64;
    let mut squared_error = 0.0_f64;
    let mut squared_reference = 0.0_f64;
    let mut nonfinite = 0;
    for (&actual, &expected) in actual.iter().zip(expected) {
        let error = actual as f64 - expected;
        max_absolute = max_absolute.max(error.abs());
        max_expected = max_expected.max(expected.abs());
        squared_error += error * error;
        squared_reference += expected * expected;
        nonfinite += usize::from(!actual.is_finite());
    }
    let rms = (squared_error / actual.len() as f64).sqrt();
    let relative_l2 = (squared_error / squared_reference).sqrt();
    let scaled_max = max_absolute / max_expected.max(1.0);
    println!(
        "f32,{label},{},{max_absolute:.6e},{rms:.6e},{relative_l2:.6e},{scaled_max:.6e},{nonfinite}",
        actual.len()
    );
    assert_eq!(nonfinite, 0, "{label} produced nonfinite f32 outputs");
    assert!(
        relative_l2 < 2.0e-5,
        "{label} relative L2 error={relative_l2}"
    );
    assert!(scaled_max < 2.0e-5, "{label} scaled error={scaled_max}");
}

fn report_f32_spectrum_metrics(
    label: &str,
    transform_len: usize,
    actual: &[Complex<f32>],
    expected: &[Complex<f64>],
) {
    let mut max_absolute = 0.0_f64;
    let mut max_expected = 0.0_f64;
    let mut squared_error = 0.0_f64;
    let mut squared_reference = 0.0_f64;
    let mut nonfinite = 0;
    for (&actual, &expected) in actual.iter().zip(expected) {
        let error = (actual.re as f64 - expected.re).hypot(actual.im as f64 - expected.im);
        max_absolute = max_absolute.max(error);
        max_expected = max_expected.max(expected.norm());
        squared_error += error * error;
        squared_reference += expected.norm_sqr();
        nonfinite += usize::from(!actual.re.is_finite() || !actual.im.is_finite());
    }
    let rms = (squared_error / actual.len() as f64).sqrt();
    let relative_l2 = (squared_error / squared_reference).sqrt();
    let scaled_max = max_absolute / max_expected.max(1.0);
    println!(
        "f32,{label},{},{max_absolute:.6e},{rms:.6e},{relative_l2:.6e},{scaled_max:.6e},{nonfinite}",
        transform_len
    );
    assert_eq!(nonfinite, 0, "{label} produced nonfinite f32 outputs");
    assert!(
        relative_l2 < 2.0e-5,
        "{label} relative L2 error={relative_l2}"
    );
    assert!(scaled_max < 2.0e-5, "{label} scaled error={scaled_max}");
}

fn assert_complex_f32_close(actual: &[Complex<f32>], expected: &[Complex<f64>], n: usize) {
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        let tolerance = 1.0e-5
            * (n as f64)
                .max(expected.re.abs())
                .max(expected.im.abs())
                .max(1.0);
        assert!(
            (actual.re as f64 - expected.re).hypot(actual.im as f64 - expected.im) <= tolerance,
            "mismatch at {index}: actual={actual}, expected={expected}, tolerance={tolerance}"
        );
    }
}

#[test]
fn f64_real_transforms_match_independent_dfts_for_lengths_one_through_sixty_four() {
    for n in 1..=64 {
        let plan = RealFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let input: Vec<_> = (0..n).map(|index| real_signal(index, n)).collect();
        let expected_spectrum = direct_r2c_f64(&input);
        let mut spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
        plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
        assert_complex_f64_close(&spectrum, &expected_spectrum, n);

        let independent_spectrum: Vec<_> = (0..plan.spectrum_len())
            .map(|index| {
                Complex::new(
                    real_signal(index + 11, n + 3),
                    real_signal(index + 7, n + 9) * 0.4,
                )
            })
            .collect();
        let expected_samples = direct_c2r_f64(&independent_spectrum, n);
        let mut actual_samples = vec![0.0; n];
        plan.inverse(&independent_spectrum, &mut actual_samples, &mut workspace)
            .unwrap();
        assert_f64_close(&actual_samples, &expected_samples, n);
    }
}

#[test]
fn f64_real_radix2_inverse_pass_matches_independent_dfts() {
    for n in [128, 512, 2048] {
        let plan = RealFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let input: Vec<_> = (0..n).map(|index| real_signal(index + 19, n + 5)).collect();
        let expected_spectrum = direct_r2c_f64(&input);
        let mut actual_spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
        plan.forward(&input, &mut actual_spectrum, &mut workspace)
            .unwrap();
        assert_complex_f64_close(&actual_spectrum, &expected_spectrum, n);

        let independent_spectrum: Vec<_> = (0..plan.spectrum_len())
            .map(|index| {
                Complex::new(
                    real_signal(index + 31, n + 7),
                    real_signal(index + 13, n + 11) * 0.2,
                )
            })
            .collect();
        let expected_samples = direct_c2r_f64(&independent_spectrum, n);
        let mut actual_samples = vec![0.0; n];
        plan.inverse(&independent_spectrum, &mut actual_samples, &mut workspace)
            .unwrap();
        assert_f64_close(&actual_samples, &expected_samples, n);
    }
}

#[test]
fn f32_real_transforms_match_independent_dfts_for_lengths_one_through_sixty_four() {
    for n in 1..=64 {
        let plan = RealFftPlan::<f32>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let input: Vec<_> = (0..n).map(|index| real_signal(index, n) as f32).collect();
        let expected_spectrum =
            direct_r2c_f64(&input.iter().map(|&value| value as f64).collect::<Vec<_>>());
        let mut spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
        plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
        assert_complex_f32_close(&spectrum, &expected_spectrum, n);

        let independent_spectrum: Vec<_> = (0..plan.spectrum_len())
            .map(|index| {
                Complex::new(
                    real_signal(index + 11, n + 3) as f32,
                    (real_signal(index + 7, n + 9) * 0.4) as f32,
                )
            })
            .collect();
        let expected_samples = direct_c2r_f64(
            &independent_spectrum
                .iter()
                .map(|value| Complex::new(value.re as f64, value.im as f64))
                .collect::<Vec<_>>(),
            n,
        );
        let mut actual_samples = vec![0.0; n];
        plan.inverse(&independent_spectrum, &mut actual_samples, &mut workspace)
            .unwrap();
        assert_f32_close(&actual_samples, &expected_samples, n);
    }
}

#[test]
fn f32_large_smooth_prime_bluestein_and_sht_lengths_match_f64_references() {
    println!("precision,operation,n,max_abs,rms,relative_l2,scaled_max,nonfinite");
    for n in [1024, 2048, 4096, 131, 137, 241, 1025, 2049, 4097] {
        let plan = RealFftPlan::<f32>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let input32: Vec<_> = (0..n)
            .map(|index| real_signal(index + 19, n + 5) as f32)
            .collect();
        let input64: Vec<_> = input32.iter().map(|&value| value as f64).collect();
        let expected_spectrum = direct_r2c_f64(&input64);
        let mut actual_spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
        plan.forward(&input32, &mut actual_spectrum, &mut workspace)
            .unwrap();
        let expected_complex64: Vec<_> = expected_spectrum
            .iter()
            .map(|value| Complex::new(value.re, value.im))
            .collect();
        report_f32_spectrum_metrics("r2c", n, &actual_spectrum, &expected_complex64);

        let spectrum32: Vec<_> = (0..plan.spectrum_len())
            .map(|index| {
                Complex::new(
                    real_signal(index + 31, n + 7) as f32,
                    (0.2 * real_signal(index + 13, n + 11)) as f32,
                )
            })
            .collect();
        let spectrum64: Vec<_> = spectrum32
            .iter()
            .map(|value| Complex::new(value.re as f64, value.im as f64))
            .collect();
        let expected_samples = direct_c2r_f64(&spectrum64, n);
        let mut actual_samples = vec![0.0; n];
        plan.inverse(&spectrum32, &mut actual_samples, &mut workspace)
            .unwrap();
        report_f32_real_metrics("c2r", &actual_samples, &expected_samples);
    }
}

#[test]
fn real_general_prime_and_bluestein_paths_match_independent_dfts() {
    for n in [127, 131, 137, 241] {
        let plan = RealFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let input: Vec<_> = (0..n).map(|index| real_signal(index + 3, n + 5)).collect();
        let expected_spectrum = direct_r2c_f64(&input);
        let mut actual_spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
        plan.forward(&input, &mut actual_spectrum, &mut workspace)
            .unwrap();
        assert_complex_f64_close(&actual_spectrum, &expected_spectrum, n);

        let independent_spectrum: Vec<_> = (0..plan.spectrum_len())
            .map(|index| {
                Complex::new(
                    real_signal(index + 17, n + 2),
                    real_signal(index + 9, n + 11) * 0.3,
                )
            })
            .collect();
        let expected_samples = direct_c2r_f64(&independent_spectrum, n);
        let mut actual_samples = vec![0.0; n];
        plan.inverse(&independent_spectrum, &mut actual_samples, &mut workspace)
            .unwrap();
        assert_f64_close(&actual_samples, &expected_samples, n);
    }

    let n = 137;
    let plan = RealFftPlan::<f32>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let input: Vec<_> = (0..n)
        .map(|index| real_signal(index + 13, n + 5) as f32)
        .collect();
    let expected_spectrum =
        direct_r2c_f64(&input.iter().map(|&value| value as f64).collect::<Vec<_>>());
    let mut actual_spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
    plan.forward(&input, &mut actual_spectrum, &mut workspace)
        .unwrap();
    assert_complex_f32_close(&actual_spectrum, &expected_spectrum, n);

    let independent_spectrum: Vec<_> = (0..plan.spectrum_len())
        .map(|index| {
            Complex::new(
                real_signal(index + 29, n + 3) as f32,
                (real_signal(index + 17, n + 11) * 0.2) as f32,
            )
        })
        .collect();
    let expected_samples = direct_c2r_f64(
        &independent_spectrum
            .iter()
            .map(|value| Complex::new(value.re as f64, value.im as f64))
            .collect::<Vec<_>>(),
        n,
    );
    let mut actual_samples = vec![0.0_f32; n];
    plan.inverse(&independent_spectrum, &mut actual_samples, &mut workspace)
        .unwrap();
    assert_f32_close(&actual_samples, &expected_samples, n);
}

#[test]
fn real_complexify_planner_path_matches_f64_and_f32_direct_dfts() {
    let n = 1026;
    let plan64 = RealFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let input64: Vec<_> = (0..n).map(|index| real_signal(index + 19, n + 7)).collect();
    let expected_spectrum64 = direct_r2c_f64(&input64);
    let mut actual_spectrum64 = vec![Complex::new(0.0, 0.0); plan64.spectrum_len()];
    plan64
        .forward(&input64, &mut actual_spectrum64, &mut workspace64)
        .unwrap();
    assert_complex_f64_close(&actual_spectrum64, &expected_spectrum64, n);

    let independent_spectrum64: Vec<_> = (0..plan64.spectrum_len())
        .map(|index| {
            Complex::new(
                real_signal(index + 23, n + 3),
                0.2 * real_signal(index + 11, n + 9),
            )
        })
        .collect();
    let expected_samples64 = direct_c2r_f64(&independent_spectrum64, n);
    let mut actual_samples64 = vec![0.0; n];
    plan64
        .inverse(
            &independent_spectrum64,
            &mut actual_samples64,
            &mut workspace64,
        )
        .unwrap();
    assert_f64_close(&actual_samples64, &expected_samples64, n);

    let plan32 = RealFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let input32: Vec<_> = input64.iter().map(|&value| value as f32).collect();
    let input32_f64: Vec<_> = input32.iter().map(|&value| value as f64).collect();
    let expected_spectrum32 = direct_r2c_f64(&input32_f64);
    let mut actual_spectrum32 = vec![Complex::new(0.0, 0.0); plan32.spectrum_len()];
    plan32
        .forward(&input32, &mut actual_spectrum32, &mut workspace32)
        .unwrap();
    assert_complex_f32_close(&actual_spectrum32, &expected_spectrum32, n);

    let independent_spectrum32: Vec<_> = independent_spectrum64
        .iter()
        .map(|value| Complex::new(value.re as f32, value.im as f32))
        .collect();
    let independent_spectrum32_f64: Vec<_> = independent_spectrum32
        .iter()
        .map(|value| Complex::new(value.re as f64, value.im as f64))
        .collect();
    let expected_samples32 = direct_c2r_f64(&independent_spectrum32_f64, n);
    let mut actual_samples32 = vec![0.0; n];
    plan32
        .inverse(
            &independent_spectrum32,
            &mut actual_samples32,
            &mut workspace32,
        )
        .unwrap();
    assert_f32_close(&actual_samples32, &expected_samples32, n);
}

#[test]
fn real_complexify_preserves_maximum_finite_transform_results() {
    let n = 1026;
    let half_len = n / 2;

    let plan64 = RealFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let mut impulse64 = vec![0.0_f64; n];
    impulse64[0] = f64::MAX;
    let mut spectrum64 = vec![Complex::new(0.0, 0.0); plan64.spectrum_len()];
    plan64
        .forward(&impulse64, &mut spectrum64, &mut workspace64)
        .unwrap();
    assert!(
        spectrum64
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    for value in &spectrum64 {
        assert!((value.re / f64::MAX - 1.0).abs() < 2.0e-12);
        assert!(value.im.abs() / f64::MAX < 2.0e-12);
    }

    for nyquist_sign in [1.0_f64, -1.0] {
        spectrum64.fill(Complex::new(0.0, 0.0));
        spectrum64[0].re = f64::MAX;
        spectrum64[half_len].re = nyquist_sign * f64::MAX;
        let mut output = vec![0.0_f64; n];
        plan64
            .inverse(&spectrum64, &mut output, &mut workspace64)
            .unwrap();
        assert!(output.iter().all(|value| value.is_finite()));
        let expected = f64::MAX / half_len as f64;
        for (index, &value) in output.iter().enumerate() {
            if (index % 2 == 0) == (nyquist_sign > 0.0) {
                assert!((value / expected - 1.0).abs() < 2.0e-12);
            } else {
                assert_eq!(value, 0.0);
            }
        }
    }

    spectrum64.fill(Complex::new(0.0, 0.0));
    spectrum64[0].re = f64::MAX;
    spectrum64[half_len].re = -f64::MAX;
    let tiny64 = 4.0 * f64::from_bits(1);
    for value in &mut spectrum64[1..half_len] {
        value.re = tiny64;
    }
    let mut dynamic_output64 = vec![0.0_f64; n];
    plan64
        .inverse(&spectrum64, &mut dynamic_output64, &mut workspace64)
        .unwrap();
    let dynamic_expected64 = tiny64 * (2 * (half_len - 1)) as f64 / n as f64;
    assert_eq!(dynamic_output64[0], dynamic_expected64);

    let plan32 = RealFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let mut impulse32 = vec![0.0_f32; n];
    impulse32[0] = f32::MAX;
    let mut spectrum32 = vec![Complex::new(0.0, 0.0); plan32.spectrum_len()];
    plan32
        .forward(&impulse32, &mut spectrum32, &mut workspace32)
        .unwrap();
    assert!(
        spectrum32
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    for value in &spectrum32 {
        assert!((value.re / f32::MAX - 1.0).abs() < 5.0e-5);
        assert!(value.im.abs() / f32::MAX < 5.0e-5);
    }

    for nyquist_sign in [1.0_f32, -1.0] {
        spectrum32.fill(Complex::new(0.0, 0.0));
        spectrum32[0].re = f32::MAX;
        spectrum32[half_len].re = nyquist_sign * f32::MAX;
        let mut output = vec![0.0_f32; n];
        plan32
            .inverse(&spectrum32, &mut output, &mut workspace32)
            .unwrap();
        assert!(output.iter().all(|value| value.is_finite()));
        let expected = f32::MAX / half_len as f32;
        for (index, &value) in output.iter().enumerate() {
            if (index % 2 == 0) == (nyquist_sign > 0.0) {
                assert!((value / expected - 1.0).abs() < 5.0e-5);
            } else {
                assert_eq!(value, 0.0);
            }
        }
    }

    spectrum32.fill(Complex::new(0.0, 0.0));
    spectrum32[0].re = f32::MAX;
    spectrum32[half_len].re = -f32::MAX;
    let tiny32 = 4.0 * f32::from_bits(1);
    for value in &mut spectrum32[1..half_len] {
        value.re = tiny32;
    }
    let mut dynamic_output32 = vec![0.0_f32; n];
    plan32
        .inverse(&spectrum32, &mut dynamic_output32, &mut workspace32)
        .unwrap();
    let dynamic_expected32 = tiny32 * (2 * (half_len - 1)) as f32 / n as f32;
    assert_eq!(dynamic_output32[0], dynamic_expected32);
}

#[test]
fn odd_real_inverse_counts_last_bin_imaginary_value_for_overflow_bound() {
    let plan64 = RealFftPlan::<f64>::new(3).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let spectrum64 = [Complex::new(0.0, 0.0), Complex::new(0.0, f64::MAX)];
    let mut output64 = [0.0; 3];
    plan64
        .inverse(&spectrum64, &mut output64, &mut workspace64)
        .unwrap();
    let expected64 = f64::MAX / 3.0 * 3.0_f64.sqrt();
    assert!(output64.iter().all(|value| value.is_finite()));
    assert_eq!(output64[0], 0.0);
    assert!((output64[1] / -expected64 - 1.0).abs() < 1.0e-12);
    assert!((output64[2] / expected64 - 1.0).abs() < 1.0e-12);

    let plan32 = RealFftPlan::<f32>::new(3).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let spectrum32 = [Complex::new(0.0, 0.0), Complex::new(0.0, f32::MAX)];
    let mut output32 = [0.0; 3];
    plan32
        .inverse(&spectrum32, &mut output32, &mut workspace32)
        .unwrap();
    let expected32 = f32::MAX / 3.0 * 3.0_f32.sqrt();
    assert!(output32.iter().all(|value| value.is_finite()));
    assert_eq!(output32[0], 0.0);
    assert!((output32[1] / -expected32 - 1.0).abs() < 1.0e-5);
    assert!((output32[2] / expected32 - 1.0).abs() < 1.0e-5);
}

#[test]
fn real_complexify_inverse_preserves_roundable_subnormal_dc() {
    let n = 1026;
    let plan64 = RealFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let tiny64 = f64::from_bits(1);
    for dc_units in [540.0, 514.0] {
        let mut spectrum64 = vec![Complex::new(0.0, 0.0); plan64.spectrum_len()];
        spectrum64[0].re = dc_units * tiny64;
        let mut output64 = vec![0.0; n];
        plan64
            .inverse(&spectrum64, &mut output64, &mut workspace64)
            .unwrap();
        assert_eq!(output64, vec![tiny64; n], "f64 DC units: {dc_units}");
    }

    let plan32 = RealFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let tiny32 = f32::from_bits(1);
    for dc_units in [540.0_f32, 514.0] {
        let mut spectrum32 = vec![Complex::new(0.0, 0.0); plan32.spectrum_len()];
        spectrum32[0].re = dc_units * tiny32;
        let mut output32 = vec![0.0; n];
        plan32
            .inverse(&spectrum32, &mut output32, &mut workspace32)
            .unwrap();
        assert_eq!(output32, vec![tiny32; n], "f32 DC units: {dc_units}");
    }
}

#[test]
fn real_bluestein_handles_representable_high_amplitude_impulses() {
    let plan64 = RealFftPlan::<f64>::new(137).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let amplitude64 = f64::MAX / 4.0;
    let mut impulse64 = vec![0.0; plan64.len()];
    impulse64[0] = amplitude64;
    let mut spectrum64 = vec![Complex::new(0.0, 0.0); plan64.spectrum_len()];
    plan64
        .forward(&impulse64, &mut spectrum64, &mut workspace64)
        .unwrap();
    assert!(
        spectrum64
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_complex_f64_close(
        &spectrum64,
        &vec![Complex::new(amplitude64, 0.0); plan64.spectrum_len()],
        plan64.len(),
    );

    let plan32 = RealFftPlan::<f32>::new(137).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let amplitude32 = f32::MAX / 4.0;
    let mut impulse32 = vec![0.0; plan32.len()];
    impulse32[0] = amplitude32;
    let mut spectrum32 = vec![Complex::new(0.0, 0.0); plan32.spectrum_len()];
    plan32
        .forward(&impulse32, &mut spectrum32, &mut workspace32)
        .unwrap();
    assert!(
        spectrum32
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_complex_f32_close(
        &spectrum32,
        &vec![Complex::new(amplitude32 as f64, 0.0); plan32.spectrum_len()],
        plan32.len(),
    );
}

#[test]
fn real_bluestein_preserves_subnormal_impulses_and_constant_inverse_spectra() {
    let n = 137;
    let tiny64 = f64::from_bits(1);
    let plan64 = RealFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let mut impulse64 = vec![0.0; n];
    impulse64[0] = tiny64;
    let mut spectrum64 = vec![Complex::new(0.0, 0.0); plan64.spectrum_len()];
    plan64
        .forward(&impulse64, &mut spectrum64, &mut workspace64)
        .unwrap();
    assert_eq!(
        spectrum64,
        vec![Complex::new(tiny64, 0.0); plan64.spectrum_len()]
    );

    let mut constant_spectrum64 = vec![Complex::new(0.0, 0.0); plan64.spectrum_len()];
    constant_spectrum64[0].re = tiny64 * n as f64;
    let mut output64 = vec![0.0; n];
    plan64
        .inverse(&constant_spectrum64, &mut output64, &mut workspace64)
        .unwrap();
    assert_eq!(output64, vec![tiny64; n]);

    let tiny32 = f32::from_bits(1);
    let plan32 = RealFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let mut impulse32 = vec![0.0; n];
    impulse32[0] = tiny32;
    let mut spectrum32 = vec![Complex::new(0.0, 0.0); plan32.spectrum_len()];
    plan32
        .forward(&impulse32, &mut spectrum32, &mut workspace32)
        .unwrap();
    assert_eq!(
        spectrum32,
        vec![Complex::new(tiny32, 0.0); plan32.spectrum_len()]
    );

    let mut constant_spectrum32 = vec![Complex::new(0.0, 0.0); plan32.spectrum_len()];
    constant_spectrum32[0].re = tiny32 * n as f32;
    let mut output32 = vec![0.0; n];
    plan32
        .inverse(&constant_spectrum32, &mut output32, &mut workspace32)
        .unwrap();
    assert_eq!(output32, vec![tiny32; n]);
}

#[test]
fn real_bluestein_scales_high_amplitude_inputs_before_the_convolution_fft() {
    let n = 137;
    let unit_input: Vec<_> = (0..n)
        .map(|index| (std::f64::consts::PI * (index * index) as f64 / n as f64).cos())
        .collect();
    let unit_expected = direct_r2c_f64(&unit_input);

    let amplitude64 = f64::MAX / 100.0;
    let input64: Vec<_> = unit_input
        .iter()
        .map(|&value| value * amplitude64)
        .collect();
    let expected64: Vec<_> = unit_expected
        .iter()
        .map(|&value| value * amplitude64)
        .collect();
    let plan64 = RealFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let mut actual64 = vec![Complex::new(0.0, 0.0); plan64.spectrum_len()];
    plan64
        .forward(&input64, &mut actual64, &mut workspace64)
        .unwrap();
    assert!(
        actual64
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_complex_f64_close(&actual64, &expected64, n);

    let amplitude32 = f32::MAX / 100.0;
    let input32: Vec<_> = unit_input
        .iter()
        .map(|&value| (value as f32) * amplitude32)
        .collect();
    let plan32 = RealFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let mut actual32 = vec![Complex::new(0.0, 0.0); plan32.spectrum_len()];
    plan32
        .forward(&input32, &mut actual32, &mut workspace32)
        .unwrap();
    assert!(
        actual32
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    let expected32: Vec<_> = unit_expected
        .iter()
        .map(|&value| value * amplitude32 as f64)
        .collect();
    assert_complex_f32_close(&actual32, &expected32, n);
}

#[test]
fn nonterminal_real_bluestein_stages_match_rustfft() {
    use rustfft::FftPlanner;

    let n = 137 * 137;
    let input: Vec<_> = (0..n).map(|index| real_signal(index + 7, n + 3)).collect();
    let mut expected_full: Vec<_> = input
        .iter()
        .map(|&value| Complex::new(value, 0.0))
        .collect();
    FftPlanner::<f64>::new()
        .plan_fft_forward(n)
        .process(&mut expected_full);

    let plan = RealFftPlan::<f64>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut actual = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
    plan.forward(&input, &mut actual, &mut workspace).unwrap();
    assert_complex_f64_close(&actual, &expected_full[..plan.spectrum_len()], n);

    let mut independent_spectrum: Vec<_> = (0..plan.spectrum_len())
        .map(|index| {
            Complex::new(
                real_signal(index + 31, n + 5),
                0.25 * real_signal(index + 13, n + 9),
            )
        })
        .collect();
    independent_spectrum[0].im = 0.0;
    let mut expected_inverse = vec![Complex::new(0.0, 0.0); n];
    expected_inverse[..plan.spectrum_len()].copy_from_slice(&independent_spectrum);
    for frequency in 1..plan.spectrum_len() {
        expected_inverse[n - frequency] = independent_spectrum[frequency].conj();
    }
    FftPlanner::<f64>::new()
        .plan_fft_inverse(n)
        .process(&mut expected_inverse);
    let inverse_scale = 1.0 / n as f64;
    let expected_samples: Vec<_> = expected_inverse
        .iter()
        .map(|value| value.re * inverse_scale)
        .collect();
    let mut actual_samples = vec![0.0; n];
    plan.inverse(&independent_spectrum, &mut actual_samples, &mut workspace)
        .unwrap();
    assert_f64_close(&actual_samples, &expected_samples, n);
}

#[test]
fn impossible_real_allocation_size_is_rejected_before_factorization() {
    assert!(matches!(
        RealFftPlan::<f64>::new(usize::MAX - 58),
        Err(FftError::AllocationFailed)
    ));
}

#[test]
fn real_transforms_cover_odd_even_and_endpoint_layouts() {
    for n in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 15, 16, 31, 32, 33] {
        let plan = RealFftPlan::<f64>::new(n).unwrap();
        assert_eq!(plan.spectrum_len(), n / 2 + 1);
        let mut workspace = plan.workspace().unwrap();

        let mut impulse = vec![0.0; n];
        impulse[0] = 1.0;
        let mut spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
        plan.forward(&impulse, &mut spectrum, &mut workspace)
            .unwrap();
        assert_complex_f64_close(&spectrum, &vec![Complex::new(1.0, 0.0); spectrum.len()], n);

        let constant = vec![0.25; n];
        plan.forward(&constant, &mut spectrum, &mut workspace)
            .unwrap();
        assert!((spectrum[0].re - 0.25 * n as f64).abs() < 1.0e-12 * n as f64);
        assert!(spectrum[0].im.abs() < 1.0e-12 * n as f64);
        assert!(
            spectrum[1..]
                .iter()
                .all(|value| value.norm() < 1.0e-11 * n as f64)
        );

        let mut roundtrip = vec![0.0; n];
        plan.inverse(&spectrum, &mut roundtrip, &mut workspace)
            .unwrap();
        assert_f64_close(&roundtrip, &constant, n);

        if n.is_multiple_of(2) {
            let nyquist: Vec<_> = (0..n)
                .map(|index| if index.is_multiple_of(2) { 1.0 } else { -1.0 })
                .collect();
            plan.forward(&nyquist, &mut spectrum, &mut workspace)
                .unwrap();
            assert_eq!(spectrum[n / 2], Complex::new(n as f64, 0.0));
        }
    }
}

#[test]
fn inverse_ignores_imaginary_dc_and_even_nyquist_components() {
    for n in [1, 2, 4, 8, 9] {
        let plan = RealFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
        spectrum[0] = Complex::new(0.75, 91.0);
        if n.is_multiple_of(2) {
            spectrum[n / 2] = Complex::new(-0.125, -43.0);
        }
        let mut actual = vec![0.0; n];
        plan.inverse(&spectrum, &mut actual, &mut workspace)
            .unwrap();
        let expected = direct_c2r_f64(&spectrum, n);
        assert_f64_close(&actual, &expected, n);
    }
}

#[test]
fn inverse_preserves_small_terms_in_high_dynamic_range_spectra() {
    let tiny64 = f64::from_bits(1);
    let plan64 = RealFftPlan::<f64>::new(4).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let spectrum64 = [
        Complex::new(f64::MAX, 0.0),
        Complex::new(2.0 * tiny64, 0.0),
        Complex::new(-f64::MAX, 0.0),
    ];
    let mut output64 = [0.0; 4];
    plan64
        .inverse(&spectrum64, &mut output64, &mut workspace64)
        .unwrap();
    assert_eq!(output64, [tiny64, f64::MAX / 2.0, -tiny64, f64::MAX / 2.0]);

    let tiny32 = f32::from_bits(1);
    let plan32 = RealFftPlan::<f32>::new(4).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let spectrum32 = [
        Complex::new(f32::MAX, 0.0),
        Complex::new(2.0 * tiny32, 0.0),
        Complex::new(-f32::MAX, 0.0),
    ];
    let mut output32 = [0.0; 4];
    plan32
        .inverse(&spectrum32, &mut output32, &mut workspace32)
        .unwrap();
    assert_eq!(output32, [tiny32, f32::MAX / 2.0, -tiny32, f32::MAX / 2.0]);
}

#[test]
fn real_inverse_normalization_fast_path_matches_random_spectra() {
    for n in [1024, 2048, 4096] {
        let mut state = 0x3c6e_f372_fe94_f82b_u64 ^ n as u64;
        let mut next_value = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            2.0 * ((state >> 11) as f64 / (1_u64 << 53) as f64) - 1.0
        };

        let spectrum64: Vec<_> = (0..=n / 2)
            .map(|_| Complex::new(next_value(), next_value()))
            .collect();
        let expected64 = direct_c2r_f64(&spectrum64, n);
        let plan64 = RealFftPlan::<f64>::new(n).unwrap();
        let mut workspace64 = plan64.workspace().unwrap();
        let mut actual64 = vec![0.0; n];
        plan64
            .inverse(&spectrum64, &mut actual64, &mut workspace64)
            .unwrap();
        assert_f64_close(&actual64, &expected64, n);

        let spectrum32: Vec<_> = spectrum64
            .iter()
            .map(|value| Complex::new(value.re as f32, value.im as f32))
            .collect();
        let rounded_spectrum64: Vec<_> = spectrum32
            .iter()
            .map(|value| Complex::new(value.re as f64, value.im as f64))
            .collect();
        let expected32 = direct_c2r_f64(&rounded_spectrum64, n);
        let plan32 = RealFftPlan::<f32>::new(n).unwrap();
        let mut workspace32 = plan32.workspace().unwrap();
        let mut actual32 = vec![0.0; n];
        plan32
            .inverse(&spectrum32, &mut actual32, &mut workspace32)
            .unwrap();
        assert_f32_close(&actual32, &expected32, n);
    }
}

#[test]
fn real_inverse_fast_and_correction_paths_cover_finite_extremes() {
    let n = 8;
    let plan64 = RealFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();

    let cancellation64 = [
        Complex::new(1.0, 0.0),
        Complex::new(1.0, -0.5),
        Complex::new(-1.0 + f64::EPSILON, 0.25),
        Complex::new(0.125, -0.75),
        Complex::new(-1.0, 0.0),
    ];
    let expected64 = direct_c2r_f64(&cancellation64, n);
    let mut actual64 = [0.0; 8];
    plan64
        .inverse(&cancellation64, &mut actual64, &mut workspace64)
        .unwrap();
    assert_f64_close(&actual64, &expected64, n);

    let mut maximum64 = [Complex::new(0.0, 0.0); 5];
    maximum64[0].re = f64::MAX;
    plan64
        .inverse(&maximum64, &mut actual64, &mut workspace64)
        .unwrap();
    assert_eq!(actual64, [f64::MAX / n as f64; 8]);

    for amplitude in [f64::MIN_POSITIVE, f64::from_bits(1)] {
        let mut spectrum = [Complex::new(0.0, 0.0); 5];
        spectrum[0].re = amplitude * n as f64;
        plan64
            .inverse(&spectrum, &mut actual64, &mut workspace64)
            .unwrap();
        assert_eq!(actual64, [amplitude; 8]);
    }

    let plan32 = RealFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let cancellation32 = cancellation64.map(|value| Complex::new(value.re as f32, value.im as f32));
    let expected32 = direct_c2r_f64(
        &cancellation32
            .iter()
            .map(|value| Complex::new(value.re as f64, value.im as f64))
            .collect::<Vec<_>>(),
        n,
    );
    let mut actual32 = [0.0; 8];
    plan32
        .inverse(&cancellation32, &mut actual32, &mut workspace32)
        .unwrap();
    assert_f32_close(&actual32, &expected32, n);

    let mut maximum32 = [Complex::new(0.0, 0.0); 5];
    maximum32[0].re = f32::MAX;
    plan32
        .inverse(&maximum32, &mut actual32, &mut workspace32)
        .unwrap();
    assert_eq!(actual32, [f32::MAX / n as f32; 8]);

    for amplitude in [f32::MIN_POSITIVE, f32::from_bits(1)] {
        let mut spectrum = [Complex::new(0.0, 0.0); 5];
        spectrum[0].re = amplitude * n as f32;
        plan32
            .inverse(&spectrum, &mut actual32, &mut workspace32)
            .unwrap();
        assert_eq!(actual32, [amplitude; 8]);
    }
}

#[test]
fn real_radix_multipass_fast_paths_and_extreme_fallbacks_are_checked() {
    let n = 64;
    let plan64 = RealFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let bounded64 = f64::MAX / (4.0 * n as f64);
    let bounded_input64 = vec![bounded64; n];
    let mut spectrum64 = vec![Complex::new(0.0, 0.0); plan64.spectrum_len()];
    plan64
        .forward(&bounded_input64, &mut spectrum64, &mut workspace64)
        .unwrap();
    assert!(
        spectrum64
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_eq!(spectrum64[0].re, bounded64 * n as f64);

    let mut impulse64 = vec![0.0; n];
    impulse64[0] = f64::MAX;
    plan64
        .forward(&impulse64, &mut spectrum64, &mut workspace64)
        .unwrap();
    assert_eq!(
        spectrum64,
        vec![Complex::new(f64::MAX, 0.0); plan64.spectrum_len()]
    );

    spectrum64.fill(Complex::new(0.0, 0.0));
    spectrum64[0].re = f64::MAX;
    spectrum64[n / 2].re = f64::MAX;
    let mut output64 = vec![0.0; n];
    plan64
        .inverse(&spectrum64, &mut output64, &mut workspace64)
        .unwrap();
    assert!(output64.iter().all(|value| value.is_finite()));
    assert_eq!(
        output64,
        (0..n)
            .map(|index| if index.is_multiple_of(2) {
                f64::MAX / (n / 2) as f64
            } else {
                0.0
            })
            .collect::<Vec<_>>()
    );

    let plan32 = RealFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let bounded32 = f32::MAX / (4.0 * n as f32);
    let bounded_input32 = vec![bounded32; n];
    let mut spectrum32 = vec![Complex::new(0.0, 0.0); plan32.spectrum_len()];
    plan32
        .forward(&bounded_input32, &mut spectrum32, &mut workspace32)
        .unwrap();
    assert!(
        spectrum32
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_eq!(spectrum32[0].re, bounded32 * n as f32);

    let mut impulse32 = vec![0.0; n];
    impulse32[0] = f32::MAX;
    plan32
        .forward(&impulse32, &mut spectrum32, &mut workspace32)
        .unwrap();
    assert_eq!(
        spectrum32,
        vec![Complex::new(f32::MAX, 0.0); plan32.spectrum_len()]
    );

    spectrum32.fill(Complex::new(0.0, 0.0));
    spectrum32[0].re = f32::MAX;
    spectrum32[n / 2].re = f32::MAX;
    let mut output32 = vec![0.0; n];
    plan32
        .inverse(&spectrum32, &mut output32, &mut workspace32)
        .unwrap();
    assert!(output32.iter().all(|value| value.is_finite()));
    assert_eq!(
        output32,
        (0..n)
            .map(|index| if index.is_multiple_of(2) {
                f32::MAX / (n / 2) as f32
            } else {
                0.0
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn invalid_real_lengths_buffers_and_workspaces_return_errors() {
    assert!(matches!(
        RealFftPlan::<f64>::new(0),
        Err(FftError::InvalidLength)
    ));
    assert!(matches!(
        RealFftPlan::<f64>::new(usize::MAX),
        Err(FftError::AllocationFailed)
    ));

    let plan = RealFftPlan::<f64>::new(5).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let short_input = [0.0; 4];
    let mut spectrum = [Complex::new(0.0, 0.0); 3];
    assert_eq!(
        plan.forward(&short_input, &mut spectrum, &mut workspace),
        Err(FftError::BufferLength {
            buffer: "input",
            expected: 5,
            actual: 4,
        })
    );
    let input = [0.0; 5];
    let mut short_spectrum = [Complex::new(0.0, 0.0); 2];
    assert_eq!(
        plan.forward(&input, &mut short_spectrum, &mut workspace),
        Err(FftError::BufferLength {
            buffer: "spectrum",
            expected: 3,
            actual: 2,
        })
    );

    let other_plan = RealFftPlan::<f64>::new(3).unwrap();
    let mut other_workspace = other_plan.workspace().unwrap();
    let mut spectrum = [Complex::new(0.0, 0.0); 3];
    assert_eq!(
        plan.forward(&input, &mut spectrum, &mut other_workspace),
        Err(FftError::BufferLength {
            buffer: "workspace",
            expected: 5,
            actual: 3,
        })
    );
}

#[test]
fn real_workspace_reports_reusable_scratch_bytes() {
    let plan = RealFftPlan::<f64>::new(8).unwrap();
    let workspace = plan.workspace().unwrap();
    assert_eq!(
        workspace.scratch_bytes(),
        Some(22 * std::mem::size_of::<Complex<f64>>() + 2 * 8 * std::mem::size_of::<f64>())
    );

    let blue_plan = RealFftPlan::<f64>::new(137).unwrap();
    let blue_workspace = blue_plan.workspace().unwrap();
    assert!(blue_workspace.scratch_bytes().unwrap() > workspace.scratch_bytes().unwrap());
}

#[test]
fn shared_real_plans_execute_concurrently_with_separate_workspaces() {
    let plan = RealFftPlan::<f64>::new(45).unwrap();
    std::thread::scope(|scope| {
        for row in 0..4 {
            let plan = &plan;
            scope.spawn(move || {
                let mut workspace = plan.workspace().unwrap();
                let input: Vec<_> = (0..plan.len())
                    .map(|index| real_signal(index + row, plan.len()))
                    .collect();
                let expected = direct_r2c_f64(&input);
                let mut spectrum = vec![Complex::new(0.0, 0.0); plan.spectrum_len()];
                plan.forward(&input, &mut spectrum, &mut workspace).unwrap();
                assert_complex_f64_close(&spectrum, &expected, plan.len());
            });
        }
    });
}

#[test]
fn contiguous_real_batches_match_direct_dfts() {
    for n in [15, 16] {
        let rows = 3;
        let plan = RealFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let input: Vec<_> = (0..rows)
            .flat_map(|row| (0..n).map(move |index| real_signal(index + 13 * row, n)))
            .collect();
        let expected_spectrum: Vec<_> = input.chunks_exact(n).flat_map(direct_r2c_f64).collect();
        let expected_output: Vec<_> = expected_spectrum
            .chunks_exact(plan.spectrum_len())
            .flat_map(|row| direct_c2r_f64(row, n))
            .collect();

        let mut spectrum = vec![Complex::new(0.0, 0.0); rows * plan.spectrum_len()];
        plan.forward_batch(&input, &mut spectrum, &mut workspace)
            .unwrap();
        assert_complex_f64_close(&spectrum, &expected_spectrum, n);

        let mut output = vec![0.0; rows * n];
        plan.inverse_batch(&expected_spectrum, &mut output, &mut workspace)
            .unwrap();
        assert_f64_close(&output, &expected_output, n);
        plan.inverse_batch(&spectrum, &mut output, &mut workspace)
            .unwrap();
        assert_f64_close(&output, &input, n);
    }
}

#[test]
fn real_batch_rejects_incomplete_rows_and_wrong_output_lengths() {
    let plan = RealFftPlan::<f64>::new(4).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let input = [0.0; 5];
    let mut spectrum = [Complex::new(0.0, 0.0); 6];
    assert_eq!(
        plan.forward_batch(&input, &mut spectrum, &mut workspace),
        Err(FftError::BatchBufferLength {
            buffer: "input",
            row_len: 4,
            actual: 5,
        })
    );

    let input = [0.0; 8];
    let mut wrong_spectrum = [Complex::new(0.0, 0.0); 4];
    assert_eq!(
        plan.forward_batch(&input, &mut wrong_spectrum, &mut workspace),
        Err(FftError::BufferLength {
            buffer: "spectrum",
            expected: 6,
            actual: 4,
        })
    );

    let incomplete_spectrum = [Complex::new(0.0, 0.0); 4];
    let mut output = [0.0; 4];
    assert_eq!(
        plan.inverse_batch(&incomplete_spectrum, &mut output, &mut workspace),
        Err(FftError::BatchBufferLength {
            buffer: "spectrum",
            row_len: 3,
            actual: 4,
        })
    );

    let other_plan = RealFftPlan::<f64>::new(3).unwrap();
    let mut other_workspace = other_plan.workspace().unwrap();
    let spectrum = [Complex::new(0.0, 0.0); 6];
    let mut output = [0.0; 8];
    assert_eq!(
        plan.inverse_batch(&spectrum, &mut output, &mut other_workspace),
        Err(FftError::BufferLength {
            buffer: "workspace",
            expected: 4,
            actual: 3,
        })
    );

    let mut empty_spectrum = [];
    let mut empty_output = [];
    plan.forward_batch(&[], &mut empty_spectrum, &mut workspace)
        .unwrap();
    plan.inverse_batch(&empty_spectrum, &mut empty_output, &mut workspace)
        .unwrap();
}

#[test]
fn f32_real_bluestein_batch_matches_independent_f64_dfts() {
    let n = 137;
    let rows = 2;
    let plan = RealFftPlan::<f32>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let input32: Vec<_> = (0..rows)
        .flat_map(|row| (0..n).map(move |index| real_signal(index + 11 * row, n) as f32))
        .collect();
    let expected_spectrum: Vec<_> = input32
        .chunks_exact(n)
        .map(|row| row.iter().map(|&value| value as f64).collect::<Vec<_>>())
        .flat_map(|row| direct_r2c_f64(&row))
        .collect();
    let mut actual_spectrum = vec![Complex::new(0.0, 0.0); rows * plan.spectrum_len()];
    plan.forward_batch(&input32, &mut actual_spectrum, &mut workspace)
        .unwrap();
    assert_complex_f32_close(&actual_spectrum, &expected_spectrum, n);

    let spectrum32: Vec<_> = (0..rows)
        .flat_map(|row| {
            (0..plan.spectrum_len()).map(move |index| {
                Complex::new(
                    real_signal(index + 23 * row, n + 3) as f32,
                    (0.2 * real_signal(index + 17 * row, n + 7)) as f32,
                )
            })
        })
        .collect();
    let spectrum64: Vec<_> = spectrum32
        .iter()
        .map(|value| Complex::new(value.re as f64, value.im as f64))
        .collect();
    let expected_output: Vec<_> = spectrum64
        .chunks_exact(plan.spectrum_len())
        .flat_map(|row| direct_c2r_f64(row, n))
        .collect();
    let mut actual_output = vec![0.0; rows * n];
    plan.inverse_batch(&spectrum32, &mut actual_output, &mut workspace)
        .unwrap();
    assert_f32_close(&actual_output, &expected_output, n);
}

#[test]
fn f32_real_complexify_batch_matches_independent_row_calls() {
    let n = 1026;
    let rows = 2;
    let plan = RealFftPlan::<f32>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let input: Vec<_> = (0..rows)
        .flat_map(|row| (0..n).map(move |index| real_signal(index + 5 * row, n) as f32))
        .collect();
    let mut batch_spectrum = vec![Complex::new(0.0, 0.0); rows * plan.spectrum_len()];
    plan.forward_batch(&input, &mut batch_spectrum, &mut workspace)
        .unwrap();
    let mut row_spectrum = vec![Complex::new(0.0, 0.0); rows * plan.spectrum_len()];
    for row in 0..rows {
        plan.forward(
            &input[row * n..(row + 1) * n],
            &mut row_spectrum[row * plan.spectrum_len()..(row + 1) * plan.spectrum_len()],
            &mut workspace,
        )
        .unwrap();
    }
    assert_eq!(batch_spectrum, row_spectrum);

    let mut batch_output = vec![0.0; rows * n];
    plan.inverse_batch(&batch_spectrum, &mut batch_output, &mut workspace)
        .unwrap();
    let mut row_output = vec![0.0; rows * n];
    for row in 0..rows {
        plan.inverse(
            &batch_spectrum[row * plan.spectrum_len()..(row + 1) * plan.spectrum_len()],
            &mut row_output[row * n..(row + 1) * n],
            &mut workspace,
        )
        .unwrap();
    }
    assert_eq!(batch_output, row_output);
}
