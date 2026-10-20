// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
// Benchmark adapter for DUCC revision 80388904b8cfc32400ba84af8a6a45aabfe60f86.

#include <algorithm>
#include <array>
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

static void read_u64(std::ifstream &in, uint64_t &value) {
  in.read(reinterpret_cast<char *>(&value), sizeof(value));
  if (!in) throw std::runtime_error("truncated fixture header");
}
static void read_values(std::ifstream &in, std::vector<double> &values) {
  in.read(reinterpret_cast<char *>(values.data()),
           static_cast<std::streamsize>(values.size() * sizeof(double)));
  if (!in) throw std::runtime_error("truncated fixture values");
}
static double relative_diff(double actual, double expected) {
  return std::abs(actual - expected) / std::max(1.0, std::abs(expected));
}

int main(int argc, char **argv) {
  if (argc != 3) {
    std::cerr << "usage: ducc-analysis-kernel-bench FIXTURE REPETITIONS\n";
    return 2;
  }
  const std::string fixture_path = argv[1];
  const size_t repetitions = std::stoul(argv[2]);
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
        relative_diff(generator.coef[i].a, coefficient_a[i]));
    coefficient_error = std::max(coefficient_error,
        relative_diff(generator.coef[i].b, coefficient_b[i]));
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
    std::fill(alm.begin(), alm.end(), std::complex<double>(0.0, 0.0));
  };
  auto run = [&]() {
    reset();
    const auto started = Clock::now();
    loop::map2alm_kernel(data, generator.coef, alm.data(), m, 0, lmax, vectors);
    const auto elapsed = Clock::now() - started;
    return std::chrono::duration<double, std::micro>(elapsed).count();
  };

  (void)run();
  std::vector<double> samples;
  samples.reserve(repetitions);
  for (size_t i = 0; i < repetitions; ++i)
    samples.push_back(run());
  std::sort(samples.begin(), samples.end());
  const double median = samples[samples.size() / 2];

  std::ifstream rust_in(fixture_path.substr(0, fixture_path.size() - 3) + ".rust",
                        std::ios::binary);
  if (!rust_in) throw std::runtime_error("cannot open Rust output for comparison");
  double l2_error = 0.0, l2_reference = 0.0, max_error = 0.0;
  for (size_t offset = 0; offset <= lmax - m; ++offset) {
    double pair[2];
    rust_in.read(reinterpret_cast<char *>(pair), sizeof(pair));
    if (!rust_in) throw std::runtime_error("truncated Rust output");
    const std::complex<double> reference(pair[0], pair[1]);
    const auto difference = alm[m + offset] - reference;
    l2_error += std::norm(difference);
    l2_reference += std::norm(reference);
    max_error = std::max(max_error, std::abs(difference));
  }
  const double relative_l2 = std::sqrt(l2_error / std::max(l2_reference, 1e-300));
  std::cout << std::fixed << std::setprecision(3)
            << "DUCC_KERNEL m=" << m << " lmax=" << lmax
            << " lanes=" << lane_count << " vectors=" << vectors
            << " repetitions=" << repetitions << " median_us=" << median
            << " samples_us=[";
  for (size_t i = 0; i < samples.size(); ++i) {
    if (i) std::cout << ",";
    std::cout << samples[i];
  }
  std::cout << "] coefficient_max_rel=" << std::scientific << coefficient_error
            << " output_relative_l2=" << relative_l2
            << " output_max_abs=" << max_error << "\n";
}
