// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
    #[inline]
    pub(super) fn paired_scalar_odd_fft_supported(&self) -> bool {
        #[cfg(test)]
        if PAIRED_ODD_FFT_DISABLED.load(Ordering::Relaxed) {
            return false;
        }
        #[cfg(rustsht_disable_paired_odd_fft)]
        {
            false
        }
        #[cfg(not(rustsht_disable_paired_odd_fft))]
        {
            self.harmonics.spin() == 0
                && !self.grid.nphi().is_multiple_of(2)
                && T::epsilon().to_f64() == Some(f64::EPSILON)
        }
    }

    #[inline]
    pub(super) fn paired_scalar_odd_forward_fft_supported(&self) -> bool {
        // The N=255 forward row benchmark and T127 whole-transform A/B do not
        // show a repeatable gain, so keep analysis on the established route.
        self.paired_scalar_odd_fft_supported() && self.grid.nphi() != 255
    }

    #[cfg(feature = "parallel")]
    pub(super) fn synthesize_scalar_ordered_parallel(
        &self,
        alm: &[Complex<T>],
        arch: pulp::Arch,
        workspace: &mut ShtWorkspace<T>,
    ) -> bool {
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        let Some(order_count) = mmax.checked_add(1) else {
            return false;
        };
        let ntheta = self.grid.ntheta();
        let spectrum_len = self.fft.spectrum_len();
        let Some(output_len) = ntheta.checked_mul(spectrum_len) else {
            return false;
        };
        if mmax >= spectrum_len || output_len > workspace.spectra.len() {
            return false;
        }
        let worker_count = rayon::current_num_threads()
            .min(workspace.execution_capacity)
            .min(workspace.recurrence_workers.len())
            .min(order_count);
        if worker_count <= 1 {
            return false;
        }

        #[cfg(all(test, feature = "parallel"))]
        let phase_probe = workspace.transform_phase_probe();
        // Assign short contiguous order blocks. The ring-major spectrum stores
        // adjacent orders next to one another, so block ownership reduces
        // cross-worker cache-line sharing while leaving enough jobs for dynamic
        // load balancing. Claim block indices atomically to avoid taking a
        // mutex for every job and avoid allocating a queue on each transform.
        let order_block_count = order_count.div_ceil(SCALAR_ORDERED_SYNTHESIS_ORDER_BLOCK_SIZE);
        let next_order_block = AtomicUsize::new(0);
        let failed = AtomicBool::new(false);
        let ShtWorkspace {
            spectra,
            theta_sin,
            theta_cos,
            ring_order_limits,
            recurrence_workers,
            #[cfg(all(test, feature = "parallel"))]
            last_scalar_ordered_decline_had_partial_output,
            ..
        } = workspace;
        #[cfg(all(test, feature = "parallel"))]
        {
            *last_scalar_ordered_decline_had_partial_output = false;
        }
        let Some(geometry) = T::validate_scalar_ordered_geometry_dispatched(
            &theta_sin[..ntheta],
            &theta_cos[..ntheta],
            &ring_order_limits[..ntheta],
        ) else {
            return false;
        };
        #[cfg(all(test, feature = "parallel"))]
        let spectrum_init_started = std::time::Instant::now();
        if !crate::legendre::scalar_ordered::initialize_inactive_spectrum_tails(
            &mut spectra[..output_len],
            spectrum_len,
            mmax,
            &geometry,
            Complex::new(T::zero(), T::zero()),
        ) {
            return false;
        }
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "sparse spectrum-tail initialization",
            spectrum_init_started,
        );
        #[cfg(all(test, feature = "parallel"))]
        let order_started = std::time::Instant::now();
        let succeeded = {
            let Some(output) = crate::legendre::scalar_ordered::DisjointSpectrumOutput::new(
                &mut spectra[..output_len],
                ntheta,
                spectrum_len,
            ) else {
                return false;
            };
            recurrence_workers[..worker_count]
                .par_iter_mut()
                .for_each(|recurrence| {
                    #[cfg(all(test, feature = "parallel"))]
                    recurrence.record_active_worker();
                    #[cfg(all(test, feature = "parallel"))]
                    let profile_order_queue = recurrence.scalar_order_leaf_profile_enabled();
                    loop {
                        if failed.load(Ordering::Relaxed) {
                            break;
                        }
                        #[cfg(all(test, feature = "parallel"))]
                        let job_claim_started = profile_order_queue.then(std::time::Instant::now);
                        let block_index = next_order_block.fetch_add(1, Ordering::Relaxed);
                        #[cfg(all(test, feature = "parallel"))]
                        let job_claim_ns = job_claim_started
                            .map(|started| started.elapsed().as_nanos())
                            .unwrap_or(0);
                        if block_index >= order_block_count {
                            break;
                        }
                        let Some(order_start) =
                            block_index.checked_mul(SCALAR_ORDERED_SYNTHESIS_ORDER_BLOCK_SIZE)
                        else {
                            failed.store(true, Ordering::Relaxed);
                            break;
                        };
                        let order_end = order_start
                            .saturating_add(SCALAR_ORDERED_SYNTHESIS_ORDER_BLOCK_SIZE)
                            .min(order_count);
                        #[cfg(all(test, feature = "parallel"))]
                        let leaf_started = recurrence.scalar_order_leaf_started();
                        let succeeded = {
                            let mut scratch = scalar_ordered_scratch(recurrence);
                            // SAFETY: each atomic block index is claimed once,
                            // blocks cover every order without overlap, and the
                            // job writes only the columns in its block.
                            unsafe {
                                T::synthesize_scalar_ordered_output_dispatched(
                                    arch,
                                    alm,
                                    lmax,
                                    mmax,
                                    order_start,
                                    order_end,
                                    self.grid.nphi(),
                                    spectrum_len,
                                    self.grid.phi0(),
                                    &self.sectoral_norms,
                                    &geometry,
                                    &output,
                                    &mut scratch,
                                )
                            }
                        };
                        #[cfg(all(test, feature = "parallel"))]
                        recurrence.record_scalar_order_leaf(
                            lmax,
                            order_start,
                            order_end,
                            &ring_order_limits[..ntheta],
                            leaf_started,
                            job_claim_ns,
                            succeeded,
                        );
                        if !succeeded {
                            failed.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                });
            !failed.load(Ordering::Relaxed)
        };
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "ordered scalar recurrence",
            order_started,
        );
        if !succeeded {
            // The order jobs write directly into `spectra`. A failed order
            // may follow completed columns, so clear them before the robust
            // fallback accumulates into this workspace.
            #[cfg(all(test, feature = "parallel"))]
            {
                *last_scalar_ordered_decline_had_partial_output = spectra[..output_len]
                    .iter()
                    .any(|value| value.re != T::zero() || value.im != T::zero());
            }
            spectra.fill(Complex::new(T::zero(), T::zero()));
            return false;
        }
        true
    }

    #[inline]
    fn copy_scalar_latitude_geometry(
        &self,
        geometry: &ScalarLatitudeGeometry<T>,
        workspace: &mut ShtWorkspace<T>,
        include_cosine_squared: bool,
    ) {
        let ntheta = geometry.theta_sin.len();
        debug_assert_eq!(geometry.theta_cos.len(), ntheta);
        debug_assert_eq!(geometry.ring_order_limits.len(), ntheta);
        workspace.theta_sin[..ntheta].copy_from_slice(&geometry.theta_sin);
        workspace.theta_cos[..ntheta].copy_from_slice(&geometry.theta_cos);
        workspace.ring_order_limits[..ntheta].copy_from_slice(&geometry.ring_order_limits);
        if include_cosine_squared {
            for ring in 0..ntheta {
                workspace.theta_cos_squared[ring] = stable_analysis_cosine_squared(
                    geometry.theta_sin[ring],
                    geometry.theta_cos[ring],
                );
            }
        }
    }

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

        #[cfg(all(test, feature = "parallel"))]
        let phase_probe = workspace.transform_phase_probe();
        #[cfg(all(test, feature = "parallel"))]
        let mut phase_started = std::time::Instant::now();

        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        let lmax = self.harmonics.lmax();
        let phi0 = self.grid.phi0();
        if let Some(geometry) = &self.scalar_latitude_geometry {
            self.copy_scalar_latitude_geometry(geometry, workspace, false);
        } else {
            for (ring, &theta) in self.grid.theta().iter().enumerate() {
                (workspace.theta_sin[ring], workspace.theta_cos[ring]) = sin_cos_from_f64(theta);
                workspace.ring_order_limits[ring] = T::polar_order_limit(
                    lmax,
                    0,
                    workspace.theta_sin[ring],
                    workspace.theta_cos[ring],
                );
            }
        }
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(&phase_probe, "latitude setup", phase_started);
            phase_started = std::time::Instant::now();
        }
        let arch = pulp::Arch::new();
        if self.scalar_legendre_strategy_with_capacity(
            LegendreOperation::Synthesis,
            workspace.execution_capacity,
        ) == LegendreStrategy::CompactTable
            && let Some(table) = self.get_safe_scalar_legendre_table_for_synthesis(alm)
            && self.synthesize_scalar_table(alm, map, workspace, table)?
        {
            return Ok(());
        }
        #[cfg(test)]
        SCALAR_RECURRENCE_SYNTHESIS_CALLS.fetch_add(1, Ordering::Relaxed);
        let ordered_scalar_used = if self.scalar_ordered_synthesis_supported() {
            if workspace.execution_capacity == 1 {
                let recurrence = &mut workspace.recurrence_workers[0];
                #[cfg(all(test, feature = "parallel"))]
                recurrence.record_active_worker();
                let mut scratch = scalar_ordered_scratch(recurrence);
                T::synthesize_scalar_ordered_dispatched(
                    arch,
                    alm,
                    lmax,
                    self.harmonics.mmax(),
                    nphi,
                    spectrum_len,
                    phi0,
                    &self.sectoral_norms,
                    &workspace.theta_sin[..ntheta],
                    &workspace.theta_cos[..ntheta],
                    &workspace.ring_order_limits[..ntheta],
                    &mut workspace.spectra,
                    &mut scratch,
                )
            } else {
                #[cfg(feature = "parallel")]
                {
                    self.synthesize_scalar_ordered_parallel(alm, arch, workspace)
                }
                #[cfg(not(feature = "parallel"))]
                {
                    false
                }
            }
        } else {
            false
        };
        if !ordered_scalar_used {
            // The ordered and table routes initialize every Fourier cell they
            // leave untouched. The general recurrence accumulates into its
            // spectrum, so establish its zero state only when that route is
            // selected (or after a declined fast path).
            workspace.spectra.fill(Complex::new(T::zero(), T::zero()));
            #[cfg(feature = "parallel")]
            {
                let execution_threads =
                    rayon::current_num_threads().min(workspace.execution_capacity);
                let group_ring_count = self
                    .fine_synthesis_ring_group_size(execution_threads)
                    .unwrap_or(workspace.ring_group_size)
                    .min(ntheta);
                let parallel_groups = ntheta.div_ceil(group_ring_count);
                let use_parallel = execution_threads > 1 && parallel_groups > 1;
                if use_parallel {
                    let cached_scalar_coefficients = self
                        .prepare_scalar_synthesis_coefficient_cache(
                            alm,
                            &mut workspace.spin_spectra,
                            &mut workspace.recurrence_workers[..parallel_groups],
                            execution_threads,
                            parallel_groups,
                        )
                        .map(|layout| ScalarSynthesisCoefficientCache {
                            values: &workspace.spin_spectra[..layout.coefficient_count],
                            row_stride: layout.row_stride,
                            half_stride: layout.half_stride,
                        });
                    let spectrum_group_len = group_ring_count.checked_mul(spectrum_len).ok_or(
                        ShtError::IntegerOverflow {
                            quantity: "parallel Fourier spectrum group size",
                        },
                    )?;
                    // CC's general synthesis recurrence has enough work in
                    // each ring group to justify finer scheduling. The
                    // coarser default batches several groups into one Rayon
                    // task and underfills the q8 pool on representative CC
                    // grids; each group already owns its bounded recurrence
                    // workspace, so letting Rayon split to one group per task
                    // does not add transform-sized scratch.
                    let min_groups_per_task = if self.grid.kind() == GridKind::ClenshawCurtis {
                        1
                    } else {
                        parallel_groups.div_ceil(execution_threads)
                    };
                    debug_assert!(parallel_groups <= workspace.recurrence_workers.len());
                    workspace
                        .spectra
                        .par_chunks_mut(spectrum_group_len)
                        .zip(workspace.recurrence_workers[..parallel_groups].par_iter_mut())
                        .enumerate()
                        .with_min_len(min_groups_per_task)
                        .map(|(group_index, (group_spectra, recurrence))| {
                            let ring_start = group_index * group_ring_count;
                            let ring_end =
                                (ring_start + group_spectra.len() / spectrum_len).min(ntheta);
                            self.synthesize_ring_group_with_scalar_coefficients(
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
                                cached_scalar_coefficients,
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
        }

        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "Legendre recurrence and spectral accumulation",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }

        self.inverse_synthesis_rows(
            &workspace.spectra,
            map,
            &mut workspace.fft,
            #[cfg(feature = "parallel")]
            &mut workspace.fft_worker_workspaces,
            #[cfg(feature = "parallel")]
            workspace.execution_capacity,
        )?;
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "inverse longitude FFT and map writeback",
            phase_started,
        );
        Ok(())
    }

    #[cfg(feature = "parallel")]
    pub(super) fn prepare_scalar_synthesis_coefficient_cache(
        &self,
        alm: &[Complex<T>],
        cache: &mut [Complex<T>],
        recurrence_workers: &mut [RecurrenceWorkspace<T>],
        execution_threads: usize,
        parallel_groups: usize,
    ) -> Option<ScalarSynthesisCoefficientCacheLayout> {
        let lmax = self.harmonics.lmax();
        let order_count = self.harmonics.mmax().checked_add(1)?;
        // Keep the cache on the paired-transition path and avoid preparing it
        // when the ring-group schedule cannot keep each worker occupied.
        if self.harmonics.spin() != 0
            || self.grid.kind() != GridKind::GaussLegendre
            || lmax < SCALAR_TRANSITION_MIN_LMAX
            || parallel_groups < execution_threads
            || !T::supports_paired_scalar()
        {
            return None;
        }

        let half_stride = lmax.checked_add(2)?.checked_div(2)?;
        let row_stride = half_stride.checked_mul(2)?;
        let coefficient_count = row_stride.checked_mul(order_count)?;
        if coefficient_count > cache.len() || recurrence_workers.len() < parallel_groups {
            return None;
        }

        let worker_count = execution_threads
            .min(order_count)
            .min(recurrence_workers.len())
            .max(1);
        let orders_per_chunk = order_count.div_ceil(worker_count);
        let chunk_len = orders_per_chunk.checked_mul(row_stride)?;
        let chunk_count = order_count.div_ceil(orders_per_chunk);
        let prepared = cache[..coefficient_count]
            .par_chunks_mut(chunk_len)
            .zip(recurrence_workers[..chunk_count].par_iter_mut())
            .enumerate()
            .all(|(chunk_index, (coefficient_chunk, recurrence))| {
                for (local_order, order_coefficients) in
                    coefficient_chunk.chunks_mut(row_stride).enumerate()
                {
                    let m = chunk_index * orders_per_chunk + local_order;
                    if m >= order_count {
                        break;
                    }
                    let pair_count = (lmax - m + 2) / 2;
                    T::fill_epsilon(m, lmax, &mut recurrence.epsilon);
                    if !T::fill_paired_scalar_factors(
                        m,
                        lmax,
                        &recurrence.epsilon,
                        &mut recurrence.spin_alpha,
                        &mut recurrence.spin_coefficient_a,
                        &mut recurrence.spin_coefficient_b,
                    ) {
                        return false;
                    }
                    let coefficient_start = self
                        .harmonics
                        .index(m, m)
                        .expect("every packed order starts at its diagonal degree");
                    let coefficient_len = lmax - m + 1;
                    let (coefficient_even, coefficient_odd) =
                        order_coefficients.split_at_mut(half_stride);
                    if !T::prepare_paired_scalar_synthesis_coefficients(
                        m,
                        lmax,
                        &recurrence.epsilon,
                        &recurrence.spin_alpha,
                        &alm[coefficient_start..coefficient_start + coefficient_len],
                        &mut coefficient_even[..pair_count],
                        &mut coefficient_odd[..pair_count],
                    ) {
                        return false;
                    }
                }
                true
            });
        prepared.then_some(ScalarSynthesisCoefficientCacheLayout {
            coefficient_count,
            row_stride,
            half_stride,
        })
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
        let execution_threads = rayon::current_num_threads().min(workspace.execution_capacity);
        #[cfg(feature = "parallel")]
        let all_inputs_safe = if execution_threads > 1 && order_count > 1 {
            order_spectra
                .par_chunks_mut(ntheta)
                .enumerate()
                .with_min_len(order_count.div_ceil(execution_threads))
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
            self.fft.inverse_bounded(
                &workspace.spectra[spectrum_start..spectrum_start + spectrum_len],
                &mut workspace.row,
                &mut workspace.fft,
            )?;
            let map_start = ring * nphi;
            map[map_start..map_start + nphi].copy_from_slice(&workspace.row);
        }
        #[cfg(test)]
        SCALAR_TABLE_SYNTHESIS_CALLS.fetch_add(1, Ordering::Relaxed);
        Ok(true)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Each synthesis group uses plan geometry and its caller-owned worker scratch."
    )]
    pub(super) fn synthesize_ring_group(
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
        self.synthesize_ring_group_impl::<false>(
            alm,
            nphi,
            spectrum_len,
            phi0,
            group_ring_start,
            spectra,
            theta_sin,
            theta_cos,
            ring_order_limits,
            arch,
            recurrence,
            None,
        )
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "The cached synthesis group receives plan geometry, optional shared coefficients, and worker scratch."
    )]
    pub(super) fn synthesize_ring_group_with_scalar_coefficients(
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
        scalar_coefficients: Option<ScalarSynthesisCoefficientCache<'_, T>>,
    ) -> Result<(), ShtError> {
        if let Some(scalar_coefficients) = scalar_coefficients {
            self.synthesize_ring_group_impl::<true>(
                alm,
                nphi,
                spectrum_len,
                phi0,
                group_ring_start,
                spectra,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                recurrence,
                Some(scalar_coefficients),
            )
        } else {
            self.synthesize_ring_group(
                alm,
                nphi,
                spectrum_len,
                phi0,
                group_ring_start,
                spectra,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                recurrence,
            )
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The synthesis group receives plan geometry, optional shared coefficients, and worker scratch."
    )]
    fn synthesize_ring_group_impl<const USE_SCALAR_COEFFICIENT_CACHE: bool>(
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
        scalar_coefficients: Option<ScalarSynthesisCoefficientCache<'_, T>>,
    ) -> Result<(), ShtError> {
        #[cfg(all(test, feature = "parallel"))]
        recurrence.record_active_worker();
        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        debug_assert_eq!(spectra.len(), theta_sin.len() * spectrum_len);
        debug_assert_eq!(theta_sin.len(), theta_cos.len());

        let nyquist_scale = T::from_f64(2.0).expect("two is representable");
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
            let coefficient_pair_count = (lmax - m + 2) / 2;
            let cached_pair = if USE_SCALAR_COEFFICIENT_CACHE {
                scalar_coefficients.map(|cache| {
                    let row_start = m * cache.row_stride;
                    let row = &cache.values[row_start..row_start + cache.row_stride];
                    (
                        &row[..coefficient_pair_count],
                        &row[cache.half_stride..cache.half_stride + coefficient_pair_count],
                    )
                })
            } else {
                None
            };
            let coefficients_prepared = paired_factors_ready
                && if cached_pair.is_some() {
                    true
                } else {
                    recurrence.fast_spin_first.len() >= coefficient_pair_count
                        && recurrence.fast_spin_second.len() >= coefficient_pair_count
                        && T::prepare_paired_scalar_synthesis_coefficients(
                            m,
                            lmax,
                            &recurrence.epsilon,
                            &recurrence.spin_alpha,
                            &alm[coefficient_start..],
                            &mut recurrence.fast_spin_first[..coefficient_pair_count],
                            &mut recurrence.fast_spin_second[..coefficient_pair_count],
                        )
                };
            let (fast_first, fast_second) = match cached_pair {
                Some((first, second)) => (first, second),
                None => (
                    recurrence
                        .fast_spin_first
                        .get(..coefficient_pair_count)
                        .unwrap_or(&[]),
                    recurrence
                        .fast_spin_second
                        .get(..coefficient_pair_count)
                        .unwrap_or(&[]),
                ),
            };
            let phase = if m == 0 {
                Complex::new(T::one(), T::zero())
            } else {
                longitude_phase::<T>(m, phi0, false)
            };
            let paired_used = if paired_factors_ready && coefficients_prepared {
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
                    let fused = T::synthesize_paired_scalar_block_partitioned_dispatched(
                        arch,
                        m,
                        lmax,
                        &theta_sin[block_start..block_end],
                        &theta_cos[block_start..block_end],
                        self.sectoral_norms[m],
                        &recurrence.spin_coefficient_a,
                        &recurrence.spin_coefficient_b,
                        fast_first,
                        fast_second,
                        ring_limits,
                        spectrum_len,
                        &mut spectra[spectrum_start..spectrum_end],
                        &mut recurrence.previous[..block_len],
                        &mut recurrence.current[..block_len],
                        &mut recurrence.next[..block_len],
                        &mut recurrence.exponents[..block_len],
                        &mut recurrence.compact_theta_sin[..block_len],
                        &mut recurrence.compact_theta_cos[..block_len],
                        &mut recurrence.compact_ring_indices[..block_len],
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
                    spectra[spectrum_index] = phased;
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

        self.analyze_map_to_coefficients(map, alm, workspace, Some(self.grid.ring_weights()), true)
    }

    /// Projects map samples onto scalar harmonics using native ring weights.
    ///
    /// This operation skips the CC meridional resampling used by exact
    /// analysis and does not require enough latitude rings to make the
    /// projection exact. On an under-resolved grid, the returned coefficients
    /// include quadrature and sampling error. Longitude sampling must still
    /// support every requested order.
    ///
    /// # Errors
    ///
    /// Returns an error for non-scalar spin, insufficient longitude sampling,
    /// a buffer length mismatch, or an incompatible workspace.
    pub fn analysis_projected_into(
        &self,
        map: &[T],
        alm: &mut [Complex<T>],
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.check_scalar_spin()?;
        self.check_analysis_longitude()?;
        check_len("map input", self.sample_count, map.len())?;
        check_len("coefficient output", self.harmonics.nalm(), alm.len())?;
        self.check_workspace(workspace)?;

        self.analyze_map_to_coefficients(map, alm, workspace, Some(self.grid.ring_weights()), false)
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

        self.analyze_map_to_coefficients(map, alm, workspace, None, false)
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
        if let Some(geometry) = &self.prepared_scalar_latitude_geometry {
            self.copy_scalar_latitude_geometry(geometry, workspace, false);
        } else {
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
        resample_cc: bool,
    ) -> Result<(), ShtError> {
        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        // All scalar analysis stages consume only the requested order prefix.
        // Keep unused longitude modes stale in the caller-owned scratch rather
        // than recombining, storing, and normalizing modes above mmax.
        let output_modes = self.harmonics.mmax() + 1;
        #[cfg(all(test, feature = "parallel"))]
        let mut phase_started = std::time::Instant::now();
        #[cfg(all(test, feature = "parallel"))]
        let phase_probe = workspace.transform_phase_probe();
        #[cfg(all(test, feature = "parallel"))]
        let mut resampling_was_parallel = false;
        let scalar_table_selected = self.scalar_legendre_strategy_with_capacity(
            LegendreOperation::Analysis,
            workspace.execution_capacity,
        ) == LegendreStrategy::CompactTable;
        let scalar_table_input_limit = scalar_table_selected
            .then(crate::legendre::ScalarLegendreTable::<T>::input_scale_limit);
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "strategy selection",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        #[cfg(feature = "parallel")]
        let parallel_fft = self.row_fft_parallelism_supported()
            && workspace
                .execution_capacity
                .min(rayon::current_num_threads())
                > 1;
        #[cfg(feature = "parallel")]
        {
            if self.paired_scalar_odd_forward_fft_supported() {
                self.forward_analysis_rows_paired(
                    map,
                    &mut workspace.spectra,
                    output_modes,
                    &mut workspace.fft,
                    &mut workspace.fft_worker_workspaces,
                    if parallel_fft {
                        workspace.execution_capacity
                    } else {
                        1
                    },
                )?;
            } else if parallel_fft {
                self.forward_analysis_rows(
                    map,
                    &mut workspace.spectra,
                    output_modes,
                    &mut workspace.fft,
                    &mut workspace.fft_worker_workspaces,
                    workspace.execution_capacity,
                )?;
            } else {
                for ring in 0..ntheta {
                    let input_start = ring * nphi;
                    let spectrum_start = ring * spectrum_len;
                    self.fft.forward_truncated(
                        &map[input_start..input_start + nphi],
                        &mut workspace.spectra[spectrum_start..spectrum_start + output_modes],
                        &mut workspace.fft,
                    )?;
                }
            }
        }
        #[cfg(not(feature = "parallel"))]
        {
            if self.paired_scalar_odd_forward_fft_supported() {
                self.forward_analysis_rows_paired(
                    map,
                    &mut workspace.spectra,
                    output_modes,
                    &mut workspace.fft,
                )?;
            } else {
                for ring in 0..ntheta {
                    let input_start = ring * nphi;
                    let spectrum_start = ring * spectrum_len;
                    self.fft.forward_truncated(
                        &map[input_start..input_start + nphi],
                        &mut workspace.spectra[spectrum_start..spectrum_start + output_modes],
                        &mut workspace.fft,
                    )?;
                }
            }
        }
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(&phase_probe, "forward FFT", phase_started);
            phase_started = std::time::Instant::now();
        }

        let mut spectra_are_prepared = false;
        let (analysis_spectra, analysis_ntheta, analysis_ring_weights) = if let Some(resampling) =
            self.cc_resampling
                .as_ref()
                .filter(|_| ring_weights.is_some() && resample_cc)
        {
            spectra_are_prepared = true;
            let longitude_scale = T::from_usize(nphi).expect("longitude count converts");
            for spectrum in workspace
                .spectra
                .chunks_exact_mut(spectrum_len)
                .take(ntheta)
            {
                for value in &mut spectrum[..output_modes] {
                    *value /= longitude_scale;
                }
            }
            #[cfg(feature = "parallel")]
            let (use_parallel_resampling, resampling_workers) = {
                let order_count = self.harmonics.mmax() + 1;
                let active_workers = rayon::current_num_threads()
                    .min(workspace.execution_capacity)
                    .min(workspace.cc_resampling_workers.len())
                    .min(order_count);
                let estimated_order_ring_pairs =
                    order_count.saturating_mul(resampling.prepared_ntheta());
                (
                    T::supports_precomputed_scalar()
                        && resampling.needs_resampling()
                        && estimated_order_ring_pairs
                            >= crate::grid::resampling::CC_RESAMPLING_PARALLEL_MIN_ORDER_RING_PAIRS
                        && active_workers > 1,
                    active_workers,
                )
            };
            #[cfg(feature = "parallel")]
            if use_parallel_resampling {
                resampling.to_prepared_parallel(
                    &workspace.spectra,
                    &mut workspace.prepared_spectra,
                    spectrum_len,
                    self.harmonics.mmax(),
                    0,
                    &mut workspace.cc_resampling_order_tile,
                    &mut workspace.cc_resampling_workers,
                    resampling_workers,
                )?;
            } else {
                resampling.to_prepared(
                    &workspace.spectra,
                    &mut workspace.prepared_spectra,
                    spectrum_len,
                    self.harmonics.mmax(),
                    0,
                    &mut workspace.cc_profile,
                    &mut workspace.cc_fft_scratch,
                )?;
            }
            #[cfg(not(feature = "parallel"))]
            resampling.to_prepared(
                &workspace.spectra,
                &mut workspace.prepared_spectra,
                spectrum_len,
                self.harmonics.mmax(),
                0,
                &mut workspace.cc_profile,
                &mut workspace.cc_fft_scratch,
            )?;
            #[cfg(all(test, feature = "parallel"))]
            {
                resampling_was_parallel = use_parallel_resampling;
            }
            if let Some(geometry) = &self.prepared_scalar_latitude_geometry {
                self.copy_scalar_latitude_geometry(geometry, workspace, true);
            } else {
                for (ring, &theta) in resampling.prepared_theta().iter().enumerate() {
                    (workspace.theta_sin[ring], workspace.theta_cos[ring]) =
                        sin_cos_from_f64::<T>(theta);
                    workspace.theta_cos_squared[ring] = stable_analysis_cosine_squared(
                        workspace.theta_sin[ring],
                        workspace.theta_cos[ring],
                    );
                    workspace.ring_order_limits[ring] = T::polar_order_limit(
                        self.harmonics.lmax(),
                        0,
                        workspace.theta_sin[ring],
                        workspace.theta_cos[ring],
                    );
                }
            }
            (
                workspace.prepared_spectra.as_slice(),
                resampling.prepared_ntheta(),
                None,
            )
        } else {
            if let Some(geometry) = &self.scalar_latitude_geometry {
                self.copy_scalar_latitude_geometry(geometry, workspace, true);
            } else {
                for (ring, &theta) in self.grid.theta().iter().enumerate() {
                    (workspace.theta_sin[ring], workspace.theta_cos[ring]) =
                        sin_cos_from_f64::<T>(theta);
                    workspace.theta_cos_squared[ring] = stable_analysis_cosine_squared(
                        workspace.theta_sin[ring],
                        workspace.theta_cos[ring],
                    );
                    workspace.ring_order_limits[ring] = T::polar_order_limit(
                        self.harmonics.lmax(),
                        0,
                        workspace.theta_sin[ring],
                        workspace.theta_cos[ring],
                    );
                }
            }
            (workspace.spectra.as_slice(), ntheta, ring_weights)
        };
        // Prepared CC/F1 spectra already include their meridional weighting;
        // GL direct spectra carry their ring weights here. In both cases the
        // checked DUCC prefix route sees the final spectrum scale.
        let weighted_gl_spectra =
            analysis_ring_weights.is_some() && self.grid.kind() == GridKind::GaussLegendre;
        let allow_insignificant_prefix = (weighted_gl_spectra || spectra_are_prepared)
            && T::scalar_analysis_prefix_skip_allowed(analysis_ntheta);
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                if resampling_was_parallel {
                    "resampling (parallel orders) and latitude setup"
                } else {
                    "resampling (serial orders) and latitude setup"
                },
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
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
        let order_preparation = ScalarAnalysisOrderPreparation {
            analysis_spectra,
            spectrum_len,
            phi0: self.grid.phi0(),
            ring_order_limits,
            scalar_table_selected,
            ring_weights: analysis_ring_weights,
            longitude_count,
            scalar_table_input_limit,
        };
        let direct_pair_source = if !scalar_table_selected
            && T::supports_paired_scalar()
            && self.harmonics.lmax() >= T::FUSED_SCALAR_ANALYSIS_MIN_LMAX
        {
            if analysis_ring_weights.is_none() || analysis_ntheta == ntheta {
                Some(ScalarAnalysisDirectSpectra {
                    values: analysis_spectra,
                    spectrum_len,
                    phi0: self.grid.phi0(),
                    ring_weights: analysis_ring_weights,
                    longitude_count,
                })
            } else {
                None
            }
        } else {
            None
        };
        #[cfg(feature = "parallel")]
        let scalar_table_inputs_safe = {
            if direct_pair_source.is_some() {
                true
            } else {
                let worker_count = rayon::current_num_threads()
                    .min(workspace.execution_capacity)
                    .min(order_count);
                if !scalar_table_selected && worker_count > 1 {
                    order_spectra
                        .par_chunks_mut(analysis_ntheta)
                        .enumerate()
                        .with_min_len(order_count.div_ceil(worker_count))
                        .for_each(|(order, order_row)| {
                            let inputs_safe = order_preparation.prepare_order(order, order_row);
                            debug_assert!(inputs_safe);
                        });
                    true
                } else {
                    order_preparation.prepare_serial(order_spectra)
                }
            }
        };
        #[cfg(not(feature = "parallel"))]
        let scalar_table_inputs_safe = if direct_pair_source.is_some() {
            true
        } else {
            order_preparation.prepare_serial(order_spectra)
        };
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "order spectrum preparation",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        let scalar_table = if scalar_table_selected && scalar_table_inputs_safe {
            self.get_scalar_legendre_table()
        } else {
            None
        };
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "Legendre table retrieval",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }

        let lmax = self.harmonics.lmax();
        let mmax = self.harmonics.mmax();
        if let Some(table) = scalar_table {
            #[cfg(test)]
            SCALAR_TABLE_ANALYSIS_CALLS.fetch_add(1, Ordering::Relaxed);
            // The compact-table accumulator writes through every coefficient
            // by addition, so retain its whole-output initialization.
            alm.fill(Complex::new(T::zero(), T::zero()));
            #[cfg(all(test, feature = "parallel"))]
            {
                crate::workspace::record_transform_phase(
                    &phase_probe,
                    "coefficient initialization",
                    phase_started,
                );
                phase_started = std::time::Instant::now();
            }
            self.analyze_scalar_table(
                order_spectra,
                alm,
                table,
                workspace.execution_capacity,
                &mut workspace.recurrence_workers,
            );
            #[cfg(all(test, feature = "parallel"))]
            {
                crate::workspace::record_transform_phase(
                    &phase_probe,
                    "Legendre recurrence and accumulation",
                    phase_started,
                );
                phase_started = std::time::Instant::now();
            }
            for l in 0..=lmax {
                let index = self
                    .harmonics
                    .index(l, 0)
                    .expect("order zero contains every degree");
                alm[index].im = T::zero();
            }
            #[cfg(all(test, feature = "parallel"))]
            crate::workspace::record_transform_phase(
                &phase_probe,
                "coefficient cleanup",
                phase_started,
            );
            return Ok(());
        }
        #[cfg(test)]
        SCALAR_RECURRENCE_ANALYSIS_CALLS.fetch_add(1, Ordering::Relaxed);
        let scalar_analysis_spectra = if let Some(source) = direct_pair_source {
            ScalarAnalysisSpectra::Direct(source)
        } else {
            ScalarAnalysisSpectra::OrderMajor {
                values: order_spectra,
                latitude_count: analysis_ntheta,
            }
        };
        // The f64 paired recurrence owns a disjoint coefficient slice per
        // harmonic order. Initialize each slice on its worker immediately
        // before accumulation instead of serially clearing the full output.
        let order_local_init = T::supports_paired_scalar();
        if !order_local_init {
            alm.fill(Complex::new(T::zero(), T::zero()));
        }
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "coefficient initialization",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        let arch = pulp::Arch::new();
        let selected_arch = std::mem::discriminant(&arch);
        let same_plan = workspace
            .analysis_pair_range_cache_id
            .as_ref()
            .is_some_and(|cache_id| {
                std::sync::Arc::ptr_eq(cache_id, &self.analysis_pair_range_cache_id)
            });
        let cache_geometry = (analysis_ntheta, allow_insignificant_prefix);
        if !same_plan
            || workspace.analysis_pair_range_arch != Some(selected_arch)
            || workspace.analysis_pair_range_geometry != Some(cache_geometry)
        {
            for cached in &workspace.analysis_pair_range_safe {
                cached.store(false, Ordering::Relaxed);
            }
            workspace.analysis_pair_range_cache_id =
                Some(std::sync::Arc::clone(&self.analysis_pair_range_cache_id));
            workspace.analysis_pair_range_arch = Some(selected_arch);
            workspace.analysis_pair_range_geometry = Some(cache_geometry);
        }

        #[cfg(feature = "parallel")]
        {
            let order_count = mmax + 1;
            let worker_count = rayon::current_num_threads()
                .min(workspace.execution_capacity)
                .min(workspace.recurrence_workers.len())
                .min(order_count);
            let use_parallel = rayon::current_num_threads() > 1 && worker_count > 1;
            if use_parallel {
                self.analyze_order_jobs_dynamic(
                    scalar_analysis_spectra,
                    analysis_ntheta,
                    theta_sin,
                    theta_cos,
                    &workspace.theta_cos_squared[..analysis_ntheta],
                    ring_order_limits,
                    arch,
                    allow_insignificant_prefix,
                    &workspace.analysis_pair_range_safe,
                    alm,
                    &mut workspace.recurrence_workers[..worker_count],
                )?;
                #[cfg(all(test, feature = "parallel"))]
                {
                    crate::workspace::record_transform_phase(
                        &phase_probe,
                        "Legendre recurrence and accumulation",
                        phase_started,
                    );
                    phase_started = std::time::Instant::now();
                }
            } else {
                #[cfg(all(test, feature = "parallel"))]
                {
                    phase_started = std::time::Instant::now();
                }
                self.analyze_order_group(
                    scalar_analysis_spectra,
                    analysis_ntheta,
                    theta_sin,
                    theta_cos,
                    &workspace.theta_cos_squared[..analysis_ntheta],
                    ring_order_limits,
                    arch,
                    allow_insignificant_prefix,
                    &workspace.analysis_pair_range_safe,
                    0,
                    order_count,
                    alm,
                    &mut workspace.recurrence_workers[0],
                )?;
                #[cfg(all(test, feature = "parallel"))]
                {
                    crate::workspace::record_transform_phase(
                        &phase_probe,
                        "Legendre recurrence and accumulation",
                        phase_started,
                    );
                    phase_started = std::time::Instant::now();
                }
            }
        }
        #[cfg(not(feature = "parallel"))]
        {
            let order_count = mmax + 1;
            self.analyze_order_group(
                scalar_analysis_spectra,
                analysis_ntheta,
                theta_sin,
                theta_cos,
                &workspace.theta_cos_squared[..analysis_ntheta],
                ring_order_limits,
                arch,
                allow_insignificant_prefix,
                &workspace.analysis_pair_range_safe,
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
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "coefficient cleanup",
            phase_started,
        );
        Ok(())
    }

    fn analyze_scalar_table(
        &self,
        order_spectra: &[Complex<T>],
        alm: &mut [Complex<T>],
        table: &crate::legendre::ScalarLegendreTable<T>,
        execution_capacity: usize,
        recurrence: &mut [crate::workspace::RecurrenceWorkspace<T>],
    ) {
        let order_count = self.harmonics.mmax() + 1;
        #[cfg(not(feature = "parallel"))]
        let _ = execution_capacity;
        #[cfg(feature = "parallel")]
        {
            let workers = rayon::current_num_threads()
                .min(execution_capacity)
                .min(recurrence.len())
                .min(order_count)
                .max(1);
            self.analyze_scalar_table_groups(
                order_spectra,
                table,
                0,
                order_count,
                alm,
                recurrence,
                workers,
            );
        }
        #[cfg(not(feature = "parallel"))]
        self.analyze_scalar_table_group(
            order_spectra,
            table,
            0,
            order_count,
            alm,
            recurrence
                .first_mut()
                .expect("the table analysis has one recurrence scratch slot"),
        );
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "Parallel table groups divide coefficient outputs and per-worker scratch together."
    )]
    fn analyze_scalar_table_groups(
        &self,
        order_spectra: &[Complex<T>],
        table: &crate::legendre::ScalarLegendreTable<T>,
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        recurrence: &mut [crate::workspace::RecurrenceWorkspace<T>],
        worker_count: usize,
    ) {
        if m_end - m_start <= 1 || worker_count <= 1 {
            self.analyze_scalar_table_group(
                order_spectra,
                table,
                m_start,
                m_end,
                alm,
                recurrence
                    .first_mut()
                    .expect("each table-analysis group has worker scratch"),
            );
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
        let (left_recurrence, right_recurrence) = recurrence.split_at_mut(left_worker_count);
        rayon::join(
            || {
                self.analyze_scalar_table_groups(
                    order_spectra,
                    table,
                    m_start,
                    m_mid,
                    left_alm,
                    left_recurrence,
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
                    right_recurrence,
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
        recurrence: &mut crate::workspace::RecurrenceWorkspace<T>,
    ) {
        // Table analysis does not use paired recurrence coefficients, so its
        // per-order reflected spectra reuse these already-sized worker buffers.
        let scratch_len = table.north_ring_count();
        let even_scratch_len = scratch_len.min(recurrence.fast_scalar_first.len());
        let odd_scratch_len = scratch_len.min(recurrence.fast_scalar_second.len());
        T::analyze_scalar_table_group(
            order_spectra,
            table,
            self.grid.ntheta(),
            self.harmonics.lmax(),
            m_start,
            m_end,
            alm,
            &mut recurrence.fast_scalar_first[..even_scratch_len],
            &mut recurrence.fast_scalar_second[..odd_scratch_len],
        );
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "Dynamic analysis jobs share plan geometry and own disjoint coefficient slices."
    )]
    fn analyze_order_jobs_dynamic(
        &self,
        spectra: ScalarAnalysisSpectra<'_, T>,
        latitude_count: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        theta_cos_squared: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        allow_insignificant_prefix: bool,
        analysis_pair_range_safe: &[AtomicBool],
        alm: &mut [Complex<T>],
        recurrence: &mut [RecurrenceWorkspace<T>],
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let order_count = self.harmonics.mmax() + 1;
        if order_count <= 1 || recurrence.len() <= 1 {
            return self.analyze_order_group(
                spectra,
                latitude_count,
                theta_sin,
                theta_cos,
                theta_cos_squared,
                ring_order_limits,
                arch,
                allow_insignificant_prefix,
                analysis_pair_range_safe,
                0,
                order_count,
                alm,
                recurrence
                    .first_mut()
                    .expect("each analysis group has at least one recurrence workspace"),
            );
        }

        let mut order_jobs = VecDeque::new();
        if order_jobs.try_reserve_exact(order_count).is_err() {
            return self.analyze_order_group(
                spectra,
                latitude_count,
                theta_sin,
                theta_cos,
                theta_cos_squared,
                ring_order_limits,
                arch,
                allow_insignificant_prefix,
                analysis_pair_range_safe,
                0,
                order_count,
                alm,
                &mut recurrence[0],
            );
        }
        let mut remaining = alm;
        for m in 0..order_count {
            let coefficient_count = lmax - m + 1;
            let (coefficients, tail) = remaining.split_at_mut(coefficient_count);
            order_jobs.push_back((m, coefficients));
            remaining = tail;
        }
        debug_assert!(remaining.is_empty());

        let profile_enabled = |worker: &RecurrenceWorkspace<T>| {
            #[cfg(test)]
            {
                worker.analysis_schedule_profile_enabled()
            }
            #[cfg(not(test))]
            {
                let _ = worker;
                false
            }
        };
        run_dynamic_order_jobs(
            order_jobs,
            recurrence,
            profile_enabled,
            |worker, (m, coefficients), profile_worker, queue_lock_ns| {
                #[cfg(test)]
                let job_started = profile_worker.then(std::time::Instant::now);
                let result = self.analyze_order_group(
                    spectra,
                    latitude_count,
                    theta_sin,
                    theta_cos,
                    theta_cos_squared,
                    ring_order_limits,
                    arch,
                    allow_insignificant_prefix,
                    analysis_pair_range_safe,
                    m,
                    m + 1,
                    coefficients,
                    worker,
                );
                #[cfg(test)]
                if let Some(started) = job_started {
                    worker.record_analysis_schedule_job(
                        m,
                        lmax,
                        ring_order_limits,
                        started.elapsed().as_nanos(),
                        queue_lock_ns,
                    );
                }
                #[cfg(not(test))]
                let _ = (profile_worker, queue_lock_ns);
                result
            },
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The recurrence group uses plan geometry, grid weights and caller-owned worker scratch."
    )]
    fn analyze_order_group(
        &self,
        spectra: ScalarAnalysisSpectra<'_, T>,
        latitude_count: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        theta_cos_squared: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        allow_insignificant_prefix: bool,
        analysis_pair_range_safe: &[AtomicBool],
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
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
        let result = self.analyze_symmetric_order_group(
            spectra,
            latitude_count,
            theta_sin,
            theta_cos,
            theta_cos_squared,
            ring_order_limits,
            arch,
            allow_insignificant_prefix,
            analysis_pair_range_safe,
            m_start,
            m_end,
            alm,
            recurrence,
        );
        #[cfg(all(test, feature = "parallel"))]
        if let Some(started) = profile_started {
            recurrence.record_analysis_leaf(
                false,
                self.harmonics.lmax(),
                m_start,
                m_end,
                ring_order_limits,
                started,
            );
        }
        result
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The symmetric analysis group receives plan geometry and worker scratch explicitly."
    )]
    #[allow(unexpected_cfgs)]
    fn analyze_symmetric_order_group(
        &self,
        spectra: ScalarAnalysisSpectra<'_, T>,
        latitude_count: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        theta_cos_squared: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        allow_insignificant_prefix: bool,
        analysis_pair_range_safe: &[AtomicBool],
        m_start: usize,
        m_end: usize,
        alm: &mut [Complex<T>],
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        // GL, CC, F1 and prepared meridional grids use mirrored colatitudes.
        // Scalar harmonics satisfy Y_lm(pi-theta) = (-1)^(l+m) Y_lm(theta).
        debug_assert_eq!(theta_sin.len(), latitude_count);
        debug_assert_eq!(theta_cos.len(), latitude_count);
        debug_assert_eq!(theta_cos_squared.len(), latitude_count);
        debug_assert_eq!(ring_order_limits.len(), latitude_count);
        let lmax = self.harmonics.lmax();
        let pair_lane_count = latitude_count.div_ceil(2);
        let pair_block_size = if T::supports_paired_scalar() {
            SCALAR_ANALYSIS_PAIR_TILE_SIZE
        } else {
            LATITUDE_BLOCK_SIZE
        };
        let range_cache_block_count = pair_lane_count.div_ceil(pair_block_size);
        debug_assert!(recurrence.previous.len() >= pair_block_size);
        let paired_simd_width = if T::supports_paired_scalar() {
            arch.dispatch(F64LaneCount).max(1)
        } else {
            1
        };
        let mut coefficient_start = 0;
        let symmetric_monotone_limits =
            symmetric_monotone_analysis_pair_limits(latitude_count, ring_order_limits);
        #[cfg(all(test, feature = "parallel"))]
        let profiling = recurrence.analysis_leaf_profile_enabled();

        for m in m_start..m_end {
            let coefficient_count = lmax - m + 1;
            let coefficients = &mut alm[coefficient_start..coefficient_start + coefficient_count];
            if T::supports_paired_scalar() {
                coefficients.fill(Complex::new(T::zero(), T::zero()));
            }
            let first_active_pair =
                first_active_symmetric_pair(m, latitude_count, ring_order_limits);
            let order_spectra = spectra.for_order(m);
            T::fill_epsilon(m, lmax, &mut recurrence.epsilon);
            let mut paired_used = T::fill_paired_scalar_factors(
                m,
                lmax,
                &recurrence.epsilon,
                &mut recurrence.spin_alpha,
                &mut recurrence.spin_coefficient_a,
                &mut recurrence.spin_coefficient_b,
            );
            for block_start in (first_active_pair..pair_lane_count).step_by(pair_block_size) {
                let block_end = (block_start + pair_block_size).min(pair_lane_count);
                let block_len = block_end - block_start;
                let compact_candidate =
                    T::supports_paired_scalar() && lmax >= 127 && !symmetric_monotone_limits;
                let lane_count = prepare_symmetric_analysis_pair_block(
                    m,
                    latitude_count,
                    block_start,
                    block_len,
                    compact_candidate,
                    symmetric_monotone_limits,
                    order_spectra,
                    ring_order_limits,
                    theta_sin,
                    theta_cos,
                    theta_cos_squared,
                    &mut recurrence.analysis_same_real,
                    &mut recurrence.analysis_same_imag,
                    &mut recurrence.analysis_opposite_real,
                    &mut recurrence.analysis_opposite_imag,
                    &mut recurrence.compact_ring_indices,
                    &mut recurrence.compact_theta_sin,
                    &mut recurrence.compact_theta_cos,
                    &mut recurrence.compact_theta_cos_squared,
                );
                if lane_count == 0 {
                    continue;
                }
                let cache_index = m
                    .checked_mul(range_cache_block_count)
                    .and_then(|order_index| order_index.checked_add(block_start / pair_block_size));
                let range_cache_enabled = !cfg!(rustsht_disable_analysis_pair_range_cache);
                let range_check_is_proven_safe = range_cache_enabled
                    && cache_index
                        .and_then(|index| analysis_pair_range_safe.get(index))
                        .is_some_and(|safe| safe.load(Ordering::Relaxed));
                // Recurrence state depends on this plan's fixed factors and
                // active geometry, not the map spectrum. Cache each block only
                // after its checked fused traversal has exercised group checks.
                let mut may_cache_range_safe = range_cache_enabled
                    && !range_check_is_proven_safe
                    && !cfg!(rustsht_unchecked_analysis_pair_range);
                let mut saw_successful_fused_block = false;
                #[cfg(all(test, feature = "parallel"))]
                let (mut block_saw_normal, mut block_saw_scaled) = (false, false);
                let compact_mixed_block = compact_candidate && lane_count < block_len;
                let fast_path_candidate = paired_used
                    && lmax >= T::FUSED_SCALAR_ANALYSIS_MIN_LMAX
                    && recurrence.fast_scalar_first.len() >= coefficient_count;
                let fused_lane_count = if fast_path_candidate {
                    lane_count.div_ceil(paired_simd_width) * paired_simd_width
                } else {
                    lane_count
                };
                debug_assert!(fused_lane_count <= pair_block_size);
                if fused_lane_count > lane_count {
                    if !compact_candidate {
                        for lane in 0..lane_count {
                            let north_ring = block_start + lane;
                            recurrence.compact_theta_sin[lane] = theta_sin[north_ring];
                            recurrence.compact_theta_cos[lane] = theta_cos[north_ring];
                            recurrence.compact_theta_cos_squared[lane] =
                                theta_cos_squared[north_ring];
                        }
                    }
                    // Equatorial dummy lanes keep the padded recurrence finite;
                    // zero spectra make their coefficient contribution exact zero.
                    for lane in lane_count..fused_lane_count {
                        recurrence.compact_theta_sin[lane] = T::one();
                        recurrence.compact_theta_cos[lane] = T::zero();
                        recurrence.compact_theta_cos_squared[lane] = T::zero();
                        recurrence.analysis_same_real[lane] = T::zero();
                        recurrence.analysis_same_imag[lane] = T::zero();
                        recurrence.analysis_opposite_real[lane] = T::zero();
                        recurrence.analysis_opposite_imag[lane] = T::zero();
                    }
                }
                let pair_sin = if fused_lane_count > lane_count {
                    &recurrence.compact_theta_sin[..fused_lane_count]
                } else if compact_mixed_block {
                    &recurrence.compact_theta_sin[..lane_count]
                } else {
                    &theta_sin[block_start..block_end]
                };
                let pair_cos = if fused_lane_count > lane_count {
                    &recurrence.compact_theta_cos[..fused_lane_count]
                } else if compact_mixed_block {
                    &recurrence.compact_theta_cos[..lane_count]
                } else {
                    &theta_cos[block_start..block_end]
                };
                let pair_cos_squared = if fused_lane_count > lane_count {
                    &recurrence.compact_theta_cos_squared[..fused_lane_count]
                } else if compact_mixed_block {
                    &recurrence.compact_theta_cos_squared[..lane_count]
                } else {
                    &theta_cos_squared[block_start..block_end]
                };
                if paired_used {
                    // Paired degrees l=m+2k use north+south for the even term;
                    // the intervening odd term uses north-south and cos(theta).
                    let staged_count = coefficient_count;
                    // DUCC's fill_m2a checks the packed spectra and cosine-weights
                    // opposite parity in one pass. Preserve the transformed
                    // values only when both range checks succeed.
                    let packed_preparation = if fast_path_candidate {
                        T::prepare_scalar_analysis_pair_inputs(
                            &recurrence.analysis_same_real[..fused_lane_count],
                            &recurrence.analysis_same_imag[..fused_lane_count],
                            &mut recurrence.analysis_opposite_real[..fused_lane_count],
                            &mut recurrence.analysis_opposite_imag[..fused_lane_count],
                            pair_cos,
                        )
                    } else {
                        None
                    };
                    let (product_range_safe, opposite_preparation) =
                        if let Some((products_safe, opposite_safe)) = packed_preparation {
                            (products_safe, Some(opposite_safe))
                        } else {
                            let products_safe = T::scalar_analysis_products_safe(
                                &recurrence.analysis_same_real[..fused_lane_count],
                                &recurrence.analysis_same_imag[..fused_lane_count],
                                &recurrence.analysis_opposite_real[..fused_lane_count],
                                &recurrence.analysis_opposite_imag[..fused_lane_count],
                                pair_cos,
                            );
                            let opposite_preparation = if fast_path_candidate && products_safe {
                                T::prepare_scalar_analysis_opposite(
                                    &mut recurrence.analysis_opposite_real[..fused_lane_count],
                                    &mut recurrence.analysis_opposite_imag[..fused_lane_count],
                                    pair_cos,
                                )
                            } else {
                                None
                            };
                            (products_safe, opposite_preparation)
                        };
                    let fast_path_eligible = opposite_preparation == Some(true);
                    let mut range_check_was_exercised = false;
                    let fused_block = if fast_path_eligible {
                        T::fused_scalar_analysis_pair_block_preweighted(
                            arch,
                            m,
                            lmax,
                            allow_insignificant_prefix,
                            range_check_is_proven_safe,
                            &mut range_check_was_exercised,
                            pair_sin,
                            pair_cos,
                            pair_cos_squared,
                            self.sectoral_norms[m],
                            &recurrence.spin_coefficient_a,
                            &recurrence.spin_coefficient_b,
                            &recurrence.analysis_same_real[..fused_lane_count],
                            &recurrence.analysis_same_imag[..fused_lane_count],
                            &recurrence.analysis_opposite_real[..fused_lane_count],
                            &recurrence.analysis_opposite_imag[..fused_lane_count],
                            &mut recurrence.fast_scalar_first[..staged_count],
                            &mut recurrence.previous,
                            &mut recurrence.current,
                            &mut recurrence.next,
                            &mut recurrence.analysis_scale_epochs,
                            &mut recurrence.analysis_scale_corrections,
                            &mut recurrence.exponents,
                        )?
                    } else {
                        false
                    };
                    if fused_block {
                        saw_successful_fused_block = true;
                        if !range_check_is_proven_safe && !range_check_was_exercised {
                            may_cache_range_safe = false;
                        }
                        #[cfg(all(test, feature = "parallel"))]
                        {
                            block_saw_normal = true;
                        }
                        for (coefficient, contribution) in coefficients
                            .iter_mut()
                            .zip(&recurrence.fast_scalar_first[..staged_count])
                        {
                            *coefficient += *contribution;
                        }
                    } else {
                        may_cache_range_safe = false;
                        if opposite_preparation.is_some() {
                            if !compact_candidate {
                                for (lane, north_ring) in recurrence.compact_ring_indices
                                    [..lane_count]
                                    .iter_mut()
                                    .enumerate()
                                {
                                    *north_ring = block_start + lane;
                                }
                            }
                            restore_symmetric_analysis_pair_opposite(
                                m,
                                latitude_count,
                                order_spectra,
                                ring_order_limits,
                                &recurrence.compact_ring_indices[..lane_count],
                                &mut recurrence.analysis_opposite_real[..lane_count],
                                &mut recurrence.analysis_opposite_imag[..lane_count],
                            );
                        }
                        let same_real = &recurrence.analysis_same_real[..lane_count];
                        let same_imag = &recurrence.analysis_same_imag[..lane_count];
                        let opposite_real = &recurrence.analysis_opposite_real[..lane_count];
                        let opposite_imag = &recurrence.analysis_opposite_imag[..lane_count];
                        paired_used = T::for_each_y_lm_paired_status_dispatched(
                            arch,
                            m,
                            lmax,
                            &pair_sin[..lane_count],
                            &pair_cos[..lane_count],
                            self.sectoral_norms[m],
                            &recurrence.spin_coefficient_a,
                            &recurrence.spin_coefficient_b,
                            &mut recurrence.previous,
                            &mut recurrence.current,
                            &mut recurrence.next,
                            &mut recurrence.exponents,
                            |l, ring_start, current, exponents, normal_range| {
                                #[cfg(all(test, feature = "parallel"))]
                                if profiling {
                                    if normal_range {
                                        block_saw_normal = true;
                                    } else {
                                        block_saw_scaled = true;
                                    }
                                }
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
                    if !fast_path_eligible {
                        may_cache_range_safe = false;
                    }
                } else {
                    may_cache_range_safe = false;
                }
                if !paired_used {
                    let same_real = &recurrence.analysis_same_real[..lane_count];
                    let same_imag = &recurrence.analysis_same_imag[..lane_count];
                    let opposite_real = &recurrence.analysis_opposite_real[..lane_count];
                    let opposite_imag = &recurrence.analysis_opposite_imag[..lane_count];
                    T::for_each_y_lm_dispatched(
                        arch,
                        m,
                        lmax,
                        &pair_sin[..lane_count],
                        &pair_cos[..lane_count],
                        self.sectoral_norms[m],
                        &recurrence.epsilon,
                        &mut recurrence.previous,
                        &mut recurrence.current,
                        &mut recurrence.next,
                        &mut recurrence.exponents,
                        |l, ring_start, current, exponents| {
                            #[cfg(all(test, feature = "parallel"))]
                            if profiling {
                                if exponents.iter().any(|&exponent| exponent != 0) {
                                    block_saw_scaled = true;
                                } else {
                                    block_saw_normal = true;
                                }
                            }
                            let coefficient = &mut coefficients[l - m];
                            let (spectrum_real, spectrum_imag) = if (l + m).is_multiple_of(2) {
                                (same_real, same_imag)
                            } else {
                                (opposite_real, opposite_imag)
                            };
                            for lane in 0..current.len() {
                                let ring = ring_start + lane;
                                *coefficient += T::multiply_complex_scaled(
                                    Complex::new(spectrum_real[ring], spectrum_imag[ring]),
                                    current[lane],
                                    exponents[lane],
                                );
                            }
                            Ok(())
                        },
                    )?;
                }
                #[cfg(all(test, feature = "parallel"))]
                if profiling {
                    recurrence.record_analysis_leaf_block(block_saw_normal, block_saw_scaled);
                }
                if may_cache_range_safe
                    && saw_successful_fused_block
                    && let Some(index) = cache_index
                    && let Some(safe) = analysis_pair_range_safe.get(index)
                {
                    safe.store(true, Ordering::Relaxed);
                }
            }
            if paired_used {
                finalize_paired_scalar_analysis_coefficients(
                    coefficients,
                    m,
                    lmax,
                    &recurrence.epsilon,
                    &recurrence.spin_alpha,
                );
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

    /// The reflected scalar recurrence is shared by the canonical GL, CC,
    /// and F1 grids. Keep CC/F1 on sampling-complete geometries until their
    /// pole and half-grid cases have passed the same numerical checks;
    /// under-resolved plans continue through the established general route.
    pub(super) fn scalar_ordered_synthesis_supported(&self) -> bool {
        if !T::supports_scalar_ordered() {
            return false;
        }
        let minimum_ntheta = match self.grid.kind() {
            GridKind::GaussLegendre => return true,
            GridKind::ClenshawCurtis => self.harmonics.lmax().checked_add(2),
            GridKind::Fejer1 => self.harmonics.lmax().checked_add(1),
        };
        minimum_ntheta.is_some_and(|minimum| self.grid.ntheta() >= minimum)
    }
}
