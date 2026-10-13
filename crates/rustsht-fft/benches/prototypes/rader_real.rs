// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// Experimental, real-only Rader prototype for p=241,257 and N=3*241.
// This benchmark does not change production planner selection. The Rader
// equations are implemented independently; no Admiral source is copied.

use rustfft::FftPlanner;
use rustsht_fft::{Complex64, ComplexFftPlan, ComplexFftWorkspace, RealFftPlan};
use std::f64::consts::TAU;
use std::hint::black_box;
use std::time::Instant;

type C = Complex64;

struct PrimePlan {
    p: usize,
    gpow: Vec<usize>,
    ginvpow: Vec<usize>,
    kernel_fwd: Vec<C>,
    kernel_inv: Vec<C>,
    inner: ComplexFftPlan<f64>,
}
struct PrimeScratch {
    conv: Vec<C>,
    inner: ComplexFftWorkspace<f64>,
}
impl PrimePlan {
    fn new(p: usize) -> Self {
        let l = p - 1;
        let g = primitive_root(p);
        let mut gpow = vec![1; l];
        for q in 1..l {
            gpow[q] = (gpow[q - 1] * g) % p;
        }
        let mut ginvpow = vec![1; l];
        for q in 0..l {
            ginvpow[q] = gpow[(l - q) % l];
        }
        let inner = ComplexFftPlan::<f64>::new(l).unwrap();
        let mut ws = inner.workspace().unwrap();
        let mut kernels = Vec::with_capacity(2);
        for sign in [-1.0, 1.0] {
            let mut b = vec![C::new(0.0, 0.0); l];
            for j in 0..l {
                let angle = sign * TAU * ginvpow[j] as f64 / p as f64;
                b[j] = C::new(angle.cos(), angle.sin());
            }
            inner.forward(&mut b, &mut ws).unwrap();
            kernels.push(b);
        }
        Self {
            p,
            gpow,
            ginvpow,
            kernel_fwd: kernels.remove(0),
            kernel_inv: kernels.remove(0),
            inner,
        }
    }
    fn workspace(&self) -> PrimeScratch {
        PrimeScratch {
            conv: vec![C::new(0.0, 0.0); self.p - 1],
            inner: self.inner.workspace().unwrap(),
        }
    }
    fn run(&self, input: &[C], output: &mut [C], inverse: bool, ws: &mut PrimeScratch) {
        let l = self.p - 1;
        let x0 = input[0];
        let mut sum = x0;
        for q in 0..l {
            let x = input[self.gpow[q]];
            ws.conv[q] = x;
            sum += x;
        }
        self.inner.forward(&mut ws.conv, &mut ws.inner).unwrap();
        let kernel = if inverse {
            &self.kernel_inv
        } else {
            &self.kernel_fwd
        };
        for (a, b) in ws.conv.iter_mut().zip(kernel) {
            *a *= *b;
        }
        self.inner.inverse(&mut ws.conv, &mut ws.inner).unwrap();
        let scale = if inverse { 1.0 / self.p as f64 } else { 1.0 };
        output[0] = sum * scale;
        for m in 0..l {
            output[self.ginvpow[m]] = (x0 + ws.conv[m]) * scale;
        }
    }
    fn plan_bytes(&self) -> usize {
        self.gpow.capacity() * std::mem::size_of::<usize>()
            + self.ginvpow.capacity() * std::mem::size_of::<usize>()
            + self.kernel_fwd.capacity() * std::mem::size_of::<C>()
            + self.kernel_inv.capacity() * std::mem::size_of::<C>()
    }
}
fn primitive_root(p: usize) -> usize {
    let mut factors = Vec::new();
    let mut n = p - 1;
    let mut d = 2;
    while d * d <= n {
        if n.is_multiple_of(d) {
            factors.push(d);
            while n.is_multiple_of(d) {
                n /= d;
            }
        }
        d += 1;
    }
    if n > 1 {
        factors.push(n);
    }
    'candidate: for g in 2..p {
        for &q in &factors {
            if powmod(g, (p - 1) / q, p) == 1 {
                continue 'candidate;
            }
        }
        return g;
    }
    unreachable!()
}
fn powmod(mut a: usize, mut b: usize, m: usize) -> usize {
    let mut out = 1;
    while b > 0 {
        if b & 1 != 0 {
            out = out * a % m;
        }
        a = a * a % m;
        b >>= 1;
    }
    out
}

