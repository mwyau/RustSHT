# RustSHT contributor instructions

RustSHT is an independent GPL-2.0-or-later Rust library implementing spherical harmonic transform algorithms used by [DUCC](https://github.com/mreineck/ducc). DUCC is the numerical reference. Consult the [decisions](plans/00-scope-and-decisions.md), [numerical specification](plans/02-numerics-and-grids.md), and [DUCC operation crosswalk](plans/06-ducc-contract-crosswalk.md) before changing numerical code.

## Writing

Apply [write-like-a-scientist/SKILL.md](https://github.com/mwyau/write-like-a-scientist/blob/main/skills/write-like-a-scientist/SKILL.md), with its general-writing and research-software profiles, to documentation, comments and docstrings. State algorithms, constraints and observed results directly. Keep README.md to the project description, the 2013 libsharp, 2018 Ishioka and 2023 general-SHT citations, and the license. Keep implementation detail in `plans/`.

## Rust API

- Use stable Rust **1.89+** (Rust 2024). AVX-512 target features and many intrinsics are stable in Rust 1.89; detect required CPU subsets at runtime and preserve a portable fallback. Do not impose global AVX-512 flags in release builds.
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html). Prefer concrete validated structs/enums with private fields, inherent methods, `Result`-returning constructors and operations, and documented panics/errors. Do not create public trait hierarchies, unsafe stride interfaces or a generic FFT-backend plugin API without a concrete need.
- The Rust core owns numerical `GridSpec`, `HarmonicSpec`, `ShtPlan`, `ShtWorkspace`, packed coefficients and transform operations. Plans contain geometry, bandlimits and spin; operation-specific methods select valid modes. Use a direct `num-complex` dependency for public `Complex64`. Plans are reusable and hold no mutable execution scratch; caller-owned workspaces serve each invocation/worker. Use checked arithmetic and validate buffer lengths, channels, spin, modes and sampling.
- Core arrays use canonical north-to-south colatitude and equally spaced eastward longitude; `phi0` is in radians. Python and spharmgrid adapters handle user coordinate order, Xarray metadata, geographic winds, physical units and spectral filtering. The numerical core does not depend on Python, NumPy, Xarray or spharmgrid.
- Port DUCC's core numerical methods from the pinned GitHub source, including GL iterative/Bogaert quadrature, CC/F1 weights and meridional resampling, and scaled scalar/spin recurrences. Do not substitute `gauss-quad` or a generic matrix implementation. Retain applicable source notices for direct translations.
- Use RustFFT as the sole FFT engine. Implement a small private even-length real-input packing/Hermitian reconstruction helper around a half-length complex RustFFT; use a full-complex fallback for odd lengths. Do not add RealFFT or implement FFT butterflies. Test the helper against direct complex FFTs.
- Build GL kernels with scalar-reference checks and `pulp` SIMD (`x86-v4` for AVX-512), blocked latitude lanes, portable runtime dispatch, and optional coarse Rayon scheduling during M1. Reserve raw `std::arch` for measured gaps. Avoid `ndarray` in the numerical core and never share mutable scratch between workers.
- Pin uv to Python 3.11 with `.python-version`, matching `abi3-py311`. Keep `python/rustsht/py.typed` as a PEP 561 marker, with public typing stubs when numerical bindings are added.
- The Python binding crate, importable package and tests live together under `python/` (`python/Cargo.toml`, `python/src/`, `python/rustsht/`, and `python/tests/`), packaged by maturin through the root `pyproject.toml`. The Rust-only `rustsht` crate does not depend on PyO3. `uv sync` builds the editable Python package and installs the `dev` group (`ducc0`, `numpy`, `pytest`); `ducc0` is not a runtime dependency. The extension currently contains only a module initializer, no numerical transforms.
- Use **Albert M. W. Yau** for new code attribution. Cite Martin Reinecke and collaborators. If copyrightable source expression is copied/adapted/directly translated from DUCC, preserve its original license and copyright notices and record the revision. Do not copy `python/test/test_sht.py` without reviewing its GPL-3.0-or-later header; implement the numerical cases independently.

## Numerical rules

- **Approved order (D11):** implement the full applicable scalar, spin/vector, derivative, analysis/synthesis and adjoint operation family on GL first. Then use the same Legendre/spin kernels for CC and F1; the latter require specialized exact-analysis and adjoint resampling.
- Distinguish synthesis `S`, analysis `A`, and their separate adjoints `S*` and `A*`. DUCC rectangular `analysis_2d` and `adjoint_analysis_2d` use STANDARD only; GRAD_ONLY/DERIV1 modes belong to synthesis and its adjoint. Use the real-field harmonic inner product, with double weight on `m>0`, in adjoint tests.
- Exact analysis needs `ntheta >= lmax+1` on GL/F1, `ntheta >= lmax+2` on CC, and `mmax <= (nphi-1)/2` (integer division). Test minimum odd/even longitude counts; initial parity uses unit `ringfactor`. Do not impose analysis limits on synthesis without checking the actual operator requirements. Reject invalid inputs instead of truncating silently.
- Verify normalization/phase and FFT scaling against independently evaluated low-degree fields and pinned DUCC. Reject nonreal input `m=0` and nonzero `l<spin` coefficients. Maintain scaled normalized Legendre recurrences, polar cutoffs, reflection parity, spin signs and high-degree bounded memory use. Do not replace CC/F1 meridional Fourier resampling with direct low-resolution quadrature.

## Long autonomous implementation sessions

Use [the milestone and long-session rules](plans/05-validation-and-milestones.md) on a **new branch from main**. Continue through M1a (including DUCC GL quadrature, FFT adapter, SIMD, coarse threading and initial benchmarks), M1b/M1c, M2a/M2b and later stages as correctness and resources permit. The selections in [00](plans/00-scope-and-decisions.md) do not need routine confirmation. Commit working increments, document skipped tests and blockers, and never weaken tests to advance. Do not push unless authorized during the implementation session or merge into `main`. End with a numerical test and commit report.

## GitHub Actions

Never add temporary GitHub Actions workflows or jobs for lockfile generation, debugging, tests, or other one-off maintenance. Use local tooling or existing permanent workflows. If the required environment is unavailable, report the limitation instead of creating temporary automation.

## Tests and changes

- Compare numerical outputs against a documented DUCC reference version, recording the specific source revision or package version in each test/benchmark report. Pair DUCC comparisons with analytic harmonics, derivatives, grid/weight checks and adjoint identities. Roundtrips alone are not sufficient.
- Test `m=0`, `m=lmax`, `mmax<lmax`, `l<spin`, `phi0`, odd/even longitude, polar rings and high degrees; expand from f64 to f32. Publish the tested grid/precision/spin/operation limit, not an assumed universal maximum.
- Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, and `uv run pytest python/tests` for binding changes; report checks not run. Record CPU, compiler, precision, mode, threads, elapsed time and memory for benchmarks.
- Record approved scope changes in [00](plans/00-scope-and-decisions.md), API decisions in [03](plans/03-api-and-python.md), and validation evidence in [05](plans/05-validation-and-milestones.md). Avoid unrequested changes to DUCC or spharmgrid. No numerical SHT implementation exists in the repository yet.
