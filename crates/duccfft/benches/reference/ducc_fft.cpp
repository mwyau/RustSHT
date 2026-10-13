// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: BSD-3-Clause

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <numeric>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "ducc0/fft/fft1d_impl.h"

namespace {

using Clock = std::chrono::steady_clock;
using Complex = ducc0::Cmplx<double>;

constexpr double tau = 6.283185307179586476925286766559;
constexpr std::size_t batch_count = 32;
constexpr std::size_t sample_count = 7;

const std::vector<std::size_t> benchmark_lengths{
    13, 17, 31, 32, 33, 41, 53, 64, 107, 109, 113, 127, 128, 131, 137, 241,
    257, 511, 512, 683, 998, 1000, 1001, 1002, 1008, 1013, 1023, 1024, 1025,
    1026, 2047, 2048, 2049, 4096, 4097, 10000, 10001, 10008,
};
const std::vector<std::pair<std::size_t, std::size_t>> sht_workloads{
    {257, 128}, {1025, 512}, {2049, 1024}, {4097, 2048}};

double real_value(std::size_t i, std::size_t row = 0) {
  const double x = static_cast<double>(i + 1 + 3 * row);
  return std::sin(0.173 * x) + 0.25 * std::cos(0.113 * (x + 1.0));
}

Complex complex_value(std::size_t i, std::size_t row = 0) {
  const double x = static_cast<double>(i + 1 + 3 * row);
  return {std::sin(0.173 * x), std::cos(0.113 * (x + 1.0))};
}

Complex spectrum_value(std::size_t k, std::size_t n, std::size_t row = 0) {
  if (k == 0 || ((n & 1U) == 0 && k == n / 2)) {
    return {0.4 + 0.01 * static_cast<double>(k + row), 0.0};
  }
  const double x = static_cast<double>(k + 1 + 5 * row);
  return {0.3 * std::sin(0.19 * x), 0.2 * std::cos(0.07 * (x + 1.0))};
}

std::size_t spectrum_size(std::size_t n) { return n / 2 + 1; }

bool ducc_vectorize() {
  const char* value = std::getenv("DUCCFFT_DUCC_VECTORIZE");
  return value != nullptr && std::string(value) == "1";
}

void unpack_real_spectrum(const double* packed, std::size_t n,
                          Complex* spectrum) {
  spectrum[0] = {packed[0], 0.0};
  const std::size_t last = n / 2;
  for (std::size_t k = 1; k <= last; ++k) {
    if ((n & 1U) == 0 && k == last) {
      spectrum[k] = {packed[n - 1], 0.0};
    } else {
      spectrum[k] = {packed[2 * k - 1], packed[2 * k]};
    }
  }
}

void pack_real_spectrum(const Complex* spectrum, std::size_t n,
                        double* packed) {
  std::fill(packed, packed + n, 0.0);
  packed[0] = spectrum[0].r;
  const std::size_t last = n / 2;
  for (std::size_t k = 1; k <= last; ++k) {
    if ((n & 1U) == 0 && k == last) {
      packed[n - 1] = spectrum[k].r;
    } else {
      packed[2 * k - 1] = spectrum[k].r;
      packed[2 * k] = spectrum[k].i;
    }
  }
}

std::vector<Complex> complex_input(std::size_t n, std::size_t row = 0) {
  std::vector<Complex> values(n);
  for (std::size_t i = 0; i < n; ++i) values[i] = complex_value(i, row);
  return values;
}

std::vector<Complex> spectrum_input(std::size_t n, std::size_t row = 0) {
  std::vector<Complex> values(spectrum_size(n));
  for (std::size_t k = 0; k < values.size(); ++k)
    values[k] = spectrum_value(k, n, row);
  return values;
}

std::vector<double> real_input(std::size_t n, std::size_t row = 0) {
  std::vector<double> values(n);
  for (std::size_t i = 0; i < n; ++i) values[i] = real_value(i, row);
  return values;
}

template <typename Prepare, typename F>
double median_prepared_ns(Prepare&& prepare, F&& operation) {
  std::vector<double> samples;
  samples.reserve(sample_count);
  prepare();
  operation();
  for (std::size_t sample = 0; sample < sample_count; ++sample) {
    prepare();
    const auto begin = Clock::now();
    operation();
    const auto end = Clock::now();
    samples.push_back(
        std::chrono::duration<double, std::nano>(end - begin).count());
  }
  std::sort(samples.begin(), samples.end());
  return samples[samples.size() / 2];
}

template <typename F> double median_ns(F&& operation) {
  return median_prepared_ns([] {}, std::forward<F>(operation));
}

template <typename Plan> double plan_construction_ns(std::size_t n,
                                                     bool vectorize) {
  volatile std::size_t observed = 0;
  constexpr std::size_t repeats = 20;
  const auto begin = Clock::now();
  for (std::size_t i = 0; i < repeats; ++i) {
    Plan plan(n, vectorize);
    observed = observed + plan.bufsize();
  }
  const auto end = Clock::now();
  (void)observed;
  return std::chrono::duration<double, std::nano>(end - begin).count() /
         static_cast<double>(repeats);
}

template <typename T> double allocation_ns(std::size_t n) {
  volatile std::size_t observed = 0;
  constexpr std::size_t repeats = 20;
  const auto begin = Clock::now();
  for (std::size_t i = 0; i < repeats; ++i) {
    std::vector<T> buffer(n);
    observed = observed + buffer.size();
  }
  const auto end = Clock::now();
  (void)observed;
  return std::chrono::duration<double, std::nano>(end - begin).count() /
         static_cast<double>(repeats);
}

enum class Mode { c2c_forward, c2c_inverse, r2c, c2r };

const char* mode_name(Mode mode) {
  switch (mode) {
    case Mode::c2c_forward: return "c2c_forward";
    case Mode::c2c_inverse: return "c2c_inverse";
    case Mode::r2c: return "r2c";
    case Mode::c2r: return "c2r";
  }
  return "unknown";
}

Mode parse_mode(const std::string& name) {
  if (name == "c2c_forward") return Mode::c2c_forward;
  if (name == "c2c_inverse") return Mode::c2c_inverse;
  if (name == "r2c") return Mode::r2c;
  if (name == "c2r") return Mode::c2r;
  throw std::runtime_error("mode must be c2c_forward, c2c_inverse, r2c, or c2r");
}

std::vector<Complex> dump(Mode mode, std::size_t n) {
  if (mode == Mode::c2c_forward || mode == Mode::c2c_inverse) {
    ducc0::pocketfft_c<double> plan(n, ducc_vectorize());
    auto values = complex_input(n);
    std::vector<Complex> scratch(plan.bufsize());
    const bool forward = mode == Mode::c2c_forward;
    plan.exec_copyback(values.data(), scratch.data(), forward ? 1.0 : 1.0 / n,
                       forward, 1);
    return values;
  }

  ducc0::pocketfft_r<double> plan(n, ducc_vectorize());
  std::vector<double> values(n);
  std::vector<double> scratch(plan.bufsize());
  if (mode == Mode::r2c) {
    for (std::size_t i = 0; i < n; ++i) values[i] = real_value(i);
    plan.exec_copyback(values.data(), scratch.data(), 1.0, true, 1);
    std::vector<Complex> output(spectrum_size(n));
    unpack_real_spectrum(values.data(), n, output.data());
    return output;
  }

  const auto spectrum = spectrum_input(n);
  pack_real_spectrum(spectrum.data(), n, values.data());
  plan.exec_copyback(values.data(), scratch.data(), 1.0 / n, false, 1);
  std::vector<Complex> output(n);
  for (std::size_t i = 0; i < n; ++i) output[i] = {values[i], 0.0};
  return output;
}

void print_dump(Mode mode, std::size_t n) {
  const auto output = dump(mode, n);
  for (std::size_t k = 0; k < output.size(); ++k)
    std::cout << mode_name(mode) << ',' << n << ',' << k << ','
              << std::setprecision(17) << output[k].r << ',' << output[k].i
              << '\n';
}

void benchmark(Mode mode, std::size_t n, std::size_t rows) {
  if (n > 10000) rows = std::min<std::size_t>(rows, 4);
  double plan_ns = 0.0;
  double scratch_ns = 0.0;
  std::size_t scratch_bytes = 0;
  double fft_ns = 0.0;
  double pack_ns = 0.0;
  double call_ns = 0.0;
  double checksum = 0.0;

  if (mode == Mode::c2c_forward || mode == Mode::c2c_inverse) {
    const bool vectorize = ducc_vectorize();
    ducc0::pocketfft_c<double> plan(n, vectorize);
    std::vector<Complex> data(rows * n);
    std::vector<Complex> scratch(plan.bufsize());
    scratch_bytes = scratch.size() * sizeof(Complex);
    const bool forward = mode == Mode::c2c_forward;
    const double scale = forward ? 1.0 : 1.0 / static_cast<double>(n);
    auto fill = [&] {
      for (std::size_t row = 0; row < rows; ++row)
        for (std::size_t i = 0; i < n; ++i)
          data[row * n + i] = complex_value(i, row);
    };
    auto run_fft = [&](double factor) {
      for (std::size_t row = 0; row < rows; ++row)
        plan.exec_copyback(data.data() + row * n, scratch.data(), factor,
                           forward, 1);
    };
    fft_ns = median_prepared_ns(fill, [&] { run_fft(1.0); }) /
             static_cast<double>(rows);
    call_ns = median_prepared_ns(fill, [&] { run_fft(scale); }) /
              static_cast<double>(rows);
    checksum = data[0].r + data[0].i;
    plan_ns = plan_construction_ns<ducc0::pocketfft_c<double>>(n, vectorize);
    scratch_ns = allocation_ns<Complex>(plan.bufsize());
  } else {
    const bool vectorize = ducc_vectorize();
    ducc0::pocketfft_r<double> plan(n, vectorize);
    const std::size_t bins = spectrum_size(n);
    std::vector<double> data(rows * n);
    std::vector<double> scratch(plan.bufsize());
    scratch_bytes = scratch.size() * sizeof(double);
    std::vector<Complex> spectra(rows * bins);
    std::vector<double> output(rows * n);
    const bool forward = mode == Mode::r2c;
    const double scale = forward ? 1.0 : 1.0 / static_cast<double>(n);

    std::vector<double> source(rows * n);
    for (std::size_t row = 0; row < rows; ++row)
      for (std::size_t i = 0; i < n; ++i)
        source[row * n + i] = real_value(i, row);
    auto fill_real = [&] {
      for (std::size_t row = 0; row < rows; ++row)
        std::copy(source.data() + row * n, source.data() + (row + 1) * n,
                  data.data() + row * n);
    };
    auto fill_spectra = [&] {
      for (std::size_t row = 0; row < rows; ++row)
        for (std::size_t k = 0; k < bins; ++k)
          spectra[row * bins + k] = spectrum_value(k, n, row);
    };
    auto fill_packed_spectrum = [&] {
      fill_spectra();
      for (std::size_t row = 0; row < rows; ++row)
        pack_real_spectrum(spectra.data() + row * bins, n,
                           data.data() + row * n);
    };

    if (forward) {
      fft_ns = median_prepared_ns(fill_real, [&] {
        for (std::size_t row = 0; row < rows; ++row)
          plan.exec_copyback(data.data() + row * n, scratch.data(), 1.0, true,
                             1);
      }) / static_cast<double>(rows);
      fill_real();
      for (std::size_t row = 0; row < rows; ++row)
        plan.exec_copyback(data.data() + row * n, scratch.data(), 1.0, true, 1);
      const double input_copy_ns = median_ns([&] {
        for (std::size_t row = 0; row < rows; ++row)
          std::copy(source.data() + row * n, source.data() + (row + 1) * n,
                    output.data() + row * n);
      });
      const double output_unpack_ns = median_ns([&] {
        for (std::size_t row = 0; row < rows; ++row)
          unpack_real_spectrum(data.data() + row * n, n,
                               spectra.data() + row * bins);
      });
      pack_ns = (input_copy_ns + output_unpack_ns) / static_cast<double>(rows);
      call_ns = median_ns([&] {
        fill_real();
        for (std::size_t row = 0; row < rows; ++row) {
          plan.exec_copyback(data.data() + row * n, scratch.data(), scale, true,
                             1);
          unpack_real_spectrum(data.data() + row * n, n,
                               spectra.data() + row * bins);
        }
      }) / static_cast<double>(rows);
      checksum = spectra[0].r + spectra[0].i;
    } else {
      fft_ns = median_prepared_ns(fill_packed_spectrum, [&] {
        for (std::size_t row = 0; row < rows; ++row)
          plan.exec_copyback(data.data() + row * n, scratch.data(), 1.0, false,
                             1);
      }) / static_cast<double>(rows);
      const double input_pack_ns = median_ns([&] {
        for (std::size_t row = 0; row < rows; ++row)
          pack_real_spectrum(spectra.data() + row * bins, n,
                             output.data() + row * n);
      });
      const double output_copy_ns = median_ns([&] {
        for (std::size_t row = 0; row < rows; ++row)
          std::copy(data.data() + row * n, data.data() + (row + 1) * n,
                    output.data() + row * n);
      });
      pack_ns = (input_pack_ns + output_copy_ns) / static_cast<double>(rows);
      call_ns = median_prepared_ns(fill_spectra, [&] {
        for (std::size_t row = 0; row < rows; ++row) {
          auto* packed = data.data() + row * n;
          pack_real_spectrum(spectra.data() + row * bins, n, packed);
          plan.exec_copyback(packed, scratch.data(), scale, false, 1);
          std::copy(packed, packed + n, output.data() + row * n);
        }
      }) / static_cast<double>(rows);
      checksum = output[0];
    }
    plan_ns = plan_construction_ns<ducc0::pocketfft_r<double>>(n, vectorize);
    scratch_ns = allocation_ns<double>(plan.bufsize());
  }

  std::cout << mode_name(mode) << ',' << n << ',' << std::setprecision(9)
            << plan_ns << ',' << scratch_ns << ',' << scratch_bytes << ','
            << fft_ns << ',' << pack_ns << ',' << call_ns << ','
            << call_ns * static_cast<double>(rows) << ',' << rows << ','
            << checksum << '\n';
}

void usage(const char* program) {
  std::cerr << "Usage:\n  " << program
            << " dump <mode> <length>\n  " << program
            << " dump-all\n  " << program
            << " bench [length ...]\n  " << program
            << " bench-sht\n  " << program
            << " bench-one <mode> <length> [rows]\n";
}

}  // namespace

