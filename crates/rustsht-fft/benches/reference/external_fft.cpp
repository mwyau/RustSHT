// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

#include <algorithm>
#include <chrono>
#include <cmath>
#include <complex>
#include <cstddef>
#include <iomanip>
#include <iostream>
#include <memory>
#include <new>
#include <numeric>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#if defined(BENCH_ADMIRAL)
#include <admiral/admiral.hpp>
#elif defined(BENCH_FFTW)
#include <fftw3.h>
#else
#error "define BENCH_ADMIRAL or BENCH_FFTW"
#endif

namespace {

using Clock = std::chrono::steady_clock;
using Complex = std::complex<double>;

template <class T>
struct AlignedAllocator {
  using value_type = T;
  AlignedAllocator() noexcept = default;
  template <class U> AlignedAllocator(const AlignedAllocator<U>&) noexcept {}
  T* allocate(std::size_t count) {
    return static_cast<T*>(::operator new(count * sizeof(T), std::align_val_t{64}));
  }
  void deallocate(T* ptr, std::size_t) noexcept {
    ::operator delete(ptr, std::align_val_t{64});
  }
  template <class U> bool operator==(const AlignedAllocator<U>&) const noexcept { return true; }
};

template <class T>
using AlignedVector = std::vector<T, AlignedAllocator<T>>;

constexpr std::size_t batch_count = 32;
constexpr std::size_t sample_count = 7;

enum class Mode { c2c_forward, c2c_inverse, r2c, c2r };

const char* mode_name(Mode mode) {
  switch (mode) {
    case Mode::c2c_forward: return "c2c_forward";
    case Mode::c2c_inverse: return "c2c_inverse";
    case Mode::r2c: return "r2c";
    case Mode::c2r: return "c2r";
  }
  throw std::logic_error("invalid transform mode");
}

Mode parse_mode(const std::string& name) {
  if (name == "c2c_forward") return Mode::c2c_forward;
  if (name == "c2c_inverse") return Mode::c2c_inverse;
  if (name == "r2c") return Mode::r2c;
  if (name == "c2r") return Mode::c2r;
  throw std::runtime_error("mode must be c2c_forward, c2c_inverse, r2c, or c2r");
}

std::size_t spectrum_size(std::size_t n) { return n / 2 + 1; }

double real_value(std::size_t i, std::size_t row) {
  const double x = static_cast<double>(i + 1 + 3 * row);
  return std::sin(0.173 * x) + 0.25 * std::cos(0.113 * (x + 1.0));
}

Complex complex_value(std::size_t i, std::size_t row) {
  const double x = static_cast<double>(i + 1 + 3 * row);
  return {std::sin(0.173 * x), std::cos(0.113 * (x + 1.0))};
}

Complex spectrum_value(std::size_t k, std::size_t n, std::size_t row) {
  if (k == 0 || ((n & 1U) == 0 && k == n / 2))
    return {0.4 + 0.01 * static_cast<double>(k + row), 0.0};
  const double x = static_cast<double>(k + 1 + 5 * row);
  return {0.3 * std::sin(0.19 * x), 0.2 * std::cos(0.07 * (x + 1.0))};
}

double median_ns(std::vector<double> samples) {
  std::sort(samples.begin(), samples.end());
  return samples[samples.size() / 2];
}

template <class F>
double measure_ns(F&& f) {
  f();
  std::vector<double> samples;
  samples.reserve(sample_count);
  for (std::size_t i = 0; i < sample_count; ++i) {
    const auto start = Clock::now();
    f();
    const auto end = Clock::now();
    samples.push_back(std::chrono::duration<double, std::nano>(end - start).count());
  }
  return median_ns(std::move(samples));
}

struct Buffers {
  std::size_t n;
  std::size_t rows;
  std::size_t bins;
  std::size_t complex_stride;
  std::size_t real_stride;
  std::size_t bins_stride;
  AlignedVector<Complex> complex_input;
  AlignedVector<Complex> complex_output;
  AlignedVector<double> real_input;
  AlignedVector<double> real_output;

