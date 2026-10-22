# Luna handoff — independent f64 algorithm review

The [technical review](f64-ducc-independent-review-2026-10-22.md) contains the
source crosswalk, measured findings, failed-experiment reconciliation and
constrained implementation sequence. [Evidence and reproduction](data/f64-ducc-independent-review-2026-10-22/README.md)
are committed separately from production source. The review implements no
optimization. Owner's later steering prioritizes f64 spin-1 source parity;
Plan 17 f32 remains paused.

**Highest-value new evidence**

- Pinned DUCC actually switches STANDARD spin-1 at T500. At T720/T1023 CC,
  converted scalar work handles every nonpolar ring; the polar pair uses
  direct spin. DUCC evaluates its two converted channels separately over
  reflected pairs. Rust shares their recurrence over both hemispheres.
- Parity separation alone does not lower the approximate vector FMA count:
  the rejected two-channel form retains about 10 FMAs/vector/pair and spills;
  the rejected one-channel form adds recurrence work. The new mechanism to
  test is active reflected-pair processing plus checked scaled handling in
  an order-owned adapter to the existing scalar ordered engine.
- Current T1023 full q8 spin-1 diagnostic worker sums put 205 ms in checked
  scalar fallback versus 88 ms in fused attempt/accumulation. Eight workers
  are active. Classify transition success/decline/final visitor by weighted
  lane-degree work before changing a kernel. These overlapping sums are
  diagnostics, not wall-time claims.
- Scalar CC q8 is different: its resampler is serial; at T720 it takes
  about 26.5 ms while recurrence takes 4.4 ms. Scalar synthesis uses the
  older ring-group route, and only four workers ran in the inspected q8
  probes. Keep the 256-pair analysis tile; task grain is a separate issue.
- GL table analysis uses its AVX2 accumulator only when `ntheta % 4 == 0`.
  This explains the T127/T159/T191 wins amid slower neighboring table cells.
  Retain 20 MiB provisionally; stratify route dispatch and cold/reuse costs
  before choosing policy. Do not add another exact-degree exception.

**CC campaign state**

At the review checkpoint no CC sweep runner or worker was alive. Thirteen
of fourteen local shards have saved JSON under
`rustsht-cc-full-sweep-2026-10-22/shards/` in the local temporary root. The final
`high-2049-3072/run.log` stops at case 41/48; no result JSON exists. Its
forty preceding progress lines do not preserve the numerical/timing records.
Exit reason is unknown. No sweep was restarted during this review.

The final process check at 14:36 Eastern found an unrelated
`sht_bench` full-secondary source runner active. It was left
untouched. No new transform experiments were started during final handoff.

Saved totals: 618 geometries, 2,472 operation/thread groups, 7,416 random
comparisons, 1,236 validation records. These cover all integer degrees
T0–256 and sampled degrees through T2048. The requested floor-quarter
manifest actually requires 632 geometries / 2,528 groups: 72 requested
groups are absent, and 16 saved T2/T3 m=1 groups are extra. The missing groups
are the final 48 plus T1/T2/T3 m=0 with both longitude shapes and A/S q1/q8.
The existing generator uses `max(1,T/4)`, so its own manifest misses this.

Preserve the frozen native workers. Expected SHA-256:

- Rust scalar: `26203773054c1e0a37e015d435cac269a59b3e599ea31e988dae6917a2c7da3a`.
- DUCC scalar: `3df1d4408407015c65a85e2909aa34219871bf29a75126b23e497bcbb396c89a`.

Production/harness revisions are `3f838847` / `cd8beaa3`; DUCC is pinned
`42ce8ab5`. The old handoff's last-shard command remains a valid frozen-binary
reproduction once the host is quiescent. Add low m=0 cases with existing
`--fixed-mmax 0 --lmax 1 --lmax 2 --lmax 3 --grid CC --dense-sweep` selectors.
Reconcile independent coverage before calling the campaign complete.
The harness currently records most numerical errors without enforcing a
tolerance; zero failure records do not establish numerical acceptance.

**Next safe sequence**

1. Read current owner instructions and synchronize safely. Preserve all
   prior untracked work and frozen results.
2. Repair coverage/acceptance reporting and incremental result persistence;
   complete missing frozen-source cells when appropriate. Do not rerun the
   complete low-degree sweep.
3. Follow the review's spin-1 frequency probe and one-order adapter experiment
   before a full-transform A/B. Preserve polar, subnormal, extreme-value,
   adjoint and partial-write fallback behavior.
4. Test scalar CC's one-group scheduling grain separately; evaluate reuse of
   its existing ordered scalar engine and bounded parallel resampler as
   distinct changes. Keep spin-1 resampling and FFT implementation intact.
5. Reassess GL policy with dispatch-stratified full/near-full order, true cold
   plans and reuse 1/2/4/8/16/32/64; retain the current cap until supported.

Validation: production all-feature release workspace tests, true serial
package-only release tests, formatting and strict Clippy passed. Ten
independent analytic native configurations and T499/T500/T511 spin-1 maps
passed targeted checks. No Python binding changed. All diagnostic processes
started for this review have finished; local diagnostic copies, fixtures and
raw assembly remain under temporary-root `rustsht-cc-independent-*`,
`ducc-cc-independent-stage-20261022/` and
`rustsht-independent-disasm/`. No implementation or new research phase
should automatically resume after the owner's quota reset.
