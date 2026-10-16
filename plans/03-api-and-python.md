# Rust API, Python bindings and spharmgrid

## Design

RustSHT is a native Rust numerical library. Its public API should follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html) and expose **mathematical operations**, not C++ wrappers or Python array semantics. Favor concrete types and inherent methods with borrowed input/output slices. Constructors validate geometry, limits and operation mode; execution validates buffer lengths and return types. Public structs have private fields and useful accessors, so precomputation and layout can evolve without breaking callers. GL, CC and F1 use one public plan and operation family. Grid-specific code constructs the nodes and weights; CC/F1 analysis and their adjoints also use prepared-grid meridional resampling.

## Native Rust types

| Type | Purpose |
| --- | --- |
| `GridKind` | Implemented: `GaussLegendre`, `ClenshawCurtis`, `Fejer1`. |
| `GridSpec` | Validated `kind`, `ntheta`, `nphi`, `phi0` (radians); constructs GL, CC and F1 grids. Core order is north-to-south and eastward-cyclic. |
| `HarmonicSpec` | Inclusive `lmax`, `mmax` and nonnegative spin; validate spin bounds, real `m=0` and `l<spin` coefficient behavior. Mode is not plan state. Implemented with order-major packed `m >= 0` coefficients. |
| `ShtPlan` | Implemented for GL, CC and F1 scalar and spin-1 through spin-4 STANDARD synthesis, analysis and both adjoints, plus gradient and derivative synthesis and their synthesis adjoints; immutable FFT, quadrature, normalization and resampling metadata. Construct with `Result`. |
| `LegendreReuseEstimate` | Optional expected table-eligible STANDARD coefficient-to-map and map-to-coefficient execution counts for `ShtPlan::new_with_legendre_reuse_estimate`; `None` retains the existing automatic policy. |
| `ShtWorkspace` | Caller-owned longitude and meridional FFT, Fourier and scalar/spin recurrence scratch; use a separate workspace for each concurrent execution. Its execution capacity is independent of the active Rayon pool, and its recurrence-worker count is inspectable. |
| `ShtError` | Structured invalid grid, undersampling, spin/mode, shape, overflow, execution-capacity, unsupported configuration and allocation errors. |

The Rust precision API uses `ShtPlan<T = f64>` and `ShtWorkspace<T = f64>`, with implementations restricted to `f32` and `f64`; the default preserves existing `ShtPlan` call sites. `Complex32` and `Complex64` are the matching packed coefficient aliases. `GridSpec` and `HarmonicSpec` are precision-independent. `GridSpec` stores angular coordinates and quadrature weights as `f64`; plan construction converts the numerical metadata to the selected execution precision. The f32 recurrence uses scalar latitude-lane loops. The f64 recurrence uses runtime-dispatched `pulp` SIMD. The Python API accepts `float64` maps and `complex128` coefficients; f32 bindings have not been implemented. Use a direct `num-complex` dependency and expose `Complex32` and `Complex64` aliases. `GridSpec`, `HarmonicSpec`, `ShtPlan`, `ShtWorkspace` and `ShtError` are implemented. `HarmonicSpec::index(l, m)` returns the order-major packed offset; coefficient validation rejects nonreal `m=0` values and nonzero coefficients with `l < spin`. Scalar and spin-1 through spin-4 plans implement STANDARD `S`, `A`, `S*` and `A*` in f64 and f32. f64 spin 3 and 4 pass full-operation checks on all three grids at `lmax=spin`; sparse STANDARD synthesis is also checked through T2047 and selected exact-analysis coefficients through T1023. f32 scalar synthesis and analysis match DUCC 0.41.0 on GL, CC and F1 at T2; spin-1 GL synthesis matches DUCC at T1, and spin-1 sparse synthesis matches selected DUCC samples at T127. f32 scalar adjoints, spin 1–4 adjoints and gradient adjoints pass all-grid tests at `lmax=spin`; analytic derivative synthesis and its adjoint pass at T2. Scalar plans implement DERIV1 synthesis and its adjoint; spin plans implement GRAD_ONLY synthesis and its adjoint. Exact analysis requires `ntheta >= lmax+1` on GL/F1 and `ntheta >= lmax+2` on CC, with `mmax <= (nphi-1)/2`. Synthesis and synthesis adjoints accept `mmax <= nphi/2`, including the real Nyquist order. `S*` uses an unweighted map dot product and the real harmonic inner product with double weight for `m>0`. GL `A*` scales STANDARD synthesis by `ring_weight/nphi`; CC/F1 `A*` applies the adjoint of prepared-grid resampling. Spin 5 and above are unsupported. `GridSpec::fft_friendly_nphi(mmax)` is an explicit suggestion for the smallest longitude count that satisfies exact Fourier sampling and factors only into 2, 3, 5 and 7. It does not change a user-specified grid. Applications that require longitude oversampling for nonlinear products or a fixed external grid must choose that count separately.

