// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

mod reusable_bridge;

use std::hint::black_box;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use reusable_bridge::{ReusableBridge, narrow_complex, narrow_real, widen_complex, widen_real};
use rustsht::{Complex32, Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

#[derive(Clone, Copy, Debug)]
enum Variant {
    NativeF32,
    AllocatingBridge,
    ReusableBridge,
    NativeF64,
}

impl Variant {
    const ALL: [Self; 4] = [
        Self::NativeF32,
        Self::AllocatingBridge,
        Self::ReusableBridge,
        Self::NativeF64,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::NativeF32 => "native_f32",
            Self::AllocatingBridge => "allocating_bridge",
            Self::ReusableBridge => "reusable_bridge",
            Self::NativeF64 => "native_f64_rounded_inputs",
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let first = args
        .next()
        .expect("use --check or LMAX SPIN THREADS OP DUMP_DIR");
    if first == "--check" {
        let dump_dir = args.next().map(PathBuf::from);
        check_all(dump_dir.as_deref());
        return;
    }

    let lmax = first.parse::<usize>().expect("lmax must be an integer");
    let spin = args.next().expect("spin").parse::<usize>().unwrap();
    let threads = args.next().expect("threads").parse::<usize>().unwrap();
    let operation = args.next().expect("operation");
    let dump_dir = PathBuf::from(args.next().expect("dump directory"));
    assert!(spin <= 1);
    assert!(threads == 1 || threads == 8);
    assert!(operation == "synthesis" || operation == "analysis");
    std::fs::create_dir_all(&dump_dir).unwrap();
    benchmark_cell(lmax, spin, threads, &operation, &dump_dir);
}

fn benchmark_cell(lmax: usize, spin: usize, threads: usize, operation: &str, dump_dir: &Path) {
    let ntheta = lmax + 1;
    let nphi = 2 * ntheta;
    let setup_start = Instant::now();
    let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap();
    let harmonics = HarmonicSpec::new(lmax, lmax, spin).unwrap();
    let plan32 = ShtPlan::<f32>::new(grid.clone(), harmonics.clone()).unwrap();
    let plan64 = ShtPlan::<f64>::new(grid, harmonics).unwrap();
    let plan_setup = setup_start.elapsed();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    let workspace_start = Instant::now();
    let mut workspace32 = pool.install(|| plan32.workspace()).unwrap();
    let mut workspace64 = pool.install(|| plan64.workspace()).unwrap();
    let mut bridge_workspace = pool.install(|| plan64.workspace()).unwrap();
    pool.install(|| {
        plan32.prepare_workspace(&mut workspace32, threads).unwrap();
        plan64.prepare_workspace(&mut workspace64, threads).unwrap();
        plan64
            .prepare_workspace(&mut bridge_workspace, threads)
            .unwrap();
    });
    let mut bridge = ReusableBridge::new(&plan64, bridge_workspace);
    let workspace_setup = workspace_start.elapsed();

    let alm32_0 = make_coefficients(lmax, spin, 0);
    let alm32_1 = if spin == 1 {
        make_coefficients(lmax, spin, 1)
    } else {
        Vec::new()
    };
    let map32_0 = make_map(lmax, nphi, 0);
    let map32_1 = if spin == 1 {
        make_map(lmax, nphi, 1)
    } else {
        Vec::new()
    };
    let alm64_0 = widen_vec_complex(&alm32_0);
    let alm64_1 = widen_vec_complex(&alm32_1);
    let map64_0 = map32_0.iter().copied().map(f64::from).collect::<Vec<_>>();
    let map64_1 = map32_1.iter().copied().map(f64::from).collect::<Vec<_>>();
    let map_len = ntheta * nphi;
    let nalm = plan32.harmonics().nalm();
    let channels = if spin == 0 { 1 } else { 2 };
    let prefix = format!("t{lmax}-s{spin}-q{threads}-{operation}");
    let capacities_before = bridge.capacities();

    println!(
        "spin={spin} T{lmax}; GL {ntheta}x{nphi}; phi0=.37; q={threads}; operation={operation}; STANDARD; five alternating samples; Rust 1.99.0"
    );
    println!(
        "plan_setup_ms={:.3} workspace_and_reusable_buffers_setup_ms={:.3}",
        millis(plan_setup),
        millis(workspace_setup)
    );
    println!(
        "reusable_f64_buffer_payload_bytes={} allocating_f64_temporary_payload_bytes={} reusable_capacities_before={capacities_before:?}",
        bridge.buffer_payload_bytes(),
        channels * (nalm * std::mem::size_of::<Complex64>() + map_len * std::mem::size_of::<f64>())
    );

    if operation == "synthesis" {
        let mut native0 = vec![0.0_f32; map_len];
        let mut native1 = if spin == 1 {
            vec![0.0_f32; map_len]
        } else {
            Vec::new()
        };
        let mut allocating0 = vec![0.0_f32; map_len];
        let mut allocating1 = if spin == 1 {
            vec![0.0_f32; map_len]
        } else {
            Vec::new()
        };
        let mut reusable0 = vec![0.0_f32; map_len];
        let mut reusable1 = if spin == 1 {
            vec![0.0_f32; map_len]
        } else {
            Vec::new()
        };
        let mut f64out0 = vec![0.0_f64; map_len];
        let mut f64out1 = if spin == 1 {
            vec![0.0_f64; map_len]
        } else {
            Vec::new()
        };

        let samples = measure_four(|variant| match variant {
            Variant::NativeF32 => pool.install(|| {
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
            Variant::AllocatingBridge => pool.install(|| {
                allocating_synthesis(
                    &plan64,
                    &mut workspace64,
                    spin,
                    &alm32_0,
                    &alm32_1,
                    &mut allocating0,
                    &mut allocating1,
                );
                black_box(allocating0[0]);
            }),
            Variant::ReusableBridge => pool.install(|| {
                if spin == 0 {
                    bridge.synthesis_scalar(&alm32_0, &mut reusable0);
                } else {
                    bridge.synthesis_spin1(&alm32_0, &alm32_1, &mut reusable0, &mut reusable1);
                }
                black_box(reusable0[0]);
            }),
            Variant::NativeF64 => pool.install(|| {
                if spin == 0 {
                    plan64
                        .synthesis_into(&alm64_0, &mut f64out0, &mut workspace64)
                        .unwrap();
                } else {
                    plan64
                        .synthesis_spin_into(
                            &alm64_0,
                            &alm64_1,
                            &mut f64out0,
                            &mut f64out1,
                            &mut workspace64,
                        )
                        .unwrap();
                }
                black_box(f64out0[0]);
            }),
        });
        print_measurements(&samples);
        write_real32(
            &dump_dir.join(format!("{prefix}-native_f32-0.f32")),
            &native0,
        );
        write_real32(
            &dump_dir.join(format!("{prefix}-allocating_bridge-0.f32")),
            &allocating0,
        );
        write_real32(
            &dump_dir.join(format!("{prefix}-reusable_bridge-0.f32")),
            &reusable0,
        );
        write_real64(
            &dump_dir.join(format!("{prefix}-native_f64_rounded_inputs-0.f64")),
            &f64out0,
        );
        if spin == 1 {
            write_real32(
                &dump_dir.join(format!("{prefix}-native_f32-1.f32")),
                &native1,
            );
            write_real32(
                &dump_dir.join(format!("{prefix}-allocating_bridge-1.f32")),
                &allocating1,
            );
            write_real32(
                &dump_dir.join(format!("{prefix}-reusable_bridge-1.f32")),
                &reusable1,
            );
            write_real64(
                &dump_dir.join(format!("{prefix}-native_f64_rounded_inputs-1.f64")),
                &f64out1,
            );
        }
        let bridge_matches_f64 = reusable_matches_f64_real(&reusable0, &f64out0)
            && (spin == 0 || reusable_matches_f64_real(&reusable1, &f64out1));
        println!("reusable_matches_f64_cast={bridge_matches_f64}");
        assert!(bridge_matches_f64);
    } else {
        let mut native0 = vec![Complex32::new(0.0, 0.0); nalm];
        let mut native1 = if spin == 1 {
            vec![Complex32::new(0.0, 0.0); nalm]
        } else {
            Vec::new()
        };
        let mut allocating0 = vec![Complex32::new(0.0, 0.0); nalm];
        let mut allocating1 = if spin == 1 {
            vec![Complex32::new(0.0, 0.0); nalm]
        } else {
            Vec::new()
        };
        let mut reusable0 = vec![Complex32::new(0.0, 0.0); nalm];
        let mut reusable1 = if spin == 1 {
            vec![Complex32::new(0.0, 0.0); nalm]
        } else {
            Vec::new()
        };
        let mut f64out0 = vec![Complex64::new(0.0, 0.0); nalm];
        let mut f64out1 = if spin == 1 {
            vec![Complex64::new(0.0, 0.0); nalm]
        } else {
            Vec::new()
        };

        let samples = measure_four(|variant| match variant {
            Variant::NativeF32 => pool.install(|| {
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
            Variant::AllocatingBridge => pool.install(|| {
                allocating_analysis(
                    &plan64,
                    &mut workspace64,
                    spin,
                    &map32_0,
                    &map32_1,
                    &mut allocating0,
                    &mut allocating1,
                );
                black_box(allocating0[0]);
            }),
            Variant::ReusableBridge => pool.install(|| {
                if spin == 0 {
                    bridge.analysis_scalar(&map32_0, &mut reusable0);
                } else {
                    bridge.analysis_spin1(&map32_0, &map32_1, &mut reusable0, &mut reusable1);
                }
                black_box(reusable0[0]);
            }),
            Variant::NativeF64 => pool.install(|| {
                if spin == 0 {
                    plan64
                        .analysis_into(&map64_0, &mut f64out0, &mut workspace64)
                        .unwrap();
                } else {
                    plan64
                        .analysis_spin_into(
                            &map64_0,
                            &map64_1,
                            &mut f64out0,
                            &mut f64out1,
                            &mut workspace64,
                        )
                        .unwrap();
                }
                black_box(f64out0[0]);
            }),
        });
        print_measurements(&samples);
        write_complex32(
            &dump_dir.join(format!("{prefix}-native_f32-0.c32")),
            &native0,
        );
        write_complex32(
            &dump_dir.join(format!("{prefix}-allocating_bridge-0.c32")),
            &allocating0,
        );
        write_complex32(
            &dump_dir.join(format!("{prefix}-reusable_bridge-0.c32")),
            &reusable0,
        );
        write_complex64(
            &dump_dir.join(format!("{prefix}-native_f64_rounded_inputs-0.c128")),
            &f64out0,
        );
        if spin == 1 {
            write_complex32(
                &dump_dir.join(format!("{prefix}-native_f32-1.c32")),
                &native1,
            );
            write_complex32(
                &dump_dir.join(format!("{prefix}-allocating_bridge-1.c32")),
                &allocating1,
            );
            write_complex32(
                &dump_dir.join(format!("{prefix}-reusable_bridge-1.c32")),
                &reusable1,
            );
            write_complex64(
                &dump_dir.join(format!("{prefix}-native_f64_rounded_inputs-1.c128")),
                &f64out1,
            );
        }
        let bridge_matches_f64 = reusable_matches_f64_complex(&reusable0, &f64out0)
            && (spin == 0 || reusable_matches_f64_complex(&reusable1, &f64out1));
        println!("reusable_matches_f64_cast={bridge_matches_f64}");
        assert!(bridge_matches_f64);
    }
    let capacities_after = bridge.capacities();
    println!(
        "reusable_capacities_after={capacities_after:?} capacities_unchanged={}",
        capacities_before == capacities_after
    );
    assert_eq!(capacities_before, capacities_after);
}

fn allocating_synthesis(
    plan: &ShtPlan<f64>,
    workspace: &mut rustsht::ShtWorkspace<f64>,
    spin: usize,
    e32: &[Complex32],
    b32: &[Complex32],
    first32: &mut [f32],
    second32: &mut [f32],
) {
    let e64 = widen_vec_complex(e32);
    let b64 = widen_vec_complex(b32);
    let nmap = plan.grid().ntheta() * plan.grid().nphi();
    let mut first64 = vec![0.0; nmap];
    let mut second64 = if spin == 1 {
        vec![0.0; nmap]
    } else {
        Vec::new()
    };
    if spin == 0 {
        plan.synthesis_into(&e64, &mut first64, workspace).unwrap();
    } else {
        plan.synthesis_spin_into(&e64, &b64, &mut first64, &mut second64, workspace)
            .unwrap();
    }
    narrow_real(&first64, first32);
    if spin == 1 {
        narrow_real(&second64, second32);
    }
}

fn allocating_analysis(
    plan: &ShtPlan<f64>,
    workspace: &mut rustsht::ShtWorkspace<f64>,
    spin: usize,
    first32: &[f32],
    second32: &[f32],
    e32: &mut [Complex32],
    b32: &mut [Complex32],
) {
    let first64 = first32.iter().copied().map(f64::from).collect::<Vec<_>>();
    let second64 = second32.iter().copied().map(f64::from).collect::<Vec<_>>();
    let mut e64 = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b64 = if spin == 1 {
        vec![Complex64::new(0.0, 0.0); e64.len()]
    } else {
        Vec::new()
    };
    if spin == 0 {
        plan.analysis_into(&first64, &mut e64, workspace).unwrap();
    } else {
        plan.analysis_spin_into(&first64, &second64, &mut e64, &mut b64, workspace)
            .unwrap();
    }
    narrow_complex(&e64, e32);
    if spin == 1 {
        narrow_complex(&b64, b32);
    }
}

fn measure_four(mut operation: impl FnMut(Variant)) -> [Vec<Duration>; 4] {
    for variant in Variant::ALL {
        operation(variant);
    }
    let mut samples: [Vec<Duration>; 4] = std::array::from_fn(|_| Vec::with_capacity(5));
    for repetition in 0..5 {
        let order = if repetition % 2 == 0 {
            Variant::ALL
        } else {
            [
                Variant::NativeF64,
                Variant::ReusableBridge,
                Variant::AllocatingBridge,
                Variant::NativeF32,
            ]
        };
        for variant in order {
            let start = Instant::now();
            operation(variant);
            samples[variant as usize].push(start.elapsed());
        }
    }
    samples
}

fn print_measurements(samples: &[Vec<Duration>; 4]) {
    for variant in Variant::ALL {
        let values = &samples[variant as usize];
        let millis = values
            .iter()
            .map(|value| value.as_secs_f64() * 1000.0)
            .collect::<Vec<_>>();
        let full_median = median(&millis);
        let mean = millis.iter().sum::<f64>() / millis.len() as f64;
        let variance = millis
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / millis.len() as f64;
        let cv = variance.sqrt() / mean;
        let first_three = median(&millis[..3]);
        let final_two = median(&millis[3..]);
        let drift = (final_two / first_three - 1.0).abs();
        let sample_text = millis
            .iter()
            .map(|value| format!("{value:.3}"))
            .collect::<Vec<_>>()
            .join(";");
        let stable = cv <= 0.05 && drift <= 0.05;
        println!(
            "{}_samples_ms={sample_text} median_ms={full_median:.3} cv={:.2}% first3_median_ms={first_three:.3} final2_median_ms={final_two:.3} drift={:.2}% stable={stable}",
            variant.name(),
            cv * 100.0,
            drift * 100.0
        );
    }
}

fn median(values: &[f64]) -> f64 {
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn millis(value: Duration) -> f64 {
    value.as_secs_f64() * 1000.0
}

fn make_coefficients(lmax: usize, spin: usize, channel: usize) -> Vec<Complex32> {
    let mut values = vec![Complex32::new(0.0, 0.0); (lmax + 1) * (lmax + 2) / 2];
    for m in 0..=lmax {
        for l in m.max(spin)..=lmax {
            let index = m * (2 * lmax + 1 - m) / 2 + l;
            let (real, imaginary, real_offset, imaginary_offset) = if channel == 0 {
                ((l * 7 + m * 11) % 19, (l * 13 + m * 5) % 23, 9, 11)
            } else {
                ((l * 3 + m * 7) % 17, (l * 5 + m * 17) % 29, 8, 14)
            };
            values[index] = Complex32::new(
                (real as i32 - real_offset) as f32 * 1.0e-5,
                if m == 0 {
                    0.0
                } else {
                    (imaginary as i32 - imaginary_offset) as f32 * 1.0e-5
                },
            );
        }
    }
    values
}

fn make_map(lmax: usize, nphi: usize, channel: usize) -> Vec<f32> {
    let mut values = vec![0.0; (lmax + 1) * nphi];
    for ring in 0..=lmax {
        for longitude in 0..nphi {
            let (numerator, denominator) = if channel == 0 {
                (((ring * 31 + longitude * 17) % 101) as i32 - 50, 101.0)
            } else {
                (((ring * 19 + longitude * 23 + 7) % 97) as i32 - 48, 97.0)
            };
            values[ring * nphi + longitude] = numerator as f32 / denominator;
        }
    }
    values
}

fn widen_vec_complex(values: &[Complex32]) -> Vec<Complex64> {
    values
        .iter()
        .map(|value| Complex64::new(f64::from(value.re), f64::from(value.im)))
        .collect()
}

fn reusable_matches_f64_real(actual: &[f32], expected: &[f64]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(&actual, &expected)| actual.to_bits() == (expected as f32).to_bits())
}

fn reusable_matches_f64_complex(actual: &[Complex32], expected: &[Complex64]) -> bool {
    actual.iter().zip(expected).all(|(&actual, &expected)| {
        actual.re.to_bits() == (expected.re as f32).to_bits()
            && actual.im.to_bits() == (expected.im as f32).to_bits()
    })
}

fn write_real32(path: &Path, values: &[f32]) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());
    for value in values {
        writer.write_all(&value.to_le_bytes()).unwrap();
    }
}

fn write_real64(path: &Path, values: &[f64]) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());
    for value in values {
        writer.write_all(&value.to_le_bytes()).unwrap();
    }
}

fn write_complex32(path: &Path, values: &[Complex32]) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());
    for value in values {
        writer.write_all(&value.re.to_le_bytes()).unwrap();
        writer.write_all(&value.im.to_le_bytes()).unwrap();
    }
}

