// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstddef>
#include <cstdlib>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

#include "ducc0/infra/threading.h"

extern "C" void* plan14_ducc_create_plan(std::size_t n, std::size_t rows,
                                          std::size_t workers);
extern "C" std::size_t plan14_ducc_scratch_len(void* opaque);
extern "C" void plan14_ducc_inverse_rows(void* opaque, const double* spectra,
                                         double* output, double* scratch,
                                         double* packed);
extern "C" void plan14_ducc_destroy_plan(void* opaque);

namespace {

using Clock = std::chrono::steady_clock;
constexpr std::size_t batches = 30;
constexpr double target_batch_ms = 30.0;

std::vector<double> read_spectra(const char* path, std::size_t n,
                                 std::size_t rows) {
  std::ifstream input(path, std::ios::binary | std::ios::ate);
  if (!input) throw std::runtime_error("cannot open spectrum fixture");
  const auto size = input.tellg();
  const auto expected = static_cast<std::streamoff>(rows * (n / 2 + 1) * 2 * sizeof(double));
  if (size != expected) throw std::runtime_error("spectrum fixture has wrong size");
  std::vector<double> result(rows * (n / 2 + 1) * 2);
  input.seekg(0);
  input.read(reinterpret_cast<char*>(result.data()), size);
  if (!input) throw std::runtime_error("failed to read spectrum fixture");
  return result;
}

template <typename F> double time_batch(std::size_t count, F&& operation) {
  const auto begin = Clock::now();
  for (std::size_t i = 0; i < count; ++i) operation();
  return std::chrono::duration<double, std::milli>(Clock::now() - begin).count();
}

double median(std::vector<double>& values) {
  std::sort(values.begin(), values.end());
  return values[values.size() / 2];
}

}  // namespace

int main(int argc, char** argv) {
  try {
    if (argc != 6)
      throw std::runtime_error("usage: ducc-q8-odd-inverse INPUT N ROWS THREADS OUTPUT");
    const auto n = static_cast<std::size_t>(std::stoull(argv[2]));
    const auto rows = static_cast<std::size_t>(std::stoull(argv[3]));
    const auto workers = static_cast<std::size_t>(std::stoull(argv[4]));
    if (n < 2 || rows == 0 || workers == 0)
      throw std::runtime_error("N, ROWS, and THREADS must be positive");
    ducc0::resize_thread_pool(workers);
    if (ducc0::thread_pool_size() != workers)
      throw std::runtime_error("DUCC did not create the requested worker pool");

    auto spectra = read_spectra(argv[1], n, rows);
    std::vector<double> output(rows * n);
    void* plan = plan14_ducc_create_plan(n, rows, workers);
    const auto scratch_len = std::max<std::size_t>(1, plan14_ducc_scratch_len(plan));
    std::vector<double> scratch(workers * scratch_len);
    std::vector<double> packed(workers * n);
    auto operation = [&] {
      plan14_ducc_inverse_rows(plan, spectra.data(), output.data(), scratch.data(),
                               packed.data());
    };

    const double first_ms = time_batch(1, operation);
    for (int warm = 0; warm < 8; ++warm) operation();
    const double calibration_ms = time_batch(12, operation) / 12.0;
    const auto calls = std::max<std::size_t>(
        1, static_cast<std::size_t>(std::llround(target_batch_ms / calibration_ms)));
    std::vector<double> samples;
    samples.reserve(batches);
    for (std::size_t batch = 0; batch < batches; ++batch)
      samples.push_back(time_batch(calls, operation) / calls);

    std::vector<double> ordered = samples;
    std::sort(ordered.begin(), ordered.end());
    const double p10 = ordered[(batches - 1) / 10];
    const double p90 = ordered[(batches - 1) * 9 / 10];
    const double med = median(ordered);
    double checksum = 0.0;
    for (const auto value : output) {
      if (!std::isfinite(value)) throw std::runtime_error("nonfinite DUCC output");
      checksum += value;
    }
    std::ofstream result(argv[5], std::ios::binary);
    result.write(reinterpret_cast<const char*>(output.data()),
                 static_cast<std::streamsize>(output.size() * sizeof(double)));
    if (!result) throw std::runtime_error("failed to write DUCC output");

    std::cout << std::setprecision(9)
              << "environment,length=" << n << ",rows=" << rows
              << ",threads=" << workers << ",batch_calls=" << calls
              << ",batches=" << batches << ",target_batch_ms=" << target_batch_ms
              << ",first_ms=" << first_ms << '\n'
              << "summary,p10_ms=" << p10 << ",median_ms=" << med
              << ",p90_ms=" << p90 << ",checksum=" << checksum << '\n';
    for (std::size_t i = 0; i < samples.size(); ++i)
      std::cout << "batch," << i << ",ms=" << samples[i] << '\n';
    plan14_ducc_destroy_plan(plan);
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
  return 0;
}
