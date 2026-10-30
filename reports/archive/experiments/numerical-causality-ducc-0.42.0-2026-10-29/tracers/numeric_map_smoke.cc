#include <complex>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
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

static uint32_t float_bits(float x) { uint32_t u; std::memcpy(&u,&x,4); return u; }

int main(int argc,char **argv) {
  using namespace ducc0;
  if (argc != 5) return 2;
  const std::string theta_path=argv[1];
  const size_t m=std::strtoul(argv[2],nullptr,10);
  const size_t l=std::strtoul(argv[3],nullptr,10);
  const bool imag=std::strtoul(argv[4],nullptr,10)!=0;
  const size_t ntheta=721, nphi=34, lmax=720;
  std::ifstream in(theta_path);
  if (!in) return 3;
  std::vector<double> theta(ntheta);
  for (size_t i=0;i<ntheta;++i) {
    std::string line;
    if (!std::getline(in,line)) return 4;
    uint64_t bits=std::stoull(line,nullptr,16);
    std::memcpy(&theta[i],&bits,8);
  }
  const size_t nalm=(lmax+1)*(lmax+2)/2;
  vmav<std::complex<float>,2> alm({1,nalm},UNINITIALIZED);
  for(size_t i=0;i<nalm;++i) alm(0,i)={0.f,0.f};
  const size_t lmstart=m*(2*lmax+1-m)/2;
  alm(0,lmstart+l)=imag ? std::complex<float>(0.f,1.f) : std::complex<float>(1.f,0.f);
  vmav<float,2> map({1,ntheta*nphi},UNINITIALIZED);
  vmav<size_t,1> mstart({m+1},UNINITIALIZED);
  for(size_t mm=0;mm<=m;++mm)
    mstart(mm)=mm*(2*lmax+1-mm)/2;
  vmav<size_t,1> nphi_v({ntheta},UNINITIALIZED), ringstart({ntheta},UNINITIALIZED);
  vmav<double,1> theta_v({ntheta},UNINITIALIZED), phi0({ntheta},UNINITIALIZED), ringfactor({ntheta},UNINITIALIZED);
  for(size_t r=0;r<ntheta;++r) {
    theta_v(r)=theta[r]; nphi_v(r)=nphi; ringstart(r)=r*nphi; phi0(r)=0.; ringfactor(r)=1.;
  }
  synthesis(alm,map,0,lmax,mstart,1,theta_v,nphi_v,phi0,ringstart,ringfactor,1,1,STANDARD,false);
  constexpr size_t rings[]={0,1,5,360,715,719,720};
  for(size_t r:rings) {
    uint64_t theta_bits; std::memcpy(&theta_bits,&theta[r],8);
    std::printf("DUCC_MAP,m=%zu,l=%zu,part=%s,ring=%zu,theta=%016llx",
      m,l,imag?"im":"re",r,(unsigned long long)theta_bits);
    for(size_t p=0;p<nphi;++p) std::printf(",%08x",float_bits(map(0,r*nphi+p)));
    std::printf("\n");
  }
  return 0;
}
