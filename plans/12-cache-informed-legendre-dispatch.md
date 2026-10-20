# Future plan 12 — Cache-informed Legendre algorithm selection

**Status: FUTURE / NOT ACTIVE.** This is a follow-on investigation after the active
[Plan 11](11-ducc-equivalent-scalar-analysis.md) performance campaign. Do not
start this work, modify dispatch, or add dependencies while Plan 11 is active.
This is deliberately **not** SHTns-style per-plan benchmarking at startup.

## Owner intent and scope

Derive a cheap, cross-platform, hardware-informed **default heuristic** for
choosing a precomputed Legendre table versus on-the-fly recurrence. Calibrate
that heuristic with benchmarks on a **small number of representative hosts,
offline**, then ship a constant-time decision at plan creation. There must be
**no runtime transform timing, synthetic workload, CPU-name/model database,
large hardware inventory, or disk calibration cache** in normal execution.

Discover cache capacities once cheaply, ideally on process/module
initialization if measured overhead is negligible, but **prefer lazy,
thread-safe initialization at the first SHT plan** to avoid penalizing Python
import or unrelated Rust use. A Rust library does not itself have an import
hook. Never repeat hardware queries on every transform, workspace, or plan.
The Python binding should reuse the same Rust result rather than probing
independently.

Start with CPU f64 GL scalar STANDARD analysis and synthesis. Extend to spin-1,
different precisions, oversampled grids, and adjoints only when matched
evidence supports separate policy decisions. Preserve explicit safety and
memory limits and the public API.

## Motivation and present evidence

