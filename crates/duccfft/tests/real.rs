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
