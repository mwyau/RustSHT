# Executed Rust f32 fused-leaf assembly summary

The inspected specialization is the V3/AVX2 instantiation of
`FusedSynthesisBlock<f32, true, false>` at source `3189fe8e`. Callgrind records
1,744 calls for the profiled GL T720 q8 transform. The following path-free
excerpt retains the instructions relevant to the loop structure:

```asm
vmovups       (...), %ymm5
vmovups       (...), %ymm9
vmulps        %ymm5, %ymm5, %ymm0
vmovups       %ymm0, 0xc0(%rsp)
vfmadd213ps   %ymm13, %ymm5, %ymm11
vfmadd213ps   %ymm14, %ymm5, %ymm12
vfmadd231ps   %ymm11, %ymm9, %ymm6
vfmadd231ps   %ymm12, %ymm9, %ymm7
vfmadd231ps   0xc0(%rsp), %ymm4, %ymm15
vfmadd213ps   %ymm10, %ymm9, %ymm15
vcmple_oqps   %ymm3, %ymm4, %ymm1
vcmpeqps      %ymm4, %ymm8, %ymm2
vcmpge_oqps   %ymm0, %ymm4, %ymm4
vptest        %ymm0, %ymm1
jb            <recurrence-update>
```

The leaf uses eight f32 lanes, vector FMAs for coefficient/recurrence and
complex accumulation, and vector comparisons plus a scalar branch to enforce
the normal-range guard. It also stores and reloads an intermediate through a
stack slot; this excerpt does not distinguish compiler spill traffic from
explicit scratch use. The source profile counts only 1.93% of whole-call Ir in
the fused leaf, so this loop is not the dominant remaining runtime path.

This is a focused, path-free summary of the executed symbol, not a cycle
estimate or a complete disassembly. DUCC's f64-promoted four-lane kernel
comparison and its twelve FMAs per four-pair body are documented in the
[matched fallback report](../f32-scoped-task-grain-2026-10-27/README.md).