  Buffers(std::size_t length, std::size_t row_count)
      : n(length), rows(row_count), bins(spectrum_size(length)),
        complex_stride((length + 3) & ~std::size_t{3}),
        real_stride((length + 7) & ~std::size_t{7}),
        bins_stride((bins + 3) & ~std::size_t{3}),
        complex_input(row_count * std::max(complex_stride, bins_stride)),
        complex_output(row_count * std::max(complex_stride, bins_stride)),
        real_input(row_count * real_stride), real_output(row_count * real_stride) {
    for (std::size_t row = 0; row < rows; ++row) {
      for (std::size_t i = 0; i < n; ++i) {
        complex_input[row * std::max(complex_stride, bins_stride) + i] = complex_value(i, row);
        real_input[row * real_stride + i] = real_value(i, row);
      }
      for (std::size_t k = 0; k < bins; ++k)
        complex_output[row * std::max(complex_stride, bins_stride) + k] = Complex{};
    }
  }

  void fill_spectrum(Mode mode) {
    if (mode != Mode::c2r) return;
    for (std::size_t row = 0; row < rows; ++row)
      for (std::size_t k = 0; k < bins; ++k)
        complex_input[row * bins_stride + k] = spectrum_value(k, n, row);
  }
};

class Runner {
 public:
  Runner(std::size_t n, Mode mode) : n_(n), mode_(mode) {
#if defined(BENCH_ADMIRAL)
    admiral::options options;
    options.nthreads = 1;
    options.eff = admiral::effort::estimate;
    if (mode == Mode::c2c_forward || mode == Mode::c2c_inverse)
      complex_plan_ = std::make_unique<admiral::plan<double>>(n, options);
    else
      real_plan_ = std::make_unique<admiral::plan_r2c<double>>(n, options);
#else
    const int length = static_cast<int>(n);
    if (mode == Mode::c2c_forward || mode == Mode::c2c_inverse) {
      complex_plan_input_.resize(n_);
      complex_plan_output_.resize(n_);
      auto* input = reinterpret_cast<fftw_complex*>(complex_plan_input_.data());
      auto* output = reinterpret_cast<fftw_complex*>(complex_plan_output_.data());
      const int sign = mode == Mode::c2c_forward ? FFTW_FORWARD : FFTW_BACKWARD;
      complex_plan_ = fftw_plan_dft_1d(length, input, output, sign,
                                       FFTW_ESTIMATE | FFTW_PRESERVE_INPUT);
    } else if (mode == Mode::r2c) {
      real_plan_input_.resize(n_);
      real_plan_output_.resize(spectrum_size(n_));
      real_plan_ = fftw_plan_dft_r2c_1d(
          length, real_plan_input_.data(),
          reinterpret_cast<fftw_complex*>(real_plan_output_.data()),
          FFTW_ESTIMATE | FFTW_PRESERVE_INPUT);
    } else {
      real_plan_output_.resize(n_);
      real_plan_input_.resize(spectrum_size(n_));
      real_plan_ = fftw_plan_dft_c2r_1d(
          length, reinterpret_cast<fftw_complex*>(real_plan_input_.data()),
          real_plan_output_.data(), FFTW_ESTIMATE | FFTW_PRESERVE_INPUT);
    }
    if ((mode == Mode::c2c_forward || mode == Mode::c2c_inverse) && !complex_plan_)
      throw std::runtime_error("FFTW complex plan construction failed");
    if ((mode == Mode::r2c || mode == Mode::c2r) && !real_plan_)
      throw std::runtime_error("FFTW real plan construction failed");
#endif
  }

  ~Runner() {
#if defined(BENCH_FFTW)
    if (complex_plan_) fftw_destroy_plan(complex_plan_);
    if (real_plan_) fftw_destroy_plan(real_plan_);
#endif
  }

  Runner(const Runner&) = delete;
  Runner& operator=(const Runner&) = delete;

