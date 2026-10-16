# ecTrans Legendre strategy study

## Scope and provenance

The source study used ecTrans revision
[`7de2ee295a0df06299a9cc78595d5148643e487b`](https://github.com/ecmwf-ifs/ectrans/tree/7de2ee295a0df06299a9cc78595d5148643e487b), checked out detached in a separate directory on 2026-10-15. The checkout was read only. ecTrans is Apache-2.0 licensed; no source code, tests, or source expression was copied or translated into RustSHT. No ecTrans dependency or production implementation was added.

The inspected implementation is in [`setup_trans.F90`](https://github.com/ecmwf-ifs/ectrans/blob/7de2ee295a0df06299a9cc78595d5148643e487b/src/trans/cpu/external/setup_trans.F90#L230-L241), [`suleg_mod.F90`](https://github.com/ecmwf-ifs/ectrans/blob/7de2ee295a0df06299a9cc78595d5148643e487b/src/trans/cpu/internal/suleg_mod.F90#L219-L236), [`ledir_mod.F90`](https://github.com/ecmwf-ifs/ectrans/blob/7de2ee295a0df06299a9cc78595d5148643e487b/src/trans/cpu/internal/ledir_mod.F90#L101-L196), [`leinv_mod.F90`](https://github.com/ecmwf-ifs/ectrans/blob/7de2ee295a0df06299a9cc78595d5148643e487b/src/trans/cpu/internal/leinv_mod.F90#L120-L175), [`tpm_flt.F90`](https://github.com/ecmwf-ifs/ectrans/blob/7de2ee295a0df06299a9cc78595d5148643e487b/src/trans/cpu/internal/tpm_flt.F90#L20-L42), and [`butterfly_alg_mod.F90`](https://github.com/ecmwf-ifs/ectrans/blob/7de2ee295a0df06299a9cc78595d5148643e487b/src/trans/cpu/algor/butterfly_alg_mod.F90#L68-L125). The mathematical background includes Wedi, Hamrud, and Mozdzynski's [fast SHT for global NWP and climate models](https://journals.ametsoc.org/doi/10.1175/MWR-D-13-00016.1), O'Neil, Woolfe, and Rokhlin's [fast special-function transform algorithm](https://www.sciencedirect.com/science/article/pii/S1063520309000888), Cheng et al.'s [interpolative decomposition for low-rank matrices](https://epubs.siam.org/doi/10.1137/030602678), Li and Yang's [interpolative butterfly factorization](https://epubs.siam.org/doi/abs/10.1137/16M1074941), and Bremer, Chen, and Yang's [ID butterfly SHT](https://epubs.siam.org/doi/abs/10.1137/20M1333845).

## ecTrans implementation

The CPU path stores per-order Legendre matrices and separates their symmetric and antisymmetric latitude components. The transform packs weighted field data into matrix columns, applies each parity block, and restores the latitude ordering. Analysis uses the transpose of the parity matrices; synthesis uses the forward matrices. The `KIFC` dimension carries fields or channels through the same matrix operation, which lets dense matrix multiplication reuse a Legendre block across multiple vertical levels.

The fast Legendre transform is conditional. Setup initializes it disabled. It is enabled by an explicit option, or indirectly when the optional Legendre-polynomial I/O setting is supplied for truncations above 511; the explicit option can override that selection. Thus it is a selectable execution strategy rather than a universal default.

For each order and parity, setup constructs the dense Legendre matrix and may compress it. The source uses a precision parameter of `1e-7` and an initial maximum block width of 64. Its direct-multiplication threshold depends on latitude count and MPI task count. Small blocks continue through DGEMM; larger blocks use the stored butterfly factors. When factors are built, the source starts from the dense matrix, extracts bounded subblocks, forms interpolative decompositions, merges factors through a dyadic hierarchy, and stores ranks, skeleton indices, interpolation data, and factor blocks. Unless configured to keep them, the dense per-order matrices are released after factor construction. Consequently, fast application can reduce persistent storage, but setup still incurs dense-matrix generation and compression work and requires temporary dense data.

The butterfly application accepts either a matrix or its transpose and carries multiple fields. This directly supports both directions of the Legendre projection. The published IDBF SHT paper reports near-quadratic complexity including precomputation and competitive performance around order 1024. That result describes a newer independently developed algorithm; it is not a performance measurement of ecTrans's implementation or of a RustSHT adaptation.

## Applicability to RustSHT

### Batched dense Legendre multiplication

Matrix-matrix multiplication is attractive when the same Legendre blocks are applied to many independent fields. A batch of 8 or 32 fields can reuse each loaded basis block and improve arithmetic intensity. For one field, this becomes matrix-vector work and may not repay storing and loading the basis. Setup and persistent matrix memory grow with the number of latitude-degree pairs across all orders.

RustSHT's current compact scalar table stores northern latitudes and uses hemisphere symmetry, but is intentionally limited to `lmax <= 255` and a 34 MiB payload. Its full-order entry count before the byte cap is approximately
`ceil(ntheta/2) * (lmax+1)*(lmax+2)/2`. At minimum GL sampling, the corresponding f64 payload would be about 0.70 GiB at T720, 2.00 GiB at T1023, 3.91 GiB at T1279, and 16.0 GiB at T2047. These are formula-based estimates for an uncapped RustSHT-style north-hemisphere table, not measurements of ecTrans storage. They explain why dense precomputation conflicts with RustSHT's existing memory policy. The atmospheric batch measurements will test whether repeated fields change the end-to-end trade-off without raising that cap.

### Butterfly-compressed transform

The Legendre matrices have the hierarchical structure used by butterfly methods, so an independently derived factorization could reduce repeated high-order application work at T1023–T2047. A single-field workload may benefit when the transform is repeated enough to amortize factorization; batched fields can improve factor application throughput further. The actual crossover depends on rank, error tolerance, setup, memory traffic, cache behavior, and the full FFT-plus-Legendre transform. Neither the asymptotic order nor the cited paper establishes a RustSHT speedup.

The source implementation's `1e-7` compression parameter is not evidence that RustSHT's high-bandlimit f64 and f32 accuracy requirements would be met. A candidate would need independent rank and error selection, analytic harmonic checks, DUCC comparisons, adjoint tests, exceptional-value tests, and whole-transform repeated-pair benchmarks. Synthesis and analysis can use the same factors in opposite directions, but the factors alone do not supply RustSHT's weighted adjoints or its spin-1 basis conversion. Spin 1 would require a separately derived factorization of the relevant spin-weighted kernels or a validated conversion to scalar potentials.

### Symmetry, blocking, and scheduling

ecTrans exploits north-south parity and blocks its matrix work before packing field columns. RustSHT already evaluates paired north/south latitude blocks in its scalar and spin kernels, stores coefficients in order-major layout, and schedules coarse groups of orders with caller-owned worker scratch. The broad scheduling ideas therefore overlap with existing RustSHT design. The remaining possible transfer is a cache-blocked batched basis application, not adding another hemisphere split or per-degree task layer.

## Recommendation

Do not change production dispatch based on this source study. First complete the 1, 8, and 32-field batch measurements and high-resolution phase profiles. Dense matrices are outside the current memory cap at every primary target. A private butterfly proof of concept is not justified until those measurements show that repeated recurrence work remains dominant for the intended field-reuse pattern and establish a target error budget. If pursued later, it must be written from the published mathematics in an isolated module, with no ecTrans source adaptation.

