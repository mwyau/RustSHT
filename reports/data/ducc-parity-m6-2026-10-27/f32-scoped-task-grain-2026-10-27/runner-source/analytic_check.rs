use rustsht::{Complex32, GridKind, GridSpec, HarmonicSpec, ShtPlan};
use std::f32::consts::PI;
use std::f64::consts::TAU;

fn metrics(actual: &[f32], expected: &[f32]) -> (f64, f32) {
    let mut diff2 = 0.0_f64;
    let mut expected2 = 0.0_f64;
    let mut max_abs = 0.0_f32;
    for (&a, &e) in actual.iter().zip(expected) {
        let d = f64::from(a) - f64::from(e);
        diff2 += d * d;
        expected2 += f64::from(e) * f64::from(e);
        max_abs = max_abs.max((a - e).abs());
    }
    ((diff2 / expected2).sqrt(), max_abs)
}

fn main() {
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, 721, 1458, 0.37).unwrap(),
        HarmonicSpec::scalar(720, 720).unwrap(),
    ).unwrap();
    let spec = plan.harmonics();
    let mut workspace = plan.workspace().unwrap();

    let mut alm = vec![Complex32::new(0.0, 0.0); spec.nalm()];
    for (l, v) in [(0, 0.7_f32), (1, -0.2), (2, 0.1)] {
        alm[spec.index(l, 0).unwrap()] = Complex32::new(v, 0.0);
    }
    let mut map = vec![f32::NAN; plan.grid().ntheta() * plan.grid().nphi()];
    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
    assert!(map.iter().all(|v| v.is_finite()));
    let mut expected = Vec::with_capacity(map.len());
    for &theta64 in plan.grid().theta() {
        let cosine = (theta64 as f32).cos();
        let value = 0.7 / (4.0 * PI).sqrt()
            - 0.2 * (3.0 / (4.0 * PI)).sqrt() * cosine
            + 0.1 * (5.0 / (16.0 * PI)).sqrt() * (3.0 * cosine * cosine - 1.0);
        expected.extend(std::iter::repeat_n(value, plan.grid().nphi()));
    }
    let (analytic_l2, analytic_max) = metrics(&map, &expected);
    let first = map.clone();
    map.fill(1.2345);
    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
    assert_eq!(map.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
               first.iter().map(|v| v.to_bits()).collect::<Vec<_>>());

    let mut recovered = vec![Complex32::new(0.0, 0.0); alm.len()];
    plan.analysis_into(&map, &mut recovered, &mut workspace).unwrap();
    let mut coef_diff2 = 0.0_f64;
    let mut coef_ref2 = 0.0_f64;
    let mut coef_max = 0.0_f32;
    for (&a, &e) in recovered.iter().zip(&alm) {
        for (av, ev) in [(a.re, e.re), (a.im, e.im)] {
            let d = f64::from(av) - f64::from(ev);
            coef_diff2 += d*d;
            coef_ref2 += f64::from(ev)*f64::from(ev);
            coef_max = coef_max.max((av-ev).abs());
        }
    }
    let coef_l2 = (coef_diff2 / coef_ref2).sqrt();

    let probe: Vec<f32> = (0..map.len()).map(|i| {
        let x = i as f64;
        ((0.013 * x).sin() + 0.2 * (0.0017 * x).cos()) as f32
    }).collect();
    let mut adjoint = vec![Complex32::new(0.0, 0.0); alm.len()];
    plan.adjoint_synthesis_into(&probe, &mut adjoint, &mut workspace).unwrap();
    let lhs: f64 = map.iter().zip(&probe).map(|(&a, &b)| f64::from(a)*f64::from(b)).sum();
    let rhs: f64 = alm.iter().zip(&adjoint).map(|(a, b)| {
        f64::from(a.re) * f64::from(b.re) + f64::from(a.im) * f64::from(b.im)
    }).sum();
    let adjoint_rel = (lhs-rhs).abs()/lhs.abs().max(rhs.abs()).max(1.0e-30);

    println!("low_modes analytic_rel_l2={analytic_l2:.9e} analytic_max_abs={analytic_max:.9e} coefficient_rel_l2={coef_l2:.9e} coefficient_max_abs={coef_max:.9e} adjoint_rel_error={adjoint_rel:.9e} repeated_bitwise=true");

    let mut sectoral = vec![Complex32::new(0.0, 0.0); alm.len()];
    sectoral[spec.index(720, 720).unwrap()] = Complex32::new(1.0, 0.0);
    let mut sectoral_map = vec![f32::NAN; map.len()];
    plan.synthesis_into(&sectoral, &mut sectoral_map, &mut workspace).unwrap();
    assert!(sectoral_map.iter().all(|v| v.is_finite()));
    let mut norm = 1.0_f64 / (4.0 * std::f64::consts::PI).sqrt();
    for m in 1..=720 { norm *= ((2*m+1) as f64/(2*m) as f64).sqrt(); }
    let mut sectoral_ref = Vec::with_capacity(map.len());
    for &theta in plan.grid().theta() {
        let amp = 2.0 * norm * theta.sin().powi(720);
        for j in 0..plan.grid().nphi() {
            let phi = plan.grid().phi0() + TAU * j as f64 / plan.grid().nphi() as f64;
            sectoral_ref.push((amp * (720.0*phi).cos()) as f32);
        }
    }
    let (sectoral_l2, sectoral_max) = metrics(&sectoral_map, &sectoral_ref);
    println!("sectoral_l720 analytic_rel_l2={sectoral_l2:.9e} analytic_max_abs={sectoral_max:.9e} finite=true");

    for (label, value) in [
        ("min_positive", f32::MIN_POSITIVE),
        ("smallest_subnormal", f32::from_bits(1)),
        ("large_finite", 1.0e30_f32),
        ("max_finite", f32::MAX),
    ] {
        let mut extreme = vec![Complex32::new(0.0, 0.0); alm.len()];
        extreme[spec.index(0,0).unwrap()] = Complex32::new(value, 0.0);
        let mut out = vec![0.0_f32; map.len()];
        plan.synthesis_into(&extreme, &mut out, &mut workspace).unwrap();
        assert!(out.iter().all(|v| v.is_finite()));
        let checksum: f64 = out.iter().map(|&v| f64::from(v)).sum();
        println!("extreme_{label} finite=true checksum={checksum:.9e}");
    }
}
