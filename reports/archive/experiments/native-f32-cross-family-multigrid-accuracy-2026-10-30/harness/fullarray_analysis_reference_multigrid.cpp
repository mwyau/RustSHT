#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

#include <quadmath.h>

using Q = __float128;

struct QC {
  Q re, im;
};

static QC operator+(QC a, QC b) { return {a.re + b.re, a.im + b.im}; }
static QC operator-(QC a, QC b) { return {a.re - b.re, a.im - b.im}; }
static QC operator*(QC a, QC b) {
  return {a.re * b.re - a.im * b.im, a.re * b.im + a.im * b.re};
}
static QC operator*(Q a, QC b) { return {a * b.re, a * b.im}; }
static std::string qstr(Q value) {
  char buffer[128];
  quadmath_snprintf(buffer, sizeof(buffer), "%+.36Qg", value);
  return buffer;
}

static Q qabs(Q value) { return fabsq(value); }
static Q qabs(QC value) { return hypotq(value.re, value.im); }

static uint64_t parse_hex64(const std::string &value) {
  return std::stoull(value, nullptr, 16);
}

static double double_from_bits(uint64_t bits) {
  double value;
  std::memcpy(&value, &bits, sizeof(value));
  return value;
}

template <typename T> static std::vector<T> read_binary(const std::string &path) {
  std::ifstream in(path, std::ios::binary);
  if (!in) throw std::runtime_error("cannot open " + path);
  in.seekg(0, std::ios::end);
  const auto bytes = static_cast<size_t>(in.tellg());
  if (bytes % sizeof(T) != 0) throw std::runtime_error("bad byte count: " + path);
  in.seekg(0);
  std::vector<T> values(bytes / sizeof(T));
  in.read(reinterpret_cast<char *>(values.data()), static_cast<std::streamsize>(bytes));
  if (!in) throw std::runtime_error("short read: " + path);
  return values;
}

template <typename T> static std::vector<QC> read_complex(const std::string &path) {
  const auto raw = read_binary<T>(path);
  if (raw.size() % 2 != 0) throw std::runtime_error("bad complex array: " + path);
  std::vector<QC> values(raw.size() / 2);
  for (size_t i = 0; i < values.size(); ++i)
    values[i] = {static_cast<Q>(raw[2 * i]), static_cast<Q>(raw[2 * i + 1])};
  return values;
}

struct Geometry {
  std::vector<Q> theta, weight;
};

static Geometry read_geometry(const std::string &path) {
  std::ifstream in(path);
  if (!in) throw std::runtime_error("cannot open " + path);
  std::string line;
  std::getline(in, line);
  Geometry g;
  while (std::getline(in, line)) {
    std::istringstream row(line);
    std::string ring, theta, weight;
    std::getline(row, ring, ',');
    std::getline(row, theta, ',');
    std::getline(row, weight, ',');
    g.theta.push_back(static_cast<Q>(double_from_bits(parse_hex64(theta))));
    g.weight.push_back(static_cast<Q>(double_from_bits(parse_hex64(weight))));
  }
  return g;
}

struct BasisFactors {
  std::array<std::array<Q, 722>, 721> a{}, b{};
  BasisFactors() {
    for (unsigned m = 0; m <= 720; ++m) {
      for (unsigned l = m + 2; l <= 721; ++l) {
        const Q lf = l, mf = m;
        a[m][l] = sqrtq((4 * lf * lf - 1) / (lf * lf - mf * mf));
        b[m][l] = sqrtq((2 * lf + 1) * ((lf - 1) * (lf - 1) - mf * mf) /
                        ((2 * lf - 3) * (lf * lf - mf * mf)));
      }
    }
  }
};

static const BasisFactors &basis_factors() {
  static const BasisFactors factors;
  return factors;
}

static void basis_into(Q theta, unsigned m, unsigned lmax, std::vector<Q> &y) {
  if (y.size() < lmax + 1) throw std::runtime_error("basis workspace is too short");
  const auto &factors = basis_factors();
  const Q pi = acosq(-1);
  const Q sine = sinq(theta), cosine = cosq(theta);
  y[0] = 1 / sqrtq(4 * pi);
  for (unsigned k = 1; k <= m; ++k)
    y[k] = -sqrtq(static_cast<Q>(2 * k + 1) / (2 * k)) * sine * y[k - 1];
  if (m == lmax) return;
  y[m + 1] = sqrtq(static_cast<Q>(2 * m + 3)) * cosine * y[m];
  for (unsigned l = m + 2; l <= lmax; ++l) {
    y[l] = factors.a[m][l] * cosine * y[l - 1] - factors.b[m][l] * y[l - 2];
  }
}

static std::vector<Q> basis(Q theta, unsigned m, unsigned lmax) {
  std::vector<Q> y(lmax + 1);
  basis_into(theta, m, lmax, y);
  return y;
}

static const std::vector<unsigned> &selected_orders() {
  static const std::vector<unsigned> orders = [] {
    std::vector<unsigned> values(721);
    for (unsigned m = 0; m <= 720; ++m) values[m] = m;
    return values;
  }();
  return orders;
}

