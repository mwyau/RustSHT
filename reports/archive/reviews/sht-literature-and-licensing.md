# Spherical harmonic transform literature, implementations, and licensing

*Research and source-license review — 2026-10-18 (US Eastern); current RustSHT development branch `opt/sht-performance-parity`.*

> **Scope / caution.** This is a technical research review and a practical licensing risk assessment, **not legal advice**. Check the license headers and dependency tree for the **exact files and revisions** before any incorporation or redistribution. A repository-wide license badge does not override exceptional per-file terms, patents, or third-party dependency licenses. The classification below concerns source reuse, not permission to independently implement published mathematics.

## Decision in brief

RustSHT's [`Cargo.toml`](../../../Cargo.toml) and [README](../../../README.md) state **`GPL-2.0-or-later`**, not `GPL-2.0-only`. This preserves an important optional GPLv2 distribution path. The root [`LICENSE`](../../../LICENSE) reproduces GPL version 2; the version-choice notice appears in Cargo/README, so keep that choice explicit on original source-file headers and on contributions.

**Recommended policy:** implement new numerical techniques from published papers and original derivations, and use reference implementations as validation or for ideas. Directly adapt source only from **license-compatible material** with the required notices. Maintain a repository build/distribution that is genuinely usable under GPLv2 if that remains a goal. Do **not** import GPLv3-only or Apache-2.0 source into that code path. A combined derivative can instead be distributed under GPLv3, but then the **combined program** is no longer distributable under GPLv2. A new independently authored algorithm can remain GPL-2.0-or-later.

### What the GPL version choice actually means

- **GPL-2.0-only:** cannot combine copyright-covered GPLv3-only source or Apache-2.0 source into a program distributed under GPLv2.
- **GPL-2.0-or-later:** recipients may choose GPLv2 or a later GPL version for original code carrying that grant. This enables combining with GPLv3-only or Apache-2.0 code **by conveying the resulting combined program under GPLv3**, not by converting GPLv3/Apache code into GPLv2 code. The individual upstream files' notices still apply.
- **MIT/BSD-3-Clause:** permissive and compatible with GPLv2 and GPLv3. Keep the original copyright and permission notices when copying/adapting substantial code; retain provenance in [third-party notices](../../../LICENSE-THIRD-PARTY) as appropriate.
- **CeCILL-2/2.1:** contains an explicit GPL compatibility clause (5.3.4); check the exact upstream version and file headers, but it is generally possible to include covered code in a GPLv2 or later combination pursuant to that clause and its notice obligations. Do **not** confuse basic CeCILL with CeCILL-B/C.
- **No express software license / all rights reserved:** public access to a Git repository is **not** a grant to copy, translate, sublicense or distribute copyright-covered source. Request permission or independently implement mathematics from papers.
- **Algorithm vs source:** mathematical ideas, methods, formulas and benchmark-defined behavior are distinct from protectable software expression. An original Rust implementation from the published method is usually a sound strategy and is **not automatically a derivative** merely because its results match. Avoid copying protected code structure, distinctive detailed pseudocode or substantial tables verbatim. A documented separation between a paper-derived specification and independent implementation is extra protection when a repo has restrictive/unclear licensing. A clean room is **not categorically mandatory**; it does not itself clear patent rights or license copied code.
- **Do not use an LLM as a license washer.** Translating or paraphrasing existing GPLv3/Apache/no-license C/C++/Python source into Rust may still be copyright-derived. Do not treat syntax changes as an independent implementation.
- **Ancillary materials:** tests, scripts, tables, data and generated code may have **different** licenses. For example, the [RustSHT agent policy](../../../AGENTS.md) notes DUCC's GPL-3.0-or-later Python tests: do not copy them verbatim into a GPLv2-compatible Rust test suite. Independently recreate test cases from public numerical specifications.

