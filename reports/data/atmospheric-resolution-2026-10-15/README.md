# Atmospheric-resolution native benchmark screen

This directory retains the first native performance screen for the
atmospheric-resolution study. It is an intermediate data checkpoint; the final
interpretation will combine these measurements with paired repeats, DUCC,
phase profiles, representative f32 runs, batch reuse, and the ecTrans study. `grid-matrix.csv` defines 29 GL grids. T42, T72, T85, T159, T255, T340, and
T360 use the longitude count selected by `GridSpec::fft_friendly_nphi`. T511, T639, T720, T799, T1023, T1279, and T2047 use every distinct count
among minimum odd, benchmark default, minimum even, and FFT-friendly
sampling. Every grid has `ntheta=lmax+1` and `mmax=lmax`. `run-native-matrix.sh` runs `m1b_bench` for f64 STANDARD synthesis and
analysis at spins zero and one. Each fresh process records five timed samples
for a reused plan/workspace at q1 and q8, first-call and setup timings, and
peak process RSS. Processes are 8 pinned CPU cores. Raw output is in
`native/`; `summarize_native.py` emits `native/summary.csv` with one row per
grid, spin, operation, and thread count. The native binary hash and source
revision are retained alongside the raw results. The initial screen contains 58 complete processes. `T720-nphi1458-spin0-pilot`
is an earlier q8-only pilot and is excluded from `summary.csv`. The first
T2047/nphi=4095 attempt overlapped a separate `rustsht-fft` cargo test; both
spin files are preserved under `native/contended/` and excluded from the
summary. A clean T2047 rerun follows the same matrix script. No other benchmark
or test process was active during that rerun. The first-screen timings indicate that minimum-even sampling is often faster
than minimum-odd sampling, especially for the larger grids. These are
screening measurements from separate fresh processes. They need paired repeats
before the size of the effect can be treated as stable. The FFT-friendly
selection is not uniformly fastest when it adds several longitude samples. `host-before.txt` and `host-after.txt` preserve host snapshots. The host is
shared, CPU affinity is set but physical cores are not isolated, and the
measurements therefore remain exposed to unrelated background activity.
