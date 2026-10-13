// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

use std::hint::black_box;
use std::time::{Duration, Instant};

use duccfft::{ComplexFftPlan, RealFftPlan};
use num_complex::Complex64;
use rustfft::{Fft, FftPlanner, FftPlannerScalar};

#[path = "prototypes/real_multipass.rs"]
mod real_multipass_prototype;

const BATCH_COUNT: usize = 32;
const SAMPLE_COUNT: usize = 7;

const BENCHMARK_LENGTHS: &[usize] = &[
    13, 17, 31, 32, 33, 41, 53, 64, 107, 109, 113, 127, 128, 131, 137, 241, 257, 511, 512, 683,
    998, 1000, 1001, 1002, 1008, 1013, 1023, 1024, 1025, 1026, 2047, 2048, 2049, 4096, 4097, 10000,
    10001, 10008,
];
const SHT_WORKLOADS: &[(usize, usize)] = &[(257, 128), (1025, 512), (2049, 1024), (4097, 2048)];

#[derive(Clone, Copy)]
enum Mode {
    ComplexForward,
    ComplexInverse,
    RealForward,
    RealInverse,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::ComplexForward => "c2c_forward",
            Self::ComplexInverse => "c2c_inverse",
            Self::RealForward => "r2c",
            Self::RealInverse => "c2r",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "c2c_forward" => Self::ComplexForward,
            "c2c_inverse" => Self::ComplexInverse,
            "r2c" => Self::RealForward,
            "c2r" => Self::RealInverse,
            _ => panic!("unknown mode {value}"),
        }
    }
}

struct ComplexPlan {
    n: usize,
    forward: std::sync::Arc<dyn Fft<f64>>,
    inverse: std::sync::Arc<dyn Fft<f64>>,
    scratch_len: usize,
}

impl ComplexPlan {
    fn new(n: usize) -> Self {
        let mut planner = FftPlanner::<f64>::new();
        Self::new_with_plans(n, planner.plan_fft_forward(n), planner.plan_fft_inverse(n))
    }

    fn new_scalar(n: usize) -> Self {
        let mut planner = FftPlannerScalar::<f64>::new();
        Self::new_with_plans(n, planner.plan_fft_forward(n), planner.plan_fft_inverse(n))
    }

    fn new_with_plans(
        n: usize,
        forward: std::sync::Arc<dyn Fft<f64>>,
        inverse: std::sync::Arc<dyn Fft<f64>>,
    ) -> Self {
        let scratch_len = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        Self {
            n,
            forward,
            inverse,
            scratch_len,
        }
    }

    fn execute_raw(&self, values: &mut [Complex64], scratch: &mut [Complex64], forward: bool) {
        if forward {
            self.forward.process_with_scratch(values, scratch);
        } else {
            self.inverse.process_with_scratch(values, scratch);
        }
    }

    fn normalize_inverse(&self, values: &mut [Complex64]) {
        let scale = 1.0 / self.n as f64;
        for value in values {
            *value *= scale;
        }
    }
}

struct RealPlan {
    n: usize,
    complex: ComplexPlan,
    twiddles: Vec<Complex64>,
}

impl RealPlan {
    fn new(n: usize) -> Self {
        Self::new_with_complex(n, ComplexPlan::new)
    }

    fn new_scalar(n: usize) -> Self {
        Self::new_with_complex(n, ComplexPlan::new_scalar)
    }

    fn new_with_complex(n: usize, make_complex: fn(usize) -> ComplexPlan) -> Self {
        let transform_len = if n.is_multiple_of(2) { n / 2 } else { n };
        let complex = make_complex(transform_len);
        let twiddles = if n.is_multiple_of(2) {
            (0..=transform_len)
                .map(|k| {
                    let angle = -std::f64::consts::PI * k as f64 / transform_len as f64;
                    Complex64::from_polar(1.0, angle)
                })
                .collect()
        } else {
            Vec::new()
        };
        Self {
            n,
            complex,
            twiddles,
        }
    }

    fn bins(&self) -> usize {
        self.n / 2 + 1
    }

    fn scratch_len(&self) -> usize {
        self.complex.scratch_len
    }

    fn prepare_forward(&self, input: &[f64], transform: &mut [Complex64]) {
        if self.n.is_multiple_of(2) {
            let (pairs, remainder) = input.as_chunks::<2>();
            debug_assert!(remainder.is_empty());
            for (slot, pair) in transform.iter_mut().zip(pairs) {
                *slot = Complex64::new(pair[0], pair[1]);
            }
        } else {
            for (slot, &sample) in transform.iter_mut().zip(input) {
                *slot = Complex64::new(sample, 0.0);
            }
        }
    }

    fn finish_forward(&self, transform: &[Complex64], output: &mut [Complex64]) {
        if self.n.is_multiple_of(2) {
            let half = self.n / 2;
            for (k, value) in output.iter_mut().enumerate() {
                let z = transform[k % half];
                let mirror = transform[(half - k % half) % half].conj();
                let even = (z + mirror) * 0.5;
                let odd = Complex64::new(0.0, -0.5) * self.twiddles[k] * (z - mirror);
                *value = even + odd;
            }
            output[0].im = 0.0;
            output[half].im = 0.0;
        } else {
            output.copy_from_slice(&transform[..self.bins()]);
            output[0].im = 0.0;
        }
    }

