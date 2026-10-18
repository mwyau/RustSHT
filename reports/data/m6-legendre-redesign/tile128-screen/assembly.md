# Tile-128 scalar synthesis code generation

The inspected binary is the isolated tile-128 release artifact with SHA-256
`5eeff4dd2087238f18278928ad2af84437d21b7caa38eb360783f9b627cf8620`. It was
built with rustc 1.99.0 and LLVM 23.1.1. On the x86-64 CPU, runtime dispatch
uses the x86-v3 AVX2/FMA path; the processor does not support AVX-512. The x86-v3 `StagedScalarSynthesis` symbol begins at `0x1e1270` and occupies
`0x1dd6` bytes (7,638 bytes). Its prologue reserves 296 bytes of stack space
for scalar and pointer state. The ordinary vector suffix at `0x1e24cc`–
`0x1e25dc` uses four f64 lanes in YMM registers. It keeps the two recurrence
values and real/imaginary sums in vector registers and uses fused multiply-add
for recurrence and accumulation. The ordinary vector loop has no call
instruction and no YMM stack spill; it stores the accumulated sums after each
vector block. A scalar tail handles remaining lanes. The source nests the degree-pair loop inside the vector-block loop. The
disassembly broadcasts the even/odd coefficients and recurrence factors for
each vector block, and executes slice-bound comparisons in the degree loop. It advances one paired-degree step per loop iteration. These are code-generation
observations; they do not explain the timing difference between tile sizes. The raw assembly was inspected from the verified binary and is not archived as
a large generated file. The exact symbol and inspected instruction range are
recorded here so the disassembly can be reproduced with `nm -CS` and `objdump`.
