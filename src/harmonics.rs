// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use num_complex::Complex;
use rustfft::num_traits::Float;

use crate::error::ShtError;

/// Bandlimit and spin for a packed set of real-field harmonic coefficients.
///
/// Coefficients are stored by increasing order `m`, with degrees `l=m..=lmax`
/// within each order. The coefficient at `(l,m)` is at [`Self::index`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HarmonicSpec {
    lmax: usize,
    mmax: usize,
    spin: usize,
    order_offsets: Vec<usize>,
    nalm: usize,
}

impl HarmonicSpec {
    /// Constructs a packed layout for inclusive degree/order limits and spin.
    ///
    /// # Errors
    ///
    /// Returns an error when `mmax > lmax`, `spin > lmax`, the packed size
    /// overflows `usize`, or allocation of order offsets fails.
    pub fn new(lmax: usize, mmax: usize, spin: usize) -> Result<Self, ShtError> {
        if mmax > lmax {
            return Err(ShtError::InvalidHarmonicBounds { lmax, mmax });
        }
        if spin > lmax {
            return Err(ShtError::SpinExceedsLmax { spin, lmax });
        }

        let nalm = packed_len(lmax, mmax)?;
        let order_count = mmax.checked_add(1).ok_or(ShtError::IntegerOverflow {
            quantity: "harmonic order count",
        })?;
        let mut order_offsets = Vec::new();
        order_offsets
            .try_reserve_exact(order_count)
            .map_err(|_| ShtError::AllocationFailed {
                array: "harmonic order-offset",
            })?;

        let mut offset = 0usize;
        for m in 0..=mmax {
            order_offsets.push(offset);
            let order_len = lmax - m + 1;
            offset = offset
                .checked_add(order_len)
                .ok_or(ShtError::IntegerOverflow {
                    quantity: "packed harmonic offset",
                })?;
        }
        debug_assert_eq!(offset, nalm);

        Ok(Self {
            lmax,
            mmax,
            spin,
            order_offsets,
            nalm,
        })
    }

    /// Constructs a scalar (`spin=0`) harmonic layout.
    pub fn scalar(lmax: usize, mmax: usize) -> Result<Self, ShtError> {
        Self::new(lmax, mmax, 0)
    }

    /// Returns the maximum degree.
    #[must_use]
    pub const fn lmax(&self) -> usize {
        self.lmax
    }

    /// Returns the maximum nonnegative order.
    #[must_use]
    pub const fn mmax(&self) -> usize {
        self.mmax
    }

    /// Returns the nonnegative spin weight.
    #[must_use]
    pub const fn spin(&self) -> usize {
        self.spin
    }

    /// Returns the number of packed `m >= 0` coefficients.
    #[must_use]
    pub const fn nalm(&self) -> usize {
        self.nalm
    }

    /// Returns the packed index of `(l,m)`, or `None` outside this layout.
    #[must_use]
    pub fn index(&self, l: usize, m: usize) -> Option<usize> {
        if m > self.mmax || l < m || l > self.lmax {
            return None;
        }
        self.order_offsets[m].checked_add(l - m)
    }

    /// Checks the packed length, real `m=0` coefficients, and spin-forbidden modes.
    ///
    /// # Errors
    ///
    /// Returns an error for a length mismatch, a nonreal `m=0` value, or a
    /// nonzero coefficient with `l < spin`.
    pub fn validate_coefficients<T: Float>(&self, alm: &[Complex<T>]) -> Result<(), ShtError> {
        if alm.len() != self.nalm {
            return Err(ShtError::CoefficientCount {
                expected: self.nalm,
                actual: alm.len(),
            });
        }

        for l in 0..=self.lmax {
            let value = alm[self.order_offsets[0] + l];
            if value.im != T::zero() {
                return Err(ShtError::NonRealMZero { degree: l });
            }
        }

        for l in 0..self.spin {
            for m in 0..=l.min(self.mmax) {
                let index = self.order_offsets[m] + (l - m);
                let value = alm[index];
                if value.re != T::zero() || value.im != T::zero() {
                    return Err(ShtError::NonzeroBelowSpin {
                        spin: self.spin,
                        degree: l,
                        order: m,
                    });
                }
            }
        }
        Ok(())
    }
}

fn packed_len(lmax: usize, mmax: usize) -> Result<usize, ShtError> {
    if mmax == 0 {
        return lmax.checked_add(1).ok_or(ShtError::IntegerOverflow {
            quantity: "packed harmonic length",
        });
    }

    let order_count = mmax.checked_add(1).ok_or(ShtError::IntegerOverflow {
        quantity: "harmonic order count",
    })?;
    let first_order_len = lmax.checked_add(1).ok_or(ShtError::IntegerOverflow {
        quantity: "packed harmonic length",
    })?;
    let last_order_len = (lmax - mmax)
        .checked_add(1)
        .ok_or(ShtError::IntegerOverflow {
            quantity: "packed harmonic length",
        })?;
    let order_len_sum =
        first_order_len
            .checked_add(last_order_len)
            .ok_or(ShtError::IntegerOverflow {
                quantity: "packed harmonic length",
            })?;

    let (factor_a, factor_b) = if order_count % 2 == 0 {
        (order_count / 2, order_len_sum)
    } else {
        (order_count, order_len_sum / 2)
    };
    factor_a
        .checked_mul(factor_b)
        .ok_or(ShtError::IntegerOverflow {
            quantity: "packed harmonic length",
        })
}

#[cfg(test)]
mod tests;
