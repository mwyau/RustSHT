use std::error::Error;
use std::fs;
use std::hint::black_box;
use rustsht::{Complex32, GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn read_f32(path: &str, count: usize) -> Result<Vec<f32>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() != count * 4 { return Err(format!("wrong f32 input length for {path}").into()); }
    Ok(bytes.as_chunks::<4>().0.iter().map(|x| f32::from_le_bytes(*x)).collect())
}

fn read_complex(path: &str, count: usize) -> Result<Vec<Complex32>, Box<dyn Error>> {
    let values = read_f32(path, count * 2)?;
    Ok(values.as_chunks::<2>().0.iter().map(|x| Complex32::new(x[0], x[1])).collect())
}

fn main() -> Result<(), Box<dyn Error>> {
    let a = std::env::args().collect::<Vec<_>>();
    if a.len() != 11 || a[1] != "--worker" {
        return Err("usage: --worker THREADS NTHETA NPHI LMAX MMAX PHI0 MAP0 MAP1 ALM0".into());
    }
    let threads: usize = a[2].parse()?;
    let ntheta: usize = a[3].parse()?;
    let nphi: usize = a[4].parse()?;
    let lmax: usize = a[5].parse()?;
    let mmax: usize = a[6].parse()?;
    let phi0: f64 = a[7].parse()?;
    let map_count = ntheta * nphi;
    let nalm = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) / 2;
    let map0 = read_f32(&a[8], map_count)?;
    let map1 = read_f32(&a[9], map_count)?;
    let alm = read_complex(&a[10], nalm)?;
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global()?;
    let grid = GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, phi0)?;
    let harmonics = HarmonicSpec::scalar(lmax, mmax)?;
    let plan = ShtPlan::<f32>::new(grid, harmonics)?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let mut map = vec![f32::NAN; map_count];
    plan.synthesis_into(&alm, &mut map, &mut workspace)?;
    black_box((&map, &map0, &map1));
    Ok(())
}
