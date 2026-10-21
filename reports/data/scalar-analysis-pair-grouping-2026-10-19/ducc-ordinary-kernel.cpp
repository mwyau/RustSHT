// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
// Benchmark driver for DUCC revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b.

#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <cmath>
#include <complex>
#include <cstdint>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <string>
#include <vector>

#include "ducc0/infra/simd.h"
#include "ducc0/sht/sht.h"
#include "ducc0/math/constants.h"
#include "ducc0/sht/sht_inner_loop.h"

namespace loop = ducc0::detail_sht::detail_sht_inner_loop;
using Clock = std::chrono::steady_clock;

extern "C" void rustsht_ducc_map2alm_kernel(
    loop::s0data_v *data, const loop::Ylmgen *generator,
    std::complex<double> *alm, size_t nth, size_t lstart, size_t lstop);

static inline void escape_memory(const void *pointer) {
#if defined(__GNUC__) || defined(__clang__)
  asm volatile("" : : "g"(pointer) : "memory");
#else
  (void)pointer;
  std::atomic_signal_fence(std::memory_order_seq_cst);
#endif
}

static void read_u64(std::ifstream &in, uint64_t &value) {
  in.read(reinterpret_cast<char *>(&value), sizeof(value));
  if (!in) throw std::runtime_error("truncated fixture header");
}
static void read_values(std::ifstream &in, std::vector<double> &values) {
  in.read(reinterpret_cast<char *>(values.data()),
           static_cast<std::streamsize>(values.size() * sizeof(double)));
  if (!in) throw std::runtime_error("truncated fixture values");
}
static double scaled_diff(double actual, double expected) {
  return std::abs(actual - expected) / std::max(1.0, std::abs(expected));
}

