# Native-f32 spin-1 analysis adaptive/preweighted composition screen

This isolated candidate combines the measured adaptive reflected-ring schedule
(32-pair scale-sensitive prefix, 256-pair suffix) with the existing
scale-epoch channel-preweighting experiment. Its base is
`053c5668079fa89f37438a488b2545b5ceab1e69`, whose `src/` tree matches the
adaptive-only screen base `00f9770741a6e3e84646217d51374e35515daaa1`. The
combined source diff SHA-256 is
`d83f1442fa7cc7b2c27880df12b352cb974394b71962e83aa21bde1ce0979ab9`; its
recoverable compressed patch is `candidate.patch.gz`.

The screen used three interleaved pairs on Ryzen 9 5950X, GL T720, full M720
and reduced M180, q1/q8 pinned to CPUs 0 and 0–7. Rust used
`-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; the matched f64 worker was built
from this exact candidate source and used widened copies of the same f32
fixtures only for the diagnostic comparison. DUCC was native f32 at pinned
revision `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

| Cell | f32 ms | matched f64 ms | f32/f64 | DUCC ms | f32/DUCC |
|---|---:|---:|---:|---:|---:|
| Full q1 | 143.254 | 37.919 | 3.758x | 29.371 | 4.833x |
| Full q8 | 19.225 | 6.694 | 2.893x | 4.771 | 4.030x |
| Reduced q1 | 46.930 | 15.002 | 3.118x | 11.871 | 3.938x |
| Reduced q8 | 6.431 | 2.239 | 2.873x | 1.754 | 3.660x |

All four cells miss the interim 2.0x f64 target. This composition was slower
than the separately screened adaptive-only candidate on both full-grid cells
and approximately even on reduced cells; the comparison is exploratory across
separate three-pair runs, not a paired candidate-versus-parent claim. This
does not justify more tile-size tuning or a 15-pair screen. The bounded result
leaves scale-sensitive recurrence/accumulation work as the B3 blocker. Random
fixture relative L2 versus DUCC was `1.847e-5` full and `2.171e-5` reduced;
independent accuracy for this combined source remains OPEN.

Validation passed: format check, release library check, strict library
Clippy, 51 nonignored `spin1_` library tests, the dedicated scaled-seed replay,
and 14 f32 integration tests. Five spin unit tests and one f32 integration
test were ignored by their existing opt-in annotations. No production code was
promoted. Exact pair records, fixture and worker hashes are in
`screen-three-pair.json`; `SHA256SUMS` covers the package artifacts.
