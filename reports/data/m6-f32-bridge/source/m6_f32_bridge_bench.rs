// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::hint::black_box;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rustsht::{Complex32, Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn widen(values: &[Complex32]) -> Vec<Complex64> {
    values
        .iter()
        .map(|value| Complex64::new(f64::from(value.re), f64::from(value.im)))
        .collect()
}

fn median(values: &[Duration]) -> Duration {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

#[derive(Clone, Copy)]
enum Variant {
    Native,
    Bridge,
    F64,
}

fn measure(mut operation: impl FnMut(Variant)) -> (Vec<Duration>, Vec<Duration>, Vec<Duration>) {
    operation(Variant::Native);
    operation(Variant::Bridge);
    operation(Variant::F64);
    let mut native_samples = Vec::with_capacity(5);
    let mut bridge_samples = Vec::with_capacity(5);
    let mut f64_samples = Vec::with_capacity(5);
    for pair in 0..5 {
        if pair % 2 == 0 {
            native_samples.push(time_variant(&mut operation, Variant::Native));
            bridge_samples.push(time_variant(&mut operation, Variant::Bridge));
            f64_samples.push(time_variant(&mut operation, Variant::F64));
        } else {
            f64_samples.push(time_variant(&mut operation, Variant::F64));
            bridge_samples.push(time_variant(&mut operation, Variant::Bridge));
            native_samples.push(time_variant(&mut operation, Variant::Native));
        }
    }
    (native_samples, bridge_samples, f64_samples)
}

fn time_variant(operation: &mut impl FnMut(Variant), variant: Variant) -> Duration {
    let start = Instant::now();
    operation(variant);
    start.elapsed()
}

fn print_samples(name: &str, samples: &[Duration]) {
    let values = samples
        .iter()
        .map(|sample| format!("{:.3}", sample.as_secs_f64() * 1000.0))
        .collect::<Vec<_>>()
        .join(";");
    println!(
        "{name}_samples_ms={values} median_ms={:.3}",
        median(samples).as_secs_f64() * 1000.0
    );
}

fn make_coefficients(lmax: usize, spin: usize, channel: usize) -> Vec<Complex32> {
    let count = (lmax + 1) * (lmax + 2) / 2;
    let mut values = vec![Complex32::new(0.0, 0.0); count];
    for m in 0..=lmax {
        for l in m.max(spin)..=lmax {
            let index = m * (2 * lmax + 1 - m) / 2 + l;
            let (real, imaginary) = if channel == 0 {
                (
                    (l * 7 + m * 11) % 19,
                    if m == 0 { 11 } else { (l * 13 + m * 5) % 23 },
                )
            } else {
                (
                    (l * 3 + m * 7) % 17,
                    if m == 0 { 14 } else { (l * 5 + m * 17) % 29 },
                )
            };
            let real_offset = if channel == 0 { 9 } else { 8 };
            let imaginary_offset = if channel == 0 { 11 } else { 14 };
            values[index] = Complex32::new(
                (real as i32 - real_offset) as f32 * 1.0e-5_f32,
                (imaginary as i32 - imaginary_offset) as f32 * 1.0e-5_f32,
            );
        }
    }
    values
}

fn make_map(lmax: usize, nphi: usize, channel: usize) -> Vec<f32> {
    let mut values = vec![0.0_f32; (lmax + 1) * nphi];
    for ring in 0..=lmax {
        for longitude in 0..nphi {
            let numerator = if channel == 0 {
                ((ring * 31 + longitude * 17) % 101) as i32 - 50
            } else {
                ((ring * 19 + longitude * 23 + 7) % 97) as i32 - 48
            };
            let denominator = if channel == 0 { 101.0_f32 } else { 97.0_f32 };
            values[ring * nphi + longitude] = numerator as f32 / denominator;
        }
    }
    values
}

fn relative_l2(left: &[f32], right: &[f32]) -> f64 {
    let mut difference = 0.0;
    let mut reference = 0.0;
    for (&left, &right) in left.iter().zip(right) {
        difference += (f64::from(left) - f64::from(right)).powi(2);
        reference += f64::from(right).powi(2);
    }
    difference.sqrt() / reference.sqrt()
}

fn write_f32(path: &Path, values: &[f32]) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());
    for value in values {
        writer.write_all(&value.to_le_bytes()).unwrap();
    }
}