int main(int argc, char **argv) {
  if (argc != 4 && argc != 5) {
    std::cerr << "usage: ducc-analysis-kernel-bench FIXTURE --calibrate REPETITIONS | FIXTURE REPETITIONS BATCHES [--map2alm-kernel]\n";
    return 2;
  }
  const bool run_map2alm_kernel = argc == 5 && std::string(argv[4]) == "--map2alm-kernel";
  if (argc == 5 && !run_map2alm_kernel)
    throw std::runtime_error("unknown optional argument");
  const std::string fixture_path = argv[1];
  const bool calibrate = std::string(argv[2]) == "--calibrate";
  const size_t repetitions = calibrate ? std::stoul(argv[3]) : std::stoul(argv[2]);
  const size_t batch_count = calibrate ? 0 : std::stoul(argv[3]);
  if (repetitions == 0 || (!calibrate && batch_count == 0))
    throw std::runtime_error("requires positive repetitions and run batches");
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
    throw std::runtime_error("fixture lanes must be a multiple of DUCC's SIMD width");

  std::vector<double> cosine(lane_count), cosine_squared(lane_count),
      previous(lane_count), current(lane_count), same_real(lane_count),
      same_imag(lane_count), opposite_real(lane_count), opposite_imag(lane_count),
      coefficient_a(pair_count), coefficient_b(pair_count);
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
  in.close();

  loop::YlmBase base(lmax, m, 0);
  loop::Ylmgen generator(base);
  generator.prepare(m);
  double coefficient_error = 0.0;
  for (size_t i = 0; i < pair_count; ++i) {
    coefficient_error = std::max(coefficient_error,
        scaled_diff(generator.coef[i].a, coefficient_a[i]));
    coefficient_error = std::max(coefficient_error,
        scaled_diff(generator.coef[i].b, coefficient_b[i]));
  }
  if (coefficient_error > 2e-15)
    throw std::runtime_error("Rust and DUCC transformed recurrence coefficients differ");

  loop::s0data_v data{};
  const size_t vectors = lane_count / loop::VLEN;
  auto copy_lanes = [](auto &destination, const std::vector<double> &source) {
    for (size_t i = 0; i < source.size(); ++i)
      destination[i / loop::VLEN][i % loop::VLEN] = source[i];
  };
  copy_lanes(data.csq, cosine_squared);
  copy_lanes(data.p1r, same_real);
  copy_lanes(data.p1i, same_imag);
  copy_lanes(data.p2r, opposite_real);
  copy_lanes(data.p2i, opposite_imag);
  std::vector<std::complex<double>> alm(lmax + 2, {0.0, 0.0});

  auto reset = [&]() {
    copy_lanes(data.lam1, previous);
    copy_lanes(data.lam2, current);
    std::fill(alm.begin() + m, alm.begin() + lmax + 1,
        std::complex<double>(0.0, 0.0));
  };
  auto run = [&]() {
    if (run_map2alm_kernel)
      rustsht_ducc_map2alm_kernel(
          &data, &generator, alm.data(), lane_count, m, lmax + 1);
    escape_memory(alm.data());
  };

  auto keep_result = [&]() {
    double checksum = 0.0;
    for (auto value = alm.begin() + m; value != alm.begin() + lmax + 1; ++value)
      checksum += value->real() + 0.5 * value->imag();
    std::atomic_signal_fence(std::memory_order_seq_cst);
    return checksum;
  };
  auto reset_batch_us = [&]() {
    const auto started = Clock::now();
    for (size_t i = 0; i < repetitions; ++i) {
      reset();
      escape_memory(data.lam1.data());
      escape_memory(alm.data());
    }
    return std::chrono::duration<double, std::micro>(Clock::now() - started).count();
  };

  if (calibrate) {
    for (size_t i = 0; i < 4; ++i) {
      reset();
      run();
    }
    const auto started = Clock::now();
    for (size_t i = 0; i < repetitions; ++i) {
      reset();
      run();
    }
    const double elapsed_us = std::chrono::duration<double, std::micro>(Clock::now() - started).count();
    const double checksum = keep_result();
    std::cout << std::fixed << std::setprecision(6)
              << "DUCC_CALIBRATION lanes=" << lane_count
              << " vlen=" << loop::VLEN
              << " repetitions=" << repetitions
              << " reset_inclusive_us=" << elapsed_us / repetitions
              << " map2alm_kernel=" << (run_map2alm_kernel ? 1 : 0)
              << " checksum=" << std::setprecision(12) << checksum << "\n";
    return 0;
  }

  constexpr size_t warmup_repetitions = 3;
  double warmup_checksum = 0.0;
  double reference_checksum = 0.0;
  for (size_t i = 0; i < warmup_repetitions; ++i) {
    reset();
    run();
    const double checksum = keep_result();
    if (i == 0)
      reference_checksum = checksum;
    else if (std::abs(checksum - reference_checksum) >
             1e-12 * std::max(1.0, std::abs(reference_checksum)))
      throw std::runtime_error("DUCC warmup result changed between repetitions");
    warmup_checksum += checksum;
  }
  if (!std::isfinite(warmup_checksum))
    throw std::runtime_error("DUCC warmup produced a non-finite result");
  std::cout << std::fixed << std::setprecision(12)
            << "DUCC_WARMUP lanes=" << lane_count
            << " repetitions=" << warmup_repetitions
            << " checksum=" << warmup_checksum << "\n";

  for (size_t batch = 0; batch < batch_count; ++batch) {
    const double reset_us = reset_batch_us();
    const auto wall_started = Clock::now();
    for (size_t i = 0; i < repetitions; ++i) {
      reset();
      run();
    }
    const double wall_us = std::chrono::duration<double, std::micro>(Clock::now() - wall_started).count();
    const double checksum = keep_result();
    const double kernel_batch_us = wall_us - reset_us;
    if (std::abs(checksum - reference_checksum) >
        1e-12 * std::max(1.0, std::abs(reference_checksum)))
      throw std::runtime_error("DUCC kernel result changed between timed batches");
    std::cout << std::fixed << std::setprecision(3)
              << "DUCC_BATCH lanes=" << lane_count << " batch=" << batch
              << " repetitions=" << repetitions
              << " mode=" << (run_map2alm_kernel ? "map2alm_kernel" : "kernel_plus_commit")
              << " kernel_us=" << kernel_batch_us << " reset_us=" << reset_us
              << " wall_us=" << wall_us
              << " checksum=" << std::setprecision(12) << checksum << "\n";

    const double inclusive_reset_us = reset_batch_us();
    const auto reset_inclusive_started = Clock::now();
    for (size_t i = 0; i < repetitions; ++i) {
      reset();
      run();
      escape_memory(data.lam1.data());
    }
    const double reset_inclusive_us =
        std::chrono::duration<double, std::micro>(Clock::now() - reset_inclusive_started).count();
    const double reset_checksum = keep_result();
    if (std::abs(reset_checksum - reference_checksum) >
        1e-12 * std::max(1.0, std::abs(reference_checksum)))
      throw std::runtime_error("DUCC reset-inclusive result changed between timed batches");
    std::cout << std::fixed << std::setprecision(3)
              << "DUCC_BATCH lanes=" << lane_count << " batch=" << batch
              << " repetitions=" << repetitions << " mode=reset_inclusive"
              << " batch_us=" << reset_inclusive_us
              << " wall_us=" << reset_inclusive_us
              << " reset_us=" << inclusive_reset_us
              << " checksum=" << std::setprecision(12) << reset_checksum << "\n";
  }

  // Restore one validated result after the final reset-only diagnostic batch.
  // This work is outside every timed region and feeds the Rust/DUCC comparison.
  reset();
  run();
  const double final_checksum = keep_result();
  if (std::abs(final_checksum - reference_checksum) >
      1e-12 * std::max(1.0, std::abs(reference_checksum)))
    throw std::runtime_error("DUCC final result differs from the validated warmup");

  std::ifstream rust_in(fixture_path.substr(0, fixture_path.size() - 3) + ".rust",
                        std::ios::binary);
  if (!rust_in) throw std::runtime_error("cannot open Rust output for comparison");
  std::ofstream ducc_out(fixture_path.substr(0, fixture_path.size() - 3) + ".ducc",
                         std::ios::binary);
  if (!ducc_out) throw std::runtime_error("cannot create DUCC output fixture");
  double l2_error = 0.0, l2_reference = 0.0, max_error = 0.0;
  for (size_t offset = 0; offset <= lmax - m; ++offset) {
    double pair[2];
    rust_in.read(reinterpret_cast<char *>(pair), sizeof(pair));
    if (!rust_in) throw std::runtime_error("truncated Rust output");
    const std::complex<double> reference(pair[0], pair[1]);
    const auto difference = alm[m + offset] - reference;
    const double output_pair[2] = {alm[m + offset].real(), alm[m + offset].imag()};
    ducc_out.write(reinterpret_cast<const char *>(output_pair), sizeof(output_pair));
    l2_error += std::norm(difference);
    l2_reference += std::norm(reference);
    max_error = std::max(max_error, std::abs(difference));
  }
  const double relative_l2 = std::sqrt(l2_error / std::max(l2_reference, 1e-300));
  std::cout << std::fixed << std::setprecision(3)
            << "DUCC_KERNEL m=" << m << " lmax=" << lmax
            << " lanes=" << lane_count << " vectors=" << vectors
              << " map2alm_kernel=" << (run_map2alm_kernel ? 1 : 0)
            << " repetitions_per_batch=" << repetitions << " batches=" << batch_count
            << " coefficient_max_scaled_diff=" << std::scientific << coefficient_error
            << " output_relative_l2=" << relative_l2
            << " output_max_abs=" << max_error << "\n";
}
