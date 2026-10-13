// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// Bounded scalar pass-kernel experiment. The pass layout follows the pinned
// DUCC cfft_multipass descriptor order; the complete C2C schedule is measured
// separately in m3-recursive-vs-multipass.csv.

use std::hint::black_box;
use std::time::Instant;

use num_complex::Complex64;
use rustfft::FftPlanner;

type C = Complex64;

const SAMPLES: usize = 7;
const ZERO: C = C::new(0.0, 0.0);
const COPY_TRAFFIC_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone)]
struct Pass {
    radix: usize,
    l1: usize,
    ido: usize,
    twiddles: Vec<C>,
}

struct MultipassPlan {
    len: usize,
    passes: Vec<Pass>,
}

impl MultipassPlan {
    fn new(len: usize) -> Self {
        let factors = factorize(len);
        let mut passes = Vec::with_capacity(factors.len());
        let mut l1 = 1;
        for radix in factors {
            let ido = len / (l1 * radix);
            let mut twiddles = vec![ZERO; radix * ido];
            for branch in 0..radix {
                for inner in 0..ido {
                    let phase = std::f64::consts::TAU * (branch * l1 * inner) as f64 / len as f64;
                    twiddles[branch * ido + inner] = C::new(phase.cos(), -phase.sin());
                }
            }
            passes.push(Pass {
                radix,
                l1,
                ido,
                twiddles,
            });
            l1 *= radix;
        }
        Self { len, passes }
    }

    fn execute(&self, data: &mut [C], scratch: &mut [C], inverse: bool) {
        self.execute_schedule(data, scratch, inverse, false);
    }

    fn execute_direct(&self, data: &mut [C], scratch: &mut [C], inverse: bool) {
        self.execute_schedule(data, scratch, inverse, true);
    }

    fn execute_schedule(
        &self,
        data: &mut [C],
        scratch: &mut [C],
        inverse: bool,
        direct_radix_2_8: bool,
    ) {
        let mut data_is_current = true;
        for pass in &self.passes {
            if pass.l1 == 1 {
                assert!(data_is_current, "the first DUCC-style pass is in-place");
                if direct_radix_2_8 {
                    dispatch_in_place_direct(data, pass, inverse);
                } else {
                    dispatch_in_place(data, pass, inverse);
                }
            } else if data_is_current {
                if direct_radix_2_8 {
                    dispatch_out_of_place_direct(data, scratch, pass, inverse);
                } else {
                    dispatch_out_of_place(data, scratch, pass, inverse);
                }
                data_is_current = false;
            } else {
                if direct_radix_2_8 {
                    dispatch_out_of_place_direct(scratch, data, pass, inverse);
                } else {
                    dispatch_out_of_place(scratch, data, pass, inverse);
                }
                data_is_current = true;
            }
        }

        let scale = if inverse { 1.0 / self.len as f64 } else { 1.0 };
        if data_is_current {
            for value in data {
                *value *= scale;
            }
        } else {
            for (destination, &value) in data.iter_mut().zip(scratch.iter()) {
                *destination = value * scale;
            }
        }
    }
}

fn dispatch_in_place(data: &mut [C], pass: &Pass, inverse: bool) {
    match pass.radix {
        2 => pass_in_place::<2>(data, pass, inverse),
        4 => pass_in_place::<4>(data, pass, inverse),
        8 => pass_in_place::<8>(data, pass, inverse),
        _ => unreachable!("prototype covers radix 2, 4, and 8"),
    }
}

fn dispatch_out_of_place(input: &[C], output: &mut [C], pass: &Pass, inverse: bool) {
    match pass.radix {
        2 => pass_out_of_place::<2>(input, output, pass, inverse),
        4 => pass_out_of_place::<4>(input, output, pass, inverse),
        8 => pass_out_of_place::<8>(input, output, pass, inverse),
        _ => unreachable!("prototype covers radix 2, 4, and 8"),
    }
}

