#include <cmath>
#include <complex>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <string>
#include <vector>
#include "ducc0/infra/simd.h"
#include "ducc0/math/constants.h"
#include "ducc0/sht/sht.h"
#include "ducc0/sht/sht_inner_loop.h"

static uint64_t read_u64(const char *p) {
  uint64_t result;
  std::memcpy(&result, p, sizeof(result));
  return result;
}

static double read_f64(const char *p) {
  double result;
  std::memcpy(&result, p, sizeof(result));
  return result;
}

static uint32_t float_bits(float value) {
  uint32_t result;
  std::memcpy(&result, &value, sizeof(result));
  return result;
}

int main(int argc, char **argv) {
  using namespace ducc0;
  using namespace ducc0::detail_sht::detail_sht_inner_loop;
  if (argc != 3) return 2;
  constexpr size_t lmax=720, band_start=600, band_len=121;
  const size_t rings[] = {0,1,5,360};
  std::ifstream coeff_file(argv[1], std::ios::binary);
  std::vector<char> coeff_data((std::istreambuf_iterator<char>(coeff_file)), {});
  std::ifstream theta_file(argv[2]);
  std::vector<uint64_t> theta_bits;
  std::string line;
  while (std::getline(theta_file,line)) theta_bits.push_back(std::strtoull(line.c_str(),nullptr,16));
  if (coeff_data.size()!=16*band_len*8 || theta_bits.size()!=lmax+1) return 3;
  std::printf("DUCC_NATIVE_F32_DENSE_BAND_TRACE,1,GL_T720,orders=1..16,degrees=600..720,phi0=0,DUCC_AVX2_lanes=%zu,reflected_north_rings=0,1,5,360\n",Tv::size());
  for (size_t m=1; m<=16; ++m) {
    YlmBase base(lmax,m,0);
    Ylmgen gen(base);
    gen.prepare(m);
    std::vector<std::complex<double>> raw_alms(lmax+2,{0.,0.});
    for (size_t j=0; j<band_len; ++j) {
      size_t off=((m-1)*band_len+j)*8;
      float re,im;
      std::memcpy(&re,&coeff_data[off],4);
      std::memcpy(&im,&coeff_data[off+4],4);
      raw_alms[band_start+j]={double(re),double(im)};
      std::printf("INPUT,%zu,%zu,%08x,%08x\n",m,band_start+j,float_bits(re),float_bits(im));
    }
    for (size_t r : rings) {
      size_t south=lmax-r;
      double theta=read_f64(reinterpret_cast<const char*>(&theta_bits[r]));
      double cth=std::cos(theta), sth=std::sin(theta);
      double csq=(std::abs(cth)>0.99)?(1.-sth)*(1.+sth):cth*cth;
      vmav<std::complex<double>,2> almtmp({lmax+2,1},UNINITIALIZED);
      for (size_t l=0; l<lmax+2; ++l) almtmp(l,0)=raw_alms[l];
      std::vector<ringdata> rdata{{lmax,r,south,cth,sth,1.}};
      vmav<std::complex<float>,3> phase({1,lmax+1,1},UNINITIALIZED);
      for(size_t i=0;i<=lmax;i++) phase(0,i,0)={0.f,0.f};
      numeric_trace_order=m;
      numeric_trace_ring=r;
      numeric_trace_part="dense";
      std::printf("PLAN,%zu,%zu,%016llx,%016llx,%016llx,%016llx,%016llx,%016llx\n",
        m,r,(unsigned long long)theta_bits[r],(unsigned long long)numeric_trace_bits(cth),
        (unsigned long long)numeric_trace_bits(sth),(unsigned long long)numeric_trace_bits(csq),
        (unsigned long long)numeric_trace_bits(gen.mfac[m]),(unsigned long long)numeric_trace_bits(gen.alpha[0]));
      inner_loop_a2m(STANDARD,almtmp,phase,rdata,gen,0);
      if(south==r) phase(0,south,0)=phase(0,r,0);
      std::printf("OUTPUT,%zu,%zu,%08x,%08x,%08x,%08x\n",m,r,
        float_bits(phase(0,r,0).real()),float_bits(phase(0,r,0).imag()),
        float_bits(phase(0,south,0).real()),float_bits(phase(0,south,0).imag()));
      double prev=0., curr=std::pow(sth,double(m))*((m&1)?-gen.mfac[m]:gen.mfac[m]);
      double p1r=0.,p1i=0.,p2r=0.,p2i=0.;
      for(size_t pair=0,l=m;l<=lmax;pair++,l+=2) {
        p1r=std::fma(curr,almtmp(l,0).real(),p1r); p1i=std::fma(curr,almtmp(l,0).imag(),p1i);
        p2r=std::fma(curr,almtmp(l+1,0).real(),p2r); p2i=std::fma(curr,almtmp(l+1,0).imag(),p2i);
        if(l+2<=lmax+1) {
          double lead=std::fma(gen.coef[pair].a,csq,gen.coef[pair].b);
          double next=std::fma(lead,curr,prev); prev=curr; curr=next;
        }
      }
      std::printf("REPLAY,%zu,%zu,%016llx,%016llx,%016llx,%016llx,%016llx,%016llx\n",m,r,
        (unsigned long long)numeric_trace_bits(p1r),(unsigned long long)numeric_trace_bits(p2r),
        (unsigned long long)numeric_trace_bits(p1i),(unsigned long long)numeric_trace_bits(p2i),
        (unsigned long long)numeric_trace_bits(std::fma(cth,p2r,p1r)),
        (unsigned long long)numeric_trace_bits(std::fma(cth,p2i,p1i)));
    }
  }
}
