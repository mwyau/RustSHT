use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use num_complex::Complex64;
use rayon::ThreadPool;
use rayon::prelude::*;
use rustfft::{Fft, FftPlanner};
use rustsht_fft::{RealFftPlan, RealFftWorkspace};

const N: usize = 361;
const ROWS: usize = 721;
const THREADS: usize = 8;
const BATCHES: usize = 30;
const TARGET_BATCH: Duration = Duration::from_millis(30);

struct ComplexWorkspace {
    transform: Vec<Complex64>,
    scratch: Vec<Complex64>,
}

fn fixture() -> Vec<Complex64> {
    (0..ROWS)
        .flat_map(|row| {
            (0..=N / 2).map(move |k| {
                if k == 0 {
                    Complex64::new(0.2 + row as f64 * 1.0e-4, 0.0)
                } else {
                    let x = (row * 181 + k + 1) as f64;
                    Complex64::new((x * 0.017).sin() * 0.7, (x * 0.013).cos() * 0.4)
                }
            })
        })
        .collect()
}

fn run_complex(
    input: &[Complex64],
    output: &mut [f64],
    plan: &Arc<dyn Fft<f64>>,
    workspaces: &mut [ComplexWorkspace],
    pool: &ThreadPool,
) {
    let bins = N / 2 + 1;
    let rows_per_worker = ROWS.div_ceil(THREADS);
    let output_chunk = rows_per_worker * N;
    pool.install(|| {
        output
            .par_chunks_mut(output_chunk)
            .zip(workspaces.par_iter_mut())
            .enumerate()
            .for_each(|(group, (out_rows, workspace))| {
                let first = group * rows_per_worker;
                for (offset, out) in out_rows.chunks_mut(N).enumerate() {
                    let row = first + offset;
                    let modes = &input[row * bins..(row + 1) * bins];
                    workspace.transform[0] = Complex64::new(modes[0].re, 0.0);
                    for (k, &mode) in modes.iter().enumerate().skip(1) {
                        workspace.transform[k] = mode;
                        workspace.transform[N - k] = mode.conj();
                    }
                    plan.process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
                    for (sample, value) in out.iter_mut().zip(&workspace.transform) {
                        *sample = value.re;
                    }
                }
            });
    });
}

fn run_real(
    input: &[Complex64],
    output: &mut [f64],
    plan: &RealFftPlan<f64>,
    workspaces: &mut [RealFftWorkspace<f64>],
    pool: &ThreadPool,
) {
    let bins = N / 2 + 1;
    let rows_per_worker = ROWS.div_ceil(THREADS);
    let output_chunk = rows_per_worker * N;
    pool.install(|| {
        output
            .par_chunks_mut(output_chunk)
            .zip(workspaces.par_iter_mut())
            .enumerate()
            .for_each(|(group, (out_rows, workspace))| {
                let first = group * rows_per_worker;
                for (offset, out) in out_rows.chunks_mut(N).enumerate() {
                    let row = first + offset;
                    let modes = &input[row * bins..(row + 1) * bins];
                    plan.inverse(modes, out, workspace).expect("real inverse");
                    for sample in out {
                        *sample *= N as f64;
                    }
                }
            });
    });
}

