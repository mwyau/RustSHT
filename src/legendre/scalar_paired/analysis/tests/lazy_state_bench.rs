use super::super::FusedAnalysisPrecision;
use std::hint::black_box;
use std::time::Instant;

const LANES: usize = 32;
const REPETITIONS: usize = 20_000;

fn samples_ns_per_state(mut run: impl FnMut()) -> Vec<f64> {
    for _ in 0..2 {
        for _ in 0..REPETITIONS {
            run();
        }
    }
    let mut samples = Vec::with_capacity(9);
    for _ in 0..9 {
        let started = Instant::now();
        for _ in 0..REPETITIONS {
            run();
        }
        samples.push(started.elapsed().as_secs_f64() * 1.0e9 / REPETITIONS as f64);
    }
    samples.sort_by(f64::total_cmp);
    samples
}

#[test]
#[ignore = "manual B1 f32 scaled-state setup microbenchmark"]
fn benchmark_b1_f32_scaled_state_setup_vs_lazy_ordinary_block() {
    let same_real = std::array::from_fn::<_, LANES, _>(|lane| {
        0.17_f32 + (0.013_f32 * (lane as f32 + 1.0)).sin()
    });
    let same_imag = std::array::from_fn::<_, LANES, _>(|lane| {
        -0.21_f32 + (0.019_f32 * (lane as f32 + 1.0)).cos()
    });
    let opposite_real = std::array::from_fn::<_, LANES, _>(|lane| {
        0.11_f32 + (0.023_f32 * (lane as f32 + 1.0)).sin()
    });
    let opposite_imag = std::array::from_fn::<_, LANES, _>(|lane| {
        -0.31_f32 + (0.029_f32 * (lane as f32 + 1.0)).cos()
    });

    let initialized = samples_ns_per_state(|| {
        black_box(<f32 as FusedAnalysisPrecision>::prepare_scaled_state(
            &same_real,
            &same_imag,
            &opposite_real,
            &opposite_imag,
        ));
    });
    let lazy_ordinary = samples_ns_per_state(|| {
        black_box(None::<super::super::F32ScaledAnalysisState>);
    });
    let init_median = initialized[initialized.len() / 2];
    let lazy_median = lazy_ordinary[lazy_ordinary.len() / 2];
    eprintln!(
        "B1_F32_SCALED_STATE_SETUP lanes={LANES} repetitions={REPETITIONS} initialized_ns={init_median:.2} lazy_none_ns={lazy_median:.2} samples_initialized_ns={initialized:?} samples_lazy_ns={lazy_ordinary:?}"
    );
}
