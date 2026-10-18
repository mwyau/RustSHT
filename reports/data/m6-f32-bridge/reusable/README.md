# Reusable f32-to-f64 bridge prototype

This isolated prototype widens f32 inputs into persistent f64 buffers, invokes
the existing f64 plan and workspace, and narrows outputs into caller buffers.
It does not change production dispatch or add a public API.

The correctness screen passed in release mode for GL T127 and T511, scalar and
spin-1, q1. It compares each bridge result for S, A, S*, and A* bit-for-bit
with the corresponding widened-input f64 result after f32 narrowing. It also
checks the scalar and spin-1 adjoint identities, analytic scalar-monopole and
spin-1 m=0 synthesis, large and subnormal finite inputs, cancellation,
downcast overflow, and reusable buffer capacities. The longitude count is
even (`2 * (T + 1)`) and `phi0=0.37` for the operator checks. The analytic
controls use independently evaluated formulas.

At T511 the reusable bridge arrays retain 6.00 MiB for scalar and 12.01 MiB
for spin-1. These figures exclude plans, workspaces, and caller-owned buffers.
This correctness check did not measure peak RSS or performance. The earlier
native f32 versus allocating-bridge timing screen remains diagnostic and
unaccepted; no timing result for this reusable version is established here.

To run the correctness screen from the repository root, temporarily copy the
two source files into the examples directory, build, and execute:

```sh
cp reports/data/m6-f32-bridge/reusable/source/reusable_bridge.rs examples/reusable_bridge.rs
cp reports/data/m6-f32-bridge/reusable/source/m6_f32_reusable_bridge.rs examples/m6_f32_reusable_bridge.rs
cargo build --release --example m6_f32_reusable_bridge
target/release/examples/m6_f32_reusable_bridge --check
rm examples/m6_f32_reusable_bridge.rs examples/reusable_bridge.rs
```

The benchmark mode accepts `LMAX SPIN THREADS OP DUMP_DIR`, where `OP` is
`synthesis` or `analysis`, and thread counts are 1 or 8. It performs one
untimed call per variant followed by five alternating in-process calls. These
samples are a screening tool, not independent-process acceptance evidence.
Any binary dumps and raw host-state diagnostics must remain local and
untracked.
