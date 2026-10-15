// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

use num_complex::Complex;
use rustsht_fft::ComplexFftPlan;

fn main() {
    const N: usize = 241;
    let tiny = f64::from_bits(1);
    let mut spectrum = vec![Complex::new(0.0, 0.0); N];
    spectrum[0].re = 26.0 * tiny;
    spectrum[2].re = 26.0 * tiny;
    spectrum[1].re = f64::MAX / 4.01;
    spectrum[N - 1].re = -spectrum[1].re;

    let expected_x0 = (52.0 * tiny) / N as f64;
    let plan = ComplexFftPlan::<f64>::new(N).expect("plan construction");
    let mut workspace = plan.workspace().expect("workspace construction");
    plan.inverse(&mut spectrum, &mut workspace)
        .expect("inverse transform");

    println!("n={N}");
    println!("tiny_bits={:016x}", tiny.to_bits());
    println!("large_bits={:016x}", (f64::MAX / 4.01).to_bits());
    println!("expected_x0={expected_x0:.17e}");
    println!("expected_x0_bits={:016x}", expected_x0.to_bits());
    println!("observed_x0_re={:.17e}", spectrum[0].re);
    println!("observed_x0_im={:.17e}", spectrum[0].im);
    println!("observed_x0_re_bits={:016x}", spectrum[0].re.to_bits());
    println!("observed_x0_im_bits={:016x}", spectrum[0].im.to_bits());
}
