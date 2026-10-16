// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
#![allow(clippy::excessive_precision)] // Preserve the printed DUCC float32 reference values.

use std::f32::consts::PI;
use std::f64::consts::TAU;

use rustsht::{Complex32, Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn make_scalar_plan(kind: GridKind, ntheta: usize, nphi: usize) -> ShtPlan<f32> {
    ShtPlan::<f32>::new(
        GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::scalar(2, 2).unwrap(),
    )
    .unwrap()
}

fn f32_coefficients(spec: &HarmonicSpec) -> Vec<Complex32> {
    let mut coefficients = vec![Complex32::new(0.0, 0.0); spec.nalm()];
    for (degree, order, value) in [
        (0, 0, Complex32::new(0.7, 0.0)),
        (1, 0, Complex32::new(-0.2, 0.0)),
        (2, 0, Complex32::new(0.1, 0.0)),
        (1, 1, Complex32::new(0.4, 0.3)),
        (2, 1, Complex32::new(-0.2, 0.15)),
        (2, 2, Complex32::new(0.1, -0.6)),
    ] {
        if let Some(index) = spec.index(degree, order) {
            coefficients[index] = value;
        }
    }
    coefficients
}

fn arbitrary_map(ntheta: usize, nphi: usize, offset: f32) -> Vec<f32> {
    let mut map = Vec::with_capacity(ntheta * nphi);
    for ring in 0..ntheta {
        for longitude in 0..nphi {
            let i = ring as f32;
            let j = longitude as f32;
            map.push((0.17 * i + 0.23 * j + offset).sin() + 0.2 * (0.11 * i - 0.31 * j).cos());
        }
    }
    map
}

fn map_inner(left: &[f32], right: &[f32]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(&a, &b)| f64::from(a) * f64::from(b))
        .sum()
}

fn harmonic_inner(spec: &HarmonicSpec, left: &[Complex32], right: &[Complex32]) -> f64 {
    let mut result = 0.0;
    for order in 0..=spec.mmax() {
        let weight = if order == 0 { 1.0 } else { 2.0 };
        for degree in order..=spec.lmax() {
            let index = spec.index(degree, order).unwrap();
            let a = left[index];
            let b = right[index];
            result += weight * f64::from(a.re * b.re + a.im * b.im);
        }
    }
    result
}

fn spin_harmonic_inner(
    spec: &HarmonicSpec,
    left_e: &[Complex32],
    left_b: &[Complex32],
    right_e: &[Complex32],
    right_b: &[Complex32],
) -> f64 {
    harmonic_inner(spec, left_e, right_e) + harmonic_inner(spec, left_b, right_b)
}

fn vector_map_inner(a: &[f32], b: &[f32], c: &[f32], d: &[f32]) -> f64 {
    map_inner(a, c) + map_inner(b, d)
}

fn assert_close(actual: f64, expected: f64, relative_tolerance: f64) {
    let scale = actual.abs().max(expected.abs()).max(1.0e-8);
    assert!(
        (actual - expected).abs() <= relative_tolerance * scale,
        "actual {actual:.9e}, expected {expected:.9e}, scaled error {:.3e}",
        (actual - expected).abs() / scale
    );
}

fn assert_values_close(actual: &[f32], expected: &[f32], tolerance: f32) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= tolerance,
            "entry {index}: actual {actual:.9e}, expected {expected:.9e}"
        );
    }
}

fn assert_coefficients_close(actual: &[Complex32], expected: &[Complex32], tolerance: f32) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).norm() <= tolerance,
            "entry {index}: actual {actual:?}, expected {expected:?}"
        );
    }
}

