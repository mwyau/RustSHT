// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Native RustSHT f32 worker for matched DUCC full-transform measurements.

use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

use pulp::{Simd, WithSimd};
use rustsht::{Complex32, GridKind, GridSpec, HarmonicSpec, ShtPlan, ShtWorkspace};

struct F32LaneCount;

impl WithSimd for F32LaneCount {
    type Output = usize;

    fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
        S::F32_LANES
    }
}

fn read_real(path: &str, count: usize) -> Result<Vec<f32>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() != count * 4 {
        return Err(format!("{path} has {} bytes; expected {}", bytes.len(), count * 4).into());
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect())
}

fn read_complex(path: &str, count: usize) -> Result<Vec<Complex32>, Box<dyn Error>> {
    let values = read_real(path, count * 2)?;
    Ok(values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Complex32::new(pair[0], pair[1]))
        .collect())
}

fn write_real(path: &str, values: &[f32]) -> Result<(), Box<dyn Error>> {
    let mut file = fs::File::create(path)?;
    for value in values {
        file.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn write_complex(path: &str, values: &[Complex32]) -> Result<(), Box<dyn Error>> {
    let mut file = fs::File::create(path)?;
    for value in values {
        file.write_all(&value.re.to_le_bytes())?;
        file.write_all(&value.im.to_le_bytes())?;
    }
    Ok(())
}

fn same_real(left: &[f32], right: &[f32]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.to_bits() == b.to_bits())
}

fn same_complex(left: &[Complex32], right: &[Complex32]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
}

fn checksum_real(values: &[f32]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let stride = (values.len() / 64).max(1);
    values
        .iter()
        .step_by(stride)
        .take(64)
        .map(|&value| value as f64)
        .sum()
}

fn checksum_complex(values: &[Complex32]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let stride = (values.len() / 64).max(1);
    values
        .iter()
        .step_by(stride)
        .take(64)
        .map(|value| (value.re + value.im) as f64)
        .sum()
}

fn json_f64_array(values: &[f64]) -> String {
    let mut result = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            result.push(',');
        }
        result.push_str(&format!("{value:.17}"));
    }
    result.push(']');
    result
}

