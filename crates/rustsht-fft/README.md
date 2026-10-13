# RustSHT-FFT

RustSHT-FFT is an independent Rust implementation of the one-dimensional real
and complex FFT algorithms used by DUCC. It provides reusable f32 and f64
plans with caller-owned workspaces for spherical harmonic transform software.

## References

- Reinecke, M., & Seljebotn, D. S. (2013). [Libsharp – spherical harmonic transforms revisited](https://doi.org/10.1051/0004-6361/201321494). *Astronomy & Astrophysics*, **554**, A112.
- Ishioka, K. (2018). [A New Recurrence Formula for Efficient Computation of Spherical Harmonic Transform](https://doi.org/10.2151/jmsj.2018-019). *Journal of the Meteorological Society of Japan*, **96**(2), 241–249.
- Reinecke, M., Belkner, S., & Carron, J. (2023). [Improved cosmic microwave background (de-)lensing using general spherical harmonic transforms](https://doi.org/10.1051/0004-6361/202346717). *Astronomy & Astrophysics*, **678**, A165.

## License

RustSHT-FFT is licensed under BSD-3-Clause. Notices for adapted DUCC source are
listed in `LICENSE-THIRD-PARTY`.
