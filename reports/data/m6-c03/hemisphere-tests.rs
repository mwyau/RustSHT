// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{Complex64, GridKind, GridSpec, HarmonicSpec};

#[test]
fn extreme_spin1_hemisphere_restores_original_coefficients() {
    for lmax in [319, 511] {
        let plan = ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, lmax + 1, 2 * (lmax + 1), 0.37).unwrap(),
            HarmonicSpec::new(lmax, 1, 1).unwrap(),
        )
        .unwrap();
        let mut workspace = plan.workspace().unwrap();
        plan.prepare_spin_latitudes(&mut workspace, 1);
        let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut b = e.clone();
        e[plan.harmonics.index(2, 1).unwrap()] = Complex64::new(1.44e308, 0.0);
        b[plan.harmonics.index(1, 1).unwrap()] = Complex64::new(0.0, -1.44e308);
        let spectrum_len = plan.fft.spectrum_len();
        let group_len = 67;
        let mut cache = Vec::new();
        assert!(plan.prepare_spin1_scalar_coefficients(&e, &b, &mut cache, 1));
        for coefficient_cache in [None, Some(cache.as_slice())] {
            let mut north_q = vec![Complex64::new(0.0, 0.0); group_len * spectrum_len];
            let mut north_u = north_q.clone();
            let mut south_q = north_q.clone();
            let mut south_u = north_q.clone();
            synthesize_group(
                &plan,
                &e,
                &b,
                coefficient_cache,
                0,
                group_len,
                plan.grid.nphi(),
                spectrum_len,
                &workspace.theta_sin,
                &workspace.theta_cos,
                &workspace.ring_order_limits,
                &mut north_q,
                &mut south_q,
                &mut north_u,
                &mut south_u,
                pulp::Arch::new(),
                &mut workspace.recurrence_workers[0],
            )
            .unwrap();
            for (ring_start, actual_q, actual_u) in [
                (0, &north_q, &north_u),
                (lmax + 1 - group_len, &south_q, &south_u),
            ] {
                let mut reference_q = vec![Complex64::new(0.0, 0.0); group_len * spectrum_len];
                let mut reference_u = reference_q.clone();
                plan.synthesize_spin_ring_group_mode(
                    &e,
                    Some(&b),
                    1,
                    false,
                    false,
                    false,
                    None,
                    plan.grid.nphi(),
                    spectrum_len,
                    plan.grid.phi0(),
                    ring_start,
                    ring_start + group_len,
                    &mut reference_q,
                    &mut reference_u,
                    &workspace.theta_sin[ring_start..ring_start + group_len],
                    &workspace.theta_cos[ring_start..ring_start + group_len],
                    &workspace.ring_order_limits,
                    pulp::Arch::new(),
                    &mut workspace.recurrence_workers[0],
                )
                .unwrap();
                let mut finite_count = 0;
                for (actual, expected) in actual_q
                    .iter()
                    .chain(actual_u)
                    .zip(reference_q.iter().chain(&reference_u))
                {
                    for (actual, expected) in [(actual.re, expected.re), (actual.im, expected.im)] {
                        if expected.is_finite() {
                            assert!(
                                actual.is_finite(),
                                "L{lmax} start={ring_start} actual={actual} expected={expected}"
                            );
                            assert!((actual / 1.44e308 - expected / 1.44e308).abs() < 2e-12);
                            finite_count += usize::from(expected != 0.0);
                        }
                    }
                }
                assert!(finite_count > group_len);
            }
        }
    }
}