    fn prepare_inverse(&self, input: &[Complex64], transform: &mut [Complex64]) {
        if self.n.is_multiple_of(2) {
            let half = self.n / 2;
            for k in 0..half {
                let a = input[k];
                let b = if k == 0 {
                    input[half]
                } else {
                    input[half - k].conj()
                };
                transform[k] =
                    (a + b) * 0.5 + Complex64::new(0.0, 0.5) * self.twiddles[k].conj() * (a - b);
            }
        } else {
            transform[0] = Complex64::new(input[0].re, 0.0);
            for (k, &value) in input.iter().enumerate().skip(1) {
                transform[k] = value;
                transform[self.n - k] = value.conj();
            }
        }
    }

    fn finish_inverse(&self, transform: &[Complex64], output: &mut [f64]) {
        if self.n.is_multiple_of(2) {
            let scale = 1.0 / (self.n / 2) as f64;
            let (pairs, remainder) = output.as_chunks_mut::<2>();
            debug_assert!(remainder.is_empty());
            for (pair, value) in pairs.iter_mut().zip(transform) {
                pair[0] = value.re * scale;
                pair[1] = value.im * scale;
            }
        } else {
            let scale = 1.0 / self.n as f64;
            for (value, sample) in output.iter_mut().zip(transform) {
                *value = sample.re * scale;
            }
        }
    }

    fn forward(
        &self,
        input: &[f64],
        output: &mut [Complex64],
        transform: &mut [Complex64],
        scratch: &mut [Complex64],
    ) {
        assert_eq!(input.len(), self.n);
        assert_eq!(output.len(), self.bins());
        self.prepare_forward(input, transform);
        self.complex
            .forward
            .process_with_scratch(transform, scratch);
        self.finish_forward(transform, output);
    }

    fn inverse(
        &self,
        input: &[Complex64],
        output: &mut [f64],
        transform: &mut [Complex64],
        scratch: &mut [Complex64],
    ) {
        assert_eq!(input.len(), self.bins());
        assert_eq!(output.len(), self.n);
        self.prepare_inverse(input, transform);
        self.complex
            .inverse
            .process_with_scratch(transform, scratch);
        self.finish_inverse(transform, output);
    }
}

fn real_value(index: usize, row: usize) -> f64 {
    let x = (index + 1 + 3 * row) as f64;
    (0.173 * x).sin() + 0.25 * (0.113 * (x + 1.0)).cos()
}

fn complex_value(index: usize, row: usize) -> Complex64 {
    let x = (index + 1 + 3 * row) as f64;
    Complex64::new((0.173 * x).sin(), (0.113 * (x + 1.0)).cos())
}

fn spectrum_value(index: usize, n: usize, row: usize) -> Complex64 {
    if index == 0 || (n.is_multiple_of(2) && index == n / 2) {
        return Complex64::new(0.4 + 0.01 * (index + row) as f64, 0.0);
    }
    let x = (index + 1 + 5 * row) as f64;
    Complex64::new(0.3 * (0.19 * x).sin(), 0.2 * (0.07 * (x + 1.0)).cos())
}

fn median_duration(mut durations: Vec<Duration>) -> Duration {
    durations.sort_unstable();
    durations[durations.len() / 2]
}

fn measure(mut elapsed_sample: impl FnMut() -> Duration) -> Duration {
    black_box(elapsed_sample());
    let mut durations = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        durations.push(elapsed_sample());
    }
    median_duration(durations)
}

fn plan_ns(n: usize, real: bool, scalar: bool) -> f64 {
    let repeats = 20;
    let start = Instant::now();
    for _ in 0..repeats {
        black_box(if real {
            (if scalar {
                RealPlan::new_scalar(n)
            } else {
                RealPlan::new(n)
            })
            .scratch_len()
        } else {
            if scalar {
                ComplexPlan::new_scalar(n).scratch_len
            } else {
                ComplexPlan::new(n).scratch_len
            }
        });
    }
    start.elapsed().as_nanos() as f64 / repeats as f64
}

fn allocation_ns<T: Default + Clone>(lengths: &[usize]) -> f64 {
    let repeats = 20;
    let start = Instant::now();
    for _ in 0..repeats {
        for &len in lengths {
            black_box(vec![T::default(); len]);
        }
    }
    start.elapsed().as_nanos() as f64 / repeats as f64
}

fn output(mode: Mode, n: usize) -> Vec<Complex64> {
    match mode {
        Mode::ComplexForward | Mode::ComplexInverse => {
            let plan = ComplexPlan::new(n);
            let mut values: Vec<_> = (0..n).map(|i| complex_value(i, 0)).collect();
            let mut scratch = vec![Complex64::new(0.0, 0.0); plan.scratch_len];
            plan.execute_raw(
                &mut values,
                &mut scratch,
                matches!(mode, Mode::ComplexForward),
            );
            if matches!(mode, Mode::ComplexInverse) {
                plan.normalize_inverse(&mut values);
            }
            values
        }
        Mode::RealForward => {
            let plan = RealPlan::new(n);
            let input: Vec<_> = (0..n).map(|i| real_value(i, 0)).collect();
            let mut values = vec![Complex64::new(0.0, 0.0); plan.bins()];
            let mut transform = vec![Complex64::new(0.0, 0.0); plan.complex.n];
            let mut scratch = vec![Complex64::new(0.0, 0.0); plan.scratch_len()];
            plan.forward(&input, &mut values, &mut transform, &mut scratch);
            values
        }
        Mode::RealInverse => {
            let plan = RealPlan::new(n);
            let input: Vec<_> = (0..plan.bins()).map(|k| spectrum_value(k, n, 0)).collect();
            let mut values = vec![0.0; n];
            let mut transform = vec![Complex64::new(0.0, 0.0); plan.complex.n];
            let mut scratch = vec![Complex64::new(0.0, 0.0); plan.scratch_len()];
            plan.inverse(&input, &mut values, &mut transform, &mut scratch);
            values
                .into_iter()
                .map(|value| Complex64::new(value, 0.0))
                .collect()
        }
    }
}

