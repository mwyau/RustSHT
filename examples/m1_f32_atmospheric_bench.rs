// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::hint::black_box;
use std::time::{Duration, Instant};

use rustsht::{Complex32, GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn median(mut durations: Vec<Duration>) -> Duration {
    assert!(!durations.is_empty());
    durations.sort_unstable();
    let middle = durations.len() / 2;
    if durations.len() % 2 == 1 {
        durations[middle]
    } else {
        durations[middle - 1] + (durations[middle] - durations[middle - 1]) / 2
    }
}

fn warm_sample_count() -> usize {
    let count = std::env::var("RUSTSHT_M1_SAMPLES")
        .map(|value| {
            value
                .parse::<usize>()
                .expect("RUSTSHT_M1_SAMPLES must be an integer")
        })
        .unwrap_or(5);
    assert!(count > 0, "RUSTSHT_M1_SAMPLES must be positive");
    count
}

fn measure(mut operation: impl FnMut()) -> (Duration, Vec<Duration>) {
    let start = Instant::now();
    operation();
    let first = start.elapsed();
    let sample_count = warm_sample_count();
    let mut samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let start = Instant::now();
        operation();
        samples.push(start.elapsed());
    }
    (first, samples)
}

fn milliseconds(durations: &[Duration]) -> String {
    durations
        .iter()
        .map(|duration| format!("{:.3}", duration.as_secs_f64() * 1000.0))
        .collect::<Vec<_>>()
        .join(",")
}