#[test]
fn f32_scalar_synthesis_and_analysis_match_ducc_0410_on_all_grids() {
    // References are from ducc0 0.41.0, using complex64 coefficients and
    // float32 maps at phi0=0.37 with one thread.
    let cases: &[(GridKind, usize, &[f32], &[Complex32])] = &[
        (
            GridKind::GaussLegendre,
            3,
            &[
                0.3614777625,
                0.3412027955,
                0.1674258113,
                0.1101376191,
                -0.2452255040,
                0.3528001904,
                0.2598930597,
                0.3243145645,
                0.5316288471,
                -0.6390007138,
                0.1484631002,
                0.2827612162,
                0.5535345674,
                0.6164205670,
                -0.1092212126,
            ],
            &[
                Complex32::new(2.5227880478, 0.0),
                Complex32::new(-0.3680503070, 0.0),
                Complex32::new(-0.0158646256, 0.0),
                Complex32::new(0.2047233731, -0.4063146710),
                Complex32::new(-0.0025090254, -0.0394883677),
                Complex32::new(-0.1357878447, 0.2137478441),
            ],
        ),
        (
            GridKind::ClenshawCurtis,
            4,
            &[
                0.1628241539,
                0.1628241539,
                0.1628241539,
                0.1628241539,
                0.1628241539,
                0.4207195342,
                0.3297321498,
                0.1929279119,
                0.2553099692,
                -0.4950830340,
                0.1963506043,
                0.2419877648,
                0.4981142581,
                0.6667162776,
                -0.4109598696,
                0.3582651615,
                0.3582651615,
                0.3582651615,
                0.3582651615,
                0.3582651615,
            ],
            &[
                Complex32::new(2.7636518478, 0.0),
                Complex32::new(-0.3271538615, 0.0),
                Complex32::new(-0.0279265419, 0.0),
                Complex32::new(0.1994562298, -0.3676843047),
                Complex32::new(-0.0001705594, -0.0252188649),
                Complex32::new(-0.1297497004, 0.2009217143),
            ],
        ),
        (
            GridKind::Fejer1,
            3,
            &[
                0.3143424988,
                0.3240856230,
                0.1681707203,
                0.0807149708,
                -0.1260045469,
                0.3528001904,
                0.2598930597,
                0.3243145645,
                0.5316288471,
                -0.6390007138,
                0.1615099758,
                0.3078776598,
                0.5448934436,
                0.5636577010,
                0.0296550095,
            ],
            &[
                Complex32::new(2.5263354778, 0.0),
                Complex32::new(-0.3291941881, 0.0),
                Complex32::new(-0.0126916729, 0.0),
                Complex32::new(0.2172976732, -0.4312911332),
                Complex32::new(-0.0028386258, -0.0446759984),
                Complex32::new(-0.1359299570, 0.2139784247),
            ],
        ),
    ];

    for &(kind, ntheta, expected_map, expected_alm) in cases {
        let plan = make_scalar_plan(kind, ntheta, 5);
        let coefficients = f32_coefficients(plan.harmonics());
        let mut map = vec![0.0_f32; ntheta * 5];
        let mut workspace = plan.workspace().unwrap();
        plan.synthesis_into(&coefficients, &mut map, &mut workspace)
            .unwrap();
        assert_values_close(&map, expected_map, 3.0e-6);

        let arbitrary = arbitrary_map(ntheta, 5, 0.0);
        let mut analyzed = vec![Complex32::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.analysis_into(&arbitrary, &mut analyzed, &mut workspace)
            .unwrap();
        assert_coefficients_close(&analyzed, expected_alm, 3.0e-6);
    }
}

#[test]
fn f32_scalar_analytic_field_and_adjoint_identities_hold_on_all_grids() {
    for (kind, ntheta) in [
        (GridKind::GaussLegendre, 3),
        (GridKind::ClenshawCurtis, 4),
        (GridKind::Fejer1, 3),
    ] {
        let plan = make_scalar_plan(kind, ntheta, 5);
        let mut workspace = plan.workspace().unwrap();
        let spec = plan.harmonics();
        let mut coefficients = vec![Complex32::new(0.0, 0.0); spec.nalm()];
        coefficients[spec.index(0, 0).unwrap()] = Complex32::new(0.7, 0.0);
        coefficients[spec.index(1, 0).unwrap()] = Complex32::new(-0.2, 0.0);
        coefficients[spec.index(2, 0).unwrap()] = Complex32::new(0.1, 0.0);
        let mut map = vec![0.0_f32; ntheta * 5];
        plan.synthesis_into(&coefficients, &mut map, &mut workspace)
            .unwrap();
        for (ring, &theta64) in plan.grid().theta().iter().enumerate() {
            let cosine = (theta64 as f32).cos();
            let expected = 0.7 / (4.0 * PI).sqrt() - 0.2 * (3.0 / (4.0 * PI)).sqrt() * cosine
                + 0.1 * (5.0 / (16.0 * PI)).sqrt() * (3.0 * cosine * cosine - 1.0);
            for sample in &map[ring * 5..(ring + 1) * 5] {
                assert!((*sample - expected).abs() <= 2.0e-6);
            }
        }

        let test_map = arbitrary_map(ntheta, 5, 0.41);
        let mut synthesized = vec![0.0_f32; ntheta * 5];
        plan.synthesis_into(&coefficients, &mut synthesized, &mut workspace)
            .unwrap();
        let mut synthesis_adjoint = vec![Complex32::new(0.0, 0.0); spec.nalm()];
        plan.adjoint_synthesis_into(&test_map, &mut synthesis_adjoint, &mut workspace)
            .unwrap();
        assert_close(
            map_inner(&synthesized, &test_map),
            harmonic_inner(spec, &coefficients, &synthesis_adjoint),
            2.0e-5,
        );

        let mut analyzed = vec![Complex32::new(0.0, 0.0); spec.nalm()];
        plan.analysis_into(&test_map, &mut analyzed, &mut workspace)
            .unwrap();
        let mut analysis_adjoint = vec![0.0_f32; ntheta * 5];
        plan.adjoint_analysis_into(&coefficients, &mut analysis_adjoint, &mut workspace)
            .unwrap();
        assert_close(
            harmonic_inner(spec, &analyzed, &coefficients),
            map_inner(&test_map, &analysis_adjoint),
            2.0e-5,
        );
    }
}

#[test]
fn f32_rectangular_even_analysis_and_even_nyquist_synthesis_are_supported() {
    let rectangular = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, 4, 6, 0.41).unwrap(),
        HarmonicSpec::scalar(3, 2).unwrap(),
    )
    .unwrap();
    let mut rectangular_workspace = rectangular.workspace().unwrap();
    let mut coefficients = vec![Complex32::new(0.0, 0.0); rectangular.harmonics().nalm()];
    for (degree, order, value) in [
        (0, 0, Complex32::new(0.3, 0.0)),
        (1, 0, Complex32::new(-0.2, 0.0)),
        (2, 1, Complex32::new(0.1, 0.17)),
        (3, 2, Complex32::new(-0.09, 0.04)),
    ] {
        coefficients[rectangular.harmonics().index(degree, order).unwrap()] = value;
    }
    let mut map = vec![0.0_f32; 24];
    rectangular
        .synthesis_into(&coefficients, &mut map, &mut rectangular_workspace)
        .unwrap();
    let mut recovered = vec![Complex32::new(0.0, 0.0); coefficients.len()];
    rectangular
        .analysis_into(&map, &mut recovered, &mut rectangular_workspace)
        .unwrap();
    assert_coefficients_close(&recovered, &coefficients, 3.0e-5);

    let nyquist = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, 4, 6, 0.41).unwrap(),
        HarmonicSpec::scalar(3, 3).unwrap(),
    )
    .unwrap();
    let spec = nyquist.harmonics();
    let mut nyquist_coefficients = vec![Complex32::new(0.0, 0.0); spec.nalm()];
    nyquist_coefficients[spec.index(3, 3).unwrap()] = Complex32::new(0.25, -0.1);
    let mut nyquist_workspace = nyquist.workspace().unwrap();
    let mut nyquist_map = vec![0.0_f32; 24];
    nyquist
        .synthesis_into(
            &nyquist_coefficients,
            &mut nyquist_map,
            &mut nyquist_workspace,
        )
        .unwrap();

    let norm = (1..=3).fold(
        1.0_f64 / (4.0 * std::f64::consts::PI).sqrt(),
        |value, order| value * ((2 * order + 1) as f64 / (2 * order) as f64).sqrt(),
    );
    for (ring, &theta) in nyquist.grid().theta().iter().enumerate() {
        let amplitude = -norm * theta.sin().powi(3);
        for longitude in 0..6 {
            let phi = 0.41 + TAU * longitude as f64 / 6.0;
            let y_re = amplitude * (3.0 * phi).cos();
            let y_im = amplitude * (3.0 * phi).sin();
            let expected = 2.0 * (0.25 * y_re + 0.1 * y_im);
            assert!((nyquist_map[ring * 6 + longitude] - expected as f32).abs() <= 3.0e-6);
        }
    }
}