static std::vector<QC> fourier_average(const std::vector<float> &map,
                                       unsigned ntheta, unsigned nphi,
                                       unsigned mmax) {
  const Q pi = acosq(-1), two_pi = 2 * pi;
  const auto &orders = selected_orders();
  std::vector<std::vector<QC>> phase(nphi, std::vector<QC>(orders.size()));
  for (unsigned j = 0; j < nphi; ++j) {
    for (size_t oi = 0; oi < orders.size(); ++oi) {
      const unsigned m = orders[oi];
      const Q angle = two_pi * static_cast<Q>(j) * m / nphi;
      phase[j][oi] = {cosq(angle), -sinq(angle)};
    }
  }
  std::vector<QC> out(static_cast<size_t>(ntheta) * (mmax + 1), {0, 0});
  for (unsigned ring = 0; ring < ntheta; ++ring) {
    for (unsigned j = 0; j < nphi; ++j) {
      const Q value = static_cast<Q>(map[static_cast<size_t>(ring) * nphi + j]);
      for (size_t oi = 0; oi < orders.size(); ++oi) {
        const unsigned m = orders[oi];
        const size_t index = static_cast<size_t>(ring) * (mmax + 1) + m;
        out[index] = out[index] + value * phase[j][oi];
      }
    }
    for (const unsigned m : orders) {
      const size_t index = static_cast<size_t>(ring) * (mmax + 1) + m;
      out[index] = (1 / static_cast<Q>(nphi)) * out[index];
    }
  }
  return out;
}

template <typename T>
static std::vector<QC> fourier_from_rust_spectrum(const std::string &path,
                                                  unsigned ntheta, unsigned nphi,
                                                  unsigned spectrum_len, unsigned mmax,
                                                  bool spectrum_already_normalized) {
  const auto spectrum = read_complex<T>(path);
  if (spectrum.size() != static_cast<size_t>(ntheta) * spectrum_len)
    throw std::runtime_error("unexpected Rust spectrum size: " + path);
  std::vector<QC> out(static_cast<size_t>(ntheta) * (mmax + 1));
  for (unsigned ring = 0; ring < ntheta; ++ring)
    for (unsigned m = 0; m <= mmax; ++m)
      out[static_cast<size_t>(ring) * (mmax + 1) + m] =
          (spectrum_already_normalized ? Q(1) : (1 / static_cast<Q>(nphi))) *
          spectrum[static_cast<size_t>(ring) * spectrum_len + m];
  return out;
}

static void write_fourier_comparison(const std::string &path,
                                    const std::vector<QC> &exact,
                                    const std::vector<QC> &from_rust,
                                    unsigned ntheta, unsigned mmax) {
  std::ofstream out(path);
  out << "ring,m,exact_re,exact_im,rust_fft_re,rust_fft_im,abs_error,relative_error\n";
  Q norm_exact = 0, error_norm = 0, max_abs = 0, max_rel = 0;
  for (unsigned ring = 0; ring < ntheta; ++ring) {
    for (unsigned m = 0; m <= mmax; ++m) {
      const size_t i = static_cast<size_t>(ring) * (mmax + 1) + m;
      const QC a = exact[i], b = from_rust[i];
      const Q error = qabs(a - b);
      const Q relative = qabs(a) == 0 ? 0 : error / qabs(a);
      out << ring << ',' << m << ',' << qstr(a.re) << ',' << qstr(a.im) << ','
          << qstr(b.re) << ',' << qstr(b.im) << ',' << qstr(error) << ','
          << qstr(relative) << '\n';
      norm_exact += a.re * a.re + a.im * a.im;
      error_norm += error * error;
      max_abs = std::max(max_abs, error);
      max_rel = std::max(max_rel, relative);
    }
  }
  std::cerr << "forward_fft rel_l2=" << qstr(sqrtq(error_norm / norm_exact))
            << " max_abs=" << qstr(max_abs) << " max_rel_nonzero=" << qstr(max_rel) << "\n";
}

static size_t packed_index(unsigned l, unsigned m, unsigned lmax, unsigned mmax) {
  (void)mmax;
  size_t offset = 0;
  for (unsigned k = 0; k < m; ++k) offset += lmax - k + 1;
  return offset + l - m;
}

struct Projection {
  std::vector<QC> coefficient;
  std::vector<Q> envelope;
};

