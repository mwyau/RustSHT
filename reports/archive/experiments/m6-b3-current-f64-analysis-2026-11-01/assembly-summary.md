# B3 AVX2 f64 leaf disassembly summary

Source parent: `31c7cd5046d2389d5f0de230577dc147ff37f873`. Static instruction counts are from the exact release benchmark binaries; they are not dynamic instruction totals.

| Kernel | Symbol bytes | `vfmadd*` | `vmulpd` | `vaddpd` | Stack frame / vector spills |
|---|---:|---:|---:|---:|---|
| Rust `NativeFusedAnalysisPairChannels<f64, false>` | 3,224 | 44 | 1 | 8 | 280-byte frame; no YMM stack references observed |
| DUCC `map2alm_kernel` | 722 | 18 | 0 | 6 | no stack references observed |

Both leaves use four-lane AVX2 f64 vectors. Their boundaries differ: DUCC advances one scalar channel per invocation and invokes it twice; Rust's leaf fuses both reflected channels. These counts alone cannot identify the cycle cost. Source and binary hashes are recorded in `recurrence-fusion-results.json`.

Representative Rust leaf instructions:

```asm
1d71c6: vfmadd231pd (%r10,%rdx,1),%ymm12,%ymm6
1d71cc: vfmadd231pd (%rbx,%rdx,1),%ymm12,%ymm7
1d71d2: vfmadd231pd (%r11,%rdx,1),%ymm12,%ymm10
1d71d8: vfmadd231pd (%r14,%rdx,1),%ymm12,%ymm11
1d71de: vmulpd (%rsi,%rdx,1),%ymm12,%ymm12
```

Representative DUCC leaf instructions:

```asm
67c490: vfmadd231pd 0x37e0(%rax),%ymm1,%ymm14
67c499: vfmadd231pd 0x3fe0(%rax),%ymm1,%ymm10
67c4a2: vfmadd132pd %ymm2,%ymm7,%ymm0
67c4a7: vfmadd213pd 0x17e0(%rax),%ymm1,%ymm0
```
