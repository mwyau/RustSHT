// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * This module translates DUCC's prepared CC/F1 resampling path and the
 * FFT-size search used to select its prepared grid.
 *
 * The resampling path is from DUCC's MR utility library, src/ducc0/sht/sht.cc,
 * Copyright (C) 2020-2025 Max-Planck-Society, Martin Reinecke, revision
 * c1ed01b8fda69d2a378190f0915f1a19762dddbf. It is licensed under the GNU
 * General Public License, version 2 or (at your option) any later version.
 *
 * The FFT-size search is adapted from DUCC's src/ducc0/fft/fft.h,
 * Copyright (C) 2010-2026 Max-Planck-Society and Copyright (C) 2019 Peter
 * Bell, authors Martin Reinecke and Peter Bell, at the same revision. That
 * source is licensed under BSD-3-Clause OR GPL-2.0-or-later; this translation
 * is distributed under GPL-2.0-or-later.
 *
 * The source code is free software; you can redistribute it and/or modify it
 * under the terms of the GNU General Public License as published by the Free
 * Software Foundation, either version 2 of the License, or (at your option)
 * any later version. It is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General
 * Public License for more details.
 */

use std::sync::Arc;

use num_complex::Complex;
use rustfft::num_traits::{Float, Zero};
use rustfft::{Fft, FftNum, FftPlanner};

use crate::error::ShtError;
use crate::grid::{GridKind, clenshaw_curtis_ring_weights};

struct FftPair<T: FftNum> {
    forward: Arc<dyn Fft<T>>,
    inverse: Arc<dyn Fft<T>>,
}

impl<T: FftNum> FftPair<T> {
    fn new(planner: &mut FftPlanner<T>, len: usize) -> Self {
        Self {
            forward: planner.plan_fft_forward(len),
            inverse: planner.plan_fft_inverse(len),
        }
    }

    fn scratch_len(&self) -> usize {
        self.forward
            .get_inplace_scratch_len()
            .max(self.inverse.get_inplace_scratch_len())
    }
}

/// Prepared CC analysis grid and CC/F1 meridional FFT operators.
pub(crate) struct CCResamplingPlan<T: FftNum = f64> {
    original_ntheta: usize,
    original_north_pole: bool,
    phase_shift: Vec<Complex<T>>,
    prepared_theta: Vec<f64>,
    original_full_len: usize,
    prepared_full_len: usize,
    full_len: usize,
    needs_resampling: bool,
    weights: Vec<T>,
    original_fft: FftPair<T>,
    prepared_fft: FftPair<T>,
    full_fft: FftPair<T>,
    fft_scratch_len: usize,
}

