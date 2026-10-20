// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

// Native benchmark adapter for the pinned DUCC public GL transform API.
// DUCC implementation source remains unmodified and is supplied at build time.

#include <algorithm>
#include <chrono>
#include <cmath>
#include <complex>
#include <cstddef>
#include <cstdint>
#include <fstream>
#include <iostream>
#include <iomanip>
#include <limits>
#include <stdexcept>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#include "ducc0/infra/mav.h"
#include "ducc0/infra/threading.h"
#include "ducc0/sht/sht.h"

namespace {

using Clock = std::chrono::steady_clock;
using Complex = std::complex<double>;
using ducc0::cmav;
using ducc0::vmav;

std::size_t parse_size(const char *value, const char *name) {
  try {
    return std::stoull(value);
  } catch (const std::exception &error) {
    throw std::runtime_error(std::string("invalid ") + name + ": " + error.what());
  }
}

double parse_double(const char *value, const char *name) {
  try {
    return std::stod(value);
  } catch (const std::exception &error) {
    throw std::runtime_error(std::string("invalid ") + name + ": " + error.what());
  }
}

template <typename T>
std::vector<T> read_values(const std::string &path, std::size_t count) {
  std::vector<T> result(count);
  std::ifstream input(path, std::ios::binary);
  if (!input)
    throw std::runtime_error("cannot open input: " + path);
  input.read(reinterpret_cast<char *>(result.data()),
             static_cast<std::streamsize>(count * sizeof(T)));
  if (input.gcount() != static_cast<std::streamsize>(count * sizeof(T)))
    throw std::runtime_error("wrong input length: " + path);
  return result;
}

template <typename T>
void write_values(const std::string &path, const std::vector<T> &values) {
  std::ofstream output(path, std::ios::binary);
  if (!output)
    throw std::runtime_error("cannot open output: " + path);
  output.write(reinterpret_cast<const char *>(values.data()),
               static_cast<std::streamsize>(values.size() * sizeof(T)));
  if (!output)
    throw std::runtime_error("failed writing output: " + path);
}

template <typename T>
double checksum(const std::vector<T> &values) {
  if (values.empty())
    return 0;
  const std::size_t stride = std::max<std::size_t>(1, values.size() / 32);
  double result = 0;
  for (std::size_t index = 0, used = 0;
       index < values.size() && used < 32; index += stride, ++used)
    result += static_cast<double>(values[index]);
  return result;
}

template <>
double checksum(const std::vector<Complex> &values) {
  if (values.empty())
    return 0;
  const std::size_t stride = std::max<std::size_t>(1, values.size() / 32);
  double result = 0;
  for (std::size_t index = 0, used = 0;
       index < values.size() && used < 32; index += stride, ++used)
    result += values[index].real() + values[index].imag();
  return result;
}

double relative_l2(const std::vector<double> &actual,
                   const std::vector<double> &expected) {
  long double error = 0, scale = 0;
  for (std::size_t i = 0; i < actual.size(); ++i) {
    const long double difference = static_cast<long double>(actual[i]) - expected[i];
    error += difference * difference;
    scale += static_cast<long double>(expected[i]) * expected[i];
  }
  return std::sqrt(static_cast<double>(error / std::max(scale, static_cast<long double>(std::numeric_limits<double>::min()))));
}

double relative_l2_complex(const std::vector<Complex> &actual,
                           const std::vector<Complex> &expected,
                           std::size_t actual_offset = 0) {
  long double error = 0, scale = 0;
  for (std::size_t i = 0; i < expected.size(); ++i) {
    const long double dr = static_cast<long double>(actual[actual_offset+i].real()) - expected[i].real();
    const long double di = static_cast<long double>(actual[actual_offset+i].imag()) - expected[i].imag();
    error += dr*dr + di*di;
    scale += static_cast<long double>(expected[i].real())*expected[i].real()
           + static_cast<long double>(expected[i].imag())*expected[i].imag();
  }
  return std::sqrt(static_cast<double>(error / std::max(scale, static_cast<long double>(std::numeric_limits<double>::min()))));
}

double real_dot(const std::vector<double> &left, const std::vector<double> &right) {
  long double result = 0;
  for (std::size_t i = 0; i < left.size(); ++i)
    result += static_cast<long double>(left[i]) * right[i];
  return static_cast<double>(result);
}

double harmonic_dot(const std::vector<Complex> &left,
                    const std::vector<Complex> &right,
                    const std::vector<std::size_t> &mstart,
                    std::size_t lmax,
                    std::size_t prefix) {
  long double result = 0;
  for (std::size_t m = 0; m < mstart.size(); ++m)
    for (std::size_t l = m; l <= lmax; ++l) {
      const auto index = mstart[m] + l;
      const double weight = m == 0 ? 1.0 : 2.0;
      result += weight * (static_cast<long double>(left[index].real()) * right[index].real()
                         + static_cast<long double>(left[index].imag()) * right[index].imag());
    }
  (void)prefix;
  return static_cast<double>(result);
}

double identity_residual(double left, double right) {
  return std::abs(left-right) / std::max({std::abs(left), std::abs(right), std::numeric_limits<double>::min()});
}

template <typename Func>
double time_once(Func &&function) {
  const auto start = Clock::now();
  function();
  return std::chrono::duration<double, std::milli>(Clock::now() - start).count();
}

void worker(int argc, char **argv) {
  if (argc != 14 || std::string(argv[1]) != "--worker")
    throw std::runtime_error(
        "usage: --worker OP THREADS NTHETA NPHI LMAX MMAX PHI0 MAP ALM OUT BATCH_TARGET_MS VALIDATE");
  const std::string operation(argv[2]);
  if (operation != "analysis" && operation != "synthesis")
    throw std::runtime_error("operation must be analysis or synthesis");
  const std::size_t threads = parse_size(argv[3], "threads");
  const std::size_t ntheta = parse_size(argv[4], "ntheta");
  const std::size_t nphi = parse_size(argv[5], "nphi");
  const std::size_t lmax = parse_size(argv[6], "lmax");
  const std::size_t mmax = parse_size(argv[7], "mmax");
  const double phi0 = parse_double(argv[8], "phi0");
  const std::string map_path(argv[9]);
  const std::string alm_path(argv[10]);
  const std::string output_path(argv[11]);
  const double target_ms = parse_double(argv[12], "batch target milliseconds");
  const bool validate = std::string(argv[13]) == "1";

  const std::size_t alm_count = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) / 2;
  const std::size_t map_count = ntheta * nphi;
  const std::size_t prefix = mmax;
  const std::size_t ducc_alm_count = prefix + alm_count;
  const auto fixture_load_start = Clock::now();
  const auto map_input = read_values<double>(map_path, map_count);
  const auto compact_alm = read_values<Complex>(alm_path, alm_count);
  const double fixture_load_ms =
      std::chrono::duration<double, std::milli>(Clock::now() - fixture_load_start).count();
  const auto setup_start = Clock::now();
  std::vector<Complex> ducc_alm(ducc_alm_count, Complex(0, 0));
  std::copy(compact_alm.begin(), compact_alm.end(), ducc_alm.begin() + prefix);
  std::vector<Complex> alm_output(ducc_alm_count, Complex(0, 0));
  std::vector<double> map_output(map_count, 0.0);