fn native_complex_output(mode: Mode, n: usize) -> Vec<Complex64> {
    let plan = ComplexFftPlan::<f64>::new(n).expect("native complex plan");
    let mut workspace = plan.workspace().expect("native complex workspace");
    let mut values: Vec<_> = (0..n).map(|i| complex_value(i, 0)).collect();
    match mode {
        Mode::ComplexForward => plan.forward(&mut values, &mut workspace),
        Mode::ComplexInverse => plan.inverse(&mut values, &mut workspace),
        Mode::RealForward | Mode::RealInverse => unreachable!("complex mode required"),
    }
    .expect("native complex transform");
    values
}

fn native_real_output(mode: Mode, n: usize) -> Vec<Complex64> {
    let plan = RealFftPlan::<f64>::new(n).expect("native real plan");
    let mut workspace = plan.workspace().expect("native real workspace");
    match mode {
        Mode::RealForward => {
            let input: Vec<_> = (0..n).map(|index| real_value(index, 0)).collect();
            let mut spectrum = vec![Complex64::new(0.0, 0.0); plan.spectrum_len()];
            plan.forward(&input, &mut spectrum, &mut workspace)
                .expect("native real forward transform");
            spectrum
        }
        Mode::RealInverse => {
            let spectrum: Vec<_> = (0..plan.spectrum_len())
                .map(|index| spectrum_value(index, n, 0))
                .collect();
            let mut output = vec![0.0; n];
            plan.inverse(&spectrum, &mut output, &mut workspace)
                .expect("native real inverse transform");
            output
                .into_iter()
                .map(|value| Complex64::new(value, 0.0))
                .collect()
        }
        Mode::ComplexForward | Mode::ComplexInverse => unreachable!("real mode required"),
    }
}

fn dump_all() {
    for &n in BENCHMARK_LENGTHS {
        for mode in [
            Mode::ComplexForward,
            Mode::ComplexInverse,
            Mode::RealForward,
            Mode::RealInverse,
        ] {
            for (index, value) in output(mode, n).iter().enumerate() {
                println!(
                    "{},{},{},{:.17e},{:.17e}",
                    mode.name(),
                    n,
                    index,
                    value.re,
                    value.im
                );
            }
        }
    }
}

fn dump_native_all() {
    for &n in BENCHMARK_LENGTHS {
        for mode in [
            Mode::ComplexForward,
            Mode::ComplexInverse,
            Mode::RealForward,
            Mode::RealInverse,
        ] {
            let values = match mode {
                Mode::ComplexForward | Mode::ComplexInverse => native_complex_output(mode, n),
                Mode::RealForward | Mode::RealInverse => native_real_output(mode, n),
            };
            for (index, value) in values.iter().enumerate() {
                println!(
                    "{},{},{},{:.17e},{:.17e}",
                    mode.name(),
                    n,
                    index,
                    value.re,
                    value.im
                );
            }
        }
    }
}

