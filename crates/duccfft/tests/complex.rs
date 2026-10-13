use num_complex::Complex;

use duccfft::{ComplexFftPlan, FftError};

fn signal_f64(index: usize, len: usize) -> Complex<f64> {
    let x = (index + 1) as f64;
    let phase = x * 0.173 + len as f64 * 0.019;
    Complex::new(
        phase.sin() + 0.2 * (phase * 0.37).cos(),
        (phase * 0.61).cos(),
    )
}

fn signal_f32(index: usize, len: usize) -> Complex<f32> {
    let value = signal_f64(index, len);
    Complex::new(value.re as f32, value.im as f32)
}

fn random_signal_f64(len: usize, seed: u64) -> Vec<Complex<f64>> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            let mut next = || {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let unit = (state >> 11) as f64 / ((1_u64 << 53) - 1) as f64;
                2.0 * unit - 1.0
            };
            Complex::new(next(), next())
        })
        .collect()
}

fn direct_dft_f64(input: &[Complex<f64>], forward: bool) -> Vec<Complex<f64>> {
    let n = input.len();
    (0..n)
        .map(|frequency| {
            input
                .iter()
                .enumerate()
                .fold(Complex::new(0.0, 0.0), |sum, (index, &value)| {
                    let angle = std::f64::consts::TAU * (index * frequency) as f64 / n as f64;
                    let angle = if forward { -angle } else { angle };
                    sum + value * Complex::new(angle.cos(), angle.sin())
                })
        })
        .collect()
}

fn direct_dft_f32(input: &[Complex<f32>], forward: bool) -> Vec<Complex<f32>> {
    let n = input.len();
    (0..n)
        .map(|frequency| {
            input
                .iter()
                .enumerate()
                .fold(Complex::new(0.0, 0.0), |sum, (index, &value)| {
                    let angle = std::f64::consts::TAU * (index * frequency) as f64 / n as f64;
                    let angle = if forward { -angle } else { angle };
                    let weight = Complex::new(angle.cos() as f32, angle.sin() as f32);
                    sum + value * weight
                })
        })
        .collect()
}

fn normalized_direct_inverse<T: Copy>(mut values: Vec<Complex<T>>, scale: T) -> Vec<Complex<T>>
where
    Complex<T>: std::ops::Mul<T, Output = Complex<T>>,
{
    for value in &mut values {
        *value = *value * scale;
    }
    values
}

fn assert_f64_close(actual: &[Complex<f64>], expected: &[Complex<f64>], n: usize) {
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        let scale = (n as f64)
            .max(expected.re.abs())
            .max(expected.im.abs())
            .max(1.0);
        let tolerance = 2.0e-12 * scale;
        assert!(
            (actual.re - expected.re).abs() <= tolerance,
            "real mismatch at {index}: actual={}, expected={}, tolerance={tolerance}",
            actual.re,
            expected.re
        );
        assert!(
            (actual.im - expected.im).abs() <= tolerance,
            "imaginary mismatch at {index}: actual={}, expected={}, tolerance={tolerance}",
            actual.im,
            expected.im
        );
    }
}

fn assert_f32_close(actual: &[Complex<f32>], expected: &[Complex<f32>], n: usize) {
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        let scale = (n as f32)
            .sqrt()
            .max(expected.re.abs())
            .max(expected.im.abs())
            .max(1.0);
        let tolerance = 4.0e-6 * scale;
        assert!(
            (actual.re - expected.re).abs() <= tolerance,
            "real mismatch at {index}: actual={}, expected={}, tolerance={tolerance}",
            actual.re,
            expected.re
        );
        assert!(
            (actual.im - expected.im).abs() <= tolerance,
            "imaginary mismatch at {index}: actual={}, expected={}, tolerance={tolerance}",
            actual.im,
            expected.im
        );
    }
}

