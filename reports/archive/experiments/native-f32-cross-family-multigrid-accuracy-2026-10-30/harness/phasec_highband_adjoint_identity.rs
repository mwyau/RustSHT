use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use num_complex::{Complex32, Complex64};
use rustsht::{GridKind, GridSpec, HarmonicSpec, ShtPlan};

const LMAX: usize = 720;
const NPHI: usize = 1458;

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

fn widen_complex(values: &[Complex32]) -> Vec<Complex64> {
    values
        .iter()
        .map(|value| Complex64::new(f64::from(value.re), f64::from(value.im)))
        .collect()
}

fn widen_map(values: &[f32]) -> Vec<f64> {
    values.iter().map(|&value| f64::from(value)).collect()
}

fn harmonic_inner32(spec: &HarmonicSpec, left: &[Complex32], right: &[Complex32]) -> (f64, f64) {
    let mut sum = 0.0;
    let mut abs_sum = 0.0;
    for m in 0..=spec.mmax() {
        let weight = if m == 0 { 1.0 } else { 2.0 };
        for l in m.max(spec.spin())..=spec.lmax() {
            let index = spec.index(l, m).unwrap();
            let a = left[index];
            let b = right[index];
            let term =
                weight * (f64::from(a.re) * f64::from(b.re) + f64::from(a.im) * f64::from(b.im));
            sum += term;
            abs_sum += term.abs();
        }
    }
    (sum, abs_sum)
}

fn harmonic_inner64(spec: &HarmonicSpec, left: &[Complex64], right: &[Complex64]) -> (f64, f64) {
    let mut sum = 0.0;
    let mut abs_sum = 0.0;
    for m in 0..=spec.mmax() {
        let weight = if m == 0 { 1.0 } else { 2.0 };
        for l in m.max(spec.spin())..=spec.lmax() {
            let index = spec.index(l, m).unwrap();
            let a = left[index];
            let b = right[index];
            let term = weight * (a.re * b.re + a.im * b.im);
            sum += term;
            abs_sum += term.abs();
        }
    }
    (sum, abs_sum)
}

fn map_inner32(left: &[f32], right: &[f32]) -> (f64, f64) {
    left.iter()
        .zip(right)
        .fold((0.0, 0.0), |(sum, abs_sum), (&a, &b)| {
            let term = f64::from(a) * f64::from(b);
            (sum + term, abs_sum + term.abs())
        })
}

fn map_inner64(left: &[f64], right: &[f64]) -> (f64, f64) {
    left.iter()
        .zip(right)
        .fold((0.0, 0.0), |(sum, abs_sum), (&a, &b)| {
            let term = a * b;
            (sum + term, abs_sum + term.abs())
        })
}

fn vector_map_inner32(a: &[f32], b: &[f32], c: &[f32], d: &[f32]) -> (f64, f64) {
    let (sum_a, abs_a) = map_inner32(a, c);
    let (sum_b, abs_b) = map_inner32(b, d);
    (sum_a + sum_b, abs_a + abs_b)
}

fn vector_map_inner64(a: &[f64], b: &[f64], c: &[f64], d: &[f64]) -> (f64, f64) {
    let (sum_a, abs_a) = map_inner64(a, c);
    let (sum_b, abs_b) = map_inner64(b, d);
    (sum_a + sum_b, abs_a + abs_b)
}

fn spin_harmonic_inner32(
    spec: &HarmonicSpec,
    left_e: &[Complex32],
    left_b: &[Complex32],
    right_e: &[Complex32],
    right_b: &[Complex32],
) -> (f64, f64) {
    let (sum_e, abs_e) = harmonic_inner32(spec, left_e, right_e);
    let (sum_b, abs_b) = harmonic_inner32(spec, left_b, right_b);
    (sum_e + sum_b, abs_e + abs_b)
}

fn spin_harmonic_inner64(
    spec: &HarmonicSpec,
    left_e: &[Complex64],
    left_b: &[Complex64],
    right_e: &[Complex64],
    right_b: &[Complex64],
) -> (f64, f64) {
    let (sum_e, abs_e) = harmonic_inner64(spec, left_e, right_e);
    let (sum_b, abs_b) = harmonic_inner64(spec, left_b, right_b);
    (sum_e + sum_b, abs_e + abs_b)
}

fn record(
    out: &mut impl Write,
    operation: &str,
    precision: &str,
    lhs: (f64, f64),
    rhs: (f64, f64),
) -> Result<(), Box<dyn Error>> {
    let difference = (lhs.0 - rhs.0).abs();
    let scaled = difference / (lhs.1 + rhs.1).max(f64::MIN_POSITIVE);
    writeln!(
        out,
        "{operation},{precision},{:.17e},{:.17e},{difference:.17e},{scaled:.17e}",
        lhs.0, rhs.0
    )?;
    Ok(())
}

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

