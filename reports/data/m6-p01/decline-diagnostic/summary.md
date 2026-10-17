# P01 q8 lane-retry route-accounting diagnostic

The T2047 scalar route test completed one uninstrumented and one instrumented synthesis. The output maps were bitwise equal. The uninstrumented call took 512.470 ms; the call with route counters took 512.947 ms, a 0.09% difference. This is an instrumentation check, not a paired performance measurement. The test used GL STANDARD f64 with `lmax=mmax=2047`, `ntheta=2048`, `nphi=4096`, `phi0=0.37`, and an eight-thread Rayon pool. It initialized deterministic coefficients at scale `1e-5`. The instrumented call recorded 2,149,580,800 potential latitude-lane/degree-pair units and 1,734,378,064 committed units. It recorded 78,241 transition calls, of which 67,676 declined after a product-range check. The per-worker sums of fused, visitor, and transition timers ranged from 77.4 to 431.5 ms; these sums are CPU time, not wall time.

## Source and run identity

The source snapshot was compared with production commit `a50f70051049e37fa71898d5ec0aecc56ff388b3`. The saved source diff is [`source.patch`](source.patch), SHA-256 `0d8dc3497b7f6b39429993927f0465ac06e31f65eb50ec69d0a84785aaaaf43a`. It changes `src/legendre/mod.rs`, `src/legendre/route_profile.rs`, `src/legendre/scalar.rs`, `src/legendre/scalar_paired.rs`, and `src/transform.rs`. The test executable SHA-256 is `2ff557b0472cb673ae50a6965cbff893df114bdda38edf86ec264f1be8139dd0`; the copied `Cargo.lock` SHA-256 is `07e3ac929c8f1f6b708b36810accef15d5a9e71918f35d541246c6df15da26d1`. The outer launch requested 8 pinned CPU cores with `taskset`; `/usr/bin/time` recorded the wrapped inner `env`/`cargo` command:

```sh
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -v env TMPDIR=<local-home-path> \
  CARGO_TARGET_DIR=<local-home-path> \
  RUSTSHT_ROUTE_LMAX=2047 RUSTSHT_ROUTE_THREADS=8 \
  cargo test --manifest-path <local-home-path> \
  --release -p rustsht --features parallel --lib \
  legendre::route_profile::m6_scalar_synthesis_routes -- --ignored --exact --nocapture
```

The saved build output does not record the compiler version; `rustc --version` at inspection returned 1.99.0. Load-average telemetry was retained only in local diagnostics. The run requested an eight-thread Rayon pool and 8 pinned CPU cores. The host CPU model was not recorded. The raw output is [`lane-retry-route.log`](lane-retry-route.log), SHA-256 `bb2f8a7462a47d2bc35873f387097966e0e98565bb2827e51c2476e7573c0db6`; `/usr/bin/time -v` output is [`lane-retry-route.time`](lane-retry-route.time), SHA-256 `bd78f473fdf8fa8f660a1670f0bee4d0ed3b5bc82ea5a47e9d1876c79c5fa45c`; the preflight is [`preflight-lane-retry.txt`](preflight-lane-retry.txt), SHA-256 `0c299b83b1ed36e5bc98214a8300c98266079a39563338fd5b3065d25d0d1617`. The whole cargo command took 13.54 s including an 11.82 s release build; the test itself reported 1.69 s. Peak RSS was 1,269,440 KiB. The test toggled only route-counter collection; it did not change route selection. It establishes bitwise equality for one deterministic T2047 scalar synthesis with counters disabled and enabled. It does not measure candidate-versus-baseline performance, estimate run-to-run variation, or exercise the spin-1 control. The default-link P01 matrix and its T511 spin-1 failure are reported in the main M6 report; this route test does not change that result.