#[test]
fn f32_spin_1_through_4_operations_and_adjoint_identities_hold_on_all_grids() {
    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        for spin in 1..=4 {
            let ntheta = match kind {
                GridKind::GaussLegendre | GridKind::Fejer1 => spin + 1,
                GridKind::ClenshawCurtis => spin + 2,
            };
            for nphi in [2 * spin + 1, 2 * spin + 2] {
                let plan = ShtPlan::<f32>::new(
                    GridSpec::new(kind, ntheta, nphi, 0.23).unwrap(),
                    HarmonicSpec::new(spin, spin, spin).unwrap(),
                )
                .unwrap();
                let spec = plan.harmonics();
                let mut workspace = plan.workspace().unwrap();
                let mut e = vec![Complex32::new(0.0, 0.0); spec.nalm()];
                let mut b = vec![Complex32::new(0.0, 0.0); spec.nalm()];
                for degree in spin..=spin {
                    for order in 0..=degree {
                        let index = spec.index(degree, order).unwrap();
                        e[index] = Complex32::new(
                            0.13 * (degree + order + 1) as f32,
                            if order == 0 { 0.0 } else { -0.07 },
                        );
                        b[index] = Complex32::new(
                            -0.09 * (degree + 2 * order + 1) as f32,
                            if order == 0 { 0.0 } else { 0.11 },
                        );
                    }
                }
                let first = arbitrary_map(ntheta, nphi, 0.1 * spin as f32);
                let second = arbitrary_map(ntheta, nphi, 0.29 + 0.07 * spin as f32);

                let mut synthesized_first = vec![0.0_f32; ntheta * nphi];
                let mut synthesized_second = vec![0.0_f32; ntheta * nphi];
                plan.synthesis_spin_into(
                    &e,
                    &b,
                    &mut synthesized_first,
                    &mut synthesized_second,
                    &mut workspace,
                )
                .unwrap();
                let mut adjoint_e = vec![Complex32::new(0.0, 0.0); spec.nalm()];
                let mut adjoint_b = vec![Complex32::new(0.0, 0.0); spec.nalm()];
                plan.adjoint_synthesis_spin_into(
                    &first,
                    &second,
                    &mut adjoint_e,
                    &mut adjoint_b,
                    &mut workspace,
                )
                .unwrap();
                assert_close(
                    vector_map_inner(&synthesized_first, &synthesized_second, &first, &second),
                    spin_harmonic_inner(spec, &e, &b, &adjoint_e, &adjoint_b),
                    3.0e-5,
                );

                let mut analyzed_e = vec![Complex32::new(0.0, 0.0); spec.nalm()];
                let mut analyzed_b = vec![Complex32::new(0.0, 0.0); spec.nalm()];
                plan.analysis_spin_into(
                    &first,
                    &second,
                    &mut analyzed_e,
                    &mut analyzed_b,
                    &mut workspace,
                )
                .unwrap();
                let mut adjoint_first = vec![0.0_f32; ntheta * nphi];
                let mut adjoint_second = vec![0.0_f32; ntheta * nphi];
                plan.adjoint_analysis_spin_into(
                    &e,
                    &b,
                    &mut adjoint_first,
                    &mut adjoint_second,
                    &mut workspace,
                )
                .unwrap();
                assert_close(
                    spin_harmonic_inner(spec, &analyzed_e, &analyzed_b, &e, &b),
                    vector_map_inner(&first, &second, &adjoint_first, &adjoint_second),
                    3.0e-5,
                );

                let mut gradient_first = vec![0.0_f32; ntheta * nphi];
                let mut gradient_second = vec![0.0_f32; ntheta * nphi];
                plan.synthesis_gradient_into(
                    &e,
                    &mut gradient_first,
                    &mut gradient_second,
                    &mut workspace,
                )
                .unwrap();
                let mut gradient_adjoint = vec![Complex32::new(0.0, 0.0); spec.nalm()];
                plan.adjoint_synthesis_gradient_into(
                    &first,
                    &second,
                    &mut gradient_adjoint,
                    &mut workspace,
                )
                .unwrap();
                assert_close(
                    vector_map_inner(&gradient_first, &gradient_second, &first, &second),
                    harmonic_inner(spec, &e, &gradient_adjoint),
                    3.0e-5,
                );
            }
        }
    }
}

