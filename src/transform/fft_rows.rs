// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(test)]
use std::sync::atomic::Ordering;

use crate::error::ShtError;
use crate::fft::RealFftWorkspace;
use crate::legendre::ShtScalar;
use crate::plan::ShtPlan;

#[cfg(test)]
use super::{
    PAIRED_FORWARD_DECLINES, PAIRED_FORWARD_SUCCESSES, PAIRED_INVERSE_DECLINES,
    PAIRED_INVERSE_SUCCESSES,
};

#[allow(private_bounds)]
impl<T: ShtScalar> ShtPlan<T> {
    #[inline]
    pub(super) fn forward_analysis_row_pair(
        &self,
        first_input: &[T],
        second_input: &[T],
        first_output: &mut [Complex<T>],
        second_output: &mut [Complex<T>],
        fft_workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), ShtError> {
        let paired = self.fft.forward_paired_truncated(
            first_input,
            second_input,
            first_output,
            second_output,
            fft_workspace,
        )?;
        #[cfg(test)]
        if paired {
            PAIRED_FORWARD_SUCCESSES.fetch_add(1, Ordering::Relaxed);
        } else {
            PAIRED_FORWARD_DECLINES.fetch_add(1, Ordering::Relaxed);
        }
        if paired {
            return Ok(());
        }
        self.fft
            .forward_truncated(first_input, first_output, fft_workspace)?;
        self.fft
            .forward_truncated(second_input, second_output, fft_workspace)
    }

    #[inline]
    pub(super) fn inverse_synthesis_row_pair(
        &self,
        first_input: &[Complex<T>],
        second_input: &[Complex<T>],
        first_output: &mut [T],
        second_output: &mut [T],
        fft_workspace: &mut RealFftWorkspace<T>,
    ) -> Result<(), ShtError> {
        let paired = self.fft.inverse_paired_series(
            first_input,
            second_input,
            first_output,
            second_output,
            fft_workspace,
        )?;
        #[cfg(test)]
        if paired {
            PAIRED_INVERSE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
        } else {
            PAIRED_INVERSE_DECLINES.fetch_add(1, Ordering::Relaxed);
        }
        if paired {
            return Ok(());
        }
        self.fft
            .inverse_series(first_input, first_output, fft_workspace)?;
        self.fft
            .inverse_series(second_input, second_output, fft_workspace)
    }

    pub(super) fn forward_analysis_rows_paired(
        &self,
        input: &[T],
        output: &mut [Complex<T>],
        output_modes: usize,
        serial_workspace: &mut RealFftWorkspace<T>,
        #[cfg(feature = "parallel")] worker_workspaces: &mut [RealFftWorkspace<T>],
        #[cfg(feature = "parallel")] execution_capacity: usize,
    ) -> Result<(), ShtError> {
        debug_assert!(self.paired_scalar_odd_forward_fft_supported());
        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        if output_modes == 0 || output_modes > spectrum_len {
            return Err(ShtError::BufferLength {
                buffer: "analysis FFT output modes",
                expected: spectrum_len,
                actual: output_modes,
            });
        }

        let pair_count = ntheta / 2;
        #[cfg(feature = "parallel")]
        {
            let worker_count = execution_capacity
                .min(rayon::current_num_threads())
                .min(worker_workspaces.len())
                .min(pair_count);
            if self.row_fft_parallelism_supported() && worker_count > 1 {
                let pairs_per_group = pair_count.div_ceil(worker_count);
                let rows_per_group =
                    pairs_per_group
                        .checked_mul(2)
                        .ok_or(ShtError::IntegerOverflow {
                            quantity: "paired analysis FFT rows per group",
                        })?;
                let output_chunk_len =
                    rows_per_group
                        .checked_mul(spectrum_len)
                        .ok_or(ShtError::IntegerOverflow {
                            quantity: "paired analysis FFT output chunk length",
                        })?;
                let paired_output_len = pair_count
                    .checked_mul(2)
                    .and_then(|rows| rows.checked_mul(spectrum_len))
                    .ok_or(ShtError::IntegerOverflow {
                        quantity: "paired analysis FFT output length",
                    })?;
                output[..paired_output_len]
                    .par_chunks_mut(output_chunk_len)
                    .zip(worker_workspaces[..worker_count].par_iter_mut())
                    .enumerate()
                    .try_for_each(|(group, (spectra, fft_workspace))| {
                        let first_ring = group * pairs_per_group * 2;
                        for (pair_offset, spectra_pair) in
                            spectra.chunks_mut(2 * spectrum_len).enumerate()
                        {
                            let ring = first_ring + pair_offset * 2;
                            let input_start = ring * nphi;
                            let (first_spectrum, second_spectrum) =
                                spectra_pair.split_at_mut(spectrum_len);
                            self.forward_analysis_row_pair(
                                &input[input_start..input_start + nphi],
                                &input[input_start + nphi..input_start + 2 * nphi],
                                &mut first_spectrum[..output_modes],
                                &mut second_spectrum[..output_modes],
                                fft_workspace,
                            )?;
                        }
                        Ok(())
                    })?;
                if ntheta.is_multiple_of(2) {
                    return Ok(());
                }
                let ring = ntheta - 1;
                let input_start = ring * nphi;
                let spectrum_start = ring * spectrum_len;
                self.fft.forward_truncated(
                    &input[input_start..input_start + nphi],
                    &mut output[spectrum_start..spectrum_start + output_modes],
                    serial_workspace,
                )?;
                return Ok(());
            }
        }

        for ring in (0..pair_count * 2).step_by(2) {
            let input_start = ring * nphi;
            let spectrum_start = ring * spectrum_len;
            let (first_spectrum, second_spectrum) = output
                [spectrum_start..spectrum_start + 2 * spectrum_len]
                .split_at_mut(spectrum_len);
            self.forward_analysis_row_pair(
                &input[input_start..input_start + nphi],
                &input[input_start + nphi..input_start + 2 * nphi],
                &mut first_spectrum[..output_modes],
                &mut second_spectrum[..output_modes],
                serial_workspace,
            )?;
        }
        if !ntheta.is_multiple_of(2) {
            let ring = ntheta - 1;
            let input_start = ring * nphi;
            let spectrum_start = ring * spectrum_len;
            self.fft.forward_truncated(
                &input[input_start..input_start + nphi],
                &mut output[spectrum_start..spectrum_start + output_modes],
                serial_workspace,
            )?;
        }
        Ok(())
    }

