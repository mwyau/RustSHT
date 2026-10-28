use super::*;

use num_complex::Complex32;
use pulp::{Arch, Simd, WithSimd};

struct F32LaneCount;

impl WithSimd for F32LaneCount {
    type Output = usize;

    fn with_simd<S: Simd>(self, _: S) -> Self::Output {
        S::F32_LANES
    }
}

struct AccumulatePacket<'a> {
    cosine: &'a [f32],
    basis: &'a [f32],
    exponents: &'a [i64],
    even: Complex32,
    odd: Complex32,
    initial_real: f32,
    initial_imag: f32,
}

impl WithSimd for AccumulatePacket<'_> {
    type Output = (bool, Vec<f32>, Vec<f32>);

    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let lane_count = S::F32_LANES;
        let even = [self.even];
        let odd = [self.odd];
        let mut real = vec![self.initial_real; lane_count];
        let mut imag = vec![self.initial_imag; lane_count];
        let mut finite_sums = true;
        F32SynthesisAccumulator {
            m: 0,
            cos_theta: self.cosine,
            coefficient_even: &even,
            coefficient_odd: &odd,
            staged_real: &mut real,
            staged_imag: &mut imag,
            finite_sums: &mut finite_sums,
        }
        .visit(0, 0, self.basis, self.exponents, true, simd)
        .expect("packet visitor should not fail");
        (finite_sums, real, imag)
    }
}

fn packet_inputs(lane_count: usize) -> (Vec<f32>, Vec<f32>, Vec<i64>) {
    let cosine = (0..lane_count)
        .map(|lane| -0.45 + 0.1 * lane as f32)
        .collect();
    let basis = (0..lane_count)
        .map(|lane| 0.75 + 0.025 * lane as f32)
        .collect();
    let exponents = (0..lane_count)
        .map(|lane| [-8, 0, 7, -12, 3, 11, -2, 5][lane % 8])
        .collect();
    (cosine, basis, exponents)
}

fn scalar_expected(
    cosine: &[f32],
    basis: &[f32],
    exponents: &[i64],
    even: Complex32,
    odd: Complex32,
    initial_real: f32,
    initial_imag: f32,
) -> (Vec<f32>, Vec<f32>) {
    let mut real = Vec::with_capacity(basis.len());
    let mut imag = Vec::with_capacity(basis.len());
    for lane in 0..basis.len() {
        let coefficient = Complex32::new(
            odd.re.mul_add(cosine[lane], even.re),
            odd.im.mul_add(cosine[lane], even.im),
        );
        let contribution = crate::legendre::scalar::multiply_complex_scaled_f32(
            coefficient,
            basis[lane],
            exponents[lane],
        );
        real.push(initial_real + contribution.re);
        imag.push(initial_imag + contribution.im);
    }
    (real, imag)
}

