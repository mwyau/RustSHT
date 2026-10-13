// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// Bounded scalar real-inverse stage experiment. The loop layout follows the
// l1/ido organization used by DUCC's rfftp4 pass. It compares the recursive
// engine's small-radix combine form with a direct scalar radix-4 stage; it is
// not a replacement real-transform plan.

use std::hint::black_box;
use std::time::Instant;

use num_complex::Complex64;

type C = Complex64;
const ZERO: C = C::new(0.0, 0.0);
const REPEATS: usize = 256;
const SAMPLES: usize = 7;

struct Radix4Stage {
    len: usize,
    l1: usize,
    ido: usize,
    twiddles: Vec<C>,
}

impl Radix4Stage {
    fn new(len: usize, l1: usize, ido: usize) -> Self {
        assert_eq!(len, 4 * l1 * ido);
        let mut twiddles = vec![ZERO; 4 * ido];
        for branch in 0..4 {
            for inner in 0..ido {
                let phase = std::f64::consts::TAU * (branch * l1 * inner) as f64 / len as f64;
                twiddles[branch * ido + inner] = C::new(phase.cos(), -phase.sin());
            }
        }
        Self {
            len,
            l1,
            ido,
            twiddles,
        }
    }

    fn generic(&self, input: &[C], output: &mut [C], inverse: bool) {
        for outer in 0..self.l1 {
            for inner in 0..self.ido {
                let mut values = [ZERO; 5];
                for branch in 0..4 {
                    values[branch] = input[inner + self.ido * (branch + 4 * outer)];
                }
                let transformed = recursive_radix4(&values, inverse);
                for branch in 0..4 {
                    let twiddle = directed_twiddle(self, branch, inner, inverse);
                    output[inner + self.ido * (outer + self.l1 * branch)] =
                        transformed[branch] * twiddle;
                }
            }
        }
    }

    fn direct(&self, input: &[C], output: &mut [C], inverse: bool) {
        for outer in 0..self.l1 {
            let input_base = self.ido * 4 * outer;
            let output_base = self.ido * outer;
            for inner in 0..self.ido {
                let x0 = input[input_base + inner];
                let x1 = input[input_base + self.ido + inner];
                let x2 = input[input_base + 2 * self.ido + inner];
                let x3 = input[input_base + 3 * self.ido + inner];
                let even_sum = x0 + x2;
                let even_difference = x0 - x2;
                let odd_sum = x1 + x3;
                let odd_difference = x1 - x3;
                let turn = quarter_turn(odd_difference, inverse);
                let y0 = even_sum + odd_sum;
                let y1 = even_difference + turn;
                let y2 = even_sum - odd_sum;
                let y3 = even_difference - turn;

                output[output_base + inner] = y0;
                for (branch, value) in [(1, y1), (2, y2), (3, y3)] {
                    output[output_base + self.ido * self.l1 * branch + inner] =
                        value * directed_twiddle(self, branch, inner, inverse);
                }
            }
        }
    }
}

fn recursive_radix4(values: &[C; 5], inverse: bool) -> [C; 4] {
    let even_sum = values[0] + values[2];
    let even_difference = values[0] - values[2];
    let odd_sum = values[1] + values[3];
    let odd_difference = values[1] - values[3];
    let turn = quarter_turn(odd_difference, false);
    let forward = [
        even_sum + odd_sum,
        even_difference + turn,
        even_sum - odd_sum,
        even_difference - turn,
    ];
    if inverse {
        [forward[0], forward[3], forward[2], forward[1]]
    } else {
        forward
    }
}

fn quarter_turn(value: C, inverse: bool) -> C {
    if inverse {
        C::new(-value.im, value.re)
    } else {
        C::new(value.im, -value.re)
    }
}

fn directed_twiddle(stage: &Radix4Stage, branch: usize, inner: usize, inverse: bool) -> C {
    let twiddle = stage.twiddles[branch * stage.ido + inner];
    if inverse { twiddle.conj() } else { twiddle }
}

fn input_value(index: usize) -> C {
    let x = (index + 1) as f64;
    C::new((0.173 * x).sin(), (0.113 * (x + 1.0)).cos())
}

fn elapsed_ns(stage: &Radix4Stage, input: &[C], inverse: bool, direct: bool) -> f64 {
    let mut output = vec![ZERO; stage.len];
    let start = Instant::now();
    for _ in 0..REPEATS {
        if direct {
            stage.direct(input, &mut output, inverse);
        } else {
            stage.generic(input, &mut output, inverse);
        }
        black_box(output.as_slice());
    }
    start.elapsed().as_nanos() as f64 / REPEATS as f64
}

fn verify(stage: &Radix4Stage, input: &[C], inverse: bool) {
    let mut generic = vec![ZERO; stage.len];
    let mut direct = vec![ZERO; stage.len];
    stage.generic(input, &mut generic, inverse);
    stage.direct(input, &mut direct, inverse);
    assert!(
        generic
            .iter()
            .chain(direct.iter())
            .all(|value| value.re.is_finite() && value.im.is_finite()),
        "nonfinite stage result at n={}",
        stage.len
    );
    let error = generic
        .iter()
        .zip(direct.iter())
        .map(|(&left, &right)| (left - right).norm())
        .fold(0.0_f64, f64::max);
    assert!(error < 2.0e-12 * stage.len as f64, "error={error}");
}

fn main() {
    let lengths: Vec<usize> = std::env::args()
        .skip(1)
        .filter(|arg| arg != "--bench")
        .map(|arg| arg.parse().expect("length must be an integer"))
        .collect();
    if lengths.is_empty() {
        return;
    }
    println!("mode,n,l1,ido,generic_ns,direct_ns,speedup,checksum");
    for len in lengths {
        assert_eq!(len % 4, 0);
        let l1 = (len / 4).next_power_of_two().clamp(1, 64);
        assert_eq!((len / (4 * l1)) * (4 * l1), len);
        let ido = len / (4 * l1);
        let stage = Radix4Stage::new(len, l1, ido);
        let input: Vec<_> = (0..len).map(input_value).collect();
        for inverse in [false, true] {
            verify(&stage, &input, inverse);
            let mut generic = Vec::with_capacity(SAMPLES);
            let mut direct = Vec::with_capacity(SAMPLES);
            for _ in 0..SAMPLES {
                generic.push(elapsed_ns(&stage, &input, inverse, false));
                direct.push(elapsed_ns(&stage, &input, inverse, true));
            }
            generic.sort_by(f64::total_cmp);
            direct.sort_by(f64::total_cmp);
            let generic_ns = generic[SAMPLES / 2];
            let direct_ns = direct[SAMPLES / 2];
            let mut output = vec![ZERO; stage.len];
            stage.direct(&input, &mut output, inverse);
            let checksum = output[0].re + output[0].im;
            println!(
                "{},{},{},{},{:.3},{:.3},{:.3},{:.9}",
                if inverse {
                    "c2r_radix4_stage"
                } else {
                    "r2c_radix4_stage"
                },
                len,
                l1,
                ido,
                generic_ns,
                direct_ns,
                generic_ns / direct_ns,
                checksum
            );
        }
    }
}
