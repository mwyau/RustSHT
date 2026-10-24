// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use num_complex::Complex64;

use crate::plan::LegendreReuseEstimate;
use crate::{GridKind, GridSpec, HarmonicSpec};

fn in_single_thread_pool<T: Send>(operation: impl FnOnce() -> T + Send) -> T {
    #[cfg(feature = "parallel")]
    {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        pool.install(operation)
    }
    #[cfg(not(feature = "parallel"))]
    operation()
}

fn make_plan(spin: usize, lmax: usize, mmax: usize, ntheta: usize, nphi: usize) -> ShtPlan {
    ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::new(lmax, mmax, spin).unwrap(),
    )
    .unwrap()
}

fn make_cc_plan(spin: usize, lmax: usize, mmax: usize, ntheta: usize, nphi: usize) -> ShtPlan {
    ShtPlan::new(
        GridSpec::new(GridKind::ClenshawCurtis, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::new(lmax, mmax, spin).unwrap(),
    )
    .unwrap()
}

fn make_f1_plan(spin: usize, lmax: usize, mmax: usize, ntheta: usize, nphi: usize) -> ShtPlan {
    ShtPlan::new(
        GridSpec::new(GridKind::Fejer1, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::new(lmax, mmax, spin).unwrap(),
    )
    .unwrap()
}

#[test]
fn spin1_prepared_latitudes_are_exactly_reflected_on_supported_grids() {
    for (kind, ntheta) in [
        (GridKind::GaussLegendre, 6),
        (GridKind::GaussLegendre, 7),
        (GridKind::ClenshawCurtis, 6),
        (GridKind::ClenshawCurtis, 7),
        (GridKind::Fejer1, 6),
        (GridKind::Fejer1, 7),
    ] {
        let plan: ShtPlan<f64> = ShtPlan::new(
            GridSpec::new(kind, ntheta, 13, 0.37).unwrap(),
            HarmonicSpec::new(5, 5, 1).unwrap(),
        )
        .unwrap();
        let mut workspace = plan.workspace().unwrap();
        plan.prepare_spin_latitudes(&mut workspace, 1);

        for north in 0..ntheta / 2 {
            let south = ntheta - 1 - north;
            assert_eq!(workspace.theta_sin[north], workspace.theta_sin[south]);
            assert_eq!(workspace.theta_cos[north], -workspace.theta_cos[south]);
            assert_eq!(
                workspace.ring_order_limits[north],
                workspace.ring_order_limits[south]
            );
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_reuse_estimate_gates_analysis_table_by_executing_pool() {
    let four_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let make = |analysis_calls| -> ShtPlan {
        ShtPlan::new_with_legendre_reuse_estimate(
            GridSpec::new(GridKind::GaussLegendre, 320, 640, 0.37).unwrap(),
            HarmonicSpec::new(159, 159, 1).unwrap(),
            LegendreReuseEstimate::new(None, Some(analysis_calls)),
        )
        .unwrap()
    };
    let before = make(29);
    let at_break_even = make(30);
    assert!(!four_threads.install(|| before.allow_spin1_analysis_table()));
    assert!(four_threads.install(|| at_break_even.allow_spin1_analysis_table()));

    let sixteen_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build()
        .unwrap();
    let q16_before = make(124);
    let q16_at_break_even = make(125);
    assert!(!sixteen_threads.install(|| q16_before.allow_spin1_analysis_table()));
    assert!(sixteen_threads.install(|| q16_at_break_even.allow_spin1_analysis_table()));
}

#[test]
fn spin1_reuse_estimate_keeps_short_synthesis_on_recurrence() {
    let make = |synthesis_calls| -> ShtPlan {
        ShtPlan::new_with_legendre_reuse_estimate(
            GridSpec::new(GridKind::GaussLegendre, 160, 321, 0.37).unwrap(),
            HarmonicSpec::new(159, 159, 1).unwrap(),
            LegendreReuseEstimate::new(Some(synthesis_calls), None),
        )
        .unwrap()
    };
    let short = make(31);
    let at_break_even = make(32);
    let (mut e, mut b) = coefficients(&short);
    for value in e.iter_mut().chain(&mut b) {
        *value *= 1e-5;
    }
    let mut short_first = vec![0.0; short.sample_count];
    let mut short_second = vec![0.0; short.sample_count];
    let mut short_workspace = in_single_thread_pool(|| short.workspace()).unwrap();
    in_single_thread_pool(|| {
        short.synthesis_spin_into(
            &e,
            &b,
            &mut short_first,
            &mut short_second,
            &mut short_workspace,
        )
    })
    .unwrap();
    assert!(short.spin1_scalar_legendre_table.get().is_none());

    let mut table_first = vec![0.0; at_break_even.sample_count];
    let mut table_second = vec![0.0; at_break_even.sample_count];
    let mut table_workspace = in_single_thread_pool(|| at_break_even.workspace()).unwrap();
    in_single_thread_pool(|| {
        at_break_even.synthesis_spin_into(
            &e,
            &b,
            &mut table_first,
            &mut table_second,
            &mut table_workspace,
        )
    })
    .unwrap();
    assert!(
        at_break_even
            .spin1_scalar_legendre_table
            .get()
            .is_some_and(Option::is_some)
    );
    for (actual, expected) in table_first.iter().zip(&short_first) {
        assert!((actual - expected).abs() <= 3e-11);
    }
    for (actual, expected) in table_second.iter().zip(&short_second) {
        assert!((actual - expected).abs() <= 3e-11);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_adjoint_synthesis_does_not_build_analysis_table() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let plan = ShtPlan::new_with_legendre_reuse_estimate(
        GridSpec::new(GridKind::GaussLegendre, 320, 640, 0.37).unwrap(),
        HarmonicSpec::new(159, 159, 1).unwrap(),
        LegendreReuseEstimate::new(None, Some(100)),
    )
    .unwrap();
    let first = vec![0.0; plan.sample_count];
    let second = vec![0.0; plan.sample_count];
    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut b_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut workspace = pool.install(|| plan.workspace()).unwrap();

    pool.install(|| {
        plan.adjoint_synthesis_spin_into(&first, &second, &mut e_alm, &mut b_alm, &mut workspace)
    })
    .unwrap();

    assert!(plan.spin1_scalar_legendre_table.get().is_none());
}

fn coefficients(plan: &ShtPlan) -> (Vec<Complex64>, Vec<Complex64>) {
    let spec = plan.harmonics();
    let mut e = vec![Complex64::new(0.0, 0.0); spec.nalm()];
    let mut b = vec![Complex64::new(0.0, 0.0); spec.nalm()];
    for m in 0..=spec.mmax() {
        for l in spec.spin().max(m)..=spec.lmax() {
            let index = spec.index(l, m).unwrap();
            e[index] = Complex64::new(
                0.03 * (1 + l + 2 * m) as f64,
                if m == 0 { 0.0 } else { 0.017 * (l - m) as f64 },
            );
            b[index] = Complex64::new(
                -0.02 * (2 + l + m) as f64,
                if m == 0 {
                    0.0
                } else {
                    0.013 * (2 * l - m) as f64
                },
            );
        }
    }
    (e, b)
}

fn arbitrary_maps(plan: &ShtPlan) -> (Vec<f64>, Vec<f64>) {
    let mut first = Vec::with_capacity(plan.sample_count);
    let mut second = Vec::with_capacity(plan.sample_count);
    for i in 0..plan.grid.ntheta() {
        for j in 0..plan.grid.nphi() {
            first.push(
                (0.17 * i as f64 + 0.23 * j as f64).sin()
                    + 0.2 * (0.11 * i as f64 - 0.31 * j as f64).cos(),
            );
            second.push(
                (0.19 * i as f64 - 0.13 * j as f64).cos()
                    - 0.15 * (0.29 * i as f64 + 0.07 * j as f64).sin(),
            );
        }
    }
    (first, second)
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.17e}, expected={expected:.17e}, error={:.3e}",
        (actual - expected).abs()
    );
}

fn assert_complex_close(actual: Complex64, expected: Complex64, tolerance: f64) {
    assert!(
        (actual - expected).norm() <= tolerance,
        "actual={actual:?}, expected={expected:?}, error={:.3e}",
        (actual - expected).norm()
    );
}

#[cfg(feature = "parallel")]
fn relative_l2_real(actual: &[f64], expected: &[f64]) -> f64 {
    let (difference, reference) = actual.iter().zip(expected).fold(
        (0.0, 0.0),
        |(difference, reference), (&actual, &expected)| {
            (
                difference + (actual - expected).powi(2),
                reference + expected.powi(2),
            )
        },
    );
    (difference / reference.max(1e-300)).sqrt()
}

#[cfg(feature = "parallel")]
fn relative_l2_complex(actual: &[Complex64], expected: &[Complex64]) -> f64 {
    let (difference, reference) = actual.iter().zip(expected).fold(
        (0.0, 0.0),
        |(difference, reference), (&actual, &expected)| {
            (
                difference + (actual - expected).norm_sqr(),
                reference + expected.norm_sqr(),
            )
        },
    );
    (difference / reference.max(1e-300)).sqrt()
}

fn map_inner_product(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(x, y)| x * y).sum()
}

#[cfg(feature = "parallel")]
#[test]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain the printed DUCC 0.41.0 high-bandlimit samples."
)]
fn spin1_hemisphere_pair_synthesis_matches_general_recurrence() {
    for (lmax, nphi, threads, sparse_high_mode) in [
        (319, 638, 4, false),
        (320, 643, 4, false),
        (321, 642, 4, false),
        (511, 1025, 8, true),
    ] {
        let plan = make_plan(1, lmax, lmax, lmax + 1, nphi);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        assert_eq!(
            pool.install(|| hemisphere::is_beneficial(&plan, threads)),
            lmax != 511 || threads < 8,
            "T{lmax} q{threads} hemisphere selection"
        );

        let (mut e, mut b) = coefficients(&plan);
        if sparse_high_mode {
            e.fill(Complex64::new(0.0, 0.0));
            b.fill(Complex64::new(0.0, 0.0));
            let high_mode = plan.harmonics().index(lmax, lmax).unwrap();
            e[high_mode] = Complex64::new(1.0, 0.0);
        } else {
            for value in e.iter_mut().chain(&mut b) {
                *value *= 1e-5;
            }
        }

        let variants = if sparse_high_mode {
            vec![(e, b)]
        } else {
            let zero = vec![Complex64::new(0.0, 0.0); e.len()];
            let mixed = (e.clone(), b.clone());
            vec![(e.clone(), zero.clone()), (zero, b.clone()), mixed]
        };

        for (variant_index, (e, b)) in variants.into_iter().enumerate() {
            let mut general_workspace = pool.install(|| plan.workspace()).unwrap();
            let mut paired_workspace = pool.install(|| plan.workspace()).unwrap();
            let mut general_first = vec![0.0; plan.sample_count];
            let mut general_second = vec![0.0; plan.sample_count];
            let mut paired_first = vec![0.0; plan.sample_count];
            let mut paired_second = vec![0.0; plan.sample_count];
            pool.install(|| {
                plan.synthesis_spin_core_with_hemisphere(
                    &e,
                    Some(&b),
                    &mut general_first,
                    &mut general_second,
                    1,
                    false,
                    &mut general_workspace,
                    false,
                )
            })
            .unwrap();
            pool.install(|| {
                plan.synthesis_spin_core_with_hemisphere(
                    &e,
                    Some(&b),
                    &mut paired_first,
                    &mut paired_second,
                    1,
                    false,
                    &mut paired_workspace,
                    true,
                )
            })
            .unwrap();

            for (component, paired, general) in [
                ("first", &paired_first, &general_first),
                ("second", &paired_second, &general_second),
            ] {
                let reference_norm = general
                    .iter()
                    .map(|value| value * value)
                    .sum::<f64>()
                    .sqrt();
                let difference_norm = paired
                    .iter()
                    .zip(general)
                    .map(|(actual, expected)| (actual - expected).powi(2))
                    .sum::<f64>()
                    .sqrt();
                let relative_error = difference_norm / reference_norm.max(1.0);
                assert!(
                    relative_error <= 8e-13,
                    "T{lmax} q{threads} {component} relative L2 error={relative_error:.3e}"
                );
            }

            if sparse_high_mode {
                // Static samples from ducc0==0.41.0, GL T511, phi0=0.37.
                let expected_first = [
                    0.007_309_641_843_778_069_5,
                    -0.007_352_834_967_383_475_3,
                    -0.007_957_060_609_725_711,
                    -0.004_764_733_847_941_087,
                    -0.007_265_830_721_471_665,
                ];
                let expected_second = [
                    1.543_633_476_857_506,
                    -1.521_639_561_779_953_4,
                    -1.153_537_148_741_637_9,
                    2.377_782_452_739_180_7,
                    -1.565_496_884_380_256_1,
                ];
                let middle_ring = plan.grid.ntheta() / 2;
                for (slot, longitude) in [0, 1, 17, nphi / 2, nphi - 1].into_iter().enumerate() {
                    let index = middle_ring * nphi + longitude;
                    assert_close(paired_first[index], expected_first[slot], 8e-11);
                    assert_close(paired_second[index], expected_second[slot], 8e-11);
                }
            }

            if lmax == 319 && variant_index == 2 {
                let (map_first, map_second) = arbitrary_maps(&plan);
                let mut adjoint_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
                let mut adjoint_b = adjoint_e.clone();
                plan.adjoint_synthesis_spin_into(
                    &map_first,
                    &map_second,
                    &mut adjoint_e,
                    &mut adjoint_b,
                    &mut paired_workspace,
                )
                .unwrap();
                let left =
                    spin_map_inner_product(&paired_first, &paired_second, &map_first, &map_second);
                let right =
                    spin_harmonic_inner_product(plan.harmonics(), &e, &b, &adjoint_e, &adjoint_b);
                let relative_error = (left - right).abs() / left.abs().max(right.abs()).max(1.0);
                assert!(
                    relative_error <= 2e-11,
                    "T{lmax} q{threads} paired synthesis adjoint error={relative_error:.3e}"
                );
            }
        }

        if lmax == 319 {
            let mut large_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let large_b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let large_mode = plan.harmonics().index(lmax, 0).unwrap();
            large_e[large_mode] = Complex64::new(1e300, 0.0);
            let mut general_workspace = pool.install(|| plan.workspace()).unwrap();
            let mut paired_workspace = pool.install(|| plan.workspace()).unwrap();
            let mut general_first = vec![0.0; plan.sample_count];
            let mut general_second = vec![0.0; plan.sample_count];
            let mut paired_first = vec![0.0; plan.sample_count];
            let mut paired_second = vec![0.0; plan.sample_count];
            pool.install(|| {
                plan.synthesis_spin_core_with_hemisphere(
                    &large_e,
                    Some(&large_b),
                    &mut general_first,
                    &mut general_second,
                    1,
                    false,
                    &mut general_workspace,
                    false,
                )
            })
            .unwrap();
            pool.install(|| {
                plan.synthesis_spin_core_with_hemisphere(
                    &large_e,
                    Some(&large_b),
                    &mut paired_first,
                    &mut paired_second,
                    1,
                    false,
                    &mut paired_workspace,
                    true,
                )
            })
            .unwrap();
            assert!(paired_first.iter().all(|value| value.is_finite()));
            assert!(paired_second.iter().all(|value| value.is_finite()));
            for (component, paired, general) in [
                ("first", &paired_first, &general_first),
                ("second", &paired_second, &general_second),
            ] {
                let scale = general.iter().map(|value| value.abs()).fold(1.0, f64::max);
                let error = paired
                    .iter()
                    .zip(general)
                    .map(|(actual, expected)| (actual - expected).abs())
                    .fold(0.0, f64::max);
                assert!(
                    error <= 3e-12 * scale,
                    "T{lmax} q{threads} extreme finite {component} error={error:.3e}, scale={scale:.3e}"
                );
            }
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_hemisphere_pair_selector_matches_measured_domain() {
    let eligible = make_plan(1, 319, 319, 320, 643);
    for (threads, expected) in [(1, false), (4, true), (8, true), (16, false)] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        assert_eq!(
            pool.install(|| hemisphere::is_beneficial(&eligible, threads)),
            expected,
            "T319 at {threads} threads"
        );
    }

    let t511 = make_plan(1, 511, 511, 512, 1025);
    let equatorial = make_plan(1, 320, 320, 321, 643);
    let partial_group = make_plan(1, 321, 321, 322, 642);
    let t720 = make_plan(1, 720, 720, 721, 1443);
    let reduced = make_plan(1, 319, 255, 320, 643);
    let cc = make_cc_plan(1, 319, 319, 321, 643);
    let other_spins: Vec<_> = [0, 2, 3, 4]
        .into_iter()
        .map(|spin| make_plan(spin, 319, 319, 320, 643))
        .collect();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    eight_threads.install(|| {
        assert!(!hemisphere::is_beneficial(&t511, 8));
        assert!(hemisphere::is_beneficial(&equatorial, 8));
        assert!(hemisphere::is_beneficial(&partial_group, 8));
        assert!(!hemisphere::is_beneficial(&t720, 8));
        assert!(!hemisphere::is_beneficial(&reduced, 8));
        assert!(!hemisphere::is_beneficial(&cc, 8));
        for plan in &other_spins {
            assert!(!hemisphere::is_beneficial(plan, 8));
            let workspace = plan.workspace().unwrap();
            assert_eq!(workspace.allocated_worker_count(), 10);
        }
    });
    let four_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    four_threads.install(|| assert!(hemisphere::is_beneficial(&t511, 4)));
}

#[cfg(feature = "parallel")]
#[test]
fn cc_f1_parallel_order_resampling_matches_serial_with_reused_workspace() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();

    for (grid_kind, lmax, ntheta) in [
        (GridKind::ClenshawCurtis, 129, 131),
        (GridKind::ClenshawCurtis, 130, 132),
        (GridKind::Fejer1, 129, 130),
        (GridKind::Fejer1, 130, 131),
    ] {
        let nphi = 2 * (lmax + 1);
        let plan = ShtPlan::new(
            GridSpec::new(grid_kind, ntheta, nphi, 0.23).unwrap(),
            HarmonicSpec::new(lmax, lmax, 1).unwrap(),
        )
        .unwrap();
        let resampling = plan.cc_resampling.as_ref().unwrap();
        let spectrum_len = plan.fft.spectrum_len();
        let mut workspace = pool.install(|| plan.workspace()).unwrap();
        plan.prepare_workspace(&mut workspace, 4).unwrap();
        let prepared_count = resampling.prepared_ntheta() * spectrum_len;
        let mut serial = vec![Complex64::new(0.0, 0.0); prepared_count];
        let mut parallel = serial.clone();
        let mut input = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];

        for generation in 0..3 {
            for (index, value) in input.iter_mut().enumerate() {
                *value = match generation {
                    0 => {
                        let phase = (index * 19) as f64 * 0.013;
                        Complex64::new(phase.sin(), phase.cos() * 0.37)
                    }
                    1 => {
                        let tiny = f64::from_bits(1 + (index % 7) as u64);
                        Complex64::new(tiny, if index.is_multiple_of(2) { -tiny } else { 0.0 })
                    }
                    _ => {
                        let sign = if index.is_multiple_of(2) { 1.0 } else { -1.0 };
                        Complex64::new(sign * 1.0e290, -sign * 2.5e289)
                    }
                };
            }
            serial.fill(Complex64::new(91.0, -37.0));
            parallel.fill(Complex64::new(-22.0, 43.0));
            workspace
                .cc_profile
                .fill(Complex64::new(f64::NAN, f64::NAN));
            resampling
                .to_prepared(
                    &input,
                    &mut serial,
                    spectrum_len,
                    lmax,
                    1,
                    &mut workspace.cc_profile,
                    &mut workspace.cc_fft_scratch,
                )
                .unwrap();
            for worker in &mut workspace.cc_resampling_workers {
                worker.profile.fill(Complex64::new(f64::NAN, f64::NAN));
            }
            resampling
                .to_prepared_parallel(
                    &input,
                    &mut parallel,
                    spectrum_len,
                    lmax,
                    1,
                    &mut workspace.cc_resampling_order_tile,
                    &mut workspace.cc_resampling_workers,
                    4,
                )
                .unwrap();
            assert_eq!(
                parallel, serial,
                "{grid_kind:?} T{lmax}, ntheta={ntheta}, pass {generation}"
            );
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn prepared_workspace_matches_fresh_workspace_for_spin1_operations_all_grids() {
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();

    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let plan = ShtPlan::new(
            GridSpec::new(kind, 260, 17, 0.37).unwrap(),
            HarmonicSpec::new(7, 5, 1).unwrap(),
        )
        .unwrap();
        let (input_e, input_b) = coefficients(&plan);
        let (input_q, input_u) = arbitrary_maps(&plan);
        let mut adapted = one.install(|| plan.workspace()).unwrap();
        assert_eq!(adapted.execution_capacity(), 1);
        one.install(|| plan.prepare_workspace(&mut adapted, 4))
            .unwrap();
        assert_eq!(adapted.execution_capacity(), 4);
        let allocated_after_prepare = adapted.allocated_worker_count();
        assert!(allocated_after_prepare > 1);
        let mut fresh = eight.install(|| plan.workspace()).unwrap();
        eight
            .install(|| plan.prepare_workspace(&mut fresh, 4))
            .unwrap();
        assert_eq!(fresh.execution_capacity(), 4);
        let probe = adapted.observe_active_workers();

        let mut adapted_q = vec![0.0; plan.sample_count];
        let mut adapted_u = vec![0.0; plan.sample_count];
        let mut fresh_q = vec![0.0; plan.sample_count];
        let mut fresh_u = vec![0.0; plan.sample_count];
        eight
            .install(|| {
                plan.synthesis_spin_into(
                    &input_e,
                    &input_b,
                    &mut adapted_q,
                    &mut adapted_u,
                    &mut adapted,
                )
            })
            .unwrap();
        let active_workers = probe.lock().expect("worker probe is not poisoned").len();
        assert!((1..=4).contains(&active_workers));
        eight
            .install(|| {
                plan.synthesis_spin_into(&input_e, &input_b, &mut fresh_q, &mut fresh_u, &mut fresh)
            })
            .unwrap();
        assert!(relative_l2_real(&adapted_q, &fresh_q) <= 1e-13);
        assert!(relative_l2_real(&adapted_u, &fresh_u) <= 1e-13);

        let mut adapted_synthesis_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut adapted_synthesis_b = adapted_synthesis_e.clone();
        let mut fresh_synthesis_e = adapted_synthesis_e.clone();
        let mut fresh_synthesis_b = adapted_synthesis_b.clone();
        eight
            .install(|| {
                plan.adjoint_synthesis_spin_into(
                    &input_q,
                    &input_u,
                    &mut adapted_synthesis_e,
                    &mut adapted_synthesis_b,
                    &mut adapted,
                )
            })
            .unwrap();
        eight
            .install(|| {
                plan.adjoint_synthesis_spin_into(
                    &input_q,
                    &input_u,
                    &mut fresh_synthesis_e,
                    &mut fresh_synthesis_b,
                    &mut fresh,
                )
            })
            .unwrap();
        assert!(relative_l2_complex(&adapted_synthesis_e, &fresh_synthesis_e) <= 1e-13);
        assert!(relative_l2_complex(&adapted_synthesis_b, &fresh_synthesis_b) <= 1e-13);
        assert_close(
            spin_map_inner_product(&adapted_q, &adapted_u, &input_q, &input_u),
            spin_harmonic_inner_product(
                plan.harmonics(),
                &input_e,
                &input_b,
                &adapted_synthesis_e,
                &adapted_synthesis_b,
            ),
            2e-10,
        );

        let mut adapted_analysis_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut adapted_analysis_b = adapted_analysis_e.clone();
        let mut fresh_analysis_e = adapted_analysis_e.clone();
        let mut fresh_analysis_b = adapted_analysis_b.clone();
        probe.lock().expect("worker probe is not poisoned").clear();
        eight
            .install(|| {
                plan.analysis_spin_into(
                    &input_q,
                    &input_u,
                    &mut adapted_analysis_e,
                    &mut adapted_analysis_b,
                    &mut adapted,
                )
            })
            .unwrap();
        let active_analysis_workers = probe.lock().expect("worker probe is not poisoned").len();
        assert!((1..=4).contains(&active_analysis_workers));
        eight
            .install(|| {
                plan.analysis_spin_into(
                    &input_q,
                    &input_u,
                    &mut fresh_analysis_e,
                    &mut fresh_analysis_b,
                    &mut fresh,
                )
            })
            .unwrap();
        assert!(relative_l2_complex(&adapted_analysis_e, &fresh_analysis_e) <= 1e-13);
        assert!(relative_l2_complex(&adapted_analysis_b, &fresh_analysis_b) <= 1e-13);

        let mut adapted_analysis_q = vec![0.0; plan.sample_count];
        let mut adapted_analysis_u = vec![0.0; plan.sample_count];
        let mut fresh_analysis_q = vec![0.0; plan.sample_count];
        let mut fresh_analysis_u = vec![0.0; plan.sample_count];
        eight
            .install(|| {
                plan.adjoint_analysis_spin_into(
                    &input_e,
                    &input_b,
                    &mut adapted_analysis_q,
                    &mut adapted_analysis_u,
                    &mut adapted,
                )
            })
            .unwrap();
        eight
            .install(|| {
                plan.adjoint_analysis_spin_into(
                    &input_e,
                    &input_b,
                    &mut fresh_analysis_q,
                    &mut fresh_analysis_u,
                    &mut fresh,
                )
            })
            .unwrap();
        assert!(relative_l2_real(&adapted_analysis_q, &fresh_analysis_q) <= 1e-13);
        assert!(relative_l2_real(&adapted_analysis_u, &fresh_analysis_u) <= 1e-13);
        assert_close(
            spin_harmonic_inner_product(
                plan.harmonics(),
                &adapted_analysis_e,
                &adapted_analysis_b,
                &input_e,
                &input_b,
            ),
            spin_map_inner_product(&input_q, &input_u, &adapted_analysis_q, &adapted_analysis_u),
            2e-10,
        );

        eight
            .install(|| plan.prepare_workspace(&mut adapted, 1))
            .unwrap();
        assert_eq!(adapted.execution_capacity(), 1);
        assert_eq!(adapted.allocated_worker_count(), allocated_after_prepare);
        let mut serial_q = vec![0.0; plan.sample_count];
        let mut serial_u = vec![0.0; plan.sample_count];
        one.install(|| {
            plan.synthesis_spin_into(
                &input_e,
                &input_b,
                &mut serial_q,
                &mut serial_u,
                &mut adapted,
            )
        })
        .unwrap();
        assert!(relative_l2_real(&serial_q, &fresh_q) <= 1e-13);
        assert!(relative_l2_real(&serial_u, &fresh_u) <= 1e-13);
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "paired spin-1 strategy benchmark; run in release mode on an idle pinned host"]
fn benchmark_spin1_hemisphere_pair_dispatch() {
    use std::hint::black_box;
    use std::time::Instant;

    for (lmax, threads) in [
        (319, 1),
        (319, 2),
        (319, 4),
        (319, 8),
        (511, 1),
        (511, 2),
        (511, 4),
        (511, 8),
        (511, 16),
        (720, 8),
    ] {
        let ntheta = lmax + 1;
        let nphi = 2 * lmax + 3;
        let plan = make_plan(1, lmax, lmax, ntheta, nphi);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        let (mut e, mut b) = coefficients(&plan);
        for value in e.iter_mut().chain(&mut b) {
            *value *= 1e-5;
        }
        let mut baseline_workspace = pool.install(|| plan.workspace()).unwrap();
        let mut selected_workspace = pool.install(|| plan.workspace()).unwrap();
        let mut baseline_first = vec![0.0; plan.sample_count];
        let mut baseline_second = vec![0.0; plan.sample_count];
        let mut selected_first = vec![0.0; plan.sample_count];
        let mut selected_second = vec![0.0; plan.sample_count];
        let mut run = |selected: bool| {
            let start = Instant::now();
            if selected {
                pool.install(|| {
                    plan.synthesis_spin_core_with_hemisphere(
                        &e,
                        Some(&b),
                        &mut selected_first,
                        &mut selected_second,
                        1,
                        false,
                        &mut selected_workspace,
                        true,
                    )
                })
                .unwrap();
                black_box((selected_first[0], selected_second[0]));
            } else {
                pool.install(|| {
                    plan.synthesis_spin_core_with_hemisphere(
                        &e,
                        Some(&b),
                        &mut baseline_first,
                        &mut baseline_second,
                        1,
                        false,
                        &mut baseline_workspace,
                        false,
                    )
                })
                .unwrap();
                black_box((baseline_first[0], baseline_second[0]));
            }
            start.elapsed()
        };

        run(false);
        run(true);
        let mut baseline = Vec::with_capacity(7);
        let mut selected = Vec::with_capacity(7);
        for sample in 0_usize..7 {
            if sample.is_multiple_of(2) {
                baseline.push(run(false));
                selected.push(run(true));
            } else {
                selected.push(run(true));
                baseline.push(run(false));
            }
        }
        let relative_l2 = |actual: &[f64], expected: &[f64]| {
            let difference = actual
                .iter()
                .zip(expected)
                .map(|(actual, expected)| (actual - expected).powi(2))
                .sum::<f64>()
                .sqrt();
            let norm = expected
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            difference / norm.max(1.0)
        };
        let error_first = relative_l2(&selected_first, &baseline_first);
        let error_second = relative_l2(&selected_second, &baseline_second);
        let as_ms = |values: &[std::time::Duration]| {
            values
                .iter()
                .map(|duration| duration.as_secs_f64() * 1000.0)
                .collect::<Vec<_>>()
        };
        eprintln!(
            "T{lmax} GL {ntheta}x{nphi} spin-1 synthesis q{threads}; baseline_ms={:?}; selected_ms={:?}; relative_l2=[{error_first:.3e},{error_second:.3e}]; selector={}",
            as_ms(&baseline),
            as_ms(&selected),
            pool.install(|| hemisphere::is_beneficial(&plan, threads))
        );
    }
}

fn harmonic_inner_product(spec: &HarmonicSpec, left: &[Complex64], right: &[Complex64]) -> f64 {
    let mut result = 0.0;
    for m in 0..=spec.mmax() {
        let weight = if m == 0 { 1.0 } else { 2.0 };
        for l in spec.spin().max(m)..=spec.lmax() {
            let index = spec.index(l, m).unwrap();
            result += weight * (left[index].conj() * right[index]).re;
        }
    }
    result
}

fn spin_map_inner_product(
    left_first: &[f64],
    left_second: &[f64],
    right_first: &[f64],
    right_second: &[f64],
) -> f64 {
    map_inner_product(left_first, right_first) + map_inner_product(left_second, right_second)
}

fn spin_harmonic_inner_product(
    spec: &HarmonicSpec,
    left_e: &[Complex64],
    left_b: &[Complex64],
    right_e: &[Complex64],
    right_b: &[Complex64],
) -> f64 {
    harmonic_inner_product(spec, left_e, right_e) + harmonic_inner_product(spec, left_b, right_b)
}

fn sectoral_spin_log_common(degree: usize, spin: usize) -> f64 {
    let log_binomial = (1..=degree - spin)
        .map(|index| ((degree + spin + index) as f64).ln() - (index as f64).ln())
        .sum::<f64>();
    0.5 * (log_binomial + ((2 * degree + 1) as f64 / (4.0 * std::f64::consts::PI)).ln())
        - std::f64::consts::LN_2
}

fn lowest_spin_harmonic_m0_reference(spin: usize, theta: f64) -> f64 {
    let central_binomial = (1..=spin)
        .map(|index| (spin + index) as f64 / index as f64)
        .product::<f64>();
    -(((2 * spin + 1) as f64 * central_binomial / (4.0 * std::f64::consts::PI)).sqrt())
        * (0.5 * theta.sin()).powi(spin as i32)
}

fn sectoral_spin_reference(
    degree: usize,
    spin: usize,
    theta: f64,
    phi: f64,
    log_common: f64,
) -> (f64, f64) {
    let (sin_half, cos_half) = (theta / 2.0).sin_cos();
    let log_power = |base: f64, exponent: usize| {
        if exponent == 0 {
            0.0
        } else if base == 0.0 {
            f64::NEG_INFINITY
        } else {
            exponent as f64 * base.ln()
        }
    };
    let plus_sign = if (degree + spin + 1).is_multiple_of(2) {
        1.0
    } else {
        -1.0
    };
    let minus_sign = if (degree + 1).is_multiple_of(2) {
        1.0
    } else {
        -1.0
    };
    let plus = plus_sign
        * (log_common + log_power(cos_half, degree + spin) + log_power(sin_half, degree - spin))
            .exp();
    let minus = minus_sign
        * (log_common + log_power(cos_half, degree - spin) + log_power(sin_half, degree + spin))
            .exp();
    (
        2.0 * (plus + minus) * (degree as f64 * phi).cos(),
        -2.0 * (plus - minus) * (degree as f64 * phi).sin(),
    )
}

#[test]
fn spin2_scalar_fast_synthesis_honors_full_and_mixed_polar_blocks() {
    let lmax = 511;
    let ntheta = lmax + 1;
    let m = 450;
    let plan = make_plan(2, lmax, lmax, ntheta, 2 * lmax + 3);
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut workspace, 2);
    let block_size = crate::legendre::LATITUDE_BLOCK_SIZE;
    let limits = &workspace.ring_order_limits[..ntheta];
    let excluded_block = limits
        .chunks(block_size)
        .position(|block| block.iter().all(|&limit| limit < m))
        .expect("the polar grid has a fully excluded block at m=450");
    let mixed_block = limits
        .chunks(block_size)
        .enumerate()
        .find_map(|(block_index, block)| {
            (block_index > 0
                && block.iter().any(|&limit| limit < m)
                && block.iter().any(|&limit| limit >= m))
            .then_some(block_index)
        })
        .expect("the cutoff crosses a 64-ring block at m=450");

    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b_alm = vec![Complex64::new(0.0, 0.0); e_alm.len()];
    for degree in m..=lmax {
        let index = plan.harmonics().index(degree, m).unwrap();
        e_alm[index] = Complex64::new((degree % 11 + 1) as f64, (degree % 7 + 1) as f64) * 1e-5;
        b_alm[index] = Complex64::new((degree % 13 + 1) as f64, -((degree % 5 + 1) as f64)) * 1e-5;
    }
    let spectrum_len = plan.fft.spectrum_len();
    let mut actual_q = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
    let mut actual_u = vec![Complex64::new(0.0, 0.0); actual_q.len()];
    plan.synthesize_spin2_scalar_order(
        m,
        &e_alm,
        &b_alm,
        spectrum_len,
        0,
        &mut actual_q,
        &mut actual_u,
        &workspace.theta_sin[..ntheta],
        &workspace.theta_cos[..ntheta],
        limits,
        pulp::Arch::new(),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();

    let all_active_limits = vec![lmax; ntheta];
    let mut reference_workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut reference_workspace, 2);
    let mut reference_q = vec![Complex64::new(0.0, 0.0); actual_q.len()];
    let mut reference_u = vec![Complex64::new(0.0, 0.0); actual_u.len()];
    plan.synthesize_spin2_scalar_order(
        m,
        &e_alm,
        &b_alm,
        spectrum_len,
        0,
        &mut reference_q,
        &mut reference_u,
        &reference_workspace.theta_sin[..ntheta],
        &reference_workspace.theta_cos[..ntheta],
        &all_active_limits,
        pulp::Arch::new(),
        &mut reference_workspace.recurrence_workers[0],
    )
    .unwrap();

    let excluded_start = excluded_block * block_size;
    let excluded_end = (excluded_start + block_size).min(ntheta);
    for ring in excluded_start..excluded_end {
        let index = ring * spectrum_len + m;
        assert_eq!(actual_q[index], Complex64::new(0.0, 0.0));
        assert_eq!(actual_u[index], Complex64::new(0.0, 0.0));
    }

    let mixed_start = mixed_block * block_size;
    let mixed_end = (mixed_start + block_size).min(ntheta);
    let mut active_output_seen = false;
    for (ring, &limit) in limits.iter().enumerate().take(mixed_end).skip(mixed_start) {
        let index = ring * spectrum_len + m;
        if limit < m {
            assert_eq!(actual_q[index], Complex64::new(0.0, 0.0));
            assert_eq!(actual_u[index], Complex64::new(0.0, 0.0));
        } else {
            active_output_seen |= actual_q[index] != Complex64::new(0.0, 0.0)
                || actual_u[index] != Complex64::new(0.0, 0.0);
            for (actual, reference) in [
                (actual_q[index], reference_q[index]),
                (actual_u[index], reference_u[index]),
            ] {
                assert!(
                    (actual - reference).norm() <= 3e-11,
                    "ring={ring}, actual={actual:?}, all-active={reference:?}"
                );
            }
        }
    }
    assert!(
        active_output_seen,
        "active lanes in a mixed block contribute"
    );

    let segment_spectrum_len = (mixed_end - mixed_start) * spectrum_len;
    let mut segment_q = vec![Complex64::new(0.0, 0.0); segment_spectrum_len];
    let mut segment_u = vec![Complex64::new(0.0, 0.0); segment_spectrum_len];
    plan.synthesize_spin2_scalar_order(
        m,
        &e_alm,
        &b_alm,
        spectrum_len,
        mixed_start,
        &mut segment_q,
        &mut segment_u,
        &workspace.theta_sin[mixed_start..mixed_end],
        &workspace.theta_cos[mixed_start..mixed_end],
        limits,
        pulp::Arch::new(),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();
    for (local_ring, ring) in (mixed_start..mixed_end).enumerate() {
        let actual_index = ring * spectrum_len + m;
        let segment_index = local_ring * spectrum_len + m;
        assert_eq!(segment_q[segment_index], actual_q[actual_index]);
        assert_eq!(segment_u[segment_index], actual_u[actual_index]);
    }
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "These decimal fixtures retain the printed DUCC reference values."
)]
fn spin_one_gl_matches_ducc_0410_and_weighted_analysis() {
    // Static outputs from ducc0==0.41.0, GL 3x7, phi0=0.37, one thread.
    let plan = make_plan(1, 2, 2, 3, 7);
    let (e, b) = coefficients(&plan);
    let mut workspace = plan.workspace().unwrap();
    let mut first = vec![0.0; plan.sample_count];
    let mut second = vec![0.0; plan.sample_count];
    plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
        .unwrap();
    for (index, expected) in [
        (0, -0.074_965_830_511_175_502),
        (2, 0.094_092_382_651_284_556),
        (10, -0.078_411_578_146_920_513),
        (20, 0.073_425_512_817_354_946),
    ] {
        assert_close(first[index], expected, 4e-14);
    }
    for (index, expected) in [
        (0, 0.053_094_402_343_952_324),
        (2, 0.231_182_369_405_343_7),
        (10, 0.107_080_931_286_266_61),
        (20, 0.051_955_952_175_169_004),
    ] {
        assert_close(second[index], expected, 4e-14);
    }

    let (input_first, input_second) = arbitrary_maps(&plan);
    let mut recovered_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut recovered_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    plan.analysis_spin_into(
        &input_first,
        &input_second,
        &mut recovered_e,
        &mut recovered_b,
        &mut workspace,
    )
    .unwrap();
    let expected_e = [
        Complex64::new(0.0, 0.0),
        Complex64::new(-2.719_595_782_603_398_5, 0.0),
        Complex64::new(0.296_440_629_674_238_01, 0.0),
        Complex64::new(0.116_869_914_995_230_15, -0.085_692_903_571_819_773),
        Complex64::new(-0.043_265_294_485_276_44, 0.153_766_887_626_409_58),
        Complex64::new(-0.070_050_301_995_330_216, 0.017_601_979_788_444_126),
    ];
    let expected_b = [
        Complex64::new(0.0, 0.0),
        Complex64::new(-3.005_376_860_452_039_4, 0.0),
        Complex64::new(-0.002_667_162_341_653_368_6, 0.0),
        Complex64::new(0.336_942_654_702_059_75, 0.250_544_257_552_348_33),
        Complex64::new(0.026_827_651_643_820_543, -0.065_628_053_641_931_43),
        Complex64::new(-0.185_782_400_932_178_02, -0.085_451_169_345_024_267),
    ];
    for ((&actual_e, &actual_b), (&expected_e, &expected_b)) in recovered_e
        .iter()
        .zip(&recovered_b)
        .zip(expected_e.iter().zip(&expected_b))
    {
        assert_complex_close(actual_e, expected_e, 8e-13);
        assert_complex_close(actual_b, expected_b, 8e-13);
    }
}

#[test]
fn spin1_scalar_fast_synthesis_matches_general_channel_cases() {
    let lmax = 511;
    let ntheta = 65;
    let nphi = 2 * lmax + 3;
    let plan = make_plan(1, lmax, lmax, ntheta, nphi);
    let (mut mixed_e, mut mixed_b) = coefficients(&plan);
    for value in mixed_e.iter_mut().chain(mixed_b.iter_mut()) {
        *value *= 1e-5;
    }
    let zero = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let cases = [
        ("E-only", mixed_e.as_slice(), Some(zero.as_slice())),
        ("B-only", zero.as_slice(), Some(mixed_b.as_slice())),
        ("mixed E/B", mixed_e.as_slice(), Some(mixed_b.as_slice())),
        ("GRAD_ONLY", mixed_e.as_slice(), None),
    ];
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut workspace, 1);

    for (name, e_alm, b_alm) in cases {
        workspace.spectra.fill(Complex64::new(0.0, 0.0));
        workspace.spin_spectra.fill(Complex64::new(0.0, 0.0));
        plan.synthesize_spin_ring_group(
            e_alm,
            b_alm,
            1,
            false,
            true,
            false,
            None,
            None,
            nphi,
            plan.fft.spectrum_len(),
            plan.grid.phi0(),
            0,
            ntheta,
            &mut workspace.spectra,
            &mut workspace.spin_spectra,
            &workspace.theta_sin[..ntheta],
            &workspace.theta_cos[..ntheta],
            &workspace.ring_order_limits[..ntheta],
            pulp::Arch::new(),
            &mut workspace.recurrence_workers[0],
        )
        .unwrap();
        let fast_first = workspace.spectra.clone();
        let fast_second = workspace.spin_spectra.clone();

        workspace.spectra.fill(Complex64::new(0.0, 0.0));
        workspace.spin_spectra.fill(Complex64::new(0.0, 0.0));
        plan.synthesize_spin_ring_group(
            e_alm,
            b_alm,
            1,
            false,
            false,
            false,
            None,
            None,
            nphi,
            plan.fft.spectrum_len(),
            plan.grid.phi0(),
            0,
            ntheta,
            &mut workspace.spectra,
            &mut workspace.spin_spectra,
            &workspace.theta_sin[..ntheta],
            &workspace.theta_cos[..ntheta],
            &workspace.ring_order_limits[..ntheta],
            pulp::Arch::new(),
            &mut workspace.recurrence_workers[0],
        )
        .unwrap();

        for (index, (fast, general)) in fast_first
            .iter()
            .zip(&workspace.spectra)
            .chain(fast_second.iter().zip(&workspace.spin_spectra))
            .enumerate()
        {
            let error = (*fast - *general).norm();
            assert!(
                error <= 3e-11,
                "{name} index={index} fast={fast:?} general={general:?} discrepancy={error:.3e}"
            );
        }
    }
}

#[test]
fn spin1_gl_table_synthesis_matches_recurrence_and_adjoint() {
    for (lmax, nphi) in [
        (72, 147),
        (127, 257),
        (128, 258),
        (144, 291),
        (159, 321),
        (175, 353),
        (191, 385),
        (255, 513),
    ] {
        let build_plan = || make_plan(1, lmax, lmax, lmax + 1, nphi);
        let plan = build_plan();
        let recurrence_plan = build_plan();
        assert!(
            recurrence_plan
                .spin1_scalar_legendre_table
                .set(None)
                .is_ok()
        );
        if lmax > SPIN1_TABLE_SYNTH_MAX_LMAX {
            let cached_table = crate::legendre::ScalarLegendreTable::build_if_beneficial(
                &plan.grid,
                &plan.harmonics,
                &plan.sectoral_norms,
            )
            .expect("T255 spin-1 table fits the analysis memory cap");
            assert!(
                plan.spin1_scalar_legendre_table
                    .set(Some(cached_table))
                    .is_ok()
            );
        }
        let (mut e, mut b) = coefficients(&plan);
        for value in e.iter_mut().chain(&mut b) {
            *value *= 1e-5;
        }
        let mut table_workspace = plan.workspace().unwrap();
        let mut recurrence_workspace = recurrence_plan.workspace().unwrap();
        let mut table_first = vec![0.0; plan.sample_count];
        let mut table_second = vec![0.0; plan.sample_count];
        let mut recurrence_first = vec![0.0; plan.sample_count];
        let mut recurrence_second = vec![0.0; plan.sample_count];
        #[cfg(feature = "parallel")]
        let single_thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        #[cfg(feature = "parallel")]
        single_thread_pool
            .install(|| {
                plan.synthesis_spin_into(
                    &e,
                    &b,
                    &mut table_first,
                    &mut table_second,
                    &mut table_workspace,
                )
            })
            .unwrap();
        #[cfg(not(feature = "parallel"))]
        plan.synthesis_spin_into(
            &e,
            &b,
            &mut table_first,
            &mut table_second,
            &mut table_workspace,
        )
        .unwrap();
        if lmax > SPIN1_TABLE_SYNTH_MAX_LMAX || spin1_synthesis_table_band_is_beneficial(lmax) {
            assert!(
                plan.spin1_scalar_legendre_table
                    .get()
                    .is_some_and(Option::is_some)
            );
        } else {
            assert!(plan.spin1_scalar_legendre_table.get().is_none());
        }
        #[cfg(feature = "parallel")]
        single_thread_pool
            .install(|| {
                recurrence_plan.synthesis_spin_into(
                    &e,
                    &b,
                    &mut recurrence_first,
                    &mut recurrence_second,
                    &mut recurrence_workspace,
                )
            })
            .unwrap();
        #[cfg(not(feature = "parallel"))]
        recurrence_plan
            .synthesis_spin_into(
                &e,
                &b,
                &mut recurrence_first,
                &mut recurrence_second,
                &mut recurrence_workspace,
            )
            .unwrap();

        for (index, (&table, &recurrence)) in table_first
            .iter()
            .chain(&table_second)
            .zip(recurrence_first.iter().chain(&recurrence_second))
            .enumerate()
        {
            let error = (table - recurrence).abs();
            assert!(
                error <= 3e-11,
                "T{lmax} spin-1 synthesis sample {index}: table={table:.17e}, recurrence={recurrence:.17e}, error={error:.3e}"
            );
        }

        let (map_first, map_second) = arbitrary_maps(&plan);
        let mut adjoint_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut adjoint_b = adjoint_e.clone();
        plan.adjoint_synthesis_spin_into(
            &map_first,
            &map_second,
            &mut adjoint_e,
            &mut adjoint_b,
            &mut table_workspace,
        )
        .unwrap();
        let lhs = spin_map_inner_product(&table_first, &table_second, &map_first, &map_second);
        let rhs = spin_harmonic_inner_product(plan.harmonics(), &e, &b, &adjoint_e, &adjoint_b);
        let error = (lhs - rhs).abs() / lhs.abs().max(rhs.abs()).max(1.0);
        assert!(
            error <= 2e-11,
            "T{lmax} spin-1 synthesis adjoint error={error:.3e}"
        );
    }
}

#[test]
fn spin1_scalar_transition_splits_polar_lanes_and_matches_general() {
    let lmax = 255;
    for grid_kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let ntheta = lmax + usize::from(grid_kind == GridKind::ClenshawCurtis) + 1;
        let nphi = 2 * lmax + 3;
        for (mode, plan_spin, derivative_norm) in [
            ("STANDARD", 1, false),
            ("GRAD_ONLY", 1, false),
            ("DERIV1", 0, true),
        ] {
            let plan = ShtPlan::new(
                GridSpec::new(grid_kind, ntheta, nphi, 0.37).unwrap(),
                HarmonicSpec::new(lmax, lmax, plan_spin).unwrap(),
            )
            .unwrap();
            let (mut e_alm, mut b_alm) = coefficients(&plan);
            for value in e_alm.iter_mut().chain(&mut b_alm) {
                *value *= 1e-5;
            }
            let mut workspace = plan.workspace().unwrap();
            plan.prepare_spin_latitudes(&mut workspace, 1);
            let transition_sine = 0.01;
            let first_block = &workspace.theta_sin[..crate::legendre::LATITUDE_BLOCK_SIZE];
            if grid_kind == GridKind::GaussLegendre || grid_kind == GridKind::Fejer1 {
                assert!(first_block[0] < transition_sine);
                assert!(first_block.iter().any(|&sine| sine >= transition_sine));
            }

            let spectrum_len = plan.fft.spectrum_len();
            let block_len = crate::legendre::LATITUDE_BLOCK_SIZE;
            let mut split_probe = None;
            if mode == "STANDARD" && matches!(grid_kind, GridKind::GaussLegendre | GridKind::Fejer1)
            {
                let m = 80;
                let scalar_lmax = lmax + 1;
                let recurrence = &mut workspace.recurrence_workers[0];
                <f64 as ShtScalar>::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
                assert!(<f64 as ShtScalar>::fill_paired_scalar_factors(
                    m,
                    scalar_lmax,
                    &recurrence.epsilon,
                    &mut recurrence.spin_alpha,
                    &mut recurrence.spin_coefficient_a,
                    &mut recurrence.spin_coefficient_b,
                ));
                let coefficient_start = plan.harmonics().index(m, m).unwrap();
                let coefficient_count = lmax - m + 1;
                crate::legendre::spin1_to_scalar(
                    m,
                    lmax,
                    &e_alm[coefficient_start..coefficient_start + coefficient_count],
                    Some(&b_alm[coefficient_start..coefficient_start + coefficient_count]),
                    &mut recurrence.fast_scalar_first,
                    &mut recurrence.fast_scalar_second,
                );

                let mut split_q = vec![Complex64::new(0.0, 0.0); block_len * spectrum_len];
                let mut split_u = vec![Complex64::new(0.0, 0.0); split_q.len()];
                let active_start = workspace.ring_order_limits[..block_len]
                    .iter()
                    .position(|&limit| m <= limit)
                    .expect("the block contains active rings");
                assert!(m <= workspace.ring_order_limits[0]);
                assert!(workspace.theta_sin[active_start] < transition_sine);
                assert!(
                    workspace.theta_sin[active_start..block_len]
                        .iter()
                        .any(|&sine| sine >= transition_sine)
                );
                let mut fused_spectra = vec![Complex64::new(0.0, 0.0); block_len * spectrum_len];
                assert!(
                    !<f64 as ShtScalar>::synthesize_paired_scalar_block_dispatched(
                        pulp::Arch::new(),
                        m,
                        scalar_lmax,
                        &workspace.theta_sin[..block_len],
                        &workspace.theta_cos[..block_len],
                        plan.sectoral_norms[m],
                        &recurrence.epsilon,
                        &recurrence.spin_alpha,
                        &recurrence.spin_coefficient_a,
                        &recurrence.spin_coefficient_b,
                        &recurrence.fast_scalar_first[m..=scalar_lmax],
                        &workspace.ring_order_limits[..block_len],
                        spectrum_len,
                        &mut fused_spectra,
                        &mut recurrence.previous[..block_len],
                        &mut recurrence.current[..block_len],
                        &mut recurrence.next[..block_len],
                        &mut recurrence.exponents[..block_len],
                    )
                    .unwrap()
                );
                let polar_count = try_synthesize_spin1_transition_safe_lanes(
                    pulp::Arch::new(),
                    m,
                    scalar_lmax,
                    spectrum_len,
                    plan.sectoral_norms[m],
                    &workspace.theta_sin[..block_len],
                    &workspace.theta_cos[..block_len],
                    &workspace.ring_order_limits[..block_len],
                    &mut split_q,
                    &mut split_u,
                    recurrence,
                )
                .unwrap()
                .expect("safe lanes complete the staged transition");
                assert!(
                    polar_count > 0,
                    "the split retains polar lanes for fallback"
                );
                assert!((0..block_len).any(|ring| split_q[ring * spectrum_len + m].norm() > 0.0));
                assert!((0..block_len).any(|ring| split_u[ring * spectrum_len + m].norm() > 0.0));
                let mut block_q = vec![Complex64::new(0.0, 0.0); block_len * spectrum_len];
                let mut block_u = vec![Complex64::new(0.0, 0.0); block_q.len()];
                plan.synthesize_spin1_scalar_block(
                    m,
                    scalar_lmax,
                    None,
                    spectrum_len,
                    0,
                    false,
                    &workspace.theta_sin[..block_len],
                    &workspace.theta_cos[..block_len],
                    &workspace.ring_order_limits[..block_len],
                    pulp::Arch::new(),
                    recurrence,
                    &mut block_q,
                    &mut block_u,
                )
                .unwrap();
                split_probe = Some((m, split_q, split_u, block_q, block_u));
            }

            let mut fast_q = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
            let mut fast_u = vec![Complex64::new(0.0, 0.0); fast_q.len()];
            plan.synthesize_spin_ring_group(
                &e_alm,
                (mode == "STANDARD").then_some(b_alm.as_slice()),
                1,
                derivative_norm,
                true,
                false,
                None,
                None,
                nphi,
                spectrum_len,
                plan.grid.phi0(),
                0,
                ntheta,
                &mut fast_q,
                &mut fast_u,
                &workspace.theta_sin[..ntheta],
                &workspace.theta_cos[..ntheta],
                &workspace.ring_order_limits[..ntheta],
                pulp::Arch::new(),
                &mut workspace.recurrence_workers[0],
            )
            .unwrap();

            let mut general_q = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
            let mut general_u = vec![Complex64::new(0.0, 0.0); general_q.len()];
            plan.synthesize_spin_ring_group(
                &e_alm,
                (mode == "STANDARD").then_some(b_alm.as_slice()),
                1,
                derivative_norm,
                false,
                false,
                None,
                None,
                nphi,
                spectrum_len,
                plan.grid.phi0(),
                0,
                ntheta,
                &mut general_q,
                &mut general_u,
                &workspace.theta_sin[..ntheta],
                &workspace.theta_cos[..ntheta],
                &workspace.ring_order_limits[..ntheta],
                pulp::Arch::new(),
                &mut workspace.recurrence_workers[0],
            )
            .unwrap();

            if let Some((m, split_q, split_u, block_q, block_u)) = split_probe {
                let phase = crate::legendre::longitude_phase::<f64>(m, plan.grid.phi0(), false);
                for ring in 0..block_len {
                    let index = ring * spectrum_len + m;
                    let expected_q = general_q[index] / phase;
                    let expected_u = general_u[index] / phase;
                    if workspace.theta_sin[ring] >= transition_sine
                        && m <= workspace.ring_order_limits[ring]
                    {
                        for (component, split, expected) in [
                            ("Q safe", split_q[index], expected_q),
                            ("U safe", split_u[index], expected_u),
                        ] {
                            let error = (split - expected).norm();
                            assert!(
                                error <= 3e-11 * expected.norm().max(1.0),
                                "grid={grid_kind:?} component={component} ring={ring} split={split:?} general={expected:?} error={error:.3e}"
                            );
                        }
                    }
                    for (component, split, expected) in [
                        ("Q full block", block_q[index], expected_q),
                        ("U full block", block_u[index], expected_u),
                    ] {
                        let error = (split - expected).norm();
                        assert!(
                            error <= 3e-11 * expected.norm().max(1.0),
                            "grid={grid_kind:?} component={component} ring={ring} split={split:?} general={expected:?} error={error:.3e}"
                        );
                    }
                }
            }

            for (component, fast, general) in
                [("Q", &fast_q, &general_q), ("U", &fast_u, &general_u)]
            {
                for (index, (&actual, &expected)) in fast.iter().zip(general).enumerate() {
                    let error = (actual - expected).norm();
                    assert!(
                        error <= 3e-11 * expected.norm().max(1.0),
                        "grid={grid_kind:?} mode={mode} component={component} index={index} fast={actual:?} general={expected:?} error={error:.3e}"
                    );
                }
            }
        }
    }
}

#[test]
fn spin1_scalar_transition_preserves_the_sine_cutoff_boundary() {
    let lmax = 511;
    let m = 64;
    let ntheta = lmax + 1;
    let nphi = 2 * lmax + 3;
    let plan = make_plan(1, lmax, m, ntheta, nphi);
    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b_alm = vec![Complex64::new(0.0, 0.0); e_alm.len()];
    for degree in m..=lmax {
        let index = plan.harmonics().index(degree, m).unwrap();
        e_alm[index] = Complex64::new(
            (degree % 13 + 1) as f64 * 1e-6,
            (degree % 7 + 1) as f64 * 2e-7,
        );
        b_alm[index] = Complex64::new(
            -((degree % 11 + 1) as f64) * 8e-7,
            (degree % 5 + 1) as f64 * 3e-7,
        );
    }

    let scalar_lmax = lmax + 1;
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut workspace, 1);
    let recurrence = &mut workspace.recurrence_workers[0];
    <f64 as ShtScalar>::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
    assert!(<f64 as ShtScalar>::fill_paired_scalar_factors(
        m,
        scalar_lmax,
        &recurrence.epsilon,
        &mut recurrence.spin_alpha,
        &mut recurrence.spin_coefficient_a,
        &mut recurrence.spin_coefficient_b,
    ));
    for degree in m..=scalar_lmax {
        recurrence.fast_scalar_first[degree] = Complex64::new(
            ((degree * 7 + 3) % 19 + 1) as f64 * 1e-6,
            ((degree * 11 + 5) % 23 + 1) as f64 * 1e-6,
        );
        recurrence.fast_scalar_second[degree] = Complex64::new(
            ((degree * 13 + 7) % 17 + 1) as f64 * 1e-6,
            -(((degree * 5 + 9) % 29 + 1) as f64) * 1e-6,
        );
    }

    let below_cutoff = [0.009_999_f64];
    let below_cosine = [(1.0 - below_cutoff[0] * below_cutoff[0]).sqrt()];
    let staged_below = <f64 as ShtScalar>::try_synthesize_paired_scalar_transition_dispatched(
        pulp::Arch::new(),
        m,
        scalar_lmax,
        &below_cutoff,
        &below_cosine,
        plan.sectoral_norms[m],
        &recurrence.epsilon,
        &recurrence.spin_alpha,
        &recurrence.spin_coefficient_a,
        &recurrence.spin_coefficient_b,
        &recurrence.fast_scalar_first[m..=scalar_lmax],
        &recurrence.fast_scalar_second[m..=scalar_lmax],
        &mut recurrence.previous,
        &mut recurrence.current,
        &mut recurrence.next,
        &mut recurrence.exponents,
        &mut recurrence.analysis_same_real,
        &mut recurrence.analysis_same_imag,
        &mut recurrence.analysis_opposite_real,
        &mut recurrence.analysis_opposite_imag,
    )
    .unwrap();
    assert!(!staged_below, "the staged path excludes sin(theta)<0.01");

    let sines: Vec<_> = (0..63)
        .map(|lane| 0.010_001 + lane as f64 * 0.0001)
        .collect();
    let cosines: Vec<_> = sines
        .iter()
        .map(|sine| (1.0 - sine * sine).sqrt())
        .collect();
    let staged_active = <f64 as ShtScalar>::try_synthesize_paired_scalar_transition_dispatched(
        pulp::Arch::new(),
        m,
        scalar_lmax,
        &sines,
        &cosines,
        plan.sectoral_norms[m],
        &recurrence.epsilon,
        &recurrence.spin_alpha,
        &recurrence.spin_coefficient_a,
        &recurrence.spin_coefficient_b,
        &recurrence.fast_scalar_first[m..=scalar_lmax],
        &recurrence.fast_scalar_second[m..=scalar_lmax],
        &mut recurrence.previous,
        &mut recurrence.current,
        &mut recurrence.next,
        &mut recurrence.exponents,
        &mut recurrence.analysis_same_real,
        &mut recurrence.analysis_same_imag,
        &mut recurrence.analysis_opposite_real,
        &mut recurrence.analysis_opposite_imag,
    )
    .unwrap();
    assert!(staged_active, "the 63-lane block enters the staged path");

    let below_cosines = below_cosine;
    for (theta_sin, theta_cos) in [(&below_cutoff[..], &below_cosines[..]), (&sines, &cosines)] {
        let limits = vec![lmax; theta_sin.len()];
        let spectrum_len = plan.fft.spectrum_len();
        let mut fast_q = vec![Complex64::new(0.0, 0.0); theta_sin.len() * spectrum_len];
        let mut fast_u = vec![Complex64::new(0.0, 0.0); fast_q.len()];
        plan.synthesize_spin_ring_group(
            &e_alm,
            Some(&b_alm),
            1,
            false,
            true,
            false,
            None,
            None,
            nphi,
            spectrum_len,
            plan.grid.phi0(),
            0,
            theta_sin.len(),
            &mut fast_q,
            &mut fast_u,
            theta_sin,
            theta_cos,
            &limits,
            pulp::Arch::new(),
            recurrence,
        )
        .unwrap();

        let mut general_q = vec![Complex64::new(0.0, 0.0); fast_q.len()];
        let mut general_u = vec![Complex64::new(0.0, 0.0); fast_q.len()];
        plan.synthesize_spin_ring_group(
            &e_alm,
            Some(&b_alm),
            1,
            false,
            false,
            false,
            None,
            None,
            nphi,
            spectrum_len,
            plan.grid.phi0(),
            0,
            theta_sin.len(),
            &mut general_q,
            &mut general_u,
            theta_sin,
            theta_cos,
            &limits,
            pulp::Arch::new(),
            recurrence,
        )
        .unwrap();

        for (actual, expected) in fast_q
            .iter()
            .zip(&general_q)
            .chain(fast_u.iter().zip(&general_u))
        {
            let error = (*actual - *expected).norm();
            assert!(
                error <= 3e-11 * expected.norm().max(1.0),
                "sin(theta) range={:?}..{:?}, actual={actual:?}, expected={expected:?}, error={error:.3e}",
                theta_sin.first(),
                theta_sin.last()
            );
        }
    }
}

#[test]
fn spin1_scalar_conversion_matches_general_on_all_grids() {
    for lmax in [14, 15, 31, 47, 63, 127, 190, 191, 208, 224] {
        for grid_kind in [
            GridKind::GaussLegendre,
            GridKind::ClenshawCurtis,
            GridKind::Fejer1,
        ] {
            let ntheta = lmax + usize::from(grid_kind == GridKind::ClenshawCurtis) + 1;
            let nphi = 2 * lmax + 3;
            let plan = ShtPlan::new(
                GridSpec::new(grid_kind, ntheta, nphi, 0.37).unwrap(),
                HarmonicSpec::new(lmax, lmax, 1).unwrap(),
            )
            .unwrap();
            let (mut e_alm, mut b_alm) = coefficients(&plan);
            for value in e_alm.iter_mut().chain(&mut b_alm) {
                *value *= 1e-5;
            }
            let zero = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let cases = [
                ("E-only", e_alm.as_slice(), Some(zero.as_slice())),
                ("B-only", zero.as_slice(), Some(b_alm.as_slice())),
                ("mixed E/B", e_alm.as_slice(), Some(b_alm.as_slice())),
                ("GRAD_ONLY", e_alm.as_slice(), None),
            ];
            let mut workspace = plan.workspace().unwrap();
            plan.prepare_spin_latitudes(&mut workspace, 1);
            let spectrum_len = plan.fft.spectrum_len();
            for (name, e_coefficients, b_coefficients) in cases {
                let mut fast_first = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
                let mut fast_second = fast_first.clone();
                plan.synthesize_spin_ring_group(
                    e_coefficients,
                    b_coefficients,
                    1,
                    false,
                    true,
                    false,
                    None,
                    None,
                    nphi,
                    spectrum_len,
                    plan.grid.phi0(),
                    0,
                    ntheta,
                    &mut fast_first,
                    &mut fast_second,
                    &workspace.theta_sin[..ntheta],
                    &workspace.theta_cos[..ntheta],
                    &workspace.ring_order_limits[..ntheta],
                    pulp::Arch::new(),
                    &mut workspace.recurrence_workers[0],
                )
                .unwrap();
                let mut general_first = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
                let mut general_second = general_first.clone();
                plan.synthesize_spin_ring_group(
                    e_coefficients,
                    b_coefficients,
                    1,
                    false,
                    false,
                    false,
                    None,
                    None,
                    nphi,
                    spectrum_len,
                    plan.grid.phi0(),
                    0,
                    ntheta,
                    &mut general_first,
                    &mut general_second,
                    &workspace.theta_sin[..ntheta],
                    &workspace.theta_cos[..ntheta],
                    &workspace.ring_order_limits[..ntheta],
                    pulp::Arch::new(),
                    &mut workspace.recurrence_workers[0],
                )
                .unwrap();
                for (index, (actual, expected)) in fast_first
                    .iter()
                    .zip(&general_first)
                    .chain(fast_second.iter().zip(&general_second))
                    .enumerate()
                {
                    let error = (*actual - *expected).norm();
                    assert!(
                        error <= 3e-11,
                        "{name}, {grid_kind:?}, lmax={lmax}, index={index}, error={error:.3e}"
                    );
                }
            }

            let mut standard_first = vec![0.0; plan.sample_count];
            let mut standard_second = vec![0.0; plan.sample_count];
            let mut gradient_first = vec![0.0; plan.sample_count];
            let mut gradient_second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(
                &e_alm,
                &zero,
                &mut standard_first,
                &mut standard_second,
                &mut workspace,
            )
            .unwrap();
            plan.synthesis_gradient_into(
                &e_alm,
                &mut gradient_first,
                &mut gradient_second,
                &mut workspace,
            )
            .unwrap();
            for (index, (standard, gradient)) in standard_first
                .iter()
                .chain(&standard_second)
                .zip(gradient_first.iter().chain(&gradient_second))
                .enumerate()
            {
                assert_close(*gradient, *standard, 5e-13);
                assert!(gradient.is_finite(), "non-finite output at index={index}");
            }

            if lmax == 63 && grid_kind == GridKind::GaussLegendre {
                let (map_first, map_second) = arbitrary_maps(&plan);
                let mut gradient_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
                plan.adjoint_synthesis_gradient_into(
                    &map_first,
                    &map_second,
                    &mut gradient_adjoint,
                    &mut workspace,
                )
                .unwrap();
                assert_close(
                    spin_map_inner_product(
                        &gradient_first,
                        &gradient_second,
                        &map_first,
                        &map_second,
                    ),
                    harmonic_inner_product(plan.harmonics(), &e_alm, &gradient_adjoint),
                    2e-10,
                );
            }
        }
    }
}

#[test]
fn spin1_scalar_synthesis_compacts_mixed_high_order_cutoff_blocks() {
    let lmax = 511;
    let m = 450;
    let ntheta = lmax + 1;
    let plan = make_plan(1, lmax, lmax, ntheta, 2 * lmax + 3);
    let spectrum_len = plan.fft.spectrum_len();
    let coefficient_start = plan.harmonics().index(m, m).unwrap();
    let coefficient_count = lmax - m + 1;
    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b_alm = vec![Complex64::new(0.0, 0.0); e_alm.len()];
    for degree in m..=lmax {
        let index = plan.harmonics().index(degree, m).unwrap();
        e_alm[index] = Complex64::new((degree % 11 + 1) as f64, (degree % 7 + 1) as f64) * 1e-5;
        b_alm[index] = Complex64::new((degree % 13 + 1) as f64, -((degree % 5 + 1) as f64)) * 1e-5;
    }

    let prepare_order = |workspace: &mut crate::workspace::ShtWorkspace| {
        let recurrence = &mut workspace.recurrence_workers[0];
        let scalar_lmax = lmax + 1;
        spin1_to_scalar(
            m,
            lmax,
            &e_alm[coefficient_start..coefficient_start + coefficient_count],
            Some(&b_alm[coefficient_start..coefficient_start + coefficient_count]),
            &mut recurrence.fast_scalar_first[..scalar_lmax + 1],
            &mut recurrence.fast_scalar_second[..scalar_lmax + 1],
        );
        <f64 as ShtScalar>::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
        assert!(<f64 as ShtScalar>::fill_paired_scalar_factors(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        ));
    };

    let mut actual_workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut actual_workspace, 1);
    let limits = actual_workspace.ring_order_limits[..ntheta].to_vec();
    assert!(
        limits
            .chunks(crate::legendre::LATITUDE_BLOCK_SIZE)
            .any(|block| {
                block.iter().any(|&limit| limit < m) && block.iter().any(|&limit| limit >= m)
            })
    );
    assert!(
        limits
            .chunks(crate::legendre::LATITUDE_BLOCK_SIZE)
            .any(|block| block.iter().all(|&limit| limit < m))
    );

    let mut actual_q = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
    let mut actual_u = vec![Complex64::new(0.0, 0.0); actual_q.len()];
    prepare_order(&mut actual_workspace);
    for block_start in (0..ntheta).step_by(crate::legendre::LATITUDE_BLOCK_SIZE) {
        let block_end = (block_start + crate::legendre::LATITUDE_BLOCK_SIZE).min(ntheta);
        let spectrum_start = block_start * spectrum_len;
        let spectrum_end = block_end * spectrum_len;
        plan.synthesize_spin1_scalar_block(
            m,
            lmax + 1,
            None,
            spectrum_len,
            block_start,
            false,
            &actual_workspace.theta_sin[block_start..block_end],
            &actual_workspace.theta_cos[block_start..block_end],
            &limits[block_start..block_end],
            pulp::Arch::new(),
            &mut actual_workspace.recurrence_workers[0],
            &mut actual_q[spectrum_start..spectrum_end],
            &mut actual_u[spectrum_start..spectrum_end],
        )
        .unwrap();
    }

    let mut reference_workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut reference_workspace, 1);
    prepare_order(&mut reference_workspace);
    let all_active_limits = vec![lmax; ntheta];
    let mut reference_q = vec![Complex64::new(0.0, 0.0); actual_q.len()];
    let mut reference_u = vec![Complex64::new(0.0, 0.0); actual_u.len()];
    for block_start in (0..ntheta).step_by(crate::legendre::LATITUDE_BLOCK_SIZE) {
        let block_end = (block_start + crate::legendre::LATITUDE_BLOCK_SIZE).min(ntheta);
        let spectrum_start = block_start * spectrum_len;
        let spectrum_end = block_end * spectrum_len;
        plan.synthesize_spin1_scalar_block(
            m,
            lmax + 1,
            None,
            spectrum_len,
            block_start,
            false,
            &reference_workspace.theta_sin[block_start..block_end],
            &reference_workspace.theta_cos[block_start..block_end],
            &all_active_limits[block_start..block_end],
            pulp::Arch::new(),
            &mut reference_workspace.recurrence_workers[0],
            &mut reference_q[spectrum_start..spectrum_end],
            &mut reference_u[spectrum_start..spectrum_end],
        )
        .unwrap();
    }

    for (ring, &limit) in limits.iter().enumerate() {
        let index = ring * spectrum_len + m;
        if m > limit {
            assert_eq!(actual_q[index], Complex64::new(0.0, 0.0));
            assert_eq!(actual_u[index], Complex64::new(0.0, 0.0));
        } else {
            let q_error = (actual_q[index] - reference_q[index]).norm();
            let u_error = (actual_u[index] - reference_u[index]).norm();
            assert!(q_error <= 3e-11, "ring={ring} Q error={q_error:.3e}");
            assert!(u_error <= 3e-11, "ring={ring} U error={u_error:.3e}");
        }
    }

    let boundary_ring = (0..ntheta)
        .filter(|&ring| limits[ring] >= m)
        .min_by_key(|&ring| limits[ring] - m)
        .expect("at least one ring is active at the selected order");
    let mut boundary_q = vec![Complex64::new(0.0, 0.0); spectrum_len];
    let mut boundary_u = vec![Complex64::new(0.0, 0.0); spectrum_len];
    plan.synthesize_spin1_scalar_block(
        m,
        lmax + 1,
        None,
        spectrum_len,
        boundary_ring,
        false,
        &[reference_workspace.theta_sin[boundary_ring]],
        &[reference_workspace.theta_cos[boundary_ring]],
        &[lmax],
        pulp::Arch::new(),
        &mut reference_workspace.recurrence_workers[0],
        &mut boundary_q,
        &mut boundary_u,
    )
    .unwrap();
    for (name, actual, expected) in [
        (
            "Q",
            actual_q[boundary_ring * spectrum_len + m],
            boundary_q[m],
        ),
        (
            "U",
            actual_u[boundary_ring * spectrum_len + m],
            boundary_u[m],
        ),
    ] {
        let scale = expected.norm();
        let error = (actual - expected).norm();
        if scale == 0.0 {
            assert_eq!(actual, expected, "boundary ring={boundary_ring} {name}");
        } else {
            assert!(
                error <= 1e-11 * scale,
                "boundary ring={boundary_ring} {name} relative error={:.3e}",
                error / scale
            );
        }
    }
}

#[test]
fn spin1_synthesis_skips_fully_excluded_high_order_segments_for_all_modes() {
    let lmax = 511;
    let m = 500;
    let nphi = 2 * lmax + 3;

    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let ntheta = lmax + usize::from(kind == GridKind::ClenshawCurtis) + 1;
        for mode in ["STANDARD", "GRAD_ONLY", "DERIV1"] {
            let plan_spin = usize::from(mode != "DERIV1");
            let plan = ShtPlan::new(
                GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
                HarmonicSpec::new(lmax, m, plan_spin).unwrap(),
            )
            .unwrap();
            let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let index = plan.harmonics().index(m, m).unwrap();
            e_alm[index] = Complex64::new(1.0, 0.2);
            let b_alm = vec![Complex64::new(0.0, 0.0); e_alm.len()];

            let mut workspace = plan.workspace().unwrap();
            let mut first = vec![0.0; plan.sample_count];
            let mut second = vec![0.0; plan.sample_count];
            match mode {
                "STANDARD" => plan
                    .synthesis_spin_into(&e_alm, &b_alm, &mut first, &mut second, &mut workspace)
                    .unwrap(),
                "GRAD_ONLY" => plan
                    .synthesis_gradient_into(&e_alm, &mut first, &mut second, &mut workspace)
                    .unwrap(),
                "DERIV1" => plan
                    .synthesis_deriv1_into(&e_alm, &mut first, &mut second, &mut workspace)
                    .unwrap(),
                _ => unreachable!(),
            }
            let first_active_ring = workspace
                .ring_order_limits
                .iter()
                .position(|&limit| limit >= m)
                .expect("sectoral order must be active on equatorial rings");
            assert!(first_active_ring > 0, "{kind:?} must exclude polar rings");
            assert!(
                workspace.ring_order_limits[..first_active_ring]
                    .iter()
                    .all(|&limit| limit < m),
                "{kind:?} {mode}: rings before the active transition should exclude m={m}"
            );
            assert!(
                first[..first_active_ring * nphi]
                    .iter()
                    .chain(&second[..first_active_ring * nphi])
                    .all(|&value| value == 0.0),
                "{kind:?} {mode}: excluded rings must remain zero"
            );
            assert!(
                first[first_active_ring * nphi..]
                    .iter()
                    .chain(&second[first_active_ring * nphi..])
                    .any(|&value| value != 0.0),
                "{kind:?} {mode}: active rings must retain the sectoral field"
            );
        }
    }
}

#[test]
fn spin1_synthesis_skips_high_orders_in_a_clenshaw_curtis_pole_segment() {
    let lmax = 511;
    let m = 200;
    let ntheta = lmax + 2;
    let nphi = 2 * lmax + 3;
    let plan = make_cc_plan(1, lmax, m, ntheta, nphi);
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut workspace, 1);
    assert!(workspace.theta_sin[0] < SPIN1_FAST_MIN_SIN_THETA);
    assert!(workspace.ring_order_limits[0] < m);

    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b_alm = vec![Complex64::new(0.0, 0.0); e_alm.len()];
    for degree in m..=lmax {
        let index = plan.harmonics().index(degree, m).unwrap();
        e_alm[index] = Complex64::new(0.03 * degree as f64, -0.02);
        b_alm[index] = Complex64::new(-0.01, 0.017 * degree as f64);
    }
    let spectrum_len = plan.fft.spectrum_len();
    let mut q_spectra = vec![Complex64::new(0.0, 0.0); spectrum_len];
    let mut u_spectra = vec![Complex64::new(0.0, 0.0); spectrum_len];
    let q_sentinel = Complex64::new(0.5, -0.25);
    let u_sentinel = Complex64::new(-0.125, 0.75);
    q_spectra[m] = q_sentinel;
    u_spectra[m] = u_sentinel;

    plan.synthesize_spin_ring_group_mode(
        &e_alm,
        Some(&b_alm),
        1,
        false,
        false,
        false,
        None,
        None,
        nphi,
        spectrum_len,
        plan.grid.phi0(),
        0,
        1,
        &mut q_spectra,
        &mut u_spectra,
        &workspace.theta_sin[..1],
        &workspace.theta_cos[..1],
        &workspace.ring_order_limits,
        pulp::Arch::new(),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();

    // The sentinel distinguishes the new segment skip from the older lane
    // cutoff: the latter reached the final phase/scaling step for this row.
    assert_eq!(q_spectra[m], q_sentinel);
    assert_eq!(u_spectra[m], u_sentinel);
}

#[test]
fn spin1_paired_coefficient_overflow_falls_back_without_partial_commit() {
    let (lmax, m, nphi) = (15, 1, 3);
    let plan = make_plan(1, lmax, m, lmax + 1, nphi);
    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b_alm = e_alm.clone();
    let magnitude = 1.40e308;
    e_alm[plan.harmonics().index(1, m).unwrap()] = Complex64::new(0.0, -magnitude);
    e_alm[plan.harmonics().index(3, m).unwrap()] = Complex64::new(0.0, magnitude);
    b_alm[plan.harmonics().index(2, m).unwrap()] = Complex64::new(magnitude, 0.0);

    let mut workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut workspace, 1);
    let spectrum_len = plan.fft.spectrum_len();
    let coefficient_start = plan.harmonics().index(m, m).unwrap();
    let coefficient_count = lmax - m + 1;
    let recurrence = &mut workspace.recurrence_workers[0];
    spin1_to_scalar(
        m,
        lmax,
        &e_alm[coefficient_start..coefficient_start + coefficient_count],
        Some(&b_alm[coefficient_start..coefficient_start + coefficient_count]),
        &mut recurrence.fast_scalar_first[..lmax + 2],
        &mut recurrence.fast_scalar_second[..lmax + 2],
    );
    assert!(
        recurrence.fast_scalar_first[..lmax + 2]
            .iter()
            .chain(&recurrence.fast_scalar_second[..lmax + 2])
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    let scalar_lmax = lmax + 1;
    <f64 as crate::legendre::ShtScalar>::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
    <f64 as crate::legendre::ShtScalar>::fill_paired_scalar_factors(
        m,
        scalar_lmax,
        &recurrence.epsilon,
        &mut recurrence.spin_alpha,
        &mut recurrence.spin_coefficient_a,
        &mut recurrence.spin_coefficient_b,
    );
    let pair_count = (scalar_lmax - m + 2) / 2;
    let prepared_first =
        <f64 as crate::legendre::ShtScalar>::prepare_paired_scalar_synthesis_coefficients(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &recurrence.spin_alpha,
            &recurrence.fast_scalar_first[m..=scalar_lmax],
            &mut recurrence.fast_spin_first[..pair_count],
            &mut recurrence.fast_spin_second[..pair_count],
        );
    let prepared_second =
        <f64 as crate::legendre::ShtScalar>::prepare_paired_scalar_synthesis_coefficients(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &recurrence.spin_alpha,
            &recurrence.fast_scalar_second[m..=scalar_lmax],
            &mut recurrence.fast_spin_pair_first[..pair_count],
            &mut recurrence.fast_spin_pair_second[..pair_count],
        );
    assert!(!prepared_first && prepared_second);

    // A synthetic near-pole lane selects separate channel preparation while
    // keeping m=1 active, so a failed channel must leave both spectra untouched.
    let theta_sin = [0.005_f64];
    let theta_cos = [(1.0_f64 - theta_sin[0].powi(2)).sqrt()];
    let ring_order_limits = [m];
    let mut fast_first = vec![Complex64::new(0.0, 0.0); spectrum_len];
    let mut fast_second = fast_first.clone();
    plan.synthesize_spin_ring_group_mode(
        &e_alm,
        Some(&b_alm),
        1,
        false,
        true,
        false,
        None,
        None,
        nphi,
        spectrum_len,
        plan.grid.phi0(),
        0,
        1,
        &mut fast_first,
        &mut fast_second,
        &theta_sin,
        &theta_cos,
        &ring_order_limits,
        pulp::Arch::new(),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();

    let mut general_first = vec![Complex64::new(0.0, 0.0); spectrum_len];
    let mut general_second = general_first.clone();
    plan.synthesize_spin_ring_group_mode(
        &e_alm,
        Some(&b_alm),
        1,
        false,
        false,
        false,
        None,
        None,
        nphi,
        spectrum_len,
        plan.grid.phi0(),
        0,
        1,
        &mut general_first,
        &mut general_second,
        &theta_sin,
        &theta_cos,
        &ring_order_limits,
        pulp::Arch::new(),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();

    let compare = |fast: f64, general: f64| {
        if general.is_finite() {
            assert!(fast.is_finite());
            let scale = general.abs().max(1.0);
            assert!((fast / scale - general / scale).abs() < 2e-12);
        } else if general.is_infinite() {
            assert!(fast.is_infinite());
            assert_eq!(fast.is_sign_positive(), general.is_sign_positive());
        } else {
            assert!(fast.is_nan());
        }
    };
    for (fast, general) in fast_first
        .iter()
        .chain(&fast_second)
        .zip(general_first.iter().chain(&general_second))
    {
        compare(fast.re, general.re);
        compare(fast.im, general.im);
    }
}

#[test]
fn spin1_nonfinite_scalar_conversion_falls_back_to_general_recurrence() {
    let magnitude = 1.44e308;
    let plan = make_plan(1, 15, 1, 16, 3);
    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b_alm = e_alm.clone();
    e_alm[plan.harmonics().index(2, 1).unwrap()] = Complex64::new(magnitude, 0.0);
    b_alm[plan.harmonics().index(1, 1).unwrap()] = Complex64::new(0.0, -magnitude);

    let coefficient_start = plan.harmonics().index(1, 1).unwrap();
    let coefficient_count = plan.harmonics().lmax() - 1 + 1;
    let mut scalar_first = vec![Complex64::new(0.0, 0.0); plan.harmonics().lmax() + 2];
    let mut scalar_second = scalar_first.clone();
    spin1_to_scalar(
        1,
        plan.harmonics().lmax(),
        &e_alm[coefficient_start..coefficient_start + coefficient_count],
        Some(&b_alm[coefficient_start..coefficient_start + coefficient_count]),
        &mut scalar_first,
        &mut scalar_second,
    );
    assert!(!scalar_first[1].re.is_finite());

    let mut workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut workspace, 1);
    let spectrum_len = plan.fft.spectrum_len();
    let mut fast_first = vec![Complex64::new(0.0, 0.0); 16 * spectrum_len];
    let mut fast_second = fast_first.clone();
    plan.synthesize_spin_ring_group_mode(
        &e_alm,
        Some(&b_alm),
        1,
        false,
        true,
        false,
        None,
        None,
        3,
        spectrum_len,
        plan.grid.phi0(),
        0,
        16,
        &mut fast_first,
        &mut fast_second,
        &workspace.theta_sin[..16],
        &workspace.theta_cos[..16],
        &workspace.ring_order_limits,
        pulp::Arch::new(),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();

    let mut general_first = vec![Complex64::new(0.0, 0.0); 16 * spectrum_len];
    let mut general_second = general_first.clone();
    plan.synthesize_spin_ring_group_mode(
        &e_alm,
        Some(&b_alm),
        1,
        false,
        false,
        false,
        None,
        None,
        3,
        spectrum_len,
        plan.grid.phi0(),
        0,
        16,
        &mut general_first,
        &mut general_second,
        &workspace.theta_sin[..16],
        &workspace.theta_cos[..16],
        &workspace.ring_order_limits,
        pulp::Arch::new(),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();
    for (fast, general) in fast_first
        .iter()
        .chain(&fast_second)
        .zip(general_first.iter().chain(&general_second))
    {
        let compare = |actual: f64, expected: f64| {
            if expected.is_finite() {
                assert!(actual.is_finite());
                assert!((actual / magnitude - expected / magnitude).abs() < 2e-12);
            } else if expected.is_infinite() {
                assert!(actual.is_infinite());
                assert_eq!(actual.is_sign_positive(), expected.is_sign_positive());
            } else {
                assert!(actual.is_nan());
            }
        };
        compare(fast.re, general.re);
        compare(fast.im, general.im);
    }
}

#[test]
fn spin1_public_synthesis_keeps_large_finite_coefficients_finite() {
    let magnitude = 1.0e307;
    let plan = make_plan(1, 15, 1, 16, 3);
    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b_alm = e_alm.clone();
    e_alm[plan.harmonics().index(2, 1).unwrap()] = Complex64::new(magnitude, 0.0);
    b_alm[plan.harmonics().index(1, 1).unwrap()] = Complex64::new(0.0, -magnitude);
    let mut first = vec![0.0; plan.sample_count];
    let mut second = first.clone();
    let mut workspace = plan.workspace().unwrap();
    plan.synthesis_spin_into(&e_alm, &b_alm, &mut first, &mut second, &mut workspace)
        .unwrap();
    assert!(first.iter().chain(&second).all(|value| value.is_finite()));
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_order_major_synthesis_reuses_workspace_across_changed_coefficients() {
    let lmax = 720;
    let mmax = 180;
    let nphi = 378;
    let single_thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let ntheta = lmax + usize::from(kind == GridKind::ClenshawCurtis) + 1;
        let plan = ShtPlan::new(
            GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::new(lmax, mmax, 1).unwrap(),
        )
        .unwrap();
        let mut single_workspace = single_thread_pool.install(|| plan.workspace()).unwrap();
        let mut parallel_workspace = eight_thread_pool.install(|| plan.workspace()).unwrap();
        assert!(parallel_workspace.spin_order_output_scratch.is_empty());

        let (regular_e, regular_b) = coefficients(&plan);
        let mut sparse_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut sparse_b = sparse_e.clone();
        sparse_e[plan.harmonics.index(lmax, mmax).unwrap()] = Complex64::new(0.75, -0.25);
        sparse_b[plan.harmonics.index(lmax - 1, mmax).unwrap()] = Complex64::new(-0.5, 0.125);

        let mut large_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut large_b = large_e.clone();
        large_e[plan.harmonics.index(2, 1).unwrap()] = Complex64::new(1.40e308, 0.0);
        large_e[plan.harmonics.index(3, 1).unwrap()] = Complex64::new(-1.40e308, 0.0);
        large_b[plan.harmonics.index(1, 1).unwrap()] = Complex64::new(0.0, -1.40e308);

        let mut unsafe_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut unsafe_b = unsafe_e.clone();
        unsafe_e[plan.harmonics.index(2, 1).unwrap()] = Complex64::new(1.44e308, 0.0);
        unsafe_b[plan.harmonics.index(1, 1).unwrap()] = Complex64::new(0.0, -1.44e308);

        let mut subnormal_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let subnormal_b = subnormal_e.clone();
        subnormal_e[plan.harmonics.index(2, 1).unwrap()] =
            Complex64::new(f64::from_bits(1), -f64::from_bits(1));

        let zero_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let zero_b = zero_e.clone();
        let coefficient_cases = [
            (&regular_e, &regular_b, "regular"),
            (&sparse_e, &sparse_b, "sectoral tail"),
            (&large_e, &large_b, "extreme finite coefficients"),
            (&unsafe_e, &unsafe_b, "unsafe converted coefficients"),
            (&subnormal_e, &subnormal_b, "subnormal"),
            (&zero_e, &zero_b, "zero after prior output"),
        ];

        let mut single_first = vec![0.0; plan.sample_count];
        let mut single_second = vec![0.0; plan.sample_count];
        let mut parallel_first = vec![0.0; plan.sample_count];
        let mut parallel_second = vec![0.0; plan.sample_count];
        for (e, b, label) in coefficient_cases {
            single_first.fill(f64::NAN);
            single_second.fill(f64::NAN);
            parallel_first.fill(f64::NAN);
            parallel_second.fill(f64::NAN);
            single_thread_pool
                .install(|| {
                    plan.synthesis_spin_into(
                        e,
                        b,
                        &mut single_first,
                        &mut single_second,
                        &mut single_workspace,
                    )
                })
                .unwrap();
            eight_thread_pool
                .install(|| {
                    plan.synthesis_spin_into(
                        e,
                        b,
                        &mut parallel_first,
                        &mut parallel_second,
                        &mut parallel_workspace,
                    )
                })
                .unwrap();
            assert_eq!(parallel_workspace.spin_order_output_scratch.len(), 8);
            let preserves_exceptional_outputs = matches!(
                label,
                "extreme finite coefficients" | "unsafe converted coefficients"
            );
            if label == "unsafe converted coefficients" {
                assert_eq!(
                    parallel_workspace.spin1_scalar_coefficient_finite_orders[1],
                    0
                );
            }
            if !preserves_exceptional_outputs {
                assert!(
                    parallel_first
                        .iter()
                        .chain(&parallel_second)
                        .all(|value| value.is_finite()),
                    "{label}"
                );
            }
            let scale = single_first
                .iter()
                .chain(&single_second)
                .filter(|value| value.is_finite())
                .map(|value| value.abs())
                .fold(1.0_f64, f64::max);
            for (&single, &parallel) in single_first
                .iter()
                .zip(&parallel_first)
                .chain(single_second.iter().zip(&parallel_second))
            {
                if single.is_finite() {
                    assert!(parallel.is_finite(), "{label} changed a finite q1 output");
                    assert!(
                        (single / scale - parallel / scale).abs() <= 2e-11,
                        "{label} changed a finite q8 output"
                    );
                } else if single.is_infinite() {
                    assert!(
                        parallel.is_infinite(),
                        "{label} changed infinity to finite/NaN"
                    );
                    assert_eq!(single.is_sign_positive(), parallel.is_sign_positive());
                } else {
                    assert!(parallel.is_nan(), "{label} changed NaN to finite/infinity");
                }
            }
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn cc_t720_full_order_prepared_component_route_matches_single_thread() {
    let plan = make_cc_plan(1, 720, 720, 722, 1458);
    let single_thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut single_workspace = single_thread_pool.install(|| plan.workspace()).unwrap();
    let mut parallel_workspace = eight_thread_pool.install(|| plan.workspace()).unwrap();
    let (e_alm, b_alm) = coefficients(&plan);
    let mut single_first = vec![0.0; plan.sample_count];
    let mut single_second = vec![0.0; plan.sample_count];
    let mut parallel_first = vec![0.0; plan.sample_count];
    let mut parallel_second = vec![0.0; plan.sample_count];

    single_thread_pool
        .install(|| {
            plan.synthesis_spin_into(
                &e_alm,
                &b_alm,
                &mut single_first,
                &mut single_second,
                &mut single_workspace,
            )
        })
        .unwrap();
    SPIN1_PREPARED_COMPONENT_ORDER_SUCCESSES.store(0, Ordering::Relaxed);
    eight_thread_pool
        .install(|| {
            plan.synthesis_spin_into(
                &e_alm,
                &b_alm,
                &mut parallel_first,
                &mut parallel_second,
                &mut parallel_workspace,
            )
        })
        .unwrap();
    assert!(
        SPIN1_PREPARED_COMPONENT_ORDER_SUCCESSES.load(Ordering::Relaxed) > 0,
        "prepared component route did not run"
    );

    let scale = single_first
        .iter()
        .chain(&single_second)
        .map(|value| value.abs())
        .fold(1.0_f64, f64::max);
    for (&single, &parallel) in single_first
        .iter()
        .zip(&parallel_first)
        .chain(single_second.iter().zip(&parallel_second))
    {
        assert!(parallel.is_finite());
        assert!((single / scale - parallel / scale).abs() <= 2.0e-11);
    }
}

#[test]
fn spin1_scalar_fast_analysis_honors_the_polar_order_cutoff() {
    let lmax = 511;
    let ntheta = lmax + 1;
    let nphi = 2 * lmax + 3;
    let m = 450;
    let plan = make_plan(1, lmax, lmax, ntheta, nphi);
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut workspace, 1);
    let ring = (0..ntheta)
        .find(|&ring| {
            workspace.theta_sin[ring] > 0.4
                && workspace.theta_sin[ring] < 0.6
                && workspace.ring_order_limits[ring] < m
        })
        .expect("a mid-latitude ring is below the high-order cutoff");
    let spectrum_len = plan.fft.spectrum_len();
    let mut q_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
    let mut u_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
    q_spectra[ring * spectrum_len + m] = Complex64::new(1.0, 0.25);
    let coefficient_count = lmax - m + 1;
    let mut e_coefficients = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut b_coefficients = vec![Complex64::new(0.0, 0.0); coefficient_count];

    plan.analyze_spin_scalar_fast_order(
        &q_spectra,
        &u_spectra,
        spectrum_len,
        nphi as f64,
        longitude_phase::<f64>(m, plan.grid.phi0(), true),
        Some(plan.grid.ring_weights()),
        true,
        m,
        0,
        ntheta,
        &workspace.theta_sin,
        &workspace.theta_cos,
        &workspace.ring_order_limits,
        pulp::Arch::new(),
        &mut e_coefficients,
        Some(&mut b_coefficients),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();
    assert!(
        e_coefficients
            .iter()
            .chain(&b_coefficients)
            .all(|value| *value == Complex64::new(0.0, 0.0))
    );

    let all_active_limits = vec![lmax; ntheta];
    e_coefficients.fill(Complex64::new(0.0, 0.0));
    b_coefficients.fill(Complex64::new(0.0, 0.0));
    plan.analyze_spin_scalar_fast_order(
        &q_spectra,
        &u_spectra,
        spectrum_len,
        nphi as f64,
        longitude_phase::<f64>(m, plan.grid.phi0(), true),
        Some(plan.grid.ring_weights()),
        true,
        m,
        0,
        ntheta,
        &workspace.theta_sin,
        &workspace.theta_cos,
        &all_active_limits,
        pulp::Arch::new(),
        &mut e_coefficients,
        Some(&mut b_coefficients),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();
    assert!(
        e_coefficients
            .iter()
            .chain(&b_coefficients)
            .any(|value| *value != Complex64::new(0.0, 0.0))
    );

    assert!(
        workspace.ring_order_limits[..ntheta]
            .chunks(crate::legendre::LATITUDE_BLOCK_SIZE)
            .any(|block| block.iter().any(|&limit| limit < m)
                && block.iter().any(|&limit| limit >= m))
    );
    let mut reference_q = vec![Complex64::new(0.0, 0.0); q_spectra.len()];
    let mut reference_u = vec![Complex64::new(0.0, 0.0); u_spectra.len()];
    for ring in 0..ntheta {
        let amplitude = ((ring * 13 % 17) as f64 - 8.0) * 1e-3;
        let index = ring * spectrum_len + m;
        q_spectra[index] = Complex64::new(amplitude, amplitude * 0.25);
        u_spectra[index] = Complex64::new(-amplitude * 0.7, amplitude * 0.4);
        if m <= workspace.ring_order_limits[ring] {
            reference_q[index] = q_spectra[index];
            reference_u[index] = u_spectra[index];
        }
    }

    e_coefficients.fill(Complex64::new(0.0, 0.0));
    b_coefficients.fill(Complex64::new(0.0, 0.0));
    plan.analyze_spin_scalar_fast_order(
        &q_spectra,
        &u_spectra,
        spectrum_len,
        nphi as f64,
        longitude_phase::<f64>(m, plan.grid.phi0(), true),
        Some(plan.grid.ring_weights()),
        true,
        m,
        0,
        ntheta,
        &workspace.theta_sin,
        &workspace.theta_cos,
        &workspace.ring_order_limits,
        pulp::Arch::new(),
        &mut e_coefficients,
        Some(&mut b_coefficients),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();
    let compacted_e = e_coefficients.clone();
    let compacted_b = b_coefficients.clone();

    e_coefficients.fill(Complex64::new(0.0, 0.0));
    b_coefficients.fill(Complex64::new(0.0, 0.0));
    plan.analyze_spin_scalar_fast_order(
        &reference_q,
        &reference_u,
        spectrum_len,
        nphi as f64,
        longitude_phase::<f64>(m, plan.grid.phi0(), true),
        Some(plan.grid.ring_weights()),
        true,
        m,
        0,
        ntheta,
        &workspace.theta_sin,
        &workspace.theta_cos,
        &all_active_limits,
        pulp::Arch::new(),
        &mut e_coefficients,
        Some(&mut b_coefficients),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();
    for (channel, actual, expected) in [
        ("E", &compacted_e, &e_coefficients),
        ("B", &compacted_b, &b_coefficients),
    ] {
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            let error = (actual - expected).norm();
            assert!(
                error <= 3e-10 * expected.norm().max(1.0),
                "mixed high-order cutoff {channel}[{index}] error={error:.3e}"
            );
        }
    }
}

#[test]
fn spin1_gl_paired_analysis_matches_full_latitude_recurrence() {
    let lmax = 511;
    // The odd latitude count includes one equatorial singleton.
    let ntheta = lmax + 2;
    let nphi = 2 * lmax + 3;
    let plan = make_plan(1, lmax, lmax, ntheta, nphi);
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut workspace, 1);
    let spectrum_len = plan.fft.spectrum_len();
    let ring_scales: Vec<_> = plan
        .grid
        .ring_weights()
        .iter()
        .zip(&workspace.theta_sin)
        .map(|(&weight, &sine)| (weight / sine) / nphi as f64)
        .collect();

    for m in [0, 17, 450] {
        let mut ring_order_limits = workspace.ring_order_limits.clone();
        if m == 17 {
            // An asymmetric cutoff must retain whichever reflected lane is active.
            ring_order_limits[ntheta / 4] = m - 1;
            ring_order_limits[ntheta - 1 - ntheta / 3] = m - 1;
        }
        let mut q_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
        let mut u_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
        for ring in 0..ntheta {
            let reflected = ntheta - 1 - ring;
            // Similar reflected values exercise the subtraction used for odd
            // scalar parity, while the small offset remains representable.
            let offset = (ring as f64 - reflected as f64) * 1.0e-8;
            q_spectra[ring * spectrum_len + m] = Complex64::new(0.2 + offset, 0.1 - offset);
            u_spectra[ring * spectrum_len + m] = Complex64::new(-0.3 + offset, 0.07 + offset);
        }

        let mut paired_e = vec![Complex64::new(0.0, 0.0); lmax - m + 1];
        let mut paired_b = paired_e.clone();
        let used = plan
            .analyze_spin_scalar_gl_paired_order(
                &q_spectra,
                &u_spectra,
                spectrum_len,
                longitude_phase::<f64>(m, plan.grid.phi0(), true),
                &ring_scales,
                m,
                &workspace.theta_sin,
                &workspace.theta_cos,
                &ring_order_limits,
                pulp::Arch::new(),
                &mut paired_e,
                Some(&mut paired_b),
                &mut workspace.recurrence_workers[0],
            )
            .unwrap();
        assert!(used);

        let mut full_e = vec![Complex64::new(0.0, 0.0); lmax - m + 1];
        let mut full_b = full_e.clone();
        plan.analyze_spin_scalar_fast_order(
            &q_spectra,
            &u_spectra,
            spectrum_len,
            nphi as f64,
            longitude_phase::<f64>(m, plan.grid.phi0(), true),
            Some(plan.grid.ring_weights()),
            true,
            m,
            0,
            ntheta,
            &workspace.theta_sin,
            &workspace.theta_cos,
            &ring_order_limits,
            pulp::Arch::new(),
            &mut full_e,
            Some(&mut full_b),
            &mut workspace.recurrence_workers[0],
        )
        .unwrap();

        for (paired, full) in paired_e
            .iter()
            .chain(&paired_b)
            .zip(full_e.iter().chain(&full_b))
        {
            let scale = full.norm().max(1.0);
            assert!((paired - full).norm() <= 2.0e-11 * scale);
        }
    }
}

#[test]
fn spin1_reflected_analysis_subranges_fall_back_for_extreme_inputs_on_all_grids() {
    let lmax = 511;
    let nphi = 2 * lmax + 3;
    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let ntheta = lmax + usize::from(kind == GridKind::ClenshawCurtis) + 1;
        let plan = ShtPlan::new(
            GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::new(lmax, lmax, 1).unwrap(),
        )
        .unwrap();
        let mut workspace = plan.workspace().unwrap();
        plan.prepare_spin_latitudes(&mut workspace, 1);
        let spectrum_len = plan.fft.spectrum_len();
        let ring_start = workspace
            .theta_sin
            .iter()
            .position(|&sine| sine >= SPIN1_FAST_MIN_SIN_THETA)
            .unwrap();
        let ring_end = workspace.theta_sin[ring_start..]
            .iter()
            .position(|&sine| sine < SPIN1_FAST_MIN_SIN_THETA)
            .map_or(ntheta, |offset| ring_start + offset);
        assert_eq!(ring_start, ntheta - ring_end);
        let ring_scales: Vec<_> = plan
            .grid
            .ring_weights()
            .iter()
            .zip(&workspace.theta_sin)
            .map(|(&weight, &sine)| (weight / sine) / nphi as f64)
            .collect();
        let mut q_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
        let u_spectra = q_spectra.clone();
        for ring in 0..ntheta {
            q_spectra[ring * spectrum_len] = Complex64::new(1.0e300, -1.0e300);
        }
        let sentinel = Complex64::new(0.25, -0.5);
        let mut paired_e = vec![sentinel; lmax + 1];
        let mut paired_b = paired_e.clone();
        let used = plan
            .analyze_spin_scalar_reflected_order(
                &q_spectra,
                &u_spectra,
                spectrum_len,
                longitude_phase::<f64>(0, plan.grid.phi0(), true),
                &ring_scales,
                0,
                ring_start,
                ring_end,
                &workspace.theta_sin,
                &workspace.theta_cos,
                &workspace.ring_order_limits,
                pulp::Arch::new(),
                &mut paired_e,
                Some(&mut paired_b),
                &mut workspace.recurrence_workers[0],
            )
            .unwrap();
        assert!(
            !used,
            "{kind:?} exceptional input should decline the fast path"
        );
        assert!(paired_e.iter().all(|&value| value == sentinel));
        assert!(paired_b.iter().all(|&value| value == sentinel));

        let mut fallback_e = vec![Complex64::new(0.0, 0.0); lmax + 1];
        let mut fallback_b = fallback_e.clone();
        plan.analyze_spin_scalar_fast_order(
            &q_spectra,
            &u_spectra,
            spectrum_len,
            nphi as f64,
            longitude_phase::<f64>(0, plan.grid.phi0(), true),
            Some(plan.grid.ring_weights()),
            true,
            0,
            ring_start,
            ring_end,
            &workspace.theta_sin,
            &workspace.theta_cos,
            &workspace.ring_order_limits,
            pulp::Arch::new(),
            &mut fallback_e,
            Some(&mut fallback_b[..]),
            &mut workspace.recurrence_workers[0],
        )
        .unwrap();
        assert!(
            fallback_e
                .iter()
                .chain(&fallback_b)
                .all(|value| value.re.is_finite() && value.im.is_finite()),
            "{kind:?} fallback should keep extreme finite inputs finite"
        );
    }
}

#[test]
fn spin1_reflected_analysis_subranges_reset_poisoned_scratch_on_gl_cc() {
    let lmax = 511;
    let nphi = 2 * lmax + 3;
    for kind in [GridKind::GaussLegendre, GridKind::ClenshawCurtis] {
        let ntheta = lmax + usize::from(kind == GridKind::ClenshawCurtis) + 1;
        let plan = ShtPlan::new(
            GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::new(lmax, lmax, 1).unwrap(),
        )
        .unwrap();
        let mut workspace = plan.workspace().unwrap();
        plan.prepare_spin_latitudes(&mut workspace, 1);
        let spectrum_len = plan.fft.spectrum_len();
        let ring_start = workspace
            .theta_sin
            .iter()
            .position(|&sine| sine >= SPIN1_FAST_MIN_SIN_THETA)
            .unwrap();
        let ring_end = workspace.theta_sin[ring_start..]
            .iter()
            .position(|&sine| sine < SPIN1_FAST_MIN_SIN_THETA)
            .map_or(ntheta, |offset| ring_start + offset);
        assert_eq!(ring_start, ntheta - ring_end);
        let ring_scales: Vec<_> = plan
            .grid
            .ring_weights()
            .iter()
            .zip(&workspace.theta_sin)
            .map(|(&weight, &sine)| (weight / sine) / nphi as f64)
            .collect();
        let mut q_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
        let mut u_spectra = q_spectra.clone();
        for ring in 0..ntheta {
            let weight = (ring + 1) as f64 / ntheta as f64;
            q_spectra[ring * spectrum_len] = Complex64::new(weight * 1.0e-3, 0.0);
            u_spectra[ring * spectrum_len] = Complex64::new(-weight * 0.7e-3, 0.0);
        }
        let phase = longitude_phase::<f64>(0, plan.grid.phi0(), true);
        let mut expected_e = vec![Complex64::new(0.0, 0.0); lmax + 1];
        let mut expected_b = expected_e.clone();
        let used = plan
            .analyze_spin_scalar_reflected_order(
                &q_spectra,
                &u_spectra,
                spectrum_len,
                phase,
                &ring_scales,
                0,
                ring_start,
                ring_end,
                &workspace.theta_sin,
                &workspace.theta_cos,
                &workspace.ring_order_limits,
                pulp::Arch::new(),
                &mut expected_e,
                Some(&mut expected_b),
                &mut workspace.recurrence_workers[0],
            )
            .unwrap();
        assert!(used, "{kind:?} normal input should use the reflected path");

        let recurrence = &mut workspace.recurrence_workers[0];
        for values in [
            &mut recurrence.previous[..],
            &mut recurrence.current[..],
            &mut recurrence.next[..],
            &mut recurrence.analysis_same_real[..],
            &mut recurrence.analysis_same_imag[..],
            &mut recurrence.analysis_opposite_real[..],
            &mut recurrence.analysis_opposite_imag[..],
        ] {
            values.fill(f64::NAN);
        }
        recurrence.epsilon.fill(f64::NAN);
        recurrence.exponents.fill(i64::MAX);
        recurrence.analysis_scale_epochs.fill(f64::NAN);
        recurrence.analysis_scale_corrections.fill(f64::NAN);
        recurrence.spin_alpha.fill(f64::NAN);
        recurrence.spin_coefficient_a.fill(f64::NAN);
        recurrence.spin_coefficient_b.fill(f64::NAN);
        recurrence.spin_previous_plus.fill(f64::NAN);
        recurrence.spin_current_plus.fill(f64::NAN);
        recurrence.spin_previous_minus.fill(f64::NAN);
        recurrence.spin_current_minus.fill(f64::NAN);
        recurrence.compact_theta_sin.fill(f64::NAN);
        recurrence.compact_theta_cos.fill(f64::NAN);
        recurrence
            .fast_scalar_first
            .fill(Complex64::new(f64::NAN, f64::NAN));
        recurrence
            .fast_scalar_second
            .fill(Complex64::new(f64::NAN, f64::NAN));
        recurrence
            .fast_spin_first
            .fill(Complex64::new(f64::NAN, f64::NAN));
        recurrence
            .fast_spin_second
            .fill(Complex64::new(f64::NAN, f64::NAN));

        let mut actual_e = vec![Complex64::new(0.0, 0.0); lmax + 1];
        let mut actual_b = actual_e.clone();
        let used = plan
            .analyze_spin_scalar_reflected_order(
                &q_spectra,
                &u_spectra,
                spectrum_len,
                phase,
                &ring_scales,
                0,
                ring_start,
                ring_end,
                &workspace.theta_sin,
                &workspace.theta_cos,
                &workspace.ring_order_limits,
                pulp::Arch::new(),
                &mut actual_e,
                Some(&mut actual_b),
                &mut workspace.recurrence_workers[0],
            )
            .unwrap();
        assert!(used, "{kind:?} poisoned scratch should be overwritten");
        for (actual, expected) in actual_e
            .iter()
            .chain(&actual_b)
            .zip(expected_e.iter().chain(&expected_b))
        {
            assert!(
                (*actual - *expected).norm() <= 2.0e-12 * expected.norm().max(1.0),
                "{kind:?} poisoned-scratch mismatch"
            );
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_analysis_reuses_poisoned_workspace_for_extremes_on_all_grids() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let lmax = 511;
    let nphi = 2 * lmax + 3;
    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let ntheta = lmax + usize::from(kind == GridKind::ClenshawCurtis) + 1;
        let plan = ShtPlan::new(
            GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::new(lmax, lmax, 1).unwrap(),
        )
        .unwrap();
        let (first, second) = arbitrary_maps(&plan);
        let mut workspace = pool.install(|| plan.workspace()).unwrap();
        let mut expected_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut expected_b = expected_e.clone();
        pool.install(|| {
            plan.analysis_spin_into(
                &first,
                &second,
                &mut expected_e,
                &mut expected_b,
                &mut workspace,
            )
        })
        .unwrap();

        workspace.analysis_ring_scales.fill(f64::NAN);
        workspace
            .spin1_scalar_coefficients
            .fill(Complex64::new(f64::NAN, f64::NAN));
        workspace.spin1_scalar_coefficient_finite_orders.fill(0);
        for recurrence in &mut workspace.recurrence_workers {
            for values in [
                &mut recurrence.previous[..],
                &mut recurrence.current[..],
                &mut recurrence.next[..],
                &mut recurrence.analysis_same_real[..],
                &mut recurrence.analysis_same_imag[..],
                &mut recurrence.analysis_opposite_real[..],
                &mut recurrence.analysis_opposite_imag[..],
            ] {
                values.fill(f64::NAN);
            }
            recurrence.epsilon.fill(f64::NAN);
            recurrence.exponents.fill(i64::MAX);
            recurrence.analysis_scale_epochs.fill(f64::NAN);
            recurrence.analysis_scale_corrections.fill(f64::NAN);
            recurrence.spin_alpha.fill(f64::NAN);
            recurrence.spin_coefficient_a.fill(f64::NAN);
            recurrence.spin_coefficient_b.fill(f64::NAN);
            recurrence.spin_previous_plus.fill(f64::NAN);
            recurrence.spin_current_plus.fill(f64::NAN);
            recurrence.spin_previous_minus.fill(f64::NAN);
            recurrence.spin_current_minus.fill(f64::NAN);
            recurrence.compact_theta_sin.fill(f64::NAN);
            recurrence.compact_theta_cos.fill(f64::NAN);
            recurrence.compact_theta_cos_squared.fill(f64::NAN);
            recurrence
                .fast_scalar_first
                .fill(Complex64::new(f64::NAN, f64::NAN));
            recurrence
                .fast_scalar_second
                .fill(Complex64::new(f64::NAN, f64::NAN));
            recurrence
                .fast_spin_first
                .fill(Complex64::new(f64::NAN, f64::NAN));
            recurrence
                .fast_spin_second
                .fill(Complex64::new(f64::NAN, f64::NAN));
            recurrence
                .fast_spin_pair_first
                .fill(Complex64::new(f64::NAN, f64::NAN));
            recurrence
                .fast_spin_pair_second
                .fill(Complex64::new(f64::NAN, f64::NAN));
            recurrence
                .weighted_spin_q_spectrum
                .fill(Complex64::new(f64::NAN, f64::NAN));
            recurrence
                .weighted_spin_u_spectrum
                .fill(Complex64::new(f64::NAN, f64::NAN));
            recurrence.scalar_order_correction.fill(f64::NAN);
            recurrence.compact_ring_indices.fill(usize::MAX);
            recurrence.spin_hemisphere_pair_basis.fill(f64::NAN);
        }

        let mut actual_e = vec![Complex64::new(f64::NAN, f64::NAN); plan.harmonics().nalm()];
        let mut actual_b = actual_e.clone();
        pool.install(|| {
            plan.analysis_spin_into(
                &first,
                &second,
                &mut actual_e,
                &mut actual_b,
                &mut workspace,
            )
        })
        .unwrap();
        assert!(
            relative_l2_complex(&actual_e, &expected_e) <= 2.0e-12,
            "{kind:?} E"
        );
        assert!(
            relative_l2_complex(&actual_b, &expected_b) <= 2.0e-12,
            "{kind:?} B"
        );

        let large_first = vec![1.0e200; plan.sample_count];
        let large_second = vec![-2.5e199; plan.sample_count];
        actual_e.fill(Complex64::new(f64::NAN, f64::NAN));
        actual_b.fill(Complex64::new(f64::NAN, f64::NAN));
        pool.install(|| {
            plan.analysis_spin_into(
                &large_first,
                &large_second,
                &mut actual_e,
                &mut actual_b,
                &mut workspace,
            )
        })
        .unwrap();
        assert!(
            actual_e
                .iter()
                .chain(&actual_b)
                .all(|value| value.re.is_finite() && value.im.is_finite()),
            "{kind:?} extreme finite analysis"
        );

        let tiny = f64::from_bits(1);
        let tiny_first = vec![tiny; plan.sample_count];
        let tiny_second = vec![-tiny; plan.sample_count];
        pool.install(|| {
            plan.analysis_spin_into(
                &tiny_first,
                &tiny_second,
                &mut actual_e,
                &mut actual_b,
                &mut workspace,
            )
        })
        .unwrap();
        assert!(
            actual_e
                .iter()
                .chain(&actual_b)
                .all(|value| value.re.is_finite() && value.im.is_finite()),
            "{kind:?} subnormal analysis"
        );
    }
}

#[test]
fn spin1_fused_analysis_stages_both_channels_before_a_late_scaled_fallback() {
    let plan = make_plan(1, 63, 63, 64, 129);
    let mut workspace = plan.workspace().unwrap();
    let recurrence = &mut workspace.recurrence_workers[0];
    let m = 0;
    let scalar_lmax = 8;
    let sin_theta = [0.35_f64, 0.6, 0.9];
    let cos_theta = sin_theta.map(|sine| (1.0 - sine * sine).sqrt());
    let q_same_real = [0.2, -0.3, 0.4];
    let q_same_imag = [-0.1, 0.05, 0.15];
    let q_opposite_real = [0.07, 0.13, -0.11];
    let q_opposite_imag = [0.09, -0.08, 0.04];
    let u_same_real = [-0.3, 0.1, 0.2];
    let u_same_imag = [0.04, 0.12, -0.07];
    let u_opposite_real = [0.14, -0.09, 0.03];
    let u_opposite_imag = [-0.05, 0.02, 0.11];
    let mut coefficient_a = vec![0.0; scalar_lmax + 1];
    let mut coefficient_b = vec![0.0; scalar_lmax + 1];
    coefficient_b[0] = 1.0;
    coefficient_a[1] = 1.0e300;
    let initial = Complex64::new(0.25, -0.125);
    recurrence.fast_scalar_first[..=scalar_lmax].fill(initial);
    recurrence.fast_scalar_second[..=scalar_lmax].fill(initial);

    let fused = ShtPlan::<f64>::try_fused_spin_scalar_analysis_pair_block(
        pulp::Arch::new(),
        true,
        false,
        m,
        scalar_lmax,
        plan.sectoral_norms[m],
        &sin_theta,
        &cos_theta,
        [true, true],
        [&q_same_real, &u_same_real],
        [&q_same_imag, &u_same_imag],
        [&q_opposite_real, &u_opposite_real],
        [&q_opposite_imag, &u_opposite_imag],
        &coefficient_a,
        &coefficient_b,
        &mut recurrence.fast_scalar_first[..=scalar_lmax],
        &mut recurrence.fast_scalar_second[..=scalar_lmax],
        &mut recurrence.fast_spin_first,
        &mut recurrence.fast_spin_second,
        &mut recurrence.previous,
        &mut recurrence.current,
        &mut recurrence.next,
        &mut recurrence.exponents,
    )
    .unwrap();
    assert!(!fused, "the second recurrence pair enters the scaled range");
    assert!(
        recurrence.fast_spin_first[..=scalar_lmax]
            .iter()
            .any(|value| *value != Complex64::new(0.0, 0.0))
    );
    assert!(
        recurrence.fast_spin_second[..=scalar_lmax]
            .iter()
            .any(|value| *value != Complex64::new(0.0, 0.0))
    );
    assert!(
        recurrence.fast_scalar_first[..=scalar_lmax]
            .iter()
            .all(|value| *value == initial)
    );
    assert!(
        recurrence.fast_scalar_second[..=scalar_lmax]
            .iter()
            .all(|value| *value == initial)
    );

    let mut expected_workspace = plan.workspace().unwrap();
    let expected_recurrence = &mut expected_workspace.recurrence_workers[0];
    expected_recurrence.fast_scalar_first[..=scalar_lmax].fill(initial);
    expected_recurrence.fast_scalar_second[..=scalar_lmax].fill(initial);
    let fallback_used = ShtPlan::<f64>::accumulate_spin_scalar_analysis_pair_block(
        pulp::Arch::new(),
        m,
        scalar_lmax,
        plan.sectoral_norms[m],
        &sin_theta,
        &cos_theta,
        true,
        true,
        &q_same_real,
        &q_same_imag,
        &q_opposite_real,
        &q_opposite_imag,
        &u_same_real,
        &u_same_imag,
        &u_opposite_real,
        &u_opposite_imag,
        &coefficient_a,
        &coefficient_b,
        &mut expected_recurrence.previous,
        &mut expected_recurrence.current,
        &mut expected_recurrence.next,
        &mut expected_recurrence.exponents,
        #[cfg(all(test, feature = "parallel"))]
        None,
        &mut expected_recurrence.fast_scalar_first[..=scalar_lmax],
        &mut expected_recurrence.fast_scalar_second[..=scalar_lmax],
    )
    .unwrap();
    assert!(fallback_used);

    let fallback_used = ShtPlan::<f64>::accumulate_spin_scalar_analysis_pair_block(
        pulp::Arch::new(),
        m,
        scalar_lmax,
        plan.sectoral_norms[m],
        &sin_theta,
        &cos_theta,
        true,
        true,
        &q_same_real,
        &q_same_imag,
        &q_opposite_real,
        &q_opposite_imag,
        &u_same_real,
        &u_same_imag,
        &u_opposite_real,
        &u_opposite_imag,
        &coefficient_a,
        &coefficient_b,
        &mut recurrence.previous,
        &mut recurrence.current,
        &mut recurrence.next,
        &mut recurrence.exponents,
        #[cfg(all(test, feature = "parallel"))]
        None,
        &mut recurrence.fast_scalar_first[..=scalar_lmax],
        &mut recurrence.fast_scalar_second[..=scalar_lmax],
    )
    .unwrap();
    assert!(fallback_used);
    assert_eq!(
        &recurrence.fast_scalar_first[..=scalar_lmax],
        &expected_recurrence.fast_scalar_first[..=scalar_lmax]
    );
    assert_eq!(
        &recurrence.fast_scalar_second[..=scalar_lmax],
        &expected_recurrence.fast_scalar_second[..=scalar_lmax]
    );
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_fused_analysis_falls_back_when_only_q_products_are_unsafe() {
    let lmax = 511;
    let ntheta = lmax + 1;
    let nphi = 2 * lmax + 3;
    let fast_plan = make_plan(1, lmax, lmax, ntheta, nphi);
    let mut general_plan = make_plan(1, lmax, lmax, ntheta, nphi);
    general_plan.spin_fast_coefficient_count = 0;
    assert!(general_plan.spin1_scalar_legendre_table.set(None).is_ok());
    let mut fast_workspace = fast_plan.workspace().unwrap();
    let mut general_workspace = general_plan.workspace().unwrap();
    fast_plan.prepare_spin_latitudes(&mut fast_workspace, 1);
    general_plan.prepare_spin_latitudes(&mut general_workspace, 1);
    let fast_ring_scales: Vec<_> = fast_plan
        .grid
        .ring_weights()
        .iter()
        .zip(&fast_workspace.theta_sin)
        .map(|(&weight, &sine)| (weight / sine) / nphi as f64)
        .collect();
    let general_ring_scales: Vec<_> = general_plan
        .grid
        .ring_weights()
        .iter()
        .zip(&general_workspace.theta_sin)
        .map(|(&weight, &sine)| (weight / sine) / nphi as f64)
        .collect();

    let spectrum_len = fast_plan.fft.spectrum_len();
    let mut q_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
    let mut u_spectra = q_spectra.clone();
    let m = 17;
    for ring in 0..ntheta {
        let index = ring * spectrum_len + m;
        if ring == ntheta / 2 {
            q_spectra[index] = Complex64::new(1.0e187, -0.5e187);
        }
        u_spectra[index] = Complex64::new((ring as f64 * 0.17).sin(), 0.25);
    }
    let coefficient_count = lmax - m + 1;
    let mut fast_e = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut fast_b = fast_e.clone();
    let mut general_e = fast_e.clone();
    let mut general_b = fast_e.clone();
    in_single_thread_pool(|| {
        fast_plan
            .analyze_spin_order_group(
                &q_spectra,
                &u_spectra,
                nphi,
                spectrum_len,
                fast_plan.grid.phi0(),
                Some(fast_plan.grid.ring_weights()),
                true,
                Some(&fast_ring_scales),
                false,
                1,
                false,
                &fast_workspace.theta_sin,
                &fast_workspace.theta_cos,
                &fast_workspace.ring_order_limits,
                pulp::Arch::new(),
                m,
                m + 1,
                &mut fast_e,
                Some(&mut fast_b),
                &mut fast_workspace.recurrence_workers[0],
            )
            .unwrap();
    });
    general_plan
        .analyze_spin_order_group(
            &q_spectra,
            &u_spectra,
            nphi,
            spectrum_len,
            general_plan.grid.phi0(),
            Some(general_plan.grid.ring_weights()),
            true,
            Some(&general_ring_scales),
            false,
            1,
            false,
            &general_workspace.theta_sin,
            &general_workspace.theta_cos,
            &general_workspace.ring_order_limits,
            pulp::Arch::new(),
            m,
            m + 1,
            &mut general_e,
            Some(&mut general_b),
            &mut general_workspace.recurrence_workers[0],
        )
        .unwrap();

    let fast = fast_e.iter().chain(&fast_b).copied().collect::<Vec<_>>();
    let general = general_e
        .iter()
        .chain(&general_b)
        .copied()
        .collect::<Vec<_>>();
    assert!(
        fast.iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    assert!(
        general
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    let scale = general
        .iter()
        .map(|value| value.norm())
        .fold(0.0_f64, f64::max);
    let fast = fast.iter().map(|value| *value / scale).collect::<Vec<_>>();
    let general = general
        .iter()
        .map(|value| *value / scale)
        .collect::<Vec<_>>();
    let error = relative_l2_complex(&fast, &general);
    assert!(error <= 3e-10, "one-channel unsafe fallback L2={error:.3e}");
}

#[test]
fn spin1_standard_analysis_scalar_conversion_matches_general_and_adjoint_on_all_grids() {
    for lmax in [14, 15, 31, 47, 63, 72, 127, 159, 191, 255] {
        for kind in [
            GridKind::GaussLegendre,
            GridKind::ClenshawCurtis,
            GridKind::Fejer1,
        ] {
            let minimum_ntheta = lmax + usize::from(kind == GridKind::ClenshawCurtis) + 1;
            let ntheta_values = if kind == GridKind::ClenshawCurtis {
                vec![minimum_ntheta, 2 * lmax + 2]
            } else if kind == GridKind::GaussLegendre {
                let atmospheric_ntheta = match lmax {
                    63 => Some(96),
                    85 => Some(128),
                    127 => Some(256),
                    159 => Some(320),
                    _ => None,
                };
                atmospheric_ntheta
                    .map_or_else(|| vec![minimum_ntheta], |value| vec![minimum_ntheta, value])
            } else {
                vec![minimum_ntheta]
            };
            for ntheta in ntheta_values {
                for nphi in [2 * lmax + 2, 2 * lmax + 3] {
                    let make_plan = || {
                        ShtPlan::new(
                            GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
                            HarmonicSpec::new(lmax, lmax, 1).unwrap(),
                        )
                        .unwrap()
                    };
                    let fast_plan = make_plan();
                    let mut general_plan = make_plan();
                    general_plan.spin_fast_coefficient_count = 0;
                    assert!(general_plan.spin1_scalar_legendre_table.set(None).is_ok());
                    let (first, second) = arbitrary_maps(&fast_plan);
                    let mut fast_workspace = fast_plan.workspace().unwrap();
                    let mut general_workspace = general_plan.workspace().unwrap();
                    let mut fast_e = vec![Complex64::new(0.0, 0.0); fast_plan.harmonics().nalm()];
                    let mut fast_b = fast_e.clone();
                    let mut general_e = fast_e.clone();
                    let mut general_b = fast_e.clone();
                    fast_plan
                        .analysis_spin_into(
                            &first,
                            &second,
                            &mut fast_e,
                            &mut fast_b,
                            &mut fast_workspace,
                        )
                        .unwrap();
                    if kind == GridKind::GaussLegendre && matches!(lmax, 63 | 72 | 85 | 127) {
                        assert!(
                            fast_plan
                                .spin1_scalar_legendre_table
                                .get()
                                .is_some_and(Option::is_some)
                        );
                    }
                    if kind == GridKind::GaussLegendre && matches!(lmax, 159 | 255) {
                        assert_eq!(
                            fast_plan
                                .spin1_scalar_legendre_table
                                .get()
                                .is_some_and(Option::is_some),
                            fast_plan.allow_spin1_analysis_table()
                        );
                    }
                    general_plan
                        .analysis_spin_into(
                            &first,
                            &second,
                            &mut general_e,
                            &mut general_b,
                            &mut general_workspace,
                        )
                        .unwrap();
                    for (channel, actual, expected) in
                        [("E", &fast_e, &general_e), ("B", &fast_b, &general_b)]
                    {
                        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate()
                        {
                            let error = (actual - expected).norm();
                            assert!(
                                error <= 3e-10 * expected.norm().max(1.0),
                                "{kind:?} T{lmax} {ntheta}x{nphi} spin-1 {channel}[{index}] error={error:.3e}"
                            );
                        }
                    }

                    let (e, b) = coefficients(&fast_plan);
                    let mut adjoint_first = vec![0.0; fast_plan.sample_count];
                    let mut adjoint_second = vec![0.0; fast_plan.sample_count];
                    fast_plan
                        .adjoint_analysis_spin_into(
                            &e,
                            &b,
                            &mut adjoint_first,
                            &mut adjoint_second,
                            &mut fast_workspace,
                        )
                        .unwrap();
                    let left = spin_harmonic_inner_product(
                        fast_plan.harmonics(),
                        &fast_e,
                        &fast_b,
                        &e,
                        &b,
                    );
                    let right =
                        spin_map_inner_product(&first, &second, &adjoint_first, &adjoint_second);
                    assert_close(left, right, 3e-9 * left.abs().max(right.abs()).max(1.0));
                }
            }
        }
    }
}

#[test]
fn concurrent_spin1_analysis_initializes_the_reusable_table_safely() {
    let plan = make_plan(1, 72, 72, 73, 147);
    let (first, second) = arbitrary_maps(&plan);
    let table_expected = plan.allow_spin1_analysis_table();
    assert!(plan.spin1_scalar_legendre_table.get().is_none());

    let run_analysis = || {
        let mut workspace = plan.workspace().unwrap();
        let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut b = e.clone();
        plan.analysis_spin_into(&first, &second, &mut e, &mut b, &mut workspace)
            .unwrap();
        (e, b)
    };
    let (left, right) = std::thread::scope(|scope| {
        let left = scope.spawn(run_analysis);
        let right = scope.spawn(run_analysis);
        (left.join().unwrap(), right.join().unwrap())
    });
    assert_eq!(left.0, right.0);
    assert_eq!(left.1, right.1);
    assert_eq!(
        plan.spin1_scalar_legendre_table
            .get()
            .is_some_and(Option::is_some),
        table_expected
    );
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_max_band_analysis_uses_recurrence_for_one_two_four_eight_and_sixteen_threads() {
    let plan = make_plan(1, 255, 255, 256, 513);
    let (first, second) = arbitrary_maps(&plan);
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let four_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let two_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let sixteen_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build()
        .unwrap();
    let mut one_workspace = one_thread.install(|| plan.workspace()).unwrap();
    let mut one_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut one_b = one_e.clone();
    one_thread
        .install(|| {
            plan.analysis_spin_into(&first, &second, &mut one_e, &mut one_b, &mut one_workspace)
        })
        .unwrap();
    assert!(plan.spin1_scalar_legendre_table.get().is_none());

    let mut two_workspace = two_threads.install(|| plan.workspace()).unwrap();
    let mut two_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut two_b = two_e.clone();
    two_threads
        .install(|| {
            assert!(!plan.allow_spin1_analysis_table());
            plan.analysis_spin_into(&first, &second, &mut two_e, &mut two_b, &mut two_workspace)
        })
        .unwrap();

    let mut four_workspace = four_threads.install(|| plan.workspace()).unwrap();
    let mut four_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut four_b = four_e.clone();
    four_threads
        .install(|| {
            assert!(!plan.allow_spin1_analysis_table());
            plan.analysis_spin_into(
                &first,
                &second,
                &mut four_e,
                &mut four_b,
                &mut four_workspace,
            )
        })
        .unwrap();
    assert!(plan.spin1_scalar_legendre_table.get().is_none());

    let mut eight_workspace = eight_threads.install(|| plan.workspace()).unwrap();
    let mut eight_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut eight_b = eight_e.clone();
    eight_threads
        .install(|| {
            assert!(!plan.allow_spin1_analysis_table());
            plan.analysis_spin_into(
                &first,
                &second,
                &mut eight_e,
                &mut eight_b,
                &mut eight_workspace,
            )
        })
        .unwrap();
    assert!(plan.spin1_scalar_legendre_table.get().is_none());

    let mut sixteen_workspace = sixteen_threads.install(|| plan.workspace()).unwrap();
    let mut sixteen_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut sixteen_b = sixteen_e.clone();
    sixteen_threads
        .install(|| {
            assert!(!plan.allow_spin1_analysis_table());
            plan.analysis_spin_into(
                &first,
                &second,
                &mut sixteen_e,
                &mut sixteen_b,
                &mut sixteen_workspace,
            )
        })
        .unwrap();
    assert!(plan.spin1_scalar_legendre_table.get().is_none());
    for (&one, &other) in one_e
        .iter()
        .zip(&four_e)
        .chain(one_b.iter().zip(&four_b))
        .chain(one_e.iter().zip(&two_e))
        .chain(one_b.iter().zip(&two_b))
        .chain(one_e.iter().zip(&eight_e))
        .chain(one_b.iter().zip(&eight_b))
        .chain(one_e.iter().zip(&sixteen_e))
        .chain(one_b.iter().zip(&sixteen_b))
    {
        assert!((one - other).norm() <= 2e-10 * one.norm().max(other.norm()).max(1.0));
    }
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_analysis_table_selection_tracks_thread_pool_and_bandlimit() {
    let plan = make_plan(1, 72, 72, 73, 147);
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    assert!(one_thread.install(|| plan.allow_spin1_analysis_table()));

    let mut workspace = one_thread.install(|| plan.workspace()).unwrap();
    let maps = vec![0.0; plan.sample_count];
    let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b = e.clone();
    one_thread
        .install(|| plan.analysis_spin_into(&maps, &maps, &mut e, &mut b, &mut workspace))
        .unwrap();
    assert!(
        plan.spin1_scalar_legendre_table
            .get()
            .is_some_and(Option::is_some)
    );

    let t85 = make_plan(1, 85, 85, 86, 173);
    assert!(one_thread.install(|| t85.allow_spin1_analysis_table()));

    let atmospheric_t85 = make_plan(1, 85, 85, 128, 256);
    assert!(one_thread.install(|| atmospheric_t85.allow_spin1_analysis_table()));
    let atmospheric_t63 = make_plan(1, 63, 63, 96, 192);
    assert!(one_thread.install(|| atmospheric_t63.allow_spin1_analysis_table()));
    let reduced_atmospheric_t127 = make_plan(1, 127, 96, 256, 512);
    assert!(!one_thread.install(|| reduced_atmospheric_t127.allow_spin1_analysis_table()));
    let atmospheric_t191 = make_plan(1, 191, 191, 384, 768);
    assert!(!one_thread.install(|| atmospheric_t191.allow_spin1_analysis_table()));
    let atmospheric_t159 = make_plan(1, 159, 159, 320, 640);
    assert!(!one_thread.install(|| atmospheric_t159.allow_spin1_analysis_table()));
    let t159_zero_maps = vec![0.0; atmospheric_t159.sample_count];
    let mut t159_one_workspace = one_thread.install(|| atmospheric_t159.workspace()).unwrap();
    let mut t159_one_e = vec![Complex64::new(0.0, 0.0); atmospheric_t159.harmonics().nalm()];
    let mut t159_one_b = t159_one_e.clone();
    one_thread
        .install(|| {
            atmospheric_t159.analysis_spin_into(
                &t159_zero_maps,
                &t159_zero_maps,
                &mut t159_one_e,
                &mut t159_one_b,
                &mut t159_one_workspace,
            )
        })
        .unwrap();
    assert!(atmospheric_t159.spin1_scalar_legendre_table.get().is_none());

    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    assert!(!eight_threads.install(|| atmospheric_t191.allow_spin1_analysis_table()));
    assert!(eight_threads.install(|| atmospheric_t159.allow_spin1_analysis_table()));
    assert!(eight_threads.install(|| atmospheric_t63.allow_spin1_analysis_table()));
    let mut eight_workspace = eight_threads.install(|| plan.workspace()).unwrap();
    assert!(eight_threads.install(|| plan.allow_spin1_analysis_table()));
    eight_threads
        .install(|| plan.analysis_spin_into(&maps, &maps, &mut e, &mut b, &mut eight_workspace))
        .unwrap();
    assert!(
        plan.spin1_scalar_legendre_table
            .get()
            .is_some_and(Option::is_some)
    );

    for thread_count in [2, 4, 16] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap();
        assert!(pool.install(|| atmospheric_t63.allow_spin1_analysis_table()));
        assert!(pool.install(|| atmospheric_t85.allow_spin1_analysis_table()));
        assert!(!pool.install(|| atmospheric_t191.allow_spin1_analysis_table()));
        assert!(pool.install(|| atmospheric_t159.allow_spin1_analysis_table()));
    }

    let mut t159_general = make_plan(1, 159, 159, 320, 640);
    t159_general.spin_fast_coefficient_count = 0;
    assert!(t159_general.spin1_scalar_legendre_table.set(None).is_ok());
    let (t159_first, t159_second) = arbitrary_maps(&atmospheric_t159);
    let mut t159_workspace = eight_threads
        .install(|| atmospheric_t159.workspace())
        .unwrap();
    let mut t159_general_workspace = eight_threads.install(|| t159_general.workspace()).unwrap();
    let mut t159_e = vec![Complex64::new(0.0, 0.0); atmospheric_t159.harmonics().nalm()];
    let mut t159_b = t159_e.clone();
    let mut t159_general_e = t159_e.clone();
    let mut t159_general_b = t159_b.clone();
    eight_threads
        .install(|| {
            atmospheric_t159.analysis_spin_into(
                &t159_first,
                &t159_second,
                &mut t159_e,
                &mut t159_b,
                &mut t159_workspace,
            )
        })
        .unwrap();
    eight_threads
        .install(|| {
            t159_general.analysis_spin_into(
                &t159_first,
                &t159_second,
                &mut t159_general_e,
                &mut t159_general_b,
                &mut t159_general_workspace,
            )
        })
        .unwrap();
    for (actual, expected) in t159_e
        .iter()
        .zip(&t159_general_e)
        .chain(t159_b.iter().zip(&t159_general_b))
    {
        assert!((*actual - *expected).norm() <= 3e-10 * expected.norm().max(1.0));
    }
    assert!(
        atmospheric_t159
            .spin1_scalar_legendre_table
            .get()
            .is_some_and(Option::is_some)
    );

    let (test_e, test_b) = coefficients(&atmospheric_t159);
    let mut adjoint_first = vec![0.0; atmospheric_t159.sample_count];
    let mut adjoint_second = vec![0.0; atmospheric_t159.sample_count];
    eight_threads
        .install(|| {
            atmospheric_t159.adjoint_analysis_spin_into(
                &test_e,
                &test_b,
                &mut adjoint_first,
                &mut adjoint_second,
                &mut t159_workspace,
            )
        })
        .unwrap();
    let left = spin_harmonic_inner_product(
        atmospheric_t159.harmonics(),
        &t159_e,
        &t159_b,
        &test_e,
        &test_b,
    );
    let right = spin_map_inner_product(&t159_first, &t159_second, &adjoint_first, &adjoint_second);
    assert_close(left, right, 3e-9 * left.abs().max(right.abs()).max(1.0));
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_single_thread_analysis_table_remains_selected_through_t224() {
    let table_plan = make_plan(1, 224, 224, 225, 451);
    let mut general_plan = make_plan(1, 224, 224, 225, 451);
    general_plan.spin_fast_coefficient_count = 0;
    let (first, second) = arbitrary_maps(&table_plan);
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    assert!(one_thread.install(|| table_plan.allow_spin1_analysis_table()));

    let mut table_workspace = one_thread.install(|| table_plan.workspace()).unwrap();
    let mut general_workspace = one_thread.install(|| general_plan.workspace()).unwrap();
    let mut table_e = vec![Complex64::new(0.0, 0.0); table_plan.harmonics().nalm()];
    let mut table_b = table_e.clone();
    let mut general_e = table_e.clone();
    let mut general_b = table_e.clone();
    one_thread
        .install(|| {
            table_plan.analysis_spin_into(
                &first,
                &second,
                &mut table_e,
                &mut table_b,
                &mut table_workspace,
            )
        })
        .unwrap();
    general_plan
        .analysis_spin_into(
            &first,
            &second,
            &mut general_e,
            &mut general_b,
            &mut general_workspace,
        )
        .unwrap();

    assert!(
        table_plan
            .spin1_scalar_legendre_table
            .get()
            .is_some_and(Option::is_some)
    );
    for (table, general) in [(&table_e, &general_e), (&table_b, &general_b)] {
        for (&actual, &expected) in table.iter().zip(general) {
            assert!(
                (actual - expected).norm() <= 3e-10 * actual.norm().max(expected.norm()).max(1.0)
            );
        }
    }

    let next_plan = make_plan(1, 225, 225, 226, 453);
    assert!(!one_thread.install(|| next_plan.allow_spin1_analysis_table()));
}

#[test]
fn spin1_table_synthesis_skips_table_build_for_extreme_finite_coefficients() {
    let plan = make_plan(1, 127, 127, 128, 257);
    let recurrence_plan = make_plan(1, 127, 127, 128, 257);
    assert!(
        recurrence_plan
            .spin1_scalar_legendre_table
            .set(None)
            .is_ok()
    );
    let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let b = e.clone();
    let index = plan.harmonics().index(127, 1).unwrap();
    e[index] = Complex64::new(1.0e150, -0.75e150);

    let mut table_workspace = plan.workspace().unwrap();
    let mut recurrence_workspace = recurrence_plan.workspace().unwrap();
    let mut table_first = vec![0.0; plan.sample_count];
    let mut table_second = vec![0.0; plan.sample_count];
    let mut recurrence_first = vec![0.0; plan.sample_count];
    let mut recurrence_second = vec![0.0; plan.sample_count];
    #[cfg(feature = "parallel")]
    let one_thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();

    #[cfg(feature = "parallel")]
    {
        one_thread_pool
            .install(|| {
                plan.synthesis_spin_into(
                    &e,
                    &b,
                    &mut table_first,
                    &mut table_second,
                    &mut table_workspace,
                )
            })
            .unwrap();
        one_thread_pool
            .install(|| {
                recurrence_plan.synthesis_spin_into(
                    &e,
                    &b,
                    &mut recurrence_first,
                    &mut recurrence_second,
                    &mut recurrence_workspace,
                )
            })
            .unwrap();
    }
    #[cfg(not(feature = "parallel"))]
    {
        plan.synthesis_spin_into(
            &e,
            &b,
            &mut table_first,
            &mut table_second,
            &mut table_workspace,
        )
        .unwrap();
        recurrence_plan
            .synthesis_spin_into(
                &e,
                &b,
                &mut recurrence_first,
                &mut recurrence_second,
                &mut recurrence_workspace,
            )
            .unwrap();
    }
    assert!(plan.spin1_scalar_legendre_table.get().is_none());
    assert!(table_first.iter().all(|value| value.is_finite()));
    assert!(table_second.iter().all(|value| value.is_finite()));
    assert_eq!(table_first, recurrence_first);
    assert_eq!(table_second, recurrence_second);
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_fine_synthesis_groups_preserve_t720_adjoint_identities() {
    let plan = make_plan(1, 720, 720, 721, 1443);
    let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut b = e.clone();
    for (l, m, e_value, b_value) in [
        (1, 0, Complex64::new(0.31, 0.0), Complex64::new(-0.17, 0.0)),
        (
            91,
            27,
            Complex64::new(-0.23, 0.19),
            Complex64::new(0.13, 0.29),
        ),
        (
            720,
            720,
            Complex64::new(0.41, -0.11),
            Complex64::new(-0.37, 0.21),
        ),
    ] {
        let index = plan.harmonics.index(l, m).unwrap();
        e[index] = e_value;
        b[index] = b_value;
    }
    let (input_first, input_second) = arbitrary_maps(&plan);
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut workspace = eight_threads.install(|| plan.workspace()).unwrap();
    assert_eq!(workspace.ring_group_size, 64);
    assert_eq!(workspace.recurrence_workers.len(), 23);

    let mut synthesized_first = vec![0.0; plan.sample_count];
    let mut synthesized_second = vec![0.0; plan.sample_count];
    eight_threads
        .install(|| {
            plan.synthesis_spin_into(
                &e,
                &b,
                &mut synthesized_first,
                &mut synthesized_second,
                &mut workspace,
            )
        })
        .unwrap();
    assert_eq!(workspace.recurrence_workers.len(), 23);

    let mut synthesis_adjoint_e = vec![Complex64::new(0.0, 0.0); e.len()];
    let mut synthesis_adjoint_b = vec![Complex64::new(0.0, 0.0); b.len()];
    eight_threads
        .install(|| {
            plan.adjoint_synthesis_spin_into(
                &input_first,
                &input_second,
                &mut synthesis_adjoint_e,
                &mut synthesis_adjoint_b,
                &mut workspace,
            )
        })
        .unwrap();
    assert_close(
        spin_map_inner_product(
            &synthesized_first,
            &synthesized_second,
            &input_first,
            &input_second,
        ),
        spin_harmonic_inner_product(
            plan.harmonics(),
            &e,
            &b,
            &synthesis_adjoint_e,
            &synthesis_adjoint_b,
        ),
        2e-9,
    );

    let mut analyzed_e = vec![Complex64::new(0.0, 0.0); e.len()];
    let mut analyzed_b = vec![Complex64::new(0.0, 0.0); b.len()];
    eight_threads
        .install(|| {
            plan.analysis_spin_into(
                &input_first,
                &input_second,
                &mut analyzed_e,
                &mut analyzed_b,
                &mut workspace,
            )
        })
        .unwrap();
    let mut analysis_adjoint_first = vec![0.0; plan.sample_count];
    let mut analysis_adjoint_second = vec![0.0; plan.sample_count];
    eight_threads
        .install(|| {
            plan.adjoint_analysis_spin_into(
                &e,
                &b,
                &mut analysis_adjoint_first,
                &mut analysis_adjoint_second,
                &mut workspace,
            )
        })
        .unwrap();
    assert_close(
        spin_harmonic_inner_product(plan.harmonics(), &analyzed_e, &analyzed_b, &e, &b),
        spin_map_inner_product(
            &input_first,
            &input_second,
            &analysis_adjoint_first,
            &analysis_adjoint_second,
        ),
        2e-9,
    );
}

#[test]
fn spin1_scalar_table_analysis_preserves_small_reflected_differences() {
    let plan = make_plan(1, 72, 72, 73, 147);
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut workspace, 1);
    let table = crate::legendre::ScalarLegendreTable::build_if_beneficial(
        &plan.grid,
        &plan.harmonics,
        &plan.sectoral_norms,
    )
    .unwrap();

    let north_ring = 12;
    let south_ring = plan.grid.ntheta() - 1 - north_ring;
    let spectrum_len = plan.fft.spectrum_len();
    let phase = longitude_phase::<f64>(0, plan.grid.phi0(), true);
    let north_scale = plan.grid.ring_weights()[north_ring] / workspace.theta_sin[north_ring];
    let south_scale = plan.grid.ring_weights()[south_ring] / workspace.theta_sin[south_ring];
    let south_spectrum = 1.0 / south_scale;
    let south_weighted = (Complex64::new(south_spectrum, 0.0) * phase * south_scale).re;
    let mut north_spectrum = 1.0 / north_scale;
    let mut reflected_values = None;
    for _ in 0..128 {
        north_spectrum = f64::from_bits(north_spectrum.to_bits() + 1);
        let north_weighted = (Complex64::new(north_spectrum, 0.0) * phase * north_scale).re;
        if north_weighted > south_weighted
            && north_weighted + south_weighted == 2.0 * south_weighted
        {
            reflected_values = Some((north_spectrum, north_weighted));
            break;
        }
    }
    let (north_spectrum, north_weighted) =
        reflected_values.expect("adjacent north values expose parity cancellation");
    let mut q_spectra = vec![Complex64::new(0.0, 0.0); plan.grid.ntheta() * spectrum_len];
    let u_spectra = q_spectra.clone();
    q_spectra[north_ring * spectrum_len] = Complex64::new(north_spectrum, 0.0);
    q_spectra[south_ring * spectrum_len] = Complex64::new(south_spectrum, 0.0);
    let mut e_coefficients = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b_coefficients = e_coefficients.clone();
    let used_table = plan
        .analyze_spin_scalar_table_order(
            &q_spectra,
            &u_spectra,
            spectrum_len,
            plan.grid.nphi() as f64,
            phase,
            plan.grid.ring_weights(),
            false,
            0,
            &workspace.theta_sin,
            &workspace.ring_order_limits,
            pulp::Arch::new(),
            &mut e_coefficients,
            Some(&mut b_coefficients),
            &table,
            &mut workspace.recurrence_workers[0],
        )
        .unwrap();

    assert!(used_table);
    let expected = table.degree_lanes(0, 1)[north_ring] * (north_weighted - south_weighted);
    let actual = workspace.recurrence_workers[0].fast_scalar_first[1].re;
    assert!(expected != 0.0);
    assert!(
        (actual - expected).abs() <= expected.abs() * 1.0e-10,
        "odd reflected contribution lost: actual={actual:.3e}, expected={expected:.3e}"
    );
}

#[test]
fn spin1_scalar_table_analysis_falls_back_for_extreme_finite_inputs() {
    let table_plan = make_plan(1, 72, 72, 73, 147);
    table_plan.spin1_scalar_legendre_table.get_or_init(|| {
        crate::legendre::ScalarLegendreTable::build_if_beneficial(
            &table_plan.grid,
            &table_plan.harmonics,
            &table_plan.sectoral_norms,
        )
    });
    let mut general_plan = make_plan(1, 72, 72, 73, 147);
    general_plan.spin_fast_coefficient_count = 0;

    let mut first = vec![0.0; table_plan.sample_count];
    let mut second = vec![0.0; table_plan.sample_count];
    for ring in 0..table_plan.grid.ntheta() {
        for longitude in 0..table_plan.grid.nphi() {
            let theta = table_plan.grid.theta()[ring];
            let phi = table_plan.grid.longitude(longitude).unwrap();
            let index = ring * table_plan.grid.nphi() + longitude;
            first[index] = 1.0e150 * (theta * 0.7 + phi * 1.3).sin();
            second[index] = 1.0e150 * (theta * 1.1 - phi * 0.9).cos();
        }
    }

    let mut table_e = vec![Complex64::new(0.0, 0.0); table_plan.harmonics().nalm()];
    let mut table_b = table_e.clone();
    let mut general_e = table_e.clone();
    let mut general_b = table_e.clone();
    let mut table_workspace = table_plan.workspace().unwrap();
    let mut general_workspace = general_plan.workspace().unwrap();
    table_plan
        .analysis_spin_into(
            &first,
            &second,
            &mut table_e,
            &mut table_b,
            &mut table_workspace,
        )
        .unwrap();
    general_plan
        .analysis_spin_into(
            &first,
            &second,
            &mut general_e,
            &mut general_b,
            &mut general_workspace,
        )
        .unwrap();

    assert!(
        table_plan
            .spin1_scalar_legendre_table
            .get()
            .is_some_and(Option::is_some)
    );
    let mut maximum_scale: f64 = 1.0;
    let mut maximum_error: f64 = 0.0;
    for (actual, expected) in [(&table_e, &general_e), (&table_b, &general_b)] {
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert!(actual.re.is_finite() && actual.im.is_finite());
            assert!(expected.re.is_finite() && expected.im.is_finite());
            maximum_scale = maximum_scale
                .max(actual.re.abs())
                .max(actual.im.abs())
                .max(expected.re.abs())
                .max(expected.im.abs());
            maximum_error = maximum_error
                .max((actual.re - expected.re).abs())
                .max((actual.im - expected.im).abs());
        }
    }
    assert!(
        maximum_error <= 3.0e-10 * maximum_scale,
        "extreme spin-1 table fallback error={maximum_error:.3e}, scale={maximum_scale:.3e}"
    );

    let unsafe_first_call_plan = make_plan(1, 72, 72, 73, 147);
    let mut unsafe_first_call_workspace = unsafe_first_call_plan.workspace().unwrap();
    let mut unsafe_first_call_e =
        vec![Complex64::new(0.0, 0.0); unsafe_first_call_plan.harmonics().nalm()];
    let mut unsafe_first_call_b = unsafe_first_call_e.clone();
    unsafe_first_call_plan
        .analysis_spin_into(
            &first,
            &second,
            &mut unsafe_first_call_e,
            &mut unsafe_first_call_b,
            &mut unsafe_first_call_workspace,
        )
        .unwrap();
    assert!(
        unsafe_first_call_plan
            .spin1_scalar_legendre_table
            .get()
            .is_none()
    );
    let mut unsafe_error: f64 = 0.0;
    let mut unsafe_scale: f64 = 1.0;
    for (actual, expected) in [
        (&unsafe_first_call_e, &general_e),
        (&unsafe_first_call_b, &general_b),
    ] {
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert!(actual.re.is_finite() && actual.im.is_finite());
            unsafe_error = unsafe_error
                .max((actual.re - expected.re).abs())
                .max((actual.im - expected.im).abs());
            unsafe_scale = unsafe_scale
                .max(actual.re.abs())
                .max(actual.im.abs())
                .max(expected.re.abs())
                .max(expected.im.abs());
        }
    }
    assert!(unsafe_error <= 3.0e-10 * unsafe_scale);
}

#[test]
fn spin2_scalar_fast_analysis_honors_the_polar_order_cutoff() {
    let lmax = 511;
    let ntheta = lmax + 1;
    let nphi = 2 * lmax + 3;
    let m = 450;
    let plan = make_plan(2, lmax, lmax, ntheta, nphi);
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_spin_latitudes(&mut workspace, 2);
    let ring = (0..ntheta)
        .find(|&ring| {
            workspace.theta_sin[ring] > 0.4
                && workspace.theta_sin[ring] < 0.6
                && workspace.ring_order_limits[ring] < m
        })
        .expect("a mid-latitude ring is below the high-order cutoff");
    let spectrum_len = plan.fft.spectrum_len();
    let mut q_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
    let u_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
    q_spectra[ring * spectrum_len + m] = Complex64::new(1.0, 0.25);
    let coefficient_count = lmax - m + 1;
    let mut e_coefficients = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut b_coefficients = vec![Complex64::new(0.0, 0.0); coefficient_count];

    plan.analyze_spin2_scalar_fast_order(
        &q_spectra,
        &u_spectra,
        spectrum_len,
        nphi as f64,
        longitude_phase::<f64>(m, plan.grid.phi0(), true),
        plan.grid.ring_weights(),
        true,
        m,
        0,
        ntheta,
        &workspace.theta_sin,
        &workspace.theta_cos,
        &workspace.ring_order_limits,
        pulp::Arch::new(),
        &mut e_coefficients,
        Some(&mut b_coefficients),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();
    assert!(
        e_coefficients
            .iter()
            .chain(&b_coefficients)
            .all(|value| *value == Complex64::new(0.0, 0.0))
    );

    let all_active_limits = vec![lmax; ntheta];
    e_coefficients.fill(Complex64::new(0.0, 0.0));
    b_coefficients.fill(Complex64::new(0.0, 0.0));
    plan.analyze_spin2_scalar_fast_order(
        &q_spectra,
        &u_spectra,
        spectrum_len,
        nphi as f64,
        longitude_phase::<f64>(m, plan.grid.phi0(), true),
        plan.grid.ring_weights(),
        true,
        m,
        0,
        ntheta,
        &workspace.theta_sin,
        &workspace.theta_cos,
        &all_active_limits,
        pulp::Arch::new(),
        &mut e_coefficients,
        Some(&mut b_coefficients),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();
    assert!(
        e_coefficients
            .iter()
            .chain(&b_coefficients)
            .any(|value| *value != Complex64::new(0.0, 0.0))
    );
}

#[test]
fn spin2_scalar_fast_analysis_uses_scaled_path_for_unsafe_channel_products() {
    let ntheta = 512;
    let nphi = 3;
    let fast_plan = make_plan(2, 511, 0, ntheta, nphi);
    let mut general_plan = make_plan(2, 511, 0, ntheta, nphi);
    general_plan.spin_fast_coefficient_count = 0;
    let q_map = vec![1.0e300; fast_plan.sample_count];
    let u_map = vec![1.0; fast_plan.sample_count];
    let mut fast_e = vec![Complex64::new(0.0, 0.0); fast_plan.harmonics.nalm()];
    let mut fast_b = vec![Complex64::new(0.0, 0.0); fast_plan.harmonics.nalm()];
    let mut general_e = vec![Complex64::new(0.0, 0.0); general_plan.harmonics.nalm()];
    let mut general_b = vec![Complex64::new(0.0, 0.0); general_plan.harmonics.nalm()];
    let mut fast_workspace = fast_plan.workspace().unwrap();
    let mut general_workspace = general_plan.workspace().unwrap();

    fast_plan
        .analysis_spin_into(
            &q_map,
            &u_map,
            &mut fast_e,
            &mut fast_b,
            &mut fast_workspace,
        )
        .unwrap();
    general_plan
        .analysis_spin_into(
            &q_map,
            &u_map,
            &mut general_e,
            &mut general_b,
            &mut general_workspace,
        )
        .unwrap();

    for (actual, expected) in [(&fast_e, &general_e), (&fast_b, &general_b)] {
        let mut error = 0.0_f64;
        let mut scale = 1.0_f64;
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert!(actual.re.is_finite() && actual.im.is_finite());
            error = error
                .max((actual.re - expected.re).abs())
                .max((actual.im - expected.im).abs());
            scale = scale
                .max(actual.re.abs())
                .max(actual.im.abs())
                .max(expected.re.abs())
                .max(expected.im.abs());
        }
        assert!(
            error <= 5.0e-10 * scale,
            "error={error:.3e}, scale={scale:.3e}"
        );
    }
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "These decimal fixtures retain the printed DUCC reference values."
)]
fn spin_two_gl_matches_ducc_0410_and_weighted_analysis() {
    // Static outputs from ducc0==0.41.0, GL 3x7, phi0=0.37, one thread.
    let plan = make_plan(2, 2, 2, 3, 7);
    let (e, b) = coefficients(&plan);
    let mut workspace = plan.workspace().unwrap();
    let mut first = vec![0.0; plan.sample_count];
    let mut second = vec![0.0; plan.sample_count];
    plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
        .unwrap();
    for (index, expected) in [
        (0, -0.103_404_620_030_628_45),
        (2, 0.048_002_344_434_381_797),
        (10, -0.070_686_531_243_965_053),
        (20, 0.000_284_063_513_471_682_9),
    ] {
        assert_close(first[index], expected, 5e-14);
    }
    for (index, expected) in [
        (0, 0.196_243_499_903_941_38),
        (2, -0.079_139_202_179_877_077),
        (10, 0.063_750_908_470_770_57),
        (20, 0.075_170_514_907_193_37),
    ] {
        assert_close(second[index], expected, 5e-14);
    }

    let (input_first, input_second) = arbitrary_maps(&plan);
    let mut recovered_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut recovered_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    plan.analysis_spin_into(
        &input_first,
        &input_second,
        &mut recovered_e,
        &mut recovered_b,
        &mut workspace,
    )
    .unwrap();
    let expected_e = [
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(-2.550_847_468_393_931_3, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.122_163_424_850_977_95, -0.080_290_382_013_937_858),
        Complex64::new(0.083_928_217_095_939_02, -0.152_913_590_740_669_4),
    ];
    let expected_b = [
        Complex64::new(0.0, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(-2.819_769_059_231_437_5, 0.0),
        Complex64::new(0.0, 0.0),
        Complex64::new(0.347_681_620_237_764_94, 0.249_694_162_014_288_01),
        Complex64::new(0.026_574_652_518_373_161, 0.058_868_006_368_103_853),
    ];
    for ((&actual_e, &actual_b), (&expected_e, &expected_b)) in recovered_e
        .iter()
        .zip(&recovered_b)
        .zip(expected_e.iter().zip(&expected_b))
    {
        assert_complex_close(actual_e, expected_e, 8e-13);
        assert_complex_close(actual_b, expected_b, 8e-13);
    }
}

#[test]
fn spin_adjoint_operators_match_ducc_and_inner_products() {
    // References: ducc0 0.41.0, GL 3x7, phi0=0.37, one thread.
    for spin in [1, 2] {
        let plan = make_plan(spin, 2, 2, 3, 7);
        let (e, b) = coefficients(&plan);
        let (input_first, input_second) = arbitrary_maps(&plan);
        let mut workspace = plan.workspace().unwrap();

        let mut synthesized_first = vec![0.0; plan.sample_count];
        let mut synthesized_second = vec![0.0; plan.sample_count];
        plan.synthesis_spin_into(
            &e,
            &b,
            &mut synthesized_first,
            &mut synthesized_second,
            &mut workspace,
        )
        .unwrap();
        let mut adjoint_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut adjoint_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.adjoint_synthesis_spin_into(
            &input_first,
            &input_second,
            &mut adjoint_e,
            &mut adjoint_b,
            &mut workspace,
        )
        .unwrap();
        let (expected_e, expected_b) = if spin == 1 {
            (
                [
                    Complex64::new(0.0, 0.0),
                    Complex64::new(-4.304_852_637_841_806, 0.0),
                    Complex64::new(0.594_467_893_478_704_9, 0.0),
                    Complex64::new(0.191_987_240_026_905_5, -0.155_484_991_948_925_9),
                    Complex64::new(0.005_566_252_562_547, 0.159_878_214_894_219_5),
                    Complex64::new(-0.111_026_790_541_622_9, 0.034_591_870_587_753_7),
                ],
                [
                    Complex64::new(0.0, 0.0),
                    Complex64::new(-4.755_618_446_083_105, 0.0),
                    Complex64::new(-0.005_348_600_090_854_1, 0.0),
                    Complex64::new(0.560_677_720_510_292_4, 0.430_912_220_287_711_6),
                    Complex64::new(0.074_918_874_664_593_5, -0.076_897_461_383_145_1),
                    Complex64::new(-0.292_729_976_427_507_2, -0.142_364_588_620_335_7),
                ],
            )
        } else {
            (
                [
                    Complex64::new(0.0, 0.0),
                    Complex64::new(0.0, 0.0),
                    Complex64::new(-3.830_844_164_316_75, 0.0),
                    Complex64::new(0.0, 0.0),
                    Complex64::new(0.190_270_797_105_077_7, -0.139_890_518_676_951_7),
                    Complex64::new(0.153_808_087_304_361_4, -0.266_731_025_579_472_3),
                ],
                [
                    Complex64::new(0.0, 0.0),
                    Complex64::new(0.0, 0.0),
                    Complex64::new(-4.233_362_144_504_837, 0.0),
                    Complex64::new(0.0, 0.0),
                    Complex64::new(0.548_745_566_635_568_1, 0.408_396_348_205_166_3),
                    Complex64::new(0.052_938_390_213_741_3, 0.103_326_719_720_155_4),
                ],
            )
        };
        for ((&actual_e, &actual_b), (&expected_e, &expected_b)) in adjoint_e
            .iter()
            .zip(&adjoint_b)
            .zip(expected_e.iter().zip(&expected_b))
        {
            assert_complex_close(actual_e, expected_e, 4e-13);
            assert_complex_close(actual_b, expected_b, 4e-13);
        }
        assert_close(
            spin_map_inner_product(
                &synthesized_first,
                &synthesized_second,
                &input_first,
                &input_second,
            ),
            spin_harmonic_inner_product(plan.harmonics(), &e, &b, &adjoint_e, &adjoint_b),
            2e-12,
        );

        let mut analyzed_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut analyzed_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.analysis_spin_into(
            &input_first,
            &input_second,
            &mut analyzed_e,
            &mut analyzed_b,
            &mut workspace,
        )
        .unwrap();
        let mut analysis_adjoint_first = vec![0.0; plan.sample_count];
        let mut analysis_adjoint_second = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_spin_into(
            &e,
            &b,
            &mut analysis_adjoint_first,
            &mut analysis_adjoint_second,
            &mut workspace,
        )
        .unwrap();
        let (expected_first, expected_second) = if spin == 1 {
            (
                [
                    (0, -0.037_382_873_397_486_7),
                    (3, 0.065_442_391_714_093_7),
                    (10, -0.062_561_838_187_364_4),
                    (20, 0.036_614_770_103_661_2),
                ],
                [
                    (0, 0.026_476_346_722_301_8),
                    (3, -0.009_328_231_563_205_1),
                    (10, 0.085_436_105_922_155_8),
                    (20, 0.025_908_640_899_011_7),
                ],
            )
        } else {
            (
                [
                    (0, -0.051_564_316_608_804_2),
                    (3, -0.037_419_391_301_285_5),
                    (10, -0.056_398_295_178_104_4),
                    (20, 0.000_141_652_674_139),
                ],
                [
                    (0, 0.097_859_863_113_170_1),
                    (3, 0.011_017_925_793_126_9),
                    (10, 0.050_864_605_894_971),
                    (20, 0.037_484_942_444_285_8),
                ],
            )
        };
        for (index, expected) in expected_first {
            assert_close(analysis_adjoint_first[index], expected, 2e-14);
        }
        for (index, expected) in expected_second {
            assert_close(analysis_adjoint_second[index], expected, 2e-14);
        }
        assert_close(
            spin_harmonic_inner_product(plan.harmonics(), &analyzed_e, &analyzed_b, &e, &b),
            spin_map_inner_product(
                &input_first,
                &input_second,
                &analysis_adjoint_first,
                &analysis_adjoint_second,
            ),
            2e-12,
        );

        let mut gradient_first = vec![0.0; plan.sample_count];
        let mut gradient_second = vec![0.0; plan.sample_count];
        plan.synthesis_gradient_into(
            &e,
            &mut gradient_first,
            &mut gradient_second,
            &mut workspace,
        )
        .unwrap();
        let mut gradient_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.adjoint_synthesis_gradient_into(
            &input_first,
            &input_second,
            &mut gradient_adjoint,
            &mut workspace,
        )
        .unwrap();
        for (&actual, &expected) in gradient_adjoint.iter().zip(&expected_e) {
            assert_complex_close(actual, expected, 4e-13);
        }
        assert_close(
            spin_map_inner_product(
                &gradient_first,
                &gradient_second,
                &input_first,
                &input_second,
            ),
            harmonic_inner_product(plan.harmonics(), &e, &gradient_adjoint),
            2e-12,
        );
    }
}

#[test]
fn spin_synthesis_adjoints_accept_even_longitude_nyquist_order() {
    // References: ducc0 0.41.0, GL 3x4, phi0=0.37, one thread.
    for (spin, expected_e, expected_b) in [
        (
            1,
            [
                Complex64::new(0.0, 0.0),
                Complex64::new(-2.029_575_686_464_789, 0.0),
                Complex64::new(0.452_936_406_865_718_45, 0.0),
                Complex64::new(0.027_248_283_458_920_986, -0.021_338_495_900_768_042),
                Complex64::new(-0.016_750_784_124_226_37, 0.096_577_982_026_280_07),
                Complex64::new(-0.015_443_713_201_206_216, -0.005_395_642_560_412_243),
            ],
            [
                Complex64::new(0.0, 0.0),
                Complex64::new(-2.890_005_846_365_780_5, 0.0),
                Complex64::new(-0.116_673_029_539_538_55, 0.0),
                Complex64::new(0.301_655_848_666_466_05, 0.211_822_691_591_908_97),
                Complex64::new(0.016_552_871_349_277_794, -0.005_261_494_054_254_939),
                Complex64::new(-0.154_574_133_111_514_08, -0.192_640_812_963_896_73),
            ],
        ),
        (
            2,
            [
                Complex64::new(0.0, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(-1.805_872_850_620_483_7, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(0.026_440_758_886_784_245, -0.018_540_320_152_576_37),
                Complex64::new(0.185_024_900_446_692_18, -0.135_227_815_896_944_1),
            ],
            [
                Complex64::new(0.0, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(-2.572_767_322_741_793, 0.0),
                Complex64::new(0.0, 0.0),
                Complex64::new(0.296_268_768_559_663_16, 0.202_773_481_944_890_12),
                Complex64::new(-0.001_495_524_759_687_719_4, 0.017_994_398_447_773_54),
            ],
        ),
    ] {
        let plan = make_plan(spin, 2, 2, 3, 4);
        let (e, b) = coefficients(&plan);
        let (input_first, input_second) = arbitrary_maps(&plan);
        let mut workspace = plan.workspace().unwrap();
        let mut synthesized_first = vec![0.0; plan.sample_count];
        let mut synthesized_second = vec![0.0; plan.sample_count];
        plan.synthesis_spin_into(
            &e,
            &b,
            &mut synthesized_first,
            &mut synthesized_second,
            &mut workspace,
        )
        .unwrap();
        let mut adjoint_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut adjoint_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.adjoint_synthesis_spin_into(
            &input_first,
            &input_second,
            &mut adjoint_e,
            &mut adjoint_b,
            &mut workspace,
        )
        .unwrap();
        for ((&actual_e, &actual_b), (&reference_e, &reference_b)) in adjoint_e
            .iter()
            .zip(&adjoint_b)
            .zip(expected_e.iter().zip(&expected_b))
        {
            assert_complex_close(actual_e, reference_e, 6e-13);
            assert_complex_close(actual_b, reference_b, 6e-13);
        }
        assert!(adjoint_e[5].im.abs() > 1e-4);
        assert_close(
            spin_map_inner_product(
                &synthesized_first,
                &synthesized_second,
                &input_first,
                &input_second,
            ),
            spin_harmonic_inner_product(plan.harmonics(), &e, &b, &adjoint_e, &adjoint_b),
            3e-12,
        );

        let mut gradient_first = vec![0.0; plan.sample_count];
        let mut gradient_second = vec![0.0; plan.sample_count];
        plan.synthesis_gradient_into(
            &e,
            &mut gradient_first,
            &mut gradient_second,
            &mut workspace,
        )
        .unwrap();
        let mut gradient_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.adjoint_synthesis_gradient_into(
            &input_first,
            &input_second,
            &mut gradient_adjoint,
            &mut workspace,
        )
        .unwrap();
        for (&actual, &expected) in gradient_adjoint.iter().zip(&adjoint_e) {
            assert_complex_close(actual, expected, 2e-14);
        }
        assert_close(
            spin_map_inner_product(
                &gradient_first,
                &gradient_second,
                &input_first,
                &input_second,
            ),
            harmonic_inner_product(plan.harmonics(), &e, &gradient_adjoint),
            3e-12,
        );
    }
}

#[test]
fn derivative_adjoint_matches_ducc_and_inner_product() {
    // Reference: ducc0 0.41.0, GL 3x7, phi0=0.37, one thread, DERIV1 mode.
    let plan = make_plan(0, 2, 2, 3, 7);
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    for (l, m, value) in [
        (0, 0, Complex64::new(0.7, 0.0)),
        (1, 0, Complex64::new(-0.2, 0.0)),
        (2, 0, Complex64::new(0.1, 0.0)),
        (1, 1, Complex64::new(0.4, 0.3)),
        (2, 1, Complex64::new(-0.2, 0.15)),
        (2, 2, Complex64::new(0.1, -0.6)),
    ] {
        alm[plan.harmonics.index(l, m).unwrap()] = value;
    }
    let (input_first, input_second) = arbitrary_maps(&plan);
    let mut workspace = plan.workspace().unwrap();
    let mut derivative_first = vec![0.0; plan.sample_count];
    let mut derivative_second = vec![0.0; plan.sample_count];
    plan.synthesis_deriv1_into(
        &alm,
        &mut derivative_first,
        &mut derivative_second,
        &mut workspace,
    )
    .unwrap();
    let mut adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    plan.adjoint_synthesis_deriv1_into(&input_first, &input_second, &mut adjoint, &mut workspace)
        .unwrap();
    for (&actual, expected) in adjoint.iter().zip([
        Complex64::new(0.0, 0.0),
        Complex64::new(-6.087_980_984_453_476, 0.0),
        Complex64::new(1.456_143_007_490_010_4, 0.0),
        Complex64::new(0.271_510_958_648_628_5, -0.219_888_984_359_642_5),
        Complex64::new(0.013_634_478_557_699_4, 0.391_620_047_477_875_4),
        Complex64::new(-0.271_958_984_605_841_6, 0.084_732_432_188_385_7),
    ]) {
        assert!((actual - expected).norm() < 5e-13);
    }
    assert_close(
        spin_map_inner_product(
            &derivative_first,
            &derivative_second,
            &input_first,
            &input_second,
        ),
        harmonic_inner_product(plan.harmonics(), &alm, &adjoint),
        2e-12,
    );
}

#[test]
fn scalar_legendre_table_is_materialized_for_standard_analysis_not_derivatives() {
    let derivative_plan = make_plan(0, 127, 127, 128, 257);
    assert!(derivative_plan.scalar_legendre_table.get().is_none());
    let alm = vec![Complex64::new(0.0, 0.0); derivative_plan.harmonics().nalm()];
    let mut first = vec![0.0; derivative_plan.sample_count];
    let mut second = vec![0.0; derivative_plan.sample_count];
    let mut derivative_workspace = derivative_plan.workspace().unwrap();
    derivative_plan
        .synthesis_deriv1_into(&alm, &mut first, &mut second, &mut derivative_workspace)
        .unwrap();
    assert!(derivative_plan.scalar_legendre_table.get().is_none());

    // Single-threaded scalar analysis retains the compact table at T127.
    let standard_plan = make_plan(0, 127, 127, 128, 257);
    assert!(standard_plan.scalar_legendre_table.get().is_none());
    let standard_map = vec![0.0; standard_plan.sample_count];
    let mut standard_alm = vec![Complex64::new(0.0, 0.0); standard_plan.harmonics().nalm()];
    let mut standard_workspace = standard_plan.workspace().unwrap();
    in_single_thread_pool(|| {
        standard_plan.analysis_into(&standard_map, &mut standard_alm, &mut standard_workspace)
    })
    .unwrap();
    assert!(
        standard_plan
            .scalar_legendre_table
            .get()
            .is_some_and(Option::is_some)
    );
}

#[test]
fn deriv1_scalar_conversion_matches_general_recurrence_and_analytic_fields() {
    for lmax in [14, 15, 31, 47, 63, 190, 191, 208, 224] {
        for grid_kind in [
            GridKind::GaussLegendre,
            GridKind::ClenshawCurtis,
            GridKind::Fejer1,
        ] {
            let ntheta = lmax + usize::from(grid_kind == GridKind::ClenshawCurtis) + 1;
            for nphi in [2 * lmax, 2 * lmax + 1] {
                let make_plan = || {
                    ShtPlan::new(
                        GridSpec::new(grid_kind, ntheta, nphi, 0.37).unwrap(),
                        HarmonicSpec::scalar(lmax, lmax).unwrap(),
                    )
                    .unwrap()
                };
                let fast_plan = make_plan();
                let mut general_plan = make_plan();
                general_plan.spin_fast_coefficient_count = 0;
                let mut alm = vec![Complex64::new(0.0, 0.0); fast_plan.harmonics.nalm()];
                for m in 0..=lmax {
                    for l in m..=lmax {
                        let index = fast_plan.harmonics.index(l, m).unwrap();
                        let re = ((l * 7 + m * 11) % 19 + 1) as f64 * 1e-4;
                        let im = if m == 0 {
                            0.0
                        } else {
                            ((l * 13 + m * 5) % 23 + 1) as f64 * 1e-4
                        };
                        alm[index] = Complex64::new(re, im);
                    }
                }

                let mut fast_workspace = fast_plan.workspace().unwrap();
                let mut general_workspace = general_plan.workspace().unwrap();
                let mut fast_theta = vec![0.0; fast_plan.sample_count];
                let mut fast_phi = vec![0.0; fast_plan.sample_count];
                let mut general_theta = vec![0.0; general_plan.sample_count];
                let mut general_phi = vec![0.0; general_plan.sample_count];
                fast_plan
                    .synthesis_deriv1_into(
                        &alm,
                        &mut fast_theta,
                        &mut fast_phi,
                        &mut fast_workspace,
                    )
                    .unwrap();
                general_plan
                    .synthesis_deriv1_into(
                        &alm,
                        &mut general_theta,
                        &mut general_phi,
                        &mut general_workspace,
                    )
                    .unwrap();
                for (component, actual, expected) in [
                    ("theta", &fast_theta, &general_theta),
                    ("phi", &fast_phi, &general_phi),
                ] {
                    let error = actual
                        .iter()
                        .zip(expected)
                        .map(|(actual, expected)| (actual - expected).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    let reference = expected
                        .iter()
                        .map(|value| value * value)
                        .sum::<f64>()
                        .sqrt();
                    let relative_error = error / reference.max(1e-300);
                    assert!(
                        relative_error <= 2e-11,
                        "grid={grid_kind:?}, nphi={nphi}, component={component}, relative error={relative_error:.3e}"
                    );
                }

                let (input_theta, input_phi) = arbitrary_maps(&fast_plan);
                let mut adjoint = vec![Complex64::new(0.0, 0.0); fast_plan.harmonics.nalm()];
                fast_plan
                    .adjoint_synthesis_deriv1_into(
                        &input_theta,
                        &input_phi,
                        &mut adjoint,
                        &mut fast_workspace,
                    )
                    .unwrap();
                let map_product =
                    spin_map_inner_product(&fast_theta, &fast_phi, &input_theta, &input_phi);
                let harmonic_product =
                    harmonic_inner_product(fast_plan.harmonics(), &alm, &adjoint);
                assert!(
                    (map_product - harmonic_product).abs()
                        <= 2e-10 * map_product.abs().max(harmonic_product.abs()).max(1.0),
                    "grid={grid_kind:?}, nphi={nphi}, map={map_product:.17e}, harmonic={harmonic_product:.17e}"
                );

                for (l, m, expected_factor) in [
                    (1, 0, (3.0 / (4.0 * std::f64::consts::PI)).sqrt()),
                    (1, 1, (3.0 / (2.0 * std::f64::consts::PI)).sqrt()),
                ] {
                    alm.fill(Complex64::new(0.0, 0.0));
                    alm[fast_plan.harmonics.index(l, m).unwrap()] = Complex64::new(1.0, 0.0);
                    fast_plan
                        .synthesis_deriv1_into(
                            &alm,
                            &mut fast_theta,
                            &mut fast_phi,
                            &mut fast_workspace,
                        )
                        .unwrap();
                    for (ring, &theta) in fast_plan.grid.theta().iter().enumerate() {
                        for longitude in 0..nphi {
                            let phi = fast_plan.grid.phi0()
                                + 2.0 * std::f64::consts::PI * longitude as f64 / nphi as f64;
                            let index = ring * nphi + longitude;
                            let (expected_theta, expected_phi) = if m == 0 {
                                (-expected_factor * theta.sin(), 0.0)
                            } else {
                                (
                                    -expected_factor * theta.cos() * phi.cos(),
                                    expected_factor * phi.sin(),
                                )
                            };
                            assert_close(fast_theta[index], expected_theta, 2e-12);
                            assert_close(fast_phi[index], expected_phi, 2e-12);
                        }
                    }
                }

                alm.fill(Complex64::new(0.0, 0.0));
                alm[fast_plan.harmonics.index(0, 0).unwrap()] = Complex64::new(1.7, 0.0);
                fast_plan
                    .synthesis_deriv1_into(
                        &alm,
                        &mut fast_theta,
                        &mut fast_phi,
                        &mut fast_workspace,
                    )
                    .unwrap();
                assert!(fast_theta.iter().all(|&value| value == 0.0));
                assert!(fast_phi.iter().all(|&value| value == 0.0));
            }
        }
    }

    let lmax = 63;
    let fast_plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, lmax + 1, 1, 0.37).unwrap(),
        HarmonicSpec::scalar(lmax, 0).unwrap(),
    )
    .unwrap();
    let mut general_plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, lmax + 1, 1, 0.37).unwrap(),
        HarmonicSpec::scalar(lmax, 0).unwrap(),
    )
    .unwrap();
    general_plan.spin_fast_coefficient_count = 0;
    let mut alm = vec![Complex64::new(0.0, 0.0); fast_plan.harmonics.nalm()];
    alm[fast_plan.harmonics.index(1, 0).unwrap()] = Complex64::new(0.9 * f64::MAX, 0.0);
    let mut fast_workspace = fast_plan.workspace().unwrap();
    let mut general_workspace = general_plan.workspace().unwrap();
    let mut fast_theta = vec![0.0; fast_plan.sample_count];
    let mut fast_phi = vec![0.0; fast_plan.sample_count];
    let mut general_theta = vec![0.0; general_plan.sample_count];
    let mut general_phi = vec![0.0; general_plan.sample_count];
    fast_plan
        .synthesis_deriv1_into(&alm, &mut fast_theta, &mut fast_phi, &mut fast_workspace)
        .unwrap();
    general_plan
        .synthesis_deriv1_into(
            &alm,
            &mut general_theta,
            &mut general_phi,
            &mut general_workspace,
        )
        .unwrap();
    assert!(fast_theta.iter().all(|value| value.is_finite()));
    assert!(fast_phi.iter().all(|value| value.is_finite()));
    assert_eq!(fast_theta, general_theta);
    assert_eq!(fast_phi, general_phi);
}

#[test]
fn spin_gradient_and_derivative_modes_match_their_definitions() {
    for spin in [1, 2] {
        let plan = make_plan(spin, spin, spin, 5, 7);
        let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        e[plan.harmonics.index(spin, 0).unwrap()] = Complex64::new(1.0, 0.0);
        let b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut workspace = plan.workspace().unwrap();
        let mut standard_first = vec![0.0; plan.sample_count];
        let mut standard_second = vec![0.0; plan.sample_count];
        let mut gradient_first = vec![0.0; plan.sample_count];
        let mut gradient_second = vec![0.0; plan.sample_count];
        plan.synthesis_spin_into(
            &e,
            &b,
            &mut standard_first,
            &mut standard_second,
            &mut workspace,
        )
        .unwrap();
        plan.synthesis_gradient_into(
            &e,
            &mut gradient_first,
            &mut gradient_second,
            &mut workspace,
        )
        .unwrap();
        for (&actual, &expected) in gradient_first.iter().zip(&standard_first) {
            assert_close(actual, expected, 2e-14);
        }
        for (&actual, &expected) in gradient_second.iter().zip(&standard_second) {
            assert_close(actual, expected, 2e-14);
        }

        let expected_factor = if spin == 1 {
            (3.0 / (8.0 * std::f64::consts::PI)).sqrt()
        } else {
            (30.0 / (64.0 * std::f64::consts::PI)).sqrt()
        };
        for ring in 0..plan.grid.ntheta() {
            let theta = plan.grid.theta()[ring];
            let expected = -expected_factor * theta.sin().powi(spin as i32);
            assert_close(standard_first[ring * plan.grid.nphi()], expected, 3e-14);
            assert_close(standard_second[ring * plan.grid.nphi()], 0.0, 3e-14);
        }
    }

    let plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, 4, 7, 0.21).unwrap(),
        HarmonicSpec::scalar(1, 1).unwrap(),
    )
    .unwrap();
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    alm[plan.harmonics.index(1, 0).unwrap()] = Complex64::new(1.0, 0.0);
    let mut theta_derivative = vec![0.0; plan.sample_count];
    let mut phi_derivative = vec![0.0; plan.sample_count];
    plan.synthesis_deriv1_into(
        &alm,
        &mut theta_derivative,
        &mut phi_derivative,
        &mut plan.workspace().unwrap(),
    )
    .unwrap();
    let norm = (3.0 / (4.0 * std::f64::consts::PI)).sqrt();
    for ring in 0..plan.grid.ntheta() {
        let expected = -norm * plan.grid.theta()[ring].sin();
        assert_close(theta_derivative[ring * plan.grid.nphi()], expected, 3e-14);
        assert_close(phi_derivative[ring * plan.grid.nphi()], 0.0, 3e-14);
    }

    alm.fill(Complex64::new(0.0, 0.0));
    alm[plan.harmonics.index(1, 1).unwrap()] = Complex64::new(1.0, 0.0);
    plan.synthesis_deriv1_into(
        &alm,
        &mut theta_derivative,
        &mut phi_derivative,
        &mut plan.workspace().unwrap(),
    )
    .unwrap();
    let derivative_norm = (3.0 / (2.0 * std::f64::consts::PI)).sqrt();
    for ring in 0..plan.grid.ntheta() {
        let theta = plan.grid.theta()[ring];
        for longitude in 0..plan.grid.nphi() {
            let phi = plan.grid.phi0()
                + 2.0 * std::f64::consts::PI * longitude as f64 / plan.grid.nphi() as f64;
            let index = ring * plan.grid.nphi() + longitude;
            assert_close(
                theta_derivative[index],
                -derivative_norm * theta.cos() * phi.cos(),
                5e-14,
            );
            assert_close(phi_derivative[index], derivative_norm * phi.sin(), 5e-14);
        }
    }

    let constant_plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, 2, 3, 0.0).unwrap(),
        HarmonicSpec::scalar(0, 0).unwrap(),
    )
    .unwrap();
    let mut zero_derivatives = [1.0; 6];
    let mut zero_phi = [1.0; 6];
    constant_plan
        .synthesis_deriv1_into(
            &[Complex64::new(2.0, 0.0)],
            &mut zero_derivatives,
            &mut zero_phi,
            &mut constant_plan.workspace().unwrap(),
        )
        .unwrap();
    assert_eq!(zero_derivatives, [0.0; 6]);
    assert_eq!(zero_phi, [0.0; 6]);
}

#[test]
fn spin_analysis_accepts_minimum_odd_even_longitudes_and_rectangular_layouts() {
    for spin in [1, 2] {
        for nphi in [3, 4] {
            let plan = make_plan(spin, 3, 1, 4, nphi);
            let (e, b) = coefficients(&plan);
            let mut first = vec![0.0; plan.sample_count];
            let mut second = vec![0.0; plan.sample_count];
            let mut workspace = plan.workspace().unwrap();
            plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
                .unwrap();
            let mut recovered_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            let mut recovered_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            plan.analysis_spin_into(
                &first,
                &second,
                &mut recovered_e,
                &mut recovered_b,
                &mut workspace,
            )
            .unwrap();
            for (&actual, &expected) in recovered_e.iter().zip(&e) {
                assert_complex_close(actual, expected, 8e-13);
            }
            for (&actual, &expected) in recovered_b.iter().zip(&b) {
                assert_complex_close(actual, expected, 8e-13);
            }
        }
    }
}

#[test]
fn spin_adjoint_analysis_rejects_even_longitude_nyquist_order() {
    for spin in [1, 2] {
        let plan = make_plan(spin, 2, 2, 3, 4);
        let mut workspace = plan.workspace().unwrap();
        let e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut first = vec![0.0; plan.sample_count];
        let mut second = vec![0.0; plan.sample_count];
        assert_eq!(
            plan.adjoint_analysis_spin_into(&e, &b, &mut first, &mut second, &mut workspace,),
            Err(ShtError::AnalysisLongitudeUndersampled { mmax: 2, nphi: 4 })
        );
    }
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain printed DUCC 0.41.0 reference values."
)]
fn spin_three_and_four_match_ducc_analytic_fields_and_adjoint_identities() {
    // DUCC 0.41.0 references use phi0=0.37 and nthreads=1.
    for spin in [3, 4] {
        let lmax = spin;
        let nphi = 2 * lmax + 3;
        for kind in [
            GridKind::GaussLegendre,
            GridKind::ClenshawCurtis,
            GridKind::Fejer1,
        ] {
            let ntheta = lmax + usize::from(kind == GridKind::ClenshawCurtis) + 1;
            let plan = ShtPlan::new(
                GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
                HarmonicSpec::new(lmax, lmax, spin).unwrap(),
            )
            .unwrap();
            let m_zero_mode = plan.harmonics().index(spin, 0).unwrap();
            let sectoral_mode = plan.harmonics().index(spin, spin).unwrap();

            let mut analytic_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let analytic_b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            analytic_e[m_zero_mode] = Complex64::new(1.0, 0.0);
            let mut workspace = plan.workspace().unwrap();
            let mut first = vec![0.0; plan.sample_count];
            let mut second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(
                &analytic_e,
                &analytic_b,
                &mut first,
                &mut second,
                &mut workspace,
            )
            .unwrap();
            let mut ducc_synthesis_max_error = 0.0_f64;
            let mut analytic_synthesis_max_error = 0.0_f64;
            let mut analytic_synthesis_error_squares = 0.0_f64;
            let mut analytic_synthesis_sample_count = 0usize;

            // Static DUCC samples use rows [0, ntheta/2, ntheta-1], j=0.
            let expected_ducc = match (kind, spin) {
                (GridKind::GaussLegendre, 3) => [
                    -0.054817524242523459,
                    -0.34701775377308403,
                    -0.054817524242523459,
                ],
                (GridKind::ClenshawCurtis, 3) => [
                    -3.337790589062273e-45,
                    -0.41722382363278404,
                    -3.337790589062273e-45,
                ],
                (GridKind::Fejer1, 3) => [
                    -0.023382345886603487,
                    -0.32901457457089117,
                    -0.023382345886603487,
                ],
                (GridKind::GaussLegendre, 4) => [
                    -0.014153550227730075,
                    -0.44253269244498256,
                    -0.014153550227730075,
                ],
                (GridKind::ClenshawCurtis, 4) => [
                    -7.0805230791197211e-60,
                    -0.36205175936061507,
                    -7.0805230791197211e-60,
                ],
                (GridKind::Fejer1, 4) => [
                    -0.0040352906061222587,
                    -0.44253269244498256,
                    -0.0040352906061222587,
                ],
                _ => unreachable!(),
            };
            let rows = [0, ntheta / 2, ntheta - 1];
            for (row, expected) in rows.into_iter().zip(expected_ducc) {
                let index = row * nphi;
                for (actual, expected) in [(first[index], expected), (second[index], 0.0)] {
                    let error = (actual - expected).abs();
                    ducc_synthesis_max_error = ducc_synthesis_max_error.max(error);
                    assert!(
                        error <= 3e-14,
                        "{kind:?} spin {spin} DUCC S error={error:.3e}"
                    );
                }
            }
            for (ring, &theta) in plan.grid.theta().iter().enumerate() {
                let expected = lowest_spin_harmonic_m0_reference(spin, theta);
                for longitude in 0..nphi {
                    let index = ring * nphi + longitude;
                    for (actual, expected) in [(first[index], expected), (second[index], 0.0)] {
                        let error = (actual - expected).abs();
                        analytic_synthesis_max_error = analytic_synthesis_max_error.max(error);
                        analytic_synthesis_error_squares += error * error;
                        analytic_synthesis_sample_count += 1;
                        assert!(
                            error <= 3e-14,
                            "{kind:?} spin {spin} analytic S error={error:.3e}"
                        );
                    }
                }
            }

            let mut recovered_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            let mut recovered_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            let mut roundtrip_analysis_max_error = 0.0_f64;
            plan.analysis_spin_into(
                &first,
                &second,
                &mut recovered_e,
                &mut recovered_b,
                &mut workspace,
            )
            .unwrap();
            for index in 0..plan.harmonics.nalm() {
                let expected = if index == m_zero_mode {
                    Complex64::new(1.0, 0.0)
                } else {
                    Complex64::new(0.0, 0.0)
                };
                let e_error = (recovered_e[index] - expected).norm();
                let b_error = recovered_b[index].norm();
                roundtrip_analysis_max_error =
                    roundtrip_analysis_max_error.max(e_error).max(b_error);
                assert!(e_error <= 3e-12 && b_error <= 3e-12);
            }

            let (input_first, input_second) = arbitrary_maps(&plan);
            let mut analyzed_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            let mut analyzed_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            plan.analysis_spin_into(
                &input_first,
                &input_second,
                &mut analyzed_e,
                &mut analyzed_b,
                &mut workspace,
            )
            .unwrap();
            let expected_analysis = match (kind, spin) {
                (GridKind::GaussLegendre, 3) => [
                    (
                        Complex64::new(-2.5624192608658785, 0.0),
                        Complex64::new(-2.5186532570202198, 0.0),
                    ),
                    (
                        Complex64::new(0.043309451773531224, -0.046575550988545114),
                        Complex64::new(0.04906763451384076, 0.048846736939148455),
                    ),
                ],
                (GridKind::ClenshawCurtis, 3) => [
                    (
                        Complex64::new(-2.6975523577414426, 0.0),
                        Complex64::new(-2.5615763553922557, 0.0),
                    ),
                    (
                        Complex64::new(0.027404575317222315, -0.04353582407459089),
                        Complex64::new(0.030457222488887239, 0.049018020596425856),
                    ),
                ],
                (GridKind::Fejer1, 3) => [
                    (
                        Complex64::new(-2.5033401301133527, 3.3144716534937704e-16),
                        Complex64::new(-2.4612400853159939, 0.0),
                    ),
                    (
                        Complex64::new(0.043957963425012382, -0.042837743704757895),
                        Complex64::new(0.049126622697667162, 0.045558983025713866),
                    ),
                ],
                (GridKind::GaussLegendre, 4) => [
                    (
                        Complex64::new(-2.2993398767809983, 0.0),
                        Complex64::new(-2.258292856214644, 0.0),
                    ),
                    (
                        Complex64::new(0.055396084413428213, 0.0073873299572970261),
                        Complex64::new(0.060665080308287669, 0.02269883381804097),
                    ),
                ],
                (GridKind::ClenshawCurtis, 4) => [
                    (
                        Complex64::new(-2.326978141824704, 8.8084162393368395e-18),
                        Complex64::new(-2.283731088252011, 4.4042081196684197e-18),
                    ),
                    (
                        Complex64::new(0.059365947319141854, 0.02309949634471475),
                        Complex64::new(0.0530982155748317, 0.011287609932186941),
                    ),
                ],
                (GridKind::Fejer1, 4) => [
                    (
                        Complex64::new(-2.3026107544882368, -5.8722774928912263e-18),
                        Complex64::new(-2.2615693641179146, -1.4680693732228064e-17),
                    ),
                    (
                        Complex64::new(0.05247260029983386, 0.0083980269847998626),
                        Complex64::new(0.057257469573341067, 0.024314133578218253),
                    ),
                ],
                _ => unreachable!(),
            };
            let mut ducc_analysis_max_error = 0.0_f64;
            for (index, (expected_e, expected_b)) in [m_zero_mode, sectoral_mode]
                .into_iter()
                .zip(expected_analysis)
            {
                let e_error = (analyzed_e[index] - expected_e).norm();
                let b_error = (analyzed_b[index] - expected_b).norm();
                ducc_analysis_max_error = ducc_analysis_max_error.max(e_error).max(b_error);
                assert!(e_error <= 2e-12 && b_error <= 2e-12);
            }

            let (e, b) = coefficients(&plan);
            let mut synthesized_first = vec![0.0; plan.sample_count];
            let mut synthesized_second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(
                &e,
                &b,
                &mut synthesized_first,
                &mut synthesized_second,
                &mut workspace,
            )
            .unwrap();
            let mut adjoint_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            let mut adjoint_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            plan.adjoint_synthesis_spin_into(
                &input_first,
                &input_second,
                &mut adjoint_e,
                &mut adjoint_b,
                &mut workspace,
            )
            .unwrap();
            let synthesis_adjoint_left = spin_map_inner_product(
                &synthesized_first,
                &synthesized_second,
                &input_first,
                &input_second,
            );
            let synthesis_adjoint_right =
                spin_harmonic_inner_product(&plan.harmonics, &e, &b, &adjoint_e, &adjoint_b);
            let mut adjoint_identity_max_error =
                (synthesis_adjoint_left - synthesis_adjoint_right).abs();
            assert!(adjoint_identity_max_error <= 2e-11);

            let mut analysis_adjoint_first = vec![0.0; plan.sample_count];
            let mut analysis_adjoint_second = vec![0.0; plan.sample_count];
            plan.adjoint_analysis_spin_into(
                &e,
                &b,
                &mut analysis_adjoint_first,
                &mut analysis_adjoint_second,
                &mut workspace,
            )
            .unwrap();
            let analysis_adjoint_left =
                spin_harmonic_inner_product(&plan.harmonics, &analyzed_e, &analyzed_b, &e, &b);
            let analysis_adjoint_right = spin_map_inner_product(
                &input_first,
                &input_second,
                &analysis_adjoint_first,
                &analysis_adjoint_second,
            );
            adjoint_identity_max_error = adjoint_identity_max_error
                .max((analysis_adjoint_left - analysis_adjoint_right).abs());
            assert!(adjoint_identity_max_error <= 2e-11);

            let zero_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            let mut gradient_first = vec![0.0; plan.sample_count];
            let mut gradient_second = vec![0.0; plan.sample_count];
            plan.synthesis_gradient_into(
                &e,
                &mut gradient_first,
                &mut gradient_second,
                &mut workspace,
            )
            .unwrap();
            let mut standard_first = vec![0.0; plan.sample_count];
            let mut standard_second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(
                &e,
                &zero_b,
                &mut standard_first,
                &mut standard_second,
                &mut workspace,
            )
            .unwrap();
            for (&gradient, &standard) in gradient_first.iter().zip(&standard_first) {
                assert_close(gradient, standard, 2e-13);
            }
            for (&gradient, &standard) in gradient_second.iter().zip(&standard_second) {
                assert_close(gradient, standard, 2e-13);
            }
            let mut gradient_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            plan.adjoint_synthesis_gradient_into(
                &input_first,
                &input_second,
                &mut gradient_adjoint,
                &mut workspace,
            )
            .unwrap();
            let gradient_adjoint_left = spin_map_inner_product(
                &gradient_first,
                &gradient_second,
                &input_first,
                &input_second,
            );
            let gradient_adjoint_right =
                harmonic_inner_product(&plan.harmonics, &e, &gradient_adjoint);
            adjoint_identity_max_error = adjoint_identity_max_error
                .max((gradient_adjoint_left - gradient_adjoint_right).abs());
            assert!(adjoint_identity_max_error <= 2e-11);
            eprintln!(
                "spin {spin} {kind:?}: DUCC S max={ducc_synthesis_max_error:.3e}; analytic S max={analytic_synthesis_max_error:.3e}, RMS={:.3e}; DUCC A selected coefficients max={ducc_analysis_max_error:.3e}; A(S) max coefficient={roundtrip_analysis_max_error:.3e}; adjoint inner-product max absolute residual={adjoint_identity_max_error:.3e}",
                (analytic_synthesis_error_squares / analytic_synthesis_sample_count as f64).sqrt(),
            );
        }
    }
}

#[test]
fn spin_operations_reject_unsupported_spin_and_invalid_coefficients() {
    let unsupported = make_plan(5, 5, 1, 6, 5);
    let mut unsupported_workspace = unsupported.workspace().unwrap();
    let unsupported_coefficients = vec![Complex64::new(0.0, 0.0); unsupported.harmonics.nalm()];
    let mut unsupported_first = vec![0.0; unsupported.sample_count];
    let mut unsupported_second = vec![0.0; unsupported.sample_count];
    let mut unsupported_e = vec![Complex64::new(0.0, 0.0); unsupported.harmonics.nalm()];
    let mut unsupported_b = vec![Complex64::new(0.0, 0.0); unsupported.harmonics.nalm()];
    assert_eq!(
        unsupported.synthesis_spin_into(
            &unsupported_coefficients,
            &unsupported_coefficients,
            &mut unsupported_first,
            &mut unsupported_second,
            &mut unsupported_workspace,
        ),
        Err(ShtError::UnsupportedSpin { spin: 5 })
    );
    assert_eq!(
        unsupported.analysis_spin_into(
            &unsupported_first,
            &unsupported_second,
            &mut unsupported_e,
            &mut unsupported_b,
            &mut unsupported_workspace,
        ),
        Err(ShtError::UnsupportedSpin { spin: 5 })
    );
    assert_eq!(
        unsupported.adjoint_synthesis_spin_into(
            &unsupported_first,
            &unsupported_second,
            &mut unsupported_e,
            &mut unsupported_b,
            &mut unsupported_workspace,
        ),
        Err(ShtError::UnsupportedSpin { spin: 5 })
    );
    assert_eq!(
        unsupported.adjoint_analysis_spin_into(
            &unsupported_coefficients,
            &unsupported_coefficients,
            &mut unsupported_first,
            &mut unsupported_second,
            &mut unsupported_workspace,
        ),
        Err(ShtError::UnsupportedSpin { spin: 5 })
    );

    let plan = make_plan(1, 1, 1, 2, 3);
    let mut workspace = plan.workspace().unwrap();
    let mut coefficients = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut first = vec![0.0; plan.sample_count];
    let mut second = vec![0.0; plan.sample_count];
    coefficients[plan.harmonics.index(1, 0).unwrap()].im = 1e-12;
    assert_eq!(
        plan.synthesis_gradient_into(&coefficients, &mut first, &mut second, &mut workspace,),
        Err(ShtError::NonRealMZero { degree: 1 })
    );

    coefficients.fill(Complex64::new(0.0, 0.0));
    coefficients[plan.harmonics.index(0, 0).unwrap()].re = 1.0;
    let zero_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    assert_eq!(
        plan.synthesis_spin_into(
            &coefficients,
            &zero_b,
            &mut first,
            &mut second,
            &mut workspace,
        ),
        Err(ShtError::NonzeroBelowSpin {
            spin: 1,
            degree: 0,
            order: 0
        })
    );
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_f1_dynamic_synthesis_groups_match_serial_and_reuse_workspace() {
    let plan = make_f1_plan(1, 720, 180, 721, 378);
    let (e, b) = coefficients(&plan);
    let (map_first, map_second) = arbitrary_maps(&plan);
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut one_workspace = one_thread.install(|| plan.workspace()).unwrap();
    let mut eight_workspace = eight_threads.install(|| plan.workspace()).unwrap();
    let mut one_first = vec![0.0; plan.sample_count];
    let mut one_second = vec![0.0; plan.sample_count];
    let mut eight_first = vec![0.0; plan.sample_count];
    let mut eight_second = vec![0.0; plan.sample_count];

    one_thread
        .install(|| {
            plan.synthesis_spin_into(&e, &b, &mut one_first, &mut one_second, &mut one_workspace)
        })
        .unwrap();
    eight_threads
        .install(|| {
            plan.synthesis_spin_into(
                &e,
                &b,
                &mut eight_first,
                &mut eight_second,
                &mut eight_workspace,
            )
        })
        .unwrap();
    assert_eq!(
        eight_workspace.spin1_scalar_coefficients.len(),
        (plan.harmonics.lmax() + 2) * 2 * (plan.harmonics.mmax() + 1)
    );
    assert_eq!(
        eight_workspace.spin1_scalar_coefficient_finite_orders.len(),
        plan.harmonics.mmax() + 1
    );
    for (actual, expected) in eight_first
        .iter()
        .zip(&one_first)
        .chain(eight_second.iter().zip(&one_second))
    {
        assert!((actual - expected).abs() <= 2.0e-11 * expected.abs().max(1.0));
    }

    eight_first.fill(f64::NAN);
    eight_second.fill(f64::NAN);
    eight_threads
        .install(|| {
            plan.synthesis_spin_into(
                &e,
                &b,
                &mut eight_first,
                &mut eight_second,
                &mut eight_workspace,
            )
        })
        .unwrap();
    assert!(
        eight_first
            .iter()
            .chain(&eight_second)
            .all(|value| value.is_finite())
    );
    for (actual, expected) in eight_first
        .iter()
        .zip(&one_first)
        .chain(eight_second.iter().zip(&one_second))
    {
        assert!((actual - expected).abs() <= 2.0e-11 * expected.abs().max(1.0));
    }

    let mut adjoint_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut adjoint_b = adjoint_e.clone();
    eight_threads
        .install(|| {
            plan.adjoint_synthesis_spin_into(
                &map_first,
                &map_second,
                &mut adjoint_e,
                &mut adjoint_b,
                &mut eight_workspace,
            )
        })
        .unwrap();
    let lhs = spin_map_inner_product(&eight_first, &eight_second, &map_first, &map_second);
    let rhs = spin_harmonic_inner_product(plan.harmonics(), &e, &b, &adjoint_e, &adjoint_b);
    assert!((lhs - rhs).abs() <= 3.0e-11 * lhs.abs().max(rhs.abs()).max(1.0));

    let mut changed_e = e.clone();
    changed_e[plan.harmonics.index(720, 180).unwrap()].re += 0.125;
    let mut changed_one_first = vec![0.0; plan.sample_count];
    let mut changed_one_second = vec![0.0; plan.sample_count];
    let mut changed_eight_first = vec![0.0; plan.sample_count];
    let mut changed_eight_second = vec![0.0; plan.sample_count];
    one_thread
        .install(|| {
            plan.synthesis_spin_into(
                &changed_e,
                &b,
                &mut changed_one_first,
                &mut changed_one_second,
                &mut one_workspace,
            )
        })
        .unwrap();
    eight_threads
        .install(|| {
            plan.synthesis_spin_into(
                &changed_e,
                &b,
                &mut changed_eight_first,
                &mut changed_eight_second,
                &mut eight_workspace,
            )
        })
        .unwrap();
    for (actual, expected) in changed_eight_first
        .iter()
        .zip(&changed_one_first)
        .chain(changed_eight_second.iter().zip(&changed_one_second))
    {
        assert!((actual - expected).abs() <= 2.0e-11 * expected.abs().max(1.0));
    }
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_cc_grouped_synthesis_reuses_checked_coefficients() {
    let plan = make_cc_plan(1, 720, 180, 722, 378);
    let (e, b) = coefficients(&plan);
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut one_workspace = one_thread.install(|| plan.workspace()).unwrap();
    let mut eight_workspace = eight_threads.install(|| plan.workspace()).unwrap();
    let mut one_first = vec![0.0; plan.sample_count];
    let mut one_second = vec![0.0; plan.sample_count];
    let mut eight_first = vec![0.0; plan.sample_count];
    let mut eight_second = vec![0.0; plan.sample_count];

    one_thread
        .install(|| {
            plan.synthesis_spin_into(&e, &b, &mut one_first, &mut one_second, &mut one_workspace)
        })
        .unwrap();
    eight_threads
        .install(|| {
            plan.synthesis_spin_into(
                &e,
                &b,
                &mut eight_first,
                &mut eight_second,
                &mut eight_workspace,
            )
        })
        .unwrap();
    assert_eq!(
        eight_workspace.spin1_scalar_coefficients.len(),
        (plan.harmonics.lmax() + 2) * 2 * (plan.harmonics.mmax() + 1)
    );
    assert_eq!(
        eight_workspace.spin1_scalar_coefficient_finite_orders.len(),
        plan.harmonics.mmax() + 1
    );
    assert!(
        eight_workspace
            .spin1_scalar_coefficient_finite_orders
            .iter()
            .all(|&finite| finite == 1)
    );
    for (actual, expected) in eight_first
        .iter()
        .zip(&one_first)
        .chain(eight_second.iter().zip(&one_second))
    {
        assert!((actual - expected).abs() <= 2.0e-11 * expected.abs().max(1.0));
    }

    let mut changed_e = e.clone();
    changed_e[plan.harmonics.index(720, 180).unwrap()].re += 0.125;
    eight_first.fill(f64::NAN);
    eight_second.fill(f64::NAN);
    let mut changed_one_first = vec![0.0; plan.sample_count];
    let mut changed_one_second = vec![0.0; plan.sample_count];
    one_thread
        .install(|| {
            plan.synthesis_spin_into(
                &changed_e,
                &b,
                &mut changed_one_first,
                &mut changed_one_second,
                &mut one_workspace,
            )
        })
        .unwrap();
    eight_threads
        .install(|| {
            plan.synthesis_spin_into(
                &changed_e,
                &b,
                &mut eight_first,
                &mut eight_second,
                &mut eight_workspace,
            )
        })
        .unwrap();
    assert!(
        eight_first
            .iter()
            .chain(&eight_second)
            .all(|value| value.is_finite())
    );
    for (actual, expected) in eight_first
        .iter()
        .zip(&changed_one_first)
        .chain(eight_second.iter().zip(&changed_one_second))
    {
        assert!((actual - expected).abs() <= 2.0e-11 * expected.abs().max(1.0));
    }

    let magnitude = 1.44e308;
    let mut extreme_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut extreme_b = extreme_e.clone();
    extreme_e[plan.harmonics.index(2, 1).unwrap()] = Complex64::new(magnitude, 0.0);
    extreme_b[plan.harmonics.index(1, 1).unwrap()] = Complex64::new(0.0, -magnitude);
    eight_first.fill(f64::NAN);
    eight_second.fill(f64::NAN);
    one_first.fill(f64::NAN);
    one_second.fill(f64::NAN);
    one_thread
        .install(|| {
            plan.synthesis_spin_into(
                &extreme_e,
                &extreme_b,
                &mut one_first,
                &mut one_second,
                &mut one_workspace,
            )
        })
        .unwrap();
    eight_threads
        .install(|| {
            plan.synthesis_spin_into(
                &extreme_e,
                &extreme_b,
                &mut eight_first,
                &mut eight_second,
                &mut eight_workspace,
            )
        })
        .unwrap();
    assert_eq!(eight_workspace.spin1_scalar_coefficient_finite_orders[1], 0);
    for (actual, expected) in eight_first
        .iter()
        .chain(&eight_second)
        .zip(one_first.iter().chain(&one_second))
    {
        if expected.is_finite() {
            assert!(actual.is_finite());
            assert!((actual / magnitude - expected / magnitude).abs() < 2.0e-12);
        } else if expected.is_infinite() {
            assert!(actual.is_infinite());
            assert_eq!(actual.is_sign_positive(), expected.is_sign_positive());
        } else {
            assert!(actual.is_nan());
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn spin_transform_matches_between_one_and_two_rayon_threads() {
    let plan = make_plan(2, 4, 4, 131, 11);
    let (e, b) = coefficients(&plan);
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let two_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let mut one_workspace = one_thread.install(|| plan.workspace()).unwrap();
    let mut two_workspace = two_threads.install(|| plan.workspace()).unwrap();
    let mut one_first = vec![0.0; plan.sample_count];
    let mut one_second = vec![0.0; plan.sample_count];
    let mut two_first = vec![0.0; plan.sample_count];
    let mut two_second = vec![0.0; plan.sample_count];
    one_thread
        .install(|| {
            plan.synthesis_spin_into(&e, &b, &mut one_first, &mut one_second, &mut one_workspace)
        })
        .unwrap();
    two_threads
        .install(|| {
            plan.synthesis_spin_into(&e, &b, &mut two_first, &mut two_second, &mut two_workspace)
        })
        .unwrap();
    for (&actual, &expected) in two_first.iter().zip(&one_first) {
        assert_close(actual, expected, 3e-14);
    }
    for (&actual, &expected) in two_second.iter().zip(&one_second) {
        assert_close(actual, expected, 3e-14);
    }

    let mut one_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut one_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut two_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut two_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    one_thread
        .install(|| {
            plan.analysis_spin_into(
                &one_first,
                &one_second,
                &mut one_e,
                &mut one_b,
                &mut one_workspace,
            )
        })
        .unwrap();
    two_threads
        .install(|| {
            plan.analysis_spin_into(
                &one_first,
                &one_second,
                &mut two_e,
                &mut two_b,
                &mut two_workspace,
            )
        })
        .unwrap();
    for (&actual, &expected) in two_e.iter().zip(&one_e) {
        assert_complex_close(actual, expected, 3e-14);
    }
    for (&actual, &expected) in two_b.iter().zip(&one_b) {
        assert_complex_close(actual, expected, 3e-14);
    }

    let (input_first, input_second) = arbitrary_maps(&plan);
    let mut one_adjoint_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut one_adjoint_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut two_adjoint_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut two_adjoint_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    one_thread
        .install(|| {
            plan.adjoint_synthesis_spin_into(
                &input_first,
                &input_second,
                &mut one_adjoint_e,
                &mut one_adjoint_b,
                &mut one_workspace,
            )
        })
        .unwrap();
    two_threads
        .install(|| {
            plan.adjoint_synthesis_spin_into(
                &input_first,
                &input_second,
                &mut two_adjoint_e,
                &mut two_adjoint_b,
                &mut two_workspace,
            )
        })
        .unwrap();
    for (&one, &two) in one_adjoint_e.iter().zip(&two_adjoint_e) {
        assert_complex_close(two, one, 3e-14);
    }
    for (&one, &two) in one_adjoint_b.iter().zip(&two_adjoint_b) {
        assert_complex_close(two, one, 3e-14);
    }

    let mut one_analysis_adjoint_first = vec![0.0; plan.sample_count];
    let mut one_analysis_adjoint_second = vec![0.0; plan.sample_count];
    let mut two_analysis_adjoint_first = vec![0.0; plan.sample_count];
    let mut two_analysis_adjoint_second = vec![0.0; plan.sample_count];
    one_thread
        .install(|| {
            plan.adjoint_analysis_spin_into(
                &e,
                &b,
                &mut one_analysis_adjoint_first,
                &mut one_analysis_adjoint_second,
                &mut one_workspace,
            )
        })
        .unwrap();
    two_threads
        .install(|| {
            plan.adjoint_analysis_spin_into(
                &e,
                &b,
                &mut two_analysis_adjoint_first,
                &mut two_analysis_adjoint_second,
                &mut two_workspace,
            )
        })
        .unwrap();
    for (&one, &two) in one_analysis_adjoint_first
        .iter()
        .zip(&two_analysis_adjoint_first)
    {
        assert_close(two, one, 3e-14);
    }
    for (&one, &two) in one_analysis_adjoint_second
        .iter()
        .zip(&two_analysis_adjoint_second)
    {
        assert_close(two, one, 3e-14);
    }

    let mut one_gradient_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut two_gradient_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    one_thread
        .install(|| {
            plan.adjoint_synthesis_gradient_into(
                &input_first,
                &input_second,
                &mut one_gradient_adjoint,
                &mut one_workspace,
            )
        })
        .unwrap();
    two_threads
        .install(|| {
            plan.adjoint_synthesis_gradient_into(
                &input_first,
                &input_second,
                &mut two_gradient_adjoint,
                &mut two_workspace,
            )
        })
        .unwrap();
    for (&one, &two) in one_gradient_adjoint.iter().zip(&two_gradient_adjoint) {
        assert_complex_close(two, one, 3e-14);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn t63_scalar_and_spin1_synthesis_match_one_and_eight_threads() {
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();

    for grid_kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let ntheta = 64 + usize::from(grid_kind == GridKind::ClenshawCurtis);
        let nphi = 129;
        let scalar_plan = ShtPlan::new(
            GridSpec::new(grid_kind, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::scalar(63, 63).unwrap(),
        )
        .unwrap();
        let spin_plan = ShtPlan::new(
            GridSpec::new(grid_kind, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::new(63, 63, 1).unwrap(),
        )
        .unwrap();
        let (mut e_alm, mut b_alm) = coefficients(&spin_plan);
        for value in e_alm.iter_mut().chain(&mut b_alm) {
            *value *= 1e-5;
        }
        let mut scalar_one_workspace = one_thread.install(|| scalar_plan.workspace()).unwrap();
        let mut scalar_eight_workspace = eight_threads.install(|| scalar_plan.workspace()).unwrap();
        let mut spin_one_workspace = one_thread.install(|| spin_plan.workspace()).unwrap();
        let mut spin_eight_workspace = eight_threads.install(|| spin_plan.workspace()).unwrap();

        let mut scalar_one = vec![0.0; scalar_plan.sample_count];
        let mut scalar_eight = vec![0.0; scalar_plan.sample_count];
        one_thread
            .install(|| {
                scalar_plan.synthesis_into(&e_alm, &mut scalar_one, &mut scalar_one_workspace)
            })
            .unwrap();
        eight_threads
            .install(|| {
                scalar_plan.synthesis_into(&e_alm, &mut scalar_eight, &mut scalar_eight_workspace)
            })
            .unwrap();
        for (&one, &eight) in scalar_one.iter().zip(&scalar_eight) {
            assert_close(eight, one, 3e-14);
        }

        let mut one_first = vec![0.0; spin_plan.sample_count];
        let mut one_second = vec![0.0; spin_plan.sample_count];
        let mut eight_first = vec![0.0; spin_plan.sample_count];
        let mut eight_second = vec![0.0; spin_plan.sample_count];
        one_thread
            .install(|| {
                spin_plan.synthesis_spin_into(
                    &e_alm,
                    &b_alm,
                    &mut one_first,
                    &mut one_second,
                    &mut spin_one_workspace,
                )
            })
            .unwrap();
        eight_threads
            .install(|| {
                spin_plan.synthesis_spin_into(
                    &e_alm,
                    &b_alm,
                    &mut eight_first,
                    &mut eight_second,
                    &mut spin_eight_workspace,
                )
            })
            .unwrap();
        for (&one, &eight) in one_first.iter().zip(&eight_first) {
            assert_close(eight, one, 3e-14);
        }
        for (&one, &eight) in one_second.iter().zip(&eight_second) {
            assert_close(eight, one, 3e-14);
        }

        let mut one_gradient_first = vec![0.0; spin_plan.sample_count];
        let mut one_gradient_second = vec![0.0; spin_plan.sample_count];
        let mut eight_gradient_first = vec![0.0; spin_plan.sample_count];
        let mut eight_gradient_second = vec![0.0; spin_plan.sample_count];
        one_thread
            .install(|| {
                spin_plan.synthesis_gradient_into(
                    &e_alm,
                    &mut one_gradient_first,
                    &mut one_gradient_second,
                    &mut spin_one_workspace,
                )
            })
            .unwrap();
        eight_threads
            .install(|| {
                spin_plan.synthesis_gradient_into(
                    &e_alm,
                    &mut eight_gradient_first,
                    &mut eight_gradient_second,
                    &mut spin_eight_workspace,
                )
            })
            .unwrap();
        for (&one, &eight) in one_gradient_first.iter().zip(&eight_gradient_first) {
            assert_close(eight, one, 3e-14);
        }
        for (&one, &eight) in one_gradient_second.iter().zip(&eight_gradient_second) {
            assert_close(eight, one, 3e-14);
        }

        let mut one_derivative_first = vec![0.0; scalar_plan.sample_count];
        let mut one_derivative_second = vec![0.0; scalar_plan.sample_count];
        let mut eight_derivative_first = vec![0.0; scalar_plan.sample_count];
        let mut eight_derivative_second = vec![0.0; scalar_plan.sample_count];
        one_thread
            .install(|| {
                scalar_plan.synthesis_deriv1_into(
                    &e_alm,
                    &mut one_derivative_first,
                    &mut one_derivative_second,
                    &mut scalar_one_workspace,
                )
            })
            .unwrap();
        eight_threads
            .install(|| {
                scalar_plan.synthesis_deriv1_into(
                    &e_alm,
                    &mut eight_derivative_first,
                    &mut eight_derivative_second,
                    &mut scalar_eight_workspace,
                )
            })
            .unwrap();
        for (&one, &eight) in one_derivative_first.iter().zip(&eight_derivative_first) {
            assert_close(eight, one, 3e-14);
        }
        for (&one, &eight) in one_derivative_second.iter().zip(&eight_derivative_second) {
            assert_close(eight, one, 3e-14);
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_parallel_coefficient_cache_matches_uncached_synthesis() {
    let plan = make_plan(1, 127, 127, 128, 257);
    let (mut e_alm, mut b_alm) = coefficients(&plan);
    for value in e_alm.iter_mut().chain(&mut b_alm) {
        *value *= 1e-6;
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut cached_workspace = pool.install(|| plan.workspace()).unwrap();
    let mut uncached_workspace = pool.install(|| plan.workspace()).unwrap();
    assert!(cached_workspace.ring_group_size < plan.grid.ntheta());
    uncached_workspace.ring_group_size = plan.grid.ntheta();

    let mut cached_first = vec![0.0; plan.sample_count];
    let mut cached_second = vec![0.0; plan.sample_count];
    let mut uncached_first = vec![0.0; plan.sample_count];
    let mut uncached_second = vec![0.0; plan.sample_count];
    pool.install(|| {
        plan.synthesis_spin_into(
            &e_alm,
            &b_alm,
            &mut cached_first,
            &mut cached_second,
            &mut cached_workspace,
        )
    })
    .unwrap();
    pool.install(|| {
        plan.synthesis_spin_into(
            &e_alm,
            &b_alm,
            &mut uncached_first,
            &mut uncached_second,
            &mut uncached_workspace,
        )
    })
    .unwrap();

    let expected_cache_len = 2 * (plan.harmonics.mmax() + 1) * (plan.harmonics.lmax() + 2);
    assert_eq!(
        cached_workspace.spin1_scalar_coefficients.len(),
        expected_cache_len
    );
    assert_eq!(
        cached_workspace
            .spin1_scalar_coefficient_finite_orders
            .len(),
        plan.harmonics.mmax() + 1
    );
    assert!(
        cached_workspace
            .spin1_scalar_coefficient_finite_orders
            .iter()
            .all(|&finite| finite == 1)
    );
    let row_len = plan.harmonics.lmax() + 2;
    let row_stride = 2 * row_len;
    for m in 0..=plan.harmonics.mmax() {
        let coefficient_start = plan.harmonics.index(m, m).unwrap();
        let coefficient_len = plan.harmonics.lmax() - m + 1;
        let e_order = &e_alm[coefficient_start..coefficient_start + coefficient_len];
        let b_order = &b_alm[coefficient_start..coefficient_start + coefficient_len];
        let mut expected_first = vec![Complex64::new(0.0, 0.0); row_len];
        let mut expected_second = vec![Complex64::new(0.0, 0.0); row_len];
        spin1_to_scalar(
            m,
            plan.harmonics.lmax(),
            e_order,
            Some(b_order),
            &mut expected_first,
            &mut expected_second,
        );

        let row_start = m * row_stride;
        assert_eq!(
            &cached_workspace.spin1_scalar_coefficients[row_start..row_start + row_len],
            expected_first.as_slice(),
            "first cached scalar coefficient row differs at m={m}"
        );
        assert_eq!(
            &cached_workspace.spin1_scalar_coefficients
                [row_start + row_len..row_start + row_stride],
            expected_second.as_slice(),
            "second cached scalar coefficient row differs at m={m}"
        );
    }
    let first_max_error = cached_first
        .iter()
        .zip(&uncached_first)
        .map(|(&cached, &uncached)| (cached - uncached).abs())
        .fold(0.0_f64, f64::max);
    let second_max_error = cached_second
        .iter()
        .zip(&uncached_second)
        .map(|(&cached, &uncached)| (cached - uncached).abs())
        .fold(0.0_f64, f64::max);
    assert!(
        first_max_error <= 2e-13,
        "first component error={first_max_error:.3e}"
    );
    assert!(
        second_max_error <= 2e-13,
        "second component error={second_max_error:.3e}"
    );

    // Reuse both workspaces after changing every coefficient. Poison the
    // cached rows and the worker-local rows so the cached path must refresh
    // its input and the polar/scaled fallback must restore its local rows
    // before reading them.
    for value in &mut e_alm {
        *value *= -0.375;
    }
    for value in &mut b_alm {
        *value *= 0.625;
    }
    let poison = Complex64::new(f64::NAN, f64::NAN);
    cached_workspace.spin1_scalar_coefficients.fill(poison);
    cached_workspace
        .spin1_scalar_coefficient_finite_orders
        .fill(0);
    for worker in &mut cached_workspace.recurrence_workers {
        worker.fast_scalar_first.fill(poison);
        worker.fast_scalar_second.fill(poison);
    }
    cached_first.fill(17.0);
    cached_second.fill(-23.0);
    uncached_first.fill(17.0);
    uncached_second.fill(-23.0);
    pool.install(|| {
        plan.synthesis_spin_into(
            &e_alm,
            &b_alm,
            &mut cached_first,
            &mut cached_second,
            &mut cached_workspace,
        )
    })
    .unwrap();
    pool.install(|| {
        plan.synthesis_spin_into(
            &e_alm,
            &b_alm,
            &mut uncached_first,
            &mut uncached_second,
            &mut uncached_workspace,
        )
    })
    .unwrap();
    let first_reuse_error = cached_first
        .iter()
        .zip(&uncached_first)
        .map(|(&cached, &uncached)| (cached - uncached).abs())
        .fold(0.0_f64, f64::max);
    let second_reuse_error = cached_second
        .iter()
        .zip(&uncached_second)
        .map(|(&cached, &uncached)| (cached - uncached).abs())
        .fold(0.0_f64, f64::max);
    assert!(
        first_reuse_error <= 2e-13,
        "reused first component error={first_reuse_error:.3e}"
    );
    assert!(
        second_reuse_error <= 2e-13,
        "reused second component error={second_reuse_error:.3e}"
    );
    assert!(
        cached_first
            .iter()
            .chain(&cached_second)
            .all(|value| value.is_finite())
    );
    assert!(
        cached_workspace
            .spin1_scalar_coefficient_finite_orders
            .iter()
            .all(|&finite| finite == 1)
    );
    eprintln!(
        "T127 GL spin-1 parallel conversion cache: initial max absolute component errors {first_max_error:.3e}, {second_max_error:.3e}; poisoned reuse errors {first_reuse_error:.3e}, {second_reuse_error:.3e}"
    );
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_t1023_order_major_cache_is_bounded_and_reused() {
    let single_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    for mmax in [255, 1023] {
        let lmax = 1023;
        let nphi = if mmax == 255 { 512 } else { 2048 };
        let plan = ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, lmax + 1, nphi, 0.23).unwrap(),
            HarmonicSpec::new(lmax, mmax, 1).unwrap(),
        )
        .unwrap();
        let mut workspace = pool.install(|| plan.workspace()).unwrap();
        pool.install(|| plan.prepare_workspace(&mut workspace, 8))
            .unwrap();
        let mut single_workspace = single_pool.install(|| plan.workspace()).unwrap();
        let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut b_alm = e_alm.clone();
        let mode = plan.harmonics.index(2, 1).unwrap();
        e_alm[mode] = Complex64::new(0.5, -0.25);
        b_alm[mode] = Complex64::new(-0.125, 0.375);
        let mut first = vec![f64::NAN; plan.sample_count];
        let mut second = first.clone();

        SPIN1_PREPARED_COMPONENT_ORDER_SUCCESSES.store(0, Ordering::Relaxed);
        pool.install(|| {
            plan.synthesis_spin_into(&e_alm, &b_alm, &mut first, &mut second, &mut workspace)
        })
        .unwrap();

        let expected_coefficient_values = 2 * (mmax + 1) * (lmax + 2);
        if mmax == 1023 {
            assert!(
                expected_coefficient_values * std::mem::size_of::<Complex64>()
                    >= SPIN1_ORDER_LOCAL_CONVERSION_MIN_BYTES
            );
            assert!(workspace.spin1_scalar_coefficients.is_empty());
            assert!(workspace.spin1_scalar_coefficient_finite_orders.is_empty());
            assert!(
                SPIN1_PREPARED_COMPONENT_ORDER_SUCCESSES.load(Ordering::Relaxed) > 0,
                "large-cache order-major path skipped the prepared component recurrence"
            );
        } else {
            assert_eq!(
                workspace.spin1_scalar_coefficients.len(),
                expected_coefficient_values
            );
            assert!(
                expected_coefficient_values * std::mem::size_of::<Complex64>()
                    <= SPIN1_SYNTH_COEFFICIENT_CACHE_MAX_BYTES
            );
            assert_eq!(
                workspace.spin1_scalar_coefficient_finite_orders.len(),
                mmax + 1
            );
            assert!(
                workspace
                    .spin1_scalar_coefficient_finite_orders
                    .iter()
                    .all(|&finite| finite == 1)
            );
        }
        assert_eq!(workspace.spin_order_output_scratch.len(), 8);
        assert!(first.iter().chain(&second).all(|value| value.is_finite()));

        let mut unsafe_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut unsafe_b = unsafe_e.clone();
        unsafe_e[mode] = Complex64::new(1.44e308, 0.0);
        unsafe_b[plan.harmonics.index(1, 1).unwrap()] = Complex64::new(0.0, -1.44e308);
        first.fill(f64::NAN);
        second.fill(f64::NAN);
        pool.install(|| {
            plan.synthesis_spin_into(
                &unsafe_e,
                &unsafe_b,
                &mut first,
                &mut second,
                &mut workspace,
            )
        })
        .unwrap();
        if !workspace.spin1_scalar_coefficient_finite_orders.is_empty() {
            assert_eq!(workspace.spin1_scalar_coefficient_finite_orders[1], 0);
        }
        let mut single_first = vec![f64::NAN; plan.sample_count];
        let mut single_second = single_first.clone();
        single_pool
            .install(|| {
                plan.synthesis_spin_into(
                    &unsafe_e,
                    &unsafe_b,
                    &mut single_first,
                    &mut single_second,
                    &mut single_workspace,
                )
            })
            .unwrap();
        let scale = single_first
            .iter()
            .chain(&single_second)
            .filter(|value| value.is_finite())
            .map(|value| value.abs())
            .fold(1.0_f64, f64::max);
        for (&single, &parallel) in single_first
            .iter()
            .zip(&first)
            .chain(single_second.iter().zip(&second))
        {
            if single.is_finite() {
                assert!(parallel.is_finite());
                assert!((single / scale - parallel / scale).abs() <= 2e-11);
            } else if single.is_infinite() {
                assert!(parallel.is_infinite());
                assert_eq!(single.is_sign_positive(), parallel.is_sign_positive());
            } else {
                assert!(parallel.is_nan());
            }
        }

        e_alm.fill(Complex64::new(0.0, 0.0));
        b_alm.fill(Complex64::new(0.0, 0.0));
        first.fill(f64::NAN);
        second.fill(f64::NAN);
        pool.install(|| {
            plan.synthesis_spin_into(&e_alm, &b_alm, &mut first, &mut second, &mut workspace)
        })
        .unwrap();
        assert!(first.iter().chain(&second).all(|&value| value == 0.0));
        assert_eq!(workspace.spin_order_output_scratch.len(), 8);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_parallel_nonfinite_conversion_cache_refreshes_fallback_flag() {
    let lmax = 127;
    let ntheta = lmax + 1;
    let plan = make_plan(1, lmax, 1, ntheta, 33);
    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b_alm = e_alm.clone();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut cached_workspace = pool.install(|| plan.workspace()).unwrap();
    let mut uncached_workspace = pool.install(|| plan.workspace()).unwrap();
    assert!(cached_workspace.ring_group_size < ntheta);
    uncached_workspace.ring_group_size = ntheta;

    // Prime the reusable cache with an ordinary finite conversion, then poison
    // its cached values and flags before replacing the coefficients with a
    // finite input whose scalar conversion overflows.
    e_alm[plan.harmonics().index(2, 1).unwrap()] = Complex64::new(1.0e-6, 0.0);
    b_alm[plan.harmonics().index(1, 1).unwrap()] = Complex64::new(0.0, -1.0e-6);
    let mut first = vec![0.0; plan.sample_count];
    let mut second = first.clone();
    pool.install(|| {
        plan.synthesis_spin_into(
            &e_alm,
            &b_alm,
            &mut first,
            &mut second,
            &mut cached_workspace,
        )
    })
    .unwrap();
    assert!(
        cached_workspace
            .spin1_scalar_coefficient_finite_orders
            .iter()
            .all(|&finite| finite == 1)
    );

    let magnitude = 1.44e308;
    e_alm[plan.harmonics().index(2, 1).unwrap()] = Complex64::new(magnitude, 0.0);
    b_alm[plan.harmonics().index(1, 1).unwrap()] = Complex64::new(0.0, -magnitude);
    cached_workspace
        .spin1_scalar_coefficients
        .fill(Complex64::new(f64::NAN, f64::NAN));
    cached_workspace
        .spin1_scalar_coefficient_finite_orders
        .fill(1);
    first.fill(17.0);
    second.fill(-23.0);
    let mut expected_first = vec![17.0; plan.sample_count];
    let mut expected_second = vec![-23.0; plan.sample_count];
    pool.install(|| {
        plan.synthesis_spin_into(
            &e_alm,
            &b_alm,
            &mut first,
            &mut second,
            &mut cached_workspace,
        )
    })
    .unwrap();
    pool.install(|| {
        plan.synthesis_spin_into(
            &e_alm,
            &b_alm,
            &mut expected_first,
            &mut expected_second,
            &mut uncached_workspace,
        )
    })
    .unwrap();
    assert_eq!(
        cached_workspace.spin1_scalar_coefficient_finite_orders[1],
        0
    );

    let compare = |actual: f64, expected: f64| {
        if expected.is_finite() {
            assert!(actual.is_finite());
            assert!((actual / magnitude - expected / magnitude).abs() < 2e-12);
        } else if expected.is_infinite() {
            assert!(actual.is_infinite());
            assert_eq!(actual.is_sign_positive(), expected.is_sign_positive());
        } else {
            assert!(actual.is_nan());
        }
    };
    for (&actual, &expected) in first
        .iter()
        .chain(&second)
        .zip(expected_first.iter().chain(&expected_second))
    {
        compare(actual, expected);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn spin1_compacted_analysis_matches_between_one_and_eight_rayon_threads() {
    let plan = make_plan(1, 511, 511, 512, 1025);
    let (first, second) = arbitrary_maps(&plan);
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut one_workspace = one_thread.install(|| plan.workspace()).unwrap();
    let mut eight_workspace = eight_threads.install(|| plan.workspace()).unwrap();
    let mut one_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut one_b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut eight_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut eight_b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];

    one_thread
        .install(|| {
            plan.analysis_spin_into(&first, &second, &mut one_e, &mut one_b, &mut one_workspace)
        })
        .unwrap();
    eight_threads
        .install(|| {
            plan.analysis_spin_into(
                &first,
                &second,
                &mut eight_e,
                &mut eight_b,
                &mut eight_workspace,
            )
        })
        .unwrap();

    for (channel, one, eight) in [("E", &one_e, &eight_e), ("B", &one_b, &eight_b)] {
        for (index, (&one, &eight)) in one.iter().zip(eight).enumerate() {
            let error = (eight - one).norm();
            assert!(
                error <= 5e-13,
                "spin-1 T511 {channel}[{index}] differs between one/eight threads"
            );
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "M4 T1023 spin-2 thread and polar-threshold check; run in release mode"]
fn m4_spin2_t1023_synthesis_matches_one_and_eight_threads() {
    let degree = 1023;
    let ntheta = degree + 1;
    let nphi = 2 * degree + 3;
    let order = 105;
    let plan = make_plan(2, degree, degree, ntheta, nphi);
    let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b = vec![Complex64::new(0.0, 0.0); e.len()];
    for (l, e_value, b_value) in [
        (order, Complex64::new(0.75, 0.0), Complex64::new(-0.25, 0.0)),
        (
            512,
            Complex64::new(0.125, -0.25),
            Complex64::new(0.5, 0.125),
        ),
        (
            degree,
            Complex64::new(-0.25, 0.0),
            Complex64::new(0.375, 0.0),
        ),
    ] {
        let index = plan.harmonics().index(l, order).unwrap();
        e[index] = e_value;
        b[index] = b_value;
    }
    let low_order_index = plan.harmonics().index(2, 2).unwrap();
    e[low_order_index] = Complex64::new(0.75, 0.0);
    b[low_order_index] = Complex64::new(-0.25, 0.0);

    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut one_workspace = one_thread.install(|| plan.workspace()).unwrap();
    let mut eight_workspace = eight_threads.install(|| plan.workspace()).unwrap();
    plan.prepare_spin_latitudes(&mut eight_workspace, 2);
    let threshold_ring = eight_workspace.theta_sin[..ntheta]
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| (*left - 0.01).abs().total_cmp(&(*right - 0.01).abs()))
        .map(|(ring, _)| ring)
        .unwrap();
    assert!((eight_workspace.theta_sin[threshold_ring] - 0.01).abs() < 0.002);
    assert!(eight_workspace.ring_order_limits[threshold_ring] >= order);
    assert!(eight_workspace.ring_order_limits[0] < order);

    let mut one_first = vec![0.0; plan.sample_count];
    let mut one_second = vec![0.0; plan.sample_count];
    one_thread
        .install(|| {
            plan.synthesis_spin_into(&e, &b, &mut one_first, &mut one_second, &mut one_workspace)
        })
        .unwrap();
    let mut eight_first = vec![0.0; plan.sample_count];
    let mut eight_second = vec![0.0; plan.sample_count];
    eight_threads
        .install(|| {
            plan.synthesis_spin_into(
                &e,
                &b,
                &mut eight_first,
                &mut eight_second,
                &mut eight_workspace,
            )
        })
        .unwrap();

    for (&actual, &expected) in eight_first.iter().zip(&one_first) {
        assert_close(actual, expected, 2e-12);
    }
    for (&actual, &expected) in eight_second.iter().zip(&one_second) {
        assert_close(actual, expected, 2e-12);
    }

    let threshold_start = threshold_ring * nphi;
    let threshold_end = threshold_start + nphi;
    let threshold_amplitude = one_first[threshold_start..threshold_end]
        .iter()
        .chain(&one_second[threshold_start..threshold_end])
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    assert!(threshold_amplitude > 1e-8);

    e[low_order_index] = Complex64::new(0.0, 0.0);
    b[low_order_index] = Complex64::new(0.0, 0.0);
    one_thread
        .install(|| {
            plan.synthesis_spin_into(&e, &b, &mut one_first, &mut one_second, &mut one_workspace)
        })
        .unwrap();
    eight_threads
        .install(|| {
            plan.synthesis_spin_into(
                &e,
                &b,
                &mut eight_first,
                &mut eight_second,
                &mut eight_workspace,
            )
        })
        .unwrap();
    assert!(one_first[..nphi].iter().all(|&value| value == 0.0));
    assert!(one_second[..nphi].iter().all(|&value| value == 0.0));
    assert!(eight_first[..nphi].iter().all(|&value| value == 0.0));
    assert!(eight_second[..nphi].iter().all(|&value| value == 0.0));
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "These high-degree decimal fixtures retain the printed DUCC reference values."
)]
fn spin_t255_gl_matches_ducc_samples_and_coefficients() {
    // Static outputs from ducc0==0.41.0, GL 256x513, phi0=0.37, one thread.
    for spin in [1, 2] {
        let plan = make_plan(spin, 255, 255, 256, 513);
        let (mut e, mut b) = (
            vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()],
            vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()],
        );
        for m in 0..=255 {
            for l in spin.max(m)..=255 {
                let index = plan.harmonics.index(l, m).unwrap();
                let real_e = ((7 * l + 11 * m) % 19) as i32 - 9;
                let imag_e = ((13 * l + 5 * m) % 23) as i32 - 11;
                let real_b = ((5 * l + 3 * m) % 17) as i32 - 8;
                let imag_b = ((3 * l + 7 * m) % 13) as i32 - 6;
                e[index] = Complex64::new(
                    1e-5 * f64::from(real_e),
                    if m == 0 {
                        0.0
                    } else {
                        1e-5 * f64::from(imag_e)
                    },
                );
                b[index] = Complex64::new(
                    0.7e-5 * f64::from(real_b),
                    if m == 0 {
                        0.0
                    } else {
                        0.9e-5 * f64::from(imag_b)
                    },
                );
            }
        }
        let mut workspace = plan.workspace().unwrap();
        let mut first = vec![0.0; plan.sample_count];
        let mut second = vec![0.0; plan.sample_count];
        plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
            .unwrap();
        let sample_indices = [0, 626, 127 * 513 + 256, 255 * 513 + 512];
        let (expected_first, expected_second) = if spin == 1 {
            (
                [
                    -0.000_171_618_524_796_108,
                    0.000_240_781_297_514_088_88,
                    0.002_306_027_070_480_382_8,
                    0.000_643_793_990_142_110_37,
                ],
                [
                    0.000_365_376_736_155_249_38,
                    -0.000_105_662_296_816_821_9,
                    0.000_513_609_667_700_186_31,
                    0.000_393_166_009_911_476_18,
                ],
            )
        } else {
            (
                [
                    -0.000_191_889_393_857_919_31,
                    -0.000_072_843_722_054_503_326,
                    0.001_119_711_711_133_881_2,
                    -0.001_035_586_726_648_950_9,
                ],
                [
                    0.000_039_856_788_143_593_73,
                    -0.000_631_962_191_664_244_82,
                    -0.002_267_234_955_300_86,
                    0.000_809_307_858_647_371_52,
                ],
            )
        };
        for (&index, &expected) in sample_indices.iter().zip(&expected_first) {
            assert_close(first[index], expected, 2e-12);
        }
        for (&index, &expected_second) in sample_indices.iter().zip(&expected_second) {
            assert_close(second[index], expected_second, 2e-12);
        }

        e.fill(Complex64::new(0.0, 0.0));
        b.fill(Complex64::new(0.0, 0.0));
        e[plan.harmonics.index(255, 255).unwrap()] = Complex64::new(1.0, 0.0);
        plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
            .unwrap();
        let sectoral_indices = [
            0,
            513,
            127 * 513,
            127 * 513 + 128,
            128 * 513 + 256,
            255 * 513 + 512,
        ];
        let (sectoral_first, sectoral_second) = if spin == 1 {
            (
                [
                    0.0,
                    0.0,
                    -0.014_502_544_345_968_933,
                    0.009_150_897_969_309_892_6,
                    -0.001_620_795_756_960_675_1,
                    0.0,
                ],
                [
                    0.0,
                    0.0,
                    0.242_923_273_826_296_67,
                    -1.853_213_608_652_077_8,
                    2.365_851_500_895_088_7,
                    0.0,
                ],
            )
        } else {
            (
                [
                    0.0,
                    0.0,
                    2.354_452_652_343_165_9,
                    -1.485_625_934_400_402_1,
                    -0.263_132_232_375_715_2,
                    0.0,
                ],
                [
                    0.0,
                    0.0,
                    -0.002_957_920_717_880_142_2,
                    0.022_565_392_114_750_955,
                    0.028_807_454_550_153_003,
                    0.0,
                ],
            )
        };
        for ((&index, &expected_first), &expected_second) in sectoral_indices
            .iter()
            .zip(&sectoral_first)
            .zip(&sectoral_second)
        {
            assert_close(first[index], expected_first, 3e-11);
            assert_close(second[index], expected_second, 3e-11);
        }

        let mut map_first = vec![0.0; plan.sample_count];
        let mut map_second = vec![0.0; plan.sample_count];
        for i in 0..256 {
            for j in 0..513 {
                map_first[i * 513 + j] = (0.013 * i as f64 + 0.017 * j as f64).sin()
                    + 0.3 * (0.019 * i as f64 - 0.011 * j as f64).cos();
                map_second[i * 513 + j] = (0.007 * i as f64 - 0.023 * j as f64).cos()
                    - 0.2 * (0.031 * i as f64 + 0.003 * j as f64).sin();
            }
        }
        let mut recovered_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut recovered_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.analysis_spin_into(
            &map_first,
            &map_second,
            &mut recovered_e,
            &mut recovered_b,
            &mut workspace,
        )
        .unwrap();
        if spin == 1 {
            assert_eq!(
                plan.spin1_scalar_legendre_table
                    .get()
                    .is_some_and(Option::is_some),
                plan.allow_spin1_analysis_table()
            );
            let general_plan = make_plan(1, 255, 255, 256, 513);
            assert!(general_plan.spin1_scalar_legendre_table.set(None).is_ok());
            let mut general_workspace = general_plan.workspace().unwrap();
            let mut general_e = vec![Complex64::new(0.0, 0.0); general_plan.harmonics.nalm()];
            let mut general_b = vec![Complex64::new(0.0, 0.0); general_plan.harmonics.nalm()];
            general_plan
                .analysis_spin_into(
                    &map_first,
                    &map_second,
                    &mut general_e,
                    &mut general_b,
                    &mut general_workspace,
                )
                .unwrap();
            for (table_channel, recurrence_channel, channel) in [
                (&recovered_e, &general_e, "E"),
                (&recovered_b, &general_b, "B"),
            ] {
                for (&table, &recurrence) in table_channel.iter().zip(recurrence_channel) {
                    let error = (table - recurrence).norm();
                    assert!(
                        error <= 3e-10 * recurrence.norm().max(1.0),
                        "T255 spin-1 {channel} table/recurrence error={error:.3e}"
                    );
                }
            }
        }
        let coefficients = [(spin, 0), (255, 0), (128, 1), (255, 128), (255, 255)];
        let (expected_e, expected_b) = if spin == 1 {
            (
                [
                    Complex64::new(-0.235_214_804_154_751_46, 0.0),
                    Complex64::new(4.730_955_291_022_578_7e-5, 0.0),
                    Complex64::new(0.000_364_309_409_390_762_61, -0.000_310_485_887_894_616_06),
                    Complex64::new(-4.874_854_665_548_187_6e-5, -3.189_712_094_041_813_8e-5),
                    Complex64::new(9.320_378_481_313_17e-5, 0.000_822_533_069_129_493_44),
                ],
                [
                    Complex64::new(-0.133_655_294_352_013_43, 0.0),
                    Complex64::new(0.000_214_779_425_271_175_49, 0.0),
                    Complex64::new(-0.001_140_372_173_998_606_1, -0.001_170_201_806_136_655_1),
                    Complex64::new(5.480_444_806_653_299e-5, 3.575_031_575_613_239e-5),
                    Complex64::new(-0.000_235_951_559_473_843_78, -0.002_141_500_278_908_898_3),
                ],
            )
        } else {
            (
                [
                    Complex64::new(-0.237_673_805_510_415_23, 0.0),
                    Complex64::new(-0.000_486_172_907_263_484_44, 0.0),
                    Complex64::new(0.001_050_541_489_860_348_4, -0.001_127_992_219_361_837_9),
                    Complex64::new(-3.476_958_601_947_758e-5, 5.540_041_297_517_928e-5),
                    Complex64::new(0.002_141_824_249_688_625_3, -0.000_232_315_219_513_176_68),
                ],
                [
                    Complex64::new(-0.197_211_989_032_294_25, 0.0),
                    Complex64::new(0.000_119_337_432_581_061_4, 0.0),
                    Complex64::new(-0.000_345_143_456_478_464_04, -0.000_663_935_366_580_957_58),
                    Complex64::new(-3.440_626_319_093_324e-5, 4.723_233_723_130_058e-5),
                    Complex64::new(0.000_822_384_251_765_275_73, -9.439_566_581_699_641e-5),
                ],
            )
        };
        for (slot, &(degree, order)) in coefficients.iter().enumerate() {
            let index = plan.harmonics.index(degree, order).unwrap();
            assert_complex_close(recovered_e[index], expected_e[slot], 2e-10);
            assert_complex_close(recovered_b[index], expected_b[slot], 2e-10);
        }

        let mut adjoint_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut adjoint_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.adjoint_synthesis_spin_into(
            &map_first,
            &map_second,
            &mut adjoint_e,
            &mut adjoint_b,
            &mut workspace,
        )
        .unwrap();
        let mut synthesis_first = vec![0.0; plan.sample_count];
        let mut synthesis_second = vec![0.0; plan.sample_count];
        plan.synthesis_spin_into(
            &recovered_e,
            &recovered_b,
            &mut synthesis_first,
            &mut synthesis_second,
            &mut workspace,
        )
        .unwrap();
        assert_close(
            spin_map_inner_product(&synthesis_first, &synthesis_second, &map_first, &map_second),
            spin_harmonic_inner_product(
                plan.harmonics(),
                &recovered_e,
                &recovered_b,
                &adjoint_e,
                &adjoint_b,
            ),
            2e-8,
        );

        let mut analysis_adjoint_first = vec![0.0; plan.sample_count];
        let mut analysis_adjoint_second = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_spin_into(
            &recovered_e,
            &recovered_b,
            &mut analysis_adjoint_first,
            &mut analysis_adjoint_second,
            &mut workspace,
        )
        .unwrap();
        assert_close(
            spin_harmonic_inner_product(
                plan.harmonics(),
                &recovered_e,
                &recovered_b,
                &recovered_e,
                &recovered_b,
            ),
            spin_map_inner_product(
                &map_first,
                &map_second,
                &analysis_adjoint_first,
                &analysis_adjoint_second,
            ),
            2e-8,
        );

        let mut gradient_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.adjoint_synthesis_gradient_into(
            &map_first,
            &map_second,
            &mut gradient_adjoint,
            &mut workspace,
        )
        .unwrap();
        let mut gradient_first = vec![0.0; plan.sample_count];
        let mut gradient_second = vec![0.0; plan.sample_count];
        plan.synthesis_gradient_into(
            &recovered_e,
            &mut gradient_first,
            &mut gradient_second,
            &mut workspace,
        )
        .unwrap();
        assert_close(
            spin_map_inner_product(&gradient_first, &gradient_second, &map_first, &map_second),
            harmonic_inner_product(plan.harmonics(), &recovered_e, &gradient_adjoint),
            2e-8,
        );
    }
}

fn assert_cc_spin_coefficients(
    e: &[Complex64],
    b: &[Complex64],
    references: &[(usize, Complex64, Complex64)],
    tolerance: f64,
) {
    for &(index, expected_e, expected_b) in references {
        assert_complex_close(e[index], expected_e, tolerance);
        assert_complex_close(b[index], expected_b, tolerance);
    }
}

fn assert_cc_spin_map_samples(
    first: &[f64],
    second: &[f64],
    references: &[(usize, f64, f64)],
    tolerance: f64,
) {
    for &(index, expected_first, expected_second) in references {
        assert_close(first[index], expected_first, tolerance);
        assert_close(second[index], expected_second, tolerance);
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn clenshaw_curtis_spin_operations_match_ducc_and_adjoint_identities() {
    // DUCC references: ducc0==0.41.0, CC 4x5, phi0=0.37, one thread.
    for spin in [1, 2] {
        let plan = make_cc_plan(spin, 2, 2, 4, 5);
        let (e, b) = coefficients(&plan);
        let (input_first, input_second) = arbitrary_maps(&plan);
        let mut workspace = plan.workspace().unwrap();

        let mut synthesized_first = vec![0.0; plan.sample_count];
        let mut synthesized_second = vec![0.0; plan.sample_count];
        plan.synthesis_spin_into(
            &e,
            &b,
            &mut synthesized_first,
            &mut synthesized_second,
            &mut workspace,
        )
        .unwrap();
        let synthesis_references = if spin == 1 {
            [
                (0, -0.130_913_993_965_584_34, 0.171_860_611_462_911_54),
                (5, -0.016_428_603_075_617_221, -0.032_805_389_433_844_825),
                (19, -0.000_928_712_053_871_821_9, 0.022_129_785_676_624_596),
            ]
        } else {
            [
                (0, -0.058_892_455_413_158_004, 0.156_275_368_893_607_24),
                (5, -0.104_375_050_603_352_67, 0.177_290_419_041_157_17),
                (19, 0.097_487_802_581_672_4, 0.098_471_217_532_856_2),
            ]
        };
        assert_cc_spin_map_samples(
            &synthesized_first,
            &synthesized_second,
            &synthesis_references,
            8e-14,
        );

        let mut analyzed_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut analyzed_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.analysis_spin_into(
            &input_first,
            &input_second,
            &mut analyzed_e,
            &mut analyzed_b,
            &mut workspace,
        )
        .unwrap();
        let analysis_references = if spin == 1 {
            [
                (
                    1,
                    Complex64::new(-2.612_955_077_711_654, 0.0),
                    Complex64::new(-3.001_662_136_091_308_5, 0.0),
                ),
                (
                    3,
                    Complex64::new(0.021_002_053_606_507_8, -0.045_793_602_877_552_11),
                    Complex64::new(0.329_592_441_937_579_47, 0.209_604_689_959_377_64),
                ),
                (
                    5,
                    Complex64::new(-0.012_393_768_821_333_053, 0.004_749_812_096_456_83),
                    Complex64::new(-0.184_929_371_801_667_8, -0.129_315_391_538_746_91),
                ),
            ]
        } else {
            [
                (
                    2,
                    Complex64::new(-2.534_258_249_101_103_6, 0.0),
                    Complex64::new(-2.912_496_978_593_333_9, 0.0),
                ),
                (
                    4,
                    Complex64::new(0.021_151_007_160_142_413, -0.034_840_695_347_129_046),
                    Complex64::new(0.332_274_115_139_816_86, 0.195_208_217_446_319_17),
                ),
                (
                    5,
                    Complex64::new(0.124_251_712_561_700_26, -0.151_791_468_343_976_8),
                    Complex64::new(0.013_886_323_920_175_251, 0.015_301_143_441_011_789),
                ),
            ]
        };
        assert_cc_spin_coefficients(&analyzed_e, &analyzed_b, &analysis_references, 8e-13);

        let mut adjoint_synthesis_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut adjoint_synthesis_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.adjoint_synthesis_spin_into(
            &input_first,
            &input_second,
            &mut adjoint_synthesis_e,
            &mut adjoint_synthesis_b,
            &mut workspace,
        )
        .unwrap();
        let synthesis_adjoint_references = if spin == 1 {
            [
                (
                    1,
                    Complex64::new(-2.339_239_031_408_69, 0.0),
                    Complex64::new(-2.687_227_686_285_246_4, 0.0),
                ),
                (
                    3,
                    Complex64::new(0.032_028_328_732_417_545, -0.110_290_988_876_756_03),
                    Complex64::new(0.505_419_123_211_926_7, 0.378_512_676_833_292_98),
                ),
                (
                    5,
                    Complex64::new(-0.011_687_714_987_987_126, 0.005_468_360_226_964_545_5),
                    Complex64::new(-0.164_640_419_195_042_44, -0.117_389_292_647_135_45),
                ),
            ]
        } else {
            [
                (
                    2,
                    Complex64::new(-2.264_958_452_861_051, 0.0),
                    Complex64::new(-2.601_897_019_112_555_7, 0.0),
                ),
                (
                    4,
                    Complex64::new(0.018_919_777_741_599_818, -0.033_495_276_055_368_756),
                    Complex64::new(0.296_712_078_719_778_34, 0.177_698_223_104_298_2),
                ),
                (
                    5,
                    Complex64::new(0.265_897_033_575_491_05, -0.274_614_979_563_999),
                    Complex64::new(0.047_895_818_012_903_844, 0.039_117_078_080_879_35),
                ),
            ]
        };
        assert_cc_spin_coefficients(
            &adjoint_synthesis_e,
            &adjoint_synthesis_b,
            &synthesis_adjoint_references,
            8e-13,
        );

        let mut adjoint_analysis_first = vec![0.0; plan.sample_count];
        let mut adjoint_analysis_second = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_spin_into(
            &e,
            &b,
            &mut adjoint_analysis_first,
            &mut adjoint_analysis_second,
            &mut workspace,
        )
        .unwrap();
        let analysis_adjoint_references = if spin == 1 {
            [
                (0, -0.030_759_606_053_735_657, 0.036_192_005_621_084_9),
                (5, -0.005_547_105_210_100_101, -0.052_974_247_507_400_68),
                (19, -0.002_069_536_116_825_507_5, 0.008_025_389_616_706_956),
            ]
        } else {
            [
                (0, -0.006_838_814_213_977_9, 0.026_952_561_786_282_143),
                (5, -0.111_987_949_435_876_78, 0.181_364_806_762_543_88),
                (19, 0.020_254_357_559_741_532, 0.018_396_658_748_369_588),
            ]
        };
        assert_cc_spin_map_samples(
            &adjoint_analysis_first,
            &adjoint_analysis_second,
            &analysis_adjoint_references,
            8e-13,
        );
        assert_close(
            spin_map_inner_product(
                &synthesized_first,
                &synthesized_second,
                &input_first,
                &input_second,
            ),
            spin_harmonic_inner_product(
                plan.harmonics(),
                &e,
                &b,
                &adjoint_synthesis_e,
                &adjoint_synthesis_b,
            ),
            2e-12,
        );
        assert_close(
            spin_harmonic_inner_product(plan.harmonics(), &analyzed_e, &analyzed_b, &e, &b),
            spin_map_inner_product(
                &input_first,
                &input_second,
                &adjoint_analysis_first,
                &adjoint_analysis_second,
            ),
            2e-12,
        );
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn clenshaw_curtis_gradient_modes_match_ducc_and_adjoint_identities() {
    // DUCC references: ducc0==0.41.0, CC 4x5, phi0=0.37, one thread.
    for spin in [1, 2] {
        let plan = make_cc_plan(spin, 2, 2, 4, 5);
        let (e, _) = coefficients(&plan);
        let (first_input, second_input) = arbitrary_maps(&plan);
        let mut workspace = plan.workspace().unwrap();
        let mut first_output = vec![0.0; plan.sample_count];
        let mut second_output = vec![0.0; plan.sample_count];

        plan.synthesis_gradient_into(&e, &mut first_output, &mut second_output, &mut workspace)
            .unwrap();
        let map_references = if spin == 1 {
            [
                (0, -0.139_001_236_673_356_7, 0.065_415_120_289_367_34),
                (5, 0.009_134_288_207_353_32, -0.034_043_948_310_996_56),
                (19, -0.031_053_446_309_564_477, 0.021_109_363_924_096_514),
            ]
        } else {
            [
                (0, -0.097_820_836_943_549_01, 0.089_319_182_349_286_58),
                (5, -0.123_730_410_826_428_91, 0.082_948_977_740_022_52),
                (19, 0.026_638_221_359_579_07, 0.129_758_381_766_572_23),
            ]
        };
        assert_cc_spin_map_samples(&first_output, &second_output, &map_references, 8e-14);

        let mut adjoint_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.adjoint_synthesis_gradient_into(
            &first_input,
            &second_input,
            &mut adjoint_e,
            &mut workspace,
        )
        .unwrap();
        let adjoint_references: &[(usize, Complex64)] = if spin == 1 {
            &[
                (1, Complex64::new(-2.339_239_031_408_69, 0.0)),
                (2, Complex64::new(0.217_924_448_403_946_23, 0.0)),
                (
                    3,
                    Complex64::new(0.032_028_328_732_417_545, -0.110_290_988_876_756_04),
                ),
                (
                    4,
                    Complex64::new(0.206_325_639_584_846_32, -0.132_032_852_262_769_56),
                ),
                (
                    5,
                    Complex64::new(-0.011_687_714_987_987_126, 0.005_468_360_226_964_545_5),
                ),
            ]
        } else {
            &[
                (2, Complex64::new(-2.264_958_452_861_051, 0.0)),
                (
                    4,
                    Complex64::new(0.018_919_777_741_599_82, -0.033_495_276_055_368_756),
                ),
                (
                    5,
                    Complex64::new(0.265_897_033_575_491_05, -0.274_614_979_563_999),
                ),
            ]
        };
        for &(index, expected) in adjoint_references {
            assert_complex_close(adjoint_e[index], expected, 8e-13);
        }

        let zero = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        assert_close(
            spin_map_inner_product(&first_output, &second_output, &first_input, &second_input),
            spin_harmonic_inner_product(plan.harmonics(), &e, &zero, &adjoint_e, &zero),
            2e-12,
        );
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn fejer1_spin_operations_match_ducc_and_adjoint_identities() {
    // DUCC references: ducc0==0.41.0, F1 3x5, phi0=0.37, one thread.
    for spin in [1, 2] {
        let plan = make_f1_plan(spin, 2, 2, 3, 5);
        let (e, b) = coefficients(&plan);
        let (first_input, second_input) = arbitrary_maps(&plan);
        let mut workspace = plan.workspace().unwrap();

        let mut first_output = vec![0.0; plan.sample_count];
        let mut second_output = vec![0.0; plan.sample_count];
        plan.synthesis_spin_into(
            &e,
            &b,
            &mut first_output,
            &mut second_output,
            &mut workspace,
        )
        .unwrap();
        let synthesis_references = if spin == 1 {
            [
                (0, -0.098_983_579_226_132_7, 0.090_368_487_589_850_75),
                (3, -0.117_184_231_245_945_29, -0.189_695_665_672_468_15),
                (7, -0.047_171_165_054_997_41, 0.155_840_669_591_128_16),
                (14, 0.086_231_450_561_689_27, 0.040_397_647_289_779_584),
            ]
        } else {
            [
                (0, -0.097_153_146_980_529_71, 0.195_088_588_456_651_36),
                (3, 0.103_197_912_334_272_11, 0.031_486_287_432_054_77),
                (7, -0.052_454_620_791_955_175, 0.073_561_786_056_501_4),
                (14, 0.073_252_121_678_326_48, 0.050_442_202_943_089_45),
            ]
        };
        assert_cc_spin_map_samples(&first_output, &second_output, &synthesis_references, 1e-13);

        let mut analyzed_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut analyzed_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.analysis_spin_into(
            &first_input,
            &second_input,
            &mut analyzed_e,
            &mut analyzed_b,
            &mut workspace,
        )
        .unwrap();
        let analysis_references: &[(usize, Complex64, Complex64)] = if spin == 1 {
            &[
                (0, Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
                (
                    1,
                    Complex64::new(-2.613_581_787_234_785, 0.0),
                    Complex64::new(-3.344_389_597_145_090_8, 0.0),
                ),
                (
                    2,
                    Complex64::new(0.416_401_365_576_376_6, 0.0),
                    Complex64::new(-0.077_155_117_237_335_41, 0.0),
                ),
                (
                    3,
                    Complex64::new(0.052_573_954_740_478_29, -0.033_812_883_596_466_066),
                    Complex64::new(0.354_425_598_444_193_47, 0.206_728_757_566_070_8),
                ),
                (
                    4,
                    Complex64::new(-0.047_003_438_697_951_185, 0.165_663_340_580_323_5),
                    Complex64::new(0.015_808_195_321_049_82, -0.025_028_977_380_949_25),
                ),
                (
                    5,
                    Complex64::new(-0.034_656_633_671_342_536, -0.002_248_879_301_036_772),
                    Complex64::new(-0.212_707_232_218_928_32, -0.154_393_997_049_120_66),
                ),
            ]
        } else {
            &[
                (0, Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
                (1, Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
                (
                    2,
                    Complex64::new(-2.311_399_538_597_643, 0.0),
                    Complex64::new(-2.959_473_335_218_845, 0.0),
                ),
                (3, Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
                (
                    4,
                    Complex64::new(0.057_418_533_880_398_46, -0.035_851_891_484_813_54),
                    Complex64::new(0.386_913_881_166_283_2, 0.224_010_756_591_424_7),
                ),
                (
                    5,
                    Complex64::new(0.128_211_784_233_880_16, -0.163_905_551_395_658_02),
                    Complex64::new(0.002_665_160_732_414_564, 0.029_348_277_404_468_725),
                ),
            ]
        };
        assert_cc_spin_coefficients(&analyzed_e, &analyzed_b, analysis_references, 1e-12);

        let mut adjoint_synthesis_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut adjoint_synthesis_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.adjoint_synthesis_spin_into(
            &first_input,
            &second_input,
            &mut adjoint_synthesis_e,
            &mut adjoint_synthesis_b,
            &mut workspace,
        )
        .unwrap();
        let synthesis_adjoint_references: &[(usize, Complex64, Complex64)] = if spin == 1 {
            &[
                (0, Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
                (
                    1,
                    Complex64::new(-2.460_490_894_976_722, 0.0),
                    Complex64::new(-3.147_529_590_926_678_3, 0.0),
                ),
                (
                    2,
                    Complex64::new(0.465_977_359_980_672_3, 0.0),
                    Complex64::new(-0.086_341_066_123_757_76, 0.0),
                ),
                (
                    3,
                    Complex64::new(0.061_675_585_208_963_934, -0.050_867_044_797_425_84),
                    Complex64::new(0.417_565_664_155_188_8, 0.260_904_225_379_543_9),
                ),
                (
                    4,
                    Complex64::new(0.056_081_249_068_286_446, 0.018_257_026_128_022_033),
                    Complex64::new(0.040_754_946_237_229_65, -0.003_388_159_719_762_755),
                ),
                (
                    5,
                    Complex64::new(-0.033_385_747_048_420_04, -0.000_398_801_863_341_388_4),
                    Complex64::new(-0.198_760_482_673_567_7, -0.147_831_957_546_748_08),
                ),
            ]
        } else {
            &[
                (0, Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
                (1, Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
                (
                    2,
                    Complex64::new(-2.067_533_006_275_986_5, 0.0),
                    Complex64::new(-2.646_551_304_731_979, 0.0),
                ),
                (3, Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0)),
                (
                    4,
                    Complex64::new(0.053_778_997_201_116_12, -0.036_997_860_053_756_99),
                    Complex64::new(0.362_932_718_197_201_6, 0.215_423_307_201_582_56),
                ),
                (
                    5,
                    Complex64::new(0.179_518_686_136_898_7, -0.214_676_347_126_946_17),
                    Complex64::new(0.009_036_843_392_207_31, 0.041_690_612_110_451_94),
                ),
            ]
        };
        assert_cc_spin_coefficients(
            &adjoint_synthesis_e,
            &adjoint_synthesis_b,
            synthesis_adjoint_references,
            1e-12,
        );
        assert_close(
            spin_map_inner_product(&first_output, &second_output, &first_input, &second_input),
            spin_harmonic_inner_product(
                plan.harmonics(),
                &e,
                &b,
                &adjoint_synthesis_e,
                &adjoint_synthesis_b,
            ),
            3e-12,
        );

        let mut adjoint_analysis_first = vec![0.0; plan.sample_count];
        let mut adjoint_analysis_second = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_spin_into(
            &e,
            &b,
            &mut adjoint_analysis_first,
            &mut adjoint_analysis_second,
            &mut workspace,
        )
        .unwrap();
        let analysis_adjoint_references = if spin == 1 {
            [
                (0, -0.047_042_696_804_017_806, 0.026_887_199_545_021_75),
                (3, -0.106_713_137_261_226_85, -0.102_157_325_893_736_05),
                (7, -0.039_471_478_607_176_695, 0.184_827_588_456_237_15),
                (14, 0.081_924_693_873_147, 0.030_783_418_598_862_095),
            ]
        } else {
            [
                (0, -0.065_334_132_124_619_47, 0.125_974_470_270_617_56),
                (3, 0.051_910_681_821_191_074, -0.001_399_536_758_166_690_5),
                (7, -0.083_749_517_018_415_4, 0.100_056_255_720_047_95),
                (14, 0.035_971_062_500_127_68, 0.017_903_107_303_638_977),
            ]
        };
        assert_cc_spin_map_samples(
            &adjoint_analysis_first,
            &adjoint_analysis_second,
            &analysis_adjoint_references,
            1e-12,
        );
        assert_close(
            spin_harmonic_inner_product(plan.harmonics(), &analyzed_e, &analyzed_b, &e, &b),
            spin_map_inner_product(
                &first_input,
                &second_input,
                &adjoint_analysis_first,
                &adjoint_analysis_second,
            ),
            3e-12,
        );

        let mut gradient_first = vec![0.0; plan.sample_count];
        let mut gradient_second = vec![0.0; plan.sample_count];
        plan.synthesis_gradient_into(
            &e,
            &mut gradient_first,
            &mut gradient_second,
            &mut workspace,
        )
        .unwrap();
        let gradient_references = if spin == 1 {
            [
                (0, -0.087_623_467_312_167_55, 0.014_832_140_398_848_83),
                (3, -0.015_538_177_488_051_037, -0.183_508_469_558_859_63),
                (7, -0.114_947_121_112_837_34, 0.080_409_927_426_210_39),
                (14, 0.029_313_997_140_149_317, 0.077_073_013_538_024_69),
            ]
        } else {
            [
                (0, -0.130_803_280_971_272_64, 0.099_459_068_600_364_14),
                (3, 0.057_479_634_101_710_766, 0.061_801_970_518_028_88),
                (7, -0.092_352_623_893_577_68, 0.013_803_004_672_628_016),
                (14, 0.044_109_913_700_552_93, 0.079_100_801_969_590_96),
            ]
        };
        assert_cc_spin_map_samples(
            &gradient_first,
            &gradient_second,
            &gradient_references,
            1e-13,
        );
        let mut gradient_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.adjoint_synthesis_gradient_into(
            &first_input,
            &second_input,
            &mut gradient_adjoint,
            &mut workspace,
        )
        .unwrap();
        for (&actual, &expected) in gradient_adjoint.iter().zip(&adjoint_synthesis_e) {
            assert_complex_close(actual, expected, 1e-12);
        }
        let zero = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        assert_close(
            spin_map_inner_product(
                &gradient_first,
                &gradient_second,
                &first_input,
                &second_input,
            ),
            spin_harmonic_inner_product(plan.harmonics(), &e, &zero, &gradient_adjoint, &zero),
            3e-12,
        );
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn fejer1_spin_analysis_accepts_minimum_even_longitudes() {
    // DUCC references: ducc0==0.41.0, F1 3x6, phi0=0.37, one thread.
    for spin in [1, 2] {
        let plan = make_f1_plan(spin, 2, 2, 3, 6);
        let (first, second) = arbitrary_maps(&plan);
        let (test_e, test_b) = coefficients(&plan);
        let mut workspace = plan.workspace().unwrap();
        let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.analysis_spin_into(&first, &second, &mut e, &mut b, &mut workspace)
            .unwrap();
        let references: &[(usize, Complex64, Complex64)] = if spin == 1 {
            &[
                (
                    1,
                    Complex64::new(-2.782_215_081_280_932, 0.0),
                    Complex64::new(-3.276_593_225_917_481_7, 0.0),
                ),
                (
                    2,
                    Complex64::new(0.378_765_260_157_021_3, 0.0),
                    Complex64::new(-0.039_664_167_427_928_24, 0.0),
                ),
                (
                    3,
                    Complex64::new(0.082_515_605_747_329_87, -0.055_128_868_172_035_71),
                    Complex64::new(0.359_306_035_718_884_86, 0.217_679_985_313_724_12),
                ),
                (
                    4,
                    Complex64::new(-0.043_435_663_782_372_54, 0.164_857_731_800_652_02),
                    Complex64::new(0.019_366_484_209_212_46, -0.043_102_027_227_444_61),
                ),
                (
                    5,
                    Complex64::new(-0.053_674_198_440_110_77, 0.007_019_993_084_877_839),
                    Complex64::new(-0.213_596_827_830_075_6, -0.113_716_309_791_530_07),
                ),
            ]
        } else {
            &[
                (
                    2,
                    Complex64::new(-2.460_671_086_141_520_6, 0.0),
                    Complex64::new(-2.899_410_239_602_208, 0.0),
                ),
                (
                    4,
                    Complex64::new(0.090_243_762_739_031_98, -0.058_670_075_372_457_37),
                    Complex64::new(0.392_142_677_549_834_8, 0.235_673_294_014_001_9),
                ),
                (
                    5,
                    Complex64::new(0.097_410_495_864_768_12, -0.165_812_137_819_841_42),
                    Complex64::new(0.011_926_363_562_435_314, 0.043_555_203_854_442_29),
                ),
            ]
        };
        assert_cc_spin_coefficients(&e, &b, references, 1e-12);

        let mut adjoint_first = vec![0.0; plan.sample_count];
        let mut adjoint_second = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_spin_into(
            &test_e,
            &test_b,
            &mut adjoint_first,
            &mut adjoint_second,
            &mut workspace,
        )
        .unwrap();
        let map_references = if spin == 1 {
            [
                (0, -0.039_202_247_336_681_5, 0.022_405_999_620_851_456),
                (6, 0.023_049_707_031_297_315, -0.105_432_639_454_428_4),
                (17, 0.055_251_603_269_724_06, 0.031_932_462_044_624_61),
            ]
        } else {
            [
                (0, -0.054_445_110_103_849_56, 0.104_978_725_225_514_62),
                (6, -0.099_464_187_296_626_87, 0.120_539_897_138_615_63),
                (17, 0.018_243_879_171_691_74, 0.028_142_641_759_291_005),
            ]
        };
        assert_cc_spin_map_samples(&adjoint_first, &adjoint_second, &map_references, 1e-12);
        assert_close(
            spin_harmonic_inner_product(plan.harmonics(), &e, &b, &test_e, &test_b),
            spin_map_inner_product(&first, &second, &adjoint_first, &adjoint_second),
            3e-12,
        );
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn fejer1_oversampled_spin_analysis_matches_ducc() {
    // DUCC references: ducc0==0.41.0, F1 7x5, phi0=0.37, one thread.
    for spin in [1, 2] {
        let plan = make_f1_plan(spin, 2, 2, 7, 5);
        let (first, second) = arbitrary_maps(&plan);
        let (test_e, test_b) = coefficients(&plan);
        let mut workspace = plan.workspace().unwrap();
        let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.analysis_spin_into(&first, &second, &mut e, &mut b, &mut workspace)
            .unwrap();
        let references: &[(usize, Complex64, Complex64)] = if spin == 1 {
            &[
                (
                    1,
                    Complex64::new(-3.203_895_321_480_334_5, 0.0),
                    Complex64::new(-2.708_826_378_284_028_7, 0.0),
                ),
                (
                    2,
                    Complex64::new(0.477_018_463_371_304_3, 0.0),
                    Complex64::new(-0.388_578_593_220_819_5, 0.0),
                ),
                (
                    3,
                    Complex64::new(-0.069_672_890_646_803_28, -0.104_044_218_935_535_63),
                    Complex64::new(0.227_574_841_630_980_8, 0.229_031_728_333_504_85),
                ),
                (
                    4,
                    Complex64::new(-0.006_867_673_128_442_871, 0.128_020_163_412_562_28),
                    Complex64::new(0.051_147_075_560_120_034, 0.042_487_852_862_102_21),
                ),
                (
                    5,
                    Complex64::new(0.032_736_999_796_070_88, 0.053_175_504_687_364_48),
                    Complex64::new(-0.129_230_975_454_172_84, -0.122_160_767_610_150_92),
                ),
            ]
        } else {
            &[
                (
                    2,
                    Complex64::new(-3.044_067_060_915_384_4, 0.0),
                    Complex64::new(-2.575_372_784_629_255_6, 0.0),
                ),
                (
                    4,
                    Complex64::new(-0.073_235_537_415_343_78, -0.092_175_904_212_888_14),
                    Complex64::new(0.237_825_729_230_251_5, 0.220_806_439_073_784_67),
                ),
                (
                    5,
                    Complex64::new(0.121_434_101_714_636_19, -0.095_755_778_193_932_41),
                    Complex64::new(0.063_969_398_139_843_72, -0.016_831_251_204_922_67),
                ),
            ]
        };
        assert_cc_spin_coefficients(&e, &b, references, 1e-12);

        let mut adjoint_first = vec![0.0; plan.sample_count];
        let mut adjoint_second = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_spin_into(
            &test_e,
            &test_b,
            &mut adjoint_first,
            &mut adjoint_second,
            &mut workspace,
        )
        .unwrap();
        let map_references = if spin == 1 {
            [
                (0, -0.013_912_921_383_420_655, 0.015_089_878_213_539_81),
                (5, -0.028_135_046_874_246_8, 0.021_269_690_587_890_234),
                (34, 0.006_039_559_673_192_504, 0.003_348_474_270_680_277),
            ]
        } else {
            [
                (0, -0.010_042_435_705_743_084, 0.021_173_231_967_596_687),
                (5, -0.035_618_697_304_531_37, 0.069_252_096_817_917_68),
                (34, 0.010_643_513_709_976_956, 0.007_527_126_895_420_223_5),
            ]
        };
        assert_cc_spin_map_samples(&adjoint_first, &adjoint_second, &map_references, 1e-12);
        assert_close(
            spin_harmonic_inner_product(plan.harmonics(), &e, &b, &test_e, &test_b),
            spin_map_inner_product(&first, &second, &adjoint_first, &adjoint_second),
            3e-12,
        );
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn clenshaw_curtis_spin_analysis_accepts_minimum_even_longitudes() {
    // DUCC references: ducc0==0.41.0, CC 4x6, phi0=0.37, one thread.
    for spin in [1, 2] {
        let plan = make_cc_plan(spin, 2, 2, 4, 6);
        let (first, second) = arbitrary_maps(&plan);
        let mut workspace = plan.workspace().unwrap();
        let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.analysis_spin_into(&first, &second, &mut e, &mut b, &mut workspace)
            .unwrap();
        let reference_indices = if spin == 1 { [1, 5] } else { [2, 5] };
        let references = if spin == 1 {
            [
                (-2.745_417_965_488_028_4, -2.960_933_933_046_018_9),
                (-0.026_862_782_484_817_417, -0.179_358_063_239_723_45),
            ]
        } else {
            [
                (-2.662_837_082_796_514_2, -2.872_925_433_354_386_1),
                (0.095_858_214_893_881_23, 0.020_663_181_787_752_796),
            ]
        };
        for (slot, &index) in reference_indices.iter().enumerate() {
            assert_close(e[index].re, references[slot].0, 8e-13);
            assert_close(b[index].re, references[slot].1, 8e-13);
            if index == 5 {
                let (expected_e_im, expected_b_im) = if spin == 1 {
                    (0.008_161_696_566_889_63, -0.093_329_874_742_590_76)
                } else {
                    (-0.149_357_800_132_309_34, 0.026_001_781_896_984_398)
                };
                assert_close(e[index].im, expected_e_im, 8e-13);
                assert_close(b[index].im, expected_b_im, 8e-13);
            }
        }
        let (test_e, test_b) = coefficients(&plan);
        let mut adjoint_first = vec![0.0; plan.sample_count];
        let mut adjoint_second = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_spin_into(
            &test_e,
            &test_b,
            &mut adjoint_first,
            &mut adjoint_second,
            &mut workspace,
        )
        .unwrap();
        assert_close(
            spin_harmonic_inner_product(plan.harmonics(), &e, &b, &test_e, &test_b),
            spin_map_inner_product(&first, &second, &adjoint_first, &adjoint_second),
            2e-12,
        );
    }
}

#[test]
#[allow(clippy::excessive_precision)]
fn clenshaw_curtis_oversampled_spin_analysis_and_adjoint_match_ducc() {
    // DUCC references: ducc0==0.41.0, CC 7x5, phi0=0.37, one thread.
    for spin in [1, 2] {
        let plan = make_cc_plan(spin, 2, 2, 7, 5);
        let (first, second) = arbitrary_maps(&plan);
        let (e, b) = coefficients(&plan);
        let mut workspace = plan.workspace().unwrap();
        let mut analyzed_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut analyzed_b = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        plan.analysis_spin_into(
            &first,
            &second,
            &mut analyzed_e,
            &mut analyzed_b,
            &mut workspace,
        )
        .unwrap();
        let analysis_indices = if spin == 1 { [1, 3, 5] } else { [2, 4, 5] };
        let analysis_references = if spin == 1 {
            [
                (
                    Complex64::new(-3.201_290_644_015_710_7, 0.0),
                    Complex64::new(-2.708_664_055_064_725_4, 0.0),
                ),
                (
                    Complex64::new(-0.071_123_529_631_083_82, -0.096_929_917_301_682_57),
                    Complex64::new(0.231_689_503_121_457_97, 0.223_052_640_302_583_62),
                ),
                (
                    Complex64::new(0.034_968_400_629_645_326, 0.048_876_147_346_835_576),
                    Complex64::new(-0.131_753_492_287_878_36, -0.117_856_284_954_902_72),
                ),
            ]
        } else {
            [
                (
                    Complex64::new(-3.056_854_799_859_842, 0.0),
                    Complex64::new(-2.587_440_593_321_033_2, 0.0),
                ),
                (
                    Complex64::new(-0.074_068_722_658_374_37, -0.084_167_475_868_191_27),
                    Complex64::new(0.239_869_490_508_036_66, 0.212_826_756_809_062_38),
                ),
                (
                    Complex64::new(0.117_119_618_474_327_16, -0.100_121_944_841_738_64),
                    Complex64::new(0.059_015_335_553_978_27, -0.019_955_044_264_876_733),
                ),
            ]
        };
        for (slot, &index) in analysis_indices.iter().enumerate() {
            assert_complex_close(analyzed_e[index], analysis_references[slot].0, 1e-12);
            assert_complex_close(analyzed_b[index], analysis_references[slot].1, 1e-12);
        }

        let mut adjoint_first = vec![0.0; plan.sample_count];
        let mut adjoint_second = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_spin_into(
            &e,
            &b,
            &mut adjoint_first,
            &mut adjoint_second,
            &mut workspace,
        )
        .unwrap();
        let map_indices = [0, 5, 34];
        let map_references = if spin == 1 {
            [
                (-0.004_700_325_047_935_751, 0.006_170_468_964_723_809),
                (-0.031_590_205_493_114_08, 0.028_840_734_143_826_846),
                (-0.000_033_344_399_608_513_03, 0.000_794_545_966_939_429),
            ]
        } else {
            [
                (-0.002_114_469_774_603_898_7, 0.005_610_897_724_036_468),
                (-0.031_006_030_509_425_925, 0.062_261_727_115_643_76),
                (0.003_500_193_878_916_505, 0.003_535_502_326_757_304_6),
            ]
        };
        for (slot, &index) in map_indices.iter().enumerate() {
            assert_close(adjoint_first[index], map_references[slot].0, 1e-12);
            assert_close(adjoint_second[index], map_references[slot].1, 1e-12);
        }
        assert_close(
            spin_harmonic_inner_product(plan.harmonics(), &analyzed_e, &analyzed_b, &e, &b),
            spin_map_inner_product(&first, &second, &adjoint_first, &adjoint_second),
            3e-12,
        );
    }
}

#[test]
#[ignore = "M3 high-bandlimit spin check; run this check in release mode"]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain printed DUCC 0.41.0 reference values."
)]
fn m3_spin_t511_all_grids_matches_ducc_and_roundtrips() {
    // DUCC 0.41.0 references use a^E_(511,511)=1, phi0=0.37, nthreads=1.
    // Synthesis samples use the middle ring and j=[0,1,17,nphi/2,nphi-1].
    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let degree = 511;
        let nphi = 2 * degree + 3;
        let ntheta = degree + usize::from(kind == GridKind::ClenshawCurtis) + 1;
        for spin in [1, 2] {
            let plan = ShtPlan::new(
                GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
                HarmonicSpec::new(degree, degree, spin).unwrap(),
            )
            .unwrap();
            let high_mode = plan.harmonics().index(degree, degree).unwrap();
            let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            e[high_mode] = Complex64::new(1.0, 0.0);
            let mut workspace = plan.workspace().unwrap();
            let mut first = vec![0.0; plan.sample_count];
            let mut second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
                .unwrap();

            let (expected_first, expected_second, expected_analysis) = match (kind, spin) {
                (GridKind::GaussLegendre, 1) => (
                    [
                        0.0073096418437780695,
                        -0.0073528349673834753,
                        -0.0079570606097257111,
                        -0.0047647338479410873,
                        -0.0072658307214716654,
                    ],
                    [
                        1.543633476857506,
                        -1.5216395617799534,
                        -1.1535371487416379,
                        2.3777824527391807,
                        -1.5654968843802561,
                    ],
                    [
                        (
                            Complex64::new(-1.1316913690695762e-6, 0.0),
                            Complex64::new(1.7844377801259401e-6, 0.0),
                        ),
                        (
                            Complex64::new(0.00011803286257438613, 0.00014519236565101198),
                            Complex64::new(0.00022277687715475659, 0.00032400522404114792),
                        ),
                    ],
                ),
                (GridKind::GaussLegendre, 2) => (
                    [
                        2.3779542916519314,
                        -2.3920057699380743,
                        -2.588570935515492,
                        -1.5500512260987234,
                        -2.3637017675834264,
                    ],
                    [
                        0.009434692505902981,
                        -0.00930026563004895,
                        -0.0070504225618825897,
                        0.014533013583765925,
                        -0.00956832171918509,
                    ],
                    [
                        (
                            Complex64::new(-2.3539237098010763e-7, 0.0),
                            Complex64::new(-1.6298360556342589e-9, 0.0),
                        ),
                        (
                            Complex64::new(-0.0003177276448436924, 0.00022984527855318484),
                            Complex64::new(0.00013365941010512702, -0.00013358471060580937),
                        ),
                    ],
                ),
                (GridKind::ClenshawCurtis, 1) => (
                    [
                        -1.4638351689042394e-16,
                        1.4724850610247736e-16,
                        1.5934878083710319e-16,
                        9.541897026078629e-17,
                        1.4550615158317755e-16,
                    ],
                    [
                        1.5473356517042232,
                        -1.525288987499108,
                        -1.1563037356813819,
                        2.3834852095914214,
                        -1.5692514953516161,
                    ],
                    [
                        (
                            Complex64::new(-1.3145826901093299e-6, 0.0),
                            Complex64::new(1.6356255846980669e-6, 0.0),
                        ),
                        (
                            Complex64::new(0.00012694718589213188, 0.0001562880938019369),
                            Complex64::new(0.00020473299652962155, 0.00029865779813071985),
                        ),
                    ],
                ),
                (GridKind::ClenshawCurtis, 2) => (
                    [
                        2.3836238728304733,
                        -2.397708853020796,
                        -2.594742674436985,
                        -1.5537468990088863,
                        -2.3693373675191456,
                    ],
                    [
                        -1.8893907695488211e-16,
                        1.8624704541003864e-16,
                        1.4119170605194811e-16,
                        -2.910380142406873e-16,
                        1.9161513451541545e-16,
                    ],
                    [
                        (
                            Complex64::new(-2.6637924093299877e-7, 0.0),
                            Complex64::new(-8.353995010094624e-9, 0.0),
                        ),
                        (
                            Complex64::new(-0.0002931680410603763, 0.00021085828219159693),
                            Complex64::new(0.00014393722381960994, -0.00014361458543622882),
                        ),
                    ],
                ),
                (GridKind::Fejer1, 1) => (
                    [
                        0.007316749359380681,
                        -0.007359984481733292,
                        -0.007964797641668348,
                        -0.004769366827352432,
                        -0.007272895637417275,
                    ],
                    [
                        1.5436262476571632,
                        -1.5216324355823123,
                        -1.1535317464545822,
                        2.3777713170220696,
                        -1.5654895527884094,
                    ],
                    [
                        (
                            Complex64::new(-1.2338013284382414e-6, 0.0),
                            Complex64::new(1.7872292461705122e-6, 0.0),
                        ),
                        (
                            Complex64::new(0.00011824949428358399, 0.0001455047912520482),
                            Complex64::new(0.00022303588228052648, 0.00032438718123881317),
                        ),
                    ],
                ),
                (GridKind::Fejer1, 2) => (
                    [
                        2.377943220638037,
                        -2.391994633504877,
                        -2.5885588839361593,
                        -1.5500440095434171,
                        -2.3636907629248403,
                    ],
                    [
                        0.009443866396981063,
                        -0.009309308810187987,
                        -0.0070572780909416146,
                        0.014547144863992632,
                        -0.009577625545589284,
                    ],
                    [
                        (
                            Complex64::new(-2.3021062515709845e-7, 0.0),
                            Complex64::new(-1.0899438901848854e-8, 0.0),
                        ),
                        (
                            Complex64::new(-0.000318105900092426, 0.00023011156140126908),
                            Complex64::new(0.00013396577697340109, -0.000133809953096526),
                        ),
                    ],
                ),
                _ => unreachable!(),
            };

            let middle_ring = ntheta / 2;
            let longitudes = [0, 1, 17, nphi / 2, nphi - 1];
            let (mut synthesis_max_error, mut synthesis_error_squares) = (0.0_f64, 0.0_f64);
            let mut synthesis_sample_count = 0usize;
            for (slot, longitude) in longitudes.into_iter().enumerate() {
                let index = middle_ring * nphi + longitude;
                for (actual, expected) in [
                    (first[index], expected_first[slot]),
                    (second[index], expected_second[slot]),
                ] {
                    let error = (actual - expected).abs();
                    synthesis_max_error = synthesis_max_error.max(error);
                    synthesis_error_squares += error * error;
                    synthesis_sample_count += 1;
                    assert!(
                        error <= 8e-11,
                        "{kind:?} spin {spin} T{degree} synthesis error={error:.3e}"
                    );
                }
            }

            let mut arbitrary_first = vec![0.0; plan.sample_count];
            let mut arbitrary_second = vec![0.0; plan.sample_count];
            for i in 0..ntheta {
                for j in 0..nphi {
                    arbitrary_first[i * nphi + j] = (0.17 * i as f64 + 0.23 * j as f64).sin()
                        + 0.2 * (0.11 * i as f64 - 0.31 * j as f64).cos();
                    arbitrary_second[i * nphi + j] = (0.19 * i as f64 - 0.13 * j as f64).cos()
                        - 0.15 * (0.29 * i as f64 + 0.07 * j as f64).sin();
                }
            }
            let mut analyzed_e = vec![Complex64::new(0.0, 0.0); e.len()];
            let mut analyzed_b = vec![Complex64::new(0.0, 0.0); b.len()];
            plan.analysis_spin_into(
                &arbitrary_first,
                &arbitrary_second,
                &mut analyzed_e,
                &mut analyzed_b,
                &mut workspace,
            )
            .unwrap();
            let analysis_indices = [plan.harmonics().index(spin, 0).unwrap(), high_mode];
            let (mut analysis_max_error, mut analysis_error_squares) = (0.0_f64, 0.0_f64);
            let mut analysis_value_count = 0usize;
            for (slot, index) in analysis_indices.into_iter().enumerate() {
                for (actual, expected) in [
                    (analyzed_e[index], expected_analysis[slot].0),
                    (analyzed_b[index], expected_analysis[slot].1),
                ] {
                    let error = (actual - expected).norm();
                    analysis_max_error = analysis_max_error.max(error);
                    analysis_error_squares += error * error;
                    analysis_value_count += 1;
                    assert!(
                        error <= 2e-10,
                        "{kind:?} spin {spin} T{degree} analysis error={error:.3e}: actual={actual:?}, expected={expected:?}"
                    );
                }
            }

            let mut recovered_e = vec![Complex64::new(0.0, 0.0); e.len()];
            let mut recovered_b = vec![Complex64::new(0.0, 0.0); b.len()];
            plan.analysis_spin_into(
                &first,
                &second,
                &mut recovered_e,
                &mut recovered_b,
                &mut workspace,
            )
            .unwrap();
            let (mut roundtrip_max_error, mut roundtrip_error_squares) = (0.0_f64, 0.0_f64);
            for index in 0..e.len() {
                let expected = if index == high_mode { 1.0 } else { 0.0 };
                let e_error = (recovered_e[index] - Complex64::new(expected, 0.0)).norm();
                let b_error = recovered_b[index].norm();
                roundtrip_max_error = roundtrip_max_error.max(e_error).max(b_error);
                roundtrip_error_squares += e_error * e_error + b_error * b_error;
                assert!(
                    e_error <= 2e-7 && b_error <= 2e-7,
                    "{kind:?} spin {spin} T{degree} coefficient {index}: E={:?}, B={:?}, expected real E={expected}",
                    recovered_e[index],
                    recovered_b[index]
                );
            }
            eprintln!(
                "M3 spin {spin} T{degree} {kind:?}: DUCC synthesis samples max abs={synthesis_max_error:.3e}, RMS={:.3e}; DUCC analysis coefficients max abs={analysis_max_error:.3e}, RMS={:.3e}; synthesis-analysis coefficient max abs={roundtrip_max_error:.3e}, RMS={:.3e}",
                (synthesis_error_squares / synthesis_sample_count as f64).sqrt(),
                (analysis_error_squares / analysis_value_count as f64).sqrt(),
                (roundtrip_error_squares / (2 * e.len()) as f64).sqrt(),
            );
        }
    }
}

#[test]
#[ignore = "M3 exact T1023 spin analysis check; run this check in release mode"]
fn m3_spin_t1023_exact_analysis_matches_ducc() {
    // DUCC 0.41.0 references use phi0=0.37 and nthreads=1.
    // Synthesis fixtures use a^E_(1023,1023)=1; analysis fixtures use the map formulas below.
    let degree = 1023;
    let nphi = 2 * degree + 3;
    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let ntheta = degree + usize::from(kind == GridKind::ClenshawCurtis) + 1;
        let grid = GridSpec::new(kind, ntheta, nphi, 0.37).unwrap();
        for spin in [1, 2] {
            let plan = ShtPlan::new(
                grid.clone(),
                HarmonicSpec::new(degree, degree, spin).unwrap(),
            )
            .unwrap();
            let high_mode = plan.harmonics().index(degree, degree).unwrap();
            let low_mode = plan.harmonics().index(spin, 0).unwrap();
            let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            e[high_mode] = Complex64::new(1.0, 0.0);
            let mut workspace = plan.workspace().unwrap();
            let mut first = vec![0.0; plan.sample_count];
            let mut second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
                .unwrap();

            let (expected_first, expected_second, expected_analysis) = match (kind, spin) {
                (GridKind::GaussLegendre, 1) => (
                    [
                        0.0002692592537113718,
                        -0.00029309150858882133,
                        -0.000673221873510172,
                        -0.005182509993825906,
                        -0.0002454213020701616,
                    ],
                    [
                        3.3797276730555463,
                        -3.3788841449939198,
                        -3.355682138717093,
                        0.16784222253861777,
                        -3.3804996956447932,
                    ],
                    [
                        (
                            Complex64::new(-3.970105309790821e-9, 0.0),
                            Complex64::new(-2.1898731870038686e-9, 0.0),
                        ),
                        (
                            Complex64::new(-5.753979607020647e-5, -2.378876053935398e-6),
                            Complex64::new(-4.556171734751286e-6, -9.071625001329272e-7),
                        ),
                    ],
                ),
                (GridKind::GaussLegendre, 2) => (
                    [
                        0.17535897243750975,
                        -0.19088007215302083,
                        -0.43844545483196995,
                        -3.37518437950739,
                        -0.15983416262243652,
                    ],
                    [
                        0.010348643387021999,
                        -0.010346060524749888,
                        -0.010275016549598708,
                        0.0005139287760327167,
                        -0.010351007301288382,
                    ],
                    [
                        (
                            Complex64::new(-2.7869138063251393e-9, 0.0),
                            Complex64::new(-2.7227224767817692e-9, 0.0),
                        ),
                        (
                            Complex64::new(1.5874021743134444e-6, -4.515016053545278e-6),
                            Complex64::new(-1.888614041135579e-6, 5.7250248487014354e-5),
                        ),
                    ],
                ),
                (GridKind::ClenshawCurtis, 1) => (
                    [
                        -1.0766275818895656e-17,
                        1.1719203623084968e-17,
                        2.6918637995237438e-17,
                        2.0722159501906292e-16,
                        9.813120230780253e-18,
                    ],
                    [
                        3.3837900421491245,
                        -3.3829455001826334,
                        -3.3597156055306705,
                        0.1680439657331586,
                        -3.3845629926950016,
                    ],
                    [
                        (
                            Complex64::new(-5.0217365309611435e-9, -1.523876245149916e-22),
                            Complex64::new(-6.057643341435993e-9, 4.2941432788971746e-20),
                        ),
                        (
                            Complex64::new(-5.6315820110577476e-5, -2.310852903137655e-6),
                            Complex64::new(-3.790328915571601e-6, -1.195645014199101e-6),
                        ),
                    ],
                ),
                (GridKind::ClenshawCurtis, 2) => (
                    [
                        0.17556913153174913,
                        -0.19110883252105015,
                        -0.43897091012165323,
                        -3.379229371800783,
                        -0.16002571599651416,
                    ],
                    [
                        -4.1378788866709244e-16,
                        4.1368461357234587e-16,
                        4.1084393819288594e-16,
                        -2.0549312137524346e-17,
                        4.1388240918120196e-16,
                    ],
                    [
                        (
                            Complex64::new(-2.8534515942596718e-9, 2.0381702198409198e-26),
                            Complex64::new(-4.0501662408845994e-9, 4.4262338146783985e-20),
                        ),
                        (
                            Complex64::new(2.2006264408083243e-6, -3.7347210725520073e-6),
                            Complex64::new(-1.8145388279689996e-6, 5.6033549984767706e-5),
                        ),
                    ],
                ),
                (GridKind::Fejer1, 1) => (
                    [
                        0.0002693904437578937,
                        -0.00029323431032401504,
                        -0.0006735498845541299,
                        -0.005185035046234766,
                        -0.0002455408776524013,
                    ],
                    [
                        3.379723706345291,
                        -3.3788801792736933,
                        -3.355678200228539,
                        0.16784202554598165,
                        -3.3804957280284307,
                    ],
                    [
                        (
                            Complex64::new(-4.745294030805409e-9, -8.276311203503119e-22),
                            Complex64::new(-4.612248802094115e-9, -5.808191933468817e-21),
                        ),
                        (
                            Complex64::new(-5.764429994134854e-5, -2.383943930654648e-6),
                            Complex64::new(-4.5584040933637115e-6, -9.081814664123858e-7),
                        ),
                    ],
                ),
                (GridKind::Fejer1, 2) => (
                    [
                        0.175358767226784,
                        -0.19087984877900355,
                        -0.4384449417489569,
                        -3.3751804297577648,
                        -0.15983397557934373,
                    ],
                    [
                        0.01035368552462822,
                        -0.010351101403916093,
                        -0.010280022814227973,
                        0.0005141791759655065,
                        -0.010356050590657263,
                    ],
                    [
                        (
                            Complex64::new(-2.7812303811775246e-9, -1.3278710101432483e-19),
                            Complex64::new(-2.7197106514111454e-9, -1.3948727442036295e-25),
                        ),
                        (
                            Complex64::new(1.5893284646122541e-6, -4.517207500789589e-6),
                            Complex64::new(-1.8935509212736803e-6, 5.735450303181667e-5),
                        ),
                    ],
                ),
                _ => unreachable!(),
            };

            let longitudes = [0, 1, 17, nphi / 2, nphi - 1];
            let middle_ring = ntheta / 2;
            let mut synthesis_max_error = 0.0_f64;
            let mut synthesis_error_squares = 0.0_f64;
            for (slot, longitude) in longitudes.into_iter().enumerate() {
                let index = middle_ring * nphi + longitude;
                for (actual, expected) in [
                    (first[index], expected_first[slot]),
                    (second[index], expected_second[slot]),
                ] {
                    let error = (actual - expected).abs();
                    synthesis_max_error = synthesis_max_error.max(error);
                    synthesis_error_squares += error * error;
                    assert!(
                        error <= 2e-10,
                        "{kind:?} spin {spin} T{degree} synthesis error={error:.3e}"
                    );
                }
            }

            let mut arbitrary_first = vec![0.0; plan.sample_count];
            let mut arbitrary_second = vec![0.0; plan.sample_count];
            for i in 0..ntheta {
                for j in 0..nphi {
                    arbitrary_first[i * nphi + j] = (0.17 * i as f64 + 0.23 * j as f64).sin()
                        + 0.2 * (0.11 * i as f64 - 0.31 * j as f64).cos();
                    arbitrary_second[i * nphi + j] = (0.19 * i as f64 - 0.13 * j as f64).cos()
                        - 0.15 * (0.29 * i as f64 + 0.07 * j as f64).sin();
                }
            }
            let mut analyzed_e = vec![Complex64::new(0.0, 0.0); e.len()];
            let mut analyzed_b = vec![Complex64::new(0.0, 0.0); b.len()];
            plan.analysis_spin_into(
                &arbitrary_first,
                &arbitrary_second,
                &mut analyzed_e,
                &mut analyzed_b,
                &mut workspace,
            )
            .unwrap();
            let (mut analysis_max_error, mut analysis_error_squares) = (0.0_f64, 0.0_f64);
            for (slot, index) in [low_mode, high_mode].into_iter().enumerate() {
                for (actual, expected) in [
                    (analyzed_e[index], expected_analysis[slot].0),
                    (analyzed_b[index], expected_analysis[slot].1),
                ] {
                    let error = (actual - expected).norm();
                    analysis_max_error = analysis_max_error.max(error);
                    analysis_error_squares += error * error;
                    assert!(
                        error <= 2e-11,
                        "{kind:?} spin {spin} T{degree} analysis error={error:.3e}"
                    );
                }
            }

            let mut recovered_e = vec![Complex64::new(0.0, 0.0); e.len()];
            let mut recovered_b = vec![Complex64::new(0.0, 0.0); b.len()];
            plan.analysis_spin_into(
                &first,
                &second,
                &mut recovered_e,
                &mut recovered_b,
                &mut workspace,
            )
            .unwrap();
            let (mut roundtrip_max_error, mut roundtrip_error_squares) = (0.0_f64, 0.0_f64);
            for index in 0..e.len() {
                let expected = if index == high_mode { 1.0 } else { 0.0 };
                let e_error = (recovered_e[index] - Complex64::new(expected, 0.0)).norm();
                let b_error = recovered_b[index].norm();
                roundtrip_max_error = roundtrip_max_error.max(e_error).max(b_error);
                roundtrip_error_squares += e_error * e_error + b_error * b_error;
                assert!(
                    e_error <= 2e-10 && b_error <= 2e-10,
                    "{kind:?} spin {spin} T{degree} coefficient {index}: E={:?}, B={:?}",
                    recovered_e[index],
                    recovered_b[index]
                );
            }
            eprintln!(
                "M3 spin {spin} T{degree} {kind:?}: DUCC synthesis samples max abs={synthesis_max_error:.3e}, RMS={:.3e}; DUCC analysis coefficients max abs={analysis_max_error:.3e}, RMS={:.3e}; synthesis-analysis coefficient max abs={roundtrip_max_error:.3e}, RMS={:.3e}",
                (synthesis_error_squares / 10.0).sqrt(),
                (analysis_error_squares / 4.0).sqrt(),
                (roundtrip_error_squares / (2 * e.len()) as f64).sqrt(),
            );
        }
    }
}

#[test]
#[ignore = "M3 sparse T1023 synthesis check; run this check in release mode"]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain printed DUCC 0.41.0 reference values."
)]
fn m3_spin_t1023_sparse_synthesis_matches_ducc() {
    // DUCC 0.41.0 references use a^E_(1023,1023)=1, phi0=0.37, nthreads=1.
    // They cover ring 31 of a 65-ring grid and j=[0,1,17,nphi/2,nphi-1].
    let degree = 1023;
    let ntheta = 65;
    let nphi = 2 * degree + 3;
    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let grid = GridSpec::new(kind, ntheta, nphi, 0.37).unwrap();
        for spin in [1, 2] {
            let plan = ShtPlan::new(
                grid.clone(),
                HarmonicSpec::new(degree, degree, spin).unwrap(),
            )
            .unwrap();
            let high_mode = plan.harmonics().index(degree, degree).unwrap();
            let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            e[high_mode] = Complex64::new(1.0, 0.0);
            let mut workspace = plan.workspace().unwrap();
            let mut first = vec![0.0; plan.sample_count];
            let mut second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
                .unwrap();

            let (expected_first, expected_second) = match (kind, spin) {
                (GridKind::GaussLegendre, 1) => (
                    [
                        -0.0026008582624004115,
                        0.00283106137020556,
                        0.006502857926006633,
                        0.05005946407273143,
                        0.0023706001277953108,
                    ],
                    [
                        1.0440133527351534,
                        -1.0437527830547122,
                        -1.0365855770823591,
                        0.05184723102990156,
                        -1.0442518340477787,
                    ],
                ),
                (GridKind::GaussLegendre, 2) => (
                    [
                        0.05435601666252876,
                        -0.05916709158518613,
                        -0.13590492757334846,
                        -1.046205824666246,
                        -0.04954379172039536,
                    ],
                    [
                        -0.10007570596629436,
                        0.10005072860881592,
                        0.09936370368177835,
                        -0.004969906020954131,
                        0.10009856600506513,
                    ],
                ),
                (GridKind::ClenshawCurtis, 1) => (
                    [
                        -0.0025172268401400556,
                        0.0027400276940073853,
                        0.006293756467087817,
                        0.048449786129678055,
                        0.0022943727288769426,
                    ],
                    [
                        0.9872920548389174,
                        -0.9870456419223036,
                        -0.9802678306104462,
                        0.04903036836369357,
                        -0.9875175794498488,
                    ],
                ),
                (GridKind::ClenshawCurtis, 2) => (
                    [
                        0.05141125476026481,
                        -0.05596168751281744,
                        -0.12854221636636906,
                        -0.9895271487152227,
                        -0.04685973429106343,
                    ],
                    [
                        -0.09686303421042028,
                        0.09683885868641512,
                        0.09617388891809334,
                        -0.004810360039752212,
                        0.09688516038675976,
                    ],
                ),
                (GridKind::Fejer1, 1) => (
                    [
                        -0.002573559915789246,
                        0.00280134683493901,
                        0.006434604583564386,
                        0.04953404497505026,
                        0.0023457185473952577,
                    ],
                    [
                        1.025145780618557,
                        -1.0248899199940147,
                        -1.0178522408856654,
                        0.05091023978554628,
                        -1.0253799520597107,
                    ],
                ),
                (GridKind::Fejer1, 2) => (
                    [
                        0.05337653486315186,
                        -0.05810091542129314,
                        -0.1334559548344109,
                        -1.0273534578707713,
                        -0.04865102500859692,
                    ],
                    [
                        -0.09902708853582051,
                        0.09900237289712947,
                        0.09832254678329747,
                        -0.004917830144685581,
                        0.0990497090415809,
                    ],
                ),
                _ => unreachable!(),
            };

            let ring = 31;
            let longitudes = [0, 1, 17, nphi / 2, nphi - 1];
            let (mut max_error, mut error_squares) = (0.0_f64, 0.0_f64);
            let mut count = 0usize;
            for (slot, longitude) in longitudes.into_iter().enumerate() {
                let index = ring * nphi + longitude;
                for (actual, expected) in [
                    (first[index], expected_first[slot]),
                    (second[index], expected_second[slot]),
                ] {
                    let error = (actual - expected).abs();
                    max_error = max_error.max(error);
                    error_squares += error * error;
                    count += 1;
                    assert!(
                        error <= 8e-11,
                        "{kind:?} spin {spin} T{degree} synthesis error={error:.3e}"
                    );
                }
            }

            let mut gradient_first = vec![0.0; plan.sample_count];
            let mut gradient_second = vec![0.0; plan.sample_count];
            plan.synthesis_gradient_into(
                &e,
                &mut gradient_first,
                &mut gradient_second,
                &mut workspace,
            )
            .unwrap();
            let (mut gradient_max_error, mut gradient_error_squares) = (0.0_f64, 0.0_f64);
            for ((&standard_first, &standard_second), (&gradient_first, &gradient_second)) in first
                .iter()
                .zip(&second)
                .zip(gradient_first.iter().zip(&gradient_second))
            {
                for error in [
                    (standard_first - gradient_first).abs(),
                    (standard_second - gradient_second).abs(),
                ] {
                    gradient_max_error = gradient_max_error.max(error);
                    gradient_error_squares += error * error;
                }
            }
            assert!(gradient_max_error <= 2e-11);
            eprintln!(
                "M3 spin {spin} T{degree} {kind:?}: DUCC sparse synthesis max abs={max_error:.3e}, RMS={:.3e}; STANDARD-GRAD_ONLY max abs={gradient_max_error:.3e}, RMS={:.3e}",
                (error_squares / count as f64).sqrt(),
                (gradient_error_squares / (2 * plan.sample_count) as f64).sqrt(),
            );
        }
    }
}

#[test]
#[ignore = "M3 high-spin exact-analysis T1023 check; run this check in release mode"]
fn m3_spin_three_and_four_t1023_exact_analysis_matches_ducc() {
    // DUCC 0.41.0 references use deterministic maps, phi0=0.37 and one thread.
    let degree = 1023;
    let nphi = 2 * degree + 3;
    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let ntheta = degree + usize::from(kind == GridKind::ClenshawCurtis) + 1;
        for spin in [3, 4] {
            let plan = ShtPlan::new(
                GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
                HarmonicSpec::new(degree, degree, spin).unwrap(),
            )
            .unwrap();
            let (expected_low, expected_sectoral) = match (kind, spin) {
                (GridKind::GaussLegendre, 3) => (
                    (
                        Complex64::new(6.081437129307585e-11, 0.0),
                        Complex64::new(1.0231104848956346e-11, 0.0),
                    ),
                    (
                        Complex64::new(-5.675308074549429e-5, -1.3894519746032559e-6),
                        Complex64::new(-4.469558524892187e-6, -2.2624639044714076e-6),
                    ),
                ),
                (GridKind::GaussLegendre, 4) => (
                    (
                        Complex64::new(5.184718104938012e-11, 0.0),
                        Complex64::new(1.6270404420515953e-11, 0.0),
                    ),
                    (
                        Complex64::new(2.9298922190006932e-6, -4.419929093855126e-6),
                        Complex64::new(-8.821792899411789e-7, 5.60501229627666e-5),
                    ),
                ),
                (GridKind::ClenshawCurtis, 3) => (
                    (
                        Complex64::new(7.269955478419545e-11, 0.0),
                        Complex64::new(2.7346913656753508e-11, 6.701164155394357e-20),
                    ),
                    (
                        Complex64::new(-5.5548274707962206e-5, -1.3095607129762022e-6),
                        Complex64::new(-3.676028538607881e-6, -3.1981614453430745e-6),
                    ),
                ),
                (GridKind::ClenshawCurtis, 4) => (
                    (
                        Complex64::new(5.2990596906847176e-11, 0.0),
                        Complex64::new(2.4128616571573914e-11, 8.261474378121507e-20),
                    ),
                    (
                        Complex64::new(4.1846426871489025e-6, -3.614433698993269e-6),
                        Complex64::new(-7.967077658667091e-7, 5.486178105231755e-5),
                    ),
                ),
                (GridKind::Fejer1, 3) => (
                    (
                        Complex64::new(7.027579354743823e-11, 0.0),
                        Complex64::new(1.9309400963416208e-11, 4.50432024596083e-21),
                    ),
                    (
                        Complex64::new(-5.685691558825787e-5, -1.3942485532993349e-6),
                        Complex64::new(-4.471712363615764e-6, -2.2652960182642644e-6),
                    ),
                ),
                (GridKind::Fejer1, 4) => (
                    (
                        Complex64::new(5.1731818492207214e-11, 0.0),
                        Complex64::new(1.625938528964583e-11, 6.361872894215043e-21),
                    ),
                    (
                        Complex64::new(2.9336278104640267e-6, -4.422048649946405e-6),
                        Complex64::new(-8.868258225951987e-7, 5.615336734718681e-5),
                    ),
                ),
                _ => unreachable!(),
            };

            let mut first = vec![0.0; plan.sample_count];
            let mut second = vec![0.0; plan.sample_count];
            for row in 0..ntheta {
                for column in 0..nphi {
                    let index = row * nphi + column;
                    let i = row as f64;
                    let j = column as f64;
                    first[index] = (0.17 * i + 0.23 * j).sin() + 0.2 * (0.11 * i - 0.31 * j).cos();
                    second[index] =
                        (0.19 * i - 0.13 * j).cos() - 0.15 * (0.29 * i + 0.07 * j).sin();
                }
            }
            let mut analyzed_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let mut analyzed_b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let mut workspace = plan.workspace().unwrap();
            plan.analysis_spin_into(
                &first,
                &second,
                &mut analyzed_e,
                &mut analyzed_b,
                &mut workspace,
            )
            .unwrap();

            let selected_modes = [
                (plan.harmonics().index(spin, 0).unwrap(), expected_low),
                (
                    plan.harmonics().index(degree, degree).unwrap(),
                    expected_sectoral,
                ),
            ];
            let mut maximum_error = 0.0_f64;
            for (index, (expected_e, expected_b)) in selected_modes {
                for (actual, expected) in [
                    (analyzed_e[index], expected_e),
                    (analyzed_b[index], expected_b),
                ] {
                    let error = (actual - expected).norm();
                    maximum_error = maximum_error.max(error);
                    assert!(
                        error <= 5e-12,
                        "{kind:?} spin {spin} T{degree} DUCC A error={error:.3e}"
                    );
                }
            }
            eprintln!(
                "M3 spin {spin} T{degree} {kind:?}: DUCC analysis selected-coefficient max abs={maximum_error:.3e}"
            );
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "M3 selected T2047 spin-1 exact analysis and adjoint check; release mode only"]
fn m3_spin1_t2047_low_order_analysis_matches_q1_and_adjoint() {
    let degree = 2047;
    let mmax = 2;
    let ntheta = degree + 1;
    let nphi = 7;
    let plan = make_plan(1, degree, mmax, ntheta, nphi);
    let mut first = vec![0.0; plan.sample_count];
    let mut second = vec![0.0; plan.sample_count];
    for ring in 0..ntheta {
        for longitude in 0..nphi {
            let index = ring * nphi + longitude;
            first[index] = (0.17 * ring as f64 + 0.23 * longitude as f64).sin()
                + 0.2 * (0.11 * ring as f64 - 0.31 * longitude as f64).cos();
            second[index] = (0.19 * ring as f64 - 0.13 * longitude as f64).cos()
                - 0.15 * (0.29 * ring as f64 + 0.07 * longitude as f64).sin();
        }
    }
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let five_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(5)
        .build()
        .unwrap();
    let mut one_workspace = one_thread.install(|| plan.workspace()).unwrap();
    let mut five_workspace = five_threads.install(|| plan.workspace()).unwrap();
    let mut one_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut one_b = one_e.clone();
    let mut five_e = one_e.clone();
    let mut five_b = one_e.clone();
    one_thread
        .install(|| {
            plan.analysis_spin_into(&first, &second, &mut one_e, &mut one_b, &mut one_workspace)
        })
        .unwrap();
    five_threads
        .install(|| {
            plan.analysis_spin_into(
                &first,
                &second,
                &mut five_e,
                &mut five_b,
                &mut five_workspace,
            )
        })
        .unwrap();
    assert!(relative_l2_complex(&five_e, &one_e) <= 3e-10);
    assert!(relative_l2_complex(&five_b, &one_b) <= 3e-10);

    let (e, b) = coefficients(&plan);
    let mut adjoint_first = vec![0.0; plan.sample_count];
    let mut adjoint_second = vec![0.0; plan.sample_count];
    five_threads
        .install(|| {
            plan.adjoint_analysis_spin_into(
                &e,
                &b,
                &mut adjoint_first,
                &mut adjoint_second,
                &mut five_workspace,
            )
        })
        .unwrap();
    let left = spin_harmonic_inner_product(plan.harmonics(), &five_e, &five_b, &e, &b);
    let right = spin_map_inner_product(&first, &second, &adjoint_first, &adjoint_second);
    assert_close(left, right, 3e-8);
}

#[test]
#[ignore = "M3 sparse T2047 synthesis check; run this check in release mode"]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain printed DUCC 0.41.0 reference values."
)]
fn m3_spin_t2047_sparse_synthesis_matches_ducc_and_sectoral_formula() {
    // DUCC 0.41.0 references use a^E_(2047,2047)=1, phi0=0.37 and nthreads=1.
    // The Wigner-d sectoral formula is checked at every sample on 65 rings.
    let degree = 2047;
    let ntheta = 65;
    let nphi = 2 * degree + 3;
    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let grid = GridSpec::new(kind, ntheta, nphi, 0.37).unwrap();
        for spin in 1..=4 {
            let plan = ShtPlan::new(
                grid.clone(),
                HarmonicSpec::new(degree, degree, spin).unwrap(),
            )
            .unwrap();
            let high_mode = plan.harmonics().index(degree, degree).unwrap();
            let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            e[high_mode] = Complex64::new(1.0, 0.0);
            let mut workspace = plan.workspace().unwrap();
            let mut first = vec![0.0; plan.sample_count];
            let mut second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
                .unwrap();

            // All three grids have theta=pi/2 at their middle ring for odd ntheta.
            // These samples are static outputs from ducc0==0.41.0.
            let (expected_first, expected_second) = match (kind, spin) {
                (GridKind::Fejer1, 1) => (
                    [
                        1.1015587667186156e-15,
                        -1.1022467302080192e-15,
                        -1.1124584825502832e-15,
                        -3.0159540880018856e-16,
                        -1.1008649739104192e-15,
                    ],
                    [
                        -1.0601942551631958,
                        1.0512460199516056,
                        0.90734996706382121,
                        -3.8874063655543365,
                        1.0691368799521002,
                    ],
                ),
                (GridKind::Fejer1, 2) => (
                    [
                        -3.8857806088139322,
                        3.8882074200449486,
                        3.9242296736300024,
                        1.0638865820242214,
                        3.8833332344912224,
                    ],
                    [
                        6.002172983489843e-16,
                        -5.9515135355864421e-16,
                        -5.1368618839031055e-16,
                        2.2008122897803329e-15,
                        -6.0528006686032456e-16,
                    ],
                ),
                (_, 1) => (
                    [
                        2.3810981481297348e-16,
                        -2.3825852304716785e-16,
                        -2.404658664341898e-16,
                        -6.5192007097157444e-17,
                        -2.3795984652978313e-16,
                    ],
                    [
                        -1.0601942551631958,
                        1.0512460199516056,
                        0.90734996706382121,
                        -3.8874063655543365,
                        1.0691368799521002,
                    ],
                ),
                (_, 2) => (
                    [
                        -3.8857806088139322,
                        3.8882074200449486,
                        3.9242296736300024,
                        1.0638865820242214,
                        3.8833332344912224,
                    ],
                    [
                        1.2974126671711845e-16,
                        -1.2864622647748256e-16,
                        -1.1103694771905273e-16,
                        4.7572133470349097e-16,
                        -1.3083562038130585e-16,
                    ],
                ),
                (_, 3) => (
                    [
                        2.5707043933917143e-15,
                        -2.5723098917256528e-15,
                        -2.596141027570257e-15,
                        -7.038323018743339e-16,
                        -2.569085291193975e-15,
                    ],
                    [
                        -1.0581250786453649,
                        1.0491943076655272,
                        0.9055790960784711,
                        -3.8798193314540286,
                        1.0670502501523527,
                    ],
                ),
                (_, 4) => (
                    [
                        -3.874410364950886,
                        3.876830075051321,
                        3.9127469233526013,
                        1.0607735267341716,
                        3.871970151928742,
                    ],
                    [
                        9.345012395551155e-16,
                        -9.266138765965464e-16,
                        -7.997776490506578e-16,
                        3.4265287229893315e-15,
                        -9.4238365724358e-16,
                    ],
                ),
                _ => unreachable!(),
            };
            let sample_longitudes = [0, 1, 17, nphi / 2, nphi - 1];
            let middle_ring = ntheta / 2;
            let mut ducc_maximum_error = 0.0_f64;
            for (slot, longitude) in sample_longitudes.into_iter().enumerate() {
                let index = middle_ring * nphi + longitude;
                for (actual, expected) in [
                    (first[index], expected_first[slot]),
                    (second[index], expected_second[slot]),
                ] {
                    let error = (actual - expected).abs();
                    ducc_maximum_error = ducc_maximum_error.max(error);
                    assert!(
                        error <= 2e-10,
                        "{kind:?} spin {spin} DUCC error={error:.3e}"
                    );
                }
            }

            let log_common = sectoral_spin_log_common(degree, spin);
            let (mut maximum_error, mut error_squares) = (0.0_f64, 0.0_f64);
            for (ring, &theta) in plan.grid.theta().iter().enumerate() {
                for longitude in 0..nphi {
                    let phi = plan.grid.longitude(longitude).unwrap();
                    let (expected_first, expected_second) =
                        sectoral_spin_reference(degree, spin, theta, phi, log_common);
                    for (actual, expected) in [
                        (first[ring * nphi + longitude], expected_first),
                        (second[ring * nphi + longitude], expected_second),
                    ] {
                        let error = (actual - expected).abs();
                        maximum_error = maximum_error.max(error);
                        error_squares += error * error;
                        assert!(
                            error <= 5e-10,
                            "{kind:?} spin {spin} T{degree} analytic error={error:.3e}"
                        );
                    }
                }
            }
            eprintln!(
                "M3 spin {spin} T{degree} {kind:?}: DUCC middle-ring samples max abs={ducc_maximum_error:.3e}; analytic map max abs={maximum_error:.3e}, RMS={:.3e}",
                (error_squares / (2 * plan.sample_count) as f64).sqrt(),
            );
        }
    }
}

#[test]
#[ignore = "M3 high-spin all-grid check; run this check in release mode"]
fn m3_spin_three_and_four_t511_all_grids_match_ducc_and_analytic_fields() {
    // DUCC 0.41.0 references use T511, phi0=0.37 and one thread.
    // The analysis map is generated independently from pixel-index formulas.
    let degree = 511;
    let nphi = 2 * degree + 3;
    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let ntheta = degree + usize::from(kind == GridKind::ClenshawCurtis) + 1;
        let grid = GridSpec::new(kind, ntheta, nphi, 0.37).unwrap();
        for spin in [3, 4] {
            let plan = ShtPlan::new(
                grid.clone(),
                HarmonicSpec::new(degree, degree, spin).unwrap(),
            )
            .unwrap();
            let high_mode = plan.harmonics().index(degree, degree).unwrap();
            let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            e[high_mode] = Complex64::new(1.0, 0.0);
            let mut workspace = plan.workspace().unwrap();
            let mut first = vec![0.0; plan.sample_count];
            let mut second = vec![0.0; plan.sample_count];
            plan.synthesis_spin_into(&e, &b, &mut first, &mut second, &mut workspace)
                .unwrap();

            let (expected_first, expected_second, expected_analysis) = match (kind, spin) {
                (GridKind::GaussLegendre, 3) => (
                    [
                        0.021758378615044895,
                        -0.02188695021363462,
                        -0.023685529484134948,
                        -0.014183031847404092,
                        -0.021627967439358613,
                    ],
                    [
                        1.5316666088753335,
                        -1.509843199479402,
                        -1.1445944644979498,
                        2.359348926174194,
                        -1.5533605224635534,
                    ],
                    [
                        (
                            Complex64::new(2.069501617069886e-8, 0.0),
                            Complex64::new(-2.7429145021623706e-8, 0.0),
                        ),
                        (
                            Complex64::new(1.4535838590743135e-6, 1.7837491274690076e-5),
                            Complex64::new(7.488982643744704e-6, -6.0721641062644385e-5),
                        ),
                        (
                            Complex64::new(0.00014862612689717672, 0.00012156006725935135),
                            Complex64::new(0.0002363540181397813, 0.00031071886783181973),
                        ),
                    ],
                ),
                (GridKind::GaussLegendre, 4) => (
                    [
                        2.350355488212745,
                        -2.364243883470501,
                        -2.5585276917540427,
                        -1.5320611582240165,
                        -2.336268380532461,
                    ],
                    [
                        0.018649684052313466,
                        -0.01838396063194246,
                        -0.013936665464417086,
                        0.028727604158337524,
                        -0.018913830722309476,
                    ],
                    [
                        (
                            Complex64::new(3.3462556051042415e-9, 0.0),
                            Complex64::new(3.668230950184262e-11, 0.0),
                        ),
                        (
                            Complex64::new(5.739156318742611e-5, 5.5155920888535827e-5),
                            Complex64::new(-3.7409098771640497e-6, -2.4811487597217522e-5),
                        ),
                        (
                            Complex64::new(-0.0003030026293630465, 0.00024227781526364243),
                            Complex64::new(0.00010893340858517173, -0.0001631130064309241),
                        ),
                    ],
                ),
                (GridKind::ClenshawCurtis, 3) => (
                    [
                        -1.6776506023809333e-15,
                        1.6875639430594517e-15,
                        1.8262409855895577e-15,
                        1.093563649358178e-15,
                        1.6675954235774899e-15,
                    ],
                    [
                        1.5352823927012231,
                        -1.5134074650896026,
                        -1.147296492555462,
                        2.3649186080079945,
                        -1.5570275188062004,
                    ],
                    [
                        (
                            Complex64::new(2.4242417101390347e-8, 0.0),
                            Complex64::new(-2.480469607986025e-8, 0.0),
                        ),
                        (
                            Complex64::new(2.4703879175392515e-6, 2.1868474330074364e-5),
                            Complex64::new(9.927404638665334e-6, -5.700715149015852e-5),
                        ),
                        (
                            Complex64::new(0.00015973845404200805, 0.00013098210953502835),
                            Complex64::new(0.000216485357678718, 0.0002870206880362535),
                        ),
                    ],
                ),
                (GridKind::ClenshawCurtis, 4) => (
                    [
                        2.3558264834744733,
                        -2.3697472071800014,
                        -2.5644832559011634,
                        -1.5356273844308002,
                        -2.341706584797356,
                    ],
                    [
                        -1.2065135921999756e-15,
                        1.1893230104429772e-15,
                        9.016118592463421e-16,
                        -1.858489655436024e-15,
                        1.2236021684348639e-15,
                    ],
                    [
                        (
                            Complex64::new(5.1929979409988405e-9, 0.0),
                            Complex64::new(2.2167944513689238e-10, 0.0),
                        ),
                        (
                            Complex64::new(5.5800829994515236e-5, 5.7695652436830015e-5),
                            Complex64::new(-3.2236603322795114e-6, -2.2763160619093822e-5),
                        ),
                        (
                            Complex64::new(-0.0002802358706607522, 0.00022159314867697247),
                            Complex64::new(0.0001174640754122743, -0.0001752720222787794),
                        ),
                    ],
                ),
                (GridKind::Fejer1, 3) => (
                    [
                        0.021779535864226204,
                        -0.02190823248230369,
                        -0.023708560641839894,
                        -0.01419682303765089,
                        -0.021648997880297963,
                    ],
                    [
                        1.5316595482353412,
                        -1.5098362394404388,
                        -1.144589188173844,
                        2.3593380501041095,
                        -1.5533533618194788,
                    ],
                    [
                        (
                            Complex64::new(2.235529970051836e-8, 0.0),
                            Complex64::new(-2.7525640862541656e-8, 0.0),
                        ),
                        (
                            Complex64::new(2.909518996783257e-7, 1.799665136412528e-5),
                            Complex64::new(7.276252362086791e-6, -6.203552076659921e-5),
                        ),
                        (
                            Complex64::new(0.00014885998864306508, 0.00012186007902761095),
                            Complex64::new(0.00023662795830389665, 0.00031109386645033506),
                        ),
                    ],
                ),
                (GridKind::Fejer1, 4) => (
                    [
                        2.350344804668872,
                        -2.3642331367969094,
                        -2.5585160619631537,
                        -1.5320541942380652,
                        -2.336257761021554,
                    ],
                    [
                        0.018667818892349784,
                        -0.018401837084131088,
                        -0.013950217393668432,
                        0.02875553870696456,
                        -0.018932222416970942,
                    ],
                    [
                        (
                            Complex64::new(3.187280571616479e-9, 0.0),
                            Complex64::new(2.6463433945618163e-10, 0.0),
                        ),
                        (
                            Complex64::new(5.808557663704942e-5, 5.716765688952045e-5),
                            Complex64::new(-1.9891484935852085e-6, -2.132051404114898e-5),
                        ),
                        (
                            Complex64::new(-0.0003033747841735894, 0.00024255981437613892),
                            Complex64::new(0.0001092267143635806, -0.00016335555962365282),
                        ),
                    ],
                ),
                _ => unreachable!(),
            };

            let sample_longitudes = [0, 1, 17, nphi / 2, nphi - 1];
            let middle_ring = ntheta / 2;
            let mut ducc_synthesis_max_error = 0.0_f64;
            for (slot, longitude) in sample_longitudes.into_iter().enumerate() {
                let index = middle_ring * nphi + longitude;
                for (actual, expected) in [
                    (first[index], expected_first[slot]),
                    (second[index], expected_second[slot]),
                ] {
                    let error = (actual - expected).abs();
                    ducc_synthesis_max_error = ducc_synthesis_max_error.max(error);
                    assert!(
                        error <= 2e-11,
                        "{kind:?} spin {spin} T{degree} DUCC S error={error:.3e}"
                    );
                }
            }

            let log_common = sectoral_spin_log_common(degree, spin);
            let mut analytic_synthesis_max_error = 0.0_f64;
            let mut analytic_synthesis_error_squares = 0.0_f64;
            for (ring, &theta) in plan.grid.theta().iter().enumerate() {
                for longitude in 0..nphi {
                    let phi = plan.grid.longitude(longitude).unwrap();
                    let (expected_first, expected_second) =
                        sectoral_spin_reference(degree, spin, theta, phi, log_common);
                    for (actual, expected) in [
                        (first[ring * nphi + longitude], expected_first),
                        (second[ring * nphi + longitude], expected_second),
                    ] {
                        let error = (actual - expected).abs();
                        analytic_synthesis_max_error = analytic_synthesis_max_error.max(error);
                        analytic_synthesis_error_squares += error * error;
                        assert!(
                            error <= 5e-11,
                            "{kind:?} spin {spin} T{degree} analytic S error={error:.3e}"
                        );
                    }
                }
            }

            for row in 0..ntheta {
                for column in 0..nphi {
                    let index = row * nphi + column;
                    let i = row as f64;
                    let j = column as f64;
                    first[index] = (0.17 * i + 0.23 * j).sin() + 0.2 * (0.11 * i - 0.31 * j).cos();
                    second[index] =
                        (0.19 * i - 0.13 * j).cos() - 0.15 * (0.29 * i + 0.07 * j).sin();
                }
            }
            let mut analyzed_e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let mut analyzed_b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            plan.analysis_spin_into(
                &first,
                &second,
                &mut analyzed_e,
                &mut analyzed_b,
                &mut workspace,
            )
            .unwrap();
            let selected_modes = [(spin, 0), (128, 1), (degree, degree)];
            let mut ducc_analysis_max_error = 0.0_f64;
            for (mode, (expected_e, expected_b)) in
                selected_modes.into_iter().zip(expected_analysis)
            {
                let index = plan.harmonics().index(mode.0, mode.1).unwrap();
                for (actual, expected) in [
                    (analyzed_e[index], expected_e),
                    (analyzed_b[index], expected_b),
                ] {
                    let error = (actual - expected).norm();
                    ducc_analysis_max_error = ducc_analysis_max_error.max(error);
                    assert!(
                        error <= 5e-12,
                        "{kind:?} spin {spin} T{degree} DUCC A mode={mode:?}, actual={actual:?}, expected={expected:?}, error={error:.3e}"
                    );
                }
            }
            eprintln!(
                "M3 spin {spin} T{degree} {kind:?}: DUCC S max abs={ducc_synthesis_max_error:.3e}; analytic S max abs={analytic_synthesis_max_error:.3e}, RMS={:.3e}; DUCC A selected coefficient max abs={ducc_analysis_max_error:.3e}",
                (analytic_synthesis_error_squares / (2 * plan.sample_count) as f64).sqrt(),
            );
        }
    }
}

#[test]
fn extreme_spin2_scalar_conversion_restores_original_coefficients() {
    for lmax in [500, 511] {
        let plan = make_plan(2, lmax, 1, lmax + 1, 2 * (lmax + 1));
        let mut workspace = plan.workspace().unwrap();
        plan.prepare_spin_latitudes(&mut workspace, 2);
        let spectrum_len = plan.fft.spectrum_len();
        let ring_start = lmax / 3;
        let ring_end = ring_start + 67;
        for channels in [0, 1, 2] {
            let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
            let mut b = e.clone();
            let index = plan.harmonics.index(2, 1).unwrap();
            if channels != 1 {
                e[index].re = 1.0e308;
            }
            if channels != 0 {
                b[index].im = 1.0e308;
            }
            let mut fast_q = vec![Complex64::new(0.0, 0.0); 67 * spectrum_len];
            let mut fast_u = fast_q.clone();
            let mut general_q = fast_q.clone();
            let mut general_u = fast_q.clone();
            for (fast, q, u) in [
                (true, &mut fast_q, &mut fast_u),
                (false, &mut general_q, &mut general_u),
            ] {
                plan.synthesize_spin_ring_group_mode(
                    &e,
                    Some(&b),
                    2,
                    false,
                    fast,
                    false,
                    None,
                    None,
                    plan.grid.nphi(),
                    spectrum_len,
                    plan.grid.phi0(),
                    ring_start,
                    ring_end,
                    q,
                    u,
                    &workspace.theta_sin[ring_start..ring_end],
                    &workspace.theta_cos[ring_start..ring_end],
                    &workspace.ring_order_limits,
                    pulp::Arch::new(),
                    &mut workspace.recurrence_workers[0],
                )
                .unwrap();
            }
            let mut finite_count = 0;
            for (actual, expected) in fast_q
                .iter()
                .chain(&fast_u)
                .zip(general_q.iter().chain(&general_u))
            {
                for (actual, expected) in [(actual.re, expected.re), (actual.im, expected.im)] {
                    if expected.is_finite() {
                        assert!(
                            actual.is_finite(),
                            "L{lmax} channels={channels} actual={actual} expected={expected}"
                        );
                        assert!((actual / 1.0e308 - expected / 1.0e308).abs() < 2e-12);
                        finite_count += usize::from(expected != 0.0);
                    }
                }
            }
            assert!(finite_count > 67);
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "focused phase profiling helper; run explicitly with RUSTSHT_PROFILE_* variables"]
fn profile_spin1_standard_synthesis_phases() {
    use std::time::Instant;

    let lmax = std::env::var("RUSTSHT_PROFILE_LMAX")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(720_usize);
    let mmax = std::env::var("RUSTSHT_PROFILE_MMAX")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(lmax);
    let grid_kind = match std::env::var("RUSTSHT_PROFILE_GRID").as_deref() {
        Ok("CC") => GridKind::ClenshawCurtis,
        Ok("F1") => GridKind::Fejer1,
        _ => GridKind::GaussLegendre,
    };
    let minimum_ntheta = lmax + usize::from(grid_kind == GridKind::ClenshawCurtis) + 1;
    let ntheta = std::env::var("RUSTSHT_PROFILE_NTHETA")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(minimum_ntheta);
    let nphi = std::env::var("RUSTSHT_PROFILE_NPHI")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| 2 * (lmax + 1));
    let thread_count = std::env::var("RUSTSHT_PROFILE_THREADS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8_usize);
    let phi0 = std::env::var("RUSTSHT_PROFILE_PHI0")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0.23_f64);
    let repetitions = std::env::var("RUSTSHT_PROFILE_REPETITIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(12_usize);
    let plan = ShtPlan::new(
        GridSpec::new(grid_kind, ntheta, nphi, phi0).unwrap(),
        HarmonicSpec::new(lmax, mmax, 1).unwrap(),
    )
    .unwrap();
    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut b_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut state = 0x7a31_9d42_8b6c_05e1_u64;
    let mut next_value = || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^= value >> 31;
        ((value >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64)) * 2.0 - 1.0
    };
    for order in 0..=mmax {
        for degree in 1.max(order)..=lmax {
            let index = plan.harmonics.index(degree, order).unwrap();
            e_alm[index].re = next_value();
            e_alm[index].im = if order == 0 { 0.0 } else { next_value() };
            b_alm[index].re = next_value();
            b_alm[index].im = if order == 0 { 0.0 } else { next_value() };
        }
    }
    let mut first = vec![0.0; plan.sample_count];
    let mut second = vec![0.0; plan.sample_count];
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .unwrap();
    let mut workspace = pool.install(|| plan.workspace()).unwrap();
    plan.prepare_workspace(&mut workspace, thread_count)
        .unwrap();
    let probe = workspace.observe_transform_phases();
    let group_ring_count = plan
        .fine_synthesis_ring_group_size(thread_count)
        .unwrap_or(workspace.ring_group_size)
        .min(plan.grid.ntheta());
    let group_count = plan.grid.ntheta().div_ceil(group_ring_count);
    let worker_count = group_count.min(thread_count);
    eprintln!(
        "PROFILE_CONFIG,grid={grid_kind:?},lmax={lmax},mmax={mmax},ntheta={ntheta},nphi={nphi},threads={thread_count},phi0={phi0}"
    );

    for _ in 0..3 {
        pool.install(|| {
            plan.synthesis_spin_into(&e_alm, &b_alm, &mut first, &mut second, &mut workspace)
        })
        .unwrap();
        std::hint::black_box((&first, &second));
    }
    eprintln!(
        "PROFILE_CACHE_STATE,lmax={lmax},mmax={mmax},coefficient_complex_values={},coefficient_bytes={},finite_orders={},order_major_scratch_workers={}",
        workspace.spin1_scalar_coefficients.len(),
        workspace.spin1_scalar_coefficients.len() * std::mem::size_of::<Complex64>(),
        workspace.spin1_scalar_coefficient_finite_orders.len(),
        workspace.spin_order_output_scratch.len(),
    );
    probe.lock().unwrap().clear();
    for worker in &mut workspace.recurrence_workers {
        worker.spin1_profile_enabled = true;
    }

    for repetition in 0..repetitions {
        for worker in &mut workspace.recurrence_workers {
            worker.spin1_profile_factor_ns = 0;
            worker.spin1_profile_pair_prepare_ns = 0;
            worker.spin1_profile_recurrence_ns = 0;
            worker.spin1_profile_fallback_ns = 0;
            worker.spin1_profile_pair_eligible_blocks = 0;
            worker.spin1_profile_pair_success_blocks = 0;
            worker.spin1_profile_fallback_blocks = 0;
            worker.spin1_profile_coefficient_safety_ns = 0;
            worker.spin1_profile_coefficient_safety_values = 0;
            worker.spin1_profile_group_ns = 0;
            worker.spin1_profile_group_worker_id = usize::MAX;
        }
        let started = Instant::now();
        pool.install(|| {
            plan.synthesis_spin_into(&e_alm, &b_alm, &mut first, &mut second, &mut workspace)
        })
        .unwrap();
        std::hint::black_box((&first, &second));
        eprintln!(
            "PROFILE_TOTAL,{lmax},{mmax},{nphi},{phi0},{thread_count},{repetition},{:.6}",
            started.elapsed().as_secs_f64() * 1.0e3,
        );
        let factor_ns: u128 = workspace
            .recurrence_workers
            .iter()
            .map(|worker| worker.spin1_profile_factor_ns)
            .sum();
        let pair_prepare_ns: u128 = workspace
            .recurrence_workers
            .iter()
            .map(|worker| worker.spin1_profile_pair_prepare_ns)
            .sum();
        let recurrence_ns: u128 = workspace
            .recurrence_workers
            .iter()
            .map(|worker| worker.spin1_profile_recurrence_ns)
            .sum();
        let fallback_ns: u128 = workspace
            .recurrence_workers
            .iter()
            .map(|worker| worker.spin1_profile_fallback_ns)
            .sum();
        let pair_eligible_blocks: usize = workspace
            .recurrence_workers
            .iter()
            .map(|worker| worker.spin1_profile_pair_eligible_blocks)
            .sum();
        let pair_success_blocks: usize = workspace
            .recurrence_workers
            .iter()
            .map(|worker| worker.spin1_profile_pair_success_blocks)
            .sum();
        let fallback_blocks: usize = workspace
            .recurrence_workers
            .iter()
            .map(|worker| worker.spin1_profile_fallback_blocks)
            .sum();
        let safety_ns: u128 = workspace
            .recurrence_workers
            .iter()
            .map(|worker| worker.spin1_profile_coefficient_safety_ns)
            .sum();
        let safety_values: u128 = workspace
            .recurrence_workers
            .iter()
            .map(|worker| worker.spin1_profile_coefficient_safety_values)
            .sum();
        eprintln!(
            "PROFILE_INNER,{lmax},{mmax},{nphi},{phi0},{thread_count},{repetition},worker-sum order factors,{:.6}",
            factor_ns as f64 / 1.0e6,
        );
        eprintln!(
            "PROFILE_INNER,{lmax},{mmax},{nphi},{phi0},{thread_count},{repetition},worker-sum paired coefficient preparation,{:.6}",
            pair_prepare_ns as f64 / 1.0e6,
        );
        eprintln!(
            "PROFILE_INNER,{lmax},{mmax},{nphi},{phi0},{thread_count},{repetition},worker-sum fused recurrence and accumulation,{:.6}",
            recurrence_ns as f64 / 1.0e6,
        );
        eprintln!(
            "PROFILE_INNER,{lmax},{mmax},{nphi},{phi0},{thread_count},{repetition},worker-sum scaled/per-degree fallback,{:.6},fallback_blocks={fallback_blocks}",
            fallback_ns as f64 / 1.0e6,
        );
        eprintln!(
            "PROFILE_ROUTES,{lmax},{mmax},{nphi},{phi0},{thread_count},{repetition},pair_eligible={pair_eligible_blocks},pair_success={pair_success_blocks},fallback={fallback_blocks}"
        );
        eprintln!(
            "PROFILE_INNER,{lmax},{mmax},{nphi},{phi0},{thread_count},{repetition},worker-sum coefficient finite scans,{:.6},complex_values={safety_values}",
            safety_ns as f64 / 1.0e6,
        );
        for (worker_index, worker) in workspace
            .recurrence_workers
            .iter()
            .take(worker_count)
            .enumerate()
        {
            eprintln!(
                "PROFILE_WORKER,{lmax},{mmax},{nphi},{thread_count},{repetition},{worker_index},{},{:.3},{:.3},{:.3},{:.3},{},{}",
                worker.spin1_profile_group_worker_id,
                worker.spin1_profile_group_ns as f64 / 1.0e3,
                worker.spin1_profile_factor_ns as f64 / 1.0e3,
                worker.spin1_profile_pair_prepare_ns as f64 / 1.0e3,
                worker.spin1_profile_recurrence_ns as f64 / 1.0e3,
                worker.spin1_profile_pair_success_blocks,
                worker.spin1_profile_fallback_blocks,
            );
        }
    }
    for (repetition, observation) in probe.lock().unwrap().iter().enumerate() {
        eprintln!(
            "PROFILE_PHASE,{lmax},{mmax},{nphi},{phi0},{thread_count},{repetition},{},{:.6}",
            observation.phase,
            observation.elapsed_ns as f64 / 1.0e6,
        );
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "focused phase profiling helper; run explicitly with RUSTSHT_PROFILE_* variables"]
fn profile_spin1_standard_analysis_phases() {
    use std::time::Instant;

    let lmax = std::env::var("RUSTSHT_PROFILE_LMAX")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(720_usize);
    let mmax = std::env::var("RUSTSHT_PROFILE_MMAX")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(lmax);
    let grid_kind = match std::env::var("RUSTSHT_PROFILE_GRID").as_deref() {
        Ok("CC") => GridKind::ClenshawCurtis,
        Ok("F1") => GridKind::Fejer1,
        _ => GridKind::GaussLegendre,
    };
    let minimum_ntheta = lmax + usize::from(grid_kind == GridKind::ClenshawCurtis) + 1;
    let ntheta = std::env::var("RUSTSHT_PROFILE_NTHETA")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(minimum_ntheta);
    let nphi = std::env::var("RUSTSHT_PROFILE_NPHI")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| 2 * (lmax + 1));
    let thread_count = std::env::var("RUSTSHT_PROFILE_THREADS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8_usize);
    let phi0 = std::env::var("RUSTSHT_PROFILE_PHI0")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0.23_f64);
    let repetitions = std::env::var("RUSTSHT_PROFILE_REPETITIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8_usize);
    let plan = ShtPlan::new(
        GridSpec::new(grid_kind, ntheta, nphi, phi0).unwrap(),
        HarmonicSpec::new(lmax, mmax, 1).unwrap(),
    )
    .unwrap();
    let mut state = 0x7a31_9d42_8b6c_05e1_u64;
    let mut next_value = || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^= value >> 31;
        ((value >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64)) * 2.0 - 1.0
    };
    let mut first = vec![0.0; plan.sample_count];
    let mut second = vec![0.0; plan.sample_count];
    for value in &mut first {
        *value = next_value();
    }
    for value in &mut second {
        *value = next_value();
    }
    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let mut b_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .unwrap();
    let mut workspace = pool.install(|| plan.workspace()).unwrap();
    plan.prepare_workspace(&mut workspace, thread_count)
        .unwrap();
    let probe = workspace.observe_transform_phases();
    eprintln!(
        "PROFILE_CONFIG,grid={grid_kind:?},lmax={lmax},mmax={mmax},ntheta={ntheta},nphi={nphi},threads={thread_count},phi0={phi0}"
    );

    for _ in 0..3 {
        pool.install(|| {
            plan.analysis_spin_into(&first, &second, &mut e_alm, &mut b_alm, &mut workspace)
        })
        .unwrap();
        std::hint::black_box((&e_alm, &b_alm));
    }
    for repetition in 0..repetitions {
        probe.lock().unwrap().clear();
        let started = Instant::now();
        pool.install(|| {
            plan.analysis_spin_into(&first, &second, &mut e_alm, &mut b_alm, &mut workspace)
        })
        .unwrap();
        std::hint::black_box((&e_alm, &b_alm));
        eprintln!(
            "PROFILE_TOTAL,{grid_kind:?},{lmax},{mmax},{ntheta},{nphi},{thread_count},{repetition},{:.6}",
            started.elapsed().as_secs_f64() * 1.0e3,
        );
        for observation in probe.lock().unwrap().iter() {
            eprintln!(
                "PROFILE_PHASE,{grid_kind:?},{lmax},{mmax},{ntheta},{nphi},{thread_count},{repetition},{},{:.6}",
                observation.phase,
                observation.elapsed_ns as f64 / 1.0e6,
            );
        }
    }
}