#[allow(clippy::too_many_arguments)]
fn run_once(
    operation: &str,
    spin: usize,
    plan: &ShtPlan<f32>,
    map0: &[f32],
    map1: &[f32],
    alm0: &[Complex32],
    alm1: &[Complex32],
    map_out0: &mut [f32],
    map_out1: &mut [f32],
    alm_out0: &mut [Complex32],
    alm_out1: &mut [Complex32],
    workspace: &mut ShtWorkspace<f32>,
) -> Result<(), rustsht::ShtError> {
    match (operation, spin) {
        ("analysis", 0) => plan.analysis_into(map0, alm_out0, workspace),
        ("analysis", 1) => plan.analysis_spin_into(map0, map1, alm_out0, alm_out1, workspace),
        ("synthesis", 0) => plan.synthesis_into(alm0, map_out0, workspace),
        ("synthesis", 1) => plan.synthesis_spin_into(alm0, alm1, map_out0, map_out1, workspace),
        _ => unreachable!("worker only supports scalar spin 0 and STANDARD spin 1"),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 18 || args[1] != "--worker" {
        return Err("usage: --worker OP SPIN GRID THREADS NTHETA NPHI LMAX MMAX PHI0 MAP0 MAP1 ALM0 ALM1 OUT0 OUT1 TARGET_MS".into());
    }
    let operation = args[2].as_str();
    if operation != "analysis" && operation != "synthesis" {
        return Err("operation must be analysis or synthesis".into());
    }
    let spin: usize = args[3].parse()?;
    if spin > 1 {
        return Err("spin must be 0 or 1".into());
    }
    let grid_kind = match args[4].as_str() {
        "GL" => GridKind::GaussLegendre,
        "CC" => GridKind::ClenshawCurtis,
        "F1" => GridKind::Fejer1,
        other => return Err(format!("unknown grid {other}").into()),
    };
    let threads: usize = args[5].parse()?;
    let ntheta: usize = args[6].parse()?;
    let nphi: usize = args[7].parse()?;
    let lmax: usize = args[8].parse()?;
    let mmax: usize = args[9].parse()?;
    let phi0: f64 = args[10].parse()?;
    let target_ms: f64 = args[17].parse()?;
    let selected_arch = pulp::Arch::new();
    let selected_arch_name = format!("{selected_arch:?}");
    let f32_lanes = selected_arch.dispatch(F32LaneCount);
    let map_count = ntheta * nphi;
    let nalm = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) / 2;

    let read_started = Instant::now();
    let map0 = read_real(&args[11], map_count)?;
    let map1 = read_real(&args[12], map_count)?;
    let alm0 = read_complex(&args[13], nalm)?;
    let alm1 = read_complex(&args[14], nalm)?;
    let fixture_load_ms = read_started.elapsed().as_secs_f64() * 1e3;
    let mut map_out0 = vec![f32::NAN; map_count];
    let mut map_out1 = vec![f32::NAN; map_count];
    let mut alm_out0 = vec![Complex32::new(f32::NAN, f32::NAN); nalm];
    let mut alm_out1 = vec![Complex32::new(f32::NAN, f32::NAN); nalm];

    let pool_started = Instant::now();
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()?;
    if rayon::current_num_threads() != threads {
        return Err(format!(
            "Rayon global pool has {} threads, expected {threads}",
            rayon::current_num_threads()
        )
        .into());
    }
    let pool_setup_ms = pool_started.elapsed().as_secs_f64() * 1e3;

    let setup_started = Instant::now();
    let grid = GridSpec::new(grid_kind, ntheta, nphi, phi0)?;
    let harmonics = HarmonicSpec::new(lmax, mmax, spin)?;
    let plan = ShtPlan::<f32>::new(grid, harmonics)?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1e3;

    let first_started = Instant::now();
    run_once(
        operation,
        spin,
        &plan,
        &map0,
        &map1,
        &alm0,
        &alm1,
        &mut map_out0,
        &mut map_out1,
        &mut alm_out0,
        &mut alm_out1,
        &mut workspace,
    )?;
    let first_ms = first_started.elapsed().as_secs_f64() * 1e3;
    let first_maps = (map_out0.clone(), map_out1.clone());
    let first_alms = (alm_out0.clone(), alm_out1.clone());

    let warmup_started = Instant::now();
    run_once(
        operation,
        spin,
        &plan,
        &map0,
        &map1,
        &alm0,
        &alm1,
        &mut map_out0,
        &mut map_out1,
        &mut alm_out0,
        &mut alm_out1,
        &mut workspace,
    )?;
    let warmup_ms = warmup_started.elapsed().as_secs_f64() * 1e3;
    let repeat_consistent = if operation == "analysis" {
        same_complex(&first_alms.0, &alm_out0)
            && (spin == 0 || same_complex(&first_alms.1, &alm_out1))
    } else {
        same_real(&first_maps.0, &map_out0) && (spin == 0 || same_real(&first_maps.1, &map_out1))
    };

    if operation == "analysis" {
        alm_out0.fill(Complex32::new(f32::NAN, f32::NAN));
        alm_out1.fill(Complex32::new(f32::NAN, f32::NAN));
    } else {
        map_out0.fill(f32::NAN);
        map_out1.fill(f32::NAN);
    }
    run_once(
        operation,
        spin,
        &plan,
        &map0,
        &map1,
        &alm0,
        &alm1,
        &mut map_out0,
        &mut map_out1,
        &mut alm_out0,
        &mut alm_out1,
        &mut workspace,
    )?;
    let reset_consistent = if operation == "analysis" {
        same_complex(&first_alms.0, &alm_out0)
            && (spin == 0 || same_complex(&first_alms.1, &alm_out1))
    } else {
        same_real(&first_maps.0, &map_out0) && (spin == 0 || same_real(&first_maps.1, &map_out1))
    };

    let individually_timed = warmup_ms >= 100.0;
    let mut repetitions = if individually_timed {
        1
    } else {
        (target_ms / warmup_ms.max(0.000001))
            .ceil()
            .clamp(1.0, 2_000_000.0) as usize
    };
    if !individually_timed {
        for _ in 0..3 {
            let started = Instant::now();
            for _ in 0..repetitions {
                run_once(
                    operation,
                    spin,
                    &plan,
                    &map0,
                    &map1,
                    &alm0,
                    &alm1,
                    &mut map_out0,
                    &mut map_out1,
                    &mut alm_out0,
                    &mut alm_out1,
                    &mut workspace,
                )?;
            }
            let elapsed = started.elapsed().as_secs_f64() * 1e3;
            if (20.0..=100.0).contains(&elapsed) {
                break;
            }
            repetitions = (repetitions as f64 * target_ms / elapsed.max(0.000001))
                .ceil()
                .clamp(1.0, 2_000_000.0) as usize;
        }
    }
    let batch_count = if individually_timed { 5 } else { 10 };
    let mut batch_ms = Vec::with_capacity(batch_count);
    let mut batch_total_ms = Vec::with_capacity(batch_count);
    let mut output_checksum = 0.0;
    for _ in 0..batch_count {
        let started = Instant::now();
        for _ in 0..repetitions {
            run_once(
                operation,
                spin,
                &plan,
                &map0,
                &map1,
                &alm0,
                &alm1,
                &mut map_out0,
                &mut map_out1,
                &mut alm_out0,
                &mut alm_out1,
                &mut workspace,
            )?;
        }
        let elapsed = started.elapsed().as_secs_f64() * 1e3;
        batch_total_ms.push(elapsed);
        batch_ms.push(elapsed / repetitions as f64);
        output_checksum += if operation == "analysis" {
            checksum_complex(&alm_out0)
                + if spin == 0 {
                    0.0
                } else {
                    checksum_complex(&alm_out1)
                }
        } else {
            checksum_real(&map_out0)
                + if spin == 0 {
                    0.0
                } else {
                    checksum_real(&map_out1)
                }
        };
        black_box(output_checksum);
    }

    if operation == "analysis" {
        write_complex(&args[15], &alm_out0)?;
        if spin == 1 {
            write_complex(&args[16], &alm_out1)?;
        }
    } else {
        write_real(&args[15], &map_out0)?;
        if spin == 1 {
            write_real(&args[16], &map_out1)?;
        }
    }
    let finite_outputs = if operation == "analysis" {
        alm_out0
            .iter()
            .all(|v| v.re.is_finite() && v.im.is_finite())
            && (spin == 0
                || alm_out1
                    .iter()
                    .all(|v| v.re.is_finite() && v.im.is_finite()))
    } else {
        map_out0.iter().all(|v| v.is_finite())
            && (spin == 0 || map_out1.iter().all(|v| v.is_finite()))
    };
    println!(
        "{{\"backend\":\"rustsht\",\"operation\":\"{operation}\",\"spin\":{spin},\"grid\":\"{}\",\"threads\":{threads},\"lmax\":{lmax},\"mmax\":{mmax},\"simd_arch\":\"{selected_arch_name}\",\"f32_lanes\":{f32_lanes},\"first_ms\":{first_ms:.17},\"warmup_ms\":{warmup_ms:.17},\"pool_setup_ms\":{pool_setup_ms:.17},\"setup_ms\":{setup_ms:.17},\"fixture_load_ms\":{fixture_load_ms:.17},\"repetitions_per_batch\":{repetitions},\"batch_ms\":{},\"batch_total_ms\":{},\"output_checksum\":{output_checksum:.17},\"finite_outputs\":{finite_outputs},\"repeat_consistent\":{repeat_consistent},\"reset_consistent\":{reset_consistent}}}",
        args[4],
        json_f64_array(&batch_ms),
        json_f64_array(&batch_total_ms),
    );
    if !finite_outputs || !repeat_consistent || !reset_consistent {
        return Err("nonfinite output or output repeat/reset mismatch".into());
    }
    Ok(())
}
