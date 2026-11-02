use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};
use std::error::Error;
use std::fs;
use std::io::Write;

#[link(name = "m6_callgrind", kind = "static")]
unsafe extern "C" {
    fn m6_callgrind_zero();
    fn m6_callgrind_start();
    fn m6_callgrind_stop();
}

fn read_f64(path: &str, count: usize) -> Result<Vec<f64>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() != count * 8 {
        return Err(format!("{path}: {} bytes, expected {}", bytes.len(), count * 8).into());
    }
    Ok(bytes.as_chunks::<8>().0.iter().map(|x| f64::from_le_bytes(*x)).collect())
}

fn same_bits(a: &[Complex64], b: &[Complex64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.re.to_bits() == y.re.to_bits() && x.im.to_bits() == y.im.to_bits())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 7 { return Err("usage: m6_b3_analysis_callgrind_f64 MAP0 MAP1 OUT_E OUT_B THREADS full|reduced".into()); }
    const LMAX: usize = 720;
    const NTHETA: usize = 721;
    let reduced = args[6] == "reduced";
    let (mmax, nphi) = if reduced { (180, 378) } else { (720, 1458) };
    let threads: usize = args[5].parse()?;
    let map_count = NTHETA * nphi;
    let map0 = read_f64(&args[1], map_count)?;
    let map1 = read_f64(&args[2], map_count)?;
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global()?;
    let plan = ShtPlan::<f64>::new(
        GridSpec::new(GridKind::GaussLegendre, NTHETA, nphi, 0.23)?,
        HarmonicSpec::new(LMAX, mmax, 1)?,
    )?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let count = plan.harmonics().nalm();
    let mut e = vec![Complex64::new(f64::NAN, f64::NAN); count];
    let mut b = vec![Complex64::new(f64::NAN, f64::NAN); count];
    plan.analysis_spin_into(&map0, &map1, &mut e, &mut b, &mut workspace)?;
    let expected_e = e.clone();
    let expected_b = b.clone();
    plan.analysis_spin_into(&map0, &map1, &mut e, &mut b, &mut workspace)?;
    assert!(same_bits(&e, &expected_e) && same_bits(&b, &expected_b));
    e.fill(Complex64::new(f64::NAN, f64::NAN));
    b.fill(Complex64::new(f64::NAN, f64::NAN));
    plan.analysis_spin_into(&map0, &map1, &mut e, &mut b, &mut workspace)?;
    assert!(same_bits(&e, &expected_e) && same_bits(&b, &expected_b));
    unsafe { m6_callgrind_zero(); m6_callgrind_start(); }
    plan.analysis_spin_into(&map0, &map1, &mut e, &mut b, &mut workspace)?;
    unsafe { m6_callgrind_stop(); }
    assert!(same_bits(&e, &expected_e) && same_bits(&b, &expected_b));
    for (path, values) in [(&args[3], &e), (&args[4], &b)] {
        let mut out = fs::File::create(path)?;
        for x in values { out.write_all(&x.re.to_le_bytes())?; out.write_all(&x.im.to_le_bytes())?; }
    }
    eprintln!("PROFILE_DONE B3_F64 mmax={mmax} q={threads} finite={} nalm={count}", e.iter().chain(&b).all(|x| x.re.is_finite() && x.im.is_finite()));
    Ok(())
}
