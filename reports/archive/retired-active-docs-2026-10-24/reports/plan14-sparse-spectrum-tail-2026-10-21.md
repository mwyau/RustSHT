# Plan 14: sparse spectrum-tail initialization for q8 synthesis

**Implementation:** `1951e8337f239f5278ccb3eb23819134c41eb62e`  
**Baseline:** `2661dbb6d4ac3fab2b295c6376e1ed50c65ca948`  
**DUCC:** 0.42.0, `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`

## Change

The parallel reflected-order synthesis kernel writes each ring's Fourier
orders from zero through the northern ring's polar order limit. The old
caller zeroed the full ring-major spectrum before dispatch. The new q8 route
instead zeros each row's contiguous suffix beginning immediately after that
active prefix. This initializes polar-excluded orders and modes above `mmax`
without clearing cells the order jobs overwrite.

The top-level eager clear was removed. The scalar table path still clears its
own destination, serial ordered synthesis still clears before computing, and
the general recurrence receives a full zeroed spectrum when selected. If a
fast path declines after writing, it clears the complete output before
fallback. The suffix limit is taken from the canonical northern ring for
both hemispheres, matching the order kernel's reflected-ring ownership even
if supplied southern limits differ.

A stale-buffer test begins with nonzero spectrum cells, verifies that the
q8 path fully overwrites its output, checks reflected polar-tail limits, and
confirms that a declined extreme-input path clears partial results before
fallback.

## Matched whole-transform results

The native benchmark reused plans and workspaces and timed complete f64 GL
synthesis calls. It used the same deterministic inputs, runtime Pulp V3
(AVX2/FMA, four f64 lanes), and pinned DUCC C++ build. Each capture used
three interleaved Rust/DUCC process pairs and ten calibrated approximately
25 ms batches per process. Setup and first-call timings were recorded outside
the warmed batches. Baseline and candidate captures used separate release
build directories.

| Case | Source | Rust median ms (capture range) | DUCC median ms | Rust/DUCC | Excess ms | Timed batches per backend |
|---|---|---:|---:|---:|---:|---:|
| T720, `mmax=180`, `nphi=361`, q8 | Baseline | 1.5553 (1.5433–1.5625) | 1.2583 | 1.231x | 0.2920 | 90 |
|  | Candidate | 1.4765 (1.4692–1.4839) | 1.2577 | 1.174x | 0.2188 | 60 |
| T720, full `mmax=720`, `nphi=1458`, q8 | Baseline | 2.9213 (2.9149–2.9277) | 2.7133 | 1.077x | 0.2080 | 60 |
|  | Candidate | 2.8197 (2.8169–2.8225) | 2.7092 | 1.041x | 0.1104 | 60 |

Across the process-median captures, Rust fell by 78.7 µs (5.1%) for reduced
`mmax` and 101.6 µs (3.5%) for full `mmax`. The Rust/DUCC excess fell by
73.2 µs and 97.5 µs respectively. The paired batch-ratio p10/median/p90 was
`1.202/1.236/1.277` → `1.155/1.173/1.207` for reduced order, and
`1.040/1.076/1.104` → `1.019/1.041/1.071` for full order. Relative L2 error
was unchanged at `5.194e-14` and `4.405e-14`; output reset checks passed.

Q8 captures had median sampled runqueue wait below 1% and CPU frequency
medians around 4.4 GHz. The full-order Rust batch p10–p90 ranges overlap
slightly, but both candidate process medians were below both baseline process
medians. The reduced-order batch distributions separated in these captures.

## Stage attribution and q1 control

One test-only 30-batch phase capture measured baseline latitude setup plus
spectrum clear at 70.0 µs. On the candidate source, latitude setup was
14.4 µs and sparse tail initialization 1.7 µs. This diagnostic shows the
expected reduction in initialization work; its phase probe had a broad
whole-call tail, so the matched uninstrumented transform batches above are
the performance basis. The recurrence and inverse-row phases were not
consistently faster in the phase probe.

Single-capture q1 controls were noisy: median runqueue wait was 3.5–7.2%,
with p90 at 6.9–15.6%. Reduced-order Rust was essentially unchanged
(10.353 → 10.348 ms), while the full-order candidate was 19.912 ms versus
20.854 ms baseline; p10–p90 ranges overlap and DUCC also shifted between
captures. No q1 improvement is claimed.

## Reproduction and validation