static std::vector<Projection> scalar_project_many(
    const Geometry &g, const std::vector<const std::vector<QC> *> &fourier,
    unsigned lmax, unsigned mmax, bool divide_by_sine, bool zero_spin_poles) {
  std::vector<Projection> result(fourier.size());
  size_t count = 0;
  for (unsigned m = 0; m <= mmax; ++m) count += lmax - m + 1;
  for (auto &projection : result) {
    projection.coefficient.assign(count, {0, 0});
    projection.envelope.assign(count, 0);
  }
  const auto &orders = selected_orders();
  std::vector<Q> y(lmax + 1);
  std::vector<QC> values(fourier.size());
  std::vector<Q> absolute_values(fourier.size());
  for (size_t ring = 0; ring < g.theta.size(); ++ring) {
    const Q theta = g.theta[ring];
    const Q sine = sinq(theta);
    const Q pi = acosq(-1);
    const bool north = qabs(theta) < static_cast<Q>(1.0e-30);
    const bool south = qabs(theta - pi) < static_cast<Q>(1.0e-15);
    if (divide_by_sine && (north || south)) {
      if (zero_spin_poles) continue;
      const Q sign = north ? -1 : 1;
      const Q weight = g.weight[ring];
      const unsigned m = 1;
      for (size_t channel = 0; channel < fourier.size(); ++channel) {
        values[channel] = (*fourier[channel])[ring * (mmax + 1) + m];
        absolute_values[channel] = qabs(values[channel]);
      }
      for (unsigned l = 1; l <= lmax; ++l) {
        const Q lf = l;
        const Q parity = !north && (l % 2 == 1) ? -1 : 1;
        const Q y_over_sine = sign * parity *
            sqrtq((2 * lf + 1) * lf * (lf + 1) / (16 * acosq(-1)));
        const size_t index = packed_index(l, m, lmax, mmax);
        const Q scale = weight * y_over_sine;
        const Q abs_scale = qabs(scale);
        for (size_t channel = 0; channel < fourier.size(); ++channel) {
          result[channel].coefficient[index] =
              result[channel].coefficient[index] + scale * values[channel];
          result[channel].envelope[index] += abs_scale * absolute_values[channel];
        }
      }
      continue;
    }
    for (const unsigned m : orders) {
      basis_into(theta, m, lmax, y);
      Q weight = g.weight[ring];
      if (divide_by_sine) weight /= sine;
      for (size_t channel = 0; channel < fourier.size(); ++channel) {
        values[channel] = (*fourier[channel])[ring * (mmax + 1) + m];
        absolute_values[channel] = qabs(values[channel]);
      }
      for (unsigned l = m; l <= lmax; ++l) {
        const size_t index = packed_index(l, m, lmax, mmax);
        const Q scale = weight * y[l];
        const Q abs_scale = qabs(scale);
        for (size_t channel = 0; channel < fourier.size(); ++channel) {
          result[channel].coefficient[index] =
              result[channel].coefficient[index] + scale * values[channel];
          result[channel].envelope[index] += abs_scale * absolute_values[channel];
        }
      }
    }
  }
  return result;
}

static Projection spin_transpose(const Projection &first, const Projection &second,
                                 unsigned lmax, unsigned mmax) {
  Projection result;
  size_t count = 0;
  for (unsigned m = 0; m <= mmax; ++m) count += lmax - m + 1;
  result.coefficient.assign(2 * count, {0, 0});
  result.envelope.assign(2 * count, 0);
  for (const unsigned m : selected_orders()) {
    QC last_first{0, 0}, last_second{0, 0};
    for (unsigned l = m; l <= lmax; ++l) {
      const size_t i = packed_index(l, m, lmax + 1, mmax);
      const size_t o = packed_index(l, m, lmax, mmax);
      if (l == 0) {
        result.coefficient[o] = {0, 0};
        result.coefficient[count + o] = {0, 0};
        result.envelope[o] = 0;
        result.envelope[count + o] = 0;
        last_first = first.coefficient[i];
        last_second = second.coefficient[i];
        continue;
      }
      QC c_first{static_cast<Q>(m) * second.coefficient[i].im,
                 -static_cast<Q>(m) * second.coefficient[i].re};
      QC c_second{static_cast<Q>(m) * first.coefficient[i].im,
                  -static_cast<Q>(m) * first.coefficient[i].re};
      Q env_first = static_cast<Q>(m) * second.envelope[i];
      Q env_second = static_cast<Q>(m) * first.envelope[i];
      if (l > m) {
        const Q lf = l, mf = m;
        const Q prev = -(lf + 1) * sqrtq((lf * lf - mf * mf) /
                                         ((2 * lf + 1) * (2 * lf - 1)));
        c_first = c_first + prev * last_first;
        c_second = c_second - prev * last_second;
        env_first += qabs(prev) * first.envelope[packed_index(l - 1, m, lmax + 1, mmax)];
        env_second += qabs(prev) * second.envelope[packed_index(l - 1, m, lmax + 1, mmax)];
      }
      const Q lf = l, mf = m;
      const Q next = lf * sqrtq(((lf + 1 + mf) * (lf + 1 - mf)) /
                                ((2 * lf + 3) * (2 * lf + 1)));
      const QC c1_next = first.coefficient[packed_index(l + 1, m, lmax + 1, mmax)];
      const QC c2_next = second.coefficient[packed_index(l + 1, m, lmax + 1, mmax)];
      c_first = c_first + next * c1_next;
      c_second = c_second - next * c2_next;
      env_first += qabs(next) * first.envelope[packed_index(l + 1, m, lmax + 1, mmax)];
      env_second += qabs(next) * second.envelope[packed_index(l + 1, m, lmax + 1, mmax)];
      const Q norm = 1 / sqrtq(lf * (lf + 1));
      result.coefficient[o] = norm * c_first;
      result.coefficient[count + o] = -norm * c_second;
      result.envelope[o] = norm * env_first;
      result.envelope[count + o] = norm * env_second;
      last_first = first.coefficient[i];
      last_second = second.coefficient[i];
    }
  }
  return result;
}

