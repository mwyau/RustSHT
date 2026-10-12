# Literature, DUCC reference and licensing

## Scientific references

The README cites:

1. Reinecke and Seljebotn (2013), *Libsharp – spherical harmonic transforms revisited*, [doi:10.1051/0004-6361/201321494](https://doi.org/10.1051/0004-6361/201321494): isolatitude SHT, spin and CPU-oriented transform methods.
2. Ishioka (2018), *A New Recurrence Formula for Efficient Computation of Spherical Harmonic Transform*, [doi:10.2151/jmsj.2018-019](https://doi.org/10.2151/jmsj.2018-019): FMA-friendly associated-Legendre recurrence and high-bandlimit stability.
3. Reinecke, Belkner and Carron (2023), *Improved cosmic microwave background (de-)lensing using general spherical harmonic transforms*, [doi:10.1051/0004-6361/202346717](https://doi.org/10.1051/0004-6361/202346717): general-grid transforms, adjoints and NUFFT applications.
4. Waldvogel (2006), *Fast Construction of the Fejér and Clenshaw–Curtis Quadrature Rules*, [doi:10.1007/s10543-006-0045-4](https://doi.org/10.1007/s10543-006-0045-4): FFT-based CC/F1 quadrature construction cited in DUCC's `get_gridweights`. The 2023 paper is **not by itself a specification for high-bandlimit CC/F1 exact analysis**. Use the actual DUCC implementation and tests to define that specific behavior. Earlier libpsht history need not be cited in the short README.

## Reference source

- DUCC project: https://gitlab.mpcdf.mpg.de/mtr/ducc
- Readable mirror: https://github.com/mreineck/ducc
- Initially pinned comparison revision: `c1ed01b8fda69d2a378190f0915f1a19762dddbf` on `ducc0` (2026-10-09).
- SHT interface: `src/ducc0/sht/sht.h`
- Algorithms, geometry, quadrature, resampling and adjoints: `src/ducc0/sht/sht.cc`, `sht_inner_loop.h`, `sht_utils.h`
- Python signatures, geometry limits and modes: `python/sht_pymod.cc`
- Test structure: `python/test/test_sht.py`
- Operator definitions: `doc/sht.rst`

Treat the commit as the **reference baseline**, not a claim that DUCC is the upstream repository for RustSHT. Track subsequent reference revisions when accepted.

## License and provenance

RustSHT uses **GPL-2.0-or-later** by project choice. Code newly authored by Albert M. W. Yau and contributors is attributed to its authors. The source currently contains no copied DUCC SHT code. If material is **copied, adapted, or directly translated** from DUCC, inspect the license header on that specific file, retain original copyright and applicable license notices, record the reference revision and changes, and check distribution obligations. DUCC SHT implementation files are GPL-2.0-or-later; identified DUCC FFT files have a separate BSD-3-Clause option. These exceptions do not apply generically.

**Do not copy DUCC tests wholesale.** The inspected `python/test/test_sht.py` file carries a **GPL-3.0-or-later** header, unlike the C++ SHT implementation's GPL-2.0-or-later header. Reproduce the mathematical test *ideas* independently; if copying test source expression becomes necessary, review resulting GPL version compatibility and attribution explicitly. RustFFT is available under MIT/Apache-2.0 licenses. Preserve required notices for all concrete dependencies and future Python wheels. Do not assume mathematical ideas, independently written code, benchmark outputs and copied expression have identical copyright implications.

## Release record

Before code-bearing distribution, review SPDX headers, third-party notices, Cargo dependency licensing, source-availability requirements and binary/wheel composition. spharmgrid's independently developed BSD-3-Clause Python source does not automatically change its license because an optional GPL backend exists; the actual form of distribution and combination still matters. This document records scientific references, source provenance, and licensing requirements.
