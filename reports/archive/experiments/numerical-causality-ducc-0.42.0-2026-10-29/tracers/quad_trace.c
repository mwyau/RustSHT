#include <quadmath.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static __float128 from_bits(const char *text) { uint64_t bits=strtoull(text,0,16); double d; memcpy(&d,&bits,8); return (__float128)d; }
static void fmt(__float128 x, char *out) { quadmath_snprintf(out,80,"%.36Qg",x); }
int main(int argc,char **argv) {
  if (argc!=4 && argc!=5) return 2;
  int trace=argc==5 && strcmp(argv[4],"trace")==0;
  size_t m=strtoul(argv[1],0,10), selected=strtoul(argv[2],0,10), lmax=720;
  __float128 theta=from_bits(argv[3]), pi=acosq(-1.Q), eps[724]={0}, alpha[364]={0}, a[364]={0}, b[364]={0};
  __float128 norm=1/sqrtq(4*pi);
  for(size_t j=1;j<=m;j++) norm *= sqrtq((__float128)(2*j+1)/(__float128)(2*j));
  for(size_t l=m+1;l<lmax+4;l++) eps[l]=sqrtq(((__float128)(l+m)*(l-m))/((__float128)(2*l+1)*(2*l-1)));
  alpha[0]=1/eps[m+1]; alpha[1]=eps[m+1]/(eps[m+2]*eps[m+3]);
  for(size_t il=1,l=m+2;l<lmax+1;il++,l+=2) alpha[il+1]=((il&1)?-1.Q:1.Q)/(eps[l+2]*eps[l+3]*alpha[il]);
  for(size_t il=0,l=m;l<lmax+2;il++,l+=2) { a[il]=((il&1)?-1.Q:1.Q)*alpha[il]*alpha[il]; b[il]=-a[il]*(eps[l+2]*eps[l+2]+eps[l+1]*eps[l+1]); }
  __float128 s=sinq(theta), c=cosq(theta), csq=fabsq(c)>.99Q?(1-s)*(1+s):c*c;
  __float128 seed=((m&1)?-1.Q:1.Q)*norm*powq(s,(__float128)m), prev=0, curr=seed, even=0, odd=0;
  size_t count=(lmax-m+2)/2;
  if(trace) { char x[80],y[80],z[80],w[80],v[80],u[80];fmt(s,x);fmt(c,y);fmt(csq,z);fmt(norm,w);fmt(seed,v);fmt(theta,u);printf("QUAD_PLAN,%zu,%s,%s,%s,%s,%s,%s\n",m,u,x,y,z,w,v); }
  for(size_t pair=0,l=m;pair<count;pair++,l+=2) {
    __float128 x=(l==selected)?1.Q:0.Q, x2=(l+2==selected)?1.Q:0.Q, x1=(l+1==selected)?1.Q:0.Q;
    __float128 ce=alpha[pair]*(eps[l+1]*x+eps[l+2]*x2), co=alpha[pair]*x1;
    __float128 ec=curr*ce, oc=curr*co;
    even=fmaq(curr,ce,even); odd=fmaq(curr,co,odd);
    if(trace) { char v[80],q[80],r[80],t[80],ee[80],oo[80],es[80],os[80];fmt(curr,v);fmt(ce,q);fmt(co,r);fmt(ec,t);fmt(oc,ee);fmt(even,es);fmt(odd,os);printf("QUAD_PAIR,%zu,%s,%s,%s,%s,%s,%s,%s\n",l,v,q,r,t,ee,es,os); }
    if(pair+1<count) {
      __float128 leading=fmaq(a[pair],csq,b[pair]), next=fmaq(leading,curr,prev);
      if(trace) { char p[80],v[80],aa[80],bb[80],cc[80],ll[80],nn[80];fmt(prev,p);fmt(curr,v);fmt(a[pair],aa);fmt(b[pair],bb);fmt(csq,cc);fmt(leading,ll);fmt(next,nn);printf("QUAD_REC,%zu,%s,%s,%s,%s,%s,%s,%s\n",l+2,p,v,aa,bb,cc,ll,nn); }
      prev=curr;curr=next;
    }
  }
  __float128 north=fmaq(odd,c,even), south=fmaq(-odd,c,even);
  if(trace) { char e[80],o[80],co[80],n[80],so[80];fmt(even,e);fmt(odd,o);fmt(c,co);fmt(north,n);fmt(south,so);printf("QUAD_OUTPUT,%s,%s,%s,%s,%s\n",e,o,co,n,so); }
  else { char out[80];fmt(north,out);puts(out); }
}
