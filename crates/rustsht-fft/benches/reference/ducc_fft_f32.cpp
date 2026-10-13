// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

#include "ducc0/fft/fft1d_impl.h"

namespace {

using Clock = std::chrono::steady_clock;
using Complex = ducc0::Cmplx<float>;
constexpr std::size_t batch_rows = 32;
constexpr std::size_t sample_count = 7;

float real_value(std::size_t i) {
  const double x = static_cast<double>(i + 1);
  volatile float sine = static_cast<float>(std::sin(0.173 * x));
  volatile float cosine = static_cast<float>(std::cos(0.113 * (x + 1.0)));
  volatile float scaled_cosine = 0.25f * cosine;
  return sine + scaled_cosine;
}

Complex spectrum_value(std::size_t k, std::size_t n, std::size_t row) {
  if (k == 0 || ((n & 1U) == 0 && k == n / 2)) {
    volatile float k_term = 0.01f * static_cast<float>(k);
    volatile float row_term = 0.003f * static_cast<float>(row);
    volatile float base_and_k = 0.4f + k_term;
    return {base_and_k + row_term, 0.0f};
  }
  volatile double row_offset = static_cast<double>(row) * 0.37;
  const double x = static_cast<double>(k + 1) + row_offset;
  return {static_cast<float>(0.3 * std::sin(0.19 * x)),
          static_cast<float>(0.2 * std::cos(0.07 * (x + 1.0)))};
}

std::size_t bins(std::size_t n) { return n / 2 + 1; }

bool vectorize() {
  const char* value = std::getenv("RUSTSHT_FFT_DUCC_VECTORIZE");
  return value != nullptr && std::string(value) == "1";
}

void unpack(const float* packed, std::size_t n, Complex* output) {
  output[0] = {packed[0], 0.0f};
  const std::size_t last = n / 2;
  for (std::size_t k = 1; k <= last; ++k) {
    if ((n & 1U) == 0 && k == last)
      output[k] = {packed[n - 1], 0.0f};
    else
      output[k] = {packed[2 * k - 1], packed[2 * k]};
  }
}

void pack(const Complex* input, std::size_t n, float* packed) {
  packed[0] = input[0].r;
  const std::size_t last = n / 2;
  for (std::size_t k = 1; k <= last; ++k) {
    if ((n & 1U) == 0 && k == last)
      packed[n - 1] = input[k].r;
    else {
      packed[2 * k - 1] = input[k].r;
      packed[2 * k] = input[k].i;
    }
  }
}

template <typename Plan> double plan_creation_ns(std::size_t n, bool use_vectorize) {
  volatile std::size_t observed = 0;
  constexpr std::size_t repetitions = 20;
  std::vector<double> samples;
  samples.reserve(repetitions);
  for (std::size_t i = 0; i < repetitions; ++i) {
    const auto begin = Clock::now();
    Plan plan(n, use_vectorize);
    const auto end = Clock::now();
    samples.push_back(
        std::chrono::duration<double, std::nano>(end - begin).count());
    observed = observed + plan.bufsize();
  }
  std::sort(samples.begin(), samples.end());
  (void)observed;
  return samples[samples.size() / 2];
}

double scratch_allocation_ns(std::size_t count) {
  volatile std::size_t observed = 0;
  constexpr std::size_t repetitions = 20;
  std::vector<double> samples;
  samples.reserve(repetitions);
  for (std::size_t i = 0; i < repetitions; ++i) {
    const auto begin = Clock::now();
    std::vector<float> scratch(count);
    const auto end = Clock::now();
    samples.push_back(
        std::chrono::duration<double, std::nano>(end - begin).count());
    observed = observed + scratch.size();
  }
  std::sort(samples.begin(), samples.end());
  (void)observed;
  return samples[samples.size() / 2];
}

double work_allocation_ns(std::size_t count) {
  volatile std::size_t observed = 0;
  constexpr std::size_t repetitions = 20;
  std::vector<double> samples;
  samples.reserve(repetitions);
  for (std::size_t i = 0; i < repetitions; ++i) {
    const auto begin = Clock::now();
    std::vector<float> work(count);
    const auto end = Clock::now();
    samples.push_back(
        std::chrono::duration<double, std::nano>(end - begin).count());
    observed = observed + work.size();
  }
  std::sort(samples.begin(), samples.end());
  (void)observed;
  return samples[samples.size() / 2];
}

struct Runner {
  Runner(std::size_t length, std::size_t row_count, bool use_vectorize)
      : n(length), rows(row_count), plan(n, use_vectorize), scratch(plan.bufsize()),
        data(n), real_input(rows * n), spectra(rows * bins(n)),
        complex_output(rows * bins(n)), real_output(rows * n) {
    for (std::size_t row = 0; row < rows; ++row) {
      for (std::size_t i = 0; i < n; ++i)
        real_input[row * n + i] = real_value(row * n + i);
      for (std::size_t k = 0; k < bins(n); ++k)
        spectra[row * bins(n) + k] = spectrum_value(k, n, row);
    }
  }

