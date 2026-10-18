# Polar-lane route diagnostic

This is an isolated route-accounting test, not a performance measurement. It
uses the f64 GL scalar synthesis prototype at T511, q1, with deterministic
coefficients. The profiler counts active latitude lane × degree-pair work. The pre-split source is the candidate recorded in
[`prototype-screen`](../prototype-screen/), with temporary test-only counters. Its raw route counts were captured before the source was edited but the exact
transient file hash was not recorded. The post-split snapshot is
[`synthesis_fast.rs`](synthesis_fast.rs), SHA-256
`b0c97dab532ae313f540cff131f5d1e1fe8e9de60142cf385ba858552a66facb`. It is applied over the source integration patch in the prototype-screen
archive (base commit `739275d20987fec35506916cb682f961e8a7ffaf`). The release
unit-test executable SHA-256 is
`67796124d41aaf9f03321783da8f1afe7987e5d8e904378d7f3f3c99532c6f5e`. Command:

```sh
RUSTSHT_FAST_ROUTE_LMAX=511 RUSTSHT_FAST_ROUTE_THREADS=1 \
  cargo test --release -p rustsht --features parallel --lib \
  legendre::synthesis_fast::tests::m6_fast_route_counts \
  -- --ignored --exact --nocapture
```

The pre-split logic declined 210 of 974 tiles. Of those, 206 were declined
because at least one lane had `sin(theta) < 0.01`; they contained approximately
12.38 million lane-pairs of otherwise ordinary work that consequently used
the robust visitor. The post-split kernel isolated 47,534 polar lane-pairs,
completed 970 tiles, and used the robust visitor for 255,104 lane-pairs. Four
ordinary tiles still declined after attempting a transition. These counts
identify broad fallback caused by a tile-level polar check; they do not prove
the full timing impact. A paired T511 process benchmark is pending in the
separate `polar-split-screen` evidence directory.
