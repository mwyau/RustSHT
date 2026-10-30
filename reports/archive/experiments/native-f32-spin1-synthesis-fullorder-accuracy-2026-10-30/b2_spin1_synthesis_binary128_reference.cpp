#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include <string>
#include <tuple>
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

static std::vector<Q> basis(Q theta, unsigned m, unsigned lmax) {
  std::vector<Q> y(lmax + 1, 0);
  const auto &factors = basis_factors();
  const Q pi = acosq(-1);
  const Q sine = sinq(theta), cosine = cosq(theta);
  y[0] = 1 / sqrtq(4 * pi);
  for (unsigned k = 1; k <= m; ++k)
    y[k] = -sqrtq(static_cast<Q>(2 * k + 1) / (2 * k)) * sine * y[k - 1];
  if (m == lmax) return y;
  y[m + 1] = sqrtq(static_cast<Q>(2 * m + 3)) * cosine * y[m];
  for (unsigned l = m + 2; l <= lmax; ++l) {
    y[l] = factors.a[m][l] * cosine * y[l - 1] - factors.b[m][l] * y[l - 2];
  }
  return y;
}

static const std::array<unsigned, 11> &selected_orders() {
  static const std::array<unsigned, 11> orders{0, 1, 2, 5, 8, 16, 64, 180, 360, 719, 720};
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

static std::vector<QC> fourier_from_rust_spectrum(const std::string &path,
                                                  unsigned ntheta, unsigned nphi,
                                                  unsigned spectrum_len, unsigned mmax) {
  const auto spectrum = read_complex<float>(path);
  if (spectrum.size() != static_cast<size_t>(ntheta) * spectrum_len)
    throw std::runtime_error("unexpected Rust spectrum size: " + path);
  std::vector<QC> out(static_cast<size_t>(ntheta) * (mmax + 1));
  for (unsigned ring = 0; ring < ntheta; ++ring)
    for (unsigned m = 0; m <= mmax; ++m)
      out[static_cast<size_t>(ring) * (mmax + 1) + m] =
          (1 / static_cast<Q>(nphi)) * spectrum[static_cast<size_t>(ring) * spectrum_len + m];
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

static Projection scalar_project(const Geometry &g, const std::vector<QC> &fourier,
                                 unsigned lmax, unsigned mmax,
                                 bool divide_by_sine) {
  Projection result;
  size_t count = 0;
  for (unsigned m = 0; m <= mmax; ++m) count += lmax - m + 1;
  result.coefficient.assign(count, {0, 0});
  result.envelope.assign(count, 0);
  const auto &orders = selected_orders();
  for (size_t ring = 0; ring < g.theta.size(); ++ring) {
    const Q sine = sinq(g.theta[ring]);
    for (const unsigned m : orders) {
      const auto y = basis(g.theta[ring], m, lmax);
      const QC f = fourier[ring * (mmax + 1) + m];
      Q weight = g.weight[ring];
      if (divide_by_sine) weight /= sine;
      for (unsigned l = m; l <= lmax; ++l) {
        const size_t index = packed_index(l, m, lmax, mmax);
        const Q scale = weight * y[l];
        result.coefficient[index] = result.coefficient[index] + scale * f;
        result.envelope[index] += qabs(scale) * qabs(f);
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
                               const std::vector<Q> &envelope, const std::vector<QC> &f32,
                               const std::vector<QC> &f64, unsigned lmax, unsigned mmax,
                               size_t offset32, size_t offset64, const std::string &operation,
                               const std::string &component) {
  std::ofstream out(path);
  out << "m,l,component,reference_re,reference_im,f32_re,f32_im,f64_re,f64_im,"
         "f32_abs,f64_abs,f32_scaled,f64_scaled\n";
  Q sum_ref = 0, sum_err32 = 0, sum_err64 = 0, max32 = 0, max64 = 0;
  Q max_scaled32 = 0, max_scaled64 = 0;
  for (const unsigned m : selected_orders()) {
    for (unsigned l = std::max(m, 600U); l <= lmax; ++l) {
      const size_t index = packed_index(l, m, lmax, mmax);
      const QC a32 = f32[offset32 + index], a64 = f64[offset64 + index];
      const QC ref = expected[index];
      const Q e32 = complex_error(a32, ref), e64 = complex_error(a64, ref);
      const Q env = std::max(envelope[index], qabs(ref));
      const Q s32 = env == 0 ? 0 : e32 / env;
      const Q s64 = env == 0 ? 0 : e64 / env;
      out << m << ',' << l << ',' << component << ',' << qstr(ref.re) << ',' << qstr(ref.im)
          << ',' << qstr(a32.re) << ',' << qstr(a32.im) << ',' << qstr(a64.re) << ','
          << qstr(a64.im) << ',' << qstr(e32) << ',' << qstr(e64) << ',' << qstr(s32)
          << ',' << qstr(s64) << '\n';
      sum_ref += ref.re * ref.re + ref.im * ref.im;
      sum_err32 += e32 * e32;
      sum_err64 += e64 * e64;
      max32 = std::max(max32, e32);
      max64 = std::max(max64, e64);
      max_scaled32 = std::max(max_scaled32, s32);
      max_scaled64 = std::max(max_scaled64, s64);
    }
  }
  const Q rel32 = sum_ref == 0 ? 0 : sqrtq(sum_err32 / sum_ref);
  const Q rel64 = sum_ref == 0 ? 0 : sqrtq(sum_err64 / sum_ref);
  std::cerr << operation << ' ' << component << " selected_modes=" << selected_orders().size()
            << " l=600.." << lmax << " max_abs_f32=" << qstr(max32)
            << " max_abs_f64=" << qstr(max64) << " rel_l2_f32=" << qstr(rel32)
            << " rel_l2_f64=" << qstr(rel64) << " max_envelope_scaled_f32="
            << qstr(max_scaled32) << " max_envelope_scaled_f64=" << qstr(max_scaled64)
            << "\n";
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

struct SpectrumStats {
  Q reference_norm = 0;
  Q error_norm = 0;
  Q max_absolute = 0;
  Q max_scaled = 0;
  unsigned max_absolute_ring = 0;
  unsigned max_scaled_ring = 0;
};

static void compare_spin_synthesis_fullorder_spectra(const std::string &directory,
                                                      const Geometry &g,
                                                      unsigned lmax, unsigned mmax,
                                                      unsigned nphi) {
  const size_t nalm = (static_cast<size_t>(lmax) + 1) * (lmax + 2) / 2;
  const auto e32 = read_complex<float>(directory + "/spin-synthesis-e-input.f32");
  const auto b32 = read_complex<float>(directory + "/spin-synthesis-b-input.f32");
  if (e32.size() != nalm || b32.size() != nalm)
    throw std::runtime_error("unexpected spin synthesis input length");
  std::vector<QC> e(nalm), b(nalm);
  for (size_t i = 0; i < nalm; ++i) {
    e[i] = {e32[i].re, e32[i].im};
    b[i] = {b32[i].re, b32[i].im};
  }
  const auto channels = prepare_spin_channels(e, b, lmax, mmax);
  if (std::getenv("B2_CHANNEL_PROPAGATION_ONLY")) {
    const auto q_spectrum = read_complex<float>(directory + "/spin-synthesis-q-spectrum.f32");
    const auto u_spectrum = read_complex<float>(directory + "/spin-synthesis-u-spectrum.f32");
    const size_t spectrum_len = nphi / 2 + 1;
    const std::array<unsigned, 3> orders{1, 16, 719};
    const std::array<unsigned, 8> rings{0, 1, 5, 360, 482, 715, 719, 720};
    std::ofstream out(directory + "/spin-synthesis-channel-propagation.csv");
    out << "ring,m,component,exact_channel_reference_re,exact_channel_reference_im,"
           "rounded_f32_channel_reference_re,rounded_f32_channel_reference_im,"
           "rust_f32_spectrum_re,rust_f32_spectrum_im,channel_prep_abs_error,"
           "remaining_abs_error\n";
    for (const unsigned m : orders) {
      for (unsigned component = 0; component < 2; ++component) {
        const std::string name = directory + "/spin-synthesis-channel-" +
                                 (component == 0 ? "first" : "second") + "-m" +
                                 std::to_string(m) + ".f32";
        const auto rounded_channels = read_complex<float>(name);
        const auto &exact_channels = component == 0 ? channels.first[m] : channels.second[m];
        const auto &spectrum = component == 0 ? q_spectrum : u_spectrum;
        for (const unsigned ring : rings) {
          const auto y = basis(g.theta[ring], m, lmax + 1);
          QC exact{0, 0}, rounded{0, 0};
          for (unsigned l = m; l <= lmax + 1; ++l) {
            exact = exact + y[l] * exact_channels[l];
            rounded = rounded + y[l] * QC{rounded_channels[l].re, rounded_channels[l].im};
          }
          const Q sine = sinq(g.theta[ring]);
          exact = (1 / sine) * exact;
          rounded = (1 / sine) * rounded;
          const auto &actual = spectrum[static_cast<size_t>(ring) * spectrum_len + m];
          const QC value{actual.re, actual.im};
          out << ring << ',' << m << ',' << (component == 0 ? "Q" : "U") << ','
              << qstr(exact.re) << ',' << qstr(exact.im) << ',' << qstr(rounded.re) << ','
              << qstr(rounded.im) << ',' << qstr(value.re) << ',' << qstr(value.im) << ','
              << qstr(qabs(rounded - exact)) << ',' << qstr(qabs(value - rounded)) << '\n';
        }
      }
    }
    return;
  }
  if (std::getenv("B2_CHANNEL_ONLY")) {
    std::ofstream channel_summary(directory + "/spin-synthesis-channel-preparation-summary.csv");
    channel_summary << "m,component,relative_l2,max_absolute,max_absolute_l\n";
    for (const unsigned m : {1U, 16U, 719U}) {
      for (unsigned component = 0; component < 2; ++component) {
        const std::string name = directory + "/spin-synthesis-channel-" +
                                 (component == 0 ? "first" : "second") + "-m" +
                                 std::to_string(m) + ".f32";
        const auto actual = read_complex<float>(name);
        const auto &reference = component == 0 ? channels.first[m] : channels.second[m];
        Q norm_ref = 0, error_norm = 0, max_absolute = 0;
        unsigned max_l = 0;
        for (unsigned l = m; l <= lmax + 1; ++l) {
          const QC value{actual[l].re, actual[l].im};
          const Q error = qabs(value - reference[l]);
          norm_ref += qabs(reference[l]) * qabs(reference[l]);
          error_norm += error * error;
          if (error > max_absolute) {
            max_absolute = error;
            max_l = l;
          }
        }
        const Q relative = sqrtq(error_norm / norm_ref);
        channel_summary << m << ',' << (component == 0 ? "first" : "second") << ','
                        << qstr(relative) << ',' << qstr(max_absolute) << ',' << max_l << '\n';
      }
    }
    return;
  }
  const auto q_spectrum = read_complex<float>(directory + "/spin-synthesis-q-spectrum.f32");
  const auto u_spectrum = read_complex<float>(directory + "/spin-synthesis-u-spectrum.f32");
  const auto q_spectrum64 = read_complex<double>(directory + "/spin-synthesis-q-spectrum.f64");
  const auto u_spectrum64 = read_complex<double>(directory + "/spin-synthesis-u-spectrum.f64");
  const auto q_map32 = read_binary<float>(directory + "/spin-synthesis-q.f32");
  const auto u_map32 = read_binary<float>(directory + "/spin-synthesis-u.f32");
  const auto q_map64 = read_binary<double>(directory + "/spin-synthesis-q.f64");
  const auto u_map64 = read_binary<double>(directory + "/spin-synthesis-u.f64");
  const size_t ntheta = g.theta.size();
  const size_t spectrum_len = nphi / 2 + 1;
  if (q_spectrum.size() != ntheta * spectrum_len ||
      u_spectrum.size() != ntheta * spectrum_len ||
      q_spectrum64.size() != ntheta * spectrum_len ||
      u_spectrum64.size() != ntheta * spectrum_len)
    throw std::runtime_error("unexpected spin synthesis spectrum length");

  std::vector<SpectrumStats> q_by_order(mmax + 1), u_by_order(mmax + 1);
  std::vector<SpectrumStats> q_by_order64(mmax + 1), u_by_order64(mmax + 1);
  std::vector<SpectrumStats> q32_vs_64_by_order(mmax + 1), u32_vs_64_by_order(mmax + 1);
  SpectrumStats q_total, u_total, q_total64, u_total64, q32_vs_64_total, u32_vs_64_total;
  std::ofstream summary(directory + "/spin-synthesis-spectrum-fullorder-summary.csv");
  summary << "m,component,f32_rel_l2,f64_rel_l2,f32_vs_f64_rel_l2,f32_max_abs,f32_max_abs_ring,"
             "f32_max_envelope_scaled,f32_max_scaled_ring,f64_max_abs,f64_max_abs_ring,"
             "f64_max_envelope_scaled,f64_max_scaled_ring\n";
  std::ofstream selected(directory + "/spin-synthesis-spectrum-selected-rings.csv");
  selected << "ring,m,component,reference_re,reference_im,f32_re,f32_im,f64_re,f64_im,"
              "f32_absolute_error,f32_envelope_scaled_error,f64_absolute_error,"
              "f64_envelope_scaled_error,f32_vs_f64_absolute_error\n";
  const std::array<unsigned, 11> selected_orders{0, 1, 2, 5, 8, 16, 64, 180, 360, 719, 720};
  const std::array<unsigned, 8> selected_rings{0, 1, 5, 360, 482, 715, 719, 720};
  const bool selected_only = std::getenv("B2_SELECTED_ONLY") != nullptr;
  const bool map_only = std::getenv("B2_MAP_ONLY") != nullptr;
  std::vector<std::vector<QC>> reference_q_by_ring(ntheta, std::vector<QC>(mmax + 1));
  std::vector<std::vector<QC>> reference_u_by_ring(ntheta, std::vector<QC>(mmax + 1));
  auto record = [](SpectrumStats &order_stats, SpectrumStats &total_stats, QC reference,
                   Q error, Q scaled, unsigned ring) {
    const Q ref_norm = reference.re * reference.re + reference.im * reference.im;
    order_stats.reference_norm += ref_norm;
    order_stats.error_norm += error * error;
    total_stats.reference_norm += ref_norm;
    total_stats.error_norm += error * error;
    if (error > order_stats.max_absolute) {
      order_stats.max_absolute = error;
      order_stats.max_absolute_ring = ring;
    }
    if (scaled > order_stats.max_scaled) {
      order_stats.max_scaled = scaled;
      order_stats.max_scaled_ring = ring;
    }
    if (error > total_stats.max_absolute) {
      total_stats.max_absolute = error;
      total_stats.max_absolute_ring = ring;
    }
    if (scaled > total_stats.max_scaled) {
      total_stats.max_scaled = scaled;
      total_stats.max_scaled_ring = ring;
    }
  };
  for (unsigned ring = 0; ring < ntheta; ++ring) {
    if ((selected_only || map_only) &&
        std::find(selected_rings.begin(), selected_rings.end(), ring) == selected_rings.end())
      continue;
    const Q sine = sinq(g.theta[ring]);
    for (unsigned m = 0; m <= mmax; ++m) {
      if (selected_only &&
          std::find(selected_orders.begin(), selected_orders.end(), m) == selected_orders.end())
        continue;
      const auto y = basis(g.theta[ring], m, lmax + 1);
      QC ref_q{0, 0}, ref_u{0, 0};
      Q envelope_q = 0, envelope_u = 0;
      for (unsigned l = m; l <= lmax + 1; ++l) {
        const QC c_q = y[l] * channels.first[m][l];
        const QC c_u = y[l] * channels.second[m][l];
        ref_q = ref_q + c_q;
        ref_u = ref_u + c_u;
        envelope_q += qabs(c_q);
        envelope_u += qabs(c_u);
      }
      ref_q = (1 / sine) * ref_q;
      ref_u = (1 / sine) * ref_u;
      reference_q_by_ring[ring][m] = ref_q;
      reference_u_by_ring[ring][m] = ref_u;
      envelope_q /= sine;
      envelope_u /= sine;
      const size_t index = static_cast<size_t>(ring) * spectrum_len + m;
      const QC actual_q{q_spectrum[index].re, q_spectrum[index].im};
      const QC actual_u{u_spectrum[index].re, u_spectrum[index].im};
      const QC actual_q64{q_spectrum64[index].re, q_spectrum64[index].im};
      const QC actual_u64{u_spectrum64[index].re, u_spectrum64[index].im};
      const Q error_q = qabs(actual_q - ref_q);
      const Q error_u = qabs(actual_u - ref_u);
      const Q error_q64 = qabs(actual_q64 - ref_q);
      const Q error_u64 = qabs(actual_u64 - ref_u);
      const Q error_q32_vs_64 = qabs(actual_q - actual_q64);
      const Q error_u32_vs_64 = qabs(actual_u - actual_u64);
      const Q scaled_q = error_q / std::max(envelope_q, qabs(ref_q));
      const Q scaled_u = error_u / std::max(envelope_u, qabs(ref_u));
      const Q scaled_q64 = error_q64 / std::max(envelope_q, qabs(ref_q));
      const Q scaled_u64 = error_u64 / std::max(envelope_u, qabs(ref_u));
      const Q scaled_q32_vs_64 = error_q32_vs_64 / std::max(envelope_q, qabs(actual_q64));
      const Q scaled_u32_vs_64 = error_u32_vs_64 / std::max(envelope_u, qabs(actual_u64));
      record(q_by_order[m], q_total, ref_q, error_q, scaled_q, ring);
      record(u_by_order[m], u_total, ref_u, error_u, scaled_u, ring);
      record(q_by_order64[m], q_total64, ref_q, error_q64, scaled_q64, ring);
      record(u_by_order64[m], u_total64, ref_u, error_u64, scaled_u64, ring);
      record(q32_vs_64_by_order[m], q32_vs_64_total, actual_q64, error_q32_vs_64,
             scaled_q32_vs_64, ring);
      record(u32_vs_64_by_order[m], u32_vs_64_total, actual_u64, error_u32_vs_64,
             scaled_u32_vs_64, ring);
      if (std::find(selected_orders.begin(), selected_orders.end(), m) != selected_orders.end() &&
          std::find(selected_rings.begin(), selected_rings.end(), ring) != selected_rings.end()) {
        selected << ring << ',' << m << ",Q," << qstr(ref_q.re) << ',' << qstr(ref_q.im)
                 << ',' << qstr(actual_q.re) << ',' << qstr(actual_q.im) << ','
                 << qstr(actual_q64.re) << ',' << qstr(actual_q64.im) << ',' << qstr(error_q)
                 << ',' << qstr(scaled_q) << ',' << qstr(error_q64) << ',' << qstr(scaled_q64)
                 << ',' << qstr(error_q32_vs_64) << '\n';
        selected << ring << ',' << m << ",U," << qstr(ref_u.re) << ',' << qstr(ref_u.im)
                 << ',' << qstr(actual_u.re) << ',' << qstr(actual_u.im) << ','
                 << qstr(actual_u64.re) << ',' << qstr(actual_u64.im) << ',' << qstr(error_u)
                 << ',' << qstr(scaled_u) << ',' << qstr(error_u64) << ',' << qstr(scaled_u64)
                 << ',' << qstr(error_u32_vs_64) << '\n';
      }
    }
  }
  auto write_stat = [&](int m, const char *component, const SpectrumStats &stats,
                        const SpectrumStats &stats64, const SpectrumStats &cross) {
    const Q relative = sqrtq(stats.error_norm / stats.reference_norm);
    const Q relative64 = sqrtq(stats64.error_norm / stats64.reference_norm);
    const Q relative_cross = sqrtq(cross.error_norm / cross.reference_norm);
    summary << m << ',' << component << ',' << qstr(relative) << ',' << qstr(relative64) << ','
            << qstr(relative_cross) << ',' << qstr(stats.max_absolute) << ','
            << stats.max_absolute_ring << ',' << qstr(stats.max_scaled) << ','
            << stats.max_scaled_ring << ',' << qstr(stats64.max_absolute) << ','
            << stats64.max_absolute_ring << ',' << qstr(stats64.max_scaled) << ','
            << stats64.max_scaled_ring << '\n';
  };
  write_stat(-1, "Q_total", q_total, q_total64, q32_vs_64_total);
  write_stat(-1, "U_total", u_total, u_total64, u32_vs_64_total);
  for (unsigned m = 0; m <= mmax; ++m) {
    if (selected_only &&
        std::find(selected_orders.begin(), selected_orders.end(), m) == selected_orders.end())
      continue;
    write_stat(static_cast<int>(m), "Q", q_by_order[m], q_by_order64[m],
               q32_vs_64_by_order[m]);
    write_stat(static_cast<int>(m), "U", u_by_order[m], u_by_order64[m],
               u32_vs_64_by_order[m]);
  }
  std::ofstream map_out(directory + "/spin-synthesis-map-selected-pixels.csv");
  map_out << "ring,longitude,component,reference,f32_spectrum_inverse,f32_map,f64_spectrum_inverse,"
             "f64_map,f32_abs_error,f32_fft_residual,f64_abs_error\n";
  const std::array<unsigned, 5> selected_longitudes{0, 1, nphi / 4, nphi / 2, nphi - 1};
  const Q pi = acosq(-1), two_pi = 2 * pi;
  for (const unsigned ring : selected_rings) {
    for (const unsigned longitude : selected_longitudes) {
      const Q phi = two_pi * static_cast<Q>(longitude) / nphi;
      for (unsigned component = 0; component < 2; ++component) {
        const auto &reference = component == 0 ? reference_q_by_ring[ring]
                                                : reference_u_by_ring[ring];
        const auto &spectrum32 = component == 0 ? q_spectrum : u_spectrum;
        const auto &spectrum64 = component == 0 ? q_spectrum64 : u_spectrum64;
        const auto &map32 = component == 0 ? q_map32 : u_map32;
        const auto &map64 = component == 0 ? q_map64 : u_map64;
        Q expected = reference[0].re;
        Q inverse32 = static_cast<Q>(spectrum32[ring * spectrum_len].re);
        Q inverse64 = static_cast<Q>(spectrum64[ring * spectrum_len].re);
        for (unsigned m = 1; m <= mmax; ++m) {
          const Q angle = static_cast<Q>(m) * phi;
          const Q cosine = cosq(angle), sine = sinq(angle);
          expected += 2 * (reference[m].re * cosine - reference[m].im * sine);
          const auto &value32 = spectrum32[ring * spectrum_len + m];
          const auto &value64 = spectrum64[ring * spectrum_len + m];
          inverse32 += 2 * (static_cast<Q>(value32.re) * cosine -
                            static_cast<Q>(value32.im) * sine);
          inverse64 += 2 * (static_cast<Q>(value64.re) * cosine -
                            static_cast<Q>(value64.im) * sine);
        }
        const size_t pixel = static_cast<size_t>(ring) * nphi + longitude;
        const Q actual32 = map32[pixel], actual64 = map64[pixel];
        map_out << ring << ',' << longitude << ',' << (component == 0 ? "Q" : "U") << ','
                << qstr(expected) << ',' << qstr(inverse32) << ',' << qstr(actual32) << ','
                << qstr(inverse64) << ',' << qstr(actual64) << ',' << qstr(qabs(actual32 - expected))
                << ',' << qstr(qabs(actual32 - inverse32)) << ','
                << qstr(qabs(actual64 - expected)) << '\n';
      }
    }
  }
  std::cerr << "fullorder spin-synthesis spectrum Q rel_l2="
            << qstr(sqrtq(q_total.error_norm / q_total.reference_norm))
            << " f64_rel_l2=" << qstr(sqrtq(q_total64.error_norm / q_total64.reference_norm))
            << " f32_vs_f64_rel_l2="
            << qstr(sqrtq(q32_vs_64_total.error_norm / q32_vs_64_total.reference_norm))
            << " max_abs=" << qstr(q_total.max_absolute)
            << " max_abs_ring=" << q_total.max_absolute_ring
            << " max_envelope_scaled=" << qstr(q_total.max_scaled)
            << " max_scaled_ring=" << q_total.max_scaled_ring << '\n';
  std::cerr << "fullorder spin-synthesis spectrum U rel_l2="
            << qstr(sqrtq(u_total.error_norm / u_total.reference_norm))
            << " f64_rel_l2=" << qstr(sqrtq(u_total64.error_norm / u_total64.reference_norm))
            << " f32_vs_f64_rel_l2="
            << qstr(sqrtq(u32_vs_64_total.error_norm / u32_vs_64_total.reference_norm))
            << " max_abs=" << qstr(u_total.max_absolute)
            << " max_abs_ring=" << u_total.max_absolute_ring
            << " max_envelope_scaled=" << qstr(u_total.max_scaled)
            << " max_scaled_ring=" << u_total.max_scaled_ring << '\n';
}

int main(int argc, char **argv) {
  if (argc != 2) throw std::runtime_error("usage: phasec_fullorder_accuracy_reference DIR");
  const std::string dir = argv[1];
  const unsigned lmax = 720, mmax = 720, ntheta = 721, nphi = 1458;
  const Geometry g = read_geometry(dir + "/geometry.csv");
  if (g.theta.size() != ntheta) throw std::runtime_error("unexpected ntheta");
  if (std::getenv("B2_SYNTHESIS_ONLY")) {
    compare_spin_synthesis_fullorder_spectra(dir, g, lmax, mmax, nphi);
    return 0;
  }

  const auto scalar_map = read_binary<float>(dir + "/scalar-map.f32");
  const auto scalar32 = read_complex<float>(dir + "/scalar-analysis.f32");
  const auto scalar64 = read_complex<double>(dir + "/scalar-analysis.f64");
  const auto scalar_f = fourier_average(scalar_map, ntheta, nphi, mmax);
  const auto scalar_ref = scalar_project(g, scalar_f, lmax, mmax, false);
  compare_projection(dir + "/scalar-analysis-highband.csv", scalar_ref.coefficient,
                     scalar_ref.envelope, scalar32, scalar64, lmax, mmax, 0, 0,
                     "scalar-analysis", "alm");

  const auto q_map = read_binary<float>(dir + "/q-map.f32");
  const auto u_map = read_binary<float>(dir + "/u-map.f32");
  const auto q_fourier = fourier_average(q_map, ntheta, nphi, mmax);
  const auto u_fourier = fourier_average(u_map, ntheta, nphi, mmax);
  const auto q_projected = scalar_project(g, q_fourier, lmax + 1, mmax, true);
  const auto u_projected = scalar_project(g, u_fourier, lmax + 1, mmax, true);
  const auto spin_ref = spin_transpose(q_projected, u_projected, lmax, mmax);
  const auto spin32e = read_complex<float>(dir + "/spin-analysis-e.f32");
  const auto spin32b = read_complex<float>(dir + "/spin-analysis-b.f32");
  const auto spin64e = read_complex<double>(dir + "/spin-analysis-e.f64");
  const auto spin64b = read_complex<double>(dir + "/spin-analysis-b.f64");
  const size_t nalm = spin32e.size();
  const std::vector<QC> expected_e(spin_ref.coefficient.begin(),
                                   spin_ref.coefficient.begin() + nalm);
  const std::vector<QC> expected_b(spin_ref.coefficient.begin() + nalm,
                                   spin_ref.coefficient.end());
  const std::vector<Q> envelope_e(spin_ref.envelope.begin(),
                                  spin_ref.envelope.begin() + nalm);
  const std::vector<Q> envelope_b(spin_ref.envelope.begin() + nalm,
                                  spin_ref.envelope.end());
  compare_projection(dir + "/spin-analysis-e-highband.csv", expected_e, envelope_e,
                     spin32e, spin64e, lmax, mmax, 0, 0, "spin-analysis", "E");
  compare_projection(dir + "/spin-analysis-b-highband.csv", expected_b, envelope_b,
                     spin32b, spin64b, lmax, mmax, 0, 0, "spin-analysis", "B");
  compare_spin_synthesis_fullorder_spectra(dir, g, lmax, mmax, nphi);
  return 0;
}
