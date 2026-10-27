use super::super::*;
use crate::error::ShtError;
use crate::legendre::{LATITUDE_BLOCK_SIZE, LegendrePrecision};
use num_complex::Complex32;
use pulp::{Simd, WithSimd};
use std::f32::consts::PI;

const BIG_F32: f32 = <f32 as LegendrePrecision>::BIG;
const SMALL_F32: f32 = <f32 as LegendrePrecision>::SMALL;
const SCALE_UP_F32: f32 = <f32 as LegendrePrecision>::SCALE_UP;
const SCALE_DOWN_F32: f32 = <f32 as LegendrePrecision>::SCALE_DOWN;

struct CheckPairStateRange<'a> {
    previous: &'a [f32],
    current: &'a [f32],
}

impl WithSimd for CheckPairStateRange<'_> {
    type Output = bool;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        pair_recurrence_state_f32_in_safe_range(simd, self.previous, self.current)
    }
}

struct NormalizePairPrechecked<'a> {
    previous: &'a mut [f32],
    current: &'a mut [f32],
    exponents: &'a mut [i64],
}

impl WithSimd for NormalizePairPrechecked<'_> {
    type Output = Result<(), ShtError>;

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        normalize_pair_lanes_f32_prechecked(simd, self.previous, self.current, self.exponents)
    }
}

struct AnalysisLanes32 {
    q_real: Vec<f32>,
    q_imag: Vec<f32>,
    u_real: Vec<f32>,
    u_imag: Vec<f32>,
    plus: Vec<f32>,
    plus_exponents: Vec<i64>,
    minus: Vec<f32>,
    minus_exponents: Vec<i64>,
    limits: Vec<usize>,
}

impl AnalysisLanes32 {
    fn new(count: usize) -> Self {
        Self {
            q_real: (0..count).map(|lane| 0.25 + lane as f32 * 0.007).collect(),
            q_imag: (0..count).map(|lane| -0.4 + lane as f32 * 0.003).collect(),
            u_real: (0..count).map(|lane| 0.15 - lane as f32 * 0.002).collect(),
            u_imag: (0..count).map(|lane| 0.55 + lane as f32 * 0.004).collect(),
            plus: (0..count).map(|lane| 0.31 + lane as f32 * 0.001).collect(),
            plus_exponents: vec![0; count],
            minus: (0..count).map(|lane| -0.17 + lane as f32 * 0.002).collect(),
            minus_exponents: vec![0; count],
            limits: vec![64; count],
        }
    }

    fn accumulate(
        &self,
        arch: Arch,
        m: usize,
        alpha_norm: f32,
        include_b: bool,
    ) -> (Complex32, Complex32) {
        let vectorizable_inputs = super::super::analysis_inputs_vectorizable::<f32>(
            m,
            &self.q_real,
            &self.q_imag,
            &self.u_real,
            &self.u_imag,
            &self.limits,
        );
        super::super::accumulate_analysis_dispatched::<f32>(
            arch,
            vectorizable_inputs,
            m,
            &self.q_real,
            &self.q_imag,
            &self.u_real,
            &self.u_imag,
            &self.plus,
            &self.plus_exponents,
            &self.minus,
            &self.minus_exponents,
            alpha_norm,
            &self.limits,
            include_b,
        )
    }

    fn scalar_reference(
        &self,
        m: usize,
        alpha_norm: f32,
        include_b: bool,
    ) -> (Complex32, Complex32) {
        let mut e_sum = Complex32::new(0.0, 0.0);
        let mut b_sum = Complex32::new(0.0, 0.0);
        for lane in 0..self.plus.len() {
            if m > self.limits[lane] {
                continue;
            }
            let plus_value =
                scaled_basis_value_f32(self.plus[lane], self.plus_exponents[lane], alpha_norm);
            let minus_value =
                scaled_basis_value_f32(self.minus[lane], self.minus_exponents[lane], alpha_norm);
            let even_basis = plus_value + minus_value;
            let odd_basis = plus_value - minus_value;
            e_sum.re += self.q_real[lane] * even_basis + self.u_imag[lane] * odd_basis;
            e_sum.im += self.q_imag[lane] * even_basis - self.u_real[lane] * odd_basis;
            if include_b {
                b_sum.re += self.u_real[lane] * even_basis - self.q_imag[lane] * odd_basis;
                b_sum.im += self.u_imag[lane] * even_basis + self.q_real[lane] * odd_basis;
            }
        }
        (e_sum, b_sum)
    }
}

