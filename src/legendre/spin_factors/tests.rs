use super::spin_normalization;

#[test]
fn shared_spin_normalization_matches_native_precision_formulas() {
    for spin in 1..=4 {
        for degree in 0..=2048 {
            for derivative in [false, true] {
                let actual_f64 = spin_normalization::<f64>(spin, degree, derivative);
                let expected_f64 = if derivative {
                    if degree == 0 {
                        0.0
                    } else {
                        let degree = degree as f64;
                        0.5 * (degree * (degree + 1.0) * (2.0 * degree + 1.0)
                            / (4.0 * std::f64::consts::PI))
                            .sqrt()
                    }
                } else {
                    let sign = if spin.is_multiple_of(2) { -1.0 } else { 1.0 };
                    sign * 0.5 * ((2 * degree + 1) as f64 / (4.0 * std::f64::consts::PI)).sqrt()
                };
                assert_eq!(actual_f64.to_bits(), expected_f64.to_bits());

                let actual_f32 = spin_normalization::<f32>(spin, degree, derivative);
                let expected_f32 = if derivative {
                    if degree == 0 {
                        0.0
                    } else {
                        let degree = degree as f32;
                        0.5 * (degree * (degree + 1.0) * (2.0 * degree + 1.0)
                            / (4.0 * std::f32::consts::PI))
                            .sqrt()
                    }
                } else {
                    let sign = if spin.is_multiple_of(2) { -1.0 } else { 1.0 };
                    sign * 0.5 * ((2 * degree + 1) as f32 / (4.0 * std::f32::consts::PI)).sqrt()
                };
                assert_eq!(actual_f32.to_bits(), expected_f32.to_bits());
            }
        }
    }
}
