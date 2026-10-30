#define main prior_b2_reference_main
#include "b2_spin1_synthesis_binary128_reference.cpp"
#undef main

static void basis_into(Q theta, unsigned m, unsigned lmax, std::vector<Q> &y) {
  if (y.size() < lmax + 1) throw std::runtime_error("short basis workspace");
  const auto &factors = basis_factors();
  const Q pi = acosq(-1);
  const Q sine = sinq(theta), cosine = cosq(theta);
  y[0] = 1 / sqrtq(4 * pi);
  for (unsigned k = 1; k <= m; ++k)
    y[k] = -sqrtq(static_cast<Q>(2 * k + 1) / (2 * k)) * sine * y[k - 1];
  if (m == lmax) return;
  y[m + 1] = sqrtq(static_cast<Q>(2 * m + 3)) * cosine * y[m];
  for (unsigned l = m + 2; l <= lmax; ++l)
    y[l] = factors.a[m][l] * cosine * y[l - 1] - factors.b[m][l] * y[l - 2];
}

static void write_complex128(const std::string &path, const std::vector<QC> &values) {
  std::ofstream out(path, std::ios::binary);
  if (!out) throw std::runtime_error("cannot write " + path);
  for (const auto value : values) {
    const double re = static_cast<double>(value.re);
    const double im = static_cast<double>(value.im);
    out.write(reinterpret_cast<const char *>(&re), sizeof(re));
    out.write(reinterpret_cast<const char *>(&im), sizeof(im));
  }
  if (!out) throw std::runtime_error("short write: " + path);
}

int main(int argc, char **argv) {
  if (argc != 2) throw std::runtime_error("usage: fullarray_map_reference DIR");
  const std::string directory = argv[1];
  constexpr unsigned lmax = 720, mmax = 720, nphi = 1458;
  const Geometry geometry = read_geometry(directory + "/geometry.csv");
  const unsigned ntheta = geometry.theta.size();
  const size_t nalm = (static_cast<size_t>(lmax) + 1) * (lmax + 2) / 2;
  const auto e32 = read_complex<float>(directory + "/spin-synthesis-e-input.f32");
  const auto b32 = read_complex<float>(directory + "/spin-synthesis-b-input.f32");
  if (geometry.theta.size() != ntheta || e32.size() != nalm || b32.size() != nalm)
    throw std::runtime_error("unexpected geometry or coefficient size");
  const auto channels = prepare_spin_channels(e32, b32, lmax, mmax);
  std::vector<bool> active_order(mmax + 1, false);
  for (unsigned m = 0; m <= mmax; ++m)
    for (unsigned l = m; l <= lmax; ++l) {
      const size_t index = packed_index(l, m, lmax, mmax);
      active_order[m] = active_order[m] || e32[index].re != 0 || e32[index].im != 0 ||
                        b32[index].re != 0 || b32[index].im != 0;
    }

  std::vector<QC> q_reference(static_cast<size_t>(ntheta) * (mmax + 1), {0, 0});
  std::vector<QC> u_reference(q_reference.size(), {0, 0});
  std::vector<Q> y(lmax + 2);
  for (unsigned ring = 0; ring < ntheta; ++ring) {
    const Q theta = geometry.theta[ring];
    const Q pi = acosq(-1);
    const bool north_pole = theta == 0;
    const bool south_pole = qabs(theta - pi) < static_cast<Q>(1.0e-15);
    if (north_pole || south_pole) {
      const Q sign = north_pole ? -1 : 1;
      for (unsigned l = 1; l <= lmax + 1; ++l) {
        const Q lf = l;
        const Q parity = south_pole && (l % 2 == 1) ? -1 : 1;
        const Q limit = sign * parity * sqrtq((2 * lf + 1) * lf * (lf + 1) /
                                               (16 * pi));
        const size_t index = static_cast<size_t>(ring) * (mmax + 1) + 1;
        q_reference[index] = q_reference[index] + limit * channels.first[1][l];
        u_reference[index] = u_reference[index] + limit * channels.second[1][l];
      }
      continue;
    }
    const Q inverse_sine = 1 / sinq(theta);
    for (unsigned m = 0; m <= mmax; ++m) {
      if (!active_order[m]) continue;
      basis_into(geometry.theta[ring], m, lmax + 1, y);
      QC q{0, 0}, u{0, 0};
      for (unsigned l = m; l <= lmax + 1; ++l) {
        q = q + y[l] * channels.first[m][l];
        u = u + y[l] * channels.second[m][l];
      }
      const size_t index = static_cast<size_t>(ring) * (mmax + 1) + m;
      q_reference[index] = inverse_sine * q;
      u_reference[index] = inverse_sine * u;
    }
  }
  write_complex128(directory + "/spin-synthesis-q-reference.c128", q_reference);
  write_complex128(directory + "/spin-synthesis-u-reference.c128", u_reference);
  std::cerr << "wrote binary128-derived Q/U spectra for rings=0..720, orders=0..720\n";
  return 0;
}
