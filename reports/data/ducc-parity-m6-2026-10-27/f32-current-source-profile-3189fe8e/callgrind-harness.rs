use std::error::Error;
use std::fs;
use std::io::Write;
use rustsht::{Complex32, GridKind, GridSpec, HarmonicSpec, ShtPlan, ShtWorkspace};

#[link(name = "m6_callgrind", kind = "static")]
unsafe extern "C" {
    fn m6_callgrind_zero();
    fn m6_callgrind_start();
    fn m6_callgrind_stop();
}

fn read_complex(path: &str, count: usize) -> Result<Vec<Complex32>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() != count * 8 {
        return Err(format!("{path} has {} bytes; expected {}", bytes.len(), count * 8).into());
    }
    let values = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect::<Vec<_>>();
    Ok(values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Complex32::new(pair[0], pair[1]))
        .collect())
}

fn same_bits(left: &[f32], right: &[f32]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(a, b)| a.to_bits() == b.to_bits())
}

fn synthesize(
    plan: &ShtPlan<f32>,
    alm: &[Complex32],
    output: &mut [f32],
    workspace: &mut ShtWorkspace<f32>,
) -> Result<(), rustsht::ShtError> {
    plan.synthesis_into(alm, output, workspace)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("usage: m6_f32_scalar_synthesis_callgrind ALM_F32 OUTPUT_F32".into());
    }
    const LMAX: usize = 720;
    const MMAX: usize = 720;
    const NTHETA: usize = 721;
    const NPHI: usize = 1458;
    const THREADS: usize = 8;
    let nalm = (MMAX + 1) * (LMAX + 1) - MMAX * (MMAX + 1) / 2;
    let alm = read_complex(&args[1], nalm)?;
    rayon::ThreadPoolBuilder::new()
        .num_threads(THREADS)
        .build_global()?;
    if rayon::current_num_threads() != THREADS {
        return Err(format!("global Rayon thread count is {}", rayon::current_num_threads()).into());
    }
    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, NTHETA, NPHI, 0.23)?,
        HarmonicSpec::new(LMAX, MMAX, 0)?,
    )?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, THREADS)?;
    let mut output = vec![f32::NAN; NTHETA * NPHI];

    synthesize(&plan, &alm, &mut output, &mut workspace)?;
    let expected = output.clone();
    for _ in 0..2 {
        synthesize(&plan, &alm, &mut output, &mut workspace)?;
        assert!(same_bits(&expected, &output), "repeat output changed");
    }
    output.fill(f32::NAN);
    synthesize(&plan, &alm, &mut output, &mut workspace)?;
    assert!(same_bits(&expected, &output), "poisoned reset output changed");

    unsafe {
        m6_callgrind_zero();
        m6_callgrind_start();
    }
    synthesize(&plan, &alm, &mut output, &mut workspace)?;
    unsafe { m6_callgrind_stop(); }
    assert!(same_bits(&expected, &output), "profile output changed");
    assert!(output.iter().all(|v| v.is_finite()), "nonfinite map output");

    let mut file = fs::File::create(&args[2])?;
    for value in &output {
        file.write_all(&value.to_le_bytes())?;
    }
    let checksum = output.iter().step_by(16_384).map(|&v| v as f64).sum::<f64>();
    eprintln!("PROFILE_DONE grid=GL lmax={LMAX} mmax={MMAX} ntheta={NTHETA} nphi={NPHI} q={THREADS} checksum={checksum:.9e}");
    Ok(())
}