  void execute(Buffers& buffers, bool normalize) const {
    const auto bins = buffers.bins;
    const auto complex_stride = std::max(buffers.complex_stride, buffers.bins_stride);
    for (std::size_t row = 0; row < buffers.rows; ++row) {
      if (mode_ == Mode::c2c_forward || mode_ == Mode::c2c_inverse) {
#if defined(BENCH_ADMIRAL)
        if (mode_ == Mode::c2c_forward)
          complex_plan_->forward(buffers.complex_input.data() + row * complex_stride,
                                 buffers.complex_output.data() + row * complex_stride);
        else
          complex_plan_->inverse(buffers.complex_input.data() + row * complex_stride,
                                 buffers.complex_output.data() + row * complex_stride);
#else
        fftw_execute_dft(complex_plan_,
                         reinterpret_cast<fftw_complex*>(buffers.complex_input.data() + row * complex_stride),
                         reinterpret_cast<fftw_complex*>(buffers.complex_output.data() + row * complex_stride));
#endif
#if defined(BENCH_FFTW)
        if (normalize && mode_ == Mode::c2c_inverse) {
          const double scale = 1.0 / static_cast<double>(n_);
          for (std::size_t i = 0; i < n_; ++i)
            buffers.complex_output[row * complex_stride + i] *= scale;
        }
#endif
      } else if (mode_ == Mode::r2c) {
#if defined(BENCH_ADMIRAL)
        real_plan_->forward(buffers.real_input.data() + row * buffers.real_stride,
                            buffers.complex_output.data() + row * buffers.bins_stride);
#else
        fftw_execute_dft_r2c(
            real_plan_, buffers.real_input.data() + row * buffers.real_stride,
            reinterpret_cast<fftw_complex*>(buffers.complex_output.data() + row * buffers.bins_stride));
#endif
      } else {
#if defined(BENCH_ADMIRAL)
        real_plan_->inverse(buffers.complex_input.data() + row * buffers.bins_stride,
                            buffers.real_output.data() + row * buffers.real_stride);
#else
        fftw_execute_dft_c2r(
            real_plan_, reinterpret_cast<fftw_complex*>(buffers.complex_input.data() + row * buffers.bins_stride),
            buffers.real_output.data() + row * buffers.real_stride);
        if (normalize) {
          const double scale = 1.0 / static_cast<double>(n_);
          for (std::size_t i = 0; i < n_; ++i)
            buffers.real_output[row * buffers.real_stride + i] *= scale;
        }
#endif
      }
    }
  }

  std::size_t plan_bytes() const { return 0; }