### Legendre reuse estimate

The compact-table implementation has measured cold-to-warm break-even points for selected operation and pool combinations. The T85 atmospheric scalar synthesis retest lowered its prior estimate from 232 to 70 calls after five paired current-source processes. `ShtPlan::new` has no knowledge of a plan's expected lifetime, so the additive `LegendreReuseEstimate` and `ShtPlan::new_with_legendre_reuse_estimate` accept optional estimates for table-eligible coefficient-to-map and map-to-coefficient execution. Counts include the first invocation and only operations that can reach a table path for the plan. Scalar plans can use the map-to-coefficient table for STANDARD analysis and adjoint synthesis; spin-1 adjoint synthesis currently cannot use its analysis table and must not count toward that estimate. `None` preserves the existing deterministic Auto policy. A finite estimate admits cold table construction only where paired measurements establish a break-even point for the exact grid geometry, operation and executing pool; unmeasured cold cells use recurrence. Once the shared table has been built, setup is paid, and a positive estimate can use it where the current operation policy selects the table. Estimates are not consumed or calibrated. `Some(0)` keeps that execution direction on recurrence, including after table initialization. The table remains lazy and subject to existing memory and numerical-safety guards. The two execution directions are estimated independently before table construction; the policy does not add their counts to qualify a cold table build. See the [reuse estimate report](../reports/legendre-reuse-estimate-2026-10-14.md) for cold/warm samples and the calculation. Current scalar GL usage:

```rust
use rustsht::{GridKind, GridSpec, HarmonicSpec, ShtPlan};

let grid = GridSpec::new(GridKind::GaussLegendre, 512, 1024, 0.0)?;
let harmonics = HarmonicSpec::scalar(511, 511)?;
let plan = ShtPlan::new(grid, harmonics)?;
let mut work = plan.workspace()?;
plan.synthesis_into(&alm, &mut map, &mut work)?;
plan.analysis_into(&map, &mut recovered, &mut work)?;
```

### Workspace execution capacity

`ShtPlan::workspace()` requests the number of threads in the active Rayon pool as its initial execution capacity. Parallel builds cap this request at 64; builds without the `parallel` feature use capacity 1. `ShtPlan::prepare_workspace(&mut workspace, capacity)` explicitly changes the capacity and reserves the recurrence scratch required by that plan. For grids with at least 128 latitude rows and 257 longitude samples, it also reserves one private row-FFT workspace per requested worker. Smaller grids keep serial row FFT execution and do not allocate that scratch. The supported range is `1..=64` with `parallel` and `1` without it. A transform uses at most the smaller of the workspace capacity and the active pool size. The workspace retains allocated scratch when its capacity is reduced, and a pool change alone does not increase capacity or allocate worker scratch. Keep capacity 1 when an outer Rayon pool schedules independent fields and each transform has its own workspace. To move an existing workspace from q1 to q8 for one transform at a time, prepare it before executing in the destination pool:

```rust
let mut work = plan.workspace()?;
plan.prepare_workspace(&mut work, 8)?;
plan.synthesis_into(&alm, &mut map, &mut work)?;
```