fn benchmark(mode: Mode, n: usize, rows: usize, scalar: bool) {
    let rows = if n > 10000 { rows.min(4) } else { rows };
    let plan_time = plan_ns(
        n,
        matches!(mode, Mode::RealForward | Mode::RealInverse),
        scalar,
    );
    let scratch_time;
    let scratch_bytes;
    let fft_time;
    let packing_time;
    let call_time;
    let checksum;

    if matches!(mode, Mode::ComplexForward | Mode::ComplexInverse) {
        let plan = if scalar {
            ComplexPlan::new_scalar(n)
        } else {
            ComplexPlan::new(n)
        };
        let mut data = vec![Complex64::new(0.0, 0.0); rows * n];
        let mut scratch = vec![Complex64::new(0.0, 0.0); plan.scratch_len];
        scratch_bytes = scratch.len() * std::mem::size_of::<Complex64>();
        let fill = |data: &mut [Complex64]| {
            for row in 0..rows {
                for i in 0..n {
                    data[row * n + i] = complex_value(i, row);
                }
            }
        };
        let forward = matches!(mode, Mode::ComplexForward);
        fft_time = measure(|| {
            fill(&mut data);
            let start = Instant::now();
            for row in 0..rows {
                let range = row * n..(row + 1) * n;
                plan.execute_raw(&mut data[range], &mut scratch, forward);
            }
            let elapsed = start.elapsed();
            black_box(data.as_slice());
            elapsed
        })
        .as_nanos() as f64
            / rows as f64;
        call_time = measure(|| {
            fill(&mut data);
            let start = Instant::now();
            for row in 0..rows {
                let range = row * n..(row + 1) * n;
                plan.execute_raw(&mut data[range.clone()], &mut scratch, forward);
                if !forward {
                    plan.normalize_inverse(&mut data[range]);
                }
            }
            let elapsed = start.elapsed();
            black_box(data.as_slice());
            elapsed
        })
        .as_nanos() as f64
            / rows as f64;
        scratch_time = allocation_ns::<Complex64>(&[plan.scratch_len]);
        packing_time = 0.0;
        checksum = data[0].re + data[0].im;
    } else {
        let plan = if scalar {
            RealPlan::new_scalar(n)
        } else {
            RealPlan::new(n)
        };
        let bins = plan.bins();
        let transform_len = plan.complex.n;
        let mut transforms = vec![Complex64::new(0.0, 0.0); rows * transform_len];
        let mut scratch = vec![Complex64::new(0.0, 0.0); plan.scratch_len()];
        scratch_bytes =
            (transforms.len() / rows + scratch.len()) * std::mem::size_of::<Complex64>();
        scratch_time = allocation_ns::<Complex64>(&[transform_len, plan.scratch_len()]);
        if matches!(mode, Mode::RealForward) {
            let input: Vec<_> = (0..rows)
                .flat_map(|row| (0..n).map(move |i| real_value(i, row)))
                .collect();
            let mut spectra = vec![Complex64::new(0.0, 0.0); rows * bins];
            fft_time = measure(|| {
                for row in 0..rows {
                    let transform = &mut transforms[row * transform_len..(row + 1) * transform_len];
                    plan.prepare_forward(&input[row * n..(row + 1) * n], transform);
                }
                let start = Instant::now();
                for row in 0..rows {
                    let transform = &mut transforms[row * transform_len..(row + 1) * transform_len];
                    plan.complex.execute_raw(transform, &mut scratch, true);
                }
                let elapsed = start.elapsed();
                black_box(transforms.as_slice());
                elapsed
            })
            .as_nanos() as f64
                / rows as f64;
            packing_time = measure(|| {
                let start = Instant::now();
                for row in 0..rows {
                    let transform = &mut transforms[row * transform_len..(row + 1) * transform_len];
                    plan.prepare_forward(&input[row * n..(row + 1) * n], transform);
                    plan.finish_forward(transform, &mut spectra[row * bins..(row + 1) * bins]);
                }
                let elapsed = start.elapsed();
                black_box(spectra.as_slice());
                elapsed
            })
            .as_nanos() as f64
                / rows as f64;
            call_time = measure(|| {
                let start = Instant::now();
                for row in 0..rows {
                    let transform = &mut transforms[row * transform_len..(row + 1) * transform_len];
                    plan.prepare_forward(&input[row * n..(row + 1) * n], transform);
                    plan.complex.execute_raw(transform, &mut scratch, true);
                    plan.finish_forward(transform, &mut spectra[row * bins..(row + 1) * bins]);
                }
                let elapsed = start.elapsed();
                black_box(spectra.as_slice());
                elapsed
            })
            .as_nanos() as f64
                / rows as f64;
            checksum = spectra[0].re + spectra[0].im;
        } else {
            let spectra: Vec<_> = (0..rows)
                .flat_map(|row| (0..bins).map(move |k| spectrum_value(k, n, row)))
                .collect();
            let mut output = vec![0.0; rows * n];
            fft_time = measure(|| {
                for row in 0..rows {
                    let transform = &mut transforms[row * transform_len..(row + 1) * transform_len];
                    plan.prepare_inverse(&spectra[row * bins..(row + 1) * bins], transform);
                }
                let start = Instant::now();
                for row in 0..rows {
                    let transform = &mut transforms[row * transform_len..(row + 1) * transform_len];
                    plan.complex.execute_raw(transform, &mut scratch, false);
                }
                let elapsed = start.elapsed();
                black_box(transforms.as_slice());
                elapsed
            })
            .as_nanos() as f64
                / rows as f64;
            packing_time = measure(|| {
                let start = Instant::now();
                for row in 0..rows {
                    let transform = &mut transforms[row * transform_len..(row + 1) * transform_len];
                    plan.prepare_inverse(&spectra[row * bins..(row + 1) * bins], transform);
                    plan.finish_inverse(transform, &mut output[row * n..(row + 1) * n]);
                }
                let elapsed = start.elapsed();
                black_box(output.as_slice());
                elapsed
            })
            .as_nanos() as f64
                / rows as f64;
            call_time = measure(|| {
                let start = Instant::now();
                for row in 0..rows {
                    let transform = &mut transforms[row * transform_len..(row + 1) * transform_len];
                    plan.prepare_inverse(&spectra[row * bins..(row + 1) * bins], transform);
                    plan.complex.execute_raw(transform, &mut scratch, false);
                    plan.finish_inverse(transform, &mut output[row * n..(row + 1) * n]);
                }
                let elapsed = start.elapsed();
                black_box(output.as_slice());
                elapsed
            })
            .as_nanos() as f64
                / rows as f64;
            checksum = output[0];
        }
    }

    println!(
        "{},{},{:.9},{:.9},{},{:.9},{:.9},{:.9},{:.9},{},{}",
        mode.name(),
        n,
        plan_time,
        scratch_time,
        scratch_bytes,
        fft_time,
        packing_time,
        call_time,
        call_time * rows as f64,
        rows,
        checksum
    );
}

