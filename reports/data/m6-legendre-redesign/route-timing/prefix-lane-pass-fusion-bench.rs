use std::hint::black_box;
use std::time::Instant;

use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn main() {
    let mut args = std::env::args().skip(1);
    let lmax: usize = args
        .next()
        .expect("supply lmax")
        .parse()
        .expect("lmax must be an integer");
    let map_path = args.next().expect("supply output map path");
    assert_eq!(lmax, 2047, "protocol v1 is frozen at T2047");

    let ntheta = lmax + 1;
    let nphi = 2 * (lmax + 1);
    let plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::scalar(lmax, lmax).unwrap(),
    )
    .unwrap();
    let mut workspace = plan.workspace().unwrap();
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    for m in 0..=lmax {
        for l in m..=lmax {
            let index = plan.harmonics().index(l, m).unwrap();
            let re = ((l * 7 + m * 11) % 19) as f64 - 9.0;
            let im = if m == 0 {
                0.0
            } else {
                ((l * 13 + m * 5) % 23) as f64 - 11.0
            };
            alm[index] = Complex64::new(re * 1e-5, im * 1e-5);
        }
    }
    let mut map = vec![0.0; ntheta * nphi];

    plan.synthesis_into(black_box(&alm), black_box(&mut map), &mut workspace)
        .unwrap();
    let warmup_map = map.clone();
    let mut samples_ms = Vec::with_capacity(5);
    for _ in 0..5 {
        let started = Instant::now();
        plan.synthesis_into(black_box(&alm), black_box(&mut map), &mut workspace)
            .unwrap();
        samples_ms.push(started.elapsed().as_secs_f64() * 1e3);
        assert!(
            map.iter()
                .zip(&warmup_map)
                .all(|(actual, expected)| actual.to_bits() == expected.to_bits()),
            "timed output differs from the warm-up output"
        );
        black_box(&map);
    }

    println!(
        "full_synthesis_samples_ms={}",
        samples_ms
            .iter()
            .map(|sample| format!("{sample:.3}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("workload=GL f64 scalar T{lmax} {ntheta}x{nphi} mmax={lmax} phi0=0.37 q1");

    let mut bytes = Vec::with_capacity(map.len() * std::mem::size_of::<f64>());
    for value in map {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write(map_path, bytes).expect("write output map");
}
