use std::hint::black_box;
use std::time::{Duration, Instant};

use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn median(mut durations: Vec<Duration>) -> Duration {
    durations.sort_unstable();
    durations[durations.len() / 2]
}

fn measure(mut operation: impl FnMut()) -> Duration {
    operation();
    let mut durations = Vec::with_capacity(5);
    for _ in 0..5 {
        let start = Instant::now();
        operation();
        durations.push(start.elapsed());
    }
    median(durations)
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let spin = arguments
        .next()
        .map(|value| value.parse().expect("spin must be an integer"))
        .unwrap_or(1);
    let lmax = arguments
        .next()
        .map(|value| value.parse().expect("lmax must be an integer"))
        .unwrap_or(511);
    let rayon_threads = arguments
        .next()
        .map(|value| value.parse().expect("thread count must be an integer"))
        .unwrap_or(8);
    let operation = arguments.next().unwrap_or_else(|| "both".to_owned());
    assert!(matches!(
        operation.as_str(),
        "both" | "synthesis" | "analysis"
    ));
    let geometry = arguments.next().unwrap_or_else(|| "GL".to_owned());
    let grid_kind = match geometry.as_str() {
        "GL" => GridKind::GaussLegendre,
        "CC" => GridKind::ClenshawCurtis,
        "F1" => GridKind::Fejer1,
        _ => panic!("geometry must be GL, CC, or F1"),
    };
    let mode = arguments.next().unwrap_or_else(|| "STANDARD".to_owned());
    assert!(matches!(mode.as_str(), "STANDARD" | "GRAD_ONLY" | "DERIV1"));
    assert!(mode != "GRAD_ONLY" || spin > 0);
    assert!(mode != "DERIV1" || spin == 0);
    assert!(mode == "STANDARD" || operation == "synthesis");
    let ntheta = lmax
        + if grid_kind == GridKind::ClenshawCurtis {
            2
        } else {
            1
        };
    let nphi = 2 * lmax + 3;
    let plan_spin = if mode == "DERIV1" { 0 } else { spin };
    let one_thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let rayon_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(rayon_threads)
        .build()
        .unwrap();
    let make_plan = || {
        ShtPlan::new(
            GridSpec::new(grid_kind, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::new(lmax, lmax, plan_spin).unwrap(),
        )
        .unwrap()
    };
    let plan_construction = measure(|| drop(black_box(make_plan())));
    let plan = make_plan();
    let workspace_allocation = measure(|| {
        drop(black_box(rayon_pool.install(|| plan.workspace()).unwrap()));
    });
    let mut workspace = rayon_pool.install(|| plan.workspace()).unwrap();
    let mut e = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let coefficient_spin = if mode == "DERIV1" { 1 } else { plan_spin };
    for m in 0..=lmax {
        for l in m.max(coefficient_spin)..=lmax {
            let index = plan.harmonics().index(l, m).unwrap();
            let e_im = if m == 0 {
                0.0
            } else {
                ((l * 13 + m * 5) % 23) as f64 - 11.0
            };
            let b_im = if m == 0 {
                0.0
            } else {
                ((l * 5 + m * 17) % 29) as f64 - 14.0
            };
            e[index] = Complex64::new(((l * 7 + m * 11) % 19) as f64 - 9.0, e_im) * 1e-5;
            b[index] = Complex64::new(((l * 3 + m * 7) % 17) as f64 - 8.0, b_im) * 1e-5;
        }
    }
    let sample_count = ntheta * nphi;
    let mut first = vec![0.0; sample_count];
    let mut second = vec![0.0; sample_count];
    let mut analyzed_e = vec![Complex64::new(0.0, 0.0); e.len()];
    let mut analyzed_b = vec![Complex64::new(0.0, 0.0); b.len()];

    let synthesis_one_thread = (operation != "analysis").then(|| {
        measure(|| {
            one_thread_pool.install(|| {
                match mode.as_str() {
                    "STANDARD" if spin == 0 => plan
                        .synthesis_into(black_box(&e), black_box(&mut first), &mut workspace)
                        .unwrap(),
                    "STANDARD" => plan
                        .synthesis_spin_into(
                            black_box(&e),
                            black_box(&b),
                            black_box(&mut first),
                            black_box(&mut second),
                            &mut workspace,
                        )
                        .unwrap(),
                    "GRAD_ONLY" => plan
                        .synthesis_gradient_into(
                            black_box(&e),
                            black_box(&mut first),
                            black_box(&mut second),
                            &mut workspace,
                        )
                        .unwrap(),
                    "DERIV1" => plan
                        .synthesis_deriv1_into(
                            black_box(&e),
                            black_box(&mut first),
                            black_box(&mut second),
                            &mut workspace,
                        )
                        .unwrap(),
                    _ => unreachable!(),
                }
                black_box(first[0]);
            });
        })
    });
    let synthesis_rayon = (operation != "analysis").then(|| {
        measure(|| {
            rayon_pool.install(|| {
                match mode.as_str() {
                    "STANDARD" if spin == 0 => plan
                        .synthesis_into(black_box(&e), black_box(&mut first), &mut workspace)
                        .unwrap(),
                    "STANDARD" => plan
                        .synthesis_spin_into(
                            black_box(&e),
                            black_box(&b),
                            black_box(&mut first),
                            black_box(&mut second),
                            &mut workspace,
                        )
                        .unwrap(),
                    "GRAD_ONLY" => plan
                        .synthesis_gradient_into(
                            black_box(&e),
                            black_box(&mut first),
                            black_box(&mut second),
                            &mut workspace,
                        )
                        .unwrap(),
                    "DERIV1" => plan
                        .synthesis_deriv1_into(
                            black_box(&e),
                            black_box(&mut first),
                            black_box(&mut second),
                            &mut workspace,
                        )
                        .unwrap(),
                    _ => unreachable!(),
                }
                black_box(first[0]);
            });
        })
    });
    if operation != "synthesis" {
        for ring in 0..ntheta {
            for longitude in 0..nphi {
                let theta = ring as f64;
                let phi = longitude as f64;
                let value =
                    (0.019 * theta + 0.037 * phi).sin() + 0.3 * (0.071 * theta - 0.013 * phi).cos();
                first[ring * nphi + longitude] = value;
                second[ring * nphi + longitude] = value;
            }
        }
    }
    let first_analysis_one_thread = (operation != "synthesis"
        && spin == 1
        && grid_kind == GridKind::GaussLegendre)
        .then(|| {
            let start = Instant::now();
            one_thread_pool.install(|| {
                plan.analysis_spin_into(
                    black_box(&first),
                    black_box(&second),
                    black_box(&mut analyzed_e),
                    black_box(&mut analyzed_b),
                    &mut workspace,
                )
                .unwrap();
                black_box(analyzed_e[0]);
            });
            start.elapsed()
        });
    let first_analysis_rayon = (operation != "synthesis"
        && spin == 1
        && grid_kind == GridKind::GaussLegendre
        && rayon_threads > 1)
        .then(|| {
            let cold_plan = make_plan();
            let mut cold_workspace = rayon_pool.install(|| cold_plan.workspace()).unwrap();
            let mut cold_e = vec![Complex64::new(0.0, 0.0); cold_plan.harmonics().nalm()];
            let mut cold_b = vec![Complex64::new(0.0, 0.0); cold_plan.harmonics().nalm()];
            let start = Instant::now();
            rayon_pool.install(|| {
                cold_plan
                    .analysis_spin_into(
                        black_box(&first),
                        black_box(&second),
                        black_box(&mut cold_e),
                        black_box(&mut cold_b),
                        &mut cold_workspace,
                    )
                    .unwrap();
                black_box(cold_e[0]);
            });
            start.elapsed()
        });
    let analysis_one_thread = (operation != "synthesis").then(|| {
        measure(|| {
            one_thread_pool.install(|| {
                if spin == 0 {
                    plan.analysis_into(
                        black_box(&first),
                        black_box(&mut analyzed_e),
                        &mut workspace,
                    )
                    .unwrap();
                } else {
                    plan.analysis_spin_into(
                        black_box(&first),
                        black_box(&second),
                        black_box(&mut analyzed_e),
                        black_box(&mut analyzed_b),
                        &mut workspace,
                    )
                    .unwrap();
                }
                black_box(analyzed_e[0]);
            });
        })
    });
    let analysis_rayon = (operation != "synthesis").then(|| {
        measure(|| {
            rayon_pool.install(|| {
                if spin == 0 {
                    plan.analysis_into(
                        black_box(&first),
                        black_box(&mut analyzed_e),
                        &mut workspace,
                    )
                    .unwrap();
                } else {
                    plan.analysis_spin_into(
                        black_box(&first),
                        black_box(&second),
                        black_box(&mut analyzed_e),
                        black_box(&mut analyzed_b),
                        &mut workspace,
                    )
                    .unwrap();
                }
                black_box(analyzed_e[0]);
            });
        })
    });

    println!(
        "spin-{spin} T{lmax}; {geometry} {ntheta}x{nphi}; f64; {mode}; 1 vs {rayon_threads} threads; {operation}; 5 samples, median"
    );
    println!(
        "plan_construction_ms={:.3}",
        plan_construction.as_secs_f64() * 1000.0
    );
    println!(
        "workspace_allocation_ms={:.3}",
        workspace_allocation.as_secs_f64() * 1000.0
    );
    if let Some(elapsed) = synthesis_one_thread {
        println!(
            "synthesis_one_thread_ms={:.3}",
            elapsed.as_secs_f64() * 1000.0
        );
    }
    if let Some(elapsed) = synthesis_rayon {
        println!("synthesis_rayon_ms={:.3}", elapsed.as_secs_f64() * 1000.0);
    }
    if let Some(elapsed) = analysis_one_thread {
        println!(
            "analysis_one_thread_ms={:.3}",
            elapsed.as_secs_f64() * 1000.0
        );
    }
    if let Some(elapsed) = analysis_rayon {
        println!("analysis_rayon_ms={:.3}", elapsed.as_secs_f64() * 1000.0);
    }
    if let Some(elapsed) = first_analysis_one_thread {
        println!(
            "analysis_first_call_one_thread_ms={:.3}",
            elapsed.as_secs_f64() * 1000.0
        );
    }
    if let Some(elapsed) = first_analysis_rayon {
        println!(
            "analysis_first_call_rayon_ms={:.3}",
            elapsed.as_secs_f64() * 1000.0
        );
    }
}