Run the prepared workspace from a Rayon pool with eight threads when that execution capacity is available. `workspace.execution_capacity()` returns the requested execution limit. `workspace.allocated_worker_count()` returns the current number of recurrence scratch slots, which can be larger than the execution limit because some synthesis schedulers overpartition groups for load balancing. Private row-FFT workspace slots are not included in this count. These accessors distinguish requested concurrency from recurrence scratch; the active Rayon pool supplies the third limit on actual workers. The Rust API uses separate methods for STANDARD `S`, `A`, `S*` and `A*`. `synthesis_gradient_into`, `adjoint_synthesis_gradient_into`, `synthesis_deriv1_into` and `adjoint_synthesis_deriv1_into` cover the supported synthesis-only modes. DERIV1 uses a spin-1 computation with scalar coefficients. Operations accept contiguous slices and return errors for invalid sizes, nonreal `m=0` coefficients, nonzero `l<spin` coefficients, spin above 4 and unsupported exact-analysis sampling. Use contiguous slices and small blocked workspaces in the numerical core. Do not require `ndarray` or a general matrix library. The plan selects runtime SIMD via `pulp` and the scalar path remains testable. Optional Rayon scheduling partitions synthesis by latitude groups and analysis by disjoint harmonic orders. Analysis splits coefficient ranges recursively and does not build a per-call task descriptor vector. The real-input FFT packing helper is private and specialized to longitude spectra.

## Code organization

Start with a **single Rust library crate** rather than a premature multi-crate workspace. Suggested modules:

```text
src/
  lib.rs                 # Public exports and crate documentation
  error.rs               # Result/error types
  grid.rs                # GridKind, GridSpec, GL/CC/F1 geometry
  harmonics.rs           # HarmonicSpec, packed alm indexing
  plan.rs                # ShtPlan and precomputed constants
  workspace.rs           # Explicit mutable FFT/Legendre scratch
  transform.rs           # Public transform methods and dispatch
  fft.rs                 # RustFFT plus a small real-input packing adapter
  legendre/
    mod.rs               # Shared recurrence metadata
    scalar.rs            # Spin-0 normalized recurrence
    spin.rs              # Spin-weighted recurrence
  meridional.rs          # CC/F1 preparation and adjoint
  kernels/               # Scalar reference and SIMD dispatch (when used)
```

These are design boundaries, not instructions to create empty modules. Start with the minimum modules actually used. Hide implementation helpers with `pub(crate)` or private visibility and expose only stable numerical types. Keep coefficient and ring layout well documented and use checked indexing. The root Cargo workspace includes the separate `python/` PyO3 extension crate alongside `python/rustsht/` and `python/tests/`. Only this member depends on PyO3; the core `rustsht` crate stays Rust-only. `python/rustsht` contains the Python package, and maturin builds `rustsht._rustsht`. Do not add a new workspace member for each numerical subsystem.

## Python binding scaffold

The uv development environment pins Python 3.11 with `.python-version`, consistent with the PyO3 `abi3-py311` baseline. The `python/rustsht/py.typed` PEP 561 marker and `__init__.pyi` stub are installed with the package. Generate Cargo and uv lockfiles with their own package managers where registries are accessible; only then enable locked installs in permanent CI. Do not create temporary GitHub Actions jobs to generate lockfiles. The root `pyproject.toml` uses maturin as its build backend. Its `[tool.maturin]` settings point to the `python` Cargo member and build the native extension as `rustsht._rustsht`, alongside `python/rustsht/__init__.py`. The Python project version is `0.1.0.dev0` (PEP 440 form) and the Rust packages use `0.1.0-dev.0` (Cargo semver form). NumPy is a runtime dependency for the array API; the `dev` group contains DUCC, NumPy and pytest. DUCC is not a runtime dependency. Source and wheel distributions include the project GPL license and `LICENSE-THIRD-PARTY`, which records the selected licenses for resolved Rust dependencies. `uv sync` builds and installs the local Python package. `uv run pytest python/tests` checks the compiled extension, numerical calls, array validation and DUCC comparisons. `cargo test --workspace` checks both Rust crates. The extension exposes the verified f64 operations through `ShtPlan`. The Python API exposes a reusable plan class and top-level procedural functions. The transforms use DUCC's unit `ringfactor`; arbitrary per-ring scaling is not exposed.

