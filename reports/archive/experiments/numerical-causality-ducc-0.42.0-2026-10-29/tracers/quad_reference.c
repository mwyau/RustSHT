#include <quadmath.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static __float128 from_bits(const char *text) {
  uint64_t bits = strtoull(text, 0, 16); double d; memcpy(&d, &bits, 8); return (__float128)d;
}
int main(int argc, char **argv) {
  if (argc != 4) return 2;
  size_t m=strtoul(argv[1],0,10), selected=strtoul(argv[2],0,10), lmax=720;
  __float128 theta=from_bits(argv[3]), pi=acosq(-1.Q), eps[724]={0}, alpha[364]={0}, a[364]={0}, b[364]={0};
  __float128 norm=1/sqrtq(4*pi);
  for (size_t j=1;j<=m;j++) norm *= sqrtq((__float128)(2*j+1)/(__float128)(2*j));
  for(size_t l=m+1;l<lmax+4;l++) eps[l]=sqrtq(((__float128)(l+m)*(l-m))/((__float128)(2*l+1)*(2*l-1)));
  alpha[0]=1/eps[m+1]; alpha[1]=eps[m+1]/(eps[m+2]*eps[m+3]);
  for(size_t il=1,l=m+2;l<lmax+1;il++,l+=2) alpha[il+1]=((il&1)?-1.Q:1.Q)/(eps[l+2]*eps[l+3]*alpha[il]);
  for(size_t il=0,l=m;l<lmax+2;il++,l+=2) { a[il]=((il&1)?-1.Q:1.Q)*alpha[il]*alpha[il]; b[il]=-a[il]*(eps[l+2]*eps[l+2]+eps[l+1]*eps[l+1]); }
  __float128 s=sinq(theta), c=cosq(theta), csq=fabsq(c)>.99Q?(1-s)*(1+s):c*c;
  __float128 curr=((m&1)?-1.Q:1.Q)*norm*powq(s,(__float128)m), prev=0, even=0, odd=0;
  size_t count=(lmax-m+2)/2;
  for(size_t pair=0,l=m;pair<count;pair++,l+=2) {
    __float128 x=(l==selected)?1.Q:0.Q, x2=(l+2==selected)?1.Q:0.Q, x1=(l+1==selected)?1.Q:0.Q;
    __float128 ce=alpha[pair]*(eps[l+1]*x+eps[l+2]*x2), co=alpha[pair]*x1;
    even=fmaq(curr,ce,even); odd=fmaq(curr,co,odd);
    if(pair+1<count) { __float128 lead=fmaq(a[pair],csq,b[pair]); __float128 next=fmaq(lead,curr,prev); prev=curr;curr=next; }
  }
  __float128 north=fmaq(odd,c,even), south=fmaq(-odd,c,even);
  char nb[128],sb[128],eb[128],ob[128];
  quadmath_snprintf(nb,sizeof(nb),"%.36Qg",north);quadmath_snprintf(sb,sizeof(sb),"%.36Qg",south);
  quadmath_snprintf(eb,sizeof(eb),"%.36Qg",even);quadmath_snprintf(ob,sizeof(ob),"%.36Qg",odd);
  printf("%s,%s,%s,%s\n",nb,sb,eb,ob);
}