  std::vector<std::size_t> mstart(mmax + 1);
  std::size_t order_offset = 0;
  for (std::size_t m = 0; m <= mmax; ++m) {
    mstart[m] = prefix + order_offset - m;
    order_offset += lmax - m + 1;
  }
  const cmav<std::size_t, 1> mstart_view(mstart.data(), {mstart.size()});
  const std::vector<double> ringfactor_values(ntheta, 1.0);
  const cmav<double, 1> ringfactor(ringfactor_values.data(), {ntheta});
  const cmav<double, 3> map_in(map_input.data(), {1, ntheta, nphi});
  const cmav<Complex, 2> alm_in(ducc_alm.data(), {1, ducc_alm_count});
  vmav<Complex, 2> alm_out(alm_output.data(), {1, ducc_alm_count});
  vmav<double, 3> map_out(map_output.data(), {1, ntheta, nphi});
  const double setup_ms =
      std::chrono::duration<double, std::milli>(Clock::now() - setup_start).count();

  auto run_once = [&]() {
    if (operation == "analysis")
      ducc0::analysis_2d(alm_out, map_in, 0, lmax, mstart_view, 1, "GL", phi0,
                         ringfactor, threads);
    else
      ducc0::synthesis_2d(alm_in, map_out, 0, lmax, mstart_view, 1, "GL", phi0,
                          ringfactor, threads, ducc0::STANDARD);
  };

