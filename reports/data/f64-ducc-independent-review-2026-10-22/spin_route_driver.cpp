// SPDX-License-Identifier: GPL-2.0-or-later
// Diagnostic only: caller-owned f64 fixtures, one public STANDARD spin-1 call.
#include <complex>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <string>
#include <vector>
#include "ducc0/infra/mav.h"
#include "ducc0/sht/sht.h"

int main(int argc, char **argv) {
  if(argc!=6) return 2;
  using C=std::complex<double>;
  size_t lmax=std::stoull(argv[1]),mmax=std::stoull(argv[2]),q=std::stoull(argv[4]);
  std::string grid=argv[3],out=argv[5];
  size_t ntheta=lmax+1+(grid=="CC"),nphi=2*(mmax+1);
  size_t nalm=(mmax+1)*(lmax+1)-mmax*(mmax+1)/2, padded=mmax+nalm;
  std::vector<C> alm(2*padded,C(0,0));
  std::vector<size_t> mstart(mmax+1);
  uint64_t state=0x7a319d428b6c05e1ULL;
  auto next=[&]() {state+=0x9e3779b97f4a7c15ULL;uint64_t v=state;
    v=(v^(v>>30))*0xbf58476d1ce4e5b9ULL;v=(v^(v>>27))*0x94d049bb133111ebULL;v^=v>>31;
    return double(v>>11)*(1./double(1ULL<<53))*2.-1.;};
  size_t offset=0;
  for(size_t m=0;m<=mmax;++m) {
    mstart[m]=mmax+offset-m;offset+=lmax-m+1;
    for(size_t l=std::max(size_t(1),m);l<=lmax;++l) {
      double er=next(),ei=m==0 ? 0. : next(),br=next(),bi=m==0 ? 0. : next();
      alm[mstart[m]+l]=C(er,ei);alm[padded+mstart[m]+l]=C(br,bi);
    }
  }
  std::vector<double> map(2*ntheta*nphi),weights(ntheta,1.);
  ducc0::synthesis_2d(ducc0::cmav<C,2>(alm.data(),{2,padded}),
    ducc0::vmav<double,3>(map.data(),{2,ntheta,nphi}),1,lmax,
    ducc0::cmav<size_t,1>(mstart.data(),{mmax+1}),1,grid,.23,
    ducc0::cmav<double,1>(weights.data(),{ntheta}),q,ducc0::STANDARD);
  if(out!="-") {std::ofstream f(out,std::ios::binary);f.write((char*)map.data(),map.size()*sizeof(double));}
}