fn write_complex64(path: &Path, values: &[Complex64]) {
    let mut writer = BufWriter::new(std::fs::File::create(path).unwrap());
    for value in values {
        writer.write_all(&value.re.to_le_bytes()).unwrap();
        writer.write_all(&value.im.to_le_bytes()).unwrap();
    }
}

// The full operator, analytic, exceptional-input, and adjoint checks are run
// by --check; benchmark-only code never changes public execution routing.
fn check_all(_dump_dir: Option<&Path>) {
    check_conversion_edges();
    check_operator_case(127, 0);
    check_operator_case(127, 1);
    check_operator_case(511, 0);
    check_operator_case(511, 1);
    check_analytic_low_degree();
    check_extreme_range();
    println!(
        "f32_bridge_check=PASS degrees=127,511 spins=0,1 operators=S,A,S*,A* adjoint_identities=scalar,spin1 analytic=scalar-monopole,spin1-m0 exceptional=large,small,cancellation,signed-zero,subnormal,downcast-overflow"
    );
}

fn check_conversion_edges() {
    let minimum_subnormal = f32::from_bits(1);
    let values = [
        0.0_f32,
        -0.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        minimum_subnormal,
        -minimum_subnormal,
        1.0e-30,
        -1.0e-30,
        f32::MAX,
        -f32::MAX,
    ];
    let mut wide = vec![0.0_f64; values.len()];
    let mut roundtrip = vec![0.0_f32; values.len()];
    widen_real(&values, &mut wide);
    narrow_real(&wide, &mut roundtrip);
    assert!(
        values
            .iter()
            .zip(roundtrip)
            .all(|(left, right)| left.to_bits() == right.to_bits())
    );
    assert_eq!(wide[1].to_bits(), (-0.0_f64).to_bits());

    let complex = [
        Complex32::new(-0.0, minimum_subnormal),
        Complex32::new(-minimum_subnormal, 0.0),
    ];
    let mut complex_wide = vec![Complex64::new(0.0, 0.0); complex.len()];
    let mut complex_roundtrip = vec![Complex32::new(0.0, 0.0); complex.len()];
    widen_complex(&complex, &mut complex_wide);
    narrow_complex(&complex_wide, &mut complex_roundtrip);
    for (left, right) in complex.iter().zip(complex_roundtrip) {
        assert_eq!(left.re.to_bits(), right.re.to_bits());
        assert_eq!(left.im.to_bits(), right.im.to_bits());
    }

    let wide_edges = [
        0.0_f64,
        -0.0,
        f64::from(f32::MAX) * 2.0,
        -f64::from(f32::MAX) * 2.0,
        f64::from(minimum_subnormal) / 2.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    let mut narrowed = vec![0.0_f32; wide_edges.len()];
    narrow_real(&wide_edges, &mut narrowed);
    assert_eq!(narrowed[0].to_bits(), 0.0_f32.to_bits());
    assert_eq!(narrowed[1].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(narrowed[2], f32::INFINITY);
    assert_eq!(narrowed[3], f32::NEG_INFINITY);
    assert_eq!(narrowed[4].to_bits(), 0.0_f32.to_bits());
    assert_eq!(narrowed[5], f32::INFINITY);
    assert_eq!(narrowed[6], f32::NEG_INFINITY);
    assert!(narrowed[7].is_nan());
}

fn check_operator_case(lmax: usize, spin: usize) {
    let ntheta = lmax + 1;
    let nphi = 2 * ntheta;
    let nmap = ntheta * nphi;
    let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap();
    let harmonics = HarmonicSpec::new(lmax, lmax, spin).unwrap();
    let plan64 = ShtPlan::<f64>::new(grid, harmonics.clone()).unwrap();
    let mut workspace64 = plan64.workspace().unwrap();
    let mut bridge_workspace = plan64.workspace().unwrap();
    plan64.prepare_workspace(&mut workspace64, 1).unwrap();
    plan64.prepare_workspace(&mut bridge_workspace, 1).unwrap();
    let mut bridge = ReusableBridge::new(&plan64, bridge_workspace);
    let bridge_capacities_before = bridge.capacities();

    let alm0 = make_coefficients(lmax, spin, 0);
    let alm1 = if spin == 1 {
        make_coefficients(lmax, spin, 1)
    } else {
        Vec::new()
    };
    let alm0_64 = widen_vec_complex(&alm0);
    let alm1_64 = widen_vec_complex(&alm1);
    let map0 = make_map(lmax, nphi, 0);
    let map1 = if spin == 1 {
        make_map(lmax, nphi, 1)
    } else {
        Vec::new()
    };
    let map0_64 = map0.iter().copied().map(f64::from).collect::<Vec<_>>();
    let map1_64 = map1.iter().copied().map(f64::from).collect::<Vec<_>>();

    if spin == 0 {
        let mut synth_ref = vec![0.0_f64; nmap];
        let mut analysis_ref = vec![Complex64::new(0.0, 0.0); harmonics.nalm()];
        let mut synth_adjoint_ref = vec![Complex64::new(0.0, 0.0); harmonics.nalm()];
        let mut analysis_adjoint_ref = vec![0.0_f64; nmap];
        plan64
            .synthesis_into(&alm0_64, &mut synth_ref, &mut workspace64)
            .unwrap();
        plan64
            .analysis_into(&map0_64, &mut analysis_ref, &mut workspace64)
            .unwrap();
        plan64
            .adjoint_synthesis_into(&map0_64, &mut synth_adjoint_ref, &mut workspace64)
            .unwrap();
        plan64
            .adjoint_analysis_into(&alm0_64, &mut analysis_adjoint_ref, &mut workspace64)
            .unwrap();

        let mut synth = vec![0.0_f32; nmap];
        let mut analysis = vec![Complex32::new(0.0, 0.0); harmonics.nalm()];
        let mut synth_adjoint = vec![Complex32::new(0.0, 0.0); harmonics.nalm()];
        let mut analysis_adjoint = vec![0.0_f32; nmap];
        bridge.synthesis_scalar(&alm0, &mut synth);
        bridge.analysis_scalar(&map0, &mut analysis);
        bridge.adjoint_synthesis_scalar(&map0, &mut synth_adjoint);
        bridge.adjoint_analysis_scalar(&alm0, &mut analysis_adjoint);
        assert!(
            reusable_matches_f64_real(&synth, &synth_ref),
            "T{lmax} scalar S"
        );
        assert!(
            reusable_matches_f64_complex(&analysis, &analysis_ref),
            "T{lmax} scalar A"
        );
        assert!(
            reusable_matches_f64_complex(&synth_adjoint, &synth_adjoint_ref),
            "T{lmax} scalar S*"
        );
        assert!(
            reusable_matches_f64_real(&analysis_adjoint, &analysis_adjoint_ref),
            "T{lmax} scalar A*"
        );

        assert_scaled_close(
            real_inner(&synth_ref, &map0_64),
            harmonic_inner(&harmonics, &alm0_64, &synth_adjoint_ref),
            5.0e-12,
            "scalar S/S*",
        );
        assert_scaled_close(
            harmonic_inner(&harmonics, &analysis_ref, &alm0_64),
            real_inner(&map0_64, &analysis_adjoint_ref),
            5.0e-12,
            "scalar A/A*",
        );
    } else {
        let mut synth_first_ref = vec![0.0_f64; nmap];
        let mut synth_second_ref = vec![0.0_f64; nmap];
        let mut analysis_e_ref = vec![Complex64::new(0.0, 0.0); harmonics.nalm()];
        let mut analysis_b_ref = vec![Complex64::new(0.0, 0.0); harmonics.nalm()];
        let mut synth_adjoint_e_ref = vec![Complex64::new(0.0, 0.0); harmonics.nalm()];
        let mut synth_adjoint_b_ref = vec![Complex64::new(0.0, 0.0); harmonics.nalm()];
        let mut analysis_adjoint_first_ref = vec![0.0_f64; nmap];
        let mut analysis_adjoint_second_ref = vec![0.0_f64; nmap];
        plan64
            .synthesis_spin_into(
                &alm0_64,
                &alm1_64,
                &mut synth_first_ref,
                &mut synth_second_ref,
                &mut workspace64,
            )
            .unwrap();
        plan64
            .analysis_spin_into(
                &map0_64,
                &map1_64,
                &mut analysis_e_ref,
                &mut analysis_b_ref,
                &mut workspace64,
            )
            .unwrap();
        plan64
            .adjoint_synthesis_spin_into(
                &map0_64,
                &map1_64,
                &mut synth_adjoint_e_ref,
                &mut synth_adjoint_b_ref,
                &mut workspace64,
            )
            .unwrap();
        plan64
            .adjoint_analysis_spin_into(
                &alm0_64,
                &alm1_64,
                &mut analysis_adjoint_first_ref,
                &mut analysis_adjoint_second_ref,
                &mut workspace64,
            )
            .unwrap();

        let mut synth_first = vec![0.0_f32; nmap];
        let mut synth_second = vec![0.0_f32; nmap];
        let mut analysis_e = vec![Complex32::new(0.0, 0.0); harmonics.nalm()];
        let mut analysis_b = vec![Complex32::new(0.0, 0.0); harmonics.nalm()];
        let mut synth_adjoint_e = vec![Complex32::new(0.0, 0.0); harmonics.nalm()];
        let mut synth_adjoint_b = vec![Complex32::new(0.0, 0.0); harmonics.nalm()];
        let mut analysis_adjoint_first = vec![0.0_f32; nmap];
        let mut analysis_adjoint_second = vec![0.0_f32; nmap];
        bridge.synthesis_spin1(&alm0, &alm1, &mut synth_first, &mut synth_second);
        bridge.analysis_spin1(&map0, &map1, &mut analysis_e, &mut analysis_b);
        bridge.adjoint_synthesis_spin1(&map0, &map1, &mut synth_adjoint_e, &mut synth_adjoint_b);
        bridge.adjoint_analysis_spin1(
            &alm0,
            &alm1,
            &mut analysis_adjoint_first,
            &mut analysis_adjoint_second,
        );
        assert!(
            reusable_matches_f64_real(&synth_first, &synth_first_ref),
            "T{lmax} spin-1 S first"
        );
        assert!(
            reusable_matches_f64_real(&synth_second, &synth_second_ref),
            "T{lmax} spin-1 S second"
        );
        assert!(
            reusable_matches_f64_complex(&analysis_e, &analysis_e_ref),
            "T{lmax} spin-1 A E"
        );
        assert!(
            reusable_matches_f64_complex(&analysis_b, &analysis_b_ref),
            "T{lmax} spin-1 A B"
        );
        assert!(
            reusable_matches_f64_complex(&synth_adjoint_e, &synth_adjoint_e_ref),
            "T{lmax} spin-1 S* E"
        );
        assert!(
            reusable_matches_f64_complex(&synth_adjoint_b, &synth_adjoint_b_ref),
            "T{lmax} spin-1 S* B"
        );
        assert!(
            reusable_matches_f64_real(&analysis_adjoint_first, &analysis_adjoint_first_ref),
            "T{lmax} spin-1 A* first"
        );
        assert!(
            reusable_matches_f64_real(&analysis_adjoint_second, &analysis_adjoint_second_ref),
            "T{lmax} spin-1 A* second"
        );

        assert_scaled_close(
            real_inner(&synth_first_ref, &map0_64) + real_inner(&synth_second_ref, &map1_64),
            harmonic_inner(&harmonics, &alm0_64, &synth_adjoint_e_ref)
                + harmonic_inner(&harmonics, &alm1_64, &synth_adjoint_b_ref),
            5.0e-12,
            "spin-1 S/S*",
        );
        assert_scaled_close(
            harmonic_inner(&harmonics, &analysis_e_ref, &alm0_64)
                + harmonic_inner(&harmonics, &analysis_b_ref, &alm1_64),
            real_inner(&map0_64, &analysis_adjoint_first_ref)
                + real_inner(&map1_64, &analysis_adjoint_second_ref),
            5.0e-12,
            "spin-1 A/A*",
        );
    }
    assert_eq!(
        bridge.capacities(),
        bridge_capacities_before,
        "T{lmax} spin-{spin} bridge buffers must not grow during execution"
    );
}

fn check_analytic_low_degree() {
    let scalar_grid = GridSpec::new(GridKind::GaussLegendre, 3, 5, 0.23).unwrap();
    let scalar_harmonics = HarmonicSpec::new(0, 0, 0).unwrap();
    let scalar_plan = ShtPlan::<f64>::new(scalar_grid, scalar_harmonics.clone()).unwrap();
    let scalar_workspace = scalar_plan.workspace().unwrap();
    let mut scalar_bridge = ReusableBridge::new(&scalar_plan, scalar_workspace);
    let scalar_alm = [Complex32::new(0.73, 0.0)];
    let mut scalar_map = vec![0.0_f32; 15];
    scalar_bridge.synthesis_scalar(&scalar_alm, &mut scalar_map);
    let scalar_expected = 0.73_f64 / (4.0 * std::f64::consts::PI).sqrt();
    for actual in scalar_map {
        assert!((f64::from(actual) - scalar_expected).abs() < 2.0e-8);
    }

    let spin_grid = GridSpec::new(GridKind::GaussLegendre, 3, 5, 0.23).unwrap();
    let spin_harmonics = HarmonicSpec::new(1, 1, 1).unwrap();
    let spin_plan = ShtPlan::<f64>::new(spin_grid, spin_harmonics.clone()).unwrap();
    let spin_workspace = spin_plan.workspace().unwrap();
    let mut spin_bridge = ReusableBridge::new(&spin_plan, spin_workspace);
    let mut e_alm = vec![Complex32::new(0.0, 0.0); spin_harmonics.nalm()];
    let b_alm = vec![Complex32::new(0.0, 0.0); spin_harmonics.nalm()];
    e_alm[spin_harmonics.index(1, 0).unwrap()] = Complex32::new(1.0, 0.0);
    let mut first = vec![0.0_f32; 15];
    let mut second = vec![0.0_f32; 15];
    spin_bridge.synthesis_spin1(&e_alm, &b_alm, &mut first, &mut second);
    let norm = -(3.0 / (8.0 * std::f64::consts::PI)).sqrt();
    for (ring, &theta) in spin_plan.grid().theta().iter().enumerate() {
        let expected = (norm * theta.sin()) as f32;
        for longitude in 0..spin_plan.grid().nphi() {
            let index = ring * spin_plan.grid().nphi() + longitude;
            assert!((first[index] - expected).abs() < 2.0e-7);
            assert!(second[index].abs() < 2.0e-7);
        }
    }
}

fn check_extreme_range() {
    let grid = GridSpec::new(GridKind::GaussLegendre, 2, 4, 0.37).unwrap();
    let harmonics = HarmonicSpec::new(1, 1, 0).unwrap();
    let plan = ShtPlan::<f64>::new(grid, harmonics.clone()).unwrap();
    let mut reference_workspace = plan.workspace().unwrap();
    let mut bridge = ReusableBridge::new(&plan, plan.workspace().unwrap());

    let mut large = vec![Complex32::new(0.0, 0.0); harmonics.nalm()];
    large[harmonics.index(0, 0).unwrap()] = Complex32::new(1.0e30, 0.0);
    large[harmonics.index(1, 0).unwrap()] = Complex32::new(-1.0e30, 0.0);
    let mut large_map = vec![0.0_f32; 8];
    bridge.synthesis_scalar(&large, &mut large_map);
    let large64 = widen_vec_complex(&large);
    let mut large_reference = vec![0.0_f64; 8];
    plan.synthesis_into(&large64, &mut large_reference, &mut reference_workspace)
        .unwrap();
    assert!(reusable_matches_f64_real(&large_map, &large_reference));
    assert!(large_map.iter().all(|value| value.is_finite()));
    assert!(
        large_map[0].abs() < 1.0e-6 * 1.0e30,
        "cancellation control={}",
        large_map[0]
    );

    let subnormal = f32::from_bits(1);
    let mut tiny = vec![Complex32::new(0.0, 0.0); harmonics.nalm()];
    tiny[harmonics.index(0, 0).unwrap()] = Complex32::new(subnormal, 0.0);
    let mut tiny_map = vec![1.0_f32; 8];
    bridge.synthesis_scalar(&tiny, &mut tiny_map);
    let tiny64 = widen_vec_complex(&tiny);
    let mut tiny_reference = vec![0.0_f64; 8];
    plan.synthesis_into(&tiny64, &mut tiny_reference, &mut reference_workspace)
        .unwrap();
    assert!(reusable_matches_f64_real(&tiny_map, &tiny_reference));
    assert!(tiny_map.iter().all(|value| value.to_bits() == 0));

    let huge_map = vec![f32::MAX; 8];
    let mut overflowed_coefficients = vec![Complex32::new(0.0, 0.0); harmonics.nalm()];
    bridge.analysis_scalar(&huge_map, &mut overflowed_coefficients);
    let huge_map64 = huge_map.iter().copied().map(f64::from).collect::<Vec<_>>();
    let mut overflow_reference = vec![Complex64::new(0.0, 0.0); harmonics.nalm()];
    plan.analysis_into(
        &huge_map64,
        &mut overflow_reference,
        &mut reference_workspace,
    )
    .unwrap();
    assert!(reusable_matches_f64_complex(
        &overflowed_coefficients,
        &overflow_reference
    ));
    assert!(
        overflowed_coefficients
            .iter()
            .any(|value| value.re.is_infinite())
    );
    assert!(
        overflowed_coefficients
            .iter()
            .all(|value| value.im == 0.0 || value.im.is_finite())
    );
}

fn real_inner(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(&left, &right)| left * right)
        .sum()
}

fn harmonic_inner(spec: &HarmonicSpec, left: &[Complex64], right: &[Complex64]) -> f64 {
    let mut result = 0.0;
    for m in 0..=spec.mmax() {
        let weight = if m == 0 { 1.0 } else { 2.0 };
        for l in m.max(spec.spin())..=spec.lmax() {
            let index = spec.index(l, m).unwrap();
            result += weight * (left[index].conj() * right[index]).re;
        }
    }
    result
}

fn assert_scaled_close(actual: f64, expected: f64, tolerance: f64, operator: &str) {
    let scale = actual.abs().max(expected.abs()).max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance * scale,
        "{operator} inner-product mismatch: actual={actual:.16e}, expected={expected:.16e}, relative={:.3e}",
        (actual - expected).abs() / scale
    );
}
