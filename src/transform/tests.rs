// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex64;
use std::f64::consts::{PI, TAU};

use super::*;
use crate::grid::{GridKind, GridSpec};
use crate::harmonics::HarmonicSpec;
#[cfg(feature = "parallel")]
use crate::plan::LegendreReuseEstimate;

fn make_plan(lmax: usize, mmax: usize, ntheta: usize, nphi: usize, phi0: f64) -> ShtPlan {
    ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, phi0).unwrap(),
        HarmonicSpec::scalar(lmax, mmax).unwrap(),
    )
    .unwrap()
}

fn make_cc_plan(lmax: usize, mmax: usize, ntheta: usize, nphi: usize, phi0: f64) -> ShtPlan {
    ShtPlan::new(
        GridSpec::new(GridKind::ClenshawCurtis, ntheta, nphi, phi0).unwrap(),
        HarmonicSpec::scalar(lmax, mmax).unwrap(),
    )
    .unwrap()
}

fn make_f1_plan(lmax: usize, mmax: usize, ntheta: usize, nphi: usize, phi0: f64) -> ShtPlan {
    ShtPlan::new(
        GridSpec::new(GridKind::Fejer1, ntheta, nphi, phi0).unwrap(),
        HarmonicSpec::scalar(lmax, mmax).unwrap(),
    )
    .unwrap()
}

fn prepare_scalar_latitudes(plan: &ShtPlan, workspace: &mut crate::workspace::ShtWorkspace) {
    for (ring, &theta) in plan.grid.theta().iter().enumerate() {
        (workspace.theta_sin[ring], workspace.theta_cos[ring]) =
            crate::legendre::sin_cos_from_f64(theta);
        workspace.ring_order_limits[ring] = <f64 as crate::legendre::ShtScalar>::polar_order_limit(
            plan.harmonics.lmax(),
            0,
            workspace.theta_sin[ring],
            workspace.theta_cos[ring],
        );
    }
}

#[test]
#[ignore = "Plan 14: capture the production T127 m31 active-ring kernel input"]
fn capture_plan14_t127_active_ring_kernel_input() {
    use std::io::Write;

    let (lmax, m, ntheta, nphi) = (127, 31, 128, 63);
    let plan = make_plan(lmax, m, ntheta, nphi, 0.23);
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut workspace, 1).unwrap();
    let map: Vec<f64> = (0..plan.sample_count)
        .map(|index| {
            let index = index as f64;
            (index * 0.000173).sin() + 0.2 * (index * 0.000071).cos()
        })
        .collect();
    let spectrum_len = plan.fft.spectrum_len();
    for ring in 0..ntheta {
        let map_start = ring * nphi;
        let spectrum_start = ring * spectrum_len;
        plan.fft
            .forward_truncated(
                &map[map_start..map_start + nphi],
                &mut workspace.spectra[spectrum_start..spectrum_start + plan.harmonics.mmax() + 1],
                &mut workspace.fft,
            )
            .unwrap();
    }
    prepare_scalar_latitudes(&plan, &mut workspace);
    for ring in 0..ntheta {
        workspace.theta_cos_squared[ring] =
            stable_analysis_cosine_squared(workspace.theta_sin[ring], workspace.theta_cos[ring]);
    }

    let mut order_spectra = vec![Complex64::new(0.0, 0.0); ntheta];
    let order_preparation = super::ScalarAnalysisOrderPreparation {
        analysis_spectra: &workspace.spectra,
        spectrum_len,
        phi0: plan.grid.phi0(),
        ring_order_limits: &workspace.ring_order_limits[..ntheta],
        scalar_table_selected: false,
        ring_weights: Some(plan.grid.ring_weights()),
        longitude_count: nphi as f64,
        scalar_table_input_limit: None,
    };
    assert!(order_preparation.prepare_order(m, &mut order_spectra));

    let pair_lane_count = ntheta.div_ceil(2);
    let block_start =
        super::first_active_symmetric_pair(m, ntheta, &workspace.ring_order_limits[..ntheta]);
    let block_len = pair_lane_count - block_start;
    let recurrence = &mut workspace.recurrence_workers[0];
    let lane_count = super::prepare_symmetric_analysis_pair_block(
        m,
        ntheta,
        block_start,
        block_len,
        true,
        &order_spectra,
        &workspace.ring_order_limits[..ntheta],
        &workspace.theta_sin[..ntheta],
        &workspace.theta_cos[..ntheta],
        &workspace.theta_cos_squared[..ntheta],
        &mut recurrence.analysis_same_real,
        &mut recurrence.analysis_same_imag,
        &mut recurrence.analysis_opposite_real,
        &mut recurrence.analysis_opposite_imag,
        &mut recurrence.compact_ring_indices,
        &mut recurrence.compact_theta_sin,
        &mut recurrence.compact_theta_cos,
        &mut recurrence.compact_theta_cos_squared,
    );
    assert!(lane_count > 0);
    let unweighted_opposite_real = recurrence.analysis_opposite_real[..lane_count].to_vec();
    let unweighted_opposite_imag = recurrence.analysis_opposite_imag[..lane_count].to_vec();
    let active_sin_theta = recurrence.compact_theta_sin[..lane_count].to_vec();
    assert_eq!(
        recurrence.analysis_same_real.len().min(lane_count),
        lane_count
    );
    let paired_inputs = <f64 as crate::legendre::ShtScalar>::prepare_scalar_analysis_pair_inputs(
        &recurrence.analysis_same_real[..lane_count],
        &recurrence.analysis_same_imag[..lane_count],
        &mut recurrence.analysis_opposite_real[..lane_count],
        &mut recurrence.analysis_opposite_imag[..lane_count],
        &recurrence.compact_theta_cos[..lane_count],
    )
    .expect("f64 paired input preflight");
    assert_eq!(paired_inputs, (true, true));

    let factor_count = lmax / 2 + 2;
    let mut epsilon = vec![0.0; lmax + 4];
    let mut alpha = vec![0.0; factor_count];
    let mut coefficient_a = vec![0.0; factor_count];
    let mut coefficient_b = vec![0.0; factor_count];
    <f64 as crate::legendre::ShtScalar>::fill_epsilon(m, lmax, &mut epsilon);
    assert!(
        <f64 as crate::legendre::ShtScalar>::fill_paired_scalar_factors(
            m,
            lmax,
            &epsilon,
            &mut alpha,
            &mut coefficient_a,
            &mut coefficient_b,
        )
    );

    let mut previous = vec![0.0; lane_count];
    let mut current = vec![0.0; lane_count];
    let mut next = vec![0.0; lane_count];
    let mut exponents = vec![0_i64; lane_count];
    let mut initial_current = vec![0.0; lane_count];
    let mut initial_exponents = vec![0_i64; lane_count];
    let mut captured_seed_lanes = 0;
    <f64 as crate::legendre::ShtScalar>::for_each_y_lm_dispatched(
        pulp::Arch::new(),
        m,
        lmax,
        &recurrence.compact_theta_sin[..lane_count],
        &recurrence.compact_theta_cos[..lane_count],
        plan.sectoral_norms[m],
        &epsilon,
        &mut previous,
        &mut current,
        &mut next,
        &mut exponents,
        |degree, ring_start, values, value_exponents| {
            if degree == m {
                let end = ring_start + values.len();
                initial_current[ring_start..end].copy_from_slice(values);
                initial_exponents[ring_start..end].copy_from_slice(value_exponents);
                captured_seed_lanes += values.len();
            }
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(captured_seed_lanes, lane_count);
    assert!(initial_exponents.iter().all(|&exponent| exponent == 0));
    let pair_count = (lmax - m + 2) / 2;
    let input_path =
        std::env::var("RUSTSHT_PLAN14_KERNEL_INPUT").expect("set RUSTSHT_PLAN14_KERNEL_INPUT");
    let mut fixture = std::fs::File::create(input_path).expect("create T127 kernel fixture");
    for value in [m, lmax, lane_count, pair_count] {
        fixture
            .write_all(&(value as u64).to_le_bytes())
            .expect("write kernel fixture header");
    }
    for values in [
        recurrence.compact_theta_cos[..lane_count].as_ref(),
        recurrence.compact_theta_cos_squared[..lane_count].as_ref(),
        previous.as_slice(),
        initial_current.as_slice(),
        recurrence.analysis_same_real[..lane_count].as_ref(),
        recurrence.analysis_same_imag[..lane_count].as_ref(),
        recurrence.analysis_opposite_real[..lane_count].as_ref(),
        recurrence.analysis_opposite_imag[..lane_count].as_ref(),
        &coefficient_a[..pair_count],
        &coefficient_b[..pair_count],
    ] {
        for value in values {
            fixture
                .write_all(&value.to_le_bytes())
                .expect("write kernel fixture values");
        }
    }
    for values in [
        active_sin_theta.as_slice(),
        unweighted_opposite_real.as_slice(),
        unweighted_opposite_imag.as_slice(),
    ] {
        for value in values {
            fixture
                .write_all(&value.to_le_bytes())
                .expect("write callback-route fixture extension");
        }
    }
    eprintln!(
        "PLAN14_KERNEL_FIXTURE m={m} lmax={lmax} ntheta={ntheta} nphi={nphi} block_start={block_start} block_len={block_len} active_lanes={lane_count} padded_lanes={} pairs={pair_count} previous_production_route=paired_callback candidate_route=fused_min_lmax_127",
        lane_count.div_ceil(4) * 4
    );
}

fn scalar_ring_group_spectra(plan: &ShtPlan, alm: &[Complex64]) -> Vec<Complex64> {
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut workspace, 1).unwrap();
    prepare_scalar_latitudes(plan, &mut workspace);
    plan.synthesize_ring_group(
        alm,
        plan.grid.nphi(),
        plan.fft.spectrum_len(),
        plan.grid.phi0(),
        0,
        &mut workspace.spectra,
        &workspace.theta_sin[..plan.grid.ntheta()],
        &workspace.theta_cos[..plan.grid.ntheta()],
        &workspace.ring_order_limits[..plan.grid.ntheta()],
        pulp::Arch::new(),
        &mut workspace.recurrence_workers[0],
    )
    .unwrap();
    workspace.spectra
}

fn coefficients(spec: &HarmonicSpec) -> Vec<Complex64> {
    let mut alm = vec![Complex64::new(0.0, 0.0); spec.nalm()];
    for (l, m, value) in [
        (0, 0, Complex64::new(0.7, 0.0)),
        (1, 0, Complex64::new(-0.2, 0.0)),
        (2, 0, Complex64::new(0.1, 0.0)),
        (1, 1, Complex64::new(0.4, 0.3)),
        (2, 1, Complex64::new(-0.2, 0.15)),
        (2, 2, Complex64::new(0.1, -0.6)),
    ] {
        if let Some(index) = spec.index(l, m) {
            alm[index] = value;
        }
    }
    alm
}

#[test]
fn paired_analysis_final_conversion_matches_indexed_reference() {
    for (m, lmax) in [(0, 0), (0, 5), (1, 6), (2, 9), (5, 10)] {
        let mut epsilon = vec![0.0; lmax + 2];
        for (degree, value) in epsilon.iter_mut().enumerate() {
            *value = 0.25 + (degree % 11) as f64 * 0.07;
        }
        let pair_count = (lmax - m) / 2 + 1;
        let alpha: Vec<_> = (0..pair_count)
            .map(|pair| 0.6 + (pair % 7) as f64 * 0.09)
            .collect();
        let mut actual: Vec<_> = (0..=lmax - m)
            .map(|offset| {
                Complex64::new(
                    (offset as f64 - 3.0) * 0.13,
                    (offset as f64 % 5.0 - 2.0) * 0.17,
                )
            })
            .collect();
        let mut expected = actual.clone();
        let mut previous_even = Complex64::new(0.0, 0.0);
        for (pair_index, l) in (m..=lmax).step_by(2).enumerate() {
            let offset = l - m;
            let even = expected[offset];
            let odd = if l < lmax {
                expected[offset + 1]
            } else {
                Complex64::new(0.0, 0.0)
            };
            let previous = if pair_index == 0 {
                Complex64::new(0.0, 0.0)
            } else {
                previous_even * (alpha[pair_index - 1] * epsilon[l])
            };
            expected[offset] = even * (alpha[pair_index] * epsilon[l + 1]) + previous;
            if l < lmax {
                expected[offset + 1] = odd * alpha[pair_index];
            }
            previous_even = even;
        }

        finalize_paired_scalar_analysis_coefficients(&mut actual, m, lmax, &epsilon, &alpha);
        assert_eq!(actual, expected, "m={m}, lmax={lmax}");
    }
}

fn analytic_y(theta: f64, phi: f64, l: usize, m: usize) -> Complex64 {
    let (sin_theta, cos_theta) = theta.sin_cos();
    let amplitude = match (l, m) {
        (0, 0) => 1.0 / (4.0 * PI).sqrt(),
        (1, 0) => (3.0 / (4.0 * PI)).sqrt() * cos_theta,
        (2, 0) => (5.0 / (16.0 * PI)).sqrt() * (3.0 * cos_theta * cos_theta - 1.0),
        (1, 1) => -(3.0 / (8.0 * PI)).sqrt() * sin_theta,
        (2, 1) => -(15.0 / (8.0 * PI)).sqrt() * sin_theta * cos_theta,
        (2, 2) => (15.0 / (32.0 * PI)).sqrt() * sin_theta * sin_theta,
        _ => 0.0,
    };
    Complex64::from_polar(amplitude, m as f64 * phi)
}

fn analytic_map(plan: &ShtPlan, alm: &[Complex64]) -> Vec<f64> {
    let mut map = vec![0.0; plan.sample_count];
    for (ring, &theta) in plan.grid.theta().iter().enumerate() {
        for j in 0..plan.grid.nphi() {
            let phi = plan.grid.longitude(j).unwrap();
            let mut value = 0.0;
            for m in 0..=plan.harmonics.mmax() {
                for l in m..=2.min(plan.harmonics.lmax()) {
                    if let Some(index) = plan.harmonics.index(l, m) {
                        let mode = alm[index] * analytic_y(theta, phi, l, m);
                        value += if m == 0 { mode.re } else { 2.0 * mode.re };
                    }
                }
            }
            map[ring * plan.grid.nphi() + j] = value;
        }
    }
    map
}

fn arbitrary_map(plan: &ShtPlan) -> Vec<f64> {
    let mut map = Vec::with_capacity(plan.sample_count);
    for i in 0..plan.grid.ntheta() {
        for j in 0..plan.grid.nphi() {
            map.push(
                (0.17 * i as f64 + 0.23 * j as f64).sin()
                    + 0.2 * (0.11 * i as f64 - 0.31 * j as f64).cos(),
            );
        }
    }
    map
}

fn map_inner_product(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(x, y)| x * y).sum()
}

fn relative_l2_real(actual: &[f64], expected: &[f64]) -> f64 {
    let (difference, reference) = actual.iter().zip(expected).fold(
        (0.0, 0.0),
        |(difference, reference), (&actual, &expected)| {
            (
                difference + (actual - expected).powi(2),
                reference + expected.powi(2),
            )
        },
    );
    (difference / reference.max(1e-300)).sqrt()
}

fn relative_l2_complex(actual: &[Complex64], expected: &[Complex64]) -> f64 {
    let (difference, reference) = actual.iter().zip(expected).fold(
        (0.0, 0.0),
        |(difference, reference), (&actual, &expected)| {
            (
                difference + (actual - expected).norm_sqr(),
                reference + expected.norm_sqr(),
            )
        },
    );
    (difference / reference.max(1e-300)).sqrt()
}

#[test]
#[ignore = "manual active-pair tile utilization report"]
fn report_scalar_analysis_pair_tile_utilization() {
    for (lmax, mmax, ntheta, nphi) in [(720, 720, 721, 1443), (300, 150, 301, 603)] {
        let plan = make_plan(lmax, mmax, ntheta, nphi, 0.37);
        let mut workspace = plan.workspace().unwrap();
        prepare_scalar_latitudes(&plan, &mut workspace);
        for ring in 0..ntheta {
            workspace.theta_cos_squared[ring] = stable_analysis_cosine_squared(
                workspace.theta_sin[ring],
                workspace.theta_cos[ring],
            );
        }

        let pair_lane_count = ntheta.div_ceil(2);
        let order_spectra = vec![Complex64::new(0.3, -0.2); ntheta];
        let recurrence = &mut workspace.recurrence_workers[0];
        let mut reference_active_lanes = None;
        for tile_size in [64, 128, 256] {
            let mut block_count = 0;
            let mut active_lanes = 0;
            let mut block_lane_capacity = 0;
            let mut simd_lane_capacity = 0;
            for m in 0..=mmax {
                let first = first_active_symmetric_pair(m, ntheta, &workspace.ring_order_limits);
                for block_start in (first..pair_lane_count).step_by(tile_size) {
                    let block_len = (block_start + tile_size).min(pair_lane_count) - block_start;
                    let packed = prepare_symmetric_analysis_pair_block(
                        m,
                        ntheta,
                        block_start,
                        block_len,
                        true,
                        &order_spectra,
                        &workspace.ring_order_limits,
                        &workspace.theta_sin,
                        &workspace.theta_cos,
                        &workspace.theta_cos_squared,
                        &mut recurrence.analysis_same_real,
                        &mut recurrence.analysis_same_imag,
                        &mut recurrence.analysis_opposite_real,
                        &mut recurrence.analysis_opposite_imag,
                        &mut recurrence.compact_ring_indices,
                        &mut recurrence.compact_theta_sin,
                        &mut recurrence.compact_theta_cos,
                        &mut recurrence.compact_theta_cos_squared,
                    );
                    block_count += 1;
                    active_lanes += packed;
                    block_lane_capacity += block_len;
                    simd_lane_capacity += packed.div_ceil(4) * 4;
                }
            }
            if let Some(reference) = reference_active_lanes {
                assert_eq!(active_lanes, reference);
            } else {
                reference_active_lanes = Some(active_lanes);
            }
            eprintln!(
                "PAIR_TILE lmax={lmax} mmax={mmax} tile={tile_size} blocks={block_count} active_lanes={active_lanes} block_lane_capacity={block_lane_capacity} simd_lane_capacity={simd_lane_capacity}"
            );
        }

        let f64_capacities = [
            recurrence.previous.capacity(),
            recurrence.current.capacity(),
            recurrence.next.capacity(),
            recurrence.analysis_scale_epochs.capacity(),
            recurrence.analysis_scale_corrections.capacity(),
            recurrence.spin_previous_plus.capacity(),
            recurrence.spin_current_plus.capacity(),
            recurrence.spin_next_plus.capacity(),
            recurrence.spin_previous_minus.capacity(),
            recurrence.spin_current_minus.capacity(),
            recurrence.spin_next_minus.capacity(),
            recurrence.analysis_same_real.capacity(),
            recurrence.analysis_same_imag.capacity(),
            recurrence.analysis_opposite_real.capacity(),
            recurrence.analysis_opposite_imag.capacity(),
            recurrence.scalar_order_correction.capacity(),
            recurrence.compact_theta_sin.capacity(),
            recurrence.compact_theta_cos.capacity(),
            recurrence.compact_theta_cos_squared.capacity(),
        ]
        .into_iter()
        .sum::<usize>();
        let i64_capacities = recurrence.exponents.capacity()
            + recurrence.spin_exponents_plus.capacity()
            + recurrence.spin_exponents_minus.capacity();
        let complex_capacities = recurrence.weighted_spin_q_spectrum.capacity()
            + recurrence.weighted_spin_u_spectrum.capacity();
        let index_capacity = recurrence.compact_ring_indices.capacity();
        let scratch_bytes = f64_capacities * std::mem::size_of::<f64>()
            + i64_capacities * std::mem::size_of::<i64>()
            + complex_capacities * std::mem::size_of::<Complex64>()
            + index_capacity * std::mem::size_of::<usize>();
        eprintln!(
            "PAIR_TILE_SCRATCH lmax={lmax} lanes={} bytes_per_worker={scratch_bytes}",
            recurrence.previous.len()
        );
    }
}

fn harmonic_inner_product(spec: &HarmonicSpec, left: &[Complex64], right: &[Complex64]) -> f64 {
    let mut result = 0.0;
    for m in 0..=spec.mmax() {
        let weight = if m == 0 { 1.0 } else { 2.0 };
        for l in m..=spec.lmax() {
            let index = spec.index(l, m).unwrap();
            result += weight * (left[index].conj() * right[index]).re;
        }
    }
    result
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance * expected.abs().max(1.0),
        "actual={actual:.16e}, expected={expected:.16e}"
    );
}

fn assert_complex_close(actual: Complex64, expected: Complex64, tolerance: f64) {
    assert!(
        (actual - expected).norm() <= tolerance,
        "actual={actual:?}, expected={expected:?}, error={:.3e}",
        (actual - expected).norm()
    );
}

fn assert_reference_close(actual: f64, expected: f64) {
    if expected == 0.0 {
        assert_eq!(actual, 0.0, "expected exact zero, got {actual:.16e}");
    } else {
        let relative_error = (actual - expected).abs() / expected.abs();
        assert!(
            relative_error <= 1e-12,
            "actual={actual:.16e}, expected={expected:.16e}, relative error={relative_error:.3e}"
        );
    }
}

