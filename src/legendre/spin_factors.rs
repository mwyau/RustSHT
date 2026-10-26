// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use rustfft::num_traits::Float;

use crate::error::ShtError;
use crate::legendre::{LegendrePrecision, ScaledValue};

/// Precision-generic state prepared once for one spin/order recurrence.
pub(crate) struct SpinInitial<T> {
    pub(crate) start_degree: usize,
    pub(crate) cos_power: usize,
    pub(crate) sin_power: usize,
    pub(crate) plus_sign: T,
    pub(crate) minus_sign: T,
    pub(crate) prefactor: ScaledValue<T>,
}

/// Fills the order-dependent spin coefficients shared by the f32 and f64
/// native recurrences.
pub(crate) fn fill_spin_factors<T: LegendrePrecision>(
    spin: usize,
    m: usize,
    lmax: usize,
    alpha: &mut [T],
    coefficient_a: &mut [T],
    coefficient_b: &mut [T],
) -> Result<SpinInitial<T>, ShtError> {
    debug_assert!(spin > 0 && spin <= lmax && m <= lmax);
    debug_assert!(alpha.len() > lmax + 1);
    debug_assert!(coefficient_a.len() > lmax + 1);
    debug_assert!(coefficient_b.len() > lmax + 1);

    let mlo = spin.min(m);
    let mhi = spin.max(m);
    let one = T::one();
    let two = one + one;
    alpha.fill(T::zero());
    coefficient_a.fill(T::zero());
    coefficient_b.fill(T::zero());
    alpha[mhi] = one;

    for l in mhi..=lmax {
        let lf = T::from_usize(l).expect("degree is representable in f32 and f64");
        let mf = T::from_usize(m).expect("order is representable in f32 and f64");
        let sf = T::from_usize(spin).expect("spin is representable in f32 and f64");
        let inv_product = ((lf + mf + one).recip()
            * (lf - mf + one).recip()
            * (lf + sf + one).recip()
            * (lf - sf + one).recip())
        .sqrt();
        let leading = (lf + one) * (two * lf + one) * inv_product;
        let coupling = if l == 0 {
            T::zero()
        } else {
            mf * sf / (lf * (lf + one))
        };

        if l > mhi {
            let ratio = (((lf + mf) * (lf - mf) * (lf + sf) * (lf - sf))
                / ((lf + mf + one) * (lf - mf + one) * (lf + sf + one) * (lf - sf + one)))
                .sqrt()
                * ((lf + one) / lf);
            alpha[l + 1] = alpha[l - 1] * ratio;
        } else {
            alpha[l + 1] = one;
        }

        let a = leading * alpha[l] / alpha[l + 1];
        coefficient_a[l + 1] = a;
        coefficient_b[l + 1] = coupling * a;
    }

    let mut prefactor = ScaledValue {
        mantissa: one,
        exponent: 0,
    };
    let binomial_lower = mhi - mlo;
    for i in 1..=binomial_lower {
        let numerator = mhi
            .checked_add(mlo)
            .and_then(|value| value.checked_add(i))
            .ok_or(ShtError::IntegerOverflow {
                quantity: "spin recurrence binomial factor",
            })?;
        let numerator = T::from_usize(numerator).expect("spin factor is representable");
        let denominator = T::from_usize(i).expect("spin factor is representable");
        prefactor = normalize_prefactor(
            prefactor.mantissa * (numerator / denominator).sqrt(),
            prefactor.exponent,
        )?;
    }

    let (cos_power, sin_power, plus_sign, minus_sign) = if mhi == m {
        (
            mhi.checked_add(spin).ok_or(ShtError::IntegerOverflow {
                quantity: "spin recurrence half-angle power",
            })?,
            mhi - spin,
            parity_sign::<T>(m),
            parity_sign::<T>(mhi - spin),
        )
    } else {
        let combined = mhi.checked_add(m).ok_or(ShtError::IntegerOverflow {
            quantity: "spin recurrence half-angle power",
        })?;
        (
            combined,
            mhi - m,
            parity_sign::<T>(spin),
            parity_sign::<T>(combined),
        )
    };

    Ok(SpinInitial {
        start_degree: mhi,
        cos_power,
        sin_power,
        plus_sign,
        minus_sign,
        prefactor,
    })
}

#[inline]
fn parity_sign<T: Float>(value: usize) -> T {
    if value.is_multiple_of(2) {
        T::one()
    } else {
        -T::one()
    }
}

fn normalize_prefactor<T: LegendrePrecision>(
    mut value: T,
    mut exponent: i64,
) -> Result<ScaledValue<T>, ShtError> {
    while value.abs() > T::BIG {
        value = value * T::SCALE_DOWN;
        exponent = exponent
            .checked_add(T::SCALE_EXPONENT)
            .ok_or(ShtError::IntegerOverflow {
                quantity: "spin recurrence exponent",
            })?;
    }
    while value != T::zero() && value.abs() < T::SMALL {
        value = value * T::SCALE_UP;
        exponent = exponent
            .checked_sub(T::SCALE_EXPONENT)
            .ok_or(ShtError::IntegerOverflow {
                quantity: "spin recurrence exponent",
            })?;
    }
    Ok(ScaledValue {
        mantissa: value,
        exponent,
    })
}
