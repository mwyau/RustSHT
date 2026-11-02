use super::super::*;
use std::hint::black_box;
use std::time::Instant;

const LANES: usize = 64;
const REPETITIONS: usize = 100_000;
const SAMPLES: usize = 9;
const WARM_BATCHES: usize = 8;

struct Fixture {
    same_real: [Vec<f64>; 2],
    same_imag: [Vec<f64>; 2],
    opposite_real: [Vec<f64>; 2],
    opposite_imag: [Vec<f64>; 2],
    basis: Vec<f64>,
    exponents: Vec<i64>,
    cosine: Vec<f64>,
}

impl Fixture {
    fn new() -> Self {
        let mut basis = Vec::with_capacity(LANES);
        let mut exponents = Vec::with_capacity(LANES);
        for lane in 0..LANES {
            let vector_lane = lane % 4;
            let vector = lane / 4;
            let exceptional = vector_lane == 0 || (vector < 7 && vector_lane == 1);
            basis.push(if exceptional {
                0.375
            } else {
                0.21 + ((lane * 29 % 101) as f64) * 0.0061
            });
            exponents.push(if exceptional { 5 } else { 0 });
        }
        let same_real = std::array::from_fn(|channel| {
            (0..LANES)
                .map(|lane| 0.13 + (((lane * (11 + channel * 2) + 7) % 97) as f64) * 0.004)
                .collect()
        });
        let same_imag = std::array::from_fn(|channel| {
            (0..LANES)
                .map(|lane| -0.17 + (((lane * (17 + channel * 2) + 3) % 89) as f64) * 0.003)
                .collect()
        });
        let opposite_real = std::array::from_fn(|channel| {
            (0..LANES)
                .map(|lane| 0.09 + (((lane * (23 + channel * 2) + 5) % 83) as f64) * 0.005)
                .collect()
        });
        let opposite_imag = std::array::from_fn(|channel| {
            (0..LANES)
                .map(|lane| -0.12 + (((lane * (31 + channel * 2) + 2) % 79) as f64) * 0.004)
                .collect()
        });
        Self {
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            basis,
            exponents,
            cosine: vec![1.0; LANES],
        }
    }

    fn fused(&self, arch: Arch) -> [(Complex64, Complex64); 2] {
        arch.dispatch(MixedAnalysisPairChannelsAccumulation::<true> {
            same_real: [&self.same_real[0], &self.same_real[1]],
            same_imag: [&self.same_imag[0], &self.same_imag[1]],
            opposite_real: [&self.opposite_real[0], &self.opposite_real[1]],
            opposite_imag: [&self.opposite_imag[0], &self.opposite_imag[1]],
            basis: &self.basis,
            exponents: &self.exponents,
            cos_theta: &self.cosine,
        })
    }

    fn sequential(&self, arch: Arch) -> [(Complex64, Complex64); 2] {
        arch.dispatch(SequentialMixedAnalysisPairChannels {
            same_real: [&self.same_real[0], &self.same_real[1]],
            same_imag: [&self.same_imag[0], &self.same_imag[1]],
            opposite_real: [&self.opposite_real[0], &self.opposite_real[1]],
            opposite_imag: [&self.opposite_imag[0], &self.opposite_imag[1]],
            basis: &self.basis,
            exponents: &self.exponents,
        })
    }
}

