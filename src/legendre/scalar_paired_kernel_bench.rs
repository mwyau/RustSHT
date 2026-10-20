// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
// Test-only direct-kernel benchmark fixture. This module is excluded from
// production builds and calls the same Pulp implementation as the analysis path.

use super::*;
use std::fs::File;
use std::io::{Read, Write};
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

fn read_u64(file: &mut File) -> u64 {
    let mut bytes = [0_u8; 8];
    file.read_exact(&mut bytes)
        .expect("read kernel fixture header");
    u64::from_le_bytes(bytes)
}

fn read_f64s(file: &mut File, count: usize) -> Vec<f64> {
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let mut bytes = [0_u8; 8];
        file.read_exact(&mut bytes)
            .expect("read kernel fixture value");
        values.push(f64::from_le_bytes(bytes));
    }
    values
}

#[expect(
    clippy::too_many_arguments,
    reason = "The test-only direct-kernel helper forwards fixture and worker-owned state explicitly."
)]
fn time_fused_scalar_analysis_pair<
    const CHECK_PAIR_GROUP_RANGE: bool,
    const FUSE_PAIR_GROUP_RANGE_CHECK: bool,
>(
    arch: Arch,
    m: usize,
    lmax: usize,
    fixture: &Fixture,
    staged: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> f64 {
    previous.copy_from_slice(&fixture.previous);
    current.copy_from_slice(&fixture.current);
    next.fill(0.0);
    staged.fill(Complex64::new(0.0, 0.0));

    let started = Instant::now();
    let succeeded = arch.dispatch(FusedScalarAnalysisPair::<
        true,
        CHECK_PAIR_GROUP_RANGE,
        FUSE_PAIR_GROUP_RANGE_CHECK,
    > {
        m,
        lmax,
        pair_start: 0,
        cos_theta: &fixture.cosine,
        cos_theta_squared: &fixture.cosine_squared,
        coefficient_a: &fixture.coefficient_a,
        coefficient_b: &fixture.coefficient_b,
        same_real: &fixture.same_real,
        same_imag: &fixture.same_imag,
        opposite_real: &fixture.opposite_real,
        opposite_imag: &fixture.opposite_imag,
        staged_coefficients: staged,
        previous,
        current,
        next,
    });
    let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
    assert!(
        succeeded,
        "ordinary SIMD route declined for m={m} lmax={lmax}"
    );
    elapsed_us
}

struct AnalysisRangeCheckLoop<'a> {
    previous: &'a [f64],
    current: &'a [f64],
    repetitions: usize,
}

impl WithSimd for AnalysisRangeCheckLoop<'_> {
    type Output = bool;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        (0..self.repetitions)
            .all(|_| super::analysis_pair_group_state_in_range(simd, self.previous, self.current))
    }
}

