// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
// Test-only direct-kernel benchmark fixture. This module is excluded from
// production builds and calls the same Pulp implementation as the analysis path.

use super::*;
use pulp::{Simd, WithSimd};
use std::fs::File;
use std::io::{Read, Write};
use std::time::Instant;

const BENCH_LMAX: usize = 720;
const BENCH_ORDER: usize = 1;

struct PackedComplexReductionProbe;

impl WithSimd for PackedComplexReductionProbe {
    type Output = Option<([f64; 2], [f64; 2])>;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        if S::F64_LANES != 4 {
            return None;
        }
        let same_re = simd.partial_load_f64s(&[1.0, -2.0, 3.0, 4.0]);
        let same_im = simd.partial_load_f64s(&[5.0, -6.0, 7.0, -8.0]);
        let opposite_re = simd.partial_load_f64s(&[-1.0, 2.0, -3.0, 4.0]);
        let opposite_im = simd.partial_load_f64s(&[8.0, 7.0, 6.0, 5.0]);
        let same = reduce_analysis_complex_pair::<S, true>(simd, same_re, same_im);
        let opposite = reduce_analysis_complex_pair::<S, true>(simd, opposite_re, opposite_im);
        Some(([same.re, same.im], [opposite.re, opposite.im]))
    }
}

#[test]
fn packed_complex_reduction_matches_scalar_lane_sums() {
    if let Some((same, opposite)) = Arch::new().dispatch(PackedComplexReductionProbe) {
        assert_eq!(same, [6.0, -2.0]);
        assert_eq!(opposite, [2.0, 26.0]);
    }
}

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

    fn from_geometry(m: usize, lmax: usize, cosine: Vec<f64>, sine: Vec<f64>) -> Self {
        assert_eq!(cosine.len(), sine.len());
        let lane_count = sine.len();
        let mut epsilon = vec![0.0; lmax + 4];
        scalar::fill_epsilon(m, lmax + 3, &mut epsilon);
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

        let cosine_squared = sine
            .iter()
            .zip(&cosine)
            .map(|(&sin_theta, &cos_theta)| {
                if cos_theta.abs() > 0.99 {
                    (1.0 - sin_theta) * (1.0 + sin_theta)
                } else {
                    cos_theta * cos_theta
                }
            })
            .collect();
        let same_real = (0..lane_count)
            .map(|lane| 0.1 + (0.017 * (lane as f64 + 1.0)).sin())
            .collect();
        let same_imag = (0..lane_count)
            .map(|lane| -0.2 + 0.4 * (0.011 * (lane as f64 + 1.0)).cos())
            .collect();
        let opposite_real = cosine
            .iter()
            .enumerate()
            .map(|(lane, &value)| value * (0.2 + (0.013 * (lane as f64 + 1.0)).sin()))
            .collect();
        let opposite_imag = cosine
            .iter()
            .enumerate()
            .map(|(lane, &value)| value * (0.3 + (0.019 * (lane as f64 + 1.0)).cos()))
            .collect();
        let pair_count = (lmax - m + 2) / 2;

        Self {
            cosine,
            cosine_squared,
            previous: vec![0.0; lane_count],
            current: vec![0.0; lane_count],
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

    fn write_prefix_input(&self, path: &str, m: usize, lmax: usize, scale_epochs: &[f64]) {
        let lane_count = self.cosine.len();
        assert_eq!(self.previous.len(), lane_count);
        assert_eq!(self.current.len(), lane_count);
        assert_eq!(scale_epochs.len(), lane_count);
        let mut file = File::create(path).expect("create prefix fixture");
        for value in [
            m as u64,
            lmax as u64,
            lane_count as u64,
            self.coefficient_a.len() as u64,
        ] {
            file.write_all(&value.to_le_bytes())
                .expect("write prefix fixture header");
        }
        for values in [
            self.cosine.as_slice(),
            self.cosine_squared.as_slice(),
            self.previous.as_slice(),
            self.current.as_slice(),
            self.same_real.as_slice(),
            self.same_imag.as_slice(),
            self.opposite_real.as_slice(),
            self.opposite_imag.as_slice(),
            self.coefficient_a.as_slice(),
            self.coefficient_b.as_slice(),
            scale_epochs,
        ] {
            for value in values {
                file.write_all(&value.to_le_bytes())
                    .expect("write prefix fixture values");
            }
        }
    }
}

struct AnalysisPrefixLoopProbe<'a> {
    m: usize,
    lmax: usize,
    cos_theta_squared: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    scale_epochs: &'a mut [f64],
    scale_corrections: &'a mut [f64],
}

impl WithSimd for AnalysisPrefixLoopProbe<'_> {
    type Output = Option<usize>;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            m,
            lmax,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            previous,
            current,
            scale_epochs,
            scale_corrections,
        } = self;
        let pair_count = (lmax - m + 2) / 2;
        let mut pair_start = 0;
        let mut below_limit = analysis_scale_epochs_below_one(simd, scale_epochs);
        let mut skip_insignificant_validation = scale_epochs.iter().all(|&epoch| epoch < 0.0);
        while below_limit && pair_start < pair_count {
            if pair_count - pair_start == 1 {
                return Some(pair_count);
            }
            let second_pair = pair_start + 1;
            if pair_count - second_pair == 1 {
                return Some(pair_count);
            }
            let (remains_below_limit, remains_insignificant_prefix) =
                advance_and_rescale_analysis_pair(
                    simd,
                    [
                        (coefficient_a[pair_start], coefficient_b[pair_start]),
                        (coefficient_a[second_pair], coefficient_b[second_pair]),
                    ],
                    cos_theta_squared,
                    (&mut *previous, &mut *current),
                    scale_epochs,
                    scale_corrections,
                    skip_insignificant_validation,
                )?;
            pair_start += 2;
            below_limit = remains_below_limit;
            skip_insignificant_validation = remains_insignificant_prefix;
        }
        Some(pair_start)
    }
}

