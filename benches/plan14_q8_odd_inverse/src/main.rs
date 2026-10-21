use std::env;
use std::fs;
use std::hint::black_box;
use std::time::{Duration, Instant};

use num_complex::Complex64;
use rayon::ThreadPool;
use rayon::prelude::*;

#[path = "../../../src/error.rs"]
#[allow(dead_code)]
mod error;
#[path = "../../../src/fft.rs"]
#[allow(dead_code)]
mod fft;

use fft::{RealFftPlan, RealFftWorkspace};

const BATCHES: usize = 30;
const TARGET_BATCH: Duration = Duration::from_millis(30);
const MMAX: usize = 180;

#[derive(Clone, Copy)]
struct Geometry {
    n: usize,
    rows: usize,
    threads: usize,
}

fn fixture(n: usize, rows: usize) -> Vec<Complex64> {
    let bins = n / 2 + 1;
    (0..rows)
        .flat_map(|row| {
            (0..bins).map(move |k| {
                if k > MMAX {
                    Complex64::new(0.0, 0.0)
                } else if k == 0 {
                    Complex64::new(0.2 + row as f64 * 1.0e-4, 0.0)
                } else {
                    let x = (row * bins + k + 1) as f64;
                    Complex64::new((x * 0.017).sin() * 0.7, (x * 0.013).cos() * 0.4)
                }
            })
        })
        .collect()
}

fn read_spectra(path: &std::path::Path, n: usize, rows: usize) -> Vec<Complex64> {
    let bytes = fs::read(path).expect("read shared complex-spectrum fixture");
    assert_eq!(bytes.len(), rows * (n / 2 + 1) * 16);
    let (chunks, remainder) = bytes.as_chunks::<16>();
    assert!(remainder.is_empty());
    chunks
        .iter()
        .map(|chunk| {
            Complex64::new(
                f64::from_le_bytes(chunk[..8].try_into().unwrap()),
                f64::from_le_bytes(chunk[8..].try_into().unwrap()),
            )
        })
        .collect()
}

fn write_fixture(n: usize, rows: usize, path: &std::path::Path) {
    let values = fixture(n, rows);
    let mut bytes = Vec::with_capacity(values.len() * 16);
    for value in values {
        bytes.extend_from_slice(&value.re.to_le_bytes());
        bytes.extend_from_slice(&value.im.to_le_bytes());
    }
    fs::write(path, bytes).expect("write shared complex-spectrum fixture");
}

fn write_real_output(path: &std::path::Path, output: &[f64]) {
    let mut bytes = Vec::with_capacity(output.len() * 8);
    for &value in output {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    fs::write(path, bytes).expect("write measured inverse output");
}

fn run_current(
    geometry: Geometry,
    input: &[Complex64],
    output: &mut [f64],
    plan: &RealFftPlan<f64>,
    workspaces: &mut [RealFftWorkspace<f64>],
    pool: &ThreadPool,
) {
    let Geometry { n, rows, threads } = geometry;
    let bins = plan.spectrum_len();
    let rows_per_worker = rows.div_ceil(threads);
    let output_chunk = rows_per_worker * n;
    if threads == 1 {
        let workspace = &mut workspaces[0];
        for (row, output_row) in output.chunks_mut(n).enumerate() {
            let start = row * bins;
            plan.inverse_series(&input[start..start + bins], output_row, workspace)
                .expect("RustSHT inverse series");
        }
    } else {
        pool.install(|| {
            output
                .par_chunks_mut(output_chunk)
                .zip(workspaces.par_iter_mut())
                .enumerate()
                .for_each(|(group, (out_rows, workspace))| {
                    let first = group * rows_per_worker;
                    for (offset, out) in out_rows.chunks_mut(n).enumerate() {
                        let row = first + offset;
                        let start = row * bins;
                        plan.inverse_series(&input[start..start + bins], out, workspace)
                            .expect("RustSHT inverse series");
                    }
                });
        });
    }
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
    let args: Vec<_> = env::args_os().collect();
    if args.len() == 5 && args[1] == "--write-fixture" {
        let n = args[2].to_string_lossy().parse().expect("FFT length");
        let rows = args[3].to_string_lossy().parse().expect("row count");
        write_fixture(n, rows, std::path::Path::new(&args[4]));
        return;
    }
    assert!(
        args.len() == 6,
        "usage: plan14-q8-odd-inverse INPUT N ROWS THREADS OUTPUT; or --write-fixture N ROWS PATH"
    );
    let input_path = std::path::Path::new(&args[1]);
    let n: usize = args[2].to_string_lossy().parse().expect("FFT length");
    let rows: usize = args[3].to_string_lossy().parse().expect("row count");
    let threads: usize = args[4].to_string_lossy().parse().expect("thread count");
    let output_path = std::path::Path::new(&args[5]);
    assert!(n >= 2 && rows > 0 && threads > 0);
    let geometry = Geometry { n, rows, threads };

    let real_plan = RealFftPlan::<f64>::new(n).expect("RustSHT FFT plan");
    let input = read_spectra(input_path, n, rows);
    let mut output = vec![0.0; rows * n];
    let mut real_workspaces: Vec<_> = (0..threads)
        .map(|_| real_plan.workspace().expect("real workspace"))
        .collect();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("thread pool");

    let mut execute = || {
        run_current(
            geometry,
            &input,
            &mut output,
            &real_plan,
            &mut real_workspaces,
            &pool,
        );
        black_box(&output);
    };
    let first_ms = elapsed_per_call(1, &mut execute);
    for _ in 0..8 {
        execute();
    }
    let calibration_ms = elapsed_per_call(12, &mut execute);
    let calls = ((TARGET_BATCH.as_secs_f64() * 1.0e3 / calibration_ms).round() as usize).max(1);
    println!(
        "environment,n={n},rows={rows},mmax={MMAX},threads={threads},path={},batch_calls={calls},batches={BATCHES},target_batch_ms={:.1},first_ms={first_ms:.6},calibration_ms={calibration_ms:.6},runtime_avx2={},runtime_fma={}",
        if n % 2 == 1 {
            "odd-complex"
        } else {
            "even-real-half"
        },
        TARGET_BATCH.as_secs_f64() * 1.0e3,
        std::is_x86_feature_detected!("avx2"),
        std::is_x86_feature_detected!("fma")
    );

    let mut samples = Vec::with_capacity(BATCHES);
    for _batch in 0..BATCHES {
        samples.push(elapsed_per_call(calls, &mut execute));
    }
    drop(execute);
    let mut ordered = samples.clone();
    let med = median(&mut ordered);
    let p10 = ordered[(BATCHES - 1) / 10];
    let p90 = ordered[(BATCHES - 1) * 9 / 10];
    println!(
        "summary,p10_ms={p10:.6},median_ms={med:.6},p90_ms={p90:.6},checksum={:.12e}",
        output.iter().sum::<f64>()
    );
    for (i, sample) in samples.iter().enumerate() {
        println!("batch,{i},ms={sample:.6}");
    }
    write_real_output(output_path, &output);
}
