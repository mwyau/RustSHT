
<experiment>:     file format elf64-x86-64


Disassembly of section .text:

000000000067cb80 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]>:
  67cb80:	push   %rbp
  67cb81:	mov    %rcx,%r11
  67cb84:	mov    %rdx,%r10
  67cb87:	lea    0x2(%r11),%rax
  67cb8b:	mov    %rsp,%rbp
  67cb8e:	push   %r13
  67cb90:	push   %r12
  67cb92:	mov    %r8,%r12
  67cb95:	push   %rbx
  67cb96:	mov    %rsi,%rbx
  67cb99:	and    $0xffffffffffffffe0,%rsp
  67cb9d:	mov    0x10(%rbp),%rcx
  67cba1:	cmp    %rax,%r9
  67cba4:	jb     67cd4d <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x1cd>
  67cbaa:	test   %rcx,%rcx
  67cbad:	je     67cd1c <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x19c>
  67cbb3:	mov    %r8,%r13
  67cbb6:	mov    %r11,%rdx
  67cbb9:	shl    $0x4,%r13
  67cbbd:	shl    $0x4,%rdx
  67cbc1:	add    %rsi,%r13
  67cbc4:	mov    %rcx,%rsi
  67cbc7:	add    %r10,%rdx
  67cbca:	shl    $0x5,%rsi
  67cbce:	add    %rdi,%rsi
  67cbd1:	nopl   0x0(%rax)
  67cbd5:	data16 cs nopw 0x0(%rax,%rax,1)
  67cbe0:	vbroadcastsd 0x8(%r13),%ymm2
  67cbe6:	vbroadcastsd (%rdx),%ymm15
  67cbeb:	mov    %rdi,%rax
  67cbee:	vbroadcastsd 0x8(%rdx),%ymm14
  67cbf4:	vbroadcastsd 0x10(%rdx),%ymm13
  67cbfa:	vmovapd %ymm2,-0x20(%rsp)
  67cc00:	vbroadcastsd 0x10(%r13),%ymm2
  67cc06:	vbroadcastsd 0x18(%rdx),%ymm12
  67cc0c:	vbroadcastsd 0x20(%rdx),%ymm11
  67cc12:	vbroadcastsd 0x28(%rdx),%ymm10
  67cc18:	vmovapd %ymm2,-0x40(%rsp)
  67cc1e:	vbroadcastsd 0x18(%r13),%ymm2
  67cc24:	vbroadcastsd 0x30(%rdx),%ymm9
  67cc2a:	vbroadcastsd 0x38(%rdx),%ymm8
  67cc30:	vbroadcastsd 0x0(%r13),%ymm7
  67cc36:	vmovapd %ymm2,-0x60(%rsp)
  67cc3c:	nopl   0x0(%rax)
  67cc40:	vmovapd %ymm13,%ymm4
  67cc44:	vmovapd %ymm15,%ymm6
  67cc48:	vmovapd %ymm14,%ymm5
  67cc4c:	add    $0x20,%rax
  67cc50:	vmovapd 0x27e0(%rax),%ymm2
  67cc58:	vmovapd -0x20(%rsp),%ymm0
  67cc5e:	vmovapd %ymm12,%ymm3
  67cc62:	vmovapd 0x1fe0(%rax),%ymm1
  67cc6a:	vfmadd213pd 0x3fe0(%rax),%ymm1,%ymm4
  67cc73:	vfmadd213pd 0x2fe0(%rax),%ymm1,%ymm6
  67cc7c:	vfmadd231pd %ymm2,%ymm7,%ymm0
  67cc81:	vfmadd213pd 0x17e0(%rax),%ymm1,%ymm0
  67cc8a:	vfmadd213pd 0x37e0(%rax),%ymm1,%ymm5
  67cc93:	vfmadd213pd 0x47e0(%rax),%ymm1,%ymm3
  67cc9c:	vfmadd231pd %ymm0,%ymm9,%ymm4
  67cca1:	vfmadd231pd %ymm0,%ymm11,%ymm6
  67cca6:	vmovapd %ymm0,0x17e0(%rax)
  67ccae:	vfmadd231pd %ymm0,%ymm10,%ymm5
  67ccb3:	vfmadd231pd %ymm0,%ymm8,%ymm3
  67ccb8:	vmovapd %ymm4,0x3fe0(%rax)
  67ccc0:	vmovapd -0x60(%rsp),%ymm4
  67ccc6:	vfmadd132pd -0x40(%rsp),%ymm4,%ymm2
  67cccd:	vmovapd %ymm6,0x2fe0(%rax)
  67ccd5:	vmovapd %ymm5,0x37e0(%rax)
  67ccdd:	vmovapd %ymm3,0x47e0(%rax)
  67cce5:	vfmadd132pd %ymm2,%ymm1,%ymm0
  67ccea:	vmovapd %ymm0,0x1fe0(%rax)
  67ccf2:	cmp    %rax,%rsi
  67ccf5:	jne    67cc40 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0xc0>
  67ccfb:	lea    0x4(%r11),%r8
  67ccff:	add    $0x6,%r11
  67cd03:	add    $0x2,%r12
  67cd07:	add    $0x40,%rdx
  67cd0b:	add    $0x20,%r13
  67cd0f:	cmp    %r11,%r9
  67cd12:	jb     67cd50 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x1d0>
  67cd14:	mov    %r8,%r11
  67cd17:	jmp    67cbe0 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x60>
  67cd1c:	lea    0x6(%r11),%rax
  67cd20:	cmp    %rax,%r9
  67cd23:	jb     67cd42 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x1c2>
  67cd25:	lea    0x8(%r11),%r8
  67cd29:	add    $0xa,%r11
  67cd2d:	add    $0x4,%r12
  67cd31:	cmp    %r11,%r9
  67cd34:	jb     67cd50 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x1d0>
  67cd36:	mov    %r8,%r11
  67cd39:	lea    0x6(%r11),%rax
  67cd3d:	cmp    %rax,%r9
  67cd40:	jae    67cd25 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x1a5>
  67cd42:	lea    -0x18(%rbp),%rsp
  67cd46:	pop    %rbx
  67cd47:	pop    %r12
  67cd49:	pop    %r13
  67cd4b:	pop    %rbp
  67cd4c:	ret
  67cd4d:	mov    %r11,%r8
  67cd50:	cmp    %r8,%r9
  67cd53:	jb     67ce54 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x2d4>
  67cd59:	test   %rcx,%rcx
  67cd5c:	je     67ce54 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x2d4>
  67cd62:	shl    $0x4,%r12
  67cd66:	mov    %r8,%rax
  67cd69:	shl    $0x5,%rcx
  67cd6d:	shl    $0x4,%rax
  67cd71:	add    %r12,%rbx
  67cd74:	lea    (%rdi,%rcx,1),%rdx
  67cd78:	lea    (%r10,%rax,1),%rsi
  67cd7c:	nopl   0x0(%rax)
  67cd80:	vbroadcastsd (%rsi),%ymm7
  67cd85:	vbroadcastsd 0x8(%rsi),%ymm6
  67cd8b:	mov    %rdi,%rax
  67cd8e:	vbroadcastsd 0x10(%rsi),%ymm5
  67cd94:	vbroadcastsd 0x18(%rsi),%ymm4
  67cd9a:	vbroadcastsd (%rbx),%ymm3
  67cd9f:	vbroadcastsd 0x8(%rbx),%ymm2
  67cda5:	data16 cs nopw 0x0(%rax,%rax,1)
  67cdb0:	vmovapd 0x2000(%rax),%ymm0
  67cdb8:	vmovapd %ymm7,%ymm1
  67cdbc:	add    $0x20,%rax
  67cdc0:	vfmadd213pd 0x2fe0(%rax),%ymm0,%ymm1
  67cdc9:	vmovapd %ymm1,0x2fe0(%rax)
  67cdd1:	vmovapd %ymm6,%ymm1
  67cdd5:	vfmadd213pd 0x37e0(%rax),%ymm0,%ymm1
  67cdde:	vmovapd %ymm1,0x37e0(%rax)
  67cde6:	vmovapd %ymm5,%ymm1
  67cdea:	vfmadd213pd 0x3fe0(%rax),%ymm0,%ymm1
  67cdf3:	vmovapd %ymm1,0x3fe0(%rax)
  67cdfb:	vmovapd %ymm4,%ymm1
  67cdff:	vfmadd213pd 0x47e0(%rax),%ymm0,%ymm1
  67ce08:	vmovapd %ymm1,0x47e0(%rax)
  67ce10:	vmovapd %ymm3,%ymm1
  67ce14:	vfmadd132pd 0x27e0(%rax),%ymm2,%ymm1
  67ce1d:	vfmadd213pd 0x17e0(%rax),%ymm0,%ymm1
  67ce26:	vmovapd %ymm0,0x17e0(%rax)
  67ce2e:	vmovapd %ymm1,0x1fe0(%rax)
  67ce36:	cmp    %rax,%rdx
  67ce39:	jne    67cdb0 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x230>
  67ce3f:	add    $0x2,%r8
  67ce43:	add    $0x20,%rsi
  67ce47:	add    $0x10,%rbx
  67ce4b:	cmp    %r8,%r9
  67ce4e:	jae    67cd80 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x200>
  67ce54:	vzeroupper
  67ce57:	lea    -0x18(%rbp),%rsp
  67ce5b:	pop    %rbx
  67ce5c:	pop    %r12
  67ce5e:	pop    %r13
  67ce60:	pop    %rbp
  67ce61:	ret
  67ce62:	nopl   (%rax)
  67ce65:	data16 cs nopw 0x0(%rax,%rax,1)
