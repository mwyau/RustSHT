# RustSHT FFT workloads

The current call sites were inspected in `src/fft.rs`, `src/meridional.rs`,
`src/grid.rs`, `src/plan.rs`, `src/workspace.rs`, `src/transform.rs`, and
`src/spin_transform.rs` on the `feat/rustsht_fft` worktree at `8e4a8df`.

## Longitude transforms

`ShtPlan` constructs one reusable `RealFftPlan<T>` at `nphi`. Its workspace
contains a real row, a transform buffer, and RustFFT scratch. Even `nphi`
packs adjacent real samples into an `nphi/2` complex transform and reconstructs
the nonnegative bins. Odd `nphi` uses an `nphi` complex transform with zero
imaginary input. RustFFT's forward and inverse transforms are unnormalized;
the real inverse applies its required scale after reconstruction. Scalar analysis runs one real forward transform per latitude ring. Scalar
synthesis runs one real inverse per ring. Spin analysis and synthesis run two
real transforms per ring, one for each map component. These ring loops are
serial and reuse the same plan and workspace scratch. The surrounding
Legendre work can use Rayon over latitude/order groups, but it does not make
the longitude FFT loop a batched call. The existing `examples/m1a_bench.rs` sets `ntheta=lmax+1` and `nphi=2*lmax+3`
for its GL benchmark. Its workload dimensions are listed below so a different
sampling convention is not inferred from the T label.

| Workload | GL grid | Longitude FFTs per scalar operation | Longitude FFTs per spin operation |
| --- | ---: | ---: | ---: |
| T127 | 128 × 257 | 128 | 256 |
| T511 | 512 × 1,025 | 512 | 1,024 |
| T1023 | 1,024 × 2,049 | 1,024 | 2,048 |
| T2047 | 2,048 × 4,097 | 2,048 | 4,096 |

These GL cases have no meridional resampling FFT. CC and F1 analysis use
complex FFTs on reflected latitude profiles. The current RustSHT plan computes
`prepared_half=good_size_complex(lmax+1)`, `prepared_ntheta=prepared_half+1`,
and `prepared_full_len=2*prepared_half`. For a CC grid,
`original_full_len=2*(ntheta-1)`; for F1 it is `2*ntheta`. If resampling is
needed, `full_len=2*prepared_full_len`; otherwise it is `original_full_len`. The exact input, prepared, and full lengths are functions of grid kind and
latitude count, not of `nphi`. `CCResamplingPlan` owns forward and inverse RustFFT plans for the original,
prepared, and full lengths. It reuses one profile and one FFT scratch buffer. Its `to_prepared` and `restore_original` methods iterate Fourier orders
serially. Each order executes two transforms if resampling is unnecessary and
four if the full-resolution resize path is active. A scalar or spin resampling
call handles `m=0..=mmax`; spin components are passed through the same profile
plan separately.

## Storage and concurrency

Longitude map rows are contiguous, with row `r` beginning at `r*nphi`. The
half-spectrum rows are contiguous with length `nphi/2+1`. The complex
meridional profile is contiguous. Plans contain reusable FFT metadata; mutable
buffers live in `ShtWorkspace`. Independent SHT calls therefore use separate
workspaces. RustSHT-FFT now provides contiguous row-major batch methods that reuse
one caller-owned workspace and do not create a worker pool. They preserve row
order but currently execute one transform at a time; across-row SIMD remains
unimplemented. The batch API comparison is in
[`m4-batch-api.md`](m4-batch-api.md). All current RustSHT FFT paths are f64 or f32 through generic numeric bounds. The benchmark suite will include both precisions after the native kernels
support them.
