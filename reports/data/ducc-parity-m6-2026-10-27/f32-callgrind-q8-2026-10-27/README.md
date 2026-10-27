# f32 GL T720 scalar synthesis Callgrind profile — 2026-10-27

This evidence profiles one warmed native-f32 scalar synthesis transform at GL `lmax=mmax=720`, `ntheta=721`, `nphi=1458`, `phi0=0.23`, `q=8`, spin 0. It compares accepted RustSHT source `a30537bdcfa0fc5273985abfd592998325a608de` with exact parent `972bacdaef57388509738d1c2cdebbb7973b89be` and pinned DUCC 0.42.0 `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

## Results

| Implementation/profile | Retired instructions (Ir) | Relative to DUCC |
|---|---:|---:|
| RustSHT parent | 9,609,850,325 | 38.013× |
| RustSHT current | 8,465,515,501 | 33.486× |
| RustSHT current, instruction/jump detail | 8,465,208,796 | 33.486× |
| DUCC | 252,805,383 | 1.000× |
| DUCC, instruction/jump detail | 252,805,704 | 1.000× |

The current RustSHT profile used 11.91% fewer Ir than its exact parent. Ir is not cycles or elapsed time, so this does not establish a timing gain or DUCC parity. A separate whole-transform screen recorded candidate/parent `0.9846` at q8 and candidate/DUCC `140.90×` for this full-mmax cell. That result is context only; Valgrind timings from this profile are not used.

All recorded Rust and DUCC output map-0 files were finite. The current profile output SHA-256 was `260e35799c6f8d3c505fc9d6f2cc5bdbd5443e03d4e75e4d488044904066d45d`; parent was `55ae238d9ec96e66b7d888fdd99646dedf28be1dff0d93261200c0195f93b8b2`; DUCC was `8b0ebf91d2af1961a2fc66eacb82329b7612f11813f2850daa28c5b28cae77d4`. Parent/current relative L2 error was `3.940015492582086e-7` (max absolute `7.197260856628418e-6`). Current/DUCC relative L2 was `1.7234307359793145e-4` (max absolute `9.644031524658203e-4`). See `summary.json` for fixture hashes, output size, profile details, and environment.

## Attribution and follow-up screen

`multiply_complex_scaled_f32` was called 20,886,428 times and accounts for 3,314,613,059 inclusive Ir (39.15%) in the current full profile. The paired fallback traversal has about 3.45B inclusive Ir (40.78%), including its nested scaling calls. These costs overlap and must not be added. The normal-factor helper also has source-attributed costs at inlined sites; those rows are not an independent total. The fused V3 block executes 1,744 times, but the existing all-lanes range gate admits only fully normal blocks. The separate route trace at parent `972bacda` observed 1,744 such blocks among 5,499 (31.7%); that trace is not a counter from the current profile.

The per-lane safe-mask follow-up was screened on GL T720 full-mmax scalar synthesis. q1 was mixed and near parity (paired candidate/parent `0.993786`); all three q8 pairs favored the parent (candidate/parent `1.019974`, range `1.016261–1.025878`). The extra mask branch and fused pass were rejected. See the [matched screen and recoverable patch](../mixed-lane-fused-synthesis-screen-2026-10-27/README.md).

DUCC `calc_alm2map` was called 1,139 times; its inclusive subtree cost was 150,669,396 Ir (about 59.6% of DUCC's total) and includes `alm2map_kernel`. The raw DUCC profile is retained, along with the relevant disassembly. Rust Callgrind logs report a non-fatal `brk segment overflow in thread #1`; all runs completed and the collected Ir totals agree with their profile files. The first zero-count/termination-part profile attempts were discarded; only the corrected valid profiles below are retained.

## Reproduction artifacts

- `profiles/*.callgrind.gz`: compressed valid parent, current, and DUCC profiles plus detailed instruction/jump profiles.
- `profiles/*.valgrind.log.gz`: exact compressed Valgrind collection logs for those five profiles.
- `source/callgrind-instrumentation.patch.gz` and `source/callgrind_client.c`: temporary worker instrumentation; decompress the patch before applying it. Neither is production source.
- `disassembly/*.txt.gz`: Rust fused V3 and DUCC `alm2map_kernel` disassembly from the pinned builds.
- `checksums.sha256`: checksums for retained files.

The instrumented region is exactly one `run_once` after setup, first-run, warmup, and repeat/reset validation. Rust was built with Rust 1.99.1, release, `parallel`, `CARGO_PROFILE_RELEASE_DEBUG=2`, and `-C target-cpu=x86-64-v3`; DUCC used GCC 15.2.0, CMake release, `-O3 -DNDEBUG -g -ffast-math -ffp-contract=fast -march=x86-64-v3`. Callgrind used `--instr-atstart=no`; detailed profiles also used `--dump-instr=yes --collect-jumps=yes`.

After the report, raw profiles, source patch, and checksums were verified outside scratch, the exact session-owned parent/current worktrees and scratch root were removed. The root measured 1,524,890,313 apparent bytes; `/tmp` available space increased by 1,547,153,408 bytes and free inodes by 10,509. The scratch root and worktree registrations are absent, and no profiling/build worker remains.