#[cfg(feature = "parallel")]
#[test]
fn weighted_order_midpoint_minimizes_coefficient_work() {
    for (lmax, m_start, m_end) in [
        (511, 0, 512),
        (511, 0, 128),
        (511, 300, 512),
        (20, 4, 11),
        (2, 0, 3),
        (511, 0, 2),
    ] {
        let midpoint = weighted_order_midpoint(lmax, m_start, m_end);
        assert!(m_start < midpoint && midpoint < m_end);
        let work = |start: usize, end: usize| (start..end).map(|m| lmax - m + 1).sum::<usize>();
        let difference = work(m_start, midpoint).abs_diff(work(midpoint, m_end));
        for candidate in (m_start + 1)..m_end {
            let candidate_difference = work(m_start, candidate).abs_diff(work(candidate, m_end));
            assert!(
                difference <= candidate_difference,
                "lmax={lmax}, m_start={m_start}, m_end={m_end}, midpoint={midpoint}, candidate={candidate}"
            );
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_active_ring_schedule_matches_one_and_eight_threads() {
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    for lmax in [255, 1023] {
        let plan = make_plan(lmax, lmax, lmax + 1, 2 * lmax + 3, 0.37);
        let input = arbitrary_map(&plan);
        let mut one_workspace = one_thread.install(|| plan.workspace()).unwrap();
        let mut eight_workspace = eight_threads.install(|| plan.workspace()).unwrap();
        let mut one_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut eight_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];

        one_thread
            .install(|| plan.analysis_into(&input, &mut one_alm, &mut one_workspace))
            .unwrap();
        eight_threads
            .install(|| plan.analysis_into(&input, &mut eight_alm, &mut eight_workspace))
            .unwrap();

        for (&one, &eight) in one_alm.iter().zip(&eight_alm) {
            assert!((one - eight).norm() < 2e-13);
        }
    }
}

#[test]
fn scalar_reflected_active_pairs_form_one_equatorial_interval() {
    for (kind, lmax, ntheta) in [
        (GridKind::GaussLegendre, 127, 128),
        (GridKind::ClenshawCurtis, 127, 130),
        (GridKind::Fejer1, 127, 128),
        (GridKind::GaussLegendre, 191, 192),
        (GridKind::ClenshawCurtis, 191, 194),
        (GridKind::Fejer1, 191, 192),
    ] {
        let grid = GridSpec::new(kind, ntheta, 2 * lmax + 3, 0.37).unwrap();
        let ring_order_limits = grid
            .theta()
            .iter()
            .map(|&theta| {
                let (sin_theta, cos_theta) = crate::legendre::sin_cos_from_f64(theta);
                <f64 as crate::legendre::ShtScalar>::polar_order_limit(
                    lmax, 0, sin_theta, cos_theta,
                )
            })
            .collect::<Vec<_>>();
        let pair_lane_count = ntheta.div_ceil(2);
        let pair_limit =
            |pair: usize| ring_order_limits[pair].max(ring_order_limits[ntheta - 1 - pair]);

        assert!(
            (1..pair_lane_count).all(|pair| pair_limit(pair - 1) <= pair_limit(pair)),
            "reflected order limits must increase toward the equator for {kind:?}, T{lmax}"
        );
        for m in 0..=lmax {
            let reference = (0..pair_lane_count)
                .find(|&pair| pair_limit(pair) >= m)
                .unwrap_or(pair_lane_count);
            let actual = first_active_symmetric_pair(m, ntheta, &ring_order_limits);
            assert_eq!(actual, reference, "{kind:?}, T{lmax}, m={m}");
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn analysis_fft_workspace_respects_shape_threshold_and_pool_migration() {
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let four_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();

    let small_plan = make_plan(63, 63, 64, 129, 0.37);
    let small_workspace = four_threads.install(|| small_plan.workspace()).unwrap();
    assert!(small_workspace.fft_worker_workspaces.is_empty());

    let plan = make_plan(127, 127, 128, 257, 0.37);
    let mut adapted_workspace = one_thread.install(|| plan.workspace()).unwrap();
    assert!(adapted_workspace.fft_worker_workspaces.is_empty());
    one_thread
        .install(|| plan.prepare_workspace(&mut adapted_workspace, 4))
        .unwrap();
    assert_eq!(adapted_workspace.execution_capacity(), 4);
    assert_eq!(adapted_workspace.fft_worker_workspaces.len(), 4);

    let input = arbitrary_map(&plan);
    let mut reference_workspace = one_thread.install(|| plan.workspace()).unwrap();
    let mut reference = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut adapted = reference.clone();
    one_thread
        .install(|| plan.analysis_into(&input, &mut reference, &mut reference_workspace))
        .unwrap();
    four_threads
        .install(|| plan.analysis_into(&input, &mut adapted, &mut adapted_workspace))
        .unwrap();
    assert!(relative_l2_complex(&adapted, &reference) <= 3e-10);
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_and_spin_analysis_match_across_pool_sizes() {
    let lmax = 511;
    let mmax = 15;
    let ntheta = lmax + 1;
    let nphi = 2 * mmax + 5;
    let scalar_plan = make_plan(lmax, mmax, ntheta, nphi, 0.37);
    let spin_plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
        HarmonicSpec::new(lmax, mmax, 1).unwrap(),
    )
    .unwrap();
    let scalar_input = arbitrary_map(&scalar_plan);
    let spin_first = arbitrary_map(&spin_plan);
    let spin_second = (0..spin_plan.sample_count)
        .map(|index| (0.013 * index as f64).cos() - 0.2 * (0.021 * index as f64).sin())
        .collect::<Vec<_>>();
    let mut scalar_reference = None;
    let mut spin_e_reference = None;
    let mut spin_b_reference = None;

    for thread_count in [1, 2, 3, 4, 5, 6, 8, 16] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap();
        let mut scalar_workspace = pool.install(|| scalar_plan.workspace()).unwrap();
        let mut spin_workspace = pool.install(|| spin_plan.workspace()).unwrap();
        let mut scalar_alm = vec![Complex64::new(0.0, 0.0); scalar_plan.harmonics().nalm()];
        let mut spin_e = vec![Complex64::new(0.0, 0.0); spin_plan.harmonics().nalm()];
        let mut spin_b = spin_e.clone();

        pool.install(|| {
            scalar_plan.analysis_into(&scalar_input, &mut scalar_alm, &mut scalar_workspace)
        })
        .unwrap();
        pool.install(|| {
            spin_plan.analysis_spin_into(
                &spin_first,
                &spin_second,
                &mut spin_e,
                &mut spin_b,
                &mut spin_workspace,
            )
        })
        .unwrap();

        if thread_count == 1 {
            scalar_reference = Some(scalar_alm);
            spin_e_reference = Some(spin_e);
            spin_b_reference = Some(spin_b);
        } else {
            assert!(
                relative_l2_complex(&scalar_alm, scalar_reference.as_ref().unwrap()) <= 3e-10,
                "scalar q{thread_count} differs from q1"
            );
            assert!(
                relative_l2_complex(&spin_e, spin_e_reference.as_ref().unwrap()) <= 3e-10,
                "spin E q{thread_count} differs from q1"
            );
            assert!(
                relative_l2_complex(&spin_b, spin_b_reference.as_ref().unwrap()) <= 3e-10,
                "spin B q{thread_count} differs from q1"
            );
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "M5 per-leaf scheduling profile; run this diagnostic in release mode"]
fn m5_analysis_leaf_imbalance_profile() {
    for (spin, lmax, thread_count) in [
        (0, 511, 3),
        (0, 511, 5),
        (0, 511, 6),
        (0, 511, 8),
        (0, 511, 16),
        (0, 1023, 3),
        (0, 1023, 5),
        (0, 1023, 6),
        (0, 1023, 8),
        (0, 1023, 16),
        (1, 511, 3),
        (1, 511, 5),
        (1, 511, 6),
        (1, 511, 8),
        (1, 511, 16),
        (1, 1023, 3),
        (1, 1023, 5),
        (1, 1023, 6),
        (1, 1023, 8),
        (1, 1023, 16),
        (0, 720, 8),
        (1, 720, 8),
        (0, 1279, 8),
        (1, 1279, 8),
        (0, 2047, 8),
        (1, 2047, 8),
    ] {
        let ntheta = lmax + 1;
        let nphi = 2 * lmax + 3;
        let plan = ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::new(lmax, lmax, spin).unwrap(),
        )
        .unwrap();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap();
        let root_worker = pool.install(rayon::current_thread_index).unwrap();
        let mut workspace = pool.install(|| plan.workspace()).unwrap();
        let scratch_slots = workspace.allocated_worker_count();
        let profile = workspace.observe_analysis_leaves();
        let sample_count = plan.sample_count;
        let mut first = vec![0.0; sample_count];
        let mut second = vec![0.0; sample_count];
        for index in 0..sample_count {
            first[index] = (index as f64 * 0.000173).sin() + 0.2 * (index as f64 * 0.000071).cos();
            second[index] =
                (index as f64 * 0.000119).cos() - 0.15 * (index as f64 * 0.000097).sin();
        }
        let mut scalar_output = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut spin_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut spin_b = spin_e.clone();

        for sample in 0..5 {
            profile
                .lock()
                .expect("analysis leaf profile is not poisoned")
                .clear();
            let started = std::time::Instant::now();
            pool.install(|| {
                if spin == 0 {
                    plan.analysis_into(&first, &mut scalar_output, &mut workspace)
                        .unwrap();
                } else {
                    plan.analysis_spin_into(
                        &first,
                        &second,
                        &mut spin_e,
                        &mut spin_b,
                        &mut workspace,
                    )
                    .unwrap();
                }
            });
            let wall_ns = started.elapsed().as_nanos();
            let mut leaves = profile
                .lock()
                .expect("analysis leaf profile is not poisoned")
                .clone();
            assert!(!leaves.is_empty(), "no analysis leaves were recorded");
            leaves.sort_by_key(|leaf| leaf.m_start);
            let worker_ids = leaves
                .iter()
                .filter_map(|leaf| leaf.worker)
                .collect::<std::collections::BTreeSet<_>>();
            let leaf_time_ns = leaves.iter().map(|leaf| leaf.elapsed_ns).sum::<u128>();
            let leaf_capacity_fraction =
                leaf_time_ns as f64 / (thread_count as f64 * wall_ns as f64);
            eprintln!(
                "LEAF_PROFILE spin={spin} T{lmax} q{thread_count} sample={} root_worker={root_worker} scratch_slots={scratch_slots} workers={worker_ids:?} wall_ms={:.3} summed_leaf_ms={:.3} leaf_capacity_fraction={:.3}",
                sample + 1,
                wall_ns as f64 / 1e6,
                leaf_time_ns as f64 / 1e6,
                leaf_capacity_fraction,
            );
            for leaf in leaves {
                eprintln!(
                    "LEAF spin={} T{lmax} q{thread_count} sample={} worker={:?} m={}..{} orders={} active_ring_order_pairs={} estimated_work={} start_ms={:.3} end_ms={:.3} elapsed_ms={:.3} normal_blocks={} scaled_blocks={} mixed_blocks={}",
                    if leaf.spin { 1 } else { 0 },
                    sample + 1,
                    leaf.worker,
                    leaf.m_start,
                    leaf.m_end,
                    leaf.m_end - leaf.m_start,
                    leaf.active_ring_order_pairs,
                    leaf.estimated_work,
                    leaf.started.duration_since(started).as_secs_f64() * 1e3,
                    leaf.finished.duration_since(started).as_secs_f64() * 1e3,
                    leaf.elapsed_ns as f64 / 1e6,
                    leaf.normal_blocks,
                    leaf.scaled_blocks,
                    leaf.mixed_blocks,
                );
            }
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "T2047/T2048/T3072 seed-route diagnostic; run as one release test outside timing runs"]
fn report_scalar_analysis_seed_routes_high_degree() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();

    for (lmax, mmax, ntheta, nphi) in [
        (2047, 2047, 2048, 4096),
        (2047, 2047, 2049, 4096),
        (2048, 2047, 2049, 4096),
        (3072, 3072, 3073, 6174),
    ] {
        let plan = make_plan(lmax, mmax, ntheta, nphi, 0.37);
        let mut workspace = pool.install(|| plan.workspace()).unwrap();
        plan.prepare_workspace(&mut workspace, 1).unwrap();
        let map = vec![0.0; plan.sample_count];
        let mut coefficients = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];

        crate::legendre::begin_scalar_analysis_route_profile();
        pool.install(|| {
            plan.analysis_into(&map, &mut coefficients, &mut workspace)
                .unwrap();
        });
        let routes: crate::legendre::ScalarAnalysisRouteCounts =
            crate::legendre::finish_scalar_analysis_route_profile();

        eprintln!(
            "SCALAR_ANALYSIS_SEED_ROUTES T{lmax} ntheta={ntheta} nphi={nphi} mmax={mmax} tile={} normal={} prefix_enabled_scaled={} checked_scaled={} prefix_init_declines={} prefix_recurrence_declines={} checked_scaled_recurrence_declines={}",
            crate::legendre::SCALAR_ANALYSIS_PAIR_TILE_SIZE,
            routes.normal_seed_blocks,
            routes.prefix_enabled_scaled_blocks,
            routes.checked_scaled_blocks,
            routes.prefix_initialization_declines,
            routes.prefix_recurrence_declines,
            routes.checked_scaled_recurrence_declines,
        );
        assert!(
            routes.normal_seed_blocks
                + routes.prefix_enabled_scaled_blocks
                + routes.checked_scaled_blocks
                > 0
        );
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "M5 current-source analysis phase profile; run this diagnostic in release mode"]
fn m5_analysis_phase_profile() {
    for (spin, lmax, thread_count) in [
        (0, 511, 1),
        (0, 511, 6),
        (0, 1023, 1),
        (0, 1023, 6),
        (1, 511, 1),
        (1, 511, 6),
        (1, 1023, 1),
        (1, 1023, 6),
        (0, 720, 1),
        (0, 720, 8),
        (0, 1023, 8),
        (0, 1279, 1),
        (0, 1279, 8),
        (0, 2047, 1),
        (0, 2047, 8),
        (1, 720, 1),
        (1, 720, 8),
        (1, 1023, 8),
        (1, 1279, 1),
        (1, 1279, 8),
        (1, 2047, 1),
        (1, 2047, 8),
    ] {
        let ntheta = lmax + 1;
        let nphi = 2 * lmax + 3;
        let plan = ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::new(lmax, lmax, spin).unwrap(),
        )
        .unwrap();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap();
        let mut workspace = pool.install(|| plan.workspace()).unwrap();
        let profile = workspace.observe_transform_phases();
        let sample_count = plan.sample_count;
        let mut first = vec![0.0; sample_count];
        let mut second = vec![0.0; sample_count];
        for index in 0..sample_count {
            first[index] = (index as f64 * 0.000173).sin() + 0.2 * (index as f64 * 0.000071).cos();
            second[index] =
                (index as f64 * 0.000119).cos() - 0.15 * (index as f64 * 0.000097).sin();
        }
        let mut scalar_output = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut spin_e = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut spin_b = spin_e.clone();

        for sample in 0..5 {
            profile
                .lock()
                .expect("analysis phase profile is not poisoned")
                .clear();
            let started = std::time::Instant::now();
            pool.install(|| {
                if spin == 0 {
                    plan.analysis_into(&first, &mut scalar_output, &mut workspace)
                        .unwrap();
                } else {
                    plan.analysis_spin_into(
                        &first,
                        &second,
                        &mut spin_e,
                        &mut spin_b,
                        &mut workspace,
                    )
                    .unwrap();
                }
            });
            let wall_ns = started.elapsed().as_nanos();
            let phases = profile
                .lock()
                .expect("analysis phase profile is not poisoned")
                .clone();
            assert!(!phases.is_empty(), "no analysis phases were recorded");
            let phase_ns = phases.iter().map(|phase| phase.elapsed_ns).sum::<u128>();
            eprintln!(
                "PHASE_PROFILE spin={spin} T{lmax} q{thread_count} sample={} wall_ms={:.3} phase_sum_ms={:.3} unmeasured_ms={:.3}",
                sample + 1,
                wall_ns as f64 / 1e6,
                phase_ns as f64 / 1e6,
                (wall_ns.saturating_sub(phase_ns)) as f64 / 1e6,
            );
            for phase in phases {
                eprintln!(
                    "PHASE spin={spin} T{lmax} q{thread_count} sample={} name={:?} elapsed_ms={:.3}",
                    sample + 1,
                    phase.phase,
                    phase.elapsed_ns as f64 / 1e6,
                );
            }
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "focused current-source scalar analysis phase profile; set RUSTSHT_PROFILE_Q and RUSTSHT_PROFILE_LMAX, optionally RUSTSHT_PROFILE_MMAX"]
fn scalar_analysis_phase_profile_selected() {
    let thread_count = std::env::var("RUSTSHT_PROFILE_Q")
        .expect("set RUSTSHT_PROFILE_Q")
        .parse::<usize>()
        .expect("RUSTSHT_PROFILE_Q must be an integer");
    assert!(thread_count > 0);
    let lmax_values = std::env::var("RUSTSHT_PROFILE_LMAX")
        .expect("set RUSTSHT_PROFILE_LMAX")
        .split(',')
        .map(|value| value.parse::<usize>().expect("invalid lmax"))
        .collect::<Vec<_>>();
    let mmax_override = std::env::var("RUSTSHT_PROFILE_MMAX")
        .ok()
        .map(|value| value.parse::<usize>().expect("invalid mmax"));

    for lmax in lmax_values {
        let (ntheta, nphi) = match lmax {
            720 => (721, 1442),
            2047 => (2049, 4096),
            3072 => (3073, 6174),
            _ => panic!("unsupported focused profile case T{lmax}"),
        };
        let mmax = mmax_override.unwrap_or(lmax);
        assert!(mmax <= lmax);
        let plan = make_plan(lmax, mmax, ntheta, nphi, 0.37);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap();
        let mut workspace = pool.install(|| plan.workspace()).unwrap();
        pool.install(|| plan.prepare_workspace(&mut workspace, thread_count))
            .unwrap();
        let probe = workspace.observe_transform_phases();
        let map = (0..plan.sample_count)
            .map(|index| {
                let index = index as f64;
                (index * 0.000173).sin() + 0.2 * (index * 0.000071).cos()
            })
            .collect::<Vec<_>>();
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];

        pool.install(|| {
            plan.analysis_into(&map, &mut alm, &mut workspace).unwrap();
        });
        for sample in 0..5 {
            probe
                .lock()
                .expect("analysis phase profile is not poisoned")
                .clear();
            let started = std::time::Instant::now();
            pool.install(|| {
                plan.analysis_into(&map, &mut alm, &mut workspace).unwrap();
            });
            let wall_ns = started.elapsed().as_nanos();
            let phases = probe
                .lock()
                .expect("analysis phase profile is not poisoned")
                .clone();
            let phase_sum_ns = phases.iter().map(|phase| phase.elapsed_ns).sum::<u128>();
            eprintln!(
                "FOCUSED_PHASE_PROFILE T{lmax} m{mmax} q{thread_count} sample={} wall_ms={:.3} phase_sum_ms={:.3} unmeasured_ms={:.3}",
                sample + 1,
                wall_ns as f64 / 1e6,
                phase_sum_ns as f64 / 1e6,
                wall_ns.saturating_sub(phase_sum_ns) as f64 / 1e6,
            );
            for phase in phases {
                eprintln!(
                    "FOCUSED_PHASE T{lmax} m{mmax} q{thread_count} sample={} name={:?} elapsed_ms={:.3}",
                    sample + 1,
                    phase.phase,
                    phase.elapsed_ns as f64 / 1e6,
                );
            }
            assert!(
                alm.iter()
                    .all(|value| value.re.is_finite() && value.im.is_finite())
            );
            std::hint::black_box(&alm);
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "Plan 14 calibrated analysis phase profile; set geometry, q and affinity"]
fn plan14_analysis_phase_batch_profile() {
    let thread_count = std::env::var("RUSTSHT_PROFILE_Q")
        .expect("set RUSTSHT_PROFILE_Q")
        .parse::<usize>()
        .expect("RUSTSHT_PROFILE_Q must be an integer");
    let lmax = std::env::var("RUSTSHT_PROFILE_LMAX")
        .expect("set RUSTSHT_PROFILE_LMAX")
        .parse::<usize>()
        .expect("RUSTSHT_PROFILE_LMAX must be an integer");
    let mmax = std::env::var("RUSTSHT_PROFILE_MMAX")
        .expect("set RUSTSHT_PROFILE_MMAX")
        .parse::<usize>()
        .expect("RUSTSHT_PROFILE_MMAX must be an integer");
    let batches = std::env::var("RUSTSHT_PROFILE_BATCHES")
        .ok()
        .map(|value| value.parse::<usize>().expect("invalid batch count"))
        .unwrap_or(30);
    let target_batch_ms = std::env::var("RUSTSHT_PROFILE_BATCH_MS")
        .ok()
        .map(|value| value.parse::<f64>().expect("invalid batch duration"))
        .unwrap_or(30.0);
    assert!(mmax <= lmax && thread_count > 0 && batches >= 30);
    assert!((20.0..=100.0).contains(&target_batch_ms));

    let ntheta = std::env::var("RUSTSHT_PROFILE_NTHETA")
        .ok()
        .map(|value| value.parse::<usize>().expect("invalid ntheta"))
        .unwrap_or(lmax + 1);
    let nphi = std::env::var("RUSTSHT_PROFILE_NPHI")
        .ok()
        .map(|value| value.parse::<usize>().expect("invalid nphi"))
        .unwrap_or_else(|| GridSpec::fft_friendly_nphi(lmax).unwrap());
    assert!(ntheta > lmax);
    assert!(nphi > 2 * mmax);
    let plan = make_plan(lmax, mmax, ntheta, nphi, 0.23);
    let force_recurrence =
        std::env::var("RUSTSHT_PROFILE_FORCE_RECURRENCE").is_ok_and(|value| value == "1");
    if force_recurrence {
        assert!(plan.scalar_legendre_table.set(None).is_ok());
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .unwrap();
    let mut workspace = pool.install(|| plan.workspace()).unwrap();
    pool.install(|| plan.prepare_workspace(&mut workspace, thread_count))
        .unwrap();
    let probe = workspace.observe_transform_phases();

    let map = if let Ok(path) = std::env::var("RUSTSHT_PROFILE_MAP") {
        let bytes = std::fs::read(path).expect("read profile map fixture");
        assert_eq!(bytes.len(), plan.sample_count * 8);
        let (chunks, remainder) = bytes.as_chunks::<8>();
        assert!(remainder.is_empty());
        chunks
            .iter()
            .map(|chunk| f64::from_le_bytes(*chunk))
            .collect::<Vec<_>>()
    } else {
        (0..plan.sample_count)
            .map(|index| {
                let index = index as f64;
                (index * 0.000173).sin() + 0.2 * (index * 0.000071).cos()
            })
            .collect::<Vec<_>>()
    };
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];

    pool.install(|| {
        plan.analysis_into(&map, &mut alm, &mut workspace).unwrap();
    });
    let started = std::time::Instant::now();
    pool.install(|| {
        plan.analysis_into(&map, &mut alm, &mut workspace).unwrap();
    });
    let calibration_call_ms = started.elapsed().as_secs_f64() * 1e3;
    let repetitions =
        ((target_batch_ms / calibration_call_ms.max(1e-6)).ceil() as usize).clamp(1, 100_000);

    let mut batch_records = Vec::with_capacity(batches);
    for batch in 0..batches {
        probe
            .lock()
            .expect("analysis phase profile is not poisoned")
            .clear();
        let started = std::time::Instant::now();
        pool.install(|| {
            for _ in 0..repetitions {
                plan.analysis_into(&map, &mut alm, &mut workspace).unwrap();
            }
            std::hint::black_box(&alm);
        });
        let wall_ms = started.elapsed().as_secs_f64() * 1e3;
        let phases = probe
            .lock()
            .expect("analysis phase profile is not poisoned")
            .clone();
        let mut phase_totals = std::collections::BTreeMap::<&str, u128>::new();
        for phase in phases {
            *phase_totals.entry(phase.phase).or_default() += phase.elapsed_ns;
        }
        assert!(
            alm.iter()
                .all(|value| value.re.is_finite() && value.im.is_finite())
        );
        let per_call_us = phase_totals
            .iter()
            .map(|(name, value)| format!("{name}={:.3}", *value as f64 / repetitions as f64 / 1e3))
            .collect::<Vec<_>>()
            .join(" ");
        eprintln!(
            "PLAN14_ANALYSIS_PHASE_BATCH T{lmax} m{mmax} grid={ntheta}x{nphi} q{thread_count} batch={} repeats={repetitions} wall_us={:.3} {}",
            batch + 1,
            wall_ms * 1e3 / repetitions as f64,
            per_call_us,
        );
        batch_records.push((wall_ms * 1e3 / repetitions as f64, phase_totals));
    }
    let quantile = |values: &[f64], q: f64| {
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        sorted[((sorted.len() - 1) as f64 * q).round() as usize]
    };
    let wall_samples = batch_records
        .iter()
        .map(|record| record.0)
        .collect::<Vec<_>>();
    let selected_arch = format!("{:?}", pulp::Arch::new());
    let mut phase_names = std::collections::BTreeSet::new();
    for (_, phases) in &batch_records {
        phase_names.extend(phases.keys().copied());
    }
    eprintln!(
        "PLAN14_ANALYSIS_PHASE_META T{lmax} m{mmax} grid={ntheta}x{nphi} q{thread_count} route={} arch={selected_arch} batches={batches} repeats={repetitions} target_batch_ms={target_batch_ms:.3} calibration_call_ms={calibration_call_ms:.6} wall_median_us={:.3} wall_min_us={:.3} wall_max_us={:.3}",
        if force_recurrence {
            "forced-recurrence"
        } else {
            "production"
        },
        quantile(&wall_samples, 0.5),
        quantile(&wall_samples, 0.0),
        quantile(&wall_samples, 1.0),
    );
    for name in phase_names {
        let values = batch_records
            .iter()
            .map(|(_, phases)| {
                phases.get(name).copied().unwrap_or(0) as f64 / repetitions as f64 / 1e3
            })
            .collect::<Vec<_>>();
        eprintln!(
            "PLAN14_ANALYSIS_PHASE_SUMMARY T{lmax} m{mmax} q{thread_count} phase={name:?} p10_us={:.3} median_us={:.3} p90_us={:.3}",
            quantile(&values, 0.1),
            quantile(&values, 0.5),
            quantile(&values, 0.9),
        );
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "M5 current-source synthesis phase profile; run this diagnostic in release mode"]
fn m5_synthesis_phase_profile() {
    for (spin, lmax, thread_count) in [
        (0, 720, 1),
        (0, 720, 8),
        (0, 1023, 1),
        (0, 1023, 8),
        (0, 1279, 1),
        (0, 1279, 8),
        (0, 2047, 1),
        (0, 2047, 8),
        (1, 720, 1),
        (1, 720, 8),
        (1, 1023, 1),
        (1, 1023, 8),
        (1, 1279, 1),
        (1, 1279, 8),
        (1, 2047, 1),
        (1, 2047, 8),
    ] {
        let ntheta = lmax + 1;
        let nphi = GridSpec::fft_friendly_nphi(lmax).unwrap();
        let plan = ShtPlan::new(
            GridSpec::new(GridKind::GaussLegendre, ntheta, nphi, 0.37).unwrap(),
            HarmonicSpec::new(lmax, lmax, spin).unwrap(),
        )
        .unwrap();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap();
        let mut workspace = pool.install(|| plan.workspace()).unwrap();
        let profile = workspace.observe_transform_phases();
        let sample_count = plan.sample_count;
        let mut first = vec![0.0; sample_count];
        let mut second = (spin != 0).then(|| vec![0.0; sample_count]);
        let mut e_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut b_alm = (spin != 0).then(|| vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()]);
        for m in 0..=lmax {
            for degree in m.max(spin)..=lmax {
                let phase = (degree * 31 + m * 17) as f64 * 0.001_37;
                let value = Complex64::new(
                    phase.sin() * 1e-4,
                    if m == 0 { 0.0 } else { phase.cos() * 1e-4 },
                );
                let index = plan.harmonics.index(degree, m).unwrap();
                e_alm[index] = value;
                if let Some(b_alm) = &mut b_alm {
                    b_alm[index] =
                        Complex64::new(value.im * 0.7, if m == 0 { 0.0 } else { -value.re * 0.7 });
                }
            }
        }

        let mut run = || {
            if spin == 0 {
                plan.synthesis_into(&e_alm, &mut first, &mut workspace)
                    .unwrap();
            } else {
                plan.synthesis_spin_into(
                    &e_alm,
                    b_alm.as_deref().unwrap(),
                    &mut first,
                    second.as_deref_mut().unwrap(),
                    &mut workspace,
                )
                .unwrap();
            }
        };
        pool.install(&mut run);

        for sample in 0..5 {
            profile
                .lock()
                .expect("transform phase profile is not poisoned")
                .clear();
            let started = std::time::Instant::now();
            pool.install(&mut run);
            let wall_ns = started.elapsed().as_nanos();
            let phases = profile
                .lock()
                .expect("transform phase profile is not poisoned")
                .clone();
            assert!(!phases.is_empty(), "no synthesis phases were recorded");
            let phase_ns = phases.iter().map(|phase| phase.elapsed_ns).sum::<u128>();
            eprintln!(
                "SYNTHESIS_PHASE_PROFILE spin={spin} T{lmax} grid={ntheta}x{nphi} q{thread_count} sample={} wall_ms={:.3} phase_sum_ms={:.3} unmeasured_ms={:.3}",
                sample + 1,
                wall_ns as f64 / 1e6,
                phase_ns as f64 / 1e6,
                (wall_ns.saturating_sub(phase_ns)) as f64 / 1e6,
            );
            for phase in phases {
                eprintln!(
                    "SYNTHESIS_PHASE spin={spin} T{lmax} grid={ntheta}x{nphi} q{thread_count} sample={} name={:?} elapsed_ms={:.3}",
                    sample + 1,
                    phase.phase,
                    phase.elapsed_ns as f64 / 1e6,
                );
            }
        }
    }
}

#[test]
fn scalar_synthesis_matches_independent_low_degree_harmonics() {
    let plan = make_plan(3, 2, 11, 9, 0.23);
    let alm = coefficients(plan.harmonics());
    let expected = analytic_map(&plan, &alm);
    let mut map = vec![0.0; plan.sample_count];
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut workspace, 1).unwrap();

    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
    for (&actual, &reference) in map.iter().zip(&expected) {
        assert_close(actual, reference, 3e-14);
    }
}

#[test]
fn scalar_t1_synthesis_keeps_extreme_finite_harmonic_values_finite() {
    let plan = make_plan(1, 0, 2, 1, 0.0);
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    alm[plan.harmonics().index(1, 0).unwrap()] = Complex64::new(1.1e308, 0.0);
    let original = alm.clone();
    let mut map = vec![0.0; plan.sample_count];
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut workspace, 1).unwrap();

    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();

    let expected = 1.1e308 / (4.0 * PI).sqrt();
    assert!(map.iter().all(|value| value.is_finite()), "map={map:?}");
    assert!((map[0] - expected).abs() / expected < 2e-14, "map={map:?}");
    assert!((map[1] + expected).abs() / expected < 2e-14, "map={map:?}");
    assert_eq!(
        alm, original,
        "fallback must preserve the input coefficients"
    );
}

#[test]
fn scalar_synthesis_compacts_mixed_high_order_cutoff_blocks() {
    let lmax = 255;
    let ntheta = lmax + 1;
    let nphi = 2 * lmax + 3;
    let plan = make_plan(lmax, lmax, ntheta, nphi, 0.37);
    let spectrum_len = plan.fft.spectrum_len();
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    for m in 0..=lmax {
        for l in m..=lmax {
            let index = plan.harmonics().index(l, m).unwrap();
            let real = ((l * 7 + m * 11) % 19 + 1) as f64 * 1e-5;
            let imaginary = if m == 0 {
                0.0
            } else {
                ((l * 13 + m * 5) % 23 + 1) as f64 * 1e-5
            };
            alm[index] = Complex64::new(real, imaginary);
        }
    }

    let mut compact_workspace = plan.workspace().unwrap();
    let mut reference_workspace = plan.workspace().unwrap();
    for (ring, &theta) in plan.grid.theta().iter().enumerate() {
        let (sin_theta, cos_theta) = crate::legendre::sin_cos_from_f64(theta);
        compact_workspace.theta_sin[ring] = sin_theta;
        compact_workspace.theta_cos[ring] = cos_theta;
        compact_workspace.ring_order_limits[ring] =
            <f64 as crate::legendre::ShtScalar>::polar_order_limit(lmax, 0, sin_theta, cos_theta);
        reference_workspace.theta_sin[ring] = sin_theta;
        reference_workspace.theta_cos[ring] = cos_theta;
        reference_workspace.ring_order_limits[ring] = lmax;
    }
    assert!(
        compact_workspace.ring_order_limits[..ntheta]
            .chunks(crate::legendre::LATITUDE_BLOCK_SIZE)
            .any(|block| block.iter().any(|&limit| limit < 245)
                && block.iter().any(|&limit| limit >= 245))
    );

    let mut compact_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
    let mut reference_spectra = vec![Complex64::new(0.0, 0.0); ntheta * spectrum_len];
    let arch = pulp::Arch::new();
    plan.synthesize_ring_group(
        &alm,
        nphi,
        spectrum_len,
        plan.grid.phi0(),
        0,
        &mut compact_spectra,
        &compact_workspace.theta_sin[..ntheta],
        &compact_workspace.theta_cos[..ntheta],
        &compact_workspace.ring_order_limits[..ntheta],
        arch,
        &mut compact_workspace.recurrence_workers[0],
    )
    .unwrap();
    plan.synthesize_ring_group(
        &alm,
        nphi,
        spectrum_len,
        plan.grid.phi0(),
        0,
        &mut reference_spectra,
        &reference_workspace.theta_sin[..ntheta],
        &reference_workspace.theta_cos[..ntheta],
        &reference_workspace.ring_order_limits[..ntheta],
        arch,
        &mut reference_workspace.recurrence_workers[0],
    )
    .unwrap();

    for (ring, &limit) in compact_workspace.ring_order_limits[..ntheta]
        .iter()
        .enumerate()
    {
        for m in 0..=lmax {
            let index = ring * spectrum_len + m;
            if m > limit {
                assert_eq!(compact_spectra[index], Complex64::new(0.0, 0.0));
            } else {
                let error = (compact_spectra[index] - reference_spectra[index]).norm();
                assert!(error <= 2e-11, "ring={ring}, m={m}, error={error:.3e}");
            }
        }
    }

    let boundary_ring = (0..ntheta)
        .filter(|&ring| compact_workspace.ring_order_limits[ring] >= 245)
        .min_by_key(|&ring| compact_workspace.ring_order_limits[ring] - 245)
        .unwrap();
    let mut boundary_workspace = plan.workspace().unwrap();
    let mut boundary_spectrum = vec![Complex64::new(0.0, 0.0); spectrum_len];
    plan.synthesize_ring_group(
        &alm,
        nphi,
        spectrum_len,
        plan.grid.phi0(),
        boundary_ring,
        &mut boundary_spectrum,
        &[compact_workspace.theta_sin[boundary_ring]],
        &[compact_workspace.theta_cos[boundary_ring]],
        &reference_workspace.ring_order_limits[..ntheta],
        arch,
        &mut boundary_workspace.recurrence_workers[0],
    )
    .unwrap();
    let actual = compact_spectra[boundary_ring * spectrum_len + 245];
    let expected = boundary_spectrum[245];
    let expected_norm = expected.norm();
    assert!(expected_norm > 0.0, "boundary reference must be nonzero");
    let relative_error = (actual - expected).norm() / expected_norm;
    assert!(
        relative_error <= 1e-11,
        "boundary ring={boundary_ring}, relative error={:.3e}",
        relative_error
    );
}

#[test]
fn scalar_t1023_transition_fallback_matches_scaled_reference_for_extreme_coefficients() {
    let lmax = 1023;
    let m = 250;
    let ring_start = 64;
    let ring_end = ring_start + crate::legendre::LATITUDE_BLOCK_SIZE;
    let ntheta = lmax + 1;
    let nphi = 2 * lmax + 3;
    let plan = make_plan(lmax, m, ntheta, nphi, 0.37);
    let spectrum_len = plan.fft.spectrum_len();
    let (theta_sin, theta_cos): (Vec<_>, Vec<_>) = plan.grid.theta()[ring_start..ring_end]
        .iter()
        .map(|theta| theta.sin_cos())
        .unzip();
    let ring_limits: Vec<_> = theta_sin
        .iter()
        .zip(&theta_cos)
        .map(|(&sin_theta, &cos_theta)| {
            <f64 as crate::legendre::ShtScalar>::polar_order_limit(lmax, 0, sin_theta, cos_theta)
        })
        .collect();
    assert!(theta_sin.iter().all(|&sin_theta| sin_theta >= 0.01));
    assert!(ring_limits.iter().all(|&limit| limit >= m));

    let mut probe_workspace = plan.workspace().unwrap();
    let recurrence = &mut probe_workspace.recurrence_workers[0];
    <f64 as crate::legendre::ShtScalar>::fill_epsilon(m, lmax, &mut recurrence.epsilon);
    assert!(
        <f64 as crate::legendre::ShtScalar>::fill_paired_scalar_factors(
            m,
            lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        )
    );
    let order_len = lmax - m + 1;
    let zero_coefficients = vec![Complex64::new(0.0, 0.0); order_len];
    let mut probe_spectra = vec![Complex64::new(0.0, 0.0); theta_sin.len() * spectrum_len];
    let fused = <f64 as crate::legendre::ShtScalar>::synthesize_paired_scalar_block_dispatched(
        pulp::Arch::new(),
        m,
        lmax,
        &theta_sin,
        &theta_cos,
        plan.sectoral_norms[m],
        &recurrence.epsilon,
        &recurrence.spin_alpha,
        &recurrence.spin_coefficient_a,
        &recurrence.spin_coefficient_b,
        &zero_coefficients,
        &ring_limits,
        spectrum_len,
        &mut probe_spectra,
        &mut recurrence.previous,
        &mut recurrence.current,
        &mut recurrence.next,
        &mut recurrence.exponents,
    )
    .unwrap();
    assert!(
        !fused,
        "the selected block must enter the continuation path"
    );

    let mut extreme_coefficients = zero_coefficients.clone();
    extreme_coefficients[1] = Complex64::new(f64::MAX, 0.0);
    let second_coefficients = zero_coefficients.clone();
    let continuation =
        <f64 as crate::legendre::ShtScalar>::try_synthesize_paired_scalar_transition_dispatched(
            pulp::Arch::new(),
            m,
            lmax,
            &theta_sin,
            &theta_cos,
            plan.sectoral_norms[m],
            &recurrence.epsilon,
            &recurrence.spin_alpha,
            &recurrence.spin_coefficient_a,
            &recurrence.spin_coefficient_b,
            &extreme_coefficients,
            &second_coefficients,
            &mut recurrence.previous,
            &mut recurrence.current,
            &mut recurrence.next,
            &mut recurrence.exponents,
            &mut recurrence.analysis_same_real,
            &mut recurrence.analysis_same_imag,
            &mut recurrence.analysis_opposite_real,
            &mut recurrence.analysis_opposite_imag,
        )
        .unwrap();
    assert!(
        !continuation,
        "overflow-prone transformed coefficients use scaling"
    );

    for coefficient in [f64::from_bits(1), f64::MAX] {
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let coefficient_start = plan.harmonics.index(m, m).unwrap();
        alm[coefficient_start + 1] = Complex64::new(coefficient, 0.0);
        let mut actual_workspace = plan.workspace().unwrap();
        let mut actual_spectra = vec![Complex64::new(0.0, 0.0); theta_sin.len() * spectrum_len];
        plan.synthesize_ring_group(
            &alm,
            nphi,
            spectrum_len,
            plan.grid.phi0(),
            0,
            &mut actual_spectra,
            &theta_sin,
            &theta_cos,
            &ring_limits,
            pulp::Arch::new(),
            &mut actual_workspace.recurrence_workers[0],
        )
        .unwrap();

        let mut reference_workspace = plan.workspace().unwrap();
        let recurrence = &mut reference_workspace.recurrence_workers[0];
        <f64 as crate::legendre::ShtScalar>::fill_epsilon(m, lmax, &mut recurrence.epsilon);
        let mut unscaled_reference = vec![Complex64::new(0.0, 0.0); theta_sin.len()];
        <f64 as crate::legendre::ShtScalar>::for_each_y_lm_dispatched(
            pulp::Arch::new(),
            m,
            lmax,
            &theta_sin,
            &theta_cos,
            plan.sectoral_norms[m],
            &recurrence.epsilon,
            &mut recurrence.previous,
            &mut recurrence.current,
            &mut recurrence.next,
            &mut recurrence.exponents,
            |l, ring_offset, current, exponents| {
                let value = alm[coefficient_start + (l - m)];
                for lane in 0..current.len() {
                    unscaled_reference[ring_offset + lane] +=
                        <f64 as crate::legendre::ShtScalar>::multiply_complex_scaled(
                            value,
                            current[lane],
                            exponents[lane],
                        );
                }
                Ok(())
            },
        )
        .unwrap();

        let phase = crate::legendre::longitude_phase::<f64>(m, plan.grid.phi0(), false);
        for lane in 0..theta_sin.len() {
            let expected = unscaled_reference[lane] * phase;
            let actual = actual_spectra[lane * spectrum_len + m];
            if coefficient == f64::from_bits(1) {
                assert_eq!(actual, expected, "subnormal input at lane {lane}");
            } else {
                let relative_error = (actual - expected).norm() / expected.norm().max(1.0);
                assert!(
                    actual.re.is_finite() && actual.im.is_finite() && relative_error <= 2e-13,
                    "lane={lane}, actual={actual:?}, expected={expected:?}, relative_error={relative_error:.3e}"
                );
            }
        }
    }
}

#[test]
fn scalar_adjoint_operators_match_ducc_and_inner_products() {
    // Reference: ducc0 0.41.0, GL 3x7, phi0=0.37, mstart for packed Rust layout.
    let plan = make_plan(2, 2, 3, 7, 0.37);
    let alm = coefficients(plan.harmonics());
    let input = arbitrary_map(&plan);
    let mut workspace = plan.workspace().unwrap();

    let mut synthesized = vec![0.0; plan.sample_count];
    plan.synthesis_into(&alm, &mut synthesized, &mut workspace)
        .unwrap();
    let mut synthesis_adjoint = vec![Complex64::new(0.0, 0.0); alm.len()];
    plan.adjoint_synthesis_into(&input, &mut synthesis_adjoint, &mut workspace)
        .unwrap();
    for (&actual, expected) in synthesis_adjoint.iter().zip([
        Complex64::new(4.648_960_499_291_433, 0.0),
        Complex64::new(-0.594_467_893_478_704_9, 0.0),
        Complex64::new(1.011_778_214_432_334_2, 0.0),
        Complex64::new(0.378_992_673_732_543_3, -0.607_778_178_500_533),
        Complex64::new(-0.033_014_359_575_363_2, -0.149_134_620_282_385_1),
        Complex64::new(-0.105_833_761_840_617_6, 0.290_989_521_058_551_9),
    ]) {
        assert!((actual - expected).norm() < 2e-14);
    }
    assert_close(
        map_inner_product(&synthesized, &input),
        harmonic_inner_product(plan.harmonics(), &alm, &synthesis_adjoint),
        2e-13,
    );

    let mut analyzed = vec![Complex64::new(0.0, 0.0); alm.len()];
    plan.analysis_into(&input, &mut analyzed, &mut workspace)
        .unwrap();
    let mut analysis_adjoint = vec![0.0; plan.sample_count];
    plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
        .unwrap();
    for (index, expected) in [
        (0, 0.180_256_485_957_303_8),
        (1, 0.226_281_635_669_034_06),
        (3, 0.095_259_119_859_373_66),
        (7, 0.281_486_856_056_618_55),
        (10, 0.368_079_725_958_397_8),
        (20, -0.068_996_291_908_175_74),
    ] {
        assert_close(analysis_adjoint[index], expected, 2e-14);
    }
    assert_close(
        harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
        map_inner_product(&input, &analysis_adjoint),
        2e-13,
    );
}

#[test]
fn scalar_synthesis_adjoint_accepts_even_longitude_nyquist_order() {
    // Reference: ducc0 0.41.0, GL 3x4, phi0=0.31, one thread.
    let plan = make_plan(2, 2, 3, 4, 0.31);
    let alm = coefficients(plan.harmonics());
    let input = arbitrary_map(&plan);
    let mut workspace = plan.workspace().unwrap();
    let mut synthesized = vec![0.0; plan.sample_count];
    plan.synthesis_into(&alm, &mut synthesized, &mut workspace)
        .unwrap();
    let mut adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.adjoint_synthesis_into(&input, &mut adjoint, &mut workspace)
        .unwrap();

    for (&actual, expected) in adjoint.iter().zip([
        Complex64::new(2.192_070_200_391_801, 0.0),
        Complex64::new(-0.452_936_406_865_718_34, 0.0),
        Complex64::new(0.478_150_955_162_161_2, 0.0),
        Complex64::new(0.219_369_995_115_942_18, -0.321_778_433_740_079_75),
        Complex64::new(0.004_395_968_203_849_482_5, -0.028_237_897_221_759_366),
        Complex64::new(-0.195_608_850_028_266_7, 0.139_646_919_817_436_15),
    ]) {
        assert!((actual - expected).norm() < 3e-14);
    }
    assert!(adjoint[plan.harmonics().index(2, 2).unwrap()].im.abs() > 0.1);
    assert_close(
        map_inner_product(&synthesized, &input),
        harmonic_inner_product(plan.harmonics(), &alm, &adjoint),
        2e-13,
    );
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain DUCC's printed reference digits."
)]
fn scalar_synthesis_matches_ducc_0410_reference() {
    // Reference: ducc0 0.41.0, GL, ntheta=3, nphi=5, phi0=0.37, nthreads=1.
    let plan = make_plan(2, 2, 3, 5, 0.37);
    let alm = coefficients(plan.harmonics());
    let expected = [
        3.61477755632444442e-1,
        3.41202806618260746e-1,
        1.67425809619942578e-1,
        1.10137639969044035e-1,
        -2.45225492833134107e-1,
        3.52800193384860350e-1,
        2.59893066238583326e-1,
        3.24314581387524226e-1,
        5.31628824359529850e-1,
        -6.39000676788184396e-1,
        1.48463124212611458e-1,
        2.82761224552306900e-1,
        5.53534560205396930e-1,
        6.16420556489261751e-1,
        -1.09221189846971128e-1,
    ];
    let mut map = vec![0.0; plan.sample_count];
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut workspace, 1).unwrap();

    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
    for (&actual, &reference) in map.iter().zip(&expected) {
        assert_close(actual, reference, 4e-14);
    }
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain DUCC's printed reference digits."
)]
fn t255_sectoral_synthesis_matches_ducc_0410_samples() {
    // Reference: ducc0 0.41.0, GL, T255, phi0=0.37, nthreads=1.
    let plan = make_plan(255, 255, 256, 513, 0.37);
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    alm[plan.harmonics().index(255, 255).unwrap()] = Complex64::new(1.0, 0.0);
    let mut map = vec![0.0; plan.sample_count];
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut workspace, 1).unwrap();

    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
    for (ring, longitude, reference) in [
        (0, 0, 0.0),
        (63, 1, 3.06179051888323780e-39),
        (127, 0, -2.37277732245063966),
        (127, 1, 2.37684826227152968),
        (128, 256, 2.65180186641488569e-1),
        (191, 512, 6.86455637290427111e-38),
        (255, 0, 0.0),
    ] {
        assert_reference_close(map[ring * 513 + longitude], reference);
    }
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain DUCC's printed reference digits."
)]
fn t255_mixed_synthesis_matches_ducc_0410_samples() {
    // Reference: ducc0 0.41.0, GL, T255, phi0=0.37, nthreads=1.
    let plan = make_plan(255, 255, 256, 513, 0.37);
    assert!(plan.scalar_legendre_table.set(None).is_ok());
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    for m in 0..=255 {
        for l in m..=255 {
            let re = ((l * 7 + m * 11) % 19) as f64 - 9.0;
            let im = if m == 0 {
                0.0
            } else {
                ((l * 13 + m * 5) % 23) as f64 - 11.0
            };
            let index = plan.harmonics().index(l, m).unwrap();
            alm[index] = Complex64::new(re * 1e-5, im * 1e-5);
        }
    }
    let mut map = vec![0.0; plan.sample_count];
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut workspace, 1).unwrap();
    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
    let reference_spectra = scalar_ring_group_spectra(&plan, &alm);
    assert!(relative_l2_complex(&workspace.spectra, &reference_spectra) < 3e-13);
    for (ring, longitude, reference) in [
        (0, 0, -7.73911562867147081e-5),
        (63, 1, 7.59821699150580896e-4),
        (127, 0, -4.01554944939784119e-4),
        (127, 1, 4.51924072746787359e-4),
        (128, 256, -2.10693373424372839e-3),
        (191, 512, -2.32708782503306599e-4),
        (217, 283, -5.04792555199083300e-2),
        (255, 512, -8.23852167514639881e-4),
    ] {
        assert_close(map[ring * 513 + longitude], reference, 2e-14);
    }
}