fn benchmark_sht_workloads() {
    println!(
        "mode,n,plan_ns,scratch_ns,scratch_bytes,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum"
    );
    for &(n, rows) in SHT_WORKLOADS {
        for mode in [
            Mode::ComplexForward,
            Mode::ComplexInverse,
            Mode::RealForward,
            Mode::RealInverse,
        ] {
            benchmark(mode, n, rows, false);
        }
    }
}

fn benchmark_sizes(sizes: Vec<usize>, scalar: bool) {
    println!(
        "mode,n,plan_ns,scratch_ns,scratch_bytes,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum"
    );
    for n in sizes {
        for mode in [
            Mode::ComplexForward,
            Mode::ComplexInverse,
            Mode::RealForward,
            Mode::RealInverse,
        ] {
            benchmark(mode, n, 1, scalar);
            benchmark(mode, n, BATCH_COUNT, scalar);
        }
    }
}

fn benchmark_native_complex(mode: Mode, n: usize, rows: usize) {
    let rows = if n > 10000 { rows.min(4) } else { rows };
    let start = Instant::now();
    for _ in 0..20 {
        black_box(
            ComplexFftPlan::<f64>::new(n)
                .expect("native FFT plan")
                .len(),
        );
    }
    let plan_ns = start.elapsed().as_nanos() as f64 / 20.0;

    let plan = ComplexFftPlan::<f64>::new(n).expect("native FFT plan");
    let start = Instant::now();
    for _ in 0..20 {
        black_box(plan.workspace().expect("native FFT workspace").len());
    }
    let scratch_ns = start.elapsed().as_nanos() as f64 / 20.0;
    let mut workspace = plan.workspace().expect("native FFT workspace");
    let workspace_bytes = workspace
        .scratch_bytes()
        .expect("native complex workspace byte count");
    let mut data = vec![Complex64::new(0.0, 0.0); rows * n];
    let fill = |data: &mut [Complex64]| {
        for row in 0..rows {
            for index in 0..n {
                data[row * n + index] = complex_value(index, row);
            }
        }
    };
    let forward = matches!(mode, Mode::ComplexForward);
    let call_time = measure(|| {
        fill(&mut data);
        let start = Instant::now();
        for row in 0..rows {
            let values = &mut data[row * n..(row + 1) * n];
            if forward {
                plan.forward(values, &mut workspace)
                    .expect("native forward FFT");
            } else {
                plan.inverse(values, &mut workspace)
                    .expect("native inverse FFT");
            }
        }
        let elapsed = start.elapsed();
        black_box(data.as_slice());
        elapsed
    })
    .as_nanos() as f64
        / rows as f64;
    let checksum = data[0].re + data[0].im;
    println!(
        "{},{},{:.9},{:.9},{},{:.9},{:.9},{},{}",
        mode.name(),
        n,
        plan_ns,
        scratch_ns,
        workspace_bytes,
        call_time,
        call_time * rows as f64,
        rows,
        checksum
    );
}

fn benchmark_native_real(mode: Mode, n: usize, rows: usize) {
    let start = Instant::now();
    for _ in 0..20 {
        black_box(RealFftPlan::<f64>::new(n).expect("native real plan").len());
    }
    let plan_time = start.elapsed().as_nanos() as f64 / 20.0;
    let plan = RealFftPlan::<f64>::new(n).expect("native real plan");
    let start = Instant::now();
    for _ in 0..20 {
        black_box(plan.workspace().expect("native real workspace").len());
    }
    let workspace_ns = start.elapsed().as_nanos() as f64 / 20.0;
    let mut workspace = plan.workspace().expect("native real workspace");
    let workspace_bytes = workspace
        .scratch_bytes()
        .expect("native real workspace byte count");
    let bins = plan.spectrum_len();
    let (operation_ns, checksum) = match mode {
        Mode::RealForward => {
            let input: Vec<_> = (0..rows)
                .flat_map(|row| (0..n).map(move |index| real_value(index, row)))
                .collect();
            let mut spectrum = vec![Complex64::new(0.0, 0.0); rows * bins];
            let operation_ns = measure(|| {
                let start = Instant::now();
                for row in 0..rows {
                    plan.forward(
                        &input[row * n..(row + 1) * n],
                        &mut spectrum[row * bins..(row + 1) * bins],
                        &mut workspace,
                    )
                    .expect("native real forward transform");
                }
                let elapsed = start.elapsed();
                black_box(spectrum.as_slice());
                elapsed
            })
            .as_nanos() as f64
                / rows as f64;
            black_box(spectrum.as_slice());
            let checksum = spectrum[0].re + spectrum[0].im;
            (operation_ns, checksum)
        }
        Mode::RealInverse => {
            let input: Vec<_> = (0..rows)
                .flat_map(|row| (0..bins).map(move |index| spectrum_value(index, n, row)))
                .collect();
            let mut output = vec![0.0; rows * n];
            let operation_ns = measure(|| {
                let start = Instant::now();
                for row in 0..rows {
                    plan.inverse(
                        &input[row * bins..(row + 1) * bins],
                        &mut output[row * n..(row + 1) * n],
                        &mut workspace,
                    )
                    .expect("native real inverse transform");
                }
                let elapsed = start.elapsed();
                black_box(output.as_slice());
                elapsed
            })
            .as_nanos() as f64
                / rows as f64;
            black_box(output.as_slice());
            (operation_ns, output[0])
        }
        Mode::ComplexForward | Mode::ComplexInverse => unreachable!("real mode required"),
    };
    println!(
        "{},{},{:.9},{:.9},{},{:.9},{:.9},{},{}",
        mode.name(),
        n,
        plan_time,
        workspace_ns,
        workspace_bytes,
        operation_ns,
        operation_ns * rows as f64,
        rows,
        checksum
    );
}