static Q complex_error(QC actual, QC expected) { return hypotq(actual.re - expected.re, actual.im - expected.im); }

static void compare_projection(const std::string &path, const std::vector<QC> &expected,
                               const std::vector<Q> &envelope, const std::vector<QC> &actual,
                               unsigned lmax, unsigned mmax, size_t offset,
                               const std::string &operation, const std::string &component,
                               const std::string &precision) {
  std::ofstream out(path);
  out << "m,component,precision,count,relative_l2,max_abs,max_abs_l,max_envelope_scaled,"
         "max_scaled_l\n";
  Q all_ref = 0, all_error = 0, all_max = 0, all_scaled = 0;
  unsigned all_max_m = 0, all_max_l = 0, all_scaled_m = 0, all_scaled_l = 0;
  for (const unsigned m : selected_orders()) {
    Q sum_ref = 0, sum_error = 0, max_abs = 0, max_scaled = 0;
    unsigned max_abs_l = m, max_scaled_l = m;
    for (unsigned l = m; l <= lmax; ++l) {
      const size_t index = packed_index(l, m, lmax, mmax);
      const QC value = actual[offset + index];
      const QC ref = expected[index];
      const Q error = complex_error(value, ref);
      const Q env = std::max(envelope[index], qabs(ref));
      const Q scaled = env == 0 ? 0 : error / env;
      sum_ref += ref.re * ref.re + ref.im * ref.im;
      sum_error += error * error;
      if (error > max_abs) {
        max_abs = error;
        max_abs_l = l;
      }
      if (scaled > max_scaled) {
        max_scaled = scaled;
        max_scaled_l = l;
      }
    }
    all_ref += sum_ref;
    all_error += sum_error;
    if (max_abs > all_max) {
      all_max = max_abs;
      all_max_m = m;
      all_max_l = max_abs_l;
    }
    if (max_scaled > all_scaled) {
      all_scaled = max_scaled;
      all_scaled_m = m;
      all_scaled_l = max_scaled_l;
    }
    out << m << ',' << component << ',' << precision << ',' << lmax - m + 1 << ','
        << qstr(sum_ref == 0 ? Q(0) : sqrtq(sum_error / sum_ref)) << ','
        << qstr(max_abs) << ',' << max_abs_l << ',' << qstr(max_scaled) << ','
        << max_scaled_l << '\n';
  }
  const Q relative_l2 = all_ref == 0 ? 0 : sqrtq(all_error / all_ref);
  std::cerr << operation << ' ' << component << ' ' << precision << " full_array_m=0.."
            << mmax << " l=m.." << lmax << " relative_l2=" << qstr(relative_l2)
            << " max_abs=" << qstr(all_max) << " at m=" << all_max_m << " l=" << all_max_l
            << " max_envelope_scaled=" << qstr(all_scaled) << " at m=" << all_scaled_m
            << " l=" << all_scaled_l << "\n";
}

static std::vector<Q> read_f32_as_q(const std::string &path) {
  const auto data = read_binary<float>(path);
  std::vector<Q> out(data.size());
  std::transform(data.begin(), data.end(), out.begin(), [](float x) { return static_cast<Q>(x); });
  return out;
}

static std::vector<QC> channel_coefficients(const std::vector<QC> &e,
                                            const std::vector<QC> &b,
                                            unsigned m, unsigned lmax,
                                            unsigned mmax, bool second) {
  const size_t row_len = lmax + 2;
  std::vector<QC> first(row_len, {0, 0}), other(row_len, {0, 0});
  QC last_first{0, 0}, last_second{0, 0};
  for (unsigned l = m; l <= lmax + 1; ++l) {
    QC c_first{0, 0}, c_second{0, 0};
    if (l > 0 && l <= lmax) {
      const Q fac = static_cast<Q>(m) / sqrtq(static_cast<Q>(l) * (l + 1));
      const QC bvalue = b[packed_index(l, m, lmax, mmax)];
      const QC evalue = e[packed_index(l, m, lmax, mmax)];
      c_first = c_first + QC{-fac * bvalue.im, fac * bvalue.re};
      c_second = c_second - QC{-fac * evalue.im, fac * evalue.re};
    }
    if (l > m && l > 1) {
      const Q lf = l, mf = m;
      const Q prev = sqrtq(((lf + mf) * (lf - mf)) /
                           ((2 * lf + 1) * (2 * lf - 1))) * (lf - 1) /
                     sqrtq(lf * (lf - 1));
      c_first = c_first - prev * last_first;
      c_second = c_second - prev * last_second;
    }
    if (l < lmax) {
      const Q lf = l, mf = m;
      const Q next = sqrtq(((lf + 1 + mf) * (lf + 1 - mf)) /
                           ((2 * lf + 3) * (2 * lf + 1))) * (-lf - 2) /
                     sqrtq((lf + 1) * (lf + 2));
      c_first = c_first - next * e[packed_index(l + 1, m, lmax, mmax)];
      c_second = c_second - next * b[packed_index(l + 1, m, lmax, mmax)];
    }
    first[l] = {-c_first.re, -c_first.im};
    other[l] = {-c_second.re, -c_second.im};
    if (l <= lmax) {
      last_first = e[packed_index(l, m, lmax, mmax)];
      last_second = b[packed_index(l, m, lmax, mmax)];
    } else {
      last_first = {0, 0};
      last_second = {0, 0};
    }
  }
  std::vector<QC> selected(row_len);
  for (unsigned l = 0; l <= lmax + 1; ++l) selected[l] = second ? other[l] : first[l];
  return selected;
}

