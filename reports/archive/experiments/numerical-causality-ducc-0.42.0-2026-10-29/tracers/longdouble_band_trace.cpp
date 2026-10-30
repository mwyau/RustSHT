#include <cmath>
#include <complex>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <vector>

static uint32_t fbits(float x) { uint32_t u; std::memcpy(&u,&x,4); return u; }
static long double read_theta(uint64_t bits) { double x; std::memcpy(&x,&bits,8); return (long double)x; }
static long double value(const unsigned char *data, size_t offset) {
  float x; std::memcpy(&x,data+offset,4); return (long double)x;
}

int main(int argc,char **argv) {
  if(argc!=3) return 2;
  constexpr size_t lmax=720, start=600, band=121;
  const size_t rings[]={0,1,5,360};
  std::vector<unsigned char> coeff(16*band*8);
  std::ifstream cf(argv[1],std::ios::binary);
  cf.read(reinterpret_cast<char*>(coeff.data()),coeff.size());
  if((size_t)cf.gcount()!=coeff.size()) return 3;
  std::ifstream tf(argv[2]); std::vector<uint64_t> theta_bits(lmax+1);
  for(size_t r=0;r<=lmax;r++) { std::string s; if(!(tf>>s)) return 4; theta_bits[r]=std::strtoull(s.c_str(),nullptr,16); }
  for(size_t m=1;m<=16;m++) {
    std::vector<long double> eps(lmax+4,0), alpha(lmax/2+2,0), a(lmax/2+2,0), b(lmax/2+2,0);
    long double norm=1/sqrtl(4*acosl(-1.L));
    for(size_t j=1;j<=m;j++) norm*=sqrtl((2.L*j+1)/(2.L*j));
    for(size_t l=m+1;l<lmax+4;l++) eps[l]=sqrtl(((long double)(l+m)*(l-m))/((long double)(2*l+1)*(2*l-1)));
    alpha[0]=1/eps[m+1]; alpha[1]=eps[m+1]/(eps[m+2]*eps[m+3]);
    for(size_t il=1,l=m+2;l<lmax+1;il++,l+=2) alpha[il+1]=((il&1)?-1.L:1.L)/(eps[l+2]*eps[l+3]*alpha[il]);
    for(size_t il=0,l=m;l<lmax+2;il++,l+=2) { a[il]=((il&1)?-1.L:1.L)*alpha[il]*alpha[il]; b[il]=-a[il]*(eps[l+2]*eps[l+2]+eps[l+1]*eps[l+1]); }
    for(size_t rr=0;rr<4;rr++) {
      size_t r=rings[rr]; long double theta=read_theta(theta_bits[r]);
      long double s=sinl(theta), c=cosl(theta), csq=fabsl(c)>.99L?(1-s)*(1+s):c*c;
      long double prev=0,curr=((m&1)?-1.L:1.L)*norm*powl(s,(long double)m);
      long double evenr=0,eveni=0,oddr=0,oddi=0;
      for(size_t pair=0,l=m;l<=lmax+1;pair++,l+=2) {
        long double xr=0,xi=0,x1r=0,x1i=0,x2r=0,x2i=0;
        if(l>=start&&l<=lmax) { size_t off=((m-1)*band+l-start)*8; xr=value(coeff.data(),off); xi=value(coeff.data(),off+4); }
        if(l+1>=start&&l+1<=lmax) { size_t off=((m-1)*band+l+1-start)*8; x1r=value(coeff.data(),off); x1i=value(coeff.data(),off+4); }
        if(l+2>=start&&l+2<=lmax) { size_t off=((m-1)*band+l+2-start)*8; x2r=value(coeff.data(),off); x2i=value(coeff.data(),off+4); }
        long double er=alpha[pair]*(eps[l+1]*xr+eps[l+2]*x2r);
        long double ei=alpha[pair]*(eps[l+1]*xi+eps[l+2]*x2i);
        long double orr=alpha[pair]*x1r, oi=alpha[pair]*x1i;
        evenr=fmal(curr,er,evenr); eveni=fmal(curr,ei,eveni);
        oddr=fmal(curr,orr,oddr); oddi=fmal(curr,oi,oddi);
        if(l+2<=lmax+1) { long double lead=fmal(a[pair],csq,b[pair]); long double next=fmal(lead,curr,prev); prev=curr; curr=next; }
      }
      long double nr=fmal(oddr,c,evenr), ni=fmal(oddi,c,eveni);
      long double sr=fmal(-oddr,c,evenr), si=fmal(-oddi,c,eveni);
      float fnr=(float)nr,fni=(float)ni,fsr=(float)sr,fsi=(float)si;
      std::printf("LONGDOUBLE_OUT,%zu,%zu,%016llx,nr=%.21Le,ni=%.21Le,sr=%.21Le,si=%.21Le,bits=%08x:%08x:%08x:%08x\n",
        m,r,(unsigned long long)theta_bits[r],nr,ni,sr,si,fbits(fnr),fbits(fni),fbits(fsr),fbits(fsi));
    }
  }
}
