# P01 source-order relocation screen

## Result

Moving the P01 scalar transition implementation to the end of
`scalar_paired.rs` did not change its relative placement against the hot
scalar kernels in the default-linked benchmark executable. Every inspected
P01 symbol shifted by the same `-0x36670` address delta, and the measured
inter-symbol gaps remained unchanged. The predeclared symbol-movement
condition was not met, so no benchmark was run for this variant.

## Source and build

The isolated copy was prepared from base commit
`47b7f16720eb2e01ecd9e8c725687269a2aad3c2`. Its P01-only source patch is
[`source.patch`](source.patch), SHA-256
`fd1e1c1951b7b22a575f833634c46b8fffff2d9e9e93fda6e6536a78ccc27dc1`. The patch adds the P01 scalar transition entry point and call site, strips
test-only M1 route counters, and restores the pre-P02 visitor coefficient
calculations. Source-file SHA-256 values are in [`source.sha256`](source.sha256). The source preparation script is retained as [`prepare.py`](prepare.py). The candidate was built in the isolated source copy with Rust/Cargo 1.99.0
using the standard release command:

```text
CARGO_TARGET_DIR=<local-home-path> cargo build --release --example m1b_bench
```

`RUSTFLAGS` and `CARGO_ENCODED_RUSTFLAGS` were empty; no custom linker or
ordering flag was used. The build log and host preflight are retained as
[`build.log`](build.log) and [`build-preflight.txt`](build-preflight.txt). The isolated candidate ELF SHA-256 is
`23f2c95feef63003ff0c288628b4b334258ddaea900012917829a4677370971c`. The default-link P01 comparison ELF SHA-256 is
`3388c0219a9d9ba214241763bd8749a35ef6e60bbe685ec779b842291ff6da2e`. The ELF is retained under the ignored `target/` tree rather than in this
report directory.

## Symbol evidence

The retained files [`default-p01-symbols.txt`](default-p01-symbols.txt) and
[`candidate-symbols.txt`](candidate-symbols.txt) were produced with `nm -nC`. The default listing was independently regenerated from the comparison ELF as
[`default-p01-symbols-verified.txt`](default-p01-symbols-verified.txt). Selected addresses are:

| Symbol | Default P01 | Relocated source | Relative observation |
| --- | ---: | ---: | --- |
| V4 P01 transition | `0x1f8130` | `0x1f4ac0` | `-0x36670` |
| V3 visitor closure | `0x1faa20` | `0x1f73b0` | `-0x36670` |
| V3 P01 transition | `0x204fb0` | `0x201940` | `-0x36670` |
| P01 scalar dispatcher | `0x2127f0` | `0x20f180` | `-0x36670` |

The V3 visitor-to-transition gap is `0xA590` in both binaries. The preceding
hot scalar block-to-V4 transition gap is `0xC050` in both (hot-block starts:
`0x1ec0e0` default and `0x1e8a70` relocated). Source-item relocation therefore
did not separate the P01 code from these kernels in the linked text. This
experiment does not explain or resolve the default-link spin-1 shift, and no
performance inference is drawn from it. [`evidence.sha256`](evidence.sha256)
records the ELF, source, build-log and symbol-list hashes.
