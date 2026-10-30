use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use num_complex::{Complex32, Complex64};
use rustsht::{GridKind, GridSpec, HarmonicSpec, ShtPlan};

const LMAX: usize = 720;
const MMAX: usize = 720;
const NPHI: usize = 1458;

fn grid_kind() -> Result<GridKind, Box<dyn Error>> {
    Ok(
        match std::env::var("PHASEC_GRID_KIND").as_deref().unwrap_or("gl") {
            "gl" => GridKind::GaussLegendre,
            "cc" => GridKind::ClenshawCurtis,
            "f1" => GridKind::Fejer1,
            other => return Err(format!("unsupported PHASEC_GRID_KIND={other}").into()),
        },
    )
}

fn sample(index: usize, seed: u64) -> f32 {
    let mut x = (index as u64).wrapping_add(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    let signed = (x % 65_535) as i32 - 32_767;
    signed as f32 / 32_768.0
}

fn coefficient(l: usize, m: usize, lane: usize) -> f32 {
    let x = (l as u64)
        .wrapping_mul(0x9e37_79b9)
        .wrapping_add((m as u64).wrapping_mul(0x85eb_ca6b))
        .wrapping_add((lane as u64).wrapping_mul(0xc2b2_ae35));
    let signed = (x % 4_095) as i32 - 2_047;
    signed as f32 / 4_096.0
}

fn write_f32(path: &Path, values: &[f32]) -> Result<(), Box<dyn Error>> {
    let mut out = BufWriter::new(File::create(path)?);
    for value in values {
        out.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn write_f64(path: &Path, values: &[f64]) -> Result<(), Box<dyn Error>> {
    let mut out = BufWriter::new(File::create(path)?);
    for value in values {
        out.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn write_complex32(path: &Path, values: &[Complex32]) -> Result<(), Box<dyn Error>> {
    let mut out = BufWriter::new(File::create(path)?);
    for value in values {
        out.write_all(&value.re.to_le_bytes())?;
        out.write_all(&value.im.to_le_bytes())?;
    }
    Ok(())
}

fn write_complex64(path: &Path, values: &[Complex64]) -> Result<(), Box<dyn Error>> {
    let mut out = BufWriter::new(File::create(path)?);
    for value in values {
        out.write_all(&value.re.to_le_bytes())?;
        out.write_all(&value.im.to_le_bytes())?;
    }
    Ok(())
}

fn widen(values: &[Complex32]) -> Vec<Complex64> {
    values
        .iter()
        .map(|value| Complex64::new(value.re as f64, value.im as f64))
        .collect()
}

fn run(directory: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(directory)?;
    let kind = grid_kind()?;
    let ntheta = if kind == GridKind::ClenshawCurtis {
        LMAX + 2
    } else {
        LMAX + 1
    };
    let grid = GridSpec::new(kind, ntheta, NPHI, 0.0)?;
    let scalar_harmonics = HarmonicSpec::new(LMAX, MMAX, 0)?;
    let spin_harmonics = HarmonicSpec::new(LMAX, MMAX, 1)?;
    let scalar32 = ShtPlan::<f32>::new(grid.clone(), scalar_harmonics.clone())?;
    let scalar64 = ShtPlan::<f64>::new(grid.clone(), scalar_harmonics.clone())?;
    let spin32 = ShtPlan::<f32>::new(grid.clone(), spin_harmonics.clone())?;
    let spin64 = ShtPlan::<f64>::new(grid.clone(), spin_harmonics.clone())?;

    let mut geometry = BufWriter::new(File::create(directory.join("geometry.csv"))?);
    writeln!(geometry, "ring,theta_f64_bits,weight_f64_bits")?;
    for (ring, (&theta, &weight)) in grid.theta().iter().zip(grid.ring_weights()).enumerate() {
        writeln!(
            geometry,
            "{ring},{:016x},{:016x}",
            theta.to_bits(),
            weight.to_bits()
        )?;
    }

    let sample_count = ntheta * NPHI;
    let map_scale_bits = std::env::var("PHASEC_MAP_SCALE_BITS")
        .map(|value| u32::from_str_radix(value.trim_start_matches("0x"), 16).unwrap())
        .unwrap_or(1.0_f32.to_bits());
    let map_scale = f32::from_bits(map_scale_bits);
    let scalar_map: Vec<f32> = (0..sample_count)
        .map(|i| sample(i, 11) * map_scale)
        .collect();
    let q_map: Vec<f32> = (0..sample_count)
        .map(|i| sample(i, 23) * map_scale)
        .collect();
    let u_map: Vec<f32> = (0..sample_count)
        .map(|i| sample(i, 47) * map_scale)
        .collect();
    write_f32(&directory.join("scalar-map.f32"), &scalar_map)?;
    write_f32(&directory.join("q-map.f32"), &q_map)?;
    write_f32(&directory.join("u-map.f32"), &u_map)?;

    let mut scalar_alm32 = vec![Complex32::new(0.0, 0.0); scalar_harmonics.nalm()];
    let mut scalar_workspace32 = scalar32.workspace()?;
    scalar32.analysis_into(&scalar_map, &mut scalar_alm32, &mut scalar_workspace32)?;
    write_complex32(&directory.join("scalar-analysis.f32"), &scalar_alm32)?;

    let scalar_map64: Vec<f64> = scalar_map.iter().map(|&value| value as f64).collect();
    let mut scalar_alm64 = vec![Complex64::new(0.0, 0.0); scalar_harmonics.nalm()];
    let mut scalar_workspace64 = scalar64.workspace()?;
    scalar64.analysis_into(&scalar_map64, &mut scalar_alm64, &mut scalar_workspace64)?;
    write_complex64(&directory.join("scalar-analysis.f64"), &scalar_alm64)?;

    let mut e_alm32 = vec![Complex32::new(0.0, 0.0); spin_harmonics.nalm()];
    let mut b_alm32 = vec![Complex32::new(0.0, 0.0); spin_harmonics.nalm()];
    let mut spin_workspace32 = spin32.workspace()?;
    spin32.analysis_spin_into(
        &q_map,
        &u_map,
        &mut e_alm32,
        &mut b_alm32,
        &mut spin_workspace32,
    )?;
    write_complex32(&directory.join("spin-analysis-e.f32"), &e_alm32)?;
    write_complex32(&directory.join("spin-analysis-b.f32"), &b_alm32)?;

    let q_map64: Vec<f64> = q_map.iter().map(|&value| value as f64).collect();
    let u_map64: Vec<f64> = u_map.iter().map(|&value| value as f64).collect();
    let mut e_alm64 = vec![Complex64::new(0.0, 0.0); spin_harmonics.nalm()];
    let mut b_alm64 = vec![Complex64::new(0.0, 0.0); spin_harmonics.nalm()];
    let mut spin_workspace64 = spin64.workspace()?;
    spin64.analysis_spin_into(
        &q_map64,
        &u_map64,
        &mut e_alm64,
        &mut b_alm64,
        &mut spin_workspace64,
    )?;
    write_complex64(&directory.join("spin-analysis-e.f64"), &e_alm64)?;
    write_complex64(&directory.join("spin-analysis-b.f64"), &b_alm64)?;

    let single_mode = std::env::var("PHASEC_SINGLE_MODE").ok();
    let single_mode = single_mode.as_deref().map(|value| {
        let fields = value.split(',').collect::<Vec<_>>();
        assert_eq!(
            fields.len(),
            3,
            "use PHASEC_SINGLE_MODE=l,m,E.re|E.im|B.re|B.im"
        );
        (
            fields[0].parse::<usize>().unwrap(),
            fields[1].parse::<usize>().unwrap(),
            fields[2],
        )
    });
    let single_scale = std::env::var("PHASEC_SINGLE_SCALE_BITS")
        .map(|value| {
            let bits = u32::from_str_radix(value.trim_start_matches("0x"), 16).unwrap();
            f32::from_bits(bits)
        })
        .unwrap_or(1.0);
    let mut e_input32 = vec![Complex32::new(0.0, 0.0); spin_harmonics.nalm()];
    let mut b_input32 = vec![Complex32::new(0.0, 0.0); spin_harmonics.nalm()];
    for m in 0..=MMAX {
        for l in 600.max(m)..=LMAX {
            let index = spin_harmonics.index(l, m).unwrap();
            if let Some((single_l, single_m, component)) = single_mode {
                if l == single_l && m == single_m {
                    match component {
                        "E.re" => e_input32[index].re = single_scale,
                        "E.im" => e_input32[index].im = single_scale,
                        "B.re" => b_input32[index].re = single_scale,
                        "B.im" => b_input32[index].im = single_scale,
                        _ => panic!("invalid single-mode component {component}"),
                    }
                }
            } else {
                e_input32[index] = Complex32::new(
                    coefficient(l, m, 1),
                    if m == 0 { 0.0 } else { coefficient(l, m, 2) },
                );
                b_input32[index] = Complex32::new(
                    coefficient(l, m, 3),
                    if m == 0 { 0.0 } else { coefficient(l, m, 4) },
                );
            }
        }
    }
    write_complex32(&directory.join("spin-synthesis-e-input.f32"), &e_input32)?;
    write_complex32(&directory.join("spin-synthesis-b-input.f32"), &b_input32)?;
    let mut q_out32 = vec![0.0_f32; sample_count];
    let mut u_out32 = vec![0.0_f32; sample_count];
    let mut synthesis_workspace32 = spin32.workspace()?;
    spin32.synthesis_spin_into(
        &e_input32,
        &b_input32,
        &mut q_out32,
        &mut u_out32,
        &mut synthesis_workspace32,
    )?;
    write_f32(&directory.join("spin-synthesis-q.f32"), &q_out32)?;
    write_f32(&directory.join("spin-synthesis-u.f32"), &u_out32)?;

    let e_input64 = widen(&e_input32);
    let b_input64 = widen(&b_input32);
    write_complex64(&directory.join("spin-synthesis-e-input.f64"), &e_input64)?;
    write_complex64(&directory.join("spin-synthesis-b-input.f64"), &b_input64)?;
    let mut q_out64 = vec![0.0_f64; sample_count];
    let mut u_out64 = vec![0.0_f64; sample_count];
    let mut synthesis_workspace64 = spin64.workspace()?;
    spin64.synthesis_spin_into(
        &e_input64,
        &b_input64,
        &mut q_out64,
        &mut u_out64,
        &mut synthesis_workspace64,
    )?;
    write_f64(&directory.join("spin-synthesis-q.f64"), &q_out64)?;
    write_f64(&directory.join("spin-synthesis-u.f64"), &u_out64)?;

    let mut metadata = BufWriter::new(File::create(directory.join("metadata.txt"))?);
    writeln!(metadata, "lmax={LMAX}")?;
    writeln!(metadata, "mmax={MMAX}")?;
    writeln!(metadata, "ntheta={ntheta}")?;
    writeln!(metadata, "nphi={NPHI}")?;
    writeln!(metadata, "phi0=0")?;
    writeln!(metadata, "grid_kind={kind:?}")?;
    writeln!(metadata, "single_mode={single_mode:?}")?;
    writeln!(metadata, "single_scale_bits={:08x}", single_scale.to_bits())?;
    writeln!(metadata, "map_scale_bits={map_scale_bits:08x}")?;
    writeln!(metadata, "input_map_seeds=scalar:11,q:23,u:47")?;
    writeln!(
        metadata,
        "rayon_threads={}",
        std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "default".into())
    )?;
    writeln!(metadata, "coefficient_band=l600..720,m0..720")?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or("missing output directory")?;
    run(Path::new(&path))
}
