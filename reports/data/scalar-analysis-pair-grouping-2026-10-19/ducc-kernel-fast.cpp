// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
// Calls DUCC's ordinary scalar analysis kernel from the pinned 0.42.0 source.

#include <complex>
#include <cstddef>

#include "ducc0/infra/simd.h"
#include "ducc0/math/constants.h"
#include "ducc0/sht/sht.h"
#include "ducc0/sht/sht_inner_loop.h"

namespace loop = ducc0::detail_sht::detail_sht_inner_loop;

extern "C" void rustsht_ducc_map2alm_kernel(
    loop::s0data_v *data, const loop::Ylmgen *generator,
    std::complex<double> *alm, size_t l, size_t il, size_t lmax,
    size_t nv2) {
  loop::map2alm_kernel(*data, generator->coef, alm, l, il, lmax, nv2);
}
