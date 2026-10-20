// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Native RustSHT side of the reproducible full-transform parity benchmark.

use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

use rayon::ThreadPoolBuilder;
use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn read_f64(path: &str, count: usize) -> Result<Vec<f64>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() != count * std::mem::size_of::<f64>() {
        return Err(format!("{} has {} bytes, expected {}", path, bytes.len(), count * 8).into());
    }
    let (chunks, remainder) = bytes.as_chunks::<8>();
    debug_assert!(remainder.is_empty());
    Ok(chunks
        .iter()
        .map(|chunk| f64::from_le_bytes(*chunk))
        .collect())
}

fn read_complex(path: &str, count: usize) -> Result<Vec<Complex64>, Box<dyn Error>> {
    let values = read_f64(path, count * 2)?;
    Ok(values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Complex64::new(pair[0], pair[1]))
        .collect())
}

fn write_f64(path: &str, values: &[f64]) -> Result<(), Box<dyn Error>> {
    let mut file = fs::File::create(path)?;
    for value in values {
        file.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn write_complex(path: &str, values: &[Complex64]) -> Result<(), Box<dyn Error>> {
    let mut file = fs::File::create(path)?;
    for value in values {
        file.write_all(&value.re.to_le_bytes())?;
        file.write_all(&value.im.to_le_bytes())?;
    }
    Ok(())
}

fn checksum_f64(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let stride = (values.len() / 32).max(1);
    values.iter().step_by(stride).take(32).sum()
}

fn checksum_complex(values: &[Complex64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let stride = (values.len() / 32).max(1);
    values
        .iter()
        .step_by(stride)
        .take(32)
        .map(|value| value.re + value.im)
        .sum()
}

fn relative_l2<T>(
    actual: &[T],
    expected: &[T],
    distance: impl Fn(&T, &T) -> f64,
    norm: impl Fn(&T) -> f64,
) -> f64 {
    let error = actual
        .iter()
        .zip(expected)
        .map(|(left, right)| distance(left, right).powi(2))
        .sum::<f64>()
        .sqrt();
    let scale = expected
        .iter()
        .map(|value| norm(value).powi(2))
        .sum::<f64>()
        .sqrt();
    error / scale.max(f64::MIN_POSITIVE)
}

fn relative_complex_l2(actual: &[Complex64], expected: &[Complex64]) -> f64 {
    relative_l2(
        actual,
        expected,
        |a, b| (*a - *b).norm(),
        |value| value.norm(),
    )
}

fn finite_real(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn finite_complex(values: &[Complex64]) -> bool {
    values
        .iter()
        .all(|value| value.re.is_finite() && value.im.is_finite())
}

fn map_dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn harmonic_dot(left: &[Complex64], right: &[Complex64], lmax: usize, mmax: usize) -> f64 {
    let mut result = 0.0;
    let mut offset = 0;
    for m in 0..=mmax {
        let order_len = lmax - m + 1;
        let weight = if m == 0 { 1.0 } else { 2.0 };
        for index in offset..offset + order_len {
            result +=
                weight * (left[index].re * right[index].re + left[index].im * right[index].im);
        }
        offset += order_len;
    }
    result
}

fn identity_residual(left: f64, right: f64) -> f64 {
    (left - right).abs() / left.abs().max(right.abs()).max(f64::MIN_POSITIVE)
}

fn parse<T: std::str::FromStr>(value: &str, name: &str) -> Result<T, Box<dyn Error>>
where
    T::Err: Error + 'static,
{
    value
        .parse()
        .map_err(|error| format!("invalid {name}: {error}").into())
}

#[allow(clippy::too_many_arguments)]
fn run_once(
    operation: &str,
    pool: &rayon::ThreadPool,
    plan: &ShtPlan,
    map: &[f64],
    alm: &[Complex64],
    map_output: &mut [f64],
    alm_output: &mut [Complex64],
    workspace: &mut rustsht::ShtWorkspace,
) -> Result<(), rustsht::ShtError> {
    pool.install(|| match operation {
        "analysis" => plan.analysis_into(map, alm_output, workspace),
        "synthesis" => plan.synthesis_into(alm, map_output, workspace),
        _ => unreachable!(),
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 14 || args[1] != "--worker" {
        return Err("usage: --worker OP THREADS NTHETA NPHI LMAX MMAX PHI0 MAP ALM OUT BATCH_TARGET_MS VALIDATE".into());
    }
    let operation = args[2].as_str();
    if operation != "analysis" && operation != "synthesis" {
        return Err("operation must be analysis or synthesis".into());
    }
    let threads: usize = parse(&args[3], "threads")?;
    let ntheta: usize = parse(&args[4], "ntheta")?;
    let nphi: usize = parse(&args[5], "nphi")?;
    let lmax: usize = parse(&args[6], "lmax")?;
    let mmax: usize = parse(&args[7], "mmax")?;
    let phi0: f64 = parse(&args[8], "phi0")?;
    let map_path = &args[9];
    let alm_path = &args[10];
    let output_path = &args[11];
    let target_ms: f64 = parse(&args[12], "batch target milliseconds")?;
    let validate = args[13] == "1";

    let fixture_load_started = Instant::now();
    let map = read_f64(map_path, ntheta * nphi)?;
    let alm_count = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) / 2;
    let alm = read_complex(alm_path, alm_count)?;
    let fixture_load_ms = fixture_load_started.elapsed().as_secs_f64() * 1e3;
    let buffer_setup_started = Instant::now();
    let mut map_output = vec![0.0; ntheta * nphi];
    let mut alm_output = vec![Complex64::new(0.0, 0.0); alm_count];
    let buffer_setup_ms = buffer_setup_started.elapsed().as_secs_f64() * 1e3;

    let pool_started = Instant::now();
    let pool = ThreadPoolBuilder::new().num_threads(threads).build()?;
    let pool_ms = pool_started.elapsed().as_secs_f64() * 1e3;
    let setup_started = Instant::now();
    let (plan, mut workspace) = pool.install(|| {
        let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, phi0)?;
        let harmonics = HarmonicSpec::scalar(lmax, mmax)?;
        let plan = ShtPlan::new(grid, harmonics)?;
        let mut workspace = plan.workspace()?;
        plan.prepare_workspace(&mut workspace, threads)?;
        Ok::<_, rustsht::ShtError>((plan, workspace))
    })?;
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1e3;

    let first_started = Instant::now();
    run_once(
        operation,
        &pool,
        &plan,
        &map,
        &alm,
        &mut map_output,
        &mut alm_output,
        &mut workspace,
    )?;
    let first_ms = first_started.elapsed().as_secs_f64() * 1e3;
    let first_map_output = map_output.clone();
    let first_alm_output = alm_output.clone();

    let warmup_started = Instant::now();
    run_once(
        operation,
        &pool,
        &plan,
        &map,
        &alm,
        &mut map_output,
        &mut alm_output,
        &mut workspace,
    )?;
    let warmup_ms = warmup_started.elapsed().as_secs_f64() * 1e3;
    let output_checksum = if operation == "analysis" {
        checksum_complex(black_box(&alm_output))
    } else {
        checksum_f64(black_box(&map_output))
    };

    let mut repetitions = if warmup_ms >= 100.0 {
        1
    } else {
        ((target_ms / warmup_ms.max(0.000_001)).ceil() as usize).clamp(1, 2_000_000)
    };
    let batch_count = if warmup_ms >= 100.0 { 5 } else { 10 };
    let mut calibration_batch_total_ms = Vec::new();
    if warmup_ms < 100.0 {
        for _ in 0..3 {
            let started = Instant::now();
            for _ in 0..repetitions {
                run_once(
                    operation,
                    &pool,
                    &plan,
                    &map,
                    &alm,
                    &mut map_output,
                    &mut alm_output,
                    &mut workspace,
                )?;
            }
            let elapsed_ms = started.elapsed().as_secs_f64() * 1e3;
            calibration_batch_total_ms.push(elapsed_ms);
            if (20.0..=100.0).contains(&elapsed_ms) {
                break;
            }
            repetitions = ((repetitions as f64 * target_ms / elapsed_ms.max(0.000_001)).ceil()
                as usize)
                .clamp(1, 2_000_000);
        }
    }
    let mut batch_ms = Vec::with_capacity(batch_count);
    let mut batch_total_ms = Vec::with_capacity(batch_count);
    let mut checksum = 0.0;
    for _ in 0..batch_count {
        let started = Instant::now();
        for _ in 0..repetitions {
            run_once(
                operation,
                &pool,
                &plan,
                &map,
                &alm,
                &mut map_output,
                &mut alm_output,
                &mut workspace,
            )?;
        }
        let elapsed_ms = started.elapsed().as_secs_f64() * 1e3;
        batch_total_ms.push(elapsed_ms);
        batch_ms.push(elapsed_ms / repetitions as f64);
        checksum += if operation == "analysis" {
            checksum_complex(black_box(&alm_output))
        } else {
            checksum_f64(black_box(&map_output))
        };
    }
    let reset_consistent = if operation == "analysis" {
        first_alm_output == alm_output
    } else {
        first_map_output == map_output
    };

    if operation == "analysis" {
        write_complex(output_path, &alm_output)?;
    } else {
        write_f64(output_path, &map_output)?;
    }

    let validation_json = if validate {
        let suffix = |name: &str| format!("{output_path}.{name}");
        let mut roundtrip_map = vec![0.0; ntheta * nphi];
        let mut roundtrip_alm = vec![Complex64::new(0.0, 0.0); alm_count];
        pool.install(|| -> Result<(), rustsht::ShtError> {
            plan.synthesis_into(&alm, &mut roundtrip_map, &mut workspace)?;
            plan.analysis_into(&roundtrip_map, &mut roundtrip_alm, &mut workspace)
        })?;
        let roundtrip_error = relative_complex_l2(&roundtrip_alm, &alm);
        write_complex(&suffix("roundtrip.alm"), &roundtrip_alm)?;

        let mut selected_modes = vec![
            (0, 0),
            (lmax, 0),
            (lmax / 2, mmax.min(lmax / 2)),
            (lmax, mmax),
        ];
        selected_modes.sort_unstable();
        selected_modes.dedup();
        for (mode_index, (degree, order)) in selected_modes.iter().copied().enumerate() {
            let mut basis = vec![Complex64::new(0.0, 0.0); alm_count];
            let index = plan
                .harmonics()
                .index(degree, order)
                .expect("selected mode is valid");
            basis[index] = Complex64::new(1.0, 0.0);
            let mut basis_map = vec![0.0; ntheta * nphi];
            let mut recovered = vec![Complex64::new(0.0, 0.0); alm_count];
            pool.install(|| -> Result<(), rustsht::ShtError> {
                plan.synthesis_into(&basis, &mut basis_map, &mut workspace)?;
                plan.analysis_into(&basis_map, &mut recovered, &mut workspace)
            })?;
            write_f64(&suffix(&format!("basis-{mode_index}.map")), &basis_map)?;
            write_complex(&suffix(&format!("basis-{mode_index}.alm")), &recovered)?;
        }

        let zero_alm = vec![Complex64::new(0.0, 0.0); alm_count];
        let mut zero_map = vec![0.0; ntheta * nphi];
        let mut zero_analysis = vec![Complex64::new(0.0, 0.0); alm_count];
        pool.install(|| -> Result<(), rustsht::ShtError> {
            plan.synthesis_into(&zero_alm, &mut zero_map, &mut workspace)?;
            plan.analysis_into(&zero_map, &mut zero_analysis, &mut workspace)
        })?;
        write_f64(&suffix("zero.map"), &zero_map)?;
        write_complex(&suffix("zero.alm"), &zero_analysis)?;

        let mut edge_alm = vec![Complex64::new(0.0, 0.0); alm_count];
        let sectoral = plan
            .harmonics()
            .index(lmax, mmax)
            .expect("sectoral mode is valid");
        edge_alm[sectoral] = Complex64::new(1.0e150, if mmax == 0 { 0.0 } else { -2.0e149 });
        if lmax >= 1 {
            edge_alm[0] = Complex64::new(1.0e150, 0.0);
            edge_alm[1] = Complex64::new(-1.0e150, 0.0);
        }
        let mut edge_map = vec![0.0; ntheta * nphi];
        let mut edge_analysis = vec![Complex64::new(0.0, 0.0); alm_count];
        pool.install(|| -> Result<(), rustsht::ShtError> {
            plan.synthesis_into(&edge_alm, &mut edge_map, &mut workspace)?;
            plan.analysis_into(&edge_map, &mut edge_analysis, &mut workspace)
        })?;
        write_f64(&suffix("extreme.map"), &edge_map)?;
        write_complex(&suffix("extreme.alm"), &edge_analysis)?;

        let mut subnormal_alm = vec![Complex64::new(0.0, 0.0); alm_count];
        subnormal_alm[0] = Complex64::new(f64::from_bits(1), 0.0);
        let mut subnormal_map = vec![0.0; ntheta * nphi];
        let subnormal_input = vec![f64::from_bits(1); ntheta * nphi];
        let mut subnormal_analysis = vec![Complex64::new(0.0, 0.0); alm_count];
        pool.install(|| -> Result<(), rustsht::ShtError> {
            plan.synthesis_into(&subnormal_alm, &mut subnormal_map, &mut workspace)?;
            plan.analysis_into(&subnormal_input, &mut subnormal_analysis, &mut workspace)
        })?;
        write_f64(&suffix("subnormal.map"), &subnormal_map)?;
        write_complex(&suffix("subnormal.alm"), &subnormal_analysis)?;

        let mut adjoint_synthesis = vec![Complex64::new(0.0, 0.0); alm_count];
        let mut synthesized = vec![0.0; ntheta * nphi];
        pool.install(|| -> Result<(), rustsht::ShtError> {
            plan.synthesis_into(&alm, &mut synthesized, &mut workspace)?;
            plan.adjoint_synthesis_into(&map, &mut adjoint_synthesis, &mut workspace)
        })?;
        let synthesis_left = map_dot(&synthesized, &map);
        let synthesis_right = harmonic_dot(&alm, &adjoint_synthesis, lmax, mmax);
        let mut analyzed = vec![Complex64::new(0.0, 0.0); alm_count];
        let mut adjoint_analysis = vec![0.0; ntheta * nphi];
        pool.install(|| -> Result<(), rustsht::ShtError> {
            plan.analysis_into(&map, &mut analyzed, &mut workspace)?;
            plan.adjoint_analysis_into(&alm, &mut adjoint_analysis, &mut workspace)
        })?;
        let analysis_left = harmonic_dot(&analyzed, &alm, lmax, mmax);
        let analysis_right = map_dot(&map, &adjoint_analysis);

        format!(
            "{{\"roundtrip_relative_l2\":{roundtrip_error:.17e},\"selected_basis_modes\":{},\"zero_finite\":{},\"extreme_finite\":{},\"subnormal_finite\":{},\"synthesis_adjoint_relative_residual\":{:.17e},\"analysis_adjoint_relative_residual\":{:.17e}}}",
            selected_modes.len(),
            finite_real(&zero_map) && finite_complex(&zero_analysis),
            finite_real(&edge_map) && finite_complex(&edge_analysis),
            finite_real(&subnormal_map) && finite_complex(&subnormal_analysis),
            identity_residual(synthesis_left, synthesis_right),
            identity_residual(analysis_left, analysis_right),
        )
    } else {
        "null".to_owned()
    };
    let rss_kib = fs::read_to_string("/proc/self/status")?
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:")?.split_whitespace().next())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);

    print!(
        "{{\"backend\":\"rustsht\",\"operation\":\"{operation}\",\"threads\":{threads},\"fixture_load_ms\":{fixture_load_ms:.9},\"buffer_setup_ms\":{buffer_setup_ms:.9},\"pool_ms\":{pool_ms:.9},\"setup_ms\":{setup_ms:.9},\"first_ms\":{first_ms:.9},\"warmup_ms\":{warmup_ms:.9},\"repetitions\":{repetitions},\"batch_ms\":["
    );
    for (index, value) in batch_ms.iter().enumerate() {
        if index != 0 {
            print!(",");
        }
        print!("{value:.9}");
    }
    print!("],\"batch_total_ms\":[");
    for (index, value) in batch_total_ms.iter().enumerate() {
        if index != 0 {
            print!(",");
        }
        print!("{value:.9}");
    }
    print!("],\"calibration_batch_total_ms\":[");
    for (index, value) in calibration_batch_total_ms.iter().enumerate() {
        if index != 0 {
            print!(",");
        }
        print!("{value:.9}");
    }
    println!(
        "],\"checksum\":{:.17e},\"reset_consistent\":{reset_consistent},\"thread_pool_size\":{},\"arch\":\"{:?}\",\"lane_width_f64\":4,\"rss_kib\":{rss_kib},\"validation\":{validation_json}}}",
        checksum + output_checksum,
        pool.current_num_threads(),
        pulp::Arch::new()
    );
    Ok(())
}
