// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

#include <algorithm>
#include <cstddef>
#include <memory>

#include "ducc0/fft/fft1d_impl.h"
#include "ducc0/infra/threading.h"

namespace {

struct Plan {
  ducc0::pocketfft_r<double> fft;
  std::size_t length;
  std::size_t rows;
  std::size_t bins;
  std::size_t workers;
  std::size_t scratch_len;

  Plan(std::size_t n, std::size_t nrings, std::size_t nworkers)
      : fft(n), length(n), rows(nrings), bins(n / 2 + 1), workers(nworkers),
        scratch_len(fft.bufsize()) {}
};

}  // namespace

extern "C" void* plan14_ducc_create_plan(std::size_t n, std::size_t rows,
                                          std::size_t workers) {
  return new Plan(n, rows, workers);
}

extern "C" std::size_t plan14_ducc_scratch_len(void* opaque) {
  return static_cast<Plan*>(opaque)->scratch_len;
}

extern "C" void plan14_ducc_inverse_rows(void* opaque, const double* spectra,
                                         double* output, double* scratch,
                                         double* packed) {
  auto* plan = static_cast<Plan*>(opaque);
  ducc0::execDynamic(plan->rows, plan->workers, 8, [&](ducc0::Scheduler& sched) {
    const auto worker = sched.thread_num();
    auto* worker_scratch = scratch + worker * plan->scratch_len;
    auto* row_buffer = packed + worker * plan->length;
    while (auto range = sched.getNext()) {
      for (std::size_t row = range.lo; row < range.hi; ++row) {
        const auto* modes = spectra + row * plan->bins * 2;
        row_buffer[0] = modes[0];
        const auto packed_modes = plan->length / 2;
        for (std::size_t m = 1; m < packed_modes; ++m) {
          row_buffer[2 * m - 1] = modes[2 * m];
          row_buffer[2 * m] = modes[2 * m + 1];
        }
        if (plan->length % 2 == 0) {
          row_buffer[plan->length - 1] = modes[2 * packed_modes];
        } else {
          const auto m = packed_modes;
          row_buffer[2 * m - 1] = modes[2 * m];
          row_buffer[2 * m] = modes[2 * m + 1];
        }
        plan->fft.exec_copyback(row_buffer, worker_scratch, 1.0, false, 1);
        std::copy_n(row_buffer, plan->length, output + row * plan->length);
      }
    }
  });
}

extern "C" void plan14_ducc_destroy_plan(void* opaque) {
  delete static_cast<Plan*>(opaque);
}