fn dispatch_in_place_direct(data: &mut [C], pass: &Pass, inverse: bool) {
    match pass.radix {
        2 => pass_in_place_direct::<2>(data, pass, inverse),
        4 => pass_in_place_direct::<4>(data, pass, inverse),
        8 => pass_in_place_direct::<8>(data, pass, inverse),
        _ => dispatch_in_place(data, pass, inverse),
    }
}

fn dispatch_out_of_place_direct(input: &[C], output: &mut [C], pass: &Pass, inverse: bool) {
    match pass.radix {
        2 => pass_out_of_place_direct::<2>(input, output, pass, inverse),
        4 => pass_out_of_place_direct::<4>(input, output, pass, inverse),
        8 => pass_out_of_place_direct::<8>(input, output, pass, inverse),
        _ => dispatch_out_of_place(input, output, pass, inverse),
    }
}

fn pass_in_place_direct<const R: usize>(data: &mut [C], pass: &Pass, inverse: bool) {
    for inner in 0..pass.ido {
        let mut values = [ZERO; 8];
        for branch in 0..R {
            values[branch] = data[inner + pass.ido * branch];
        }
        let transformed = small_dft::<R>(&values, inverse);
        data[inner] = transformed[0];
        for branch in 1..R {
            let twiddle = directed_twiddle(pass, branch, inner, inverse);
            data[inner + pass.ido * branch] = transformed[branch] * twiddle;
        }
    }
}

fn pass_out_of_place_direct<const R: usize>(
    input: &[C],
    output: &mut [C],
    pass: &Pass,
    inverse: bool,
) {
    if R == 2 {
        for outer in 0..pass.l1 {
            for inner in 0..pass.ido {
                let left = input[inner + pass.ido * (2 * outer)];
                let right = input[inner + pass.ido * (1 + 2 * outer)];
                output[inner + pass.ido * outer] = left + right;
                output[inner + pass.ido * (outer + pass.l1)] =
                    (left - right) * directed_twiddle(pass, 1, inner, inverse);
            }
        }
        return;
    }

    for outer in 0..pass.l1 {
        for inner in 0..pass.ido {
            let mut values = [ZERO; 8];
            for branch in 0..R {
                values[branch] = input[inner + pass.ido * (branch + R * outer)];
            }
            let transformed = small_dft::<R>(&values, inverse);
            output[inner + pass.ido * outer] = transformed[0];
            for branch in 1..R {
                let twiddle = directed_twiddle(pass, branch, inner, inverse);
                output[inner + pass.ido * (outer + pass.l1 * branch)] =
                    transformed[branch] * twiddle;
            }
        }
    }
}

fn pass_in_place<const R: usize>(data: &mut [C], pass: &Pass, inverse: bool) {
    let mut input = [ZERO; 8];
    for inner in 0..pass.ido {
        for branch in 0..R {
            input[branch] = data[inner + pass.ido * branch];
        }
        let output = small_dft::<R>(&input, inverse);
        for branch in 0..R {
            let twiddle = directed_twiddle(pass, branch, inner, inverse);
            data[inner + pass.ido * branch] = output[branch] * twiddle;
        }
    }
}

fn pass_out_of_place<const R: usize>(input: &[C], output: &mut [C], pass: &Pass, inverse: bool) {
    let mut values = [ZERO; 8];
    for k in 0..pass.l1 {
        for inner in 0..pass.ido {
            for branch in 0..R {
                values[branch] = input[inner + pass.ido * (branch + R * k)];
            }
            let transformed = small_dft::<R>(&values, inverse);
            for branch in 0..R {
                let twiddle = directed_twiddle(pass, branch, inner, inverse);
                output[inner + pass.ido * (k + pass.l1 * branch)] = transformed[branch] * twiddle;
            }
        }
    }
}

fn directed_twiddle(pass: &Pass, branch: usize, inner: usize, inverse: bool) -> C {
    let value = pass.twiddles[branch * pass.ido + inner];
    if inverse { value.conj() } else { value }
}

