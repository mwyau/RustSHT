// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
    #[expect(
        clippy::too_many_arguments,
        reason = "The shared STANDARD, GRAD_ONLY and DERIV1 driver receives two channels, outputs and recurrence mode."
    )]
    pub(super) fn synthesis_spin_core(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        first_component: &mut [T],
        second_component: &mut [T],
        recurrence_spin: usize,
        derivative_norm: bool,
        workspace: &mut ShtWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.synthesis_spin_core_with_hemisphere(
            e_alm,
            b_alm,
            first_component,
            second_component,
            recurrence_spin,
            derivative_norm,
            workspace,
            true,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The shared transform core receives channels, outputs, recurrence mode and strategy control."
    )]
    pub(super) fn synthesis_spin_core_with_hemisphere(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        first_component: &mut [T],
        second_component: &mut [T],
        recurrence_spin: usize,
        derivative_norm: bool,
        workspace: &mut ShtWorkspace<T>,
        allow_hemisphere: bool,
    ) -> Result<(), ShtError> {
        #[cfg(not(feature = "parallel"))]
        let _ = allow_hemisphere;

        if !matches!(recurrence_spin, 1..=MAX_SUPPORTED_SPIN) {
            return Err(ShtError::UnsupportedSpin {
                spin: recurrence_spin,
            });
        }
        if !derivative_norm && self.harmonics.spin() != recurrence_spin {
            return Err(ShtError::UnsupportedSpin {
                spin: self.harmonics.spin(),
            });
        }
        self.harmonics.validate_coefficients(e_alm)?;
        if let Some(b_alm) = b_alm {
            self.harmonics.validate_coefficients(b_alm)?;
        }
        check_len(
            "first spin map output",
            self.sample_count,
            first_component.len(),
        )?;
        check_len(
            "second spin map output",
            self.sample_count,
            second_component.len(),
        )?;
        self.check_synthesis_longitude()?;
        self.check_workspace(workspace)?;

        #[cfg(feature = "parallel")]
        if allow_hemisphere
            && recurrence_spin == 1
            && !derivative_norm
            && let Some(b_alm) = b_alm
            && hemisphere::is_beneficial(
                self,
                current_execution_thread_count().min(workspace.execution_capacity),
            )
        {
            return hemisphere::synthesize(
                self,
                e_alm,
                b_alm,
                first_component,
                second_component,
                workspace,
            );
        }

        #[cfg(all(test, feature = "parallel"))]
        let phase_probe = workspace.transform_phase_probe();
        #[cfg(all(test, feature = "parallel"))]
        let mut phase_started = std::time::Instant::now();

        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        let phi0 = self.grid.phi0();
        let allow_fast_scalar = !derivative_norm
            || (recurrence_spin == 1
                && self.harmonics.spin() == 0
                && T::supports_paired_spin()
                && self.harmonics.lmax() >= SPIN1_FAST_MIN_LMAX);
        let can_use_spin1_table = spin1_synthesis_table_band_is_beneficial(self.harmonics.lmax())
            && recurrence_spin == 1
            && self.harmonics.spin() == 1
            && b_alm.is_some_and(|b_coefficients| {
                self.spin1_scalar_legendre_table
                    .get()
                    .is_some_and(Option::is_some)
                    || spin1_synthesis_coefficients_safe_for_table(e_alm, b_coefficients)
            })
            && !derivative_norm
            && self.grid.kind() == crate::grid::GridKind::GaussLegendre
            && self.grid.ntheta() == self.harmonics.lmax() + 1
            && single_threaded_execution()
            && self.legendre_reuse_allows_table(
                LegendreOperation::Synthesis,
                self.spin1_scalar_legendre_table
                    .get()
                    .is_some_and(Option::is_some),
            )
            && self
                .grid
                .theta()
                .iter()
                .all(|&theta| theta.sin() >= SPIN1_PAIR_MIN_SIN_THETA);
        if can_use_spin1_table {
            self.spin1_scalar_legendre_table.get_or_init(|| {
                crate::legendre::ScalarLegendreTable::build_if_beneficial(
                    &self.grid,
                    &self.harmonics,
                    &self.sectoral_norms,
                )
            });
        }
        self.prepare_spin_latitudes(workspace, recurrence_spin);
        workspace
            .spectra
            .fill(Complex::<T>::new(T::zero(), T::zero()));
        workspace
            .spin_spectra
            .fill(Complex::<T>::new(T::zero(), T::zero()));
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "latitude and spin-spectrum setup",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }
        let arch = pulp::Arch::new();

        #[cfg(feature = "parallel")]
        {
            let pool_threads = current_execution_thread_count();
            let execution_threads = pool_threads.min(workspace.execution_capacity);
            let f1_synthesis_group_size =
                self.spin1_f1_synthesis_ring_group_size(execution_threads);
            let group_size = if !derivative_norm && b_alm.is_some() {
                self.fine_synthesis_ring_group_size(execution_threads)
                    .or(f1_synthesis_group_size)
                    .map_or(workspace.ring_group_size, |size| {
                        size.min(workspace.ring_group_size)
                    })
            } else {
                workspace.ring_group_size
            };
            let group_ring_count = group_size.min(ntheta);
            let group_count = ntheta.div_ceil(group_ring_count);
            let use_parallel = execution_threads > 1 && group_count > 1;
            if use_parallel {
                // Exact CC groups share the same order-independent E/B
                // conversion. Cache it only when this geometry has surplus
                // ring groups and enough order-ring work to amortize prepare.
                let cc_coefficients_reused_across_groups = self.grid.kind()
                    == crate::grid::GridKind::ClenshawCurtis
                    && self.grid.ntheta() == self.harmonics.lmax() + 2
                    && group_count > execution_threads
                    && (self.harmonics.mmax() + 1)
                        .checked_mul(ntheta)
                        .is_some_and(|work| work >= execution_threads.saturating_mul(12_000));
                let reusable_spin1_coefficient_geometry = (self.grid.kind()
                    == crate::grid::GridKind::GaussLegendre
                    && self.grid.ntheta() > self.harmonics.lmax())
                    || (self.grid.kind() == crate::grid::GridKind::Fejer1
                        && f1_synthesis_group_size.is_some())
                    || cc_coefficients_reused_across_groups;
                let use_spin1_coefficient_cache = recurrence_spin == 1
                    && self.harmonics.spin() == 1
                    && b_alm.is_some()
                    && !derivative_norm
                    && reusable_spin1_coefficient_geometry
                    && allow_fast_scalar
                    && self.harmonics.lmax() >= SPIN1_FAST_MIN_LMAX
                    && self.harmonics.lmax() <= SPIN1_SYNTH_COEFFICIENT_CACHE_MAX_LMAX
                    && T::supports_spin1_synthesis()
                    && execution_threads >= 8
                    && workspace.ring_group_size < ntheta;
                // For exact CC cache-enabled synthesis, use the existing
                // one-group queue when ring groups outnumber workers. With
                // fewer groups, retain the lower-overhead static iterator.
                let use_dynamic_spin1_group_queue = (use_spin1_coefficient_cache
                    && (self.grid.kind() != crate::grid::GridKind::ClenshawCurtis
                        || group_count > execution_threads))
                    || (recurrence_spin == 1
                        && !derivative_norm
                        && b_alm.is_some()
                        && f1_synthesis_group_size.is_some());
                let required_recurrence_workers = if use_dynamic_spin1_group_queue {
                    group_count.min(execution_threads)
                } else {
                    group_count
                };
                debug_assert!(required_recurrence_workers <= workspace.recurrence_workers.len());
                let order_tile_count = (self.harmonics.mmax() + 1).div_ceil(SPIN1_ORDER_MAJOR_TILE);
                let order_eligibility_tile_count =
                    (self.harmonics.mmax() + 1).div_ceil(SPIN1_ORDER_MAJOR_ELIGIBILITY_TILE);
                let order_major_queue_eligible = recurrence_spin == 1
                    && self.harmonics.spin() == 1
                    && !derivative_norm
                    && b_alm.is_some()
                    && execution_threads >= 8
                    && order_eligibility_tile_count >= execution_threads.saturating_mul(2)
                    && use_dynamic_spin1_group_queue
                    && use_spin1_coefficient_cache;
                let local_coefficient_cache_bytes = self
                    .harmonics
                    .lmax()
                    .checked_add(2)
                    .and_then(|row_len| row_len.checked_mul(2))
                    .and_then(|row_stride| {
                        self.harmonics
                            .mmax()
                            .checked_add(1)
                            .and_then(|order_count| row_stride.checked_mul(order_count))
                    })
                    .and_then(|coefficient_count| {
                        coefficient_count.checked_mul(std::mem::size_of::<Complex<T>>())
                    });
                let use_order_local_coefficients = order_major_queue_eligible
                    && local_coefficient_cache_bytes
                        .is_some_and(|bytes| bytes >= SPIN1_ORDER_LOCAL_CONVERSION_MIN_BYTES);
                #[cfg(all(test, feature = "parallel"))]
                let coefficient_prep_started = std::time::Instant::now();
                let spin1_coefficients_ready =
                    if use_spin1_coefficient_cache && !use_order_local_coefficients {
                        self.prepare_spin1_scalar_coefficients(
                            e_alm,
                            b_alm.expect("the cache requires both spin-1 channels"),
                            &mut workspace.spin1_scalar_coefficients,
                            &mut workspace.spin1_scalar_coefficient_finite_orders,
                            execution_threads,
                        )
                    } else {
                        false
                    };
                let order_worker_count = (order_major_queue_eligible
                    && (spin1_coefficients_ready || use_order_local_coefficients))
                    .then_some(
                        order_tile_count
                            .min(execution_threads)
                            .min(workspace.recurrence_workers.len()),
                    );
                if let Some(order_worker_count) = order_worker_count {
                    workspace.ensure_spin_order_output_scratch_count(order_worker_count, ntheta)?;
                }
                let spin1_coefficients = spin1_coefficients_ready
                    .then_some(workspace.spin1_scalar_coefficients.as_slice());
                let spin1_coefficient_finite_orders = spin1_coefficients_ready
                    .then_some(workspace.spin1_scalar_coefficient_finite_orders.as_slice());
                #[cfg(all(test, feature = "parallel"))]
                {
                    crate::workspace::record_transform_phase(
                        &phase_probe,
                        "spin-1 scalar coefficient cache preparation",
                        coefficient_prep_started,
                    );
                    phase_started = std::time::Instant::now();
                }
                let group_spectrum_len = group_ring_count.checked_mul(spectrum_len).ok_or(
                    ShtError::IntegerOverflow {
                        quantity: "spin Fourier spectrum group size",
                    },
                )?;
                if let Some(order_worker_count) = order_worker_count {
                    self.synthesize_spin1_order_major_queue(
                        e_alm,
                        b_alm.expect("order-major spin-1 synthesis requires B coefficients"),
                        nphi,
                        spectrum_len,
                        phi0,
                        spin1_coefficients,
                        spin1_coefficient_finite_orders,
                        &mut workspace.spectra,
                        &mut workspace.spin_spectra,
                        &workspace.theta_sin[..ntheta],
                        &workspace.theta_cos[..ntheta],
                        &workspace.ring_order_limits[..ntheta],
                        arch,
                        &mut workspace.recurrence_workers[..execution_threads],
                        &mut workspace.spin_order_output_scratch[..order_worker_count],
                    )?;
                    // The order-major queue wrote each complete order.
                } else if use_dynamic_spin1_group_queue {
                    // Spin-1 synthesis groups have strongly varying work near
                    // polar cutoffs. Queue one group at a time to balance the
                    // work, while limiting active workers to the capacity
                    // used to prepare this workspace.
                    let group_queue = Mutex::new(
                        workspace
                            .spectra
                            .chunks_mut(group_spectrum_len)
                            .zip(workspace.spin_spectra.chunks_mut(group_spectrum_len))
                            .enumerate(),
                    );
                    let worker_count = group_count.min(execution_threads);
                    workspace.recurrence_workers[..worker_count]
                        .par_iter_mut()
                        .try_for_each(|recurrence| {
                            #[cfg(all(test, feature = "parallel"))]
                            if recurrence.spin1_profile_enabled {
                                recurrence.spin1_profile_group_worker_id =
                                    rayon::current_thread_index().unwrap_or(usize::MAX);
                            }
                            loop {
                                let group = group_queue
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                                    .next();
                                let Some((group_index, (q_spectra, u_spectra))) = group else {
                                    break;
                                };
                                let ring_start = group_index * group_ring_count;
                                let ring_end =
                                    (ring_start + q_spectra.len() / spectrum_len).min(ntheta);
                                #[cfg(all(test, feature = "parallel"))]
                                let group_started = recurrence
                                    .spin1_profile_enabled
                                    .then(std::time::Instant::now);
                                let result = self.synthesize_spin_ring_group(
                                    e_alm,
                                    b_alm,
                                    recurrence_spin,
                                    derivative_norm,
                                    allow_fast_scalar,
                                    can_use_spin1_table,
                                    spin1_coefficients,
                                    spin1_coefficient_finite_orders,
                                    nphi,
                                    spectrum_len,
                                    phi0,
                                    ring_start,
                                    ring_end,
                                    q_spectra,
                                    u_spectra,
                                    &workspace.theta_sin[ring_start..ring_end],
                                    &workspace.theta_cos[ring_start..ring_end],
                                    &workspace.ring_order_limits,
                                    arch,
                                    recurrence,
                                );
                                #[cfg(all(test, feature = "parallel"))]
                                if let Some(group_started) = group_started {
                                    recurrence.spin1_profile_group_ns +=
                                        group_started.elapsed().as_nanos();
                                }
                                result?;
                            }
                            Ok(())
                        })?;
                } else {
                    // Fine-grain only when pairing groups would leave worker
                    // slots idle; finer chunks hurt small groups.
                    let minimum_group_chunk =
                        if T::prefers_fine_spin1_synthesis_groups(group_count, execution_threads)
                            && pool_threads <= workspace.execution_capacity
                            && recurrence_spin == 1
                            && self.harmonics.spin() == 1
                            && !derivative_norm
                            && b_alm.is_some()
                            && execution_threads > 1
                            && group_count > execution_threads
                        {
                            1
                        } else {
                            group_count.div_ceil(execution_threads)
                        };
                    workspace
                        .spectra
                        .par_chunks_mut(group_spectrum_len)
                        .zip(workspace.spin_spectra.par_chunks_mut(group_spectrum_len))
                        .zip(workspace.recurrence_workers[..group_count].par_iter_mut())
                        .enumerate()
                        .with_min_len(minimum_group_chunk)
                        .try_for_each(|(group_index, ((q_spectra, u_spectra), recurrence))| {
                            let ring_start = group_index * group_ring_count;
                            let ring_end =
                                (ring_start + q_spectra.len() / spectrum_len).min(ntheta);
                            self.synthesize_spin_ring_group(
                                e_alm,
                                b_alm,
                                recurrence_spin,
                                derivative_norm,
                                allow_fast_scalar,
                                can_use_spin1_table,
                                spin1_coefficients,
                                spin1_coefficient_finite_orders,
                                nphi,
                                spectrum_len,
                                phi0,
                                ring_start,
                                ring_end,
                                q_spectra,
                                u_spectra,
                                &workspace.theta_sin[ring_start..ring_end],
                                &workspace.theta_cos[ring_start..ring_end],
                                &workspace.ring_order_limits,
                                arch,
                                recurrence,
                            )
                        })?;
                }
            } else {
                self.synthesize_spin_ring_group(
                    e_alm,
                    b_alm,
                    recurrence_spin,
                    derivative_norm,
                    allow_fast_scalar,
                    can_use_spin1_table,
                    None,
                    None,
                    nphi,
                    spectrum_len,
                    phi0,
                    0,
                    ntheta,
                    &mut workspace.spectra,
                    &mut workspace.spin_spectra,
                    &workspace.theta_sin[..ntheta],
                    &workspace.theta_cos[..ntheta],
                    &workspace.ring_order_limits[..ntheta],
                    arch,
                    &mut workspace.recurrence_workers[0],
                )?;
            }
        }
        #[cfg(not(feature = "parallel"))]
        self.synthesize_spin_ring_group(
            e_alm,
            b_alm,
            recurrence_spin,
            derivative_norm,
            allow_fast_scalar,
            can_use_spin1_table,
            None,
            None,
            nphi,
            spectrum_len,
            phi0,
            0,
            ntheta,
            &mut workspace.spectra,
            &mut workspace.spin_spectra,
            &workspace.theta_sin[..ntheta],
            &workspace.theta_cos[..ntheta],
            &workspace.ring_order_limits[..ntheta],
            arch,
            &mut workspace.recurrence_workers[0],
        )?;

        #[cfg(all(test, feature = "parallel"))]
        {
            crate::workspace::record_transform_phase(
                &phase_probe,
                "order scheduling, Legendre recurrence, and spectral accumulation",
                phase_started,
            );
            phase_started = std::time::Instant::now();
        }

        self.inverse_synthesis_rows(
            &workspace.spectra,
            first_component,
            &mut workspace.fft,
            #[cfg(feature = "parallel")]
            &mut workspace.fft_worker_workspaces,
            #[cfg(feature = "parallel")]
            workspace.execution_capacity,
        )?;
        self.inverse_synthesis_rows(
            &workspace.spin_spectra,
            second_component,
            &mut workspace.fft,
            #[cfg(feature = "parallel")]
            &mut workspace.fft_worker_workspaces,
            #[cfg(feature = "parallel")]
            workspace.execution_capacity,
        )?;
        #[cfg(all(test, feature = "parallel"))]
        crate::workspace::record_transform_phase(
            &phase_probe,
            "two inverse longitude FFTs and map writeback",
            phase_started,
        );
        Ok(())
    }

    #[cfg(feature = "parallel")]
    pub(super) fn prepare_spin1_scalar_coefficients(
        &self,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        coefficients: &mut Vec<Complex<T>>,
        finite_orders: &mut Vec<u8>,
        execution_capacity: usize,
    ) -> bool {
        let row_len = match self.harmonics.lmax().checked_add(2) {
            Some(value) => value,
            None => return false,
        };
        let row_stride = match row_len.checked_mul(2) {
            Some(value) => value,
            None => return false,
        };
        let order_count = match self.harmonics.mmax().checked_add(1) {
            Some(value) => value,
            None => return false,
        };
        let coefficient_count = match row_stride.checked_mul(order_count) {
            Some(value) => value,
            None => return false,
        };
        let Some(byte_count) = coefficient_count.checked_mul(std::mem::size_of::<Complex<T>>())
        else {
            return false;
        };
        if byte_count > SPIN1_SYNTH_COEFFICIENT_CACHE_MAX_BYTES {
            return false;
        }
        if coefficients.capacity() < coefficient_count
            && coefficients
                .try_reserve_exact(coefficient_count.saturating_sub(coefficients.len()))
                .is_err()
        {
            return false;
        }
        if finite_orders.capacity() < order_count
            && finite_orders
                .try_reserve_exact(order_count.saturating_sub(finite_orders.len()))
                .is_err()
        {
            return false;
        }
        coefficients.resize(coefficient_count, Complex::new(T::zero(), T::zero()));
        finite_orders.resize(order_count, 0);

        let worker_count = current_execution_thread_count()
            .min(execution_capacity)
            .min(order_count)
            .max(1);
        coefficients
            .par_chunks_mut(row_stride)
            .zip(finite_orders.par_iter_mut())
            .enumerate()
            .with_min_len(order_count.div_ceil(worker_count))
            .for_each(|(m, (order_coefficients, finite))| {
                let coefficient_start = self
                    .harmonics
                    .index(m, m)
                    .expect("every supported order starts at its diagonal");
                let coefficient_len = self.harmonics.lmax() - m + 1;
                let (first, second) = order_coefficients.split_at_mut(row_len);
                spin1_to_scalar(
                    m,
                    self.harmonics.lmax(),
                    &e_alm[coefficient_start..coefficient_start + coefficient_len],
                    Some(&b_alm[coefficient_start..coefficient_start + coefficient_len]),
                    first,
                    second,
                );
                *finite = u8::from(
                    first
                        .iter()
                        .chain(second.iter())
                        .all(|value| value.re.is_finite() && value.im.is_finite()),
                );
            });
        true
    }

    #[cfg(feature = "parallel")]
    #[expect(
        clippy::too_many_arguments,
        reason = "Order-major synthesis tasks share transform inputs, outputs and worker scratch."
    )]
    pub(super) fn synthesize_spin1_order_major_queue(
        &self,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        spin1_scalar_coefficients: Option<&[Complex<T>]>,
        spin1_scalar_coefficient_finite_orders: Option<&[u8]>,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence_workers: &mut [RecurrenceWorkspace<T>],
        output_scratch: &mut [crate::workspace::SpinOrderOutputScratch<T>],
    ) -> Result<(), ShtError> {
        let ntheta = self.grid.ntheta();
        let order_count = self.harmonics.mmax() + 1;
        let tile_count = order_count.div_ceil(SPIN1_ORDER_MAJOR_TILE);
        let worker_count = tile_count
            .min(recurrence_workers.len())
            .min(output_scratch.len());
        debug_assert!(worker_count > 0);
        debug_assert_eq!(theta_sin.len(), ntheta);
        debug_assert_eq!(theta_cos.len(), ntheta);
        debug_assert_eq!(ring_order_limits.len(), ntheta);
        debug_assert_eq!(q_spectra.len(), ntheta * spectrum_len);
        debug_assert_eq!(u_spectra.len(), q_spectra.len());

        let q_output = crate::legendre::scalar_ordered::DisjointSpectrumOutput::new(
            q_spectra,
            ntheta,
            spectrum_len,
        )
        .expect("the plan workspace has a full Q spectrum");
        let u_output = crate::legendre::scalar_ordered::DisjointSpectrumOutput::new(
            u_spectra,
            ntheta,
            spectrum_len,
        )
        .expect("the plan workspace has a full U spectrum");
        let next_tile = AtomicUsize::new(0);

        recurrence_workers[..worker_count]
            .par_iter_mut()
            .zip(output_scratch[..worker_count].par_iter_mut())
            .try_for_each(|(recurrence, output_scratch)| {
                #[cfg(all(test, feature = "parallel"))]
                if recurrence.spin1_profile_enabled {
                    recurrence.spin1_profile_group_worker_id =
                        rayon::current_thread_index().unwrap_or(usize::MAX);
                }
                loop {
                    let tile_start = next_tile.fetch_add(SPIN1_ORDER_MAJOR_TILE, Ordering::Relaxed);
                    if tile_start >= order_count {
                        break;
                    }
                    let tile_end = (tile_start + SPIN1_ORDER_MAJOR_TILE).min(order_count);
                    #[cfg(all(test, feature = "parallel"))]
                    let tile_started = recurrence
                        .spin1_profile_enabled
                        .then(std::time::Instant::now);
                    for m in tile_start..tile_end {
                        let q_row = &mut output_scratch.q[..ntheta];
                        let u_row = &mut output_scratch.u[..ntheta];
                        q_row.fill(Complex::new(T::zero(), T::zero()));
                        u_row.fill(Complex::new(T::zero(), T::zero()));
                        self.synthesize_spin_ring_group_orders(
                            e_alm,
                            Some(b_alm),
                            1,
                            false,
                            true,
                            false,
                            spin1_scalar_coefficients,
                            spin1_scalar_coefficient_finite_orders,
                            nphi,
                            1,
                            phi0,
                            0,
                            ntheta,
                            m,
                            m + 1,
                            q_row,
                            u_row,
                            theta_sin,
                            theta_cos,
                            ring_order_limits,
                            arch,
                            recurrence,
                        )?;
                        // SAFETY: each atomic tile is claimed once and contains
                        // distinct orders; this writer owns one order column
                        // until the worker loop advances to the next order.
                        let q_writer = unsafe { q_output.writer_for_unique_order(m) }
                            .expect("the queued order is within the spectrum");
                        // SAFETY: the U tile has the same unique order ownership.
                        let u_writer = unsafe { u_output.writer_for_unique_order(m) }
                            .expect("the queued order is within the spectrum");
                        for ring in 0..ntheta {
                            q_writer.write(ring, q_row[ring]);
                            u_writer.write(ring, u_row[ring]);
                        }
                    }
                    #[cfg(all(test, feature = "parallel"))]
                    if let Some(tile_started) = tile_started {
                        recurrence.spin1_profile_group_ns += tile_started.elapsed().as_nanos();
                    }
                }
                Ok::<(), ShtError>(())
            })?;
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Each ring group receives plan inputs, optional prepared coefficients and worker scratch."
    )]
    pub(super) fn synthesize_spin_ring_group(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        spin: usize,
        derivative_norm: bool,
        allow_fast_scalar: bool,
        allow_spin1_table: bool,
        spin1_scalar_coefficients: Option<&[Complex<T>]>,
        spin1_scalar_coefficient_finite_orders: Option<&[u8]>,
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_start: usize,
        ring_end: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.synthesize_spin_ring_group_orders(
            e_alm,
            b_alm,
            spin,
            derivative_norm,
            allow_fast_scalar,
            allow_spin1_table,
            spin1_scalar_coefficients,
            spin1_scalar_coefficient_finite_orders,
            nphi,
            spectrum_len,
            phi0,
            ring_start,
            ring_end,
            0,
            self.harmonics.mmax() + 1,
            q_spectra,
            u_spectra,
            theta_sin,
            theta_cos,
            ring_order_limits,
            arch,
            recurrence,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The order-major queue restricts each ring traversal to one order range."
    )]
    pub(super) fn synthesize_spin_ring_group_orders(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        spin: usize,
        derivative_norm: bool,
        allow_fast_scalar: bool,
        allow_spin1_table: bool,
        spin1_scalar_coefficients: Option<&[Complex<T>]>,
        spin1_scalar_coefficient_finite_orders: Option<&[u8]>,
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_start: usize,
        ring_end: usize,
        order_start: usize,
        order_end: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        #[cfg(all(test, feature = "parallel"))]
        recurrence.record_active_worker();
        debug_assert_eq!(ring_end - ring_start, theta_sin.len());
        debug_assert_eq!(theta_cos.len(), theta_sin.len());
        debug_assert_eq!(q_spectra.len(), theta_sin.len() * spectrum_len);
        debug_assert_eq!(u_spectra.len(), q_spectra.len());
        let fast_scalar_coefficient_count = match spin {
            1 => self.harmonics.lmax() + 2,
            2 => self.harmonics.lmax() + 5,
            _ => 0,
        };
        let fast_scalar_min_lmax = match spin {
            1 => SPIN1_FAST_MIN_LMAX,
            2 => SPIN2_FAST_MIN_LMAX,
            _ => usize::MAX,
        };
        let fast_scalar_enabled = allow_fast_scalar
            && matches!(spin, 1 | 2)
            && (!derivative_norm || (spin == 1 && self.harmonics.spin() == 0))
            && (spin == 1 || b_alm.is_some())
            && self.harmonics.lmax() >= fast_scalar_min_lmax
            && ((spin == 1 && T::supports_spin1_synthesis())
                || (spin == 2 && T::supports_paired_spin()))
            && recurrence.fast_scalar_first.len() >= fast_scalar_coefficient_count
            && (spin != 2 || self.spin2_fast_f1.len() > self.harmonics.lmax());
        let reflected_q1_order_major = spin == 1
            && fast_scalar_enabled
            && !derivative_norm
            && !allow_spin1_table
            && b_alm.is_some()
            && self.harmonics.spin() == 1
            && self.grid.kind() == crate::grid::GridKind::GaussLegendre
            && self.grid.ntheta() == self.harmonics.lmax() + 1
            && spectrum_len == self.fft.spectrum_len()
            && ring_start == 0
            && ring_end == self.grid.ntheta()
            && T::supports_spin1_synthesis()
            && order_start == 0
            && order_end == self.harmonics.mmax() + 1
            && q_spectra.len() == theta_sin.len() * spectrum_len
            && u_spectra.len() == q_spectra.len()
            && single_threaded_execution();
        if reflected_q1_order_major {
            return self.synthesize_spin_ring_group_mode_orders(
                e_alm,
                b_alm,
                spin,
                derivative_norm,
                fast_scalar_enabled,
                allow_spin1_table,
                spin1_scalar_coefficients,
                spin1_scalar_coefficient_finite_orders,
                nphi,
                spectrum_len,
                phi0,
                ring_start,
                ring_end,
                order_start,
                order_end,
                q_spectra,
                u_spectra,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                recurrence,
            );
        }
        let fast_sine_min = match spin {
            1 => SPIN1_FAST_MIN_SIN_THETA,
            2 => SPIN2_FAST_MIN_SIN_THETA,
            _ => 1.0,
        };
        let fast_sine_min =
            T::from_f64(fast_sine_min).expect("spin fast sine limit is representable");
        let mut segment_start = 0;
        while segment_start < theta_sin.len() {
            let segment_fast = fast_scalar_enabled && theta_sin[segment_start] >= fast_sine_min;
            let mut segment_end = segment_start + 1;
            while segment_end < theta_sin.len()
                && (fast_scalar_enabled && theta_sin[segment_end] >= fast_sine_min) == segment_fast
            {
                segment_end += 1;
            }
            let spectrum_start = segment_start * spectrum_len;
            let spectrum_end = segment_end * spectrum_len;
            self.synthesize_spin_ring_group_mode_orders(
                e_alm,
                b_alm,
                spin,
                derivative_norm,
                segment_fast,
                allow_spin1_table,
                spin1_scalar_coefficients,
                spin1_scalar_coefficient_finite_orders,
                nphi,
                spectrum_len,
                phi0,
                ring_start + segment_start,
                ring_start + segment_end,
                order_start,
                order_end,
                &mut q_spectra[spectrum_start..spectrum_end],
                &mut u_spectra[spectrum_start..spectrum_end],
                &theta_sin[segment_start..segment_end],
                &theta_cos[segment_start..segment_end],
                ring_order_limits,
                arch,
                recurrence,
            )?;
            segment_start = segment_end;
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Each latitude segment receives transform inputs, optional coefficients and worker scratch."
    )]
    #[cfg(test)]
    pub(super) fn synthesize_spin_ring_group_mode(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        spin: usize,
        derivative_norm: bool,
        fast_scalar: bool,
        allow_spin1_table: bool,
        spin1_scalar_coefficients: Option<&[Complex<T>]>,
        spin1_scalar_coefficient_finite_orders: Option<&[u8]>,
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_start: usize,
        ring_end: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.synthesize_spin_ring_group_mode_orders(
            e_alm,
            b_alm,
            spin,
            derivative_norm,
            fast_scalar,
            allow_spin1_table,
            spin1_scalar_coefficients,
            spin1_scalar_coefficient_finite_orders,
            nphi,
            spectrum_len,
            phi0,
            ring_start,
            ring_end,
            0,
            self.harmonics.mmax() + 1,
            q_spectra,
            u_spectra,
            theta_sin,
            theta_cos,
            ring_order_limits,
            arch,
            recurrence,
        )
    }

    pub(super) fn spin1_to_scalar_order_with_prepared_pairs(
        &self,
        m: usize,
        lmax: usize,
        e_coefficients: &[Complex<T>],
        b_coefficients: &[Complex<T>],
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> (bool, bool, bool) {
        let scalar_lmax = lmax + 1;
        let factor_count = (scalar_lmax - m + 2) / 2;
        T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
        let factors_ready = T::fill_paired_scalar_factors(
            m,
            scalar_lmax,
            &recurrence.epsilon,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        );
        let pair_scratch_ready = factor_count <= recurrence.fast_spin_first.len()
            && factor_count <= recurrence.fast_spin_second.len()
            && factor_count <= recurrence.fast_spin_pair_first.len()
            && factor_count <= recurrence.fast_spin_pair_second.len();
        let component_limit = crate::legendre::ScalarLegendreTable::<T>::input_scale_limit()
            / T::from_f64(4.0).expect("four is representable");
        let min_normal = T::from_f64(f64::MIN_POSITIVE).expect("f64 minimum normal converts");
        let value_is_prepared_safe = |value: T| {
            crate::legendre::ScalarLegendreTable::<T>::input_value_is_safe(value, component_limit)
                && (value == T::zero() || value.abs() >= min_normal)
        };

        let mut converted_finite = true;
        let mut prepared_inputs_safe = true;
        let mut prepared_outputs_finite = true;
        let mut next_pair = 0usize;
        let mut first_window = [Complex::new(T::zero(), T::zero()); 3];
        let mut second_window = [Complex::new(T::zero(), T::zero()); 3];
        spin1_to_scalar_with_visit(
            m,
            lmax,
            e_coefficients,
            Some(b_coefficients),
            &mut recurrence.fast_scalar_first[..lmax + 2],
            &mut recurrence.fast_scalar_second[..lmax + 2],
            |degree, first, second| {
                converted_finite &= first.re.is_finite()
                    && first.im.is_finite()
                    && second.re.is_finite()
                    && second.im.is_finite();
                prepared_inputs_safe &= value_is_prepared_safe(first.re)
                    && value_is_prepared_safe(first.im)
                    && value_is_prepared_safe(second.re)
                    && value_is_prepared_safe(second.im);

                let offset = degree - m;
                let slot = offset % 3;
                first_window[slot] = first;
                second_window[slot] = second;

                if next_pair < factor_count {
                    let pair_degree = m + 2 * next_pair;
                    if degree == (pair_degree + 2).min(scalar_lmax) {
                        if factors_ready
                            && pair_scratch_ready
                            && prepared_inputs_safe
                            && prepared_outputs_finite
                        {
                            let pair_offset = pair_degree - m;
                            let prepared =
                                crate::legendre::prepare_synthesis_coefficient_pair_values(
                                    pair_degree,
                                    scalar_lmax,
                                    &recurrence.epsilon,
                                    recurrence.spin_alpha[next_pair],
                                    [
                                        first_window[pair_offset % 3],
                                        first_window[(pair_offset + 1) % 3],
                                        first_window[(pair_offset + 2) % 3],
                                    ],
                                    [
                                        second_window[pair_offset % 3],
                                        second_window[(pair_offset + 1) % 3],
                                        second_window[(pair_offset + 2) % 3],
                                    ],
                                );
                            let values = [
                                prepared.first_even,
                                prepared.first_odd,
                                prepared.second_even,
                                prepared.second_odd,
                            ];
                            prepared_outputs_finite = values
                                .iter()
                                .all(|value| value.re.is_finite() && value.im.is_finite());
                            if prepared_outputs_finite {
                                recurrence.fast_spin_first[next_pair] = prepared.first_even;
                                recurrence.fast_spin_second[next_pair] = prepared.first_odd;
                                recurrence.fast_spin_pair_first[next_pair] = prepared.second_even;
                                recurrence.fast_spin_pair_second[next_pair] = prepared.second_odd;
                            }
                        }
                        next_pair += 1;
                    }
                }
            },
        );

        let pair_coefficients_ready = factors_ready
            && pair_scratch_ready
            && prepared_inputs_safe
            && prepared_outputs_finite
            && next_pair == factor_count;
        (converted_finite, factors_ready, pair_coefficients_ready)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The order-major queue restricts each latitude segment to an order range."
    )]
    pub(super) fn synthesize_spin_ring_group_mode_orders(
        &self,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        spin: usize,
        derivative_norm: bool,
        fast_scalar: bool,
        allow_spin1_table: bool,
        spin1_scalar_coefficients: Option<&[Complex<T>]>,
        spin1_scalar_coefficient_finite_orders: Option<&[u8]>,
        nphi: usize,
        spectrum_len: usize,
        phi0: f64,
        ring_start: usize,
        ring_end: usize,
        order_start: usize,
        order_end: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let fused_local_preparation_eligible = lmax
            .checked_add(2)
            .and_then(|row_len| row_len.checked_mul(2))
            .and_then(|row_stride| {
                self.harmonics
                    .mmax()
                    .checked_add(1)
                    .and_then(|order_count| row_stride.checked_mul(order_count))
            })
            .and_then(|coefficient_count| {
                coefficient_count.checked_mul(std::mem::size_of::<Complex<T>>())
            })
            .is_some_and(|bytes| bytes < SPIN1_ORDER_FUSED_LOCAL_PREPARATION_MAX_BYTES);
        debug_assert_eq!(ring_end - ring_start, theta_sin.len());
        debug_assert_eq!(q_spectra.len(), (ring_end - ring_start) * spectrum_len);
        debug_assert_eq!(u_spectra.len(), q_spectra.len());

        let nyquist_scale = T::from_f64(2.0).expect("two is representable");
        let maximum_ring_order = (spin == 1).then(|| {
            ring_order_limits[ring_start..ring_end]
                .iter()
                .copied()
                .max()
                .unwrap_or(0)
        });
        let exact_cc_spin1_pole = spin == 1
            && self.harmonics.spin() == 1
            && !derivative_norm
            && T::supports_paired_spin()
            && self.grid.kind() == crate::grid::GridKind::ClenshawCurtis
            && theta_sin.len() == 1
            && theta_sin[0] == T::zero()
            && theta_cos[0].abs() == T::one();
        for m in order_start..order_end {
            let output_order = m - order_start;
            // Synthesis cleared these spectra, and the polar cutoff excludes this
            // order from every ring in the segment.
            if maximum_ring_order.is_some_and(|limit| m > limit) {
                continue;
            }
            // At an exact pole, a spin-1 harmonic has support only at |m|=1.
            // The general recurrence clamps half-angle powers away from zero,
            // which otherwise makes 0*m artifacts for finite coefficients.
            // Keep exceptional rows on the checked recurrence path.
            if exact_cc_spin1_pole && m != 1 {
                let coefficient_start = self
                    .harmonics
                    .index(m, m)
                    .expect("every packed order starts at its diagonal degree");
                let coefficient_count = lmax - m + 1;
                let coefficient_limit =
                    T::from_f64(16.0).expect("pole fast-path coefficient limit converts");
                let min_normal =
                    T::from_f64(f64::MIN_POSITIVE).expect("f64 minimum normal converts");
                let row_is_safe = e_alm[coefficient_start..coefficient_start + coefficient_count]
                    .iter()
                    .chain(b_alm.into_iter().flat_map(|values| {
                        values[coefficient_start..coefficient_start + coefficient_count].iter()
                    }))
                    .flat_map(|coefficient| [coefficient.re, coefficient.im])
                    .all(|value| {
                        value.is_finite()
                            && value.abs() <= coefficient_limit
                            && (value == T::zero() || value.abs() >= min_normal)
                    });
                if row_is_safe {
                    continue;
                }
            }
            let phase = longitude_phase::<T>(m, phi0, false);
            let mut fast_scalar_order = fast_scalar;
            let mut local_pair_coefficients_prepared = false;
            let mut local_paired_factors_ready = false;
            if fast_scalar_order && spin == 1 {
                let coefficient_start = self
                    .harmonics
                    .index(m, m)
                    .expect("every packed order starts at its diagonal degree");
                let coefficient_count = lmax - m + 1;
                let use_fused_local_preparation = !derivative_norm
                    && b_alm.is_some()
                    && spin1_scalar_coefficients.is_none()
                    && spin1_scalar_coefficient_finite_orders.is_none()
                    && !allow_spin1_table
                    && T::supports_spin1_synthesis()
                    && fused_local_preparation_eligible
                    && ring_start + ring_end == self.grid.ntheta()
                    && order_start == m
                    && order_end == m + 1
                    && output_order == 0
                    && spectrum_len == 1
                    && q_spectra.len() == theta_sin.len()
                    && u_spectra.len() == theta_sin.len();
                if use_fused_local_preparation {
                    let b_coefficients = &b_alm.expect("both spin-1 channels are available")
                        [coefficient_start..coefficient_start + coefficient_count];
                    let (finite, factors_ready, pair_coefficients_ready) = self
                        .spin1_to_scalar_order_with_prepared_pairs(
                            m,
                            lmax,
                            &e_alm[coefficient_start..coefficient_start + coefficient_count],
                            b_coefficients,
                            recurrence,
                        );
                    fast_scalar_order = finite;
                    local_paired_factors_ready = factors_ready;
                    local_pair_coefficients_prepared = pair_coefficients_ready;
                } else {
                    let coefficient_slice = if derivative_norm {
                        let scaled = &mut recurrence.fast_spin_first[..coefficient_count];
                        for (offset, coefficient) in scaled.iter_mut().enumerate() {
                            let degree = m + offset;
                            let degree = T::from_usize(degree).expect("degree converts");
                            let scale = if degree == T::zero() {
                                T::zero()
                            } else {
                                (degree * (degree + T::one())).sqrt()
                            };
                            *coefficient = e_alm[coefficient_start + offset] * scale;
                            if !coefficient.re.is_finite() || !coefficient.im.is_finite() {
                                fast_scalar_order = false;
                                break;
                            }
                        }
                        fast_scalar_order.then_some(&scaled[..])
                    } else {
                        Some(&e_alm[coefficient_start..coefficient_start + coefficient_count])
                    };
                    if let Some(finite_orders) = spin1_scalar_coefficient_finite_orders {
                        debug_assert!(spin1_scalar_coefficients.is_some());
                        debug_assert!(m < finite_orders.len());
                        fast_scalar_order = finite_orders[m] != 0;
                    } else if let Some(e_coefficients) = coefficient_slice {
                        let (scalar_first, scalar_second): (&[Complex<T>], &[Complex<T>]) =
                            if let Some(coefficients) = spin1_scalar_coefficients {
                                let row_len = lmax + 2;
                                let row_stride = row_len * 2;
                                let row_start = m * row_stride;
                                (
                                    &coefficients[row_start..row_start + row_len],
                                    &coefficients[row_start + row_len..row_start + row_stride],
                                )
                            } else {
                                spin1_to_scalar(
                                    m,
                                    lmax,
                                    e_coefficients,
                                    b_alm.map(|values| {
                                        &values[coefficient_start
                                            ..coefficient_start + coefficient_count]
                                    }),
                                    &mut recurrence.fast_scalar_first[..lmax + 2],
                                    &mut recurrence.fast_scalar_second[..lmax + 2],
                                );
                                (
                                    &recurrence.fast_scalar_first[..lmax + 2],
                                    &recurrence.fast_scalar_second[..lmax + 2],
                                )
                            };
                        // Conversion may overflow even when the checked spin
                        // recurrence can still represent the sampled field.
                        #[cfg(all(test, feature = "parallel"))]
                        let safety_started = recurrence
                            .spin1_profile_enabled
                            .then(std::time::Instant::now);
                        fast_scalar_order = scalar_first
                            .iter()
                            .chain(scalar_second)
                            .all(|value| value.re.is_finite() && value.im.is_finite());
                        #[cfg(all(test, feature = "parallel"))]
                        if let Some(safety_started) = safety_started {
                            recurrence.spin1_profile_coefficient_safety_ns +=
                                safety_started.elapsed().as_nanos();
                            recurrence.spin1_profile_coefficient_safety_values +=
                                (scalar_first.len() + scalar_second.len()) as u128;
                        }
                    }
                }
            }
            if fast_scalar_order {
                if spin == 2 {
                    self.synthesize_spin2_scalar_order_to_order(
                        m,
                        e_alm,
                        b_alm.expect("the scalar conversion requires E and B"),
                        spectrum_len,
                        ring_start,
                        output_order,
                        q_spectra,
                        u_spectra,
                        theta_sin,
                        theta_cos,
                        ring_order_limits,
                        arch,
                        recurrence,
                    )?;
                } else {
                    let scalar_lmax = lmax + 1;
                    #[cfg(all(test, feature = "parallel"))]
                    let factor_started = recurrence
                        .spin1_profile_enabled
                        .then(std::time::Instant::now);
                    let paired_factors_ready = if local_paired_factors_ready {
                        true
                    } else {
                        T::fill_epsilon(m, scalar_lmax, &mut recurrence.epsilon);
                        T::fill_paired_scalar_factors(
                            m,
                            scalar_lmax,
                            &recurrence.epsilon,
                            &mut recurrence.spin_alpha,
                            &mut recurrence.spin_coefficient_a,
                            &mut recurrence.spin_coefficient_b,
                        )
                    };
                    debug_assert!(paired_factors_ready);
                    #[cfg(all(test, feature = "parallel"))]
                    if let Some(factor_started) = factor_started {
                        recurrence.spin1_profile_factor_ns += factor_started.elapsed().as_nanos();
                    }
                    let pair_min_sine = T::from_f64(SPIN1_PAIR_MIN_SIN_THETA)
                        .expect("spin-1 paired scalar sine limit converts");
                    // Checked pair coefficients depend on the order, not the latitude block.
                    let can_reuse_pair_coefficients = spin == 1
                        && !derivative_norm
                        && b_alm.is_some()
                        && !allow_spin1_table
                        && T::supports_spin1_synthesis();
                    let coefficient_pair_count = (scalar_lmax - m + 2) / 2;
                    let pair_scratch_ready = coefficient_pair_count
                        <= recurrence.fast_spin_first.len()
                        && coefficient_pair_count <= recurrence.fast_spin_second.len()
                        && coefficient_pair_count <= recurrence.fast_spin_pair_first.len()
                        && coefficient_pair_count <= recurrence.fast_spin_pair_second.len();
                    let can_prepare_shared_pair = can_reuse_pair_coefficients && pair_scratch_ready;
                    let pair_safe_block = |start: usize, end: usize| {
                        theta_sin[start..end]
                            .iter()
                            .all(|&sine| sine >= pair_min_sine)
                            && ring_order_limits[ring_start + start..ring_start + end]
                                .iter()
                                .all(|&limit| limit >= m)
                    };
                    let q1_order_major_component = spin == 1
                        && self.harmonics.spin() == 1
                        && self.grid.kind() == crate::grid::GridKind::GaussLegendre
                        && order_start == 0
                        && order_end == self.harmonics.mmax() + 1
                        && T::supports_spin1_synthesis()
                        && output_order == m
                        && spectrum_len == self.fft.spectrum_len()
                        && ring_start == 0
                        && ring_end == self.grid.ntheta()
                        && q_spectra.len() == theta_sin.len() * spectrum_len
                        && single_threaded_execution();
                    let contiguous_component_order = order_start == m
                        && order_end == m + 1
                        && output_order == 0
                        && spectrum_len == 1
                        && q_spectra.len() == theta_sin.len();
                    // Reduced CC/F1 q8 cross-thread checks exceed the existing
                    // tolerance with the reflected component route. Keep those
                    // orders on the established paired-latitude recurrence.
                    let prepared_component_attempted = can_prepare_shared_pair
                        && paired_factors_ready
                        && !allow_spin1_table
                        && ring_start + ring_end == self.grid.ntheta()
                        && (q1_order_major_component
                            || (contiguous_component_order
                                && (self.harmonics.mmax() == lmax
                                    || self.grid.kind() == crate::grid::GridKind::GaussLegendre)))
                        && u_spectra.len() == q_spectra.len();
                    let prepared_component_safe_start = if prepared_component_attempted {
                        if let Some(coefficients) = spin1_scalar_coefficients {
                            let row_len = lmax + 2;
                            let row_stride = row_len * 2;
                            let row_start = m * row_stride;
                            let scalar_first = &coefficients[row_start..row_start + row_len];
                            let scalar_second =
                                &coefficients[row_start + row_len..row_start + row_stride];
                            self.try_synthesize_spin1_prepared_component_order_to_strided_spectra(
                                m,
                                scalar_lmax,
                                &scalar_first[m..=scalar_lmax],
                                &scalar_second[m..=scalar_lmax],
                                false,
                                theta_sin,
                                theta_cos,
                                &ring_order_limits[ring_start..ring_end],
                                spectrum_len,
                                output_order,
                                arch,
                                recurrence,
                                q_spectra,
                                u_spectra,
                            )
                        } else if local_pair_coefficients_prepared {
                            self.try_synthesize_spin1_prepared_component_order_to_strided_spectra(
                                m,
                                scalar_lmax,
                                &[],
                                &[],
                                true,
                                theta_sin,
                                theta_cos,
                                &ring_order_limits[ring_start..ring_end],
                                spectrum_len,
                                output_order,
                                arch,
                                recurrence,
                                q_spectra,
                                u_spectra,
                            )
                        } else {
                            self.try_synthesize_spin1_prepared_component_order_from_workspace(
                                m,
                                scalar_lmax,
                                theta_sin,
                                theta_cos,
                                &ring_order_limits[ring_start..ring_end],
                                spectrum_len,
                                output_order,
                                arch,
                                recurrence,
                                q_spectra,
                                u_spectra,
                            )
                        }
                    } else {
                        None
                    };
                    let component_prepared = prepared_component_safe_start.is_some();
                    let fallback_ranges = if let Some(safe_start) = prepared_component_safe_start {
                        #[cfg(all(test, feature = "parallel"))]
                        SPIN1_PREPARED_COMPONENT_ORDER_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                        let segment_end = theta_sin.len();
                        [(0, safe_start), (segment_end - safe_start, segment_end)]
                    } else {
                        // A declined component pass may have staged one channel
                        // or earlier ring batches. Discard the whole local order
                        // before entering the established checked fallback.
                        if prepared_component_attempted {
                            for ring in 0..theta_sin.len() {
                                let spectrum_index = ring * spectrum_len + output_order;
                                q_spectra[spectrum_index] = Complex::new(T::zero(), T::zero());
                                u_spectra[spectrum_index] = Complex::new(T::zero(), T::zero());
                            }
                        }
                        [(0, theta_sin.len()), (0, 0)]
                    };
                    // In the full-order, single-worker GL route, each order is
                    // active on one contiguous latitude interval. Trim wholly
                    // excluded polar rings before forming SIMD blocks so a
                    // mixed active/inactive block cannot force the checked
                    // per-degree path for its otherwise ordinary lanes.
                    let trim_excluded_gl_rings = !component_prepared
                        && spin == 1
                        && self.harmonics.lmax() >= SPIN1_TRANSITION_MIN_LMAX
                        && self.harmonics.mmax() == self.harmonics.lmax()
                        && self.grid.kind() == crate::grid::GridKind::GaussLegendre
                        && self.grid.ntheta() == self.harmonics.lmax() + 1
                        && single_threaded_execution()
                        && ring_start == 0
                        && ring_end == self.grid.ntheta();
                    for (range_start, range_end) in fallback_ranges {
                        let (active_start, active_end) =
                            if trim_excluded_gl_rings && range_start < range_end {
                                let limits = &ring_order_limits
                                    [ring_start + range_start..ring_start + range_end];
                                let first_active = limits.iter().position(|&limit| m <= limit);
                                let last_active = limits.iter().rposition(|&limit| m <= limit);
                                match (first_active, last_active) {
                                    (Some(first), Some(last))
                                        if limits[first..=last].iter().all(|&limit| m <= limit) =>
                                    {
                                        (range_start + first, range_start + last + 1)
                                    }
                                    _ => (range_start, range_end),
                                }
                            } else {
                                (range_start, range_end)
                            };
                        let mut block_start = active_start;
                        while block_start < active_end {
                            let block_end = (block_start + crate::legendre::LATITUDE_BLOCK_SIZE)
                                .min(active_end);
                            let mut run_end = block_end;
                            if can_prepare_shared_pair && pair_safe_block(block_start, block_end) {
                                while run_end < active_end {
                                    let next_end = (run_end + crate::legendre::LATITUDE_BLOCK_SIZE)
                                        .min(active_end);
                                    if !pair_safe_block(run_end, next_end) {
                                        break;
                                    }
                                    run_end = next_end;
                                }
                            }
                            let pair_coefficients_prepared = if component_prepared {
                                Some(true)
                            } else if run_end > block_end {
                                let cached_scalar_row =
                                    spin1_scalar_coefficients.map(|coefficients| {
                                        let row_len = lmax + 2;
                                        let row_stride = row_len * 2;
                                        let row_start = m * row_stride;
                                        (
                                            &coefficients[row_start..row_start + row_len],
                                            &coefficients
                                                [row_start + row_len..row_start + row_stride],
                                        )
                                    });
                                let (scalar_first, scalar_second) = cached_scalar_row.unwrap_or((
                                    &recurrence.fast_scalar_first[..scalar_lmax + 1],
                                    &recurrence.fast_scalar_second[..scalar_lmax + 1],
                                ));
                                #[cfg(all(test, feature = "parallel"))]
                                let prepare_started = recurrence
                                    .spin1_profile_enabled
                                    .then(std::time::Instant::now);
                                let prepared = T::prepare_paired_scalar_synthesis_coefficient_pair(
                                    m,
                                    scalar_lmax,
                                    &recurrence.epsilon,
                                    &recurrence.spin_alpha,
                                    &scalar_first[m..=scalar_lmax],
                                    &scalar_second[m..=scalar_lmax],
                                    &mut recurrence.fast_spin_first[..coefficient_pair_count],
                                    &mut recurrence.fast_spin_second[..coefficient_pair_count],
                                    &mut recurrence.fast_spin_pair_first[..coefficient_pair_count],
                                    &mut recurrence.fast_spin_pair_second[..coefficient_pair_count],
                                );
                                #[cfg(all(test, feature = "parallel"))]
                                if let Some(prepare_started) = prepare_started {
                                    recurrence.spin1_profile_pair_prepare_ns +=
                                        prepare_started.elapsed().as_nanos();
                                }
                                Some(prepared)
                            } else {
                                None
                            };
                            for block_start in
                                (block_start..run_end).step_by(crate::legendre::LATITUDE_BLOCK_SIZE)
                            {
                                let block_end = (block_start
                                    + crate::legendre::LATITUDE_BLOCK_SIZE)
                                    .min(run_end);
                                let spectrum_start = block_start * spectrum_len;
                                let spectrum_end = block_end * spectrum_len;
                                self.synthesize_spin1_scalar_block_to_order(
                                    m,
                                    scalar_lmax,
                                    spin1_scalar_coefficients,
                                    spectrum_len,
                                    ring_start + block_start,
                                    output_order,
                                    allow_spin1_table,
                                    &theta_sin[block_start..block_end],
                                    &theta_cos[block_start..block_end],
                                    &ring_order_limits
                                        [ring_start + block_start..ring_start + block_end],
                                    arch,
                                    recurrence,
                                    &mut q_spectra[spectrum_start..spectrum_end],
                                    &mut u_spectra[spectrum_start..spectrum_end],
                                    pair_coefficients_prepared,
                                )?;
                            }
                            block_start = run_end;
                        }
                    }
                }
            } else {
                self.synthesize_general_spin_order_to_order(
                    m,
                    e_alm,
                    b_alm,
                    spin,
                    derivative_norm,
                    spectrum_len,
                    ring_start,
                    output_order,
                    q_spectra,
                    u_spectra,
                    theta_sin,
                    theta_cos,
                    ring_order_limits,
                    arch,
                    recurrence,
                )?;
            }
            for group_ring in 0..ring_end - ring_start {
                let spectrum_index = group_ring * spectrum_len + output_order;
                let phased_q = q_spectra[spectrum_index] * phase;
                let phased_u = u_spectra[spectrum_index] * phase;
                if nphi.is_multiple_of(2) && m == nphi / 2 {
                    q_spectra[spectrum_index] =
                        Complex::new(nyquist_scale * phased_q.re, T::zero());
                    u_spectra[spectrum_index] =
                        Complex::new(nyquist_scale * phased_u.re, T::zero());
                } else {
                    q_spectra[spectrum_index] = phased_q;
                    u_spectra[spectrum_index] = phased_u;
                }
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Original-coefficient fallback uses one order and caller-owned scratch."
    )]
    #[cfg(feature = "parallel")]
    pub(super) fn synthesize_general_spin_order(
        &self,
        m: usize,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        spin: usize,
        derivative_norm: bool,
        spectrum_len: usize,
        ring_start: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.synthesize_general_spin_order_to_order(
            m,
            e_alm,
            b_alm,
            spin,
            derivative_norm,
            spectrum_len,
            ring_start,
            m,
            q_spectra,
            u_spectra,
            theta_sin,
            theta_cos,
            ring_order_limits,
            arch,
            recurrence,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The order-major path places one order into a compact worker-local output row."
    )]
    pub(super) fn synthesize_general_spin_order_to_order(
        &self,
        m: usize,
        e_alm: &[Complex<T>],
        b_alm: Option<&[Complex<T>]>,
        spin: usize,
        derivative_norm: bool,
        spectrum_len: usize,
        ring_start: usize,
        output_order: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let initial = T::fill_spin_factors(
            spin,
            m,
            lmax,
            &mut recurrence.spin_alpha,
            &mut recurrence.spin_coefficient_a,
            &mut recurrence.spin_coefficient_b,
        )?;
        T::for_each_spin_lm_dispatched(
            arch,
            &initial,
            lmax,
            theta_sin,
            theta_cos,
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
            |degree, group_ring_start, plus, plus_exp, minus, minus_exp| {
                let coefficient_index = self
                    .harmonics
                    .index(degree, m)
                    .expect("the recurrence visits degrees in the packed order");
                let e_coefficient = e_alm[coefficient_index];
                let b_coefficient = b_alm
                    .map(|values| values[coefficient_index])
                    .unwrap_or_else(|| Complex::new(T::zero(), T::zero()));
                let alpha_norm = recurrence.spin_alpha[degree]
                    * T::spin_normalization(spin, degree, derivative_norm);
                for lane in 0..plus.len() {
                    let group_ring = group_ring_start + lane;
                    let global_ring = ring_start + group_ring;
                    if m > ring_order_limits[global_ring] {
                        continue;
                    }
                    let plus_value = T::scaled_basis_value(plus[lane], plus_exp[lane], alpha_norm);
                    let minus_value =
                        T::scaled_basis_value(minus[lane], minus_exp[lane], alpha_norm);
                    let even_basis = plus_value + minus_value;
                    let odd_basis = plus_value - minus_value;
                    let q_value = e_coefficient * even_basis
                        + Complex::<T>::new(T::zero(), -T::one()) * b_coefficient * odd_basis;
                    let u_value =
                        Complex::<T>::new(T::zero(), T::one()) * e_coefficient * odd_basis
                            + b_coefficient * even_basis;
                    let spectrum_index = group_ring * spectrum_len + output_order;
                    q_spectra[spectrum_index] += q_value;
                    u_spectra[spectrum_index] += u_value;
                }
                Ok(())
            },
        )?;
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Spin-1 scalar synthesis uses the plan's recurrence factors and worker scratch."
    )]
    #[cfg(any(feature = "parallel", test))]
    pub(super) fn synthesize_spin1_scalar_block(
        &self,
        m: usize,
        scalar_lmax: usize,
        cached_scalar_coefficients: Option<&[Complex<T>]>,
        spectrum_len: usize,
        global_ring_start: usize,
        allow_table: bool,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
    ) -> Result<(), ShtError> {
        self.synthesize_spin1_scalar_block_to_order(
            m,
            scalar_lmax,
            cached_scalar_coefficients,
            spectrum_len,
            global_ring_start,
            m,
            allow_table,
            theta_sin,
            theta_cos,
            ring_order_limits,
            arch,
            recurrence,
            q_spectra,
            u_spectra,
            None,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Order-major scalar synthesis writes one packed order into worker-local spectra."
    )]
    pub(super) fn synthesize_spin1_scalar_block_to_order(
        &self,
        m: usize,
        scalar_lmax: usize,
        cached_scalar_coefficients: Option<&[Complex<T>]>,
        spectrum_len: usize,
        global_ring_start: usize,
        output_order: usize,
        allow_table: bool,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        pair_coefficients_prepared: Option<bool>,
    ) -> Result<(), ShtError> {
        let block_len = theta_sin.len();
        debug_assert_eq!(theta_cos.len(), block_len);
        debug_assert_eq!(ring_order_limits.len(), block_len);
        debug_assert_eq!(q_spectra.len(), block_len * spectrum_len);
        debug_assert_eq!(u_spectra.len(), q_spectra.len());

        let cached_scalar_row = cached_scalar_coefficients.map(|coefficients| {
            let row_len = self.harmonics.lmax() + 2;
            let row_stride = row_len * 2;
            let row_start = m * row_stride;
            (
                &coefficients[row_start..row_start + row_len],
                &coefficients[row_start + row_len..row_start + row_stride],
            )
        });
        let has_excluded_ring = ring_order_limits.iter().any(|&limit| m > limit);
        if ring_order_limits.iter().all(|&limit| m > limit) {
            return Ok(());
        }

        #[cfg(all(test, feature = "parallel"))]
        let profile_bin = if m <= 255 {
            0
        } else if m <= 767 {
            1
        } else {
            2
        };
        let coefficient_pair_count = (scalar_lmax - m + 2) / 2;
        let mut prepared_pair_for_transition = false;
        let pair_min_sine = T::from_f64(SPIN1_PAIR_MIN_SIN_THETA)
            .expect("spin-1 paired scalar sine limit converts");
        if allow_table
            && self.harmonics.lmax() >= SPIN1_TABLE_SYNTH_MIN_LMAX
            && let Some(table) = self
                .spin1_scalar_legendre_table
                .get()
                .and_then(Option::as_ref)
            && theta_sin.iter().all(|&sine| sine >= pair_min_sine)
            && (m..=scalar_lmax).all(|degree| {
                let (scalar_first, scalar_second) = cached_scalar_row.unwrap_or((
                    &recurrence.fast_scalar_first[..scalar_lmax + 1],
                    &recurrence.fast_scalar_second[..scalar_lmax + 1],
                ));
                let first = scalar_first[degree];
                let second = scalar_second[degree];
                table.input_is_safe(first.re)
                    && table.input_is_safe(first.im)
                    && table.input_is_safe(second.re)
                    && table.input_is_safe(second.im)
            })
        {
            let ntheta = self.grid.ntheta();
            for degree in m..=scalar_lmax {
                let (scalar_first, scalar_second) = cached_scalar_row.unwrap_or((
                    &recurrence.fast_scalar_first[..scalar_lmax + 1],
                    &recurrence.fast_scalar_second[..scalar_lmax + 1],
                ));
                let basis_lanes = table.degree_lanes(m, degree - m);
                let first_coefficient = scalar_first[degree];
                let second_coefficient = scalar_second[degree];
                let reflected_odd = !(degree + m).is_multiple_of(2);
                for lane in 0..block_len {
                    if m > ring_order_limits[lane] {
                        continue;
                    }
                    let global_ring = global_ring_start + lane;
                    let mirrored_ring = ntheta - 1 - global_ring;
                    let north_ring = global_ring.min(mirrored_ring);
                    let mut basis = basis_lanes[north_ring];
                    if global_ring > mirrored_ring && reflected_odd {
                        basis = -basis;
                    }
                    let weighted_basis = basis / theta_sin[lane];
                    let spectrum_index = lane * spectrum_len + output_order;
                    q_spectra[spectrum_index] += first_coefficient * weighted_basis;
                    u_spectra[spectrum_index] += second_coefficient * weighted_basis;
                }
            }
            return Ok(());
        }

        if !has_excluded_ring {
            let paired_channels_eligible = T::supports_spin1_synthesis()
                && recurrence.fast_spin_first.len() >= (scalar_lmax - m + 2) / 2
                && recurrence.fast_spin_pair_first.len() >= (scalar_lmax - m + 2) / 2
                && theta_sin.iter().all(|&sine| sine >= pair_min_sine);
            #[cfg(all(test, feature = "parallel"))]
            if recurrence.spin1_profile_enabled && paired_channels_eligible {
                recurrence.spin1_profile_pair_eligible_blocks += 1;
                let profile = &mut recurrence.spin1_profile_route_bins[profile_bin];
                profile.eligible_blocks += 1;
                profile.eligible_lane_pair_work += (block_len * coefficient_pair_count) as u128;
            }
            let prepared_pair = if paired_channels_eligible {
                #[cfg(all(test, feature = "parallel"))]
                let prepare_started = recurrence
                    .spin1_profile_enabled
                    .then(std::time::Instant::now);
                let (scalar_first, scalar_second) = cached_scalar_row.unwrap_or((
                    &recurrence.fast_scalar_first[..scalar_lmax + 1],
                    &recurrence.fast_scalar_second[..scalar_lmax + 1],
                ));
                let prepared_pair = pair_coefficients_prepared.unwrap_or_else(|| {
                    T::prepare_paired_scalar_synthesis_coefficient_pair(
                        m,
                        scalar_lmax,
                        &recurrence.epsilon,
                        &recurrence.spin_alpha,
                        &scalar_first[m..=scalar_lmax],
                        &scalar_second[m..=scalar_lmax],
                        &mut recurrence.fast_spin_first[..coefficient_pair_count],
                        &mut recurrence.fast_spin_second[..coefficient_pair_count],
                        &mut recurrence.fast_spin_pair_first[..coefficient_pair_count],
                        &mut recurrence.fast_spin_pair_second[..coefficient_pair_count],
                    )
                });
                #[cfg(all(test, feature = "parallel"))]
                if let Some(prepare_started) = prepare_started {
                    recurrence.spin1_profile_pair_prepare_ns +=
                        prepare_started.elapsed().as_nanos();
                }
                prepared_pair
            } else {
                false
            };
            #[cfg(all(test, feature = "parallel"))]
            if recurrence.spin1_profile_enabled && paired_channels_eligible && !prepared_pair {
                recurrence.spin1_profile_route_bins[profile_bin].coefficient_prep_declines += 1;
            }
            prepared_pair_for_transition = prepared_pair;
            if prepared_pair {
                #[cfg(all(test, feature = "parallel"))]
                if recurrence.spin1_profile_enabled {
                    recurrence.spin1_profile_route_bins[profile_bin].regular_kernel_attempts += 1;
                }
                #[cfg(all(test, feature = "parallel"))]
                let recurrence_started = recurrence
                    .spin1_profile_enabled
                    .then(std::time::Instant::now);
                let fused_pair = T::synthesize_prepared_scalar_pair_block_dispatched(
                    arch,
                    m,
                    output_order,
                    scalar_lmax,
                    theta_sin,
                    theta_cos,
                    self.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &recurrence.fast_spin_first[..coefficient_pair_count],
                    &recurrence.fast_spin_second[..coefficient_pair_count],
                    &recurrence.fast_spin_pair_first[..coefficient_pair_count],
                    &recurrence.fast_spin_pair_second[..coefficient_pair_count],
                    ring_order_limits,
                    spectrum_len,
                    q_spectra,
                    u_spectra,
                    &mut recurrence.previous[..block_len],
                    &mut recurrence.current[..block_len],
                    &mut recurrence.next[..block_len],
                    &mut recurrence.exponents[..block_len],
                    &mut recurrence.analysis_same_real[..block_len],
                    &mut recurrence.analysis_same_imag[..block_len],
                    &mut recurrence.analysis_opposite_real[..block_len],
                    &mut recurrence.analysis_opposite_imag[..block_len],
                )?;
                #[cfg(all(test, feature = "parallel"))]
                if let Some(recurrence_started) = recurrence_started {
                    recurrence.spin1_profile_recurrence_ns +=
                        recurrence_started.elapsed().as_nanos();
                }
                if fused_pair {
                    #[cfg(all(test, feature = "parallel"))]
                    if recurrence.spin1_profile_enabled {
                        recurrence.spin1_profile_pair_success_blocks += 1;
                        let profile = &mut recurrence.spin1_profile_route_bins[profile_bin];
                        profile.ordinary_fused_successes += 1;
                        profile.ordinary_fused_lane_pair_work +=
                            (block_len * coefficient_pair_count) as u128;
                    }
                    for (ring, &sin_theta) in theta_sin.iter().enumerate() {
                        let spectrum_index = ring * spectrum_len + output_order;
                        let inverse_sine = sin_theta.recip();
                        q_spectra[spectrum_index] *= inverse_sine;
                        u_spectra[spectrum_index] *= inverse_sine;
                    }
                    return Ok(());
                }
            }

            // The separate-channel fallback is useful only when shared pair
            // scratch is unavailable or the latitude block is too close to a
            // pole for the dual-channel kernel. Prepare both channels before
            // either consumer writes spectra so a single unsafe channel cannot
            // cause a partial commit.
            let prepared_separate = if !paired_channels_eligible {
                let (scalar_first, scalar_second) = cached_scalar_row.unwrap_or((
                    &recurrence.fast_scalar_first[..scalar_lmax + 1],
                    &recurrence.fast_scalar_second[..scalar_lmax + 1],
                ));
                let scratch_ready = recurrence.fast_spin_first.len() >= coefficient_pair_count
                    && recurrence.fast_spin_second.len() >= coefficient_pair_count
                    && recurrence.fast_spin_pair_first.len() >= coefficient_pair_count
                    && recurrence.fast_spin_pair_second.len() >= coefficient_pair_count;
                scratch_ready
                    && T::prepare_paired_scalar_synthesis_coefficients(
                        m,
                        scalar_lmax,
                        &recurrence.epsilon,
                        &recurrence.spin_alpha,
                        &scalar_first[m..=scalar_lmax],
                        &mut recurrence.fast_spin_first[..coefficient_pair_count],
                        &mut recurrence.fast_spin_second[..coefficient_pair_count],
                    )
                    && T::prepare_paired_scalar_synthesis_coefficients(
                        m,
                        scalar_lmax,
                        &recurrence.epsilon,
                        &recurrence.spin_alpha,
                        &scalar_second[m..=scalar_lmax],
                        &mut recurrence.fast_spin_pair_first[..coefficient_pair_count],
                        &mut recurrence.fast_spin_pair_second[..coefficient_pair_count],
                    )
            } else {
                false
            };
            let fused_first = if prepared_separate {
                T::synthesize_paired_scalar_block_prepared_dispatched(
                    arch,
                    m,
                    output_order,
                    scalar_lmax,
                    theta_sin,
                    theta_cos,
                    self.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &recurrence.fast_spin_first[..coefficient_pair_count],
                    &recurrence.fast_spin_second[..coefficient_pair_count],
                    ring_order_limits,
                    spectrum_len,
                    q_spectra,
                    &mut recurrence.previous[..block_len],
                    &mut recurrence.current[..block_len],
                    &mut recurrence.next[..block_len],
                    &mut recurrence.exponents[..block_len],
                )?
            } else {
                false
            };
            let fused_second = if prepared_separate {
                T::synthesize_paired_scalar_block_prepared_dispatched(
                    arch,
                    m,
                    output_order,
                    scalar_lmax,
                    theta_sin,
                    theta_cos,
                    self.sectoral_norms[m],
                    &recurrence.spin_coefficient_a,
                    &recurrence.spin_coefficient_b,
                    &recurrence.fast_spin_pair_first[..coefficient_pair_count],
                    &recurrence.fast_spin_pair_second[..coefficient_pair_count],
                    ring_order_limits,
                    spectrum_len,
                    u_spectra,
                    &mut recurrence.previous[..block_len],
                    &mut recurrence.current[..block_len],
                    &mut recurrence.next[..block_len],
                    &mut recurrence.exponents[..block_len],
                )?
            } else {
                false
            };
            debug_assert_eq!(fused_first, fused_second);
            if fused_first && fused_second {
                for (ring, &sin_theta) in theta_sin.iter().enumerate() {
                    let spectrum_index = ring * spectrum_len + output_order;
                    let inverse_sine = sin_theta.recip();
                    q_spectra[spectrum_index] *= inverse_sine;
                    u_spectra[spectrum_index] *= inverse_sine;
                }
                return Ok(());
            }
        }

        // The common paired route reads the shared rows directly. The
        // scaled and per-degree fallbacks still consume worker-local rows, so
        // populate those only after the paired route declines.
        if let Some((scalar_first, scalar_second)) = cached_scalar_row {
            recurrence.fast_scalar_first[..scalar_lmax + 1].copy_from_slice(scalar_first);
            recurrence.fast_scalar_second[..scalar_lmax + 1].copy_from_slice(scalar_second);
        }
        #[cfg(all(test, feature = "parallel"))]
        let fallback_started = recurrence
            .spin1_profile_enabled
            .then(std::time::Instant::now);
        #[cfg(all(test, feature = "parallel"))]
        if recurrence.spin1_profile_enabled {
            recurrence.spin1_profile_fallback_blocks += 1;
            recurrence.spin1_profile_route_bins[profile_bin].fallback_entries += 1;
        }
        let mut split_transition_attempted = false;
        let mut split_polar_count = None;
        if self.harmonics.lmax() >= SPIN1_TRANSITION_MIN_LMAX {
            let transition_min_sine =
                T::from_f64(0.01).expect("paired scalar transition sine limit converts");
            let active_start = ring_order_limits
                .iter()
                .position(|&limit| m <= limit)
                .expect("the block has at least one active ring");
            let active_end = ring_order_limits
                .iter()
                .rposition(|&limit| m <= limit)
                .expect("the block has at least one active ring")
                + 1;
            let has_polar_endpoint = theta_sin[active_start] < transition_min_sine
                || theta_sin[active_end - 1] < transition_min_sine;
            if has_polar_endpoint {
                split_transition_attempted = true;
                if theta_sin[active_start..active_end]
                    .iter()
                    .any(|&sine| sine >= transition_min_sine)
                {
                    split_polar_count = try_synthesize_spin1_transition_safe_lanes_to_order(
                        arch,
                        m,
                        scalar_lmax,
                        spectrum_len,
                        output_order,
                        self.sectoral_norms[m],
                        theta_sin,
                        theta_cos,
                        ring_order_limits,
                        q_spectra,
                        u_spectra,
                        recurrence,
                    )?;
                }
            }
        }
        if split_polar_count == Some(0) {
            #[cfg(all(test, feature = "parallel"))]
            if let Some(fallback_started) = fallback_started {
                recurrence.spin1_profile_fallback_ns += fallback_started.elapsed().as_nanos();
            }
            return Ok(());
        }
        let (fallback_sin, fallback_cos, compact_ring_indices, fallback_len, split_polar_fallback) =
            if let Some(polar_count) = split_polar_count {
                (
                    &recurrence.spin_previous_minus[..polar_count],
                    &recurrence.spin_current_minus[..polar_count],
                    None,
                    polar_count,
                    true,
                )
            } else if has_excluded_ring {
                let mut compact_len = 0;
                for (ring, &limit) in ring_order_limits.iter().enumerate() {
                    if m <= limit {
                        recurrence.compact_theta_sin[compact_len] = theta_sin[ring];
                        recurrence.compact_theta_cos[compact_len] = theta_cos[ring];
                        recurrence.compact_ring_indices[compact_len] = ring;
                        compact_len += 1;
                    }
                }
                debug_assert!(compact_len > 0);
                (
                    &recurrence.compact_theta_sin[..compact_len],
                    &recurrence.compact_theta_cos[..compact_len],
                    Some(&recurrence.compact_ring_indices[..compact_len]),
                    compact_len,
                    false,
                )
            } else {
                (theta_sin, theta_cos, None, block_len, false)
            };
        let mut staged_pair =
            if self.harmonics.lmax() >= SPIN1_TRANSITION_MIN_LMAX && !split_transition_attempted {
                if prepared_pair_for_transition {
                    T::try_synthesize_prepared_paired_scalar_transition_dispatched(
                        arch,
                        m,
                        scalar_lmax,
                        fallback_sin,
                        fallback_cos,
                        self.sectoral_norms[m],
                        &recurrence.spin_coefficient_a,
                        &recurrence.spin_coefficient_b,
                        &recurrence.fast_spin_first,
                        &recurrence.fast_spin_second,
                        &recurrence.fast_spin_pair_first,
                        &recurrence.fast_spin_pair_second,
                        &mut recurrence.previous[..fallback_len],
                        &mut recurrence.current[..fallback_len],
                        &mut recurrence.next[..fallback_len],
                        &mut recurrence.exponents[..fallback_len],
                        &mut recurrence.analysis_same_real[..fallback_len],
                        &mut recurrence.analysis_same_imag[..fallback_len],
                        &mut recurrence.analysis_opposite_real[..fallback_len],
                        &mut recurrence.analysis_opposite_imag[..fallback_len],
                    )?
                } else {
                    T::try_synthesize_paired_scalar_transition_dispatched(
                        arch,
                        m,
                        scalar_lmax,
                        fallback_sin,
                        fallback_cos,
                        self.sectoral_norms[m],
                        &recurrence.epsilon,
                        &recurrence.spin_alpha,
                        &recurrence.spin_coefficient_a,
                        &recurrence.spin_coefficient_b,
                        &recurrence.fast_scalar_first[m..=scalar_lmax],
                        &recurrence.fast_scalar_second[m..=scalar_lmax],
                        &mut recurrence.previous[..fallback_len],
                        &mut recurrence.current[..fallback_len],
                        &mut recurrence.next[..fallback_len],
                        &mut recurrence.exponents[..fallback_len],
                        &mut recurrence.analysis_same_real[..fallback_len],
                        &mut recurrence.analysis_same_imag[..fallback_len],
                        &mut recurrence.analysis_opposite_real[..fallback_len],
                        &mut recurrence.analysis_opposite_imag[..fallback_len],
                    )?
                }
            } else {
                false
            };
        if staged_pair {
            for (lane, &sin_theta) in fallback_sin.iter().enumerate() {
                let inverse_sine = sin_theta.recip();
                staged_pair = [
                    recurrence.analysis_same_real[lane] * inverse_sine,
                    recurrence.analysis_same_imag[lane] * inverse_sine,
                    recurrence.analysis_opposite_real[lane] * inverse_sine,
                    recurrence.analysis_opposite_imag[lane] * inverse_sine,
                ]
                .into_iter()
                .all(|value| value.is_finite());
                if !staged_pair {
                    break;
                }
            }
        }
        if staged_pair {
            #[cfg(all(test, feature = "parallel"))]
            if recurrence.spin1_profile_enabled {
                recurrence.spin1_profile_route_bins[profile_bin].transition_successes += 1;
            }
            for lane in 0..fallback_len {
                let local_ring = compact_ring_indices.map_or(lane, |indices| indices[lane]);
                let spectrum_index = local_ring * spectrum_len + output_order;
                let inverse_sine = fallback_sin[lane].recip();
                q_spectra[spectrum_index] += Complex::new(
                    recurrence.analysis_same_real[lane] * inverse_sine,
                    recurrence.analysis_same_imag[lane] * inverse_sine,
                );
                u_spectra[spectrum_index] += Complex::new(
                    recurrence.analysis_opposite_real[lane] * inverse_sine,
                    recurrence.analysis_opposite_imag[lane] * inverse_sine,
                );
            }
            #[cfg(all(test, feature = "parallel"))]
            if let Some(fallback_started) = fallback_started {
                recurrence.spin1_profile_fallback_ns += fallback_started.elapsed().as_nanos();
            }
            return Ok(());
        }

        // Polar synthesis falls back to one scalar recurrence per degree.
        // For the paired f64 route, cache each lane's inverse sine once
        // instead of recomputing the same division for every degree.
        let inverse_sines = if T::supports_spin1_synthesis() {
            for (inverse_sine, &sine) in recurrence.analysis_same_real[..fallback_len]
                .iter_mut()
                .zip(fallback_sin)
            {
                *inverse_sine = sine.recip();
            }
            Some(&recurrence.analysis_same_real[..fallback_len])
        } else {
            None
        };
        #[cfg(all(test, feature = "parallel"))]
        if recurrence.spin1_profile_enabled {
            let profile = &mut recurrence.spin1_profile_route_bins[profile_bin];
            profile.generic_per_degree_blocks += 1;
            profile.generic_lane_pair_work += (fallback_len * coefficient_pair_count) as u128;
        }
        T::for_each_y_lm_dispatched(
            arch,
            m,
            scalar_lmax,
            fallback_sin,
            fallback_cos,
            self.sectoral_norms[m],
            &recurrence.epsilon,
            &mut recurrence.previous[..fallback_len],
            &mut recurrence.current[..fallback_len],
            &mut recurrence.next[..fallback_len],
            &mut recurrence.exponents[..fallback_len],
            |l, block_ring_start, basis, exponents| {
                for lane in 0..basis.len() {
                    let fallback_ring = block_ring_start + lane;
                    let group_ring = if split_polar_fallback {
                        usize::try_from(recurrence.spin_exponents_minus[fallback_ring])
                            .expect("saved ring index is nonnegative")
                    } else {
                        compact_ring_indices.map_or(fallback_ring, |indices| indices[fallback_ring])
                    };
                    let inverse_sine = match inverse_sines {
                        Some(inverse_sines) => inverse_sines[fallback_ring],
                        None => fallback_sin[fallback_ring].recip(),
                    };
                    let first = T::multiply_complex_scaled_product(
                        recurrence.fast_scalar_first[l],
                        basis[lane],
                        inverse_sine,
                        exponents[lane],
                    );
                    let second = T::multiply_complex_scaled_product(
                        recurrence.fast_scalar_second[l],
                        basis[lane],
                        inverse_sine,
                        exponents[lane],
                    );
                    let spectrum_index = group_ring * spectrum_len + output_order;
                    q_spectra[spectrum_index] += first;
                    u_spectra[spectrum_index] += second;
                }
                Ok(())
            },
        )?;
        #[cfg(all(test, feature = "parallel"))]
        if let Some(fallback_started) = fallback_started {
            recurrence.spin1_profile_fallback_ns += fallback_started.elapsed().as_nanos();
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The reflected component adapter writes one order into strided Fourier spectra."
    )]
    pub(super) fn try_synthesize_spin1_prepared_component_order_to_strided_spectra(
        &self,
        m: usize,
        scalar_lmax: usize,
        scalar_first: &[Complex<T>],
        scalar_second: &[Complex<T>],
        pair_coefficients_prepared: bool,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        spectrum_len: usize,
        output_order: usize,
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
    ) -> Option<usize> {
        let ntheta = theta_sin.len();
        let expected_spectrum_len = ntheta.checked_mul(spectrum_len)?;
        if theta_sin.len() != ntheta
            || theta_cos.len() != ntheta
            || ring_order_limits.len() != ntheta
            || output_order >= spectrum_len
            || q_spectra.len() != expected_spectrum_len
            || u_spectra.len() != expected_spectrum_len
        {
            return None;
        }
        let north_count = ntheta.div_ceil(2);
        let pair_min_sine = T::from_f64(SPIN1_PAIR_MIN_SIN_THETA)
            .expect("spin-1 paired scalar sine limit converts");
        let equator_cosine_tolerance = T::from_f64(1.0e-14).expect("equator tolerance converts");
        let mut safe_start = north_count;
        for north in 0..north_count {
            let south = ntheta - 1 - north;
            if m <= ring_order_limits[north]
                && m <= ring_order_limits[south]
                && theta_sin[north] >= pair_min_sine
                && theta_sin[south] >= pair_min_sine
            {
                safe_start = north;
                break;
            }
        }
        if safe_start == north_count {
            return None;
        }
        // The active, non-polar mirrored pairs must form one central band.
        // Preserve the existing lane-wise route if a grid ever violates that
        // geometry or its exact reflected construction.
        for north in safe_start..north_count {
            let south = ntheta - 1 - north;
            if m > ring_order_limits[north]
                || m > ring_order_limits[south]
                || theta_sin[north] < pair_min_sine
                || theta_sin[south] < pair_min_sine
                || theta_sin[north] != theta_sin[south]
                || (south == north && theta_cos[north].abs() > equator_cosine_tolerance)
                || (south != north && theta_cos[north] != -theta_cos[south])
                || ring_order_limits[north] != ring_order_limits[south]
            {
                return None;
            }
        }

        if !pair_coefficients_prepared {
            // Keep the legacy spin route for exceptional and subnormal
            // converted coefficients. The scalar ordered recurrence is used
            // only when both channels satisfy the existing input bound.
            let component_limit = crate::legendre::ScalarLegendreTable::<T>::input_scale_limit()
                / T::from_f64(4.0).expect("four is representable");
            let min_normal = T::from_f64(f64::MIN_POSITIVE).expect("f64 minimum normal converts");
            if scalar_first
                .iter()
                .chain(scalar_second)
                .flat_map(|coefficient| [coefficient.re, coefficient.im])
                .any(|value| {
                    !crate::legendre::ScalarLegendreTable::<T>::input_value_is_safe(
                        value,
                        component_limit,
                    ) || (value != T::zero() && value.abs() < min_normal)
                })
            {
                return None;
            }

            let pair_count = (scalar_lmax - m + 2) / 2;
            if !T::prepare_paired_scalar_synthesis_coefficient_pair(
                m,
                scalar_lmax,
                &recurrence.epsilon,
                &recurrence.spin_alpha,
                scalar_first,
                scalar_second,
                &mut recurrence.fast_spin_first[..pair_count],
                &mut recurrence.fast_spin_second[..pair_count],
                &mut recurrence.fast_spin_pair_first[..pair_count],
                &mut recurrence.fast_spin_pair_second[..pair_count],
            ) {
                return None;
            }
        }
        let simd_lanes = T::scalar_ordered_lane_count(arch);
        if simd_lanes == 0 {
            return None;
        }

        let mut pair_start = safe_start;
        while pair_start < north_count {
            let active_lanes =
                (north_count - pair_start).min(crate::legendre::SCALAR_ORDER_BATCH_SIZE);
            let vector_lanes = active_lanes.div_ceil(simd_lanes).checked_mul(simd_lanes)?;
            if vector_lanes > crate::legendre::SCALAR_ORDER_BATCH_SIZE
                || vector_lanes > recurrence.spin_previous_plus.len()
                || vector_lanes > recurrence.compact_theta_cos_squared.len()
                || active_lanes > recurrence.compact_ring_indices.len()
            {
                return None;
            }
            for lane in 0..active_lanes {
                let north = pair_start + lane;
                let cosine = theta_cos[north];
                let sine = theta_sin[north];
                recurrence.compact_ring_indices[lane] = north;
                recurrence.spin_previous_plus[lane] = sine;
                recurrence.compact_theta_cos_squared[lane] =
                    if cosine.abs() > T::from_f64(0.99).expect("0.99 is representable") {
                        (T::one() - sine) * (T::one() + sine)
                    } else {
                        cosine * cosine
                    };
            }
            let last = active_lanes - 1;
            for lane in active_lanes..vector_lanes {
                recurrence.spin_previous_plus[lane] = recurrence.spin_previous_plus[last];
                recurrence.compact_theta_cos_squared[lane] =
                    recurrence.compact_theta_cos_squared[last];
            }

            if !synthesize_spin1_prepared_scalar_component_batch(
                arch,
                m,
                scalar_lmax,
                self.sectoral_norms[m],
                active_lanes,
                vector_lanes,
                false,
                recurrence,
            ) {
                return None;
            }
            for lane in 0..active_lanes {
                let north = recurrence.compact_ring_indices[lane];
                let south = ntheta - 1 - north;
                let odd_re = recurrence.analysis_opposite_real[lane] * theta_cos[north];
                let odd_im = recurrence.analysis_opposite_imag[lane] * theta_cos[north];
                let even_re = recurrence.analysis_same_real[lane];
                let even_im = recurrence.analysis_same_imag[lane];
                let north_value =
                    Complex::new(even_re + odd_re, even_im + odd_im) / theta_sin[north];
                let south_value =
                    Complex::new(even_re - odd_re, even_im - odd_im) / theta_sin[south];
                if !north_value.re.is_finite()
                    || !north_value.im.is_finite()
                    || !south_value.re.is_finite()
                    || !south_value.im.is_finite()
                {
                    return None;
                }
                let north_spectrum_index = north * spectrum_len + output_order;
                q_spectra[north_spectrum_index] = north_value;
                if south != north {
                    let south_spectrum_index = south * spectrum_len + output_order;
                    q_spectra[south_spectrum_index] = south_value;
                }
            }

            if !synthesize_spin1_prepared_scalar_component_batch(
                arch,
                m,
                scalar_lmax,
                self.sectoral_norms[m],
                active_lanes,
                vector_lanes,
                true,
                recurrence,
            ) {
                return None;
            }
            for lane in 0..active_lanes {
                let north = recurrence.compact_ring_indices[lane];
                let south = ntheta - 1 - north;
                let odd_re = recurrence.analysis_opposite_real[lane] * theta_cos[north];
                let odd_im = recurrence.analysis_opposite_imag[lane] * theta_cos[north];
                let even_re = recurrence.analysis_same_real[lane];
                let even_im = recurrence.analysis_same_imag[lane];
                let north_value =
                    Complex::new(even_re + odd_re, even_im + odd_im) / theta_sin[north];
                let south_value =
                    Complex::new(even_re - odd_re, even_im - odd_im) / theta_sin[south];
                if !north_value.re.is_finite()
                    || !north_value.im.is_finite()
                    || !south_value.re.is_finite()
                    || !south_value.im.is_finite()
                {
                    return None;
                }
                let north_spectrum_index = north * spectrum_len + output_order;
                u_spectra[north_spectrum_index] = north_value;
                if south != north {
                    let south_spectrum_index = south * spectrum_len + output_order;
                    u_spectra[south_spectrum_index] = south_value;
                }
            }
            pair_start += active_lanes;
        }
        Some(safe_start)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The order-major queue converts one order into the worker's scalar coefficient rows."
    )]
    pub(super) fn try_synthesize_spin1_prepared_component_order_from_workspace(
        &self,
        m: usize,
        scalar_lmax: usize,
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        spectrum_len: usize,
        output_order: usize,
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
    ) -> Option<usize> {
        let pair_count = scalar_lmax.checked_sub(m)?.checked_add(2)? / 2;
        let first = recurrence.fast_scalar_first.get(m..=scalar_lmax)?;
        let second = recurrence.fast_scalar_second.get(m..=scalar_lmax)?;
        let component_limit = crate::legendre::ScalarLegendreTable::<T>::input_scale_limit()
            / T::from_f64(4.0).expect("four is representable");
        let min_normal = T::from_f64(f64::MIN_POSITIVE).expect("f64 minimum normal converts");
        if first
            .iter()
            .chain(second)
            .flat_map(|coefficient| [coefficient.re, coefficient.im])
            .any(|value| {
                !crate::legendre::ScalarLegendreTable::<T>::input_value_is_safe(
                    value,
                    component_limit,
                ) || (value != T::zero() && value.abs() < min_normal)
            })
        {
            return None;
        }
        if pair_count > recurrence.fast_spin_first.len()
            || pair_count > recurrence.fast_spin_second.len()
            || pair_count > recurrence.fast_spin_pair_first.len()
            || pair_count > recurrence.fast_spin_pair_second.len()
            || !T::prepare_paired_scalar_synthesis_coefficient_pair(
                m,
                scalar_lmax,
                &recurrence.epsilon,
                &recurrence.spin_alpha,
                first,
                second,
                &mut recurrence.fast_spin_first[..pair_count],
                &mut recurrence.fast_spin_second[..pair_count],
                &mut recurrence.fast_spin_pair_first[..pair_count],
                &mut recurrence.fast_spin_pair_second[..pair_count],
            )
        {
            return None;
        }
        self.try_synthesize_spin1_prepared_component_order_to_strided_spectra(
            m,
            scalar_lmax,
            &[],
            &[],
            true,
            theta_sin,
            theta_cos,
            ring_order_limits,
            spectrum_len,
            output_order,
            arch,
            recurrence,
            q_spectra,
            u_spectra,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Spin-2 scalar synthesis uses the plan's coefficient factors and worker scratch."
    )]
    #[cfg(test)]
    pub(super) fn synthesize_spin2_scalar_order(
        &self,
        m: usize,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        spectrum_len: usize,
        ring_start: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        self.synthesize_spin2_scalar_order_to_order(
            m,
            e_alm,
            b_alm,
            spectrum_len,
            ring_start,
            m,
            q_spectra,
            u_spectra,
            theta_sin,
            theta_cos,
            ring_order_limits,
            arch,
            recurrence,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Order-major spin-2 fallback places one order into worker-local spectra."
    )]
    pub(super) fn synthesize_spin2_scalar_order_to_order(
        &self,
        m: usize,
        e_alm: &[Complex<T>],
        b_alm: &[Complex<T>],
        spectrum_len: usize,
        ring_start: usize,
        output_order: usize,
        q_spectra: &mut [Complex<T>],
        u_spectra: &mut [Complex<T>],
        theta_sin: &[T],
        theta_cos: &[T],
        ring_order_limits: &[usize],
        arch: pulp::Arch,
        recurrence: &mut RecurrenceWorkspace<T>,
    ) -> Result<(), ShtError> {
        let lmax = self.harmonics.lmax();
        let scalar_lmax = lmax + 2;
        let coefficient_start = self
            .harmonics
            .index(m, m)
            .expect("every packed order starts at its diagonal degree");
        let coefficient_count = lmax - m + 1;
        spin2_to_scalar(
            m,
            lmax,
            &e_alm[coefficient_start..coefficient_start + coefficient_count],
            &b_alm[coefficient_start..coefficient_start + coefficient_count],
            &mut recurrence.fast_scalar_first[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_scalar_second[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_spin_first[..self.spin_fast_coefficient_count],
            &mut recurrence.fast_spin_second[..self.spin_fast_coefficient_count],
            &self.spin2_fast_f1,
            &self.spin2_fast_f2,
        );
        let converted_finite = recurrence.fast_scalar_first[m..=scalar_lmax]
            .iter()
            .chain(&recurrence.fast_scalar_second[m..=scalar_lmax])
            .all(|value| value.re.is_finite() && value.im.is_finite());
        if !converted_finite {
            return self.synthesize_general_spin_order_to_order(
                m,
                e_alm,
                Some(b_alm),
                2,
                false,
                spectrum_len,
                ring_start,
                output_order,
                q_spectra,
                u_spectra,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                recurrence,
            );
        }
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
        let pair_count = (scalar_lmax - m + 2) / 2;
        let pairs_finite = paired_factors_ready
            && T::prepare_paired_scalar_synthesis_coefficients(
                m,
                scalar_lmax,
                &recurrence.epsilon,
                &recurrence.spin_alpha,
                &recurrence.fast_scalar_first[m..=scalar_lmax],
                &mut recurrence.fast_spin_first[..pair_count],
                &mut recurrence.fast_spin_second[..pair_count],
            )
            && T::prepare_paired_scalar_synthesis_coefficients(
                m,
                scalar_lmax,
                &recurrence.epsilon,
                &recurrence.spin_alpha,
                &recurrence.fast_scalar_second[m..=scalar_lmax],
                &mut recurrence.fast_spin_first[..pair_count],
                &mut recurrence.fast_spin_second[..pair_count],
            );
        if !pairs_finite {
            return self.synthesize_general_spin_order_to_order(
                m,
                e_alm,
                Some(b_alm),
                2,
                false,
                spectrum_len,
                ring_start,
                output_order,
                q_spectra,
                u_spectra,
                theta_sin,
                theta_cos,
                ring_order_limits,
                arch,
                recurrence,
            );
        }

        for block_start in (0..theta_sin.len()).step_by(crate::legendre::LATITUDE_BLOCK_SIZE) {
            let block_end =
                (block_start + crate::legendre::LATITUDE_BLOCK_SIZE).min(theta_sin.len());
            let block_len = block_end - block_start;
            let block_sine = &theta_sin[block_start..block_end];
            let block_cosine = &theta_cos[block_start..block_end];
            let block_order_limits =
                &ring_order_limits[ring_start + block_start..ring_start + block_end];
            let spectrum_start = block_start * spectrum_len;
            let spectrum_end = block_end * spectrum_len;
            if block_order_limits.iter().all(|&limit| m > limit) {
                continue;
            }
            let fused_pair = T::synthesize_paired_scalar_pair_block_dispatched(
                arch,
                m,
                scalar_lmax,
                block_sine,
                block_cosine,
                self.sectoral_norms[m],
                &recurrence.epsilon,
                &recurrence.spin_alpha,
                &recurrence.spin_coefficient_a,
                &recurrence.spin_coefficient_b,
                &recurrence.fast_scalar_first[m..=scalar_lmax],
                &recurrence.fast_scalar_second[m..=scalar_lmax],
                block_order_limits,
                spectrum_len,
                &mut q_spectra[spectrum_start..spectrum_end],
                &mut u_spectra[spectrum_start..spectrum_end],
                &mut recurrence.previous[..block_len],
                &mut recurrence.current[..block_len],
                &mut recurrence.next[..block_len],
                &mut recurrence.exponents[..block_len],
                &mut recurrence.analysis_same_real[..block_len],
                &mut recurrence.analysis_same_imag[..block_len],
                &mut recurrence.analysis_opposite_real[..block_len],
                &mut recurrence.analysis_opposite_imag[..block_len],
            )?;
            if fused_pair {
                for (local_ring, &sine) in block_sine.iter().enumerate() {
                    let spectrum_index = local_ring * spectrum_len + output_order;
                    let inverse_sine = sine.recip();
                    let inverse_sine_squared = inverse_sine * inverse_sine;
                    q_spectra[spectrum_start + spectrum_index] *= inverse_sine_squared;
                    u_spectra[spectrum_start + spectrum_index] *= inverse_sine_squared;
                }
                continue;
            }

            let (fallback_sine, fallback_cosine, compact_ring_indices) =
                if block_order_limits.iter().any(|&limit| m > limit) {
                    let mut compact_len = 0;
                    for (local_ring, &limit) in block_order_limits.iter().enumerate() {
                        if m <= limit {
                            recurrence.compact_theta_sin[compact_len] = block_sine[local_ring];
                            recurrence.compact_theta_cos[compact_len] = block_cosine[local_ring];
                            recurrence.compact_ring_indices[compact_len] = local_ring;
                            compact_len += 1;
                        }
                    }
                    debug_assert!(compact_len > 0);
                    (
                        &recurrence.compact_theta_sin[..compact_len],
                        &recurrence.compact_theta_cos[..compact_len],
                        Some(&recurrence.compact_ring_indices[..compact_len]),
                    )
                } else {
                    (block_sine, block_cosine, None)
                };

            let mut staged_pair = T::try_synthesize_paired_scalar_transition_dispatched(
                arch,
                m,
                scalar_lmax,
                fallback_sine,
                fallback_cosine,
                self.sectoral_norms[m],
                &recurrence.epsilon,
                &recurrence.spin_alpha,
                &recurrence.spin_coefficient_a,
                &recurrence.spin_coefficient_b,
                &recurrence.fast_scalar_first[m..=scalar_lmax],
                &recurrence.fast_scalar_second[m..=scalar_lmax],
                &mut recurrence.previous[..fallback_sine.len()],
                &mut recurrence.current[..fallback_sine.len()],
                &mut recurrence.next[..fallback_sine.len()],
                &mut recurrence.exponents[..fallback_sine.len()],
                &mut recurrence.analysis_same_real[..fallback_sine.len()],
                &mut recurrence.analysis_same_imag[..fallback_sine.len()],
                &mut recurrence.analysis_opposite_real[..fallback_sine.len()],
                &mut recurrence.analysis_opposite_imag[..fallback_sine.len()],
            )?;
            if staged_pair {
                for (lane, &sine) in fallback_sine.iter().enumerate() {
                    let inverse_sine = sine.recip();
                    let inverse_sine_squared = inverse_sine * inverse_sine;
                    staged_pair = [
                        recurrence.analysis_same_real[lane] * inverse_sine_squared,
                        recurrence.analysis_same_imag[lane] * inverse_sine_squared,
                        recurrence.analysis_opposite_real[lane] * inverse_sine_squared,
                        recurrence.analysis_opposite_imag[lane] * inverse_sine_squared,
                    ]
                    .into_iter()
                    .all(|value| value.is_finite());
                    if !staged_pair {
                        break;
                    }
                }
            }
            if staged_pair {
                for lane in 0..fallback_sine.len() {
                    let local_ring = compact_ring_indices.map_or(lane, |indices| indices[lane]);
                    let group_ring = block_start + local_ring;
                    let inverse_sine = fallback_sine[lane].recip();
                    let inverse_sine_squared = inverse_sine * inverse_sine;
                    let spectrum_index = group_ring * spectrum_len + output_order;
                    q_spectra[spectrum_index] += Complex::new(
                        recurrence.analysis_same_real[lane] * inverse_sine_squared,
                        recurrence.analysis_same_imag[lane] * inverse_sine_squared,
                    );
                    u_spectra[spectrum_index] += Complex::new(
                        recurrence.analysis_opposite_real[lane] * inverse_sine_squared,
                        recurrence.analysis_opposite_imag[lane] * inverse_sine_squared,
                    );
                }
                continue;
            }

            T::for_each_y_lm_paired_dispatched(
                arch,
                m,
                scalar_lmax,
                fallback_sine,
                fallback_cosine,
                self.sectoral_norms[m],
                &recurrence.spin_coefficient_a,
                &recurrence.spin_coefficient_b,
                &mut recurrence.previous[..block_len],
                &mut recurrence.current[..block_len],
                &mut recurrence.next[..block_len],
                &mut recurrence.exponents[..block_len],
                |l, block_ring_start, basis, exponents| {
                    let pair_index = (l - m) / 2;
                    let next_even = l + 2 <= scalar_lmax;
                    let next_first = if next_even {
                        recurrence.fast_scalar_first[l + 2]
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    let next_second = if next_even {
                        recurrence.fast_scalar_second[l + 2]
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    let even_first = (recurrence.fast_scalar_first[l] * recurrence.epsilon[l + 1]
                        + next_first * recurrence.epsilon[l + 2])
                        * recurrence.spin_alpha[pair_index];
                    let even_second = (recurrence.fast_scalar_second[l]
                        * recurrence.epsilon[l + 1]
                        + next_second * recurrence.epsilon[l + 2])
                        * recurrence.spin_alpha[pair_index];
                    let odd_first = if l < scalar_lmax {
                        recurrence.fast_scalar_first[l + 1] * recurrence.spin_alpha[pair_index]
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    let odd_second = if l < scalar_lmax {
                        recurrence.fast_scalar_second[l + 1] * recurrence.spin_alpha[pair_index]
                    } else {
                        Complex::new(T::zero(), T::zero())
                    };
                    for lane in 0..basis.len() {
                        let fallback_lane = block_ring_start + lane;
                        let local_ring = compact_ring_indices
                            .map_or(fallback_lane, |indices| indices[fallback_lane]);
                        let group_ring = block_start + local_ring;
                        let global_ring = ring_start + group_ring;
                        if m > ring_order_limits[global_ring] {
                            continue;
                        }
                        let inverse_sine = fallback_sine[fallback_lane].recip();
                        let inverse_sine_squared = inverse_sine * inverse_sine;
                        let cosine_over_sine_squared =
                            fallback_cosine[fallback_lane] * inverse_sine_squared;
                        let first = T::multiply_complex_scaled_product(
                            even_first,
                            basis[lane],
                            inverse_sine_squared,
                            exponents[lane],
                        ) + T::multiply_complex_scaled_product(
                            odd_first,
                            basis[lane],
                            cosine_over_sine_squared,
                            exponents[lane],
                        );
                        let second = T::multiply_complex_scaled_product(
                            even_second,
                            basis[lane],
                            inverse_sine_squared,
                            exponents[lane],
                        ) + T::multiply_complex_scaled_product(
                            odd_second,
                            basis[lane],
                            cosine_over_sine_squared,
                            exponents[lane],
                        );
                        let spectrum_index = group_ring * spectrum_len + output_order;
                        q_spectra[spectrum_index] += first;
                        u_spectra[spectrum_index] += second;
                    }
                    Ok(())
                },
            )?;
        }
        Ok(())
    }
}
