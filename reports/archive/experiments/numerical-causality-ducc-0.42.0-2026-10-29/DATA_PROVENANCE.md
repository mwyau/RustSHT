# Data and build provenance

## Source identities

| Component | Identity |
|---|---|
| RustSHT isolated candidate base | b6a96aa7161a64f678b614d73478496f50f0551e |
| RustSHT isolated native-f32 candidate commit | 45380d7e4bd849a33ffef7d91978ee18fd7e4193 |
| RustSHT sparse trace source tree (single-mode capture) | 9e6f800d6102de2ca87a8d6c86ac3feaeef59cb7 |
| RustSHT dense trace source tree (candidate plus dense trace hook) | a21a4eb9a9792bf6e21044418ba2ec236dd90c3b |
| Pinned DUCC source | 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b |
| DUCC trace-instrumented source tree | 1bcaa09ba03edb584beec10d2ec94a93ea14ad49 |
| Native-f32 candidate patch SHA-256 | 626c3013f4bcade70371c29caf44ebdab85073ae4c12fe2419aa6c933af14aeb |
| Exact executed Rust sparse trace patch SHA-256 | 3adb24ae7668ce7b896e15bf4e9edf2fa6902523ea447ee176f0547fb45a46d8 |
| Packaged portable Rust sparse trace patch SHA-256 | 5f00ba475a9ded19fdbe3d8c20083e9815746b21e853996acfd618d36d499a5e |
| Exact executed Rust dense trace patch SHA-256 | a94b67af6d21a14e2826068da6e93bbae625745c3222e6fa19acde33077ff4a6 |
| Packaged portable Rust dense trace patch SHA-256 | cfba2c29340583aefe60efa8ebd9661f083b6faf54a118797c4fd85ddd9877a1 |
| DUCC test-only trace patch SHA-256 | 289165ee3318654a6c5094ef59d9662009561f13714edec6b9c1c9a6bfaa6f6a |
| Rust sparse-trace test binary SHA-256 | 0aecdda7267cdf3e2f0cb501c3fa7fb04400283832533e6b79589596702b0063 |
| Rust dense-trace test binary SHA-256 | 930db2b5ce9faf1752299d49a8bd8d423d0c120735528923bc856eb8cd1878a9 |
| DUCC mode-trace executable SHA-256 | af456fe772d4546090b5d8644cbc657bbf58da55419f92cf0eff139fe4785429 |
| DUCC dense-trace executable SHA-256 | c544453576a7a230c3ad8b560b02977388593397a53dfb81db6270d69d2484c1 |
| Binary128 dense-reference executable SHA-256 | cad539ec3ff0ca5677cf25c769f674d86cec972f8ec7da8ff9945bc32432723f |
| Long-double dense-reference executable SHA-256 | 9896361108cd974c16e6324a9ef5bb58c17cdf9a1fe08857f4e90d908b3f6a7a |

The packaged patch byte hashes are repeated in the SHA256SUMS manifest. The candidate patch reconstructs the isolated generic source; the following Rust patch adds only ignored trace observation code/tests. DUCC instrumentation is a separate patch against the pinned source. For repository hygiene, the two packaged Rust trace patches use relative default output filenames instead of the local scratch paths present in the exact executed patch; all recorded runs supplied explicit output paths through environment variables, so the numerical code and data flow are unchanged. The exact executed sparse/dense patch hashes and source-tree identities are listed above as capture provenance; the packaged patches are the portable replay versions.

## Build and execution environment

- Host: AMD Ryzen 9 5950X; AVX2 and FMA available.
- Rust: rustc 1.99.1 (58cf8f9c3 2026-10-13), Cargo 1.99.1 (5f94df478 2026-08-27), release test build, RUSTFLAGS=-C target-cpu=x86-64-v3 -Aunexpected_cfgs.
- Rust direct generic trace: eight f32 lanes; one worker, no thread scheduler.
- DUCC: GCC 15.2.0; Release; -march=x86-64-v3; -ffp-contract=fast -fexcess-precision=fast -fno-math-errno -fno-signed-zeros -fno-trapping-math -fassociative-math -freciprocal-math; trace macro enabled; four f64 lanes; direct trace uses one ring/job.
- Binary128 reference: GCC __float128 / libquadmath, 113-bit significand. The earlier sparse/public-map long-double cross-check maximum output delta was 2.7804e-14; the dense high-band independent cross-check maximum was 1.2543818651e-16.
- No transform timing was collected from instrumented binaries.

