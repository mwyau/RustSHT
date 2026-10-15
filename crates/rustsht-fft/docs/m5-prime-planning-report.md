# Restricted prime routes and Bluestein lengths

This report records the standalone FFT M5 route and convolution-length measurements after M4 commit `0cddc591e1a52f22ac0a62836f529c11789b0c45`.

## Decision

The paired direct-prime kernel remains selected for complex radices 73 and 103. Rader remains selected for real f64 N=257 and N=723 when its existing safety checks pass. Bluestein remains the fallback for unsafe Rader inputs. The measured M=512 convolution is selected for complex radix 241 and for the f64 N=723 Bluestein fallback. Real N=241 retains M=484. Real f32 N=723 also retains M=484. The M=512 rule is private and limited to these measured plans. The promotion gate was a median public-call improvement of at least 3%, a paired 95% bootstrap interval above parity, and no stable target-cell slowdown greater than 5%. Alternatives below that threshold were not promoted.

## Measurement conditions

Measurements used an x86-64 CPU, logical one CPU core, Rust 1.99.0, the Cargo bench profile and default portable runtime dispatch. No `target-cpu=native` or custom `RUSTFLAGS` were set. The paired-process runner alternated baseline and candidate order. Each process reports the median of seven timed samples; acceptance comparisons used six process pairs. Logs retain CPU load, binary SHA-256, commands, setup times, workspace sizes and timing samples. The host was shared. The final C2C p=241 and N=723 fallback comparisons had narrow paired intervals; first-call estimates and sub-1% pad differences remain inconclusive. These measurements cover FFT calls, not complete SHT operations.

## Paired direct arithmetic and Bluestein

The table gives the direct-kernel speedup over Bluestein for single and 32-row complex calls. Values above 1 favor direct arithmetic; intervals are paired 95% bootstrap intervals.

| Prime and complete length | Direction and rows | Direct speedup range |
| --- | --- | ---: |
| p=73 | N=73, both directions, 1/32 rows | 1.16–1.42×; every cell interval is above 1 |
| p=73 | N=146, both directions, 1/32 rows | 1.25–1.40×; every cell interval is above 1 |
| p=103 | N=103 forward, 32 rows | 1.10× [1.07, 1.11] |
| p=103 | N=103 inverse, 1/32 rows | 1.01–1.02×; intervals include parity |
| p=103 | N=1442 forward, 1/32 rows | 1.10–1.12×; intervals are above 1 |
| p=103 | N=1442 inverse, 1/32 rows | approximately 1.00×; intervals include parity |

The direct routes are already the production routes. The screen supports keeping them and does not justify a route threshold change. Bluestein increased scratch from 3,504 to 8,432 bytes at p=73 and from 4,944 to 11,888 bytes at p=103.

## Rader and Bluestein

The six-pair comparison covered R2C/C2R at N=257 and N=723, ordinary, sparse, cancellation and broad-dynamic-range inputs, and one/32-row calls. For ordinary, sparse and cancellation inputs, Rader was 2.8–4.9× faster than the forced Bluestein route across the measured cells. The broad-dynamic inputs took the existing guarded fallback. The guard and fallback made those calls about 1.16–1.18× slower than directly selecting Bluestein at N=257 and 1.24–1.30× slower at N=723. The guard was retained; its numerical fallback condition was not changed. Rader adds 21,776 retained plan bytes and 8,448 workspace bytes at N=257. At N=723 it adds 22,400 plan bytes and 7,936 workspace bytes over a real plan that has Bluestein but no Rader plan.

## Bluestein convolution-length screen

The first screen used three alternating pairs of full public real calls, both directions, with one and 32 rows. B241 and B257 used the Bluestein route with Rader disabled for this pad-only comparison. B683 used real N=2049. A separate three-pair C2C screen at N=683 checked the B683 candidates.

