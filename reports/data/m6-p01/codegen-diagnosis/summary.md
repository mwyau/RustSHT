# P01 spin-1 code-layout diagnosis

## Scope and binaries

This is a read-only comparison of the retained pre-P01 baseline, default-link
P01, and favorable ordered-link P01 executables. No benchmark or build was
run. The binaries are x86-64 PIE executables built with rustc 1.99.0; the
default and ordered P01 binaries carry the same P01 source-patch SHA-256
`fff74a3059aec58741072b3eb1a96421c006ddcc4ba6847949098d17266faecf`.

| Binary | SHA-256 | `.text` start | `.text` size |
| --- | --- | ---: | ---: |
| Pre-P01 baseline | `4fd894d46b7195d8a7a4c24cc52f3f04a85ccd1c213d5a2c896f7c828156ccd0` | `0x75800` | `0x20df05` |
| Default-link P01 | `3388c0219a9d9ba214241763bd8749a35ef6e60bbe685ec779b842291ff6da2e` | `0x75d60` | `0x2141b5` |
| Ordered-link P01 | `22cb00437aa6cbe33222fc1a8140694c69ef8ff63694fbaa7310f50561b968f2` | `0x75d60` | `0x2141a5` |

P01 increases `.text` by `0x62b0` bytes over the baseline. The ordered and
default P01 binaries have nearly identical text sizes; ordering changes where
functions sit within that text. The successful ordered link command was not
retained, so its exact flag remains unknown.

## Spin-1 hot-path symbols

`nm -S -nC` shows these symbol sizes are unchanged across all three binaries:

| Function | Baseline | Default P01 | Ordered P01 | Size |
| --- | ---: | ---: | ---: | ---: |
| `synthesize_spin_ring_group` | `0xb8eb0` | `0xb9410` | `0xc28b0` | `0x4e5e` |
| `synthesize_spin1_scalar_block` | `0xbe040` | `0xbe5a0` | `0xc7a40` | `0x1e60` |
| spin-1 safe-lane helper | `0x10caa0` | `0x10d070` | `0x116510` | `0x796` |
| V4 spin-1 recurrence kernel | `0x148d90` | `0x149360` | `0x152800` | `0xcd1` |
| V3 spin-1 recurrence kernel | `0x14c570` | `0x14cb40` | `0x155fe0` | `0x8db` |

In the baseline-to-default comparison, the ring group and scalar block keep
the same offsets within `.text`; the helper and V3/V4 kernels move by `0x70`
relative to `.text`. In the default-to-ordered comparison, every listed
spin-1 symbol moves together by `+0x94a0`, preserving its internal spacing. The P01 dispatcher and V3/V4 transition kernels move from near the end of
`.text` in the default binary (`0x1f8130`, `0x204fb0`, `0x2127f0`) to near its
start in the ordered binary (`0x7a900`, `0x7c950`, `0x79090`).

## Disassembly and interpretation

The disassembly covers `synthesize_spin1_scalar_block`, its safe-lane helper,
and its V3/V4 recurrence kernels. After normalizing relocated addresses and
RIP-relative displacements, their normalized instruction streams have
matching SHA-256 digests across the baseline, default P01, and ordered P01
binaries. Function sizes also match. Full disassembly is in
[`spin1-hot-path-disassembly.txt`](spin1-hot-path-disassembly.txt); normalized
instruction digests and instruction counts are in
[`normalized-disassembly.tsv`](normalized-disassembly.tsv). Symbol and text
section addresses are in [`spin1-p01-symbols.tsv`](spin1-p01-symbols.tsv) and
[`binary-sections.tsv`](binary-sections.tsv). Run
[`diagnose.py`](diagnose.py) to regenerate these listings from the retained
ELFs; binary and evidence SHA-256 values are in
[`evidence.sha256`](evidence.sha256). The same-source ordered/default screen measured T511 spin-1 synthesis at
`0.96668` [0.96024, 0.97375] and T1023 scalar synthesis at `1.00025`
[0.99767, 1.01304]. Together with the unchanged spin-1 instructions, this
supports a final-executable layout effect. It does not identify a particular
cache conflict or displaced spin-1 instruction as the cause of the default
P01 shift. The diagnosis therefore establishes executable-level layout
correlation, not a source-level mechanism.

## Recommendation

Shelve P01 in its current form. Its scalar gain is material, but the default
link repeatedly crosses the spin-1 regression guard, and the favorable layout
is an executable-level result with an unretained link command. The route
profile still identifies general scalar visitor work as the next M2 target:
visitor timer sums were `191.910 ms` at T1023 and `1505.354 ms` at T2047 q1. The matched T1023 scalar synthesis took `296.372 ms` in Rust, compared with
`49.555 ms` for DUCC and `34.670 ms` for the documented SHTns adapter. P02's
prepared-coefficient reuse did not materially improve the direct comparison,
so the next M2 step should target the visitor's work itself while retaining
the scaled recurrence and safe fallback behavior.
