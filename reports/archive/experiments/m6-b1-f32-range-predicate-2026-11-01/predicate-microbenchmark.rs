// Temporary test-module probe used for the archived direct predicate timings.
struct F32PairRangeBenchmark<'a> {
    previous: &'a [f32],
    current: &'a [f32],
    iterations: usize,
}

impl WithSimd for F32PairRangeBenchmark<'_> {
    type Output = (u128, u128);

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let start = std::time::Instant::now();
        let mut exact_sum = 0_u64;
        for _ in 0..self.iterations {
            exact_sum = exact_sum.wrapping_add(u64::from(
                numeric::f32_pair_lanes_need_scaling::<S, true>(
                    simd,
                    std::hint::black_box(self.previous),
                    std::hint::black_box(self.current),
                ),
            ));
        }
        std::hint::black_box(exact_sum);
        let exact = start.elapsed().as_nanos();

        let start = std::time::Instant::now();
        let mut fast_sum = 0_u64;
        for _ in 0..self.iterations {
            fast_sum = fast_sum.wrapping_add(u64::from(f32_scalar_analysis_pair_needs_scaling(
                simd,
                std::hint::black_box(self.previous),
                std::hint::black_box(self.current),
            )));
        }
        std::hint::black_box(fast_sum);
        (exact, start.elapsed().as_nanos())
    }
}

#[test]
#[ignore = "direct B1 f32 range-predicate timing probe"]
fn b1_f32_range_predicate_microbenchmark_corrected() {
    let arch = Arch::new();
    let lanes = arch.dispatch(F32LaneCount);
    let small = <f32 as LegendrePrecision>::SMALL;
    for (name, suspect_lane) in [
        ("ordinary", None),
        ("suspect-first-vector", Some(0)),
        ("suspect-last-vector", Some(lanes * 7)),
    ] {
        let mut previous = vec![0.75; lanes * 8];
        let mut current = vec![-1.25; lanes * 8];
        if let Some(lane) = suspect_lane {
            previous[lane] = small * 0.5;
            current[lane] = small * 0.25;
        }
        let mut samples = Vec::new();
        for _ in 0..7 {
            let pair = arch.dispatch(F32PairRangeBenchmark {
                previous: &previous,
                current: &current,
                iterations: 100_000,
            });
            samples.push(pair);
        }
        let mut exact = samples.iter().map(|p| p.0).collect::<Vec<_>>();
        let mut fast = samples.iter().map(|p| p.1).collect::<Vec<_>>();
        exact.sort_unstable();
        fast.sort_unstable();
        println!(
            "{name}: exact={}ns fast={}ns ratio={:.4}",
            exact[3],
            fast[3],
            fast[3] as f64 / exact[3] as f64
        );
    }
}