Rust used 1.99.1 / LLVM 23.1.1, strict FP, and
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`. DUCC used GCC 15.2 with SHT
flags `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the driver used
`-O3 -fno-fast-math -ffp-contract=fast -march=x86-64-v3`. The candidate was
built from the exact source diff later committed as `1951e833`; its Git base
reported `2661dbb6` with a dirty worktree during capture. No unrelated source
changes were present. Batch arrays, fixture hashes, process distributions,
first-call/setup data, dispatch records and host observations are in
[`summary.json`](data/plan14-sparse-spectrum-tail-2026-10-21/summary.json).

Reproduction uses the commands in
[`benchmark_ducc_full_transform_parity.py`](../scripts/benchmark_ducc_full_transform_parity.py)
with `--lmax 720 --mmax 180 --fixed-ntheta 721 --fixed-nphi 361
--operation synthesis --threads 8`, and with `--mmax 720 --fixed-nphi 1458`
for the full-order control. Use separate target directories for baseline and
candidate, pinned DUCC `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`, and repeat
three process pairs per capture.

Release tests passed with and without the `parallel` feature; formatting and
strict Clippy passed. The full suites retain existing analytic, adjoint,
spin, all-grid, f32/f64, subnormal and extreme-finite coverage.

The remaining q8 gap is about 0.219 ms for the reduced odd case and 0.110 ms
for full-order even sampling. Continue Plan 14 by re-ranking the current
recurrence and odd inverse-row costs; do not attribute the remaining gap to
spectrum initialization or claim DUCC parity from this change.

## Expanded validation and current-source comparison

The sparse initializer was rechecked at the integrated production source
`1951e833`. Test-only observations now distinguish a late decline after
earlier orders have completed from a decline before any output exists.

- The same prepared q8 workspace was reused three times with different
  coefficients and a poisoned nonzero spectrum for both `ntheta=33` and
  `ntheta=34`; the matching odd/even longitude lengths were 33 and 36.
  Complete spectra matched the ring-group reference within `4e-13`, including
  polar-excluded tails.
- At T300, a subnormal coefficient in order 100 declined after earlier
  normal orders had written nonzero spectra. A test-only flag confirmed the
  partial output existed before the production clear. The direct route then
  left an all-zero spectrum for generic fallback. Public q8 output matched
  q1 and the ring-group spectrum reference within `4e-13`.
- Finite `0.75*f64::MAX` monopole and quadrupole coefficients declined the
  ordered path after transformed-coefficient checks. The q8 fallback remained
  finite and matched the independent analytic map after coefficient scaling
  to `<5e-13`; q1/q8 maps agreed within `5e-13`.

