# Rust API, Python bindings and spharmgrid

## Design

RustSHT is a native Rust numerical library. Its public API should follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html) and expose **mathematical operations**, not C++ wrappers or Python array semantics. Favor concrete types and inherent methods with borrowed input/output slices. Constructors validate geometry, limits and operation mode; execution validates buffer lengths and return types. Public structs have private fields and useful accessors, so precomputation and layout can evolve without breaking callers. GL, CC and F1 use one public plan and operation family. Internally, grid-specific modules produce nodes, weights and CC/F1 meridional resampling; the scalar and spin recurrence drivers do not know whether the target grid is GL, CC or F1. Keep numerical normalization explicit.

## Draft native Rust types

| Type | Purpose |
| --- | --- |
| `GridKind` | `GaussLegendre`, `ClenshawCurtis`, `Fejer1` |
| `GridSpec` | Validated `kind`, `ntheta`, `nphi`, `phi0` (radians). Core order is north-to-south and eastward-cyclic. |
| `HarmonicSpec` | Inclusive `lmax`, `mmax`, nonnegative spin and mode; validate channels and `l<spin` behavior. |
| `TransformMode` | `Standard`, `GradientOnly`, `FirstDerivative`. Enforce DUCC operation/mode applicability. |
| `ShtPlan` | Immutable reusable FFT plans, quadrature and recurrence coefficients; chosen CPU dispatch. Construct with `Result`. |
| `ShtWorkspace` | Explicit execution scratch, separate for each concurrent execution/worker; size derived from plan. |
| `ShtError` | Structured invalid grid, undersampling, spin/mode, shape, overflow, unsupported configuration and allocation errors. |

`rustfft::num_complex::Complex<f64>` may be used for packed coefficients to avoid a separate complex dependency initially; decide whether to re-export a stable `Complex64` alias before a public release. Generalize to f32 only when validated. Do not preemptively expose internal FFT types, planner traits, SIMD implementations, or thread-pool types. Provisional usage (illustration, **not implemented or finalized**):

```rust
use rustsht::{GridKind, GridSpec, HarmonicSpec, ShtPlan};

let grid = GridSpec::new(GridKind::GaussLegendre, 512, 1024, 0.0)?;
let harmonics = HarmonicSpec::scalar(511, 511)?;
let plan = ShtPlan::new(grid, harmonics)?;
let mut work = plan.workspace()?;
plan.synthesis_into(&alm, &mut map, &mut work)?;
plan.analysis_into(&map, &mut recovered, &mut work)?;
```

The signatures, error type, scratch lifetime and zero-allocation guarantees need a proof-of-concept. Implement `S`, `A`, `S*` and `A*` as separate methods with real mathematical semantics. A plan for non-STANDARD synthesis must reject unsupported analysis calls instead of implying that an unimplemented derivative analysis exists. Offer plan accessors for coefficient count, component count, grid coordinates, weights and safe analysis degree.

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
  fft.rs                 # Private RustFFT integration
  legendre/
    mod.rs               # Shared recurrence metadata
    scalar.rs            # Spin-0 normalized recurrence
    spin.rs              # Spin-weighted recurrence
  meridional.rs          # CC/F1 preparation and adjoint
  kernels/               # Scalar, SIMD and architecture dispatch later
```

These are design boundaries, not instructions to create empty modules. Start with the minimum modules actually used. Hide implementation helpers with `pub(crate)` or private visibility and expose only stable numerical types. Keep coefficient and ring layout well documented and use checked indexing. The root Cargo workspace includes the separate `python/` PyO3 extension crate alongside `python/rustsht/` and `python/tests/`. Only this member depends on PyO3; the core `rustsht` crate stays Rust-only. `python/rustsht` contains the Python package, and maturin builds `rustsht._rustsht`. Do not add a new workspace member for each numerical subsystem.

## Python binding scaffold

The uv development environment pins Python 3.11 with `.python-version`, consistent with the PyO3 `abi3-py311` baseline. The `python/rustsht/py.typed` PEP 561 marker is installed with the package. Add stubs for public numerical bindings when implemented. Generate Cargo and uv lockfiles with their own package managers where registries are accessible; only then enable locked installs in permanent CI. Do not create temporary GitHub Actions jobs to generate lockfiles. The root `pyproject.toml` uses maturin as its build backend. Its `[tool.maturin]` settings point to the `python` Cargo member and build the native extension as `rustsht._rustsht`, alongside `python/rustsht/__init__.py`. The Python project version is `0.1.0.dev0` (PEP 440 form) and the Rust packages use `0.1.0-dev.0` (Cargo semver form). The `dev` group contains DUCC, NumPy and pytest; the published runtime dependency list is empty. Run `uv sync` to build/install the local Python package, then `uv run pytest python/tests` to check import of the compiled extension. `cargo test --workspace` checks both Rust crates. No SHT functions or fictitious placeholders are exposed by the extension yet. Add Python SHT bindings only after the corresponding native Rust operations are implemented and verified.

## Python binding

Expose two API styles if useful: DUCC-familiar `analysis_2d`, `synthesis_2d`, `adjoint_synthesis_2d`, `adjoint_analysis_2d` and `get_gridweights`; and a reusable Python plan class. Accept `float64/complex128` first, later `float32/complex64`. Validate shapes `(ncomp, ntheta, nphi)` and packed coefficient arrays, release the GIL for computation, and document when NumPy inputs are copied. Noncontiguous array support and zero-copy borrowing are later measured design decisions; avoid undefined aliasing.

## spharmgrid adapter boundary

spharmgrid's [`Grid`](https://github.com/mwyau/spharmgrid/blob/main/src/spharmgrid/grids.py) accepts latitude in either order and one full longitude cycle. Its [`grid_layout`](https://github.com/mwyau/spharmgrid/blob/main/src/spharmgrid/grids.py) converts to north-to-south, eastward order before calling the numerical backend; `grid_capabilities` implements the DUCC-safe GL/F1/CC analysis limits. The [`_ducc.py` adapter](https://github.com/mwyau/spharmgrid/blob/main/src/spharmgrid/_ducc.py) calls scalar analysis/synthesis, angular-derivative synthesis and spin-1 E/B vector transforms, with geographic wind conversion `(u,v) -> (-v,u)`. A RustSHT adapter should match that numerical boundary. spharmgrid, **not the Rust crate**, owns Xarray coordinates/CF metadata, truncation masks (including `lmin` and triangular/trapezoidal/rhomboidal modes), atmospheric filtering and radius units. Do not copy `TransformSpec`'s geophysical spectral-mask semantics into a low-level plan. The candidate `backend="rustsht"` option remains an API proposal; DUCC remains the default until parity is tested.

## Remaining choices

- Plan mode at construction or per method; exact spin coverage and derivative mode shapes.
- Public re-export of `num_complex`, scalar generic implementation versus f64/f32 separate kernels.
- Contiguous versus strided/batch buffers; thread configuration and `Send`/`Sync` guarantees.
- Independent PyPI package metadata and GPL wheel/source distribution.
