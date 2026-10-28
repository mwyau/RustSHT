# Executed AVX2 code comparison

The disassembly came from the exact Rust parent `061a0467` and pinned DUCC `42ce8ab5`, built with x86-64-v3/AVX2/FMA. Symbol addresses are from those local binaries; they are useful for mapping the original local disassembly and are not stable across rebuilds.

## DUCC `alm2map_kernel`

The executed body at the `alm2map_kernel` specialization is a packed four-lane `double` loop unrolled across four recurrence pairs. It carries the paired recurrence and accumulation in YMM registers and uses packed FMA instructions for the recurrence and four coefficient accumulators. The loop is unit stride, stores the updated recurrence and accumulators, and branches only at the loop boundary. The body has 13 static `vfmadd*pd` instructions in the inspected binary and no function calls. It follows a prefix routine that maintains scaled state, transitions through `iter_to_ieee`, applies the residual scale once in `calc_alm2map`, and enters this bulk suffix.

## Rust scaled f32 fallback

The executed `multiply_component_scaled_with_normal_factor_f32` helper begins with a scalar `vmulss`, then extracts and checks exponent fields, tests the adjusted exponent range, rebuilds the result bits, and branches to a larger split/scale exceptional path when the normal fast case is unavailable. A complex contribution invokes this component logic for both real and imaginary parts. This source path is called millions of times in the measured fallback and accounts for the largest disjoint source attribution.

This is instruction-structure evidence, not a cycle estimate. Full machine-specific disassembly remains in local untracked captures.
