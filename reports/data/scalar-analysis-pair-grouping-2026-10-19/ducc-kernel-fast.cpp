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

extern "C" void rustsht_ducc_calc_map2alm(
    loop::s0data_v *data, const loop::Ylmgen *generator,
    std::complex<double> *alm, size_t nth, size_t lstart, size_t lstop) {
  const size_t nv2 = (nth + loop::VLEN - 1) / loop::VLEN;
  loop::init_lambda(*generator, *data, nv2);
  loop::calc_map2alm(alm, *generator, *data, nth, lstart, lstop);
  // Preserve DUCC inner_loop_m2a's scalar coefficient adjustment in the
  // timed path; RustSHT performs the corresponding paired finalization.
  std::complex<double> previous_even(0.0, 0.0);
  double previous_alpha = 0.0;
  for (size_t pair = 0, l = generator->m; l <= generator->lmax;
       ++pair, l += 2) {
    const std::complex<double> even = alm[l];
    const std::complex<double> odd =
        (l < generator->lmax) ? alm[l + 1] : std::complex<double>(0.0, 0.0);
    const std::complex<double> previous =
        previous_even * (previous_alpha * generator->eps[l]);
    const double alpha = generator->alpha[pair];
    alm[l] = even * (alpha * generator->eps[l + 1]) + previous;
    if (l < generator->lmax)
      alm[l + 1] = odd * alpha;
    previous_even = even;
    previous_alpha = alpha;
  }
}
