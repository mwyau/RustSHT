// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * This module translates DUCC's prepared Clenshaw-Curtis resampling path and
 * the FFT-size search used to select its prepared grid.
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

use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};

use crate::error::ShtError;
use crate::grid::clenshaw_curtis_ring_weights;

struct FftPair {
    forward: Arc<dyn Fft<f64>>,
    inverse: Arc<dyn Fft<f64>>,
}

impl FftPair {
    fn new(planner: &mut FftPlanner<f64>, len: usize) -> Self {
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

/// Prepared CC analysis grid and its meridional FFT operators.
pub(crate) struct CCResamplingPlan {
    original_ntheta: usize,
    prepared_theta: Vec<f64>,
    original_full_len: usize,
    prepared_full_len: usize,
    full_len: usize,
    needs_resampling: bool,
    weights: Vec<f64>,
    original_fft: FftPair,
    prepared_fft: FftPair,
    full_fft: FftPair,
    fft_scratch_len: usize,
}

impl CCResamplingPlan {
    pub(crate) fn new(original_ntheta: usize, lmax: usize) -> Result<Self, ShtError> {
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
        let original_full_len = original_ntheta
            .checked_sub(1)
            .and_then(|value| value.checked_mul(2))
            .ok_or(ShtError::IntegerOverflow {
                quantity: "CC reflected latitude length",
            })?;
        let minimum_skip = lmax
            .checked_mul(2)
            .and_then(|value| value.checked_add(2))
            .ok_or(ShtError::IntegerOverflow {
                quantity: "CC resampling threshold",
            })?;
        let needs_resampling = original_ntheta < minimum_skip;
        let full_len = if needs_resampling {
            prepared_full_len
                .checked_mul(2)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "CC meridional FFT length",
                })?
        } else {
            original_full_len
        };
        let weights = clenshaw_curtis_ring_weights(full_len / 2 + 1)?;

        let mut prepared_theta = allocate_f64(prepared_ntheta, "prepared CC colatitudes")?;
        let north_count = prepared_ntheta.div_ceil(2);
        for i in 0..north_count {
            let theta = std::f64::consts::PI * i as f64 / (prepared_ntheta - 1) as f64;
            let south = prepared_ntheta - 1 - i;
            prepared_theta[i] = theta;
            prepared_theta[south] = std::f64::consts::PI - theta;
        }
        prepared_theta[0] = 0.0;

        let mut planner = FftPlanner::<f64>::new();
        let original_fft = FftPair::new(&mut planner, original_full_len);
        let prepared_fft = FftPair::new(&mut planner, prepared_full_len);
        let full_fft = FftPair::new(&mut planner, full_len);
        let fft_scratch_len = original_fft
            .scratch_len()
            .max(prepared_fft.scratch_len())
            .max(full_fft.scratch_len());

        Ok(Self {
            original_ntheta,
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
        self.full_len
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
        input: &[Complex64],
        output: &mut [Complex64],
        spectrum_len: usize,
        mmax: usize,
        spin: usize,
        profile: &mut [Complex64],
        fft_scratch: &mut [Complex64],
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
        check_len("CC meridional profile", self.full_len, profile.len())?;
        check_len(
            "CC meridional FFT scratch",
            self.fft_scratch_len,
            fft_scratch.len(),
        )?;
        output.fill(Complex64::new(0.0, 0.0));
        let nfull_in = self.original_full_len;
        let nfull_out = self.prepared_full_len;
        let norm = 0.5
            / (nfull_out as f64
                * if self.needs_resampling {
                    nfull_in as f64
                } else {
                    1.0
                });
        let parity = if spin.is_multiple_of(2) { 1.0 } else { -1.0 };

        for m in 0..=mmax {
            let reflected_sign = parity * if m.is_multiple_of(2) { 1.0 } else { -1.0 };
            self.fill_reflected_profile(
                input,
                self.original_ntheta,
                spectrum_len,
                m,
                reflected_sign,
                profile,
            );
            if self.needs_resampling {
                self.original_fft
                    .forward
                    .process_with_scratch(&mut profile[..nfull_in], fft_scratch);
                pad_spectrum(profile, nfull_in, self.full_len);
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
                let mirror = reflected_ring(nfull_out, ring);
                let endpoint_scale = if mirror == ring { 0.5 } else { 1.0 };
                output[ring * spectrum_len + m] =
                    endpoint_scale * norm * (profile[ring] + reflected_sign * profile[mirror]);
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
        input: &[Complex64],
        output: &mut [Complex64],
        spectrum_len: usize,
        mmax: usize,
        spin: usize,
        profile: &mut [Complex64],
        fft_scratch: &mut [Complex64],
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
        check_len("CC meridional profile", self.full_len, profile.len())?;
        check_len(
            "CC meridional FFT scratch",
            self.fft_scratch_len,
            fft_scratch.len(),
        )?;
        output.fill(Complex64::new(0.0, 0.0));
        let nfull_in = self.prepared_full_len;
        let nfull_out = self.original_full_len;
        let norm = 0.5
            / (nfull_in as f64
                * if self.needs_resampling {
                    nfull_out as f64
                } else {
                    1.0
                });
        let parity = if spin.is_multiple_of(2) { 1.0 } else { -1.0 };

        for m in 0..=mmax {
            let reflected_sign = parity * if m.is_multiple_of(2) { 1.0 } else { -1.0 };
            self.fill_reflected_profile(
                input,
                self.prepared_ntheta(),
                spectrum_len,
                m,
                reflected_sign,
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
                truncate_spectrum(profile, self.full_len, nfull_out);
                self.original_fft
                    .forward
                    .process_with_scratch(&mut profile[..nfull_out], fft_scratch);
            }

            for ring in 0..self.original_ntheta {
                let mirror = reflected_ring(nfull_out, ring);
                let endpoint_scale = if mirror == ring { 0.5 } else { 1.0 };
                output[ring * spectrum_len + m] =
                    endpoint_scale * norm * (profile[ring] + reflected_sign * profile[mirror]);
            }
        }
        Ok(())
    }

    fn fill_reflected_profile(
        &self,
        input: &[Complex64],
        ring_count: usize,
        spectrum_len: usize,
        m: usize,
        reflected_sign: f64,
        profile: &mut [Complex64],
    ) {
        profile.fill(Complex64::new(0.0, 0.0));
        let full_len = 2 * (ring_count - 1);
        for ring in 0..ring_count {
            let mirror = full_len - ring;
            let value = input[ring * spectrum_len + m];
            profile[ring] = value;
            if mirror < full_len && mirror != ring {
                profile[mirror] = reflected_sign * value;
            } else {
                profile[ring] = 0.5 * (profile[ring] + reflected_sign * value);
            }
        }
    }
}

fn multiply_cc_weights(profile: &mut [Complex64], full_len: usize, weights: &[f64]) {
    let half = full_len / 2;
    for i in 0..=half {
        let mirror = full_len - i;
        let mut scale = weights[i];
        if i == 0 || i == mirror {
            scale *= 2.0;
        }
        profile[i] *= scale;
        if mirror < full_len && mirror != i {
            profile[mirror] *= weights[i];
        }
    }
}

fn pad_spectrum(profile: &mut [Complex64], old_len: usize, new_len: usize) {
    debug_assert!(new_len >= old_len);
    if new_len == old_len {
        return;
    }
    let move_count = old_len / 2;
    profile.copy_within(old_len - move_count..old_len, new_len - move_count);
    profile[new_len - move_count - (new_len - old_len)..new_len - move_count]
        .fill(Complex64::new(0.0, 0.0));
}

fn truncate_spectrum(profile: &mut [Complex64], old_len: usize, new_len: usize) {
    debug_assert!(new_len <= old_len);
    if new_len == old_len {
        return;
    }
    let move_count = new_len / 2;
    let destination = new_len - move_count;
    profile.copy_within(old_len - move_count..old_len, destination);
}

fn reflected_ring(full_len: usize, ring: usize) -> usize {
    let mirror = full_len - ring;
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

fn allocate_f64(len: usize, array: &'static str) -> Result<Vec<f64>, ShtError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| ShtError::AllocationFailed { array })?;
    values.resize(len, 0.0);
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
