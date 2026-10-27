use super::*;
use std::f64::consts::PI;

fn reference_polar_order_limit_f64(
    lmax: usize,
    spin: usize,
    sin_theta: f64,
    cos_theta: f64,
) -> usize {
    let lmax_float = lmax as f64;
    let offset = (0.01 * lmax_float).max(100.0);
    let b = -2.0 * spin as f64 * cos_theta.abs();
    let t = lmax_float * sin_theta + offset;
    let c = (spin as f64).powi(2) - t * t;
    let discriminant = b * b - 4.0 * c;
    if discriminant <= 0.0 {
        return lmax;
    }
    (((-b + discriminant.sqrt()) * 0.5).min(lmax_float) + 0.5) as usize
}

fn reference_polar_order_limit_f32(
    lmax: usize,
    spin: usize,
    sin_theta: f32,
    cos_theta: f32,
) -> usize {
    let lmax_float = lmax as f32;
    let offset = (0.01 * lmax_float).max(100.0);
    let b = -2.0 * spin as f32 * cos_theta.abs();
    let t = lmax_float * sin_theta + offset;
    let c = (spin as f32).powi(2) - t * t;
    let discriminant = b * b - 4.0 * c;
    if discriminant <= 0.0 {
        return lmax;
    }
    (((-b + discriminant.sqrt()) * 0.5).min(lmax_float) + 0.5) as usize
}

#[test]
fn shared_polar_order_limit_matches_native_precision_formulas() {
    fn check(lmax: usize, spin: usize, sin_theta: f64, cos_theta: f64) {
        let expected_f64 = reference_polar_order_limit_f64(lmax, spin, sin_theta, cos_theta);
        assert_eq!(
            polar_order_limit(lmax, spin, sin_theta, cos_theta),
            expected_f64
        );
        assert_eq!(
            <f64 as crate::legendre::ShtScalar>::polar_order_limit(
                lmax, spin, sin_theta, cos_theta
            ),
            expected_f64
        );
        let sin_theta = sin_theta as f32;
        let cos_theta = cos_theta as f32;
        let expected_f32 = reference_polar_order_limit_f32(lmax, spin, sin_theta, cos_theta);
        assert_eq!(
            polar_order_limit(lmax, spin, sin_theta, cos_theta),
            expected_f32
        );
        assert_eq!(
            <f32 as crate::legendre::ShtScalar>::polar_order_limit(
                lmax, spin, sin_theta, cos_theta
            ),
            expected_f32
        );
    }

    let angles = [0.0, 1.0e-5, 0.1, 0.5, PI / 2.0, PI - 1.0e-5, PI];
    for lmax in [0, 1, 2, 15, 99, 100, 101, 255, 511, 720, 1023, 2048, 10_000] {
        for spin in [0, 1, 2, 4] {
            for angle in angles {
                let (sin_theta, cos_theta) = angle.sin_cos();
                check(lmax, spin, sin_theta, cos_theta);
            }
        }
    }

    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        for lmax in [255, 511, 720, 1023] {
            let ntheta = lmax + 1;
            let grid = GridSpec::new(kind, ntheta, 1, 0.0).unwrap();
            for spin in [0, 1, 2, 4] {
                for &theta in grid.theta() {
                    check(lmax, spin, theta.sin(), theta.cos());
                }
            }
        }
    }
}

fn is_fft_friendly(mut value: usize) -> bool {
    for factor in [2, 3, 5, 7] {
        while value.is_multiple_of(factor) {
            value /= factor;
        }
    }
    value == 1
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "grid value differs from reference beyond tolerance"
    );
}

#[test]
fn fft_friendly_nphi_is_minimal_and_sampling_safe() {
    for mmax in 0..=1024 {
        let minimum = 2 * mmax + 1;
        let actual = GridSpec::fft_friendly_nphi(mmax).unwrap();
        assert!(actual >= minimum);
        assert!(is_fft_friendly(actual));
        assert!((minimum..actual).all(|candidate| !is_fft_friendly(candidate)));
    }

    for (mmax, expected) in [
        (63, 128),
        (72, 147),
        (127, 256),
        (255, 512),
        (511, 1024),
        (720, 1458),
        (1023, 2048),
    ] {
        assert_eq!(GridSpec::fft_friendly_nphi(mmax), Ok(expected));
    }

    let error = GridSpec::fft_friendly_nphi(usize::MAX / 2).unwrap_err();
    assert!(matches!(error, ShtError::IntegerOverflow { .. }));
}

