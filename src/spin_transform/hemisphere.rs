// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;
use rayon::prelude::*;

use super::{
    SPIN1_PAIR_MIN_SIN_THETA, ShtPlan, ShtScalar, ShtWorkspace, longitude_phase, spin1_to_scalar,
};
use crate::error::ShtError;
use crate::legendre::LATITUDE_BLOCK_SIZE;
use crate::plan::current_execution_thread_count;
use crate::workspace::RecurrenceWorkspace;

/// Selects paired scalar synthesis only in the measured full-band GL window.
pub(super) fn is_beneficial<T: ShtScalar>(plan: &ShtPlan<T>, execution_capacity: usize) -> bool {
    super::spin1_hemisphere_worker_count(plan, execution_capacity).is_some()
}

#[expect(
    clippy::too_many_arguments,
    reason = "The paired group receives both hemisphere outputs and its worker scratch."
)]
fn synthesize_group<T: ShtScalar>(
    plan: &ShtPlan<T>,
    e_alm: &[Complex<T>],
    b_alm: &[Complex<T>],
    coefficient_cache: Option<&[Complex<T>]>,
    ring_start: usize,
    ring_end: usize,
    nphi: usize,
    spectrum_len: usize,
    theta_sin: &[T],
    theta_cos: &[T],
    ring_order_limits: &[usize],
    north_q: &mut [Complex<T>],
    south_q: &mut [Complex<T>],
    north_u: &mut [Complex<T>],
    south_u: &mut [Complex<T>],
    arch: pulp::Arch,
    recurrence: &mut RecurrenceWorkspace<T>,
) -> Result<(), ShtError> {
    #[cfg(all(test, feature = "parallel"))]
    recurrence.record_active_worker();
    let lmax = plan.harmonics.lmax();
    let mmax = plan.harmonics.mmax();
    let ntheta = plan.grid.ntheta();
    let group_len = ring_end - ring_start;
    debug_assert_eq!(north_q.len(), group_len * spectrum_len);
    debug_assert_eq!(south_q.len(), north_q.len());
    debug_assert_eq!(north_u.len(), north_q.len());
    debug_assert_eq!(south_u.len(), north_q.len());
    let zero = T::from_f64(0.0).expect("zero is representable");
    let pair_min_sine =
        T::from_f64(SPIN1_PAIR_MIN_SIN_THETA).expect("spin-1 pairing cutoff is representable");
    let mut south_sin = [zero; LATITUDE_BLOCK_SIZE];
    let mut south_cos = [zero; LATITUDE_BLOCK_SIZE];
    let mut south_limits = [0_usize; LATITUDE_BLOCK_SIZE];

    for m in 0..=mmax {
        let scalar_lmax = lmax + 1;
        let coefficient_count = lmax - m + 1;
        if let Some(coefficients) = coefficient_cache {
            let row_len = lmax + 2;
            let row_stride = 2 * row_len;
            let row_start = m * row_stride;
            recurrence.fast_scalar_first[..row_len]
                .copy_from_slice(&coefficients[row_start..row_start + row_len]);
            recurrence.fast_scalar_second[..row_len]
                .copy_from_slice(&coefficients[row_start + row_len..row_start + row_stride]);
        } else {
            let coefficient_start = plan
                .harmonics
                .index(m, m)
                .expect("each order starts at its diagonal degree");
            spin1_to_scalar(
                m,
                lmax,
                &e_alm[coefficient_start..coefficient_start + coefficient_count],
                Some(&b_alm[coefficient_start..coefficient_start + coefficient_count]),
                &mut recurrence.fast_scalar_first[..lmax + 2],
                &mut recurrence.fast_scalar_second[..lmax + 2],
            );
        }

        T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
        let factors_ready = T::fill_paired_scalar_factors(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        );
        let pair_count = (scalar_lmax - m + 2) / 2;

        for block_start in (0..group_len).step_by(LATITUDE_BLOCK_SIZE) {
            let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(group_len);
            let block_len = block_end - block_start;
            let north_first_ring = ring_start + block_start;
            let north_end_ring = ring_start + block_end;
            let south_first_ring = ntheta - north_end_ring;
            let north_offset = block_start * spectrum_len;
            let north_end = block_end * spectrum_len;
            let south_offset = (group_len - block_end) * spectrum_len;
            let south_end = (group_len - block_start) * spectrum_len;
            let all_lanes_safe = (block_start..block_end).all(|lane| {
                let north = ring_start + lane;
                let south = ntheta - 1 - north;
                ring_order_limits[north] >= m
                    && ring_order_limits[south] >= m
                    && theta_sin[north] >= pair_min_sine
                    && theta_sin[south] >= pair_min_sine
            });

            let first_ready = all_lanes_safe
                && factors_ready
                && T::prepare_paired_scalar_synthesis_coefficients(
                    m,
                    scalar_lmax,
                    &recurrence.epsilon,
                    &recurrence.spin_alpha,
                    &recurrence.fast_scalar_first[m..=scalar_lmax],
                    &mut recurrence.fast_spin_first[..pair_count],
                    &mut recurrence.fast_spin_second[..pair_count],
                );
            let second_ready = all_lanes_safe
                && factors_ready
                && T::prepare_paired_scalar_synthesis_coefficients(
                    m,
                    scalar_lmax,
                    &recurrence.epsilon,
                    &recurrence.spin_alpha,
                    &recurrence.fast_scalar_second[m..=scalar_lmax],
                    &mut recurrence.fast_spin_pair_first[..pair_count],
                    &mut recurrence.fast_spin_pair_second[..pair_count],
                );
            let used_pair = if first_ready && second_ready {
                let mut sums = [
                    &mut recurrence.analysis_same_real[..block_len],
                    &mut recurrence.analysis_same_imag[..block_len],
                    &mut recurrence.analysis_opposite_real[..block_len],
                    &mut recurrence.analysis_opposite_imag[..block_len],
                    &mut recurrence.spin_previous_plus[..block_len],
                    &mut recurrence.spin_current_plus[..block_len],
                    &mut recurrence.spin_next_plus[..block_len],
                    &mut recurrence.spin_previous_minus[..block_len],
                ];
                T::synthesize_hemisphere_pair_block_dispatched(
                    arch,
                    m,
                    scalar_lmax,
                    &theta_sin[north_first_ring..north_end_ring],
                    &theta_cos[north_first_ring..north_end_ring],
                    plan.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &recurrence.fast_spin_first[..pair_count],
                    &recurrence.fast_spin_second[..pair_count],
                    &recurrence.fast_spin_pair_first[..pair_count],
                    &recurrence.fast_spin_pair_second[..pair_count],
                    spectrum_len,
                    &mut north_q[north_offset..north_end],
                    &mut south_q[south_offset..south_end],
                    &mut north_u[north_offset..north_end],
                    &mut south_u[south_offset..south_end],
                    &mut recurrence.current[..block_len],
                    &mut sums,
                )?
            } else {
                false
            };

            if !used_pair {
                plan.synthesize_spin1_scalar_block(
                    m,
                    scalar_lmax,
                    spectrum_len,
                    north_first_ring,
                    false,
                    &theta_sin[north_first_ring..north_end_ring],
                    &theta_cos[north_first_ring..north_end_ring],
                    &ring_order_limits[north_first_ring..north_end_ring],
                    arch,
                    recurrence,
                    &mut north_q[north_offset..north_end],
                    &mut north_u[north_offset..north_end],
                )?;
                for lane in 0..block_len {
                    let north = north_end_ring - 1 - lane;
                    let south = ntheta - 1 - north;
                    south_sin[lane] = theta_sin[south];
                    south_cos[lane] = theta_cos[south];
                    south_limits[lane] = ring_order_limits[south];
                }
                plan.synthesize_spin1_scalar_block(
                    m,
                    scalar_lmax,
                    spectrum_len,
                    south_first_ring,
                    false,
                    &south_sin[..block_len],
                    &south_cos[..block_len],
                    &south_limits[..block_len],
                    arch,
                    recurrence,
                    &mut south_q[south_offset..south_end],
                    &mut south_u[south_offset..south_end],
                )?;
            }
        }

        let phase = longitude_phase::<T>(m, plan.grid.phi0(), false);
        let scale = T::from_usize(nphi).expect("longitude count converts");
        let nyquist_scale = scale * T::from_f64(2.0).expect("two is representable");
        for lane in 0..group_len {
            let north_index = lane * spectrum_len + m;
            let south_index = (group_len - 1 - lane) * spectrum_len + m;
            for (spectra, index) in [
                (&mut *north_q, north_index),
                (&mut *north_u, north_index),
                (&mut *south_q, south_index),
                (&mut *south_u, south_index),
            ] {
                let value = spectra[index] * phase;
                spectra[index] = if nphi.is_multiple_of(2) && m == nphi / 2 {
                    Complex::new(nyquist_scale * value.re, zero)
                } else {
                    value * scale
                };
            }
        }
    }
    Ok(())
}

