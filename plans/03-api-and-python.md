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
| `ShtWorkspace` | Caller-owned longitude and meridional FFT, Fourier and scalar/spin recurrence scratch; use a separate workspace for each concurrent execution. Worker scratch is sized from the active Rayon pool when the workspace is created. |
| `ShtError` | Structured invalid grid, undersampling, spin/mode, shape, overflow, unsupported configuration and allocation errors. |

Use a direct `num-complex` dependency and expose a `Complex64` alias. `GridSpec`, `HarmonicSpec`, `ShtPlan`, `ShtWorkspace`, `ShtError` and the alias are implemented. `HarmonicSpec::index(l, m)` returns the order-major packed offset; coefficient validation rejects nonreal `m=0` values and nonzero coefficients with `l < spin`. Scalar and spin-1 through spin-4 plans implement STANDARD `S`, `A`, `S*` and `A*`. Spin 3 and 4 pass full-operation checks on all three grids at `lmax=spin`; sparse STANDARD synthesis is also checked through T2047 and selected exact-analysis coefficients through T1023. Scalar plans also implement DERIV1 synthesis and its adjoint; spin plans implement GRAD_ONLY synthesis and its adjoint. Exact analysis requires `ntheta >= lmax+1` on GL/F1 and `ntheta >= lmax+2` on CC, with `mmax <= (nphi-1)/2`. Synthesis and synthesis adjoints accept `mmax <= nphi/2`, including the real Nyquist order. `S*` uses an unweighted map dot product and the real harmonic inner product with double weight for `m>0`. GL `A*` scales STANDARD synthesis by `ring_weight/nphi`; CC/F1 `A*` applies the adjoint of prepared-grid resampling. Spin 5 and above and f32 are not implemented. Current scalar GL usage:

```rust
use rustsht::{GridKind, GridSpec, HarmonicSpec, ShtPlan};

let grid = GridSpec::new(GridKind::GaussLegendre, 512, 1024, 0.0)?;
let harmonics = HarmonicSpec::scalar(511, 511)?;
let plan = ShtPlan::new(grid, harmonics)?;
let mut work = plan.workspace()?;
plan.synthesis_into(&alm, &mut map, &mut work)?;
plan.analysis_into(&map, &mut recovered, &mut work)?;
```

The Rust API uses separate methods for STANDARD `S`, `A`, `S*` and `A*`. `synthesis_gradient_into`, `adjoint_synthesis_gradient_into`, `synthesis_deriv1_into` and `adjoint_synthesis_deriv1_into` cover the supported synthesis-only modes. DERIV1 uses a spin-1 computation with scalar coefficients. Operations accept contiguous slices and return errors for invalid sizes, nonreal `m=0` coefficients, nonzero `l<spin` coefficients, spin above 4 and unsupported exact-analysis sampling. Use contiguous slices and small blocked workspaces in the numerical core. Do not require `ndarray` or a general matrix library. The plan selects runtime SIMD via `pulp` and the scalar path remains testable. Optional Rayon scheduling partitions synthesis by latitude groups and analysis by disjoint harmonic orders. Analysis splits coefficient ranges recursively and does not build a per-call task descriptor vector. The real-input FFT packing helper is private and specialized to longitude spectra.

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

`rustsht.ShtPlan(kind, ntheta, nphi, lmax, mmax, spin=0, phi0=0.0)` accepts `kind` values `"GL"`, `"CC"` and `"F1"`. Map rings run north to south, longitude samples run eastward, and `phi0` is in radians. The plan exposes scalar and spin STANDARD `synthesis`, `analysis`, `adjoint_synthesis` and `adjoint_analysis`, spin `synthesis_gradient` and its adjoint, and scalar `synthesis_deriv1` and its adjoint. Standard coefficient arrays use order-major packing and shapes `(1, nalm)` for scalar plans and `(2, nalm)` for spin plans. Standard maps use component-first shapes `(ncomp, ntheta, nphi)`. Gradient and derivative synthesis take one coefficient channel and return two map components. The top-level `synthesis_2d`, `analysis_2d`, `adjoint_synthesis_2d`, `adjoint_analysis_2d` and `get_gridweights` functions use the same packed coefficient layout and grid conventions. `analysis_2d` infers grid dimensions from the map; `adjoint_analysis_2d` takes `ntheta` and `nphi`. The analysis functions use STANDARD mode. Synthesis and adjoint synthesis accept `STANDARD`, `GRAD_ONLY` and `DERIV1`; `DERIV1` uses `spin=1` with one scalar coefficient channel. `get_gridweights` returns ring weights that sum to `4*pi`; divide by `nphi` for per-pixel weights. The binding accepts `complex128` coefficients and `float64` maps. Inputs must be C-contiguous; the binding copies them into Rust-owned vectors before releasing the GIL. Each call creates a workspace and returns a new NumPy array. Input shape and numerical-operation errors are reported as `ValueError`; an unsupported NumPy dtype is rejected during argument conversion. Exact-analysis sampling limits match the Rust API. Noncontiguous inputs, zero-copy borrowing, f32/complex64 and per-call thread control are not implemented.

## spharmgrid adapter boundary

spharmgrid's [`Grid`](https://github.com/mwyau/spharmgrid/blob/main/src/spharmgrid/grids.py) accepts latitude in either order and one full longitude cycle. Its [`grid_layout`](https://github.com/mwyau/spharmgrid/blob/main/src/spharmgrid/grids.py) converts to north-to-south, eastward order before calling the numerical backend; `grid_capabilities` implements the DUCC-safe GL/F1/CC analysis limits. The [`_ducc.py` adapter](https://github.com/mwyau/spharmgrid/blob/main/src/spharmgrid/_ducc.py) calls scalar analysis/synthesis, angular-derivative synthesis and spin-1 E/B vector transforms, with geographic wind conversion `(u,v) -> (-v,u)`. A RustSHT adapter should match that numerical boundary. spharmgrid, **not the Rust crate**, owns Xarray coordinates/CF metadata, truncation masks (including `lmin` and triangular/trapezoidal/rhomboidal modes), atmospheric filtering and radius units. Do not copy `TransformSpec`'s geophysical spectral-mask semantics into a low-level plan. The candidate `backend="rustsht"` option remains an API proposal; DUCC remains the default until parity is tested.

## Remaining choices

- f64/f32 kernel sharing and Python thread configuration.
- Strided and batched inputs and zero-copy borrowing.