fn write_f64(path: &Path, values: &[f64]) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());
    for value in values {
        writer.write_all(&value.to_le_bytes()).unwrap();
    }
}

fn write_c32(path: &Path, values: &[Complex32]) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());
    for value in values {
        writer.write_all(&value.re.to_le_bytes()).unwrap();
        writer.write_all(&value.im.to_le_bytes()).unwrap();
    }
}

fn write_c64(path: &Path, values: &[Complex64]) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());
    for value in values {
        writer.write_all(&value.re.to_le_bytes()).unwrap();
        writer.write_all(&value.im.to_le_bytes()).unwrap();
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let lmax = args.next().expect("lmax").parse::<usize>().unwrap();
    let spin = args.next().expect("spin").parse::<usize>().unwrap();
    let threads = args.next().expect("threads").parse::<usize>().unwrap();
    let operation = args.next().expect("operation");
    let dump_dir = PathBuf::from(args.next().expect("dump directory"));
    assert!(spin <= 1);
    assert!(threads > 0);
    assert!(operation == "synthesis" || operation == "analysis");
    std::fs::create_dir_all(&dump_dir).unwrap();

    let ntheta = lmax + 1;
    let nphi = 2 * (lmax + 1);
    let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap();
    let harmonics = HarmonicSpec::new(lmax, lmax, spin).unwrap();
    let plan32 = ShtPlan::<f32>::new(grid.clone(), harmonics.clone()).unwrap();
    let plan64 = ShtPlan::<f64>::new(grid, harmonics).unwrap();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    let (mut workspace32, mut workspace64) =
        pool.install(|| (plan32.workspace().unwrap(), plan64.workspace().unwrap()));
    pool.install(|| {
        plan32.prepare_workspace(&mut workspace32, threads).unwrap();
        plan64.prepare_workspace(&mut workspace64, threads).unwrap();
    });

    let alm32_0 = make_coefficients(lmax, spin, 0);
    let alm32_1 = if spin == 1 {
        make_coefficients(lmax, spin, 1)
    } else {
        Vec::new()
    };
    let alm64_0 = widen(&alm32_0);
    let alm64_1 = widen(&alm32_1);
    let map32_0 = make_map(lmax, nphi, 0);
    let map32_1 = if spin == 1 {
        make_map(lmax, nphi, 1)
    } else {
        Vec::new()
    };
    let map64_0: Vec<_> = map32_0.iter().copied().map(f64::from).collect();
    let map64_1: Vec<_> = map32_1.iter().copied().map(f64::from).collect();
    let map_len = ntheta * nphi;
    let nalm = plan32.harmonics().nalm();
    let prefix = format!("l{lmax}-s{spin}-q{threads}");

    println!(
        "spin={spin} T{lmax}; GL {ntheta}x{nphi}; phi0=0.37; q={threads}; operation={operation}; five alternating in-process samples; Rust 1.99"
    );
    println!(
        "bridge=exact f32-to-f64 widening, f64 ShtPlan, per-call f64 temporary allocations, explicit f64-to-f32 output conversion"
    );
    let bridge_temporary_bytes = (if spin == 0 { 1 } else { 2 })
        * (nalm * std::mem::size_of::<Complex64>() + map_len * std::mem::size_of::<f64>());
    println!("bridge_temporary_input_output_bytes={bridge_temporary_bytes}");

    if operation == "synthesis" {
        let mut native0 = vec![0.0_f32; map_len];
        let mut native1 = if spin == 1 {
            vec![0.0_f32; map_len]
        } else {
            Vec::new()
        };
        let mut bridge0 = vec![0.0_f32; map_len];
        let mut bridge1 = if spin == 1 {
            vec![0.0_f32; map_len]
        } else {
            Vec::new()
        };
        let mut f64_output0 = vec![0.0_f64; map_len];
        let mut f64_output1 = if spin == 1 {
            vec![0.0_f64; map_len]
        } else {
            Vec::new()
        };

        let (native_samples, bridge_samples, f64_samples) = measure(|variant| match variant {
            Variant::Native => pool.install(|| {
                if spin == 0 {
                    plan32
                        .synthesis_into(&alm32_0, &mut native0, &mut workspace32)
                        .unwrap();
                } else {
                    plan32
                        .synthesis_spin_into(
                            &alm32_0,
                            &alm32_1,
                            &mut native0,
                            &mut native1,
                            &mut workspace32,
                        )
                        .unwrap();
                }
                black_box(native0[0]);
            }),
            Variant::Bridge => {
                let wide0 = widen(&alm32_0);
                let wide1 = widen(&alm32_1);
                let mut output0 = vec![0.0_f64; map_len];
                let mut output1 = if spin == 1 {
                    vec![0.0_f64; map_len]
                } else {
                    Vec::new()
                };
                pool.install(|| {
                    if spin == 0 {
                        plan64
                            .synthesis_into(&wide0, &mut output0, &mut workspace64)
                            .unwrap();
                    } else {
                        plan64
                            .synthesis_spin_into(
                                &wide0,
                                &wide1,
                                &mut output0,
                                &mut output1,
                                &mut workspace64,
                            )
                            .unwrap();
                    }
                    black_box(output0[0]);
                });
                for (target, &source) in bridge0.iter_mut().zip(&output0) {
                    *target = source as f32;
                }
                for (target, &source) in bridge1.iter_mut().zip(&output1) {
                    *target = source as f32;
                }
                black_box(bridge0[0]);
            }
            Variant::F64 => pool.install(|| {
                if spin == 0 {
                    plan64
                        .synthesis_into(&alm64_0, &mut f64_output0, &mut workspace64)
                        .unwrap();
                } else {
                    plan64
                        .synthesis_spin_into(
                            &alm64_0,
                            &alm64_1,
                            &mut f64_output0,
                            &mut f64_output1,
                            &mut workspace64,
                        )
                        .unwrap();
                }
                black_box(f64_output0[0]);
            }),
        });
        print_samples("native_f32", &native_samples);
        print_samples("f32_through_f64_bridge", &bridge_samples);
        print_samples("native_f64_rounded_inputs", &f64_samples);

        println!(
            "channel0_native_bridge_relative_l2={:.8e}",
            relative_l2(&native0, &bridge0)
        );
        write_f32(
            &dump_dir.join(format!("{prefix}-native-synthesis-0.f32")),
            &native0,
        );
        write_f32(
            &dump_dir.join(format!("{prefix}-bridge-synthesis-0.f32")),
            &bridge0,
        );
        write_f64(
            &dump_dir.join(format!("{prefix}-f64-synthesis-0.f64")),
            &f64_output0,
        );
        if spin == 1 {
            println!(
                "channel1_native_bridge_relative_l2={:.8e}",
                relative_l2(&native1, &bridge1)
            );
            write_f32(
                &dump_dir.join(format!("{prefix}-native-synthesis-1.f32")),
                &native1,
            );
            write_f32(
                &dump_dir.join(format!("{prefix}-bridge-synthesis-1.f32")),
                &bridge1,
            );
            write_f64(
                &dump_dir.join(format!("{prefix}-f64-synthesis-1.f64")),
                &f64_output1,
            );
        }
    } else {
        let mut native0 = vec![Complex32::new(0.0, 0.0); nalm];
        let mut native1 = if spin == 1 {
            vec![Complex32::new(0.0, 0.0); nalm]
        } else {
            Vec::new()
        };
        let mut bridge0 = vec![Complex32::new(0.0, 0.0); nalm];
        let mut bridge1 = if spin == 1 {
            vec![Complex32::new(0.0, 0.0); nalm]
        } else {
            Vec::new()
        };
        let mut f64_output0 = vec![Complex64::new(0.0, 0.0); nalm];
        let mut f64_output1 = if spin == 1 {
            vec![Complex64::new(0.0, 0.0); nalm]
        } else {
            Vec::new()
        };

        let (native_samples, bridge_samples, f64_samples) = measure(|variant| match variant {
            Variant::Native => pool.install(|| {
                if spin == 0 {
                    plan32
                        .analysis_into(&map32_0, &mut native0, &mut workspace32)
                        .unwrap();
                } else {
                    plan32
                        .analysis_spin_into(
                            &map32_0,
                            &map32_1,
                            &mut native0,
                            &mut native1,
                            &mut workspace32,
                        )
                        .unwrap();
                }
                black_box(native0[0]);
            }),
            Variant::Bridge => {
                let wide0: Vec<_> = map32_0.iter().copied().map(f64::from).collect();
                let wide1: Vec<_> = map32_1.iter().copied().map(f64::from).collect();
                let mut output0 = vec![Complex64::new(0.0, 0.0); nalm];
                let mut output1 = if spin == 1 {
                    vec![Complex64::new(0.0, 0.0); nalm]
                } else {
                    Vec::new()
                };
                pool.install(|| {
                    if spin == 0 {
                        plan64
                            .analysis_into(&wide0, &mut output0, &mut workspace64)
                            .unwrap();
                    } else {
                        plan64
                            .analysis_spin_into(
                                &wide0,
                                &wide1,
                                &mut output0,
                                &mut output1,
                                &mut workspace64,
                            )
                            .unwrap();
                    }
                    black_box(output0[0]);
                });
                for (target, source) in bridge0.iter_mut().zip(&output0) {
                    *target = Complex32::new(source.re as f32, source.im as f32);
                }
                for (target, source) in bridge1.iter_mut().zip(&output1) {
                    *target = Complex32::new(source.re as f32, source.im as f32);
                }
                black_box(bridge0[0]);
            }
            Variant::F64 => pool.install(|| {
                if spin == 0 {
                    plan64
                        .analysis_into(&map64_0, &mut f64_output0, &mut workspace64)
                        .unwrap();
                } else {
                    plan64
                        .analysis_spin_into(
                            &map64_0,
                            &map64_1,
                            &mut f64_output0,
                            &mut f64_output1,
                            &mut workspace64,
                        )
                        .unwrap();
                }
                black_box(f64_output0[0]);
            }),
        });
        print_samples("native_f32", &native_samples);
        print_samples("f32_through_f64_bridge", &bridge_samples);
        print_samples("native_f64_rounded_inputs", &f64_samples);

        println!(
            "channel0_native_bridge_relative_l2={:.8e}",
            relative_l2_complex(&native0, &bridge0)
        );
        write_c32(
            &dump_dir.join(format!("{prefix}-native-analysis-0.c32")),
            &native0,
        );
        write_c32(
            &dump_dir.join(format!("{prefix}-bridge-analysis-0.c32")),
            &bridge0,
        );
        write_c64(
            &dump_dir.join(format!("{prefix}-f64-analysis-0.c128")),
            &f64_output0,
        );
        if spin == 1 {
            println!(
                "channel1_native_bridge_relative_l2={:.8e}",
                relative_l2_complex(&native1, &bridge1)
            );
            write_c32(
                &dump_dir.join(format!("{prefix}-native-analysis-1.c32")),
                &native1,
            );
            write_c32(
                &dump_dir.join(format!("{prefix}-bridge-analysis-1.c32")),
                &bridge1,
            );
            write_c64(
                &dump_dir.join(format!("{prefix}-f64-analysis-1.c128")),
                &f64_output1,
            );
        }
    }
}

fn relative_l2_complex(left: &[Complex32], right: &[Complex32]) -> f64 {
    let mut difference = 0.0;
    let mut reference = 0.0;
    for (&left, &right) in left.iter().zip(right) {
        difference += (f64::from(left.re) - f64::from(right.re)).powi(2)
            + (f64::from(left.im) - f64::from(right.im)).powi(2);
        reference += f64::from(right.re).powi(2) + f64::from(right.im).powi(2);
    }
    difference.sqrt() / reference.sqrt()
}