struct AnalysisPrefixBatchProbe<'a, const TIME_KERNEL: bool> {
    repetitions: usize,
    m: usize,
    lmax: usize,
    cos_theta_squared: &'a [f64],
    coefficient_a: &'a [f64],
    coefficient_b: &'a [f64],
    initial_previous: &'a [f64],
    initial_current: &'a [f64],
    initial_scale_epochs: &'a [f64],
    initial_scale_corrections: &'a [f64],
    previous: &'a mut [f64],
    current: &'a mut [f64],
    scale_epochs: &'a mut [f64],
    scale_corrections: &'a mut [f64],
}

impl<const TIME_KERNEL: bool> WithSimd for AnalysisPrefixBatchProbe<'_, TIME_KERNEL> {
    type Output = Option<(f64, usize)>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            repetitions,
            m,
            lmax,
            cos_theta_squared,
            coefficient_a,
            coefficient_b,
            initial_previous,
            initial_current,
            initial_scale_epochs,
            initial_scale_corrections,
            previous,
            current,
            scale_epochs,
            scale_corrections,
        } = self;
        let mut kernel_us = 0.0;
        let mut final_pair_start = 0;
        for _ in 0..repetitions {
            previous.copy_from_slice(initial_previous);
            current.copy_from_slice(initial_current);
            scale_epochs.copy_from_slice(initial_scale_epochs);
            scale_corrections.copy_from_slice(initial_scale_corrections);
            let probe = AnalysisPrefixLoopProbe {
                m,
                lmax,
                cos_theta_squared,
                coefficient_a,
                coefficient_b,
                previous: &mut *previous,
                current: &mut *current,
                scale_epochs: &mut *scale_epochs,
                scale_corrections: &mut *scale_corrections,
            };
            final_pair_start = if TIME_KERNEL {
                let started = Instant::now();
                let result = probe.with_simd(simd)?;
                kernel_us += started.elapsed().as_secs_f64() * 1.0e6;
                result
            } else {
                probe.with_simd(simd)?
            };
            std::hint::black_box((&*previous, &*current, &*scale_epochs, final_pair_start));
        }
        Some((kernel_us, final_pair_start))
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
    const ACCUMULATE: bool,
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

    let mut range_check_was_exercised = false;
    let started = Instant::now();
    let succeeded = arch.dispatch(FusedScalarAnalysisPair::<
        true,
        CHECK_PAIR_GROUP_RANGE,
        FUSE_PAIR_GROUP_RANGE_CHECK,
        ACCUMULATE,
    > {
        m,
        lmax,
        range_check_was_exercised: &mut range_check_was_exercised,
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

#[allow(unexpected_cfgs)]
fn run_analysis_pair_component<const ACCUMULATE: bool>(
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> bool {
    if ACCUMULATE {
        run_fused_scalar_analysis_pair::<false>(arch, fixture, staged, previous, current, next)
    } else {
        run_fused_scalar_analysis_pair_variant::<
            false,
            { cfg!(rustsht_fused_analysis_pair_range_check) },
            false,
        >(arch, fixture, staged, previous, current, next)
    }
}

fn time_analysis_pair_components<const ACCUMULATE: bool>(
    repetitions: usize,
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> (f64, f64, f64) {
    let reset_started = Instant::now();
    for _ in 0..repetitions {
        previous.copy_from_slice(&fixture.previous);
        current.copy_from_slice(&fixture.current);
        staged.fill(Complex64::new(0.0, 0.0));
        std::hint::black_box((&*staged, &*previous, &*current, &*next));
    }
    let reset_us = reset_started.elapsed().as_secs_f64() * 1.0e6;

    let wall_started = Instant::now();
    for _ in 0..repetitions {
        previous.copy_from_slice(&fixture.previous);
        current.copy_from_slice(&fixture.current);
        staged.fill(Complex64::new(0.0, 0.0));
        assert!(run_analysis_pair_component::<ACCUMULATE>(
            arch, fixture, staged, previous, current, next,
        ));
        std::hint::black_box((&*staged, &*previous, &*current, &*next));
    }
    let wall_us = wall_started.elapsed().as_secs_f64() * 1.0e6;
    ((wall_us - reset_us).max(0.0), wall_us, reset_us)
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
    let mut checked_output = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut unchecked_output = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut previous = vec![0.0; lane_count];
    let mut current = vec![0.0; lane_count];
    let mut next = vec![0.0; lane_count];
    let mut component_recurrence_staged = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut component_recurrence_previous = vec![0.0; lane_count];
    let mut component_recurrence_current = vec![0.0; lane_count];
    let mut component_recurrence_next = vec![0.0; lane_count];

    assert!(run_analysis_pair_component::<true>(
        arch,
        &fixture,
        &mut unchecked_staged,
        &mut previous,
        &mut current,
        &mut next,
    ));
    assert!(run_analysis_pair_component::<false>(
        arch,
        &fixture,
        &mut component_recurrence_staged,
        &mut component_recurrence_previous,
        &mut component_recurrence_current,
        &mut component_recurrence_next,
    ));
    assert_eq!(previous, component_recurrence_previous);
    assert_eq!(current, component_recurrence_current);
    assert_eq!(next, component_recurrence_next);
    assert!(
        component_recurrence_staged
            .iter()
            .all(|value| *value == Complex64::new(0.0, 0.0))
    );
    let mut component_reference = vec![Complex64::new(0.0, 0.0); coefficient_count];
    let mut component_reference_previous = vec![0.0; lane_count];
    let mut component_reference_current = vec![0.0; lane_count];
    let mut component_reference_next = vec![0.0; lane_count];
    assert!(run_fused_scalar_analysis_pair::<false>(
        arch,
        &fixture,
        &mut component_reference,
        &mut component_reference_previous,
        &mut component_reference_current,
        &mut component_reference_next,
    ));
    assert_eq!(unchecked_staged, component_reference);
    eprintln!(
        "COMPONENT_VALIDATION lanes={lane_count} recurrence_state_equal=true reference_coefficients_equal=true"
    );

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
            commit_staged_coefficients(&checked_staged, &mut checked_output);
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
            commit_staged_coefficients(&unchecked_staged, &mut unchecked_output);
            assert_eq!(checked_output, unchecked_output);
            write_complex(&checked_output_path, &checked_output);
            write_complex(&unchecked_output_path, &unchecked_output);
            eprintln!("RUST_PREPARED lanes={lane_count} output_equal=true");
        }
        Ok("calibrate") => {
            let calibration_repetitions =
                std::env::var("RUSTSHT_KERNEL_BENCH_CALIBRATION_REPETITIONS")
                    .expect("RUSTSHT_KERNEL_BENCH_CALIBRATION_REPETITIONS must be set")
                    .parse::<usize>()
                    .expect("valid calibration repetition count");
            let commit_calibration_repetitions =
                std::env::var("RUSTSHT_KERNEL_BENCH_COMMIT_CALIBRATION_REPETITIONS")
                    .expect("RUSTSHT_KERNEL_BENCH_COMMIT_CALIBRATION_REPETITIONS must be set")
                    .parse::<usize>()
                    .expect("valid commit calibration repetition count");
            let checked_us = calibrate_fused_scalar_analysis_pair::<true>(
                calibration_repetitions,
                arch,
                &fixture,
                &mut checked_staged,
                &mut checked_output,
                &mut previous,
                &mut current,
                &mut next,
            );
            let unchecked_us = calibrate_fused_scalar_analysis_pair::<false>(
                calibration_repetitions,
                arch,
                &fixture,
                &mut unchecked_staged,
                &mut unchecked_output,
                &mut previous,
                &mut current,
                &mut next,
            );
            let commit_us = calibrate_commit_only(
                commit_calibration_repetitions,
                &checked_staged,
                &mut checked_output,
            );
            assert_eq!(checked_staged, unchecked_staged);
            eprintln!(
                "RUST_CALIBRATION lanes={lane_count} repetitions={calibration_repetitions} checked_us={checked_us:.6} unchecked_us={unchecked_us:.6} commit_repetitions={commit_calibration_repetitions} commit_us={commit_us:.6} output_equal=true"
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
            let commit_repetitions = std::env::var("RUSTSHT_KERNEL_BENCH_COMMIT_REPETITIONS")
                .expect("RUSTSHT_KERNEL_BENCH_COMMIT_REPETITIONS must be set")
                .parse::<usize>()
                .expect("valid commit repetition count");
            assert!(batch_count > 0);
            assert!(repetitions > 0);
            assert!(commit_repetitions > 0);

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
                commit_staged_coefficients(&checked_staged, &mut checked_output);
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
                commit_staged_coefficients(&unchecked_staged, &mut unchecked_output);
            }
            assert_eq!(checked_staged, unchecked_staged);
            assert_eq!(checked_output, unchecked_output);
            for _ in 0..3 {
                time_analysis_pair_components::<true>(
                    1,
                    arch,
                    &fixture,
                    &mut unchecked_staged,
                    &mut previous,
                    &mut current,
                    &mut next,
                );
                time_analysis_pair_components::<false>(
                    1,
                    arch,
                    &fixture,
                    &mut component_recurrence_staged,
                    &mut component_recurrence_previous,
                    &mut component_recurrence_current,
                    &mut component_recurrence_next,
                );
            }

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
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} mode=kernel_only variant=checked kernel_us={:.3} reset_us={:.3} wall_us={:.3}",
                    checked_kernel.0, checked_kernel.2, checked_kernel.1
                );
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} mode=kernel_only variant=unchecked kernel_us={:.3} reset_us={:.3} wall_us={:.3}",
                    unchecked_kernel.0, unchecked_kernel.2, unchecked_kernel.1
                );

                let (checked_commit, unchecked_commit) = if checked_first {
                    let checked =
                        commit_only_batch(commit_repetitions, &checked_staged, &mut checked_output);
                    let unchecked = commit_only_batch(
                        commit_repetitions,
                        &unchecked_staged,
                        &mut unchecked_output,
                    );
                    (checked, unchecked)
                } else {
                    let unchecked = commit_only_batch(
                        commit_repetitions,
                        &unchecked_staged,
                        &mut unchecked_output,
                    );
                    let checked =
                        commit_only_batch(commit_repetitions, &checked_staged, &mut checked_output);
                    (checked, unchecked)
                };
                assert_eq!(checked_output, unchecked_output);
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={commit_repetitions} mode=commit_only variant=checked commit_us={checked_commit:.3}"
                );
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={commit_repetitions} mode=commit_only variant=unchecked commit_us={unchecked_commit:.3}"
                );

                let (checked_combined, unchecked_combined) = if checked_first {
                    let checked = kernel_plus_commit_batch::<true>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut checked_staged,
                        &mut checked_output,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    let unchecked = kernel_plus_commit_batch::<false>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut unchecked_staged,
                        &mut unchecked_output,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    (checked, unchecked)
                } else {
                    let unchecked = kernel_plus_commit_batch::<false>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut unchecked_staged,
                        &mut unchecked_output,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    let checked = kernel_plus_commit_batch::<true>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut checked_staged,
                        &mut checked_output,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    (checked, unchecked)
                };
                assert_eq!(checked_output, unchecked_output);
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} mode=kernel_plus_commit variant=checked kernel_us={:.3} reset_us={:.3} wall_us={:.3}",
                    checked_combined.0, checked_combined.2, checked_combined.1
                );
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} mode=kernel_plus_commit variant=unchecked kernel_us={:.3} reset_us={:.3} wall_us={:.3}",
                    unchecked_combined.0, unchecked_combined.2, unchecked_combined.1
                );

                let (checked_reset, unchecked_reset) = if checked_first {
                    let checked = reset_inclusive_batch::<true>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut checked_staged,
                        &mut checked_output,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    let unchecked = reset_inclusive_batch::<false>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut unchecked_staged,
                        &mut unchecked_output,
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
                        &mut unchecked_output,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    let checked = reset_inclusive_batch::<true>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut checked_staged,
                        &mut checked_output,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    (checked, unchecked)
                };
                assert_eq!(checked_output, unchecked_output);
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} mode=reset_inclusive variant=checked batch_us={:.3} reset_us={:.3} wall_us={:.3}",
                    checked_reset.0, checked_reset.1, checked_reset.0
                );
                eprintln!(
                    "RUST_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} mode=reset_inclusive variant=unchecked batch_us={:.3} reset_us={:.3} wall_us={:.3}",
                    unchecked_reset.0, unchecked_reset.1, unchecked_reset.0
                );

                let (component_full, component_recurrence) = if checked_first {
                    let full = time_analysis_pair_components::<true>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut unchecked_staged,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    let recurrence = time_analysis_pair_components::<false>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut component_recurrence_staged,
                        &mut component_recurrence_previous,
                        &mut component_recurrence_current,
                        &mut component_recurrence_next,
                    );
                    (full, recurrence)
                } else {
                    let recurrence = time_analysis_pair_components::<false>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut component_recurrence_staged,
                        &mut component_recurrence_previous,
                        &mut component_recurrence_current,
                        &mut component_recurrence_next,
                    );
                    let full = time_analysis_pair_components::<true>(
                        repetitions,
                        arch,
                        &fixture,
                        &mut unchecked_staged,
                        &mut previous,
                        &mut current,
                        &mut next,
                    );
                    (full, recurrence)
                };
                assert_eq!(previous, component_recurrence_previous);
                assert_eq!(current, component_recurrence_current);
                assert_eq!(next, component_recurrence_next);
                eprintln!(
                    "COMPONENT_BATCH lanes={lane_count} batch={batch} repetitions={repetitions} full_kernel_us={:.3} recurrence_kernel_us={:.3} full_reset_us={:.3} recurrence_reset_us={:.3} full_wall_us={:.3} recurrence_wall_us={:.3}",
                    component_full.0,
                    component_recurrence.0,
                    component_full.2,
                    component_recurrence.2,
                    component_full.1,
                    component_recurrence.1,
                );
            }

            write_complex(&checked_output_path, &checked_output);
            write_complex(&unchecked_output_path, &unchecked_output);
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
        true,
    >(arch, fixture, staged, previous, current, next)
}