#[test]
fn simd_normalizer_precheck_matches_scalar_scaler_for_boundaries_and_tail() {
    let safe_previous = vec![
        0.0, -0.0, 0.25, -0.5, SMALL_F32, -SMALL_F32, BIG_F32, -BIG_F32, 0.0, 1.0, 0.0, 0.25,
        -0.75, 0.0, 2.0, 0.0, 0.5,
    ];
    let safe_current = vec![
        0.0, 0.0, -0.25, 0.5, 0.0, SMALL_F32, 0.0, BIG_F32, -0.0, -1.0, 0.0, -0.25, 0.75, 0.0,
        -2.0, 0.0, -0.5,
    ];
    let mut unsafe_previous = safe_previous.clone();
    let mut unsafe_current = safe_current.clone();
    unsafe_previous[0] = f32::from_bits(1);
    unsafe_previous[1] = f32::MAX;
    unsafe_current[2] = -f32::MAX;
    unsafe_current[8] = f32::from_bits(SMALL_F32.to_bits() - 1);
    unsafe_previous[9] = BIG_F32 * 2.0;
    unsafe_current[16] = f32::from_bits(SMALL_F32.to_bits() - 1);
    let mut nonfinite = safe_current.clone();
    nonfinite[3] = f32::INFINITY;
    nonfinite[11] = f32::NAN;

    for (name, arch) in [("scalar", Arch::Scalar), ("runtime", Arch::new())] {
        assert!(
            arch.dispatch(CheckPairStateRange {
                previous: &safe_previous,
                current: &safe_current,
            }),
            "{name} backend rejected safe values"
        );
        assert!(!arch.dispatch(CheckPairStateRange {
            previous: &unsafe_previous,
            current: &unsafe_current,
        }));
        assert!(!arch.dispatch(CheckPairStateRange {
            previous: &safe_previous,
            current: &nonfinite,
        }));

        for (initial_previous, initial_current) in [
            (&safe_previous, &safe_current),
            (&unsafe_previous, &unsafe_current),
        ] {
            let mut expected_previous = initial_previous.clone();
            let mut expected_current = initial_current.clone();
            let mut expected_exponents = vec![19_i64; initial_previous.len()];
            normalize_pair_lanes_f32(
                &mut expected_previous,
                &mut expected_current,
                &mut expected_exponents,
            )
            .unwrap();

            let mut actual_previous = initial_previous.clone();
            let mut actual_current = initial_current.clone();
            let mut actual_exponents = vec![19_i64; initial_previous.len()];
            arch.dispatch(NormalizePairPrechecked {
                previous: &mut actual_previous,
                current: &mut actual_current,
                exponents: &mut actual_exponents,
            })
            .unwrap();
            assert_eq!(
                actual_previous
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
                expected_previous
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
            );
            assert_eq!(
                actual_current
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
                expected_current
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
            );
            assert_eq!(actual_exponents, expected_exponents);
        }
    }
}

#[test]
fn f32_spin_analysis_simd_accumulation_matches_scalar_blocks_and_tails() {
    for count in [64, 61] {
        let lanes = AnalysisLanes32::new(count);
        let expected = lanes.scalar_reference(4, 0.75, true);
        assert!(super::super::analysis_inputs_vectorizable::<f32>(
            4,
            &lanes.q_real,
            &lanes.q_imag,
            &lanes.u_real,
            &lanes.u_imag,
            &lanes.limits,
        ));
        for arch in [Arch::Scalar, Arch::new()] {
            let actual = lanes.accumulate(arch, 4, 0.75, true);
            assert!((actual.0 - expected.0).norm() < 1.0e-4);
            assert!((actual.1 - expected.1).norm() < 1.0e-4);

            let e_only = lanes.accumulate(arch, 4, 0.75, false);
            assert!((e_only.0 - expected.0).norm() < 1.0e-4);
            assert_eq!(e_only.1, Complex32::new(0.0, 0.0));
        }
    }
}

#[test]
fn f32_spin_analysis_falls_back_when_simd_lane_partials_can_overflow() {
    let mut lanes = AnalysisLanes32::new(16);
    for lane in 0..lanes.q_real.len() {
        lanes.q_real[lane] = if lane.is_multiple_of(2) {
            f32::MAX
        } else {
            -f32::MAX
        };
        lanes.q_imag[lane] = 0.0;
        lanes.u_real[lane] = 0.0;
        lanes.u_imag[lane] = 0.0;
        lanes.plus[lane] = 0.5;
        lanes.minus[lane] = 0.5;
    }

    assert!(super::super::analysis_inputs_vectorizable::<f32>(
        4,
        &lanes.q_real,
        &lanes.q_imag,
        &lanes.u_real,
        &lanes.u_imag,
        &lanes.limits,
    ));
    let expected = lanes.scalar_reference(4, 1.0, false);
    assert_eq!(expected.0, Complex32::new(0.0, 0.0));
    for arch in [Arch::Scalar, Arch::new()] {
        assert_eq!(lanes.accumulate(arch, 4, 1.0, false), expected);
    }
}