  void forward() {
    for (std::size_t row = 0; row < rows; ++row) {
      float* packed = data.data();
      std::copy(real_input.data() + row * n,
                real_input.data() + (row + 1) * n, packed);
      plan.exec_copyback(packed, scratch.data(), 1.0f, true, 1);
      unpack(packed, n, complex_output.data() + row * bins(n));
    }
  }

  void inverse() {
    for (std::size_t row = 0; row < rows; ++row) {
      float* packed = data.data();
      pack(spectra.data() + row * bins(n), n, packed);
      plan.exec_copyback(packed, scratch.data(), 1.0f / static_cast<float>(n),
                         false, 1);
      std::copy(packed, packed + n, real_output.data() + row * n);
    }
  }

  double checksum(bool is_forward) const {
    double result = 0.0;
    if (is_forward) {
      for (const auto value : complex_output)
        result += static_cast<double>(value.r) * 0.73 +
                  static_cast<double>(value.i) * 0.41;
    } else {
      for (float value : real_output) result += static_cast<double>(value);
    }
    return result;
  }

  std::size_t n;
  std::size_t rows;
  ducc0::pocketfft_r<float> plan;
  std::vector<float> scratch;
  std::vector<float> data;
  std::vector<float> real_input;
  std::vector<Complex> spectra;
  std::vector<Complex> complex_output;
  std::vector<float> real_output;
};

template <typename F>
double time_per_transform(F&& operation, std::size_t repeats,
                          std::size_t rows) {
  const auto begin = Clock::now();
  for (std::size_t i = 0; i < repeats; ++i) operation();
  const auto end = Clock::now();
  return std::chrono::duration<double, std::nano>(end - begin).count() /
         static_cast<double>(repeats * rows);
}

void benchmark(std::size_t n, std::size_t rows, const std::string& process) {
  const bool use_vectorize = vectorize();
  const double plan_ns =
      plan_creation_ns<ducc0::pocketfft_r<float>>(n, use_vectorize);
  Runner runner(n, rows, use_vectorize);
  const double scratch_ns = scratch_allocation_ns(runner.plan.bufsize());
  const double work_ns = work_allocation_ns(n);
  const std::size_t repeats = rows == 1 ? std::max<std::size_t>(262144 / n, 16)
                                        : std::max<std::size_t>((262144 / n) / rows, 1);

  runner.forward();
  runner.inverse();
  std::cout << std::setprecision(12);
  for (std::size_t sample = 0; sample < sample_count; ++sample) {
    const bool forward_first = (sample & 1U) == 0;
    for (const bool is_forward : {forward_first, !forward_first}) {
      const double ns = is_forward
                            ? time_per_transform([&] { runner.forward(); }, repeats, rows)
                            : time_per_transform([&] { runner.inverse(); }, repeats, rows);
      const double check = runner.checksum(is_forward);
      const char* build = std::getenv("RUSTSHT_FFT_BUILD_LABEL");
      std::cout << process << ',' << (build ? build : "unspecified") << ','
                << (is_forward ? "r2c" : "c2r") << ',' << n
                << ',' << rows << ',' << repeats << ',' << sample << ','
                << (use_vectorize ? 1 : 0) << ','
                << plan_ns << ',' << scratch_ns << ','
                << runner.scratch.size() * sizeof(float) << ',' << work_ns
                << ',' << runner.data.size() * sizeof(float) << ',' << ns << ','
                << check << '\n';
    }
  }
}

}  // namespace

int main(int argc, char** argv) {
  if (argc != 2) {
    std::cerr << "Usage: " << argv[0] << " <length>\n";
    return 2;
  }
  try {
    const std::size_t n = std::stoull(argv[1]);
    const std::string process = std::getenv("RUSTSHT_FFT_PROCESS_SAMPLE")
                                    ? std::getenv("RUSTSHT_FFT_PROCESS_SAMPLE")
                                    : "0";
    std::cout << "process_sample,build,mode,n,rows,repeats,sample,vectorize,"
                 "plan_ns,scratch_alloc_ns,plan_scratch_bytes,work_alloc_ns,"
                 "work_bytes,call_ns,checksum\n";
    benchmark(n, 1, process);
    benchmark(n, batch_rows, process);
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
  return 0;
}
