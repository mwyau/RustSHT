# Bounded f64 internals for f32 scalar synthesis

The public transform API continues to accept and return `f32`. For GL scalar
synthesis at T720, the plan may use the existing shared RustSHT `f64`
`ShtPlan` implementation internally when the geometry is one of these measured
cells:

- `lmax=720`, `mmax=720`, `ntheta=721`, `nphi=1458`;
- `lmax=720`, `mmax=180`, `ntheta=721`, `nphi=378`.

This policy reuses the existing f64 plan, recurrence and FFT implementation.
It does not maintain a second Legendre algorithm. Coefficients are promoted
into reusable `Complex64` workspace, the f64 transform writes a reusable f64
map, and results are narrowed into the caller's f32 output. q>1 conversions
use reusable 8192-element Rayon chunks. Plan and workspace setup is outside
the warmed transform timing; the whole-call performance evidence includes
both conversions, the transform and output writeback.

The adapter declines to the existing native-f32 transform when any input
component is nonfinite, subnormal, or has magnitude above `2^100`. It also
declines if the f64 transform errors, or if a nonzero f64 result narrows to a
f32 zero, subnormal or nonfinite value. A decline after narrowing is followed
by a complete native-f32 transform, which overwrites the output. Zero inputs
and zero f64 results are allowed. These guards preserve native behavior for
the exceptional ranges identified in the f32 comparison.

The f64 route can change final-bit rounding for ordinary finite inputs. It is
limited to the two measured T720 scalar-synthesis geometries; scalar analysis,
spin transforms, other bandlimits, grids and longitude sizes keep their native
f32 algorithms. The four routed cells passed independent analytic and adjoint
checks, including the T720 sectoral mode; input and output range declines and
poisoned-workspace reset were also tested. See the [M6 evidence ledger](../reports/DUCC_PARITY_M6.md)
for exact revisions, comparisons and remaining gaps.

This route adds reusable memory to an eligible f32 plan/workspace. Measured
full-mmax plan plus workspace use was 38.3 MB q1 and 42.8 MB q8, including
12.57 MB of conversion buffers; reduced-mmax use was 10.9 MB q1 and 13.8 MB
q8, including 4.01 MB of conversion buffers. After warm-up the transform made
zero allocations. The native-f32 parent used 8.6/10.1 MB full and 2.3/3.8 MB
reduced q1/q8. These figures are specific to the tested Rust build and
workspace capacities.
