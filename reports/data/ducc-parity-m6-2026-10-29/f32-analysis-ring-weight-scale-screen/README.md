# f32 scalar-analysis ring-weight scale screen

## Decision

Reject this optimization. Caching the typed `ring_weight / nphi` value
preserved output bit-for-bit and reduced q1 full Callgrind instruction
references by 1.08%, but it produced no repeatable whole-call improvement in
three of the four cells. Reduced q1 improved by about 1%, which is too small
to address the remaining gap and did not justify expanding the candidate.

The reusable f32-I/O/f64-analysis route remains an isolated diagnostic. It
still misses DUCC in every cell. Stage D is next: target the f32 spin-1
synthesis gap using a measured f64-internal or dual-channel reflected route
through the existing hemisphere scheduler, with q1/q8 route counters.

## Provenance

- Rust source parent: `1f90f71f7be0effa04455076b3243f92444c3bf5`.
- The comparison parent applies the existing shared f32-I/O/f64-analysis
  patch `0b7785c1ca43bed3f5acf233cd56ab0ec4bf9315d04f6842f2b74f646af24844`.
- Candidate full patch SHA-256:
  `e7996b4539b33052f7b01d146feb3dd46aa677b0af7c24ff3d5b82a70df2fd57`.
  It adds a typed per-ring scale vector to scalar plans and passes the already
  rounded quotient into the shared scalar-analysis preparation. It adds no
  recurrence implementation.
- DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b` (0.42.0), rebuilt for this
  experiment.
- Rust: rustc 1.99.1 / LLVM 23.1.1, `--locked --release --features parallel`,
  `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`.
- DUCC: GCC 15.2 with `-O3 -DNDEBUG -ffast-math -ffp-contract=fast
  -march=x86-64-v3`; the harness source uses `-fno-fast-math`.
- Host: AMD Ryzen 9 5950X, AVX2/FMA; q1 CPU `[0]`, q8 CPUs
  `[0,2,4,6,8,10,12,14]`. Each GL T720 spin-0 analysis cell used three
  interleaved fresh-process parent/candidate/DUCC comparisons and a 25 ms
  warmed-call target. Public calls included the native-f32 row FFT, f64
  recurrence, conversions and output writeback. DUCC used native-f32 I/O.
- Fixtures and SHA-256 values match the Stage B catalog:
  [`fixtures-sha256.json`](../f32-io-f64-internal-stage-b/fixtures-sha256.json).
- `parent.patch.gz` and `candidate.patch.gz` reconstruct the two Rust sources
  from the source parent. `results-3.json.gz`, `results-3.tsv`, binary hashes,
  benchmark runner, DUCC harness, and Callgrind profiles are retained here.

## Whole-call screen

Times are medians in milliseconds. Parent/candidate bootstrap intervals use
only three paired samples and are directional screens, not final acceptance
precision. Candidate output was bitwise identical to its comparison parent in
all cells (`relative-L2 = 0`).

| Cell | Parent | Candidate | DUCC f32 | Candidate/parent [95% CI] | Candidate/DUCC [95% CI] | Status |
|---|---:|---:|---:|---:|---:|---|
| Full q1 | 16.218 | 16.256 | 13.945 | 1.003 [0.996, 1.005] | 1.159 [1.157, 1.169] | GAP; neutral |
| Reduced q1 | 6.438 | 6.374 | 5.719 | 0.990 [0.986, 0.992] | 1.112 [1.107, 1.122] | GAP; about 1% faster |
| Full q8 | 3.000 | 3.010 | 2.484 | 1.003 [0.992, 1.003] | 1.211 [1.097, 1.251] | GAP; neutral |
| Reduced q8 | 1.235 | 1.235 | 0.994 | 1.000 [0.998, 1.027] | 1.243 [1.195, 1.248] | GAP; neutral |

Candidate/DUCC maximum relative-L2 error remained `1.34e-7` reduced and
`1.46e-7` full, matching the comparison parent's output. The only
candidate/parent timing interval that excluded 1.0 was reduced q1, at about a
1% gain; the other cells were neutral. No 15-pair extension was warranted.

## Callgrind and source evidence

The q1 full run collected eight public analysis calls. The comparison parent
recorded `2,442,717,071` Ir; the candidate recorded `2,416,247,615` Ir, a
`1.08%` reduction. Under Callgrind both binaries took approximately `776 ms`
per call, so the instruction reduction did not improve whole-call runtime.
Ir are instruction references, not cycles.

The source confirms the candidate computes each quotient once at plan creation
using the same typed `T::from_f64(weight) / T::from_usize(nphi)` operations as
before. The exact output equality checks the rounding-preserving claim. The
small Ir reduction and absent whole-call gain show that this repeated
normalization is not the measured parity blocker.

## Validation actually run

- `cargo fmt --all` — passed.
- `cargo check --locked --release --features parallel` — passed.
- Focused typed-scale test for f32/f64 GL, CC, and F1 plans — passed.
- The four 3-pair public-call screens passed finite, repeat, and workspace
  reset checks. Candidate output matched parent bit-for-bit and had the same
  DUCC error as the preceding f64-analysis route.
- No broad exceptional-value or adjacent f64 performance matrix was run; the
  performance screen rejected promotion before those gates.
