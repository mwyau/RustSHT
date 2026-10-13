// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

#[path = "../benches/prototypes/c2c_across_row.rs"]
mod across_row;

use across_row::AcrossRowPlan;
use num_complex::Complex64;

fn input_value(row: usize, index: usize, _len: usize) -> Complex64 {
    let x = (index + 1 + 3 * row) as f64;
    Complex64::new((0.173 * x).sin(), (0.113 * (x + 1.0)).cos())
}

fn direct_dft(input: &[Complex64], forward: bool) -> Vec<Complex64> {
    let len = input.len();
    (0..len)
        .map(|frequency| {
            let mut sum = Complex64::new(0.0, 0.0);
            for (index, &value) in input.iter().enumerate() {
                let sign = if forward { -1.0 } else { 1.0 };
                let angle = sign * std::f64::consts::TAU * (frequency * index) as f64 / len as f64;
                sum += value * Complex64::new(angle.cos(), angle.sin());
            }
            if forward { sum } else { sum / len as f64 }
        })
        .collect()
}

fn assert_close(actual: &[Complex64], expected: &[Complex64], len: usize) {
    let tolerance = 2.0e-11 * len as f64;
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).norm() <= tolerance,
            "index={index}, actual={actual:?}, expected={expected:?}, tolerance={tolerance}"
        );
    }
}

#[test]
fn across_row_scalar_and_avx2_prototypes_match_direct_dfts() {
    for len in [1, 2, 8, 16, 64, 256, 1024] {
        let plan = AcrossRowPlan::new(len).unwrap();
        for rows in if len == 1024 {
            vec![2, 3]
        } else {
            vec![1, 3, 4]
        } {
            for inverse in [false, true] {
                let input: Vec<_> = (0..rows)
                    .flat_map(|row| (0..len).map(move |index| input_value(row, index, len)))
                    .collect();
                let expected: Vec<_> = input
                    .chunks_exact(len)
                    .flat_map(|row| direct_dft(row, !inverse))
                    .collect();

                for use_scalar in [false, true] {
                    let mut actual = input.clone();
                    let mut workspace = plan.workspace(rows).unwrap();
                    assert_eq!(workspace.rows(), rows);
                    assert_eq!(
                        workspace.scratch_bytes(),
                        rows * len * std::mem::size_of::<Complex64>()
                    );
                    if use_scalar {
                        plan.execute_scalar(&mut actual, &mut workspace, inverse)
                            .unwrap();
                    } else {
                        plan.execute(&mut actual, &mut workspace, inverse).unwrap();
                    }
                    assert_close(&actual, &expected, len);
                }
            }
        }
    }
}

#[test]
fn across_row_prototype_checks_batch_and_workspace_shapes() {
    let plan = AcrossRowPlan::new(8).unwrap();
    let mut wrong_workspace = plan.workspace(2).unwrap();
    let mut values = vec![Complex64::new(0.0, 0.0); 8];
    assert!(
        plan.execute(&mut values, &mut wrong_workspace, false)
            .is_err()
    );
    assert!(
        plan.execute(&mut values[..7], &mut wrong_workspace, false)
            .is_err()
    );
    assert!(AcrossRowPlan::new(0).is_err());
    assert!(AcrossRowPlan::new(12).is_err());
}