#[test]
fn small_rules_match_closed_form() {
    let one = GridSpec::new(GridKind::GaussLegendre, 1, 1, 0.0).unwrap();
    close(one.theta()[0], PI / 2.0, 2e-16);
    close(one.ring_weights()[0], 4.0 * PI, 2e-15);

    let two = GridSpec::new(GridKind::GaussLegendre, 2, 3, 0.0).unwrap();
    close(two.theta()[0], (1.0_f64 / 3.0).sqrt().acos(), 3e-16);
    close(two.theta()[1], PI - (1.0_f64 / 3.0).sqrt().acos(), 3e-16);
    close(two.ring_weights()[0], 2.0 * PI, 2e-15);

    let three = GridSpec::new(GridKind::GaussLegendre, 3, 4, 0.0).unwrap();
    close(three.theta()[0].cos(), (3.0_f64 / 5.0).sqrt(), 3e-16);
    close(three.theta()[1], PI / 2.0, 3e-16);
    close(three.ring_weights()[0], 10.0 * PI / 9.0, 3e-15);
    close(three.ring_weights()[1], 16.0 * PI / 9.0, 3e-15);
}

#[test]
fn gauss_legendre_moments_are_exact_for_small_rules() {
    for ntheta in 1..=16 {
        let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, 1, 0.0).unwrap();
        for degree in 0..2 * ntheta {
            let actual: f64 = grid
                .theta()
                .iter()
                .zip(grid.ring_weights())
                .map(|(&theta, &weight)| theta.cos().powi(degree as i32) * weight / TAU)
                .sum();
            let expected = if degree % 2 == 0 {
                2.0 / (degree as f64 + 1.0)
            } else {
                0.0
            };
            close(actual, expected, 3e-14);
        }
    }
}

