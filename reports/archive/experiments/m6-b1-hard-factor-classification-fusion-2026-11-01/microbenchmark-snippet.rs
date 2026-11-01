struct B1F32HardClassBench;

impl WithSimd for B1F32HardClassBench {
    type Output = (Vec<u128>, Vec<u128>, usize, usize);

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        use std::hint::black_box;
        use std::time::Instant;

        let width = S::F32_LANES;
        assert_eq!(width, 8);
        let exponents = std::array::from_fn::<_, 64, _>(|lane| match lane / width {
            0..=4 => 0,
            5 => (lane % width) as i64,
            6 => -100 + (lane % width) as i64,
            _ => [-100, -98, -96, -94, -92, -90, -88, -86][lane % width],
        });
        let cos_theta = [1.0_f32; 64];
        let mut baseline_factors = [0.0_f32; 64];
        let mut candidate_factors = [0.0_f32; 64];
        let mut baseline_hard_factors = [0.0_f32; 64];
        let mut candidate_hard_factors = [0.0_f32; 64];
        let mut baseline_scale_exponents = [0_i64; 64];
        let mut candidate_scale_exponents = [0_i64; 64];
        let mut baseline_scalar_indices = [0_u8; 64];
        let mut candidate_scalar_indices = [0_u8; 64];
        let mut baseline_vector_indices = [0_u8; 64];
        let mut candidate_vector_indices = [0_u8; 64];
        let iterations = 20_000;
        let mut baseline_ns = Vec::with_capacity(9);
        let mut candidate_ns = Vec::with_capacity(9);
        let mut baseline_scalar_count = 0;
        let mut candidate_scalar_count = 0;
        let mut baseline_vector_count = 0;
        let mut candidate_vector_count = 0;

        for sample in 0..9 {
            for candidate in [sample % 2 != 0, sample % 2 == 0] {
                let start = Instant::now();
                for _ in 0..iterations {
                    if candidate {
                        candidate_hard_factors.fill(0.0);
                        candidate_scalar_count = 0;
                        candidate_vector_count = 0;
                        fill_b1_f32_analysis_factors::<S, true>(
                            simd,
                            &exponents,
                            0,
                            0,
                            &cos_theta,
                            &mut candidate_factors,
                            &mut candidate_hard_factors,
                            &mut candidate_scale_exponents,
                            &mut candidate_scalar_indices,
                            &mut candidate_scalar_count,
                            &mut candidate_vector_indices,
                            &mut candidate_vector_count,
                        );
                        black_box((
                            &candidate_factors,
                            &candidate_hard_factors,
                            &candidate_scale_exponents,
                            &candidate_scalar_indices,
                            &candidate_vector_indices,
                            candidate_scalar_count,
                            candidate_vector_count,
                        ));
                    } else {
                        baseline_hard_factors.fill(0.0);
                        baseline_scalar_count = 0;
                        baseline_vector_count = 0;
                        for (vector_index, chunk) in exponents.chunks_exact(width).enumerate() {
                            let lane_start = vector_index * width;
                            let first = chunk[0];
                            let exponent = first;
                            let factor = if exponent < -86 {
                                0.0
                            } else {
                                numeric::scale_binary_f32(1.0, exponent)
                            };
                            if chunk.iter().all(|&x| x == first) {
                                simd.partial_store_f32s(
                                    &mut baseline_factors[lane_start..lane_start + width],
                                    simd.splat_f32s(factor),
                                );
                            } else {
                                for lane in 0..width {
                                    let exponent = chunk[lane];
                                    baseline_factors[lane_start + lane] = if exponent < -86 {
                                        0.0
                                    } else {
                                        numeric::scale_binary_f32(1.0, exponent)
                                    };
                                }
                            }
                        }
                        for (vector_index, scale_exponent) in
                            baseline_scale_exponents.iter_mut().take(64 / width).enumerate()
                        {
                            let lane_start = vector_index * width;
                            let lane_end = lane_start + width;
                            let mut hard_common = None::<i64>;
                            for lane in lane_start..lane_end {
                                if baseline_factors[lane] == 0.0 {
                                    hard_common = Some(
                                        hard_common.map_or(exponents[lane], |maximum| {
                                            maximum.max(exponents[lane])
                                        }),
                                    );
                                }
                            }
                            let Some(hard_common) = hard_common else {
                                continue;
                            };
                            *scale_exponent = hard_common;
                            let mut grouped_lanes = false;
                            for lane in lane_start..lane_end {
                                if baseline_factors[lane] != 0.0 {
                                    continue;
                                }
                                let factor_exponent = exponents[lane] - hard_common;
                                if factor_exponent < -45 {
                                    baseline_scalar_indices[baseline_scalar_count] = lane as u8;
                                    baseline_scalar_count += 1;
                                } else {
                                    baseline_hard_factors[lane] =
                                        numeric::scale_binary_f32(1.0, factor_exponent);
                                    grouped_lanes = true;
                                }
                            }
                            if grouped_lanes {
                                baseline_vector_indices[baseline_vector_count] = vector_index as u8;
                                baseline_vector_count += 1;
                            }
                        }
                        black_box((
                            &baseline_factors,
                            &baseline_hard_factors,
                            &baseline_scale_exponents,
                            &baseline_scalar_indices,
                            &baseline_vector_indices,
                            baseline_scalar_count,
                            baseline_vector_count,
                        ));
                    }
                }
                let elapsed = start.elapsed().as_nanos();
                if candidate {
                    candidate_ns.push(elapsed);
                } else {
                    baseline_ns.push(elapsed);
                }
            }
        }

        assert_eq!(baseline_factors.map(f32::to_bits), candidate_factors.map(f32::to_bits));
        assert_eq!(baseline_hard_factors.map(f32::to_bits), candidate_hard_factors.map(f32::to_bits));
        assert_eq!(baseline_scalar_count, candidate_scalar_count);
        assert_eq!(baseline_vector_count, candidate_vector_count);
        (baseline_ns, candidate_ns, baseline_scalar_count, baseline_vector_count)
    }
}

#[test]
#[ignore = "temporary hard-factor classification microbenchmark"]
fn benchmark_b1_f32_hard_factor_classification() {
    let (baseline_ns, candidate_ns, scalar_count, vector_count) =
        Arch::new().dispatch(B1F32HardClassBench);
    eprintln!("baseline_ns={baseline_ns:?} candidate_ns={candidate_ns:?} hard_scalar={scalar_count} hard_vector={vector_count}");
}