#[test]
fn f32_spin_analysis_zero_inputs_use_simd_and_scaled_lanes_use_scalar_accumulation() {
    let mut lanes = AnalysisLanes32::new(64);
    lanes.plus_exponents[11] = -800;
    lanes.limits[7] = 3;
    let expected = lanes.scalar_reference(4, 0.75, true);
    assert!(!super::super::analysis_inputs_vectorizable::<f32>(
        4,
        &lanes.q_real,
        &lanes.q_imag,
        &lanes.u_real,
        &lanes.u_imag,
        &lanes.limits,
    ));
    for arch in [Arch::Scalar, Arch::new()] {
        let actual = lanes.accumulate(arch, 4, 0.75, true);
        assert_eq!(actual, expected);
    }

    lanes.limits.fill(64);
    lanes.q_real[4] = 0.0;
    lanes.u_imag[9] = -0.0;
    lanes.plus[12] = 0.0;
    lanes.minus[17] = -0.0;
    assert!(
        lanes
            .plus
            .iter()
            .all(|&value| value == 0.0 || value.is_normal())
    );
    assert!(
        lanes
            .minus
            .iter()
            .all(|&value| value == 0.0 || value.is_normal())
    );
    let zero_expected = lanes.scalar_reference(4, 0.75, true);
    assert!(super::super::analysis_inputs_vectorizable::<f32>(
        4,
        &lanes.q_real,
        &lanes.q_imag,
        &lanes.u_real,
        &lanes.u_imag,
        &lanes.limits,
    ));
    for arch in [Arch::Scalar, Arch::new()] {
        let actual = lanes.accumulate(arch, 4, 0.75, true);
        assert!((actual.0 - zero_expected.0).norm() < 1.0e-4);
        assert!((actual.1 - zero_expected.1).norm() < 1.0e-4);
    }

    lanes.q_real[4] = f32::from_bits(1);
    lanes.plus[4] = f32::from_bits(1);
    assert!(
        !lanes
            .plus
            .iter()
            .all(|&value| value == 0.0 || value.is_normal())
    );
    assert!(!super::super::analysis_inputs_vectorizable::<f32>(
        4,
        &lanes.q_real,
        &lanes.q_imag,
        &lanes.u_real,
        &lanes.u_imag,
        &lanes.limits,
    ));
}

fn reference_normalize_pair_lanes_f32(
    previous: &mut [f32],
    current: &mut [f32],
    exponents: &mut [i64],
) {
    for lane in 0..previous.len() {
        let mut maximum = previous[lane].abs().max(current[lane].abs());
        while maximum > BIG_F32 {
            previous[lane] *= SCALE_DOWN_F32;
            current[lane] *= SCALE_DOWN_F32;
            exponents[lane] += 80;
            maximum *= SCALE_DOWN_F32;
        }
        while maximum != 0.0 && maximum < SMALL_F32 {
            previous[lane] *= SCALE_UP_F32;
            current[lane] *= SCALE_UP_F32;
            exponents[lane] -= 80;
            maximum *= SCALE_UP_F32;
        }
    }
}

#[test]
fn f32_spin_pair_normalization_matches_float_magnitude_reference() {
    let initial_previous = [
        0.0,
        f32::from_bits(1),
        f32::from_bits(SMALL_F32.to_bits() - 1),
        -f32::from_bits(BIG_F32.to_bits() + 1),
        f32::MAX,
        f32::MIN_POSITIVE,
        -0.5,
        0.0,
    ];
    let initial_current = [
        -0.0,
        0.0,
        0.0,
        BIG_F32,
        f32::MIN_POSITIVE,
        -f32::from_bits(1),
        0.25,
        SMALL_F32,
    ];
    let mut actual_previous = initial_previous;
    let mut actual_current = initial_current;
    let mut actual_exponents = [0; 8];
    let mut expected_previous = initial_previous;
    let mut expected_current = initial_current;
    let mut expected_exponents = [0; 8];

    normalize_pair_lanes_f32(
        &mut actual_previous,
        &mut actual_current,
        &mut actual_exponents,
    )
    .unwrap();
    reference_normalize_pair_lanes_f32(
        &mut expected_previous,
        &mut expected_current,
        &mut expected_exponents,
    );

    for lane in 0..initial_previous.len() {
        assert_eq!(
            actual_previous[lane].to_bits(),
            expected_previous[lane].to_bits()
        );
        assert_eq!(
            actual_current[lane].to_bits(),
            expected_current[lane].to_bits()
        );
        assert_eq!(actual_exponents[lane], expected_exponents[lane]);
    }
}

