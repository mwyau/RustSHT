// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::hint::black_box;
use std::time::{Duration, Instant};

use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

#[derive(Clone, Copy)]
enum Operation {
    Synthesis,
    Analysis,
}

#[derive(Clone, Copy)]
enum Schedule {
    Sequential,
    ParallelOneThreadWorkspaces,
    ParallelPoolSizedWorkspaces,
}

struct FieldData {
    e_alm: Vec<Complex64>,
    b_alm: Vec<Complex64>,
    first_map: Vec<f64>,
    second_map: Vec<f64>,
    output_first: Vec<f64>,
    output_second: Vec<f64>,
    output_e: Vec<Complex64>,
    output_b: Vec<Complex64>,
}

struct Data {
    fields: Vec<FieldData>,
}

fn rss_kib() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let parse = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
    };
    (parse("VmRSS:"), parse("VmHWM:"))
}

fn make_data(plan: &ShtPlan, fields: usize) -> Data {
    let sample_count = plan.grid().ntheta() * plan.grid().nphi();
    let nalm = plan.harmonics().nalm();
    let mut data = Vec::with_capacity(fields);
    for field in 0..fields {
        let mut field_data = FieldData {
            e_alm: vec![Complex64::new(0.0, 0.0); nalm],
            b_alm: vec![Complex64::new(0.0, 0.0); nalm],
            first_map: vec![0.0; sample_count],
            second_map: vec![0.0; sample_count],
            output_first: vec![0.0; sample_count],
            output_second: vec![0.0; sample_count],
            output_e: vec![Complex64::new(0.0, 0.0); nalm],
            output_b: vec![Complex64::new(0.0, 0.0); nalm],
        };
        for m in 0..=plan.harmonics().mmax() {
            for l in m.max(plan.harmonics().spin())..=plan.harmonics().lmax() {
                let index = plan.harmonics().index(l, m).unwrap();
                let field_shift = field * 101;
                let e_real = (l * 7 + m * 11 + field_shift) % 19;
                let e_imag = (l * 13 + m * 5 + field_shift) % 23;
                let b_real = (l * 3 + m * 7 + field_shift) % 17;
                let b_imag = (l * 5 + m * 17 + field_shift) % 29;
                let im = if m == 0 { 0.0 } else { e_imag as f64 - 11.0 };
                let b_im = if m == 0 { 0.0 } else { b_imag as f64 - 14.0 };
                field_data.e_alm[index] = Complex64::new(e_real as f64 - 9.0, im) * 1e-5;
                field_data.b_alm[index] = Complex64::new(b_real as f64 - 8.0, b_im) * 1e-5;
            }
        }
        for (i, (first, second)) in field_data
            .first_map
            .iter_mut()
            .zip(&mut field_data.second_map)
            .enumerate()
        {
            let theta = (i / plan.grid().nphi()) as f64;
            let phi = (i % plan.grid().nphi()) as f64;
            let phase = field as f64 * 0.17;
            *first = (0.019 * theta + 0.037 * phi + phase).sin()
                + 0.3 * (0.071 * theta - 0.013 * phi - phase).cos();
            *second = *first;
        }
        data.push(field_data);
    }
    Data { fields: data }
}

fn run_one(
    plan: &ShtPlan,
    operation: Operation,
    spin: usize,
    data: &mut FieldData,
    workspace: &mut rustsht::ShtWorkspace,
) {
    match (operation, spin) {
        (Operation::Synthesis, 0) => plan
            .synthesis_into(
                black_box(&data.e_alm),
                black_box(&mut data.output_first),
                workspace,
            )
            .unwrap(),
        (Operation::Synthesis, 1) => plan
            .synthesis_spin_into(
                black_box(&data.e_alm),
                black_box(&data.b_alm),
                black_box(&mut data.output_first),
                black_box(&mut data.output_second),
                workspace,
            )
            .unwrap(),
        (Operation::Analysis, 0) => plan
            .analysis_into(
                black_box(&data.first_map),
                black_box(&mut data.output_e),
                workspace,
            )
            .unwrap(),
        (Operation::Analysis, 1) => plan
            .analysis_spin_into(
                black_box(&data.first_map),
                black_box(&data.second_map),
                black_box(&mut data.output_e),
                black_box(&mut data.output_b),
                workspace,
            )
            .unwrap(),
        _ => unreachable!(),
    }
    black_box(data.output_first.first());
    black_box(data.output_e.first());
}