#[test]
#[ignore = "manual direct Rust/DUCC SIMD-kernel comparison"]
fn benchmark_fused_scalar_analysis_pair_kernel() {
    let output_base = output_base();
    let arch = Arch::new();
    eprintln!("RUST_KERNEL_ARCH {arch:?}");
    let lane_count = std::env::var("RUSTSHT_KERNEL_BENCH_LANES")
        .expect("RUSTSHT_KERNEL_BENCH_LANES must be set")
        .parse::<usize>()
        .expect("valid lane count");
    assert!([64, 128, 256].contains(&lane_count));
    let fixture = Fixture::new(lane_count);
    let input_path = format!("{output_base}-{lane_count}.in");
    let checked_output_path = format!("{output_base}-{lane_count}.rust");
    let unchecked_output_path = format!("{output_base}-{lane_count}.rust.unchecked");
    fixture.write_input(&input_path, lane_count);

    let coefficient_count = BENCH_LMAX - BENCH_ORDER + 1;
    let mut checked_staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut unchecked_staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut previous = vec![0.0; lane_count];
    let mut current = vec![0.0; lane_count];
    let mut next = vec![0.0; lane_count];

    match std::env::var("RUSTSHT_KERNEL_BENCH_MODE").as_deref() {
        Ok("prepare") => {
            reset_fused_scalar_analysis_pair(
                &fixture,
                &mut checked_staged,
                &mut previous,
                &mut current,
            );
            assert!(run_fused_scalar_analysis_pair::<true>(
                arch,
                &fixture,
                &mut checked_staged,
                &mut previous,
                &mut current,
                &mut next,
            ));
            reset_fused_scalar_analysis_pair(
                &fixture,
                &mut unchecked_staged,
                &mut previous,
                &mut current,
            );
            assert!(run_fused_scalar_analysis_pair::<false>(
                arch,
                &fixture,
                &mut unchecked_staged,
                &mut previous,
                &mut current,
                &mut next,
            ));
            assert_eq!(checked_staged, unchecked_staged);
            write_complex(&checked_output_path, &checked_staged);
            write_complex(&unchecked_output_path, &unchecked_staged);
            eprintln!("RUST_PREPARED lanes={lane_count} output_equal=true");
        }
        Ok("calibrate") => {
            let calibration_repetitions =
                std::env::var("RUSTSHT_KERNEL_BENCH_CALIBRATION_REPETITIONS")
                    .expect("RUSTSHT_KERNEL_BENCH_CALIBRATION_REPETITIONS must be set")
                    .parse::<usize>()
                    .expect("valid calibration repetition count");
            let checked_us = calibrate_fused_scalar_analysis_pair::<true>(
                calibration_repetitions,
                arch,
                &fixture,
                &mut checked_staged,
                &mut previous,
                &mut current,
                &mut next,
            );
            let unchecked_us = calibrate_fused_scalar_analysis_pair::<false>(
                calibration_repetitions,
                arch,
                &fixture,
                &mut unchecked_staged,
                &mut previous,
                &mut current,
                &mut next,
            );
            assert_eq!(checked_staged, unchecked_staged);
            eprintln!(
                "RUST_CALIBRATION lanes={lane_count} repetitions={calibration_repetitions} checked_us={checked_us:.6} unchecked_us={unchecked_us:.6} output_equal=true"
            );
        }
        Ok("run") => {
            let repetitions = std::env::var("RUSTSHT_KERNEL_BENCH_REPETITIONS")
                .expect("RUSTSHT_KERNEL_BENCH_REPETITIONS must be set")
                .parse::<usize>()
                .expect("valid repetition count");
            let batch_count = std::env::var("RUSTSHT_KERNEL_BENCH_BATCHES")
                .expect("RUSTSHT_KERNEL_BENCH_BATCHES must be set")
                .parse::<usize>()
                .expect("valid batch count");
            assert!(batch_count >= 30);
            assert!(repetitions > 0);

            // Warm both monomorphizations before collecting calibrated batches.
            for _ in 0..3 {
                reset_fused_scalar_analysis_pair(
                    &fixture,
                    &mut checked_staged,
                    &mut previous,
                    &mut current,
                );
                assert!(run_fused_scalar_analysis_pair::<true>(
                    arch,
                    &fixture,
                    &mut checked_staged,
                    &mut previous,
                    &mut current,
                    &mut next,
                ));
                reset_fused_scalar_analysis_pair(
                    &fixture,
                    &mut unchecked_staged,
                    &mut previous,
                    &mut current,
                );
                assert!(run_fused_scalar_analysis_pair::<false>(
                    arch,
                    &fixture,
                    &mut unchecked_staged,
                    &mut previous,
                    &mut current,
                    &mut next,
                ));
            }
            assert_eq!(checked_staged, unchecked_staged);

            for batch in 0..batch_count {
                let checked_first = batch.is_multiple_of(2);
                let (checked_kernel, unchecked_kernel) = if checked_first {
                    let checked = kernel_only_batch::<true>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut checked_staged,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    let unchecked = kernel_only_batch::<false>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut unchecked_staged,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    (checked, unchecked)
                } else {
                    let unchecked = kernel_only_batch::<false>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut unchecked_staged,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    let checked = kernel_only_batch::<true>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut checked_staged,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    (checked, unchecked)
                };
                assert_eq!(checked_staged, unchecked_staged);
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} mode=kernel_only variant=checked kernel_us={:.3} wall_us={:.3}",
                    checked_kernel.0, checked_kernel.1
                );
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} mode=kernel_only variant=unchecked kernel_us={:.3} wall_us={:.3}",
                    unchecked_kernel.0, unchecked_kernel.1
                );

                let (checked_reset, unchecked_reset) = if checked_first {
                    let checked = reset_inclusive_batch::<true>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut checked_staged,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    let unchecked = reset_inclusive_batch::<false>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut unchecked_staged,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    (checked, unchecked)
                } else {
                    let unchecked = reset_inclusive_batch::<false>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut unchecked_staged,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    let checked = reset_inclusive_batch::<true>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut checked_staged,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    (checked, unchecked)
                };
                assert_eq!(checked_staged, unchecked_staged);
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} mode=reset_inclusive variant=checked batch_us={checked_reset:.3}"
                );
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} mode=reset_inclusive variant=unchecked batch_us={unchecked_reset:.3}"
                );
            }

            write_complex(&checked_output_path, &checked_staged);
            write_complex(&unchecked_output_path, &unchecked_staged);
            eprintln!(
                "RUST_BATCHES_COMPLETE lanes={lane_count} batches={batch_count} repetitions={repetitions} output_equal=true"
            );
        }
        other => panic!("unknown RUSTSHT_KERNEL_BENCH_MODE: {other:?}"),
    }
}

