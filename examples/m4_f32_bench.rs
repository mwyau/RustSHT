// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::hint::black_box;
use std::mem::size_of;
use std::time::{Duration, Instant};

use rustsht::{Complex32, GridKind, GridSpec, HarmonicSpec, ShtPlan};

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
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::new(lmax, lmax, 1).unwrap(),
    )
    .unwrap();
    let mut workspace = rayon_pool.install(|| plan.workspace()).unwrap();
    let mut e = vec![Complex32::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b = vec![Complex32::new(0.0, 0.0); plan.harmonics().nalm()];
    for order in 0..=lmax {
        for degree in order.max(1)..=lmax {
            let index = plan.harmonics().index(degree, order).unwrap();
            let e_im = if order == 0 {
                0.0
            } else {
                ((degree * 13 + order * 5) % 23) as f32 - 11.0
            };
            let b_im = if order == 0 {
                0.0
            } else {
                ((degree * 5 + order * 17) % 29) as f32 - 14.0
            };
            e[index] = Complex32::new(((degree * 7 + order * 11) % 19) as f32 - 9.0, e_im) * 1.0e-5;
            b[index] = Complex32::new(((degree * 3 + order * 7) % 17) as f32 - 8.0, b_im) * 1.0e-5;
        }
    }

    let sample_count = ntheta * nphi;
    let mut first = vec![0.0_f32; sample_count];
    let mut second = vec![0.0_f32; sample_count];
    for ring in 0..ntheta {
        for longitude in 0..nphi {
            let i = ring as f32;
            let j = longitude as f32;
            let index = ring * nphi + longitude;
            first[index] = (0.017 * i + 0.023 * j).sin() + 0.2 * (0.011 * i - 0.031 * j).cos();
            second[index] = (0.019 * i - 0.013 * j).cos() - 0.15 * (0.029 * i + 0.007 * j).sin();
        }
    }
    let mut analyzed_e = vec![Complex32::new(0.0, 0.0); e.len()];
    let mut analyzed_b = vec![Complex32::new(0.0, 0.0); b.len()];

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

    let spectrum_len = nphi / 2 + 1;
    let worker_bytes = (lmax + 1) * size_of::<f32>()
        + 3 * 64 * size_of::<f32>()
        + 64 * size_of::<i64>()
        + 3 * (lmax + 2) * size_of::<f32>()
        + 6 * 64 * size_of::<f32>()
        + 2 * 64 * size_of::<i64>();
    let numeric_bytes = (first.len() + second.len()) * size_of::<f32>()
        + (e.len() + b.len() + analyzed_e.len() + analyzed_b.len()) * size_of::<Complex32>()
        + 2 * ntheta * spectrum_len * size_of::<Complex32>()
        + nphi * size_of::<f32>()
        + 2 * ntheta * size_of::<f32>()
        + ntheta * size_of::<usize>()
        + rayon_threads * worker_bytes
        + (lmax + 1) * size_of::<f32>()
        + 2 * ntheta * size_of::<f64>();
    println!(
        "spin-1 T{lmax}; GL {ntheta}x{nphi}; f32; 1 vs {rayon_threads} threads; one warmup, 5 samples, median"
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
    println!(
        "estimated_core_numeric_arrays_MiB={:.2}",
        numeric_bytes as f64 / 2_f64.powi(20)
    );
}