## Fixture

data/gl-t720-theta.bits contains 721 lowercase hexadecimal f64 bit patterns, one per line. The values come from the Rust GL T720 grid and are passed unchanged to DUCC. Its SHA-256 is 1f16212ea7ef50ccaf9a52b552f9a822fb7052da1e8b164efdd858bdfd2a6f03.

data/fixtures.json defines GL T720, nphi=34, phi0=0, spin=0, unit real and unit imaginary coefficient bits, tested orders/degrees/rings, and the matched case count. Binary128 reference construction is implemented in tracers/quad_trace.c.

## Raw table hashes and record counts

For CSV datasets, “records” excludes the header/trace-version line. Raw hashes are SHA-256 of the uncompressed bytes before gzip packaging. The committed gzip bytes and every other package file are independently covered by SHA256SUMS.

| Data file | Records | Uncompressed/raw SHA-256 |
|---|---:|---|
| accum-stage-comparison.csv.gz | 357 | 28f4468403ec1a929b1189a71c2c8ccb440c341a265d1acba3449c7dce245ece |
| controlled-v2.csv.gz | 3,358 | 5123304009651029fc433cc67bd81136a59f179ddaf85feef9976cab29613289 |
| ducc-actual.csv.gz | 31,905 | 8d66452de4cdcec203d6bb62f039585d0197fa6d31cfdea538c6e3b461c74309 |
| ducc-modes.csv.gz | 259,826 | bbe451d311f0859963a5f87cb4040315960a789cfb0829067ebb38bf6a821e91 |
| ducc-public-map.csv.gz | 384 | f07e74e3d1968bc083dcf5618d6fd9fbb3a2e3528bb7805e61260faf00e520a4 |
| ducc-zonal.csv.gz | 2,903 | 76b49bafc4ca99ffb86a2d8f9117e6e0fe53c9cf5311e85293b64f4396838973 |
| first-divergence-selected.csv.gz | 357 | dcd5dfed6d3b0bb718de1299805e19e87e9c47d52e8d3b1d8eb149e2f4261e21 |
| longdouble-public-map.csv.gz | 13,089 | f7f1bf6e9f29e18b960cb1a9bf584dbaab550eb78323070ddc8e553b38c67e2e |
| mode-comparison.csv.gz | 357 | 372ee51c2e8261c80dfb0fa013990d0adcaed09e84de160e5d2a9ed147e0aa83 |
| quad-recurrence.csv.gz | 30,128 | bf37e63cfb47e0ad9d4c38d701e565405e3e457509027f9eaa456cadc2336d38 |
| quad-selected-traces.csv.gz | 7,174 | fd86e51fdebb8aa5c6ca110628f4c67bdcb91f6148bd3147870eafbec597a2b9 |
| recurrence-growth.csv.gz | 14,581 | b8243e034bf92dc43d4f6eb654f1cf99c0032bd3dc704096edead0a3cd9df7ba |
| rust-low-order.csv.gz | 2,158 | 6f5947ca9e9ec5ce8fa6895a112039ac3a9c09a20351941ad5788a98f88b74b8 |
| rust-modes.csv.gz | 151,900 | abb2829cfafb0a59b12da959b26549dbfc0b472417b360647078db83ca6f454c |
| rust-native-f32.csv.gz | 17,360 | 00e3ce309df391cff815bfa110b9abd49f11455b9cc865f75e35c7321506b66c |
| rust-public-map.csv.gz | 385 | b6f4a464eb5464b616a8323a4c52b788db7b3ff2fbcf0da3e1e74185169c161b |
| rust-zonal-f32.csv.gz | 1,448 | 12e980a8e968dbf3652d31dd3d4d6ca4c349a349e4f60041e99209bbe17153ce |

Additional independent reference outputs are in longdouble-map-comparison.json.gz; public-map and stage summaries are in public-map-comparison.json.gz, map-summary.json.gz, and stage-summary.json. The gzip files are retained to keep this package compact; the uncompressed fixture is small and directly consumable.

## Dense upper-band trace provenance