#[inline(never)]
fn run_fused_scalar_analysis_pair_variant<
    const CHECK_PAIR_GROUP_RANGE: bool,
    const FUSE_PAIR_GROUP_RANGE_CHECK: bool,
    const ACCUMULATE: bool,
>(
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> bool {
    let mut range_check_was_exercised = false;
    arch.dispatch(FusedScalarAnalysisPair::<
        true,
        CHECK_PAIR_GROUP_RANGE,
        FUSE_PAIR_GROUP_RANGE_CHECK,
        ACCUMULATE,
    > {
        m: BENCH_ORDER,
        lmax: BENCH_LMAX,
        range_check_was_exercised: &mut range_check_was_exercised,
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

#[expect(
    clippy::too_many_arguments,
    reason = "The benchmark forwards the operation state explicitly to match the production kernel inputs."
)]
fn calibrate_fused_scalar_analysis_pair<const CHECK_PAIR_GROUP_RANGE: bool>(
    repetitions: usize,
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    output: &mut [Complex64],
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
        output.fill(Complex64::new(0.0, 0.0));
        let succeeded = run_fused_scalar_analysis_pair::<CHECK_PAIR_GROUP_RANGE>(
            arch, fixture, staged, previous, current, next,
        );
        if succeeded {
            commit_staged_coefficients(staged, output);
        }
        all_succeeded &= succeeded;
        std::hint::black_box((&*staged, &*output, &*previous, &*current, &*next));
    }
    let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6 / repetitions as f64;
    assert!(all_succeeded);
    elapsed_us
}

fn calibrate_commit_only(
    repetitions: usize,
    staged: &[Complex64],
    output: &mut [Complex64],
) -> f64 {
    output.fill(Complex64::new(0.0, 0.0));
    let started = Instant::now();
    for _ in 0..repetitions {
        commit_staged_coefficients(staged, output);
    }
    let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6 / repetitions as f64;
    std::hint::black_box(&*output);
    assert!(
        output
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
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
) -> (f64, f64, f64) {
    // Time whole batches, not individual calls. Subtract a separate batch of
    // identical resets to estimate the recurrence/staging time without
    // adding a clock read to every kernel invocation.
    let reset_started = Instant::now();
    for _ in 0..repetitions {
        reset_fused_scalar_analysis_pair(fixture, staged, previous, current);
        std::hint::black_box((&*staged, &*previous, &*current));
    }
    let reset_us = reset_started.elapsed().as_secs_f64() * 1.0e6;

    let wall_started = Instant::now();
    let mut all_succeeded = true;
    for _ in 0..repetitions {
        reset_fused_scalar_analysis_pair(fixture, staged, previous, current);
        let succeeded = run_fused_scalar_analysis_pair::<CHECK_PAIR_GROUP_RANGE>(
            arch, fixture, staged, previous, current, next,
        );
        all_succeeded &= succeeded;
        std::hint::black_box((&*staged, &*previous, &*current, &*next));
    }
    assert!(all_succeeded);
    let wall_us = wall_started.elapsed().as_secs_f64() * 1.0e6;
    ((wall_us - reset_us).max(0.0), wall_us, reset_us)
}

fn commit_staged_coefficients(staged: &[Complex64], output: &mut [Complex64]) {
    for (coefficient, contribution) in output.iter_mut().zip(staged) {
        *coefficient += *contribution;
    }
}

fn reset_analysis_kernel_state(
    fixture: &Fixture,
    output: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
) {
    previous.copy_from_slice(&fixture.previous);
    current.copy_from_slice(&fixture.current);
    output.fill(Complex64::new(0.0, 0.0));
}

fn commit_only_batch(repetitions: usize, staged: &[Complex64], output: &mut [Complex64]) -> f64 {
    output.fill(Complex64::new(0.0, 0.0));
    let started = Instant::now();
    for _ in 0..repetitions {
        commit_staged_coefficients(staged, output);
    }
    let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
    std::hint::black_box(&*output);
    assert!(
        output
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    elapsed_us
}

#[expect(
    clippy::too_many_arguments,
    reason = "The benchmark forwards the operation state explicitly to match the production kernel inputs."
)]
fn kernel_plus_commit_batch<const CHECK_PAIR_GROUP_RANGE: bool>(
    repetitions: usize,
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    output: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> (f64, f64, f64) {
    // This is the reset-excluded recurrence plus coefficient-commit estimate;
    // the caller also records the unadjusted operation-batch wall time.
    let reset_started = Instant::now();
    for _ in 0..repetitions {
        reset_analysis_kernel_state(fixture, output, previous, current);
        std::hint::black_box((&*output, &*previous, &*current));
    }
    let reset_us = reset_started.elapsed().as_secs_f64() * 1.0e6;

    let wall_started = Instant::now();
    let mut all_succeeded = true;
    for _ in 0..repetitions {
        reset_analysis_kernel_state(fixture, output, previous, current);
        let succeeded = run_fused_scalar_analysis_pair::<CHECK_PAIR_GROUP_RANGE>(
            arch, fixture, staged, previous, current, next,
        );
        if succeeded {
            commit_staged_coefficients(staged, output);
        }
        all_succeeded &= succeeded;
        std::hint::black_box((&*staged, &*output, &*previous, &*current, &*next));
    }
    assert!(all_succeeded);
    let wall_us = wall_started.elapsed().as_secs_f64() * 1.0e6;
    ((wall_us - reset_us).max(0.0), wall_us, reset_us)
}

#[expect(
    clippy::too_many_arguments,
    reason = "The benchmark forwards the operation state explicitly to match the production kernel inputs."
)]
fn reset_inclusive_batch<const CHECK_PAIR_GROUP_RANGE: bool>(
    repetitions: usize,
    arch: Arch,
    fixture: &Fixture,
    staged: &mut [Complex64],
    output: &mut [Complex64],
    previous: &mut [f64],
    current: &mut [f64],
    next: &mut [f64],
) -> (f64, f64) {
    // Retain both values: the unadjusted operation time is the reset-inclusive
    // result, while reset-only is reported to expose buffer-management cost.
    let reset_started = Instant::now();
    for _ in 0..repetitions {
        reset_analysis_kernel_state(fixture, output, previous, current);
        std::hint::black_box((&*output, &*previous, &*current));
    }
    let reset_us = reset_started.elapsed().as_secs_f64() * 1.0e6;

    let started = Instant::now();
    let mut all_succeeded = true;
    for _ in 0..repetitions {
        reset_analysis_kernel_state(fixture, output, previous, current);
        let succeeded = run_fused_scalar_analysis_pair::<CHECK_PAIR_GROUP_RANGE>(
            arch, fixture, staged, previous, current, next,
        );
        if succeeded {
            commit_staged_coefficients(staged, output);
        }
        all_succeeded &= succeeded;
        std::hint::black_box((&*staged, &*output, &*previous, &*current, &*next));
    }
    let elapsed_us = started.elapsed().as_secs_f64() * 1.0e6;
    assert!(all_succeeded);
    (elapsed_us, reset_us)
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
            let _ = time_fused_scalar_analysis_pair::<true, false, true>(
                arch,
                BENCH_ORDER,
                BENCH_LMAX,
                &fixture,
                &mut scanned_staged,
                &mut previous,
                &mut current,
                &mut next,
            );
            let _ = time_fused_scalar_analysis_pair::<true, true, true>(
                arch,
                BENCH_ORDER,
                BENCH_LMAX,
                &fixture,
                &mut fused_staged,
                &mut previous,
                &mut current,
                &mut next,
            );
            let _ = time_fused_scalar_analysis_pair::<false, false, true>(
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
        let scanned_succeeded = run_fused_scalar_analysis_pair_variant::<true, false, true>(
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
        let fused_succeeded = run_fused_scalar_analysis_pair_variant::<true, true, true>(
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
    let grid = crate::GridSpec::new(crate::GridKind::GaussLegendre, 721, 1443, 0.37).unwrap();
    const REALISTIC_M: usize = 500;
    for lane_count in [64, 128, 256] {
        let mut synthetic = Fixture::new(lane_count);
        synthetic.previous.fill(0.0);
        synthetic.current.fill(2.0_f64.powi(-100));
        run_prefix_probe_case(
            arch,
            &output_base,
            "synthetic",
            BENCH_ORDER,
            BENCH_LMAX,
            synthetic,
            vec![-1.0; lane_count],
        );

        let active_geometry: Vec<_> = grid
            .theta()
            .iter()
            .map(|&theta| (theta.sin(), theta.cos()))
            .filter(|&(sine, cosine)| {
                REALISTIC_M <= scalar::polar_order_limit(BENCH_LMAX, 0, sine, cosine)
            })
            .take(lane_count)
            .collect();
        assert_eq!(active_geometry.len(), lane_count);
        let sine: Vec<_> = active_geometry.iter().map(|&(sine, _)| sine).collect();
        let cosine: Vec<_> = active_geometry.iter().map(|&(_, cosine)| cosine).collect();
        let sectoral_norm = scalar::sectoral_normalizations(REALISTIC_M).unwrap()[REALISTIC_M];
        let mut previous = vec![0.0; lane_count];
        let mut current = vec![0.0; lane_count];
        let mut scale_epochs = vec![0.0; lane_count];
        assert!(
            !initialize_scaled_analysis_sectoral_lanes(
                arch,
                REALISTIC_M,
                BENCH_LMAX,
                &sine,
                sectoral_norm,
                &mut previous,
                &mut current,
                &mut scale_epochs,
            )
            .unwrap()
            .expect("realistic GL sectoral seed")
        );
        assert!(scale_epochs.iter().all(|epoch| epoch.is_finite()));
        let initially_below_limit = scale_epochs.iter().all(|&epoch| epoch < 1.0);
        assert!(current.iter().any(|value| value.abs() <= ANALYSIS_FTOL));
        let scaled_lanes = current
            .iter()
            .filter(|value| value.abs() <= ANALYSIS_FTOL)
            .count();
        let mut realistic = Fixture::from_geometry(REALISTIC_M, BENCH_LMAX, cosine, sine);
        realistic.previous = previous;
        realistic.current = current;
        eprintln!(
            "RUST_PREFIX_SEED kind=gl_sectoral m={REALISTIC_M} lanes={lane_count} below_limit={initially_below_limit} insignificant_seed_lanes={scaled_lanes}"
        );
        run_prefix_probe_case(
            arch,
            &output_base,
            "gl-sectoral",
            REALISTIC_M,
            BENCH_LMAX,
            realistic,
            scale_epochs,
        );
    }
}

fn run_prefix_probe_case(
    arch: Arch,
    output_base: &str,
    label: &str,
    m: usize,
    lmax: usize,
    fixture: Fixture,
    initial_scale_epochs: Vec<f64>,
) {
    let lane_count = fixture.cosine.len();
    let case_key = format!("{label}-{lane_count}");
    if std::env::var("RUSTSHT_PREFIX_ONLY_CASE").is_ok_and(|selected| selected != case_key) {
        return;
    }
    let input_path = format!("{output_base}-{label}-{lane_count}.in");
    let state_path = format!("{output_base}-{label}-{lane_count}.rust_state");
    fixture.write_prefix_input(&input_path, m, lmax, &initial_scale_epochs);
    let mut previous = fixture.previous.clone();
    let mut current = fixture.current.clone();
    let mut scale_epochs = initial_scale_epochs.clone();
    let mut scale_corrections = vec![0.0; lane_count];
    let pair_count = (lmax - m + 2) / 2;
    let preflight_started = Instant::now();
    assert!(analysis_scale_epochs_allow_recurrence(
        &initial_scale_epochs,
        pair_count,
    ));
    assert!(fill_analysis_scale_corrections(
        &fixture.current,
        &initial_scale_epochs,
        &mut scale_corrections,
    ));
    let preflight_us = preflight_started.elapsed().as_secs_f64() * 1.0e6;
    let initial_scale_corrections = scale_corrections.clone();

    // Match DUCC's iter_to_ieee boundary: prefix recurrence and reset-inclusive
    // batches are timed independently, outside Rust's scale/correction preflight.
    let bootstrap_repetitions = 1024;
    for _ in 0..4 {
        prefix_probe_batch(
            1,
            arch,
            m,
            lmax,
            &fixture,
            &initial_scale_epochs,
            &initial_scale_corrections,
            &mut previous,
            &mut current,
            &mut scale_epochs,
            &mut scale_corrections,
        );
    }
    let (bootstrap_kernel_us, bootstrap_pair_start) = prefix_probe_batch(
        bootstrap_repetitions,
        arch,
        m,
        lmax,
        &fixture,
        &initial_scale_epochs,
        &initial_scale_corrections,
        &mut previous,
        &mut current,
        &mut scale_epochs,
        &mut scale_corrections,
    );
    let bootstrap_wall_us = prefix_probe_wall_batch(
        bootstrap_repetitions,
        arch,
        m,
        lmax,
        &fixture,
        &initial_scale_epochs,
        &initial_scale_corrections,
        &mut previous,
        &mut current,
        &mut scale_epochs,
        &mut scale_corrections,
    );
    let bootstrap_timer_us_per_call = prefix_probe_timer_overhead(bootstrap_repetitions);
    let bootstrap_kernel_per_call =
        bootstrap_kernel_us / bootstrap_repetitions as f64 - bootstrap_timer_us_per_call;
    let bootstrap_wall_per_call = bootstrap_wall_us / bootstrap_repetitions as f64;
    let calibration_kernel_repetitions = if bootstrap_pair_start == 0 {
        bootstrap_repetitions
    } else {
        prefix_probe_repetitions(bootstrap_kernel_per_call)
    };
    let calibration_wall_repetitions = if bootstrap_pair_start == 0 {
        bootstrap_repetitions
    } else {
        prefix_probe_repetitions(bootstrap_wall_per_call)
    };
    let (calibration_kernel_us, calibration_pair_start) = prefix_probe_batch(
        calibration_kernel_repetitions,
        arch,
        m,
        lmax,
        &fixture,
        &initial_scale_epochs,
        &initial_scale_corrections,
        &mut previous,
        &mut current,
        &mut scale_epochs,
        &mut scale_corrections,
    );
    let calibration_wall_us = prefix_probe_wall_batch(
        calibration_wall_repetitions,
        arch,
        m,
        lmax,
        &fixture,
        &initial_scale_epochs,
        &initial_scale_corrections,
        &mut previous,
        &mut current,
        &mut scale_epochs,
        &mut scale_corrections,
    );
    let timer_us_per_call = prefix_probe_timer_overhead(bootstrap_repetitions);
    let kernel_per_call =
        calibration_kernel_us / calibration_kernel_repetitions as f64 - timer_us_per_call;
    let wall_per_call = calibration_wall_us / calibration_wall_repetitions as f64;
    eprintln!(
        "RUST_PREFIX_CALIBRATION kind={label} m={m} lanes={lane_count} kernel_repetitions={calibration_kernel_repetitions} wall_repetitions={calibration_wall_repetitions} kernel_us_per_call={kernel_per_call:.6} wall_us_per_call={wall_per_call:.6} timer_us_per_call={timer_us_per_call:.6} preflight_us={preflight_us:.3} pair_start={calibration_pair_start}"
    );

    if std::env::var("RUSTSHT_PREFIX_CALIBRATE_ONLY").as_deref() == Ok("1") {
        write_f64s(&state_path, [&previous, &current, &scale_epochs]);
        return;
    }

    let case_suffix = format!("{}_{}", label.replace('-', "_").to_uppercase(), lane_count);
    let kernel_repetitions = std::env::var(format!("RUSTSHT_PREFIX_KERNEL_REPS_{case_suffix}"))
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(calibration_kernel_repetitions);
    let wall_repetitions = std::env::var(format!("RUSTSHT_PREFIX_WALL_REPS_{case_suffix}"))
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(calibration_wall_repetitions);
    let batches = std::env::var("RUSTSHT_PREFIX_BATCHES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30_usize);
    assert!(kernel_repetitions > 0 && wall_repetitions > 0 && batches > 0);

    for _ in 0..2 {
        prefix_probe_batch(
            kernel_repetitions,
            arch,
            m,
            lmax,
            &fixture,
            &initial_scale_epochs,
            &initial_scale_corrections,
            &mut previous,
            &mut current,
            &mut scale_epochs,
            &mut scale_corrections,
        );
        prefix_probe_wall_batch(
            wall_repetitions,
            arch,
            m,
            lmax,
            &fixture,
            &initial_scale_epochs,
            &initial_scale_corrections,
            &mut previous,
            &mut current,
            &mut scale_epochs,
            &mut scale_corrections,
        );
    }
    let timed_timer_us_per_call = prefix_probe_timer_overhead(bootstrap_repetitions);

    for batch in 0..batches {
        let (kernel_us, pair_start) = prefix_probe_batch(
            kernel_repetitions,
            arch,
            m,
            lmax,
            &fixture,
            &initial_scale_epochs,
            &initial_scale_corrections,
            &mut previous,
            &mut current,
            &mut scale_epochs,
            &mut scale_corrections,
        );
        let wall_us = prefix_probe_wall_batch(
            wall_repetitions,
            arch,
            m,
            lmax,
            &fixture,
            &initial_scale_epochs,
            &initial_scale_corrections,
            &mut previous,
            &mut current,
            &mut scale_epochs,
            &mut scale_corrections,
        );
        assert!(
            previous
                .iter()
                .chain(&current)
                .all(|value| value.is_finite())
        );
        eprintln!(
            "RUST_PREFIX_KERNEL_BATCH kind={label} m={m} lanes={lane_count} batch={batch} repetitions={kernel_repetitions} kernel_us={kernel_us:.3} timer_us_per_call={timed_timer_us_per_call:.6} pair_start={pair_start}"
        );
        eprintln!(
            "RUST_PREFIX_WALL_BATCH kind={label} m={m} lanes={lane_count} batch={batch} repetitions={wall_repetitions} wall_us={wall_us:.3}"
        );
    }

    write_f64s(&state_path, [&previous, &current, &scale_epochs]);
}

#[expect(
    clippy::too_many_arguments,
    reason = "The prefix probe uses the same reset state, geometry and SIMD dispatch for each repeated operation."
)]
fn prefix_probe_batch(
    repetitions: usize,
    arch: Arch,
    m: usize,
    lmax: usize,
    fixture: &Fixture,
    initial_scale_epochs: &[f64],
    initial_scale_corrections: &[f64],
    previous: &mut [f64],
    current: &mut [f64],
    scale_epochs: &mut [f64],
    scale_corrections: &mut [f64],
) -> (f64, usize) {
    arch.dispatch(AnalysisPrefixBatchProbe::<true> {
        repetitions,
        m,
        lmax,
        cos_theta_squared: &fixture.cosine_squared,
        coefficient_a: &fixture.coefficient_a,
        coefficient_b: &fixture.coefficient_b,
        initial_previous: &fixture.previous,
        initial_current: &fixture.current,
        initial_scale_epochs,
        initial_scale_corrections,
        previous,
        current,
        scale_epochs,
        scale_corrections,
    })
    .expect("scaled prefix recurrence batch")
}

#[expect(
    clippy::too_many_arguments,
    reason = "The reset-inclusive probe uses the same state, geometry and SIMD dispatch as the kernel-only probe."
)]
fn prefix_probe_wall_batch(
    repetitions: usize,
    arch: Arch,
    m: usize,
    lmax: usize,
    fixture: &Fixture,
    initial_scale_epochs: &[f64],
    initial_scale_corrections: &[f64],
    previous: &mut [f64],
    current: &mut [f64],
    scale_epochs: &mut [f64],
    scale_corrections: &mut [f64],
) -> f64 {
    let wall_started = Instant::now();
    let (_, final_pair_start) = arch
        .dispatch(AnalysisPrefixBatchProbe::<false> {
            repetitions,
            m,
            lmax,
            cos_theta_squared: &fixture.cosine_squared,
            coefficient_a: &fixture.coefficient_a,
            coefficient_b: &fixture.coefficient_b,
            initial_previous: &fixture.previous,
            initial_current: &fixture.current,
            initial_scale_epochs,
            initial_scale_corrections,
            previous,
            current,
            scale_epochs,
            scale_corrections,
        })
        .expect("reset-inclusive prefix recurrence batch");
    std::hint::black_box(final_pair_start);
    wall_started.elapsed().as_secs_f64() * 1.0e6
}

