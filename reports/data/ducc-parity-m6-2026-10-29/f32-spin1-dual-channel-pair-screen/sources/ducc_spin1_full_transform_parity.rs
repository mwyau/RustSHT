// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Native RustSHT side of the matched spin-1 E/B benchmark.

use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan, ShtWorkspace};

fn read_f64(path: &str, count: usize) -> Result<Vec<f64>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() != count * 8 {
        return Err(format!("{path} has {} bytes; expected {}", bytes.len(), count * 8).into());
    }
    Ok(bytes
        .as_chunks::<8>()
        .0
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
        .map(|v| Complex64::new(v[0], v[1]))
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

fn checksum(values: &[f64]) -> f64 {
    values
        .iter()
        .step_by((values.len() / 64).max(1))
        .take(64)
        .sum()
}

fn checksum_complex(values: &[Complex64]) -> f64 {
    values
        .iter()
        .step_by((values.len() / 64).max(1))
        .take(64)
        .map(|v| v.re + v.im)
        .sum()
}

fn same_real(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

fn same_complex(a: &[Complex64], b: &[Complex64]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.re.to_bits() == y.re.to_bits() && x.im.to_bits() == y.im.to_bits())
}

#[allow(clippy::too_many_arguments)]
fn run_once(
    operation: &str,
    plan: &ShtPlan,
    map_first: &[f64],
    map_second: &[f64],
    e: &[Complex64],
    b: &[Complex64],
    out_first: &mut [f64],
    out_second: &mut [f64],
    out_e: &mut [Complex64],
    out_b: &mut [Complex64],
    workspace: &mut ShtWorkspace,
) -> Result<(), rustsht::ShtError> {
    match operation {
        "analysis" => plan.analysis_spin_into(map_first, map_second, out_e, out_b, workspace),
        "synthesis" => plan.synthesis_spin_into(e, b, out_first, out_second, workspace),
        _ => unreachable!(),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 19 || args[1] != "--worker" {
        return Err("usage: --worker OP GRID THREADS NTHETA NPHI LMAX MMAX PHI0 MAP0 MAP1 E B OUT0 OUT1 OUTE OUTB TARGET_MS".into());
    }
    let operation = args[2].as_str();
    if operation != "analysis" && operation != "synthesis" {
        return Err("operation must be analysis or synthesis".into());
    }
    let grid_kind = match args[3].as_str() {
        "GL" => GridKind::GaussLegendre,
        "CC" => GridKind::ClenshawCurtis,
        "F1" => GridKind::Fejer1,
        other => return Err(format!("unknown grid {other}").into()),
    };
    let threads: usize = args[4].parse()?;
    let ntheta: usize = args[5].parse()?;
    let nphi: usize = args[6].parse()?;
    let lmax: usize = args[7].parse()?;
    let mmax: usize = args[8].parse()?;
    let phi0: f64 = args[9].parse()?;
    let target_ms: f64 = args[18].parse()?;
    let map_count = ntheta * nphi;
    let nalm = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) / 2;

    let read_started = Instant::now();
    let map_first = read_f64(&args[10], map_count)?;
    let map_second = read_f64(&args[11], map_count)?;
    let e = read_complex(&args[12], nalm)?;
    let b = read_complex(&args[13], nalm)?;
    let read_ms = read_started.elapsed().as_secs_f64() * 1e3;
    let mut out_first = vec![0.0; map_count];
    let mut out_second = vec![0.0; map_count];
    let mut out_e = vec![Complex64::new(0.0, 0.0); nalm];
    let mut out_b = vec![Complex64::new(0.0, 0.0); nalm];

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
    let pool_ms = pool_started.elapsed().as_secs_f64() * 1e3;

    let setup_started = Instant::now();
    let grid = GridSpec::new(grid_kind, ntheta, nphi, phi0)?;
    let harmonics = HarmonicSpec::new(lmax, mmax, 1)?;
    let plan = ShtPlan::new(grid, harmonics)?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1e3;

    let first_started = Instant::now();
    run_once(
        operation,
        &plan,
        &map_first,
        &map_second,
        &e,
        &b,
        &mut out_first,
        &mut out_second,
        &mut out_e,
        &mut out_b,
        &mut workspace,
    )?;
    let first_ms = first_started.elapsed().as_secs_f64() * 1e3;
    let first_outputs = (
        out_first.clone(),
        out_second.clone(),
        out_e.clone(),
        out_b.clone(),
    );

    let warm_started = Instant::now();
    run_once(
        operation,
        &plan,
        &map_first,
        &map_second,
        &e,
        &b,
        &mut out_first,
        &mut out_second,
        &mut out_e,
        &mut out_b,
        &mut workspace,
    )?;
    let warmup_ms = warm_started.elapsed().as_secs_f64() * 1e3;
    let stable = if operation == "analysis" {
        same_complex(&first_outputs.2, &out_e) && same_complex(&first_outputs.3, &out_b)
    } else {
        same_real(&first_outputs.0, &out_first) && same_real(&first_outputs.1, &out_second)
    };

    // Poison the destinations the operation writes. Forbidden l<spin output
    // remains its defined zero; unrelated outputs are not part of this call.
    if operation == "analysis" {
        out_e[1..].fill(Complex64::new(f64::NAN, f64::NAN));
        out_b[1..].fill(Complex64::new(f64::NAN, f64::NAN));
    } else {
        out_first.fill(f64::NAN);
        out_second.fill(f64::NAN);
    }
    run_once(
        operation,
        &plan,
        &map_first,
        &map_second,
        &e,
        &b,
        &mut out_first,
        &mut out_second,
        &mut out_e,
        &mut out_b,
        &mut workspace,
    )?;
    let reset_consistent = if operation == "analysis" {
        same_complex(&first_outputs.2, &out_e) && same_complex(&first_outputs.3, &out_b)
    } else {
        same_real(&first_outputs.0, &out_first) && same_real(&first_outputs.1, &out_second)
    };

    let mut repetitions = if warmup_ms >= 100.0 {
        1
    } else {
        ((target_ms / warmup_ms.max(0.000_001)).ceil() as usize).clamp(1, 2_000_000)
    };
    if warmup_ms < 100.0 {
        for _ in 0..3 {
            let start = Instant::now();
            for _ in 0..repetitions {
                run_once(
                    operation,
                    &plan,
                    &map_first,
                    &map_second,
                    &e,
                    &b,
                    &mut out_first,
                    &mut out_second,
                    &mut out_e,
                    &mut out_b,
                    &mut workspace,
                )?;
                black_box((&out_first, &out_second, &out_e, &out_b));
            }
            let elapsed = start.elapsed().as_secs_f64() * 1e3;
            if (20.0..=100.0).contains(&elapsed) {
                break;
            }
            repetitions = ((repetitions as f64 * target_ms / elapsed.max(0.000_001)).ceil()
                as usize)
                .clamp(1, 2_000_000);
        }
    }
    let batch_count = if warmup_ms >= 100.0 { 5 } else { 10 };
    let mut batch_ms = Vec::with_capacity(batch_count);
    let mut batch_total_ms = Vec::with_capacity(batch_count);
    let mut output_checksum = 0.0;
    for _ in 0..batch_count {
        let start = Instant::now();
        for _ in 0..repetitions {
            run_once(
                operation,
                &plan,
                &map_first,
                &map_second,
                &e,
                &b,
                &mut out_first,
                &mut out_second,
                &mut out_e,
                &mut out_b,
                &mut workspace,
            )?;
            black_box((&out_first, &out_second, &out_e, &out_b));
        }
        let elapsed = start.elapsed().as_secs_f64() * 1e3;
        batch_total_ms.push(elapsed);
        batch_ms.push(elapsed / repetitions as f64);
        output_checksum += if operation == "analysis" {
            checksum_complex(&out_e) + checksum_complex(&out_b)
        } else {
            checksum(&out_first) + checksum(&out_second)
        };
    }

    if operation == "analysis" {
        write_complex(&args[16], &out_e)?;
        write_complex(&args[17], &out_b)?;
    } else {
        write_f64(&args[14], &out_first)?;
        write_f64(&args[15], &out_second)?;
    }
    println!(
        "{{\"backend\":\"rustsht\",\"operation\":\"{operation}\",\"grid\":\"{}\",\"threads\":{threads},\"lmax\":{lmax},\"mmax\":{mmax},\"first_ms\":{first_ms},\"warmup_ms\":{warmup_ms},\"setup_ms\":{setup_ms},\"fixture_load_ms\":{read_ms},\"pool_ms\":{pool_ms},\"repetitions_per_batch\":{repetitions},\"batch_ms\":{batch_ms:?},\"batch_total_ms\":{batch_total_ms:?},\"output_checksum\":{output_checksum},\"repeat_consistent\":{stable},\"reset_consistent\":{reset_consistent}}}",
        args[3]
    );
    if !stable || !reset_consistent {
        return Err("repeat/reset output mismatch".into());
    }
    Ok(())
}
