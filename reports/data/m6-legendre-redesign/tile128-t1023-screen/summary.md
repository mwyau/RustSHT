# ACTIVE_TILE_SIZE=128 T1023 diagnostic

This screen is incomplete. It ran the baseline first and stopped before the
candidate process because the baseline's postflight CPU pressure exceeded the
predeclared limit. No comparison or acceptance result can be calculated.

## Verified artifacts and configuration

- Baseline source revision:
  `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`; extension SHA-256
  `69d3a2138e8012f331a94c8c09347433e526e5c8362c917cfefcb849c7177036`.
- Candidate source base revision:
  `739275d20987fec35506916cb682f961e8a7ffaf`; active tile size 128. Candidate
  patch SHA-256 `bd5ce130e0b619f87660eaa80d04e004605fcabc35e6769d3ca18051d0f0b2c7`;
  extension SHA-256
  `5eeff4dd2087238f18278928ad2af84437d21b7caa38eb360783f9b627cf8620`.
- Harness SHA-256:
  `72cd1986a220573c1a445bd6a6aa96b9df272a6bff1dc833843ee3f274c77739`.
- T1023, GL, scalar f64 STANDARD synthesis into output, backend `all`,
  `ntheta=1024`, `nphi=2048`, full `mmax=1023`, `phi0=0.37`, one thread pinned
  to one CPU core, `runs=1`. Each process requests the harness correctness call, one
  warm-up and five timed samples. No build was run for this screen.

## Baseline process

The baseline process completed with GNU time exit status 0. Rust synthesis
samples were `300.971, 298.175, 296.277, 296.549, 296.407 ms` (median
`296.549 ms`, CV `0.600%`, first-three/final-two median drift `0.572%`). The
process stability calculation passes. The harness reported relative L2 error
`1.182e-13` against DUCC. GNU time recorded 18.89 s elapsed, 99% CPU, 275,928
KiB peak RSS, and involuntary context-switch detail omitted. The CPU-pressure check failed the declared gate; exact transient readings remain local. The runner therefore stopped before launching the candidate. The CPU-pressure validity decisions, timing samples, elapsed time,
and peak RSS are summarized above. Raw process, command, and host captures are
local and untracked. No sample was retried or discarded. This result remains a
historical diagnostic under the original 0.50% protocol; protocol v2 does not
retroactively change its status.

## Result

The T1023 pair is incomplete and inconclusive. Candidate timing, paired ratio,
and candidate stability are unavailable. The retained T511 tile-128 screen is
separate evidence and is not combined with this diagnostic. No further cell or
retry was run.