fn elapsed_per_call<F: FnMut()>(calls: usize, mut operation: F) -> f64 {
    let start = Instant::now();
    for _ in 0..calls {
        operation();
        black_box(0x91a7_u64);
    }
    start.elapsed().as_secs_f64() * 1.0e3 / calls as f64
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn main() {
    let mut planner = FftPlanner::<f64>::new();
    let complex_plan = planner.plan_fft_inverse(N);
    let complex_scratch = complex_plan.get_inplace_scratch_len();
    let real_plan = RealFftPlan::<f64>::new(N).expect("real plan");
    let input = fixture();
    let mut complex_output = vec![0.0; ROWS * N];
    let mut real_output = vec![0.0; ROWS * N];
    let mut complex_workspaces: Vec<_> = (0..THREADS)
        .map(|_| ComplexWorkspace {
            transform: vec![Complex64::new(0.0, 0.0); N],
            scratch: vec![Complex64::new(0.0, 0.0); complex_scratch],
        })
        .collect();
    let mut real_workspaces: Vec<_> = (0..THREADS)
        .map(|_| real_plan.workspace().expect("real workspace"))
        .collect();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(THREADS)
        .build()
        .expect("thread pool");

    run_complex(
        &input,
        &mut complex_output,
        &complex_plan,
        &mut complex_workspaces,
        &pool,
    );
    run_real(
        &input,
        &mut real_output,
        &real_plan,
        &mut real_workspaces,
        &pool,
    );
    let (mut l2, mut max_abs, mut norm) = (0.0_f64, 0.0_f64, 0.0_f64);
    for (&a, &b) in complex_output.iter().zip(&real_output) {
        l2 += (a - b) * (a - b);
        norm += a * a;
        max_abs = max_abs.max((a - b).abs());
    }
    println!(
        "validation,relative_l2={:.9e},max_abs={:.9e}",
        (l2 / norm).sqrt(),
        max_abs
    );

    for _ in 0..8 {
        run_complex(
            &input,
            &mut complex_output,
            &complex_plan,
            &mut complex_workspaces,
            &pool,
        );
        run_real(
            &input,
            &mut real_output,
            &real_plan,
            &mut real_workspaces,
            &pool,
        );
    }
    let complex_ms = elapsed_per_call(12, || {
        run_complex(
            &input,
            &mut complex_output,
            &complex_plan,
            &mut complex_workspaces,
            &pool,
        )
    });
    let real_ms = elapsed_per_call(12, || {
        run_real(
            &input,
            &mut real_output,
            &real_plan,
            &mut real_workspaces,
            &pool,
        )
    });
    let calls =
        ((TARGET_BATCH.as_secs_f64() * 1.0e3 / complex_ms.max(real_ms)).round() as usize).max(1);
    println!(
        "environment,n={N},rows={ROWS},threads={THREADS},batch_calls={calls},batches={BATCHES},target_ms={:.1}",
        TARGET_BATCH.as_secs_f64() * 1.0e3
    );

    let mut c_batches = Vec::with_capacity(BATCHES);
    let mut r_batches = Vec::with_capacity(BATCHES);
    for batch in 0..BATCHES {
        if batch % 2 == 0 {
            c_batches.push(elapsed_per_call(calls, || {
                run_complex(
                    &input,
                    &mut complex_output,
                    &complex_plan,
                    &mut complex_workspaces,
                    &pool,
                )
            }));
            r_batches.push(elapsed_per_call(calls, || {
                run_real(
                    &input,
                    &mut real_output,
                    &real_plan,
                    &mut real_workspaces,
                    &pool,
                )
            }));
        } else {
            r_batches.push(elapsed_per_call(calls, || {
                run_real(
                    &input,
                    &mut real_output,
                    &real_plan,
                    &mut real_workspaces,
                    &pool,
                )
            }));
            c_batches.push(elapsed_per_call(calls, || {
                run_complex(
                    &input,
                    &mut complex_output,
                    &complex_plan,
                    &mut complex_workspaces,
                    &pool,
                )
            }));
        }
        black_box(complex_output[batch % complex_output.len()]);
        black_box(real_output[batch % real_output.len()]);
    }
    let mut c_sorted = c_batches.clone();
    let mut r_sorted = r_batches.clone();
    let c_median = median(&mut c_sorted);
    let c_p10 = c_sorted[(BATCHES - 1) / 10];
    let c_p90 = c_sorted[(BATCHES - 1) * 9 / 10];
    let r_median = median(&mut r_sorted);
    let r_p10 = r_sorted[(BATCHES - 1) / 10];
    let r_p90 = r_sorted[(BATCHES - 1) * 9 / 10];
    println!(
        "summary,complex_p10_median_p90_ms={c_p10:.6},{c_median:.6},{c_p90:.6},real_p10_median_p90_ms={r_p10:.6},{r_median:.6},{r_p90:.6},ratio={:.5}",
        r_median / c_median
    );
    for i in 0..BATCHES {
        println!(
            "batch,{i},complex_ms={:.6},real_ms={:.6}",
            c_batches[i], r_batches[i]
        );
    }
}