`data/high-band-alm0.f32` is the 16-by-121 complex f32 band extracted from the archived full fixture. The raw source fixture SHA-256 is `03cc74b6eb5508e6c1c85f427ac183ece8177d7b36a31aefc3bfe4ece62beb96`; the extracted file SHA-256 is `bb40e9dc604d336095a364e11c10303ec90de8547aa721176101278fce386a8d`. The trace uses exact theta bits from `gl-t720-theta.bits` (SHA-256 above), north representative rings 0/1/5/360 and reflected outputs 720/719/715/360, 16 orders, and 256 real/imaginary pre-FFT output components.

Raw dense traces are gzip-compressed under `data/dense-band/`. Hashes below refer to the uncompressed CSV bytes; compressed file hashes are in `SHA256SUMS`.

| Raw trace | Lines | Uncompressed SHA-256 |
|---|---:|---|
| rust-f32-trace.csv.gz | 41,081 | 74e695a16acea55d0e0bbcfdde2b8f76756b12413888a34f27c027fee376c41e |
| ducc-f64-trace.csv.gz | 25,524 | e7e19c04dce943090dd2d1f8a7dd4368c14cf39a43a6e7e48dcc24b258011883 |
| quad128-reference.csv.gz | 40,969 | 878d77ba72acc6e1bddbe90ead017828a0d50bebc4e4af0e36aa0ad6d0fa6954 |
| longdouble-reference.csv.gz | 64 | 5651e1acb10443886b57969e610567f7a1643489ad705f98e591ee51a045f1b2 |

The derived recurrence table has 22,768 matched updates; the prepared-pair table has 3,936 rows; the accumulation table has 256 rows; the first-divergence and prefix tables each have 64 rows. The independent binary128/long-double comparison has 256 component rows and zero correctly-rounded f32 bit disagreements. All derived tables and their deterministic comparators are retained with compressed-byte checksums.

The long-double tracer uses host x86 extended precision (64-bit significand) and independently evaluates normalization, factors, recurrence, coefficient pairing, accumulation, and reflected reconstruction. Its 256 final f32 component bits match the binary128 reference; its maximum decimal output difference from binary128 is 1.2543818651e-16. DUCC's narrowed output bits also match binary128 for all 256 components. No instrumented executable was used for timing.

## Interpretation boundaries

The 357 pre-FFT cases use isolated coefficients, not random dense spectra. The 385-cell public map table is also a set of single-coefficient inputs. The controlled substitutions are diagnostic replays; their deltas are not additive. The selected reference matrix does not set an application tolerance. No full/reduced random T720 benchmark was run, and no performance claim is made.

## Seed-only and seed/cos² replay follow-up

`tracers/seed_only_replay.py` uses an exact rational-to-binary32 round-to-nearest-even routine for `(-1)^m * f32(sin(theta))^m * f32(sectoral_norm)`. The routine's corrected-seed bits match all 357 entries in the archived `first-divergence-selected.csv.gz`; its sectoral normalization bits match 48 recorded plan rows, and its actual seed values match 40 overlapping sparse/dense cases. It replays the captured dense native-f32 `a`, `b`, `cos²` operands with libc `fmaf`, then uses the same f32 pairs/cosine with 100-digit Decimal accumulation to isolate recurrence-state sensitivity. This is not the runtime accumulation or final map.

`tracers/seed_cos2_interaction.py` additionally consumes the saved f32 trace from `../cos2-plan-time-control-2026-10-30/data/dense-high-band/rust-native-f32-dense-trace.csv.gz`. That candidate is based on Rust source `45380d7e4bd849a33ffef7d91978ee18fd7e4193`; its exact combined candidate/test patch SHA-256 is `87d02ad7262c90123123d48e0ac63909704330e691ccbf7f22473097c57fef8c`. The candidate changes only the plan-time f64 `cos²` value narrowed to f32. The interaction replay validates unchanged sectoral seed, sine, cosine, recurrence `a/b`, and prepared pairs before combining it with the exact-product seed. Captured trace hashes and output counts are recorded in `data/dense-band/seed-only-summary.json` and `seed-cos2-interaction-summary.json`; every compressed output is covered by `SHA256SUMS`.

Both replay commands are deterministic: the new gzip artifacts have a fixed zero timestamp, and rerunning into a separate destination reproduced every packaged output byte-for-byte on Python `3.14.4`. The actual `fmaf` execution is verified against all 22,752 original dense recurrence updates. Mixed-path Decimal accumulation is diagnostic only; no FFT, map, runtime cost, or science-tolerance claim is implied.
