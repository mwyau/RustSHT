// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

#[test]
fn high_bandlimit_extreme_finite_monopole_stays_finite() {
    let lmax = 511;
    let ntheta = lmax + 1;
    let nphi = 2 * ntheta;
    let plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::scalar(lmax, lmax).unwrap(),
    )
    .unwrap();
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    alm[plan.harmonics().index(0, 0).unwrap()] = Complex64::new(1.0e300, 0.0);
    let mut workspace = plan.workspace().unwrap();
    let mut map = vec![0.0; ntheta * nphi];
    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
    let expected = 1.0e300 / (4.0 * std::f64::consts::PI).sqrt();
    assert!(map.iter().all(|value| value.is_finite()));
    assert!(
        map.iter()
            .all(|&value| (value / expected - 1.0).abs() < 1.0e-12)
    );
}