fn values_over_rings(
    arch: Arch,
    spin: usize,
    m: usize,
    lmax: usize,
    theta: &[f32],
) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
    let count = lmax + 2;
    let mut alpha = vec![0.0; count];
    let mut coefficient_a = vec![0.0; count];
    let mut coefficient_b = vec![0.0; count];
    let initial = crate::legendre::spin_factors::fill_spin_factors::<f32>(
        spin,
        m,
        lmax,
        &mut alpha,
        &mut coefficient_a,
        &mut coefficient_b,
    )
    .unwrap();
    let sin_theta = theta.iter().map(|value| value.sin()).collect::<Vec<_>>();
    let cos_theta = theta.iter().map(|value| value.cos()).collect::<Vec<_>>();
    let scratch_len = theta.len().min(LATITUDE_BLOCK_SIZE);
    let mut previous_plus = vec![0.0; scratch_len];
    let mut current_plus = vec![0.0; scratch_len];
    let mut next_plus = vec![0.0; scratch_len];
    let mut exponent_plus = vec![0; scratch_len];
    let mut previous_minus = vec![0.0; scratch_len];
    let mut current_minus = vec![0.0; scratch_len];
    let mut next_minus = vec![0.0; scratch_len];
    let mut exponent_minus = vec![0; scratch_len];
    let degree_count = lmax - initial.start_degree + 1;
    let mut plus_values = vec![vec![0.0; theta.len()]; degree_count];
    let mut minus_values = vec![vec![0.0; theta.len()]; degree_count];
    for_each_spin_lm_dispatched_typed::<f32, _>(
        arch,
        &initial,
        lmax,
        &sin_theta,
        &cos_theta,
        &coefficient_a,
        &coefficient_b,
        &mut previous_plus,
        &mut current_plus,
        &mut next_plus,
        &mut exponent_plus,
        &mut previous_minus,
        &mut current_minus,
        &mut next_minus,
        &mut exponent_minus,
        |degree, ring_start, plus, plus_exponents, minus, minus_exponents| {
            let factor =
                crate::legendre::spin_factors::spin_normalization::<f32>(spin, degree, false)
                    * alpha[degree];
            for lane in 0..plus.len() {
                plus_values[degree - initial.start_degree][ring_start + lane] =
                    scaled_basis_value_f32(plus[lane], plus_exponents[lane], factor);
                minus_values[degree - initial.start_degree][ring_start + lane] =
                    scaled_basis_value_f32(minus[lane], minus_exponents[lane], factor);
            }
            Ok(())
        },
    )
    .unwrap();
    (plus_values, minus_values)
}

#[test]
fn runtime_simd_recurrence_matches_scalar_fallback_across_blocks_and_tails() {
    let mut theta = (0..129)
        .map(|ring| PI * (ring as f32 + 0.5) / 129.0)
        .collect::<Vec<_>>();
    theta[0] = 1.0e-6;
    theta[1] = 1.0e-3;
    theta[127] = PI - 1.0e-3;
    theta[128] = PI - 1.0e-6;

    for spin in 1..=4 {
        for m in [0, spin, 96] {
            let expected = values_over_rings(Arch::Scalar, spin, m, 96, &theta);
            let actual = values_over_rings(Arch::new(), spin, m, 96, &theta);
            for (sign, expected_values, actual_values) in [
                ("plus", &expected.0, &actual.0),
                ("minus", &expected.1, &actual.1),
            ] {
                for (degree_offset, (expected_rings, actual_rings)) in
                    expected_values.iter().zip(actual_values).enumerate()
                {
                    for (ring, (&expected, &actual)) in
                        expected_rings.iter().zip(actual_rings).enumerate()
                    {
                        assert!(
                            actual.is_finite(),
                            "spin={spin}, m={m}, sign={sign}, l={}, ring={ring}: non-finite result {actual}",
                            spin.max(m) + degree_offset
                        );
                        assert!(
                            (actual - expected).abs() <= 5.0e-6,
                            "spin={spin}, m={m}, sign={sign}, l={}, ring={ring}: actual {actual:.9e}, scalar {expected:.9e}",
                            spin.max(m) + degree_offset
                        );
                    }
                }
            }
        }
    }
}