    pub(crate) fn forward_analysis_rows(
        &self,
        input: &[T],
        output: &mut [Complex<T>],
        output_modes: usize,
        serial_workspace: &mut RealFftWorkspace<T>,
        #[cfg(feature = "parallel")] worker_workspaces: &mut [RealFftWorkspace<T>],
        #[cfg(feature = "parallel")] execution_capacity: usize,
    ) -> Result<(), ShtError> {
        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        if output_modes == 0 || output_modes > spectrum_len {
            return Err(ShtError::BufferLength {
                buffer: "analysis FFT output modes",
                expected: spectrum_len,
                actual: output_modes,
            });
        }

        #[cfg(feature = "parallel")]
        {
            let worker_count = execution_capacity
                .min(rayon::current_num_threads())
                .min(worker_workspaces.len())
                .min(ntheta);
            if worker_count > 1 {
                let rings_per_group = ntheta.div_ceil(worker_count);
                let output_chunk_len =
                    rings_per_group
                        .checked_mul(spectrum_len)
                        .ok_or(ShtError::IntegerOverflow {
                            quantity: "analysis FFT output chunk length",
                        })?;
                output
                    .par_chunks_mut(output_chunk_len)
                    .zip(worker_workspaces[..worker_count].par_iter_mut())
                    .enumerate()
                    .try_for_each(|(group, (spectra, fft_workspace))| {
                        let first_ring = group * rings_per_group;
                        for (offset, spectrum) in spectra.chunks_mut(spectrum_len).enumerate() {
                            let ring = first_ring + offset;
                            let input_start = ring * nphi;
                            self.fft.forward_truncated(
                                &input[input_start..input_start + nphi],
                                &mut spectrum[..output_modes],
                                fft_workspace,
                            )?;
                        }
                        Ok(())
                    })?;
                return Ok(());
            }
        }

        for ring in 0..ntheta {
            let input_start = ring * nphi;
            let spectrum_start = ring * spectrum_len;
            self.fft.forward_truncated(
                &input[input_start..input_start + nphi],
                &mut output[spectrum_start..spectrum_start + output_modes],
                serial_workspace,
            )?;
        }
        Ok(())
    }

