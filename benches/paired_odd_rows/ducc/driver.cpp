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

extern "C" void* rustsht_paired_rows_ducc_create(std::size_t n,
                                                  std::size_t rows,
                                                  std::size_t modes,
                                                  std::size_t workers);
extern "C" std::size_t rustsht_paired_rows_ducc_scratch_len(void* opaque);
extern "C" void rustsht_paired_rows_ducc_execute(
    void* opaque, bool inverse, const double* input, double* output,
    double* scratch, double* row_buffers);
extern "C" void rustsht_paired_rows_ducc_destroy(void* opaque);

namespace {

using Clock = std::chrono::steady_clock;
constexpr std::size_t batches = 30;
constexpr double target_batch_ms = 30.0;

std::vector<double> read_values(const char* path, std::size_t count) {
  std::ifstream input(path, std::ios::binary | std::ios::ate);
  if (!input) throw std::runtime_error("cannot open input fixture");
  const auto size = input.tellg();
  const auto expected = static_cast<std::streamoff>(count * sizeof(double));
  if (size != expected) throw std::runtime_error("input fixture has wrong size");
  std::vector<double> result(count);
  input.seekg(0);
  input.read(reinterpret_cast<char*>(result.data()), size);
  if (!input) throw std::runtime_error("failed to read input fixture");
  return result;
}

template <typename F>
double time_batch(std::size_t count, F&& operation) {
  const auto begin = Clock::now();
  for (std::size_t i = 0; i < count; ++i) operation();
  return std::chrono::duration<double, std::milli>(Clock::now() - begin).count();
}

double percentile(const std::vector<double>& sorted, std::size_t numerator,
                  std::size_t denominator) {
  return sorted[(sorted.size() - 1) * numerator / denominator];
}

void write_values(const char* path, const std::vector<double>& values) {
  std::ofstream output(path, std::ios::binary);
  output.write(reinterpret_cast<const char*>(values.data()),
               static_cast<std::streamsize>(values.size() * sizeof(double)));
  if (!output) throw std::runtime_error("failed to write output fixture");
}

}  // namespace

int main(int argc, char** argv) {
  try {
    if (argc != 8 && argc != 9)
      throw std::runtime_error(
          "usage: ducc-row-pair OPERATION N ROWS THREADS SHAPE INPUT OUTPUT [--once]");
    const std::string operation(argv[1]);
    if (operation != "forward" && operation != "inverse")
      throw std::runtime_error("operation must be forward or inverse");
    const auto n = static_cast<std::size_t>(std::stoull(argv[2]));
    const auto rows = static_cast<std::size_t>(std::stoull(argv[3]));
    const auto workers = static_cast<std::size_t>(std::stoull(argv[4]));
    const std::string shape(argv[5]);
    const bool once = argc == 9 && std::string(argv[8]) == "--once";
    if (shape != "full" && shape != "reduced" && shape != "polar")
      throw std::runtime_error("shape must be full, reduced, or polar");
    if (n < 2 || rows == 0 || workers == 0)
      throw std::runtime_error("N, ROWS, and THREADS must be positive");

    const auto bins = n / 2 + 1;
    const auto max_order = shape == "reduced" ? (bins - 1) / 2 : bins - 1;
    const auto modes = max_order + 1;
    const bool inverse = operation == "inverse";
    const auto input_count = inverse ? rows * bins * 2 : rows * n;
    const auto output_count = inverse ? rows * n : rows * modes * 2;
    auto input = read_values(argv[6], input_count);
    std::vector<double> output(output_count);

    ducc0::resize_thread_pool(workers);
    if (ducc0::thread_pool_size() != workers)
      throw std::runtime_error("DUCC did not create the requested worker pool");
    void* plan = rustsht_paired_rows_ducc_create(n, rows, modes, workers);
    const auto scratch_len = std::max<std::size_t>(1,
        rustsht_paired_rows_ducc_scratch_len(plan));
    std::vector<double> scratch(workers * scratch_len);
    std::vector<double> row_buffers(workers * n);
    auto execute = [&] {
      rustsht_paired_rows_ducc_execute(plan, inverse, input.data(), output.data(),
                                       scratch.data(), row_buffers.data());
    };

    const double first_ms = time_batch(1, execute);
    if (once) {
      write_values(argv[7], output);
      std::cout << "once,route=ducc-real,operation=" << operation << ",n=" << n
                << ",rows=" << rows << ",threads=" << workers << ",shape="
                << shape << '\n';
      rustsht_paired_rows_ducc_destroy(plan);
      return 0;
    }

    for (int warm = 0; warm < 8; ++warm) execute();
    const double calibration_ms = time_batch(12, execute) / 12.0;
    const auto calls = std::max<std::size_t>(
        1, static_cast<std::size_t>(std::llround(target_batch_ms / calibration_ms)));
    std::vector<double> samples;
    samples.reserve(batches);
    for (std::size_t batch = 0; batch < batches; ++batch)
      samples.push_back(time_batch(calls, execute) / calls);

    std::vector<double> ordered = samples;
    std::sort(ordered.begin(), ordered.end());
    const double p10 = percentile(ordered, 1, 10);
    const double median = ordered[ordered.size() / 2];
    const double p90 = percentile(ordered, 9, 10);
    double checksum = 0.0;
    for (const auto value : output) {
      if (!std::isfinite(value)) throw std::runtime_error("nonfinite DUCC output");
      checksum += value;
    }
    write_values(argv[7], output);
    std::cout << std::setprecision(9)
              << "environment,route=ducc-real,operation=" << operation
              << ",n=" << n << ",rows=" << rows << ",modes=" << modes
              << ",shape=" << shape << ",threads=" << workers
              << ",batch_calls=" << calls << ",batches=" << batches
              << ",target_batch_ms=" << target_batch_ms << ",first_ms=" << first_ms
              << ",calibration_ms=" << calibration_ms
              << ",scratch_bytes_per_worker="
              << (scratch_len + n) * sizeof(double) << '\n'
              << "summary,p10_ms=" << p10 << ",median_ms=" << median
              << ",p90_ms=" << p90 << ",us_per_row_pair="
              << median * 1000.0 / (rows / 2.0) << ",checksum=" << checksum << '\n';
    for (std::size_t i = 0; i < samples.size(); ++i)
      std::cout << "batch," << i << ",ms=" << samples[i] << '\n';
    rustsht_paired_rows_ducc_destroy(plan);
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
  return 0;
}