fn prefix_probe_timer_overhead(repetitions: usize) -> f64 {
    let mut elapsed_us = 0.0;
    for _ in 0..repetitions {
        let started = Instant::now();
        std::hint::black_box(());
        elapsed_us += started.elapsed().as_secs_f64() * 1.0e6;
    }
    elapsed_us / repetitions as f64
}

fn prefix_probe_repetitions(us_per_call: f64) -> usize {
    const TARGET_BATCH_US: f64 = 32_000.0;
    const MAX_CALIBRATION_BATCH_US: f64 = 80_000.0;
    assert!(us_per_call.is_finite() && us_per_call > 0.0);
    let minimum = (20_000.0 / us_per_call).ceil() as usize;
    let maximum = (MAX_CALIBRATION_BATCH_US / us_per_call).floor() as usize;
    assert!(minimum <= maximum, "batch duration does not fit 20-80 ms");
    ((TARGET_BATCH_US / us_per_call).round() as usize).clamp(minimum, maximum)
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
        let mut range_check_was_exercised = false;
        let started = Instant::now();
        let succeeded = arch
            .dispatch(FusedScaledScalarAnalysisPair::<true> {
                m,
                lmax,
                range_check_was_exercised: &mut range_check_was_exercised,
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
                false,
            )
        })
        .collect();
    let mut q8_wall_us = Vec::with_capacity(5);
    for sample in 0..6 {
        let started = Instant::now();
        std::thread::scope(|scope| {
            for (previous, current, next, epochs, corrections, staged, range_check_was_exercised) in
                &mut workers
            {
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
                        *range_check_was_exercised = false;
                        assert!(
                            arch.dispatch(FusedScaledScalarAnalysisPair::<true> {
                                m,
                                lmax,
                                range_check_was_exercised,
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
        let mut range_check_was_exercised = false;
        let started = Instant::now();
        let succeeded = arch.dispatch(FusedScalarAnalysisPair::<true> {
            m,
            lmax,
            range_check_was_exercised: &mut range_check_was_exercised,
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
                false,
            )
        })
        .collect();
    let mut q8_wall_us = Vec::with_capacity(5);
    for sample in 0..6 {
        let started = Instant::now();
        std::thread::scope(|scope| {
            for (previous, current, next, staged, range_check_was_exercised) in &mut workers {
                let fixture = &fixture;
                scope.spawn(move || {
                    for _ in 0..128 {
                        previous.copy_from_slice(&fixture.previous);
                        current.copy_from_slice(&fixture.current);
                        next.fill(0.0);
                        staged.fill(Complex64::new(0.0, 0.0));
                        *range_check_was_exercised = false;
                        assert!(arch.dispatch(FusedScalarAnalysisPair::<true> {
                            m,
                            lmax,
                            range_check_was_exercised,
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