Primary GPL guidance: [FSF GPL compatibility FAQ](https://www.gnu.org/licenses/gpl-faq.en.html#v2v3Compatibility); [FSF license list](https://www.gnu.org/licenses/license-list.en.html); [FSF clear GPL version identification](https://www.gnu.org/licenses/identify-licenses-clearly.html). These describe legal license compatibility, not technical performance.

## Source-by-source reuse matrix

Legend: **Direct port** means source-derived C/C++/Python/Fortran → Rust translation/adaptation; **paper implementation** means original code based on mathematical descriptions without copying source expression. Status is verified from linked upstream declarations as of this review, subject to revision and per-file checks.

| Code / relevant algorithms | Verified upstream status | Direct code into GPL-2.0-or-later RustSHT? | Independent paper implementation? | RustSHT recommendation |
| --- | --- | --- | --- | --- |
| [DUCC](https://github.com/mreineck/ducc) — recurrence, scaling, spin, FFT | **GPL-2.0-or-later** for source; selected FFT files additionally **BSD-3-Clause** (dual option), per [README](https://github.com/mreineck/ducc#licensing-terms) | **Yes**, preserving applicable headers, modifications and provenance; select BSD for qualifying individual FFT sources only after checking their headers | Yes | **Existing primary reference**. Continue faithful validated ports; annotate actual source origin |
| [SHTns](https://nschaeff.bitbucket.io/shtns/) — Ishioka-style recurrences, optimized spin | **CeCILL** (documentation identifies **CeCILL 2.1**; confirm exact downloaded release and individual source headers) | **Generally yes through CeCILL's GPL compatibility clause 5.3.4**, with correct notices; confirm on specific release | Yes | Credible CPU alternative; do not assume it is ordinary GPLv2-licensed code |
| [FastTransforms](https://github.com/MikaelSlevinsky/FastTransforms) — fast polynomial conversions and spherical-to-Fourier | **MIT**; [license](https://github.com/MikaelSlevinsky/FastTransforms/blob/master/LICENSE) names Slevinsky and Dawes | **Yes**, for its own MIT code with retained MIT copyright/license text | Yes | **Best accessible alternate CPU source implementation**, but inspect dependent libraries and individual files |
| [S2FFT](https://github.com/astro-informatics/s2fft) — spin/Wigner-d algorithms | **MIT**, [LICENCE.txt](https://github.com/astro-informatics/s2fft/blob/main/LICENCE.txt) | **Yes**, for MIT-covered source with notices | Yes | Strong spin-1/2 reference; Python/JAX/PyTorch dependencies and GPU design are not intrinsic to a native Rust algorithm |
| [fastSHT](https://github.com/liuhao-cn/fastSHT) — batched many-map SHT | Repo [LICENSE](https://github.com/liuhao-cn/fastSHT/blob/main/LICENSE) and metadata say **GPLv3**; no verified 'or later' grant for the relevant code, so conservatively treat as GPL-3.0-only pending source-header review | **Not if retaining a GPLv2 distribution option**. A GPLv3 **combined** distribution can be possible with RustSHT's GPL-2.0-or-later code, preserving licenses | **Yes**, if genuinely independently coded from [paper](https://arxiv.org/abs/2208.10154) | Use the paper for batch GEMM/blocking and the repo for external performance/correctness comparison; avoid translating implementation if retaining GPLv2 availability |
| [ECMWF ecTrans](https://github.com/ecmwf-ifs/ectrans) — operational global Legendre/FFT/transforms and fast Legendre | **Apache-2.0** ([LICENSE](https://github.com/ecmwf-ifs/ectrans/blob/develop/LICENSE), [README](https://github.com/ecmwf-ifs/ectrans)); **open source** since standalone packaging in 2022 | **Not for a GPLv2-conveyable combined work**. Combining covered Apache-2.0 source with GPL-2.0-or-later project normally requires a **GPLv3** combined distribution | **Yes**, from publications/independently derived algorithms | Important **correction to older review:** operational implementation *is publicly inspectable*. Great reference, but do not translate covered Apache code if preserving GPLv2 distribution |
| [ButterflyLab](https://github.com/ButterflyLab/ButterflyLab) — IDBF / butterfly research | README explicitly says **'copyright reserved'**; no verified permissive source grant | **No**, without permission or verified license for particular files | **Yes**, from published butterfly/IDBF literature | Treat as **research/oracle only**. An academic GitHub repo is not automatically licensed for reuse |
| [cunuSHT](https://github.com/Sebastian-Belkner/cunuSHT) — NUFFT-based irregular spin SHT | No root or nested license file found in checked GitHub tree; no verified grant | **No**, absent specific permission/license; separately verify dependencies too | **Yes**, from [paper](https://arxiv.org/abs/2406.14542) | Useful for nonuniform grids, not a default CPU GL backend |
| Wavemoth — compressed/butterfly SHT | Historical research code and paper, [arXiv:1110.4874](https://arxiv.org/abs/1110.4874); **current authoritative source license not established in this review** | **Unresolved: do not copy yet** | **Yes**, implement from the published mathematical description | Historical performance comparison; seek original distribution and exact headers before attempting a source port |

**Do not assume dependency licenses are inherited from a top-level package.** FastTransforms' [README](https://github.com/MikaelSlevinsky/FastTransforms) lists FFTW3, OpenBLAS and MPFR. In particular [FFTW is GPL-2.0-or-later](https://ftp.fftw.org/doc/License-and-Copyright.html) and not MIT merely because FastTransforms itself is MIT. GPLv2-or-later FFTW is GPL-compatible with RustSHT, but adding direct FFTW linkage would alter the no-FFI/native design and redistribution obligations. Check each dependency's *actual selected license option* and whether it is used merely for an isolated research benchmark or incorporated into a distributed binary.

## Algorithms and implementation maturity

| Technique and literature | Numerical opportunity | Existing implementation and maturity | Initial Rust experiment |
| --- | --- | --- | --- |
| **IDBF / butterfly Legendre**, Bremer–Chen–Yang, *SIAM J. Sci. Comput.* (2021), [DOI 10.1137/20M1333845](https://doi.org/10.1137/20M1333845), [preprint](https://arxiv.org/abs/2004.11346) | Compressed complementary low-rank associated Legendre matrices; asymptotic improvement at high bandwidth with factor setup/storage | ButterflyLab research implementation, **not licensed for reuse** under a verified open license | Standalone, independently authored prototype for one `m` and GL grid; test `A_m`, transpose, error, preprocessing, memory and amortization over reused plans |
| **Operational fast Legendre**, Wedi–Hamrud–Mozdzynski (2013) and ECMWF IFS/ecTrans | Strong evidence that fast Legendre transforms can work at NWP scale | **Production/operational ecTrans**, Apache-2.0 source now public; MPI/GPU/BLAS/FFTW stack more expansive than RustSHT | Read papers, compare timings and workload design; independently implement only if demonstrated benefit and licensing appropriate |
| **Spherical-to-bivariate Fourier**, Slevinsky (2019), [DOI 10.1016/j.acha.2017.11.001](https://doi.org/10.1016/j.acha.2017.11.001), [preprint](https://arxiv.org/abs/1705.05448) | Fast orthogonal polynomial/basis conversion, especially appealing for equiangular or CC/F1 | **FastTransforms** MIT numerical library; not every packaged kernel delivers the paper's optimal asymptotic complexity | Review numerical conversion and stability; run independent microbench before architecting a new production backend |
| **Many-field batched SHT**, Tian and collaborators (2022), [arXiv:2208.10154](https://arxiv.org/abs/2208.10154) | Reuse same geometry and Legendre coefficients across fields via BLAS-3/matrix panels | **fastSHT** GPLv3 research implementation, relatively old upstream source | **Lower-risk first experiment** for batches `B=1,4,16,64`; generate bounded, reusable panels instead of huge precomputed dense matrices; preserve single-map fast route |
| **Spin-weighted Wigner-d transforms**, Price–McEwen (2024), [DOI 10.1016/j.jcp.2024.113109](https://doi.org/10.1016/j.jcp.2024.113109), [S2FFT](https://github.com/astro-informatics/s2fft) | Differentiable, parallelizable spin transforms; stability and recurrence alternatives | Maintained **MIT S2FFT** package; strong reference, not proven faster than DUCC on CPUs | Evaluate standalone spin-1 recurrence and exact adjoint/numerical properties before replacing live code |
| **Nonoscillatory phase/Jacobi evaluation**, Bremer and related work, e.g. [phase-function paper](https://arxiv.org/search/?query=Bremer+nonoscillatory+associated+Legendre&searchtype=all) | Alternative high-degree pointwise Legendre evaluation, potential factor generation | Primarily scientific algorithm literature; FastTransforms implements related polynomial-transform ideas | Research/precompute only; no demonstrated full SHT speedup on current CPU workloads |
| **Irregular/grid-general SHT**, cunuSHT [2024 paper](https://arxiv.org/abs/2406.14542) | NUFFT and double-Fourier-sphere route for irregular points, GPU | Public research code **without verified reuse license** | Defer unless introducing an irregular-grid API |
| **Partial-region transforms**, Carron–Reinecke (2026), [arXiv:2603.17166](https://arxiv.org/abs/2603.17166) | Regional/partial-sky fast transform, not ordinary global GL | Paper; source/reuse license not verified here | Out of current global SHT scope |
| **Manifold butterfly harmonics**, Beckman et al. (2026), [arXiv:2605.21828](https://arxiv.org/abs/2605.21828) | Generalizes structured harmonics beyond the sphere | Research, not validated as an ordinary spherical GL backend | Defer |

**Publication claims are not current comparative benchmarks.** Production baseline is RustSHT's current `src/legendre/scalar_ordered.rs` with DUCC comparisons. Any novel algorithm should first demonstrate reproducible **whole-transform** or amortized per-field improvement, numerical error, setup cost and memory use on realistic, varied bandlimits. Avoid replacing stable spin/adjoint/grid behavior to chase asymptotic complexity alone.

## Port-versus-independent-implementation policy for coding agents

1. **Classify before reading detailed code:** note upstream URL, exact commit, file path, file license/header, copyright holder, and whether the goal is *source adaptation* or *new paper implementation*. Do not conflate scientific citation with permission to copy source.
2. **Green — direct source adaptation:** existing GPL-2.0-or-later DUCC, confirmed MIT FastTransforms/S2FFT, selected BSD-3-Clause DUCC FFT sources, or correctly identified GPL-compatible CeCILL source. Preserve applicable header/notices, document that code was ported/adapted, track URL and commit. Maintain the project's own Rust names rather than labeling classes after competitors.
3. **Yellow — current RustSHT GPLv2 option would be affected:** GPLv3 fastSHT or Apache-2.0 ecTrans source. Either (a) use those programs strictly as *independent benchmarking or numerical oracles* and derive Rust code from papers, or (b) explicitly choose a GPLv3 combined distribution after owner approval, with a per-file license inventory and packaging review. **Do not silently make this project GPLv3-only** through a source port.
4. **Red — rights unclear/restricted:** ButterflyLab, cunuSHT and Wavemoth until specific terms are verified. **No copying, translating, generating code from pasted source, or importing opaque code/tables** into the production repository without express permission. Independent mathematical reconstruction from papers is acceptable subject to patent and other rights checks.
5. **Tests/data and other rights:** paper citations and equations are fine, but use original text, original tests/analytic constructions and independently generated fixtures. Do not copy GPLv3-only Python test code from DUCC. Preserve regression evidence. Avoid adding FFTW/BLAS/other FFI dependencies to the production core merely because a reference implementation uses them.
6. **Separate implementation evidence from licensing:** independent validation against a reference library does not copy its implementation. Static source comparisons can be used to understand high-level algorithms, but for a restrictive-license source, prefer a paper-based numerical design document written independently of source details. Do not infer that a clean-room protocol alone defeats a patent or guarantees non-derivation.
7. **Before merge/release:** audit `cargo metadata` and wheel/static binary dependencies; confirm notices in `LICENSE-THIRD-PARTY`, source headers and package manifests; check code provenance; verify no GPLv3/Apache/no-license text entered the GPLv2-compatible delivery path. Obtain legal review if source-derived work has ambiguous provenance or a significant distribution consequence.

### Licensing decision tree

```text
Need an algorithm from another implementation?
  |
  +-- Public paper / mathematical specification available?
  |     +-- Yes: independently design, implement and test in Rust
  |               -> RustSHT-owned GPL-2.0-or-later code (subject to patents/etc.)
  |
  +-- Want to translate/adapt upstream source directly?
        |
        +-- GPL-2.0-or-later / MIT / BSD-3 / confirmed GPL-compatible CeCILL
        |     -> source port permitted with provenance, source licenses/notices
        |
        +-- GPLv3 / Apache-2.0
        |     -> combined distribution requires GPLv3-compatible terms;
        |        preserving a GPLv2 option means NO covered source copying
        |
        +-- license unclear or all rights reserved
              -> no source port; get written permission or use the paper
```

## Recommended RustSHT research sequence

1. **Finish active performance and numerical portability milestones** in [the active roadmap](../plans/05-validation-and-milestones.md): q8 bottleneck, scalar analysis and spin-1. Do not divert the active agent into a second large research backend now.
2. **Independent batched Legendre prototype** with small panel sizes and no fixed machine-specific cutoff; compare actual `B` and realistic atmospheric grid/degree distributions to current RustSHT and DUCC. This can be entirely RustSHT-original, using the fastSHT *paper* and related general matrix batching literature without GPLv3 source adaptation.
3. **Separate IDBF/butterfly experiment**, derived from the Bremer–Chen–Yang paper. Do not port ButterflyLab source. Measure setup, factor storage, apply and transpose errors, plus amortized per-field throughput, then compare with the latest scalar recurrence at modest, irregular and high `L`.
4. **FastTransforms / S2FFT focused source studies** when a specific alternative polynomial/spin algorithm appears valuable. MIT grants source reuse with notices; actual benefit on native Rust CPU remains to be measured. 
5. **ecTrans as an operational performance oracle** and architectural reference. Record relevant CPU and transform workloads, but avoid an Apache-code-to-Rust translation if GPLv2 distribution availability is to be retained.

## Primary references and licensing sources

- [RustSHT package license choice](../../../Cargo.toml) and [README](../../../README.md); [third-party notices](../../../LICENSE-THIRD-PARTY).
- [GNU GPL license compatibility FAQ](https://www.gnu.org/licenses/gpl-faq.en.html#v2v3Compatibility) and [license list](https://www.gnu.org/licenses/license-list.en.html); [GPL version identification](https://www.gnu.org/licenses/identify-licenses-clearly.html).
- [DUCC licensing terms](https://github.com/mreineck/ducc#licensing-terms) and [source](https://github.com/mreineck/ducc).
- [SHTns documentation](https://nschaeff.bitbucket.io/shtns/) and [CeCILL 2.1 text (OSI)](https://opensource.org/license/CECILL-2.1), particularly §5.3.4.
- [FastTransforms MIT license](https://github.com/MikaelSlevinsky/FastTransforms/blob/master/LICENSE) and [source](https://github.com/MikaelSlevinsky/FastTransforms).
- [S2FFT MIT license](https://github.com/astro-informatics/s2fft/blob/main/LICENCE.txt) and [source](https://github.com/astro-informatics/s2fft).
- [fastSHT GPLv3 license](https://github.com/liuhao-cn/fastSHT/blob/main/LICENSE) and [paper](https://arxiv.org/abs/2208.10154).
- [ecTrans Apache-2.0](https://github.com/ecmwf-ifs/ectrans/blob/develop/LICENSE), [repo](https://github.com/ecmwf-ifs/ectrans) and [ECMWF docs](https://sites.ecmwf.int/docs/ectrans/).
- [ButterflyLab README with reserved copyright](https://github.com/ButterflyLab/ButterflyLab#4-copy-right).
- [cunuSHT repository](https://github.com/Sebastian-Belkner/cunuSHT), where no reusable license grant was located in the checked tree.
- [FFTW GPL license](https://ftp.fftw.org/doc/License-and-Copyright.html).

**Operational rule:** direct port where the *actual* source grants compatible rights; otherwise build an independent algorithm from literature. Keep the original author's scientific attribution even when no code is reused.