fn reset_fused_scalar_analysis_pair(
    fixture: &Fixture,
    staged: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
) {
    previous.copy_from_slice(&fixture.previous);
    current.copy_from_slice(&fixture.current);
    staged.fill(Complex64::new(0.0, 0.0));
}

#[allow(unexpected_cfgs)]
#[inline(always)]
fn run_fused_scalar_analysis_pair<const CHECK_PAIR_GROUP_RANGE: bool>(
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> bool {
    run_fused_scalar_analysis_pair_variant::<
        CHECK_PAIR_GROUP_RANGE,
        { cfg!(rustsht_fused_analysis_pair_range_check) },
    >(arch, fixture, staged, previous, current, next)
}

#[inline(never)]
fn run_fused_scalar_analysis_pair_variant<
    const CHECK_PAIR_GROUP_RANGE: bool,
    const FUSE_PAIR_GROUP_RANGE_CHECK: bool,
>(
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> bool {
    arch.dispatch(FusedScalarAnalysisPair::<
        true,
        CHECK_PAIR_GROUP_RANGE,
        FUSE_PAIR_GROUP_RANGE_CHECK,
    > {
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
        staged_coefficients: std::hint::black_box(staged),
        previous: std::hint::black_box(previous),
        current: std::hint::black_box(current),
        next: std::hint::black_box(next),
    })
}

fn calibrate_fused_scalar_analysis_pair<const CHECK_PAIR_GROUP_RANGE: bool>(
    repetitions: usize,
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> f64 {
    for _ in 0..4 {
        reset_fused_scalar_analysis_pair(fixture, staged, previous, current);
        assert!(run_fused_scalar_analysis_pair::<CHECK_PAIR_GROUP_RANGE>(
            arch, fixture, staged, previous, current, next,
        ));
    }
    let started = Instant::now();
    let mut all_succeeded = true;
    for _ in 0..repetitions {
        reset_fused_scalar_analysis_pair(fixture, staged, previous, current);
        all_succeeded &= run_fused_scalar_analysis_pair::<CHECK_PAIR_GROUP_RANGE>(
            arch, fixture, staged, previous, current, next,
        );
        std::hint::black_box((&*staged, &*previous, &*current, &*next));
    }
    let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6 / repetitions as f64;
    assert!(all_succeeded);
    elapsed_us
}

fn kernel_only_batch<const CHECK_PAIR_GROUP_RANGE: bool>(
    repetitions: usize,
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> (f64, f64) {
    let wall_started = Instant::now();
    let mut kernel_duration = std::time::Duration::ZERO;
    let mut all_succeeded = true;
    for _ in 0..repetitions {
        reset_fused_scalar_analysis_pair(fixture, staged, previous, current);
        let started = Instant::now();
        let succeeded = run_fused_scalar_analysis_pair::<CHECK_PAIR_GROUP_RANGE>(
            arch, fixture, staged, previous, current, next,
        );
        kernel_duration += started.elapsed();
        all_succeeded &= succeeded;
        std::hint::black_box((&*staged, &*previous, &*current, &*next));
    }
    assert!(all_succeeded);
    (
        kernel_duration.as_secs_f64() * 1.0e6,
        wall_started.elapsed().as_secs_f64() * 1.0e6,
    )
}

fn reset_inclusive_batch<const CHECK_PAIR_GROUP_RANGE: bool>(
    repetitions: usize,
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> f64 {
    let started = Instant::now();
    let mut all_succeeded = true;
    for _ in 0..repetitions {
        reset_fused_scalar_analysis_pair(fixture, staged, previous, current);
        all_succeeded &= run_fused_scalar_analysis_pair::<CHECK_PAIR_GROUP_RANGE>(
            arch, fixture, staged, previous, current, next,
        );
        std::hint::black_box((&*staged, &*previous, &*current, &*next));
    }
    let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
    assert!(all_succeeded);
    elapsed_us
}

#[test]
fn unchecked_group_range_matches_checked_for_zero_cancellation_and_tails() {
    let arch = Arch::new();
    for lane_count in [1, 3, 5, 7, 9, 17] {
        let mut fixture = Fixture::new(lane_count);
        let coefficient_count = BENCH_LMAX - BENCH_ORDER + 1;
        let mut scanned_staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut fused_staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut unchecked_staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
        let mut previous = vec![0.0; lane_count];
        let mut current = vec![0.0; lane_count];
        let mut next = vec![0.0; lane_count];

        for values in [[0.0; 4], [1.0, -1.0, 1.0, -1.0]] {
            for lane in 0..lane_count {
                let value = values[lane % values.len()];
                fixture.same_real[lane] = value;
                fixture.same_imag[lane] = -value;
                fixture.opposite_real[lane] = value;
                fixture.opposite_imag[lane] = value * 0.5;
            }
            let _ = time_fused_scalar_analysis_pair::<true, false>(
                arch,
                BENCH_ORDER,
                BENCH_LMAX,
                &fixture,
                &mut scanned_staged,
                &mut previous,
                &mut current,
                &mut next,
            );
            let _ = time_fused_scalar_analysis_pair::<true, true>(
                arch,
                BENCH_ORDER,
                BENCH_LMAX,
                &fixture,
                &mut fused_staged,
                &mut previous,
                &mut current,
                &mut next,
            );
            let _ = time_fused_scalar_analysis_pair::<false, false>(
                arch,
                BENCH_ORDER,
                BENCH_LMAX,
                &fixture,
                &mut unchecked_staged,
                &mut previous,
                &mut current,
                &mut next,
            );
            assert_eq!(scanned_staged, fused_staged);
            assert_eq!(scanned_staged, unchecked_staged);
        }

        // A subnormal recurrence state must take the same fallback decision
        // whether the final group state is checked in place or reread after
        // the recurrence pass.
        fixture.previous.fill(0.0);
        fixture.current.fill(f64::from_bits(1));
        previous.copy_from_slice(&fixture.previous);
        current.copy_from_slice(&fixture.current);
        next.fill(0.0);
        scanned_staged.fill(Complex64::new(0.0, 0.0));
        fused_staged.fill(Complex64::new(0.0, 0.0));
        let scanned_succeeded = run_fused_scalar_analysis_pair_variant::<true, false>(
            arch,
            &fixture,
            &mut scanned_staged,
            &mut previous,
            &mut current,
            &mut next,
        );
        previous.copy_from_slice(&fixture.previous);
        current.copy_from_slice(&fixture.current);
        next.fill(0.0);
        let fused_succeeded = run_fused_scalar_analysis_pair_variant::<true, true>(
            arch,
            &fixture,
            &mut fused_staged,
            &mut previous,
            &mut current,
            &mut next,
        );
        assert_eq!(scanned_succeeded, fused_succeeded);
        assert!(
            !fused_succeeded,
            "subnormal state did not decline fast route"
        );
        assert_eq!(scanned_staged, fused_staged);
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

#[test]
#[ignore = "manual production scaled-block comparison against pinned DUCC"]
fn benchmark_production_scaled_scalar_analysis_block() {
    let input_path = std::env::var("RUSTSHT_SCALED_KERNEL_INPUT")
        .expect("RUSTSHT_SCALED_KERNEL_INPUT must name the captured block");
    let output_path = input_path
        .strip_suffix(".in")
        .expect("scaled kernel input ends in .in")
        .to_owned()
        + ".rust";
    let mut file = File::open(&input_path).expect("open production scaled kernel fixture");
    let m = read_u64(&mut file) as usize;
    let lmax = read_u64(&mut file) as usize;
    let lane_count = read_u64(&mut file) as usize;
    let pair_count = read_u64(&mut file) as usize;
    let fixture = Fixture {
        cosine: read_f64s(&mut file, lane_count),
        cosine_squared: read_f64s(&mut file, lane_count),
        previous: read_f64s(&mut file, lane_count),
        current: read_f64s(&mut file, lane_count),
        same_real: read_f64s(&mut file, lane_count),
        same_imag: read_f64s(&mut file, lane_count),
        opposite_real: read_f64s(&mut file, lane_count),
        opposite_imag: read_f64s(&mut file, lane_count),
        coefficient_a: read_f64s(&mut file, pair_count),
        coefficient_b: read_f64s(&mut file, pair_count),
    };
    let initial_scale_epochs = read_f64s(&mut file, lane_count);
    assert_eq!(pair_count, (lmax - m + 2) / 2);

    let arch = Arch::new();
    eprintln!("RUST_SCALED_KERNEL_ARCH {arch:?}");
    let mut previous = fixture.previous.clone();
    let mut current = fixture.current.clone();
    let mut next = vec![0.0; lane_count];
    let mut scale_epochs = initial_scale_epochs.clone();
    let mut scale_corrections = vec![0.0; lane_count];
    let mut staged = vec![Complex64::new(0.0, 0.0); lmax - m + 1];
    let mut samples_us = Vec::with_capacity(5);
    for sample in 0..6 {
        previous.copy_from_slice(&fixture.previous);
        current.copy_from_slice(&fixture.current);
        next.fill(0.0);
        scale_epochs.copy_from_slice(&initial_scale_epochs);
        scale_corrections.fill(0.0);
        staged.fill(Complex64::new(0.0, 0.0));
        let started = Instant::now();
        let succeeded = arch
            .dispatch(FusedScaledScalarAnalysisPair::<true> {
                m,
                lmax,
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
            .expect("production scaled kernel route");
        let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
        assert!(succeeded, "production scaled kernel declined");
        if sample != 0 {
            samples_us.push(elapsed_us);
        }
    }
    write_complex(&output_path, &staged);
    samples_us.sort_by(f64::total_cmp);
    eprintln!(
        "RUST_SCALED_KERNEL m={m} lmax={lmax} lanes={lane_count} repetitions=5 median_us={:.3} samples_us={samples_us:?}",
        samples_us[2]
    );

    let mut workers: Vec<_> = (0..8)
        .map(|_| {
            (
                fixture.previous.clone(),
                fixture.current.clone(),
                vec![0.0; lane_count],
                initial_scale_epochs.clone(),
                vec![0.0; lane_count],
                vec![Complex64::new(0.0, 0.0); lmax - m + 1],
            )
        })
        .collect();
    let mut q8_wall_us = Vec::with_capacity(5);
    for sample in 0..6 {
        let started = Instant::now();
        std::thread::scope(|scope| {
            for (previous, current, next, epochs, corrections, staged) in &mut workers {
                let fixture = &fixture;
                let initial_scale_epochs = &initial_scale_epochs;
                scope.spawn(move || {
                    for _ in 0..128 {
                        previous.copy_from_slice(&fixture.previous);
                        current.copy_from_slice(&fixture.current);
                        next.fill(0.0);
                        epochs.copy_from_slice(initial_scale_epochs);
                        corrections.fill(0.0);
                        staged.fill(Complex64::new(0.0, 0.0));
                        assert!(
                            arch.dispatch(FusedScaledScalarAnalysisPair::<true> {
                                m,
                                lmax,
                                cos_theta: &fixture.cosine,
                                cos_theta_squared: &fixture.cosine_squared,
                                coefficient_a: &fixture.coefficient_a,
                                coefficient_b: &fixture.coefficient_b,
                                same_real: &fixture.same_real,
                                same_imag: &fixture.same_imag,
                                opposite_real: &fixture.opposite_real,
                                opposite_imag: &fixture.opposite_imag,
                                staged_coefficients: staged,
                                previous,
                                current,
                                next,
                                scale_epochs: epochs,
                                scale_corrections: corrections,
                            })
                            .expect("q8 scaled kernel route")
                        );
                    }
                });
            }
        });
        let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
        if sample != 0 {
            q8_wall_us.push(elapsed_us);
        }
    }
    q8_wall_us.sort_by(f64::total_cmp);
    eprintln!(
        "RUST_SCALED_KERNEL_Q8 m={m} lmax={lmax} lanes={lane_count} calls_per_worker=128 wall_median_us={:.3} per_kernel_us={:.3} samples_wall_us={q8_wall_us:?}",
        q8_wall_us[2],
        q8_wall_us[2] / (8.0 * 128.0)
    );
}

#[test]
#[ignore = "manual production ordinary-block comparison against pinned DUCC"]
fn benchmark_production_ordinary_scalar_analysis_block() {
    let input_path = std::env::var("RUSTSHT_ORDINARY_KERNEL_INPUT")
        .expect("RUSTSHT_ORDINARY_KERNEL_INPUT must name the captured block");
    let output_path = input_path
        .strip_suffix(".in")
        .expect("ordinary kernel input ends in .in")
        .to_owned()
        + ".rust";
    let mut file = File::open(&input_path).expect("open production ordinary kernel fixture");
    let m = read_u64(&mut file) as usize;
    let lmax = read_u64(&mut file) as usize;
    let lane_count = read_u64(&mut file) as usize;
    let pair_count = read_u64(&mut file) as usize;
    let fixture = Fixture {
        cosine: read_f64s(&mut file, lane_count),
        cosine_squared: read_f64s(&mut file, lane_count),
        previous: read_f64s(&mut file, lane_count),
        current: read_f64s(&mut file, lane_count),
        same_real: read_f64s(&mut file, lane_count),
        same_imag: read_f64s(&mut file, lane_count),
        opposite_real: read_f64s(&mut file, lane_count),
        opposite_imag: read_f64s(&mut file, lane_count),
        coefficient_a: read_f64s(&mut file, pair_count),
        coefficient_b: read_f64s(&mut file, pair_count),
    };
    assert_eq!(pair_count, (lmax - m + 2) / 2);

    let arch = Arch::new();
    eprintln!("RUST_ORDINARY_KERNEL_ARCH {arch:?}");
    let mut previous = fixture.previous.clone();
    let mut current = fixture.current.clone();
    let mut next = vec![0.0; lane_count];
    let mut staged = vec![Complex64::new(0.0, 0.0); lmax - m + 1];
    let mut samples_us = Vec::with_capacity(5);
    for sample in 0..6 {
        previous.copy_from_slice(&fixture.previous);
        current.copy_from_slice(&fixture.current);
        next.fill(0.0);
        staged.fill(Complex64::new(0.0, 0.0));
        let started = Instant::now();
        let succeeded = arch.dispatch(FusedScalarAnalysisPair::<true> {
            m,
            lmax,
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
        assert!(succeeded, "production ordinary SIMD route declined");
        if sample != 0 {
            samples_us.push(elapsed_us);
        }
    }
    write_complex(&output_path, &staged);
    samples_us.sort_by(f64::total_cmp);
    eprintln!(
        "RUST_PRODUCTION_ORDINARY m={m} lmax={lmax} lanes={lane_count} repetitions=5 median_us={:.3} samples_us={samples_us:?}",
        samples_us[2]
    );

    let mut workers: Vec<_> = (0..8)
        .map(|_| {
            (
                fixture.previous.clone(),
                fixture.current.clone(),
                vec![0.0; lane_count],
                vec![Complex64::new(0.0, 0.0); lmax - m + 1],
            )
        })
        .collect();
    let mut q8_wall_us = Vec::with_capacity(5);
    for sample in 0..6 {
        let started = Instant::now();
        std::thread::scope(|scope| {
            for (previous, current, next, staged) in &mut workers {
                let fixture = &fixture;
                scope.spawn(move || {
                    for _ in 0..128 {
                        previous.copy_from_slice(&fixture.previous);
                        current.copy_from_slice(&fixture.current);
                        next.fill(0.0);
                        staged.fill(Complex64::new(0.0, 0.0));
                        assert!(arch.dispatch(FusedScalarAnalysisPair::<true> {
                            m,
                            lmax,
                            pair_start: 0,
                            cos_theta: &fixture.cosine,
                            cos_theta_squared: &fixture.cosine_squared,
                            coefficient_a: &fixture.coefficient_a,
                            coefficient_b: &fixture.coefficient_b,
                            same_real: &fixture.same_real,
                            same_imag: &fixture.same_imag,
                            opposite_real: &fixture.opposite_real,
                            opposite_imag: &fixture.opposite_imag,
                            staged_coefficients: staged,
                            previous,
                            current,
                            next,
                        }));
                    }
                });
            }
        });
        let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
        if sample != 0 {
            q8_wall_us.push(elapsed_us);
        }
    }
    q8_wall_us.sort_by(f64::total_cmp);
    eprintln!(
        "RUST_PRODUCTION_ORDINARY_Q8 m={m} lmax={lmax} lanes={lane_count} calls_per_worker=128 wall_median_us={:.3} per_kernel_us={:.3} samples_wall_us={q8_wall_us:?}",
        q8_wall_us[2],
        q8_wall_us[2] / (8.0 * 128.0)
    );
}

#[test]
#[ignore = "manual range-check-only attribution for the ordinary analysis block"]
fn benchmark_production_ordinary_range_check_loop() {
    let input_path = std::env::var("RUSTSHT_ORDINARY_KERNEL_INPUT")
        .expect("RUSTSHT_ORDINARY_KERNEL_INPUT must name the captured block");
    let mut file = File::open(&input_path).expect("open production ordinary kernel fixture");
    let m = read_u64(&mut file) as usize;
    let lmax = read_u64(&mut file) as usize;
    let lane_count = read_u64(&mut file) as usize;
    let pair_count = read_u64(&mut file) as usize;
    let _cosine = read_f64s(&mut file, lane_count);
    let _cosine_squared = read_f64s(&mut file, lane_count);
    let previous = read_f64s(&mut file, lane_count);
    let current = read_f64s(&mut file, lane_count);
    let _spectra_and_coefficients = read_f64s(&mut file, 4 * lane_count + 2 * pair_count);
    let arch = Arch::new();
    let check_count = ((lmax - m + 2) / 2).saturating_sub(2).div_ceil(2);
    let mut samples_us = Vec::with_capacity(5);
    for sample in 0..6 {
        let started = Instant::now();
        let all_in_range = arch.dispatch(AnalysisRangeCheckLoop {
            previous: &previous,
            current: &current,
            repetitions: check_count,
        });
        let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
        assert!(all_in_range);
        if sample != 0 {
            samples_us.push(elapsed_us);
        }
    }
    samples_us.sort_by(f64::total_cmp);
    eprintln!(
        "RUST_RANGE_CHECK_ONLY m={m} lmax={lmax} lanes={lane_count} checks={check_count} median_us={:.3} samples_us={samples_us:?}",
        samples_us[2]
    );
}

#[test]
#[ignore = "manual coefficient-commit attribution for the ordinary analysis block"]
fn benchmark_production_ordinary_coefficient_commit() {
    let input_path = std::env::var("RUSTSHT_ORDINARY_KERNEL_INPUT")
        .expect("RUSTSHT_ORDINARY_KERNEL_INPUT must name the captured block");
    let output_path = input_path
        .strip_suffix(".in")
        .expect("ordinary kernel input ends in .in")
        .to_owned()
        + ".rust";
    let mut input = File::open(&input_path).expect("open ordinary kernel fixture");
    let m = read_u64(&mut input) as usize;
    let lmax = read_u64(&mut input) as usize;
    let _lane_count = read_u64(&mut input);
    let coefficient_count = (lmax - m + 1) as usize;
    let mut output = File::open(&output_path).expect("open Rust staged coefficients");
    let mut staged = Vec::with_capacity(coefficient_count);
    for _ in 0..coefficient_count {
        let mut re = [0_u8; 8];
        let mut im = [0_u8; 8];
        output.read_exact(&mut re).expect("read staged coefficient");
        output.read_exact(&mut im).expect("read staged coefficient");
        staged.push(Complex64::new(
            f64::from_le_bytes(re),
            f64::from_le_bytes(im),
        ));
    }
    let mut coefficients = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut samples_us = Vec::with_capacity(5);
    for sample in 0..6 {
        coefficients.fill(Complex64::new(0.0, 0.0));
        let started = Instant::now();
        for (coefficient, contribution) in coefficients.iter_mut().zip(&staged) {
            *coefficient += *contribution;
        }
        let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
        if sample != 0 {
            samples_us.push(elapsed_us);
        }
    }
    samples_us.sort_by(f64::total_cmp);
    eprintln!(
        "RUST_COEFFICIENT_COMMIT m={m} lmax={lmax} count={coefficient_count} median_us={:.3} samples_us={samples_us:?}",
        samples_us[2]
    );
}

#[test]
#[cfg(feature = "parallel")]
#[ignore = "manual check of late ordinary-range declines on valid full T720 GL analysis"]
fn count_t720_production_late_ordinary_range_declines() {
    let grid = crate::GridSpec::new(crate::GridKind::GaussLegendre, 721, 1442, 0.37).unwrap();
    let harmonics = crate::HarmonicSpec::new(720, 720, 0).unwrap();
    let plan = crate::ShtPlan::new(grid, harmonics).unwrap();
    let ntheta = plan.grid().ntheta();
    let nphi = plan.grid().nphi();
    let input: Vec<_> = (0..ntheta * nphi)
        .map(|index| {
            let theta = (index / nphi) as f64;
            let phi = (index % nphi) as f64;
            (0.019 * theta + 0.037 * phi).sin() + 0.3 * (0.071 * theta - 0.013 * phi).cos()
        })
        .collect();

    for thread_count in [1, 8] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap();
        let mut workspace = pool.install(|| plan.workspace()).unwrap();
        let mut output = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        super::ANALYSIS_PAIR_GROUP_RANGE_DECLINES.store(0, std::sync::atomic::Ordering::Relaxed);
        pool.install(|| plan.analysis_into(&input, &mut output, &mut workspace))
            .unwrap();
        let declines =
            super::ANALYSIS_PAIR_GROUP_RANGE_DECLINES.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(declines, 0, "valid T720 GL analysis hit the late fallback");
        eprintln!(
            "T720_GL_ORDINARY_GROUP_RANGE thread_count={thread_count} late_declines={declines}"
        );
    }
}