fn run_batch(
    pool: &rayon::ThreadPool,
    plan: &ShtPlan,
    operation: Operation,
    spin: usize,
    schedule: Schedule,
    data: &mut Data,
    workspaces: &mut [rustsht::ShtWorkspace],
) {
    match schedule {
        Schedule::Sequential => pool.install(|| {
            for field in &mut data.fields {
                run_one(plan, operation, spin, field, &mut workspaces[0]);
            }
        }),
        Schedule::ParallelOneThreadWorkspaces | Schedule::ParallelPoolSizedWorkspaces => {
            use rayon::prelude::*;
            pool.install(|| {
                workspaces
                    .par_iter_mut()
                    .zip(data.fields.par_iter_mut())
                    .try_for_each(|(workspace, field)| {
                        run_one(plan, operation, spin, field, workspace);
                        Ok::<(), rustsht::ShtError>(())
                    })
                    .unwrap();
            });
        }
    }
}

fn sample_batch(
    pool: &rayon::ThreadPool,
    plan: &ShtPlan,
    operation: Operation,
    spin: usize,
    schedule: Schedule,
    data: &mut Data,
    workspaces: &mut [rustsht::ShtWorkspace],
) -> Duration {
    let start = Instant::now();
    run_batch(pool, plan, operation, spin, schedule, data, workspaces);
    start.elapsed()
}

fn parse_schedule(text: &str) -> Schedule {
    match text {
        "sequential" => Schedule::Sequential,
        "parallel-one" => Schedule::ParallelOneThreadWorkspaces,
        "parallel-pool" => Schedule::ParallelPoolSizedWorkspaces,
        _ => panic!("schedule must be sequential, parallel-one, or parallel-pool"),
    }
}

