# Current-source f32 scalar synthesis profile — 2026-10-27

This profile measures exact production source `3189fe8ee49f125d0d4569549fb880a6b6468bb8` on GL T720 full-mmax f32 scalar synthesis, spin 0, q8 (`721 × 1458`, `phi0=0.23`). It uses the deterministic `alm0` fixture from `scripts/benchmark_ducc_f32_native.py` (SHA-256 `03cc74b6eb5508e6c1c85f427ac183ece8177d7b36a31aefc3bfe4ece62beb96`). The measured output is finite and its SHA-256 `260e35799c6f8d3c505fc9d6f2cc5bdbd5443e03d4e75e4d488044904066d45d` matches the earlier exact fixture output.

## Profile

Callgrind 3.26.0 collected **8,366,416,331 Ir** for one warmed `synthesis_into` call. The worker prepared its plan/workspace and ran repeat and poisoned-output reset checks before collection. Rust 1.99.1 / LLVM 23.1.1, `--locked --release --features parallel`, release debug info level 2, `-C target-cpu=x86-64-v3`; Pulp selected the V3/AVX2 kernel. The process used a fixed eight-physical-core affinity on an AMD Ryzen 9 5950X; exact host CPU IDs are omitted here. Callgrind cache and branch simulation were disabled; instruction addresses and jumps were collected. The f32 reference is pinned DUCC 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`; DUCC was not profiled in this specific Callgrind run.

Disjoint source attribution shows the f32 paired fallback iterator called 3,755 times and accounted for 4.193B inclusive Ir (50.12% of the whole call, including its children). The callback itself accounted for 1.443B Ir (17.24%). In that callback, 92,811,832 contribution iterations took the exponent-zero branch; 7,891,996 nonzero-exponent contributions called `multiply_complex_scaled_f32`, with 1.657B inclusive Ir across those calls. A separate scalar fallback call site invoked the same helper 12,587,348 times and accounted for 3.067B inclusive Ir. Across both sites, the helper was called 20,479,344 times. These helper totals are nested under their respective traversal contexts and must not be added to the traversal inclusive total.

The generic AVX2 fused block was entered 1,744 times and accounted for 161.3M inclusive Ir (1.93%). Its emitted V3 loop contains YMM `vfmadd*ps` operations for coefficient formation, recurrence and two-component accumulation, plus vector range comparisons. This confirms that the fused SIMD leaf is present and executes, while the paired degree/latitude callback and scalar fallback scaling still account for substantial work. Callgrind Ir are instruction references, not elapsed time or cycles; the inclusive Rayon wait rows are not algorithm work.

## Interpretation and follow-up

The trace showed the shared f32 fused leaf executes, but only in initially all-normal blocks. The later exact-parent route census and lane-level profile found that disjoint safe spans cover only 12.9% of mixed full-mmax contribution visits, so a mixed-lane or per-vector retry was not pursued. The measured schedule change is documented in the [scoped task-grain report](../f32-scoped-task-grain-2026-10-27/README.md); it leaves the recurrence and exceptional fallback unchanged.

## Reproduction and evidence

- [`profile-summary.json`](profile-summary.json): compact counts, source/compiler/build provenance and hashes.
- [`callgrind-harness.rs`](callgrind-harness.rs) and [`callgrind-shim.c`](callgrind-shim.c): exact temporary harness and client-request shim.
- [`fused-v3-assembly-summary.md`](fused-v3-assembly-summary.md): curated path-free AVX2/FMA observations from the executed V3 leaf; Callgrind records 1,744 calls to this symbol.
- [`checksums.sha256`](checksums.sha256): evidence checksums.

The two initial invalid profile attempts used the wrong Valgrind launch nesting and were discarded. The retained profile pins the worker before Valgrind starts, so Callgrind wraps the actual worker.