#[test]
fn f32_spin_one_gl_synthesis_matches_ducc_0410() {
    // Reference: ducc0 0.41.0, GL 2x3, spin=1, phi0=0.23, one thread.
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, 2, 3, 0.23).unwrap(),
        HarmonicSpec::new(1, 1, 1).unwrap(),
    )
    .unwrap();
    let mut e = vec![Complex32::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b = vec![Complex32::new(0.0, 0.0); plan.harmonics().nalm()];
    e[plan.harmonics().index(1, 0).unwrap()] = Complex32::new(0.26, 0.0);
    e[plan.harmonics().index(1, 1).unwrap()] = Complex32::new(0.39, -0.07);
    b[plan.harmonics().index(1, 0).unwrap()] = Complex32::new(-0.27, 0.0);
    b[plan.harmonics().index(1, 1).unwrap()] = Complex32::new(-0.12, 0.11);

    let mut first = vec![0.0_f32; 6];
    let mut second = vec![0.0_f32; 6];
    let mut workspace = plan.workspace().unwrap();
    plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
        .unwrap();
    assert_values_close(
        &first,
        &[
            -0.2239303440,
            0.0670698583,
            -0.0631734356,
            -0.0006871219,
            -0.0546924919,
            -0.1646543145,
        ],
        2.0e-6,
    );
    assert_values_close(
        &second,
        &[
            0.1263405234,
            0.2379926145,
            -0.1358363330,
            0.0462721549,
            0.2390626967,
            -0.0568380430,
        ],
        2.0e-6,
    );
}

