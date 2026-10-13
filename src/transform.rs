// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::error::ShtError;
use crate::grid::GridKind;
use crate::legendre::{LATITUDE_BLOCK_SIZE, ShtScalar, longitude_phase, sin_cos_from_f64};
use crate::plan::ShtPlan;
use crate::workspace::{RecurrenceWorkspace, ShtWorkspace};

const SCALAR_TRANSITION_MIN_LMAX: usize = 1023;

fn paired_scalar_coefficients_are_finite<T: ShtScalar>(
    m: usize,
    lmax: usize,
    epsilon: &[T],
    alpha: &[T],
    coefficients: &[Complex<T>],
) -> bool {
    let pair_count = (lmax - m + 2) / 2;
    for (pair, &pair_alpha) in alpha.iter().take(pair_count).enumerate() {
        let l = m + 2 * pair;
        let offset = l - m;
        let next_even = if l + 2 <= lmax {
            coefficients[offset + 2]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        let even =
            (coefficients[offset] * epsilon[l + 1] + next_even * epsilon[l + 2]) * pair_alpha;
        let odd = if l < lmax {
            coefficients[offset + 1] * pair_alpha
        } else {
            Complex::new(T::zero(), T::zero())
        };
        if !even.re.is_finite()
            || !even.im.is_finite()
            || !odd.re.is_finite()
            || !odd.im.is_finite()
        {
            return false;
        }
    }
    true
}

#[cfg(feature = "parallel")]
pub(crate) fn weighted_order_midpoint(lmax: usize, m_start: usize, m_end: usize) -> usize {
    debug_assert!(m_start < m_end);
    debug_assert!(m_end - 1 <= lmax);
    debug_assert!(m_end - m_start > 1);

    let total_work = (m_start..m_end).map(|m| lmax - m + 1).sum::<usize>();
    let mut left_work = 0;
    let mut best_mid = m_start + 1;
    let mut best_difference = usize::MAX;
    for m in m_start..(m_end - 1) {
        left_work += lmax - m + 1;
        let right_work = total_work - left_work;
        let difference = left_work.abs_diff(right_work);
        if difference < best_difference {
            best_difference = difference;
            best_mid = m + 1;
        }
    }
    best_mid
}

#[cfg(feature = "parallel")]
pub(crate) fn active_ring_weighted_order_midpoint(
    lmax: usize,
    m_start: usize,
    m_end: usize,
    ring_order_limits: &[usize],
) -> usize {
    debug_assert!(m_start < m_end);
    debug_assert!(m_end - 1 <= lmax);
    debug_assert!(m_end - m_start > 1);

    let order_work = |m: usize| {
        let active_lanes = ring_order_limits
            .iter()
            .filter(|&&limit| m <= limit)
            .count() as u128;
        (lmax - m + 1) as u128 * active_lanes
    };
    let total_work = (m_start..m_end).map(order_work).sum::<u128>();
    if total_work == 0 {
        return weighted_order_midpoint(lmax, m_start, m_end);
    }

    let mut left_work = 0_u128;
    let mut best_mid = m_start + 1;
    let mut best_difference = u128::MAX;
    for m in m_start..(m_end - 1) {
        left_work += order_work(m);
        let right_work = total_work - left_work;
        let difference = left_work.abs_diff(right_work);
        if difference < best_difference {
            best_difference = difference;
            best_mid = m + 1;
        }
    }
    best_mid
}

fn accumulate_precomputed_synthesis_order<T: ShtScalar>(
    plan: &ShtPlan<T>,
    table: &crate::legendre::ScalarLegendreTable<T>,
    alm: &[Complex<T>],
    ring_order_limits: &[usize],
    m: usize,
    order_row: &mut [Complex<T>],
) -> bool {
    let ntheta = order_row.len();
    let nphi = plan.grid.nphi();
    let lmax = plan.harmonics.lmax();
    for degree_offset in 0..=(lmax - m) {
        let l = m + degree_offset;
        let coefficient = alm[plan
            .harmonics
            .index(l, m)
            .expect("precomputed scalar order is within the harmonic layout")];
        if !table.input_is_safe(coefficient.re) || !table.input_is_safe(coefficient.im) {
            return false;
        }
        let parity = if (l + m).is_multiple_of(2) {
            T::one()
        } else {
            -T::one()
        };
        for (north_ring, &basis) in table.degree_lanes(m, degree_offset).iter().enumerate() {
            let south_ring = ntheta - 1 - north_ring;
            if m <= ring_order_limits[north_ring] {
                order_row[north_ring] += coefficient * basis;
            }
            if south_ring != north_ring && m <= ring_order_limits[south_ring] {
                order_row[south_ring] += coefficient * (parity * basis);
            }
        }
    }

    let phase = if m == 0 {
        Complex::new(T::one(), T::zero())
    } else {
        longitude_phase::<T>(m, plan.grid.phi0(), false)
    };
    let scale = phase * T::from_usize(nphi).expect("longitude count converts");
    for value in order_row {
        *value *= scale;
    }
    true
}

#[inline]
#[expect(
    clippy::too_many_arguments,
    reason = "The block preparation maps mirrored inputs into existing worker scratch."
)]
fn prepare_symmetric_analysis_pair_block<T: ShtScalar>(
    m: usize,
    latitude_count: usize,
    block_start: usize,
    block_len: usize,
    compact: bool,
    order_spectra: &[Complex<T>],
    ring_order_limits: &[usize],
    theta_sin: &[T],
    theta_cos: &[T],
    same_parity: &mut [Complex<T>],
    opposite_parity: &mut [Complex<T>],
    compact_theta_sin: &mut [T],
    compact_theta_cos: &mut [T],
) -> usize {
    let mut active_pair_lanes = 0;
    for lane in 0..block_len {
        let north_ring = block_start + lane;
        let south_ring = latitude_count - 1 - north_ring;
        let north_active = m <= ring_order_limits[north_ring];
        let south_active = south_ring != north_ring && m <= ring_order_limits[south_ring];
        if compact && (north_active || south_active) {
            active_pair_lanes += 1;
        }
        let north = if north_active {
            order_spectra[north_ring]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        // The equatorial singleton has no reflected sample and contributes once to both parities.
        let south = if south_active {
            order_spectra[south_ring]
        } else {
            Complex::new(T::zero(), T::zero())
        };
        same_parity[lane] = north + south;
        opposite_parity[lane] = north - south;
    }

    if compact && active_pair_lanes < block_len {
        let mut compact_len = 0;
        for lane in 0..block_len {
            let north_ring = block_start + lane;
            let south_ring = latitude_count - 1 - north_ring;
            let north_active = m <= ring_order_limits[north_ring];
            let south_active = south_ring != north_ring && m <= ring_order_limits[south_ring];
            if north_active || south_active {
                compact_theta_sin[compact_len] = theta_sin[north_ring];
                compact_theta_cos[compact_len] = theta_cos[north_ring];
                same_parity[compact_len] = same_parity[lane];
                opposite_parity[compact_len] = opposite_parity[lane];
                compact_len += 1;
            }
        }
        compact_len
    } else {
        block_len
    }
}

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
    /// Synthesizes one real scalar field into north-to-south row-major samples.
    ///
    /// For even longitude counts, the Nyquist order is allowed and its
    /// positive and negative modes are combined into the real Nyquist bin.
    ///
    /// # Errors
    ///
    /// Returns an error for non-scalar spin, invalid coefficients, insufficient
    /// longitude sampling, a buffer length mismatch, or an incompatible workspace.
    pub fn synthesis_into(
        &self,
        alm: &[Complex<T>],
        map: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.check_scalar_spin()?;
        self.harmonics.validate_coefficients(alm)?;
        self.check_synthesis_order()?;
        check_len("map output", self.sample_count, map.len())?;
        self.check_workspace(workspace)?;

        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        let lmax = self.harmonics.lmax();
        let phi0 = self.grid.phi0();
        for (ring, &theta) in self.grid.theta().iter().enumerate() {
            (workspace.theta_sin[ring], workspace.theta_cos[ring]) = sin_cos_from_f64(theta);
            workspace.ring_order_limits[ring] = T::polar_order_limit(
                lmax,
                0,
                workspace.theta_sin[ring],
                workspace.theta_cos[ring],
            );
        }
        workspace.spectra.fill(Complex::new(T::zero(), T::zero()));
        if let Some(table) = self.scalar_legendre_table.as_ref()
            && self.synthesize_scalar_table(alm, map, workspace, table)?
        {
            return Ok(());
        }
        let arch = pulp::Arch::new();
        #[cfg(feature = "parallel")]
        {
            let group_ring_count = workspace.ring_group_size.min(ntheta);
            let parallel_groups = ntheta.div_ceil(group_ring_count);
            let use_parallel = rayon::current_num_threads() > 1 && parallel_groups > 1;
            if use_parallel {
                let spectrum_group_len = group_ring_count.checked_mul(spectrum_len).ok_or(
                    ShtError::IntegerOverflow {
                        quantity: "parallel Fourier spectrum group size",
                    },
                )?;
                debug_assert!(parallel_groups <= workspace.recurrence_workers.len());
                workspace
                    .spectra
                    .par_chunks_mut(spectrum_group_len)
                    .zip(workspace.recurrence_workers[..parallel_groups].par_iter_mut())
                    .enumerate()
                    .map(|(group_index, (group_spectra, recurrence))| {
                        let ring_start = group_index * group_ring_count;
                        let ring_end =
                            (ring_start + group_spectra.len() / spectrum_len).min(ntheta);
                        self.synthesize_ring_group(
                            alm,
                            nphi,
                            spectrum_len,
                            phi0,
                            ring_start,
                            &mut group_spectra[..(ring_end - ring_start) * spectrum_len],
                            &workspace.theta_sin[ring_start..ring_end],
                            &workspace.theta_cos[ring_start..ring_end],
                            &workspace.ring_order_limits,
                            arch,
                            recurrence,
                        )
                    })
                    .try_for_each(|result| result)?;
            } else {
                self.synthesize_ring_group(
                    alm,
                    nphi,
                    spectrum_len,
                    phi0,
                    0,
                    &mut workspace.spectra,
                    &workspace.theta_sin[..ntheta],
                    &workspace.theta_cos[..ntheta],
                    &workspace.ring_order_limits[..ntheta],
                    arch,
                    &mut workspace.recurrence_workers[0],
                )?;
            }
        }
        #[cfg(not(feature = "parallel"))]
        {
            self.synthesize_ring_group(
                alm,
                nphi,
                spectrum_len,
                phi0,
                0,
                &mut workspace.spectra,
                &workspace.theta_sin[..ntheta],
                &workspace.theta_cos[..ntheta],
                &workspace.ring_order_limits[..ntheta],
                arch,
                &mut workspace.recurrence_workers[0],
            )?;
        }

        for ring in 0..ntheta {
            let start = ring * spectrum_len;
            let ring_spectrum = &workspace.spectra[start..start + spectrum_len];
            self.fft
                .inverse(ring_spectrum, &mut workspace.row, &mut workspace.fft)?;
            let output_start = ring * nphi;
            map[output_start..output_start + nphi].copy_from_slice(&workspace.row);
        }
        Ok(())
    }

    fn synthesize_scalar_table(
        &self,
        alm: &[Complex<T>],
        map: &mut [T],
        workspace: &mut ShtWorkspace<T>,
        table: &crate::legendre::ScalarLegendreTable<T>,
    ) -> Result<bool, ShtError> {
        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        let mmax = self.harmonics.mmax();
        let order_count = mmax + 1;
        let order_spectrum_count =
            order_count
                .checked_mul(ntheta)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "precomputed scalar synthesis spectrum count",
                })?;
        let order_spectra = &mut workspace.spin_spectra[..order_spectrum_count];
        order_spectra.fill(Complex::new(T::zero(), T::zero()));

        #[cfg(feature = "parallel")]
        let all_inputs_safe = if rayon::current_num_threads() > 1 && order_count > 1 {
            order_spectra
                .par_chunks_mut(ntheta)
                .enumerate()
                .all(|(m, order_row)| {
                    accumulate_precomputed_synthesis_order(
                        self,
                        table,
                        alm,
                        &workspace.ring_order_limits[..ntheta],
                        m,
                        order_row,
                    )
                })
        } else {
            order_spectra
                .chunks_mut(ntheta)
                .enumerate()
                .all(|(m, order_row)| {
                    accumulate_precomputed_synthesis_order(
                        self,
                        table,
                        alm,
                        &workspace.ring_order_limits[..ntheta],
                        m,
                        order_row,
                    )
                })
        };
        #[cfg(not(feature = "parallel"))]
        let all_inputs_safe = order_spectra
            .chunks_mut(ntheta)
            .enumerate()
            .all(|(m, order_row)| {
                accumulate_precomputed_synthesis_order(
                    self,
                    table,
                    alm,
                    &workspace.ring_order_limits[..ntheta],
                    m,
                    order_row,
                )
            });

        if !all_inputs_safe {
            return Ok(false);
        }

        workspace.spectra.fill(Complex::new(T::zero(), T::zero()));
        for m in 0..order_count {
            for ring in 0..ntheta {
                workspace.spectra[ring * spectrum_len + m] = order_spectra[m * ntheta + ring];
            }
        }
        for ring in 0..ntheta {
            let spectrum_start = ring * spectrum_len;
            self.fft.inverse(
                &workspace.spectra[spectrum_start..spectrum_start + spectrum_len],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            let map_start = ring * nphi;
            map[map_start..map_start + nphi].copy_from_slice(&workspace.row);
        }
        Ok(true)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Each synthesis group uses plan geometry and its caller-owned worker scratch."
    )]
    fn synthesize_ring_group(
        &self,
        alm: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        group_ring_start: usize,
        spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        debug_assert_eq!(spectra.len(), theta_sin.len() * spectrum_len);
        debug_assert_eq!(theta_sin.len(), theta_cos.len());

        let fourier_scale = T::from_usize(nphi).expect("longitude count converts");
        let nyquist_scale = fourier_scale * T::from_f64(2.0).expect("two is representable");
        for m in 0..=mmax {
            T::fill_epsilon(m, lmax, &mut recurrence.epsilon);
            let coefficient_start = self
                .harmonics
                .index(m, m)
                .expect("every packed order starts at its diagonal degree");
            let transition_enabled =
                lmax >= SCALAR_TRANSITION_MIN_LMAX && T::supports_paired_scalar();
            let mut transition_coefficients_ready = false;
            let paired_factors_ready = T::fill_paired_scalar_factors(
                m,
                lmax,
                &recurrence.epsilon,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            );
            // Extreme finite coefficients can overflow DUCC's paired-degree
            // coefficient transform even when the final scaled contribution
            // is representable. Keep those orders on the unpaired scaled path.
            let paired_coefficients_finite = !transition_enabled
                || paired_scalar_coefficients_are_finite(
                    m,
                    lmax,
                    &recurrence.epsilon,
                    &recurrence.spin_alpha,
                    &alm[coefficient_start..],
                );
            let coefficient_pair_count = (lmax - m + 2) / 2;
            let coefficients_prepared = paired_factors_ready
                && paired_coefficients_finite
                && recurrence.fast_spin_first.len() >= coefficient_pair_count
                && recurrence.fast_spin_second.len() >= coefficient_pair_count
                && T::prepare_paired_scalar_synthesis_coefficients(
                    m,
                    lmax,
                    &recurrence.epsilon,
                    &recurrence.spin_alpha,
                    &alm[coefficient_start..],
                    &mut recurrence.fast_spin_first[..coefficient_pair_count],
                    &mut recurrence.fast_spin_second[..coefficient_pair_count],
                );
            let phase = if m == 0 {
                Complex::new(T::one(), T::zero())
            } else {
                longitude_phase::<T>(m, phi0, false)
            };
            let paired_used = if paired_factors_ready
                && paired_coefficients_finite
                && coefficients_prepared
            {
                for block_start in (0..theta_sin.len()).step_by(LATITUDE_BLOCK_SIZE) {
                    let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(theta_sin.len());
                    let block_len = block_end - block_start;
                    let ring_limits = &ring_order_limits
                        [group_ring_start + block_start..group_ring_start + block_end];
                    if ring_limits.iter().all(|&limit| m > limit) {
                        continue;
                    }
                    let spectrum_start = block_start * spectrum_len;
                    let spectrum_end = block_end * spectrum_len;
                    let fused = T::synthesize_paired_scalar_block_prepared_dispatched(
                        arch,
                        m,
                        lmax,
                        &theta_sin[block_start..block_end],
                        &theta_cos[block_start..block_end],
                        self.sectoral_norms[m],
                        &recurrence.spin_coefficient_a,
                        &recurrence.spin_coefficient_b,
                        &recurrence.fast_spin_first[..coefficient_pair_count],
                        &recurrence.fast_spin_second[..coefficient_pair_count],
                        ring_limits,
                        spectrum_len,
                        &mut spectra[spectrum_start..spectrum_end],
                        &mut recurrence.previous[..block_len],
                        &mut recurrence.current[..block_len],
                        &mut recurrence.next[..block_len],
                        &mut recurrence.exponents[..block_len],
                    )?;
                    if !fused {
                        let transition =
                            if transition_enabled && ring_limits.iter().all(|&limit| m <= limit) {
                                let coefficient_count = lmax - m + 1;
                                if !transition_coefficients_ready {
                                    recurrence.fast_scalar_second[..coefficient_count]
                                        .fill(Complex::new(T::zero(), T::zero()));
                                    transition_coefficients_ready = true;
                                }
                                T::try_synthesize_paired_scalar_transition_dispatched(
                                    arch,
                                    m,
                                    lmax,
                                    &theta_sin[block_start..block_end],
                                    &theta_cos[block_start..block_end],
                                    self.sectoral_norms[m],
                                    &recurrence.epsilon,
                                    &recurrence.spin_alpha,
                                    &recurrence.spin_coefficient_a,
                                    &recurrence.spin_coefficient_b,
                                    &alm[coefficient_start..coefficient_start + coefficient_count],
                                    &recurrence.fast_scalar_second[..coefficient_count],
                                    &mut recurrence.previous[..block_len],
                                    &mut recurrence.current[..block_len],
                                    &mut recurrence.next[..block_len],
                                    &mut recurrence.exponents[..block_len],
                                    &mut recurrence.analysis_same_real[..block_len],
                                    &mut recurrence.analysis_same_imag[..block_len],
                                    &mut recurrence.analysis_opposite_real[..block_len],
                                    &mut recurrence.analysis_opposite_imag[..block_len],
                                )?
                            } else {
                                false
                            };
                        if transition {
                            for lane in 0..block_len {
                                spectra[spectrum_start + lane * spectrum_len + m] += Complex::new(
                                    recurrence.analysis_same_real[lane],
                                    recurrence.analysis_same_imag[lane],
                                );
                            }
                        } else {
                            let compact_mixed_block =
                                lmax >= 255 && ring_limits.iter().any(|&limit| m > limit);
                            if compact_mixed_block {
                                let mut compact_len = 0;
                                for (ring, &limit) in ring_limits.iter().enumerate() {
                                    if m <= limit {
                                        recurrence.compact_theta_sin[compact_len] =
                                            theta_sin[block_start + ring];
                                        recurrence.compact_theta_cos[compact_len] =
                                            theta_cos[block_start + ring];
                                        recurrence.compact_ring_indices[compact_len] = ring;
                                        compact_len += 1;
                                    }
                                }
                                debug_assert!(compact_len > 0);
                                T::for_each_y_lm_paired_dispatched(
                                    arch,
                                    m,
                                    lmax,
                                    &recurrence.compact_theta_sin[..compact_len],
                                    &recurrence.compact_theta_cos[..compact_len],
                                    self.sectoral_norms[m],
                                    &recurrence.spin_coefficient_a,
                                    &recurrence.spin_coefficient_b,
                                    &mut recurrence.previous[..compact_len],
                                    &mut recurrence.current[..compact_len],
                                    &mut recurrence.next[..compact_len],
                                    &mut recurrence.exponents[..compact_len],
                                    |l, ring_start, current, exponents| {
                                        let pair_index = (l - m) / 2;
                                        let coefficient_offset = l - m;
                                        let next_even_coefficient = if l + 2 <= lmax {
                                            alm[coefficient_start + coefficient_offset + 2]
                                        } else {
                                            Complex::new(T::zero(), T::zero())
                                        };
                                        let coefficient_even = (alm
                                            [coefficient_start + coefficient_offset]
                                            * recurrence.epsilon[l + 1]
                                            + next_even_coefficient * recurrence.epsilon[l + 2])
                                            * recurrence.spin_alpha[pair_index];
                                        let coefficient_odd = if l < lmax {
                                            alm[coefficient_start + coefficient_offset + 1]
                                                * recurrence.spin_alpha[pair_index]
                                        } else {
                                            Complex::new(T::zero(), T::zero())
                                        };
                                        for lane in 0..current.len() {
                                            let fallback_ring = ring_start + lane;
                                            let local_ring =
                                                recurrence.compact_ring_indices[fallback_ring];
                                            let group_ring = block_start + local_ring;
                                            let contribution = T::multiply_complex_scaled(
                                                coefficient_even,
                                                current[lane],
                                                exponents[lane],
                                            )
                                                + T::multiply_complex_scaled_product(
                                                    coefficient_odd,
                                                    current[lane],
                                                    recurrence.compact_theta_cos[fallback_ring],
                                                    exponents[lane],
                                                );
                                            let spectrum_index = group_ring * spectrum_len + m;
                                            spectra[spectrum_index] += contribution;
                                        }
                                        Ok(())
                                    },
                                )?;
                            } else {
                                T::for_each_y_lm_paired_dispatched(
                                    arch,
                                    m,
                                    lmax,
                                    &theta_sin[block_start..block_end],
                                    &theta_cos[block_start..block_end],
                                    self.sectoral_norms[m],
                                    &recurrence.spin_coefficient_a,
                                    &recurrence.spin_coefficient_b,
                                    &mut recurrence.previous[..block_len],
                                    &mut recurrence.current[..block_len],
                                    &mut recurrence.next[..block_len],
                                    &mut recurrence.exponents[..block_len],
                                    |l, ring_start, current, exponents| {
                                        let pair_index = (l - m) / 2;
                                        let coefficient_offset = l - m;
                                        let next_even_coefficient = if l + 2 <= lmax {
                                            alm[coefficient_start + coefficient_offset + 2]
                                        } else {
                                            Complex::new(T::zero(), T::zero())
                                        };
                                        let coefficient_even = (alm
                                            [coefficient_start + coefficient_offset]
                                            * recurrence.epsilon[l + 1]
                                            + next_even_coefficient * recurrence.epsilon[l + 2])
                                            * recurrence.spin_alpha[pair_index];
                                        let coefficient_odd = if l < lmax {
                                            alm[coefficient_start + coefficient_offset + 1]
                                                * recurrence.spin_alpha[pair_index]
                                        } else {
                                            Complex::new(T::zero(), T::zero())
                                        };
                                        for lane in 0..current.len() {
                                            let group_ring = block_start + ring_start + lane;
                                            let global_ring = group_ring_start + group_ring;
                                            if m > ring_order_limits[global_ring] {
                                                continue;
                                            }
                                            let contribution = T::multiply_complex_scaled(
                                                coefficient_even,
                                                current[lane],
                                                exponents[lane],
                                            )
                                                + T::multiply_complex_scaled_product(
                                                    coefficient_odd,
                                                    current[lane],
                                                    theta_cos[group_ring],
                                                    exponents[lane],
                                                );
                                            let spectrum_index = group_ring * spectrum_len + m;
                                            spectra[spectrum_index] += contribution;
                                        }
                                        Ok(())
                                    },
                                )?;
                            }
                        }
                    }
                }
                true
            } else {
                false
            };
            if !paired_used {
                T::for_each_y_lm_dispatched(
                    arch,
                    m,
                    lmax,
                    theta_sin,
                    theta_cos,
                    self.sectoral_norms[m],
                    &recurrence.epsilon,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                    |l, ring_start, current, exponents| {
                        let coefficient = alm[coefficient_start + (l - m)];
                        for lane in 0..current.len() {
                            let group_ring = ring_start + lane;
                            let global_ring = group_ring_start + group_ring;
                            if m > ring_order_limits[global_ring] {
                                continue;
                            }
                            let spectrum_index = group_ring * spectrum_len + m;
                            spectra[spectrum_index] += T::multiply_complex_scaled(
                                coefficient,
                                current[lane],
                                exponents[lane],
                            );
                        }
                        Ok(())
                    },
                )?;
            }
            for group_ring in 0..theta_sin.len() {
                let global_ring = group_ring_start + group_ring;
                if m > ring_order_limits[global_ring] {
                    continue;
                }
                let spectrum_index = group_ring * spectrum_len + m;
                let phased = spectra[spectrum_index] * phase;
                if nphi.is_multiple_of(2) && m == nphi / 2 {
                    spectra[spectrum_index] = Complex::new(nyquist_scale * phased.re, T::zero());
                } else {
                    spectra[spectrum_index] = phased * fourier_scale;
                }
            }
        }
        Ok(())
    }

    /// Analyzes samples into real-field scalar harmonic coefficients.
    ///
    /// Exact GL and F1 analysis require `ntheta >= lmax + 1`; exact CC analysis
    /// requires `ntheta >= lmax + 2`. All require
    /// `mmax <= (nphi - 1) / 2`.
    ///
    /// # Errors
    ///
    /// Returns an error for non-scalar spin, insufficient exact-analysis
    /// sampling, a buffer length mismatch, or an incompatible workspace.
    pub fn analysis_into(
        &self,
        map: &[T],
        alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.check_scalar_spin()?;
        self.check_analysis_sampling()?;
        check_len("map input", self.sample_count, map.len())?;
        check_len("coefficient output", self.harmonics.nalm(), alm.len())?;
        self.check_workspace(workspace)?;

        self.analyze_map_to_coefficients(map, alm, workspace, Some(self.grid.ring_weights()))
    }

    /// Applies the adjoint of scalar synthesis under the Euclidean map dot
    /// product and the real harmonic inner product.
    ///
    /// Unlike exact analysis, this operation has no latitude sampling floor.
    /// It accepts the real Nyquist order for even longitude counts.
    ///
    /// # Errors
    ///
    /// Returns an error for non-scalar spin, insufficient longitude sampling,
    /// a buffer length mismatch, or an incompatible workspace.
    pub fn adjoint_synthesis_into(
        &self,
        map: &[T],
        alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.check_scalar_spin()?;
        self.check_synthesis_order()?;
        check_len("map input", self.sample_count, map.len())?;
        check_len("coefficient output", self.harmonics.nalm(), alm.len())?;
        self.check_workspace(workspace)?;

        self.analyze_map_to_coefficients(map, alm, workspace, None)
    }

    /// Applies the adjoint of exact scalar analysis.
    ///
    /// Exact GL and F1 analysis require `ntheta >= lmax + 1`; exact CC analysis
    /// requires `ntheta >= lmax + 2`. All require
    /// `mmax <= (nphi - 1) / 2`. GL output is STANDARD synthesis scaled on
    /// each ring by `ring_weight / nphi`; CC output uses the adjoint of the
    /// prepared-grid resampling operator.
    ///
    /// # Errors
    ///
    /// Returns an error for non-scalar spin, insufficient exact-analysis
    /// sampling, invalid coefficients, a buffer length mismatch, or an
    /// incompatible workspace.
    pub fn adjoint_analysis_into(
        &self,
        alm: &[Complex<T>],
        map: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.check_scalar_spin()?;
        self.check_analysis_sampling()?;
        self.harmonics.validate_coefficients(alm)?;
        check_len("coefficient input", self.harmonics.nalm(), alm.len())?;
        check_len("map output", self.sample_count, map.len())?;
        self.check_workspace(workspace)?;

        if self.cc_resampling.is_some() {
            self.adjoint_analysis_cc_into(alm, map, workspace)
        } else {
            self.synthesis_into(alm, map, workspace)?;
            self.apply_analysis_pixel_weights(map);
            Ok(())
        }
    }

    fn adjoint_analysis_cc_into(
        &self,
        alm: &[Complex<T>],
        map: &mut [T],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        let resampling = self
            .cc_resampling
            .as_ref()
            .expect("CC adjoint analysis requires the prepared CC operator");
        let prepared_ntheta = resampling.prepared_ntheta();
        for (ring, &theta) in resampling.prepared_theta().iter().enumerate() {
            (workspace.theta_sin[ring], workspace.theta_cos[ring]) = sin_cos_from_f64::<T>(theta);
            workspace.ring_order_limits[ring] = T::polar_order_limit(
                self.harmonics.lmax(),
                0,
                workspace.theta_sin[ring],
                workspace.theta_cos[ring],
            );
        }
        workspace
            .prepared_spectra
            .fill(Complex::new(T::zero(), T::zero()));
        self.synthesize_ring_group(
            alm,
            1,
            self.fft.spectrum_len(),
            self.grid.phi0(),
            0,
            &mut workspace.prepared_spectra,
            &workspace.theta_sin[..prepared_ntheta],
            &workspace.theta_cos[..prepared_ntheta],
            &workspace.ring_order_limits[..prepared_ntheta],
            pulp::Arch::new(),
            &mut workspace.recurrence_workers[0],
        )?;
        resampling.restore_original(
            &workspace.prepared_spectra,
            &mut workspace.spectra,
            self.fft.spectrum_len(),
            self.harmonics.mmax(),
            0,
            &mut workspace.cc_profile,
            &mut workspace.cc_fft_scratch,
        )?;
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        for ring in 0..self.grid.ntheta() {
            let start = ring * spectrum_len;
            workspace.spectra[start].im = T::zero();
            self.fft.inverse(
                &workspace.spectra[start..start + spectrum_len],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            let output_start = ring * nphi;
            map[output_start..output_start + nphi].copy_from_slice(&workspace.row);
        }
        Ok(())
    }

    fn analyze_map_to_coefficients(
        &self,
        map: &[T],
        alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
        ring_weights: Option<&[f64]>,
    ) -> Result<(), ShtError> {
        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        let scalar_table = self.scalar_legendre_table.as_ref();
        let mut scalar_table_inputs_safe = true;
        let spectra = &mut workspace.spectra;
        for ring in 0..ntheta {
            let input_start = ring * nphi;
            let spectrum_start = ring * spectrum_len;
            self.fft.forward(
                &map[input_start..input_start + nphi],
                &mut spectra[spectrum_start..spectrum_start + spectrum_len],
                &mut workspace.fft,
            )?;
        }

        let (analysis_spectra, analysis_ntheta, analysis_ring_weights) = if let Some(resampling) =
            self.cc_resampling
                .as_ref()
                .filter(|_| ring_weights.is_some())
        {
            for value in &mut workspace.spectra {
                *value /= T::from_usize(nphi).expect("longitude count converts");
            }
            resampling.to_prepared(
                &workspace.spectra,
                &mut workspace.prepared_spectra,
                spectrum_len,
                self.harmonics.mmax(),
                0,
                &mut workspace.cc_profile,
                &mut workspace.cc_fft_scratch,
            )?;
            for (ring, &theta) in resampling.prepared_theta().iter().enumerate() {
                (workspace.theta_sin[ring], workspace.theta_cos[ring]) =
                    sin_cos_from_f64::<T>(theta);
                workspace.ring_order_limits[ring] = T::polar_order_limit(
                    self.harmonics.lmax(),
                    0,
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
            }
            (
                workspace.prepared_spectra.as_slice(),
                resampling.prepared_ntheta(),
                None,
            )
        } else {
            for (ring, &theta) in self.grid.theta().iter().enumerate() {
                (workspace.theta_sin[ring], workspace.theta_cos[ring]) =
                    sin_cos_from_f64::<T>(theta);
                workspace.ring_order_limits[ring] = T::polar_order_limit(
                    self.harmonics.lmax(),
                    0,
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
            }
            (workspace.spectra.as_slice(), ntheta, ring_weights)
        };
        let theta_sin = &workspace.theta_sin[..analysis_ntheta];
        let theta_cos = &workspace.theta_cos[..analysis_ntheta];
        let ring_order_limits = &workspace.ring_order_limits[..analysis_ntheta];

        // Scalar plans do not use their spin-spectrum buffers, so analysis can
        // use them for a full order-major copy without growing the workspace.
        let order_count = self.harmonics.mmax() + 1;
        let order_spectrum_len =
            order_count
                .checked_mul(analysis_ntheta)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "order-major analysis spectrum count",
                })?;
        let order_spectra = if analysis_ntheta == ntheta {
            &mut workspace.spin_spectra[..order_spectrum_len]
        } else {
            &mut workspace.prepared_spin_spectra[..order_spectrum_len]
        };
        let longitude_count = T::from_usize(nphi).expect("longitude count converts");
        for m in 0..order_count {
            let phase = if m == 0 {
                Complex::new(T::one(), T::zero())
            } else {
                longitude_phase::<T>(m, self.grid.phi0(), true)
            };
            for ring in 0..analysis_ntheta {
                let mut weighted = if scalar_table.is_some() && m > ring_order_limits[ring] {
                    Complex::new(T::zero(), T::zero())
                } else {
                    analysis_spectra[ring * spectrum_len + m] * phase
                };
                if let Some(ring_weights) = analysis_ring_weights {
                    let weight =
                        T::from_f64(ring_weights[ring]).expect("ring weight is representable");
                    weighted *= weight / longitude_count;
                }
                if let Some(table) = scalar_table {
                    scalar_table_inputs_safe &=
                        table.input_is_safe(weighted.re) && table.input_is_safe(weighted.im);
                }
                order_spectra[m * analysis_ntheta + ring] = weighted;
            }
        }
        let scalar_table = scalar_table.filter(|_| scalar_table_inputs_safe);

        alm.fill(Complex::new(T::zero(), T::zero()));
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        if let Some(table) = scalar_table {
            self.analyze_scalar_table(order_spectra, alm, table);
            for l in 0..=lmax {
                let index = self
                    .harmonics
                    .index(l, 0)
                    .expect("order zero contains every degree");
                alm[index].im = T::zero();
            }
            return Ok(());
        }
        let arch = pulp::Arch::new();

        #[cfg(feature = "parallel")]
        {
            let order_count = mmax + 1;
            let worker_count = rayon::current_num_threads()
                .min(workspace.recurrence_workers.len())
                .min(order_count);
            let use_parallel = rayon::current_num_threads() > 1 && worker_count > 1;
            if use_parallel {
                self.analyze_order_groups_parallel(
                    order_spectra,
                    analysis_ntheta,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    0,
                    order_count,
                    alm,
                    &mut workspace.recurrence_workers[..worker_count],
                )?;
            } else {
                self.analyze_order_group(
                    order_spectra,
                    analysis_ntheta,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    0,
                    order_count,
                    alm,
                    &mut workspace.recurrence_workers[0],
                )?;
            }
        }
        #[cfg(not(feature = "parallel"))]
        {
            let order_count = mmax + 1;
            self.analyze_order_group(
                order_spectra,
                analysis_ntheta,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                0,
                order_count,
                alm,
                &mut workspace.recurrence_workers[0],
            )?;
        }
        for l in 0..=lmax {
            let index = self
                .harmonics
                .index(l, 0)
                .expect("order zero contains every degree");
            alm[index].im = T::zero();
        }
        Ok(())
    }

    fn analyze_scalar_table(
        &self,
        order_spectra: &[Complex<T>],
        alm: &mut [Complex<T>],
        table: &crate::legendre::ScalarLegendreTable<T>,
    ) {
        let order_count = self.harmonics.mmax() + 1;
        #[cfg(feature = "parallel")]
        {
            let workers = rayon::current_num_threads().min(order_count).max(1);
            self.analyze_scalar_table_groups(order_spectra, table, 0, order_count, alm, workers);
        }
        #[cfg(not(feature = "parallel"))]
        self.analyze_scalar_table_group(order_spectra, table, 0, order_count, alm);
    }

    #[cfg(feature = "parallel")]
    fn analyze_scalar_table_groups(
        &self,
        order_spectra: &[Complex<T>],
        table: &crate::legendre::ScalarLegendreTable<T>,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        worker_count: usize,
    ) {
        if m_end - m_start <= 1 || worker_count <= 1 {
            self.analyze_scalar_table_group(order_spectra, table, m_start, m_end, alm);
            return;
        }
        let m_mid = weighted_order_midpoint(self.harmonics.lmax(), m_start, m_end);
        let coefficient_start = self
            .harmonics
            .index(m_start, m_start)
            .expect("every packed order starts at its diagonal degree");
        let coefficient_mid = self
            .harmonics
            .index(m_mid, m_mid)
            .expect("the end of an order group starts the next order");
        let (left_alm, right_alm) = alm.split_at_mut(coefficient_mid - coefficient_start);
        let left_worker_count = worker_count.div_ceil(2);
        rayon::join(
            || {
                self.analyze_scalar_table_groups(
                    order_spectra,
                    table,
                    m_start,
                    m_mid,
                    left_alm,
                    left_worker_count,
                );
            },
            || {
                self.analyze_scalar_table_groups(
                    order_spectra,
                    table,
                    m_mid,
                    m_end,
                    right_alm,
                    worker_count - left_worker_count,
                );
            },
        );
    }

    fn analyze_scalar_table_group(
        &self,
        order_spectra: &[Complex<T>],
        table: &crate::legendre::ScalarLegendreTable<T>,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
    ) {
        let ntheta = self.grid.ntheta();
        let lmax = self.harmonics.lmax();
        let mut coefficient_start = 0;
        for m in m_start..m_end {
            let coefficient_count = lmax - m + 1;
            let coefficients = &mut alm[coefficient_start..coefficient_start + coefficient_count];
            for (degree_offset, coefficient) in coefficients.iter_mut().enumerate() {
                let l = m + degree_offset;
                let parity = if (l + m).is_multiple_of(2) {
                    T::one()
                } else {
                    -T::one()
                };
                let mut sum = Complex::new(T::zero(), T::zero());
                for (north_ring, &basis) in table.degree_lanes(m, degree_offset).iter().enumerate()
                {
                    let south_ring = ntheta - 1 - north_ring;
                    let north = order_spectra[m * ntheta + north_ring];
                    let south = if south_ring != north_ring {
                        order_spectra[m * ntheta + south_ring] * parity
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    sum += (north + south) * basis;
                }
                *coefficient = sum;
            }
            coefficient_start += coefficient_count;
        }
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "The parallel analysis groups share plan geometry and split caller-owned outputs and scratch."
    )]
    fn analyze_order_groups_parallel(
        &self,
        spectra: &[Complex<T>],
        latitude_count: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        recurrence: &mut [RecurrenceWorkspace<T>],
    ) -> Result<(), ShtError> {
        let order_count = m_end - m_start;
        if order_count <= 1 || recurrence.len() <= 1 {
            return self.analyze_order_group(
                spectra,
                latitude_count,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                m_start,
                m_end,
                alm,
                recurrence
                    .first_mut()
                    .expect("each analysis group has at least one recurrence workspace"),
            );
        }

        let m_mid = if self.harmonics.lmax() >= 511 {
            active_ring_weighted_order_midpoint(
                self.harmonics.lmax(),
                m_start,
                m_end,
                ring_order_limits,
            )
        } else {
            weighted_order_midpoint(self.harmonics.lmax(), m_start, m_end)
        };
        let coefficient_start = self
            .harmonics
            .index(m_start, m_start)
            .expect("every packed order starts at its diagonal degree");
        let coefficient_mid = self
            .harmonics
            .index(m_mid, m_mid)
            .expect("the end of an order group starts the next order");
        let coefficient_split = coefficient_mid - coefficient_start;
        let (left_alm, right_alm) = alm.split_at_mut(coefficient_split);
        let left_worker_count = recurrence.len().div_ceil(2);
        let (left_recurrence, right_recurrence) = recurrence.split_at_mut(left_worker_count);

        let (left_result, right_result) = rayon::join(
            || {
                self.analyze_order_groups_parallel(
                    spectra,
                    latitude_count,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    m_start,
                    m_mid,
                    left_alm,
                    left_recurrence,
                )
            },
            || {
                self.analyze_order_groups_parallel(
                    spectra,
                    latitude_count,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    m_mid,
                    m_end,
                    right_alm,
                    right_recurrence,
                )
            },
        );
        left_result?;
        right_result
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The recurrence group uses plan geometry, grid weights and caller-owned worker scratch."
    )]
    fn analyze_order_group(
        &self,
        spectra: &[Complex<T>],
        latitude_count: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.analyze_symmetric_order_group(
            spectra,
            latitude_count,
            theta_sin,
            theta_cos,
            ring_order_limits,
            arch,
            m_start,
            m_end,
            alm,
            recurrence,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The symmetric analysis group receives plan geometry and worker scratch explicitly."
    )]
    fn analyze_symmetric_order_group(
        &self,
        spectra: &[Complex<T>],
        latitude_count: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        // GL, CC, F1 and prepared meridional grids use mirrored colatitudes.
        // Scalar harmonics satisfy Y_lm(pi-theta) = (-1)^(l+m) Y_lm(theta).
        debug_assert_eq!(theta_sin.len(), latitude_count);
        debug_assert_eq!(theta_cos.len(), latitude_count);
        debug_assert_eq!(ring_order_limits.len(), latitude_count);
        let lmax = self.harmonics.lmax();
        let pair_lane_count = latitude_count.div_ceil(2);
        let mut minimum_block_limit = lmax;
        for block_start in (0..pair_lane_count).step_by(LATITUDE_BLOCK_SIZE) {
            let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(pair_lane_count);
            let block_limit =
                ring_order_limits[block_end - 1].max(ring_order_limits[latitude_count - block_end]);
            minimum_block_limit = minimum_block_limit.min(block_limit);
        }
        let mut coefficient_start = 0;

        for m in m_start..m_end {
            let coefficient_count = lmax - m + 1;
            let coefficients = &mut alm[coefficient_start..coefficient_start + coefficient_count];
            T::fill_epsilon(m, lmax, &mut recurrence.epsilon);
            let mut paired_used = T::fill_paired_scalar_factors(
                m,
                lmax,
                &recurrence.epsilon,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            );

            for block_start in (0..pair_lane_count).step_by(LATITUDE_BLOCK_SIZE) {
                let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(pair_lane_count);
                let block_len = block_end - block_start;
                if m > minimum_block_limit && block_len < pair_lane_count {
                    // The scalar cutoff rises toward the equator on each hemisphere;
                    // these endpoints bound the cutoff for every mirrored pair.
                    let block_limit = ring_order_limits[block_end - 1]
                        .max(ring_order_limits[latitude_count - block_end]);
                    if m > block_limit {
                        continue;
                    }
                }
                let order_spectra = &spectra[m * latitude_count..(m + 1) * latitude_count];
                let compact_candidate = T::supports_paired_scalar() && lmax >= 127;
                let lane_count = prepare_symmetric_analysis_pair_block(
                    m,
                    latitude_count,
                    block_start,
                    block_len,
                    compact_candidate,
                    order_spectra,
                    ring_order_limits,
                    theta_sin,
                    theta_cos,
                    &mut recurrence.weighted_spin_q_spectrum,
                    &mut recurrence.weighted_spin_u_spectrum,
                    &mut recurrence.compact_theta_sin,
                    &mut recurrence.compact_theta_cos,
                );
                if lane_count == 0 {
                    continue;
                }
                let compact_mixed_block = compact_candidate && lane_count < block_len;
                let pair_sin = if compact_mixed_block {
                    &recurrence.compact_theta_sin[..lane_count]
                } else {
                    &theta_sin[block_start..block_end]
                };
                let pair_cos = if compact_mixed_block {
                    &recurrence.compact_theta_cos[..lane_count]
                } else {
                    &theta_cos[block_start..block_end]
                };
                let same_parity_spectrum = &recurrence.weighted_spin_q_spectrum[..lane_count];
                let opposite_parity_spectrum = &recurrence.weighted_spin_u_spectrum[..lane_count];
                if paired_used {
                    for lane in 0..lane_count {
                        recurrence.analysis_same_real[lane] = same_parity_spectrum[lane].re;
                        recurrence.analysis_same_imag[lane] = same_parity_spectrum[lane].im;
                        recurrence.analysis_opposite_real[lane] = opposite_parity_spectrum[lane].re;
                        recurrence.analysis_opposite_imag[lane] = opposite_parity_spectrum[lane].im;
                    }
                    let same_real = &recurrence.analysis_same_real[..lane_count];
                    let same_imag = &recurrence.analysis_same_imag[..lane_count];
                    let opposite_real = &recurrence.analysis_opposite_real[..lane_count];
                    let opposite_imag = &recurrence.analysis_opposite_imag[..lane_count];
                    let product_range_safe = T::scalar_analysis_products_safe(
                        same_real,
                        same_imag,
                        opposite_real,
                        opposite_imag,
                        pair_cos,
                    );
                    // Paired degrees l=m+2k use north+south for the even term;
                    // the intervening odd term uses north-south and cos(theta).
                    paired_used = T::for_each_y_lm_paired_status_dispatched(
                        arch,
                        m,
                        lmax,
                        pair_sin,
                        pair_cos,
                        self.sectoral_norms[m],
                        &recurrence.spin_coefficient_a,
                        &recurrence.spin_coefficient_b,
                        &mut recurrence.previous,
                        &mut recurrence.current,
                        &mut recurrence.next,
                        &mut recurrence.exponents,
                        |l, ring_start, current, exponents, normal_range| {
                            let offset = l - m;
                            let mut even_coefficient = coefficients[offset];
                            let mut odd_coefficient = if l < lmax {
                                coefficients[offset + 1]
                            } else {
                                Complex::new(T::zero(), T::zero())
                            };
                            let lane_end = ring_start + current.len();
                            let (same_sum, opposite_sum) = T::accumulate_scalar_analysis_pair(
                                arch,
                                product_range_safe,
                                normal_range,
                                &same_real[ring_start..lane_end],
                                &same_imag[ring_start..lane_end],
                                &opposite_real[ring_start..lane_end],
                                &opposite_imag[ring_start..lane_end],
                                current,
                                exponents,
                                &pair_cos[ring_start..lane_end],
                            );
                            even_coefficient += same_sum;
                            if l < lmax {
                                odd_coefficient += opposite_sum;
                            }
                            coefficients[offset] = even_coefficient;
                            if l < lmax {
                                coefficients[offset + 1] = odd_coefficient;
                            }
                            Ok(())
                        },
                    )?;
                }
                if !paired_used {
                    T::for_each_y_lm_dispatched(
                        arch,
                        m,
                        lmax,
                        pair_sin,
                        pair_cos,
                        self.sectoral_norms[m],
                        &recurrence.epsilon,
                        &mut recurrence.previous,
                        &mut recurrence.current,
                        &mut recurrence.next,
                        &mut recurrence.exponents,
                        |l, ring_start, current, exponents| {
                            let coefficient = &mut coefficients[l - m];
                            let paired_spectra = if (l + m).is_multiple_of(2) {
                                same_parity_spectrum
                            } else {
                                opposite_parity_spectrum
                            };
                            for lane in 0..current.len() {
                                let ring = ring_start + lane;
                                *coefficient += T::multiply_complex_scaled(
                                    paired_spectra[ring],
                                    current[lane],
                                    exponents[lane],
                                );
                            }
                            Ok(())
                        },
                    )?;
                }
            }
            if paired_used {
                let mut previous_even = Complex::new(T::zero(), T::zero());
                for (pair_index, l) in (m..=lmax).step_by(2).enumerate() {
                    let offset = l - m;
                    let even = coefficients[offset];
                    let odd = if l < lmax {
                        coefficients[offset + 1]
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    let previous = if pair_index == 0 {
                        Complex::new(T::zero(), T::zero())
                    } else {
                        previous_even
                            * (recurrence.spin_alpha[pair_index - 1] * recurrence.epsilon[l])
                    };
                    coefficients[offset] = even
                        * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                        + previous;
                    if l < lmax {
                        coefficients[offset + 1] = odd * recurrence.spin_alpha[pair_index];
                    }
                    previous_even = even;
                }
            }
            coefficient_start += coefficient_count;
        }
        debug_assert_eq!(coefficient_start, alm.len());
        Ok(())
    }

    fn check_scalar_spin(&self) -> Result<(), ShtError> {
        if self.harmonics.spin() == 0 {
            Ok(())
        } else {
            Err(ShtError::UnsupportedSpin {
                spin: self.harmonics.spin(),
            })
        }
    }

    fn check_synthesis_order(&self) -> Result<(), ShtError> {
        if self.harmonics.mmax() <= self.grid.nphi() / 2 {
            Ok(())
        } else {
            Err(ShtError::SynthesisLongitudeUndersampled {
                mmax: self.harmonics.mmax(),
                nphi: self.grid.nphi(),
            })
        }
    }

    pub(crate) fn check_analysis_sampling(&self) -> Result<(), ShtError> {
        let minimum_ntheta = match self.grid.kind() {
            GridKind::GaussLegendre => self.harmonics.lmax().checked_add(1),
            GridKind::Fejer1 => self.harmonics.lmax().checked_add(1),
            GridKind::ClenshawCurtis => self.harmonics.lmax().checked_add(2),
        }
        .ok_or(ShtError::IntegerOverflow {
            quantity: "exact-analysis latitude count",
        })?;
        if self.grid.ntheta() < minimum_ntheta {
            return Err(ShtError::AnalysisLatitudeUndersampled {
                lmax: self.harmonics.lmax(),
                ntheta: self.grid.ntheta(),
            });
        }
        if self.harmonics.mmax() > (self.grid.nphi() - 1) / 2 {
            return Err(ShtError::AnalysisLongitudeUndersampled {
                mmax: self.harmonics.mmax(),
                nphi: self.grid.nphi(),
            });
        }
        Ok(())
    }

    pub(crate) fn apply_analysis_pixel_weights(&self, map: &mut [T]) {
        let nphi = self.grid.nphi();
        for (ring, &ring_weight) in self.grid.ring_weights().iter().enumerate() {
            let scale = T::from_f64(ring_weight / nphi as f64)
                .expect("analysis pixel weight is representable");
            let start = ring * nphi;
            for sample in &mut map[start..start + nphi] {
                *sample *= scale;
            }
        }
    }

    pub(crate) fn check_workspace(&self, workspace: &ShtWorkspace<T>) -> Result<(), ShtError> {
        let prepared_ntheta = self
            .cc_resampling
            .as_ref()
            .map_or(0, |resampling| resampling.prepared_ntheta());
        let latitude_count = self.grid.ntheta().max(prepared_ntheta);
        let recurrence_scratch_len = latitude_count.min(LATITUDE_BLOCK_SIZE);
        if workspace.ring_group_size == 0 {
            return Err(ShtError::BufferLength {
                buffer: "workspace latitude group size",
                expected: 1,
                actual: 0,
            });
        }
        check_len(
            "workspace Fourier spectra",
            self.spectrum_count,
            workspace.spectra.len(),
        )?;
        check_len(
            "workspace spin Fourier spectra",
            self.spectrum_count,
            workspace.spin_spectra.len(),
        )?;
        let prepared_spectrum_count = prepared_ntheta.checked_mul(self.fft.spectrum_len()).ok_or(
            ShtError::IntegerOverflow {
                quantity: "prepared CC Fourier spectrum count",
            },
        )?;
        check_len(
            "workspace prepared CC Fourier spectra",
            prepared_spectrum_count,
            workspace.prepared_spectra.len(),
        )?;
        check_len(
            "workspace prepared spin CC Fourier spectra",
            prepared_spectrum_count,
            workspace.prepared_spin_spectra.len(),
        )?;
        check_len(
            "workspace CC meridional profile",
            self.cc_resampling
                .as_ref()
                .map_or(0, |resampling| resampling.workspace_profile_len()),
            workspace.cc_profile.len(),
        )?;
        check_len(
            "workspace CC meridional FFT scratch",
            self.cc_resampling
                .as_ref()
                .map_or(0, |resampling| resampling.workspace_fft_scratch_len()),
            workspace.cc_fft_scratch.len(),
        )?;
        check_len(
            "workspace longitude row",
            self.grid.nphi(),
            workspace.row.len(),
        )?;
        for (buffer, actual) in [
            ("workspace latitude sine values", workspace.theta_sin.len()),
            (
                "workspace latitude cosine values",
                workspace.theta_cos.len(),
            ),
            (
                "workspace latitude order limits",
                workspace.ring_order_limits.len(),
            ),
        ] {
            check_len(buffer, latitude_count, actual)?;
        }
        let expected_worker_count = self
            .grid
            .ntheta()
            .div_ceil(workspace.ring_group_size.min(self.grid.ntheta()));
        if workspace.recurrence_workers.len() < expected_worker_count {
            return Err(ShtError::BufferLength {
                buffer: "workspace recurrence worker groups",
                expected: expected_worker_count,
                actual: workspace.recurrence_workers.len(),
            });
        }
        for recurrence in &workspace.recurrence_workers {
            for (buffer, expected, actual) in [
                (
                    "workspace scalar recurrence factors",
                    self.recurrence_factor_count,
                    recurrence.epsilon.len(),
                ),
                (
                    "workspace previous recurrence values",
                    recurrence_scratch_len,
                    recurrence.previous.len(),
                ),
                (
                    "workspace current recurrence values",
                    recurrence_scratch_len,
                    recurrence.current.len(),
                ),
                (
                    "workspace next recurrence values",
                    recurrence_scratch_len,
                    recurrence.next.len(),
                ),
                (
                    "workspace recurrence exponents",
                    recurrence_scratch_len,
                    recurrence.exponents.len(),
                ),
                (
                    "workspace spin recurrence alpha factors",
                    self.spin_recurrence_factor_count,
                    recurrence.spin_alpha.len(),
                ),
                (
                    "workspace spin recurrence A factors",
                    self.spin_recurrence_factor_count,
                    recurrence.spin_coefficient_a.len(),
                ),
                (
                    "workspace spin recurrence B factors",
                    self.spin_recurrence_factor_count,
                    recurrence.spin_coefficient_b.len(),
                ),
                (
                    "workspace same-parity analysis real spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_same_real.len(),
                ),
                (
                    "workspace same-parity analysis imaginary spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_same_imag.len(),
                ),
                (
                    "workspace opposite-parity analysis real spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_opposite_real.len(),
                ),
                (
                    "workspace opposite-parity analysis imaginary spectrum",
                    recurrence_scratch_len,
                    recurrence.analysis_opposite_imag.len(),
                ),
                (
                    "workspace spin-1 fast scalar first coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_scalar_first.len(),
                ),
                (
                    "workspace spin-1 fast scalar second coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_scalar_second.len(),
                ),
                (
                    "workspace spin-1 fast E coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_spin_first.len(),
                ),
                (
                    "workspace spin-1 fast B coefficients",
                    self.spin_fast_coefficient_count,
                    recurrence.fast_spin_second.len(),
                ),
                (
                    "workspace positive-spin previous values",
                    recurrence_scratch_len,
                    recurrence.spin_previous_plus.len(),
                ),
                (
                    "workspace positive-spin current values",
                    recurrence_scratch_len,
                    recurrence.spin_current_plus.len(),
                ),
                (
                    "workspace positive-spin next values",
                    recurrence_scratch_len,
                    recurrence.spin_next_plus.len(),
                ),
                (
                    "workspace positive-spin exponents",
                    recurrence_scratch_len,
                    recurrence.spin_exponents_plus.len(),
                ),
                (
                    "workspace negative-spin previous values",
                    recurrence_scratch_len,
                    recurrence.spin_previous_minus.len(),
                ),
                (
                    "workspace negative-spin current values",
                    recurrence_scratch_len,
                    recurrence.spin_current_minus.len(),
                ),
                (
                    "workspace negative-spin next values",
                    recurrence_scratch_len,
                    recurrence.spin_next_minus.len(),
                ),
                (
                    "workspace negative-spin exponents",
                    recurrence_scratch_len,
                    recurrence.spin_exponents_minus.len(),
                ),
            ] {
                check_len(buffer, expected, actual)?;
            }
        }
        Ok(())
    }
}

fn check_len(buffer: &'static str, expected: usize, actual: usize) -> Result<(), ShtError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ShtError::BufferLength {
            buffer,
            expected,
            actual,
        })
    }
}

#[cfg(test)]
mod tests;
