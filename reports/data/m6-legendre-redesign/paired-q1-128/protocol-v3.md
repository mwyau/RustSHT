# Tile-128 independent q1 acquisition v3

This is a new, complete acquisition after the v2 T1023 baseline cell stopped on a DUCC timing-drift failure. The v2 sample remains diagnostic and is not reused, removed, or reclassified. V3 runs both variants from pair 1 under the fixed matrix below; it does not retry one failed process or replace an unfavorable sample.

## Frozen sources and workload

- Baseline: clean scalar worktree at source HEAD `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`; extension SHA-256 `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Candidate: tile-128 source at HEAD `739275d20987fec35506916cb682f961e8a7ffaf` plus patch SHA-256 `bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7`; `synthesis_fast.rs` SHA-256 `0c0ee956205f782626e6ab0ffc6914cfa9f0d1e20caf5075111a53a9c9449b25`; extension SHA-256 `5eeff4dd2087238f18278928ad2af84437d21b7caa38eb360783f9b627cf8620`.
- Harness: `examples/python_perf_baseline.py`, SHA-256 `72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`.
- References: DUCC 0.42.0, source `80388904b8cfc32400ba84af8a6a45aabfe60f86`, extension SHA-256 `1218502fa03116cd14578fe73f716f5a1d88db74d296359cc9e8c27e04047d1e`; SHTns source `4e69ceb3fc5f19475d55b03ff0fdec210dc3075e`, library SHA-256 `641195ac574ba02078f3238b357caecf0e268b3f4ef7eba1d87898197fcc888f`.
- Each process runs GL f64 scalar `STANDARD` `synthesis_into`, full `mmax`, `phi0=0.37`, one Rayon worker, backend `all`, one harness warm-up, then five timed calls per backend. Baseline and candidate use identical dimensions and binaries throughout the acquisition. Do not rebuild or edit either source during collection.

## Fixed process order

For each bandlimit, run six fresh-process pairs. Pair 1 is baseline then candidate, pair 2 candidate then baseline, alternating thereafter. Use labels `v3-T1023-pair01-baseline` through `pair06-candidate`, with the same naming for T2047 and T511. Run T1023 first. Run T2047 only if every T1023 process passes the gates. Evaluate the T511 scalar q1 control after the high-bandlimit q1 screen and before any q8 test.

Use one allowed CPU for each process. The CPU-set value and observed placement remain local. Record the worker count and the selected hardware/software configuration in the committed summary. Preserve all raw preflight, during, postflight, GNU-time, process, frequency, placement, and console captures locally and untracked.

After the T1023 and T2047 cells pass their process gates, run the six-pair T511 control regardless of the observed high-bandlimit speedup. This keeps the control in the predeclared matrix rather than selecting it after seeing results.

## Validity and expansion rules

- Preflight and postflight CPU PSI `some avg10` above 2.00% rejects the process. Readings from 0.50% through 2.00% are warnings; record the pressure-gate decision and keep exact readings local.
- Each RustSHT, DUCC, and SHTns five-call series must have population CV at most 5% and first-three/final-two median drift at most 5%. Do not drop samples or retry a process. Stop the current bandlimit cell on a failed process gate.
- Review CPU affinity, worker placement, context switches, CPU frequency, concurrent CPU-intensive work, cgroup pressure, and PSI. Clear interference makes a process inconclusive even when PSI is below 2.00%. Stop before the next process until the completed process passes this review.
- Compute six paired candidate/baseline Rust synthesis median ratios. Use 50,000 bootstrap resamples, seed `20261017`, median per resample, and linearly interpolated 2.5th/97.5th percentiles. The high-resolution screen passes only if one of T1023/T2047 has median ratio at most 0.90 and interval upper bound below 1.00, and the other interval upper bound is at most 1.02. The T511 control interval upper bound must be at most 1.02 before q8 controls.
- This screen does not accept the candidate. Candidate maps must agree with DUCC and pass the existing numerical contract. Do not begin q8 or broader regression measurements unless every required q1 process is valid and the expansion rule passes.
