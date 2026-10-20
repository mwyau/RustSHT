// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
// Benchmark adapter for DUCC revision 80388904b8cfc32400ba84af8a6a45aabfe60f86.

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
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
  if (argc != 3) {
    std::cerr << "usage: ducc-analysis-prefix-bench FIXTURE REPETITIONS\n";
    return 2;
  }
  const std::string fixture_path = argv[1];
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
      coefficient_b(pair_count);
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
  auto copy_lanes = [](auto &destination, const std::vector<double> &source) {
    for (size_t i = 0; i < source.size(); ++i)
      destination[i / loop::VLEN][i % loop::VLEN] = source[i];
  };
  copy_lanes(data.csq, cosine_squared);
  auto reset = [&]() {
    for (size_t i = 0; i < vectors; ++i) {
      data.lam1[i] = loop::Tv(0.0);
      data.lam2[i] = loop::Tv(0x1p-100);
      data.scale[i] = loop::Tv(-1.0);
    }
  };
  auto run = [&]() {
    reset();
    size_t l = 0, il = 0;
    const auto started = Clock::now();
    loop::iter_to_ieee(generator, data, l, il, vectors, m, lmax + 1);
    return std::chrono::duration<double, std::micro>(Clock::now() - started).count();
  };

  const size_t repetitions = std::stoul(argv[2]);
  (void)run();
  std::vector<double> samples;
  for (size_t i = 0; i < repetitions; ++i) samples.push_back(run());
  std::sort(samples.begin(), samples.end());

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
            << " vectors=" << vectors << " repetitions=" << repetitions
            << " median_us=" << samples[samples.size()/2] << " samples_us=[";
  for (size_t i = 0; i < samples.size(); ++i) {
    if (i) std::cout << ",";
    std::cout << samples[i];
  }
  std::cout << "] state_relative_l2=" << std::scientific << state_relative_l2
            << " state_max_abs=" << state_max_abs << " scale_mismatches=" << scale_mismatches << "\n";
}
