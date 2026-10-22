# Independent review — advisory, subordinate to owner instructions

Inspected `opt/sht-performance-parity` at `7b1629d4` on 2026-10-22 under the
owner's explicit independent-review authorization. Production code is unchanged
by this review. The current active directive and later owner instructions take
precedence over the implementation suggestions below.

The [f64 algorithm review](reports/f64-ducc-independent-review-2026-10-22.md)
and [compact Luna handoff](reports/handoff-f64-independent-review-2026-10-22.md)
contain source references, numerical controls and reproducible evidence.

- **Spin-1:** DUCC's actual STANDARD T>=500 path converts ordinary rings to two
  scalar streams over reflected pairs, with direct spin at the polar pair.
  Rust already uses scalar conversion; the remaining large-band path evaluates
  hemispheres independently and spends substantial worker time in checked
  scalar fallbacks. The rejected parity-only and loop-reversal experiments
  remain rejected. Investigate active reflected pairs and scaled transition
  behavior as a new mechanism, preserving original-coefficient fallback.
- **Scalar CC:** thirteen saved native shards establish synthesis gaps in their
  covered cells. The final high shard is stopped without saved JSON. The
  independent floor-quarter manifest has 72 requested groups missing and
  16 extra saved groups. Do not label the sweep complete. Scalar analysis's
  serial resampling and scalar synthesis's four-worker q8 underfill are
  separate from spin-1's already parallel resampler and order queue.
- **GL:** the table's AVX2 accumulator is selected only for `ntheta % 4 == 0`.
  This explains much of the nonmonotonic forced-route behavior. Treat 20 MiB
  as a provisional resource guard pending dispatch-stratified cold/reuse
  evidence; do not encode another arbitrary cutoff.
- **Evidence:** the native harness records many numerical errors without
  enforcing tolerance gates. Zero recorded failures is not proof that every
  numerical acceptance condition passed. Reconcile saved metrics explicitly.

Production all-feature release tests, package-only serial release tests,
formatting and strict Clippy passed in this review. Native analytic controls
and pinned-DUCC spin-1 T499/T500/T511 comparisons passed targeted validation.
No production optimization, f32 optimization, FFT change or full sweep was
started. Keep production scalar-analysis tiles at 256 reflected pairs.

The earlier stale `spharm.` test references were fixed in `a317ec46`; that
finding is resolved. Owner-approved pyspharm integration and the guarded paired
odd-longitude FFT route remain intentional. This review does not authorize
moving/removing either integration, changing exact CC limits, publication,
or resuming paused f32 work.
