// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
// Test-only direct-kernel benchmark fixture. This module is excluded from
// production builds and calls the same Pulp implementation as the analysis path.

use super::*;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

const BENCH_LMAX: usize = 720;
const BENCH_ORDER: usize = 1;

fn output_base() -> String {
    std::env::var("RUSTSHT_KERNEL_BENCH_BASE").unwrap_or_else(|_| {
        std::env::temp_dir()
            .join("rustsht-analysis-kernel")
            .to_string_lossy()
            .into_owned()
    })
}

struct Fixture {
    cosine: Vec<f64>,
    cosine_squared: Vec<f64>,
    previous: Vec<f64>,
    current: Vec<f64>,
    same_real: Vec<f64>,
    same_imag: Vec<f64>,
    opposite_real: Vec<f64>,
    opposite_imag: Vec<f64>,
    coefficient_a: Vec<f64>,
    coefficient_b: Vec<f64>,
}

impl Fixture {
    fn new(lane_count: usize) -> Self {
        let m = BENCH_ORDER;
        let lmax = BENCH_LMAX;
        let mut epsilon = vec![0.0; lmax + 4];
        for (degree, value) in epsilon.iter_mut().enumerate().take(lmax + 4).skip(m + 1) {
            let numerator = ((degree + m) * (degree - m)) as f64;
            let denominator = ((2 * degree + 1) * (2 * degree - 1)) as f64;
            *value = (numerator / denominator).sqrt();
        }

        let factor_count = lmax / 2 + 2;
        let mut alpha = vec![0.0; factor_count];
        let mut coefficient_a = vec![0.0; factor_count];
        let mut coefficient_b = vec![0.0; factor_count];
        assert!(fill_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        ));

        let mut cosine = Vec::with_capacity(lane_count);
        let mut cosine_squared = Vec::with_capacity(lane_count);
        let previous = vec![0.0; lane_count];
        let mut current = Vec::with_capacity(lane_count);
        let mut same_real = Vec::with_capacity(lane_count);
        let mut same_imag = Vec::with_capacity(lane_count);
        let mut opposite_real = Vec::with_capacity(lane_count);
        let mut opposite_imag = Vec::with_capacity(lane_count);

        for lane in 0..lane_count {
            let c = -0.5 + (lane as f64 + 0.5) / lane_count as f64;
            let s = (1.0 - c * c).sqrt();
            cosine.push(c);
            cosine_squared.push(c * c);
            current.push(-((3.0 / (8.0 * std::f64::consts::PI)).sqrt()) * s);
            let x = lane as f64 + 1.0;
            same_real.push(0.1 + (0.017 * x).sin());
            same_imag.push(-0.2 + 0.4 * (0.011 * x).cos());
            opposite_real.push(c * (0.2 + (0.013 * x).sin()));
            opposite_imag.push(c * (0.3 + (0.019 * x).cos()));
        }

        let pair_count = (lmax - m + 2) / 2;
        Self {
            cosine,
            cosine_squared,
            previous,
            current,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            coefficient_a: coefficient_a[..pair_count].to_vec(),
            coefficient_b: coefficient_b[..pair_count].to_vec(),
        }
    }

    fn write_input(&self, path: &str, lane_count: usize) {
        let mut file = File::create(path).expect("create kernel fixture");
        for value in [
            BENCH_ORDER as u64,
            BENCH_LMAX as u64,
            lane_count as u64,
            self.coefficient_a.len() as u64,
        ] {
            file.write_all(&value.to_le_bytes())
                .expect("write kernel fixture header");
        }
        for values in [
            &self.cosine,
            &self.cosine_squared,
            &self.previous,
            &self.current,
            &self.same_real,
            &self.same_imag,
            &self.opposite_real,
            &self.opposite_imag,
            &self.coefficient_a,
            &self.coefficient_b,
        ] {
            for value in values {
                file.write_all(&value.to_le_bytes())
                    .expect("write kernel fixture values");
            }
        }
    }
}

fn write_complex(path: &str, values: &[Complex64]) {
    let mut file = File::create(path).expect("create Rust kernel output");
    for value in values {
        file.write_all(&value.re.to_le_bytes())
            .expect("write Rust kernel output");
        file.write_all(&value.im.to_le_bytes())
            .expect("write Rust kernel output");
    }
}

fn write_f64s(path: &str, arrays: [&[f64]; 3]) {
    let mut file = File::create(path).expect("create Rust prefix state");
    for values in arrays {
        for value in values {
            file.write_all(&value.to_le_bytes())
                .expect("write Rust prefix state");
        }
    }
}

