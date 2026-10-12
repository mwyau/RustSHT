# RustSHT

RustSHT is being developed to implement spherical harmonic transform algorithms used by [DUCC](https://github.com/mreineck/ducc) in Rust, using [RustFFT](https://github.com/ejmahler/RustFFT). The planned library targets high-bandlimit scalar, vector and spin-weighted analysis, synthesis, derivatives and adjoints on Gauss–Legendre (GL), Clenshaw–Curtis (CC) and Fejér-1 (F1) grids, with native Rust and Python interfaces. Numerical transforms are not implemented yet.

## References

- Reinecke, M., & Seljebotn, D. S. (2013). [Libsharp – spherical harmonic transforms revisited](https://doi.org/10.1051/0004-6361/201321494). *Astronomy & Astrophysics*, **554**, A112.
- Ishioka, K. (2018). [A New Recurrence Formula for Efficient Computation of Spherical Harmonic Transform](https://doi.org/10.2151/jmsj.2018-019). *Journal of the Meteorological Society of Japan*, **96**(2), 241–249.
- Reinecke, M., Belkner, S., & Carron, J. (2023). [Improved cosmic microwave background (de-)lensing using general spherical harmonic transforms](https://doi.org/10.1051/0004-6361/202346717). *Astronomy & Astrophysics*, **678**, A165.

## License

[GPL-2.0-or-later](LICENSE). Copyright (c) 2026 Albert M. W. Yau and contributors.
