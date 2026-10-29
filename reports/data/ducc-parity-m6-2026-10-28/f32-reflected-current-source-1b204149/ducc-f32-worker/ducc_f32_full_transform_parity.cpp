// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

// Native f32 adapter around pinned DUCC's public SHT API. DUCC sources are
// linked unmodified; only this driver is compiled without fast-math.

#include <algorithm>
#include <chrono>
#include <cmath>
#include <complex>
#include <cstddef>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <stdexcept>
#include <string>
#include <vector>

#include "ducc0/infra/mav.h"
#include "ducc0/sht/sht.h"

namespace {
using Clock = std::chrono::steady_clock;
using Complex = std::complex<float>;
using ducc0::cmav;
using ducc0::vmav;
static_assert(sizeof(Complex) == 2 * sizeof(float));

std::size_t parse_size(const char *value) { return std::stoull(value); }
double parse_double(const char *value) { return std::stod(value); }

template <typename T>
std::vector<T> read_values(const std::string &path, std::size_t count) {
  std::vector<T> result(count);
  std::ifstream input(path, std::ios::binary);
  if (!input) throw std::runtime_error("cannot open input: " + path);
  input.read(reinterpret_cast<char *>(result.data()),
             static_cast<std::streamsize>(count * sizeof(T)));
  if (input.gcount() != static_cast<std::streamsize>(count * sizeof(T)))
    throw std::runtime_error("wrong input length: " + path);
  return result;
}

template <typename T>
void write_values(const std::string &path, const std::vector<T> &values) {
  std::ofstream output(path, std::ios::binary);
  if (!output) throw std::runtime_error("cannot open output: " + path);
  output.write(reinterpret_cast<const char *>(values.data()),
               static_cast<std::streamsize>(values.size() * sizeof(T)));
  if (!output) throw std::runtime_error("write failed: " + path);
}

template <typename T>
bool same_bits(const std::vector<T> &a, const std::vector<T> &b) {
  if (a.size() != b.size()) return false;
  for (std::size_t i = 0; i < a.size(); ++i)
    if (std::memcmp(&a[i], &b[i], sizeof(T)) != 0) return false;
  return true;
}

double checksum(const std::vector<float> &values, std::size_t count) {
  double result = 0;
  const auto stride = std::max<std::size_t>(1, count / 64);
  for (std::size_t i = 0, used = 0; i < count && used < 64; i += stride, ++used)
    result += values[i];
  return result;
}

double checksum(const std::vector<Complex> &values, std::size_t count) {
  double result = 0;
  const auto stride = std::max<std::size_t>(1, count / 64);
  for (std::size_t i = 0, used = 0; i < count && used < 64; i += stride, ++used)
    result += values[i].real() + values[i].imag();
  return result;
}

double checksum_active_alms(const std::vector<Complex> &values, std::size_t ncomp,
                            std::size_t padded_count, std::size_t lmax,
                            std::size_t mmax, const std::vector<std::size_t> &mstart) {
  double result = 0;
  const auto active_count = ncomp * ((mmax + 1) * (lmax + 1) - mmax * (mmax + 1) / 2);
  const auto stride = std::max<std::size_t>(1, active_count / 64);
  std::size_t active_index = 0, used = 0;
  for (std::size_t c = 0; c < ncomp; ++c)
    for (std::size_t m = 0; m <= mmax; ++m)
      for (std::size_t l = m; l <= lmax; ++l, ++active_index)
        if (active_index % stride == 0 && used < 64) {
          const auto value = values[c * padded_count + mstart[m] + l];
          result += value.real() + value.imag();
          ++used;
        }
  return result;
}

template <typename Func>
double time_once(Func &&function) {
  const auto start = Clock::now();
  function();
  return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

void worker(int argc, char **argv) {
  if (argc != 18 || std::string(argv[1]) != "--worker")
    throw std::runtime_error("usage: --worker OP SPIN GRID THREADS NTHETA NPHI LMAX MMAX PHI0 MAP0 MAP1 ALM0 ALM1 OUT0 OUT1 TARGET_MS");
  const std::string operation(argv[2]);
  const std::size_t spin = parse_size(argv[3]);
  const std::string grid(argv[4]);
  if ((operation != "analysis" && operation != "synthesis") || spin > 1 ||
      (grid != "GL" && grid != "CC" && grid != "F1"))
    throw std::runtime_error("invalid operation, spin or grid");
  const std::size_t threads = parse_size(argv[5]);
  const std::size_t ntheta = parse_size(argv[6]);
  const std::size_t nphi = parse_size(argv[7]);
  const std::size_t lmax = parse_size(argv[8]);
  const std::size_t mmax = parse_size(argv[9]);
  const double phi0 = parse_double(argv[10]);
  const double target_ms = parse_double(argv[17]);
  const std::size_t ncomp = spin == 0 ? 1 : 2;
  const std::size_t map_count = ntheta * nphi;
  const std::size_t nalm = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) / 2;
  const std::size_t padded_count = mmax + nalm;

  const auto read_start = Clock::now();
  const auto map0 = read_values<float>(argv[11], map_count);
  const auto map1 = read_values<float>(argv[12], map_count);
  const auto alm0 = read_values<Complex>(argv[13], nalm);
  const auto alm1 = read_values<Complex>(argv[14], nalm);
  const double fixture_load_ms = std::chrono::duration<double, std::milli>(Clock::now() - read_start).count();

  const auto setup_start = Clock::now();
  std::vector<float> map_input(ncomp * map_count, 0.f), map_output(ncomp * map_count,
      std::numeric_limits<float>::quiet_NaN());
  std::copy(map0.begin(), map0.end(), map_input.begin());
  if (spin == 1) std::copy(map1.begin(), map1.end(), map_input.begin() + map_count);
  std::vector<Complex> alm_input(ncomp * padded_count, Complex(0, 0));
  std::vector<Complex> alm_output(ncomp * padded_count,
      Complex(std::numeric_limits<float>::quiet_NaN(), std::numeric_limits<float>::quiet_NaN()));
  std::vector<std::size_t> mstart(mmax + 1);
  std::size_t offset = 0;
  for (std::size_t m = 0; m <= mmax; ++m) {
    mstart[m] = mmax + offset - m;
    offset += lmax - m + 1;
  }
  for (std::size_t m = 0, i = 0; m <= mmax; ++m)
    for (std::size_t l = m; l <= lmax; ++l, ++i) {
      alm_input[mstart[m] + l] = alm0[i];
      if (spin == 1) alm_input[padded_count + mstart[m] + l] = alm1[i];
    }

  const cmav<float, 3> map_in(map_input.data(), {ncomp, ntheta, nphi});
  const vmav<float, 3> map_out(map_output.data(), {ncomp, ntheta, nphi});
  const cmav<Complex, 2> alm_in(alm_input.data(), {ncomp, padded_count});
  const vmav<Complex, 2> alm_out(alm_output.data(), {ncomp, padded_count});
  const cmav<std::size_t, 1> mstart_view(mstart.data(), {mstart.size()});
  const std::vector<double> ringfactor_values(ntheta, 1.0);
  const cmav<double, 1> ringfactor(ringfactor_values.data(), {ntheta});
  const double setup_ms = std::chrono::duration<double, std::milli>(Clock::now() - setup_start).count();

  auto run_once = [&]() {
    if (operation == "analysis")
      ducc0::analysis_2d(alm_out, map_in, spin, lmax, mstart_view, 1,
                         grid, phi0, ringfactor, threads);
    else
      ducc0::synthesis_2d(alm_in, map_out, spin, lmax, mstart_view, 1,
                          grid, phi0, ringfactor, threads, ducc0::STANDARD);
  };
  const double first_ms = time_once(run_once);
  const auto first_maps = map_output;
  const auto first_alms = alm_output;
  const double warmup_ms = time_once(run_once);
  const bool repeat_consistent = operation == "analysis"
      ? same_bits(first_alms, alm_output) : same_bits(first_maps, map_output);
  if (operation == "analysis") {
    for (std::size_t m = 0; m <= mmax; ++m)
      for (std::size_t l = m; l <= lmax; ++l)
        for (std::size_t c = 0; c < ncomp; ++c)
          alm_output[c * padded_count + mstart[m] + l] = Complex(
              std::numeric_limits<float>::quiet_NaN(), std::numeric_limits<float>::quiet_NaN());
  } else {
    std::fill(map_output.begin(), map_output.end(), std::numeric_limits<float>::quiet_NaN());
  }
  run_once();
  const bool reset_consistent = operation == "analysis"
      ? same_bits(first_alms, alm_output) : same_bits(first_maps, map_output);

  std::size_t repetitions = warmup_ms >= 100.0 ? 1 : std::clamp<std::size_t>(
      static_cast<std::size_t>(std::ceil(target_ms / std::max(warmup_ms, 0.000001))), 1, 2000000);
  if (warmup_ms < 100.0) {
    for (int calibration = 0; calibration < 3; ++calibration) {
      const double elapsed = time_once([&]() {
        for (std::size_t repeat = 0; repeat < repetitions; ++repeat) run_once();
      });
      if (elapsed >= 20.0 && elapsed <= 100.0) break;
      repetitions = std::clamp<std::size_t>(static_cast<std::size_t>(
          std::ceil(repetitions * target_ms / std::max(elapsed, 0.000001))), 1, 2000000);
    }
  }
  const std::size_t batch_count = warmup_ms >= 100.0 ? 5 : 10;
  std::vector<double> batch_ms, batch_total_ms;
  double output_checksum = 0;
  for (std::size_t batch = 0; batch < batch_count; ++batch) {
    const double elapsed = time_once([&]() {
      for (std::size_t repeat = 0; repeat < repetitions; ++repeat) run_once();
    });
    batch_total_ms.push_back(elapsed);
    batch_ms.push_back(elapsed / repetitions);
    output_checksum += operation == "analysis"
        ? checksum_active_alms(alm_output, ncomp, padded_count, lmax, mmax, mstart)
        : checksum(map_output, ncomp * map_count);
  }
  if (operation == "analysis") {
    std::vector<Complex> out0(nalm), out1(nalm);
    for (std::size_t m = 0, i = 0; m <= mmax; ++m)
      for (std::size_t l = m; l <= lmax; ++l, ++i) {
        out0[i] = alm_output[mstart[m] + l];
        if (spin == 1) out1[i] = alm_output[padded_count + mstart[m] + l];
      }
    write_values(argv[15], out0);
    if (spin == 1) write_values(argv[16], out1);
  } else {
    std::vector<float> out0(map_count), out1(map_count);
    std::copy_n(map_output.begin(), map_count, out0.begin());
    if (spin == 1) std::copy_n(map_output.begin() + map_count, map_count, out1.begin());
    write_values(argv[15], out0);
    if (spin == 1) write_values(argv[16], out1);
  }
  auto print_array = [](const std::vector<double> &values) {
    std::cout << '[';
    for (std::size_t i = 0; i < values.size(); ++i)
      std::cout << (i ? "," : "") << values[i];
    std::cout << ']';
  };
  bool finite_outputs = true;
  if (operation == "analysis") {
    for (std::size_t m = 0; m <= mmax; ++m)
      for (std::size_t l = m; l <= lmax; ++l)
        for (std::size_t c = 0; c < ncomp; ++c) {
          const auto value = alm_output[c * padded_count + mstart[m] + l];
          finite_outputs = finite_outputs && std::isfinite(value.real()) && std::isfinite(value.imag());
        }
  } else {
    finite_outputs = std::all_of(map_output.begin(), map_output.end(), [](auto v) { return std::isfinite(v); });
  }
  std::cout << std::setprecision(17)
      << "{\"backend\":\"ducc\",\"operation\":\"" << operation
      << "\",\"spin\":" << spin << ",\"grid\":\"" << grid
      << "\",\"threads\":" << threads << ",\"lmax\":" << lmax
      << ",\"mmax\":" << mmax << ",\"first_ms\":" << first_ms
      << ",\"warmup_ms\":" << warmup_ms << ",\"setup_ms\":" << setup_ms
      << ",\"fixture_load_ms\":" << fixture_load_ms
      << ",\"repetitions_per_batch\":" << repetitions << ",\"batch_ms\":";
  print_array(batch_ms);
  std::cout << ",\"batch_total_ms\":";
  print_array(batch_total_ms);
  std::cout << ",\"output_checksum\":" << output_checksum
      << ",\"finite_outputs\":" << (finite_outputs ? "true" : "false")
      << ",\"repeat_consistent\":" << (repeat_consistent ? "true" : "false")
      << ",\"reset_consistent\":" << (reset_consistent ? "true" : "false") << "}\n";
  if (!finite_outputs || !repeat_consistent || !reset_consistent)
    throw std::runtime_error("nonfinite output or repeated/reset output mismatch");
}
} // namespace

int main(int argc, char **argv) {
  try { worker(argc, argv); }
  catch (const std::exception &error) { std::cerr << error.what() << '\n'; return 1; }
}
