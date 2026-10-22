// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

#include <algorithm>
#include <cstddef>
#include <stdexcept>

#include "ducc0/fft/fft1d_impl.h"
#include "ducc0/infra/threading.h"

namespace {

struct Plan {
  ducc0::pocketfft_r<double> fft;
  std::size_t length;
  std::size_t rows;
  std::size_t bins;
  std::size_t modes;
  std::size_t workers;
  std::size_t scratch_len;

  Plan(std::size_t n, std::size_t nrows, std::size_t nmodes,
       std::size_t nworkers)
      : fft(n),
        length(n),
        rows(nrows),
        bins(n / 2 + 1),
        modes(nmodes),
        workers(nworkers),
        scratch_len(fft.bufsize()) {}
};

}  // namespace

extern "C" void* rustsht_paired_rows_ducc_create(std::size_t n,
                                                  std::size_t rows,
                                                  std::size_t modes,
                                                  std::size_t workers) {
  return new Plan(n, rows, modes, workers);
}

extern "C" std::size_t rustsht_paired_rows_ducc_scratch_len(void* opaque) {
  return static_cast<Plan*>(opaque)->scratch_len;
}

extern "C" void rustsht_paired_rows_ducc_execute(
    void* opaque, bool inverse, const double* input, double* output,
    double* scratch, double* row_buffers) {
  auto* plan = static_cast<Plan*>(opaque);
  ducc0::execDynamic(plan->rows, plan->workers, 8, [&](ducc0::Scheduler& sched) {
    const auto worker = sched.thread_num();
    auto* worker_scratch = scratch + worker * plan->scratch_len;
    auto* row_buffer = row_buffers + worker * plan->length;
    while (auto range = sched.getNext()) {
      for (std::size_t row = range.lo; row < range.hi; ++row) {
        if (inverse) {
          const auto* spectrum = input + row * plan->bins * 2;
          row_buffer[0] = spectrum[0];
          const auto highest = plan->length / 2;
          for (std::size_t k = 1; k <= highest; ++k) {
            if (plan->length % 2 == 0 && k == highest) {
              row_buffer[plan->length - 1] = spectrum[2 * k];
            } else {
              row_buffer[2 * k - 1] = spectrum[2 * k];
              row_buffer[2 * k] = spectrum[2 * k + 1];
            }
          }
          plan->fft.exec_copyback(row_buffer, worker_scratch, 1.0, false, 1);
          std::copy_n(row_buffer, plan->length, output + row * plan->length);
        } else {
          std::copy_n(input + row * plan->length, plan->length, row_buffer);
          plan->fft.exec_copyback(row_buffer, worker_scratch, 1.0, true, 1);
          auto* spectrum = output + row * plan->modes * 2;
          spectrum[0] = row_buffer[0];
          spectrum[1] = 0.0;
          for (std::size_t k = 1; k < plan->modes; ++k) {
            if (plan->length % 2 == 0 && k == plan->length / 2) {
              spectrum[2 * k] = row_buffer[plan->length - 1];
              spectrum[2 * k + 1] = 0.0;
            } else {
              spectrum[2 * k] = row_buffer[2 * k - 1];
              spectrum[2 * k + 1] = row_buffer[2 * k];
            }
          }
        }
      }
    }
  });
}

extern "C" void rustsht_paired_rows_ducc_destroy(void* opaque) {
  delete static_cast<Plan*>(opaque);
}
