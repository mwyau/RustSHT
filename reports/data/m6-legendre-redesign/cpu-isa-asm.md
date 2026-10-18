# CPU and SIMD assembly inspection

## Build provenance

The isolated baseline worktree was clean at `91c2cb8ff1ce576bd6dfa6764fd7fb516a00295a`. The host is an x86-64 CPU (host topology omitted), with AVX2 and FMA available and no AVX-512 feature listed. The process affinity and effective cpuset were CPUs `0-31`; cgroup `cpu.max` was `max 100000` (no quota). The source and machine records are in [`cpu-isa-asm/source.txt`](cpu-isa-asm/source.txt) and [`cpu-isa-asm/host.txt`](cpu-isa-asm/host.txt). Toolchain: `rustc 1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, Cargo `1.99.0`. The single compile command was:

```text
cargo rustc -vv -p rustsht --release --features parallel --lib -- --emit=asm
```

It completed successfully in 17.67 seconds. The verbose rustc invocation used the host target `x86_64-unknown-linux-gnu`, `-C opt-level=3`, `-C embed-bitcode=no`, and `-C strip=debuginfo`, with `default` and `parallel` features. `RUSTFLAGS`, encoded target flags, and repository/user Cargo config files were absent. The invocation contains no `-C target-cpu`, `-C target-feature`, or LTO flag; the release compiler command and environment are retained in [`cpu-isa-asm/compile.log`](cpu-isa-asm/compile.log) and [`cpu-isa-asm/cargo-flags.txt`](cpu-isa-asm/cargo-flags.txt). Cargo still requested its standard metadata/link outputs alongside the additional `--emit=asm`; no tests or benchmarks were run.

## Emitted kernels

The emitted assembly was `rustsht-3408679d315fb0ff.s`, SHA-256 `9c80a3655c245c9b07568d8e972dde2cb60e0d1895784a2906f79fd58daba2e5`. The full 6.2 MB generated file remains in the local diagnostic capture and is not part of the branch. Relevant excerpts with original assembly line numbers are in [`cpu-isa-asm/hot-loop-excerpts.txt`](cpu-isa-asm/hot-loop-excerpts.txt); symbol matches are in [`cpu-isa-asm/simd-symbols.txt`](cpu-isa-asm/simd-symbols.txt). The pinned source revision and compile command are enough to regenerate it.

| Runtime specialization in symbol | Instructions and width | Inner-loop observations |
|---|---|---|
| `pulp::x86::v3` `FusedSynthesisBlock` (`.s:112486–112986`) | AVX2 YMM operations over four `f64` lanes, with fused multiply-add. See loads at `112564–112567`, FMA recurrence/accumulation at `112578–112607`, and writes at `112614–112618`. | The SIMD degree-pair loop contains no call instruction. The function reserves 104 stack bytes and saves pointer/length metadata; there are no YMM-to-stack stores in the main vector loop. Bounds checks branch to cold `panic_bounds_check` blocks after it (`112910–112984`). |
| `pulp::x86::v4` `FusedSynthesisBlock` (`.s:101445–101883`) | AVX-512 ZMM operations over eight `f64` lanes, with fused multiply-add. See loads at `101523–101526`, FMA recurrence/accumulation at `101537–101563`, and writes at `101570–101574`. | The same structure uses a 104-byte stack frame for metadata and has no ZMM-to-stack stores in the main vector loop. Bounds-check panic calls are in the cold tail (`101806–101880`). The host does not advertise AVX-512, so this variant is present in the artifact but is not supported by this CPU. |

The host’s listed AVX2/FMA features make `v3` the widest supported Pulp specialization in this build. `pulp::Arch::new()` provides runtime dispatch, but no executable was run here, so actual runtime selection was not observed. The compiler emitted both specialized symbols without a global `target-cpu=native` setting. These symbols correspond to the ordinary paired kernel `FusedSynthesisBlock::with_simd` (`src/legendre/scalar_paired.rs:1982–2084`) and scaled visitor `ForEachPairedYlm::with_simd` (`:2899` onward) in this baseline. The scaled paired visitor has a different call shape. Its v3 symbol begins at `.s:111894`; it calls `scalar::scaled_power` during lane initialization (`111968–111971`), calls `normalize_pair_lanes` in setup paths (`112132–112141`), and calls the generic visitor closure inside the degree-pair loop (`112173–112185`, call at `112180`). The v4 specialization likewise calls the closure from its loop (`100940–100951`, call at `100947`). The visitor functions reserve 232 bytes (v3) and 264 bytes (v4) of stack space for state and metadata. Their vector recurrence body uses YMM/ZMM registers, while the callback call and associated `vzeroupper` remain between pair steps. This is a concrete code-generation difference from the fused ordinary path, not proof that callback overhead explains the measured whole-transform visitor cost.

## Limits

This is compiler-output inspection only. It establishes which specialized code was emitted, its vector width, and the call/stack structure visible in the selected kernels. It does not measure runtime dispatch, execution throughput, cache behavior, or performance impact. No benchmark, test, or second build was run.