| Prime | Default M | Candidate M | Observed result |
| --- | ---: | ---: | --- |
| p=241 | 484 | 512 | 512 gave a 1.23× speedup in forced-Bluestein real N=723 calls. |
| p=257 | 525 | 528 | Candidate was about 6% slower. |
| p=257 | 525 | 560 | Candidate was about 1–3% slower. |
| p=257 | 525 | 1024 | Candidate was about 1.8× slower. |
| p=683 | 1372 | 1386 | Candidate was about 10–12% slower in both real N=2049 and complex N=683 calls. |
| p=683 | 1372 | 1400 | Some cells favored M=1400 by less than 1%; this did not meet the 3% gate and was not confirmed with six pairs. |
| p=683 | 1372 | 2048 | Candidate was about 1.5× slower in both real and complex calls. |

The forced real N=723 screen did not by itself establish a production change. The final comparison used the production Rader plan and exercised its broad-dynamic fallback. At N=723, M=512 gave about a 1.17× speedup in broad-dynamic R2C and C2R calls at both row counts. The ordinary C2R one-row median favored M=512 by 1.051×, but its paired interval crossed parity; the remaining ordinary, sparse, and cancellation cells showed no stable change. The N=257 control does not use the p=241 pad override. Its twelve ordinary, sparse, and cancellation medians were 1.7–3.9% slower in M5 than M4, with several paired intervals below parity. These control differences stayed within the 5% slowdown limit.

## M=512 selection and numerical boundary

The final six-pair complex N=241 public-call comparison favored M=512 in all four direction/row cells:

| Operation | Rows | M=484 time | M=512 time | Speedup and 95% interval |
| --- | ---: | ---: | ---: | ---: |
| C2C forward | 1 | 11.13 µs | 9.45 µs | 1.196× [1.171, 1.222] |
| C2C forward | 32 | 10.76 µs | 8.70 µs | 1.236× [1.228, 1.255] |
| C2C inverse | 1 | 11.12 µs | 9.11 µs | 1.222× [1.216, 1.232] |
| C2C inverse | 32 | 11.19 µs | 9.05 µs | 1.236× [1.213, 1.238] |

M=512 adds 384 retained plan bytes and 800 workspace bytes for C2C N=241. For the f64 real N=723 Rader plan, it adds the same 384 plan bytes and 800 workspace bytes. Plan construction medians were lower by about 1 µs in these comparisons; workspace construction changed by less than 30 ns. A blanket p=241 to M=512 rule failed the maximum-DC inverse check for real N=241. That plan therefore uses M=484. The f64 N=723 Rader fallback passes the extreme-input test with M=512. The f32 real N=723 plan uses M=484 because it does not use the Rader fallback route. Complex p=241 at M=512 passes maximum-DC inverse, high-amplitude impulse-forward, and subnormal impulse/constant-spectrum tests in f32 and f64; the f32 random forward and inverse results also match an independent f64 DFT. An exploratory complex p=241 inverse with near-maximum cancelling bins and small subnormal terms produced an output near `1e289` where the exact normalized sum rounds to zero. The same input failed at M=484 and M=512, so it does not distinguish the pad choices. This is a numerical limitation of the p=241 Bluestein path and is outside the M5 pad-selection gate. First-call samples did not establish a change. C2C N=241 forward first-call medians were 13.10 µs at M=484 and 13.01 µs at M=512; inverse medians were 14.94 and 13.86 µs. The paired intervals crossed parity. Warm medians favored M=512, but those first-call-run intervals also crossed parity; the accepted warm-call evidence is the `bench-native-c2c` public-call comparison above.

## Validation and scope

The standalone crate passed `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and debug and release `cargo test --workspace --all-features`. The new tests check the selected convolution lengths, keep real N=241 and f32 real N=723 at M=484, exercise complex p=241 maximum-DC, high-amplitude and subnormal inputs in both precisions, compare f32 complex p=241 against an independent f64 DFT, and test the N=723 unsafe Rader fallback. Existing direct DFT, RustFFT, extreme-input, subnormal and Rader-route tests passed. No test tolerance or safety predicate changed. No M5 performance rerun against RustFFT, DUCC, FFTW or Admiral was collected. Those libraries remain numerical references in the existing tests; historical cross-library timing results are not used as evidence for this pad change. No public planner API, generalized cost model or hidden constructor benchmarking was added. The [raw paired samples and summaries](../benches/results/m5-prime-planning/) include route-forcing screens, final six-pair measurements, first-call samples and measured plan/workspace payloads. The [review log](review-log.md) records the independent review disposition.
