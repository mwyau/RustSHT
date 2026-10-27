use super::*;

struct AnalysisLanes {
    q_real: Vec<f64>,
    q_imag: Vec<f64>,
    u_real: Vec<f64>,
    u_imag: Vec<f64>,
    plus: Vec<f64>,
    plus_exponents: Vec<i64>,
    minus: Vec<f64>,
    minus_exponents: Vec<i64>,
    limits: Vec<usize>,
}

impl AnalysisLanes {
    fn new(count: usize) -> Self {
        Self {
            q_real: (0..count).map(|lane| 0.25 + lane as f64 * 0.007).collect(),
            q_imag: (0..count).map(|lane| -0.4 + lane as f64 * 0.003).collect(),
            u_real: (0..count).map(|lane| 0.15 - lane as f64 * 0.002).collect(),
            u_imag: (0..count).map(|lane| 0.55 + lane as f64 * 0.004).collect(),
            plus: (0..count).map(|lane| 0.31 + lane as f64 * 0.001).collect(),
            plus_exponents: vec![0; count],
            minus: (0..count).map(|lane| -0.17 + lane as f64 * 0.002).collect(),
            minus_exponents: vec![0; count],
            limits: vec![64; count],
        }
    }

    fn accumulate(
        &self,
        arch: Arch,
        m: usize,
        alpha_norm: f64,
        include_b: bool,
    ) -> (Complex64, Complex64) {
        let vectorizable_inputs = analysis_inputs_vectorizable(
            m,
            &self.q_real,
            &self.q_imag,
            &self.u_real,
            &self.u_imag,
            &self.limits,
        );
        accumulate_analysis_dispatched(
            arch,
            vectorizable_inputs,
            m,
            &self.q_real,
            &self.q_imag,
            &self.u_real,
            &self.u_imag,
            &self.plus,
            &self.plus_exponents,
            &self.minus,
            &self.minus_exponents,
            alpha_norm,
            &self.limits,
            include_b,
        )
    }

    fn scalar_reference(
        &self,
        m: usize,
        alpha_norm: f64,
        include_b: bool,
    ) -> (Complex64, Complex64) {
        let mut e_sum = Complex64::new(0.0, 0.0);
        let mut b_sum = Complex64::new(0.0, 0.0);
        for lane in 0..self.plus.len() {
            if m > self.limits[lane] {
                continue;
            }
            let plus_value =
                scaled_basis_value(self.plus[lane], self.plus_exponents[lane], alpha_norm);
            let minus_value =
                scaled_basis_value(self.minus[lane], self.minus_exponents[lane], alpha_norm);
            let even_basis = plus_value + minus_value;
            let odd_basis = plus_value - minus_value;
            let q = Complex64::new(self.q_real[lane], self.q_imag[lane]);
            let u = Complex64::new(self.u_real[lane], self.u_imag[lane]);
            e_sum += q * even_basis + Complex64::new(0.0, -1.0) * u * odd_basis;
            if include_b {
                b_sum += Complex64::new(0.0, 1.0) * q * odd_basis + u * even_basis;
            }
        }
        (e_sum, b_sum)
    }
}

#[test]
fn spin_analysis_simd_accumulation_matches_scalar_blocks_and_tails() {
    for count in [64, 61] {
        let lanes = AnalysisLanes::new(count);
        let expected = lanes.scalar_reference(4, 0.75, true);
        for arch in [Arch::Scalar, Arch::new()] {
            let actual = lanes.accumulate(arch, 4, 0.75, true);
            assert!((actual.0 - expected.0).norm() < 2e-14);
            assert!((actual.1 - expected.1).norm() < 2e-14);
        }
    }
}

#[test]
fn spin_analysis_scaled_and_inactive_lanes_use_scalar_accumulation() {
    let mut lanes = AnalysisLanes::new(64);
    lanes.plus_exponents[11] = -800;
    lanes.limits[7] = 3;
    let expected = lanes.scalar_reference(4, 0.75, true);
    for arch in [Arch::Scalar, Arch::new()] {
        let actual = lanes.accumulate(arch, 4, 0.75, true);
        assert_eq!(actual.0, expected.0);
        assert_eq!(actual.1, expected.1);
    }
}