fn benchmark_batch_api_sample(
    process_sample: &str,
    mode: &'static str,
    n: usize,
    rows: usize,
    mut execute: impl FnMut(bool) -> (Duration, f64),
) {
    assert!(rows > 0, "batch API benchmark requires at least one row");
    black_box(execute(false));
    black_box(execute(true));
    for pair in 0..SAMPLE_COUNT {
        let (single, batch, single_checksum, batch_checksum, first) = if pair.is_multiple_of(2) {
            let (single, single_checksum) = execute(false);
            let (batch, batch_checksum) = execute(true);
            (single, batch, single_checksum, batch_checksum, "single")
        } else {
            let (batch, batch_checksum) = execute(true);
            let (single, single_checksum) = execute(false);
            (single, batch, single_checksum, batch_checksum, "batch")
        };
        let tolerance = 32.0 * f64::EPSILON * single_checksum.abs().max(1.0);
        assert!(
            (single_checksum - batch_checksum).abs() <= tolerance,
            "{mode} batch checksum mismatch: {single_checksum} != {batch_checksum}"
        );
        println!(
            "{process_sample},{mode},{n},{rows},{pair},{first},{:.9},{:.9},{single_checksum:.17e},{batch_checksum:.17e}",
            single.as_nanos() as f64 / rows as f64,
            batch.as_nanos() as f64 / rows as f64,
        );
    }
}

fn benchmark_native_batch_api(n: usize, rows: usize, process_sample: &str) {
    let complex_plan = ComplexFftPlan::<f64>::new(n).expect("native complex plan");
    let mut complex_workspace = complex_plan.workspace().expect("native complex workspace");
    let complex_source: Vec<_> = (0..rows)
        .flat_map(|row| (0..n).map(move |index| complex_value(index, row)))
        .collect();
    let mut complex_values = vec![Complex64::new(0.0, 0.0); rows * n];
    for (mode, inverse) in [("c2c_forward", false), ("c2c_inverse", true)] {
        let mut execute = |batch: bool| {
            complex_values.copy_from_slice(&complex_source);
            let start = Instant::now();
            if batch {
                if inverse {
                    complex_plan
                        .inverse_batch(&mut complex_values, &mut complex_workspace)
                        .expect("native inverse batch");
                } else {
                    complex_plan
                        .forward_batch(&mut complex_values, &mut complex_workspace)
                        .expect("native forward batch");
                }
            } else {
                for row in complex_values.chunks_exact_mut(n) {
                    if inverse {
                        complex_plan
                            .inverse(row, &mut complex_workspace)
                            .expect("native inverse row");
                    } else {
                        complex_plan
                            .forward(row, &mut complex_workspace)
                            .expect("native forward row");
                    }
                }
            }
            let elapsed = start.elapsed();
            black_box(complex_values.as_slice());
            let checksum = complex_values
                .iter()
                .fold(0.0, |sum, value| sum + 0.17 * value.re + 0.11 * value.im);
            (elapsed, checksum)
        };
        benchmark_batch_api_sample(process_sample, mode, n, rows, &mut execute);
    }

    let real_plan = RealFftPlan::<f64>::new(n).expect("native real plan");
    let mut real_workspace = real_plan.workspace().expect("native real workspace");
    let bins = real_plan.spectrum_len();
    let real_input: Vec<_> = (0..rows)
        .flat_map(|row| (0..n).map(move |index| real_value(index, row)))
        .collect();
    let mut real_spectrum = vec![Complex64::new(0.0, 0.0); rows * bins];
    let mut forward = |batch: bool| {
        let start = Instant::now();
        if batch {
            real_plan
                .forward_batch(&real_input, &mut real_spectrum, &mut real_workspace)
                .expect("native real forward batch");
        } else {
            for row in 0..rows {
                real_plan
                    .forward(
                        &real_input[row * n..(row + 1) * n],
                        &mut real_spectrum[row * bins..(row + 1) * bins],
                        &mut real_workspace,
                    )
                    .expect("native real forward row");
            }
        }
        let elapsed = start.elapsed();
        black_box(real_spectrum.as_slice());
        let checksum = real_spectrum
            .iter()
            .fold(0.0, |sum, value| sum + 0.17 * value.re + 0.11 * value.im);
        (elapsed, checksum)
    };
    benchmark_batch_api_sample(process_sample, "r2c", n, rows, &mut forward);

    let real_spectrum_input: Vec<_> = (0..rows)
        .flat_map(|row| (0..bins).map(move |index| spectrum_value(index, n, row)))
        .collect();
    let mut real_output = vec![0.0; rows * n];
    let mut inverse = |batch: bool| {
        let start = Instant::now();
        if batch {
            real_plan
                .inverse_batch(&real_spectrum_input, &mut real_output, &mut real_workspace)
                .expect("native real inverse batch");
        } else {
            for row in 0..rows {
                real_plan
                    .inverse(
                        &real_spectrum_input[row * bins..(row + 1) * bins],
                        &mut real_output[row * n..(row + 1) * n],
                        &mut real_workspace,
                    )
                    .expect("native real inverse row");
            }
        }
        let elapsed = start.elapsed();
        black_box(real_output.as_slice());
        let checksum = real_output.iter().sum();
        (elapsed, checksum)
    };
    benchmark_batch_api_sample(process_sample, "c2r", n, rows, &mut inverse);
}