/// Executes the measured spin-1 STANDARD synthesis strategy using paired rings.
pub(super) fn synthesize<T: ShtScalar>(
    plan: &ShtPlan<T>,
    e_alm: &[Complex<T>],
    b_alm: &[Complex<T>],
    first_component: &mut [T],
    second_component: &mut [T],
    workspace: &mut ShtWorkspace<T>,
) -> Result<(), ShtError> {
    let ntheta = plan.grid.ntheta();
    let nphi = plan.grid.nphi();
    let spectrum_len = plan.fft.spectrum_len();
    let half = ntheta / 2;
    let arch = pulp::Arch::new();
    plan.prepare_spin_latitudes(workspace, 1);
    workspace.spectra.fill(Complex::new(T::zero(), T::zero()));
    workspace
        .spin_spectra
        .fill(Complex::new(T::zero(), T::zero()));

    if !ntheta.is_multiple_of(2) {
        let start = half * spectrum_len;
        plan.synthesize_spin_ring_group(
            e_alm,
            Some(b_alm),
            1,
            false,
            true,
            false,
            None,
            nphi,
            spectrum_len,
            plan.grid.phi0(),
            half,
            half + 1,
            &mut workspace.spectra[start..start + spectrum_len],
            &mut workspace.spin_spectra[start..start + spectrum_len],
            &workspace.theta_sin[half..half + 1],
            &workspace.theta_cos[half..half + 1],
            &workspace.ring_order_limits,
            arch,
            &mut workspace.recurrence_workers[0],
        )?;
    }

    let thread_count = current_execution_thread_count().min(workspace.execution_capacity);
    let target_group_size = half.div_ceil(2 * thread_count).max(1);
    let full_group_count = half / target_group_size;
    let first_group_len = half % target_group_size;
    let group_count = full_group_count + usize::from(first_group_len > 0);
    // General polar/scaled fallback kernels retain their 64-lane workspace
    // contract even when a scheduling group is smaller.
    debug_assert!(group_count.max(1) <= workspace.recurrence_workers.len());

    let coefficient_cache = (thread_count >= 8
        && plan.harmonics.lmax() <= super::SPIN1_SYNTH_COEFFICIENT_CACHE_MAX_LMAX
        && plan.prepare_spin1_scalar_coefficients(
            e_alm,
            b_alm,
            &mut workspace.spin1_scalar_coefficients,
            thread_count,
        ))
    .then_some(workspace.spin1_scalar_coefficients.as_slice());

    let theta_sin = &workspace.theta_sin[..ntheta];
    let theta_cos = &workspace.theta_cos[..ntheta];
    let ring_order_limits = &workspace.ring_order_limits[..ntheta];
    let north_spectrum_len = half * spectrum_len;
    let south_offset = usize::from(!ntheta.is_multiple_of(2)) * spectrum_len;
    let (north_q, q_rest) = workspace.spectra.split_at_mut(north_spectrum_len);
    let (north_u, u_rest) = workspace.spin_spectra.split_at_mut(north_spectrum_len);
    let south_q = &mut q_rest[south_offset..];
    let south_u = &mut u_rest[south_offset..];

    if first_group_len > 0 {
        let first_len = first_group_len * spectrum_len;
        let south_start = (half - first_group_len) * spectrum_len;
        synthesize_group(
            plan,
            e_alm,
            b_alm,
            coefficient_cache,
            0,
            first_group_len,
            nphi,
            spectrum_len,
            theta_sin,
            theta_cos,
            ring_order_limits,
            &mut north_q[..first_len],
            &mut south_q[south_start..],
            &mut north_u[..first_len],
            &mut south_u[south_start..],
            arch,
            &mut workspace.recurrence_workers[0],
        )?;
    }

    let full_start = first_group_len * spectrum_len;
    let full_len = full_group_count * target_group_size * spectrum_len;
    let workers_start = usize::from(first_group_len > 0);
    if full_group_count > 0 {
        north_q[full_start..full_start + full_len]
            .par_chunks_mut(target_group_size * spectrum_len)
            .zip(
                south_q[..full_len]
                    .par_chunks_mut(target_group_size * spectrum_len)
                    .rev(),
            )
            .zip(
                north_u[full_start..full_start + full_len]
                    .par_chunks_mut(target_group_size * spectrum_len),
            )
            .zip(
                south_u[..full_len]
                    .par_chunks_mut(target_group_size * spectrum_len)
                    .rev(),
            )
            .zip(
                workspace.recurrence_workers[workers_start..workers_start + full_group_count]
                    .par_iter_mut(),
            )
            .enumerate()
            .with_min_len(full_group_count.div_ceil(thread_count))
            .try_for_each(|(group_index, item)| {
                let ((((north_q, south_q), north_u), south_u), recurrence) = item;
                let ring_start = first_group_len + group_index * target_group_size;
                let ring_end = ring_start + target_group_size;
                synthesize_group(
                    plan,
                    e_alm,
                    b_alm,
                    coefficient_cache,
                    ring_start,
                    ring_end,
                    nphi,
                    spectrum_len,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    north_q,
                    south_q,
                    north_u,
                    south_u,
                    arch,
                    recurrence,
                )
            })?;
    }

    for ring in 0..ntheta {
        let spectrum_start = ring * spectrum_len;
        let spectrum_end = spectrum_start + spectrum_len;
        plan.fft.inverse(
            &workspace.spectra[spectrum_start..spectrum_end],
            &mut workspace.row,
            &mut workspace.fft,
        )?;
        let output_start = ring * nphi;
        first_component[output_start..output_start + nphi].copy_from_slice(&workspace.row);
        plan.fft.inverse(
            &workspace.spin_spectra[spectrum_start..spectrum_end],
            &mut workspace.row,
            &mut workspace.fft,
        )?;
        second_component[output_start..output_start + nphi].copy_from_slice(&workspace.row);
    }
    Ok(())
}