## Python binding

`rustsht.ShtPlan(kind, ntheta, nphi, lmax, mmax, spin=0, phi0=0.0)` accepts `kind` values `"GL"`, `"CC"` and `"F1"`. Map rings run north to south, longitude samples run eastward, and `phi0` is in radians. The plan exposes scalar and spin STANDARD `synthesis`, `analysis`, `adjoint_synthesis` and `adjoint_analysis`, spin `synthesis_gradient` and its adjoint, and scalar `synthesis_deriv1` and its adjoint. Standard coefficient arrays use order-major packing and shapes `(1, nalm)` for scalar plans and `(2, nalm)` for spin plans. Standard maps use component-first shapes `(ncomp, ntheta, nphi)`. Gradient and derivative synthesis take one coefficient channel and return two map components. The top-level `synthesis_2d`, `analysis_2d`, `adjoint_synthesis_2d`, `adjoint_analysis_2d` and `get_gridweights` functions use the same packed coefficient layout and grid conventions. `analysis_2d` infers grid dimensions from the map; `adjoint_analysis_2d` takes `ntheta` and `nphi`. The analysis functions use STANDARD mode. Synthesis and adjoint synthesis accept `STANDARD`, `GRAD_ONLY` and `DERIV1`; `DERIV1` uses `spin=1` with one scalar coefficient channel. `get_gridweights` returns ring weights that sum to `4*pi`; divide by `nphi` for per-pixel weights. The binding accepts C-contiguous `complex128` coefficients and `float64` maps. It borrows input slices while numerical work runs with the GIL released, then returns a newly allocated NumPy array. The reusable plan also provides `synthesis_into(alm, out)` and `analysis_into(map, out)` for standard scalar and spin operations; these require an exact-shape, C-contiguous, writable output of the expected dtype and reject input/output overlap. Keep both arrays alive and do not access or mutate either through another alias until the call returns. Each Python plan lazily creates and reuses one workspace. Calls on the same plan serialize while they use that workspace; separate plan instances execute independently. Procedural functions create a plan and workspace per call. Input shape and numerical-operation errors are reported as `ValueError`; an unsupported NumPy dtype is rejected during argument conversion. Exact-analysis sampling limits match the Rust API. Noncontiguous inputs, f32/complex64 and per-call thread control are not implemented.

## spharmgrid adapter boundary

spharmgrid's [`Grid`](https://github.com/mwyau/spharmgrid/blob/main/src/spharmgrid/grids.py) accepts latitude in either order and one full longitude cycle. Its [`grid_layout`](https://github.com/mwyau/spharmgrid/blob/main/src/spharmgrid/grids.py) converts to north-to-south, eastward order before calling the numerical backend; `grid_capabilities` implements the DUCC-safe GL/F1/CC analysis limits. The [`_ducc.py` adapter](https://github.com/mwyau/spharmgrid/blob/main/src/spharmgrid/_ducc.py) calls scalar analysis/synthesis, angular-derivative synthesis and spin-1 E/B vector transforms, with geographic wind conversion `(u,v) -> (-v,u)`. A RustSHT adapter should match that numerical boundary. spharmgrid, **not the Rust crate**, owns Xarray coordinates/CF metadata, truncation masks (including `lmin` and triangular/trapezoidal/rhomboidal modes), atmospheric filtering and radius units. Do not copy `TransformSpec`'s geophysical spectral-mask semantics into a low-level plan. The candidate `backend="rustsht"` option remains an API proposal; DUCC remains the default until parity is tested.

## Remaining choices

- f32 SIMD support and Python thread configuration.
- Strided and batched inputs.