impl<T: FftNum + Float> CCResamplingPlan<T> {
    pub(crate) fn new(
        original_kind: GridKind,
        original_ntheta: usize,
        lmax: usize,
    ) -> Result<Self, ShtError> {
        let original_north_pole = match original_kind {
            GridKind::ClenshawCurtis => true,
            GridKind::Fejer1 => false,
            GridKind::GaussLegendre => {
                unreachable!("CC/F1 resampling is only prepared for those grid kinds")
            }
        };
        let minimum_prepared = lmax.checked_add(1).ok_or(ShtError::IntegerOverflow {
            quantity: "prepared CC latitude count",
        })?;
        let prepared_half = good_size_complex(minimum_prepared)?;
        let prepared_ntheta = prepared_half
            .checked_add(1)
            .ok_or(ShtError::IntegerOverflow {
                quantity: "prepared CC latitude count",
            })?;
        let prepared_full_len = prepared_ntheta
            .checked_sub(1)
            .and_then(|value| value.checked_mul(2))
            .ok_or(ShtError::IntegerOverflow {
                quantity: "prepared CC reflected latitude length",
            })?;
        let original_reflected_len = original_ntheta
            .checked_sub(1)
            .and_then(|value| value.checked_mul(2));
        let original_full_len = match original_kind {
            GridKind::ClenshawCurtis => original_reflected_len,
            GridKind::Fejer1 => original_ntheta.checked_mul(2),
            GridKind::GaussLegendre => unreachable!("validated above"),
        }
        .ok_or(ShtError::IntegerOverflow {
            quantity: "CC/F1 reflected latitude length",
        })?;
        let minimum_skip = lmax
            .checked_mul(2)
            .and_then(|value| value.checked_add(2))
            .ok_or(ShtError::IntegerOverflow {
                quantity: "CC resampling threshold",
            })?;
        let needs_resampling = match original_kind {
            GridKind::ClenshawCurtis => original_ntheta < minimum_skip,
            GridKind::Fejer1 => true,
            GridKind::GaussLegendre => unreachable!("validated above"),
        };
        let full_len = if needs_resampling {
            prepared_full_len
                .checked_mul(2)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "CC meridional FFT length",
                })?
        } else {
            original_full_len
        };
        let weights = clenshaw_curtis_ring_weights(full_len / 2 + 1)?
            .into_iter()
            .map(|weight| T::from_f64(weight).expect("quadrature weight is representable"))
            .collect();

        let mut prepared_theta = allocate_real(prepared_ntheta, "prepared CC colatitudes")?;
        let north_count = prepared_ntheta.div_ceil(2);
        for i in 0..north_count {
            let theta = std::f64::consts::PI * i as f64 / (prepared_ntheta - 1) as f64;
            let south = prepared_ntheta - 1 - i;
            prepared_theta[i] = theta;
            prepared_theta[south] = std::f64::consts::PI - theta;
        }
        prepared_theta[0] = 0.0;

        let mut phase_shift = allocate_complex(
            if original_north_pole {
                0
            } else {
                original_ntheta + 1
            },
            "F1 meridional phase shift",
        )?;
        if !original_north_pole {
            for (index, value) in phase_shift.iter_mut().enumerate() {
                let angle = std::f64::consts::PI * index as f64 / original_full_len as f64;
                let (sin, cos) = T::from_f64(angle)
                    .expect("phase angle is representable")
                    .sin_cos();
                *value = Complex::new(cos, sin);
            }
        }

        let mut planner = FftPlanner::<T>::new();
        let original_fft = FftPair::new(&mut planner, original_full_len);
        let prepared_fft = FftPair::new(&mut planner, prepared_full_len);
        let full_fft = FftPair::new(&mut planner, full_len);
        let fft_scratch_len = original_fft
            .scratch_len()
            .max(prepared_fft.scratch_len())
            .max(full_fft.scratch_len());

        Ok(Self {
            original_ntheta,
            original_north_pole,
            phase_shift,
            prepared_theta,
            original_full_len,
            prepared_full_len,
            full_len,
            needs_resampling,
            weights,
            original_fft,
            prepared_fft,
            full_fft,
            fft_scratch_len,
        })
    }

    pub(crate) fn prepared_theta(&self) -> &[f64] {
        &self.prepared_theta
    }

    pub(crate) const fn prepared_ntheta(&self) -> usize {
        self.prepared_theta.len()
    }

    pub(crate) const fn workspace_profile_len(&self) -> usize {
        if self.original_full_len > self.full_len {
            self.original_full_len
        } else {
            self.full_len
        }
    }

    pub(crate) const fn workspace_fft_scratch_len(&self) -> usize {
        self.fft_scratch_len
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The resampler receives spectra, Fourier limits and caller-owned scratch."
    )]
    pub(crate) fn to_prepared(
        &self,
        input: &[Complex<T>],
        output: &mut [Complex<T>],
        spectrum_len: usize,
        mmax: usize,
        spin: usize,
        profile: &mut [Complex<T>],
        fft_scratch: &mut [Complex<T>],
    ) -> Result<(), ShtError> {
        check_len(
            "CC input Fourier spectra",
            checked_product(
                self.original_ntheta,
                spectrum_len,
                "CC input Fourier spectra",
            )?,
            input.len(),
        )?;
        check_len(
            "prepared CC Fourier spectra",
            checked_product(
                self.prepared_ntheta(),
                spectrum_len,
                "prepared CC Fourier spectra",
            )?,
            output.len(),
        )?;
        check_len(
            "CC/F1 meridional profile",
            self.workspace_profile_len(),
            profile.len(),
        )?;
        check_len(
            "CC meridional FFT scratch",
            self.fft_scratch_len,
            fft_scratch.len(),
        )?;
        output.fill(Complex::new(T::zero(), T::zero()));
        let nfull_in = self.original_full_len;
        let nfull_out = self.prepared_full_len;
        let norm = T::from_f64(0.5).expect("half is representable")
            / (T::from_usize(nfull_out).expect("usize converts to the transform scalar")
                * if self.needs_resampling {
                    T::from_usize(nfull_in).expect("usize converts to the transform scalar")
                } else {
                    T::one()
                });
        let parity = if spin.is_multiple_of(2) {
            T::one()
        } else {
            -T::one()
        };

        for m in 0..=mmax {
            let order_parity = if m.is_multiple_of(2) {
                T::one()
            } else {
                -T::one()
            };
            let reflected_sign = parity * order_parity;
            self.fill_reflected_profile(
                input,
                self.original_ntheta,
                nfull_in,
                spectrum_len,
                m,
                reflected_sign,
                self.original_north_pole,
                profile,
            );
            if self.needs_resampling {
                self.original_fft
                    .forward
                    .process_with_scratch(&mut profile[..nfull_in], fft_scratch);
                apply_phase_shift(
                    &mut profile[..nfull_in],
                    self.original_ntheta,
                    &self.phase_shift,
                );
                if self.full_len > nfull_in {
                    pad_spectrum(profile, nfull_in, self.full_len);
                } else if self.full_len < nfull_in {
                    truncate_spectrum(profile, nfull_in, self.full_len);
                }
                self.full_fft
                    .inverse
                    .process_with_scratch(&mut profile[..self.full_len], fft_scratch);
            }
            multiply_cc_weights(profile, self.full_len, &self.weights);
            self.full_fft
                .forward
                .process_with_scratch(&mut profile[..self.full_len], fft_scratch);
            truncate_spectrum(profile, self.full_len, nfull_out);
            self.prepared_fft
                .inverse
                .process_with_scratch(&mut profile[..nfull_out], fft_scratch);

            for ring in 0..self.prepared_ntheta() {
                let mirror = reflected_ring(nfull_out, ring, true);
                let endpoint_scale = if mirror == ring {
                    T::from_f64(0.5).expect("half is representable")
                } else {
                    T::one()
                };
                output[ring * spectrum_len + m] =
                    (profile[ring] + profile[mirror] * reflected_sign) * (endpoint_scale * norm);
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The resampler receives spectra, Fourier limits and caller-owned scratch."
    )]
    pub(crate) fn restore_original(
        &self,
        input: &[Complex<T>],
        output: &mut [Complex<T>],
        spectrum_len: usize,
        mmax: usize,
        spin: usize,
        profile: &mut [Complex<T>],
        fft_scratch: &mut [Complex<T>],
    ) -> Result<(), ShtError> {
        check_len(
            "prepared CC Fourier spectra",
            checked_product(
                self.prepared_ntheta(),
                spectrum_len,
                "prepared CC Fourier spectra",
            )?,
            input.len(),
        )?;
        check_len(
            "CC output Fourier spectra",
            checked_product(
                self.original_ntheta,
                spectrum_len,
                "CC output Fourier spectra",
            )?,
            output.len(),
        )?;
        check_len(
            "CC/F1 meridional profile",
            self.workspace_profile_len(),
            profile.len(),
        )?;
        check_len(
            "CC meridional FFT scratch",
            self.fft_scratch_len,
            fft_scratch.len(),
        )?;
        output.fill(Complex::new(T::zero(), T::zero()));
        let nfull_in = self.prepared_full_len;
        let nfull_out = self.original_full_len;
        let norm = T::from_f64(0.5).expect("half is representable")
            / (T::from_usize(nfull_in).expect("usize converts to the transform scalar")
                * if self.needs_resampling {
                    T::from_usize(nfull_out).expect("usize converts to the transform scalar")
                } else {
                    T::one()
                });
        let parity = if spin.is_multiple_of(2) {
            T::one()
        } else {
            -T::one()
        };

        for m in 0..=mmax {
            let order_parity = if m.is_multiple_of(2) {
                T::one()
            } else {
                -T::one()
            };
            let reflected_sign = parity * order_parity;
            self.fill_reflected_profile(
                input,
                self.prepared_ntheta(),
                nfull_in,
                spectrum_len,
                m,
                reflected_sign,
                true,
                profile,
            );
            self.prepared_fft
                .inverse
                .process_with_scratch(&mut profile[..nfull_in], fft_scratch);
            pad_spectrum(profile, nfull_in, self.full_len);
            self.full_fft
                .forward
                .process_with_scratch(&mut profile[..self.full_len], fft_scratch);
            multiply_cc_weights(profile, self.full_len, &self.weights);

            if self.needs_resampling {
                self.full_fft
                    .inverse
                    .process_with_scratch(&mut profile[..self.full_len], fft_scratch);
                if nfull_out > self.full_len {
                    pad_spectrum(profile, self.full_len, nfull_out);
                } else if nfull_out < self.full_len {
                    truncate_spectrum(profile, self.full_len, nfull_out);
                }
                apply_phase_shift(
                    &mut profile[..nfull_out],
                    self.original_ntheta,
                    &self.phase_shift,
                );
                self.original_fft
                    .forward
                    .process_with_scratch(&mut profile[..nfull_out], fft_scratch);
            }

            for ring in 0..self.original_ntheta {
                let mirror = reflected_ring(nfull_out, ring, self.original_north_pole);
                let endpoint_scale = if mirror == ring {
                    T::from_f64(0.5).expect("half is representable")
                } else {
                    T::one()
                };
                output[ring * spectrum_len + m] =
                    (profile[ring] + profile[mirror] * reflected_sign) * (endpoint_scale * norm);
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The profile reflection needs the input layout, Fourier mode, parity and pole convention."
    )]
    fn fill_reflected_profile(
        &self,
        input: &[Complex<T>],
        ring_count: usize,
        full_len: usize,
        spectrum_len: usize,
        m: usize,
        reflected_sign: T,
        north_pole_included: bool,
        profile: &mut [Complex<T>],
    ) {
        profile.fill(Complex::new(T::zero(), T::zero()));
        for ring in 0..ring_count {
            let mirror = full_len - 1 + usize::from(north_pole_included) - ring;
            let value = input[ring * spectrum_len + m];
            profile[ring] = value;
            if mirror < full_len && mirror != ring {
                profile[mirror] = value * reflected_sign;
            } else {
                profile[ring] = (profile[ring] + value * reflected_sign)
                    * T::from_f64(0.5).expect("half is representable");
            }
        }
    }
}

fn apply_phase_shift<T: FftNum + Float>(
    profile: &mut [Complex<T>],
    ring_count: usize,
    shift: &[Complex<T>],
) {
    if shift.is_empty() {
        return;
    }
    let mut mirror = profile.len() - 1;
    for index in 1..ring_count + 1 {
        if index > mirror {
            break;
        }
        if index != mirror {
            profile[index] = profile[index] * shift[index].conj();
        }
        profile[mirror] = profile[mirror] * shift[index];
        mirror -= 1;
    }
}

fn multiply_cc_weights<T: FftNum + Float>(
    profile: &mut [Complex<T>],
    full_len: usize,
    weights: &[T],
) {
    let half = full_len / 2;
    for i in 0..=half {
        let mirror = full_len - i;
        let mut scale = weights[i];
        if i == 0 || i == mirror {
            scale = scale * T::from_f64(2.0).expect("two is representable");
        }
        profile[i] = profile[i] * scale;
        if mirror < full_len && mirror != i {
            profile[mirror] = profile[mirror] * weights[i];
        }
    }
}

fn pad_spectrum<T: FftNum + Zero>(profile: &mut [Complex<T>], old_len: usize, new_len: usize) {
    debug_assert!(new_len >= old_len);
    if new_len == old_len {
        return;
    }
    let move_count = old_len / 2;
    profile.copy_within(old_len - move_count..old_len, new_len - move_count);
    profile[new_len - move_count - (new_len - old_len)..new_len - move_count]
        .fill(Complex::new(T::zero(), T::zero()));
}

fn truncate_spectrum<T: FftNum>(profile: &mut [Complex<T>], old_len: usize, new_len: usize) {
    debug_assert!(new_len <= old_len);
    if new_len == old_len {
        return;
    }
    let move_count = new_len / 2;
    let destination = new_len - move_count;
    profile.copy_within(old_len - move_count..old_len, destination);
}

fn reflected_ring(full_len: usize, ring: usize, north_pole_included: bool) -> usize {
    let mirror = full_len - 1 + usize::from(north_pole_included) - ring;
    if mirror == full_len { 0 } else { mirror }
}

fn good_size_complex(n: usize) -> Result<usize, ShtError> {
    if n <= 12 {
        return Ok(n);
    }
    let mut best = n.checked_mul(2).ok_or(ShtError::IntegerOverflow {
        quantity: "prepared CC FFT friendly size",
    })?;
    let mut factor_11 = 1usize;
    while factor_11 < best {
        let mut factor_7 = factor_11;
        while factor_7 < best {
            let mut factor_5 = factor_7;
            while factor_5 < best {
                best = iter_23(n, factor_5, best)?;
                if best == n {
                    return Ok(n);
                }
                let Some(next) = factor_5.checked_mul(5) else {
                    break;
                };
                factor_5 = next;
            }
            let Some(next) = factor_7.checked_mul(7) else {
                break;
            };
            factor_7 = next;
        }
        let Some(next) = factor_11.checked_mul(11) else {
            break;
        };
        factor_11 = next;
    }
    Ok(best)
}

fn iter_23(n: usize, mut value: usize, mut best: usize) -> Result<usize, ShtError> {
    while value < n {
        value = value.checked_mul(2).ok_or(ShtError::IntegerOverflow {
            quantity: "prepared CC FFT size search",
        })?;
    }
    loop {
        if value < n {
            value = value.checked_mul(3).ok_or(ShtError::IntegerOverflow {
                quantity: "prepared CC FFT size search",
            })?;
        } else if value > n {
            best = best.min(value);
            if value & 1 == 1 {
                return Ok(best);
            }
            value >>= 1;
        } else {
            return Ok(n);
        }
    }
}

fn allocate_real<T: FftNum + Zero>(len: usize, array: &'static str) -> Result<Vec<T>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, T::zero());
    Ok(values)
}

fn allocate_complex<T: FftNum + Zero>(
    len: usize,
    array: &'static str,
) -> Result<Vec<Complex<T>>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, Complex::new(T::zero(), T::zero()));
    Ok(values)
}

fn checked_product(left: usize, right: usize, quantity: &'static str) -> Result<usize, ShtError> {
    left.checked_mul(right)
        .ok_or(ShtError::IntegerOverflow { quantity })
}

fn check_len(buffer: &'static str, expected: usize, actual: usize) -> Result<(), ShtError> {
    if expected == actual {
        Ok(())
    } else {
        Err(ShtError::BufferLength {
            buffer,
            expected,
            actual,
        })
    }
}
