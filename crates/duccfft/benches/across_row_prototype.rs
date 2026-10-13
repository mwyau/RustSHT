// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

#[path = "prototypes/c2c_across_row.rs"]
mod across_row;

use std::hint::black_box;
use std::time::Instant;

use across_row::AcrossRowPlan;
use duccfft::ComplexFftPlan;
use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};

const LENGTHS: &[usize] = &[256, 1024];
const ROW_COUNTS: &[usize] = &[2, 3, 4, 8, 16, 32];
const PAIRS: usize = 7;

fn input_value(row: usize, index: usize, _len: usize) -> Complex64 {
    let x = (index + 1 + 3 * row) as f64;
    Complex64::new((0.173 * x).sin(), (0.113 * (x + 1.0)).cos())
}

fn source(len: usize, rows: usize) -> Vec<Complex64> {
    (0..rows)
        .flat_map(|row| (0..len).map(move |index| input_value(row, index, len)))
        .collect()
}

fn checksum(values: &[Complex64]) -> f64 {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            (index as f64 + 1.0).mul_add(value.re, (index as f64 + 0.5) * value.im)
        })
        .sum()
}

fn assert_close(actual: &[Complex64], expected: &[Complex64], len: usize) {
    let max_error = actual
        .iter()
        .zip(expected)
        .map(|(&actual, &expected)| (actual - expected).norm())
        .fold(0.0, f64::max);
    let tolerance = 1.0e-9 * len as f64;
    assert!(
        max_error <= tolerance,
        "max_error={max_error}, tolerance={tolerance}"
    );
}

fn verify(
    len: usize,
    rows: usize,
    inverse: bool,
    native: &ComplexFftPlan<f64>,
    across: &AcrossRowPlan,
    rustfft: &dyn Fft<f64>,
) {
    let input = source(len, rows);
    let mut reference = input.clone();
    let mut native_workspace = native.workspace().expect("DUCCFFT workspace");
    if inverse {
        native
            .inverse_batch(&mut reference, &mut native_workspace)
            .expect("DUCCFFT inverse batch");
    } else {
        native
            .forward_batch(&mut reference, &mut native_workspace)
            .expect("DUCCFFT forward batch");
    }

    let mut actual_across = input.clone();
    let mut across_workspace = across.workspace(rows).expect("across-row workspace");
    across
        .execute(&mut actual_across, &mut across_workspace, inverse)
        .expect("across-row execution");
    assert_close(&actual_across, &reference, len);

    let mut actual_rustfft = input;
    let mut scratch = vec![Complex64::new(0.0, 0.0); rustfft.get_inplace_scratch_len()];
    for row in actual_rustfft.chunks_exact_mut(len) {
        rustfft.process_with_scratch(row, &mut scratch);
        if inverse {
            let scale = 1.0 / len as f64;
            for value in row {
                *value *= scale;
            }
        }
    }
    assert_close(&actual_rustfft, &reference, len);
}

fn measure<F>(
    source: &[Complex64],
    work: &mut [Complex64],
    rows: usize,
    operation: &mut F,
) -> (f64, f64)
where
    F: FnMut(&mut [Complex64]),
{
    work.copy_from_slice(source);
    let start = Instant::now();
    operation(work);
    black_box(&*work);
    let ns_per_transform = start.elapsed().as_nanos() as f64 / rows as f64;
    (ns_per_transform, black_box(checksum(work)))
}

fn main() {
    let process_sample = std::env::var("DUCCFFT_PROCESS_SAMPLE").unwrap_or_else(|_| "local".into());
    println!(
        "process_sample,n,rows,direction,pair,order,implementation,operations,ns_per_transform,checksum,scratch_bytes"
    );

    for &len in LENGTHS {
        let native = ComplexFftPlan::<f64>::new(len).expect("DUCCFFT plan");
        let across = AcrossRowPlan::new(len).expect("across-row plan");
        let mut native_workspace = native.workspace().expect("DUCCFFT workspace");
        let directions = [("forward", false), ("inverse", true)];
        let mut planner = FftPlanner::<f64>::new();
        let rustfft_forward = planner.plan_fft_forward(len);
        let rustfft_inverse = planner.plan_fft_inverse(len);

        for &rows in ROW_COUNTS {
            let mut across_workspace = across.workspace(rows).expect("across-row workspace");
            assert_eq!(across_workspace.rows(), rows);
            let source = source(len, rows);
            let mut native_values = source.clone();
            let mut across_values = source.clone();
            let mut rustfft_values = source.clone();
            let mut rustfft_scratch = vec![
                Complex64::new(0.0, 0.0);
                rustfft_forward
                    .get_inplace_scratch_len()
                    .max(rustfft_inverse.get_inplace_scratch_len())
            ];

            for &(direction, inverse) in &directions {
                let rustfft_plan = if inverse {
                    &*rustfft_inverse
                } else {
                    &*rustfft_forward
                };
                verify(len, rows, inverse, &native, &across, rustfft_plan);

                for pair in 0..PAIRS {
                    let order: [usize; 4] = std::array::from_fn(|offset| (pair + offset) % 4);
                    for &implementation in &order {
                        let (name, timing, checksum_value, scratch_bytes) = match implementation {
                            0 => {
                                let mut operation = |values: &mut [Complex64]| {
                                    if inverse {
                                        native
                                            .inverse_batch(values, &mut native_workspace)
                                            .expect("DUCCFFT inverse batch");
                                    } else {
                                        native
                                            .forward_batch(values, &mut native_workspace)
                                            .expect("DUCCFFT forward batch");
                                    }
                                };
                                let (timing, checksum_value) =
                                    measure(&source, &mut native_values, rows, &mut operation);
                                (
                                    "duccfft_batch",
                                    timing,
                                    checksum_value,
                                    native_workspace.scratch_bytes().unwrap_or(0),
                                )
                            }
                            1 => {
                                let mut operation = |values: &mut [Complex64]| {
                                    for row in values.chunks_exact_mut(len) {
                                        rustfft_plan
                                            .process_with_scratch(row, &mut rustfft_scratch);
                                        if inverse {
                                            let scale = 1.0 / len as f64;
                                            for value in row {
                                                *value *= scale;
                                            }
                                        }
                                    }
                                };
                                let (timing, checksum_value) =
                                    measure(&source, &mut rustfft_values, rows, &mut operation);
                                (
                                    "rustfft_serial",
                                    timing,
                                    checksum_value,
                                    rustfft_scratch.len() * std::mem::size_of::<Complex64>(),
                                )
                            }
                            2 => {
                                let mut operation = |values: &mut [Complex64]| {
                                    across
                                        .execute(values, &mut across_workspace, inverse)
                                        .expect("across-row execution");
                                };
                                let (timing, checksum_value) =
                                    measure(&source, &mut across_values, rows, &mut operation);
                                (
                                    "across_row_runtime",
                                    timing,
                                    checksum_value,
                                    across_workspace.scratch_bytes(),
                                )
                            }
                            _ => {
                                let mut operation = |values: &mut [Complex64]| {
                                    across
                                        .execute_scalar(values, &mut across_workspace, inverse)
                                        .expect("scalar across-row execution");
                                };
                                let (timing, checksum_value) =
                                    measure(&source, &mut across_values, rows, &mut operation);
                                (
                                    "across_row_scalar",
                                    timing,
                                    checksum_value,
                                    across_workspace.scratch_bytes(),
                                )
                            }
                        };
                        println!(
                            "{process_sample},{len},{rows},{direction},{pair},{implementation},{name},1,{timing:.3},{checksum_value:.17e},{scratch_bytes}"
                        );
                    }
                }
            }
        }
    }
}