fn output_checksum(operation: Operation, fields: &[FieldData]) -> f64 {
    fields
        .iter()
        .map(|field| match operation {
            Operation::Synthesis => {
                field.output_first.iter().sum::<f64>()
                    + field.output_second.iter().sum::<f64>() * 0.7
            }
            Operation::Analysis => {
                field
                    .output_e
                    .iter()
                    .map(|value| value.re + value.im * 0.3)
                    .sum::<f64>()
                    + field
                        .output_b
                        .iter()
                        .map(|value| value.re + value.im * 0.7)
                        .sum::<f64>()
            }
        })
        .sum()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let spin: usize = args.next().unwrap_or_else(|| "0".into()).parse().unwrap();
    let lmax: usize = args.next().unwrap_or_else(|| "127".into()).parse().unwrap();
    let threads: usize = args.next().unwrap_or_else(|| "8".into()).parse().unwrap();
    let operation = match args.next().as_deref().unwrap_or("synthesis") {
        "synthesis" => Operation::Synthesis,
        "analysis" => Operation::Analysis,
        _ => panic!("operation must be synthesis or analysis"),
    };
    let fields: usize = args.next().unwrap_or_else(|| "4".into()).parse().unwrap();
    let schedule_text = args.next().unwrap_or_else(|| "sequential".into());
    let schedule = parse_schedule(&schedule_text);
    let ntheta: usize = args
        .next()
        .unwrap_or_else(|| (lmax + 1).to_string())
        .parse()
        .unwrap();
    let nphi: usize = args
        .next()
        .unwrap_or_else(|| (2 * lmax + 3).to_string())
        .parse()
        .unwrap();
    let mmax: usize = args
        .next()
        .unwrap_or_else(|| lmax.to_string())
        .parse()
        .unwrap();
    let reps: usize = args.next().unwrap_or_else(|| "5".into()).parse().unwrap();
    assert!(matches!(spin, 0 | 1));
    assert!(fields > 0 && reps > 0);
    if matches!(schedule, Schedule::Sequential) && fields > 1 {
        // Sequential execution needs only one reusable workspace.
    }

    let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap();
    let harmonics = HarmonicSpec::new(lmax, mmax, spin).unwrap();
    let plan_start = Instant::now();
    let plan = ShtPlan::new(grid, harmonics).unwrap();
    let plan_construction = plan_start.elapsed();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    let one_thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let workspace_count = if matches!(schedule, Schedule::Sequential) {
        1
    } else {
        fields
    };
    let rss_before_workspaces = rss_kib();
    let workspace_start = Instant::now();
    let mut workspaces = match schedule {
        Schedule::Sequential | Schedule::ParallelPoolSizedWorkspaces => pool.install(|| {
            (0..workspace_count)
                .map(|_| plan.workspace().unwrap())
                .collect::<Vec<_>>()
        }),
        Schedule::ParallelOneThreadWorkspaces => one_thread_pool.install(|| {
            (0..workspace_count)
                .map(|_| plan.workspace().unwrap())
                .collect::<Vec<_>>()
        }),
    };
    let workspace_allocation = workspace_start.elapsed();
    let rss_after_workspaces = rss_kib();
    let mut data = make_data(&plan, fields);

    let first_batch = sample_batch(
        &pool,
        &plan,
        operation,
        spin,
        schedule,
        &mut data,
        &mut workspaces,
    );
    let _ = sample_batch(
        &pool,
        &plan,
        operation,
        spin,
        schedule,
        &mut data,
        &mut workspaces,
    );
    let rss_after_warm = rss_kib();
    let mut samples = Vec::with_capacity(reps);
    for _ in 0..reps {
        samples.push(sample_batch(
            &pool,
            &plan,
            operation,
            spin,
            schedule,
            &mut data,
            &mut workspaces,
        ));
    }
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    let sample_text = samples
        .iter()
        .map(|sample| format!("{:.3}", sample.as_secs_f64() * 1000.0))
        .collect::<Vec<_>>()
        .join(",");
    let workspace_strategy = match schedule {
        Schedule::Sequential => "one-pool-sized-workspace",
        Schedule::ParallelOneThreadWorkspaces => "one-thread-workspaces",
        Schedule::ParallelPoolSizedWorkspaces => "pool-sized-workspaces",
    };
    let operation_name = match operation {
        Operation::Synthesis => "synthesis",
        Operation::Analysis => "analysis",
    };
    println!(
        "spin={spin} T{lmax} GL {ntheta}x{nphi} mmax={mmax} f64 operation={operation_name} threads={threads} fields={fields} schedule={schedule_text} workspace={workspace_strategy} reps={reps}"
    );
    println!(
        "plan_construction_ms={:.3}",
        plan_construction.as_secs_f64() * 1000.0
    );
    println!(
        "workspace_allocation_ms={:.3}",
        workspace_allocation.as_secs_f64() * 1000.0
    );
    println!(
        "rss_before_workspaces_kib={} rss_after_workspaces_kib={} workspace_rss_delta_kib={} peak_rss_kib={}",
        rss_before_workspaces.0,
        rss_after_workspaces.0,
        rss_after_workspaces
            .0
            .saturating_sub(rss_before_workspaces.0),
        rss_after_workspaces.1
    );
    println!(
        "rss_after_warm_kib={} peak_rss_after_warm_kib={}",
        rss_after_warm.0, rss_after_warm.1
    );
    println!("first_batch_ms={:.3}", first_batch.as_secs_f64() * 1000.0);
    println!("warm_batch_samples_ms=[{sample_text}]");
    println!("warm_batch_median_ms={:.3}", median.as_secs_f64() * 1000.0);
    println!(
        "warm_per_field_median_ms={:.3} fields_per_second={:.3}",
        median.as_secs_f64() * 1000.0 / fields as f64,
        fields as f64 / median.as_secs_f64()
    );
    println!(
        "output_checksum={:.17e}",
        output_checksum(operation, &data.fields)
    );
}
