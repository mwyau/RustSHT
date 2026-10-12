use std::hint::black_box;
use std::time::{Duration, Instant};

use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn median(mut durations: Vec<Duration>) -> Duration {
    durations.sort_unstable();
    durations[durations.len() / 2]
}

fn measure(mut operation: impl FnMut()) -> Duration {
    operation();
    let mut durations = Vec::with_capacity(5);
    for _ in 0..5 {
        let start = Instant::now();
        operation();
        durations.push(start.elapsed());
    }
    median(durations)
}

fn main() {
    let lmax = 511;
    let ntheta = lmax + 1;
    let nphi = 2 * lmax + 3;
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
    let mut analyzed = vec![Complex64::new(0.0, 0.0); alm.len()];

    let synthesis = measure(|| {
        plan.synthesis_into(black_box(&alm), black_box(&mut map), &mut workspace)
            .unwrap();
        black_box(map[0]);
    });
    let analysis = measure(|| {
        plan.analysis_into(black_box(&map), black_box(&mut analyzed), &mut workspace)
            .unwrap();
        black_box(analyzed[0]);
    });

    let spectrum_len = nphi / 2 + 1;
    let core_bytes = map.len() * size_of::<f64>()
        + alm.len() * size_of::<Complex64>()
        + analyzed.len() * size_of::<Complex64>()
        + ntheta * spectrum_len * size_of::<Complex64>()
        + nphi * size_of::<f64>()
        + (lmax + 1) * size_of::<f64>()
        + 2 * ntheta * size_of::<f64>()
        + ntheta * size_of::<bool>()
        + 64 * (3 * size_of::<f64>() + size_of::<i64>())
        + 2 * ntheta * size_of::<f64>()
        + (lmax + 1) * size_of::<f64>();
    println!("T{lmax}; GL {ntheta}x{nphi}; f64; one thread; 5 samples, median");
    println!("synthesis_ms={:.3}", synthesis.as_secs_f64() * 1000.0);
    println!("analysis_ms={:.3}", analysis.as_secs_f64() * 1000.0);
    println!(
        "estimated_numeric_arrays_MiB={:.2}",
        core_bytes as f64 / 2f64.powi(20)
    );
}
