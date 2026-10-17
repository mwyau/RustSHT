// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use rustsht::{Complex32, Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

#[test]
fn finite_extreme_scalar_monopole() {
    for nphi in [1, 2, 3, 8, 9, 32] {
        let plan = ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, 1, nphi, 0.37).unwrap(),
            HarmonicSpec::scalar(0, 0).unwrap(),
        )
        .unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut map = vec![0.0; nphi];
        plan.synthesis_into(&[Complex64::new(1.0e308, 0.0)], &mut map, &mut workspace)
            .unwrap();
        let expected = 1.0e308 / (4.0 * std::f64::consts::PI).sqrt();
        assert!(
            map.iter()
                .all(|&value| (value / expected - 1.0).abs() < 1.0e-14),
            "nphi={nphi}, {map:?}"
        );
    }
}

#[test]
fn finite_extreme_f32_scalar_monopole() {
    for nphi in [1, 2, 3, 8, 9, 32] {
        let plan = ShtPlan::<f32>::new(
            GridSpec::new(GridKind::GaussLegendre, 1, nphi, 0.37).unwrap(),
            HarmonicSpec::scalar(0, 0).unwrap(),
        )
        .unwrap();
        let mut workspace = plan.workspace().unwrap();
        let mut map = vec![0.0; nphi];
        plan.synthesis_into(&[Complex32::new(2.0e38, 0.0)], &mut map, &mut workspace)
            .unwrap();
        let expected = 2.0e38 / (4.0 * std::f32::consts::PI).sqrt();
        assert!(
            map.iter()
                .all(|&value| (value / expected - 1.0).abs() < 2.0e-6),
            "nphi={nphi}, {map:?}"
        );
    }
}

macro_rules! range_controls {
    ($test:ident, $real:ty, $complex:ty, $magnitude:expr, $tolerance:expr) => {
        #[test]
        fn $test() {
            let magnitude: $real = $magnitude;
            let close = |actual: $real, expected: $real| {
                let scaled = actual / magnitude;
                assert!(scaled.is_finite());
                assert!(
                    (scaled - expected).abs() <= $tolerance * expected.abs().max(1.0),
                    "scaled={scaled}, expected={expected}"
                );
            };
            for kind in [
                GridKind::GaussLegendre,
                GridKind::ClenshawCurtis,
                GridKind::Fejer1,
            ] {
                for spin in 0..=4 {
                    let lmax = 4;
                    let ntheta = if kind == GridKind::ClenshawCurtis {
                        6
                    } else {
                        5
                    };
                    for nphi in [9, 10] {
                        let plan = ShtPlan::<$real>::new(
                            GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
                            HarmonicSpec::new(lmax, lmax, spin).unwrap(),
                        )
                        .unwrap();
                        let mut workspace = plan.workspace().unwrap();
                        let mut e = vec![<$complex>::new(0.0, 0.0); plan.harmonics().nalm()];
                        let mut b = e.clone();
                        for m in 0..=lmax {
                            for l in m.max(spin)..=lmax {
                                let index = plan.harmonics().index(l, m).unwrap();
                                e[index] = <$complex>::new(
                                    0.03 * (l + 1) as $real,
                                    if m == 0 { 0.0 } else { -0.02 * m as $real },
                                );
                                b[index] = e[index] * -0.7;
                            }
                        }
                        let huge_e: Vec<_> = e.iter().map(|&value| value * magnitude).collect();
                        let huge_b: Vec<_> = b.iter().map(|&value| value * magnitude).collect();
                        let mut first = vec![0.0; ntheta * nphi];
                        let mut second = first.clone();
                        let mut huge_first = first.clone();
                        let mut huge_second = first.clone();
                        if spin == 0 {
                            plan.synthesis_into(&e, &mut first, &mut workspace).unwrap();
                            plan.synthesis_into(&huge_e, &mut huge_first, &mut workspace)
                                .unwrap();
                        } else {
                            plan.synthesis_spin_into(
                                &e,
                                &b,
                                &mut first,
                                &mut second,
                                &mut workspace,
                            )
                            .unwrap();
                            plan.synthesis_spin_into(
                                &huge_e,
                                &huge_b,
                                &mut huge_first,
                                &mut huge_second,
                                &mut workspace,
                            )
                            .unwrap();
                        }
                        for (&actual, &expected) in huge_first
                            .iter()
                            .chain(&huge_second)
                            .zip(first.iter().chain(&second))
                        {
                            close(actual, expected);
                        }
                        for adjoint in [false, true] {
                            let mut actual_e = e.clone();
                            let mut actual_b = b.clone();
                            let mut expected_e = e.clone();
                            let mut expected_b = b.clone();
                            if spin == 0 {
                                if adjoint {
                                    plan.adjoint_synthesis_into(
                                        &first,
                                        &mut expected_e,
                                        &mut workspace,
                                    )
                                    .unwrap();
                                    plan.adjoint_synthesis_into(
                                        &huge_first,
                                        &mut actual_e,
                                        &mut workspace,
                                    )
                                    .unwrap();
                                } else {
                                    plan.analysis_into(&first, &mut expected_e, &mut workspace)
                                        .unwrap();
                                    plan.analysis_into(&huge_first, &mut actual_e, &mut workspace)
                                        .unwrap();
                                }
                                actual_b.fill(<$complex>::new(0.0, 0.0));
                                expected_b.fill(<$complex>::new(0.0, 0.0));
                            } else if adjoint {
                                plan.adjoint_synthesis_spin_into(
                                    &first,
                                    &second,
                                    &mut expected_e,
                                    &mut expected_b,
                                    &mut workspace,
                                )
                                .unwrap();
                                plan.adjoint_synthesis_spin_into(
                                    &huge_first,
                                    &huge_second,
                                    &mut actual_e,
                                    &mut actual_b,
                                    &mut workspace,
                                )
                                .unwrap();
                            } else {
                                plan.analysis_spin_into(
                                    &first,
                                    &second,
                                    &mut expected_e,
                                    &mut expected_b,
                                    &mut workspace,
                                )
                                .unwrap();
                                plan.analysis_spin_into(
                                    &huge_first,
                                    &huge_second,
                                    &mut actual_e,
                                    &mut actual_b,
                                    &mut workspace,
                                )
                                .unwrap();
                            }
                            for (&actual, &expected) in actual_e
                                .iter()
                                .chain(&actual_b)
                                .zip(expected_e.iter().chain(&expected_b))
                            {
                                close(actual.re, expected.re);
                                close(actual.im, expected.im);
                            }
                        }
                        if spin == 0 {
                            plan.adjoint_analysis_into(&e, &mut first, &mut workspace)
                                .unwrap();
                            plan.adjoint_analysis_into(&huge_e, &mut huge_first, &mut workspace)
                                .unwrap();
                        } else {
                            plan.adjoint_analysis_spin_into(
                                &e,
                                &b,
                                &mut first,
                                &mut second,
                                &mut workspace,
                            )
                            .unwrap();
                            plan.adjoint_analysis_spin_into(
                                &huge_e,
                                &huge_b,
                                &mut huge_first,
                                &mut huge_second,
                                &mut workspace,
                            )
                            .unwrap();
                        }
                        for (&actual, &expected) in huge_first
                            .iter()
                            .chain(&huge_second)
                            .zip(first.iter().chain(&second))
                        {
                            close(actual, expected);
                        }
                    }
                }
            }
        }
    };
}

range_controls!(
    f64_operator_range_controls,
    f64,
    Complex64,
    1.0e306,
    2.0e-11
);
range_controls!(f32_operator_range_controls, f32, Complex32, 1.0e35, 2.0e-5);