  const double first_ms = time_once(run_once);
  const auto first_map_output = map_output;
  const auto first_alm_output = alm_output;
  const double warmup_ms = time_once(run_once);
  const double output_checksum = operation == "analysis" ? checksum(alm_output)
                                                          : checksum(map_output);
  std::size_t repetitions = warmup_ms >= 100.0
      ? 1
      : std::clamp<std::size_t>(
            static_cast<std::size_t>(std::ceil(target_ms / std::max(warmup_ms, 0.000001))),
            1, 2000000);
  const std::size_t batch_count = warmup_ms >= 100.0 ? 5 : 10;
  std::vector<double> calibration_batch_total_ms;
  if (warmup_ms < 100.0) {
    for (std::size_t calibration = 0; calibration < 3; ++calibration) {
      const double elapsed = time_once([&]() {
        for (std::size_t repeat = 0; repeat < repetitions; ++repeat)
          run_once();
      });
      calibration_batch_total_ms.push_back(elapsed);
      if (elapsed >= 20.0 && elapsed <= 100.0)
        break;
      repetitions = std::clamp<std::size_t>(
          static_cast<std::size_t>(std::ceil(repetitions * target_ms / std::max(elapsed, 0.000001))),
          1, 2000000);
    }
  }
  std::vector<double> batch_ms;
  std::vector<double> batch_total_ms;
  batch_ms.reserve(batch_count);
  batch_total_ms.reserve(batch_count);
  double measured_checksum = 0;
  for (std::size_t batch = 0; batch < batch_count; ++batch) {
    const double elapsed = time_once([&]() {
      for (std::size_t repeat = 0; repeat < repetitions; ++repeat)
        run_once();
    });
    batch_total_ms.push_back(elapsed);
    batch_ms.push_back(elapsed / repetitions);
    measured_checksum += operation == "analysis" ? checksum(alm_output)
                                                  : checksum(map_output);
  }
  const bool reset_consistent = operation == "analysis"
      ? first_alm_output == alm_output
      : first_map_output == map_output;

  if (operation == "analysis")
    write_values(output_path, alm_output);
  else
    write_values(output_path, map_output);

  std::string validation_json = "null";
  if (validate) {
    const auto suffix = [&](const std::string &name) { return output_path + "." + name; };
    auto run_analysis_for = [&](const std::vector<double> &input) {
      const cmav<double, 3> map_view(input.data(), {1, ntheta, nphi});
      ducc0::analysis_2d(alm_out, map_view, 0, lmax, mstart_view, 1, "GL", phi0,
                         ringfactor, threads);
    };
    const auto run_synthesis = [&]() {
      ducc0::synthesis_2d(alm_in, map_out, 0, lmax, mstart_view, 1, "GL", phi0,
                          ringfactor, threads, ducc0::STANDARD);
    };

    run_synthesis();
    const auto roundtrip_map = map_output;
    run_analysis_for(roundtrip_map);
    const double roundtrip_error = relative_l2_complex(alm_output, compact_alm, prefix);
    write_values(suffix("roundtrip.alm"), alm_output);

    std::vector<std::pair<std::size_t, std::size_t>> modes{
        {0,0}, {lmax,0}, {lmax/2, std::min(mmax,lmax/2)}, {lmax,mmax}};
    std::sort(modes.begin(), modes.end());
    modes.erase(std::unique(modes.begin(), modes.end()), modes.end());
    for (std::size_t mode_index = 0; mode_index < modes.size(); ++mode_index) {
      const auto [degree, order] = modes[mode_index];
      std::fill(ducc_alm.begin(), ducc_alm.end(), Complex(0,0));
      const auto offset = mstart[order] + degree;
      ducc_alm[offset] = Complex(1,0);
      run_synthesis();
      const auto basis_map = map_output;
      write_values(suffix("basis-" + std::to_string(mode_index) + ".map"), basis_map);
      run_analysis_for(basis_map);
      write_values(suffix("basis-" + std::to_string(mode_index) + ".alm"), alm_output);
    }

    std::fill(ducc_alm.begin(), ducc_alm.end(), Complex(0,0));
    run_synthesis();
    const auto zero_map = map_output;
    run_analysis_for(zero_map);
    write_values(suffix("zero.map"), zero_map);
    write_values(suffix("zero.alm"), alm_output);
    const bool zero_finite = std::all_of(zero_map.begin(), zero_map.end(), [](double x) { return std::isfinite(x); })
        && std::all_of(alm_output.begin()+prefix, alm_output.end(), [](Complex x) { return std::isfinite(x.real()) && std::isfinite(x.imag()); });

    std::fill(ducc_alm.begin(), ducc_alm.end(), Complex(0,0));
    const auto sectoral = mstart[mmax] + lmax;
    ducc_alm[sectoral] = Complex(1.0e150, mmax == 0 ? 0.0 : -2.0e149);
    if (lmax >= 1) {
      ducc_alm[mstart[0]] = Complex(1.0e150,0);
      ducc_alm[mstart[0]+1] = Complex(-1.0e150,0);
    }
    run_synthesis();
    const auto extreme_map = map_output;
    run_analysis_for(extreme_map);
    write_values(suffix("extreme.map"), extreme_map);
    write_values(suffix("extreme.alm"), alm_output);
    const bool extreme_finite = std::all_of(extreme_map.begin(), extreme_map.end(), [](double x) { return std::isfinite(x); })
        && std::all_of(alm_output.begin()+prefix, alm_output.end(), [](Complex x) { return std::isfinite(x.real()) && std::isfinite(x.imag()); });

    std::fill(ducc_alm.begin(), ducc_alm.end(), Complex(0,0));
    ducc_alm[prefix] = Complex(std::numeric_limits<double>::denorm_min(),0);
    run_synthesis();
    const auto subnormal_map = map_output;
    std::vector<double> subnormal_input(map_count, std::numeric_limits<double>::denorm_min());
    run_analysis_for(subnormal_input);
    write_values(suffix("subnormal.map"), subnormal_map);
    write_values(suffix("subnormal.alm"), alm_output);
    const bool subnormal_finite = std::all_of(subnormal_map.begin(), subnormal_map.end(), [](double x) { return std::isfinite(x); })
        && std::all_of(alm_output.begin()+prefix, alm_output.end(), [](Complex x) { return std::isfinite(x.real()) && std::isfinite(x.imag()); });

    std::fill(ducc_alm.begin(), ducc_alm.end(), Complex(0,0));
    std::copy(compact_alm.begin(), compact_alm.end(), ducc_alm.begin() + prefix);
    const cmav<Complex, 2> alm_for_adjoint(ducc_alm.data(), {1, ducc_alm_count});
    ducc0::synthesis_2d(alm_in, map_out, 0, lmax, mstart_view, 1, "GL", phi0,
                        ringfactor, threads, ducc0::STANDARD);
    const auto synthesized = map_output;
    ducc0::adjoint_synthesis_2d(alm_out, map_in, 0, lmax, mstart_view, 1, "GL", phi0,
                               ringfactor, threads, ducc0::STANDARD);
    const double synth_left = real_dot(synthesized, map_input);
    const double synth_right = harmonic_dot(ducc_alm, alm_output, mstart, lmax, prefix);
    run_analysis_for(map_input);
    const auto analyzed = alm_output;
    vmav<double, 3> adjoint_map(map_output.data(), {1, ntheta, nphi});
    ducc0::adjoint_analysis_2d(alm_for_adjoint, adjoint_map, 0, lmax, mstart_view, 1,
                              "GL", phi0, ringfactor, threads);
    const double analysis_left = harmonic_dot(analyzed, ducc_alm, mstart, lmax, prefix);
    const double analysis_right = real_dot(map_input, map_output);
    std::ostringstream checks;
    checks << std::setprecision(17)
           << "{\"roundtrip_relative_l2\":" << roundtrip_error
           << ",\"selected_basis_modes\":" << modes.size()
           << ",\"zero_finite\":" << (zero_finite ? "true" : "false")
           << ",\"extreme_finite\":" << (extreme_finite ? "true" : "false")
           << ",\"subnormal_finite\":" << (subnormal_finite ? "true" : "false")
           << ",\"synthesis_adjoint_relative_residual\":" << identity_residual(synth_left, synth_right)
           << ",\"analysis_adjoint_relative_residual\":" << identity_residual(analysis_left, analysis_right) << "}";
    validation_json = checks.str();
  }

