#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>
static uint32_t fbits(float x) { uint32_t u; std::memcpy(&u,&x,4); return u; }
static uint64_t dbits(double x) { uint64_t u; std::memcpy(&u,&x,8); return u; }
int main(int argc,char**argv) {
  const size_t m=std::strtoul(argv[1],nullptr,10);
  const size_t selected_l=std::strtoul(argv[2],nullptr,10);
  const long double theta=(long double)std::strtod(argv[3],nullptr);
  const size_t lmax=720;
  std::vector<long double> eps(lmax+4,0), alpha(lmax/2+2,0), a(lmax/2+2,0), b(lmax/2+2,0), alm(lmax+2,0);
  const long double pi=acosl(-1.0L);
  long double mfac=1.0L/sqrtl(4*pi);
  for(size_t j=1;j<=m;++j) mfac*=sqrtl((2.0L*j+1)/(2.0L*j));
  for(size_t l=m+1;l<lmax+4;++l)
    eps[l]=sqrtl(((long double)(l+m)*(long double)(l-m))/((long double)(2*l+1)*(long double)(2*l-1)));
  alpha[0]=1/eps[m+1];
  alpha[1]=eps[m+1]/(eps[m+2]*eps[m+3]);
  for(size_t il=1,l=m+2;l<lmax+1;++il,l+=2)
    alpha[il+1]=((il&1)?-1.0L:1.0L)/(eps[l+2]*eps[l+3]*alpha[il]);
  for(size_t il=0,l=m;l<lmax+2;++il,l+=2) {
    a[il]=((il&1)?-1.0L:1.0L)*alpha[il]*alpha[il];
    b[il]=-a[il]*(eps[l+2]*eps[l+2]+eps[l+1]*eps[l+1]);
  }
  alm[selected_l]=1;
  const long double s=sinl(theta), c=cosl(theta);
  const long double csq=fabsl(c)>0.99L ? (1-s)*(1+s) : c*c;
  long double seed=((m&1)?-1.0L:1.0L)*mfac*powl(s,(long double)m);
  long double prev=0, curr=seed;
  long double even_sum=0,odd_sum=0;
  for(size_t pair=0,l=m;l<=lmax;pair++,l+=2) {
    long double next_even=(l+2<=lmax)?alm[l+2]:0;
    long double even=alpha[pair]*(eps[l+1]*alm[l]+eps[l+2]*next_even);
    long double odd=(l<lmax)?alpha[pair]*alm[l+1]:0;
    even_sum=fmal(curr,even,even_sum);
    odd_sum=fmal(curr,odd,odd_sum);
    if(pair+1<=(lmax-m)/2) {
      long double leading=fmal(a[pair],csq,b[pair]);
      long double next=fmal(leading,curr,prev);
      prev=curr; curr=next;
    }
  }
  const long double north=fmal(odd_sum,c,even_sum);
  const long double south=fmal(-odd_sum,c,even_sum);
  const float nf=(float)north;
  std::printf("REF,m=%zu,l=%zu,theta=%016llx,sin=%016llx,cos=%016llx,csq=%La,north=%La,north_decimal=%.21Le,north_f32=%08x,seed=%La,seed_decimal=%.21Le,seed_f32=%08x,south=%La,south_f32=%08x\n",m,selected_l,(unsigned long long)dbits((double)theta),(unsigned long long)dbits((double)s),(unsigned long long)dbits((double)c),csq,north,north,fbits(nf),seed,seed,fbits((float)seed),south,fbits((float)south));
}