fn rss_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn dump_f32_if_requested(label: &str, values: &[f32]) {
    let Some(directory) = std::env::var_os("RUSTSHT_F32_DUMP_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(values));
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write(directory.join(label), bytes).unwrap();
}

fn dump_complex32_if_requested(label: &str, values: &[Complex32]) {
    let Some(directory) = std::env::var_os("RUSTSHT_F32_DUMP_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(values));
    for value in values {
        bytes.extend_from_slice(&value.re.to_le_bytes());
        bytes.extend_from_slice(&value.im.to_le_bytes());
    }
    std::fs::write(directory.join(label), bytes).unwrap();
}

fn main() {
    let mut args = std::env::args().skip(1);
    let spin: usize = args.next().unwrap_or_else(|| "1".into()).parse().unwrap();
    let lmax: usize = args.next().unwrap_or_else(|| "511".into()).parse().unwrap();
    let nphi: usize = args
        .next()
        .unwrap_or_else(|| (2 * lmax + 3).to_string())
        .parse()
        .unwrap();
    let threads: usize = args.next().unwrap_or_else(|| "8".into()).parse().unwrap();
    assert!(matches!(spin, 0 | 1));
    assert!(threads > 0);

    let ntheta = lmax + 1;
    let plan_started = Instant::now();
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::new(lmax, lmax, spin).unwrap(),
    )
    .unwrap();
    let plan_ms = plan_started.elapsed().as_secs_f64() * 1000.0;
    let one_thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    let workspace_started = Instant::now();
    let mut workspace = pool.install(|| plan.workspace()).unwrap();
    let workspace_ms = workspace_started.elapsed().as_secs_f64() * 1000.0;

    let nalm = plan.harmonics().nalm();
    let sample_count = ntheta * nphi;
    let mut e = vec![Complex32::new(0.0, 0.0); nalm];
    let mut b = vec![Complex32::new(0.0, 0.0); nalm];
    let first_count = if spin == 0 { 1 } else { 2 };
    let warm_samples = warm_sample_count();
    for m in 0..=lmax {
        for l in m.max(spin)..=lmax {
            let index = plan.harmonics().index(l, m).unwrap();
            let e_im = if m == 0 {
                0.0
            } else {
                ((l * 13 + m * 5) % 23) as f32 - 11.0
            };
            let b_im = if m == 0 {
                0.0
            } else {
                ((l * 5 + m * 17) % 29) as f32 - 14.0
            };
            e[index] = Complex32::new(((l * 7 + m * 11) % 19) as f32 - 9.0, e_im) * 1.0e-5;
            b[index] = Complex32::new(((l * 3 + m * 7) % 17) as f32 - 8.0, b_im) * 1.0e-5;
        }
    }

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
    let mut analyzed_e = vec![Complex32::new(0.0, 0.0); nalm];
    let mut analyzed_b = vec![Complex32::new(0.0, 0.0); nalm];

    let (synthesis_q1_first, synthesis_q1) = measure(|| {
        one_thread_pool.install(|| {
            if spin == 0 {
                plan.synthesis_into(black_box(&e), black_box(&mut first[..]), &mut workspace)
                    .unwrap();
            } else {
                plan.synthesis_spin_into(
                    black_box(&e),
                    black_box(&b),
                    black_box(&mut first[..]),
                    black_box(&mut second[..]),
                    &mut workspace,
                )
                .unwrap();
            }
            black_box(first[0]);
        });
    });
    let (synthesis_qn_first, synthesis_qn) = measure(|| {
        pool.install(|| {
            if spin == 0 {
                plan.synthesis_into(black_box(&e), black_box(&mut first[..]), &mut workspace)
                    .unwrap();
            } else {
                plan.synthesis_spin_into(
                    black_box(&e),
                    black_box(&b),
                    black_box(&mut first[..]),
                    black_box(&mut second[..]),
                    &mut workspace,
                )
                .unwrap();
            }
            black_box(first[0]);
        });
    });
    let (analysis_q1_first, analysis_q1) = measure(|| {
        one_thread_pool.install(|| {
            if spin == 0 {
                plan.analysis_into(
                    black_box(&first[..]),
                    black_box(&mut analyzed_e),
                    &mut workspace,
                )
                .unwrap();
            } else {
                plan.analysis_spin_into(
                    black_box(&first[..]),
                    black_box(&second[..]),
                    black_box(&mut analyzed_e),
                    black_box(&mut analyzed_b),
                    &mut workspace,
                )
                .unwrap();
            }
            black_box(analyzed_e[0]);
        });
    });
    let (analysis_qn_first, analysis_qn) = measure(|| {
        pool.install(|| {
            if spin == 0 {
                plan.analysis_into(
                    black_box(&first[..]),
                    black_box(&mut analyzed_e),
                    &mut workspace,
                )
                .unwrap();
            } else {
                plan.analysis_spin_into(
                    black_box(&first[..]),
                    black_box(&second[..]),
                    black_box(&mut analyzed_e),
                    black_box(&mut analyzed_b),
                    &mut workspace,
                )
                .unwrap();
            }
            black_box(analyzed_e[0]);
        });
    });

    println!(
        "spin={spin} T{lmax}; GL {ntheta}x{nphi}; f32; {first_count} map component(s); q1 vs q{threads}; one warmup and {warm_samples} warm sample(s) per operation"
    );
    println!("plan_construction_ms={plan_ms:.3}");
    println!("workspace_allocation_ms={workspace_ms:.3}");
    println!(
        "synthesis_q1_first_ms={:.3}",
        synthesis_q1_first.as_secs_f64() * 1000.0
    );
    println!("synthesis_q1_samples_ms={}", milliseconds(&synthesis_q1));
    println!(
        "synthesis_q1_median_ms={:.3}",
        median(synthesis_q1.clone()).as_secs_f64() * 1000.0
    );
    println!(
        "synthesis_qn_first_ms={:.3}",
        synthesis_qn_first.as_secs_f64() * 1000.0
    );
    println!("synthesis_qn_samples_ms={}", milliseconds(&synthesis_qn));
    println!(
        "synthesis_qn_median_ms={:.3}",
        median(synthesis_qn.clone()).as_secs_f64() * 1000.0
    );
    println!(
        "analysis_q1_first_ms={:.3}",
        analysis_q1_first.as_secs_f64() * 1000.0
    );
    println!("analysis_q1_samples_ms={}", milliseconds(&analysis_q1));
    println!(
        "analysis_q1_median_ms={:.3}",
        median(analysis_q1.clone()).as_secs_f64() * 1000.0
    );
    println!(
        "analysis_qn_first_ms={:.3}",
        analysis_qn_first.as_secs_f64() * 1000.0
    );
    println!("analysis_qn_samples_ms={}", milliseconds(&analysis_qn));
    println!(
        "analysis_qn_median_ms={:.3}",
        median(analysis_qn.clone()).as_secs_f64() * 1000.0
    );
    println!("peak_rss_kib={}", rss_kib());

    if std::env::var_os("RUSTSHT_F32_DUMP_DIR").is_some() {
        dump_f32_if_requested(&format!("spin{spin}-synthesis-channel0.f32"), &first);
        dump_complex32_if_requested(&format!("spin{spin}-analysis-channel0.c32"), &analyzed_e);
        if spin == 1 {
            dump_f32_if_requested("spin1-synthesis-channel1.f32", &second);
            dump_complex32_if_requested("spin1-analysis-channel1.c32", &analyzed_b);
        }
    }
}