#[test]
fn t255_reduced_order_synthesis_matches_closed_form_on_odd_and_even_longitudes() {
    for nphi in [5, 6] {
        let plan = make_plan(255, 2, 256, nphi, 0.47);
        assert!(plan.scalar_legendre_table.set(None).is_ok());
        let alm = coefficients(plan.harmonics());
        let expected = analytic_map(&plan, &alm);
        let mut actual = vec![0.0; plan.sample_count];
        let mut workspace = plan.workspace().unwrap();
        plan.prepare_workspace(&mut workspace, 1).unwrap();

        plan.synthesis_into(&alm, &mut actual, &mut workspace)
            .unwrap();
        assert!(
            relative_l2_real(&actual, &expected) <= 2e-13,
            "nphi={nphi}, relative L2 error={:.3e}",
            relative_l2_real(&actual, &expected)
        );
    }
}

#[test]
fn scalar_ordered_batches_more_than_256_active_reflected_rings() {
    let plan = make_plan(510, 0, 601, 5, 0.37);
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    for (degree, value) in [(0, 0.7), (1, -0.2), (2, 0.1)] {
        alm[plan.harmonics().index(degree, 0).unwrap()] = Complex64::new(value, 0.0);
    }

    let expected = analytic_map(&plan, &alm);
    let mut candidate_map = vec![0.0; plan.sample_count];
    let mut candidate_workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut candidate_workspace, 1).unwrap();
    plan.synthesis_into(&alm, &mut candidate_map, &mut candidate_workspace)
        .unwrap();
    assert!(relative_l2_real(&candidate_map, &expected) < 2e-13);

    alm[plan.harmonics().index(510, 0).unwrap()] = Complex64::new(1e-4, 0.0);
    plan.synthesis_into(&alm, &mut candidate_map, &mut candidate_workspace)
        .unwrap();
    let reference_spectra = scalar_ring_group_spectra(&plan, &alm);
    assert!(relative_l2_complex(&candidate_workspace.spectra, &reference_spectra) < 5e-13);
}