fn run(directory: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(directory)?;
    let kind = grid_kind()?;
    let ntheta = LMAX + 1 + usize::from(kind == GridKind::ClenshawCurtis);
    let grid = GridSpec::new(kind, ntheta, NPHI, 0.0)?;
    let scalar_spec = HarmonicSpec::new(LMAX, LMAX, 0)?;
    let spin_spec = HarmonicSpec::new(LMAX, LMAX, 1)?;
    let scalar32 = ShtPlan::<f32>::new(grid.clone(), scalar_spec.clone())?;
    let scalar64 = ShtPlan::<f64>::new(grid.clone(), scalar_spec.clone())?;
    let spin32 = ShtPlan::<f32>::new(grid.clone(), spin_spec.clone())?;
    let spin64 = ShtPlan::<f64>::new(grid, spin_spec.clone())?;
    let sample_count = ntheta * NPHI;

    let scalar_map32: Vec<f32> = (0..sample_count).map(|i| sample(i, 11)).collect();
    let first_map32: Vec<f32> = (0..sample_count).map(|i| sample(i, 23)).collect();
    let second_map32: Vec<f32> = (0..sample_count).map(|i| sample(i, 47)).collect();
    let synthesis_probe_first32: Vec<f32> = (0..sample_count).map(|i| sample(i, 61)).collect();
    let synthesis_probe_second32: Vec<f32> = (0..sample_count).map(|i| sample(i, 73)).collect();
    let scalar_map64 = widen_map(&scalar_map32);
    let first_map64 = widen_map(&first_map32);
    let second_map64 = widen_map(&second_map32);
    let synthesis_probe_first64 = widen_map(&synthesis_probe_first32);
    let synthesis_probe_second64 = widen_map(&synthesis_probe_second32);

    let mut scalar_probe32 = vec![Complex32::new(0.0, 0.0); scalar_spec.nalm()];
    let mut e_probe32 = vec![Complex32::new(0.0, 0.0); spin_spec.nalm()];
    let mut b_probe32 = vec![Complex32::new(0.0, 0.0); spin_spec.nalm()];
    for m in 0..=LMAX {
        for l in 600.max(m)..=LMAX {
            let index0 = scalar_spec.index(l, m).unwrap();
            let index1 = spin_spec.index(l, m).unwrap();
            scalar_probe32[index0] = Complex32::new(
                coefficient(l, m, 1),
                if m == 0 { 0.0 } else { coefficient(l, m, 2) },
            );
            e_probe32[index1] = Complex32::new(
                coefficient(l, m, 3),
                if m == 0 { 0.0 } else { coefficient(l, m, 4) },
            );
            b_probe32[index1] = Complex32::new(
                coefficient(l, m, 5),
                if m == 0 { 0.0 } else { coefficient(l, m, 6) },
            );
        }
    }
    let scalar_probe64 = widen_complex(&scalar_probe32);
    let e_probe64 = widen_complex(&e_probe32);
    let b_probe64 = widen_complex(&b_probe32);

    let mut scalar_alm32 = vec![Complex32::new(0.0, 0.0); scalar_spec.nalm()];
    let mut scalar_alm64 = vec![Complex64::new(0.0, 0.0); scalar_spec.nalm()];
    let mut scalar_workspace32 = scalar32.workspace()?;
    let mut scalar_workspace64 = scalar64.workspace()?;
    scalar32.analysis_into(&scalar_map32, &mut scalar_alm32, &mut scalar_workspace32)?;
    scalar64.analysis_into(&scalar_map64, &mut scalar_alm64, &mut scalar_workspace64)?;
    let mut scalar_adjoint_map32 = vec![0.0_f32; sample_count];
    let mut scalar_adjoint_map64 = vec![0.0_f64; sample_count];
    scalar32.adjoint_analysis_into(
        &scalar_probe32,
        &mut scalar_adjoint_map32,
        &mut scalar_workspace32,
    )?;
    scalar64.adjoint_analysis_into(
        &scalar_probe64,
        &mut scalar_adjoint_map64,
        &mut scalar_workspace64,
    )?;

    let mut first_synthesis32 = vec![0.0_f32; sample_count];
    let mut second_synthesis32 = vec![0.0_f32; sample_count];
    let mut first_synthesis64 = vec![0.0_f64; sample_count];
    let mut second_synthesis64 = vec![0.0_f64; sample_count];
    let mut e_adjoint32 = vec![Complex32::new(0.0, 0.0); spin_spec.nalm()];
    let mut b_adjoint32 = vec![Complex32::new(0.0, 0.0); spin_spec.nalm()];
    let mut e_adjoint64 = vec![Complex64::new(0.0, 0.0); spin_spec.nalm()];
    let mut b_adjoint64 = vec![Complex64::new(0.0, 0.0); spin_spec.nalm()];
    let mut spin_workspace32 = spin32.workspace()?;
    let mut spin_workspace64 = spin64.workspace()?;
    spin32.synthesis_spin_into(
        &e_probe32,
        &b_probe32,
        &mut first_synthesis32,
        &mut second_synthesis32,
        &mut spin_workspace32,
    )?;
    spin64.synthesis_spin_into(
        &e_probe64,
        &b_probe64,
        &mut first_synthesis64,
        &mut second_synthesis64,
        &mut spin_workspace64,
    )?;
    spin32.adjoint_synthesis_spin_into(
        &synthesis_probe_first32,
        &synthesis_probe_second32,
        &mut e_adjoint32,
        &mut b_adjoint32,
        &mut spin_workspace32,
    )?;
    spin64.adjoint_synthesis_spin_into(
        &synthesis_probe_first64,
        &synthesis_probe_second64,
        &mut e_adjoint64,
        &mut b_adjoint64,
        &mut spin_workspace64,
    )?;

    let mut analyzed_e32 = vec![Complex32::new(0.0, 0.0); spin_spec.nalm()];
    let mut analyzed_b32 = vec![Complex32::new(0.0, 0.0); spin_spec.nalm()];
    let mut analyzed_e64 = vec![Complex64::new(0.0, 0.0); spin_spec.nalm()];
    let mut analyzed_b64 = vec![Complex64::new(0.0, 0.0); spin_spec.nalm()];
    spin32.analysis_spin_into(
        &first_map32,
        &second_map32,
        &mut analyzed_e32,
        &mut analyzed_b32,
        &mut spin_workspace32,
    )?;
    spin64.analysis_spin_into(
        &first_map64,
        &second_map64,
        &mut analyzed_e64,
        &mut analyzed_b64,
        &mut spin_workspace64,
    )?;
    let mut analysis_adjoint_first32 = vec![0.0_f32; sample_count];
    let mut analysis_adjoint_second32 = vec![0.0_f32; sample_count];
    let mut analysis_adjoint_first64 = vec![0.0_f64; sample_count];
    let mut analysis_adjoint_second64 = vec![0.0_f64; sample_count];
    spin32.adjoint_analysis_spin_into(
        &e_probe32,
        &b_probe32,
        &mut analysis_adjoint_first32,
        &mut analysis_adjoint_second32,
        &mut spin_workspace32,
    )?;
    spin64.adjoint_analysis_spin_into(
        &e_probe64,
        &b_probe64,
        &mut analysis_adjoint_first64,
        &mut analysis_adjoint_second64,
        &mut spin_workspace64,
    )?;

    let mut output = BufWriter::new(File::create(directory.join("adjoint-identities.csv"))?);
    writeln!(
        output,
        "operation,precision,lhs,rhs,absolute_difference,condition_scaled_residual"
    )?;
    record(
        &mut output,
        "scalar-analysis",
        "f32",
        harmonic_inner32(&scalar_spec, &scalar_alm32, &scalar_probe32),
        map_inner32(&scalar_map32, &scalar_adjoint_map32),
    )?;
    record(
        &mut output,
        "scalar-analysis",
        "f64",
        harmonic_inner64(&scalar_spec, &scalar_alm64, &scalar_probe64),
        map_inner64(&scalar_map64, &scalar_adjoint_map64),
    )?;
    record(
        &mut output,
        "spin1-synthesis",
        "f32",
        vector_map_inner32(
            &first_synthesis32,
            &second_synthesis32,
            &synthesis_probe_first32,
            &synthesis_probe_second32,
        ),
        spin_harmonic_inner32(
            &spin_spec,
            &e_probe32,
            &b_probe32,
            &e_adjoint32,
            &b_adjoint32,
        ),
    )?;
    record(
        &mut output,
        "spin1-synthesis",
        "f64",
        vector_map_inner64(
            &first_synthesis64,
            &second_synthesis64,
            &synthesis_probe_first64,
            &synthesis_probe_second64,
        ),
        spin_harmonic_inner64(
            &spin_spec,
            &e_probe64,
            &b_probe64,
            &e_adjoint64,
            &b_adjoint64,
        ),
    )?;
    record(
        &mut output,
        "spin1-analysis",
        "f32",
        spin_harmonic_inner32(
            &spin_spec,
            &analyzed_e32,
            &analyzed_b32,
            &e_probe32,
            &b_probe32,
        ),
        vector_map_inner32(
            &first_map32,
            &second_map32,
            &analysis_adjoint_first32,
            &analysis_adjoint_second32,
        ),
    )?;
    record(
        &mut output,
        "spin1-analysis",
        "f64",
        spin_harmonic_inner64(
            &spin_spec,
            &analyzed_e64,
            &analyzed_b64,
            &e_probe64,
            &b_probe64,
        ),
        vector_map_inner64(
            &first_map64,
            &second_map64,
            &analysis_adjoint_first64,
            &analysis_adjoint_second64,
        ),
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args().nth(1).ok_or("missing output directory")?;
    run(Path::new(&directory))
}
