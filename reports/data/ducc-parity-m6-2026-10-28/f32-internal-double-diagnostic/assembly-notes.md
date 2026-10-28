# AVX2 assembly notes — diagnostic binaries

Exact symbols were extracted from the matching release binaries built at source `102675a171f59faeab93fece11817ab589bbacc8` and pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

- `ducc0::detail_sht::detail_sht_inner_loop::calc_alm2map` begins by calling `iter_to_ieee` once. Its scale/correction checks use packed `vcmpltpd`, `vcmplepd`, `vtestpd`, `vblendvpd`, and packed multiply/add operations. On the profiled random fixture its counted scaled-prefix loop iterations are zero; it proceeds to the suffix call.
- `alm2map_kernel` uses YMM packed doubles (four latitude lanes) and several `vfmadd*pd` operations in the unrolled recurrence/accumulation body. The main loop stores recurrence state contiguously and calls no helper. The symbol is 0x2e2 bytes in the matched binary.
- `rustsht::legendre::scalar::multiply_component_scaled_with_normal_factor_f32` is 0x3c4 bytes and performs scalar `vmulss` plus repeated exponent-bit masking/shifting and range branches. The final fallback block calls the compiler runtime `__powisf2`. This is the executed scalar component path attributed by the prior exact-source Callgrind profile.

These instructions explain the work structure; they do not imply a cycle ratio. The worker-level timing comparison remains pending an isolated host window.
