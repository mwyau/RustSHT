// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
use rustsht::{Complex32, GridKind, GridSpec, HarmonicSpec, ShtPlan};
use std::{error::Error, fs, io::Write};
#[link(name = "m6_callgrind", kind = "static")]
unsafe extern "C" {
    fn m6_callgrind_zero();
    fn m6_callgrind_start();
    fn m6_callgrind_stop();
}
fn read_complex(path: &str, count: usize) -> Result<Vec<Complex32>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() != count * 8 {
        return Err(format!("bad input length: {}", bytes.len()).into());
    }
    let values = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|x| f32::from_le_bytes(*x))
        .collect::<Vec<_>>();
    Ok(values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|x| Complex32::new(x[0], x[1]))
        .collect())
}
fn same_bits(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 6 {
        return Err("usage: ALM OUTPUT MMAX NPHI THREADS".into());
    }
    const LMAX: usize = 720;
    const NTHETA: usize = 721;
    const PHI0: f64 = 0.23;
    let mmax: usize = args[3].parse()?;
    let nphi: usize = args[4].parse()?;
    let threads: usize = args[5].parse()?;
    let nalm = (mmax + 1) * (LMAX + 1) - mmax * (mmax + 1) / 2;
    let alm = read_complex(&args[1], nalm)?;
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()?;
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, NTHETA, nphi, PHI0)?,
        HarmonicSpec::new(LMAX, mmax, 0)?,
    )?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let mut output = vec![f32::NAN; NTHETA * nphi];
    plan.synthesis_into(&alm, &mut output, &mut workspace)?;
    let expected = output.clone();
    for _ in 0..2 {
        plan.synthesis_into(&alm, &mut output, &mut workspace)?;
        assert!(same_bits(&expected, &output));
    }
    output.fill(f32::NAN);
    plan.synthesis_into(&alm, &mut output, &mut workspace)?;
    assert!(same_bits(&expected, &output));
    unsafe {
        m6_callgrind_zero();
        m6_callgrind_start();
    }
    plan.synthesis_into(&alm, &mut output, &mut workspace)?;
    unsafe {
        m6_callgrind_stop();
    }
    assert!(same_bits(&expected, &output));
    assert!(output.iter().all(|x| x.is_finite()));
    let mut file = fs::File::create(&args[2])?;
    for value in &output {
        file.write_all(&value.to_le_bytes())?;
    }
    eprintln!("PROFILE_DONE GL T720 mmax={mmax} nphi={nphi} q={threads}");
    Ok(())
}
