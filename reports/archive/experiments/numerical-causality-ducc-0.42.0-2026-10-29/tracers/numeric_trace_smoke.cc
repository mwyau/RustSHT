#include <cmath>
#include <complex>
#include <cstdio>
#include <cstdint>
#include <cstring>
#include <cstdlib>
#include <string>
#include <vector>
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

static uint32_t float_bits(float value) {
  uint32_t result;
  std::memcpy(&result, &value, sizeof(result));
  return result;
}

int main(int argc, char **argv) {
  using namespace ducc0;
  using namespace ducc0::detail_sht::detail_sht_inner_loop;
  const size_t m = std::strtoul(argv[1], nullptr, 10);
  const size_t l = std::strtoul(argv[2], nullptr, 10);
  const double theta = std::strtod(argv[3], nullptr);
  const float re = std::strtof(argv[4], nullptr);
  const float im = std::strtof(argv[5], nullptr);
  const size_t ring = (argc>6) ? std::strtoul(argv[6], nullptr, 10) : 0;
  const size_t lmax = 720;
  YlmBase base(lmax, m, 0);
  Ylmgen gen(base);
  gen.prepare(m);
  vmav<std::complex<double>,2> almtmp({lmax+2, 1}, UNINITIALIZED);
  for (size_t i=0; i<lmax+2; ++i) almtmp(i,0) = {0.,0.};
  almtmp(l,0) = {double(re), double(im)};
  const double cth=std::cos(theta), sth=std::sin(theta);
  const double csq=(std::abs(cth)>0.99) ? (1.-sth)*(1.+sth) : cth*cth;
  numeric_trace_order=m;
  numeric_trace_ring=ring;
  std::string part_tag=(im!=0.f) ? "im" : "re";
  part_tag += "_l" + std::to_string(l);
  numeric_trace_part=part_tag.c_str();
  std::printf("DUCC_PLAN,%zu,%zu,%zu,%016llx,%016llx,%016llx,%016llx,%d,%016llx,%016llx,%016llx,%016llx,%016llx,%016llx,%08x,%08x\n",
      m,l,ring,(unsigned long long)numeric_trace_bits(theta),
      (unsigned long long)numeric_trace_bits(cth),(unsigned long long)numeric_trace_bits(sth),
      (unsigned long long)numeric_trace_bits(csq),std::abs(cth)>0.99,
      (unsigned long long)numeric_trace_bits(gen.mfac[m]),
      (unsigned long long)numeric_trace_bits(gen.eps[l]),
      (unsigned long long)numeric_trace_bits(gen.eps[l+1]),
      (unsigned long long)numeric_trace_bits(gen.alpha[(l-m)/2]),
      (unsigned long long)numeric_trace_bits(gen.coef[(l-m)/2].a),
      (unsigned long long)numeric_trace_bits(gen.coef[(l-m)/2].b),
      float_bits(re),float_bits(im));
  std::vector<ringdata> rdata{{lmax,0,1,cth,sth,1.}};
  vmav<std::complex<float>,3> phase({1,2,1}, UNINITIALIZED);
  inner_loop_a2m(STANDARD, almtmp, phase, rdata, gen, 0);
  for (size_t pair=0; pair<=(lmax-m)/2; ++pair)
    {
    size_t degree=m+2*pair;
    std::printf("DUCC_PAIR,%zu,%zu,%zu,%016llx,%016llx,%016llx,%016llx\n",
      m,ring,degree,
      (unsigned long long)numeric_trace_bits(almtmp(degree,0).real()),
      (unsigned long long)numeric_trace_bits(almtmp(degree,0).imag()),
      (unsigned long long)numeric_trace_bits(almtmp(degree+1,0).real()),
      (unsigned long long)numeric_trace_bits(almtmp(degree+1,0).imag()));
    }
  std::printf("DUCC_OUTPUT,%zu,%zu,%s,%08x,%08x,%08x,%08x\n",m,ring,numeric_trace_part,
      float_bits(phase(0,0,0).real()),float_bits(phase(0,0,0).imag()),
      float_bits(phase(0,1,0).real()),float_bits(phase(0,1,0).imag()));
}