fn small_dft<const R: usize>(input: &[C; 8], inverse: bool) -> [C; 8] {
    let mut output = [ZERO; 8];
    if R == 2 {
        output[0] = input[0] + input[1];
        output[1] = input[0] - input[1];
    } else if R == 4 {
        let even_sum = input[0] + input[2];
        let even_difference = input[0] - input[2];
        let odd_sum = input[1] + input[3];
        let odd_difference = input[1] - input[3];
        let turn = rotate_quarter(odd_difference, inverse);
        output[0] = even_sum + odd_sum;
        output[2] = even_sum - odd_sum;
        output[1] = even_difference + turn;
        output[3] = even_difference - turn;
    } else {
        let mut a1 = input[1] + input[5];
        let mut a5 = input[1] - input[5];
        let mut a3 = input[3] + input[7];
        let mut a7 = input[3] - input[7];
        let old_a1 = a1;
        a1 += a3;
        a3 = rotate_quarter(old_a1 - a3, inverse);
        a7 = rotate_quarter(a7, inverse);
        let old_a5 = a5;
        a5 += a7;
        a7 = rotate_eighth_three(old_a5 - a7, inverse);
        a5 = rotate_eighth(a5, inverse);

        let a0 = input[0] + input[4];
        let a4 = input[0] - input[4];
        let a2 = input[2] + input[6];
        let a6 = rotate_quarter(input[2] - input[6], inverse);
        output[0] = a0 + a2 + a1;
        output[4] = a0 + a2 - a1;
        output[2] = a0 - a2 + a3;
        output[6] = a0 - a2 - a3;
        output[1] = a4 + a6 + a5;
        output[5] = a4 + a6 - a5;
        output[3] = a4 - a6 + a7;
        output[7] = a4 - a6 - a7;
    }
    output
}

fn rotate_quarter(value: C, inverse: bool) -> C {
    if inverse {
        C::new(-value.im, value.re)
    } else {
        C::new(value.im, -value.re)
    }
}

fn rotate_eighth(value: C, inverse: bool) -> C {
    let c = std::f64::consts::FRAC_1_SQRT_2;
    if inverse {
        C::new((value.re - value.im) * c, (value.re + value.im) * c)
    } else {
        C::new((value.re + value.im) * c, (-value.re + value.im) * c)
    }
}

fn rotate_eighth_three(value: C, inverse: bool) -> C {
    let c = std::f64::consts::FRAC_1_SQRT_2;
    if inverse {
        C::new((-value.re - value.im) * c, (value.re - value.im) * c)
    } else {
        C::new((value.im - value.re) * c, (-value.re - value.im) * c)
    }
}

fn factorize(mut remaining: usize) -> Vec<usize> {
    let mut factors = Vec::new();
    while remaining & 7 == 0 {
        factors.push(8);
        remaining >>= 3;
    }
    while remaining & 3 == 0 {
        factors.push(4);
        remaining >>= 2;
    }
    if remaining & 1 == 0 {
        remaining >>= 1;
        factors.push(2);
        let last = factors.len() - 1;
        factors.swap(0, last);
    }
    let mut divisor = 3;
    while divisor <= remaining / divisor {
        while remaining.is_multiple_of(divisor) {
            factors.push(divisor);
            remaining /= divisor;
        }
        divisor += 2;
    }
    if remaining > 1 {
        factors.push(remaining);
    }
    factors
}

fn input_value(index: usize) -> C {
    let x = (index + 1) as f64;
    C::new((0.173 * x).sin(), (0.113 * (x + 1.0)).cos())
}

fn verify(plan: &MultipassPlan, inverse: bool, direct: bool) {
    let mut actual: Vec<_> = (0..plan.len).map(input_value).collect();
    let mut scratch = vec![ZERO; plan.len];
    if direct {
        plan.execute_direct(&mut actual, &mut scratch, inverse);
    } else {
        plan.execute(&mut actual, &mut scratch, inverse);
    }

    let mut expected: Vec<_> = (0..plan.len).map(input_value).collect();
    let mut planner = FftPlanner::<f64>::new();
    let reference = if inverse {
        planner.plan_fft_inverse(plan.len)
    } else {
        planner.plan_fft_forward(plan.len)
    };
    reference.process(&mut expected);
    if inverse {
        for value in &mut expected {
            *value /= plan.len as f64;
        }
    }
    assert!(
        actual
            .iter()
            .chain(expected.iter())
            .all(|value| value.re.is_finite() && value.im.is_finite()),
        "nonfinite prototype result at n={}",
        plan.len
    );
    let error = actual
        .iter()
        .zip(expected)
        .map(|(&actual, expected)| (actual - expected).norm())
        .fold(0.0_f64, f64::max);
    assert!(
        error < 2.0e-10 * plan.len as f64,
        "n={} error={error}",
        plan.len
    );
}