struct RaderRealPlan {
    n: usize,
    prime: usize,
    inner_prime: PrimePlan,
    mix_fwd: Vec<C>,
    mix_inv: Vec<C>,
    radix3_inv: [C; 9],
}
struct RaderRealWorkspace {
    prime: PrimeScratch,
    complex_in: Vec<C>,
    complex_out: Vec<C>,
    prime_in: Vec<C>,
    prime_out: Vec<C>,
    partial: Vec<C>,
}
impl RaderRealPlan {
    fn new(n: usize) -> Self {
        let prime = if is_prime(n) {
            n
        } else if n == 3 * 241 {
            241
        } else {
            panic!("unsupported n={n}")
        };
        let inner_prime = PrimePlan::new(prime);
        let (mix_fwd, mix_inv, radix3_inv) = if n == prime {
            (Vec::new(), Vec::new(), [C::new(0.0, 0.0); 9])
        } else {
            let mut mf = vec![C::new(0.0, 0.0); 3 * n];
            for r in 0..3 {
                for k in 0..n {
                    let a = -TAU * (r * k) as f64 / n as f64;
                    mf[r * n + k] = C::new(a.cos(), a.sin());
                }
            }
            let mut mi = vec![C::new(0.0, 0.0); 3 * prime];
            for r in 0..3 {
                for q in 0..prime {
                    let a = TAU * (r * q) as f64 / n as f64;
                    mi[r * prime + q] = C::new(a.cos(), a.sin());
                }
            }
            let mut r3 = [C::new(0.0, 0.0); 9];
            for r in 0..3 {
                for t in 0..3 {
                    let a = TAU * (r * t) as f64 / 3.0;
                    r3[r * 3 + t] = C::new(a.cos(), a.sin());
                }
            }
            (mf, mi, r3)
        };
        Self {
            n,
            prime,
            inner_prime,
            mix_fwd,
            mix_inv,
            radix3_inv,
        }
    }
    fn workspace(&self) -> RaderRealWorkspace {
        let p = self.prime;
        RaderRealWorkspace {
            prime: self.inner_prime.workspace(),
            complex_in: vec![C::new(0.0, 0.0); self.n],
            complex_out: vec![C::new(0.0, 0.0); self.n],
            prime_in: vec![C::new(0.0, 0.0); p],
            prime_out: vec![C::new(0.0, 0.0); p],
            partial: vec![C::new(0.0, 0.0); if self.n == p { 0 } else { 3 * p }],
        }
    }
    fn forward(&self, input: &[f64], output: &mut [C], ws: &mut RaderRealWorkspace) {
        if self.n == self.prime {
            for (destination, &sample) in ws.complex_in.iter_mut().zip(input) {
                *destination = C::new(sample, 0.0);
            }
            self.inner_prime
                .run(&ws.complex_in, &mut ws.complex_out, false, &mut ws.prime);
        } else {
            let p = self.prime;
            for r in 0..3 {
                for s in 0..p {
                    ws.prime_in[s] = C::new(input[3 * s + r], 0.0);
                }
                self.inner_prime
                    .run(&ws.prime_in, &mut ws.prime_out, false, &mut ws.prime);
                ws.partial[r * p..(r + 1) * p].copy_from_slice(&ws.prime_out);
            }
            for k in 0..self.n {
                let mut sum = C::new(0.0, 0.0);
                for r in 0..3 {
                    sum += ws.partial[r * p + k % p] * self.mix_fwd[r * self.n + k];
                }
                ws.complex_out[k] = sum;
            }
        }
        let h = self.n / 2 + 1;
        output.copy_from_slice(&ws.complex_out[..h]);
    }
    fn inverse(&self, spectrum: &[C], output: &mut [f64], ws: &mut RaderRealWorkspace) {
        ws.complex_in[0] = spectrum[0];
        for (k, &value) in spectrum.iter().enumerate().take(self.n / 2 + 1).skip(1) {
            ws.complex_in[k] = value;
            ws.complex_in[self.n - k] = value.conj();
        }
        if self.n == self.prime {
            self.inner_prime
                .run(&ws.complex_in, &mut ws.complex_out, true, &mut ws.prime);
        } else {
            let p = self.prime;
            for r in 0..3 {
                for q in 0..p {
                    let mut sum = C::new(0.0, 0.0);
                    for t in 0..3 {
                        sum += ws.complex_in[q + t * p] * self.radix3_inv[r * 3 + t];
                    }
                    ws.prime_in[q] = sum * self.mix_inv[r * p + q];
                }
                self.inner_prime
                    .run(&ws.prime_in, &mut ws.prime_out, true, &mut ws.prime);
                for s in 0..p {
                    output[3 * s + r] = ws.prime_out[s].re / 3.0;
                }
            }
            return;
        }
        for (destination, value) in output.iter_mut().zip(&ws.complex_out) {
            *destination = value.re;
        }
    }
    fn workspace_bytes(&self, ws: &RaderRealWorkspace) -> usize {
        let vecs = [
            &ws.complex_in,
            &ws.complex_out,
            &ws.prime_in,
            &ws.prime_out,
            &ws.partial,
        ];
        vecs.iter()
            .map(|v| v.capacity() * std::mem::size_of::<C>())
            .sum::<usize>()
            + ws.prime.conv.capacity() * std::mem::size_of::<C>()
            + ws.prime.inner.scratch_bytes().unwrap()
    }
    fn plan_bytes(&self) -> usize {
        self.inner_prime.plan_bytes()
            + (self.mix_fwd.capacity() + self.mix_inv.capacity() + self.radix3_inv.len())
                * std::mem::size_of::<C>()
    }
}
fn is_prime(n: usize) -> bool {
    if n < 2 {
        return false;
    }
    (2..=((n as f64).sqrt() as usize)).all(|d| !n.is_multiple_of(d))
}