fn report_f32_complex_metrics(label: &str, actual: &[Complex<f32>], expected: &[Complex<f64>]) {
    let mut max_absolute = 0.0_f64;
    let mut max_expected = 0.0_f64;
    let mut squared_error = 0.0_f64;
    let mut squared_reference = 0.0_f64;
    let mut nonfinite = 0;
    for (&actual, &expected) in actual.iter().zip(expected) {
        let real_error = actual.re as f64 - expected.re;
        let imag_error = actual.im as f64 - expected.im;
        let error_norm = real_error.hypot(imag_error);
        max_absolute = max_absolute.max(error_norm);
        max_expected = max_expected.max(expected.norm());
        squared_error += error_norm * error_norm;
        squared_reference += expected.norm_sqr();
        nonfinite += usize::from(!actual.re.is_finite() || !actual.im.is_finite());
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

#[test]
fn f64_mixed_radix_matches_direct_dft_for_lengths_one_through_sixty_four() {
    for n in 1..=64 {
        let original: Vec<_> = (0..n).map(|index| signal_f64(index, n)).collect();
        let expected = direct_dft_f64(&original, true);
        let plan = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut actual = original.clone();

        plan.forward(&mut actual, &mut workspace).unwrap();
        assert_f64_close(&actual, &expected, n);

        let independent_spectrum: Vec<_> =
            (0..n).map(|index| signal_f64(index + 11, n + 3)).collect();
        let inverse_expected =
            normalized_direct_inverse(direct_dft_f64(&independent_spectrum, false), 1.0 / n as f64);
        let mut inverse_actual = independent_spectrum;
        plan.inverse(&mut inverse_actual, &mut workspace).unwrap();
        assert_f64_close(&inverse_actual, &inverse_expected, n);

        plan.inverse(&mut actual, &mut workspace).unwrap();
        assert_f64_close(&actual, &original, n);
    }
}

#[test]
fn f32_mixed_radix_matches_direct_dft_for_lengths_one_through_sixty_four() {
    for n in 1..=64 {
        let original: Vec<_> = (0..n).map(|index| signal_f32(index, n)).collect();
        let expected = direct_dft_f32(&original, true);
        let plan = ComplexFftPlan::<f32>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut actual = original.clone();

        plan.forward(&mut actual, &mut workspace).unwrap();
        assert_f32_close(&actual, &expected, n);

        let independent_spectrum: Vec<_> =
            (0..n).map(|index| signal_f32(index + 11, n + 3)).collect();
        let inverse_expected =
            normalized_direct_inverse(direct_dft_f32(&independent_spectrum, false), 1.0 / n as f32);
        let mut inverse_actual = independent_spectrum;
        plan.inverse(&mut inverse_actual, &mut workspace).unwrap();
        assert_f32_close(&inverse_actual, &inverse_expected, n);

        plan.inverse(&mut actual, &mut workspace).unwrap();
        assert_f32_close(&actual, &original, n);
    }
}

#[test]
fn f32_large_smooth_prime_bluestein_and_sht_lengths_match_f64_references() {
    println!("precision,operation,n,max_abs,rms,relative_l2,scaled_max,nonfinite");
    for n in [1024, 127, 137, 4097] {
        let original32: Vec<_> = (0..n).map(|index| signal_f32(index, n)).collect();
        let original64: Vec<_> = original32
            .iter()
            .map(|value| Complex::new(value.re as f64, value.im as f64))
            .collect();
        let expected_forward = direct_dft_f64(&original64, true);
        let plan = ComplexFftPlan::<f32>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut actual_forward = original32;
        plan.forward(&mut actual_forward, &mut workspace).unwrap();
        report_f32_complex_metrics("c2c_forward", &actual_forward, &expected_forward);

        let spectrum32: Vec<_> = (0..n).map(|index| signal_f32(index + 17, n + 3)).collect();
        let spectrum64: Vec<_> = spectrum32
            .iter()
            .map(|value| Complex::new(value.re as f64, value.im as f64))
            .collect();
        let expected_inverse =
            normalized_direct_inverse(direct_dft_f64(&spectrum64, false), 1.0 / n as f64);
        let mut actual_inverse = spectrum32;
        plan.inverse(&mut actual_inverse, &mut workspace).unwrap();
        report_f32_complex_metrics("c2c_inverse", &actual_inverse, &expected_inverse);
    }
}

#[test]
fn fixed_radices_match_analytic_impulses_constants_and_fourier_modes() {
    for n in [7, 8, 11, 16] {
        let plan = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();

        let mut impulse = vec![Complex::new(0.0, 0.0); n];
        impulse[0] = Complex::new(1.0, 0.0);
        plan.forward(&mut impulse, &mut workspace).unwrap();
        assert_f64_close(&impulse, &vec![Complex::new(1.0, 0.0); n], n);

        let value = Complex::new(0.25, -0.5);
        let mut constant = vec![value; n];
        plan.forward(&mut constant, &mut workspace).unwrap();
        let mut expected = vec![Complex::new(0.0, 0.0); n];
        expected[0] = value * n as f64;
        assert_f64_close(&constant, &expected, n);

        let mode = n / 3;
        let mut fourier_mode: Vec<_> = (0..n)
            .map(|index| {
                let phase = std::f64::consts::TAU * (mode * index) as f64 / n as f64;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();
        plan.forward(&mut fourier_mode, &mut workspace).unwrap();
        expected.fill(Complex::new(0.0, 0.0));
        expected[mode] = Complex::new(n as f64, 0.0);
        assert_f64_close(&fourier_mode, &expected, n);

        let random = random_signal_f64(n, 0x9e37_79b9_7f4a_7c15 ^ n as u64);
        let expected = direct_dft_f64(&random, true);
        let mut actual = random;
        plan.forward(&mut actual, &mut workspace).unwrap();
        assert_f64_close(&actual, &expected, n);
    }
}

#[test]
fn general_prime_and_bluestein_paths_match_direct_dfts() {
    for n in [107, 109, 113, 137, 241] {
        let input: Vec<_> = (0..n).map(|index| signal_f64(index + 5, n + 2)).collect();
        let expected = direct_dft_f64(&input, true);
        let plan = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut actual = input.clone();
        plan.forward(&mut actual, &mut workspace).unwrap();
        assert_f64_close(&actual, &expected, n);

        let independent_spectrum: Vec<_> =
            (0..n).map(|index| signal_f64(index + 19, n + 7)).collect();
        let expected_inverse =
            normalized_direct_inverse(direct_dft_f64(&independent_spectrum, false), 1.0 / n as f64);
        let mut actual_inverse = independent_spectrum;
        plan.inverse(&mut actual_inverse, &mut workspace).unwrap();
        assert_f64_close(&actual_inverse, &expected_inverse, n);
    }

    let n = 137;
    let input: Vec<_> = (0..n).map(|index| signal_f32(index + 3, n)).collect();
    let expected = direct_dft_f32(&input, true);
    let plan = ComplexFftPlan::<f32>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut actual = input;
    plan.forward(&mut actual, &mut workspace).unwrap();
    assert_f32_close(&actual, &expected, n);

    let independent_spectrum: Vec<_> = (0..n).map(|index| signal_f32(index + 23, n + 7)).collect();
    let expected_inverse =
        normalized_direct_inverse(direct_dft_f32(&independent_spectrum, false), 1.0 / n as f32);
    let mut actual_inverse = independent_spectrum;
    plan.inverse(&mut actual_inverse, &mut workspace).unwrap();
    assert_f32_close(&actual_inverse, &expected_inverse, n);
}

#[test]
fn bluestein_handles_representable_high_amplitude_impulses() {
    let plan64 = ComplexFftPlan::<f64>::new(113).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let amplitude64 = f64::MAX / 4.0;
    let mut impulse64 = vec![Complex::new(0.0, 0.0); plan64.len()];
    impulse64[0] = Complex::new(amplitude64, 0.0);
    plan64.forward(&mut impulse64, &mut workspace64).unwrap();
    assert!(
        impulse64
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_f64_close(
        &impulse64,
        &vec![Complex::new(amplitude64, 0.0); plan64.len()],
        plan64.len(),
    );

    let plan32 = ComplexFftPlan::<f32>::new(113).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let amplitude32 = f32::MAX / 4.0;
    let mut impulse32 = vec![Complex::new(0.0, 0.0); plan32.len()];
    impulse32[0] = Complex::new(amplitude32, 0.0);
    plan32.forward(&mut impulse32, &mut workspace32).unwrap();
    assert!(
        impulse32
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_f32_close(
        &impulse32,
        &vec![Complex::new(amplitude32, 0.0); plan32.len()],
        plan32.len(),
    );
}

#[test]
fn bluestein_preserves_subnormal_impulses_and_constant_inverse_spectra() {
    let n = 113;
    let tiny64 = f64::from_bits(1);
    let plan64 = ComplexFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let mut impulse64 = vec![Complex::new(0.0, 0.0); n];
    impulse64[0] = Complex::new(tiny64, 0.0);
    plan64.forward(&mut impulse64, &mut workspace64).unwrap();
    assert_eq!(impulse64, vec![Complex::new(tiny64, 0.0); n]);

    let mut constant_spectrum64 = vec![Complex::new(tiny64, 0.0); n];
    plan64
        .inverse(&mut constant_spectrum64, &mut workspace64)
        .unwrap();
    let mut expected64 = vec![Complex::new(0.0, 0.0); n];
    expected64[0] = Complex::new(tiny64, 0.0);
    assert_eq!(constant_spectrum64, expected64);

    let tiny32 = f32::from_bits(1);
    let plan32 = ComplexFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let mut impulse32 = vec![Complex::new(0.0, 0.0); n];
    impulse32[0] = Complex::new(tiny32, 0.0);
    plan32.forward(&mut impulse32, &mut workspace32).unwrap();
    assert_eq!(impulse32, vec![Complex::new(tiny32, 0.0); n]);

    let mut constant_spectrum32 = vec![Complex::new(tiny32, 0.0); n];
    plan32
        .inverse(&mut constant_spectrum32, &mut workspace32)
        .unwrap();
    let mut expected32 = vec![Complex::new(0.0, 0.0); n];
    expected32[0] = Complex::new(tiny32, 0.0);
    assert_eq!(constant_spectrum32, expected32);
}

#[test]
fn bluestein_scales_high_amplitude_inputs_before_the_convolution_fft() {
    let n = 113;
    let unit_input64: Vec<_> = (0..n)
        .map(|index| {
            let phase = std::f64::consts::PI * (index * index) as f64 / n as f64;
            Complex::new(phase.cos(), phase.sin())
        })
        .collect();
    let unit_expected64 = direct_dft_f64(&unit_input64, true);
    let amplitude64 = f64::MAX / 100.0;
    let expected64: Vec<_> = unit_expected64
        .iter()
        .map(|&value| value * amplitude64)
        .collect();
    let mut actual64: Vec<_> = unit_input64
        .iter()
        .map(|&value| value * amplitude64)
        .collect();
    let plan64 = ComplexFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    plan64.forward(&mut actual64, &mut workspace64).unwrap();
    assert!(
        actual64
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_f64_close(&actual64, &expected64, n);

    let unit_input32: Vec<_> = (0..n)
        .map(|index| {
            let phase = std::f64::consts::PI * (index * index) as f64 / n as f64;
            Complex::new(phase.cos() as f32, phase.sin() as f32)
        })
        .collect();
    let unit_expected32 = direct_dft_f32(&unit_input32, true);
    let amplitude32 = f32::MAX / 100.0;
    let expected32: Vec<_> = unit_expected32
        .iter()
        .map(|&value| value * amplitude32)
        .collect();
    let mut actual32: Vec<_> = unit_input32
        .iter()
        .map(|&value| value * amplitude32)
        .collect();
    let plan32 = ComplexFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    plan32.forward(&mut actual32, &mut workspace32).unwrap();
    assert!(
        actual32
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_f32_close(&actual32, &expected32, n);
}

#[test]
fn nonterminal_bluestein_stages_match_rustfft() {
    use rustfft::FftPlanner;

    let n = 113 * 113;
    let input: Vec<_> = (0..n).map(|index| signal_f64(index + 7, n + 3)).collect();
    let mut expected = input.clone();
    FftPlanner::<f64>::new()
        .plan_fft_forward(n)
        .process(&mut expected);

    let plan = ComplexFftPlan::<f64>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut actual = input;
    plan.forward(&mut actual, &mut workspace).unwrap();
    assert_f64_close(&actual, &expected, n);
}

#[test]
fn impossible_allocation_size_is_rejected_before_factorization() {
    let len = usize::MAX - 58;
    assert!(matches!(
        ComplexFftPlan::<f64>::new(len),
        Err(FftError::AllocationFailed)
    ));
}

#[test]
fn normalized_inverse_handles_maximum_finite_constant_spectra() {
    let plan = ComplexFftPlan::<f64>::new(2).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut values = [Complex::new(f64::MAX, 0.0); 2];
    plan.inverse(&mut values, &mut workspace).unwrap();
    assert!(
        values
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_eq!(values[0], Complex::new(f64::MAX, 0.0));
    assert_eq!(values[1], Complex::new(0.0, 0.0));

    let plan = ComplexFftPlan::<f32>::new(2).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut values = [Complex::new(f32::MAX, 0.0); 2];
    plan.inverse(&mut values, &mut workspace).unwrap();
    assert!(
        values
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_eq!(values[0], Complex::new(f32::MAX, 0.0));
    assert_eq!(values[1], Complex::new(0.0, 0.0));
}

#[test]
fn normalized_inverse_preserves_representable_subnormal_results() {
    let tiny64 = f64::from_bits(1);
    let plan64 = ComplexFftPlan::<f64>::new(2).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let mut values64 = [Complex::new(tiny64, 0.0); 2];
    plan64.inverse(&mut values64, &mut workspace64).unwrap();
    assert_eq!(
        values64,
        [Complex::new(tiny64, 0.0), Complex::new(0.0, 0.0)]
    );
    let mut values64 = [Complex::new(tiny64, 0.0), Complex::new(-tiny64, 0.0)];
    plan64.inverse(&mut values64, &mut workspace64).unwrap();
    assert_eq!(
        values64,
        [Complex::new(0.0, 0.0), Complex::new(tiny64, 0.0)]
    );

    let tiny32 = f32::from_bits(1);
    let plan32 = ComplexFftPlan::<f32>::new(2).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let mut values32 = [Complex::new(tiny32, 0.0); 2];
    plan32.inverse(&mut values32, &mut workspace32).unwrap();
    assert_eq!(
        values32,
        [Complex::new(tiny32, 0.0), Complex::new(0.0, 0.0)]
    );
    let mut values32 = [Complex::new(tiny32, 0.0), Complex::new(-tiny32, 0.0)];
    plan32.inverse(&mut values32, &mut workspace32).unwrap();
    assert_eq!(
        values32,
        [Complex::new(0.0, 0.0), Complex::new(tiny32, 0.0)]
    );
}

#[test]
fn normalized_inverse_preserves_small_terms_in_high_dynamic_range_groups() {
    let tiny64 = f64::from_bits(1);
    let plan64 = ComplexFftPlan::<f64>::new(4).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let mut values64 = [
        Complex::new(f64::MAX, 0.0),
        Complex::new(2.0 * tiny64, 0.0),
        Complex::new(-f64::MAX, 0.0),
        Complex::new(2.0 * tiny64, 0.0),
    ];
    plan64.inverse(&mut values64, &mut workspace64).unwrap();
    assert_eq!(
        values64,
        [
            Complex::new(tiny64, 0.0),
            Complex::new(f64::MAX / 2.0, 0.0),
            Complex::new(-tiny64, 0.0),
            Complex::new(f64::MAX / 2.0, 0.0),
        ]
    );

    let tiny32 = f32::from_bits(1);
    let plan32 = ComplexFftPlan::<f32>::new(4).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let mut values32 = [
        Complex::new(f32::MAX, 0.0),
        Complex::new(2.0 * tiny32, 0.0),
        Complex::new(-f32::MAX, 0.0),
        Complex::new(2.0 * tiny32, 0.0),
    ];
    plan32.inverse(&mut values32, &mut workspace32).unwrap();
    assert_eq!(
        values32,
        [
            Complex::new(tiny32, 0.0),
            Complex::new(f32::MAX / 2.0, 0.0),
            Complex::new(-tiny32, 0.0),
            Complex::new(f32::MAX / 2.0, 0.0),
        ]
    );
}

#[test]
fn invalid_lengths_and_buffers_return_errors() {
    assert!(matches!(
        ComplexFftPlan::<f64>::new(0),
        Err(FftError::InvalidLength)
    ));
    assert!(matches!(
        ComplexFftPlan::<f64>::new(usize::MAX),
        Err(FftError::AllocationFailed)
    ));

    let plan = ComplexFftPlan::<f64>::new(4).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut short_values = [Complex::new(0.0, 0.0); 3];
    assert_eq!(
        plan.forward(&mut short_values, &mut workspace),
        Err(FftError::BufferLength {
            buffer: "values",
            expected: 4,
            actual: 3,
        })
    );

    let other_plan = ComplexFftPlan::<f64>::new(3).unwrap();
    let mut short_workspace = other_plan.workspace().unwrap();
    let mut values = [Complex::new(0.0, 0.0); 4];
    assert_eq!(
        plan.forward(&mut values, &mut short_workspace),
        Err(FftError::BufferLength {
            buffer: "workspace",
            expected: 4,
            actual: 3,
        })
    );
}

#[test]
fn workspace_reports_reusable_scratch_bytes() {
    let plan = ComplexFftPlan::<f64>::new(8).unwrap();
    let workspace = plan.workspace().unwrap();
    assert_eq!(
        workspace.scratch_bytes(),
        Some(24 * std::mem::size_of::<Complex<f64>>())
    );

    let blue_plan = ComplexFftPlan::<f64>::new(137).unwrap();
    let blue_workspace = blue_plan.workspace().unwrap();
    assert!(blue_workspace.scratch_bytes().unwrap() > workspace.scratch_bytes().unwrap());
}

#[test]
fn shared_plan_executes_concurrently_with_separate_workspaces() {
    let plan = ComplexFftPlan::<f64>::new(45).unwrap();
    std::thread::scope(|scope| {
        for row in 0..4 {
            let plan = &plan;
            scope.spawn(move || {
                let mut workspace = plan.workspace().unwrap();
                let mut values: Vec<_> = (0..plan.len())
                    .map(|index| signal_f64(index + row, plan.len()))
                    .collect();
                let original = values.clone();
                plan.forward(&mut values, &mut workspace).unwrap();
                plan.inverse(&mut values, &mut workspace).unwrap();
                assert_f64_close(&values, &original, plan.len());
            });
        }
    });
}

#[test]
fn shared_f32_plan_executes_concurrently_with_separate_workspaces() {
    let plan = ComplexFftPlan::<f32>::new(40).unwrap();
    std::thread::scope(|scope| {
        for row in 0..4 {
            let plan = &plan;
            scope.spawn(move || {
                let mut workspace = plan.workspace().unwrap();
                let original: Vec<_> = (0..plan.len())
                    .map(|index| signal_f32(index + row, plan.len()))
                    .collect();
                let expected = direct_dft_f32(&original, true);
                let mut values = original;
                plan.forward(&mut values, &mut workspace).unwrap();
                assert_f32_close(&values, &expected, plan.len());
            });
        }
    });
}

#[test]
fn contiguous_complex_batches_match_independent_dfts() {
    for n in [16, 17] {
        let rows = 3;
        let plan = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let input: Vec<_> = (0..rows)
            .flat_map(|row| (0..n).map(move |index| signal_f64(index + 11 * row, n)))
            .collect();
        let expected_forward: Vec<_> = input
            .chunks_exact(n)
            .flat_map(|row| direct_dft_f64(row, true))
            .collect();
        let expected_inverse: Vec<_> = expected_forward
            .chunks_exact(n)
            .flat_map(|row| normalized_direct_inverse(direct_dft_f64(row, false), 1.0 / n as f64))
            .collect();

        let mut values = input.clone();
        plan.forward_batch(&mut values, &mut workspace).unwrap();
        assert_f64_close(&values, &expected_forward, n);
        plan.inverse_batch(&mut values, &mut workspace).unwrap();
        assert_f64_close(&values, &expected_inverse, n);
        assert_f64_close(&values, &input, n);
    }
}

#[test]
fn complex_batch_rejects_incomplete_rows_and_accepts_empty_batches() {
    let plan = ComplexFftPlan::<f64>::new(4).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut incomplete = vec![Complex::new(0.0, 0.0); 5];
    assert_eq!(
        plan.forward_batch(&mut incomplete, &mut workspace),
        Err(FftError::BatchBufferLength {
            buffer: "values",
            row_len: 4,
            actual: 5,
        })
    );
    let mut incomplete = vec![Complex::new(0.0, 0.0); 5];
    assert_eq!(
        plan.inverse_batch(&mut incomplete, &mut workspace),
        Err(FftError::BatchBufferLength {
            buffer: "values",
            row_len: 4,
            actual: 5,
        })
    );
    let other_plan = ComplexFftPlan::<f64>::new(3).unwrap();
    let mut other_workspace = other_plan.workspace().unwrap();
    let mut values = vec![Complex::new(0.0, 0.0); 8];
    assert_eq!(
        plan.forward_batch(&mut values, &mut other_workspace),
        Err(FftError::BufferLength {
            buffer: "workspace",
            expected: 4,
            actual: 3,
        })
    );
    let mut empty = [];
    plan.forward_batch(&mut empty, &mut workspace).unwrap();
    plan.inverse_batch(&mut empty, &mut workspace).unwrap();
}

#[test]
fn f32_complex_bluestein_batch_matches_independent_f64_dfts() {
    let n = 137;
    let rows = 2;
    let plan = ComplexFftPlan::<f32>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let input32: Vec<_> = (0..rows)
        .flat_map(|row| (0..n).map(move |index| signal_f32(index + 7 * row, n)))
        .collect();
    let input64: Vec<_> = input32
        .iter()
        .map(|value| Complex::new(value.re as f64, value.im as f64))
        .collect();
    let expected_forward: Vec<_> = input64
        .chunks_exact(n)
        .flat_map(|row| direct_dft_f64(row, true))
        .map(|value| Complex::new(value.re as f32, value.im as f32))
        .collect();

    let mut values = input32;
    plan.forward_batch(&mut values, &mut workspace).unwrap();
    assert_f32_close(&values, &expected_forward, n);

    let spectrum32: Vec<_> = (0..rows)
        .flat_map(|row| (0..n).map(move |index| signal_f32(index + 19 * row, n + 3)))
        .collect();
    let spectrum64: Vec<_> = spectrum32
        .iter()
        .map(|value| Complex::new(value.re as f64, value.im as f64))
        .collect();
    let expected_inverse: Vec<_> = spectrum64
        .chunks_exact(n)
        .flat_map(|row| normalized_direct_inverse(direct_dft_f64(row, false), 1.0 / n as f64))
        .map(|value| Complex::new(value.re as f32, value.im as f32))
        .collect();
    let mut inverse_values = spectrum32;
    plan.inverse_batch(&mut inverse_values, &mut workspace)
        .unwrap();
    assert_f32_close(&inverse_values, &expected_inverse, n);
}
