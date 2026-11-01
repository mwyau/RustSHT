#[inline(always)]
fn baseline_b1_f32_factor_fill(exponents: &[i64], common: i64, shift: i64, factors: &mut [f32]) {
    for (factor, &exponent) in factors.iter_mut().zip(exponents) {
        let exponent = exponent.saturating_sub(common).saturating_add(shift);
        *factor = if exponent < -86 {
            0.0
        } else {
            numeric::scale_binary_f32(1.0, exponent)
        };
    }
}

struct B1F32FactorFillBenchmark;

impl WithSimd for B1F32FactorFillBenchmark {
    type Output = (usize, Vec<u128>, Vec<u128>);

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        use std::hint::black_box;
        use std::time::Instant;

        assert_eq!(S::F32_LANES, 8);
        let exponents = std::array::from_fn::<_, 64, _>(|lane| {
            let vector = lane / 8;
            if vector == 7 {
                7 + (lane % 8) as i64
            } else {
                vector as i64
            }
        });
        let mut baseline_ns = Vec::with_capacity(9);
        let mut candidate_ns = Vec::with_capacity(9);
        for sample in 0..9 {
            for candidate in [sample % 2 != 0, sample % 2 == 0] {
                let mut factors = [0.0_f32; 64];
                let start = Instant::now();
                for _ in 0..20_000 {
                    let inputs = black_box(&exponents);
                    let output = black_box(&mut factors);
                    if candidate {
                        fill_b1_f32_analysis_factors::<S>(simd, inputs, 0, 0, output);
                    } else {
                        baseline_b1_f32_factor_fill(inputs, 0, 0, output);
                    }
                    black_box(&factors);
                }
                let elapsed = start.elapsed().as_nanos();
                if candidate {
                    candidate_ns.push(elapsed);
                } else {
                    baseline_ns.push(elapsed);
                }
            }
        }
        (S::F32_LANES, baseline_ns, candidate_ns)
    }
}

#[test]
#[ignore = "temporary direct B1 factor-fill microbenchmark"]
fn benchmark_b1_f32_uniform_factor_fill() {
    let (lanes, baseline_ns, candidate_ns) = Arch::new().dispatch(B1F32FactorFillBenchmark);
    eprintln!("lanes={lanes} baseline_ns={baseline_ns:?} candidate_ns={candidate_ns:?}");
    assert_eq!(baseline_ns.len(), 9);
    assert_eq!(candidate_ns.len(), 9);
}
