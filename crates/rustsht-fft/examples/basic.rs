// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

use rustsht_fft::{Complex, ComplexFftPlan, RealFftPlan};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let real_plan = RealFftPlan::<f64>::new(8)?;
    let mut real_workspace = real_plan.workspace()?;
    let input = [0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0];
    let mut spectrum = vec![Complex::new(0.0, 0.0); real_plan.spectrum_len()];
    real_plan.forward(&input, &mut spectrum, &mut real_workspace)?;

    let mut reconstructed = vec![0.0; real_plan.len()];
    real_plan.inverse(&spectrum, &mut reconstructed, &mut real_workspace)?;
    assert!(
        input
            .iter()
            .zip(&reconstructed)
            .all(|(left, right)| (left - right).abs() < 1.0e-12)
    );

    let complex_plan = ComplexFftPlan::<f32>::new(7)?;
    let mut complex_workspace = complex_plan.workspace()?;
    let mut values = vec![Complex::new(1.0_f32, 0.0); complex_plan.len()];
    complex_plan.forward(&mut values, &mut complex_workspace)?;
    complex_plan.inverse(&mut values, &mut complex_workspace)?;

    Ok(())
}