fn direct(input: &[C], inverse: bool) -> Vec<C> {
    let n = input.len();
    let sign = if inverse { 1.0 } else { -1.0 };
    (0..n)
        .map(|k| {
            (0..n).fold(C::new(0.0, 0.0), |s, j| {
                let a = sign * TAU * (j * k) as f64 / n as f64;
                s + input[j] * C::new(a.cos(), a.sin())
            }) * if inverse { 1.0 / n as f64 } else { 1.0 }
        })
        .collect()
}
fn rustfft_real_forward(input: &[f64]) -> Vec<C> {
    let n = input.len();
    let mut values: Vec<C> = input.iter().map(|&x| C::new(x, 0.0)).collect();
    FftPlanner::<f64>::new()
        .plan_fft_forward(n)
        .process(&mut values);
    values[..n / 2 + 1].to_vec()
}
fn rustfft_real_inverse(spectrum: &[C], n: usize) -> Vec<f64> {
    let mut values = vec![C::new(0.0, 0.0); n];
    values[0] = C::new(spectrum[0].re, 0.0);
    for (k, &value) in spectrum.iter().enumerate().take(n / 2 + 1).skip(1) {
        values[k] = value;
        values[n - k] = value.conj();
    }
    FftPlanner::<f64>::new()
        .plan_fft_inverse(n)
        .process(&mut values);
    values.iter().map(|x| x.re / n as f64).collect()
}
fn norm_err(a: &[C], b: &[C]) -> (f64, f64) {
    let mut max: f64 = 0.0;
    let mut rms = 0.0;
    let mut scale: f64 = 0.0;
    for (x, y) in a.iter().zip(b) {
        let e = (*x - *y).norm();
        max = max.max(e);
        rms += e * e;
        scale = scale.max(y.norm());
    }
    (
        max / scale.max(f64::MIN_POSITIVE),
        (rms / a.len() as f64).sqrt() / scale.max(f64::MIN_POSITIVE),
    )
}
fn validate() {
    for p in [241, 257] {
        let plan = PrimePlan::new(p);
        let mut ws = plan.workspace();
        let mut input = Vec::new();
        for j in 0..p {
            input.push(C::new(
                ((j * 31 % 127) as f64 - 63.0) / 71.0,
                ((j * 19 % 89) as f64 - 44.0) / 93.0,
            ));
        }
        for inverse in [false, true] {
            let mut got = vec![C::new(0.0, 0.0); p];
            plan.run(&input, &mut got, inverse, &mut ws);
            let expect = direct(&input, inverse);
            let (e, r) = norm_err(&got, &expect);
            assert!(
                e < 2e-13 && r < 5e-14,
                "prime {p} inverse={inverse} err={e} rms={r}"
            );
        }
        // Cancellation-heavy and high dynamic range input. Require finite output and compare to the production real plan at p.
        for pattern in 0..4 {
            let x: Vec<f64> = (0..p)
                .map(|j| match pattern {
                    0 => {
                        if j % 2 == 0 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    1 => {
                        if j % 7 == 0 {
                            1.0
                        } else {
                            1e-12
                        }
                    }
                    2 => {
                        if j % 2 == 0 {
                            f64::MAX / (4.0 * p as f64)
                        } else {
                            -f64::MAX / (4.0 * p as f64)
                        }
                    }
                    _ => {
                        if j % 2 == 0 {
                            f64::MIN_POSITIVE
                        } else {
                            f64::from_bits(1)
                        }
                    }
                })
                .collect();
            let rplan = RaderRealPlan::new(p);
            let mut rw = rplan.workspace();
            let mut got = vec![C::new(0.0, 0.0); p / 2 + 1];
            rplan.forward(&x, &mut got, &mut rw);
            assert!(
                got.iter().all(|v| v.re.is_finite() && v.im.is_finite()),
                "nonfinite p={p} pattern={pattern}"
            );
            let prod = RealFftPlan::<f64>::new(p).unwrap();
            let mut pw = prod.workspace().unwrap();
            let mut refout = vec![C::new(0.0, 0.0); p / 2 + 1];
            prod.forward(&x, &mut refout, &mut pw).unwrap();
            let (e, r) = norm_err(&got, &refout);
            assert!(
                e < 3e-12,
                "real p={p} pattern={pattern} normalized max error={e} rms={r}"
            );
        }
        println!("validated prime={p}");
    }
    for n in [257, 723] {
        let p = RaderRealPlan::new(n);
        let mut rw = p.workspace();
        let prod = RealFftPlan::<f64>::new(n).unwrap();
        let mut pw = prod.workspace().unwrap();
        for pattern in 0..5 {
            let input: Vec<f64> = (0..n)
                .map(|j| match pattern {
                    0 => ((j * 37 % 251) as f64 - 125.0) / 47.0,
                    1 => {
                        if j % 2 == 0 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    2 => {
                        if j % 7 == 0 {
                            1.0
                        } else {
                            1e-12
                        }
                    }
                    3 => {
                        if j % 2 == 0 {
                            f64::MAX / (4.0 * n as f64)
                        } else {
                            -f64::MAX / (4.0 * n as f64)
                        }
                    }
                    _ => {
                        if j % 2 == 0 {
                            f64::MIN_POSITIVE
                        } else {
                            f64::from_bits(1)
                        }
                    }
                })
                .collect();
            let mut got = vec![C::new(0.0, 0.0); n / 2 + 1];
            let mut refout = got.clone();
            p.forward(&input, &mut got, &mut rw);
            prod.forward(&input, &mut refout, &mut pw).unwrap();
            assert!(
                got.iter().all(|v| v.re.is_finite() && v.im.is_finite()),
                "forward nonfinite n={n} pattern={pattern}"
            );
            let (e, r) = norm_err(&got, &refout);
            assert!(
                e < 5e-12,
                "real forward n={n} pattern={pattern} relative max={e} rms={r}"
            );
            if pattern < 2 {
                let rustfft = rustfft_real_forward(&input);
                let (re, rr) = norm_err(&got, &rustfft);
                assert!(re < 5e-13, "RustFFT R2C n={n} max={re} rms={rr}");
                let expected = direct(
                    &input.iter().map(|&x| C::new(x, 0.0)).collect::<Vec<_>>(),
                    false,
                );
                let (de, dr) = norm_err(&got, &expected[..n / 2 + 1]);
                assert!(
                    de < 5e-13,
                    "direct real forward n={n} pattern={pattern} max={de} rms={dr}"
                );
            }
            let mut inv = vec![0.0; n];
            let mut inv_ref = vec![0.0; n];
            p.inverse(&got, &mut inv, &mut rw);
            prod.inverse(&got, &mut inv_ref, &mut pw).unwrap();
            let scale = inv_ref
                .iter()
                .map(|v| v.abs())
                .fold(f64::MIN_POSITIVE, f64::max);
            let err = inv
                .iter()
                .zip(&inv_ref)
                .map(|(x, y)| (x - y).abs())
                .fold(0.0, f64::max)
                / scale;
            assert!(
                err < 5e-12,
                "real inverse n={n} pattern={pattern} scaled error={err}"
            );
            if pattern < 2 {
                let mut full = vec![C::new(0.0, 0.0); n];
                full[0] = got[0];
                for k in 1..=n / 2 {
                    full[k] = got[k];
                    full[n - k] = got[k].conj();
                }
                let expected = direct(&full, true);
                let scale = expected
                    .iter()
                    .map(|v| v.re.abs())
                    .fold(f64::MIN_POSITIVE, f64::max);
                let de = inv
                    .iter()
                    .zip(&expected)
                    .map(|(x, y)| (x - y.re).abs())
                    .fold(0.0, f64::max)
                    / scale;
                assert!(
                    de < 5e-13,
                    "direct real inverse n={n} pattern={pattern} scaled error={de}"
                );
            }
        }
        for pattern in 0..5 {
            let mut spec: Vec<C> = (0..=n / 2)
                .map(|k| match pattern {
                    0 => spectrum_value(k, n, 0),
                    1 => C::new(
                        if k % 2 == 0 { 1.0 } else { -1.0 },
                        if k % 3 == 0 { 0.5 } else { -0.5 },
                    ),
                    2 => C::new(
                        if k % 7 == 0 { 1.0 } else { 1e-12 },
                        if k % 11 == 0 { -0.75 } else { 2e-13 },
                    ),
                    3 => C::new(
                        if k % 2 == 0 {
                            f64::MAX / (8.0 * n as f64)
                        } else {
                            -f64::MAX / (8.0 * n as f64)
                        },
                        if k % 3 == 0 {
                            f64::MAX / (16.0 * n as f64)
                        } else {
                            -f64::MAX / (16.0 * n as f64)
                        },
                    ),
                    _ => C::new(
                        if k % 2 == 0 {
                            f64::MIN_POSITIVE
                        } else {
                            f64::from_bits(1)
                        },
                        if k % 3 == 0 {
                            f64::from_bits(1)
                        } else {
                            -f64::from_bits(1)
                        },
                    ),
                })
                .collect();
            spec[0].im = 0.0;
            let mut got = vec![0.0; n];
            let mut refout = vec![0.0; n];
            p.inverse(&spec, &mut got, &mut rw);
            prod.inverse(&spec, &mut refout, &mut pw).unwrap();
            assert!(
                got.iter().all(|v| v.is_finite()),
                "inverse nonfinite n={n} pattern={pattern}"
            );
            let scale = spec
                .iter()
                .map(|z| z.norm())
                .fold(f64::MIN_POSITIVE, f64::max);
            let err = got
                .iter()
                .zip(&refout)
                .map(|(x, y)| (x - y).abs())
                .fold(0.0, f64::max)
                / scale;
            assert!(
                err < 5e-12,
                "real inverse n={n} spectrum pattern={pattern} scaled-by-spectrum error={err}"
            );
            if pattern < 2 {
                let rustfft = rustfft_real_inverse(&spec, n);
                let re = got
                    .iter()
                    .zip(&rustfft)
                    .map(|(x, y)| (x - y).abs())
                    .fold(0.0, f64::max)
                    / scale;
                assert!(re < 5e-13, "RustFFT C2R n={n} scaled error={re}");
                let mut full = vec![C::new(0.0, 0.0); n];
                full[0] = spec[0];
                for k in 1..=n / 2 {
                    full[k] = spec[k];
                    full[n - k] = spec[k].conj();
                }
                let expected = direct(&full, true);
                let de = got
                    .iter()
                    .zip(&expected)
                    .map(|(x, y)| (x - y.re).abs())
                    .fold(0.0, f64::max)
                    / scale;
                assert!(
                    de < 5e-13,
                    "direct c2r n={n} spectrum pattern={pattern} scaled error={de}"
                );
            }
        }
        println!(
            "validated R2C/C2R n={n}: direct DFT for normal/cancellation; production RustSHT-FFT comparison for normal, cancellation, dynamic-range, near-overflow and subnormal cases"
        );
    }
    println!("validated Rader prime 241,257 and mixed n=723 against independent direct DFTs");
}
fn dump_rader(direction: &str, n: usize) {
    assert!(matches!(direction, "r2c" | "c2r"));
    let plan = RaderRealPlan::new(n);
    let mut ws = plan.workspace();
    if direction == "r2c" {
        let input: Vec<_> = (0..n).map(|i| real_value(i, 0)).collect();
        let mut output = vec![C::new(0.0, 0.0); n / 2 + 1];
        plan.forward(&input, &mut output, &mut ws);
        for (k, value) in output.iter().enumerate() {
            println!("{direction},{n},{k},{:.17e},{:.17e}", value.re, value.im);
        }
    } else {
        let spectrum: Vec<_> = (0..=n / 2).map(|k| spectrum_value(k, n, 0)).collect();
        let mut output = vec![0.0; n];
        plan.inverse(&spectrum, &mut output, &mut ws);
        for (i, value) in output.iter().enumerate() {
            println!("{direction},{n},{i},{value:.17e},0.00000000000000000e0");
        }
    }
}
fn real_value(index: usize, row: usize) -> f64 {
    let x = (index + 1 + 3 * row) as f64;
    (0.173 * x).sin() + 0.25 * (0.113 * (x + 1.0)).cos()
}
fn spectrum_value(index: usize, n: usize, row: usize) -> C {
    if index == 0 || (n.is_multiple_of(2) && index == n / 2) {
        return C::new(0.4 + 0.01 * (index + row) as f64, 0.0);
    }
    let x = (index + 1 + 5 * row) as f64;
    C::new(0.3 * (0.19 * x).sin(), 0.2 * (0.07 * (x + 1.0)).cos())
}
fn median(mut x: Vec<u128>) -> u128 {
    x.sort_unstable();
    x[x.len() / 2]
}
fn benchmark(mode: &str, n: usize, rows: usize, dir: &str) {
    assert!(matches!(dir, "r2c" | "c2r"));
    let x: Vec<f64> = (0..rows)
        .flat_map(|r| (0..n).map(move |i| real_value(i, r)))
        .collect();
    let h = n / 2 + 1;
    let mut y: Vec<C> = (0..rows)
        .flat_map(|r| (0..h).map(move |k| spectrum_value(k, n, r)))
        .collect();
    let mut out = vec![0.0; n * rows];

    let mut plan_times = Vec::new();
    for _ in 0..7 {
        let t = Instant::now();
        if mode == "rader" {
            black_box(RaderRealPlan::new(n));
        } else {
            black_box(RealFftPlan::<f64>::new(n).unwrap());
        }
        plan_times.push(t.elapsed().as_nanos());
    }
    let setup_ns = median(plan_times);
    let (scratch_ns, scratch_bytes, plan_table_bytes);
    if mode == "rader" {
        let p = RaderRealPlan::new(n);
        let mut allocs = Vec::new();
        for _ in 0..7 {
            let t = Instant::now();
            black_box(p.workspace());
            allocs.push(t.elapsed().as_nanos());
        }
        scratch_ns = median(allocs);
        let mut ws = p.workspace();
        scratch_bytes = p.workspace_bytes(&ws);
        plan_table_bytes = p.plan_bytes();
        let mut samples = Vec::new();
        for rep in 0..7 {
            let t = Instant::now();
            for _ in 0..10 {
                for row in 0..rows {
                    if dir == "r2c" {
                        p.forward(
                            &x[row * n..(row + 1) * n],
                            &mut y[row * h..(row + 1) * h],
                            &mut ws,
                        );
                    } else {
                        p.inverse(
                            &y[row * h..(row + 1) * h],
                            &mut out[row * n..(row + 1) * n],
                            &mut ws,
                        );
                    }
                }
            }
            let elapsed = t.elapsed().as_nanos();
            if rep >= 2 {
                samples.push(elapsed / 10);
            }
        }
        let batch_ns = median(samples);
        let checksum = if dir == "r2c" {
            y.iter().map(|z| z.re).sum::<f64>()
        } else {
            out.iter().sum::<f64>()
        };
        println!(
            "mode={mode},n={n},rows={rows},dir={dir},plan_ns={setup_ns},workspace_ns={scratch_ns},rader_table_bytes={plan_table_bytes},scratch_bytes={scratch_bytes},batch_ns={batch_ns},ns_per_transform={:.3},checksum={checksum:.17e}",
            batch_ns as f64 / rows as f64
        );
    } else {
        let p = RealFftPlan::<f64>::new(n).unwrap();
        let mut allocs = Vec::new();
        for _ in 0..7 {
            let t = Instant::now();
            black_box(p.workspace().unwrap());
            allocs.push(t.elapsed().as_nanos());
        }
        scratch_ns = median(allocs);
        let mut ws = p.workspace().unwrap();
        scratch_bytes = ws.scratch_bytes().unwrap();
        plan_table_bytes = 0;
        let mut samples = Vec::new();
        for rep in 0..7 {
            let t = Instant::now();
            for _ in 0..10 {
                for row in 0..rows {
                    if dir == "r2c" {
                        p.forward(
                            &x[row * n..(row + 1) * n],
                            &mut y[row * h..(row + 1) * h],
                            &mut ws,
                        )
                        .unwrap();
                    } else {
                        p.inverse(
                            &y[row * h..(row + 1) * h],
                            &mut out[row * n..(row + 1) * n],
                            &mut ws,
                        )
                        .unwrap();
                    }
                }
            }
            let elapsed = t.elapsed().as_nanos();
            if rep >= 2 {
                samples.push(elapsed / 10);
            }
        }
        let batch_ns = median(samples);
        let checksum = if dir == "r2c" {
            y.iter().map(|z| z.re).sum::<f64>()
        } else {
            out.iter().sum::<f64>()
        };
        println!(
            "mode={mode},n={n},rows={rows},dir={dir},plan_ns={setup_ns},workspace_ns={scratch_ns},rader_table_bytes={plan_table_bytes},scratch_bytes={scratch_bytes},batch_ns={batch_ns},ns_per_transform={:.3},checksum={checksum:.17e}",
            batch_ns as f64 / rows as f64
        );
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("validate") {
        "validate" => validate(),
        "dump" => dump_rader(&args[2], args[3].parse().unwrap()),
        "bench" => {
            let mode = &args[2];
            let n = args[3].parse().unwrap();
            let rows = args[4].parse().unwrap();
            let dir = &args[5];
            benchmark(mode, n, rows, dir);
        }
        _ => panic!("use validate, dump r2c|c2r N, or bench MODE N ROWS r2c|c2r"),
    }
}