fn assert_same_bits(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (&actual, &expected) in actual.iter().zip(expected) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

#[test]
fn scaled_f32_packet_matches_native_scalar_arithmetic() {
    let lane_count = Arch::new().dispatch(F32LaneCount);
    let (cosine, basis, exponents) = packet_inputs(lane_count);
    let even = Complex32::new(0.375, -0.25);
    let odd = Complex32::new(-0.125, 0.1875);
    let initial_real = 0.125;
    let initial_imag = -0.25;
    let expected = scalar_expected(
        &cosine,
        &basis,
        &exponents,
        even,
        odd,
        initial_real,
        initial_imag,
    );
    let (finite, actual_real, actual_imag) = Arch::new().dispatch(AccumulatePacket {
        cosine: &cosine,
        basis: &basis,
        exponents: &exponents,
        even,
        odd,
        initial_real,
        initial_imag,
    });
    assert!(finite);
    assert_same_bits(&actual_real, &expected.0);
    assert_same_bits(&actual_imag, &expected.1);
}

#[test]
fn f32_scaled_packet_matches_scalar_for_mixed_extreme_exponents() {
    let lane_count = Arch::new().dispatch(F32LaneCount);
    let cosine = (0..lane_count)
        .map(|lane| -0.4 + 0.1 * lane as f32)
        .collect::<Vec<_>>();
    let basis = vec![1.0_f32; lane_count];
    let exponent_pattern = [-300, -150, -149, -29, -20, 128, 140, 220];
    let exponents = (0..lane_count)
        .map(|lane| exponent_pattern[lane % exponent_pattern.len()])
        .collect::<Vec<_>>();
    let even = Complex32::new(1.0e-30, -1.0e-30);
    let odd = Complex32::new(0.0, 0.0);
    let expected = scalar_expected(&cosine, &basis, &exponents, even, odd, 0.0, 0.0);
    let (finite, actual_real, actual_imag) = Arch::new().dispatch(AccumulatePacket {
        cosine: &cosine,
        basis: &basis,
        exponents: &exponents,
        even,
        odd,
        initial_real: 0.0,
        initial_imag: 0.0,
    });
    assert!(finite);
    assert_same_bits(&actual_real, &expected.0);
    assert_same_bits(&actual_imag, &expected.1);
    assert!(actual_real.iter().any(|value| value.is_subnormal()));
    assert!(actual_real.iter().any(|value| value.is_normal()));
}

#[test]
fn f32_packet_falls_back_for_subnormal_and_out_of_range_values() {
    let lane_count = Arch::new().dispatch(F32LaneCount);
    let (cosine, mut basis, mut exponents) = packet_inputs(lane_count);
    let even = Complex32::new(0.5, -0.25);
    let odd = Complex32::new(0.125, 0.0625);

    basis[0] = f32::from_bits(1);
    let mut expected = scalar_expected(&cosine, &basis, &exponents, even, odd, 0.0, 0.0);
    let (finite, actual_real, actual_imag) = Arch::new().dispatch(AccumulatePacket {
        cosine: &cosine,
        basis: &basis,
        exponents: &exponents,
        even,
        odd,
        initial_real: 0.0,
        initial_imag: 0.0,
    });
    assert!(finite);
    assert_same_bits(&actual_real, &expected.0);
    assert_same_bits(&actual_imag, &expected.1);

    basis[0] = 1.0;
    exponents[0] = -126;
    let subnormal_scale = Complex32::new(0.5, -0.25);
    let zero_odd = Complex32::new(0.0, 0.0);
    expected = scalar_expected(
        &cosine,
        &basis,
        &exponents,
        subnormal_scale,
        zero_odd,
        0.0,
        0.0,
    );
    let (finite, actual_real, actual_imag) = Arch::new().dispatch(AccumulatePacket {
        cosine: &cosine,
        basis: &basis,
        exponents: &exponents,
        even: subnormal_scale,
        odd: zero_odd,
        initial_real: 0.0,
        initial_imag: 0.0,
    });
    assert!(finite);
    assert!(actual_real[0].is_subnormal());
    assert_same_bits(&actual_real, &expected.0);
    assert_same_bits(&actual_imag, &expected.1);

    basis[0] = 1.0;
    exponents[0] = -300;
    expected = scalar_expected(&cosine, &basis, &exponents, even, odd, 0.0, 0.0);
    let (finite, actual_real, actual_imag) = Arch::new().dispatch(AccumulatePacket {
        cosine: &cosine,
        basis: &basis,
        exponents: &exponents,
        even,
        odd,
        initial_real: 0.0,
        initial_imag: 0.0,
    });
    assert!(finite);
    assert_same_bits(&actual_real, &expected.0);
    assert_same_bits(&actual_imag, &expected.1);

    exponents[0] = 127;
    basis[0] = 2.0;
    let (finite, _, _) = Arch::new().dispatch(AccumulatePacket {
        cosine: &cosine,
        basis: &basis,
        exponents: &exponents,
        even: Complex32::new(1.0, 0.0),
        odd: Complex32::new(0.0, 0.0),
        initial_real: 0.0,
        initial_imag: 0.0,
    });
    assert!(!finite);

    exponents[0] = 0;
    basis[0] = 1.0;
    let (finite, _, _) = Arch::new().dispatch(AccumulatePacket {
        cosine: &cosine,
        basis: &basis,
        exponents: &exponents,
        even: Complex32::new(f32::INFINITY, 0.0),
        odd: Complex32::new(0.0, 0.0),
        initial_real: 0.0,
        initial_imag: 0.0,
    });
    assert!(!finite);
}

#[test]
fn f32_scaled_packet_declines_zero_product_that_positive_exponent_can_recover() {
    let lane_count = Arch::new().dispatch(F32LaneCount);
    let cosine = vec![0.0_f32; lane_count];
    let mut basis = vec![1.0_f32; lane_count];
    let mut exponents = vec![0_i64; lane_count];
    basis[0] = 1.0e-20;
    exponents[0] = 30;
    let even = Complex32::new(1.0e-30, -1.0e-30);
    let odd = Complex32::new(0.0, 0.0);
    let expected = scalar_expected(&cosine, &basis, &exponents, even, odd, 0.0, 0.0);
    assert_eq!((even.re * basis[0]).to_bits(), 0.0_f32.to_bits());
    assert_ne!(expected.0[0].to_bits(), 0.0_f32.to_bits());

    let (finite, actual_real, actual_imag) = Arch::new().dispatch(AccumulatePacket {
        cosine: &cosine,
        basis: &basis,
        exponents: &exponents,
        even,
        odd,
        initial_real: 0.0,
        initial_imag: 0.0,
    });
    assert!(finite);
    assert_same_bits(&actual_real, &expected.0);
    assert_same_bits(&actual_imag, &expected.1);
}

#[test]
fn f32_scaled_packet_keeps_partial_simd_tail_on_exact_scalar_path() {
    let lane_count = Arch::new().dispatch(F32LaneCount);
    let tail_len = lane_count - 1;
    let (cosine, basis, exponents) = packet_inputs(tail_len);
    let even = Complex32::new(0.125, -0.25);
    let odd = Complex32::new(-0.0625, 0.03125);
    let expected = scalar_expected(&cosine, &basis, &exponents, even, odd, 0.0, 0.0);
    let (finite, actual_real, actual_imag) = Arch::new().dispatch(AccumulatePacket {
        cosine: &cosine,
        basis: &basis,
        exponents: &exponents,
        even,
        odd,
        initial_real: 0.0,
        initial_imag: 0.0,
    });
    assert!(finite);
    assert_same_bits(&actual_real[..tail_len], &expected.0);
    assert_same_bits(&actual_imag[..tail_len], &expected.1);
}
