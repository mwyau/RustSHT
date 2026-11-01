use super::super::*;
use crate::grid::{GridKind, GridSpec};
use std::hint::black_box;
use std::time::Instant;

#[test]
#[ignore = "manual B1 sectoral seed initialization microbenchmark"]
fn b1_f64_sectoral_seed_initialization_microbenchmark() {
    const LMAX: usize = 720;
    const ITERATIONS: usize = 8_000;
    let grid = GridSpec::new(GridKind::GaussLegendre, LMAX + 1, 2 * LMAX + 18, 0.23).unwrap();
    let sine = grid.theta()[256..=360]
        .iter()
        .map(|&theta| theta.sin())
        .collect::<Vec<_>>();
    let norms = scalar::sectoral_normalizations(LMAX).unwrap();
    let arch = Arch::new();
    let mut records = Vec::new();

    for m in [1, 180, 360, 720] {
        let sectoral_norm = norms[m];
        let mut previous = vec![0.0; sine.len()];
        let mut current = vec![0.0; sine.len()];
        let mut exponents = vec![0_i64; sine.len()];
        let mut samples = Vec::new();
        for pair in 0..7 {
            let start_scalar = Instant::now();
            for _ in 0..ITERATIONS {
                <f64 as PairedScalarRecurrencePrecision>::initialize_pair_state(
                    m,
                    &sine,
                    sectoral_norm,
                    &mut previous,
                    &mut current,
                    &mut exponents,
                )
                .unwrap();
                black_box((&previous, &current, &exponents));
            }
            let scalar_ns = start_scalar.elapsed().as_nanos() as f64 / ITERATIONS as f64;

            let start_simd = Instant::now();
            for _ in 0..ITERATIONS {
                assert!(<f64 as FusedAnalysisPrecision>::try_initialize_analysis_pair_state(
                    arch,
                    m,
                    LMAX,
                    &sine,
                    sectoral_norm,
                    &mut previous,
                    &mut current,
                    &mut exponents,
                )
                .unwrap());
                black_box((&previous, &current, &exponents));
            }
            let simd_ns = start_simd.elapsed().as_nanos() as f64 / ITERATIONS as f64;
            samples.push((pair, scalar_ns, simd_ns));
        }
        records.push((m, samples));
    }
    eprintln!("seed_init_bench lane_count={} arch={arch:?} pairs={records:?}", sine.len());
}