    pub(crate) fn inverse_synthesis_rows(
        &self,
        spectra: &[Complex<T>],
        output: &mut [T],
        serial_workspace: &mut RealFftWorkspace<T>,
        #[cfg(feature = "parallel")] worker_workspaces: &mut [RealFftWorkspace<T>],
        #[cfg(feature = "parallel")] execution_capacity: usize,
    ) -> Result<(), ShtError> {
        let ntheta = self.grid.ntheta();
        let nphi = self.grid.nphi();
        let spectrum_len = self.fft.spectrum_len();
        debug_assert_eq!(spectra.len(), ntheta * spectrum_len);
        debug_assert_eq!(output.len(), ntheta * nphi);

        if self.paired_scalar_odd_fft_supported() {
            let pair_count = ntheta / 2;
            #[cfg(feature = "parallel")]
            {
                let worker_count = execution_capacity
                    .min(rayon::current_num_threads())
                    .min(worker_workspaces.len())
                    .min(pair_count);
                if self.row_fft_parallelism_supported() && worker_count > 1 {
                    let pairs_per_group = pair_count.div_ceil(worker_count);
                    let rows_per_group =
                        pairs_per_group
                            .checked_mul(2)
                            .ok_or(ShtError::IntegerOverflow {
                                quantity: "paired synthesis FFT rows per group",
                            })?;
                    let output_chunk_len =
                        rows_per_group
                            .checked_mul(nphi)
                            .ok_or(ShtError::IntegerOverflow {
                                quantity: "paired synthesis FFT output chunk length",
                            })?;
                    let paired_output_len = pair_count
                        .checked_mul(2)
                        .and_then(|rows| rows.checked_mul(nphi))
                        .ok_or(ShtError::IntegerOverflow {
                            quantity: "paired synthesis FFT output length",
                        })?;
                    output[..paired_output_len]
                        .par_chunks_mut(output_chunk_len)
                        .zip(worker_workspaces[..worker_count].par_iter_mut())
                        .enumerate()
                        .try_for_each(|(group, (output_rows, fft_workspace))| {
                            let first_ring = group * pairs_per_group * 2;
                            for (pair_offset, output_pair) in
                                output_rows.chunks_mut(2 * nphi).enumerate()
                            {
                                let ring = first_ring + pair_offset * 2;
                                let spectrum_start = ring * spectrum_len;
                                let (first_output, second_output) = output_pair.split_at_mut(nphi);
                                self.inverse_synthesis_row_pair(
                                    &spectra[spectrum_start..spectrum_start + spectrum_len],
                                    &spectra[spectrum_start + spectrum_len
                                        ..spectrum_start + 2 * spectrum_len],
                                    first_output,
                                    second_output,
                                    fft_workspace,
                                )?;
                            }
                            Ok(())
                        })?;
                    if ntheta.is_multiple_of(2) {
                        return Ok(());
                    }
                    let ring = ntheta - 1;
                    let spectrum_start = ring * spectrum_len;
                    let output_start = ring * nphi;
                    self.fft.inverse_series(
                        &spectra[spectrum_start..spectrum_start + spectrum_len],
                        &mut output[output_start..output_start + nphi],
                        serial_workspace,
                    )?;
                    return Ok(());
                }
            }

            for ring in (0..pair_count * 2).step_by(2) {
                let spectrum_start = ring * spectrum_len;
                let output_start = ring * nphi;
                let (first_output, second_output) =
                    output[output_start..output_start + 2 * nphi].split_at_mut(nphi);
                self.inverse_synthesis_row_pair(
                    &spectra[spectrum_start..spectrum_start + spectrum_len],
                    &spectra[spectrum_start + spectrum_len..spectrum_start + 2 * spectrum_len],
                    first_output,
                    second_output,
                    serial_workspace,
                )?;
            }
            if !ntheta.is_multiple_of(2) {
                let ring = ntheta - 1;
                let spectrum_start = ring * spectrum_len;
                let output_start = ring * nphi;
                self.fft.inverse_series(
                    &spectra[spectrum_start..spectrum_start + spectrum_len],
                    &mut output[output_start..output_start + nphi],
                    serial_workspace,
                )?;
            }
            return Ok(());
        }

        #[cfg(feature = "parallel")]
        {
            let worker_count = execution_capacity
                .min(rayon::current_num_threads())
                .min(worker_workspaces.len())
                .min(ntheta);
            if self.row_fft_parallelism_supported() && worker_count > 1 {
                let rings_per_group = ntheta.div_ceil(worker_count);
                let output_chunk_len =
                    rings_per_group
                        .checked_mul(nphi)
                        .ok_or(ShtError::IntegerOverflow {
                            quantity: "synthesis FFT output chunk length",
                        })?;
                output
                    .par_chunks_mut(output_chunk_len)
                    .zip(worker_workspaces[..worker_count].par_iter_mut())
                    .enumerate()
                    .try_for_each(|(group, (output_rows, fft_workspace))| {
                        let first_ring = group * rings_per_group;
                        for (offset, output_row) in output_rows.chunks_mut(nphi).enumerate() {
                            let ring = first_ring + offset;
                            let spectrum_start = ring * spectrum_len;
                            self.fft.inverse_series(
                                &spectra[spectrum_start..spectrum_start + spectrum_len],
                                output_row,
                                fft_workspace,
                            )?;
                        }
                        Ok(())
                    })?;
                return Ok(());
            }
        }

        for ring in 0..ntheta {
            let spectrum_start = ring * spectrum_len;
            let output_start = ring * nphi;
            self.fft.inverse_series(
                &spectra[spectrum_start..spectrum_start + spectrum_len],
                &mut output[output_start..output_start + nphi],
                serial_workspace,
            )?;
        }
        Ok(())
    }
}