fn profile_native_real_inverse(n: usize, high_dynamic_range: bool, iterations: usize) {
    let plan = RealFftPlan::<f64>::new(n).expect("native real plan");
    let mut workspace = plan.workspace().expect("native real workspace");
    let mut spectrum: Vec<_> = (0..plan.spectrum_len())
        .map(|index| spectrum_value(index, n, 0))
        .collect();
    if high_dynamic_range {
        spectrum.fill(Complex64::new(f64::MAX / 8.0, 0.0));
    }
    let mut output = vec![0.0; n];
    for _ in 0..3 {
        plan.inverse(&spectrum, &mut output, &mut workspace)
            .expect("native real inverse transform");
    }
    let start = Instant::now();
    for _ in 0..iterations {
        plan.inverse(&spectrum, &mut output, &mut workspace)
            .expect("native real inverse transform");
    }
    let elapsed = start.elapsed();
    black_box(output.as_slice());
    assert!(output.iter().all(|value| value.is_finite()));
    let checksum: f64 = output.iter().sum();
    black_box(checksum);
    println!(
        "c2r_profile,{},{},{:.3},{:.9}",
        n,
        if high_dynamic_range { "high" } else { "normal" },
        elapsed.as_nanos() as f64 / iterations as f64,
        checksum
    );
}

fn profile_native_complex(n: usize, forward: bool, iterations: usize) {
    let plan = ComplexFftPlan::<f64>::new(n).expect("native complex plan");
    let mut workspace = plan.workspace().expect("native complex workspace");
    let input: Vec<_> = (0..n).map(|index| complex_value(index, 0)).collect();
    let mut values = vec![Complex64::new(0.0, 0.0); n];
    for _ in 0..3 {
        values.copy_from_slice(&input);
        if forward {
            plan.forward(&mut values, &mut workspace)
                .expect("native complex forward transform");
        } else {
            plan.inverse(&mut values, &mut workspace)
                .expect("native complex inverse transform");
        }
    }
    let start = Instant::now();
    for _ in 0..iterations {
        values.copy_from_slice(&input);
        if forward {
            plan.forward(&mut values, &mut workspace)
                .expect("native complex forward transform");
        } else {
            plan.inverse(&mut values, &mut workspace)
                .expect("native complex inverse transform");
        }
    }
    let elapsed = start.elapsed();
    black_box(values.as_slice());
    assert!(
        values
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
    );
    let checksum = values
        .iter()
        .fold(0.0, |sum, value| sum + value.re * 0.17 + value.im * 0.11);
    black_box(checksum);
    println!(
        "c2c_profile,{},{},{:.3},{:.9}",
        n,
        if forward { "forward" } else { "inverse" },
        elapsed.as_nanos() as f64 / iterations as f64,
        checksum
    );
}

