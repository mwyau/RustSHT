use num_complex::Complex;

use rustsht_fft::{ComplexFftPlan, FftError};

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

fn assert_f64_close_at_scale(actual: &[Complex<f64>], expected: &[Complex<f64>], scale: f64) {
    let tolerance = 2.0e-10 * scale;
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            actual.re.is_finite() && actual.im.is_finite(),
            "index={index}"
        );
        assert!(
            (actual.re - expected.re).abs() <= tolerance
                && (actual.im - expected.im).abs() <= tolerance,
            "index={index}, actual={actual}, expected={expected}, tolerance={tolerance}"
        );
    }
}

fn assert_f32_close_at_scale(actual: &[Complex<f32>], expected: &[Complex<f64>], scale: f64) {
    let tolerance = 5.0e-5 * scale;
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            actual.re.is_finite() && actual.im.is_finite(),
            "index={index}"
        );
        assert!(
            (actual.re as f64 - expected.re).abs() <= tolerance
                && (actual.im as f64 - expected.im).abs() <= tolerance,
            "index={index}, actual={actual}, expected={expected}, tolerance={tolerance}"
        );
    }
}

fn unaligned_offset<T>(storage: &[T]) -> usize {
    (1..=4)
        .find(|&offset| {
            !(storage.as_ptr() as usize + offset * std::mem::size_of::<T>()).is_multiple_of(32)
        })
        .expect("slack must contain a 32-byte-unaligned slice")
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
    for n in [
        150, 240, 256, 300, 484, 512, 726, 1024, 1372, 1452, 2904, 127, 137, 4097,
    ] {
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
fn f64_large_smooth_multipass_matches_independent_dfts() {
    let n = 1024;
    let input = random_signal_f64(n, 0x8f3c_71a2_d905_b64e);
    let expected_forward = direct_dft_f64(&input, true);
    let plan = ComplexFftPlan::<f64>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut actual_forward = input;
    plan.forward(&mut actual_forward, &mut workspace).unwrap();
    assert_f64_close(&actual_forward, &expected_forward, n);

    let spectrum = random_signal_f64(n, 0x6c21_4b7e_a930_d58f);
    let expected_inverse =
        normalized_direct_inverse(direct_dft_f64(&spectrum, false), 1.0 / n as f64);
    let mut actual_inverse = spectrum;
    plan.inverse(&mut actual_inverse, &mut workspace).unwrap();
    assert_f64_close(&actual_inverse, &expected_inverse, n);
}

#[test]
fn f64_sht_meridional_complex_lengths_match_independent_dfts() {
    for n in [
        146, 150, 240, 256, 300, 484, 512, 722, 726, 1024, 1372, 1442, 1452, 2904,
    ] {
        let input = random_signal_f64(n, 0x1b73_5a94_c6d8_e20f ^ n as u64);
        let expected_forward = direct_dft_f64(&input, true);
        let plan = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut actual_forward = input;
        plan.forward(&mut actual_forward, &mut workspace).unwrap();
        assert_f64_close(&actual_forward, &expected_forward, n);

        let spectrum = random_signal_f64(n, 0x5e29_14a7_b3c8_d60f ^ n as u64);
        let expected_inverse =
            normalized_direct_inverse(direct_dft_f64(&spectrum, false), 1.0 / n as f64);
        let mut actual_inverse = spectrum;
        plan.inverse(&mut actual_inverse, &mut workspace).unwrap();
        assert_f64_close(&actual_inverse, &expected_inverse, n);
    }
}

#[test]
fn m3_vector_prefix_handles_unaligned_f32_and_f64_slices() {
    for n in [300, 2904] {
        let plan64 = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace64 = plan64.workspace().unwrap();
        let input64: Vec<_> = (0..n).map(|index| signal_f64(index, n)).collect();
        let expected_forward64 = direct_dft_f64(&input64, true);
        let mut storage64 = vec![Complex::new(-7.0, 9.0); n + 5];
        let offset64 = unaligned_offset(&storage64);
        storage64[offset64..offset64 + n].copy_from_slice(&input64);
        let forward64 = &mut storage64[offset64..offset64 + n];
        assert_ne!((forward64.as_ptr() as usize) % 32, 0);
        plan64.forward(forward64, &mut workspace64).unwrap();
        assert_f64_close(forward64, &expected_forward64, n);
        assert_eq!(storage64[offset64 - 1], Complex::new(-7.0, 9.0));
        assert_eq!(storage64[offset64 + n], Complex::new(-7.0, 9.0));

        let spectrum64: Vec<_> = (0..n).map(|index| signal_f64(index + 17, n + 3)).collect();
        let expected_inverse64 =
            normalized_direct_inverse(direct_dft_f64(&spectrum64, false), 1.0 / n as f64);
        storage64[offset64..offset64 + n].copy_from_slice(&spectrum64);
        let inverse64 = &mut storage64[offset64..offset64 + n];
        plan64.inverse(inverse64, &mut workspace64).unwrap();
        assert_f64_close(inverse64, &expected_inverse64, n);
        assert_eq!(storage64[offset64 - 1], Complex::new(-7.0, 9.0));
        assert_eq!(storage64[offset64 + n], Complex::new(-7.0, 9.0));

        let plan32 = ComplexFftPlan::<f32>::new(n).unwrap();
        let mut workspace32 = plan32.workspace().unwrap();
        let input32: Vec<_> = input64
            .iter()
            .map(|value| Complex::new(value.re as f32, value.im as f32))
            .collect();
        let input32_as64: Vec<_> = input32
            .iter()
            .map(|value| Complex::new(value.re as f64, value.im as f64))
            .collect();
        let expected_forward32 = direct_dft_f64(&input32_as64, true);
        let mut storage32 = vec![Complex::new(-7.0_f32, 9.0_f32); n + 5];
        let offset32 = unaligned_offset(&storage32);
        storage32[offset32..offset32 + n].copy_from_slice(&input32);
        let forward32 = &mut storage32[offset32..offset32 + n];
        assert_ne!((forward32.as_ptr() as usize) % 32, 0);
        plan32.forward(forward32, &mut workspace32).unwrap();
        assert_f32_close_at_scale(forward32, &expected_forward32, n as f64);
        assert_eq!(storage32[offset32 - 1], Complex::new(-7.0, 9.0));
        assert_eq!(storage32[offset32 + n], Complex::new(-7.0, 9.0));

        let spectrum32: Vec<_> = spectrum64
            .iter()
            .map(|value| Complex::new(value.re as f32, value.im as f32))
            .collect();
        let spectrum32_as64: Vec<_> = spectrum32
            .iter()
            .map(|value| Complex::new(value.re as f64, value.im as f64))
            .collect();
        let expected_inverse32 =
            normalized_direct_inverse(direct_dft_f64(&spectrum32_as64, false), 1.0 / n as f64);
        storage32[offset32..offset32 + n].copy_from_slice(&spectrum32);
        let inverse32 = &mut storage32[offset32..offset32 + n];
        plan32.inverse(inverse32, &mut workspace32).unwrap();
        assert_f32_close_at_scale(inverse32, &expected_inverse32, n as f64);
        assert_eq!(storage32[offset32 - 1], Complex::new(-7.0, 9.0));
        assert_eq!(storage32[offset32 + n], Complex::new(-7.0, 9.0));
    }
}

#[test]
fn f64_4096_smooth_transform_matches_rustfft_forward_and_inverse() {
    use rustfft::FftPlanner;

    let n = 4096;
    let input = random_signal_f64(n, 0x71d3_9a4c_2e85_b60f);
    let mut expected_forward = input.clone();
    FftPlanner::<f64>::new()
        .plan_fft_forward(n)
        .process(&mut expected_forward);

    let plan = ComplexFftPlan::<f64>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut actual_forward = input;
    plan.forward(&mut actual_forward, &mut workspace).unwrap();
    assert_f64_close(&actual_forward, &expected_forward, n);

    let spectrum = random_signal_f64(n, 0x2c8f_4e17_a563_d901);
    let mut expected_inverse = spectrum.clone();
    FftPlanner::<f64>::new()
        .plan_fft_inverse(n)
        .process(&mut expected_inverse);
    let expected_inverse = normalized_direct_inverse(expected_inverse, 1.0 / n as f64);
    let mut actual_inverse = spectrum;
    plan.inverse(&mut actual_inverse, &mut workspace).unwrap();
    assert_f64_close(&actual_inverse, &expected_inverse, n);
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
    for n in [13, 17, 19, 23, 31, 41, 53, 73, 103, 107, 109, 113, 137, 241] {
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

    for n in [13, 17, 19, 31, 41, 73, 103, 109] {
        let input: Vec<_> = (0..n).map(|index| signal_f32(index + 11, n)).collect();
        let expected = direct_dft_f32(&input, true);
        let plan = ComplexFftPlan::<f32>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut actual = input;
        plan.forward(&mut actual, &mut workspace).unwrap();
        assert_f32_close(&actual, &expected, n);

        let independent_spectrum: Vec<_> =
            (0..n).map(|index| signal_f32(index + 23, n + 7)).collect();
        let expected_inverse =
            normalized_direct_inverse(direct_dft_f32(&independent_spectrum, false), 1.0 / n as f32);
        let mut actual_inverse = independent_spectrum;
        plan.inverse(&mut actual_inverse, &mut workspace).unwrap();
        assert_f32_close(&actual_inverse, &expected_inverse, n);
    }
}

#[test]
fn p241_bluestein_pad_matches_f64_reference_for_f32() {
    let n = 241;
    let input32: Vec<_> = (0..n).map(|index| signal_f32(index + 5, n + 2)).collect();
    let input64: Vec<_> = input32
        .iter()
        .map(|value| Complex::new(value.re as f64, value.im as f64))
        .collect();
    let expected_forward: Vec<_> = direct_dft_f64(&input64, true)
        .into_iter()
        .map(|value| Complex::new(value.re as f32, value.im as f32))
        .collect();
    let plan = ComplexFftPlan::<f32>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut actual_forward = input32;
    plan.forward(&mut actual_forward, &mut workspace).unwrap();
    assert_f32_close(&actual_forward, &expected_forward, n);

    let spectrum32: Vec<_> = (0..n).map(|index| signal_f32(index + 19, n + 7)).collect();
    let spectrum64: Vec<_> = spectrum32
        .iter()
        .map(|value| Complex::new(value.re as f64, value.im as f64))
        .collect();
    let expected_inverse: Vec<_> =
        normalized_direct_inverse(direct_dft_f64(&spectrum64, false), 1.0 / n as f64)
            .into_iter()
            .map(|value| Complex::new(value.re as f32, value.im as f32))
            .collect();
    let mut actual_inverse = spectrum32;
    plan.inverse(&mut actual_inverse, &mut workspace).unwrap();
    assert_f32_close(&actual_inverse, &expected_inverse, n);
}

#[test]
fn direct_general_primes_preserve_analytic_modes_extremes_and_subnormals() {
    for n in [13, 17, 19, 31, 41, 73, 103, 109] {
        let plan = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace = plan.workspace().unwrap();

        for forward in [true, false] {
            let constant_value = Complex::new(0.25, -0.5);
            let mut constant = vec![constant_value; n];
            let mut expected = direct_dft_f64(&constant, forward);
            if !forward {
                expected = normalized_direct_inverse(expected, 1.0 / n as f64);
            }
            if forward {
                plan.forward(&mut constant, &mut workspace).unwrap();
            } else {
                plan.inverse(&mut constant, &mut workspace).unwrap();
            }
            assert_f64_close(&constant, &expected, n);

            let mut impulse = vec![Complex::new(0.0, 0.0); n];
            impulse[n / 3] = Complex::new(0.75, -0.125);
            let mut expected = direct_dft_f64(&impulse, forward);
            if !forward {
                expected = normalized_direct_inverse(expected, 1.0 / n as f64);
            }
            if forward {
                plan.forward(&mut impulse, &mut workspace).unwrap();
            } else {
                plan.inverse(&mut impulse, &mut workspace).unwrap();
            }
            assert_f64_close(&impulse, &expected, n);

            let mode = n / 3;
            let sign = if forward { 1.0 } else { -1.0 };
            let mut fourier_mode: Vec<_> = (0..n)
                .map(|index| {
                    let phase = sign * std::f64::consts::TAU * (mode * index) as f64 / n as f64;
                    Complex::new(phase.cos(), phase.sin())
                })
                .collect();
            let mut expected = direct_dft_f64(&fourier_mode, forward);
            if !forward {
                expected = normalized_direct_inverse(expected, 1.0 / n as f64);
            }
            if forward {
                plan.forward(&mut fourier_mode, &mut workspace).unwrap();
            } else {
                plan.inverse(&mut fourier_mode, &mut workspace).unwrap();
            }
            assert_f64_close(&fourier_mode, &expected, n);
        }

        let amplitude = f64::MAX / 4.0;
        let mut large_impulse = vec![Complex::new(0.0, 0.0); n];
        large_impulse[0] = Complex::new(amplitude, 0.0);
        plan.forward(&mut large_impulse, &mut workspace).unwrap();
        assert!(
            large_impulse
                .iter()
                .all(|value| value.re.is_finite() && value.im.is_finite())
        );
        assert_f64_close(&large_impulse, &vec![Complex::new(amplitude, 0.0); n], n);
    }

    let n = 103;
    let tiny64 = f64::from_bits(1);
    let plan64 = ComplexFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let mut impulse64 = vec![Complex::new(0.0, 0.0); n];
    impulse64[0] = Complex::new(tiny64, 0.0);
    plan64.forward(&mut impulse64, &mut workspace64).unwrap();
    assert_eq!(impulse64, vec![Complex::new(tiny64, 0.0); n]);

    let tiny32 = f32::from_bits(1);
    let plan32 = ComplexFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let mut impulse32 = vec![Complex::new(0.0, 0.0); n];
    impulse32[0] = Complex::new(tiny32, 0.0);
    plan32.forward(&mut impulse32, &mut workspace32).unwrap();
    assert_eq!(impulse32, vec![Complex::new(tiny32, 0.0); n]);
}

#[test]
fn inverse_prescaling_preserves_high_dynamic_direct_and_mixed_primes() {
    let amplitude64 = f64::MAX / 4.01;
    assert_ne!(
        amplitude64 - (amplitude64 * (1.0 / 103.0)) * 103.0,
        0.0,
        "f64 fixture must have a nonzero radix-103 prescale residual"
    );
    assert_ne!(
        amplitude64 / 2.0 - ((amplitude64 / 2.0) * (1.0 / 7.0)) * 7.0,
        0.0,
        "f64 fixture must have a nonzero radix-7 prescale residual"
    );
    assert_ne!(
        amplitude64 / 14.0 - ((amplitude64 / 14.0) * (1.0 / 103.0)) * 103.0,
        0.0,
        "f64 mixed fixture must retain a radix-103 residual after radix 2 and 7"
    );

    let amplitude32 = f32::MAX / 4.65;
    assert_ne!(
        amplitude32 - (amplitude32 * (1.0_f32 / 103.0)) * 103.0,
        0.0,
        "f32 fixture must have a nonzero radix-103 prescale residual"
    );
    assert_ne!(
        amplitude32 / 2.0 - ((amplitude32 / 2.0) * (1.0_f32 / 7.0)) * 7.0,
        0.0,
        "f32 fixture must have a nonzero radix-7 prescale residual"
    );
    assert_ne!(
        amplitude32 / 14.0 - ((amplitude32 / 14.0) * (1.0_f32 / 103.0)) * 103.0,
        0.0,
        "f32 mixed fixture must retain a radix-103 residual after radix 2 and 7"
    );

    for n in [103, 146, 722, 1442] {
        let plan64 = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace64 = plan64.workspace().unwrap();
        let mut spectrum64 = vec![Complex::new(0.0, 0.0); n];
        spectrum64[0] = Complex::new(amplitude64, 0.0);
        spectrum64[1] = Complex::new(-amplitude64 * 0.5, 0.0);
        spectrum64[n - 1] = Complex::new(amplitude64 * 0.25, 0.0);
        let expected64 =
            normalized_direct_inverse(direct_dft_f64(&spectrum64, false), 1.0 / n as f64);
        plan64.inverse(&mut spectrum64, &mut workspace64).unwrap();
        assert_f64_close_at_scale(&spectrum64, &expected64, amplitude64 / n as f64);

        let plan32 = ComplexFftPlan::<f32>::new(n).unwrap();
        let mut workspace32 = plan32.workspace().unwrap();
        let mut spectrum32 = vec![Complex::new(0.0, 0.0); n];
        spectrum32[0] = Complex::new(amplitude32, 0.0);
        spectrum32[1] = Complex::new(-amplitude32 * 0.5, 0.0);
        spectrum32[n - 1] = Complex::new(amplitude32 * 0.25, 0.0);
        let reference32: Vec<_> = spectrum32
            .iter()
            .map(|value| Complex::new(value.re as f64, value.im as f64))
            .collect();
        let expected32 =
            normalized_direct_inverse(direct_dft_f64(&reference32, false), 1.0 / n as f64);
        plan32.inverse(&mut spectrum32, &mut workspace32).unwrap();
        assert_f32_close_at_scale(&spectrum32, &expected32, amplitude32 as f64 / n as f64);
    }
}

#[test]
fn general_prime_inverse_recovers_roundable_subnormal_residuals() {
    let n = 103;

    let tiny64 = f64::from_bits(1);
    let plan64 = ComplexFftPlan::<f64>::new(n).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let mut spectrum64 = vec![Complex::new(0.0, 0.0); n];
    spectrum64[0].re = 26.0 * tiny64;
    spectrum64[2].re = 26.0 * tiny64;
    spectrum64[1].re = f64::MAX / 4.01;
    spectrum64[n - 1].re = -spectrum64[1].re;
    plan64.inverse(&mut spectrum64, &mut workspace64).unwrap();
    assert!(
        spectrum64
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_eq!(spectrum64[0], Complex::new(tiny64, 0.0));

    let tiny32 = f32::from_bits(1);
    let plan32 = ComplexFftPlan::<f32>::new(n).unwrap();
    let mut workspace32 = plan32.workspace().unwrap();
    let mut spectrum32 = vec![Complex::new(0.0, 0.0); n];
    spectrum32[0].re = 26.0 * tiny32;
    spectrum32[2].re = 26.0 * tiny32;
    spectrum32[1].re = f32::MAX / 4.65;
    spectrum32[n - 1].re = -spectrum32[1].re;
    plan32.inverse(&mut spectrum32, &mut workspace32).unwrap();
    assert!(
        spectrum32
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert_eq!(spectrum32[0], Complex::new(tiny32, 0.0));
}

#[test]
fn general_prime_inverse_keeps_maximum_finite_dc_spectra_finite() {
    for n in [103, 241] {
        let plan64 = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace64 = plan64.workspace().unwrap();
        let mut spectrum64 = vec![Complex::new(0.0, 0.0); n];
        spectrum64[0] = Complex::new(f64::MAX, 0.0);
        plan64.inverse(&mut spectrum64, &mut workspace64).unwrap();
        let expected64 = f64::MAX / n as f64;
        for (index, value) in spectrum64.iter().enumerate() {
            assert!(
                value.re.is_finite() && value.im.is_finite(),
                "n={n}, index={index}"
            );
            assert!(
                (value.re - expected64).abs() <= 2.0e-12 * expected64,
                "n={n}, index={index}, actual={}, expected={expected64}",
                value.re
            );
            assert!(
                value.im.abs() <= 2.0e-12 * expected64,
                "n={n}, index={index}"
            );
        }

        let plan32 = ComplexFftPlan::<f32>::new(n).unwrap();
        let mut workspace32 = plan32.workspace().unwrap();
        let mut spectrum32 = vec![Complex::new(0.0, 0.0); n];
        spectrum32[0] = Complex::new(f32::MAX, 0.0);
        plan32.inverse(&mut spectrum32, &mut workspace32).unwrap();
        let expected32 = f32::MAX / n as f32;
        for (index, value) in spectrum32.iter().enumerate() {
            assert!(
                value.re.is_finite() && value.im.is_finite(),
                "n={n}, index={index}"
            );
            assert!(
                (value.re - expected32).abs() <= 4.0e-6 * expected32,
                "n={n}, index={index}, actual={}, expected={expected32}",
                value.re
            );
            assert!(
                value.im.abs() <= 4.0e-6 * expected32,
                "n={n}, index={index}"
            );
        }
    }
}

#[test]
fn bluestein_handles_representable_high_amplitude_impulses() {
    for n in [113, 241] {
        let plan64 = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace64 = plan64.workspace().unwrap();
        let amplitude64 = f64::MAX / 4.0;
        let mut impulse64 = vec![Complex::new(0.0, 0.0); plan64.len()];
        impulse64[0] = Complex::new(amplitude64, 0.0);
        plan64.forward(&mut impulse64, &mut workspace64).unwrap();
        assert!(
            impulse64
                .iter()
                .all(|value| value.re.is_finite() && value.im.is_finite()),
            "f64 n={n}"
        );
        assert_f64_close(
            &impulse64,
            &vec![Complex::new(amplitude64, 0.0); plan64.len()],
            plan64.len(),
        );

        let plan32 = ComplexFftPlan::<f32>::new(n).unwrap();
        let mut workspace32 = plan32.workspace().unwrap();
        let amplitude32 = f32::MAX / 4.0;
        let mut impulse32 = vec![Complex::new(0.0, 0.0); plan32.len()];
        impulse32[0] = Complex::new(amplitude32, 0.0);
        plan32.forward(&mut impulse32, &mut workspace32).unwrap();
        assert!(
            impulse32
                .iter()
                .all(|value| value.re.is_finite() && value.im.is_finite()),
            "f32 n={n}"
        );
        assert_f32_close(
            &impulse32,
            &vec![Complex::new(amplitude32, 0.0); plan32.len()],
            plan32.len(),
        );
    }
}

#[test]
fn bluestein_preserves_subnormal_impulses_and_constant_inverse_spectra() {
    for n in [113, 241] {
        let tiny64 = f64::from_bits(1);
        let plan64 = ComplexFftPlan::<f64>::new(n).unwrap();
        let mut workspace64 = plan64.workspace().unwrap();
        let mut impulse64 = vec![Complex::new(0.0, 0.0); n];
        impulse64[0] = Complex::new(tiny64, 0.0);
        plan64.forward(&mut impulse64, &mut workspace64).unwrap();
        assert_eq!(impulse64, vec![Complex::new(tiny64, 0.0); n], "f64 n={n}");

        let mut constant_spectrum64 = vec![Complex::new(tiny64, 0.0); n];
        plan64
            .inverse(&mut constant_spectrum64, &mut workspace64)
            .unwrap();
        let mut expected64 = vec![Complex::new(0.0, 0.0); n];
        expected64[0] = Complex::new(tiny64, 0.0);
        assert_eq!(constant_spectrum64, expected64, "f64 n={n}");

        let tiny32 = f32::from_bits(1);
        let plan32 = ComplexFftPlan::<f32>::new(n).unwrap();
        let mut workspace32 = plan32.workspace().unwrap();
        let mut impulse32 = vec![Complex::new(0.0, 0.0); n];
        impulse32[0] = Complex::new(tiny32, 0.0);
        plan32.forward(&mut impulse32, &mut workspace32).unwrap();
        assert_eq!(impulse32, vec![Complex::new(tiny32, 0.0); n], "f32 n={n}");

        let mut constant_spectrum32 = vec![Complex::new(tiny32, 0.0); n];
        plan32
            .inverse(&mut constant_spectrum32, &mut workspace32)
            .unwrap();
        let mut expected32 = vec![Complex::new(0.0, 0.0); n];
        expected32[0] = Complex::new(tiny32, 0.0);
        assert_eq!(constant_spectrum32, expected32, "f32 n={n}");
    }
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
fn normalized_inverse_large_finite_spectrum_keeps_scalar_residual_fallback() {
    let n = 4096;
    let plan = ComplexFftPlan::<f64>::new(n).unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut spectrum = vec![Complex::new(0.0, 0.0); n];
    spectrum[0] = Complex::new(f64::MAX, 0.0);

    plan.inverse(&mut spectrum, &mut workspace).unwrap();

    let expected = f64::MAX / n as f64;
    for (index, value) in spectrum.iter().enumerate() {
        assert!(
            value.re.is_finite() && value.im.is_finite(),
            "index={index}"
        );
        assert!(
            (value.re - expected).abs() <= 2.0e-12 * expected,
            "index={index}, actual={}, expected={expected}",
            value.re
        );
        assert!(value.im.abs() <= 2.0e-12 * expected, "index={index}");
    }
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
