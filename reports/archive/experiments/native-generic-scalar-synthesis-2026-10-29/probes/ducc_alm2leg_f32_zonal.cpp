#include <cmath>
#include <complex>
#include <cstdint>
#include <cstring>
#include <iomanip>
#include <iostream>
#include <vector>
#include "ducc0/infra/mav.h"
#include "ducc0/sht/sht.h"

int main() {
  using C = std::complex<float>;
  constexpr std::size_t lmax = 720;
  const float sine = 0.0033330852f;
  const double theta = std::asin(static_cast<double>(sine));
  std::vector<C> alm(lmax + 1, C(0.f, 0.f));
  alm[lmax] = C(1.f, 0.f);
  const std::size_t m = 0, start = 0;
  std::vector<std::size_t> mv{m}, starts{start};
  std::vector<C> leg(1);
  ducc0::cmav<C, 2> alm_view(alm.data(), {1, alm.size()});
  ducc0::vmav<C, 3> leg_view(leg.data(), {1, 1, 1});
  ducc0::cmav<std::size_t, 1> m_view(mv.data(), {1});
  ducc0::cmav<std::size_t, 1> start_view(starts.data(), {1});
  ducc0::cmav<double, 1> theta_view(&theta, {1});
  ducc0::alm2leg(alm_view, leg_view, 0, lmax, m_view, start_view, 1,
                  theta_view, 1, ducc0::STANDARD);
  const double x = std::sqrt((1.0 - static_cast<double>(sine)) *
                             (1.0 + static_cast<double>(sine)));
  double prev = 0.0;
  double curr = 1.0 / std::sqrt(4.0 * std::acos(-1.0));
  for (std::size_t l = 1; l <= lmax; ++l) {
    auto eps = [](std::size_t degree) {
      if (degree == 0) return 0.0;
      const double d = static_cast<double>(degree);
      return std::sqrt((d * d) / ((2.0 * d + 1.0) * (2.0 * d - 1.0)));
    };
    const double next = (x * curr - eps(l - 1) * prev) / eps(l);
    prev = curr;
    curr = next;
  }
  const float ducc_value = leg[0].real();
  std::uint32_t bits;
  std::memcpy(&bits, &ducc_value, sizeof(bits));
  std::cout << std::setprecision(17)
            << "{\"sine_f32\":" << sine
            << ",\"theta_f64\":" << theta
            << ",\"x_from_sine_f64\":" << x
            << ",\"ducc_f32\":" << ducc_value
            << ",\"ducc_bits\":" << std::hex << bits << std::dec
            << ",\"direct_f64\":" << curr
            << ",\"relative_ducc_f64\":" << (ducc_value / curr - 1.0)
            << "}\n";
}