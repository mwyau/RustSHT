use rustsht::{Complex32, GridKind, GridSpec, HarmonicSpec, ShtPlan};
use std::error::Error;
use std::fs;
use std::io::Write;

#[link(name = "m6_callgrind", kind = "static")]
unsafe extern "C" {
    fn m6_callgrind_zero();
    fn m6_callgrind_start();
    fn m6_callgrind_stop();
    fn m6_callgrind_toggle();
    fn m6_b1_set_callgrind_region(region: usize);
    fn m6_b1_reset_factor_counts();
    fn m6_b1_factor_uniform_vectors() -> usize;
    fn m6_b1_factor_mixed_vectors() -> usize;
    fn m6_b1_factor_adjacent_reuses() -> usize;
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err("usage: m6_b1_callgrind_f32 MAP_F32 OUTPUT_F32 THREADS REGION".into());
    }
    const LMAX: usize = 720;
    const MMAX: usize = 720;
    const NTHETA: usize = 721;
    const NPHI: usize = 1458;
    let threads: usize = args[3].parse()?;
    let region: usize = args[4].parse()?;
    let bytes = fs::read(&args[1])?;
    if bytes.len() != NTHETA * NPHI * 4 {
        return Err(format!(
            "map bytes {} != expected {}",
            bytes.len(),
            NTHETA * NPHI * 4
        )
        .into());
    }
    let map = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect::<Vec<_>>();
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()?;
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, NTHETA, NPHI, 0.23)?,
        HarmonicSpec::scalar(LMAX, MMAX)?,
    )?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let mut alm = vec![Complex32::new(f32::NAN, f32::NAN); plan.harmonics().nalm()];
    plan.analysis_into(&map, &mut alm, &mut workspace)?;
    let expected = alm.clone();
    plan.analysis_into(&map, &mut alm, &mut workspace)?;
    assert!(
        alm.iter()
            .zip(&expected)
            .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
    );
    alm.fill(Complex32::new(f32::NAN, f32::NAN));
    plan.analysis_into(&map, &mut alm, &mut workspace)?;
    assert!(
        alm.iter()
            .zip(&expected)
            .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
    );
    unsafe {
        m6_b1_set_callgrind_region(region);
        m6_callgrind_zero();
        m6_callgrind_start();
        m6_callgrind_toggle();
    }
    let profile_whole_call = region == 99;
    if profile_whole_call {
        unsafe { m6_callgrind_toggle() };
    }
    unsafe { m6_b1_reset_factor_counts() };
    plan.analysis_into(&map, &mut alm, &mut workspace)?;
    if profile_whole_call {
        unsafe { m6_callgrind_toggle() };
    }
    eprintln!(
        "FACTOR_COUNTS uniform={} mixed={} adjacent_reuses={}",
        unsafe { m6_b1_factor_uniform_vectors() },
        unsafe { m6_b1_factor_mixed_vectors() },
        unsafe { m6_b1_factor_adjacent_reuses() },
    );
    unsafe {
        m6_callgrind_stop();
    }
    assert!(
        alm.iter()
            .zip(&expected)
            .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
    );
    let mut output = fs::File::create(&args[2])?;
    for coefficient in &alm {
        output.write_all(&coefficient.re.to_le_bytes())?;
        output.write_all(&coefficient.im.to_le_bytes())?;
    }
    eprintln!(
        "PROFILE_DONE B1_F32 GL T720 M720 q={threads} finite={} nalm={}",
        alm.iter().all(|x| x.re.is_finite() && x.im.is_finite()),
        alm.len()
    );
    Ok(())
}
