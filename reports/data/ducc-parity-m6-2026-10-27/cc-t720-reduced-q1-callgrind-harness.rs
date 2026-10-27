use std::error::Error;
use std::fs::File;
use std::io::Write;
use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};

unsafe extern "C" {
    fn m6_callgrind_zero();
    fn m6_callgrind_start();
    fn m6_callgrind_stop();
}

fn next_unit(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    ((value >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64)) * 2.0 - 1.0
}

fn same_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

fn write_f64(path: &str, values: &[f64]) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(path)?;
    for value in values {
        file.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: m6_spin1_synthesis_callgrind OUT0 OUT1".into());
    }
    let (lmax, mmax, ntheta, nphi, threads) = (720, 180, 722, 378, 1);
    let plan = ShtPlan::new(
        GridSpec::new(GridKind::ClenshawCurtis, ntheta, nphi, 0.23)?,
        HarmonicSpec::new(lmax, mmax, 1)?,
    )?;
    let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut b_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut state = 0x7a31_9d42_8b6c_05e1_u64;
    for order in 0..=mmax {
        for degree in 1.max(order)..=lmax {
            let index = plan.harmonics().index(degree, order).unwrap();
            e_alm[index].re = next_unit(&mut state);
            e_alm[index].im = if order == 0 { 0.0 } else { next_unit(&mut state) };
            b_alm[index].re = next_unit(&mut state);
            b_alm[index].im = if order == 0 { 0.0 } else { next_unit(&mut state) };
        }
    }
    let mut first = vec![0.0; ((ntheta) * (nphi))];
    let mut second = vec![0.0; ((ntheta) * (nphi))];
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build()?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let run_once = |first: &mut [f64], second: &mut [f64], workspace: &mut _| {
        pool.install(|| plan.synthesis_spin_into(&e_alm, &b_alm, first, second, workspace))
    };
    run_once(&mut first, &mut second, &mut workspace)?;
    let expected_first = first.clone();
    let expected_second = second.clone();
    for _ in 0..2 {
        run_once(&mut first, &mut second, &mut workspace)?;
        assert!(same_bits(&expected_first, &first) && same_bits(&expected_second, &second));
    }
    unsafe {
        m6_callgrind_zero();
        m6_callgrind_start();
    }
    run_once(&mut first, &mut second, &mut workspace)?;
    unsafe { m6_callgrind_stop(); }
    assert!(same_bits(&expected_first, &first) && same_bits(&expected_second, &second));
    assert!(first.iter().chain(&second).all(|value| value.is_finite()));
    write_f64(&args[1], &first)?;
    write_f64(&args[2], &second)?;
    eprintln!("PROFILE_DONE backend=rustsht grid=CC lmax={lmax} mmax={mmax} ntheta={ntheta} nphi={nphi} q={threads}");
    Ok(())
}