#[test]
fn scalar_ordered_falls_back_safely_for_subnormal_and_extreme_coefficients() {
    let plan = make_plan(255, 0, 256, 5, 0.37);
    assert!(plan.scalar_legendre_table.set(None).is_ok());

    for (monopole, quadrupole) in [(f64::from_bits(1), 0.0), (0.75 * f64::MAX, 0.75 * f64::MAX)] {
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        alm[plan.harmonics().index(0, 0).unwrap()] = Complex64::new(monopole, 0.0);
        alm[plan.harmonics().index(2, 0).unwrap()] = Complex64::new(quadrupole, 0.0);
        let mut map = vec![0.0; plan.sample_count];
        let mut workspace = plan.workspace().unwrap();
        plan.prepare_workspace(&mut workspace, 1).unwrap();
        plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();

        assert!(map.iter().all(|value| value.is_finite()));
        if !monopole.is_subnormal() {
            let coefficient_scale = monopole.abs().max(quadrupole.abs());
            let scaled_alm = alm
                .iter()
                .map(|coefficient| *coefficient / coefficient_scale)
                .collect::<Vec<_>>();
            let expected_map = analytic_map(&plan, &scaled_alm);
            let scaled_error = map
                .iter()
                .zip(expected_map)
                .map(|(&actual, expected)| (actual / coefficient_scale - expected).abs())
                .fold(0.0_f64, f64::max);
            assert!(
                scaled_error < 5e-13,
                "scaled analytic error={scaled_error:.3e}"
            );
        }
        let reference_spectra = scalar_ring_group_spectra(&plan, &alm);
        if monopole.is_subnormal() {
            assert_eq!(workspace.spectra, reference_spectra);
        } else {
            assert!(relative_l2_complex(&workspace.spectra, &reference_spectra) < 3e-13);
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_ordered_parallel_orders_match_serial_and_ring_group_synthesis() {
    let plan = make_plan(300, 150, 301, 301, 0.37);
    assert!(plan.scalar_legendre_table.set(None).is_ok());
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    for m in 0..=plan.harmonics().mmax() {
        for l in m..=plan.harmonics().lmax() {
            let re = ((l * 7 + m * 11) % 19) as f64 - 9.0;
            let im = if m == 0 {
                0.0
            } else {
                ((l * 13 + m * 5) % 23) as f64 - 11.0
            };
            alm[plan.harmonics().index(l, m).unwrap()] = Complex64::new(re * 1e-6, im * 1e-6);
        }
    }

    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut serial_workspace = one_thread.install(|| plan.workspace()).unwrap();
    one_thread
        .install(|| plan.prepare_workspace(&mut serial_workspace, 1))
        .unwrap();
    let mut parallel_workspace = eight_threads.install(|| plan.workspace()).unwrap();
    eight_threads
        .install(|| plan.prepare_workspace(&mut parallel_workspace, 8))
        .unwrap();
    prepare_scalar_latitudes(&plan, &mut parallel_workspace);
    let worker_probe = parallel_workspace.observe_active_workers();

    eight_threads.install(|| {
        parallel_workspace.spectra.fill(Complex64::new(0.0, 0.0));
        assert!(plan.synthesize_scalar_ordered_parallel(
            &alm,
            pulp::Arch::new(),
            &mut parallel_workspace,
        ));
    });
    assert!(worker_probe.lock().unwrap().len() > 1);
    let reference_spectra = scalar_ring_group_spectra(&plan, &alm);
    let reference_error = relative_l2_complex(&parallel_workspace.spectra, &reference_spectra);
    assert!(
        reference_error < 4e-13,
        "reference error={reference_error:.3e}"
    );

    let mut serial_map = vec![0.0; plan.sample_count];
    let mut parallel_map = vec![0.0; plan.sample_count];
    one_thread
        .install(|| plan.synthesis_into(&alm, &mut serial_map, &mut serial_workspace))
        .unwrap();
    eight_threads
        .install(|| plan.synthesis_into(&alm, &mut parallel_map, &mut parallel_workspace))
        .unwrap();
    assert!(relative_l2_real(&parallel_map, &serial_map) < 4e-13);

    alm.fill(Complex64::new(0.0, 0.0));
    alm[plan.harmonics().index(100, 100).unwrap()] = Complex64::new(f64::from_bits(1), 0.0);
    parallel_workspace.spectra.fill(Complex64::new(3.0, -2.0));
    eight_threads.install(|| {
        assert!(!plan.synthesize_scalar_ordered_parallel(
            &alm,
            pulp::Arch::new(),
            &mut parallel_workspace,
        ));
    });
    assert!(
        parallel_workspace
            .spectra
            .iter()
            .all(|&value| value == Complex64::new(0.0, 0.0)),
        "failed direct-order execution must clear partial spectral output before fallback"
    );

    one_thread
        .install(|| plan.synthesis_into(&alm, &mut serial_map, &mut serial_workspace))
        .unwrap();
    eight_threads
        .install(|| plan.synthesis_into(&alm, &mut parallel_map, &mut parallel_workspace))
        .unwrap();
    assert_eq!(parallel_map, serial_map);
    assert_eq!(
        parallel_workspace.spectra,
        scalar_ring_group_spectra(&plan, &alm)
    );
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_ordered_parallel_orders_above_t511_match_serial_and_ring_group() {
    let plan = make_plan(511, 128, 512, 257, 0.23);
    assert!(plan.scalar_legendre_table.set(None).is_ok());
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    for m in 0..=plan.harmonics().mmax() {
        for l in m..=plan.harmonics().lmax() {
            let re = ((l * 3 + m * 7) % 29) as f64 - 14.0;
            let im = if m == 0 {
                0.0
            } else {
                ((l * 11 + m * 13) % 31) as f64 - 15.0
            };
            alm[plan.harmonics().index(l, m).unwrap()] = Complex64::new(re * 1e-7, im * 1e-7);
        }
    }

    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut serial_workspace = one_thread.install(|| plan.workspace()).unwrap();
    one_thread
        .install(|| plan.prepare_workspace(&mut serial_workspace, 1))
        .unwrap();
    let mut parallel_workspace = eight_threads.install(|| plan.workspace()).unwrap();
    eight_threads
        .install(|| plan.prepare_workspace(&mut parallel_workspace, 8))
        .unwrap();
    let worker_probe = parallel_workspace.observe_active_workers();
    let mut serial_map = vec![0.0; plan.sample_count];
    let mut parallel_map = vec![0.0; plan.sample_count];

    one_thread
        .install(|| plan.synthesis_into(&alm, &mut serial_map, &mut serial_workspace))
        .unwrap();
    eight_threads
        .install(|| plan.synthesis_into(&alm, &mut parallel_map, &mut parallel_workspace))
        .unwrap();

    assert!(worker_probe.lock().unwrap().len() > 1);
    assert!(relative_l2_real(&parallel_map, &serial_map) < 8e-13);
    let reference_spectra = scalar_ring_group_spectra(&plan, &alm);
    let reference_error = relative_l2_complex(&parallel_workspace.spectra, &reference_spectra);
    assert!(
        reference_error < 8e-13,
        "reference error={reference_error:.3e}"
    );
}

#[test]
fn precomputed_scalar_gl_matches_recurrence_for_all_four_operators() {
    for (lmax, mmax, ntheta, nphi) in [
        (63, 63, 64, 128),
        (63, 63, 96, 192),
        (126, 126, 127, 255),
        (127, 127, 128, 255),
        (127, 127, 129, 257),
    ] {
        let table_plan = make_plan(lmax, mmax, ntheta, nphi, 0.37);
        assert!(table_plan.get_scalar_legendre_table().is_some());
        let recurrence_plan = make_plan(lmax, mmax, ntheta, nphi, 0.37);
        assert!(recurrence_plan.scalar_legendre_table.set(None).is_ok());
        #[cfg(feature = "parallel")]
        let table_threads = 2;
        #[cfg(feature = "parallel")]
        let table_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(table_threads)
            .build()
            .unwrap();
        let mut input_alm = vec![Complex64::new(0.0, 0.0); table_plan.harmonics().nalm()];
        for m in 0..=mmax {
            for l in m..=lmax {
                let im = if m == 0 {
                    0.0
                } else {
                    ((l * 13 + m * 5) % 23) as f64 - 11.0
                };
                input_alm[table_plan.harmonics().index(l, m).unwrap()] =
                    Complex64::new(((l * 7 + m * 11) % 19) as f64 * 1e-5, im * 1e-5);
            }
        }
        let input_map = arbitrary_map(&table_plan);
        let mut recurrence_workspace = recurrence_plan.workspace().unwrap();
        #[cfg(feature = "parallel")]
        let mut table_workspace = table_pool.install(|| table_plan.workspace()).unwrap();
        #[cfg(not(feature = "parallel"))]
        let mut table_workspace = table_plan.workspace().unwrap();
        let mut recurrence_map = vec![0.0; table_plan.sample_count];
        let mut recurrence_alm = vec![Complex64::new(0.0, 0.0); input_alm.len()];
        let mut recurrence_s_adjoint = recurrence_alm.clone();
        let mut recurrence_a_adjoint = vec![0.0; table_plan.sample_count];

        recurrence_plan
            .synthesis_into(&input_alm, &mut recurrence_map, &mut recurrence_workspace)
            .unwrap();
        recurrence_plan
            .analysis_into(&input_map, &mut recurrence_alm, &mut recurrence_workspace)
            .unwrap();
        recurrence_plan
            .adjoint_synthesis_into(
                &input_map,
                &mut recurrence_s_adjoint,
                &mut recurrence_workspace,
            )
            .unwrap();
        recurrence_plan
            .adjoint_analysis_into(
                &input_alm,
                &mut recurrence_a_adjoint,
                &mut recurrence_workspace,
            )
            .unwrap();

        let mut table_map = vec![0.0; table_plan.sample_count];
        let mut table_alm = vec![Complex64::new(0.0, 0.0); input_alm.len()];
        let mut table_s_adjoint = table_alm.clone();
        let mut table_a_adjoint = vec![0.0; table_plan.sample_count];
        let mut run_table_operators = || {
            #[cfg(feature = "parallel")]
            {
                assert_eq!(
                    table_plan.scalar_legendre_strategy(LegendreOperation::Synthesis),
                    if lmax <= 63 {
                        LegendreStrategy::CompactTable
                    } else {
                        LegendreStrategy::Recurrence
                    }
                );
                assert_eq!(
                    table_plan.scalar_legendre_strategy(LegendreOperation::Analysis),
                    LegendreStrategy::CompactTable
                );
            }
            table_plan
                .synthesis_into(&input_alm, &mut table_map, &mut table_workspace)
                .unwrap();
            table_plan
                .analysis_into(&input_map, &mut table_alm, &mut table_workspace)
                .unwrap();
            table_plan
                .adjoint_synthesis_into(&input_map, &mut table_s_adjoint, &mut table_workspace)
                .unwrap();
            table_plan
                .adjoint_analysis_into(&input_alm, &mut table_a_adjoint, &mut table_workspace)
                .unwrap();
        };
        #[cfg(feature = "parallel")]
        table_pool.install(&mut run_table_operators);
        #[cfg(not(feature = "parallel"))]
        run_table_operators();

        assert!(relative_l2_real(&table_map, &recurrence_map) < 2e-12);
        assert!(relative_l2_complex(&table_alm, &recurrence_alm) < 2e-12);
        assert!(relative_l2_complex(&table_s_adjoint, &recurrence_s_adjoint) < 2e-12);
        assert!(relative_l2_real(&table_a_adjoint, &recurrence_a_adjoint) < 2e-12);
        if lmax == 72 || ntheta > lmax + 1 {
            assert_close(
                map_inner_product(&table_map, &input_map),
                harmonic_inner_product(&table_plan.harmonics, &input_alm, &table_s_adjoint),
                2e-12,
            );
            assert_close(
                harmonic_inner_product(&table_plan.harmonics, &table_alm, &input_alm),
                map_inner_product(&input_map, &table_a_adjoint),
                2e-12,
            );
        }
    }
}

#[test]
fn precomputed_scalar_synthesis_applies_asymmetric_mirrored_cutoffs() {
    let plan = make_plan(127, 127, 128, 257, 0.37);
    let table = plan
        .get_scalar_legendre_table()
        .expect("the T127 GL scalar table is eligible");
    let north_ring = 20;
    let south_ring = plan.grid.ntheta() - 1 - north_ring;
    let m = 64;
    let l = m + 1;
    let basis = table.degree_lanes(m, l - m)[north_ring];
    assert_ne!(basis, 0.0);

    let coefficient = Complex64::new(0.4, -0.7);
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    alm[plan.harmonics().index(l, m).unwrap()] = coefficient;
    let mut ring_limits = vec![plan.harmonics().lmax(); plan.grid.ntheta()];
    ring_limits[north_ring] = m - 1;
    ring_limits[south_ring] = m;
    let mut order_row = vec![Complex64::new(0.0, 0.0); plan.grid.ntheta()];

    assert!(accumulate_precomputed_synthesis_order(
        &plan,
        table,
        &alm,
        &ring_limits,
        m,
        &mut order_row,
    ));

    let phase = Complex64::from_polar(1.0, m as f64 * plan.grid.phi0());
    assert_eq!(order_row[north_ring], Complex64::new(0.0, 0.0));
    let expected_south = coefficient * (-basis) * phase * plan.grid.nphi() as f64;
    assert!((order_row[south_ring] - expected_south).norm() < 1e-13);
}

#[test]
fn precomputed_scalar_table_supports_spin1_scalar_degree_extension() {
    let grid = GridSpec::new(GridKind::GaussLegendre, 128, 257, 0.37).unwrap();
    let harmonics = HarmonicSpec::new(127, 127, 1).unwrap();
    let plan: ShtPlan<f64> = ShtPlan::new(grid, harmonics).unwrap();
    assert!(plan.scalar_legendre_table.get().is_none());
    assert!(plan.spin1_scalar_legendre_table.get().is_none());
    let table = crate::legendre::ScalarLegendreTable::build_if_beneficial(
        &plan.grid,
        &plan.harmonics,
        &plan.sectoral_norms,
    )
    .unwrap();
    assert_eq!(table.north_ring_count(), 64);
    assert_eq!(table.degree_lanes(0, 128).len(), 64);
    assert!(
        table
            .degree_lanes(0, 128)
            .iter()
            .all(|value| value.is_finite())
    );

    let high_band_grid = GridSpec::new(GridKind::GaussLegendre, 256, 513, 0.37).unwrap();
    let high_band_harmonics = HarmonicSpec::new(255, 255, 1).unwrap();
    let high_band_plan: ShtPlan<f64> = ShtPlan::new(high_band_grid, high_band_harmonics).unwrap();
    let high_band_table = crate::legendre::ScalarLegendreTable::build_if_beneficial(
        &high_band_plan.grid,
        &high_band_plan.harmonics,
        &high_band_plan.sectoral_norms,
    )
    .expect("the bounded T255 spin-1 scalar table is eligible");
    assert_eq!(high_band_table.degree_lanes(0, 256).len(), 128);
    assert!(
        high_band_table
            .degree_lanes(0, 256)
            .iter()
            .all(|&value| value == 0.0)
    );
}

#[test]
fn precomputed_scalar_table_respects_bandlimit_and_grid_bounds() {
    assert!(
        make_plan(62, 62, 63, 127, 0.37)
            .get_scalar_legendre_table()
            .is_none()
    );
    assert!(
        make_plan(63, 63, 64, 129, 0.37)
            .get_scalar_legendre_table()
            .is_some()
    );
    assert!(
        make_plan(63, 63, 129, 129, 0.37)
            .get_scalar_legendre_table()
            .is_none(),
        "unmeasured latitude oversampling must stay on recurrence"
    );
    assert!(
        make_plan(72, 72, 73, 147, 0.37)
            .get_scalar_legendre_table()
            .is_some()
    );
    assert!(
        make_plan(127, 127, 128, 257, 0.37)
            .get_scalar_legendre_table()
            .is_some()
    );
    assert!(
        make_plan(255, 255, 256, 513, 0.37)
            .get_scalar_legendre_table()
            .is_some()
    );
    assert!(
        make_plan(360, 360, 361, 723, 0.37)
            .get_scalar_legendre_table()
            .is_none()
    );
    assert!(
        make_plan(720, 720, 721, 1443, 0.37)
            .get_scalar_legendre_table()
            .is_none()
    );

    let cc_plan = make_cc_plan(127, 127, 129, 257, 0.37);
    assert!(cc_plan.get_scalar_legendre_table().is_none());
    let f1_plan = make_f1_plan(127, 127, 128, 257, 0.37);
    assert!(f1_plan.get_scalar_legendre_table().is_none());

    let f32_grid = GridSpec::new(GridKind::GaussLegendre, 128, 257, 0.37).unwrap();
    let f32_harmonics = HarmonicSpec::scalar(127, 127).unwrap();
    let f32_plan: ShtPlan<f32> = ShtPlan::new(f32_grid, f32_harmonics).unwrap();
    assert!(f32_plan.get_scalar_legendre_table().is_none());
}

#[test]
fn precomputed_scalar_table_uses_scaled_recurrence_for_extreme_inputs() {
    let plan = make_plan(255, 255, 256, 513, 0.37);
    assert!(plan.get_scalar_legendre_table().is_some());
    let recurrence_plan = make_plan(255, 255, 256, 513, 0.37);
    assert!(recurrence_plan.scalar_legendre_table.set(None).is_ok());
    let mut workspace = plan.workspace().unwrap();
    let mut recurrence_workspace = recurrence_plan.workspace().unwrap();
    let mut input_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    input_alm[plan.harmonics().index(220, 200).unwrap()] = Complex64::new(1.0e300, -2.0e299);
    let mut recurrence_map = vec![0.0; plan.sample_count];
    recurrence_plan
        .synthesis_into(&input_alm, &mut recurrence_map, &mut recurrence_workspace)
        .unwrap();
    let mut fallback_map = vec![0.0; plan.sample_count];
    plan.synthesis_into(&input_alm, &mut fallback_map, &mut workspace)
        .unwrap();
    assert_eq!(fallback_map, recurrence_map);

    let ring = plan.grid.ntheta() / 2;
    let order = 200;
    let mut input_map = vec![0.0; plan.sample_count];
    for longitude in 0..plan.grid.nphi() {
        let phi = plan.grid.longitude(longitude).unwrap();
        input_map[ring * plan.grid.nphi() + longitude] = 1.0e300 * (order as f64 * phi).cos();
    }
    let mut recurrence_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    recurrence_plan
        .analysis_into(&input_map, &mut recurrence_alm, &mut recurrence_workspace)
        .unwrap();
    let mut fallback_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.analysis_into(&input_map, &mut fallback_alm, &mut workspace)
        .unwrap();
    assert_eq!(fallback_alm, recurrence_alm);
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_table_initializes_safely_for_concurrent_cross_pool_operations() {
    let plan = std::sync::Arc::new(make_plan(127, 127, 128, 257, 0.37));
    assert!(plan.scalar_legendre_table.get().is_none());
    let input_map = arbitrary_map(&plan);
    let pool_one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let pool_four = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let mut workspace_one = pool_one.install(|| plan.workspace()).unwrap();
    let mut workspace_four = pool_four.install(|| plan.workspace()).unwrap();
    let mut alm_one = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut alm_four = alm_one.clone();

    std::thread::scope(|scope| {
        let plan_one = &plan;
        let map_one = &input_map;
        let handle_one = scope.spawn(|| {
            pool_one.install(|| plan_one.analysis_into(map_one, &mut alm_one, &mut workspace_one))
        });
        let plan_four = &plan;
        let map_four = &input_map;
        let handle_four = scope.spawn(|| {
            pool_four
                .install(|| plan_four.analysis_into(map_four, &mut alm_four, &mut workspace_four))
        });
        handle_one.join().unwrap().unwrap();
        handle_four.join().unwrap().unwrap();
    });

    assert_eq!(alm_one, alm_four);
    assert!(
        plan.scalar_legendre_table
            .get()
            .is_some_and(Option::is_some)
    );
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_strategy_selects_against_the_executing_pool() {
    let small_plan = make_plan(72, 72, 73, 147, 0.37);
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    assert_eq!(
        one_thread
            .install(|| { small_plan.scalar_legendre_strategy(LegendreOperation::Synthesis) }),
        LegendreStrategy::Recurrence
    );
    let mut one_thread_workspace = one_thread.install(|| small_plan.workspace()).unwrap();
    let alm = vec![Complex64::new(0.0, 0.0); small_plan.harmonics().nalm()];
    let mut map = vec![0.0; small_plan.sample_count];
    one_thread
        .install(|| small_plan.synthesis_into(&alm, &mut map, &mut one_thread_workspace))
        .unwrap();
    assert!(small_plan.scalar_legendre_table.get().is_none());

    assert_eq!(
        eight_threads
            .install(|| { small_plan.scalar_legendre_strategy(LegendreOperation::Synthesis) }),
        LegendreStrategy::Recurrence
    );
    let mut eight_thread_workspace = eight_threads.install(|| small_plan.workspace()).unwrap();
    eight_threads
        .install(|| small_plan.synthesis_into(&alm, &mut map, &mut eight_thread_workspace))
        .unwrap();
    assert!(small_plan.scalar_legendre_table.get().is_none());

    let t86_plan = make_plan(86, 86, 87, 175, 0.37);
    assert_eq!(
        one_thread.install(|| t86_plan.scalar_legendre_strategy(LegendreOperation::Synthesis)),
        LegendreStrategy::Recurrence
    );
    let t127_plan = make_plan(127, 127, 128, 257, 0.37);
    assert_eq!(
        one_thread.install(|| t127_plan.scalar_legendre_strategy(LegendreOperation::Synthesis)),
        LegendreStrategy::Recurrence
    );
    assert_eq!(
        eight_threads.install(|| t127_plan.scalar_legendre_strategy(LegendreOperation::Synthesis)),
        LegendreStrategy::Recurrence
    );

    let large_plan = make_plan(255, 255, 256, 513, 0.37);
    assert_eq!(
        one_thread.install(|| { large_plan.scalar_legendre_strategy(LegendreOperation::Analysis) }),
        LegendreStrategy::Recurrence
    );
    let input_map = arbitrary_map(&large_plan);
    let mut coefficients = vec![Complex64::new(0.0, 0.0); large_plan.harmonics().nalm()];
    let mut large_workspace = one_thread.install(|| large_plan.workspace()).unwrap();
    one_thread
        .install(|| large_plan.analysis_into(&input_map, &mut coefficients, &mut large_workspace))
        .unwrap();
    assert!(large_plan.scalar_legendre_table.get().is_none());

    let sixteen_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build()
        .unwrap();
    assert_eq!(
        sixteen_threads
            .install(|| large_plan.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_t64_dispatch_keeps_analysis_tables_and_uses_synthesis_recurrence() {
    use LegendreOperation::{Analysis, Synthesis};
    use LegendreStrategy::{CompactTable, Recurrence};

    let pools = [
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap(),
        rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap(),
    ];
    let operations = [
        (63, Synthesis, [Recurrence, CompactTable]),
        (63, Analysis, [CompactTable, Recurrence]),
        (64, Synthesis, [Recurrence, Recurrence]),
        (64, Analysis, [CompactTable, CompactTable]),
    ];

    for (lmax, operation, expected_by_pool) in operations {
        for (pool_index, pool) in pools.iter().enumerate() {
            let plan = make_plan(lmax, lmax, lmax + 1, 2 * lmax + 2, 0.37);
            let expected = expected_by_pool[pool_index];
            assert_eq!(
                pool.install(|| plan.scalar_legendre_strategy(operation)),
                expected,
                "T{lmax} {operation:?} q{} strategy",
                pool.current_num_threads()
            );

            let mut workspace = pool.install(|| plan.workspace()).unwrap();
            let alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            let map = vec![0.0; plan.sample_count];
            match operation {
                Synthesis => {
                    let mut output = vec![0.0; plan.sample_count];
                    pool.install(|| plan.synthesis_into(&alm, &mut output, &mut workspace))
                        .unwrap();
                }
                Analysis => {
                    let mut output = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
                    pool.install(|| plan.analysis_into(&map, &mut output, &mut workspace))
                        .unwrap();
                }
            }
            assert_eq!(
                plan.scalar_legendre_table
                    .get()
                    .is_some_and(Option::is_some),
                expected == CompactTable,
                "T{lmax} {operation:?} q{} executed route",
                pool.current_num_threads()
            );
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_reuse_estimate_gates_tables_at_measured_break_even() {
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let make = |synthesis_calls, analysis_calls| -> ShtPlan {
        ShtPlan::new_with_legendre_reuse_estimate(
            GridSpec::new(GridKind::GaussLegendre, 96, 192, 0.37).unwrap(),
            HarmonicSpec::scalar(63, 63).unwrap(),
            LegendreReuseEstimate::new(synthesis_calls, analysis_calls),
        )
        .unwrap()
    };

    let synthesis_short = make(Some(39), None);
    let synthesis_break_even = make(Some(40), None);
    assert_eq!(
        eight_threads
            .install(|| synthesis_short.scalar_legendre_strategy(LegendreOperation::Synthesis)),
        LegendreStrategy::Recurrence
    );
    assert_eq!(
        eight_threads.install(|| {
            synthesis_break_even.scalar_legendre_strategy(LegendreOperation::Synthesis)
        }),
        LegendreStrategy::CompactTable
    );

    let analysis_short = make(None, Some(20));
    let analysis_break_even = make(None, Some(21));
    assert_eq!(
        one_thread.install(|| analysis_short.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );
    assert_eq!(
        one_thread
            .install(|| analysis_break_even.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::CompactTable
    );

    // The T63 q8 exact-grid exception remains in effect.
    assert_eq!(
        eight_threads
            .install(|| analysis_break_even.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_reuse_estimates_cannot_override_the_t64_cutoff() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let make = |calls| -> ShtPlan {
        ShtPlan::new_with_legendre_reuse_estimate(
            GridSpec::new(GridKind::GaussLegendre, 128, 256, 0.37).unwrap(),
            HarmonicSpec::scalar(85, 85).unwrap(),
            LegendreReuseEstimate::new(Some(calls), None),
        )
        .unwrap()
    };
    let before = make(69);
    let at_break_even = make(70);
    assert_eq!(
        pool.install(|| before.scalar_legendre_strategy(LegendreOperation::Synthesis)),
        LegendreStrategy::Recurrence
    );
    assert_eq!(
        pool.install(|| at_break_even.scalar_legendre_strategy(LegendreOperation::Synthesis)),
        LegendreStrategy::Recurrence
    );
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_reuse_estimate_counts_adjoint_calls_by_execution_direction() {
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let grid = GridSpec::new(GridKind::GaussLegendre, 96, 192, 0.37).unwrap();
    let harmonics = HarmonicSpec::scalar(63, 63).unwrap();

    let synthesis_adjoint = ShtPlan::new_with_legendre_reuse_estimate(
        grid.clone(),
        harmonics.clone(),
        LegendreReuseEstimate::new(Some(0), Some(21)),
    )
    .unwrap();
    let map = vec![0.0; synthesis_adjoint.sample_count];
    let mut alm = vec![Complex64::new(0.0, 0.0); harmonics.nalm()];
    let mut workspace = one_thread
        .install(|| synthesis_adjoint.workspace())
        .unwrap();
    one_thread
        .install(|| synthesis_adjoint.adjoint_synthesis_into(&map, &mut alm, &mut workspace))
        .unwrap();
    assert!(synthesis_adjoint.scalar_legendre_table.get().is_some());

    let analysis_adjoint = ShtPlan::new_with_legendre_reuse_estimate(
        grid.clone(),
        harmonics.clone(),
        LegendreReuseEstimate::new(Some(40), Some(0)),
    )
    .unwrap();
    let mut output = vec![0.0; analysis_adjoint.sample_count];
    let workspace = &mut eight_threads
        .install(|| analysis_adjoint.workspace())
        .unwrap();
    eight_threads
        .install(|| analysis_adjoint.adjoint_analysis_into(&alm, &mut output, workspace))
        .unwrap();
    assert!(analysis_adjoint.scalar_legendre_table.get().is_some());
    let mut adjoint_coefficients = vec![Complex64::new(0.0, 0.0); harmonics.nalm()];
    let workspace = &mut eight_threads
        .install(|| analysis_adjoint.workspace())
        .unwrap();
    eight_threads
        .install(|| {
            analysis_adjoint.adjoint_synthesis_into(&map, &mut adjoint_coefficients, workspace)
        })
        .unwrap();
    assert_eq!(
        eight_threads
            .install(|| analysis_adjoint.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_reuse_estimate_controls_analysis_table_at_t159() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let make = |calls| -> ShtPlan {
        ShtPlan::new_with_legendre_reuse_estimate(
            GridSpec::new(GridKind::GaussLegendre, 320, 640, 0.37).unwrap(),
            HarmonicSpec::scalar(159, 159).unwrap(),
            LegendreReuseEstimate::new(None, Some(calls)),
        )
        .unwrap()
    };
    assert_eq!(
        pool.install(|| make(157).scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );
    assert_eq!(
        pool.install(|| make(158).scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::CompactTable
    );
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_strategy_bounds_oversampled_tables_by_operation_and_truncation() {
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let two_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let four_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let sixteen_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build()
        .unwrap();

    let exact_t63 = make_plan(63, 63, 64, 128, 0.37);
    assert_eq!(
        eight_threads.install(|| exact_t63.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );

    let t63 = make_plan(63, 63, 96, 192, 0.37);
    assert_eq!(
        one_thread.install(|| t63.scalar_legendre_strategy(LegendreOperation::Synthesis)),
        LegendreStrategy::Recurrence
    );
    assert_eq!(
        one_thread.install(|| t63.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::CompactTable
    );
    assert_eq!(
        two_threads.install(|| t63.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::CompactTable
    );
    assert_eq!(
        four_threads.install(|| t63.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );
    assert_eq!(
        eight_threads.install(|| t63.scalar_legendre_strategy(LegendreOperation::Synthesis)),
        LegendreStrategy::CompactTable
    );
    assert_eq!(
        eight_threads.install(|| t63.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );
    assert_eq!(
        sixteen_threads.install(|| t63.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );

    let reduced_t127 = make_plan(127, 96, 256, 512, 0.37);
    assert_eq!(
        eight_threads
            .install(|| reduced_t127.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );

    let t159 = make_plan(159, 159, 320, 640, 0.37);
    for pool in [&one_thread, &two_threads, &four_threads, &eight_threads] {
        assert_eq!(
            pool.install(|| t159.scalar_legendre_strategy(LegendreOperation::Analysis)),
            LegendreStrategy::CompactTable
        );
    }
    assert_eq!(
        sixteen_threads.install(|| t159.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );
    let mut t159_slow_workspace = sixteen_threads.install(|| t159.workspace()).unwrap();
    let t159_map = vec![0.0; t159.sample_count];
    let mut t159_alm = vec![Complex64::new(0.0, 0.0); t159.harmonics().nalm()];
    sixteen_threads
        .install(|| t159.analysis_into(&t159_map, &mut t159_alm, &mut t159_slow_workspace))
        .unwrap();
    assert!(t159.scalar_legendre_table.get().is_none());

    let t191 = make_plan(191, 191, 384, 768, 0.37);
    assert_eq!(
        one_thread.install(|| t191.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );
    assert_eq!(
        eight_threads.install(|| t191.scalar_legendre_strategy(LegendreOperation::Analysis)),
        LegendreStrategy::Recurrence
    );
    assert_eq!(
        eight_threads.install(|| t191.scalar_legendre_strategy(LegendreOperation::Synthesis)),
        LegendreStrategy::Recurrence
    );

    let t255 = make_plan(255, 255, 512, 1024, 0.37);
    assert!(
        !crate::legendre::ScalarLegendreTable::<f64>::supports_grid_and_bandlimit(
            &t255.grid,
            &t255.harmonics
        )
    );
}

#[test]
fn scalar_analysis_recovers_closed_form_fields_at_minimum_sampling() {
    for nphi in [5, 6] {
        let plan = make_plan(3, 2, 4, nphi, 0.47);
        let expected = coefficients(plan.harmonics());
        let map = analytic_map(&plan, &expected);
        let mut actual = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut workspace = plan.workspace().unwrap();

        plan.analysis_into(&map, &mut actual, &mut workspace)
            .unwrap();
        for (actual, expected) in actual.iter().zip(&expected) {
            assert!((*actual - *expected).norm() < 2e-13);
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
fn workspace_sizes_analysis_scratch_for_order_parallelism() {
    let plan = make_plan(127, 127, 128, 257, 0.37);
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let sixteen_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build()
        .unwrap();
    let mut one_thread_workspace = one_thread.install(|| plan.workspace()).unwrap();
    let mut sixteen_thread_workspace = sixteen_threads.install(|| plan.workspace()).unwrap();

    assert_eq!(
        sixteen_thread_workspace.ring_group_size,
        LATITUDE_BLOCK_SIZE / 4
    );
    assert_eq!(sixteen_thread_workspace.recurrence_workers.len(), 16);

    let map = arbitrary_map(&plan);
    let mut one_thread_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut sixteen_thread_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    one_thread
        .install(|| plan.analysis_into(&map, &mut one_thread_alm, &mut one_thread_workspace))
        .unwrap();
    sixteen_threads
        .install(|| {
            plan.analysis_into(&map, &mut sixteen_thread_alm, &mut sixteen_thread_workspace)
        })
        .unwrap();

    for (&single, &parallel) in one_thread_alm.iter().zip(&sixteen_thread_alm) {
        assert!((single - parallel).norm() < 2e-13);
    }

    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut reused_workspace_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    eight_threads
        .install(|| {
            plan.analysis_into(
                &map,
                &mut reused_workspace_alm,
                &mut sixteen_thread_workspace,
            )
        })
        .unwrap();
    for (&single, &parallel) in one_thread_alm.iter().zip(&reused_workspace_alm) {
        assert!((single - parallel).norm() < 2e-13);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn workspace_ring_groups_add_parallelism_for_underfilled_synthesis() {
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let cases = [
        (GridKind::GaussLegendre, 63, 64, 16, 8),
        (GridKind::ClenshawCurtis, 63, 65, 16, 8),
        (GridKind::Fejer1, 63, 64, 16, 8),
        (GridKind::GaussLegendre, 127, 128, 16, 8),
        (GridKind::ClenshawCurtis, 127, 129, 16, 9),
        (GridKind::GaussLegendre, 255, 256, 32, 8),
        (GridKind::GaussLegendre, 511, 512, 32, 16),
        (GridKind::ClenshawCurtis, 511, 513, 64, 9),
        (GridKind::GaussLegendre, 1023, 1024, 128, 8),
        (GridKind::ClenshawCurtis, 1023, 1025, 128, 9),
    ];

    for (kind, lmax, ntheta, expected_group_size, expected_workers) in cases {
        let plan = ShtPlan::<f64>::new(
            GridSpec::new(kind, ntheta, 2 * lmax + 3, 0.37).unwrap(),
            HarmonicSpec::scalar(lmax, lmax).unwrap(),
        )
        .unwrap();
        let workspace = eight_threads.install(|| plan.workspace()).unwrap();
        assert_eq!(workspace.ring_group_size, expected_group_size);
        assert_eq!(workspace.recurrence_workers.len(), expected_workers);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn fine_synthesis_groups_follow_the_measured_spin_and_grid_policy() {
    let four_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();

    let scalar_t511 = make_plan(511, 511, 512, 1025, 0.37);
    let scalar_t511_workspace = four_threads.install(|| scalar_t511.workspace()).unwrap();
    assert_eq!(scalar_t511.fine_synthesis_ring_group_size(4), Some(32));
    assert_eq!(scalar_t511_workspace.ring_group_size, 64);
    assert_eq!(scalar_t511_workspace.recurrence_workers.len(), 16);

    let scalar_t720 = make_plan(720, 720, 721, 1443, 0.37);
    let scalar_t720_workspace = four_threads.install(|| scalar_t720.workspace()).unwrap();
    assert_eq!(scalar_t720.fine_synthesis_ring_group_size(4), Some(32));
    assert_eq!(scalar_t720_workspace.ring_group_size, 192);
    assert_eq!(scalar_t720_workspace.recurrence_workers.len(), 23);

    let spin1_t720: ShtPlan<f64> = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, 721, 1443, 0.37).unwrap(),
        HarmonicSpec::new(720, 720, 1).unwrap(),
    )
    .unwrap();
    let spin1_four_threads = four_threads.install(|| spin1_t720.workspace()).unwrap();
    assert_eq!(spin1_t720.fine_synthesis_ring_group_size(4), None);
    assert_eq!(spin1_four_threads.ring_group_size, 192);
    let spin1_eight_threads = eight_threads.install(|| spin1_t720.workspace()).unwrap();
    assert_eq!(spin1_t720.fine_synthesis_ring_group_size(8), Some(32));
    assert_eq!(spin1_eight_threads.ring_group_size, 64);
    assert_eq!(spin1_eight_threads.recurrence_workers.len(), 23);

    let reduced_order = make_plan(720, 360, 721, 1443, 0.37);
    let reduced_workspace = eight_threads.install(|| reduced_order.workspace()).unwrap();
    assert_eq!(reduced_order.fine_synthesis_ring_group_size(8), None);
    assert_eq!(reduced_workspace.ring_group_size, 64);

    let cc_plan = make_cc_plan(720, 720, 722, 1443, 0.37);
    let cc_workspace = eight_threads.install(|| cc_plan.workspace()).unwrap();
    assert_eq!(cc_plan.fine_synthesis_ring_group_size(8), None);
    assert_eq!(cc_workspace.ring_group_size, 64);
}

#[cfg(feature = "parallel")]
#[test]
fn scalar_synthesis_coefficient_cache_matches_uncached_eligible_shapes() {
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let group_ring_count = 64;
    for (lmax, mmax, ntheta, nphi) in [(1023, 1023, 1024, 2049), (1279, 900, 1281, 2561)] {
        let plan = make_plan(lmax, mmax, ntheta, nphi, 0.37);
        let mut eight_thread_workspace = eight_threads.install(|| plan.workspace()).unwrap();
        let group_ring_count = group_ring_count.min(ntheta);
        let ring_group_size = plan
            .fine_synthesis_ring_group_size(8)
            .unwrap_or(eight_thread_workspace.ring_group_size)
            .min(ntheta);
        let parallel_groups = ntheta.div_ceil(ring_group_size);
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        for m in 0..=plan.harmonics.mmax() {
            for l in m..=plan.harmonics.lmax() {
                let real = ((l * 7 + m * 11) % 19) as f64 - 9.0;
                let imaginary = ((l * 13 + m * 5) % 23) as f64 - 11.0;
                alm[plan.harmonics.index(l, m).unwrap()] =
                    Complex64::new(real * 1.0e-5, if m == 0 { 0.0 } else { imaginary * 1.0e-5 });
            }
        }
        for (ring, &theta) in plan.grid.theta().iter().enumerate() {
            let (sin_theta, cos_theta) = sin_cos_from_f64(theta);
            eight_thread_workspace.theta_sin[ring] = sin_theta;
            eight_thread_workspace.theta_cos[ring] = cos_theta;
            eight_thread_workspace.ring_order_limits[ring] =
                f64::polar_order_limit(plan.harmonics.lmax(), 0, sin_theta, cos_theta);
        }
        let layout = eight_threads
            .install(|| {
                plan.prepare_scalar_synthesis_coefficient_cache(
                    &alm,
                    &mut eight_thread_workspace.spin_spectra,
                    &mut eight_thread_workspace.recurrence_workers[..parallel_groups],
                    8,
                    parallel_groups,
                )
            })
            .expect("eligible cache fits the existing scalar workspace buffer");
        assert_eq!(layout.row_stride, (lmax + 2) / 2 * 2);
        assert_eq!(layout.half_stride, (lmax + 2) / 2);
        assert_eq!(layout.coefficient_count, layout.row_stride * (mmax + 1));

        let ring_start = ntheta / 4;
        let ring_end = ring_start + group_ring_count;
        let spectrum_len = plan.fft.spectrum_len();
        let mut cached_spectra = vec![Complex64::new(0.0, 0.0); group_ring_count * spectrum_len];
        let mut uncached_spectra = cached_spectra.clone();
        let cached_coefficients = ScalarSynthesisCoefficientCache {
            values: &eight_thread_workspace.spin_spectra[..layout.coefficient_count],
            row_stride: layout.row_stride,
            half_stride: layout.half_stride,
        };
        eight_threads
            .install(|| {
                plan.synthesize_ring_group_with_scalar_coefficients(
                    &alm,
                    plan.grid.nphi(),
                    spectrum_len,
                    plan.grid.phi0(),
                    ring_start,
                    &mut cached_spectra,
                    &eight_thread_workspace.theta_sin[ring_start..ring_end],
                    &eight_thread_workspace.theta_cos[ring_start..ring_end],
                    &eight_thread_workspace.ring_order_limits,
                    pulp::Arch::new(),
                    &mut eight_thread_workspace.recurrence_workers[0],
                    Some(cached_coefficients),
                )
            })
            .unwrap();
        eight_threads
            .install(|| {
                plan.synthesize_ring_group(
                    &alm,
                    plan.grid.nphi(),
                    spectrum_len,
                    plan.grid.phi0(),
                    ring_start,
                    &mut uncached_spectra,
                    &eight_thread_workspace.theta_sin[ring_start..ring_end],
                    &eight_thread_workspace.theta_cos[ring_start..ring_end],
                    &eight_thread_workspace.ring_order_limits,
                    pulp::Arch::new(),
                    &mut eight_thread_workspace.recurrence_workers[0],
                )
            })
            .unwrap();
        assert!(
            relative_l2_complex(&cached_spectra, &uncached_spectra) < 2.0e-15,
            "cached synthesis differs at T{lmax}, mmax={mmax}"
        );
    }
}

#[cfg(feature = "parallel")]
#[test]
fn fine_scalar_synthesis_groups_preserve_high_band_adjoint_identities() {
    let plan = make_plan(511, 511, 512, 1025, 0.37);
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
    for (l, m, value) in [
        (0, 0, Complex64::new(0.7, 0.0)),
        (37, 11, Complex64::new(-0.23, 0.17)),
        (511, 511, Complex64::new(0.31, -0.19)),
    ] {
        alm[plan.harmonics.index(l, m).unwrap()] = value;
    }
    let input = arbitrary_map(&plan);
    let four_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let mut workspace = four_threads.install(|| plan.workspace()).unwrap();
    let mut synthesized = vec![0.0; plan.sample_count];
    four_threads
        .install(|| plan.synthesis_into(&alm, &mut synthesized, &mut workspace))
        .unwrap();
    assert_eq!(workspace.recurrence_workers.len(), 16);

    let mut synthesis_adjoint = vec![Complex64::new(0.0, 0.0); alm.len()];
    four_threads
        .install(|| plan.adjoint_synthesis_into(&input, &mut synthesis_adjoint, &mut workspace))
        .unwrap();
    assert_close(
        map_inner_product(&synthesized, &input),
        harmonic_inner_product(plan.harmonics(), &alm, &synthesis_adjoint),
        2e-10,
    );

    let mut analyzed = vec![Complex64::new(0.0, 0.0); alm.len()];
    four_threads
        .install(|| plan.analysis_into(&input, &mut analyzed, &mut workspace))
        .unwrap();
    let mut analysis_adjoint = vec![0.0; plan.sample_count];
    four_threads
        .install(|| plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace))
        .unwrap();
    assert_close(
        harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
        map_inner_product(&input, &analysis_adjoint),
        2e-10,
    );
}

#[cfg(feature = "parallel")]
#[test]
fn synthesis_workspace_from_larger_pool_is_safe_in_smaller_pool() {
    let plan = make_plan(3, 2, 128, 9, 0.41);
    let input = coefficients(plan.harmonics());
    let eight_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let two_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();

    let mut shared_workspace = eight_threads.install(|| plan.workspace()).unwrap();
    let mut single_workspace = one_thread.install(|| plan.workspace()).unwrap();
    assert_eq!(shared_workspace.ring_group_size, LATITUDE_BLOCK_SIZE / 4);

    let mut shared_output = vec![0.0; plan.sample_count];
    let mut single_output = vec![0.0; plan.sample_count];
    two_threads
        .install(|| plan.synthesis_into(&input, &mut shared_output, &mut shared_workspace))
        .unwrap();
    one_thread
        .install(|| plan.synthesis_into(&input, &mut single_output, &mut single_workspace))
        .unwrap();

    for (&shared, &single) in shared_output.iter().zip(&single_output) {
        assert!((shared - single).abs() < 2e-14);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn t1023_workspace_pool_migration_records_active_synthesis_workers() {
    let plan = ShtPlan::new_with_legendre_reuse_estimate(
        GridSpec::new(GridKind::GaussLegendre, 1024, 2048, 0.23).unwrap(),
        HarmonicSpec::scalar(1023, 1023).unwrap(),
        LegendreReuseEstimate::new(Some(0), Some(0)),
    )
    .unwrap();
    assert_eq!(
        plan.scalar_legendre_strategy(LegendreOperation::Synthesis),
        LegendreStrategy::Recurrence
    );
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let three = rayon::ThreadPoolBuilder::new()
        .num_threads(3)
        .build()
        .unwrap();
    let eight = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let sixteen = rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build()
        .unwrap();
    let migrations = [
        ("q1->q8", &one, &eight, 1, 1),
        ("q8->q1", &eight, &one, 1, 1),
        ("q1->q16", &one, &sixteen, 1, 1),
        ("q16->q8", &sixteen, &eight, 2, 8),
        ("q3->q8", &three, &eight, 2, 4),
    ];
    let coefficients = coefficients(plan.harmonics());
    let mut output = vec![0.0; plan.sample_count];

    for (label, source, destination, minimum_active, maximum_active) in migrations {
        let mut workspace = source.install(|| plan.workspace()).unwrap();
        let probe = workspace.observe_active_workers();
        destination
            .install(|| plan.synthesis_into(&coefficients, &mut output, &mut workspace))
            .unwrap();
        let active = probe.lock().expect("worker probe is not poisoned").len();
        eprintln!(
            "T1023 recurrence synthesis {label}: {} active Rayon workers; {} recurrence scratch slots",
            active,
            workspace.recurrence_workers.len()
        );
        assert!(
            (minimum_active..=maximum_active).contains(&active),
            "{label} observed {active} active workers"
        );
    }

    let mut adapted = one.install(|| plan.workspace()).unwrap();
    one.install(|| plan.prepare_workspace(&mut adapted, 8))
        .unwrap();
    assert_eq!(adapted.execution_capacity(), 8);
    assert_eq!(adapted.allocated_worker_count(), 8);
    let probe = adapted.observe_active_workers();
    let mut adapted_output = vec![0.0; plan.sample_count];
    eight
        .install(|| plan.synthesis_into(&coefficients, &mut adapted_output, &mut adapted))
        .unwrap();
    let active = probe.lock().expect("worker probe is not poisoned").len();
    eprintln!(
        "T1023 recurrence synthesis q1->q8 after explicit preparation: {active} active Rayon workers; {} recurrence scratch slots",
        adapted.allocated_worker_count()
    );
    assert!((2..=8).contains(&active));

    let mut fresh = eight.install(|| plan.workspace()).unwrap();
    let mut fresh_output = vec![0.0; plan.sample_count];
    eight
        .install(|| plan.synthesis_into(&coefficients, &mut fresh_output, &mut fresh))
        .unwrap();
    assert!(relative_l2_real(&adapted_output, &fresh_output) <= 1e-13);
}

#[cfg(feature = "parallel")]
#[test]
fn prepared_workspace_matches_fresh_workspace_for_scalar_operations_all_grids() {
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let eight = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();

    for kind in [
        GridKind::GaussLegendre,
        GridKind::ClenshawCurtis,
        GridKind::Fejer1,
    ] {
        let plan = ShtPlan::new(
            GridSpec::new(kind, 260, 17, 0.37).unwrap(),
            HarmonicSpec::scalar(7, 5).unwrap(),
        )
        .unwrap();
        let input_alm = coefficients(plan.harmonics());
        let input_map = arbitrary_map(&plan);
        let mut adapted = one.install(|| plan.workspace()).unwrap();
        assert_eq!(adapted.execution_capacity(), 1);
        assert!(matches!(
            plan.prepare_workspace(
                &mut adapted,
                ShtWorkspace::<f64>::MAX_EXECUTION_CAPACITY + 1,
            ),
            Err(ShtError::InvalidExecutionCapacity { .. })
        ));
        assert_eq!(adapted.execution_capacity(), 1);
        one.install(|| plan.prepare_workspace(&mut adapted, 4))
            .unwrap();
        assert_eq!(adapted.execution_capacity(), 4);
        let allocated_after_prepare = adapted.allocated_worker_count();
        assert!(allocated_after_prepare > 1);
        let mut fresh = eight.install(|| plan.workspace()).unwrap();
        eight
            .install(|| plan.prepare_workspace(&mut fresh, 4))
            .unwrap();
        assert_eq!(fresh.execution_capacity(), 4);
        let probe = adapted.observe_active_workers();

        let mut adapted_synthesis = vec![0.0; plan.sample_count];
        let mut fresh_synthesis = vec![0.0; plan.sample_count];
        eight
            .install(|| plan.synthesis_into(&input_alm, &mut adapted_synthesis, &mut adapted))
            .unwrap();
        let active_workers = probe.lock().expect("worker probe is not poisoned").len();
        assert!((1..=4).contains(&active_workers));
        eight
            .install(|| plan.synthesis_into(&input_alm, &mut fresh_synthesis, &mut fresh))
            .unwrap();
        assert!(relative_l2_real(&adapted_synthesis, &fresh_synthesis) <= 1e-13);

        let mut adapted_synthesis_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut fresh_synthesis_adjoint = adapted_synthesis_adjoint.clone();
        eight
            .install(|| {
                plan.adjoint_synthesis_into(
                    &input_map,
                    &mut adapted_synthesis_adjoint,
                    &mut adapted,
                )
            })
            .unwrap();
        eight
            .install(|| {
                plan.adjoint_synthesis_into(&input_map, &mut fresh_synthesis_adjoint, &mut fresh)
            })
            .unwrap();
        assert!(relative_l2_complex(&adapted_synthesis_adjoint, &fresh_synthesis_adjoint) <= 1e-13);
        assert_close(
            map_inner_product(&adapted_synthesis, &input_map),
            harmonic_inner_product(plan.harmonics(), &input_alm, &adapted_synthesis_adjoint),
            2e-10,
        );

        let mut adapted_analysis = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        let mut fresh_analysis = adapted_analysis.clone();
        probe.lock().expect("worker probe is not poisoned").clear();
        eight
            .install(|| plan.analysis_into(&input_map, &mut adapted_analysis, &mut adapted))
            .unwrap();
        let active_analysis_workers = probe.lock().expect("worker probe is not poisoned").len();
        assert!((1..=4).contains(&active_analysis_workers));
        eight
            .install(|| plan.analysis_into(&input_map, &mut fresh_analysis, &mut fresh))
            .unwrap();
        assert!(relative_l2_complex(&adapted_analysis, &fresh_analysis) <= 1e-13);

        let mut adapted_analysis_adjoint = vec![0.0; plan.sample_count];
        let mut fresh_analysis_adjoint = vec![0.0; plan.sample_count];
        eight
            .install(|| {
                plan.adjoint_analysis_into(&input_alm, &mut adapted_analysis_adjoint, &mut adapted)
            })
            .unwrap();
        eight
            .install(|| {
                plan.adjoint_analysis_into(&input_alm, &mut fresh_analysis_adjoint, &mut fresh)
            })
            .unwrap();
        assert!(relative_l2_real(&adapted_analysis_adjoint, &fresh_analysis_adjoint) <= 1e-13);
        assert_close(
            harmonic_inner_product(plan.harmonics(), &adapted_analysis, &input_alm),
            map_inner_product(&input_map, &adapted_analysis_adjoint),
            2e-10,
        );

        eight
            .install(|| plan.prepare_workspace(&mut adapted, 1))
            .unwrap();
        assert_eq!(adapted.execution_capacity(), 1);
        assert_eq!(adapted.allocated_worker_count(), allocated_after_prepare);
        let mut serial_synthesis = vec![0.0; plan.sample_count];
        one.install(|| plan.synthesis_into(&input_alm, &mut serial_synthesis, &mut adapted))
            .unwrap();
        assert!(relative_l2_real(&serial_synthesis, &fresh_synthesis) <= 1e-13);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn rayon_groups_match_one_thread_for_synthesis_and_analysis() {
    let plan = make_plan(3, 2, 131, 9, 0.41);
    let input_alm = coefficients(plan.harmonics());
    let expected_map = analytic_map(&plan, &input_alm);
    let one_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let two_threads = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let mut one_thread_workspace = one_thread.install(|| plan.workspace()).unwrap();
    let mut two_thread_workspace = two_threads.install(|| plan.workspace()).unwrap();
    let mut one_thread_map = vec![0.0; plan.sample_count];
    let mut two_thread_map = vec![0.0; plan.sample_count];

    one_thread
        .install(|| plan.synthesis_into(&input_alm, &mut one_thread_map, &mut one_thread_workspace))
        .unwrap();
    two_threads
        .install(|| plan.synthesis_into(&input_alm, &mut two_thread_map, &mut two_thread_workspace))
        .unwrap();
    for (&one_thread, &two_threads) in one_thread_map.iter().zip(&two_thread_map) {
        assert_close(two_threads, one_thread, 2e-14);
    }
    for (&actual, &expected) in two_thread_map.iter().zip(&expected_map) {
        assert_close(actual, expected, 3e-14);
    }

    let mut one_thread_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut two_thread_alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    one_thread
        .install(|| {
            plan.analysis_into(
                &expected_map,
                &mut one_thread_alm,
                &mut one_thread_workspace,
            )
        })
        .unwrap();
    two_threads
        .install(|| {
            plan.analysis_into(
                &expected_map,
                &mut two_thread_alm,
                &mut two_thread_workspace,
            )
        })
        .unwrap();
    for (&one_thread, &two_threads) in one_thread_alm.iter().zip(&two_thread_alm) {
        assert!((one_thread - two_threads).norm() < 2e-14);
    }
    for (&actual, &expected) in two_thread_alm.iter().zip(&input_alm) {
        assert!((actual - expected).norm() < 2e-13);
    }

    let input_map = arbitrary_map(&plan);
    let mut one_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut two_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    one_thread
        .install(|| {
            plan.adjoint_synthesis_into(&input_map, &mut one_adjoint, &mut one_thread_workspace)
        })
        .unwrap();
    two_threads
        .install(|| {
            plan.adjoint_synthesis_into(&input_map, &mut two_adjoint, &mut two_thread_workspace)
        })
        .unwrap();
    for (&one, &two) in one_adjoint.iter().zip(&two_adjoint) {
        assert!((one - two).norm() < 2e-14);
    }

    let mut one_analysis_adjoint = vec![0.0; plan.sample_count];
    let mut two_analysis_adjoint = vec![0.0; plan.sample_count];
    one_thread
        .install(|| {
            plan.adjoint_analysis_into(
                &input_alm,
                &mut one_analysis_adjoint,
                &mut one_thread_workspace,
            )
        })
        .unwrap();
    two_threads
        .install(|| {
            plan.adjoint_analysis_into(
                &input_alm,
                &mut two_analysis_adjoint,
                &mut two_thread_workspace,
            )
        })
        .unwrap();
    for (&one, &two) in one_analysis_adjoint.iter().zip(&two_analysis_adjoint) {
        assert_close(two, one, 2e-14);
    }

    let mut one_derivative_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut two_derivative_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    one_thread
        .install(|| {
            plan.adjoint_synthesis_deriv1_into(
                &input_map,
                &expected_map,
                &mut one_derivative_adjoint,
                &mut one_thread_workspace,
            )
        })
        .unwrap();
    two_threads
        .install(|| {
            plan.adjoint_synthesis_deriv1_into(
                &input_map,
                &expected_map,
                &mut two_derivative_adjoint,
                &mut two_thread_workspace,
            )
        })
        .unwrap();
    for (&one, &two) in one_derivative_adjoint.iter().zip(&two_derivative_adjoint) {
        assert!((one - two).norm() < 2e-14);
    }
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain DUCC's printed reference digits."
)]
fn scalar_analysis_matches_ducc_0410_reference() {
    // Reference: ducc0 0.41.0, GL, ntheta=3, nphi=5, phi0=0.37, nthreads=1.
    let plan = make_plan(2, 2, 3, 5, 0.37);
    let map: Vec<_> = (0..3)
        .flat_map(|i| {
            (0..5).map(move |j| {
                let i = i as f64;
                let j = j as f64;
                (0.27 * i + 0.41 * j).sin() + 0.1 * i * j
            })
        })
        .collect();
    let expected = [
        Complex64::new(3.29516125962815387e0, 0.0),
        Complex64::new(-8.02149484639918087e-1, 0.0),
        Complex64::new(-4.75137285362010608e-2, 0.0),
        Complex64::new(4.74880729100706500e-1, -7.04706284735987509e-1),
        Complex64::new(-3.49490137183501304e-2, 1.41321010292757131e-2),
        Complex64::new(-2.49437516790312230e-1, 3.67573142006427211e-1),
    ];
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut workspace = plan.workspace().unwrap();

    plan.analysis_into(&map, &mut alm, &mut workspace).unwrap();
    for (&actual, &reference) in alm.iter().zip(&expected) {
        assert!((actual - reference).norm() < 4e-14);
    }
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain DUCC's printed reference digits."
)]
fn t255_analysis_matches_ducc_0410_coefficients() {
    // Reference: ducc0 0.41.0, GL, T255, phi0=0.37, nthreads=1.
    // The cutoff boundary creates mixed north/south pair blocks for compaction.
    let plan = make_plan(255, 255, 256, 513, 0.37);
    let map: Vec<_> = (0..256)
        .flat_map(|i| {
            (0..513).map(move |j| {
                let i = i as f64;
                let j = j as f64;
                (0.019 * i + 0.037 * j).sin() + 0.3 * (0.071 * i - 0.013 * j).cos()
            })
        })
        .collect();
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut workspace = plan.workspace().unwrap();

    plan.analysis_into(&map, &mut alm, &mut workspace).unwrap();
    for (l, m, reference) in [
        (0, 0, Complex64::new(6.76004192896470860e-3, 0.0)),
        (1, 0, Complex64::new(1.29270045097557157e-2, 0.0)),
        (255, 0, Complex64::new(-5.40717096686140754e-9, 0.0)),
        (
            128,
            64,
            Complex64::new(-3.85208519952169484e-5, -9.74532652404812478e-6),
        ),
        (
            200,
            128,
            Complex64::new(2.69942915754136812e-7, -3.89128455741458547e-7),
        ),
        (
            255,
            254,
            Complex64::new(-6.43558637760604195e-5, -1.66103146151738605e-5),
        ),
        (
            255,
            255,
            Complex64::new(-5.16216786170599655e-5, 5.78457625404296755e-6),
        ),
    ] {
        let index = plan.harmonics().index(l, m).unwrap();
        assert!(
            (alm[index] - reference).norm() < 4e-13,
            "(l={l}, m={m}): actual={:?}, expected={reference:?}",
            alm[index]
        );
    }

    let mut adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.adjoint_synthesis_into(&map, &mut adjoint, &mut workspace)
        .unwrap();
    let mut synthesized = vec![0.0; plan.sample_count];
    plan.synthesis_into(&alm, &mut synthesized, &mut workspace)
        .unwrap();
    assert_close(
        map_inner_product(&synthesized, &map),
        harmonic_inner_product(plan.harmonics(), &alm, &adjoint),
        3e-10,
    );

    #[cfg(feature = "parallel")]
    {
        let one_thread = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let eight_threads = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap();
        let mut one_workspace = one_thread.install(|| plan.workspace()).unwrap();
        let mut eight_workspace = eight_threads.install(|| plan.workspace()).unwrap();
        let mut one_thread_alm = vec![Complex64::new(0.0, 0.0); alm.len()];
        let mut eight_thread_alm = vec![Complex64::new(0.0, 0.0); alm.len()];
        one_thread
            .install(|| plan.analysis_into(&map, &mut one_thread_alm, &mut one_workspace))
            .unwrap();
        eight_threads
            .install(|| plan.analysis_into(&map, &mut eight_thread_alm, &mut eight_workspace))
            .unwrap();
        for (&one, &eight) in one_thread_alm.iter().zip(&eight_thread_alm) {
            assert!((one - eight).norm() < 4e-13);
        }
    }

    let mut analysis_adjoint = vec![0.0; plan.sample_count];
    plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
        .unwrap();
    assert_close(
        harmonic_inner_product(plan.harmonics(), &alm, &alm),
        map_inner_product(&map, &analysis_adjoint),
        3e-10,
    );

    let second_component: Vec<_> = (0..256)
        .flat_map(|i| {
            (0..513).map(move |j| {
                let i = i as f64;
                let j = j as f64;
                (0.19 * i - 0.13 * j).cos() - 0.15 * (0.29 * i + 0.07 * j).sin()
            })
        })
        .collect();
    let mut derivative_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.adjoint_synthesis_deriv1_into(
        &map,
        &second_component,
        &mut derivative_adjoint,
        &mut workspace,
    )
    .unwrap();
    let mut derivative_theta = vec![0.0; plan.sample_count];
    let mut derivative_phi = vec![0.0; plan.sample_count];
    plan.synthesis_deriv1_into(
        &alm,
        &mut derivative_theta,
        &mut derivative_phi,
        &mut workspace,
    )
    .unwrap();
    assert_close(
        map_inner_product(&derivative_theta, &map)
            + map_inner_product(&derivative_phi, &second_component),
        harmonic_inner_product(plan.harmonics(), &alm, &derivative_adjoint),
        3e-10,
    );
}

#[test]
#[allow(clippy::excessive_precision)]
fn t191_analysis_compaction_matches_ducc_all_grids() {
    // Reference: ducc0==0.41.0, T191, nphi=385, phi0=0.37, one thread.
    let references = [
        (
            GridKind::GaussLegendre,
            [
                Complex64::new(-1.101_262_571_078_748_08e-4, 0.0),
                Complex64::new(1.845_452_940_566_445_29e-4, 0.0),
                Complex64::new(-1.080_429_232_554_343_58e-8, 0.0),
                Complex64::new(-6.990_743_748_790_974_32e-5, 1.820_040_638_319_435_70e-5),
                Complex64::new(-1.647_871_086_006_443_26e-4, -1.574_360_945_669_752_92e-4),
                Complex64::new(1.763_932_922_834_878_08e-4, -4.649_072_237_886_739_25e-4),
            ],
        ),
        (
            GridKind::ClenshawCurtis,
            [
                Complex64::new(-1.253_917_058_211_111_45e-4, 0.0),
                Complex64::new(1.627_367_181_468_490_85e-4, 0.0),
                Complex64::new(-7.442_831_898_843_664_73e-6, 0.0),
                Complex64::new(-9.377_048_021_241_667_09e-5, 1.910_311_040_542_690_34e-5),
                Complex64::new(-1.484_896_390_713_879_52e-4, -1.468_151_142_568_995_24e-4),
                Complex64::new(1.931_827_497_450_281_54e-4, -5.084_929_619_890_495_46e-4),
            ],
        ),
        (
            GridKind::Fejer1,
            [
                Complex64::new(-1.142_513_565_486_694_15e-4, 0.0),
                Complex64::new(1.754_341_756_180_375_02e-4, 0.0),
                Complex64::new(-6.610_233_887_425_625_58e-8, 0.0),
                Complex64::new(-9.750_456_225_752_872_35e-5, 2.472_513_103_221_867_65e-5),
                Complex64::new(-1.628_661_903_813_164_06e-4, -1.556_001_811_173_575_36e-4),
                Complex64::new(1.761_913_095_214_543_59e-4, -4.643_748_404_299_327_15e-4),
            ],
        ),
    ];
    let sampled_modes = [(0, 0), (1, 0), (191, 0), (64, 32), (150, 98), (191, 190)];

    for (kind, expected) in references {
        let lmax = 191;
        let ntheta = lmax + usize::from(kind == GridKind::ClenshawCurtis) + 1;
        let plan = ShtPlan::new(
            GridSpec::new(kind, ntheta, 2 * lmax + 3, 0.37).unwrap(),
            HarmonicSpec::scalar(lmax, lmax).unwrap(),
        )
        .unwrap();
        let map = arbitrary_map(&plan);
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        let mut workspace = plan.workspace().unwrap();
        plan.analysis_into(&map, &mut alm, &mut workspace).unwrap();

        for (&(l, m), &reference) in sampled_modes.iter().zip(&expected) {
            let index = plan.harmonics().index(l, m).unwrap();
            assert!(
                (alm[index] - reference).norm() < 2e-13,
                "{kind:?} T191 (l={l}, m={m}): actual={:?}, expected={reference:?}",
                alm[index]
            );
        }

        let mut adjoint_map = vec![0.0; plan.sample_count];
        plan.adjoint_analysis_into(&alm, &mut adjoint_map, &mut workspace)
            .unwrap();
        assert_close(
            harmonic_inner_product(plan.harmonics(), &alm, &alm),
            map_inner_product(&map, &adjoint_map),
            3e-10,
        );
    }
}

#[test]
fn scalar_t511_gl_analysis_matches_ducc_and_analytic_sectoral_field() {
    // DUCC 0.41.0 references use a_(511,511)=1, phi0=0.37 and nthreads=1.
    let degree = 511;
    let nphi = 2 * degree + 3;
    let plan = make_plan(degree, degree, degree + 1, nphi, 0.37);
    let sectoral_index = plan.harmonics().index(degree, degree).unwrap();
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    alm[sectoral_index] = Complex64::new(1.0, 0.0);
    let mut workspace = plan.workspace().unwrap();
    let mut map = vec![0.0; plan.sample_count];
    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();

    let ducc_values = [
        -2.3872256264278287,
        2.4013318896018663,
        2.5986634372168815,
        1.5560946744052795,
        2.3729175336200483,
    ];
    let middle_ring = plan.grid.ntheta() / 2;
    for (longitude, expected) in [0, 1, 17, nphi / 2, nphi - 1].into_iter().zip(ducc_values) {
        let actual = map[middle_ring * nphi + longitude];
        assert!((actual - expected).abs() <= 5.0e-11);
    }

    let mut sectoral_norm = 1.0 / (4.0 * std::f64::consts::PI).sqrt();
    for l in 1..=degree {
        sectoral_norm *= ((2 * l + 1) as f64 / (2 * l) as f64).sqrt();
    }
    let sign = if degree.is_multiple_of(2) { 1.0 } else { -1.0 };
    for (ring, &theta) in plan.grid.theta().iter().enumerate() {
        let amplitude = 2.0 * sign * sectoral_norm * theta.sin().powi(degree as i32);
        for longitude in 0..nphi {
            let phi = plan.grid.longitude(longitude).unwrap();
            let expected = amplitude * (degree as f64 * phi).cos();
            let actual = map[ring * nphi + longitude];
            assert!((actual - expected).abs() <= 5.0e-10);
        }
    }

    let mut recovered = vec![Complex64::new(0.0, 0.0); alm.len()];
    plan.analysis_into(&map, &mut recovered, &mut workspace)
        .unwrap();
    for (index, actual) in recovered.iter().enumerate() {
        let expected = if index == sectoral_index { 1.0 } else { 0.0 };
        assert!(
            (*actual - Complex64::new(expected, 0.0)).norm() <= 2.0e-7,
            "coefficient {index}: actual={actual:?}, expected={expected}"
        );
    }
}

#[test]
#[allow(unexpected_cfgs)]
fn scalar_analysis_pair_range_cache_matches_checked_recurrence_and_is_plan_scoped() {
    let plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, 1025, 513, 0.37).unwrap(),
        HarmonicSpec::scalar(511, 255).unwrap(),
    )
    .unwrap();
    assert!(plan.scalar_legendre_table.set(None).is_ok());
    let mut cached_workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut cached_workspace, 1).unwrap();

    let input = arbitrary_map(&plan);
    let mut first = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.analysis_into(&input, &mut first, &mut cached_workspace)
        .unwrap();
    if !cfg!(rustsht_unchecked_analysis_pair_range)
        && !cfg!(rustsht_disable_analysis_pair_range_cache)
    {
        assert!(
            cached_workspace
                .analysis_pair_range_safe
                .iter()
                .any(|safe| safe.load(std::sync::atomic::Ordering::Relaxed))
        );
    }
    assert!(std::sync::Arc::ptr_eq(
        cached_workspace
            .analysis_pair_range_cache_id
            .as_ref()
            .unwrap(),
        &plan.analysis_pair_range_cache_id,
    ));

    let mut cached_second = vec![Complex64::new(0.0, 0.0); first.len()];
    plan.analysis_into(&input, &mut cached_second, &mut cached_workspace)
        .unwrap();
    let mut checked_workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut checked_workspace, 1).unwrap();
    let mut checked = vec![Complex64::new(0.0, 0.0); first.len()];
    plan.analysis_into(&input, &mut checked, &mut checked_workspace)
        .unwrap();
    assert_eq!(cached_second, checked);

    let zero_input = vec![0.0; plan.sample_count];
    let mut cached_zero = vec![Complex64::new(0.0, 0.0); first.len()];
    plan.analysis_into(&zero_input, &mut cached_zero, &mut cached_workspace)
        .unwrap();
    assert!(
        cached_zero
            .iter()
            .all(|value| *value == Complex64::new(0.0, 0.0))
    );

    let subnormal_input: Vec<_> = input.iter().map(|value| value * 1.0e-310).collect();
    let mut cached_subnormal = vec![Complex64::new(0.0, 0.0); first.len()];
    plan.analysis_into(
        &subnormal_input,
        &mut cached_subnormal,
        &mut cached_workspace,
    )
    .unwrap();
    let mut checked_subnormal = vec![Complex64::new(0.0, 0.0); first.len()];
    plan.analysis_into(
        &subnormal_input,
        &mut checked_subnormal,
        &mut checked_workspace,
    )
    .unwrap();
    assert_eq!(cached_subnormal, checked_subnormal);

    let extreme_input: Vec<_> = input.iter().map(|value| value * 1.0e300).collect();
    let mut cached_extreme = vec![Complex64::new(0.0, 0.0); first.len()];
    plan.analysis_into(&extreme_input, &mut cached_extreme, &mut cached_workspace)
        .unwrap();
    let mut checked_extreme = vec![Complex64::new(0.0, 0.0); first.len()];
    plan.analysis_into(&extreme_input, &mut checked_extreme, &mut checked_workspace)
        .unwrap();
    assert_eq!(cached_extreme, checked_extreme);

    let mut cached_adjoint = vec![Complex64::new(0.0, 0.0); first.len()];
    plan.adjoint_synthesis_into(&input, &mut cached_adjoint, &mut cached_workspace)
        .unwrap();
    assert_eq!(
        cached_workspace.analysis_pair_range_geometry,
        Some((plan.grid.ntheta(), false))
    );
    let mut checked_adjoint_workspace = plan.workspace().unwrap();
    let mut checked_adjoint = vec![Complex64::new(0.0, 0.0); first.len()];
    plan.adjoint_synthesis_into(&input, &mut checked_adjoint, &mut checked_adjoint_workspace)
        .unwrap();
    assert_eq!(cached_adjoint, checked_adjoint);

    let compatible_plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, 1025, 513, 0.19).unwrap(),
        HarmonicSpec::scalar(511, 255).unwrap(),
    )
    .unwrap();
    assert!(compatible_plan.scalar_legendre_table.set(None).is_ok());
    let mut compatible_alm = vec![Complex64::new(0.0, 0.0); compatible_plan.harmonics().nalm()];
    compatible_plan
        .analysis_into(&input, &mut compatible_alm, &mut cached_workspace)
        .unwrap();
    let mut compatible_reference_workspace = compatible_plan.workspace().unwrap();
    compatible_plan
        .prepare_workspace(&mut compatible_reference_workspace, 1)
        .unwrap();
    let mut compatible_reference =
        vec![Complex64::new(0.0, 0.0); compatible_plan.harmonics().nalm()];
    compatible_plan
        .analysis_into(
            &input,
            &mut compatible_reference,
            &mut compatible_reference_workspace,
        )
        .unwrap();
    assert_eq!(compatible_alm, compatible_reference);
    assert!(std::sync::Arc::ptr_eq(
        cached_workspace
            .analysis_pair_range_cache_id
            .as_ref()
            .unwrap(),
        &compatible_plan.analysis_pair_range_cache_id,
    ));
}

#[test]
fn scalar_analysis_fused_route_cutoff_is_f64_specific_and_active_at_t16() {
    assert_eq!(
        <f64 as crate::legendre::ShtScalar>::FUSED_SCALAR_ANALYSIS_MIN_LMAX,
        16
    );
    assert_eq!(
        <f32 as crate::legendre::ShtScalar>::FUSED_SCALAR_ANALYSIS_MIN_LMAX,
        255
    );

    for (lmax, mmax, ntheta, nphi, expect_fused) in [
        (15, 15, 16, 31, false),
        (16, 16, 17, 33, true),
        (31, 15, 32, 31, true),
        (31, 31, 32, 63, true),
        (32, 16, 33, 33, true),
        (63, 15, 64, 31, true),
        (64, 16, 65, 33, true),
        (126, 31, 127, 63, true),
        (127, 31, 128, 63, true),
    ] {
        let plan = make_plan(lmax, mmax, ntheta, nphi, 0.23);
        assert!(plan.scalar_legendre_table.set(None).is_ok());
        let mut workspace = plan.workspace().unwrap();
        plan.prepare_workspace(&mut workspace, 1).unwrap();
        let input = arbitrary_map(&plan);
        let mut output = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        plan.analysis_into(&input, &mut output, &mut workspace)
            .unwrap();
        let fused_blocks = workspace
            .analysis_pair_range_safe
            .iter()
            .filter(|safe| safe.load(std::sync::atomic::Ordering::Relaxed))
            .count();
        let used_fused_route = fused_blocks != 0;
        eprintln!("PLAN14_T{lmax}_FUSED_BLOCKS {fused_blocks}");
        assert_eq!(
            used_fused_route, expect_fused,
            "unexpected fused-route dispatch at T{lmax}"
        );
    }
}

#[test]
fn scalar_analysis_pair_range_cache_invalidates_prepared_geometry() {
    let plan = ShtPlan::new(
        GridSpec::new(GridKind::ClenshawCurtis, 1025, 7, 0.37).unwrap(),
        HarmonicSpec::scalar(255, 1).unwrap(),
    )
    .unwrap();
    assert!(plan.scalar_legendre_table.set(None).is_ok());
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut workspace, 1).unwrap();
    let input = arbitrary_map(&plan);
    let mut analyzed = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.analysis_into(&input, &mut analyzed, &mut workspace)
        .unwrap();
    let prepared_ntheta = plan.cc_resampling.as_ref().unwrap().prepared_ntheta();
    assert_eq!(
        workspace.analysis_pair_range_geometry,
        Some((prepared_ntheta, false))
    );

    let mut cached_adjoint = vec![Complex64::new(0.0, 0.0); analyzed.len()];
    plan.adjoint_synthesis_into(&input, &mut cached_adjoint, &mut workspace)
        .unwrap();
    assert_eq!(
        workspace.analysis_pair_range_geometry,
        Some((plan.grid.ntheta(), false))
    );
    let mut checked_workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut checked_workspace, 1).unwrap();
    let mut checked_adjoint = vec![Complex64::new(0.0, 0.0); analyzed.len()];
    plan.adjoint_synthesis_into(&input, &mut checked_adjoint, &mut checked_workspace)
        .unwrap();
    assert_eq!(cached_adjoint, checked_adjoint);
}

#[test]
fn scalar_analysis_pair_compaction_preserves_mixed_cutoff_lanes() {
    let m = 3;
    let latitude_count = 129;
    let block_start = 0;
    let block_len = 64;
    let spectra: Vec<_> = (0..latitude_count)
        .map(|ring| Complex64::new(ring as f64 + 1.0, -(ring as f64 + 2.0)))
        .collect();
    let mut limits = vec![0; latitude_count];
    // Exercise north-only, south-only, and two-sided active pairs.
    limits[2] = m;
    limits[latitude_count - 1 - 4] = m;
    limits[6] = m;
    limits[latitude_count - 1 - 6] = m;
    // An odd-latitude grid has one equatorial singleton in its own pair block.
    limits[latitude_count / 2] = m;
    let theta_sin: Vec<_> = (0..latitude_count)
        .map(|ring| 0.01 * (ring as f64 + 1.0))
        .collect();
    let theta_cos: Vec<_> = (0..latitude_count)
        .map(|ring| -0.01 * (ring as f64 + 1.0))
        .collect();
    let theta_cos_squared: Vec<_> = theta_cos.iter().map(|cosine| cosine * cosine).collect();

    let mut full_same_real = vec![0.0; block_len];
    let mut full_same_imag = vec![0.0; block_len];
    let mut full_opposite_real = vec![0.0; block_len];
    let mut full_opposite_imag = vec![0.0; block_len];
    let mut full_north_ring = vec![0; block_len];
    let mut unused_sin = vec![0.0; block_len];
    let mut unused_cos = vec![0.0; block_len];
    let mut unused_cos_squared = vec![0.0; block_len];
    let full_len = super::prepare_symmetric_analysis_pair_block(
        m,
        latitude_count,
        block_start,
        block_len,
        false,
        &spectra,
        &limits,
        &theta_sin,
        &theta_cos,
        &theta_cos_squared,
        &mut full_same_real,
        &mut full_same_imag,
        &mut full_opposite_real,
        &mut full_opposite_imag,
        &mut full_north_ring,
        &mut unused_sin,
        &mut unused_cos,
        &mut unused_cos_squared,
    );
    assert_eq!(full_len, block_len);

    let mut compact_same_real = vec![0.0; block_len];
    let mut compact_same_imag = vec![0.0; block_len];
    let mut compact_opposite_real = vec![0.0; block_len];
    let mut compact_opposite_imag = vec![0.0; block_len];
    let mut compact_north_ring = vec![0; block_len];
    let mut compact_sin = vec![0.0; block_len];
    let mut compact_cos = vec![0.0; block_len];
    let mut compact_cos_squared = vec![0.0; block_len];
    let compact_len = super::prepare_symmetric_analysis_pair_block(
        m,
        latitude_count,
        block_start,
        block_len,
        true,
        &spectra,
        &limits,
        &theta_sin,
        &theta_cos,
        &theta_cos_squared,
        &mut compact_same_real,
        &mut compact_same_imag,
        &mut compact_opposite_real,
        &mut compact_opposite_imag,
        &mut compact_north_ring,
        &mut compact_sin,
        &mut compact_cos,
        &mut compact_cos_squared,
    );
    let active_lanes = [2, 4, 6];
    assert_eq!(compact_len, active_lanes.len());
    for (packed_lane, &source_lane) in active_lanes.iter().enumerate() {
        assert_eq!(compact_north_ring[packed_lane], source_lane);
        assert_eq!(compact_sin[packed_lane], theta_sin[source_lane]);
        assert_eq!(compact_cos[packed_lane], theta_cos[source_lane]);
        assert_eq!(
            compact_cos_squared[packed_lane],
            theta_cos_squared[source_lane]
        );
        assert_eq!(compact_same_real[packed_lane], full_same_real[source_lane]);
        assert_eq!(compact_same_imag[packed_lane], full_same_imag[source_lane]);
        assert_eq!(
            compact_opposite_real[packed_lane],
            full_opposite_real[source_lane]
        );
        assert_eq!(
            compact_opposite_imag[packed_lane],
            full_opposite_imag[source_lane]
        );
    }
    let expected_opposite_real = compact_opposite_real[..compact_len].to_vec();
    let expected_opposite_imag = compact_opposite_imag[..compact_len].to_vec();
    for lane in 0..compact_len {
        compact_opposite_real[lane] *= compact_cos[lane];
        compact_opposite_imag[lane] *= compact_cos[lane];
    }
    super::restore_symmetric_analysis_pair_opposite(
        m,
        latitude_count,
        &spectra,
        &limits,
        &compact_north_ring[..compact_len],
        &mut compact_opposite_real[..compact_len],
        &mut compact_opposite_imag[..compact_len],
    );
    assert_eq!(
        &compact_opposite_real[..compact_len],
        expected_opposite_real
    );
    assert_eq!(
        &compact_opposite_imag[..compact_len],
        expected_opposite_imag
    );
    assert_eq!(full_same_real[2], spectra[2].re);
    assert_eq!(full_same_imag[2], spectra[2].im);
    assert_eq!(full_opposite_real[2], spectra[2].re);
    assert_eq!(full_opposite_imag[2], spectra[2].im);
    let south_only = spectra[latitude_count - 1 - 4];
    assert_eq!(full_same_real[4], south_only.re);
    assert_eq!(full_same_imag[4], south_only.im);
    assert_eq!(full_opposite_real[4], -south_only.re);
    assert_eq!(full_opposite_imag[4], -south_only.im);
    let north_pair = spectra[6];
    let south_pair = spectra[latitude_count - 1 - 6];
    assert_eq!(full_same_real[6], north_pair.re + south_pair.re);
    assert_eq!(full_same_imag[6], north_pair.im + south_pair.im);
    assert_eq!(full_opposite_real[6], north_pair.re - south_pair.re);
    assert_eq!(full_opposite_imag[6], north_pair.im - south_pair.im);

    let equator_start = latitude_count / 2;
    let mut equator_same_real = [0.0];
    let mut equator_same_imag = [0.0];
    let mut equator_opposite_real = [0.0];
    let mut equator_opposite_imag = [0.0];
    let mut equator_north_ring = [0];
    let mut equator_sin = [0.0];
    let mut equator_cos = [0.0];
    let mut equator_cos_squared = [0.0];
    let equator_len = super::prepare_symmetric_analysis_pair_block(
        m,
        latitude_count,
        equator_start,
        1,
        true,
        &spectra,
        &limits,
        &theta_sin,
        &theta_cos,
        &theta_cos_squared,
        &mut equator_same_real,
        &mut equator_same_imag,
        &mut equator_opposite_real,
        &mut equator_opposite_imag,
        &mut equator_north_ring,
        &mut equator_sin,
        &mut equator_cos,
        &mut equator_cos_squared,
    );
    assert_eq!(equator_len, 1);
    assert_eq!(equator_north_ring[0], equator_start);
    assert_eq!(equator_same_real[0], spectra[equator_start].re);
    assert_eq!(equator_same_imag[0], spectra[equator_start].im);
    assert_eq!(equator_opposite_real[0], spectra[equator_start].re);
    assert_eq!(equator_opposite_imag[0], spectra[equator_start].im);
}

#[test]
fn synthesis_combines_the_even_longitude_nyquist_mode() {
    let plan = make_plan(2, 2, 4, 4, 0.31);
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    alm[plan.harmonics().index(2, 2).unwrap()] = Complex64::new(0.6, -0.4);
    let expected = analytic_map(&plan, &alm);
    let mut map = vec![0.0; plan.sample_count];
    let mut workspace = plan.workspace().unwrap();
    plan.prepare_workspace(&mut workspace, 1).unwrap();

    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();
    for (&actual, &reference) in map.iter().zip(&expected) {
        assert_close(actual, reference, 3e-14);
    }
}

#[test]
fn operations_reject_undersampling_buffers_and_unimplemented_spin() {
    let too_few_rings = make_plan(3, 1, 3, 5, 0.0);
    let mut workspace = too_few_rings.workspace().unwrap();
    let map = vec![0.0; too_few_rings.sample_count];
    let mut alm = vec![Complex64::new(0.0, 0.0); too_few_rings.harmonics().nalm()];
    assert_eq!(
        too_few_rings.analysis_into(&map, &mut alm, &mut workspace),
        Err(ShtError::AnalysisLatitudeUndersampled { lmax: 3, ntheta: 3 })
    );
    let mut adjoint_map = vec![0.0; too_few_rings.sample_count];
    assert_eq!(
        too_few_rings.adjoint_analysis_into(&alm, &mut adjoint_map, &mut workspace),
        Err(ShtError::AnalysisLatitudeUndersampled { lmax: 3, ntheta: 3 })
    );

    let nyquist_analysis = make_plan(2, 2, 3, 4, 0.0);
    let mut nyquist_workspace = nyquist_analysis.workspace().unwrap();
    let nyquist_map = vec![0.0; nyquist_analysis.sample_count];
    let mut nyquist_alm = vec![Complex64::new(0.0, 0.0); nyquist_analysis.harmonics().nalm()];
    assert_eq!(
        nyquist_analysis.analysis_into(&nyquist_map, &mut nyquist_alm, &mut nyquist_workspace),
        Err(ShtError::AnalysisLongitudeUndersampled { mmax: 2, nphi: 4 })
    );
    let mut nyquist_adjoint_map = vec![0.0; nyquist_analysis.sample_count];
    assert_eq!(
        nyquist_analysis.adjoint_analysis_into(
            &nyquist_alm,
            &mut nyquist_adjoint_map,
            &mut nyquist_workspace
        ),
        Err(ShtError::AnalysisLongitudeUndersampled { mmax: 2, nphi: 4 })
    );

    let too_many_orders = make_plan(3, 3, 4, 4, 0.0);
    let mut order_workspace = too_many_orders.workspace().unwrap();
    let order_alm = vec![Complex64::new(0.0, 0.0); too_many_orders.harmonics().nalm()];
    let mut order_map = vec![0.0; too_many_orders.sample_count];
    assert_eq!(
        too_many_orders.synthesis_into(&order_alm, &mut order_map, &mut order_workspace),
        Err(ShtError::SynthesisLongitudeUndersampled { mmax: 3, nphi: 4 })
    );

    assert_eq!(
        nyquist_analysis.synthesis_into(&nyquist_alm, &mut order_map[..3], &mut nyquist_workspace),
        Err(ShtError::BufferLength {
            buffer: "map output",
            expected: nyquist_analysis.sample_count,
            actual: 3
        })
    );

    let spin_plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, 3, 5, 0.0).unwrap(),
        HarmonicSpec::new(2, 1, 1).unwrap(),
    )
    .unwrap();
    let mut spin_workspace = spin_plan.workspace().unwrap();
    let spin_alm = vec![Complex64::new(0.0, 0.0); spin_plan.harmonics().nalm()];
    let mut spin_map = vec![0.0; spin_plan.sample_count];
    assert_eq!(
        spin_plan.synthesis_into(&spin_alm, &mut spin_map, &mut spin_workspace),
        Err(ShtError::UnsupportedSpin { spin: 1 })
    );
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "The fixture values retain printed ducc0 0.41.0 outputs."
)]
fn clenshaw_curtis_scalar_operations_match_ducc_and_adjoint_identities() {
    // DUCC reference: ducc0==0.41.0, CC 4x5, phi0=0.37, one thread.
    let plan = make_cc_plan(2, 2, 4, 5, 0.37);
    let alm = coefficients(plan.harmonics());
    let map = arbitrary_map(&plan);
    let mut workspace = plan.workspace().unwrap();

    let mut synthesized = vec![0.0; plan.sample_count];
    plan.synthesis_into(&alm, &mut synthesized, &mut workspace)
        .unwrap();
    let analytic = analytic_map(&plan, &alm);
    for (&actual, &expected) in synthesized.iter().zip(&analytic) {
        assert_close(actual, expected, 5e-14);
    }
    for (index, expected) in [
        (0, 0.162_824_164_911_634_76),
        (1, 0.162_824_164_911_634_76),
        (2, 0.162_824_164_911_634_76),
        (3, 0.162_824_164_911_634_76),
        (4, 0.162_824_164_911_634_76),
        (5, 0.420_719_530_114_416_26),
        (10, 0.196_350_610_822_396_35),
        (15, 0.358_265_169_672_802_75),
        (19, 0.358_265_169_672_802_75),
    ] {
        assert_close(synthesized[index], expected, 3e-14);
    }

    let mut analyzed = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.analysis_into(&map, &mut analyzed, &mut workspace)
        .unwrap();
    let expected_analysis = [
        Complex64::new(2.763_651_652_288_861_4, 0.0),
        Complex64::new(-0.327_153_874_969_685_84, 0.0),
        Complex64::new(-0.027_926_626_089_303_408, 0.0),
        Complex64::new(0.199_456_242_909_442_36, -0.367_684_272_467_260_32),
        Complex64::new(-0.000_170_607_730_796_683_42, -0.025_218_840_386_616_675),
        Complex64::new(-0.129_749_726_124_326_17, 0.200_921_712_854_862_38),
    ];
    for (&actual, expected) in analyzed.iter().zip(expected_analysis) {
        assert_complex_close(actual, expected, 5e-13);
    }

    let mut synthesis_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.adjoint_synthesis_into(&map, &mut synthesis_adjoint, &mut workspace)
        .unwrap();
    let expected_synthesis_adjoint = [
        Complex64::new(4.355_007_000_427_095, 0.0),
        Complex64::new(-1.105_228_186_479_379_1, 0.0),
        Complex64::new(4.190_099_197_329_241, 0.0),
        Complex64::new(0.178_562_514_316_362_64, -0.329_168_078_214_253_7),
        Complex64::new(-0.000_190_919_979_042_051_46, -0.028_221_349_968_109_447),
        Complex64::new(-0.115_918_089_919_755_52, 0.179_490_470_259_712_76),
    ];
    for (&actual, expected) in synthesis_adjoint.iter().zip(expected_synthesis_adjoint) {
        assert_complex_close(actual, expected, 5e-13);
    }

    let mut analysis_adjoint = vec![0.0; plan.sample_count];
    plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
        .unwrap();
    for (index, expected) in [
        (0, 0.001_918_749_569_064_584_5),
        (1, 0.029_734_322_064_137_043),
        (2, 0.031_597_426_064_432_131),
        (3, 0.000_767_287_972_306_324_17),
        (4, 0.048_788_395_282_923_627),
        (5, 0.460_244_027_175_822_19),
        (10, 0.259_746_036_479_518_88),
        (15, 0.040_122_946_665_733_537),
        (19, 0.086_992_592_379_592_532),
    ] {
        assert_close(analysis_adjoint[index], expected, 3e-13);
    }
    assert_close(
        map_inner_product(&synthesized, &map),
        harmonic_inner_product(plan.harmonics(), &alm, &synthesis_adjoint),
        5e-13,
    );
    assert_close(
        harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
        map_inner_product(&map, &analysis_adjoint),
        5e-13,
    );
}

#[test]
#[allow(clippy::excessive_precision)]
fn clenshaw_curtis_deriv1_and_adjoint_match_ducc() {
    // DUCC references: ducc0==0.41.0, CC 4x5, phi0=0.37, one thread.
    let plan = make_cc_plan(2, 2, 4, 5, 0.37);
    let alm = coefficients(plan.harmonics());
    let mut theta_derivative = vec![0.0; plan.sample_count];
    let mut phi_derivative = vec![0.0; plan.sample_count];
    let mut workspace = plan.workspace().unwrap();

    plan.synthesis_deriv1_into(
        &alm,
        &mut theta_derivative,
        &mut phi_derivative,
        &mut workspace,
    )
    .unwrap();
    for (index, expected_theta, expected_phi) in [
        (0, 0.189_187_523_633_865_48, 0.397_551_229_793_163_65),
        (5, 0.045_449_220_099_812_365, 0.848_041_919_092_327_8),
        (10, -0.248_109_173_647_102_92, 0.743_707_688_160_535_6),
        (19, 0.351_033_268_398_765_7, -0.469_131_834_353_264_5),
    ] {
        assert_close(theta_derivative[index], expected_theta, 8e-14);
        assert_close(phi_derivative[index], expected_phi, 8e-14);
    }

    let mut first_input = vec![0.0; plan.sample_count];
    let mut second_input = vec![0.0; plan.sample_count];
    for i in 0..plan.grid.ntheta() {
        for j in 0..plan.grid.nphi() {
            let index = i * plan.grid.nphi() + j;
            first_input[index] = (0.17 * i as f64 + 0.23 * j as f64).sin()
                + 0.2 * (0.11 * i as f64 - 0.31 * j as f64).cos();
            second_input[index] = (0.19 * i as f64 - 0.13 * j as f64).cos()
                - 0.15 * (0.29 * i as f64 + 0.07 * j as f64).sin();
        }
    }
    let mut adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.adjoint_synthesis_deriv1_into(&first_input, &second_input, &mut adjoint, &mut workspace)
        .unwrap();
    let expected_adjoint = [
        Complex64::new(0.0, 0.0),
        Complex64::new(-3.308_183_563_850_672, 0.0),
        Complex64::new(0.533_803_701_067_148_1, 0.0),
        Complex64::new(0.045_294_896_873_528_77, -0.155_975_012_277_048_55),
        Complex64::new(0.505_392_537_836_259_8, -0.323_413_117_328_060_7),
        Complex64::new(-0.028_628_937_979_647_68, 0.013_394_692_285_793_142),
    ];
    for (&actual, expected) in adjoint.iter().zip(expected_adjoint) {
        assert_complex_close(actual, expected, 8e-13);
    }
    assert_close(
        map_inner_product(&theta_derivative, &first_input)
            + map_inner_product(&phi_derivative, &second_input),
        harmonic_inner_product(plan.harmonics(), &alm, &adjoint),
        2e-12,
    );
}

#[test]
#[allow(clippy::excessive_precision)]
fn fejer1_scalar_operations_match_ducc_and_adjoint_identities() {
    // DUCC references: ducc0==0.41.0, F1 3x5, phi0=0.37, one thread.
    let plan = make_f1_plan(2, 2, 3, 5, 0.37);
    let alm = coefficients(plan.harmonics());
    let map = arbitrary_map(&plan);
    let mut workspace = plan.workspace().unwrap();
    let mut synthesized = vec![0.0; plan.sample_count];
    plan.synthesis_into(&alm, &mut synthesized, &mut workspace)
        .unwrap();
    for (&actual, &expected) in synthesized.iter().zip(analytic_map(&plan, &alm).iter()) {
        assert_close(actual, expected, 5e-14);
    }
    for (index, expected) in [
        (0, 0.314_342_495_875_079_34),
        (3, 0.080_714_992_574_932_31),
        (7, 0.324_314_581_387_524_2),
        (14, 0.029_654_993_002_577_9),
    ] {
        assert_close(synthesized[index], expected, 5e-14);
    }

    let mut analyzed = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.analysis_into(&map, &mut analyzed, &mut workspace)
        .unwrap();
    let expected_analysis = [
        Complex64::new(2.526_335_455_435_708_7, 0.0),
        Complex64::new(-0.329_194_184_006_445_57, 0.0),
        Complex64::new(-0.012_691_648_946_562_495, 0.0),
        Complex64::new(0.217_297_712_402_338_9, -0.431_291_149_674_174_2),
        Complex64::new(-0.002_838_643_247_449_916, -0.044_675_978_762_751_395),
        Complex64::new(-0.135_929_960_056_900_4, 0.213_978_455_917_123_7),
    ];
    for (&actual, expected) in analyzed.iter().zip(expected_analysis) {
        assert_complex_close(actual, expected, 8e-13);
    }

    let mut synthesis_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.adjoint_synthesis_into(&map, &mut synthesis_adjoint, &mut workspace)
        .unwrap();
    let expected_synthesis_adjoint = [
        Complex64::new(3.007_121_966_993_341, 0.0),
        Complex64::new(-0.589_419_918_244_447_6, 0.0),
        Complex64::new(1.659_728_243_091_294, 0.0),
        Complex64::new(0.204_545_838_622_175_46, -0.405_907_874_492_659_64),
        Complex64::new(-0.003_176_606_984_808_496_3, -0.049_995_020_092_221_82),
        Complex64::new(-0.121_563_609_841_810_32, 0.191_357_035_312_117_95),
    ];
    for (&actual, expected) in synthesis_adjoint.iter().zip(expected_synthesis_adjoint) {
        assert_complex_close(actual, expected, 8e-13);
    }
    assert_close(
        map_inner_product(&synthesized, &map),
        harmonic_inner_product(plan.harmonics(), &alm, &synthesis_adjoint),
        2e-12,
    );

    let mut analysis_adjoint = vec![0.0; plan.sample_count];
    plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
        .unwrap();
    for (index, expected) in [
        (0, 0.221_949_056_029_639_9),
        (3, 0.013_177_515_252_603_91),
        (7, 0.442_143_828_934_253_97),
        (14, -0.089_168_661_536_922_03),
    ] {
        assert_close(analysis_adjoint[index], expected, 8e-13);
    }
    assert_close(
        harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
        map_inner_product(&map, &analysis_adjoint),
        2e-12,
    );

    let mut first_input = map.clone();
    let mut second_input = vec![0.0; plan.sample_count];
    for i in 0..plan.grid.ntheta() {
        for j in 0..plan.grid.nphi() {
            let index = i * plan.grid.nphi() + j;
            first_input[index] = (0.17 * i as f64 + 0.23 * j as f64).sin()
                + 0.2 * (0.11 * i as f64 - 0.31 * j as f64).cos();
            second_input[index] = (0.19 * i as f64 - 0.13 * j as f64).cos()
                - 0.15 * (0.29 * i as f64 + 0.07 * j as f64).sin();
        }
    }
    let mut theta_derivative = vec![0.0; plan.sample_count];
    let mut phi_derivative = vec![0.0; plan.sample_count];
    plan.synthesis_deriv1_into(
        &alm,
        &mut theta_derivative,
        &mut phi_derivative,
        &mut workspace,
    )
    .unwrap();
    for (index, expected_theta, expected_phi) in [
        (0, 0.314_714_294_731_826_5, 0.673_782_712_223_989_2),
        (3, 0.102_519_439_228_165_21, -0.490_236_302_038_752_24),
        (7, 0.337_281_414_970_432_6, 0.752_572_816_780_536_8),
        (14, 0.835_739_806_187_284_8, -0.434_961_404_658_846_7),
    ] {
        assert_close(theta_derivative[index], expected_theta, 8e-14);
        assert_close(phi_derivative[index], expected_phi, 8e-14);
    }
    let mut derivative_adjoint = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.adjoint_synthesis_deriv1_into(
        &first_input,
        &second_input,
        &mut derivative_adjoint,
        &mut workspace,
    )
    .unwrap();
    let expected_derivative_adjoint = [
        Complex64::new(0.0, 0.0),
        Complex64::new(-3.479_659_593_771_595, 0.0),
        Complex64::new(1.141_406_763_641_841_2, 0.0),
        Complex64::new(0.087_222_449_069_814_26, -0.071_936_864_630_359_4),
        Complex64::new(0.137_370_444_355_236_3, 0.044_720_398_234_314_444),
        Complex64::new(-0.081_778_044_950_258_63, -0.000_976_861_073_657_549_6),
    ];
    for (&actual, expected) in derivative_adjoint.iter().zip(expected_derivative_adjoint) {
        assert_complex_close(actual, expected, 8e-13);
    }
    assert_close(
        map_inner_product(&theta_derivative, &first_input)
            + map_inner_product(&phi_derivative, &second_input),
        harmonic_inner_product(plan.harmonics(), &alm, &derivative_adjoint),
        2e-12,
    );

    let undersampled = make_f1_plan(2, 2, 2, 5, 0.37);
    let mut undersampled_alm = vec![Complex64::new(0.0, 0.0); undersampled.harmonics().nalm()];
    let mut undersampled_workspace = undersampled.workspace().unwrap();
    assert_eq!(
        undersampled.analysis_into(
            &arbitrary_map(&undersampled),
            &mut undersampled_alm,
            &mut undersampled_workspace,
        ),
        Err(ShtError::AnalysisLatitudeUndersampled { lmax: 2, ntheta: 2 })
    );
}

#[test]
#[allow(clippy::excessive_precision)]
fn fejer1_analysis_accepts_minimum_even_longitudes() {
    // DUCC references: ducc0==0.41.0, F1 3x6, phi0=0.37, one thread.
    let plan = make_f1_plan(2, 2, 3, 6, 0.37);
    let map = arbitrary_map(&plan);
    let alm = coefficients(plan.harmonics());
    let mut workspace = plan.workspace().unwrap();
    let mut analyzed = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.analysis_into(&map, &mut analyzed, &mut workspace)
        .unwrap();
    let expected = [
        Complex64::new(2.689_316_937_951_824, 0.0),
        Complex64::new(-0.299_440_230_160_603_4, 0.0),
        Complex64::new(-0.013_893_099_564_934_452, 0.0),
        Complex64::new(0.224_633_317_762_154_5, -0.435_041_605_183_639_2),
        Complex64::new(-0.010_149_034_800_194_424, -0.063_517_653_564_240_91),
        Complex64::new(-0.093_868_080_667_352_89, 0.212_305_006_847_431_7),
    ];
    for (&actual, expected) in analyzed.iter().zip(expected) {
        assert_complex_close(actual, expected, 1e-12);
    }

    let mut analysis_adjoint = vec![0.0; plan.sample_count];
    plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
        .unwrap();
    for (index, expected) in [
        (0, 0.184_957_546_691_366_58),
        (6, 0.397_620_363_614_759_6),
        (17, -0.097_552_061_176_245_71),
    ] {
        assert_close(analysis_adjoint[index], expected, 1e-12);
    }
    assert_close(
        harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
        map_inner_product(&map, &analysis_adjoint),
        2e-12,
    );
}

#[test]
#[allow(clippy::excessive_precision)]
fn fejer1_oversampled_analysis_matches_ducc() {
    // DUCC references: ducc0==0.41.0, F1 7x5, phi0=0.37, one thread.
    let plan = make_f1_plan(2, 2, 7, 5, 0.37);
    let map = arbitrary_map(&plan);
    let alm = coefficients(plan.harmonics());
    let mut workspace = plan.workspace().unwrap();
    let mut analyzed = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.analysis_into(&map, &mut analyzed, &mut workspace)
        .unwrap();
    let expected = [
        Complex64::new(3.286_660_215_255_482_7, 0.0),
        Complex64::new(-0.509_258_770_448_473_7, 0.0),
        Complex64::new(-0.107_215_381_801_019_3, 0.0),
        Complex64::new(0.194_947_339_035_520_2, -0.271_488_412_725_603_3),
        Complex64::new(0.011_389_941_302_963_157, -0.106_062_635_167_317_38),
        Complex64::new(-0.102_728_760_424_834_82, 0.148_405_483_952_625_12),
    ];
    for (&actual, expected) in analyzed.iter().zip(expected) {
        assert_complex_close(actual, expected, 1e-12);
    }

    let mut analysis_adjoint = vec![0.0; plan.sample_count];
    plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
        .unwrap();
    for (index, expected) in [
        (0, 0.029_402_401_338_101_222),
        (5, 0.123_113_649_028_543_33),
        (34, 0.022_831_874_150_001_888),
    ] {
        assert_close(analysis_adjoint[index], expected, 1e-12);
    }
    assert_close(
        harmonic_inner_product(plan.harmonics(), &analyzed, &alm),
        map_inner_product(&map, &analysis_adjoint),
        2e-12,
    );
}

#[test]
#[allow(clippy::excessive_precision)]
fn clenshaw_curtis_analysis_matches_ducc_without_initial_resampling() {
    // DUCC reference: ducc0==0.41.0, CC 7x5, phi0=0.37, one thread.
    let plan = make_cc_plan(2, 2, 7, 5, 0.37);
    let map = arbitrary_map(&plan);
    let mut coefficients = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut workspace = plan.workspace().unwrap();
    plan.analysis_into(&map, &mut coefficients, &mut workspace)
        .unwrap();
    let expected = [
        Complex64::new(3.312_543_485_375_359_7, 0.0),
        Complex64::new(-0.440_646_384_701_871_9, 0.0),
        Complex64::new(-0.080_662_065_710_822_795, 0.0),
        Complex64::new(0.194_847_818_808_176_12, -0.271_708_531_406_952_56),
        Complex64::new(0.009_293_865_922_364_808, -0.086_372_734_621_443_25),
        Complex64::new(-0.103_237_895_255_551_15, 0.149_175_081_850_113_86),
    ];
    for (&actual, expected) in coefficients.iter().zip(expected) {
        assert_complex_close(actual, expected, 8e-13);
    }

    let alm = self::coefficients(plan.harmonics());
    let mut analysis_adjoint = vec![0.0; plan.sample_count];
    plan.adjoint_analysis_into(&alm, &mut analysis_adjoint, &mut workspace)
        .unwrap();
    for (index, expected) in [
        (0, 0.005_846_025_146_437_542),
        (1, 0.005_846_025_146_437_542),
        (2, 0.005_846_025_146_437_542),
        (3, 0.005_846_025_146_437_542),
        (4, 0.005_846_025_146_437_542),
        (5, 0.100_321_125_155_782_02),
        (10, 0.241_687_658_976_767_94),
        (15, 0.230_819_221_966_427_18),
        (34, 0.012_863_122_572_356_323),
    ] {
        assert_close(analysis_adjoint[index], expected, 8e-13);
    }
}

#[test]
#[expect(
    clippy::excessive_precision,
    reason = "The fixture values retain printed ducc0 0.41.0 outputs."
)]
fn clenshaw_curtis_minimum_even_longitude_count_matches_ducc() {
    // DUCC reference: ducc0==0.41.0, CC 4x6, phi0=0.37, one thread.
    let plan = make_cc_plan(2, 2, 4, 6, 0.37);
    let map = arbitrary_map(&plan);
    let mut coefficients = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let mut workspace = plan.workspace().unwrap();
    plan.analysis_into(&map, &mut coefficients, &mut workspace)
        .unwrap();
    let expected = [
        Complex64::new(2.903_561_518_322_166_4, 0.0),
        Complex64::new(-0.293_423_837_509_477_2, 0.0),
        Complex64::new(-0.030_031_189_191_877_417, 0.0),
        Complex64::new(0.210_231_077_576_941_93, -0.360_541_545_558_988_21),
        Complex64::new(-0.003_831_592_732_144_799_6, -0.034_803_087_930_531_502),
        Complex64::new(-0.089_972_128_440_015_786, 0.193_588_058_937_762_71),
    ];
    for (&actual, expected) in coefficients.iter().zip(expected) {
        assert_complex_close(actual, expected, 8e-13);
    }
    let test_alm = self::coefficients(plan.harmonics());
    let mut analysis_adjoint = vec![0.0; plan.sample_count];
    plan.adjoint_analysis_into(&test_alm, &mut analysis_adjoint, &mut workspace)
        .unwrap();
    assert_close(
        harmonic_inner_product(plan.harmonics(), &coefficients, &test_alm),
        map_inner_product(&map, &analysis_adjoint),
        8e-13,
    );

    let undersampled_longitudes = make_cc_plan(2, 2, 4, 4, 0.37);
    let mut undersampled_workspace = undersampled_longitudes.workspace().unwrap();
    let mut undersampled_coefficients =
        vec![Complex64::new(0.0, 0.0); undersampled_longitudes.harmonics().nalm()];
    let undersampled_map = vec![0.0; undersampled_longitudes.sample_count];
    assert_eq!(
        undersampled_longitudes.analysis_into(
            &undersampled_map,
            &mut undersampled_coefficients,
            &mut undersampled_workspace,
        ),
        Err(ShtError::AnalysisLongitudeUndersampled { mmax: 2, nphi: 4 })
    );

    let undersampled_latitudes = make_cc_plan(2, 2, 3, 5, 0.37);
    let mut latitude_workspace = undersampled_latitudes.workspace().unwrap();
    let mut latitude_coefficients =
        vec![Complex64::new(0.0, 0.0); undersampled_latitudes.harmonics().nalm()];
    let latitude_map = vec![0.0; undersampled_latitudes.sample_count];
    assert_eq!(
        undersampled_latitudes.analysis_into(
            &latitude_map,
            &mut latitude_coefficients,
            &mut latitude_workspace,
        ),
        Err(ShtError::AnalysisLatitudeUndersampled { lmax: 2, ntheta: 3 })
    );
}

#[test]
fn clenshaw_curtis_t255_sectoral_analysis_matches_closed_form() {
    let plan = make_cc_plan(255, 255, 257, 513, 0.37);
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let sectoral_index = plan.harmonics().index(255, 255).unwrap();
    alm[sectoral_index] = Complex64::new(1.0, 0.0);
    let mut workspace = plan.workspace().unwrap();
    let mut map = vec![0.0; plan.sample_count];
    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();

    let mut sectoral_norm = 1.0 / (4.0 * PI).sqrt();
    for degree in 1..=255 {
        sectoral_norm *= ((2 * degree + 1) as f64 / (2 * degree) as f64).sqrt();
    }
    for (ring, &theta) in plan.grid.theta().iter().enumerate() {
        let amplitude = -2.0 * sectoral_norm * theta.sin().powi(255);
        for longitude in 0..plan.grid.nphi() {
            let phi = plan.grid.phi0() + TAU * longitude as f64 / plan.grid.nphi() as f64;
            let expected = amplitude * (255.0 * phi).cos();
            assert_close(map[ring * plan.grid.nphi() + longitude], expected, 4e-12);
        }
    }

    let mut recovered = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.analysis_into(&map, &mut recovered, &mut workspace)
        .unwrap();
    for (index, &actual) in recovered.iter().enumerate() {
        let expected = if index == sectoral_index { 1.0 } else { 0.0 };
        assert!(
            (actual.re - expected).abs() <= 2e-9 && actual.im.abs() <= 2e-9,
            "coefficient {index}: actual={actual:?}, expected real={expected}"
        );
    }
}

#[test]
fn fejer1_t255_sectoral_analysis_matches_closed_form() {
    let plan = make_f1_plan(255, 255, 256, 513, 0.37);
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    let sectoral_index = plan.harmonics().index(255, 255).unwrap();
    alm[sectoral_index] = Complex64::new(1.0, 0.0);
    let mut workspace = plan.workspace().unwrap();
    let mut map = vec![0.0; plan.sample_count];
    plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();

    let mut sectoral_norm = 1.0 / (4.0 * PI).sqrt();
    for degree in 1..=255 {
        sectoral_norm *= ((2 * degree + 1) as f64 / (2 * degree) as f64).sqrt();
    }
    for (ring, &theta) in plan.grid.theta().iter().enumerate() {
        let amplitude = -2.0 * sectoral_norm * theta.sin().powi(255);
        for longitude in 0..plan.grid.nphi() {
            let phi = plan.grid.phi0() + TAU * longitude as f64 / plan.grid.nphi() as f64;
            let expected = amplitude * (255.0 * phi).cos();
            assert_close(map[ring * plan.grid.nphi() + longitude], expected, 4e-12);
        }
    }

    let mut recovered = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    plan.analysis_into(&map, &mut recovered, &mut workspace)
        .unwrap();
    for (index, &actual) in recovered.iter().enumerate() {
        let expected = if index == sectoral_index { 1.0 } else { 0.0 };
        assert!(
            (actual.re - expected).abs() <= 2e-9 && actual.im.abs() <= 2e-9,
            "coefficient {index}: actual={actual:?}, expected real={expected}"
        );
    }
}

#[test]
#[ignore = "M3 high-bandlimit acceptance; run this check in release mode"]
#[expect(
    clippy::excessive_precision,
    reason = "The literals retain printed DUCC 0.41.0 reference values."
)]
fn m3_scalar_t511_t1023_t2047_all_grids_matches_ducc_and_closed_form() {
    // DUCC 0.41.0 references use a_(L,L)=1, phi0=0.37, nthreads=1.
    // The five values are the middle-ring samples at j=[0,1,17,nphi/2,nphi-1].
    for degree in [511, 1023, 2047] {
        let nphi = 2 * degree + 3;
        for kind in [
            GridKind::GaussLegendre,
            GridKind::ClenshawCurtis,
            GridKind::Fejer1,
        ] {
            let ntheta = degree + usize::from(kind == GridKind::ClenshawCurtis) + 1;
            let plan = ShtPlan::new(
                GridSpec::new(kind, ntheta, nphi, 0.37).unwrap(),
                HarmonicSpec::scalar(degree, degree).unwrap(),
            )
            .unwrap();
            let sectoral_index = plan.harmonics().index(degree, degree).unwrap();
            let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
            alm[sectoral_index] = Complex64::new(1.0, 0.0);
            let mut workspace = plan.workspace().unwrap();
            plan.prepare_workspace(&mut workspace, 1).unwrap();
            let mut map = vec![0.0; plan.sample_count];
            plan.synthesis_into(&alm, &mut map, &mut workspace).unwrap();

            let expected_ducc = match (degree, kind) {
                (511, GridKind::GaussLegendre) => [
                    -2.3872256264278287,
                    2.4013318896018663,
                    2.5986634372168815,
                    1.5560946744052795,
                    2.3729175336200483,
                ],
                (511, GridKind::ClenshawCurtis) => [
                    -2.3929622710868896,
                    2.4071024324473411,
                    2.6049081794661149,
                    1.5598340621297142,
                    2.3786197950841248,
                ],
                (511, GridKind::Fejer1) => [
                    -2.3872144245647005,
                    2.4013206215462395,
                    2.598651243198931,
                    1.5560873725569058,
                    2.3729063988964887,
                ],
                (1023, GridKind::GaussLegendre) => [
                    -0.17570114687664429,
                    0.19125253260213948,
                    0.43930098463756251,
                    3.3817703089638673,
                    0.1601460438121452,
                ],
                (1023, GridKind::ClenshawCurtis) => [
                    -0.17591254311800286,
                    0.19148263961772272,
                    0.4398295331338507,
                    3.3858391129822953,
                    0.1603387248067325,
                ],
                (1023, GridKind::Fejer1) => [
                    -0.17570094045838802,
                    0.19125230791371206,
                    0.43930046853539056,
                    3.3817663359725523,
                    0.16014585566842582,
                ],
                (2047, GridKind::GaussLegendre) => [
                    3.887238035280272,
                    -3.8896657567270916,
                    -3.9257015210558133,
                    -1.0642856103322809,
                    -3.884789743029231,
                ],
                (2047, GridKind::ClenshawCurtis) => [
                    3.889578098030304,
                    -3.8920072809313684,
                    -3.9280647382845477,
                    -1.0649262953352392,
                    -3.8871283319416836,
                ],
                (2047, GridKind::Fejer1) => [
                    3.887236892735931,
                    -3.889664613469189,
                    -3.925700367206211,
                    -1.0642852975154442,
                    -3.8847886012044954,
                ],
                _ => unreachable!(),
            };
            let middle_ring = ntheta / 2;
            let longitudes = [0, 1, 17, nphi / 2, nphi - 1];
            let mut ducc_max_error: f64 = 0.0;
            for (longitude, expected) in longitudes.into_iter().zip(expected_ducc) {
                let error = (map[middle_ring * nphi + longitude] - expected).abs();
                ducc_max_error = ducc_max_error.max(error);
                assert!(error <= 5e-11, "{kind:?} T{degree} DUCC error={error:.3e}");
            }

            let mut sectoral_norm = 1.0 / (4.0 * PI).sqrt();
            for l in 1..=degree {
                sectoral_norm *= ((2 * l + 1) as f64 / (2 * l) as f64).sqrt();
            }
            let sign = if degree.is_multiple_of(2) { 1.0 } else { -1.0 };
            let (mut analytic_max_error, mut analytic_error_squares) = (0.0_f64, 0.0_f64);
            let mut analytic_sample_count = 0usize;
            for (ring, &theta) in plan.grid.theta().iter().enumerate() {
                let amplitude = 2.0 * sign * sectoral_norm * theta.sin().powi(degree as i32);
                for longitude in 0..nphi {
                    let phi = plan.grid.longitude(longitude).unwrap();
                    let expected = amplitude * (degree as f64 * phi).cos();
                    let error = (map[ring * nphi + longitude] - expected).abs();
                    analytic_max_error = analytic_max_error.max(error);
                    analytic_error_squares += error * error;
                    analytic_sample_count += 1;
                    assert!(
                        error <= 5e-10,
                        "{kind:?} T{degree} analytic map error={error:.3e}"
                    );
                }
            }

            let mut recovered = vec![Complex64::new(0.0, 0.0); alm.len()];
            plan.analysis_into(&map, &mut recovered, &mut workspace)
                .unwrap();
            let (mut coefficient_max_error, mut coefficient_error_squares) = (0.0_f64, 0.0_f64);
            for (index, actual) in recovered.iter().enumerate() {
                let expected = if index == sectoral_index { 1.0 } else { 0.0 };
                let error = (actual - Complex64::new(expected, 0.0)).norm();
                coefficient_max_error = coefficient_max_error.max(error);
                coefficient_error_squares += error * error;
                assert!(
                    error <= 2e-7,
                    "{kind:?} T{degree} coefficient {index}: actual={actual:?}, expected={expected}"
                );
            }
            eprintln!(
                "M3 scalar T{degree} {kind:?}: DUCC middle-row max abs={ducc_max_error:.3e}; analytic map max abs={analytic_max_error:.3e}, RMS={:.3e}; analysis coefficient max abs={coefficient_max_error:.3e}, RMS={:.3e}",
                (analytic_error_squares / analytic_sample_count as f64).sqrt(),
                (coefficient_error_squares / recovered.len() as f64).sqrt(),
            );
        }
    }
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "focused single-thread timing screen for the ordered scalar candidate"]
fn scalar_ordered_candidate_t255_timing_screen() {
    use std::hint::black_box;
    use std::time::Instant;

    fn sample_ms(mut operation: impl FnMut()) -> Vec<f64> {
        operation();
        (0..5)
            .map(|_| {
                let started = Instant::now();
                operation();
                started.elapsed().as_secs_f64() * 1e3
            })
            .collect()
    }

    fn prepare_latitudes(plan: &ShtPlan, workspace: &mut crate::workspace::ShtWorkspace) {
        for (ring, &theta) in plan.grid.theta().iter().enumerate() {
            (workspace.theta_sin[ring], workspace.theta_cos[ring]) =
                crate::legendre::sin_cos_from_f64(theta);
            workspace.ring_order_limits[ring] =
                <f64 as crate::legendre::ShtScalar>::polar_order_limit(
                    plan.harmonics.lmax(),
                    0,
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
        }
    }

    fn candidate_legendre(
        plan: &ShtPlan,
        alm: &[Complex64],
        workspace: &mut crate::workspace::ShtWorkspace,
    ) {
        let recurrence = &mut workspace.recurrence_workers[0];
        let mut scratch = crate::legendre::scalar_ordered::ScalarOrderedScratch {
            epsilon: &mut recurrence.epsilon,
            alpha: &mut recurrence.spin_alpha,
            coefficient_a: &mut recurrence.spin_coefficient_a,
            coefficient_b: &mut recurrence.spin_coefficient_b,
            coefficient_even: &mut recurrence.fast_spin_first,
            coefficient_odd: &mut recurrence.fast_spin_second,
            packed_sin: &mut recurrence.compact_theta_sin,
            packed_cos: &mut recurrence.compact_theta_cos,
            packed_csq: &mut recurrence.spin_current_plus,
            packed_north: &mut recurrence.compact_ring_indices,
            previous: &mut recurrence.previous,
            current: &mut recurrence.current,
            scale: &mut recurrence.exponents,
            corfac: &mut recurrence.scalar_order_correction,
            sum_even_re: &mut recurrence.analysis_same_real,
            sum_even_im: &mut recurrence.analysis_same_imag,
            sum_odd_re: &mut recurrence.analysis_opposite_real,
            sum_odd_im: &mut recurrence.analysis_opposite_imag,
        };
        assert!(
            <f64 as crate::legendre::ShtScalar>::synthesize_scalar_ordered_dispatched(
                pulp::Arch::new(),
                alm,
                plan.harmonics.lmax(),
                plan.harmonics.mmax(),
                plan.grid.nphi(),
                plan.fft.spectrum_len(),
                plan.grid.phi0(),
                &plan.sectoral_norms,
                &workspace.theta_sin[..plan.grid.ntheta()],
                &workspace.theta_cos[..plan.grid.ntheta()],
                &workspace.ring_order_limits[..plan.grid.ntheta()],
                &mut workspace.spectra,
                &mut scratch,
            )
        );
    }

    fn run_case(lmax: usize, ntheta: usize, nphi: usize) {
        let plan = make_plan(lmax, lmax, ntheta, nphi, 0.37);
        assert!(plan.scalar_legendre_table.set(None).is_ok());
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics.nalm()];
        for m in 0..=lmax {
            for l in m..=lmax {
                let re = ((l * 7 + m * 11) % 19) as f64 - 9.0;
                let im = if m == 0 {
                    0.0
                } else {
                    ((l * 13 + m * 5) % 23) as f64 - 11.0
                };
                alm[plan.harmonics.index(l, m).unwrap()] = Complex64::new(re * 1e-5, im * 1e-5);
            }
        }

        let one_thread = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let two_threads = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let mut candidate_workspace = one_thread.install(|| plan.workspace()).unwrap();
        one_thread
            .install(|| plan.prepare_workspace(&mut candidate_workspace, 1))
            .unwrap();
        let mut reference_workspace = two_threads.install(|| plan.workspace()).unwrap();
        two_threads
            .install(|| plan.prepare_workspace(&mut reference_workspace, 2))
            .unwrap();
        prepare_latitudes(&plan, &mut candidate_workspace);
        prepare_latitudes(&plan, &mut reference_workspace);

        candidate_legendre(&plan, &alm, &mut candidate_workspace);
        plan.synthesize_ring_group(
            &alm,
            plan.grid.nphi(),
            plan.fft.spectrum_len(),
            plan.grid.phi0(),
            0,
            &mut reference_workspace.spectra,
            &reference_workspace.theta_sin[..plan.grid.ntheta()],
            &reference_workspace.theta_cos[..plan.grid.ntheta()],
            &reference_workspace.ring_order_limits[..plan.grid.ntheta()],
            pulp::Arch::new(),
            &mut reference_workspace.recurrence_workers[0],
        )
        .unwrap();
        assert!(
            relative_l2_complex(&candidate_workspace.spectra, &reference_workspace.spectra) < 3e-13
        );

        let candidate_legendre_ms =
            sample_ms(|| candidate_legendre(&plan, &alm, &mut candidate_workspace));
        let reference_legendre_ms = sample_ms(|| {
            reference_workspace.spectra.fill(Complex64::new(0.0, 0.0));
            plan.synthesize_ring_group(
                &alm,
                plan.grid.nphi(),
                plan.fft.spectrum_len(),
                plan.grid.phi0(),
                0,
                &mut reference_workspace.spectra,
                &reference_workspace.theta_sin[..plan.grid.ntheta()],
                &reference_workspace.theta_cos[..plan.grid.ntheta()],
                &reference_workspace.ring_order_limits[..plan.grid.ntheta()],
                pulp::Arch::new(),
                &mut reference_workspace.recurrence_workers[0],
            )
            .unwrap();
            black_box(reference_workspace.spectra[0]);
        });

        let mut candidate_map = vec![0.0; plan.sample_count];
        let mut reference_map = vec![0.0; plan.sample_count];
        let candidate_full_ms = sample_ms(|| {
            one_thread
                .install(|| plan.synthesis_into(&alm, &mut candidate_map, &mut candidate_workspace))
                .unwrap();
            black_box(candidate_map[0]);
        });
        let reference_full_ms = sample_ms(|| {
            one_thread
                .install(|| plan.synthesis_into(&alm, &mut reference_map, &mut reference_workspace))
                .unwrap();
            black_box(reference_map[0]);
        });
        assert!(relative_l2_real(&candidate_map, &reference_map) < 3e-13);
        eprintln!(
            "SCALAR_ORDERED_T{lmax} grid={ntheta}x{nphi} f64 q1 samples=5 legendre_candidate_ms={candidate_legendre_ms:?} legendre_ring_group_ms={reference_legendre_ms:?} full_candidate_ms={candidate_full_ms:?} full_ring_group_ms={reference_full_ms:?}"
        );
    }

    run_case(255, 256, 513);
    run_case(300, 301, 603);
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "test-only odd-length inverse phase attribution at T720 q8"]
fn plan14_odd_inverse_phase_attribution() {
    use std::hint::black_box;
    use std::time::Instant;

    let plan = make_plan(720, 180, 721, 361, 0.23);
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    for m in 0..=plan.harmonics().mmax() {
        for l in m..=plan.harmonics().lmax() {
            let re = ((l * 17 + m * 13) % 61) as f64 - 30.0;
            let im = if m == 0 {
                0.0
            } else {
                ((l * 19 + m * 7) % 67) as f64 - 33.0
            };
            alm[plan.harmonics().index(l, m).unwrap()] = Complex64::new(re * 1.0e-7, im * 1.0e-7);
        }
    }

    let q8 = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let mut workspace = q8.install(|| plan.workspace()).unwrap();
    q8.install(|| plan.prepare_workspace(&mut workspace, 8))
        .unwrap();
    let mut map = vec![0.0; plan.sample_count];

    for _ in 0..3 {
        q8.install(|| plan.synthesis_into(&alm, &mut map, &mut workspace))
            .unwrap();
    }

    let mut uninstrumented = Vec::with_capacity(10);
    for _ in 0..10 {
        let started = Instant::now();
        q8.install(|| plan.synthesis_into(&alm, &mut map, &mut workspace))
            .unwrap();
        uninstrumented.push(started.elapsed().as_secs_f64() * 1.0e3);
        black_box(map[0]);
    }
    for fft_workspace in &mut workspace.fft_worker_workspaces {
        fft_workspace.set_odd_inverse_phase_enabled(true);
    }
    let mut instrumented = Vec::with_capacity(10);
    for _ in 0..10 {
        let started = Instant::now();
        q8.install(|| plan.synthesis_into(&alm, &mut map, &mut workspace))
            .unwrap();
        instrumented.push(started.elapsed().as_secs_f64() * 1.0e3);
        black_box(map[0]);
    }
    let median = |values: &mut [f64]| {
        values.sort_by(f64::total_cmp);
        values[values.len() / 2]
    };
    let mut uninstrumented_median = uninstrumented.clone();
    let mut instrumented_median = instrumented.clone();
    eprintln!(
        "PLAN14_ODD_FFT_TIMER_OVERHEAD uninstrumented_ms={:.6} instrumented_ms={:.6} uninstrumented_calls=10 instrumented_calls=10",
        median(&mut uninstrumented_median),
        median(&mut instrumented_median),
    );

    const BATCH_COUNT: usize = 30;
    const CALLS_PER_BATCH: usize = 16;
    let mut batch_wall = Vec::with_capacity(BATCH_COUNT);
    let mut batch_inverse = Vec::with_capacity(BATCH_COUNT);
    let mut max_worker_total = Vec::with_capacity(BATCH_COUNT);
    let mut component_cpu = [
        Vec::with_capacity(BATCH_COUNT),
        Vec::with_capacity(BATCH_COUNT),
        Vec::with_capacity(BATCH_COUNT),
    ];
    let mut component_max_worker = [
        Vec::with_capacity(BATCH_COUNT),
        Vec::with_capacity(BATCH_COUNT),
        Vec::with_capacity(BATCH_COUNT),
    ];

    for batch in 0..BATCH_COUNT {
        for fft_workspace in &mut workspace.fft_worker_workspaces {
            fft_workspace.take_odd_inverse_phase_ns();
        }
        let phases = workspace.observe_transform_phases();
        let started = Instant::now();
        for _ in 0..CALLS_PER_BATCH {
            q8.install(|| plan.synthesis_into(&alm, &mut map, &mut workspace))
                .unwrap();
            black_box(map[0]);
        }
        let wall_ms = started.elapsed().as_secs_f64() * 1.0e3 / CALLS_PER_BATCH as f64;
        let phases = phases.lock().unwrap();
        let inverse_ms = phases
            .iter()
            .filter(|observation| observation.phase == "inverse longitude FFT and map writeback")
            .map(|observation| observation.elapsed_ns as f64 * 1.0e-6)
            .sum::<f64>()
            / CALLS_PER_BATCH as f64;
        drop(phases);

        let mut worker_components = Vec::with_capacity(workspace.fft_worker_workspaces.len());
        for fft_workspace in &mut workspace.fft_worker_workspaces {
            worker_components.push(fft_workspace.take_odd_inverse_phase_ns());
        }
        let sums = std::array::from_fn::<_, 3, _>(|stage| {
            worker_components
                .iter()
                .map(|components| components[stage])
                .sum::<u64>() as f64
                / CALLS_PER_BATCH as f64
                / 1.0e3
        });
        let maxima = std::array::from_fn::<_, 3, _>(|stage| {
            worker_components
                .iter()
                .map(|components| components[stage])
                .max()
                .unwrap_or(0) as f64
                / CALLS_PER_BATCH as f64
                / 1.0e3
        });
        let max_worker_total_us = worker_components
            .iter()
            .map(|components| components.iter().sum::<u64>())
            .max()
            .unwrap_or(0) as f64
            / CALLS_PER_BATCH as f64
            / 1.0e3;
        batch_wall.push(wall_ms);
        batch_inverse.push(inverse_ms);
        max_worker_total.push(max_worker_total_us);
        for stage in 0..3 {
            component_cpu[stage].push(sums[stage]);
            component_max_worker[stage].push(maxima[stage]);
        }
        eprintln!(
            "PLAN14_ODD_FFT_BATCH batch={batch} calls={CALLS_PER_BATCH} wall_ms={wall_ms:.6} inverse_stage_ms={inverse_ms:.6} pack_cpu_us={:.3} fft_cpu_us={:.3} output_cpu_us={:.3} max_worker_pack_us={:.3} max_worker_fft_us={:.3} max_worker_output_us={:.3} max_worker_total_us={max_worker_total_us:.3}",
            sums[0], sums[1], sums[2], maxima[0], maxima[1], maxima[2],
        );
    }

    let summarize = |values: &[f64]| {
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        (
            sorted[(sorted.len() - 1) / 10],
            sorted[sorted.len() / 2],
            sorted[(sorted.len() - 1) * 9 / 10],
        )
    };
    let (wall_p10, wall_median, wall_p90) = summarize(&batch_wall);
    let (inverse_p10, inverse_median, inverse_p90) = summarize(&batch_inverse);
    let (worker_p10, worker_median, worker_p90) = summarize(&max_worker_total);
    eprintln!(
        "PLAN14_ODD_FFT_SUMMARY cases=30 calls_per_batch=16 wall_ms_p10_median_p90={wall_p10:.6},{wall_median:.6},{wall_p90:.6} inverse_stage_ms_p10_median_p90={inverse_p10:.6},{inverse_median:.6},{inverse_p90:.6} max_worker_total_us_p10_median_p90={worker_p10:.3},{worker_median:.3},{worker_p90:.3}"
    );
    for (stage, name) in ["pack", "complex_inverse", "real_output"]
        .iter()
        .enumerate()
    {
        let (p10, med, p90) = summarize(&component_cpu[stage]);
        let (max_p10, max_med, max_p90) = summarize(&component_max_worker[stage]);
        eprintln!(
            "PLAN14_ODD_FFT_COMPONENT name={name} summed_worker_cpu_us_p10_median_p90={p10:.3},{med:.3},{p90:.3} max_worker_us_p10_median_p90={max_p10:.3},{max_med:.3},{max_p90:.3}"
        );
    }

    let q1 = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let mut q1_workspace = q1.install(|| plan.workspace()).unwrap();
    q1.install(|| plan.prepare_workspace(&mut q1_workspace, 1))
        .unwrap();
    let mut q1_map = vec![0.0; plan.sample_count];
    q1.install(|| plan.synthesis_into(&alm, &mut q1_map, &mut q1_workspace))
        .unwrap();
    assert!(relative_l2_real(&q1_map, &map) < 4e-13);
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "light q1/q4/q8 screen for the ordered scalar parallel route"]
fn scalar_ordered_parallel_order_timing_screen() {
    use std::hint::black_box;
    use std::time::Instant;

    let run_case = |lmax, mmax, ntheta, nphi| {
        let plan = make_plan(lmax, mmax, ntheta, nphi, 0.37);
        assert!(plan.scalar_legendre_table.set(None).is_ok());
        let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
        for m in 0..=plan.harmonics().mmax() {
            for l in m..=plan.harmonics().lmax() {
                let re = ((l * 7 + m * 11) % 19) as f64 - 9.0;
                let im = if m == 0 {
                    0.0
                } else {
                    ((l * 13 + m * 5) % 23) as f64 - 11.0
                };
                alm[plan.harmonics().index(l, m).unwrap()] = Complex64::new(re * 1e-6, im * 1e-6);
            }
        }

        let one_thread = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let four_threads = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let eight_threads = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap();
        let mut q1_workspace = one_thread.install(|| plan.workspace()).unwrap();
        one_thread
            .install(|| plan.prepare_workspace(&mut q1_workspace, 1))
            .unwrap();
        let mut q4_workspace = four_threads.install(|| plan.workspace()).unwrap();
        four_threads
            .install(|| plan.prepare_workspace(&mut q4_workspace, 4))
            .unwrap();
        let mut q8_workspace = eight_threads.install(|| plan.workspace()).unwrap();
        eight_threads
            .install(|| plan.prepare_workspace(&mut q8_workspace, 8))
            .unwrap();
        let mut q1_map = vec![0.0; plan.sample_count];
        let mut q4_map = vec![0.0; plan.sample_count];
        let mut q8_map = vec![0.0; plan.sample_count];

        one_thread
            .install(|| plan.synthesis_into(&alm, &mut q1_map, &mut q1_workspace))
            .unwrap();
        four_threads
            .install(|| plan.synthesis_into(&alm, &mut q4_map, &mut q4_workspace))
            .unwrap();
        eight_threads
            .install(|| plan.synthesis_into(&alm, &mut q8_map, &mut q8_workspace))
            .unwrap();
        assert!(relative_l2_real(&q1_map, &q8_map) < 4e-13);
        assert!(relative_l2_real(&q1_map, &q4_map) < 4e-13);

        let mut q1_samples = Vec::with_capacity(5);
        let mut q4_samples = Vec::with_capacity(5);
        let mut q8_samples = Vec::with_capacity(5);
        for _ in 0..5 {
            let started = Instant::now();
            one_thread
                .install(|| plan.synthesis_into(&alm, &mut q1_map, &mut q1_workspace))
                .unwrap();
            q1_samples.push(started.elapsed().as_secs_f64() * 1e3);
            black_box(q1_map[0]);

            let started = Instant::now();
            four_threads
                .install(|| plan.synthesis_into(&alm, &mut q4_map, &mut q4_workspace))
                .unwrap();
            q4_samples.push(started.elapsed().as_secs_f64() * 1e3);
            black_box(q4_map[0]);

            let started = Instant::now();
            eight_threads
                .install(|| plan.synthesis_into(&alm, &mut q8_map, &mut q8_workspace))
                .unwrap();
            q8_samples.push(started.elapsed().as_secs_f64() * 1e3);
            black_box(q8_map[0]);
        }
        assert!(relative_l2_real(&q1_map, &q8_map) < 4e-13);
        assert!(relative_l2_real(&q1_map, &q4_map) < 4e-13);

        let phase_probe = q8_workspace.observe_transform_phases();
        let order_leaf_probe = q8_workspace.observe_scalar_order_leaves();
        eight_threads
            .install(|| plan.synthesis_into(&alm, &mut q8_map, &mut q8_workspace))
            .unwrap();
        assert!(relative_l2_real(&q1_map, &q8_map) < 4e-13);
        let phases = phase_probe.lock().unwrap().clone();
        let order_leaves = order_leaf_probe.lock().unwrap().clone();
        let mut worker_work = std::collections::BTreeMap::<usize, (usize, u128, u128, u128)>::new();
        for leaf in &order_leaves {
            debug_assert!(leaf.m_start < leaf.m_end);
            let work = worker_work
                .entry(leaf.worker.unwrap_or(usize::MAX))
                .or_default();
            work.0 += leaf.m_end - leaf.m_start;
            work.1 += leaf.active_ring_order_pairs;
            work.2 += leaf.estimated_work;
            work.3 += leaf.elapsed_ns;
        }
        let worker_summary: Vec<_> = worker_work
            .into_iter()
            .map(
                |(worker, (orders, active_pairs, estimated_work, elapsed_ns))| {
                    (
                        worker,
                        orders,
                        active_pairs,
                        estimated_work,
                        elapsed_ns as f64 * 1e-6,
                    )
                },
            )
            .collect();
        eprintln!(
            "SCALAR_ORDERED_T{lmax} grid={ntheta}x{nphi} f64 full synthesis q1_ms={q1_samples:?} q4_ms={q4_samples:?} q8_ms={q8_samples:?} phases={phases:?} order_work_by_worker={worker_summary:?}"
        );
    };

    run_case(300, 150, 301, 603);
    run_case(720, 720, 721, 1442);
}