#[test]
fn f32_derivative_synthesis_matches_analytic_harmonic_and_adjoint_identity() {
    let plan = make_scalar_plan(GridKind::GaussLegendre, 3, 5);
    let spec = plan.harmonics();
    let mut workspace = plan.workspace().unwrap();
    let mut alm = vec![Complex32::new(0.0, 0.0); spec.nalm()];
    let coefficient = Complex32::new(0.3, 0.2);
    alm[spec.index(1, 1).unwrap()] = coefficient;
    let mut theta_derivative = vec![0.0_f32; 15];
    let mut phi_derivative = vec![0.0_f32; 15];
    plan.synthesis_deriv1_into(
        &alm,
        &mut theta_derivative,
        &mut phi_derivative,
        &mut workspace,
    )
    .unwrap();
    let amplitude = (3.0 / (8.0 * PI)).sqrt();
    for (ring, &theta64) in plan.grid().theta().iter().enumerate() {
        let theta = theta64 as f32;
        let cosine = theta.cos();
        for longitude in 0..5 {
            let phi = 0.37_f32 + TAU as f32 * longitude as f32 / 5.0;
            let (sin_phi, cos_phi) = phi.sin_cos();
            let real_part = coefficient.re * cos_phi - coefficient.im * sin_phi;
            let imaginary_part = coefficient.re * sin_phi + coefficient.im * cos_phi;
            let index = ring * 5 + longitude;
            let expected_theta = -2.0 * amplitude * cosine * real_part;
            let expected_phi = 2.0 * amplitude * imaginary_part;
            assert!((theta_derivative[index] - expected_theta).abs() <= 3.0e-6);
            assert!((phi_derivative[index] - expected_phi).abs() <= 3.0e-6);
        }
    }

    let first = arbitrary_map(3, 5, 0.14);
    let second = arbitrary_map(3, 5, -0.33);
    let mut adjoint = vec![Complex32::new(0.0, 0.0); spec.nalm()];
    plan.adjoint_synthesis_deriv1_into(&first, &second, &mut adjoint, &mut workspace)
        .unwrap();
    assert_close(
        map_inner(&theta_derivative, &first) + map_inner(&phi_derivative, &second),
        harmonic_inner(spec, &alm, &adjoint),
        3.0e-5,
    );
}