struct SequentialMixedAnalysisPairChannels<'a> {
    same_real: [&'a [f64]; 2],
    same_imag: [&'a [f64]; 2],
    opposite_real: [&'a [f64]; 2],
    opposite_imag: [&'a [f64]; 2],
    basis: &'a [f64],
    exponents: &'a [i64],
}

impl WithSimd for SequentialMixedAnalysisPairChannels<'_> {
    type Output = [(Complex64, Complex64); 2];

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        [0, 1].map(|channel| {
            accumulate_one_channel::<S>(
                simd,
                self.same_real[channel],
                self.same_imag[channel],
                self.opposite_real[channel],
                self.opposite_imag[channel],
                self.basis,
                self.exponents,
            )
        })
    }
}

#[inline(always)]
fn accumulate_one_channel<S: Simd>(
    simd: S,
    same_real: &[f64],
    same_imag: &[f64],
    opposite_real: &[f64],
    opposite_imag: &[f64],
    basis: &[f64],
    exponents: &[i64],
) -> (Complex64, Complex64) {
    let (basis_vectors, basis_tail) = S::as_simd_f64s(basis);
    let (exponent_vectors, exponent_tail) = S::as_simd_i64s(exponents);
    let (same_real_vectors, _) = S::as_simd_f64s(same_real);
    let (same_imag_vectors, _) = S::as_simd_f64s(same_imag);
    let (opposite_real_vectors, _) = S::as_simd_f64s(opposite_real);
    let (opposite_imag_vectors, _) = S::as_simd_f64s(opposite_imag);
    let zero_f64 = simd.splat_f64s(0.0);
    let zero_i64 = simd.splat_i64s(0);
    let minimum = simd.splat_f64s(scalar::SMALL);
    let maximum = simd.splat_f64s(scalar::BIG);
    let mut exceptional_vector_lanes = 0_u64;
    let mut has_lane_masks = true;
    let mut sums = [zero_f64; 4];

    for index in 0..basis_vectors.len() {
        let magnitude = simd.abs_f64s(basis_vectors[index]);
        let in_range = simd.and_m64s(
            simd.less_than_or_equal_f64s(magnitude, maximum),
            simd.or_m64s(
                simd.equal_f64s(magnitude, zero_f64),
                simd.greater_than_or_equal_f64s(magnitude, minimum),
            ),
        );
        let ordinary = simd.and_m64s(simd.equal_i64s(exponent_vectors[index], zero_i64), in_range);
        if let Some(ordinary_lanes) = analysis_mask_lane_bits::<S>(ordinary) {
            exceptional_vector_lanes |= u64::from(!ordinary_lanes & 0b1111) << (index * 4);
        } else {
            has_lane_masks = false;
        }
        let ordinary_basis = simd.select_f64s(ordinary, basis_vectors[index], zero_f64);
        sums[0] = simd.add_f64s(
            sums[0],
            simd.mul_f64s(same_real_vectors[index], ordinary_basis),
        );
        sums[1] = simd.add_f64s(
            sums[1],
            simd.mul_f64s(same_imag_vectors[index], ordinary_basis),
        );
        sums[2] = simd.add_f64s(
            sums[2],
            simd.mul_f64s(opposite_real_vectors[index], ordinary_basis),
        );
        sums[3] = simd.add_f64s(
            sums[3],
            simd.mul_f64s(opposite_imag_vectors[index], ordinary_basis),
        );
    }

    let mut same = Complex64::new(simd.reduce_sum_f64s(sums[0]), simd.reduce_sum_f64s(sums[1]));
    let mut opposite = Complex64::new(simd.reduce_sum_f64s(sums[2]), simd.reduce_sum_f64s(sums[3]));
    let vector_lane_count = basis.len() - basis_tail.len();
    if has_lane_masks {
        for vector_index in 0..basis_vectors.len() {
            let lane_start = vector_index * 4;
            let mut exception_mask = ((exceptional_vector_lanes >> lane_start) & 0b1111) as u8;
            while exception_mask != 0 {
                let lane_offset = exception_mask.trailing_zeros() as usize;
                exception_mask &= exception_mask - 1;
                let lane = lane_start + lane_offset;
                same += scalar::multiply_complex_scaled(
                    Complex64::new(same_real[lane], same_imag[lane]),
                    basis[lane],
                    exponents[lane],
                );
                opposite += scalar::multiply_complex_scaled(
                    Complex64::new(opposite_real[lane], opposite_imag[lane]),
                    basis[lane],
                    exponents[lane],
                );
            }
        }
    } else {
        for lane in 0..vector_lane_count {
            if exponents[lane] == 0 && scalar_value_in_normal_range(basis[lane]) {
                continue;
            }
            same += scalar::multiply_complex_scaled(
                Complex64::new(same_real[lane], same_imag[lane]),
                basis[lane],
                exponents[lane],
            );
            opposite += scalar::multiply_complex_scaled(
                Complex64::new(opposite_real[lane], opposite_imag[lane]),
                basis[lane],
                exponents[lane],
            );
        }
    }
    for lane in 0..basis_tail.len() {
        let index = vector_lane_count + lane;
        if exponent_tail[lane] == 0 && scalar_value_in_normal_range(basis_tail[lane]) {
            same.re += same_real[index] * basis_tail[lane];
            same.im += same_imag[index] * basis_tail[lane];
            opposite.re += opposite_real[index] * basis_tail[lane];
            opposite.im += opposite_imag[index] * basis_tail[lane];
        } else {
            same += scalar::multiply_complex_scaled(
                Complex64::new(same_real[index], same_imag[index]),
                basis_tail[lane],
                exponent_tail[lane],
            );
            opposite += scalar::multiply_complex_scaled(
                Complex64::new(opposite_real[index], opposite_imag[index]),
                basis_tail[lane],
                exponent_tail[lane],
            );
        }
    }
    (same, opposite)
}

fn sample_ns(mut run: impl FnMut()) -> f64 {
    let started = Instant::now();
    for _ in 0..REPETITIONS {
        run();
    }
    started.elapsed().as_secs_f64() * 1.0e9 / REPETITIONS as f64
}

#[test]
#[ignore = "manual B3 scaled-prefix accumulator-only comparison; no recurrence advancement"]
fn benchmark_b3_mixed_accumulator_against_sequential_channels() {
    let arch = Arch::new();
    let fixture = Fixture::new();
    let fused = fixture.fused(arch);
    let sequential = fixture.sequential(arch);
    for channel in 0..2 {
        assert_eq!(fused[channel].0, sequential[channel].0);
        assert_eq!(fused[channel].1, sequential[channel].1);
    }
    eprintln!("B3_ACCUM_ARCH {arch:?} lanes={LANES} exceptional_lanes=23");

    for _ in 0..WARM_BATCHES {
        for _ in 0..REPETITIONS {
            black_box(fixture.fused(arch));
        }
    }
    for _ in 0..WARM_BATCHES {
        for _ in 0..REPETITIONS {
            black_box(fixture.sequential(arch));
        }
    }
    let mut fused_then_sequential = Vec::with_capacity(SAMPLES);
    let mut sequential_then_fused = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        let (fused_ns, sequential_ns) = if sample % 2 == 0 {
            let fused = sample_ns(|| {
                black_box(fixture.fused(arch));
            });
            let sequential = sample_ns(|| {
                black_box(fixture.sequential(arch));
            });
            (fused, sequential)
        } else {
            let sequential = sample_ns(|| {
                black_box(fixture.sequential(arch));
            });
            let fused = sample_ns(|| {
                black_box(fixture.fused(arch));
            });
            (fused, sequential)
        };
        let ratio = sequential_ns / fused_ns;
        eprintln!(
            "B3_ACCUM_SAMPLE {sample} fused_ns={fused_ns:.2} sequential_ns={sequential_ns:.2} sequential_over_fused={ratio:.5}"
        );
        fused_then_sequential.push(fused_ns);
        sequential_then_fused.push(sequential_ns);
    }
    let fused_median = median(&mut fused_then_sequential);
    let sequential_median = median(&mut sequential_then_fused);
    eprintln!(
        "B3_ACCUM_SUMMARY repetitions={REPETITIONS} fused_median_ns={fused_median:.2} sequential_median_ns={sequential_median:.2} sequential_over_fused={:.5}",
        sequential_median / fused_median
    );
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}
