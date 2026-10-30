#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <string>
#include <vector>
static uint32_t fbits(float x) { uint32_t u; std::memcpy(&u,&x,4); return u; }
int main(int argc,char **argv) {
  if (argc!=2) return 2;
  const size_t lmax=720, ntheta=721, nphi=34;
  const size_t orders[]={0,1,2,5,8,16};
  const size_t degrees[]={600,650,700,719,720};
  const size_t rings[]={0,1,5,360,715,719,720};
  std::ifstream in(argv[1]);
  if (!in) return 3;
  std::vector<double> theta(ntheta);
  for (size_t r=0;r<ntheta;++r) {
    std::string line;
    if (!std::getline(in,line)) return 4;
    uint64_t bits=std::stoull(line,nullptr,16);
    std::memcpy(&theta[r],&bits,8);
  }
  const long double pi=acosl(-1.L);
  for (size_t m:orders) for (size_t selected_l:degrees) {
    if (selected_l<m) continue;
    for (int part=0;part<(m==0?1:2);++part) {
      std::vector<long double> eps(lmax+4,0), alpha(lmax/2+4,0), a(lmax/2+4,0), b(lmax/2+4,0), alm(lmax+3,0);
      long double mfac=1.L/sqrtl(4.L*pi);
      for(size_t j=1;j<=m;++j) mfac*=sqrtl((2.L*j+1)/(2.L*j));
      for(size_t l=m+1;l<lmax+4;++l)
        eps[l]=sqrtl(((long double)(l+m)*(long double)(l-m))/((long double)(2*l+1)*(long double)(2*l-1)));
      alpha[0]=1.L/eps[m+1];
      alpha[1]=eps[m+1]/(eps[m+2]*eps[m+3]);
      for(size_t il=1,l=m+2;l<lmax+1;++il,l+=2)
        alpha[il+1]=((il&1)?-1.L:1.L)/(eps[l+2]*eps[l+3]*alpha[il]);
      for(size_t il=0,l=m;l<lmax+2;++il,l+=2) {
        a[il]=((il&1)?-1.L:1.L)*alpha[il]*alpha[il];
        b[il]=-a[il]*(eps[l+2]*eps[l+2]+eps[l+1]*eps[l+1]);
      }
      alm[selected_l]=1.L;
      for(size_t ring:rings) {
        const long double angle=(long double)theta[ring];
        const long double s=sinl(angle), c=cosl(angle);
        const long double csq=fabsl(c)>0.99L ? (1.L-s)*(1.L+s) : c*c;
        const long double seed=((m&1)?-1.L:1.L)*mfac*powl(s,(long double)m);
        long double prev=0.L,curr=seed,even_sum=0.L,odd_sum=0.L;
        const size_t pair_count=(lmax-m+2)/2;
        for(size_t pair=0,l=m;pair<pair_count;++pair,l+=2) {
          long double even=alpha[pair]*(eps[l+1]*alm[l]+eps[l+2]*alm[l+2]);
          long double odd=alpha[pair]*alm[l+1];
          even_sum=fmal(curr,even,even_sum);
          odd_sum=fmal(curr,odd,odd_sum);
          if(pair+1<pair_count) {
            long double leading=fmal(a[pair],csq,b[pair]);
            long double next=fmal(leading,curr,prev);
            prev=curr;curr=next;
          }
        }
        const long double fourier=even_sum+odd_sum*c;
        for(size_t p=0;p<nphi;++p) {
          const long double phi=2.L*pi*(long double)p/(long double)nphi;
          const long double value=m==0 ? fourier : 2.L*fourier*(part==0?cosl((long double)m*phi):-sinl((long double)m*phi));
          std::printf("REF_MAP,%zu,%zu,%s,%zu,%zu,%La,%08x\n",m,selected_l,part==0?"re":"im",ring,p,value,fbits((float)value));
        }
      }
    }
  }
}