#[test]
fn f32_t127_sectoral_synthesis_remains_finite_and_matches_analytic_values() {
    let lmax = 127;
    let ntheta = lmax + 1;
    let nphi = 2 * lmax + 3;
    let phi0 = 0.19;
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, phi0).unwrap(),
        HarmonicSpec::scalar(lmax, lmax).unwrap(),
    )
    .unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut alm = vec![Complex32::new(0.0, 0.0); plan.harmonics().nalm()];
    alm[plan.harmonics().index(lmax, lmax).unwrap()] = Complex32::new(1.0, 0.0);
    let mut map = vec![0.0_f32; ntheta * nphi];
    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
    assert!(map.iter().all(|value| value.is_finite()));

    let mut sectoral_norm = 1.0_f64 / (4.0 * std::f64::consts::PI).sqrt();
    for order in 1..=lmax {
        sectoral_norm *= ((2 * order + 1) as f64 / (2 * order) as f64).sqrt();
    }
    for (ring, &theta) in plan.grid().theta().iter().enumerate() {
        let expected_amplitude = 2.0
            * sectoral_norm
            * theta.sin().powi(lmax as i32)
            * if lmax.is_multiple_of(2) { 1.0 } else { -1.0 };
        for longitude in 0..nphi {
            let phi = phi0 + TAU * longitude as f64 / nphi as f64;
            let expected = (expected_amplitude * (lmax as f64 * phi).cos()) as f32;
            let actual = map[ring * nphi + longitude];
            assert!((actual - expected).abs() <= 2.0e-4 * expected.abs().max(1.0e-4));
        }
    }
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain printed DUCC 0.41.0 float32 analysis references."
)]
fn f32_spin1_gl_t511_analysis_matches_ducc_0410() {
    // DUCC 0.41.0, source revision c1ed01b8fda69d2a378190f0915f1a19762dddbf;
    // GL 512x1025, T511, phi0=0.37, nthreads=1. The maps use integer-derived
    // float32 values so this test can reproduce the DUCC inputs exactly.
    let lmax = 511;
    let ntheta = lmax + 1;
    let nphi = 2 * lmax + 3;
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::new(lmax, lmax, 1).unwrap(),
    )
    .unwrap();
    let mut first = Vec::with_capacity(ntheta * nphi);
    let mut second = Vec::with_capacity(ntheta * nphi);
    for ring in 0..ntheta {
        for longitude in 0..nphi {
            first.push((((ring * 31 + longitude * 17) % 101) as i32 - 50) as f32 / 101.0);
            second.push((((ring * 19 + longitude * 23 + 7) % 97) as i32 - 48) as f32 / 97.0);
        }
    }

    let mut workspace = plan.workspace().unwrap();
    let mut analyzed_e = vec![Complex32::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut analyzed_b = vec![Complex32::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.analysis_spin_into(
        &first,
        &second,
        &mut analyzed_e,
        &mut analyzed_b,
        &mut workspace,
    )
    .unwrap();

    for (degree, order, expected_e, expected_b) in [
        (
            1,
            0,
            Complex32::new(-4.1415304963e-8, 0.0),
            Complex32::new(1.8658342427e-8, 0.0),
        ),
        (
            1,
            1,
            Complex32::new(-6.5692361773e-8, -2.7879175946e-7),
            Complex32::new(-6.5725276954e-7, -5.0758973202e-7),
        ),
        (
            2,
            1,
            Complex32::new(-7.4319234500e-7, 8.8196873094e-7),
            Complex32::new(3.8780314071e-7, -9.5410712220e-8),
        ),
        (
            37,
            2,
            Complex32::new(2.1354905755e-7, 2.4964915610e-6),
            Complex32::new(-2.1649178006e-9, 3.7691387433e-7),
        ),
        (
            127,
            31,
            Complex32::new(-3.8519733607e-6, 2.3106015306e-6),
            Complex32::new(1.9163400793e-5, 9.2727168521e-6),
        ),
        (
            255,
            128,
            Complex32::new(-1.4265302525e-5, 1.9322160369e-5),
            Complex32::new(3.0104571124e-5, 1.7585711248e-5),
        ),
        (
            511,
            511,
            Complex32::new(7.2307525443e-6, 6.4677387854e-6),
            Complex32::new(-1.6438199964e-5, -2.2784601242e-5),
        ),
    ] {
        let index = plan.harmonics().index(degree, order).unwrap();
        assert!(
            (analyzed_e[index] - expected_e).norm() <= 3.0e-7,
            "E_{{{degree},{order}}}: actual={:?}, DUCC={expected_e:?}",
            analyzed_e[index]
        );
        assert!(
            (analyzed_b[index] - expected_b).norm() <= 3.0e-7,
            "B_{{{degree},{order}}}: actual={:?}, DUCC={expected_b:?}",
            analyzed_b[index]
        );
    }
}

#[test]
fn f32_t127_spin1_gl_sparse_synthesis_matches_ducc_0410() {
    // Reference: ducc0 0.41.0, GL 128x257, spin=1, a_127,127=1, phi0=0.19.
    let lmax = 127;
    let ntheta = lmax + 1;
    let nphi = 257;
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.19).unwrap(),
        HarmonicSpec::new(lmax, lmax, 1).unwrap(),
    )
    .unwrap();
    let mut alm = vec![Complex32::new(0.0, 0.0); plan.harmonics().nalm()];
    alm[plan.harmonics().index(lmax, lmax).unwrap()] = Complex32::new(1.0, 0.0);
    let mut first = vec![0.0_f32; ntheta * nphi];
    let mut second = vec![0.0_f32; ntheta * nphi];
    let mut workspace = plan.workspace().unwrap();
    plan.synthesis_gradient_into(&alm, &mut first, &mut second, &mut workspace)
        .unwrap();
    assert!(first.iter().chain(&second).all(|value| value.is_finite()));

    for &(ring, longitude, expected_first, expected_second) in &[
        (31, 0, -2.605_968_4e-20, -5.722_241_6e-20),
        (31, 64, 4.739_284_5e-20, 1.402_982_2e-20),
        (63, 0, -0.013_074_684, -1.675_917_6),
        (63, 1, 0.012_314_796, 1.714_007_4),
        (63, 64, 0.023_777_973, 0.410_902_32),
        (63, 128, -0.020_242_743, 1.100_166),
        (64, 0, 0.013_074_684, -1.675_917_6),
        (64, 1, -0.012_314_796, 1.714_007_4),
        (96, 128, 4.034_663_6e-20, 3.756_399_1e-20),
    ] {
        let index = ring * nphi + longitude;
        assert!((first[index] - expected_first).abs() <= 5.0e-5);
        assert!((second[index] - expected_second).abs() <= 5.0e-5);
    }
}

