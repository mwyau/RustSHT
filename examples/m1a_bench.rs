use std::hint::black_box;
use std::time::{Duration, Instant};

use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

// Keep this aligned with the private recurrence block in `legendre::scalar`.
const LATITUDE_BLOCK_SIZE: usize = 64;

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
    let lmax = arguments
        .next()
        .map(|value| value.parse().expect("lmax must be an integer"))
        .unwrap_or(511);
    let ntheta = lmax + 1;
    let nphi = 2 * lmax + 3;
    let rayon_threads = arguments
        .next()
        .map(|value| value.parse().expect("thread count must be an integer"))
        .unwrap_or(8);
    let operation = arguments.next().unwrap_or_else(|| "both".to_owned());
    assert!(matches!(
        operation.as_str(),
        "both" | "synthesis" | "analysis"
    ));
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
            GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::scalar(lmax, lmax).unwrap(),
        )
        .unwrap()
    };
    let plan_construction = measure(|| drop(black_box(make_plan())));
    let plan = make_plan();
    let workspace_allocation = measure(|| {
        drop(black_box(rayon_pool.install(|| plan.workspace()).unwrap()));
    });
    let mut workspace = rayon_pool.install(|| plan.workspace()).unwrap();
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    for m in 0..=lmax {
        for l in m..=lmax {
            let index = plan.harmonics().index(l, m).unwrap();
            let re = ((l * 7 + m * 11) % 19) as f64 - 9.0;
            let im = if m == 0 {
                0.0
            } else {
                ((l * 13 + m * 5) % 23) as f64 - 11.0
            };
            alm[index] = Complex64::new(re * 1e-5, im * 1e-5);
        }
    }
    let mut map = vec![0.0; ntheta * nphi];
    for ring in 0..ntheta {
        for longitude in 0..nphi {
            let theta = ring as f64;
            let phi = longitude as f64;
            map[ring * nphi + longitude] =
                (0.019 * theta + 0.037 * phi).sin() + 0.3 * (0.071 * theta - 0.013 * phi).cos();
        }
    }
    let mut analyzed = vec![Complex64::new(0.0, 0.0); alm.len()];

    let synthesis_one_thread = (operation != "analysis").then(|| {
        measure(|| {
            one_thread_pool.install(|| {
                plan.synthesis_into(black_box(&alm), black_box(&mut map), &mut workspace)
                    .unwrap();
                black_box(map[0]);
            });
        })
    });
    let synthesis_rayon = (operation != "analysis").then(|| {
        measure(|| {
            rayon_pool.install(|| {
                plan.synthesis_into(black_box(&alm), black_box(&mut map), &mut workspace)
                    .unwrap();
                black_box(map[0]);
            });
        })
    });
    let analysis_one_thread = (operation != "synthesis").then(|| {
        measure(|| {
            one_thread_pool.install(|| {
                plan.analysis_into(black_box(&map), black_box(&mut analyzed), &mut workspace)
                    .unwrap();
                black_box(analyzed[0]);
            });
        })
    });
    let analysis_rayon = (operation != "synthesis").then(|| {
        measure(|| {
            rayon_pool.install(|| {
                plan.analysis_into(black_box(&map), black_box(&mut analyzed), &mut workspace)
                    .unwrap();
                black_box(analyzed[0]);
            });
        })
    });

    let spectrum_len = nphi / 2 + 1;
    let spin_recurrence_factors = rayon_threads * 3 * (lmax + 2) * size_of::<f64>();
    let spin_recurrence_lanes =
        rayon_threads * LATITUDE_BLOCK_SIZE * (6 * size_of::<f64>() + 2 * size_of::<i64>());
    let core_bytes = map.len() * size_of::<f64>()
        + alm.len() * size_of::<Complex64>()
        + analyzed.len() * size_of::<Complex64>()
        + 2 * ntheta * spectrum_len * size_of::<Complex64>()
        + nphi * size_of::<f64>()
        + rayon_threads * (lmax + 4) * size_of::<f64>()
        + 2 * ntheta * size_of::<f64>()
        + ntheta * size_of::<usize>()
        + rayon_threads * LATITUDE_BLOCK_SIZE * (3 * size_of::<f64>() + size_of::<i64>())
        + 2 * ntheta * size_of::<f64>()
        + (lmax + 1) * size_of::<f64>()
        + spin_recurrence_factors
        + spin_recurrence_lanes
        + rayon_threads * LATITUDE_BLOCK_SIZE * 4 * size_of::<f64>()
        + rayon_threads * LATITUDE_BLOCK_SIZE * 2 * size_of::<Complex64>()
        + rayon_threads * LATITUDE_BLOCK_SIZE * (2 * size_of::<f64>() + size_of::<usize>());
    println!(
        "T{lmax}; GL {ntheta}x{nphi}; f64; 1 vs {rayon_threads} threads; {operation}; 5 samples, median"
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
    println!(
        "estimated_numeric_arrays_MiB={:.2}",
        core_bytes as f64 / 2f64.powi(20)
    );
}
