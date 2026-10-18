# P01-only q8 synthesis route profiles

The T1023 and T2047 GL STANDARD f64 scalar profiles used the retained P01-only
source snapshot, with P02 excluded. Each run warmed once, measured one
uninstrumented synthesis, then one instrumented synthesis. The complete maps
matched bitwise. The committed M1 q8 baseline profiles use the same grid,
thread count, and counters. Route timers are sums of worker CPU time; they are
not transform wall times.

| T | Baseline / P01 instrumented wall time (ms) | Fused CPU ms | Visitor CPU ms | Transition CPU ms | Route timer total change | Active worker max/min, baseline → P01 |
|---:|---:|---:|---:|---:|---:|---:|
| 1023 | 79.954 / 49.815 | 63.043 → 59.419 | 208.757 → 74.094 | 13.748 → 62.979 | 285.548 → 196.492 ms (−31.2%) | 6.93x → 4.09x |
| 2047 | 790.333 / 587.113 | 381.821 → 382.185 | 1567.608 → 1377.664 | 1247.983 → 761.175 | 3197.412 → 2521.024 ms (−21.1%) | 9.53x → 7.02x |

The committed route-unit totals are unchanged: 225,005,258 at T1023 and
1,734,378,064 at T2047. P01 transfers substantial visitor work into its
scalar transition. At T1023 the visitor timer falls by 64.5% while transition
time rises from 13.748 to 62.979 ms. At T2047 visitor time falls by 12.1% and
transition time by 39.0%; fused time is effectively unchanged. This agrees
with the paired scalar whole-transform gains but does not establish an
accepted source change. P01 reduces the largest per-worker timer imbalance but does not remove it. At T1023 worker sums span 9.751–39.882 ms; at T2047 they span 76.687–538.503
ms. The profile therefore supports visitor and scalar-transition work as the
measured targets while leaving a q8 scheduling imbalance, especially at
T2047. The candidate T2047 counter records 11,620 transition entries, 8,612 completed
transitions, and 2,976 declined transactions. Thirty-two entries returned
during pre-transaction safety checks and therefore have no completed/declined
outcome record; no prefix or fused work was discarded for those entries. The
raw per-worker log retains the counts. Instrumentation changed measured wall time by +0.78% at T1023 and +0.43% at
T2047. The profile is a single diagnostic run at each size, not a paired
performance acceptance benchmark. It does not isolate the T511 spin-1
default-link slowdown or satisfy P01's full control matrix. P01 remains
unaccepted. Structured route results, timing and peak-RSS summaries, source and binary hashes, and build provenance are retained beside this summary. Raw host and process captures remain local and untracked.