#[test]
fn f32_longitude_phase_handles_large_finite_phi0_for_scalar_and_spin() {
    let phi0 = f64::MAX;
    let scalar32 = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, 3, 5, phi0).unwrap(),
        HarmonicSpec::scalar(2, 2).unwrap(),
    )
    .unwrap();
    let scalar64 = ShtPlan::<f64>::new(
        GridSpec::new(GridKind::GaussLegendre, 3, 5, phi0).unwrap(),
        HarmonicSpec::scalar(2, 2).unwrap(),
    )
    .unwrap();
    let mut sectoral = vec![Complex32::new(0.0, 0.0); scalar32.harmonics().nalm()];
    let sectoral_coefficient = Complex32::new(0.37, -0.21);
    sectoral[scalar32.harmonics().index(2, 2).unwrap()] = sectoral_coefficient;
    let mut sectoral_map = vec![0.0_f32; 15];
    scalar32
        .synthesis_into(
            &sectoral,
            &mut sectoral_map,
            &mut scalar32.workspace().unwrap(),
        )
        .unwrap();
    let (sin_phi0, cos_phi0) = phi0.sin_cos();
    let phase_at_phi0 = Complex64::new(cos_phi0, sin_phi0);
    let phase_at_twice_phi0 = phase_at_phi0 * phase_at_phi0;
    let sectoral_norm = (15.0 / (32.0 * std::f64::consts::PI)).sqrt();
    for (ring, &theta) in scalar32.grid().theta().iter().enumerate() {
        let amplitude = 2.0 * sectoral_norm * theta.sin().powi(2);
        for longitude in 0..5 {
            let ring_phase = Complex64::from_polar(1.0, TAU * 2.0 * longitude as f64 / 5.0);
            let coefficient = Complex64::new(
                f64::from(sectoral_coefficient.re),
                f64::from(sectoral_coefficient.im),
            );
            let expected = (amplitude * (coefficient * phase_at_twice_phi0 * ring_phase).re) as f32;
            let actual = sectoral_map[ring * 5 + longitude];
            assert!(
                (actual - expected).abs() <= 3.0e-6,
                "sample ({ring}, {longitude}): actual {actual:.9e}, expected {expected:.9e}"
            );
        }
    }

    let coefficients32 = f32_coefficients(scalar32.harmonics());
    let coefficients64 = coefficients32
        .iter()
        .map(|value| Complex64::new(f64::from(value.re), f64::from(value.im)))
        .collect::<Vec<_>>();
    let mut map32 = vec![0.0_f32; 15];
    let mut map64 = vec![0.0_f64; 15];
    scalar32
        .synthesis_into(
            &coefficients32,
            &mut map32,
            &mut scalar32.workspace().unwrap(),
        )
        .unwrap();
    scalar64
        .synthesis_into(
            &coefficients64,
            &mut map64,
            &mut scalar64.workspace().unwrap(),
        )
        .unwrap();
    assert!(map32.iter().all(|value| value.is_finite()));
    assert!(map64.iter().all(|value| value.is_finite()));
    for (&value32, &value64) in map32.iter().zip(&map64) {
        assert!((f64::from(value32) - value64).abs() <= 3.0e-6);
    }
    let scalar_map32 = arbitrary_map(3, 5, 0.41);
    let scalar_map64 = scalar_map32
        .iter()
        .map(|&value| f64::from(value))
        .collect::<Vec<_>>();
    let mut analyzed32 = vec![Complex32::new(0.0, 0.0); scalar32.harmonics().nalm()];
    let mut analyzed64 = vec![Complex64::new(0.0, 0.0); scalar64.harmonics().nalm()];
    scalar32
        .analysis_into(
            &scalar_map32,
            &mut analyzed32,
            &mut scalar32.workspace().unwrap(),
        )
        .unwrap();
    scalar64
        .analysis_into(
            &scalar_map64,
            &mut analyzed64,
            &mut scalar64.workspace().unwrap(),
        )
        .unwrap();
    for (&value32, &value64) in analyzed32.iter().zip(&analyzed64) {
        assert!((f64::from(value32.re) - value64.re).abs() <= 3.0e-6);
        assert!((f64::from(value32.im) - value64.im).abs() <= 3.0e-6);
    }

    let spin32 = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, 2, 3, phi0).unwrap(),
        HarmonicSpec::new(1, 1, 1).unwrap(),
    )
    .unwrap();
    let spin64 = ShtPlan::<f64>::new(
        GridSpec::new(GridKind::GaussLegendre, 2, 3, phi0).unwrap(),
        HarmonicSpec::new(1, 1, 1).unwrap(),
    )
    .unwrap();
    let mut e32 = vec![Complex32::new(0.0, 0.0); spin32.harmonics().nalm()];
    let mut b32 = e32.clone();
    e32[spin32.harmonics().index(1, 0).unwrap()] = Complex32::new(0.23, 0.0);
    e32[spin32.harmonics().index(1, 1).unwrap()] = Complex32::new(-0.31, 0.17);
    b32[spin32.harmonics().index(1, 0).unwrap()] = Complex32::new(-0.11, 0.0);
    b32[spin32.harmonics().index(1, 1).unwrap()] = Complex32::new(0.07, -0.19);
    let to_f64 = |values: &[Complex32]| {
        values
            .iter()
            .map(|value| Complex64::new(f64::from(value.re), f64::from(value.im)))
            .collect::<Vec<_>>()
    };
    let e64 = to_f64(&e32);
    let b64 = to_f64(&b32);
    let mut first32 = vec![0.0_f32; 6];
    let mut second32 = vec![0.0_f32; 6];
    let mut first64 = vec![0.0_f64; 6];
    let mut second64 = vec![0.0_f64; 6];
    spin32
        .synthesis_spin_into(
            &e32,
            &b32,
            &mut first32,
            &mut second32,
            &mut spin32.workspace().unwrap(),
        )
        .unwrap();
    spin64
        .synthesis_spin_into(
            &e64,
            &b64,
            &mut first64,
            &mut second64,
            &mut spin64.workspace().unwrap(),
        )
        .unwrap();
    for (values32, values64) in [(&first32, &first64), (&second32, &second64)] {
        assert!(values32.iter().all(|value| value.is_finite()));
        assert!(values64.iter().all(|value| value.is_finite()));
        for (&value32, &value64) in values32.iter().zip(values64) {
            assert!((f64::from(value32) - value64).abs() <= 3.0e-6);
        }
    }
    let mut analyzed_e32 = vec![Complex32::new(0.0, 0.0); spin32.harmonics().nalm()];
    let mut analyzed_b32 = vec![Complex32::new(0.0, 0.0); spin32.harmonics().nalm()];
    let mut analyzed_e64 = vec![Complex64::new(0.0, 0.0); spin64.harmonics().nalm()];
    let mut analyzed_b64 = vec![Complex64::new(0.0, 0.0); spin64.harmonics().nalm()];
    spin32
        .analysis_spin_into(
            &first32,
            &second32,
            &mut analyzed_e32,
            &mut analyzed_b32,
            &mut spin32.workspace().unwrap(),
        )
        .unwrap();
    spin64
        .analysis_spin_into(
            &first64,
            &second64,
            &mut analyzed_e64,
            &mut analyzed_b64,
            &mut spin64.workspace().unwrap(),
        )
        .unwrap();
    for (values32, values64) in [
        (&analyzed_e32, &analyzed_e64),
        (&analyzed_b32, &analyzed_b64),
    ] {
        for (&value32, &value64) in values32.iter().zip(values64) {
            assert!((f64::from(value32.re) - value64.re).abs() <= 3.0e-6);
            assert!((f64::from(value32.im) - value64.im).abs() <= 3.0e-6);
        }
    }
}

#[test]
fn f32_cc_scalar_synthesis_is_constant_at_the_south_pole() {
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::ClenshawCurtis, 4, 5, 0.31).unwrap(),
        HarmonicSpec::scalar(1, 1).unwrap(),
    )
    .unwrap();
    let mut coefficients = vec![Complex32::new(0.0, 0.0); plan.harmonics().nalm()];
    coefficients[plan.harmonics().index(1, 1).unwrap()] = Complex32::new(1.0, 0.0);
    let mut map = vec![0.0_f32; 20];
    plan.synthesis_into(&coefficients, &mut map, &mut plan.workspace().unwrap())
        .unwrap();

    let south_pole = &map[15..20];
    assert!(
        south_pole.iter().all(|&sample| sample == 0.0),
        "south-pole samples must be independent of longitude and exactly zero: {south_pole:?}"
    );
}
