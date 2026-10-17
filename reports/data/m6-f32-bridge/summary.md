# M6 f32-to-f64 bridge screen

The private bridge widened public f32 coefficients or maps to f64, ran the
f64 `ShtPlan`, and downcast outputs to f32. It beat native f32 in all 24
screen cells, with speedups from 3.63x to 31.54x. The bridge remains
slower than DUCC f32 in most T511/T1023 cells; this screen does not establish
f32 parity or accept a production change. The maximum full-array relative-L2 error versus DUCC f32 was 5.474e-08 for the
bridge and 4.873e-05 for native f32. Bridge outputs differed from DUCC f64
cast to f32 by at most 2.949e-11. Each Rust cell used one process, one warmup, and five within-process
alternating samples for native f32, bridge, and f64 with the same f32-rounded
inputs. DUCC f32 and f64 each used one warmup and five timed calls in a
separate q-specific process. The Rust process retained both plans and
workspaces for comparison; its peak RSS of 236.7 MiB is not a per-mode
memory measurement. The bridge's per-call temporary arrays occupy up to
48.0 MiB at T1023 spin-1. The CSV contains all 24 GL STANDARD cells for T127/T511/T1023, scalar and
spin-1, q1/q8, synthesis and analysis. DUCC measurements use version 0.41.0
at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`. The 24 successful Rust cell preflights recorded ELF SHA-256
`2d5ddf6febd6ef9fd480a0c2cd97feb1f10f89ee0e003097cfa2b2be0ea6e4fb`. A separate T127 spin-1 smoke attempt failed with `NonRealMZero { degree: 1 }`
and is excluded. A later replay used a different local ELF
(`aaf5808ed463632ac0926360265b3411f557eb2f8226f49510245a42c3bb54b6`) and
failed with the same error; it is retained but excluded. The successful
ELF was not retained with a build log tying it to the archived source
snapshot, so these timings remain a screening result pending a clean,
isolated rebuild and rerun. See `provenance.md`.
