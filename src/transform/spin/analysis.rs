// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
    #[expect(
        clippy::too_many_arguments,
        reason = "The analysis group shares spectra and grid metadata and owns two coefficient slices plus worker scratch."
    )]
    pub(super) fn analyze_spin_order_group(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_weights: Option<&[f64]>,
        normalize_fourier: bool,
        analysis_ring_scales: Option<&[T]>,
        spectra_preweighted: bool,
        recurrence_spin: usize,
        derivative_norm: bool,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        spin1_analysis_pair_range_safe: &[AtomicBool],
        arch: pulp::Arch,
        m_start: usize,
        m_end: usize,
        e_alm: &mut [Complex<T>],
        mut b_alm: Option<&mut [Complex<T>]>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        #[cfg(all(test, feature = "parallel"))]
        recurrence.record_active_worker();
        #[cfg(all(test, feature = "parallel"))]
        let profile_started = recurrence
            .analysis_leaf_profile_enabled()
            .then(std::time::Instant::now);
        #[cfg(all(test, feature = "parallel"))]
        recurrence.reset_analysis_leaf_block_counts();
        let lmax = self.harmonics.lmax();
        let longitude_count = T::from_usize(nphi).expect("longitude count converts");
        let fast_scalar_coefficient_count = match recurrence_spin {
            1 => lmax + 2,
            2 => lmax + 5,
            _ => 0,
        };
        let fast_scalar_min_lmax = match recurrence_spin {
            1 => SPIN1_FAST_MIN_LMAX,
            2 => SPIN2_FAST_MIN_LMAX,
            _ => usize::MAX,
        };
        let fast_scalar_domain = match recurrence_spin {
            1 => ring_weights.is_some() || spectra_preweighted,
            2 => self.grid.kind() == crate::grid::GridKind::GaussLegendre && ring_weights.is_some(),
            _ => false,
        };
        let fast_scalar_enabled = fast_scalar_domain
            && self.harmonics.spin() == recurrence_spin
            && !derivative_norm
            && (recurrence_spin == 1 || b_alm.is_some())
            && lmax >= fast_scalar_min_lmax
            && match recurrence_spin {
                1 => T::supports_spin1_analysis_pairs(),
                2 => T::supports_paired_spin(),
                _ => false,
            }
            && recurrence.fast_scalar_first.len() >= fast_scalar_coefficient_count
            && (recurrence_spin != 2 || self.spin2_fast_f1.len() > lmax)
            && recurrence.fast_spin_first.len() > lmax;
        let fast_range = if fast_scalar_enabled {
            let minimum_sine = match recurrence_spin {
                1 => SPIN1_FAST_MIN_SIN_THETA,
                2 => SPIN2_FAST_MIN_SIN_THETA,
                _ => unreachable!("only the spin-1/2 scalar fast paths are enabled"),
            };
            let minimum_sine =
                T::from_f64(minimum_sine).expect("spin fast sine limit is representable");
            let start = theta_sin
                .iter()
                .position(|&sine| sine >= minimum_sine)
                .unwrap_or(theta_sin.len());
            let end = theta_sin[start..]
                .iter()
                .position(|&sine| sine < minimum_sine)
                .map_or(theta_sin.len(), |offset| start + offset);
            (start < end).then_some((start, end))
        } else {
            None
        };
        let mut coefficient_start = 0;
        for m in m_start..m_end {
            let coefficient_count = lmax - m + 1;
            let e_coefficients =
                &mut e_alm[coefficient_start..coefficient_start + coefficient_count];
            let mut b_coefficients = b_alm.as_deref_mut().map(|values| {
                &mut values[coefficient_start..coefficient_start + coefficient_count]
            });
            let phase = longitude_phase::<T>(m, phi0, true);
            if recurrence_spin == 1 && fast_range == Some((0, theta_sin.len())) {
                if self.allow_spin1_analysis_table()
                    && let Some(table) = self
                        .spin1_scalar_legendre_table
                        .get()
                        .and_then(Option::as_ref)
                {
                    let used_table = self.analyze_spin_scalar_table_order(
                        q_spectra,
                        u_spectra,
                        spectrum_len,
                        longitude_count,
                        phase,
                        ring_weights.expect("the spin scalar table requires exact GL weights"),
                        normalize_fourier,
                        m,
                        theta_sin,
                        ring_order_limits,
                        arch,
                        e_coefficients,
                        b_coefficients.as_deref_mut(),
                        table,
                        recurrence,
                    )?;
                    if used_table {
                        coefficient_start += coefficient_count;
                        continue;
                    }
                }
                let paired_order = analysis_ring_scales.is_some()
                    && self.analyze_spin_scalar_reflected_order(
                        q_spectra,
                        u_spectra,
                        spectrum_len,
                        phase,
                        analysis_ring_scales.expect("spin analysis has reflected ring scales"),
                        m,
                        0,
                        theta_sin.len(),
                        theta_sin,
                        theta_cos,
                        ring_order_limits,
                        spin1_analysis_pair_range_safe,
                        arch,
                        e_coefficients,
                        b_coefficients.as_deref_mut(),
                        recurrence,
                    )?;
                if paired_order {
                    coefficient_start += coefficient_count;
                    continue;
                }
                self.analyze_spin_scalar_fast_order(
                    q_spectra,
                    u_spectra,
                    spectrum_len,
                    longitude_count,
                    phase,
                    ring_weights,
                    normalize_fourier,
                    m,
                    0,
                    theta_sin.len(),
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    e_coefficients,
                    b_coefficients.as_deref_mut(),
                    recurrence,
                )?;
                coefficient_start += coefficient_count;
                continue;
            }
            let mut initial = T::fill_spin_factors(
                recurrence_spin,
                m,
                lmax,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            )?;
            if let Some((fast_start, fast_end)) = fast_range {
                if recurrence_spin == 2 {
                    self.analyze_spin2_scalar_fast_order(
                        q_spectra,
                        u_spectra,
                        spectrum_len,
                        longitude_count,
                        phase,
                        ring_weights.expect("the fast analysis path uses exact ring weights"),
                        normalize_fourier,
                        m,
                        fast_start,
                        fast_end,
                        theta_sin,
                        theta_cos,
                        ring_order_limits,
                        arch,
                        e_coefficients,
                        b_coefficients.as_deref_mut(),
                        recurrence,
                    )?;
                } else {
                    let paired_order = recurrence_spin == 1
                        && analysis_ring_scales.is_some()
                        && self.analyze_spin_scalar_reflected_order(
                            q_spectra,
                            u_spectra,
                            spectrum_len,
                            phase,
                            analysis_ring_scales.expect("spin analysis has reflected ring scales"),
                            m,
                            fast_start,
                            fast_end,
                            theta_sin,
                            theta_cos,
                            ring_order_limits,
                            spin1_analysis_pair_range_safe,
                            arch,
                            e_coefficients,
                            b_coefficients.as_deref_mut(),
                            recurrence,
                        )?;
                    if !paired_order {
                        self.analyze_spin_scalar_fast_order(
                            q_spectra,
                            u_spectra,
                            spectrum_len,
                            longitude_count,
                            phase,
                            ring_weights,
                            normalize_fourier,
                            m,
                            fast_start,
                            fast_end,
                            theta_sin,
                            theta_cos,
                            ring_order_limits,
                            arch,
                            e_coefficients,
                            b_coefficients.as_deref_mut(),
                            recurrence,
                        )?;
                    }
                }
                initial = T::fill_spin_factors(
                    recurrence_spin,
                    m,
                    lmax,
                    &mut recurrence.spin_alpha,
                    &mut recurrence.spin_coefficient_a,
                    &mut recurrence.spin_coefficient_b,
                )?;
            }
            let first_normal_end = fast_range.map_or(theta_sin.len(), |(start, _)| start);
            let second_normal_start = fast_range.map_or(theta_sin.len(), |(_, end)| end);
            for (segment_start, segment_end) in [
                (0, first_normal_end),
                (second_normal_start, theta_sin.len()),
            ] {
                for block_start in (segment_start..segment_end).step_by(LATITUDE_BLOCK_SIZE) {
                    let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(segment_end);
                    let block_len = block_end - block_start;
                    for lane in 0..block_len {
                        let ring = block_start + lane;
                        let spectrum_start = ring * spectrum_len;
                        let mut q_weighted = q_spectra[spectrum_start + m] * phase;
                        let mut u_weighted = u_spectra[spectrum_start + m] * phase;
                        let mut scale = ring_weights.map_or(T::one(), |weights| {
                            T::from_f64(weights[ring]).expect("ring weight is representable")
                        });
                        if normalize_fourier {
                            scale /= longitude_count;
                        }
                        q_weighted *= scale;
                        u_weighted *= scale;
                        recurrence.weighted_spin_q_spectrum[lane] = q_weighted;
                        recurrence.weighted_spin_u_spectrum[lane] = u_weighted;
                    }
                    let weighted_q_spectrum = &recurrence.weighted_spin_q_spectrum[..block_len];
                    let weighted_u_spectrum = &recurrence.weighted_spin_u_spectrum[..block_len];
                    if T::supports_spin_analysis_simd() {
                        for lane in 0..block_len {
                            recurrence.analysis_same_real[lane] = weighted_q_spectrum[lane].re;
                            recurrence.analysis_same_imag[lane] = weighted_q_spectrum[lane].im;
                            recurrence.analysis_opposite_real[lane] = weighted_u_spectrum[lane].re;
                            recurrence.analysis_opposite_imag[lane] = weighted_u_spectrum[lane].im;
                        }
                    }
                    let analysis_q_real = &recurrence.analysis_same_real[..block_len];
                    let analysis_q_imag = &recurrence.analysis_same_imag[..block_len];
                    let analysis_u_real = &recurrence.analysis_opposite_real[..block_len];
                    let analysis_u_imag = &recurrence.analysis_opposite_imag[..block_len];
                    let vectorizable_inputs = T::spin_analysis_inputs_vectorizable(
                        m,
                        analysis_q_real,
                        analysis_q_imag,
                        analysis_u_real,
                        analysis_u_imag,
                        &ring_order_limits[block_start..block_end],
                    );
                    T::for_each_spin_lm_dispatched(
                        arch,
                        &initial,
                        lmax,
                        &theta_sin[block_start..block_end],
                        &theta_cos[block_start..block_end],
                        &recurrence.spin_coefficient_a,
                        &recurrence.spin_coefficient_b,
                        &mut recurrence.spin_previous_plus,
                        &mut recurrence.spin_current_plus,
                        &mut recurrence.spin_next_plus,
                        &mut recurrence.spin_exponents_plus,
                        &mut recurrence.spin_previous_minus,
                        &mut recurrence.spin_current_minus,
                        &mut recurrence.spin_next_minus,
                        &mut recurrence.spin_exponents_minus,
                        |degree, ring_start, plus, plus_exp, minus, minus_exp| {
                            let alpha_norm = recurrence.spin_alpha[degree]
                                * T::spin_normalization(recurrence_spin, degree, derivative_norm);
                            let coefficient = &mut e_coefficients[degree - m];
                            let mut b_coefficient = b_coefficients
                                .as_deref_mut()
                                .map(|values| &mut values[degree - m]);
                            if T::supports_spin_analysis_simd() {
                                let lane_end = ring_start + plus.len();
                                let (e_contribution, b_contribution) =
                                    T::accumulate_spin_analysis_block(
                                        arch,
                                        vectorizable_inputs,
                                        m,
                                        &analysis_q_real[ring_start..lane_end],
                                        &analysis_q_imag[ring_start..lane_end],
                                        &analysis_u_real[ring_start..lane_end],
                                        &analysis_u_imag[ring_start..lane_end],
                                        plus,
                                        plus_exp,
                                        minus,
                                        minus_exp,
                                        alpha_norm,
                                        &ring_order_limits
                                            [block_start + ring_start..block_start + lane_end],
                                        b_coefficient.is_some(),
                                    );
                                *coefficient += e_contribution;
                                if let Some(b_coefficient) = b_coefficient.as_mut() {
                                    **b_coefficient += b_contribution;
                                }
                            } else {
                                for lane in 0..plus.len() {
                                    let ring = ring_start + lane;
                                    if m > ring_order_limits[block_start + ring] {
                                        continue;
                                    }
                                    let plus_value = T::scaled_basis_value(
                                        plus[lane],
                                        plus_exp[lane],
                                        alpha_norm,
                                    );
                                    let minus_value = T::scaled_basis_value(
                                        minus[lane],
                                        minus_exp[lane],
                                        alpha_norm,
                                    );
                                    let even_basis = plus_value + minus_value;
                                    let odd_basis = plus_value - minus_value;
                                    let q_fourier = weighted_q_spectrum[ring];
                                    let u_fourier = weighted_u_spectrum[ring];
                                    *coefficient += q_fourier * even_basis
                                        + Complex::<T>::new(T::zero(), -T::one())
                                            * u_fourier
                                            * odd_basis;
                                    if let Some(b_coefficient) = b_coefficient.as_mut() {
                                        **b_coefficient += Complex::<T>::new(T::zero(), T::one())
                                            * q_fourier
                                            * odd_basis
                                            + u_fourier * even_basis;
                                    }
                                }
                            }
                            Ok(())
                        },
                    )?;
                }
            }
            coefficient_start += coefficient_count;
        }
        debug_assert_eq!(coefficient_start, e_alm.len());
        if let Some(b_alm) = b_alm {
            debug_assert_eq!(coefficient_start, b_alm.len());
        }
        #[cfg(all(test, feature = "parallel"))]
        if let Some(started) = profile_started {
            recurrence.record_analysis_leaf(true, lmax, m_start, m_end, ring_order_limits, started);
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The paired scalar reducer receives the parity spectra and recurrence block explicitly."
    )]
    pub(super) fn accumulate_spin_scalar_analysis_pair_block(
        arch: pulp::Arch,
        m: usize,
        scalar_lmax: usize,
        sectoral_norm: T,
        block_sine: &[T],
        block_cosine: &[T],
        q_products_safe: bool,
        u_products_safe: bool,
        q_even_real: &[T],
        q_even_imag: &[T],
        q_odd_real: &[T],
        q_odd_imag: &[T],
        u_even_real: &[T],
        u_even_imag: &[T],
        u_odd_real: &[T],
        u_odd_imag: &[T],
        coefficient_a: &[T],
        coefficient_b: &[T],
        previous: &mut [T],
        current: &mut [T],
        next: &mut [T],
        exponents: &mut [i64],
        #[cfg(all(test, feature = "parallel"))] profile_range_status: Option<&mut (bool, bool)>,
        scalar_first: &mut [Complex<T>],
        scalar_second: &mut [Complex<T>],
    ) -> Result<bool, ShtError> {
        #[cfg(all(test, feature = "parallel"))]
        let mut profile_range_status = profile_range_status;
        let paired_used = T::for_each_y_lm_paired_status_dispatched(
            arch,
            m,
            scalar_lmax,
            block_sine,
            block_cosine,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            previous,
            current,
            next,
            exponents,
            |l, local_ring_start, basis, exponents, normal_range| {
                #[cfg(all(test, feature = "parallel"))]
                if let Some((normal, scaled)) = profile_range_status.as_deref_mut() {
                    if normal_range {
                        *normal = true;
                    } else {
                        *scaled = true;
                    }
                }
                let mut first_even = scalar_first[l];
                let mut second_even = scalar_second[l];
                let mut first_odd = if l < scalar_lmax {
                    scalar_first[l + 1]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let mut second_odd = if l < scalar_lmax {
                    scalar_second[l + 1]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let lane_end = local_ring_start + basis.len();
                let lanes = local_ring_start..lane_end;
                let cosine = &block_cosine[lanes.clone()];
                let [(q_even, q_odd), (u_even, u_odd)] =
                    T::accumulate_scalar_analysis_pair_channels(
                        arch,
                        [q_products_safe, u_products_safe],
                        normal_range,
                        [&q_even_real[lanes.clone()], &u_even_real[lanes.clone()]],
                        [&q_even_imag[lanes.clone()], &u_even_imag[lanes.clone()]],
                        [&q_odd_real[lanes.clone()], &u_odd_real[lanes.clone()]],
                        [&q_odd_imag[lanes.clone()], &u_odd_imag[lanes]],
                        basis,
                        exponents,
                        cosine,
                    );
                first_even += q_even;
                second_even += u_even;
                if l < scalar_lmax {
                    first_odd += q_odd;
                    second_odd += u_odd;
                }
                scalar_first[l] = first_even;
                scalar_second[l] = second_even;
                if l < scalar_lmax {
                    scalar_first[l + 1] = first_odd;
                    scalar_second[l + 1] = second_odd;
                }
                Ok(())
            },
        )?;
        Ok(paired_used)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The spin-1 analysis block receives paired spectra and stages both coefficient rows."
    )]
    pub(super) fn try_fused_spin_scalar_analysis_pair_block(
        arch: pulp::Arch,
        recurrence_spin: usize,
        sequential_channels: bool,
        opposite_preweighted: bool,
        shared_recurrence_sequential_accumulation: bool,
        range_check_is_proven_safe: bool,
        range_check_was_exercised: &mut bool,
        m: usize,
        scalar_lmax: usize,
        sectoral_norm: T,
        sin_theta: &[T],
        cos_theta: &[T],
        products_safe: [bool; 2],
        same_real: [&[T]; 2],
        same_imag: [&[T]; 2],
        opposite_real: [&[T]; 2],
        opposite_imag: [&[T]; 2],
        coefficient_a: &[T],
        coefficient_b: &[T],
        scalar_first: &mut [Complex<T>],
        scalar_second: &mut [Complex<T>],
        staged_first: &mut [Complex<T>],
        staged_second: &mut [Complex<T>],
        previous: &mut [T],
        current: &mut [T],
        next: &mut [T],
        exponents: &mut [i64],
    ) -> Result<bool, ShtError> {
        let pair_kernel_supported = match recurrence_spin {
            1 => T::supports_spin1_analysis_pairs(),
            2 => T::supports_paired_spin(),
            _ => false,
        };
        if !pair_kernel_supported || !products_safe[0] || !products_safe[1] || m > scalar_lmax {
            return Ok(false);
        }

        let staged_count = scalar_lmax - m + 1;
        if scalar_first.len() < scalar_lmax + 1
            || scalar_second.len() < scalar_lmax + 1
            || staged_first.len() < staged_count
            || staged_second.len() < staged_count
            || previous.len() < sin_theta.len()
            || current.len() < sin_theta.len()
            || next.len() < sin_theta.len()
            || exponents.len() < sin_theta.len()
        {
            return Ok(false);
        }

        let fused = T::fused_scalar_analysis_pair_channels_block(
            arch,
            sequential_channels,
            opposite_preweighted,
            shared_recurrence_sequential_accumulation,
            range_check_is_proven_safe,
            range_check_was_exercised,
            m,
            scalar_lmax,
            sin_theta,
            cos_theta,
            sectoral_norm,
            coefficient_a,
            coefficient_b,
            same_real,
            same_imag,
            opposite_real,
            opposite_imag,
            &mut staged_first[..staged_count],
            &mut staged_second[..staged_count],
            previous,
            current,
            next,
            exponents,
        )?;
        if fused {
            for offset in 0..staged_count {
                scalar_first[m + offset] += staged_first[offset];
                scalar_second[m + offset] += staged_second[offset];
            }
        }
        Ok(fused)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The reflected analysis path receives paired Fourier data, its latitude range, and worker scratch explicitly."
    )]
    pub(super) fn analyze_spin_scalar_reflected_order(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        spectrum_len: usize,
        phase: Complex<T>,
        ring_scales: &[T],
        m: usize,
        ring_start: usize,
        ring_end: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        spin1_analysis_pair_range_safe: &[AtomicBool],
        arch: pulp::Arch,
        e_coefficients: &mut [Complex<T>],
        mut b_coefficients: Option<&mut [Complex<T>]>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<bool, ShtError> {
        if !T::supports_spin1_analysis_pairs() {
            return Ok(false);
        }
        debug_assert_eq!(theta_sin.len(), theta_cos.len());
        debug_assert_eq!(theta_sin.len(), ring_order_limits.len());
        debug_assert_eq!(ring_scales.len(), theta_sin.len());

        let ntheta = theta_sin.len();
        if ring_start >= ring_end || ring_end > ntheta || ring_start != ntheta - ring_end {
            return Ok(false);
        }

        let scalar_lmax = self.harmonics.lmax() + 1;
        let scalar_coefficient_count = scalar_lmax + 1;
        let fused_analysis_enabled = self.harmonics.lmax() >= SPIN1_FUSED_ANALYSIS_MIN_LMAX;
        let sequential_channels = fused_analysis_enabled && scalar_lmax < 1024;
        let shared_recurrence_sequential_accumulation = sequential_channels
            && self.grid.kind() == crate::grid::GridKind::GaussLegendre
            && matches!(current_execution_thread_count(), 1 | 8);
        let range_cache_enabled = sequential_channels
            && !cfg!(rustsht_disable_analysis_pair_range_cache)
            && !cfg!(rustsht_unchecked_analysis_pair_range);
        let range_cache_block_count = theta_sin.len().div_ceil(2).div_ceil(LATITUDE_BLOCK_SIZE);
        #[cfg(all(test, feature = "parallel"))]
        let profiling = recurrence.analysis_leaf_profile_enabled();
        recurrence.fast_scalar_first[m..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        recurrence.fast_scalar_second[m..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
        let paired_factors_ready = T::fill_paired_scalar_factors(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        );
        debug_assert!(paired_factors_ready);

        let pair_count = (ring_end - ring_start).div_ceil(2);
        let compaction_enabled = self.harmonics.lmax() >= 127;
        for block_start in (0..pair_count).step_by(LATITUDE_BLOCK_SIZE) {
            let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(pair_count);
            let block_len = block_end - block_start;
            let mut active_lanes = 0;
            for lane in 0..block_len {
                let pair_index = block_start + lane;
                let north_ring = ring_start + pair_index;
                let south_ring = ring_end - 1 - pair_index;
                let north_active = m <= ring_order_limits[north_ring];
                let south_active = south_ring != north_ring && m <= ring_order_limits[south_ring];
                if north_active || south_active {
                    active_lanes += 1;
                }

                let north_spectrum_start = north_ring * spectrum_len + m;
                let south_spectrum_start = south_ring * spectrum_len + m;
                let north_q = if north_active {
                    q_spectra[north_spectrum_start] * phase * ring_scales[north_ring]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let south_q = if south_active {
                    q_spectra[south_spectrum_start] * phase * ring_scales[south_ring]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let north_u = if north_active {
                    u_spectra[north_spectrum_start] * phase * ring_scales[north_ring]
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let south_u = if south_active {
                    u_spectra[south_spectrum_start] * phase * ring_scales[south_ring]
                } else {
                    Complex::new(T::zero(), T::zero())
                };

                recurrence.analysis_same_real[lane] = north_q.re;
                recurrence.analysis_same_imag[lane] = north_q.im;
                recurrence.analysis_opposite_real[lane] = south_q.re;
                recurrence.analysis_opposite_imag[lane] = south_q.im;
                recurrence.spin_previous_plus[lane] = north_u.re;
                recurrence.spin_current_plus[lane] = north_u.im;
                recurrence.spin_previous_minus[lane] = south_u.re;
                recurrence.spin_current_minus[lane] = south_u.im;
            }
            if active_lanes == 0 {
                continue;
            }
            let range_cache_index = m
                .checked_mul(range_cache_block_count)
                .and_then(|order_index| order_index.checked_add(block_start / LATITUDE_BLOCK_SIZE));
            let range_check_is_proven_safe = range_cache_enabled
                && range_cache_index
                    .and_then(|index| spin1_analysis_pair_range_safe.get(index))
                    .is_some_and(|safe| safe.load(Ordering::Relaxed));

            // Check each reflected input before forming its sum and difference.
            // This keeps very large finite values on the established recurrence
            // path instead of overflowing an intermediate parity combination.
            let block_ring_start = ring_start + block_start;
            let block_ring_end = ring_start + block_end;
            let all_cosines = &theta_cos[block_ring_start..block_ring_end];
            let q_inputs_safe = T::scalar_analysis_products_safe(
                &recurrence.analysis_same_real[..block_len],
                &recurrence.analysis_same_imag[..block_len],
                &recurrence.analysis_opposite_real[..block_len],
                &recurrence.analysis_opposite_imag[..block_len],
                all_cosines,
            );
            let u_inputs_safe = T::scalar_analysis_products_safe(
                &recurrence.spin_previous_plus[..block_len],
                &recurrence.spin_current_plus[..block_len],
                &recurrence.spin_previous_minus[..block_len],
                &recurrence.spin_current_minus[..block_len],
                all_cosines,
            );
            if !q_inputs_safe || !u_inputs_safe {
                return Ok(false);
            }

            let compact = compaction_enabled && active_lanes < block_len;
            let mut compact_len = 0;
            for lane in 0..block_len {
                let pair_index = block_start + lane;
                let north_ring = ring_start + pair_index;
                let south_ring = ring_end - 1 - pair_index;
                let active = m <= ring_order_limits[north_ring]
                    || (south_ring != north_ring && m <= ring_order_limits[south_ring]);
                if compact && !active {
                    continue;
                }
                let output_lane = if compact { compact_len } else { lane };
                let north_q_re = recurrence.analysis_same_real[lane];
                let north_q_im = recurrence.analysis_same_imag[lane];
                let south_q_re = recurrence.analysis_opposite_real[lane];
                let south_q_im = recurrence.analysis_opposite_imag[lane];
                let north_u_re = recurrence.spin_previous_plus[lane];
                let north_u_im = recurrence.spin_current_plus[lane];
                let south_u_re = recurrence.spin_previous_minus[lane];
                let south_u_im = recurrence.spin_current_minus[lane];

                recurrence.analysis_same_real[output_lane] = north_q_re + south_q_re;
                recurrence.analysis_same_imag[output_lane] = north_q_im + south_q_im;
                recurrence.analysis_opposite_real[output_lane] = north_q_re - south_q_re;
                recurrence.analysis_opposite_imag[output_lane] = north_q_im - south_q_im;
                recurrence.spin_previous_plus[output_lane] = north_u_re + south_u_re;
                recurrence.spin_current_plus[output_lane] = north_u_im + south_u_im;
                recurrence.spin_previous_minus[output_lane] = north_u_re - south_u_re;
                recurrence.spin_current_minus[output_lane] = north_u_im - south_u_im;
                if compact {
                    recurrence.compact_theta_sin[output_lane] = theta_sin[north_ring];
                    recurrence.compact_theta_cos[output_lane] = theta_cos[north_ring];
                }
                compact_len += 1;
            }

            let lane_count = if compact { compact_len } else { block_len };
            let block_sine = if compact {
                &recurrence.compact_theta_sin[..lane_count]
            } else {
                &theta_sin[block_ring_start..block_ring_end]
            };
            let block_cosine = if compact {
                &recurrence.compact_theta_cos[..lane_count]
            } else {
                &theta_cos[block_ring_start..block_ring_end]
            };
            let q_products_safe = T::scalar_analysis_products_safe(
                &recurrence.analysis_same_real[..lane_count],
                &recurrence.analysis_same_imag[..lane_count],
                &recurrence.analysis_opposite_real[..lane_count],
                &recurrence.analysis_opposite_imag[..lane_count],
                block_cosine,
            );
            let u_products_safe = T::scalar_analysis_products_safe(
                &recurrence.spin_previous_plus[..lane_count],
                &recurrence.spin_current_plus[..lane_count],
                &recurrence.spin_previous_minus[..lane_count],
                &recurrence.spin_current_minus[..lane_count],
                block_cosine,
            );
            if !q_products_safe || !u_products_safe {
                return Ok(false);
            }
            let opposite_preweighted = sequential_channels && T::supports_paired_spin();
            #[cfg(all(test, feature = "parallel"))]
            let mut block_range_status = (false, false);
            let mut range_check_was_exercised = false;
            let fused_block = fused_analysis_enabled
                && T::supports_spin1_analysis_pairs()
                && Self::try_fused_spin_scalar_analysis_pair_block(
                    arch,
                    1,
                    sequential_channels,
                    opposite_preweighted,
                    shared_recurrence_sequential_accumulation,
                    range_check_is_proven_safe,
                    &mut range_check_was_exercised,
                    m,
                    scalar_lmax,
                    self.sectoral_norms[m],
                    block_sine,
                    block_cosine,
                    [q_products_safe, u_products_safe],
                    [
                        &recurrence.analysis_same_real[..lane_count],
                        &recurrence.spin_previous_plus[..lane_count],
                    ],
                    [
                        &recurrence.analysis_same_imag[..lane_count],
                        &recurrence.spin_current_plus[..lane_count],
                    ],
                    [
                        &recurrence.analysis_opposite_real[..lane_count],
                        &recurrence.spin_previous_minus[..lane_count],
                    ],
                    [
                        &recurrence.analysis_opposite_imag[..lane_count],
                        &recurrence.spin_current_minus[..lane_count],
                    ],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.fast_scalar_first[..scalar_coefficient_count],
                    &mut recurrence.fast_scalar_second[..scalar_coefficient_count],
                    &mut recurrence.fast_spin_first,
                    &mut recurrence.fast_spin_second,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                )?;
            if fused_block
                && range_cache_enabled
                && !range_check_is_proven_safe
                && range_check_was_exercised
                && let Some(index) = range_cache_index
                && let Some(safe) = spin1_analysis_pair_range_safe.get(index)
            {
                safe.store(true, Ordering::Relaxed);
            }
            #[cfg(all(test, feature = "parallel"))]
            if fused_block {
                block_range_status.0 = true;
            }
            if !fused_block {
                #[cfg(all(test, feature = "parallel"))]
                let profile_range_status = profiling.then_some(&mut block_range_status);
                let paired_used = Self::accumulate_spin_scalar_analysis_pair_block(
                    arch,
                    m,
                    scalar_lmax,
                    self.sectoral_norms[m],
                    block_sine,
                    block_cosine,
                    q_products_safe,
                    u_products_safe,
                    &recurrence.analysis_same_real[..lane_count],
                    &recurrence.analysis_same_imag[..lane_count],
                    &recurrence.analysis_opposite_real[..lane_count],
                    &recurrence.analysis_opposite_imag[..lane_count],
                    &recurrence.spin_previous_plus[..lane_count],
                    &recurrence.spin_current_plus[..lane_count],
                    &recurrence.spin_previous_minus[..lane_count],
                    &recurrence.spin_current_minus[..lane_count],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                    #[cfg(all(test, feature = "parallel"))]
                    profile_range_status,
                    &mut recurrence.fast_scalar_first[..scalar_coefficient_count],
                    &mut recurrence.fast_scalar_second[..scalar_coefficient_count],
                )?;
                if !paired_used {
                    #[cfg(all(test, feature = "parallel"))]
                    if profiling {
                        recurrence
                            .record_analysis_leaf_block(block_range_status.0, block_range_status.1);
                    }
                    return Ok(false);
                }
            }
            #[cfg(all(test, feature = "parallel"))]
            if profiling {
                recurrence.record_analysis_leaf_block(block_range_status.0, block_range_status.1);
            }
        }

        let mut previous_even_first = Complex::new(T::zero(), T::zero());
        let mut previous_even_second = Complex::new(T::zero(), T::zero());
        for (pair_index, l) in (m..=scalar_lmax).step_by(2).enumerate() {
            let even_first = recurrence.fast_scalar_first[l];
            let even_second = recurrence.fast_scalar_second[l];
            let odd_first = if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let odd_second = if l < scalar_lmax {
                recurrence.fast_scalar_second[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let previous_factor = if pair_index == 0 {
                T::zero()
            } else {
                recurrence.spin_alpha[pair_index - 1] * recurrence.epsilon[l]
            };
            recurrence.fast_scalar_first[l] = even_first
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_first * previous_factor;
            recurrence.fast_scalar_second[l] = even_second
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_second * previous_factor;
            if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1] = odd_first * recurrence.spin_alpha[pair_index];
                recurrence.fast_scalar_second[l + 1] =
                    odd_second * recurrence.spin_alpha[pair_index];
            }
            previous_even_first = even_first;
            previous_even_second = even_second;
        }

        let coefficient_count = self.harmonics.lmax() - m + 1;
        let fast_e = &mut recurrence.fast_spin_first[..coefficient_count];
        let fast_b = &mut recurrence.fast_spin_second[..coefficient_count];
        scalar_to_spin1_with_epsilon(
            m,
            self.harmonics.lmax(),
            &recurrence.fast_scalar_first[..scalar_coefficient_count],
            &recurrence.fast_scalar_second[..scalar_coefficient_count],
            &recurrence.epsilon,
            fast_e,
            fast_b,
        );
        for index in 0..coefficient_count {
            e_coefficients[index] += fast_e[index];
            if let Some(b_coefficients) = b_coefficients.as_deref_mut() {
                b_coefficients[index] += fast_b[index];
            }
        }
        Ok(true)
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_arguments,
        reason = "Existing GL tests exercise the full-grid reflected analysis adapter."
    )]
    pub(super) fn analyze_spin_scalar_gl_paired_order(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        spectrum_len: usize,
        phase: Complex<T>,
        ring_scales: &[T],
        m: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        e_coefficients: &mut [Complex<T>],
        b_coefficients: Option<&mut [Complex<T>]>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<bool, ShtError> {
        self.analyze_spin_scalar_reflected_order(
            q_spectra,
            u_spectra,
            spectrum_len,
            phase,
            ring_scales,
            m,
            0,
            theta_sin.len(),
            theta_sin,
            theta_cos,
            ring_order_limits,
            &[],
            arch,
            e_coefficients,
            b_coefficients,
            recurrence,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The bounded spin-1 table projection receives order geometry and worker-owned scratch."
    )]
    pub(super) fn analyze_spin_scalar_table_order(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        spectrum_len: usize,
        longitude_count: T,
        phase: Complex<T>,
        ring_weights: &[f64],
        normalize_fourier: bool,
        m: usize,
        theta_sin: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        e_coefficients: &mut [Complex<T>],
        mut b_coefficients: Option<&mut [Complex<T>]>,
        table: &crate::legendre::ScalarLegendreTable<T>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<bool, ShtError> {
        if !T::supports_precomputed_scalar() || !T::supports_paired_spin() {
            return Ok(false);
        }
        let lmax = self.harmonics.lmax();
        let scalar_lmax = lmax + 1;
        let scalar_coefficient_count = scalar_lmax + 1;
        recurrence.fast_scalar_first[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        recurrence.fast_scalar_second[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));

        let ntheta = theta_sin.len();
        let north_ring_count = table.north_ring_count();
        let unit_cosine = [T::one(); LATITUDE_BLOCK_SIZE];
        let weighted_north_q = &mut recurrence.weighted_spin_q_spectrum;
        let weighted_south_q = &mut recurrence.weighted_spin_u_spectrum;
        let weighted_north_u_real = &mut recurrence.analysis_same_real;
        let weighted_north_u_imag = &mut recurrence.analysis_same_imag;
        let weighted_south_u_real = &mut recurrence.analysis_opposite_real;
        let weighted_south_u_imag = &mut recurrence.analysis_opposite_imag;
        let q_real = &mut recurrence.previous;
        let q_imag = &mut recurrence.current;
        let u_real = &mut recurrence.spin_previous_plus;
        let u_imag = &mut recurrence.spin_previous_minus;
        let exponents = &mut recurrence.exponents;

        for block_start in (0..north_ring_count).step_by(LATITUDE_BLOCK_SIZE) {
            let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(north_ring_count);
            let block_len = block_end - block_start;
            let mut active = false;
            exponents[..block_len].fill(0);
            for (lane, north_ring) in (block_start..block_end).enumerate() {
                let south_ring = ntheta - 1 - north_ring;
                let north_active = m <= ring_order_limits[north_ring];
                let south_active = south_ring != north_ring && m <= ring_order_limits[south_ring];
                active |= north_active || south_active;

                let weighted = |ring: usize, spectrum: Complex<T>| {
                    let mut scale = T::from_f64(ring_weights[ring])
                        .expect("ring weight is representable")
                        / theta_sin[ring];
                    if normalize_fourier {
                        scale /= longitude_count;
                    }
                    spectrum * phase * scale
                };
                let spectrum_start = north_ring * spectrum_len;
                let north_q = if north_active {
                    weighted(north_ring, q_spectra[spectrum_start + m])
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let north_u = if north_active {
                    weighted(north_ring, u_spectra[spectrum_start + m])
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let south_spectrum_start = south_ring * spectrum_len;
                let south_q = if south_active {
                    weighted(south_ring, q_spectra[south_spectrum_start + m])
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                let south_u = if south_active {
                    weighted(south_ring, u_spectra[south_spectrum_start + m])
                } else {
                    Complex::new(T::zero(), T::zero())
                };
                if !table.input_is_safe(north_q.re)
                    || !table.input_is_safe(north_q.im)
                    || !table.input_is_safe(south_q.re)
                    || !table.input_is_safe(south_q.im)
                    || !table.input_is_safe(north_u.re)
                    || !table.input_is_safe(north_u.im)
                    || !table.input_is_safe(south_u.re)
                    || !table.input_is_safe(south_u.im)
                {
                    return Ok(false);
                }
                weighted_north_q[lane] = north_q;
                weighted_south_q[lane] = south_q;
                weighted_north_u_real[lane] = north_u.re;
                weighted_north_u_imag[lane] = north_u.im;
                weighted_south_u_real[lane] = south_u.re;
                weighted_south_u_imag[lane] = south_u.im;
            }
            if !active {
                continue;
            }

            for lane in 0..block_len {
                let north_q = weighted_north_q[lane];
                let south_q = weighted_south_q[lane];
                weighted_north_q[lane] = north_q + south_q;
                weighted_south_q[lane] = north_q - south_q;
                let north_u =
                    Complex::new(weighted_north_u_real[lane], weighted_north_u_imag[lane]);
                let south_u =
                    Complex::new(weighted_south_u_real[lane], weighted_south_u_imag[lane]);
                weighted_north_u_real[lane] = north_u.re + south_u.re;
                weighted_north_u_imag[lane] = north_u.im + south_u.im;
                weighted_south_u_real[lane] = north_u.re - south_u.re;
                weighted_south_u_imag[lane] = north_u.im - south_u.im;
                q_real[lane] = weighted_north_q[lane].re;
                q_imag[lane] = weighted_north_q[lane].im;
                u_real[lane] = weighted_north_u_real[lane];
                u_imag[lane] = weighted_north_u_imag[lane];
            }
            let products_safe = T::scalar_analysis_products_safe(
                &q_real[..block_len],
                &q_imag[..block_len],
                &u_real[..block_len],
                &u_imag[..block_len],
                &unit_cosine[..block_len],
            );
            if !products_safe {
                return Ok(false);
            }

            for degree_offset in 0..(scalar_lmax - m + 1) {
                let (q_sums, u_real_sums, u_imag_sums) = if degree_offset.is_multiple_of(2) {
                    (
                        &weighted_north_q[..],
                        &weighted_north_u_real[..],
                        &weighted_north_u_imag[..],
                    )
                } else {
                    (
                        &weighted_south_q[..],
                        &weighted_south_u_real[..],
                        &weighted_south_u_imag[..],
                    )
                };
                for lane in 0..block_len {
                    q_real[lane] = q_sums[lane].re;
                    q_imag[lane] = q_sums[lane].im;
                    u_real[lane] = u_real_sums[lane];
                    u_imag[lane] = u_imag_sums[lane];
                }
                let basis = &table.degree_lanes(m, degree_offset)[block_start..block_end];
                let (q_sum, u_sum) = T::accumulate_scalar_analysis_pair(
                    arch,
                    true,
                    true,
                    &q_real[..block_len],
                    &q_imag[..block_len],
                    &u_real[..block_len],
                    &u_imag[..block_len],
                    basis,
                    &exponents[..block_len],
                    &unit_cosine[..block_len],
                );
                let degree = m + degree_offset;
                recurrence.fast_scalar_first[degree] += q_sum;
                recurrence.fast_scalar_second[degree] += u_sum;
            }
        }

        let coefficient_count = lmax - m + 1;
        let fast_e = &mut recurrence.fast_spin_first[..coefficient_count];
        let fast_b = &mut recurrence.fast_spin_second[..coefficient_count];
        scalar_to_spin1(
            m,
            lmax,
            &recurrence.fast_scalar_first[..scalar_coefficient_count],
            &recurrence.fast_scalar_second[..scalar_coefficient_count],
            fast_e,
            fast_b,
        );
        for index in 0..coefficient_count {
            e_coefficients[index] += fast_e[index];
            if let Some(b_coefficients) = b_coefficients.as_deref_mut() {
                b_coefficients[index] += fast_b[index];
            }
        }
        Ok(true)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The spin-1 fast analysis segment needs its Fourier weights, latitude lanes, and worker scratch."
    )]
    pub(super) fn analyze_spin_scalar_fast_order(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        spectrum_len: usize,
        longitude_count: T,
        phase: Complex<T>,
        ring_weights: Option<&[f64]>,
        normalize_fourier: bool,
        m: usize,
        ring_start: usize,
        ring_end: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        e_coefficients: &mut [Complex<T>],
        mut b_coefficients: Option<&mut [Complex<T>]>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let scalar_lmax = self.harmonics.lmax() + 1;
        let scalar_coefficient_count = scalar_lmax + 1;
        let fused_analysis_enabled =
            self.harmonics.lmax() >= SPIN1_FUSED_ANALYSIS_MIN_LMAX && single_threaded_execution();
        recurrence.fast_scalar_first[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        recurrence.fast_scalar_second[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
        let paired_factors_ready = T::fill_paired_scalar_factors(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        );
        debug_assert!(paired_factors_ready);

        for block_start in (ring_start..ring_end).step_by(LATITUDE_BLOCK_SIZE) {
            let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(ring_end);
            if ring_order_limits[block_start..block_end]
                .iter()
                .all(|&limit| m > limit)
            {
                continue;
            }
            let block_len = block_end - block_start;
            let compaction_enabled = self.harmonics.lmax() >= 127;
            let mut active_lanes = 0;
            for lane in 0..block_len {
                let ring = block_start + lane;
                if m <= ring_order_limits[ring] {
                    if compaction_enabled {
                        active_lanes += 1;
                    }
                    let spectrum_start = ring * spectrum_len;
                    let ring_weight = ring_weights.map_or(T::one(), |weights| {
                        T::from_f64(weights[ring]).expect("ring weight is representable")
                    });
                    let mut scale = ring_weight / theta_sin[ring];
                    if normalize_fourier {
                        scale /= longitude_count;
                    }
                    recurrence.weighted_spin_q_spectrum[lane] =
                        q_spectra[spectrum_start + m] * phase * scale;
                    recurrence.weighted_spin_u_spectrum[lane] =
                        u_spectra[spectrum_start + m] * phase * scale;
                } else {
                    recurrence.weighted_spin_q_spectrum[lane] = Complex::new(T::zero(), T::zero());
                    recurrence.weighted_spin_u_spectrum[lane] = Complex::new(T::zero(), T::zero());
                }
            }
            let compact = compaction_enabled && active_lanes < block_len;
            if compact {
                let mut compact_len = 0;
                for lane in 0..block_len {
                    let ring = block_start + lane;
                    if m <= ring_order_limits[ring] {
                        recurrence.compact_theta_sin[compact_len] = theta_sin[ring];
                        recurrence.compact_theta_cos[compact_len] = theta_cos[ring];
                        let weighted_q = recurrence.weighted_spin_q_spectrum[lane];
                        let weighted_u = recurrence.weighted_spin_u_spectrum[lane];
                        recurrence.weighted_spin_q_spectrum[compact_len] = weighted_q;
                        recurrence.weighted_spin_u_spectrum[compact_len] = weighted_u;
                        compact_len += 1;
                    }
                }
                debug_assert_eq!(compact_len, active_lanes);
            }
            let (block_sine, block_cosine, lane_count) = if compact {
                (
                    &recurrence.compact_theta_sin[..active_lanes],
                    &recurrence.compact_theta_cos[..active_lanes],
                    active_lanes,
                )
            } else {
                (
                    &theta_sin[block_start..block_end],
                    &theta_cos[block_start..block_end],
                    block_len,
                )
            };
            for lane in 0..lane_count {
                recurrence.analysis_same_real[lane] = recurrence.weighted_spin_q_spectrum[lane].re;
                recurrence.analysis_same_imag[lane] = recurrence.weighted_spin_q_spectrum[lane].im;
                recurrence.analysis_opposite_real[lane] =
                    recurrence.weighted_spin_u_spectrum[lane].re;
                recurrence.analysis_opposite_imag[lane] =
                    recurrence.weighted_spin_u_spectrum[lane].im;
            }
            let q_real = &recurrence.analysis_same_real[..lane_count];
            let q_imag = &recurrence.analysis_same_imag[..lane_count];
            let u_real = &recurrence.analysis_opposite_real[..lane_count];
            let u_imag = &recurrence.analysis_opposite_imag[..lane_count];
            let q_products_safe =
                T::scalar_analysis_products_safe(q_real, q_imag, q_real, q_imag, block_cosine);
            let u_products_safe =
                T::scalar_analysis_products_safe(u_real, u_imag, u_real, u_imag, block_cosine);
            let mut range_check_was_exercised = false;
            let fused_block = fused_analysis_enabled
                && T::supports_paired_spin()
                && Self::try_fused_spin_scalar_analysis_pair_block(
                    arch,
                    2,
                    false,
                    false,
                    false,
                    false,
                    &mut range_check_was_exercised,
                    m,
                    scalar_lmax,
                    self.sectoral_norms[m],
                    block_sine,
                    block_cosine,
                    [q_products_safe, u_products_safe],
                    [q_real, u_real],
                    [q_imag, u_imag],
                    [q_real, u_real],
                    [q_imag, u_imag],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.fast_scalar_first[..scalar_coefficient_count],
                    &mut recurrence.fast_scalar_second[..scalar_coefficient_count],
                    &mut recurrence.fast_spin_first,
                    &mut recurrence.fast_spin_second,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                )?;
            if !fused_block {
                let paired_used = Self::accumulate_spin_scalar_analysis_pair_block(
                    arch,
                    m,
                    scalar_lmax,
                    self.sectoral_norms[m],
                    block_sine,
                    block_cosine,
                    q_products_safe,
                    u_products_safe,
                    q_real,
                    q_imag,
                    q_real,
                    q_imag,
                    u_real,
                    u_imag,
                    u_real,
                    u_imag,
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                    #[cfg(all(test, feature = "parallel"))]
                    None,
                    &mut recurrence.fast_scalar_first[..scalar_coefficient_count],
                    &mut recurrence.fast_scalar_second[..scalar_coefficient_count],
                )?;
                debug_assert!(paired_used);
            }
        }

        let mut previous_even_first = Complex::new(T::zero(), T::zero());
        let mut previous_even_second = Complex::new(T::zero(), T::zero());
        for (pair_index, l) in (m..=scalar_lmax).step_by(2).enumerate() {
            let even_first = recurrence.fast_scalar_first[l];
            let even_second = recurrence.fast_scalar_second[l];
            let odd_first = if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let odd_second = if l < scalar_lmax {
                recurrence.fast_scalar_second[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let previous_factor = if pair_index == 0 {
                T::zero()
            } else {
                recurrence.spin_alpha[pair_index - 1] * recurrence.epsilon[l]
            };
            recurrence.fast_scalar_first[l] = even_first
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_first * previous_factor;
            recurrence.fast_scalar_second[l] = even_second
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_second * previous_factor;
            if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1] = odd_first * recurrence.spin_alpha[pair_index];
                recurrence.fast_scalar_second[l + 1] =
                    odd_second * recurrence.spin_alpha[pair_index];
            }
            previous_even_first = even_first;
            previous_even_second = even_second;
        }

        let coefficient_count = self.harmonics.lmax() - m + 1;
        let fast_e = &mut recurrence.fast_spin_first[..coefficient_count];
        let fast_b = &mut recurrence.fast_spin_second[..coefficient_count];
        scalar_to_spin1_with_epsilon(
            m,
            self.harmonics.lmax(),
            &recurrence.fast_scalar_first[..scalar_coefficient_count],
            &recurrence.fast_scalar_second[..scalar_coefficient_count],
            &recurrence.epsilon,
            fast_e,
            fast_b,
        );
        for index in 0..coefficient_count {
            e_coefficients[index] += fast_e[index];
            if let Some(b_coefficients) = b_coefficients.as_deref_mut() {
                b_coefficients[index] += fast_b[index];
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Spin-2 scalar analysis needs the Fourier weights, latitude lanes, and worker scratch."
    )]
    pub(super) fn analyze_spin2_scalar_fast_order(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        spectrum_len: usize,
        longitude_count: T,
        phase: Complex<T>,
        ring_weights: &[f64],
        normalize_fourier: bool,
        m: usize,
        ring_start: usize,
        ring_end: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        e_coefficients: &mut [Complex<T>],
        mut b_coefficients: Option<&mut [Complex<T>]>,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let scalar_lmax = lmax + 2;
        let scalar_coefficient_count = scalar_lmax + 1;
        recurrence.fast_scalar_first[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        recurrence.fast_scalar_second[..scalar_coefficient_count]
            .fill(Complex::new(T::zero(), T::zero()));
        T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
        let paired_factors_ready = T::fill_paired_scalar_factors(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        );
        debug_assert!(paired_factors_ready);

        for block_start in (ring_start..ring_end).step_by(LATITUDE_BLOCK_SIZE) {
            let block_end = (block_start + LATITUDE_BLOCK_SIZE).min(ring_end);
            if ring_order_limits[block_start..block_end]
                .iter()
                .all(|&limit| m > limit)
            {
                continue;
            }
            let block_len = block_end - block_start;
            for lane in 0..block_len {
                let ring = block_start + lane;
                if m <= ring_order_limits[ring] {
                    let spectrum_start = ring * spectrum_len;
                    let sine = theta_sin[ring];
                    let mut scale = T::from_f64(ring_weights[ring])
                        .expect("ring weight is representable")
                        / (sine * sine);
                    if normalize_fourier {
                        scale /= longitude_count;
                    }
                    let weighted_q = q_spectra[spectrum_start + m] * phase * scale;
                    let weighted_u = u_spectra[spectrum_start + m] * phase * scale;
                    recurrence.analysis_same_real[lane] = weighted_q.re;
                    recurrence.analysis_same_imag[lane] = weighted_q.im;
                    recurrence.analysis_opposite_real[lane] = weighted_u.re;
                    recurrence.analysis_opposite_imag[lane] = weighted_u.im;
                } else {
                    recurrence.analysis_same_real[lane] = T::zero();
                    recurrence.analysis_same_imag[lane] = T::zero();
                    recurrence.analysis_opposite_real[lane] = T::zero();
                    recurrence.analysis_opposite_imag[lane] = T::zero();
                }
            }
            let q_real = &recurrence.analysis_same_real[..block_len];
            let q_imag = &recurrence.analysis_same_imag[..block_len];
            let u_real = &recurrence.analysis_opposite_real[..block_len];
            let u_imag = &recurrence.analysis_opposite_imag[..block_len];
            let block_cosine = &theta_cos[block_start..block_end];
            let q_products_safe =
                T::scalar_analysis_products_safe(q_real, q_imag, q_real, q_imag, block_cosine);
            let u_products_safe =
                T::scalar_analysis_products_safe(u_real, u_imag, u_real, u_imag, block_cosine);
            let fused_count = scalar_lmax - m + 1;
            let mut range_check_was_exercised = false;
            let fused_block = if q_products_safe && u_products_safe {
                T::fused_scalar_analysis_pair_channels_block(
                    arch,
                    false,
                    false,
                    false,
                    false,
                    &mut range_check_was_exercised,
                    m,
                    scalar_lmax,
                    &theta_sin[block_start..block_end],
                    block_cosine,
                    self.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    [q_real, u_real],
                    [q_imag, u_imag],
                    [q_real, u_real],
                    [q_imag, u_imag],
                    &mut recurrence.fast_spin_first[..fused_count],
                    &mut recurrence.fast_spin_second[..fused_count],
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                )?
            } else {
                false
            };
            if fused_block {
                for offset in 0..fused_count {
                    let degree = m + offset;
                    recurrence.fast_scalar_first[degree] += recurrence.fast_spin_first[offset];
                    recurrence.fast_scalar_second[degree] += recurrence.fast_spin_second[offset];
                }
            } else {
                let paired_used = T::for_each_y_lm_paired_status_dispatched(
                    arch,
                    m,
                    scalar_lmax,
                    &theta_sin[block_start..block_end],
                    &theta_cos[block_start..block_end],
                    self.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &mut recurrence.previous,
                    &mut recurrence.current,
                    &mut recurrence.next,
                    &mut recurrence.exponents,
                    |l, local_ring_start, basis, exponents, normal_range| {
                        let mut first_even = recurrence.fast_scalar_first[l];
                        let mut second_even = recurrence.fast_scalar_second[l];
                        let mut first_odd = if l < scalar_lmax {
                            recurrence.fast_scalar_first[l + 1]
                        } else {
                            Complex::new(T::zero(), T::zero())
                        };
                        let mut second_odd = if l < scalar_lmax {
                            recurrence.fast_scalar_second[l + 1]
                        } else {
                            Complex::new(T::zero(), T::zero())
                        };
                        let lane_end = local_ring_start + basis.len();
                        let lanes = local_ring_start..lane_end;
                        let cosine =
                            &block_cosine[local_ring_start..local_ring_start + basis.len()];
                        let (q_even, q_odd) = T::accumulate_scalar_analysis_pair(
                            arch,
                            q_products_safe,
                            normal_range,
                            &q_real[lanes.clone()],
                            &q_imag[lanes.clone()],
                            &q_real[lanes.clone()],
                            &q_imag[lanes.clone()],
                            basis,
                            exponents,
                            cosine,
                        );
                        let (u_even, u_odd) = T::accumulate_scalar_analysis_pair(
                            arch,
                            u_products_safe,
                            normal_range,
                            &u_real[lanes.clone()],
                            &u_imag[lanes.clone()],
                            &u_real[lanes.clone()],
                            &u_imag[lanes],
                            basis,
                            exponents,
                            cosine,
                        );
                        first_even += q_even;
                        second_even += u_even;
                        if l < scalar_lmax {
                            first_odd += q_odd;
                            second_odd += u_odd;
                        }
                        recurrence.fast_scalar_first[l] = first_even;
                        recurrence.fast_scalar_second[l] = second_even;
                        if l < scalar_lmax {
                            recurrence.fast_scalar_first[l + 1] = first_odd;
                            recurrence.fast_scalar_second[l + 1] = second_odd;
                        }
                        Ok(())
                    },
                )?;
                debug_assert!(paired_used);
            }
        }

        let mut previous_even_first = Complex::new(T::zero(), T::zero());
        let mut previous_even_second = Complex::new(T::zero(), T::zero());
        for (pair_index, l) in (m..=scalar_lmax).step_by(2).enumerate() {
            let even_first = recurrence.fast_scalar_first[l];
            let even_second = recurrence.fast_scalar_second[l];
            let odd_first = if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let odd_second = if l < scalar_lmax {
                recurrence.fast_scalar_second[l + 1]
            } else {
                Complex::new(T::zero(), T::zero())
            };
            let previous_factor = if pair_index == 0 {
                T::zero()
            } else {
                recurrence.spin_alpha[pair_index - 1] * recurrence.epsilon[l]
            };
            recurrence.fast_scalar_first[l] = even_first
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_first * previous_factor;
            recurrence.fast_scalar_second[l] = even_second
                * (recurrence.spin_alpha[pair_index] * recurrence.epsilon[l + 1])
                + previous_even_second * previous_factor;
            if l < scalar_lmax {
                recurrence.fast_scalar_first[l + 1] = odd_first * recurrence.spin_alpha[pair_index];
                recurrence.fast_scalar_second[l + 1] =
                    odd_second * recurrence.spin_alpha[pair_index];
            }
            previous_even_first = even_first;
            previous_even_second = even_second;
        }

        scalar_to_spin2(
            m,
            lmax,
            &mut recurrence.fast_scalar_first[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_scalar_second[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_spin_first[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_spin_second[..self.spin_fast_coefficient_count],
            &self.spin2_fast_f1,
            &self.spin2_fast_f2,
        );
        for l in m..=lmax {
            let index = l - m;
            e_coefficients[index] += recurrence.fast_scalar_first[l];
            if let Some(b_coefficients) = b_coefficients.as_deref_mut() {
                b_coefficients[index] += recurrence.fast_scalar_second[l];
            }
        }
        Ok(())
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "Dynamic spin-analysis jobs share immutable geometry and own disjoint E/B coefficient slices."
    )]
    pub(super) fn analyze_spin_order_jobs_dynamic(
        &self,
        q_spectra: &[Complex<T>],
        u_spectra: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_weights: Option<&[f64]>,
        normalize_fourier: bool,
        analysis_ring_scales: Option<&[T]>,
        spectra_preweighted: bool,
        recurrence_spin: usize,
        derivative_norm: bool,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        spin1_analysis_pair_range_safe: &[AtomicBool],
        arch: pulp::Arch,
        e_alm: &mut [Complex<T>],
        b_alm: Option<&mut [Complex<T>]>,
        recurrence: &mut [RecurrenceWorkspace<T>],
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let order_count = self.harmonics.mmax() + 1;
        if order_count <= 1 || recurrence.len() <= 1 {
            return self.analyze_spin_order_group(
                q_spectra,
                u_spectra,
                nphi,
                spectrum_len,
                phi0,
                ring_weights,
                normalize_fourier,
                analysis_ring_scales,
                spectra_preweighted,
                recurrence_spin,
                derivative_norm,
                theta_sin,
                theta_cos,
                ring_order_limits,
                spin1_analysis_pair_range_safe,
                arch,
                0,
                order_count,
                e_alm,
                b_alm,
                recurrence
                    .first_mut()
                    .expect("each spin-analysis group has at least one recurrence workspace"),
            );
        }

        let mut order_jobs = VecDeque::new();
        if order_jobs.try_reserve_exact(order_count).is_err() {
            return self.analyze_spin_order_group(
                q_spectra,
                u_spectra,
                nphi,
                spectrum_len,
                phi0,
                ring_weights,
                normalize_fourier,
                analysis_ring_scales,
                spectra_preweighted,
                recurrence_spin,
                derivative_norm,
                theta_sin,
                theta_cos,
                ring_order_limits,
                spin1_analysis_pair_range_safe,
                arch,
                0,
                order_count,
                e_alm,
                b_alm,
                &mut recurrence[0],
            );
        }

        let mut remaining_e = e_alm;
        let mut remaining_b = b_alm;
        for m in 0..order_count {
            let coefficient_count = lmax - m + 1;
            let (e_coefficients, e_tail) = remaining_e.split_at_mut(coefficient_count);
            remaining_e = e_tail;
            let b_coefficients = match remaining_b.take() {
                Some(b_remaining) => {
                    let (b_order, b_tail) = b_remaining.split_at_mut(coefficient_count);
                    remaining_b = Some(b_tail);
                    Some(b_order)
                }
                None => None,
            };
            order_jobs.push_back((m, e_coefficients, b_coefficients));
        }
        debug_assert!(remaining_e.is_empty());
        debug_assert!(remaining_b.as_ref().is_none_or(|tail| tail.is_empty()));

        // Match DUCC's dynamic harmonic-order schedule. Each task owns one
        // packed coefficient order and reuses its recurrence workspace.
        run_dynamic_order_jobs(
            order_jobs,
            recurrence,
            |_| false,
            |worker, (m, e_coefficients, b_coefficients), _, _| {
                self.analyze_spin_order_group(
                    q_spectra,
                    u_spectra,
                    nphi,
                    spectrum_len,
                    phi0,
                    ring_weights,
                    normalize_fourier,
                    analysis_ring_scales,
                    spectra_preweighted,
                    recurrence_spin,
                    derivative_norm,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    spin1_analysis_pair_range_safe,
                    arch,
                    m,
                    m + 1,
                    e_coefficients,
                    b_coefficients,
                    worker,
                )
            },
        )
    }
}
