# Synthesis phase profile provenance

The profile logs contain phase timings and are retained as measurement evidence. The raw host and process captures remain local and untracked.

| Run | Source revision | Worktree diff SHA-256 | Test binary SHA-256 |
| --- | --- | --- | --- |
| Invalid nonreal m=0 attempt, excluded | `8332b40afad09f210a173ce02e90393c34c84988` | `02caf485812c7f254a154d76b54304c868bb598256633f854eee80ea2f887909` | `6ca47bea052b0cea11880cb30d3692627e4e89c63c91444b58421c7af3ac3679` |
| Valid even-longitude profile | `8332b40afad09f210a173ce02e90393c34c84988` | `fd908f17c332b28ba9b87a4c97164619803c829f12c0db09a7f48ec498dd7572` | `ae71f120b36838a938959df1402c07f12297c53961db06ceb786bb70dcef614c` |
| Valid odd-longitude profile | `8332b40afad09f210a173ce02e90393c34c84988` | `b44e6ab74230c43c53441489e2b183df0f485d08c1a4ad7d197ca0a79e48235e` | `1af9e844f799d31c733de351a1faf0d8dacf0b7787d81b38f85fbde77f3797f2` |

Toolchain: rustc 1.99.0, Cargo 1.99.0, LLVM 23.1.1, target `x86_64-unknown-linux-gnu`, release profile, no `RUSTFLAGS` or `target-cpu` override. The diagnostic used the pinned DUCC 0.41.0 source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf` for reference context.

The host architecture was x86_64 and AVX2/FMA were available. Thirty-two logical CPUs were visible to the diagnostic process. Physical-core count, CPU model, and installed RAM were not recorded for these runs. The invalid m=0 attempt is excluded from the reported measurements. Per-process affinity identifiers, frequency, pressure snapshots, and process listings are not retained in this repository.