struct SpinChannels {
  std::vector<std::vector<QC>> first, second;
};

static SpinChannels prepare_spin_channels(const std::vector<QC> &e,
                                          const std::vector<QC> &b,
                                          unsigned lmax, unsigned mmax) {
  SpinChannels result;
  result.first.resize(mmax + 1);
  result.second.resize(mmax + 1);
  for (unsigned m = 0; m <= mmax; ++m) {
    result.first[m] = channel_coefficients(e, b, m, lmax, mmax, false);
    result.second[m] = channel_coefficients(e, b, m, lmax, mmax, true);
  }
  return result;
}

static void compare_spin_synthesis(const std::string &directory, const Geometry &g,
                                   unsigned lmax, unsigned mmax, unsigned nphi,
                                   const SpinChannels &channels,
                                   const std::vector<QC> &spectra32q,
                                   const std::vector<QC> &spectra32u,
                                   const std::vector<Q> &actual32q,
                                   const std::vector<Q> &actual32u,
                                   const std::vector<Q> &actual64q,
                                   const std::vector<Q> &actual64u) {
  const size_t ntheta = g.theta.size();
  const unsigned spectrum_len = nphi / 2 + 1;
  const Q pi = acosq(-1), two_pi = 2 * pi;
  std::vector<Q> refq(ntheta * nphi), refu(ntheta * nphi), envelope(ntheta * nphi);
  std::vector<Q> fftq(ntheta * nphi), fftu(ntheta * nphi);
  std::ofstream spectrum_out(directory + "/spin-synthesis-spectrum-comparison.csv");
  spectrum_out << "ring,m,component,reference_re,reference_im,f32_re,f32_im,abs_error\n";
  Q spectrum_norm_q = 0, spectrum_norm_u = 0;
  Q spectrum_error_q = 0, spectrum_error_u = 0;
  Q spectrum_max_q = 0, spectrum_max_u = 0;
  for (size_t ring = 0; ring < ntheta; ++ring) {
    const Q sine = sinq(g.theta[ring]);
    std::vector<QC> fq(mmax + 1, {0, 0}), fu(mmax + 1, {0, 0});
    Q env = 0;
    for (unsigned m = 0; m <= mmax; ++m) {
      const auto y = basis(g.theta[ring], m, lmax + 1);
      for (unsigned l = m; l <= lmax + 1; ++l) {
        fq[m] = fq[m] + y[l] * channels.first[m][l];
        fu[m] = fu[m] + y[l] * channels.second[m][l];
      }
      fq[m] = (1 / sine) * fq[m];
      fu[m] = (1 / sine) * fu[m];
      const QC q_value = spectra32q[ring * spectrum_len + m];
      const QC u_value = spectra32u[ring * spectrum_len + m];
      const Q q_error = qabs(q_value - fq[m]);
      const Q u_error = qabs(u_value - fu[m]);
      spectrum_out << ring << ',' << m << ",Q," << qstr(fq[m].re) << ',' << qstr(fq[m].im)
                   << ',' << qstr(q_value.re) << ',' << qstr(q_value.im) << ','
                   << qstr(q_error) << '\n';
      spectrum_out << ring << ',' << m << ",U," << qstr(fu[m].re) << ',' << qstr(fu[m].im)
                   << ',' << qstr(u_value.re) << ',' << qstr(u_value.im) << ','
                   << qstr(u_error) << '\n';
      spectrum_norm_q += fq[m].re * fq[m].re + fq[m].im * fq[m].im;
      spectrum_norm_u += fu[m].re * fu[m].re + fu[m].im * fu[m].im;
      spectrum_error_q += q_error * q_error;
      spectrum_error_u += u_error * u_error;
      spectrum_max_q = std::max(spectrum_max_q, q_error);
      spectrum_max_u = std::max(spectrum_max_u, u_error);
      env += (m == 0 ? 1 : 2) * (qabs(fq[m]) + qabs(fu[m]));
    }
    for (unsigned j = 0; j < nphi; ++j) {
      const Q phi = two_pi * static_cast<Q>(j) / nphi;
      Q q = fq[0].re, u = fu[0].re;
      Q q_from_fft = spectra32q[ring * spectrum_len].re;
      Q u_from_fft = spectra32u[ring * spectrum_len].re;
      for (unsigned m = 1; m <= mmax; ++m) {
        const Q angle = m * phi;
        q += 2 * (fq[m].re * cosq(angle) - fq[m].im * sinq(angle));
        u += 2 * (fu[m].re * cosq(angle) - fu[m].im * sinq(angle));
        const QC q_spec = spectra32q[ring * spectrum_len + m];
        const QC u_spec = spectra32u[ring * spectrum_len + m];
        q_from_fft += 2 * (q_spec.re * cosq(angle) - q_spec.im * sinq(angle));
        u_from_fft += 2 * (u_spec.re * cosq(angle) - u_spec.im * sinq(angle));
      }
      const size_t i = ring * nphi + j;
      refq[i] = q;
      refu[i] = u;
      fftq[i] = q_from_fft;
      fftu[i] = u_from_fft;
      envelope[i] = env;
    }
  }

  std::ofstream out(directory + "/spin-synthesis-comparison.csv");
  out << "ring,longitude,component,reference,f32_spectrum_inverse,f32_map,f64_map,"
         "f32_abs,f64_abs,f32_fft_abs,f32_scaled,f64_scaled\n";
  Q norm_ref_q = 0, norm_ref_u = 0, err32_q = 0, err64_q = 0, err32_u = 0, err64_u = 0;
  Q fft_err_q = 0, fft_err_u = 0;
  Q max32_q = 0, max64_q = 0, max32_u = 0, max64_u = 0;
  Q maxscaled32_q = 0, maxscaled64_q = 0, maxscaled32_u = 0, maxscaled64_u = 0;
  for (size_t i = 0; i < refq.size(); ++i) {
    for (unsigned c = 0; c < 2; ++c) {
      const Q ref = c == 0 ? refq[i] : refu[i];
      const Q inv = c == 0 ? fftq[i] : fftu[i];
      const Q v32 = c == 0 ? actual32q[i] : actual32u[i];
      const Q v64 = c == 0 ? actual64q[i] : actual64u[i];
      const Q e32 = qabs(v32 - ref), e64 = qabs(v64 - ref);
      const Q fft_error = qabs(v32 - inv);
      const Q den = std::max(envelope[i], qabs(ref));
      const Q s32 = den == 0 ? 0 : e32 / den;
      const Q s64 = den == 0 ? 0 : e64 / den;
      out << i / nphi << ',' << i % nphi << ',' << (c == 0 ? "Q" : "U") << ','
          << qstr(ref) << ',' << qstr(inv) << ',' << qstr(v32) << ',' << qstr(v64) << ','
          << qstr(e32) << ',' << qstr(e64) << ',' << qstr(fft_error) << ',' << qstr(s32)
          << ',' << qstr(s64) << '\n';
      if (c == 0) {
        norm_ref_q += ref * ref;
        err32_q += e32 * e32;
        err64_q += e64 * e64;
        fft_err_q += fft_error * fft_error;
        max32_q = std::max(max32_q, e32);
        max64_q = std::max(max64_q, e64);
        maxscaled32_q = std::max(maxscaled32_q, s32);
        maxscaled64_q = std::max(maxscaled64_q, s64);
      } else {
        norm_ref_u += ref * ref;
        err32_u += e32 * e32;
        err64_u += e64 * e64;
        fft_err_u += fft_error * fft_error;
        max32_u = std::max(max32_u, e32);
        max64_u = std::max(max64_u, e64);
        maxscaled32_u = std::max(maxscaled32_u, s32);
        maxscaled64_u = std::max(maxscaled64_u, s64);
      }
    }
  }
  const Q count = static_cast<Q>(refq.size());
  std::cerr << "spin-synthesis Q max_abs_f32=" << qstr(max32_q)
            << " max_abs_f64=" << qstr(max64_q)
            << " rel_l2_f32=" << qstr(sqrtq(err32_q / norm_ref_q))
            << " rel_l2_f64=" << qstr(sqrtq(err64_q / norm_ref_q))
            << " max_envelope_scaled_f32=" << qstr(maxscaled32_q)
            << " max_envelope_scaled_f64=" << qstr(maxscaled64_q) << "\n";
  std::cerr << "spin-synthesis Q spectrum_rel_l2="
            << qstr(sqrtq(spectrum_error_q / spectrum_norm_q))
            << " spectrum_max_abs=" << qstr(spectrum_max_q)
            << " inverse_fft_residual_rel_l2=" << qstr(sqrtq(fft_err_q / norm_ref_q)) << "\n";
  std::cerr << "spin-synthesis U max_abs_f32=" << qstr(max32_u)
            << " max_abs_f64=" << qstr(max64_u)
            << " rel_l2_f32=" << qstr(sqrtq(err32_u / norm_ref_u))
            << " rel_l2_f64=" << qstr(sqrtq(err64_u / norm_ref_u))
            << " max_envelope_scaled_f32=" << qstr(maxscaled32_u)
            << " max_envelope_scaled_f64=" << qstr(maxscaled64_u) << "\n";
  std::cerr << "spin-synthesis U spectrum_rel_l2="
            << qstr(sqrtq(spectrum_error_u / spectrum_norm_u))
            << " spectrum_max_abs=" << qstr(spectrum_max_u)
            << " inverse_fft_residual_rel_l2=" << qstr(sqrtq(fft_err_u / norm_ref_u)) << "\n";
  (void)count;
}

