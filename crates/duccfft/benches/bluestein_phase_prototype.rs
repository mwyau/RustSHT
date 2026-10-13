// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// Separates the reusable Bluestein convolution phases for selected complex
// primes. The phase code uses the same chirp identities and Rust multipass
// convolution plan as the production executor.

use std::hint::black_box;
use std::time::{Duration, Instant};

use duccfft::{ComplexFftPlan, ComplexFftWorkspace};
use num_complex::Complex64;

type C = Complex64;
const ZERO: C = C::new(0.0, 0.0);
const SAMPLE_COUNT: usize = 7;

struct ProfilePlan {
    len: usize,
    convolution_len: usize,
    chirp: Vec<C>,
    kernel_spectrum: Vec<C>,
    convolution: ComplexFftPlan<f64>,
    convolution_workspace: ComplexFftWorkspace<f64>,
    buffer: Vec<C>,
    output: Vec<C>,
    full_plan: ComplexFftPlan<f64>,
    full_workspace: ComplexFftWorkspace<f64>,
}

impl ProfilePlan {
    fn new(len: usize) -> Self {
        let minimum = 2 * len - 1;
        let convolution_len = good_size(minimum);
        let convolution = ComplexFftPlan::<f64>::new(convolution_len).expect("convolution plan");
        let mut convolution_workspace = convolution.workspace().expect("convolution workspace");
        let mut chirp = vec![ZERO; len];
        let mut kernel_spectrum = vec![ZERO; convolution_len];
        let buffer = vec![ZERO; convolution_len];
        for (index, value) in chirp.iter_mut().enumerate() {
            let phase = std::f64::consts::PI * (index * index) as f64 / len as f64;
            *value = C::from_polar(1.0, -phase);
        }
        kernel_spectrum[0] = C::new(1.0, 0.0);
        for index in 1..len {
            let value = chirp[index].conj();
            kernel_spectrum[index] = value;
            kernel_spectrum[convolution_len - index] = value;
        }
        convolution
            .forward(&mut kernel_spectrum, &mut convolution_workspace)
            .expect("kernel forward");
        let full_plan = ComplexFftPlan::<f64>::new(len).expect("full plan");
        let full_workspace = full_plan.workspace().expect("full workspace");
        Self {
            len,
            convolution_len,
            chirp,
            kernel_spectrum,
            convolution,
            convolution_workspace,
            buffer,
            output: vec![ZERO; len],
            full_plan,
            full_workspace,
        }
    }

    fn scratch_bytes(&self) -> usize {
        self.full_workspace
            .scratch_bytes()
            .expect("full workspace scratch size")
    }

    fn prepare_chirped_input(&mut self, input: &[C]) {
        for (index, value) in self.buffer.iter_mut().take(self.len).enumerate() {
            *value = input[index] * self.chirp[index];
        }
        self.buffer[self.len..].fill(ZERO);
    }

    fn phases(&mut self, input: &[C]) -> [f64; 5] {
        [
            measure_phase(self, input, Phase::ChirpPack),
            measure_phase(self, input, Phase::ForwardFft),
            measure_phase(self, input, Phase::SpectralProduct),
            measure_phase(self, input, Phase::InverseFft),
            measure_phase(self, input, Phase::FinalChirp),
        ]
    }

    fn execute(&mut self, input: &[C]) {
        self.prepare_chirped_input(input);
        self.convolution
            .forward(&mut self.buffer, &mut self.convolution_workspace)
            .expect("forward convolution FFT");
        for (value, &kernel) in self.buffer.iter_mut().zip(&self.kernel_spectrum) {
            *value *= kernel;
        }
        self.convolution
            .inverse(&mut self.buffer, &mut self.convolution_workspace)
            .expect("inverse convolution FFT");
        for index in 0..self.len {
            self.output[index] = self.buffer[index] * self.chirp[index];
        }
    }
}

fn good_size(minimum: usize) -> usize {
    (minimum..)
        .find(|&candidate| {
            let mut remaining = candidate;
            for factor in [2, 3, 5, 7, 11] {
                while remaining.is_multiple_of(factor) {
                    remaining /= factor;
                }
            }
            remaining == 1
        })
        .expect("smooth convolution length")
}

fn input_value(index: usize) -> C {
    let x = (index + 1) as f64;
    C::new((0.173 * x).sin(), (0.113 * (x + 1.0)).cos())
}

fn median(mut values: Vec<Duration>) -> Duration {
    values.sort_unstable();
    values[values.len() / 2]
}

fn measure(mut operation: impl FnMut()) -> f64 {
    operation();
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let start = Instant::now();
        operation();
        samples.push(start.elapsed());
    }
    median(samples).as_nanos() as f64
}