The full-transform comparison below uses the integrated source, f64 scalar GL
synthesis, the same deterministic inputs on each backend, Rust 1.99.1 / LLVM
23.1.1 and DUCC 0.42.0 at `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.
Rust's runtime backend was Pulp V3 AVX2/FMA (four f64 lanes). DUCC used GCC
15.2 and `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`; the driver
kept strict floating-point validation flags. Each case has three interleaved
process pairs, ten calibrated ~25 ms batches per pair and backend, with setup
and first-call timings outside the warmed batches. The table gives warmed
per-call p10/median/p90, paired ratio p10/p90, full-call excess, relative L2,
and first-call Rust/DUCC latency in milliseconds.

| mmax / nphi / q | Rust p10 / median / p90 | DUCC p10 / median / p90 | Rust/DUCC [paired p10, p90] | Excess | Relative L2 | First call Rust/DUCC |
|---|---:|---:|---:|---:|---:|---:|
| 180 / 361 / 1 | 10.270 / 10.301 / 10.374 | 8.562 / 8.663 / 8.863 | 1.189 [1.166, 1.209] | +1.638 | 5.19e-14 | 10.284 / 9.740 |
| 180 / 361 / 8 | 1.463 / 1.477 / 1.509 | 1.255 / 1.267 / 1.276 | 1.166 [1.155, 1.191] | +0.210 | 5.19e-14 | 1.733 / 2.714 |
| 180 / 378 / 1 | 8.806 / 9.260 / 9.303 | 8.266 / 8.724 / 8.874 | 1.061 [0.994, 1.121] | +0.536 | 4.71e-14 | 9.224 / 9.190 |
| 180 / 378 / 8 | 1.243 / 1.263 / 1.276 | 1.209 / 1.220 / 1.229 | 1.035 [1.019, 1.043] | +0.042 | 4.71e-14 | 1.571 / 2.752 |
| 720 / 1441 / 1 | 27.903 / 28.096 / 29.502 | 27.812 / 27.977 / 28.316 | 1.004 [0.990, 1.058] | +0.119 | 4.45e-14 | 29.570 / 31.613 |
| 720 / 1441 / 8 | 3.895 / 3.961 / 4.195 | 3.882 / 3.904 / 3.933 | 1.015 [0.995, 1.084] | +0.058 | 4.45e-14 | 5.342 / 7.975 |
| 720 / 1458 / 1 | 19.679 / 19.896 / 20.046 | 18.801 / 19.061 / 19.958 | 1.044 [0.986, 1.057] | +0.835 | 4.41e-14 | 21.599 / 22.794 |
| 720 / 1458 / 8 | 2.787 / 2.838 / 2.946 | 2.689 / 2.711 / 2.770 | 1.047 [1.023, 1.078] | +0.127 | 4.41e-14 | 3.526 / 6.736 |

The maximum absolute Rust/DUCC output error was `3.743e-10`; all reset
consistency checks passed. Setup and fixture-load timings are recorded
separately in the machine-readable rows. The reduced-odd capture had
capture-wide median sampled runqueue wait of `0.30%` and p90 `15.2%`; its q8
paired batch ratio distribution remains separated from unity. Full-order odd q8 and the
q1 even/odd controls have paired ranges that cross unity, so they are not
claimed as reliable regressions. The complete raw per-process arrays, host
frequency/wait summaries, first-call/setup costs and numerical rows are in
[`current-source results`](data/plan14-sparse-spectrum-current-2026-10-21/results.json)
and the four underlying captures.

The optimized benchmark example was built from production source at
`1951e833`. The only later Rust source edits in the worktree were
`cfg(test)` validation observations and assertions; they are not compiled
into the release example. Each capture's metadata records the dirty test-only
worktree state.

### Matched q8 recurrence and inverse-row profile

At T720, `mmax=180`, `ntheta=721`, `nphi=361`, Rust and DUCC were timed on
the same 114,211-coefficient fixture (`phi0=0.23`; SHA-256
`639321e6a923c1a668b16609d061e624706df540538230a510b6110f9c0dbc83`).
Three interleaved process pairs each collected 30 calibrated ~30 ms batches.
Rust timers cover Legendre plus spectrum production and inverse FFT plus map
writeback. Temporary timers in the pinned DUCC copy cover `alm2leg` and
`leg2map`. Values below are p10/median/p90 over the 90 per-batch averages.

| Stage | Rust | DUCC | Median excess |
|---|---:|---:|---:|
| Recurrence and spectrum / `alm2leg` | 1.173 / 1.185 / 1.204 ms | 1.057 / 1.072 / 1.081 ms | +0.112 ms |
| Inverse FFT and writeback / `leg2map` | 0.275 / 0.286 / 0.313 ms | 0.162 / 0.165 / 0.167 ms | +0.121 ms |
| Whole call | 1.467 / 1.486 / 1.529 ms | 1.238 / 1.255 / 1.266 ms | +0.231 ms |

The stage totals account for the whole-call excess within timer and boundary
variation. They rank two material components rather than identifying one
specific instruction-level defect. Rust's sparse tail initialization was
`1.62 µs` median (`1.54–1.72 µs` p10–p90); removing the old full spectrum
clear is not responsible for the remaining ~`0.23 ms`. DUCC's `leg2map`
uses packed real pocketfft, while Rust's odd-length route uses its existing
complex inverse path. The previously measured Rust C2R alternative was
8.8–10.0% slower than production and is not reconsidered here. No recurrence
change is justified by a new source-level mechanism. Keep `1951e833`, record
these two remaining stage costs, and move to Plan 14 closeout rather than
adding another unvalidated kernel or FFT route.

Reproduction: run the four T720 commands with `--lmax 720`,
`--fixed-ntheta 721`, `--operation synthesis`, and `--threads 1 --threads 8`;
use `(mmax,nphi)=(180,361),(180,378),(720,1441),(720,1458)`, with the
first command building and the next using `--skip-build`. The exact commands,
all 30-batch process data, and stage batches are linked from
[`current-source results`](data/plan14-sparse-spectrum-current-2026-10-21/results.json)
and [`stage_profile.json`](data/plan14-sparse-spectrum-current-2026-10-21/stage_profile.json).