#[test]
#[ignore = "manual direct Rust/DUCC SIMD-kernel comparison"]
fn benchmark_fused_scalar_analysis_pair_kernel() {
    let output_base = output_base();
    let arch = Arch::new();
    eprintln!("RUST_KERNEL_ARCH {arch:?}");

    for lane_count in [64, 256] {
        let fixture = Fixture::new(lane_count);
        let input_path = format!("{output_base}-{lane_count}.in");
        let output_path = format!("{output_base}-{lane_count}.rust");
        fixture.write_input(&input_path, lane_count);

        let coefficient_count = BENCH_LMAX - BENCH_ORDER + 1;
        let mut staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut previous = vec![0.0; lane_count];
        let mut current = vec![0.0; lane_count];
        let mut next = vec![0.0; lane_count];
        let mut samples_us = Vec::with_capacity(5);

        for sample in 0..6 {
            previous.copy_from_slice(&fixture.previous);
            current.copy_from_slice(&fixture.current);
            next.fill(0.0);
            staged.fill(Complex64::new(0.0, 0.0));

            let started = Instant::now();
            let succeeded = arch.dispatch(FusedScalarAnalysisPair::<true> {
                m: BENCH_ORDER,
                lmax: BENCH_LMAX,
                pair_start: 0,
                cos_theta: &fixture.cosine,
                cos_theta_squared: &fixture.cosine_squared,
                coefficient_a: &fixture.coefficient_a,
                coefficient_b: &fixture.coefficient_b,
                same_real: &fixture.same_real,
                same_imag: &fixture.same_imag,
                opposite_real: &fixture.opposite_real,
                opposite_imag: &fixture.opposite_imag,
                staged_coefficients: &mut staged,
                previous: &mut previous,
                current: &mut current,
                next: &mut next,
            });
            let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
            assert!(
                succeeded,
                "ordinary SIMD route declined for {lane_count} lanes"
            );
            if sample != 0 {
                samples_us.push(elapsed_us);
            }
        }

        write_complex(&output_path, &staged);
        samples_us.sort_by(f64::total_cmp);
        eprintln!(
            "RUST_KERNEL m={} lmax={} lanes={} repetitions=5 median_us={:.3} samples_us={:?}",
            BENCH_ORDER, BENCH_LMAX, lane_count, samples_us[2], samples_us
        );
    }
}

#[test]
#[ignore = "manual Rust scaled-prefix comparison against the pinned DUCC route"]
fn benchmark_fused_scaled_scalar_analysis_prefix() {
    let arch = Arch::new();
    let output_base = output_base();
    eprintln!("RUST_PREFIX_ARCH {arch:?}");
    for lane_count in [64, 256] {
        let fixture = Fixture::new(lane_count);
        let input_path = format!("{output_base}-{lane_count}.in");
        let state_path = format!("{output_base}-{lane_count}.rust_state");
        fixture.write_input(&input_path, lane_count);
        let mut previous = vec![0.0; lane_count];
        let mut current = vec![2.0_f64.powi(-100); lane_count];
        let mut next = vec![0.0; lane_count];
        let mut scale_epochs = vec![-1.0; lane_count];
        let mut scale_corrections = vec![0.0; lane_count];
        let mut staged = vec![Complex64::new(17.0, -9.0); BENCH_LMAX - BENCH_ORDER + 1];
        let mut samples_us = Vec::with_capacity(5);

        for sample in 0..6 {
            previous.fill(0.0);
            current.fill(2.0_f64.powi(-100));
            next.fill(0.0);
            scale_epochs.fill(-1.0);
            scale_corrections.fill(0.0);
            staged.fill(Complex64::new(17.0, -9.0));

            let started = Instant::now();
            let succeeded = arch
                .dispatch(FusedScaledScalarAnalysisPair::<true> {
                    m: BENCH_ORDER,
                    lmax: BENCH_LMAX,
                    cos_theta: &fixture.cosine,
                    cos_theta_squared: &fixture.cosine_squared,
                    coefficient_a: &fixture.coefficient_a,
                    coefficient_b: &fixture.coefficient_b,
                    same_real: &fixture.same_real,
                    same_imag: &fixture.same_imag,
                    opposite_real: &fixture.opposite_real,
                    opposite_imag: &fixture.opposite_imag,
                    staged_coefficients: &mut staged,
                    previous: &mut previous,
                    current: &mut current,
                    next: &mut next,
                    scale_epochs: &mut scale_epochs,
                    scale_corrections: &mut scale_corrections,
                })
                .expect("scaled prefix route");
            let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
            assert!(
                succeeded,
                "scaled prefix route declined for {lane_count} lanes"
            );
            if sample != 0 {
                samples_us.push(elapsed_us);
            }
        }
        assert!(
            staged
                .iter()
                .all(|coefficient| *coefficient == Complex64::new(0.0, 0.0))
        );
        write_f64s(&state_path, [&previous, &current, &scale_epochs]);
        samples_us.sort_by(f64::total_cmp);
        eprintln!(
            "RUST_PREFIX m={} lmax={} lanes={} repetitions=5 median_us={:.3} samples_us={:?}",
            BENCH_ORDER, BENCH_LMAX, lane_count, samples_us[2], samples_us
        );
    }
}
