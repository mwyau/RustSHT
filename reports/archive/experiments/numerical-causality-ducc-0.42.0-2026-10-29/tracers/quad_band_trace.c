#include <quadmath.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void fmtq(__float128 x, char *out) { quadmath_snprintf(out,96,"%.36Qg",x); }
static uint64_t parse_hex(const char *text) { return strtoull(text,0,16); }
static float to_float(__float128 value) { volatile float result=(float)value; return result; }
static uint32_t fbits(float x) { uint32_t b; memcpy(&b,&x,4); return b; }

int main(int argc,char **argv) {
  if (argc!=3) return 2;
  const size_t lmax=720, start=600, band=121;
  const size_t rings[]={0,1,5,360,715,719,720};
  unsigned char coeff[16*band*8];
  FILE *cf=fopen(argv[1],"rb"); if(!cf || fread(coeff,1,sizeof(coeff),cf)!=sizeof(coeff)) return 3; fclose(cf);
  FILE *tf=fopen(argv[2],"r"); if(!tf) return 4;
  uint64_t theta_bits[721]; char line[64];
  for(size_t r=0;r<=lmax;r++) { if(!fgets(line,sizeof(line),tf)) return 5; theta_bits[r]=parse_hex(line); }
  fclose(tf);
  puts("QUAD128_DENSE_BAND_REFERENCE,1,GL_T720,orders=1..16,degrees=600..720,geometry=quad_sincos");
  for(size_t m=1;m<=16;m++) {
    __float128 eps[724]={0}, alpha[364]={0}, a[364]={0}, b[364]={0};
    __float128 pi=acosq(-1.Q), norm=1/sqrtq(4*pi);
    for(size_t j=1;j<=m;j++) norm*=sqrtq((__float128)(2*j+1)/(__float128)(2*j));
    for(size_t l=m+1;l<lmax+4;l++) eps[l]=sqrtq(((__float128)(l+m)*(l-m))/((__float128)(2*l+1)*(2*l-1)));
    alpha[0]=1/eps[m+1]; alpha[1]=eps[m+1]/(eps[m+2]*eps[m+3]);
    for(size_t il=1,l=m+2;l<lmax+1;il++,l+=2) alpha[il+1]=((il&1)?-1.Q:1.Q)/(eps[l+2]*eps[l+3]*alpha[il]);
    for(size_t il=0,l=m;l<lmax+2;il++,l+=2) { a[il]=((il&1)?-1.Q:1.Q)*alpha[il]*alpha[il]; b[il]=-a[il]*(eps[l+2]*eps[l+2]+eps[l+1]*eps[l+1]); }
    for(size_t rr=0;rr<7;rr++) {
      size_t r=rings[rr]; double td; memcpy(&td,&theta_bits[r],8); __float128 theta=(__float128)td;
      __float128 s=sinq(theta), c=cosq(theta), csq=fabsq(c)>.99Q?(1-s)*(1+s):c*c;
      __float128 prev=0, curr=((m&1)?-1.Q:1.Q)*norm*powq(s,(__float128)m);
      __float128 evenr=0, eveni=0, oddr=0, oddi=0;
      for(size_t pair=0,l=m;l<=lmax+1;pair++,l+=2) {
        __float128 xre=0,xim=0,x1re=0,x1im=0,x2re=0,x2im=0;
        if(l>=start && l<=lmax) { size_t off=((m-1)*band+(l-start))*8; float re,im; memcpy(&re,coeff+off,4); memcpy(&im,coeff+off+4,4); xre=re; xim=im; }
        if(l+1>=start && l+1<=lmax) { size_t off=((m-1)*band+(l+1-start))*8; float re,im; memcpy(&re,coeff+off,4); memcpy(&im,coeff+off+4,4); x1re=re; x1im=im; }
        if(l+2>=start && l+2<=lmax) { size_t off=((m-1)*band+(l+2-start))*8; float re,im; memcpy(&re,coeff+off,4); memcpy(&im,coeff+off+4,4); x2re=re; x2im=im; }
        __float128 cere=alpha[pair]*(eps[l+1]*xre+eps[l+2]*x2re);
        __float128 ceim=alpha[pair]*(eps[l+1]*xim+eps[l+2]*x2im);
        __float128 core=alpha[pair]*x1re, coim=alpha[pair]*x1im;
        if(r==0 && l<=lmax && l+2>=start) {
          char er[96],ei[96],orr[96],oi[96]; fmtq(cere,er);fmtq(ceim,ei);fmtq(core,orr);fmtq(coim,oi);
          printf("PAIR_STATE,%zu,%zu,%s,%s,%s,%s,%08x,%08x,%08x,%08x\n",
            m,l,er,ei,orr,oi,fbits(to_float(cere)),fbits(to_float(ceim)),fbits(to_float(core)),fbits(to_float(coim)));
        }
        evenr=fmaq(curr,cere,evenr); eveni=fmaq(curr,ceim,eveni);
        oddr=fmaq(curr,core,oddr); oddi=fmaq(curr,coim,oddi);
        if(l+2<=lmax+1) {
          __float128 lead=fmaq(a[pair],csq,b[pair]);
          __float128 next=fmaq(lead,curr,prev);
          char pv[96],cv[96],av[96],bv[96],gv[96],lv[96],nv[96];
          fmtq(prev,pv);fmtq(curr,cv);fmtq(a[pair],av);fmtq(b[pair],bv);fmtq(csq,gv);fmtq(lead,lv);fmtq(next,nv);
          printf("QUAD_REC,%zu,%zu,%zu,%s,%s,%s,%s,%s,%s,%s,%08x,%08x,%08x,%08x,%08x,%08x,%08x\n",
            m,r,l+2,pv,cv,av,bv,gv,lv,nv,fbits(to_float(prev)),fbits(to_float(curr)),
            fbits(to_float(a[pair])),fbits(to_float(b[pair])),fbits(to_float(csq)),
            fbits(to_float(lead)),fbits(to_float(next)));
          prev=curr; curr=next;
        }
      }
      __float128 nr=fmaq(oddr,c,evenr), ni=fmaq(oddi,c,eveni), sr=fmaq(-oddr,c,evenr), si=fmaq(-oddi,c,eveni);
      char q[8][96]; fmtq(c,q[0]);fmtq(evenr,q[1]);fmtq(oddr,q[2]);fmtq(nr,q[3]);fmtq(sr,q[4]);fmtq(eveni,q[5]);fmtq(oddi,q[6]);fmtq(ni,q[7]);
      float nrf=to_float(nr), nif=to_float(ni), srf=to_float(sr), sif=to_float(si);
      printf("QUAD_OUT,%zu,%zu,%016llx,c=%s,evenr=%s,oddr=%s,nr=%s,sr=%s,eveni=%s,oddi=%s,ni=%s,bits=%08x:%08x:%08x:%08x\n",
        m,r,(unsigned long long)theta_bits[r],q[0],q[1],q[2],q[3],q[4],q[5],q[6],q[7],fbits(nrf),fbits(nif),fbits(srf),fbits(sif));
    }
  }
}