fn benchmark(len: usize, inverse: bool, direct: bool) {
    let plan_time = average_ns(20, || {
        black_box(MultipassPlan::new(len).passes.len());
    });
    let plan = MultipassPlan::new(len);
    let scratch_time = average_ns(20, || {
        black_box(vec![ZERO; len].len());
    });
    let mut data: Vec<_> = (0..len).map(input_value).collect();
    let mut scratch = vec![ZERO; len];
    let mut samples = Vec::with_capacity(SAMPLES);
    for sample in 0..=SAMPLES {
        for (index, value) in data.iter_mut().enumerate() {
            *value = input_value(index);
        }
        let start = Instant::now();
        if direct {
            plan.execute_direct(&mut data, &mut scratch, inverse);
        } else {
            plan.execute(&mut data, &mut scratch, inverse);
        }
        black_box(&data);
        if sample != 0 {
            samples.push(start.elapsed().as_nanos() as f64);
        }
    }
    samples.sort_by(f64::total_cmp);
    let operation_time = samples[SAMPLES / 2];
    let checksum = data[0].re + data[0].im;
    println!(
        "{},{},{:.3},{:.3},{:.3},{},{}",
        if inverse && direct {
            "c2c_inverse_direct_2_4_8"
        } else if !inverse && direct {
            "c2c_forward_direct_2_4_8"
        } else if inverse {
            "c2c_inverse"
        } else {
            "c2c_forward"
        },
        len,
        plan_time,
        scratch_time,
        operation_time,
        plan.passes.len(),
        checksum
    );
}

fn average_ns(repeats: usize, mut operation: impl FnMut()) -> f64 {
    let start = Instant::now();
    for _ in 0..repeats {
        operation();
    }
    start.elapsed().as_nanos() as f64 / repeats as f64
}

fn profile_copyback(len: usize) {
    assert!(len > 0);
    let source: Vec<_> = (0..len)
        .map(|index| C::new(index as f64, -(index as f64)))
        .collect();
    let mut destination = vec![ZERO; len];
    let repeats = (COPY_TRAFFIC_BYTES / (2 * len * std::mem::size_of::<C>())).max(1);
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..repeats {
            destination.copy_from_slice(&source);
            black_box(destination.as_slice());
        }
        samples.push(start.elapsed().as_nanos() as f64 / repeats as f64);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "copyback,{len},{repeats},{},{:.3},{:.9}",
        2 * len * std::mem::size_of::<C>(),
        samples[SAMPLES / 2],
        destination[0].re + destination[len - 1].im
    );
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|argument| argument == "copyback")
    {
        println!("operation,n,repeats_per_sample,bytes_read_and_written,copyback_ns,checksum");
        for value in arguments.iter().skip(1).filter(|value| *value != "--bench") {
            profile_copyback(value.parse().expect("length must be an integer"));
        }
        return;
    }
    let lengths: Vec<&str> = arguments
        .iter()
        .map(String::as_str)
        .filter(|value| *value != "--bench")
        .collect();
    if lengths.is_empty() || lengths.iter().any(|value| value.starts_with('-')) {
        return;
    }
    println!("mode,n,plan_ns,workspace_ns,public_operation_ns,pass_count,checksum");
    let lengths: Vec<usize> = lengths
        .into_iter()
        .map(|value| value.parse().expect("length must be an integer"))
        .collect();
    for len in lengths {
        let plan = MultipassPlan::new(len);
        verify(&plan, false, false);
        verify(&plan, true, false);
        verify(&plan, false, true);
        verify(&plan, true, true);
        benchmark(len, false, false);
        benchmark(len, true, false);
        benchmark(len, false, true);
        benchmark(len, true, true);
    }
}