int main(int argc, char **argv) {
  if (argc < 2 || argc > 3)
    throw std::runtime_error("usage: phasec_fullorder_accuracy_reference DIR [spin-only|pre-normalized|prepared]");
  const std::string dir = argv[1];
  const std::string mode = argc == 3 ? argv[2] : "";
  const bool spin_only = mode == "spin-only";
  const bool prepared_mode = mode == "prepared";
  const bool spectrum_already_normalized = prepared_mode || mode == "pre-normalized";
  const unsigned lmax = 720, mmax = 720, nphi = 1458;
  const Geometry g = read_geometry(
      dir + (prepared_mode ? "/prepared-geometry.csv" : "/geometry.csv"));
  const unsigned ntheta = static_cast<unsigned>(g.theta.size());
  if (ntheta < lmax + 1) throw std::runtime_error("undersampled latitude geometry");

  const unsigned spectrum_len = nphi / 2 + 1;
  if (!spin_only) {
    const auto scalar_f32_spectrum = fourier_from_rust_spectrum<float>(
        dir + (prepared_mode ? "/scalar-analysis-prepared-spectrum.f32"
                          : "/scalar-analysis-spectrum.f32"),
        ntheta, nphi, spectrum_len, mmax, spectrum_already_normalized);
    const auto scalar_f64_spectrum = fourier_from_rust_spectrum<double>(
        dir + (prepared_mode ? "/scalar-analysis-prepared-spectrum.f64"
                          : "/scalar-analysis-spectrum.f64"),
        ntheta, nphi, spectrum_len, mmax, spectrum_already_normalized);
    const auto scalar_references = scalar_project_many(
        g, {&scalar_f32_spectrum, &scalar_f64_spectrum}, lmax, mmax, false, false);
    std::cerr << "scalar binary128 projection complete\n";
    const auto scalar32 = read_complex<float>(dir + "/scalar-analysis.f32");
    const auto scalar64 = read_complex<double>(dir + "/scalar-analysis.f64");
    compare_projection(dir + "/scalar-analysis-f32-full-array.csv", scalar_references[0].coefficient,
                       scalar_references[0].envelope, scalar32, lmax, mmax, 0,
                       "scalar-analysis", "alm", "f32");
    compare_projection(dir + "/scalar-analysis-f64-full-array.csv", scalar_references[1].coefficient,
                       scalar_references[1].envelope, scalar64, lmax, mmax, 0,
                       "scalar-analysis", "alm", "f64");
    compare_projection(dir + "/scalar-analysis-f32-vs-f64-spectrum-reference.csv",
                       scalar_references[1].coefficient, scalar_references[1].envelope,
                       scalar32, lmax, mmax, 0, "scalar-analysis", "alm",
                       "f32 actual vs f64-spectrum binary128 reference");
    compare_projection(dir + "/scalar-analysis-fft-input-sensitivity.csv",
                       scalar_references[1].coefficient, scalar_references[1].envelope,
                       scalar_references[0].coefficient, lmax, mmax, 0,
                       "scalar-analysis", "alm", "f32-spectrum reference vs f64-spectrum reference");
  }

  const auto q_f32_spectrum = fourier_from_rust_spectrum<float>(
      dir + (prepared_mode ? "/spin-analysis-q-prepared-spectrum.f32"
                            : "/spin-analysis-q-spectrum.f32"),
      ntheta, nphi, spectrum_len, mmax, spectrum_already_normalized);
  const auto u_f32_spectrum = fourier_from_rust_spectrum<float>(
      dir + (prepared_mode ? "/spin-analysis-u-prepared-spectrum.f32"
                            : "/spin-analysis-u-spectrum.f32"),
      ntheta, nphi, spectrum_len, mmax, spectrum_already_normalized);
  const auto q_f64_spectrum = fourier_from_rust_spectrum<double>(
      dir + (prepared_mode ? "/spin-analysis-q-prepared-spectrum.f64"
                            : "/spin-analysis-q-spectrum.f64"),
      ntheta, nphi, spectrum_len, mmax, spectrum_already_normalized);
  const auto u_f64_spectrum = fourier_from_rust_spectrum<double>(
      dir + (prepared_mode ? "/spin-analysis-u-prepared-spectrum.f64"
                            : "/spin-analysis-u-spectrum.f64"),
      ntheta, nphi, spectrum_len, mmax, spectrum_already_normalized);
  // The spin-1 / sin(theta) basis has a finite analytic m=1 limit at the
  // prepared-grid endpoints. Preserve it here; dropping endpoint rows made
  // the independent f64 control wrong by about 0.5% in that order.
  const auto spin_scalar_references =
      scalar_project_many(g, {&q_f32_spectrum, &u_f32_spectrum,
                              &q_f64_spectrum, &u_f64_spectrum},
                          lmax + 1, mmax, true, false);
  std::cerr << "spin binary128 scalar projections complete\n";
  const auto spin_ref32 = spin_transpose(spin_scalar_references[0], spin_scalar_references[1],
                                        lmax, mmax);
  const auto spin_ref64 = spin_transpose(spin_scalar_references[2], spin_scalar_references[3],
                                        lmax, mmax);
  const auto spin32e = read_complex<float>(dir + "/spin-analysis-e.f32");
  const auto spin32b = read_complex<float>(dir + "/spin-analysis-b.f32");
  const auto spin64e = read_complex<double>(dir + "/spin-analysis-e.f64");
  const auto spin64b = read_complex<double>(dir + "/spin-analysis-b.f64");
  const size_t nalm = spin32e.size();
  if (spin32b.size() != nalm || spin64e.size() != nalm || spin64b.size() != nalm)
    throw std::runtime_error("E/B array sizes differ");
  if (spin_ref32.coefficient.size() != 2 * nalm || spin_ref32.envelope.size() != 2 * nalm ||
      spin_ref64.coefficient.size() != 2 * nalm || spin_ref64.envelope.size() != 2 * nalm)
    throw std::runtime_error("E/B reference array sizes differ");
  std::vector<QC> spin32 = spin32e;
  spin32.insert(spin32.end(), spin32b.begin(), spin32b.end());
  std::vector<QC> spin_b_reference(spin_ref32.coefficient.begin() + nalm,
                                   spin_ref32.coefficient.end());
  std::vector<Q> spin_b_envelope(spin_ref32.envelope.begin() + nalm,
                                 spin_ref32.envelope.end());
  const std::vector<QC> spin64 = spin64e;
  std::vector<QC> spin64both = spin64;
  spin64both.insert(spin64both.end(), spin64b.begin(), spin64b.end());
  std::vector<QC> spin64e_reference(spin_ref64.coefficient.begin(),
                                    spin_ref64.coefficient.begin() + nalm);
  std::vector<Q> spin64e_envelope(spin_ref64.envelope.begin(),
                                  spin_ref64.envelope.begin() + nalm);
  std::vector<QC> spin64b_reference(spin_ref64.coefficient.begin() + nalm,
                                    spin_ref64.coefficient.end());
  std::vector<Q> spin64b_envelope(spin_ref64.envelope.begin() + nalm,
                                 spin_ref64.envelope.end());
  compare_projection(dir + "/spin-analysis-e-f32-full-array.csv", spin_ref32.coefficient,
                     spin_ref32.envelope, spin32, lmax, mmax, 0,
                     "spin-analysis", "E", "f32");
  compare_projection(dir + "/spin-analysis-b-f32-full-array.csv", spin_b_reference,
                     spin_b_envelope, spin32b, lmax, mmax, 0,
                     "spin-analysis", "B", "f32");
  compare_projection(dir + "/spin-analysis-e-f64-full-array.csv", spin64e_reference,
                     spin64e_envelope, spin64both, lmax, mmax, 0,
                     "spin-analysis", "E", "f64");
  compare_projection(dir + "/spin-analysis-b-f64-full-array.csv", spin64b_reference,
                     spin64b_envelope, spin64b, lmax, mmax, 0,
                     "spin-analysis", "B", "f64");
  compare_projection(dir + "/spin-analysis-e-f32-vs-f64-spectrum-reference.csv",
                     spin64e_reference, spin64e_envelope, spin32e, lmax, mmax, 0,
                     "spin-analysis", "E", "f32 actual vs f64-spectrum binary128 reference");
  compare_projection(dir + "/spin-analysis-b-f32-vs-f64-spectrum-reference.csv",
                     spin64b_reference, spin64b_envelope, spin32b, lmax, mmax, 0,
                     "spin-analysis", "B", "f32 actual vs f64-spectrum binary128 reference");
  compare_projection(dir + "/spin-analysis-e-fft-input-sensitivity.csv",
                     std::vector<QC>(spin_ref64.coefficient.begin(),
                                     spin_ref64.coefficient.begin() + nalm),
                     spin64e_envelope,
                     std::vector<QC>(spin_ref32.coefficient.begin(),
                                     spin_ref32.coefficient.begin() + nalm),
                     lmax, mmax, 0, "spin-analysis", "E",
                     "f32-spectrum reference vs f64-spectrum reference");
  compare_projection(dir + "/spin-analysis-b-fft-input-sensitivity.csv",
                     spin64b_reference, spin64b_envelope, spin_b_reference,
                     lmax, mmax, 0, "spin-analysis", "B",
                     "f32-spectrum reference vs f64-spectrum reference");
  return 0;
}
