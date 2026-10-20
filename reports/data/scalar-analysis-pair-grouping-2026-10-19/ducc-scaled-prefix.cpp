// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
// Benchmark adapter for DUCC revision 80388904b8cfc32400ba84af8a6a45aabfe60f86.

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

#include "ducc0/infra/simd.h"
#include "ducc0/sht/sht.h"
#include "ducc0/math/constants.h"
#include "ducc0/sht/sht_inner_loop.h"

namespace loop = ducc0::detail_sht::detail_sht_inner_loop;
using Clock = std::chrono::steady_clock;

static void read_u64(std::ifstream &in, uint64_t &value) {
  in.read(reinterpret_cast<char *>(&value), sizeof(value));
  if (!in) throw std::runtime_error("truncated fixture header");
}
static void read_values(std::ifstream &in, std::vector<double> &values) {
  in.read(reinterpret_cast<char *>(values.data()),
           static_cast<std::streamsize>(values.size() * sizeof(double)));
  if (!in) throw std::runtime_error("truncated fixture values");
}

int main(int argc, char **argv) {
  const bool calibrate = argc == 4 && std::string(argv[2]) == "--calibrate";
  if ((calibrate && argc != 4) || (!calibrate && argc != 5)) {
    std::cerr << "usage: ducc-analysis-prefix-bench FIXTURE --calibrate REPETITIONS | FIXTURE KERNEL_REPETITIONS WALL_REPETITIONS BATCHES\n";
    return 2;
  }
  const std::string fixture_path = argv[1];
  const size_t kernel_repetitions = calibrate ? std::stoul(argv[3]) : std::stoul(argv[2]);
  const size_t wall_repetitions = calibrate ? kernel_repetitions : std::stoul(argv[3]);
  const size_t batch_count = calibrate ? 0 : std::stoul(argv[4]);
  if (kernel_repetitions == 0 || wall_repetitions == 0 || (!calibrate && batch_count == 0))
    throw std::runtime_error("requires positive repetitions and batches");
  std::ifstream in(fixture_path, std::ios::binary);
  if (!in) throw std::runtime_error("cannot open Rust-generated fixture");
  uint64_t m64, lmax64, lane_count64, pair_count64;
  read_u64(in, m64);
  read_u64(in, lmax64);
  read_u64(in, lane_count64);
  read_u64(in, pair_count64);
  const size_t m = static_cast<size_t>(m64);
  const size_t lmax = static_cast<size_t>(lmax64);
  const size_t lane_count = static_cast<size_t>(lane_count64);
  const size_t pair_count = static_cast<size_t>(pair_count64);
  if (lane_count % loop::VLEN != 0)
    throw std::runtime_error("fixture lanes must be a multiple of DUCC SIMD width");

  std::vector<double> cosine(lane_count), cosine_squared(lane_count), previous(lane_count),
      current(lane_count), same_real(lane_count), same_imag(lane_count),
      opposite_real(lane_count), opposite_imag(lane_count), coefficient_a(pair_count),
      coefficient_b(pair_count), initial_scale_epochs(lane_count);
  read_values(in, cosine);
  read_values(in, cosine_squared);
  read_values(in, previous);
  read_values(in, current);
  read_values(in, same_real);
  read_values(in, same_imag);
  read_values(in, opposite_real);
  read_values(in, opposite_imag);
  read_values(in, coefficient_a);
  read_values(in, coefficient_b);
  read_values(in, initial_scale_epochs);
  (void)cosine;
  (void)same_real;
  (void)same_imag;
  (void)opposite_real;
  (void)opposite_imag;

  loop::YlmBase base(lmax, m, 0);
  loop::Ylmgen generator(base);
  generator.prepare(m);
  for (size_t i = 0; i < pair_count; ++i) {
    if (std::abs(generator.coef[i].a - coefficient_a[i]) > 1e-14 ||
        std::abs(generator.coef[i].b - coefficient_b[i]) > 1e-14)
      throw std::runtime_error("Rust and DUCC recurrence factors differ");
  }

  loop::s0data_v data{};
  const size_t vectors = lane_count / loop::VLEN;
  static_assert(sizeof(loop::Tv) == loop::VLEN * sizeof(double));
  auto copy_lanes = [](auto &destination, const std::vector<double> &source) {
    std::memcpy(destination.data(), source.data(), source.size() * sizeof(double));
  };
  copy_lanes(data.csq, cosine_squared);
  copy_lanes(data.lam1, previous);
  copy_lanes(data.lam2, current);
  copy_lanes(data.scale, initial_scale_epochs);
  auto reset = [&]() {
    copy_lanes(data.lam1, previous);
    copy_lanes(data.lam2, current);
    copy_lanes(data.scale, initial_scale_epochs);
  };
  auto compiler_barrier = [&]() {
    __asm__ volatile("" : : "g"(data.lam1.data()), "g"(data.lam2.data()),
                     "g"(data.scale.data()) : "memory");
  };
  size_t final_l = 0, final_il = 0;
  auto run_algorithm = [&]() {
    size_t l = 0, il = 0;
    loop::iter_to_ieee(generator, data, l, il, vectors, m, lmax + 1);
    std::atomic_signal_fence(std::memory_order_seq_cst);
    if (l > generator.lmax) il = (lmax - m + 2) / 2;
    final_l = l;
    final_il = il;
    compiler_barrier();
  };
  auto run = [&]() {
    const auto started = Clock::now();
    run_algorithm();
    return std::chrono::duration<double, std::micro>(Clock::now() - started).count();
  };
  auto measure_timer_overhead = [&](size_t count) {
    double elapsed_us = 0.0;
    for (size_t i = 0; i < count; ++i) {
      const auto started = Clock::now();
      std::atomic_signal_fence(std::memory_order_seq_cst);
      compiler_barrier();
      elapsed_us += std::chrono::duration<double, std::micro>(Clock::now() - started).count();
    }
    return elapsed_us / count;
  };
  auto choose_repetitions = [](double us_per_call) {
    constexpr double target_us = 32000.0;
    constexpr double max_batch_us = 80000.0;
    if (!(us_per_call > 0.0))
      throw std::runtime_error("non-positive calibration duration");
    const size_t minimum = static_cast<size_t>(std::ceil(20000.0 / us_per_call));
    const size_t maximum = static_cast<size_t>(std::floor(max_batch_us / us_per_call));
    if (minimum > maximum)
      throw std::runtime_error("kernel and reset-inclusive durations do not fit 20-80 ms");
    const size_t target = static_cast<size_t>(std::round(target_us / us_per_call));
    return std::min(std::max(target, minimum), maximum);
  };

  for (size_t i = 0; i < 4; ++i) {
    reset();
    run();
  }
  constexpr size_t timer_calibration_repetitions = 1024;
  if (calibrate) {
    double bootstrap_kernel_us = 0.0;
    for (size_t i = 0; i < kernel_repetitions; ++i) {
      reset();
      bootstrap_kernel_us += run();
      compiler_barrier();
    }
    const auto bootstrap_wall_started = Clock::now();
    for (size_t i = 0; i < wall_repetitions; ++i) {
      reset();
      run_algorithm();
    }
    const double bootstrap_wall_us =
        std::chrono::duration<double, std::micro>(Clock::now() - bootstrap_wall_started).count();
    const double bootstrap_timer_us_per_call = measure_timer_overhead(timer_calibration_repetitions);
    const double bootstrap_kernel_per_call =
        bootstrap_kernel_us / kernel_repetitions - bootstrap_timer_us_per_call;
    const double bootstrap_wall_per_call = bootstrap_wall_us / wall_repetitions;
    const size_t calibrated_kernel_repetitions = final_il == 0
        ? kernel_repetitions
        : choose_repetitions(bootstrap_kernel_per_call);
    const size_t calibrated_wall_repetitions = final_il == 0
        ? wall_repetitions
        : choose_repetitions(bootstrap_wall_per_call);
    double kernel_us = 0.0;
    for (size_t i = 0; i < calibrated_kernel_repetitions; ++i) {
      reset();
      kernel_us += run();
      compiler_barrier();
    }
    const auto wall_started = Clock::now();
    for (size_t i = 0; i < calibrated_wall_repetitions; ++i) {
      reset();
      run_algorithm();
    }
    const double wall_us = std::chrono::duration<double, std::micro>(Clock::now() - wall_started).count();
    const double timer_us_per_call = measure_timer_overhead(timer_calibration_repetitions);
    const double checksum = data.lam1[0][0] + data.lam2[0][0] + data.scale[0][0];
    std::cout << std::fixed << std::setprecision(6)
              << "DUCC_PREFIX_CALIBRATION m=" << m << " lanes=" << lane_count
              << " vlen=" << loop::VLEN << " vectors=" << vectors
              << " kernel_repetitions=" << calibrated_kernel_repetitions
              << " wall_repetitions=" << calibrated_wall_repetitions
              << " kernel_us_per_call=" << kernel_us / calibrated_kernel_repetitions - timer_us_per_call
              << " wall_us_per_call=" << wall_us / calibrated_wall_repetitions
              << " timer_us_per_call=" << timer_us_per_call
              << " checksum=" << checksum << "\n";
  } else {
    for (size_t warmup = 0; warmup < 2; ++warmup) {
      for (size_t i = 0; i < kernel_repetitions; ++i) {
        reset();
        run();
        compiler_barrier();
      }
      for (size_t i = 0; i < wall_repetitions; ++i) {
        reset();
        run_algorithm();
      }
    }
    const double timer_us_per_call = measure_timer_overhead(timer_calibration_repetitions);
    for (size_t batch = 0; batch < batch_count; ++batch) {
      double kernel_us = 0.0;
      for (size_t i = 0; i < kernel_repetitions; ++i) {
        reset();
        kernel_us += run();
        compiler_barrier();
      }
      const auto wall_started = Clock::now();
      for (size_t i = 0; i < wall_repetitions; ++i) {
        reset();
        run_algorithm();
      }
      const double wall_us = std::chrono::duration<double, std::micro>(Clock::now() - wall_started).count();
      const double checksum = data.lam1[0][0] + data.lam2[0][0] + data.scale[0][0];
      std::cout << std::fixed << std::setprecision(3)
                << "DUCC_PREFIX_KERNEL_BATCH m=" << m << " lanes=" << lane_count
                << " batch=" << batch << " repetitions=" << kernel_repetitions
                << " kernel_us=" << kernel_us
                << " timer_us_per_call=" << timer_us_per_call
                << " checksum=" << checksum << "\n";
      std::cout << "DUCC_PREFIX_WALL_BATCH m=" << m << " lanes=" << lane_count
                << " batch=" << batch << " repetitions=" << wall_repetitions
                << " wall_us=" << wall_us << "\n";
    }
  }

  const std::string state_path = fixture_path.substr(0, fixture_path.size() - 3) + ".rust_state";
  std::ifstream state_in(state_path, std::ios::binary);
  if (!state_in) throw std::runtime_error("cannot open Rust prefix state");
  std::vector<double> rust_previous(lane_count), rust_current(lane_count), rust_scale(lane_count);
  read_values(state_in, rust_previous);
  read_values(state_in, rust_current);
  read_values(state_in, rust_scale);
  double state_max_abs = 0.0;
  double state_l2_diff = 0.0;
  double state_l2_rust = 0.0;
  size_t scale_mismatches = 0;
  for (size_t i = 0; i < lane_count; ++i) {
    const double ducc_previous = data.lam1[i / loop::VLEN][i % loop::VLEN];
    const double ducc_current = data.lam2[i / loop::VLEN][i % loop::VLEN];
    const double ducc_scale = data.scale[i / loop::VLEN][i % loop::VLEN];
    for (const auto [ducc_value, rust_value] : {
             std::pair{ducc_previous, rust_previous[i]},
             std::pair{ducc_current, rust_current[i]}}) {
      state_max_abs = std::max(state_max_abs, std::abs(ducc_value - rust_value));
      state_l2_diff += (ducc_value - rust_value) * (ducc_value - rust_value);
      state_l2_rust += rust_value * rust_value;
    }
    if (ducc_scale != rust_scale[i]) ++scale_mismatches;
  }
  const double state_relative_l2 = std::sqrt(state_l2_diff / std::max(state_l2_rust, 1e-300));
  if (scale_mismatches != 0 || state_relative_l2 > 1e-12)
    throw std::runtime_error("Rust and DUCC prefix states differ");

  std::cout << std::fixed << std::setprecision(3)
            << "DUCC_PREFIX m=" << m << " lmax=" << lmax << " lanes=" << lane_count
            << " vlen=" << loop::VLEN << " vectors=" << vectors
            << " kernel_repetitions=" << kernel_repetitions << " wall_repetitions=" << wall_repetitions
            << " batches=" << batch_count << " prefix_l=" << final_l << " prefix_pair=" << final_il
            << " state_relative_l2=" << std::scientific << state_relative_l2
            << " state_max_abs=" << state_max_abs << " scale_mismatches=" << scale_mismatches << "\n";
}