int main(int argc, char** argv) {
  try {
    if (argc < 2) {
      usage(argv[0]);
      return 2;
    }
    const std::string command(argv[1]);
    if (command == "dump" && argc == 4) {
      print_dump(parse_mode(argv[2]), std::stoull(argv[3]));
      return 0;
    }
    if (command == "dump-all" && argc == 2) {
      for (const auto n : benchmark_lengths)
        for (const auto mode : {Mode::c2c_forward, Mode::c2c_inverse,
                                Mode::r2c, Mode::c2r})
          print_dump(mode, n);
      return 0;
    }
    if (command == "bench-one" && (argc == 4 || argc == 5)) {
      std::cout << "mode,n,plan_ns,scratch_ns,scratch_bytes,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum\n";
      if (argc == 5) {
        benchmark(parse_mode(argv[2]), std::stoull(argv[3]),
                  std::stoull(argv[4]));
      } else {
        benchmark(parse_mode(argv[2]), std::stoull(argv[3]), 1);
        benchmark(parse_mode(argv[2]), std::stoull(argv[3]), batch_count);
      }
      return 0;
    }
    if (command == "bench-sht" && argc == 2) {
      std::cout << "mode,n,plan_ns,scratch_ns,scratch_bytes,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum\n";
      for (const auto [n, rows] : sht_workloads)
        for (const auto mode : {Mode::c2c_forward, Mode::c2c_inverse,
                                Mode::r2c, Mode::c2r})
          benchmark(mode, n, rows);
      return 0;
    }
    if (command == "bench") {
      std::cout << "mode,n,plan_ns,scratch_ns,scratch_bytes,fft_ns,pack_ns,call_ns,batch_ns,batch_count,checksum\n";
      if (argc == 2) {
        for (const auto n : benchmark_lengths)
          for (const auto mode : {Mode::c2c_forward, Mode::c2c_inverse,
                                  Mode::r2c, Mode::c2r}) {
            benchmark(mode, n, 1);
            benchmark(mode, n, batch_count);
          }
      } else {
        for (int arg = 2; arg < argc; ++arg) {
          const auto n = std::stoull(argv[arg]);
          for (const auto mode : {Mode::c2c_forward, Mode::c2c_inverse,
                                  Mode::r2c, Mode::c2r}) {
            benchmark(mode, n, 1);
            benchmark(mode, n, batch_count);
          }
        }
      }
      return 0;
    }
    usage(argv[0]);
    return 2;
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
