# Plan 15 GL spin-1 row-parallel FFT A/B

This narrow experiment compares the production spin-1 STANDARD synthesis worker before and after routing each output channel through the existing row-parallel inverse-FFT helper. It does not change the FFT algorithm.

`rowfft-ab.json` contains the four T720 workloads, interleaved three-process-pair measurements, all timed-call latencies, setup/first/warm timings, per-process timing arrays and setup/first/warm timings, output consistency checks, binary hashes and compiler/source metadata. The before binary is source `4523b927`; the candidate is source `ac8e7547`. DUCC is pinned to `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`. See the [focused report](../../plan15-spin1-gl-rowfft-2026-10-21.md) for interpretation and limitations.