  std::cout << "{\"backend\":\"ducc\",\"operation\":\"" << operation
            << "\",\"threads\":" << threads << ",\"setup_ms\":" << setup_ms
            << ",\"fixture_load_ms\":" << fixture_load_ms
            << ",\"first_ms\":" << first_ms << ",\"warmup_ms\":" << warmup_ms
            << ",\"repetitions\":" << repetitions << ",\"batch_ms\":[";
  for (std::size_t index = 0; index < batch_ms.size(); ++index) {
    if (index != 0)
      std::cout << ',';
    std::cout << batch_ms[index];
  }
  std::cout << "],\"batch_total_ms\":[";
  for (std::size_t index = 0; index < batch_total_ms.size(); ++index) {
    if (index != 0)
      std::cout << ',';
    std::cout << batch_total_ms[index];
  }
  std::cout << "],\"calibration_batch_total_ms\":[";
  for (std::size_t index = 0; index < calibration_batch_total_ms.size(); ++index) {
    if (index != 0)
      std::cout << ',';
    std::cout << calibration_batch_total_ms[index];
  }
  std::cout << "],\"checksum\":" << output_checksum + measured_checksum
            << ",\"reset_consistent\":" << (reset_consistent ? "true" : "false")
            << ",\"thread_pool_size\":" << ducc0::thread_pool_size()
            << ",\"arch\":\"x86-64-v3 fixed build\",\"lane_width_f64\":4,\"validation\":"
            << validation_json << "}\n";
}

} // namespace

int main(int argc, char **argv) {
  try {
    worker(argc, argv);
    return 0;
  } catch (const std::exception &error) {
    std::cerr << "ducc_full_transform_parity: " << error.what() << '\n';
    return 1;
  }
}
