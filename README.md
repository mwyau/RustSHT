# RustSHT

RustSHT implements spherical harmonic transforms in Rust, using [DUCC](https://github.com/mreineck/ducc) as its numerical reference. It supports scalar and spin-weighted synthesis and analysis, their adjoints, scalar derivatives and gradient-only synthesis on Gauss–Legendre (GL), Clenshaw–Curtis (CC) and Fejér-1 (F1) grids through Rust and Python APIs.

## Rust API migration

The former `rustsht::pyspharm` Rust module has been removed. Its numerical functions now live beside their mathematical owners:

| Former path | Current path |
| --- | --- |
| `rustsht::pyspharm::associated_legendre_{f32,f64}` | `rustsht::legendre::pointwise::associated_legendre_{f32,f64}` |
| `rustsht::pyspharm::specintrp_{f32,f64}` | `rustsht::evaluation::interpolate_spectrum_at_longitude_{f32,f64}` |
| `rustsht::pyspharm::synthesize_at_point(s)_{f32,f64}` | `rustsht::evaluation::synthesize_at_point(s)_{f32,f64}` |
| `rustsht::pyspharm::{GeodesicPoint, geodesic_points}` | `rustsht::grid::geodesic::{GeodesicPoint, geodesic_points}` |
| `GeodesicPoint::pyspharm_lat_lon_degrees` | `GeodesicPoint::legacy_lat_lon_degrees` |

The pointwise basis keeps its non-Condon–Shortley phase, packing order and degree-zero normalization. The Python package and `spharm_rs` API are unchanged.

## References

- Reinecke, M., & Seljebotn, D. S. (2013). [Libsharp – spherical harmonic transforms revisited](https://doi.org/10.1051/0004-6361/201321494). *Astronomy & Astrophysics*, **554**, A112.
- Ishioka, K. (2018). [A New Recurrence Formula for Efficient Computation of Spherical Harmonic Transform](https://doi.org/10.2151/jmsj.2018-019). *Journal of the Meteorological Society of Japan*, **96**(2), 241–249.
- Reinecke, M., Belkner, S., & Carron, J. (2023). [Improved cosmic microwave background (de-)lensing using general spherical harmonic transforms](https://doi.org/10.1051/0004-6361/202346717). *Astronomy & Astrophysics*, **678**, A165.

## License

[GPL-2.0-or-later](LICENSE). Copyright (c) 2026 Albert M. W. Yau and contributors.
