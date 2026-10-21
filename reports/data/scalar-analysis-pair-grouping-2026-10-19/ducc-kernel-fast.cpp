// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
// Calls DUCC's ordinary scalar analysis kernel from the pinned 0.42.0 source.
// The caller supplies the same preinitialized recurrence state as RustSHT's
// fused pair block, so this measures map2alm_kernel without init_lambda or
// transform-level coefficient correction.

#include <complex>
#include <cstddef>

#include "ducc0/infra/simd.h"
#include "ducc0/math/constants.h"
#include "ducc0/sht/sht.h"
#include "ducc0/sht/sht_inner_loop.h"

namespace loop = ducc0::detail_sht::detail_sht_inner_loop;

extern "C" void rustsht_ducc_map2alm_kernel(
    loop::s0data_v *data, const loop::Ylmgen *generator,
    std::complex<double> *alm, size_t nth, size_t lstart, size_t lstop) {
  const size_t nv2 = (nth + loop::VLEN - 1) / loop::VLEN;
  const size_t il = (lstart - generator->m) / 2;
  loop::map2alm_kernel(*data, generator->coef, alm, lstart, il,
                       lstop - 1, nv2);
}
