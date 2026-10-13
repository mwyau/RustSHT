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
        3 => {
            let (branch0, tail) = output.split_at_mut(pass.l1);
            let (branch1, branch2) = tail.split_at_mut(pass.l1);
            let two = scalar::<T>(2.0);
            let taur = scalar::<T>(-0.5);
            let taui = scalar::<T>(0.866_025_403_784_438_6);
            let (groups, _) = input.as_chunks::<3>();
            for (k, src) in groups.iter().enumerate() {
                let tr2 = two * src[1];
                let cr2 = src[0] + taur * tr2;
                let ci3 = two * taui * src[2];
                branch0[k] = (src[0] + tr2) * scale;
                branch1[k] = (cr2 - ci3) * scale;
                branch2[k] = (cr2 + ci3) * scale;
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
        5 => {
            let (branch0, tail) = output.split_at_mut(pass.l1);
            let (branch1, tail) = tail.split_at_mut(pass.l1);
            let (branch2, tail) = tail.split_at_mut(pass.l1);
            let (branch3, branch4) = tail.split_at_mut(pass.l1);
            let two = scalar::<T>(2.0);
            let tr11 = scalar::<T>(0.309_016_994_374_947_45);
            let ti11 = scalar::<T>(0.951_056_516_295_153_5);
            let tr12 = scalar::<T>(-0.809_016_994_374_947_5);
            let ti12 = scalar::<T>(0.587_785_252_292_473_1);
            let (groups, _) = input.as_chunks::<5>();
            for (k, src) in groups.iter().enumerate() {
                let ti5 = two * src[2];
                let ti4 = two * src[4];
                let tr2 = two * src[1];
                let tr3 = two * src[3];
                let cr2 = src[0] + tr11 * tr2 + tr12 * tr3;
                let cr3 = src[0] + tr12 * tr2 + tr11 * tr3;
                let ci5 = ti11 * ti5 + ti12 * ti4;
                let ci4 = ti12 * ti5 - ti11 * ti4;
                branch0[k] = (src[0] + tr2 + tr3) * scale;
                branch1[k] = (cr2 - ci5) * scale;
                branch2[k] = (cr3 - ci4) * scale;
                branch3[k] = (cr3 + ci4) * scale;
                branch4[k] = (cr2 + ci5) * scale;
            }
        }
        _ => super::inverse_general_prime(pass, input, output, scale),
    }
}
