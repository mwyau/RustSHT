#include <vector>
#include <cmath>
#include <cstring>
#include <complex>
#include <array>
#include <algorithm>
#include <iomanip>
#include <iostream>
#include "ducc0/infra/simd.h"
#include "ducc0/sht/sht.h"
#include "ducc0/sht/sphere_interpol.h"
#include "ducc0/fft/fft.h"
#include "ducc0/nufft/nufft.h"
#include "ducc0/math/math_utils.h"
#include "ducc0/math/gl_integrator.h"
#include "ducc0/math/constants.h"
#include "ducc0/math/solvers.h"
#include "ducc0/sht/sht_utils.h"
#include "ducc0/infra/timers.h"
#include "ducc0/sht/sht_inner_loop.h"
int main() {
  std::cout << std::setprecision(17);
  ducc0::GL_Integrator integrator(720);
  auto raw = integrator.thetas();
  for (size_t i=0; i<720; ++i) {
    const double theta = raw[719-i];
    const double sine = std::sin(theta), cosine = std::cos(theta);
    const double csq = (std::abs(cosine)>0.99) ? (1.-sine)*(1.+sine) : cosine*cosine;
    std::cout << "G," << i << ',' << theta << ',' << sine << ',' << cosine << ',' << csq << '\n';
  }
  ducc0::detail_sht::YlmBase base(720, 16, 0);
  ducc0::detail_sht::Ylmgen gen(base);
  auto eps = [](size_t l, size_t m) {
    if (l==m) return 0.0;
    return std::sqrt((double(l)-double(m))*(double(l)+double(m)) /
                     ((2.*double(l)+1.)*(2.*double(l)-1.)));
  };
  auto alm_value = [](size_t degree, size_t m) {
    if (degree>720 || degree<600) return 0.0;
    const float seed = float((degree*37 + m*13)%997) / 997.0f;
    const float value = (seed - 0.5f) / 721.0f;
    return double(value);
  };
  for (size_t m=1; m<=16; ++m) {
    gen.prepare(m);
    for (size_t il=0, l=m; l<=720; ++il, l+=2) {
      if (l<600) continue;
      const auto f=gen.coef[il];
      const double alpha=gen.alpha[il];
      const double eps1=gen.eps[l+1], eps2=gen.eps[l+2];
      const double e1=eps1*alpha, e2=eps2*alpha;
      std::cout << "F," << m << ',' << l << ',' << alpha << ',' << f.a << ',' << f.b
                << ',' << eps1 << ',' << eps2 << ',' << e1 << ',' << e2 << ',' << alpha << '\n';
      const double even = gen.alpha[il]*(gen.eps[l+1]*alm_value(l,m)
                                           +gen.eps[l+2]*alm_value(l+2,m));
      const double odd = gen.alpha[il]*alm_value(l+1,m);
      std::cout << "P," << m << ',' << l << ',' << even << ',' << odd << '\n';
    }
  }
}