#[test]
fn both_quadrature_algorithms_are_symmetric_and_normalized() {
    for ntheta in [99, 100, 101, 127, 255, 1024] {
        let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, 1, 0.0).unwrap();
        for i in 0..ntheta {
            close(grid.theta()[i], PI - grid.theta()[ntheta - 1 - i], 3e-15);
            close(
                grid.ring_weights()[i],
                grid.ring_weights()[ntheta - 1 - i],
                3e-15,
            );
            assert!(grid.ring_weights()[i] > 0.0);
        }
        close(grid.ring_weights().iter().sum(), 4.0 * PI, 2e-13);
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn selected_nodes_and_weights_match_pinned_ducc() {
    // Values from GL_Integrator at DUCC revision
    // c1ed01b8fda69d2a378190f0915f1a19762dddbf, with weights multiplied by 2π.
    let reference = [
        (99, 0, 0.024168999361183122, 0.004709081805298769),
        (100, 0, 0.023928513799576875, 0.0046158446368925986),
        (101, 0, 0.023692766811651021, 0.004525349275429071),
        (127, 31, 0.782325906824648, 0.10913384776173643),
        (255, 63, 0.7838631013671257, 0.05454494109708152),
        (1024, 256, 0.7873148222088625, 0.013650025993590937),
    ];

    for (ntheta, ring, expected_theta, expected_weight) in reference {
        let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, 1, 0.0).unwrap();
        close(grid.theta()[ring], expected_theta, 2.5e-15);
        close(grid.ring_weights()[ring], expected_weight, 2e-15);
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn clenshaw_curtis_nodes_and_weights_match_ducc_0410() {
    let references = [
        (2, vec![TAU, TAU]),
        (
            3,
            vec![
                2.094_395_102_393_195_7,
                8.377_580_409_572_783,
                2.094_395_102_393_195_7,
            ],
        ),
        (
            4,
            vec![
                0.698_131_700_797_731_6,
                5.585_053_606_381_853,
                5.585_053_606_381_853,
                0.698_131_700_797_731_6,
            ],
        ),
        (
            5,
            vec![
                0.418_879_020_478_639_34,
                3.351_032_163_829_112_5,
                5.026_548_245_743_669,
                3.351_032_163_829_112_5,
                0.418_879_020_478_639_34,
            ],
        ),
    ];
    for (ntheta, expected_weights) in references {
        let grid = GridSpec::new(GridKind::ClenshawCurtis, ntheta, 1, 0.0).unwrap();
        for (ring, (&theta, &weight)) in grid.theta().iter().zip(grid.ring_weights()).enumerate() {
            let north_ring = ring.min(ntheta - 1 - ring);
            let north_theta = PI * north_ring as f64 / (ntheta - 1) as f64;
            let expected_theta = if ring <= ntheta - 1 - ring {
                north_theta
            } else {
                PI - north_theta
            };
            close(theta, expected_theta, 0.0);
            close(weight, expected_weights[ring], 2e-15);
        }
        close(grid.ring_weights().iter().sum(), 4.0 * PI, 2e-14);
    }

    for ntheta in 2..=20 {
        let grid = GridSpec::new(GridKind::ClenshawCurtis, ntheta, 1, 0.0).unwrap();
        for degree in 0..ntheta {
            let actual: f64 = grid
                .theta()
                .iter()
                .zip(grid.ring_weights())
                .map(|(&theta, &weight)| theta.cos().powi(degree as i32) * weight / TAU)
                .sum();
            let expected = if degree % 2 == 0 {
                2.0 / (degree as f64 + 1.0)
            } else {
                0.0
            };
            close(actual, expected, 4e-14);
        }
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn fejer1_nodes_and_weights_match_ducc_0410() {
    let references = [
        (1, vec![12.566_370_614_359_172]),
        (2, vec![TAU, TAU]),
        (
            3,
            vec![
                2.792_526_803_190_927,
                6.981_317_007_977_318,
                2.792_526_803_190_927,
            ],
        ),
        (
            4,
            vec![
                1.660_631_674_203_671_5,
                4.622_553_632_975_915,
                4.622_553_632_975_915,
                1.660_631_674_203_671_5,
            ],
        ),
        (
            5,
            vec![
                1.054_200_549_522_407,
                3.302_141_263_455_439_5,
                3.853_686_988_403_479_2,
                3.302_141_263_455_439_5,
                1.054_200_549_522_407,
            ],
        ),
        (
            6,
            vec![
                0.745_569_186_077_503_9,
                2.373_647_782_712_288,
                3.163_968_338_389_794,
                3.163_968_338_389_794,
                2.373_647_782_712_288,
                0.745_569_186_077_503_9,
            ],
        ),
    ];
    for (ntheta, expected_weights) in references {
        let grid = GridSpec::new(GridKind::Fejer1, ntheta, 1, 0.0).unwrap();
        for (ring, (&theta, &weight)) in grid.theta().iter().zip(grid.ring_weights()).enumerate() {
            let north_ring = ring.min(ntheta - 1 - ring);
            let north_theta = PI * (north_ring as f64 + 0.5) / ntheta as f64;
            let expected_theta = if ring <= ntheta - 1 - ring {
                north_theta
            } else {
                PI - north_theta
            };
            close(theta, expected_theta, 0.0);
            close(weight, expected_weights[ring], 2e-15);
        }
        close(grid.ring_weights().iter().sum(), 4.0 * PI, 2e-14);
    }

    for ntheta in 1..=20 {
        let grid = GridSpec::new(GridKind::Fejer1, ntheta, 1, 0.0).unwrap();
        for degree in 0..ntheta {
            let actual: f64 = grid
                .theta()
                .iter()
                .zip(grid.ring_weights())
                .map(|(&theta, &weight)| theta.cos().powi(degree as i32) * weight / TAU)
                .sum();
            let expected = if degree % 2 == 0 {
                2.0 / (degree as f64 + 1.0)
            } else {
                0.0
            };
            close(actual, expected, 5e-14);
        }
    }
}

#[test]
fn grid_rejects_invalid_shapes_and_longitude_origin() {
    assert_eq!(
        GridSpec::new(GridKind::GaussLegendre, 0, 1, 0.0),
        Err(ShtError::InvalidThetaCount)
    );
    assert_eq!(
        GridSpec::new(GridKind::GaussLegendre, 1, 0, 0.0),
        Err(ShtError::InvalidPhiCount)
    );
    assert_eq!(
        GridSpec::new(GridKind::GaussLegendre, 1, 1, f64::NAN),
        Err(ShtError::NonFinitePhi0)
    );
}

#[test]
fn longitude_samples_follow_phi0_and_eastward_order() {
    let grid = GridSpec::new(GridKind::GaussLegendre, 2, 4, 0.25).unwrap();
    close(grid.longitude(0).unwrap(), 0.25, 0.0);
    close(grid.longitude(1).unwrap(), 0.25 + PI / 2.0, 2e-16);
    assert_eq!(grid.longitude(4), None);
}
