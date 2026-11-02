// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::error::Error;
use std::fs;
use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn read_f32_as_f64(path: &str, expected: usize) -> Result<Vec<f64>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() != expected * 4 {
        return Err(format!("{path} has {} bytes; expected {}", bytes.len(), expected * 4).into());
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f64::from(f32::from_le_bytes(*chunk)))
        .collect())
}

fn same_bits(left: &[Complex64], right: &[Complex64]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| {
            a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits()
        })
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err("usage: m6_b3_rdpru_probe MAP0_F32 MAP1_F32 THREADS".into());
    }
    const LMAX: usize = 720;
    const MMAX: usize = 720;
    const NTHETA: usize = 721;
    const NPHI: usize = 1458;
    let threads: usize = args[3].parse()?;
    let map_count = NTHETA * NPHI;
    let map0 = read_f32_as_f64(&args[1], map_count)?;
    let map1 = read_f32_as_f64(&args[2], map_count)?;
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global()?;
    let plan = ShtPlan::<f64>::new(
        GridSpec::new(GridKind::GaussLegendre, NTHETA, NPHI, 0.23)?,
        HarmonicSpec::new(LMAX, MMAX, 1)?,
    )?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let count = plan.harmonics().nalm();
    let mut e = vec![Complex64::new(f64::NAN, f64::NAN); count];
    let mut b = vec![Complex64::new(f64::NAN, f64::NAN); count];
    let mut expected_e = Vec::new();
    let mut expected_b = Vec::new();
    let mut measured_elapsed = std::time::Duration::ZERO;
    for iteration in 0..21 {
        let started = std::time::Instant::now();
        plan.analysis_spin_into(&map0, &map1, &mut e, &mut b, &mut workspace)?;
        if iteration > 0 {
            measured_elapsed += started.elapsed();
        }
        if iteration == 0 {
            expected_e.clone_from(&e);
            expected_b.clone_from(&b);
        } else {
            assert!(same_bits(&e, &expected_e) && same_bits(&b, &expected_b));
        }
    }
    eprintln!(
        "RDPRU_DONE B3 f64 GL T720 M720 q={threads} finite={} calls=21",
        e.iter().chain(&b).all(|x| x.re.is_finite() && x.im.is_finite())
    );
    eprintln!(
        "M6_WALL_B3 f64 GL T720 M720 q={threads} mean_ms={:.6} measured_calls=20",
        measured_elapsed.as_secs_f64() * 1000.0 / 20.0
    );
    Ok(())
}
