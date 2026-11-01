use super::super::FusedAnalysisPrecision;
use pulp::{Arch, Simd, WithSimd};
use std::hint::black_box;
use std::time::Instant;

const LANES: usize = 256;
const DEGREE_PAIRS: usize = 360;

struct B1ScaledAccumBench<'a> {
    same_real: &'a [f32],
    same_imag: &'a [f32],
    opposite_real: &'a [f32],
    opposite_imag: &'a [f32],
    basis_a: &'a [f32],
    basis_b: &'a [f32],
    exponents: &'a [i64],
    cosine: &'a [f32],
}

impl WithSimd for B1ScaledAccumBench<'_> {
    type Output = (u128, u128, f64, f32);

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let setup_started = Instant::now();
        let mut state = <f32 as FusedAnalysisPrecision>::prepare_scaled_state(
            self.same_real,
            self.same_imag,
            self.opposite_real,
            self.opposite_imag,
        );
        let setup_ns = setup_started.elapsed().as_nanos();

        let first_started = Instant::now();
        let first = <f32 as FusedAnalysisPrecision>::accumulate_scaled::<S, true>(
            simd,
            self.same_real,
            self.same_imag,
            self.opposite_real,
            self.opposite_imag,
            self.basis_a,
            self.exponents,
            self.cosine,
            &mut state,
        );
        let first_ns = first_started.elapsed().as_nanos();
        let mut checksum = first.0.re + first.0.im + first.1.re + first.1.im;

        let steady_started = Instant::now();
        for pair in 1..DEGREE_PAIRS {
            let basis = if pair & 1 == 0 {
                self.basis_a
            } else {
                self.basis_b
            };
            let sums = <f32 as FusedAnalysisPrecision>::accumulate_scaled::<S, true>(
                simd,
                self.same_real,
                self.same_imag,
                self.opposite_real,
                self.opposite_imag,
                basis,
                self.exponents,
                self.cosine,
                &mut state,
            );
            checksum = black_box(checksum + sums.0.re + sums.0.im + sums.1.re + sums.1.im);
        }
        let steady_ns_per_pair =
            steady_started.elapsed().as_secs_f64() * 1.0e9 / (DEGREE_PAIRS - 1) as f64;
        (setup_ns, first_ns, steady_ns_per_pair, checksum)
    }
}

#[test]
#[ignore = "manual B1 f32 scaled-accumulation microbenchmark; run release and pin one CPU"]
fn benchmark_b1_f32_scaled_accumulation_t720() {
    let same_real = (0..LANES)
        .map(|lane| 0.15_f32 + (lane as f32 * 0.017).sin() * 0.25)
        .collect::<Vec<_>>();
    let same_imag = (0..LANES)
        .map(|lane| -0.2_f32 + (lane as f32 * 0.011).cos() * 0.3)
        .collect::<Vec<_>>();
    let opposite_real = (0..LANES)
        .map(|lane| 0.1_f32 + (lane as f32 * 0.013).sin() * 0.2)
        .collect::<Vec<_>>();
    let opposite_imag = (0..LANES)
        .map(|lane| -0.3_f32 + (lane as f32 * 0.019).cos() * 0.15)
        .collect::<Vec<_>>();
    let basis_a = (0..LANES)
        .map(|lane| (lane as f32 * 0.031).sin() * 0.4)
        .collect::<Vec<_>>();
    let basis_b = (0..LANES)
        .map(|lane| (lane as f32 * 0.027).cos() * 0.35)
        .collect::<Vec<_>>();
    let exponents = (0..LANES)
        .map(|lane| -2 * (lane % 80) as i64)
        .collect::<Vec<_>>();
    let cosine = (0..LANES)
        .map(|lane| -0.85_f32 + lane as f32 * 1.7 / LANES as f32)
        .collect::<Vec<_>>();

    let mut samples = Vec::with_capacity(9);
    for _ in 0..9 {
        samples.push(Arch::new().dispatch(B1ScaledAccumBench {
            same_real: &same_real,
            same_imag: &same_imag,
            opposite_real: &opposite_real,
            opposite_imag: &opposite_imag,
            basis_a: &basis_a,
            basis_b: &basis_b,
            exponents: &exponents,
            cosine: &cosine,
        }));
    }
    let steady = samples.iter().map(|sample| sample.2).collect::<Vec<_>>();
    let mut sorted = steady.clone();
    sorted.sort_by(f64::total_cmp);
    eprintln!(
        "B1_F32_SCALED_ACCUM lanes={LANES} degree_pairs={DEGREE_PAIRS} arch={:?} setup_ns={:?} first_scaled_pair_ns={:?} steady_ns_per_pair={steady:?} steady_median_ns_per_pair={:.2} checksum={:.8e}",
        Arch::new(),
        samples.iter().map(|sample| sample.0).collect::<Vec<_>>(),
        samples.iter().map(|sample| sample.1).collect::<Vec<_>>(),
        sorted[sorted.len() / 2],
        samples.iter().map(|sample| sample.3).sum::<f32>(),
    );
    assert!(samples.iter().all(|sample| sample.3.is_finite()));
}
