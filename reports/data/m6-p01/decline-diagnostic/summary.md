# P01 q8 lane-retry route-accounting diagnostic

The T2047 scalar route test completed one uninstrumented and one instrumented synthesis. The output maps were bitwise equal. The uninstrumented call took 512.470 ms; the call with route counters took 512.947 ms, a 0.09% difference. This is an instrumentation check, not a paired performance measurement. The test used GL STANDARD f64 with `lmax=mmax=2047`, `ntheta=2048`, `nphi=4096`, `phi0=0.37`, and an eight-thread Rayon pool. It initialized deterministic coefficients at scale `1e-5`. The instrumented call recorded 2,149,580,800 potential latitude-lane/degree-pair units and 1,734,378,064 committed units. It recorded 78,241 transition calls, of which 67,676 declined after a product-range check. The per-worker sums of fused, visitor, and transition timers ranged from 77.4 to 431.5 ms; these sums are CPU time, not wall time.

## Source and run identity

The source snapshot was compared with production commit `a50f70051049e37fa71898d5ec0aecc56ff388b3`. The saved source diff is [`source.patch`](source.patch), SHA-256 `0d8dc3497b7f6b39429993927f0465ac06e31f65eb50ec69d0a84785aaaaf43a`. It changes `src/legendre/mod.rs`, `src/legendre/route_profile.rs`, `src/legendre/scalar.rs`, `src/legendre/scalar_paired.rs`, and `src/transform.rs`. The test executable SHA-256 is `2ff557b0472cb673ae50a6965cbff893df114bdda38edf86ec264f1be8139dd0`; the copied `Cargo.lock` SHA-256 is `07e3ac929c8f1f6b708b36810accef15d5a9e71918f35d541246c6df15da26d1`. The release test used an eight-thread Rayon pool. Its exact local launch command is not retained.

```sh
taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" /usr/bin/time -v env TMPDIR=<local-home-path> \
  CARGO_TARGET_DIR=<local-home-path> \
  RUSTSHT_ROUTE_LMAX=2047 RUSTSHT_ROUTE_THREADS=8 \
  cargo test --manifest-path <local-home-path> \
  --release -p rustsht --features parallel --lib \
  legendre::route_profile::m6_scalar_synthesis_routes -- --ignored --exact --nocapture
```

The saved build output does not record the compiler version; `rustc --version` at inspection returned 1.99.0. The run requested an eight-thread Rayon pool and matched CPU affinity. Host telemetry remains local. Local raw test output, GNU-time data and preflight captures are untracked. This summary retains the test result, elapsed time, peak RSS and source identity. The whole cargo command took 13.54 s including an 11.82 s release build; the test itself reported 1.69 s. Peak RSS was 1,269,440 KiB. The test toggled only route-counter collection; it did not change route selection. It establishes bitwise equality for one deterministic T2047 scalar synthesis with counters disabled and enabled. It does not measure candidate-versus-baseline performance, estimate run-to-run variation, or exercise the spin-1 control. The default-link P01 matrix and its T511 spin-1 failure are reported in the main M6 report; this route test does not change that result.
