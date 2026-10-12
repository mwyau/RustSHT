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
    let rayon_threads = 8;
    let one_thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let rayon_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(rayon_threads)
        .build()
        .unwrap();
    let plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::new(lmax, lmax, 1).unwrap(),
    )
    .unwrap();
    let mut workspace = rayon_pool.install(|| plan.workspace()).unwrap();
    let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    for m in 0..=lmax {
        for l in m.max(1)..=lmax {
            let index = plan.harmonics().index(l, m).unwrap();
            let e_im = if m == 0 {
                0.0
            } else {
                ((l * 13 + m * 5) % 23) as f64 - 11.0
            };
            let b_im = if m == 0 {
                0.0
            } else {
                ((l * 5 + m * 17) % 29) as f64 - 14.0
            };
            e[index] = Complex64::new(((l * 7 + m * 11) % 19) as f64 - 9.0, e_im) * 1e-5;
            b[index] = Complex64::new(((l * 3 + m * 7) % 17) as f64 - 8.0, b_im) * 1e-5;
        }
    }
    let sample_count = ntheta * nphi;
    let mut first = vec![0.0; sample_count];
    let mut second = vec![0.0; sample_count];
    let mut analyzed_e = vec![Complex64::new(0.0, 0.0); e.len()];
    let mut analyzed_b = vec![Complex64::new(0.0, 0.0); b.len()];

    let synthesis_one_thread = measure(|| {
        one_thread_pool.install(|| {
            plan.synthesis_spin_into(
                black_box(&e),
                black_box(&b),
                black_box(&mut first),
                black_box(&mut second),
                &mut workspace,
            )
            .unwrap();
            black_box(first[0]);
        });
    });
    let synthesis_rayon = measure(|| {
        rayon_pool.install(|| {
            plan.synthesis_spin_into(
                black_box(&e),
                black_box(&b),
                black_box(&mut first),
                black_box(&mut second),
                &mut workspace,
            )
            .unwrap();
            black_box(first[0]);
        });
    });
    let analysis_one_thread = measure(|| {
        one_thread_pool.install(|| {
            plan.analysis_spin_into(
                black_box(&first),
                black_box(&second),
                black_box(&mut analyzed_e),
                black_box(&mut analyzed_b),
                &mut workspace,
            )
            .unwrap();
            black_box(analyzed_e[0]);
        });
    });
    let analysis_rayon = measure(|| {
        rayon_pool.install(|| {
            plan.analysis_spin_into(
                black_box(&first),
                black_box(&second),
                black_box(&mut analyzed_e),
                black_box(&mut analyzed_b),
                &mut workspace,
            )
            .unwrap();
            black_box(analyzed_e[0]);
        });
    });

    println!(
        "spin-1 T{lmax}; GL {ntheta}x{nphi}; f64; 1 vs {rayon_threads} threads; 5 samples, median"
    );
    println!(
        "synthesis_one_thread_ms={:.3}",
        synthesis_one_thread.as_secs_f64() * 1000.0
    );
    println!(
        "synthesis_rayon_ms={:.3}",
        synthesis_rayon.as_secs_f64() * 1000.0
    );
    println!(
        "analysis_one_thread_ms={:.3}",
        analysis_one_thread.as_secs_f64() * 1000.0
    );
    println!(
        "analysis_rayon_ms={:.3}",
        analysis_rayon.as_secs_f64() * 1000.0
    );
}
