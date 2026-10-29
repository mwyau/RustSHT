// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

// Native benchmark adapter for pinned DUCC's public spin-1 SHT API.
// DUCC source files are linked unmodified and use the flags in CMakeLists.txt.

#include <algorithm>
#include <chrono>
#include <cmath>
#include <complex>
#include <cstddef>
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
using Complex = std::complex<double>;
using ducc0::cmav;
using ducc0::vmav;

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
double checksum(const std::vector<T> &values) {
  double result = 0;
  const std::size_t stride = std::max<std::size_t>(1, values.size() / 64);
  for (std::size_t i = 0, used = 0; i < values.size() && used < 64; i += stride, ++used)
    result += static_cast<double>(values[i]);
  return result;
}

template <>
double checksum(const std::vector<Complex> &values) {
  double result = 0;
  const std::size_t stride = std::max<std::size_t>(1, values.size() / 64);
  for (std::size_t i = 0, used = 0; i < values.size() && used < 64; i += stride, ++used)
    result += values[i].real() + values[i].imag();
  return result;
}

template <typename T>
bool same_bits(const std::vector<T> &a, const std::vector<T> &b) {
  return a == b;
}

template <typename Func>
double time_once(Func &&function) {
  const auto start = Clock::now();
  function();
  return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

void worker(int argc, char **argv) {
  if (argc != 19 || std::string(argv[1]) != "--worker")
    throw std::runtime_error("usage: --worker OP GRID THREADS NTHETA NPHI LMAX MMAX PHI0 MAP0 MAP1 E B OUT0 OUT1 OUTE OUTB TARGET_MS");
  const std::string operation(argv[2]);
  const std::string grid(argv[3]);
  if ((operation != "analysis" && operation != "synthesis") ||
      (grid != "GL" && grid != "CC" && grid != "F1"))
    throw std::runtime_error("operation must be analysis/synthesis and grid GL/CC/F1");
  const std::size_t threads = parse_size(argv[4]);
  const std::size_t ntheta = parse_size(argv[5]);
  const std::size_t nphi = parse_size(argv[6]);
  const std::size_t lmax = parse_size(argv[7]);
  const std::size_t mmax = parse_size(argv[8]);
  const double phi0 = parse_double(argv[9]);
  const double target_ms = parse_double(argv[18]);
  const std::size_t map_count = ntheta * nphi;
  const std::size_t nalm = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) / 2;
  const std::size_t prefix = mmax;
  const std::size_t padded_alm_count = prefix + nalm;

  const auto read_start = Clock::now();
  const auto map0 = read_values<double>(argv[10], map_count);
  const auto map1 = read_values<double>(argv[11], map_count);
  const auto e_compact = read_values<Complex>(argv[12], nalm);
  const auto b_compact = read_values<Complex>(argv[13], nalm);
  const double read_ms = std::chrono::duration<double, std::milli>(Clock::now() - read_start).count();

  const auto setup_start = Clock::now();
  std::vector<double> map_input(2 * map_count), map_output(2 * map_count, 0.0);
  std::copy(map0.begin(), map0.end(), map_input.begin());
  std::copy(map1.begin(), map1.end(), map_input.begin() + map_count);
  std::vector<Complex> alm_input(2 * padded_alm_count, Complex(0, 0));
  std::vector<Complex> alm_output(2 * padded_alm_count, Complex(0, 0));
  std::copy(e_compact.begin(), e_compact.end(), alm_input.begin() + prefix);
  std::copy(b_compact.begin(), b_compact.end(), alm_input.begin() + padded_alm_count + prefix);
  std::vector<std::size_t> mstart(mmax + 1);
  std::size_t offset = 0;
  for (std::size_t m = 0; m <= mmax; ++m) {
    mstart[m] = prefix + offset - m;
    offset += lmax - m + 1;
  }

  const cmav<double, 3> map_in(map_input.data(), {2, ntheta, nphi});
  const vmav<double, 3> map_out(map_output.data(), {2, ntheta, nphi});
  const cmav<Complex, 2> alm_in(alm_input.data(), {2, padded_alm_count});
  const vmav<Complex, 2> alm_out(alm_output.data(), {2, padded_alm_count});
  const cmav<std::size_t, 1> mstart_view(mstart.data(), {mstart.size()});
  const std::vector<double> ringfactor_values(ntheta, 1.0);
  const cmav<double, 1> ringfactor(ringfactor_values.data(), {ntheta});
  const double setup_ms = std::chrono::duration<double, std::milli>(Clock::now() - setup_start).count();

  auto run_once = [&]() {
    if (operation == "analysis")
      ducc0::analysis_2d(alm_out, map_in, 1, lmax, mstart_view, 1, grid, phi0, ringfactor, threads);
    else
      ducc0::synthesis_2d(alm_in, map_out, 1, lmax, mstart_view, 1, grid, phi0, ringfactor, threads, ducc0::STANDARD);
  };

  const double first_ms = time_once(run_once);
  const auto first_maps = map_output;
  const auto first_alms = alm_output;
  const double warmup_ms = time_once(run_once);
  const bool repeat_consistent = operation == "analysis" ? same_bits(first_alms, alm_output)
                                                           : same_bits(first_maps, map_output);
  if (operation == "analysis") {
    for (std::size_t m = 0; m <= mmax; ++m)
      for (std::size_t l = std::max<std::size_t>(m, 1); l <= lmax; ++l)
        for (std::size_t component = 0; component < 2; ++component)
          alm_output[component * padded_alm_count + mstart[m] + l] =
              Complex(std::numeric_limits<double>::quiet_NaN(), std::numeric_limits<double>::quiet_NaN());
  } else {
    std::fill(map_output.begin(), map_output.end(), std::numeric_limits<double>::quiet_NaN());
  }
  run_once();
  const bool reset_consistent = operation == "analysis" ? same_bits(first_alms, alm_output)
                                                          : same_bits(first_maps, map_output);

  std::size_t repetitions = warmup_ms >= 100.0 ? 1 : std::clamp<std::size_t>(
      static_cast<std::size_t>(std::ceil(target_ms / std::max(warmup_ms, 0.000001))), 1, 2000000);
  if (warmup_ms < 100.0) {
    for (int calibration = 0; calibration < 3; ++calibration) {
      const double elapsed = time_once([&]() {
        for (std::size_t repeat = 0; repeat < repetitions; ++repeat) {
          run_once();
        }
      });
      if (elapsed >= 20.0 && elapsed <= 100.0) break;
      repetitions = std::clamp<std::size_t>(static_cast<std::size_t>(std::ceil(repetitions * target_ms / std::max(elapsed, 0.000001))), 1, 2000000);
    }
  }
  const std::size_t batch_count = warmup_ms >= 100.0 ? 5 : 10;
  std::vector<double> batch_ms, batch_total_ms;
  double output_checksum = 0;
  for (std::size_t batch = 0; batch < batch_count; ++batch) {
    const double elapsed = time_once([&]() {
      for (std::size_t repeat = 0; repeat < repetitions; ++repeat) {
        run_once();
      }
    });
    batch_total_ms.push_back(elapsed);
    batch_ms.push_back(elapsed / repetitions);
    output_checksum += operation == "analysis" ? checksum(alm_output) : checksum(map_output);
  }
  if (operation == "analysis") {
    std::vector<Complex> out_e(nalm), out_b(nalm);
    std::copy_n(alm_output.begin() + prefix, nalm, out_e.begin());
    std::copy_n(alm_output.begin() + padded_alm_count + prefix, nalm, out_b.begin());
    write_values(argv[16], out_e);
    write_values(argv[17], out_b);
  } else {
    std::vector<double> out0(map_output.begin(), map_output.begin() + map_count);
    std::vector<double> out1(map_output.begin() + map_count, map_output.end());
    write_values(argv[14], out0);
    write_values(argv[15], out1);
  }
  std::cout << std::setprecision(17)
            << "{\"backend\":\"ducc\",\"operation\":\"" << operation
            << "\",\"grid\":\"" << grid << "\",\"threads\":" << threads
            << ",\"lmax\":" << lmax << ",\"mmax\":" << mmax
            << ",\"first_ms\":" << first_ms << ",\"warmup_ms\":" << warmup_ms
            << ",\"setup_ms\":" << setup_ms << ",\"fixture_load_ms\":" << read_ms << ",\"repetitions_per_batch\":" << repetitions
            << ",\"batch_ms\":[";
  for (std::size_t i = 0; i < batch_ms.size(); ++i) std::cout << (i ? "," : "") << batch_ms[i];
  std::cout << "],\"batch_total_ms\":[";
  for (std::size_t i = 0; i < batch_total_ms.size(); ++i) std::cout << (i ? "," : "") << batch_total_ms[i];
  std::cout << "],\"output_checksum\":" << output_checksum
            << ",\"repeat_consistent\":" << (repeat_consistent ? "true" : "false")
            << ",\"reset_consistent\":" << (reset_consistent ? "true" : "false") << "}\n";
  if (!repeat_consistent || !reset_consistent) throw std::runtime_error("repeat/reset output mismatch");
}
} // namespace

int main(int argc, char **argv) {
  try { worker(argc, argv); }
  catch (const std::exception &error) { std::cerr << error.what() << '\n'; return 1; }
}
