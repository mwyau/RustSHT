// SPDX-FileCopyrightText: 2010-2026 Max-Planck-Society (DUCC FFT pass equations)
// SPDX-FileCopyrightText: 2019 Peter Bell (DUCC FFT pass equations)
// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause
//
// Terminal real inverse radix equations are adapted from DUCC fft1d_impl.h
// at c1ed01b8fda69d2a378190f0915f1a19762dddbf.

use num_traits::{Float, FromPrimitive};

use super::{Pass, scalar};

#[inline(never)]
pub(super) fn execute<T: Float + FromPrimitive>(
    pass: &Pass<T>,
    input: &[T],
    output: &mut [T],
    scale: T,
) {
    debug_assert_eq!(pass.ido, 1);
    match pass.radix {
        2 => {
            let (first, second) = output.split_at_mut(pass.l1);
            let (pairs, _) = input.as_chunks::<2>();
            for (k, src) in pairs.iter().enumerate() {
                first[k] = (src[0] + src[1]) * scale;
                second[k] = (src[0] - src[1]) * scale;
            }
        }
        4 => {
            let (branch0, tail) = output.split_at_mut(pass.l1);
            let (branch1, tail) = tail.split_at_mut(pass.l1);
            let (branch2, branch3) = tail.split_at_mut(pass.l1);
            let two = scalar::<T>(2.0);
            let (groups, _) = input.as_chunks::<4>();
            for (k, src) in groups.iter().enumerate() {
                let tr2 = src[0] + src[3];
                let tr1 = src[0] - src[3];
                let tr3 = two * src[1];
                let tr4 = two * src[2];
                branch0[k] = (tr2 + tr3) * scale;
                branch1[k] = (tr1 - tr4) * scale;
                branch2[k] = (tr2 - tr3) * scale;
                branch3[k] = (tr1 + tr4) * scale;
            }
        }
        _ => unreachable!("real multipass plan contains only radix 2 and 4"),
    }
}