#[derive(Clone, Copy)]
enum Phase {
    ChirpPack,
    ForwardFft,
    SpectralProduct,
    InverseFft,
    FinalChirp,
}

fn measure_phase(plan: &mut ProfilePlan, input: &[C], phase: Phase) -> f64 {
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for sample in 0..=SAMPLE_COUNT {
        match phase {
            Phase::ChirpPack => {}
            Phase::ForwardFft => plan.prepare_chirped_input(input),
            Phase::SpectralProduct | Phase::InverseFft | Phase::FinalChirp => {
                plan.prepare_chirped_input(input);
                plan.convolution
                    .forward(&mut plan.buffer, &mut plan.convolution_workspace)
                    .expect("forward convolution FFT setup");
                if matches!(phase, Phase::InverseFft | Phase::FinalChirp) {
                    for (value, &kernel) in plan.buffer.iter_mut().zip(&plan.kernel_spectrum) {
                        *value *= kernel;
                    }
                }
                if matches!(phase, Phase::FinalChirp) {
                    plan.convolution
                        .inverse(&mut plan.buffer, &mut plan.convolution_workspace)
                        .expect("inverse convolution FFT setup");
                }
            }
        }
        let start = Instant::now();
        match phase {
            Phase::ChirpPack => plan.prepare_chirped_input(input),
            Phase::ForwardFft => plan
                .convolution
                .forward(&mut plan.buffer, &mut plan.convolution_workspace)
                .expect("forward convolution FFT"),
            Phase::SpectralProduct => {
                for (value, &kernel) in plan.buffer.iter_mut().zip(&plan.kernel_spectrum) {
                    *value *= kernel;
                }
            }
            Phase::InverseFft => plan
                .convolution
                .inverse(&mut plan.buffer, &mut plan.convolution_workspace)
                .expect("inverse convolution FFT"),
            Phase::FinalChirp => {
                for index in 0..plan.len {
                    plan.output[index] = plan.buffer[index] * plan.chirp[index];
                }
            }
        }
        match phase {
            Phase::FinalChirp => black_box(plan.output.as_slice()),
            _ => black_box(plan.buffer.as_slice()),
        };
        let elapsed = start.elapsed();
        if sample > 0 {
            samples.push(elapsed);
        }
    }
    median(samples).as_nanos() as f64
}

fn main() {
    let lengths: Vec<usize> = std::env::args()
        .skip(1)
        .filter(|arg| arg != "--bench")
        .map(|arg| arg.parse().expect("length must be an integer"))
        .collect();
    if lengths.is_empty() {
        return;
    }
    println!(
        "n,convolution_len,plan_ns,workspace_ns,scratch_bytes,chirp_pack_ns,forward_fft_ns,spectral_product_ns,inverse_fft_ns,final_chirp_ns,phase_sum_ns,prototype_operation_ns,ratio_phase_sum_prototype,max_abs_error,checksum"
    );
    for len in lengths {
        let plan_ns = measure(|| {
            black_box(ComplexFftPlan::<f64>::new(len).expect("full plan").len());
        });
        let mut profile = ProfilePlan::new(len);
        let workspace_ns = measure(|| {
            black_box(profile.full_plan.workspace().expect("full workspace").len());
        });
        let scratch_bytes = profile.scratch_bytes();
        let input: Vec<_> = (0..len).map(input_value).collect();
        profile.execute(&input);
        let mut expected = input.clone();
        profile
            .full_plan
            .forward(&mut expected, &mut profile.full_workspace)
            .expect("full forward");
        let max_error = profile
            .output
            .iter()
            .zip(&expected)
            .map(|(&actual, &reference)| (actual - reference).norm())
            .fold(0.0_f64, f64::max);
        assert!(
            profile
                .output
                .iter()
                .chain(expected.iter())
                .all(|value| value.re.is_finite() && value.im.is_finite()),
            "nonfinite result at n={len}"
        );
        assert!(
            max_error < 2.0e-10 * len as f64,
            "n={len}, error={max_error}"
        );

        let phases = profile.phases(&input);
        let phase_sum: f64 = phases.iter().sum();
        let prototype_operation_ns = measure(|| {
            profile.execute(&input);
            black_box(profile.output.as_slice());
        });
        println!(
            "{len},{},{plan_ns:.3},{workspace_ns:.3},{scratch_bytes},{:.3},{:.3},{:.3},{:.3},{:.3},{phase_sum:.3},{prototype_operation_ns:.3},{:.4},{max_error:.9e},{:.9}",
            profile.convolution_len,
            phases[0],
            phases[1],
            phases[2],
            phases[3],
            phases[4],
            phase_sum / prototype_operation_ns,
            profile.output[0].re + profile.output[0].im
        );
    }
}
