// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

use num_complex::Complex;
use num_traits::{Float, FromPrimitive};

use crate::simd::SimdPass;

use super::{ComplexPass, ComplexPassKernel, Direction, PassExecution};

#[inline(never)]
#[cold]
pub(super) fn execute_avx2<T: Float + FromPrimitive>(
    input: &[Complex<T>],
    output: &mut [Complex<T>],
    pass: &ComplexPass<T>,
    execution: &mut PassExecution<'_, T>,
    kernel: Option<ComplexPassKernel<T>>,
) -> bool {
    let Some(kernel) = kernel else {
        return false;
    };
    let inverse = execution.direction == Direction::Inverse;
    if execution.blue_plan.is_some()
        || pass.radix != 11
        || pass.ido != 1
        || pass.l1 < 2
        || (inverse && !execution.defer_inverse_normalization)
    {
        return false;
    }

    let vector_l1 = pass.l1 - pass.l1 % 2;
    let simd_pass = SimdPass {
        radix: pass.radix,
        l1: pass.l1,
        ido: pass.ido,
        twiddles: &pass.twiddles,
        inverse,
        inverse_scale: if inverse {
            execution.inverse_output_scale
        } else {
            T::one()
        },
    };
    // SAFETY: runtime dispatch checked AVX2, the plan constructs the radix-11
    // ido-one pass, the buffers are disjoint, and unsafe inverse inputs fall
    // back to the scalar path above.
    unsafe { kernel(input, output, &simd_pass) };

    let radix_root_stride = execution.total_len / pass.radix;
    let output_scale = execution.inverse_output_scale;
    for outer in vector_l1..pass.l1 {
        for (branch, slot) in execution
            .radix_input
            .iter_mut()
            .enumerate()
            .take(pass.radix)
        {
            *slot = input[branch + pass.radix * outer];
        }
        execution.transform::<11>(11, radix_root_stride);
        for (branch, &transformed) in execution.radix_output.iter().enumerate().take(pass.radix) {
            let mut value = transformed;
            if inverse {
                value = value * output_scale;
            }
            output[outer + pass.l1 * branch] = value;
        }
    }

    #[cfg(test)]
    crate::m0_record_route(format!(
        "c2c-avx2:radix11-ido1 n={} l1={} vector_l1={} scalar_l1_tail={}",
        execution.total_len,
        pass.l1,
        vector_l1,
        pass.l1 - vector_l1
    ));
    true
}
