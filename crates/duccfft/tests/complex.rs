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