#[test]
fn scaled_spin_basis_matches_binary_scaling_reference() {
    let values = [
        0.0,
        -0.0,
        0.125,
        -0.75,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::INFINITY,
        f64::NAN,
    ];
    let normalizations = [
        0.0,
        -0.0,
        0.5,
        -1.25,
        f64::MIN_POSITIVE,
        f64::INFINITY,
        f64::NAN,
    ];
    let exponents = [-4097, -1022, -1, 0, 1, 1023, 4097];

    for value in values {
        for alpha_norm in normalizations {
            for exponent in exponents {
                let expected = if value == 0.0 || alpha_norm == 0.0 {
                    0.0
                } else {
                    crate::legendre::scale_binary(value * alpha_norm, exponent)
                };
                let actual = scaled_basis_value(value, exponent, alpha_norm);
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
        }
    }
}

fn evaluate(spin: usize, m: usize, lmax: usize, theta: &[f64]) -> Vec<(usize, Vec<f64>, Vec<f64>)> {
    let count = lmax + 2;
    let mut alpha = vec![0.0; count];
    let mut coefficient_a = vec![0.0; count];
    let mut coefficient_b = vec![0.0; count];
    let initial = crate::legendre::spin_factors::fill_spin_factors::<f64>(
        spin,
        m,
        lmax,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    )
    .unwrap();
    let sin_theta = theta.iter().map(|value| value.sin()).collect::<Vec<_>>();
    let cos_theta = theta.iter().map(|value| value.cos()).collect::<Vec<_>>();
    let scratch_len = theta.len();
    let mut previous_plus = vec![0.0; scratch_len];
    let mut current_plus = vec![0.0; scratch_len];
    let mut next_plus = vec![0.0; scratch_len];
    let mut exponent_plus = vec![0; scratch_len];
    let mut previous_minus = vec![0.0; scratch_len];
    let mut current_minus = vec![0.0; scratch_len];
    let mut next_minus = vec![0.0; scratch_len];
    let mut exponent_minus = vec![0; scratch_len];
    let mut result = Vec::new();
    for_each_spin_lm_dispatched(
        Arch::new(),
        &initial,
        lmax,
        &sin_theta,
        &cos_theta,
        &coefficient_a,
        &coefficient_b,
        &mut previous_plus,
        &mut current_plus,
        &mut next_plus,
        &mut exponent_plus,
        &mut previous_minus,
        &mut current_minus,
        &mut next_minus,
        &mut exponent_minus,
        |degree, _, plus, plus_exp, minus, minus_exp| {
            let norm =
                crate::legendre::spin_factors::spin_normalization::<f64>(spin, degree, false)
                    * alpha[degree];
            result.push((
                degree,
                plus.iter()
                    .zip(plus_exp)
                    .map(|(&value, &exponent)| scaled_basis_value(value, exponent, norm))
                    .collect(),
                minus
                    .iter()
                    .zip(minus_exp)
                    .map(|(&value, &exponent)| scaled_basis_value(value, exponent, norm))
                    .collect(),
            ));
            Ok(())
        },
    )
    .unwrap();
    result
}

#[test]
fn low_degree_spin_bases_match_closed_forms() {
    let theta = [0.31, 0.77, 1.43, 2.81];
    let spin1_m0 = evaluate(1, 0, 1, &theta);
    let expected_norm1 = (3.0 / (8.0 * PI)).sqrt();
    for (ring, &angle) in theta.iter().enumerate() {
        let expected = -expected_norm1 * angle.sin();
        assert!((spin1_m0[0].1[ring] - expected * 0.5).abs() < 2e-15);
        assert!((spin1_m0[0].2[ring] - expected * 0.5).abs() < 2e-15);
    }

    let spin1_m1 = evaluate(1, 1, 1, &theta);
    for (ring, &angle) in theta.iter().enumerate() {
        let norm = 0.5 * (3.0 / (4.0 * PI)).sqrt();
        let cosine = angle.cos();
        assert!((spin1_m1[0].1[ring] + norm * (1.0 + cosine) * 0.5).abs() < 2e-15);
        assert!((spin1_m1[0].2[ring] - norm * (1.0 - cosine) * 0.5).abs() < 2e-15);
    }

    let spin2_m0 = evaluate(2, 0, 2, &theta);
    let expected_norm2 = -(30.0 / (64.0 * PI)).sqrt();
    for (ring, &angle) in theta.iter().enumerate() {
        let expected = expected_norm2 * angle.sin().powi(2);
        assert!((spin2_m0[0].1[ring] - expected * 0.5).abs() < 2e-15);
        assert!((spin2_m0[0].2[ring] - expected * 0.5).abs() < 2e-15);
    }

    for spin in [3, 4] {
        let basis = evaluate(spin, 0, spin, &theta);
        let central_binomial = (1..=spin)
            .map(|index| (spin + index) as f64 / index as f64)
            .product::<f64>();
        for (ring, &angle) in theta.iter().enumerate() {
            let half_angle_product = (angle / 2.0).sin() * (angle / 2.0).cos();
            let expected = -0.5
                * (((2 * spin + 1) as f64 * central_binomial / (4.0 * PI)).sqrt())
                * half_angle_product.powi(spin as i32);
            assert!((basis[0].1[ring] - expected).abs() < 3e-15);
            assert!((basis[0].2[ring] - expected).abs() < 3e-15);
        }
    }
}

mod f32;
