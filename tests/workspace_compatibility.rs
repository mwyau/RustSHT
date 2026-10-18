// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn cross_order_workspace(spin: usize, capacity: usize) {
    let lmax = if spin == 0 { 511 } else { 255 };
    let grid = GridSpec::new(GridKind::GaussLegendre, lmax + 1, 2 * (lmax + 1), 0.37).unwrap();
    let low = ShtPlan::new(grid.clone(), HarmonicSpec::new(lmax, 0, spin).unwrap()).unwrap();
    let full = ShtPlan::new(grid, HarmonicSpec::new(lmax, lmax, spin).unwrap()).unwrap();
    let mut workspace = low.workspace().unwrap();
    low.prepare_workspace(&mut workspace, capacity).unwrap();
    let prior_workers = workspace.allocated_worker_count();
    let preparation = full.prepare_workspace(&mut workspace, capacity);
    let map = vec![0.0; (lmax + 1) * 2 * (lmax + 1)];
    let sentinel = Complex64::new(7.0, -2.0);
    let mut e = vec![sentinel; full.harmonics().nalm()];
    let mut b = e.clone();
    let analysis = if spin == 0 {
        full.analysis_into(&map, &mut e, &mut workspace)
    } else {
        full.analysis_spin_into(&map, &map, &mut e, &mut b, &mut workspace)
    };
    preparation.unwrap();
    analysis.unwrap();
    assert!(e.iter().all(|value| value.norm() == 0.0));
    if spin != 0 {
        assert!(b.iter().all(|value| value.norm() == 0.0));
    }
    assert_eq!(workspace.execution_capacity(), capacity);
    assert_eq!(workspace.allocated_worker_count(), prior_workers);

    // A workspace prepared for the larger order plan also serves the reduced
    // order plan without reallocating or changing the execution budget.
    let mut larger = full.workspace().unwrap();
    low.prepare_workspace(&mut larger, capacity).unwrap();
    let mut alm = vec![Complex64::new(0.0, 0.0); low.harmonics().nalm()];
    alm[spin] = Complex64::new(0.75, 0.0);
    let mut first = map.clone();
    let mut second = map.clone();
    let mut expected_first = map.clone();
    let mut expected_second = map.clone();
    if spin == 0 {
        low.synthesis_into(&alm, &mut first, &mut larger).unwrap();
        low.synthesis_into(&alm, &mut expected_first, &mut workspace)
            .unwrap();
        low.analysis_into(&first, &mut alm, &mut larger).unwrap();
    } else {
        let zero = vec![Complex64::new(0.0, 0.0); alm.len()];
        low.synthesis_spin_into(&alm, &zero, &mut first, &mut second, &mut larger)
            .unwrap();
        low.synthesis_spin_into(
            &alm,
            &zero,
            &mut expected_first,
            &mut expected_second,
            &mut workspace,
        )
        .unwrap();
        let mut recovered_b = zero;
        low.analysis_spin_into(&first, &second, &mut alm, &mut recovered_b, &mut larger)
            .unwrap();
        assert!(recovered_b.iter().all(|value| value.norm() < 1.0e-12));
    }
    assert_eq!(first, expected_first);
    assert_eq!(second, expected_second);
    assert!((alm[spin].re - 0.75).abs() < 1.0e-12);
    assert_eq!(larger.execution_capacity(), capacity);
}

#[test]
fn cross_order_workspace_compatibility() {
    #[cfg(feature = "parallel")]
    for capacity in [1, 2, 3] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(capacity)
            .build()
            .unwrap();
        pool.install(|| {
            for spin in [0, 1] {
                cross_order_workspace(spin, capacity);
            }
        });
    }
    #[cfg(not(feature = "parallel"))]
    for spin in [0, 1] {
        cross_order_workspace(spin, 1);
    }
}
