// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

use std::hint::black_box;
use std::time::{Duration, Instant};

use duccfft::{ComplexFftPlan, RealFftPlan};
use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};

const BATCH_COUNT: usize = 32;
const SAMPLE_COUNT: usize = 7;

const BENCHMARK_LENGTHS: &[usize] = &[
    31, 32, 33, 64, 107, 109, 113, 127, 128, 131, 137, 241, 257, 511, 512, 683, 998, 1000, 1001,
    1002, 1008, 1013, 1023, 1024, 1025, 1026, 2047, 2048, 2049, 4097, 10000, 10001, 10008,
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
        let forward = planner.plan_fft_forward(n);
        let inverse = planner.plan_fft_inverse(n);
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
        let transform_len = if n.is_multiple_of(2) { n / 2 } else { n };
        let complex = ComplexPlan::new(transform_len);
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

fn plan_ns(n: usize, real: bool) -> f64 {
    let repeats = 20;
    let start = Instant::now();
    for _ in 0..repeats {
        black_box(if real {
            RealPlan::new(n).scratch_len()
        } else {
            ComplexPlan::new(n).scratch_len
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

fn benchmark(mode: Mode, n: usize, rows: usize) {
    let rows = if n > 10000 { rows.min(4) } else { rows };
    let plan_time = plan_ns(n, matches!(mode, Mode::RealForward | Mode::RealInverse));
    let scratch_time;
    let fft_time;
    let packing_time;
    let call_time;
    let checksum;

    if matches!(mode, Mode::ComplexForward | Mode::ComplexInverse) {
        let plan = ComplexPlan::new(n);
        let mut data = vec![Complex64::new(0.0, 0.0); rows * n];
        let mut scratch = vec![Complex64::new(0.0, 0.0); plan.scratch_len];
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
            start.elapsed()
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
            start.elapsed()
        })
        .as_nanos() as f64
            / rows as f64;
        scratch_time = allocation_ns::<Complex64>(&[plan.scratch_len]);
        packing_time = 0.0;
        checksum = data[0].re + data[0].im;
    } else {
        let plan = RealPlan::new(n);
        let bins = plan.bins();
        let transform_len = plan.complex.n;
        let mut transforms = vec![Complex64::new(0.0, 0.0); rows * transform_len];
        let mut scratch = vec![Complex64::new(0.0, 0.0); plan.scratch_len()];
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
                start.elapsed()
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
                start.elapsed()
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
                start.elapsed()
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
                start.elapsed()
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
                start.elapsed()
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
                start.elapsed()
            })
            .as_nanos() as f64
                / rows as f64;
            checksum = output[0];
        }
    }

    println!(
        "{},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{},{}",
        mode.name(),
        n,
        plan_time,
        scratch_time,
        fft_time,
        packing_time,
        call_time,
        call_time * rows as f64,
        rows,
        checksum
    );
}

fn benchmark_sht_workloads() {
    println!("mode,n,plan_ns,scratch_ns,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum");
    for &(n, rows) in SHT_WORKLOADS {
        for mode in [
            Mode::ComplexForward,
            Mode::ComplexInverse,
            Mode::RealForward,
            Mode::RealInverse,
        ] {
            benchmark(mode, n, rows);
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
        start.elapsed()
    })
    .as_nanos() as f64
        / rows as f64;
    let checksum = data[0].re + data[0].im;
    println!(
        "{},{},{:.9},{:.9},{:.9},{:.9},{},{}",
        mode.name(),
        n,
        plan_ns,
        scratch_ns,
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
        "{},{},{:.9},{:.9},{:.9},{:.9},{},{}",
        mode.name(),
        n,
        plan_time,
        workspace_ns,
        operation_ns,
        operation_ns * rows as f64,
        rows,
        checksum
    );
}

fn main() {
    let mut args = std::env::args().skip(1).filter(|arg| arg != "--bench");
    match args.next().as_deref() {
        Some("dump-all") => dump_all(),
        Some("dump-native-all") => dump_native_all(),
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
                    "mode,n,plan_ns,scratch_ns,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum"
                );
                benchmark(mode, n, rows.parse().expect("row count must be an integer"));
            } else {
                println!(
                    "mode,n,plan_ns,scratch_ns,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum"
                );
                benchmark(mode, n, 1);
                benchmark(mode, n, BATCH_COUNT);
            }
        }
        Some("bench-sht") => {
            benchmark_sht_workloads();
        }
        Some("bench-native") => {
            println!(
                "mode,n,plan_ns,workspace_ns,public_operation_ns,batch_operation_ns,batch_count,checksum"
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
                "mode,n,plan_ns,workspace_ns,public_operation_ns,batch_operation_ns,batch_count,checksum"
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
        Some("bench") => {
            println!(
                "mode,n,plan_ns,scratch_ns,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum"
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
                for mode in [
                    Mode::ComplexForward,
                    Mode::ComplexInverse,
                    Mode::RealForward,
                    Mode::RealInverse,
                ] {
                    benchmark(mode, n, 1);
                    benchmark(mode, n, BATCH_COUNT);
                }
            }
        }
        _ => {
            eprintln!(
                "usage: rustfft-bench dump-all | dump-native-all | dump MODE N | bench [N ...] | bench-sht | bench-native [N ...] | bench-native-real N [BATCH ...]"
            );
            eprintln!("no benchmark was run; use `bench` to start the workload");
        }
    }
}
