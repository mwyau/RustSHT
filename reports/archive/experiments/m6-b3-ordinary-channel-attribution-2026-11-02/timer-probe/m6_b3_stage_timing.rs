// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::error::Error;
use std::time::Instant;

use rustsht::{GridKind, GridSpec, HarmonicSpec, ShtPlan};

fn sample(index: usize, stream: u64) -> f64 {
    let mut value = (index as u64)
        .wrapping_add(stream.wrapping_mul(0x9e37_79b9_7f4a_7c15))
        .wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    ((value >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64)) - 0.5
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err("usage: m6_b3_stage_timing NPHI MMAX THREADS PHI0".into());
    }
    const NTHETA: usize = 721;
    const LMAX: usize = 720;
    let nphi = args[1].parse::<usize>()?;
    let mmax = args[2].parse::<usize>()?;
    let threads = args[3].parse::<usize>()?;
    let phi0 = args[4].parse::<f64>()?;
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()?;
    let plan = ShtPlan::<f64>::new(
        GridSpec::new(GridKind::GaussLegendre, NTHETA, nphi, phi0)?,
        HarmonicSpec::new(LMAX, mmax, 1)?,
    )?;
    let map_len = NTHETA * nphi;
    let q = (0..map_len).map(|i| sample(i, 0)).collect::<Vec<_>>();
    let u = (0..map_len).map(|i| sample(i, 1)).collect::<Vec<_>>();
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let count = plan.harmonics().nalm();
    let mut e = vec![rustsht::Complex64::new(f64::NAN, f64::NAN); count];
    let mut b = vec![rustsht::Complex64::new(f64::NAN, f64::NAN); count];
    for _ in 0..2 {
        plan.analysis_spin_into(&q, &u, &mut e, &mut b, &mut workspace)?;
    }
    #[cfg(rustsht_m6_b3_profile_ordinary_stages)]
    rustsht::legendre::__m6_b3_profile_reset();
    let started = Instant::now();
    plan.analysis_spin_into(&q, &u, &mut e, &mut b, &mut workspace)?;
    let elapsed_ns = started.elapsed().as_nanos();
    #[cfg(rustsht_m6_b3_profile_ordinary_stages)]
    let [
        recurrence_first_ns,
        second_channel_ns,
        range_scan_ns,
        groups,
        sampled_calls,
    ] = rustsht::legendre::__m6_b3_profile_snapshot();
    #[cfg(not(rustsht_m6_b3_profile_ordinary_stages))]
    let [
        recurrence_first_ns,
        second_channel_ns,
        range_scan_ns,
        groups,
        sampled_calls,
    ] = [0; 5];
    let finite = e
        .iter()
        .chain(&b)
        .all(|z| z.re.is_finite() && z.im.is_finite());
    let checksum = e.iter().chain(&b).fold(0_u64, |hash, value| {
        hash.rotate_left(7)
            ^ value.re.to_bits().wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ value.im.to_bits().wrapping_mul(0xbf58_476d_1ce4_e5b9)
    });
    println!(
        "{{\"nphi\":{nphi},\"mmax\":{mmax},\"threads\":{threads},\"phi0\":{phi0},\"elapsed_ns\":{elapsed_ns},\"recurrence_first_sampled_ns\":{recurrence_first_ns},\"second_channel_sampled_ns\":{second_channel_ns},\"range_scan_sampled_ns\":{range_scan_ns},\"pair_groups_sampled\":{groups},\"sampled_order_calls\":{sampled_calls},\"finite\":{finite},\"checksum\":{checksum}}}"
    );
    Ok(())
}