 private:
  std::size_t n_;
  Mode mode_;
#if defined(BENCH_ADMIRAL)
  std::unique_ptr<admiral::plan<double>> complex_plan_;
  std::unique_ptr<admiral::plan_r2c<double>> real_plan_;
#else
  fftw_plan complex_plan_ = nullptr;
  fftw_plan real_plan_ = nullptr;
  AlignedVector<Complex> complex_plan_input_;
  AlignedVector<Complex> complex_plan_output_;
  AlignedVector<double> real_plan_input_;
  AlignedVector<double> real_plan_output_;
#endif
};

double checksum(const Buffers& buffers, Mode mode) {
  if (mode == Mode::c2c_forward || mode == Mode::c2c_inverse)
    return buffers.complex_output[0].real() + buffers.complex_output[0].imag();
  if (mode == Mode::r2c)
    return buffers.complex_output[0].real() + buffers.complex_output[0].imag();
  return buffers.real_output[0];
}

void dump(Mode mode, std::size_t n) {
  Runner plan(n, mode);
  Buffers buffers(n, 1);
  buffers.fill_spectrum(mode);
  plan.execute(buffers, true);
  std::cout << std::setprecision(17);
  if (mode == Mode::c2c_forward || mode == Mode::c2c_inverse || mode == Mode::r2c) {
    const std::size_t count = mode == Mode::r2c ? buffers.bins : n;
    for (std::size_t i = 0; i < count; ++i) {
      const auto value = buffers.complex_output[i];
      std::cout << mode_name(mode) << ',' << n << ',' << i << ','
                << value.real() << ',' << value.imag() << '\n';
    }
  } else {
    for (std::size_t i = 0; i < n; ++i)
      std::cout << mode_name(mode) << ',' << n << ',' << i << ','
                << buffers.real_output[i] << ",0\n";
  }
}

void benchmark(Mode mode, std::size_t n, std::size_t rows) {
  const auto plan_start = Clock::now();
  for (int repeat = 0; repeat < 20; ++repeat) {
    Runner plan(n, mode);
    (void)plan.plan_bytes();
  }
  const auto plan_end = Clock::now();
  const double plan_ns = std::chrono::duration<double, std::nano>(plan_end - plan_start).count() / 20.0;

  Runner plan(n, mode);
  Buffers buffers(n, rows);
  buffers.fill_spectrum(mode);
  const double fft_ns = measure_ns([&] { plan.execute(buffers, false); }) / rows;
  const double call_ns = measure_ns([&] { plan.execute(buffers, true); }) / rows;
  const double value = checksum(buffers, mode);
  std::cout << mode_name(mode) << ',' << n << ',' << std::fixed << std::setprecision(9)
            << plan_ns << ",0.000000000,0," << fft_ns << ",0.000000000," << call_ns
            << ',' << call_ns * static_cast<double>(rows) << ',' << rows << ','
            << std::setprecision(17) << value << '\n';
}

const std::vector<std::size_t> default_lengths{
    19, 41, 73, 103, 109, 113, 131, 137, 146, 150, 240, 241, 256, 257, 300, 484,
    512, 513, 525, 683, 722, 723, 726, 1023, 1024, 1025, 1372, 1441, 1442, 1443,
    1452, 2047, 2048, 2049, 2904, 4097};
const std::vector<std::pair<std::size_t, std::size_t>> real_sht_rows{
    {257, 128}, {723, 361}, {1025, 512}, {1443, 721}, {2049, 1024}, {4097, 2048}};
const std::vector<std::pair<std::size_t, std::size_t>> complex_sht_rows{
    {146, 73}, {150, 73}, {300, 73}, {256, 128}, {512, 256}, {722, 361},
    {726, 361}, {1024, 512}, {1442, 721}, {1452, 721}, {2904, 721}};

void run_lengths(const std::vector<std::size_t>& lengths) {
  for (const auto n : lengths) {
    const auto batch_rows = n > 10000 ? std::size_t{4} : batch_count;
    for (const auto mode : {Mode::c2c_forward, Mode::c2c_inverse, Mode::r2c, Mode::c2r}) {
      benchmark(mode, n, 1);
      benchmark(mode, n, batch_rows);
    }
  }
}

void run_sht_workloads() {
  for (const auto [n, rows] : complex_sht_rows) {
    benchmark(Mode::c2c_forward, n, rows);
    benchmark(Mode::c2c_inverse, n, rows);
  }
  for (const auto [n, rows] : real_sht_rows) {
    benchmark(Mode::r2c, n, rows);
    benchmark(Mode::c2r, n, rows);
  }
}

}  // namespace

int main(int argc, char** argv) {
  try {
    if (argc == 4 && std::string(argv[1]) == "dump") {
      dump(parse_mode(argv[2]), std::stoull(argv[3]));
      return 0;
    }
    std::cout << "mode,n,plan_ns,workspace_ns,workspace_bytes,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum\n";
    if (argc == 2 && std::string(argv[1]) == "--sht") {
      run_sht_workloads();
      return 0;
    }
    std::vector<std::size_t> lengths;
    for (int i = 1; i < argc; ++i) lengths.push_back(std::stoull(argv[i]));
    if (lengths.empty()) lengths = default_lengths;
    run_lengths(lengths);
    return 0;
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