- Martin Reinecke suggested in [DUCC issue #86, comment
  6294831374](https://github.com/mreineck/ducc/issues/86#issuecomment-6294831374)
  that a useful transition might be where the table approximately fills L2.
  This is a hypothesis, **not** proof of a universal L2 cutoff.
- SHTns can select its fastest algorithm through initialization-time
  measurements ([optimization documentation](https://nschaeff.bitbucket.io/shtns/opt.html)).
  **Do not adopt that runtime tuning overhead.** Its
  `sht_quick_init`/`sht_gauss_fly` options are useful design context, not
  requirements.
- RustSHT's [T64 dispatch study](../reports/scalar-t64-recurrence-transition-2026-10-20.md)
  rejected one universal T64 cutoff: q1 full-order analysis with recurrence
  regressed around T64/T127/T128, whereas scalar synthesis benefited.
  Current production switches **f64 scalar synthesis to recurrence for T64+**;
  scalar analysis retains its operation-specific table policy. Spin-1 is
  independently selected. **Preserve this baseline** until new measurements
  justify changes.
- On the benchmark Ryzen 9 5950X, L2 is 512 KiB/core and L3 is 32 MiB per
  eight-core CCD, *not* one 64 MiB cache shared by all 16 cores. With full
  `mmax=lmax`, minimum GL `ntheta=lmax+1`, and an f64 compact table,
  approximate basis payloads are T62 504 KiB, T63 520 KiB, T64 553 KiB,
  T127 4.03 MiB, T191 13.57 MiB, T248 29.68 MiB and T255 32.13 MiB.
  Crossing L2 at ~T63 and an L3 domain at ~T255 merits testing, but the
  table-to-recurrence switch **itself** also explains part of the observed
  T255/T256 cliff. Neither observation isolates cache misses.

## Phase 1 — Fast cache-size discovery (no benchmark and no CPU identity lookup)

Investigate a very small, optional `CacheTopology` provider returning
`Option` values for cache capacity in **bytes**, level/type, and sharing
information **when reliable**. Prefer platform primitives, conditional
compilation, and at most one small dependency only if it materially reduces
maintenance. Validate actual accuracy and cost on supported systems before
choosing the implementation.

| Platform | Preferred mechanism | Caveats / fallback |
| --- | --- | --- |
| Linux, glibc | `sysconf(_SC_LEVEL2_CACHE_SIZE)` and `_SC_LEVEL3_CACHE_SIZE` if available and valid | The identifiers and accuracy are libc-/architecture-dependent; zero, negative or implausible values mean unknown. |
| Linux, including ARM64 or musl | A **one-time** read of `/sys/devices/system/cpu/cpu*/cache/index*/{level,type,size,shared_cpu_list}`, restricted to available/allowed CPUs when practicable | Works across CPU architectures; sysfs may be missing, sandboxed, masked or virtualized. Prefer topology-bearing sysfs data when aggregate L3 sharing matters. Do not walk every CPU unnecessarily. |
| macOS, Intel and Apple silicon | `sysctlbyname` for `hw.perflevelN.l2cachesize`, `hw.perflevelN.l3cachesize`, `cpusperl2/cpusperl3` where offered; fall back to `hw.l2cachesize`/`hw.l3cachesize` | Apple documents that generic `hw.l*cachesize` values can describe the **least performant** core type on heterogeneous systems. Some architectures have shared L2 and no L3. |
| Windows, x86-64 and ARM64 | `GetLogicalProcessorInformationEx(RelationCache)`; inspect `CACHE_RELATIONSHIP` level, size, type and processor-group sharing | Query once, handle variable-size buffer and processor groups, use allowed affinity where available; unknown on error. |
| x86/x86-64 fallback | Deterministic-cache CPUID leaves or a vetted tiny `raw-cpuid` helper | Fast and avoids CPU brand/model identification, but describes the **executing CPU**, not necessarily the process affinity or cache domains; verify AMD/Intel, hybrid and hypervisor behavior. |

The `cache-size` Rust crate is a **candidate for x86 only**, not a
cross-platform solution; as currently documented, non-x86 returns `None`.
Do not add it until its AMD/Intel handling, licensing, dependency footprint
and alternatives have been reviewed:
https://docs.rs/cache-size/latest/cache_size/.

Sources:
- [glibc `sysconf` cache constants](https://sourceware.org/glibc/manual/2.41/html_node/Constants-for-Sysconf.html)
- [Linux sysfs cache attributes](https://code.googlesource.com/linux/torvalds/linux/+/26a1ccc6c117be8e33e0410fce8c5298b0015b99/Documentation/ABI/testing/sysfs-devices-system-cpu)
- [Apple cache/performance-level properties](https://developer.apple.com/documentation/kernel/1387446-sysctlbyname/determining_system_capabilities)
- [Windows `GetLogicalProcessorInformationEx`](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex)
- [Windows `CACHE_RELATIONSHIP`](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-cache_relationship)

**Cost contract:** one discovery per process (for example Rust `OnceLock`),
no calibration, subprocess, Python package, shell invocation, unbounded sysfs
crawl, or user-visible import delay. Measure first-probe and cached-read cost
on the target platforms, including Python import and first-plan latency.
Select eager Python module initialization **only if demonstrably cheaper and
simpler** than lazy first-plan initialization. Unsupported or unreliable
hardware information must gracefully return `None` and fall back to the
existing production policy; never guess a CPU model from its name.

**Topology:** record that an L2 may be private or shared, an L3 may be shared
by only one core cluster/CCD, and worker threads may span clusters. Do not
sum disjoint L3 caches or assume a table can occupy the whole nominal cache.
Cgroups, affinity changes, hybrid cores, VM reports and processor migration
can make a once-per-process value inexact; use a conservative policy or
existing defaults where the relevant sharing information is uncertain.

## Phase 2 — Estimate the *actual* Legendre table working set

Reuse/refactor, rather than duplicate, the overflow-checked element count in
`src/legendre/scalar_table.rs::supported_layout`. The current table stores
north-hemisphere basis samples for every permitted `(m,l)`; for scalar spin 0:

```text
north_rings = ceil(ntheta / 2)
basis_elements = north_rings * sum_{m=0..mmax}(lmax - m + 1)
basis_bytes = basis_elements * size_of::<T>()
```

Include the modest order-offset metadata and estimate **competing hot arrays
and per-worker scratch** separately. When spin 1 is considered, account for
its `lmax+1` table degree and eligibility rules. `mmax`, `ntheta`, spin,
precision and geometry change the footprint: `lmax` alone is inadequate.

Also model the **working-set distribution**: order-parallel workers may each
touch only a subset of the table, with different `m` workloads; other
schedules may stream or share the table. A whole-table-to-one-L2 comparison
is a useful first feature, **not** automatically the right denominator.
Account for available cache *per sharing domain* and concurrency/affinity
at least approximately.

## Phase 3 — Offline multi-host calibration, not user startup tuning

Run matched **RustSHT table versus RustSHT recurrence** benchmarks on a few
available, distinct cache architectures (ideally at least three: e.g. a
small-private-L2/multi-CCD Zen host; a larger/different L2 or L3 x86 host; and
Apple silicon or another ARM64 host). Add a Windows host when feasible, and
do not invent unavailable access. Report measured cache sizes and affinity.

- Exact/oversampled GL; full and reduced `mmax`; both scalar analysis and
  synthesis; q1 and q8 or appropriate platform thread counts; optionally
  different reuse expectations. Include T40/T56/T62/T63/T64/T72/T85/T96/
  T127/T128/T159/T191/T223/T248/T255/T256 and neighboring transition
  degrees where informative. Focus on representative T72–T720 atmospheric
  uses without neglecting low-degree behavior. Do **not** require a huge
  high-degree campaign to establish the compact-table cutoff.
- Use one source/toolchain and identical input/FFT geometry for each
  table/recurrence pair; force algorithms via test/benchmark-only selection,
  not changes to normal public defaults. Preserve correctness against DUCC,
  analytic modes, extremes, reuse and adjoints.
- Record **isolated Legendre stage** and full SHT latency, cold plan/table
  construction, first call, warmed calls, peak memory, and q1/q8 separately.
  Calibrate 20–100 ms repeated batches with >=30 samples over independent
  interleaved runs under low CPU contention. Document noise/uncertainty.
- Optional Linux `perf` cache-miss/bandwidth counters are useful for
  attribution when permitted, but are **not required** for the production
  heuristic. Never use hardware counters during normal initialization.

Fit a **small, explainable heuristic** or piecewise lookup expressed in terms
of `basis_bytes / effective_L2_bytes`, `basis_bytes / effective_LLC_bytes`,
threading/sharing, operation, and possibly reuse. Prefer a few conservative
regions or calibrated constants to a large CPU-family lookup or an
overfitted regression model. Compare with the existing static dispatch, test
on held-out CPUs/workloads, and report worst-case regressions. Near ties,
prefer less memory and initialization cost. A cache-only model may prove
insufficient; in that case retain the current policy rather than shipping an
unreliable heuristic.

## Phase 4 — Integrate only a validated cheap default

- Keep detection and lookup **outside timed transform calls**; cache once,
  and choose per plan/operation/effective thread count without timing anything.
  Do not make one global cutoff for analysis, synthesis or spin-1.
- Respect existing table eligibility, memory caps, overflow checks,
  exceptional-input safety, checked recurrence fallback, and estimated call
  reuse. An inferred cache size must **never override numerical safety**.
- On unknown/untrustworthy cache or affinity, use the measured existing
  policy (currently T64+ synthesis recurrence; analysis and spin-1 remain
  operation-specific). Avoid changing public API for this task; introduce a
  documented override only if concrete benchmark/testing needs justify it.
- Unit-test mock cache hierarchies, mixed-affinity/multi-CCD hosts, missing
  L3, incorrect hypervisor values and unknown results. Verify Linux/macOS/
  Windows builds if practical, plus no-default-features and numerical tests.
  A one-process probe should be race-free and not leak native handles.
- Measure first plan creation and Python import before/after. **No SHTns-style
  benchmark at startup or import under any default setting.**

## Future deliverables and decision rule

1. Short cross-platform probe evaluation: correctness, OS/architecture
   coverage, syscall/CPUID cost, portability and fallback.
2. Reproducible offline benchmark CSV and report containing machine/cache
   attributes, table bytes, call counts, latency distributions and chosen
   boundary by operation/threads. No need to commit raw host diagnostics.
3. A transparent, unit-tested default heuristic with before/after
   performance on development and held-out hosts, **only if actually better**
   than the current dispatch; otherwise a report rejecting cache-based
   automatic selection is a valid outcome.
4. An explicit recorded non-goal: runtime autotuning, cache-size estimation
   by memory-latency microbenchmarks, CPU-brand databases, or automatic
   calibration files.

This plan is a **future TODO**, not permission to interrupt Plan 11, restart
the T64 experiments, or add an expensive initialization-time autotuner.
