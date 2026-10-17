# M2 visitor inspection

## Scope

The isolated source copy is clean production HEAD `a50f70051049e37fa71898d5ec0aecc56ff388b3`, extracted under `target/m6-m2/source`. This note records one candidate mechanism before any source edit. The shared worktree has not been changed.

## Observed work

In `src/transform.rs::synthesize_ring_group`, the general paired scalar synthesis visitor computes the even and odd coefficient factors for a degree pair, then loops over latitude lanes. Each lane calls `multiply_complex_scaled` for the even term and `multiply_complex_scaled_product` for the odd term. Those helpers repeat coefficient-component, basis, product-range, and exponent checks for each lane. The visitor API `for_each_y_lm_paired_dispatched` discards the `normal_range` status already computed by `for_each_y_lm_paired_status_dispatched`. When that status is true, every lane in the visit has exponent zero and a zero-or-normal-range basis value. A conservative coefficient/product guard can therefore classify the ordinary branch once for the degree pair and block, instead of repeating coefficient checks in both robust helpers for every lane. Any coefficient or product outside the ordinary range can continue through the existing scaled helpers unchanged.

## Profile support and expected benefit

The committed route profile counts 2,639,324 normal and 541,870 scaled visitor lanes at T511, and 18,174,506 normal and 4,881,166 scaled visitor lanes at T1023 q1. Visitor timer sums are 26.408 ms and 191.910 ms at those limits. This makes repeated per-lane coefficient classification a bounded candidate for the majority normal visitor work. The profile does not split timer cost by normal versus scaled lanes, so the benefit is an estimate; the initial whole-transform screen must establish whether it is material. The candidate leaves coefficient preparation, recurrence updates, order of degree accumulation, and the robust scaled fallback in place. It does not reuse precomputed coefficients across blocks (the rejected P02 change).

## Fast-path range bound

For f64, the normal recurrence interval has binary exponents `[-400, 400]` (`SMALL` and `BIG`). A nonzero coefficient component is eligible only if its exponent combined with the entire basis interval, and for the odd term with the block's full cosine exponent interval, remains within `[-1021, 1020]` at every multiply in the existing order. This leaves headroom from the f64 normal limits `[-1022, 1023]`; the odd term checks both `coefficient * basis` and `(coefficient * basis) * cosine`. Zero coefficient components are exact-zero cases. Nonfinite, subnormal, zero cosine with a nonzero odd component, and out-of-bound values decline the fast path. For each lane, the helper verifies that each calculated intermediate is normal or an exact zero from a zero operand. It then forms the same even-plus-odd sum and accepts only a normal or exact-zero result. A subnormal sum, overflow, or any failed check uses the unchanged scaled helpers. Boundary tests cover both coefficient exponent edges, odd-term cosine bounds, subnormal products, overflow, and cancellation to subnormal or exact zero.