fn main() {
    let mut args = std::env::args().skip(1).filter(|arg| arg != "--bench");
    match args.next().as_deref() {
        Some("dump-all") => dump_all(),
        Some("dump-native-all") => dump_native_all(),
        Some("dump-native") => {
            let mode = Mode::parse(&args.next().expect("mode is required"));
            let n = args.next().expect("length is required").parse().unwrap();
            let values = match mode {
                Mode::ComplexForward | Mode::ComplexInverse => native_complex_output(mode, n),
                Mode::RealForward | Mode::RealInverse => native_real_output(mode, n),
            };
            for (index, value) in values.iter().enumerate() {
                println!(
                    "{},{},{},{:.17e},{:.17e}",
                    mode.name(),
                    n,
                    index,
                    value.re,
                    value.im
                );
            }
        }
        Some("dump") => {
            let mode = Mode::parse(&args.next().expect("mode is required"));
            let n = args.next().expect("length is required").parse().unwrap();
            for (index, value) in output(mode, n).iter().enumerate() {
                println!(
                    "{},{},{},{:.17e},{:.17e}",
                    mode.name(),
                    n,
                    index,
                    value.re,
                    value.im
                );
            }
        }
        Some("bench-one") => {
            let mode = Mode::parse(&args.next().expect("mode is required"));
            let n = args.next().expect("length is required").parse().unwrap();
            if let Some(rows) = args.next() {
                println!(
                    "mode,n,plan_ns,scratch_ns,scratch_bytes,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum"
                );
                benchmark(
                    mode,
                    n,
                    rows.parse().expect("row count must be an integer"),
                    false,
                );
            } else {
                println!(
                    "mode,n,plan_ns,scratch_ns,scratch_bytes,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum"
                );
                benchmark(mode, n, 1, false);
                benchmark(mode, n, BATCH_COUNT, false);
            }
        }
        Some("bench-sht") => {
            benchmark_sht_workloads();
        }
        Some("bench-real-multipass") => {
            let lengths: Vec<String> = args.collect();
            let lengths = if lengths.is_empty() {
                [64, 128, 256, 512, 1024, 2048, 4096]
                    .into_iter()
                    .map(|len| len.to_string())
                    .collect::<Vec<_>>()
            } else {
                lengths
            };
            real_multipass_prototype::bench(lengths.into_iter());
        }
        Some("bench-native") => {
            println!(
                "mode,n,plan_ns,workspace_ns,workspace_bytes,public_operation_ns,batch_operation_ns,batch_count,checksum"
            );
            let sizes: Vec<usize> = args
                .map(|value| value.parse().expect("length must be an integer"))
                .collect();
            let sizes = if sizes.is_empty() {
                BENCHMARK_LENGTHS.to_vec()
            } else {
                sizes
            };
            for n in sizes {
                for mode in [Mode::ComplexForward, Mode::ComplexInverse] {
                    benchmark_native_complex(mode, n, 1);
                    benchmark_native_complex(mode, n, BATCH_COUNT);
                }
                for mode in [Mode::RealForward, Mode::RealInverse] {
                    benchmark_native_real(mode, n, 1);
                    benchmark_native_real(mode, n, BATCH_COUNT);
                }
            }
        }
        Some("bench-native-real") => {
            println!(
                "mode,n,plan_ns,workspace_ns,workspace_bytes,public_operation_ns,batch_operation_ns,batch_count,checksum"
            );
            let n = args
                .next()
                .expect("length is required")
                .parse()
                .expect("length must be an integer");
            let row_counts: Vec<usize> = args
                .map(|value| value.parse().expect("row count must be an integer"))
                .collect();
            let row_counts = if row_counts.is_empty() {
                vec![1, BATCH_COUNT]
            } else {
                row_counts
            };
            for mode in [Mode::RealForward, Mode::RealInverse] {
                for &rows in &row_counts {
                    benchmark_native_real(mode, n, rows);
                }
            }
        }
        Some("bench-native-c2c") => {
            println!(
                "mode,n,plan_ns,workspace_ns,workspace_bytes,public_operation_ns,batch_operation_ns,batch_count,checksum"
            );
            let n = args
                .next()
                .expect("length is required")
                .parse()
                .expect("length must be an integer");
            let row_counts: Vec<usize> = args
                .map(|value| value.parse().expect("row count must be an integer"))
                .collect();
            let row_counts = if row_counts.is_empty() {
                vec![1, BATCH_COUNT]
            } else {
                row_counts
            };
            for mode in [Mode::ComplexForward, Mode::ComplexInverse] {
                for &rows in &row_counts {
                    benchmark_native_complex(mode, n, rows);
                }
            }
        }
        Some("bench-native-batch-api") => {
            println!(
                "process_sample,mode,n,rows,pair,first,single_ns_per_transform,batch_api_ns_per_transform,single_checksum,batch_checksum"
            );
            let n = args
                .next()
                .expect("length is required")
                .parse()
                .expect("length must be an integer");
            let row_counts: Vec<usize> = args
                .map(|value| value.parse().expect("row count must be an integer"))
                .collect();
            let row_counts = if row_counts.is_empty() {
                vec![1, BATCH_COUNT]
            } else {
                row_counts
            };
            let process_sample =
                std::env::var("DUCCFFT_PROCESS_SAMPLE").unwrap_or_else(|_| String::from("0"));
            for rows in row_counts {
                benchmark_native_batch_api(n, rows, &process_sample);
            }
        }
        Some("bench-profile-real-inverse") => {
            println!("mode,n,input,operation_ns,checksum");
            let n = args
                .next()
                .expect("length is required")
                .parse()
                .expect("length must be an integer");
            let range = args.next().expect("input range is required");
            let iterations = args
                .next()
                .map(|value| value.parse().expect("iteration count must be an integer"))
                .unwrap_or(100);
            assert!(iterations > 0, "iteration count must be positive");
            assert!(matches!(range.as_str(), "normal" | "high"));
            profile_native_real_inverse(n, range == "high", iterations);
        }
        Some("bench-profile-complex") => {
            println!("mode,n,direction,operation_ns,checksum");
            let n = args
                .next()
                .expect("length is required")
                .parse()
                .expect("length must be an integer");
            let direction = args.next().expect("direction is required");
            let iterations = args
                .next()
                .map(|value| value.parse().expect("iteration count must be an integer"))
                .unwrap_or(100);
            assert!(iterations > 0, "iteration count must be positive");
            assert!(matches!(direction.as_str(), "forward" | "inverse"));
            profile_native_complex(n, direction == "forward", iterations);
        }
        Some("bench") | Some("bench-scalar") => {
            let scalar = std::env::args().any(|arg| arg == "bench-scalar");
            let sizes: Vec<usize> = args
                .map(|value| value.parse().expect("length must be an integer"))
                .collect();
            let sizes = if sizes.is_empty() {
                BENCHMARK_LENGTHS.to_vec()
            } else {
                sizes
            };
            benchmark_sizes(sizes, scalar);
        }
        _ => {
            eprintln!(
                "usage: rustfft-bench dump-all | dump-native-all | dump MODE N | dump-native MODE N | bench [N ...] | bench-scalar [N ...] | bench-sht | bench-real-multipass [N ...] | bench-native [N ...] | bench-native-real N [BATCH ...] | bench-native-c2c N [BATCH ...] | bench-native-batch-api N [BATCH ...] | bench-profile-real-inverse N normal|high [ITERATIONS] | bench-profile-complex N forward|inverse [ITERATIONS]"
            );
            eprintln!("no benchmark was run; use `bench` to start the workload");
        }
    }
}
