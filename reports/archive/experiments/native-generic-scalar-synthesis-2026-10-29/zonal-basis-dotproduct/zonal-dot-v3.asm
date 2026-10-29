
candidate ELF: file format elf64-x86-64


Disassembly of section .text:

00000000001f1860 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>>:
  1f1860:	push   %rbp
  1f1861:	push   %r15
  1f1863:	push   %r14
  1f1865:	push   %r13
  1f1867:	push   %r12
  1f1869:	push   %rbx
  1f186a:	sub    $0x468,%rsp
  1f1871:	mov    (%rdi),%rax
  1f1874:	mov    %rax,0x60(%rsp)
  1f1879:	mov    0x8(%rdi),%rax
  1f187d:	mov    %rax,0x28(%rsp)
  1f1882:	mov    0x40(%rdi),%rax
  1f1886:	mov    %rax,0x20(%rsp)
  1f188b:	mov    0x48(%rdi),%rax
  1f188f:	mov    %rax,0x10(%rsp)
  1f1894:	mov    0x50(%rdi),%rbp
  1f1898:	mov    0x58(%rdi),%rax
  1f189c:	mov    %rax,0x48(%rsp)
  1f18a1:	mov    0x60(%rdi),%rbx
  1f18a5:	mov    0x10(%rdi),%rax
  1f18a9:	mov    %rax,0x40(%rsp)
  1f18ae:	mov    0x18(%rdi),%r14
  1f18b2:	mov    0x20(%rdi),%rax
  1f18b6:	mov    %rax,0x38(%rsp)
  1f18bb:	mov    0x28(%rdi),%r15
  1f18bf:	mov    0x30(%rdi),%rax
  1f18c3:	mov    %rax,0x8(%rsp)
  1f18c8:	mov    0x38(%rdi),%rax
  1f18cc:	mov    %rax,(%rsp)
  1f18d0:	lea    0x68(%rsp),%rdi
  1f18d5:	mov    $0x400,%edx
  1f18da:	xor    %esi,%esi
  1f18dc:	call   *0xb6d8e(%rip)        # 2a8670 <memset@GLIBC_2.2.5>
  1f18e2:	test   %rbx,%rbx
  1f18e5:	je     1f1f34 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6d4>
  1f18eb:	incq   0x20(%rsp)
  1f18f0:	jne    1f1b5d <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x2fd>
  1f18f6:	xor    %edx,%edx
  1f18f8:	mov    $0x8,%esi
  1f18fd:	mov    0xb74ec(%rip),%r8        # 2a8df0 <_DYNAMIC+0xa00>
  1f1904:	lea    0x68(%rsp),%rax
  1f1909:	vpxor  %xmm2,%xmm2,%xmm2
  1f190d:	jmp    1f191c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xbc>
  1f190f:	nop
  1f1910:	add    %r9,%rdx
  1f1913:	cmp    %rbx,%rdx
  1f1916:	jae    1f1f34 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6d4>
  1f191c:	mov    %rbx,%r9
  1f191f:	sub    %rdx,%r9
  1f1922:	cmp    $0x8,%r9
  1f1926:	cmovae %rsi,%r9
  1f192a:	mov    0x808(%r8),%rdi
  1f1931:	vxorps %xmm0,%xmm0,%xmm0
  1f1935:	lea    0x2(%rip),%rcx        # 1f193e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xde>
  1f193c:	jmp    *%rdi
  1f193e:	mov    %rbx,%rcx
  1f1941:	sub    %rdx,%rcx
  1f1944:	je     1f1910 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xb0>
  1f1946:	vmovd  0x68(%rsp),%xmm0
  1f194c:	vmovd  %xmm0,%edi
  1f1950:	add    %edi,%edi
  1f1952:	cmp    $0xfeffffff,%edi
  1f1958:	ja     1f197e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x11e>
  1f195a:	mov    %rdx,%rdi
  1f195d:	imul   0x10(%rsp),%rdi
  1f1963:	cmp    (%rsp),%rdi
  1f1967:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f196d:	vpblendd $0x1,%xmm0,%xmm2,%xmm0
  1f1973:	mov    0x8(%rsp),%r10
  1f1978:	vmovq  %xmm0,(%r10,%rdi,8)
  1f197e:	cmp    $0x1,%rcx
  1f1982:	je     1f1910 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xb0>
  1f1984:	vmovd  0x6c(%rsp),%xmm0
  1f198a:	vmovd  %xmm0,%edi
  1f198e:	add    %edi,%edi
  1f1990:	cmp    $0xfeffffff,%edi
  1f1996:	ja     1f19bd <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x15d>
  1f1998:	lea    0x1(%rdx),%rdi
  1f199c:	imul   0x10(%rsp),%rdi
  1f19a2:	cmp    (%rsp),%rdi
  1f19a6:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f19ac:	vpblendd $0x1,%xmm0,%xmm2,%xmm0
  1f19b2:	mov    0x8(%rsp),%r10
  1f19b7:	vmovq  %xmm0,(%r10,%rdi,8)
  1f19bd:	cmp    $0x2,%rcx
  1f19c1:	je     1f1910 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xb0>
  1f19c7:	vmovd  0x70(%rsp),%xmm0
  1f19cd:	vmovd  %xmm0,%edi
  1f19d1:	add    %edi,%edi
  1f19d3:	cmp    $0xfeffffff,%edi
  1f19d9:	ja     1f1a00 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x1a0>
  1f19db:	lea    0x2(%rdx),%rdi
  1f19df:	imul   0x10(%rsp),%rdi
  1f19e5:	cmp    (%rsp),%rdi
  1f19e9:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f19ef:	vpblendd $0x1,%xmm0,%xmm2,%xmm0
  1f19f5:	mov    0x8(%rsp),%r10
  1f19fa:	vmovq  %xmm0,(%r10,%rdi,8)
  1f1a00:	cmp    $0x3,%rcx
  1f1a04:	je     1f1910 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xb0>
  1f1a0a:	vmovd  0x74(%rsp),%xmm0
  1f1a10:	vmovd  %xmm0,%edi
  1f1a14:	add    %edi,%edi
  1f1a16:	cmp    $0xfeffffff,%edi
  1f1a1c:	ja     1f1a43 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x1e3>
  1f1a1e:	lea    0x3(%rdx),%rdi
  1f1a22:	imul   0x10(%rsp),%rdi
  1f1a28:	cmp    (%rsp),%rdi
  1f1a2c:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1a32:	vpblendd $0x1,%xmm0,%xmm2,%xmm0
  1f1a38:	mov    0x8(%rsp),%r10
  1f1a3d:	vmovq  %xmm0,(%r10,%rdi,8)
  1f1a43:	cmp    $0x4,%rcx
  1f1a47:	je     1f1910 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xb0>
  1f1a4d:	vmovd  0x78(%rsp),%xmm0
  1f1a53:	vmovd  %xmm0,%edi
  1f1a57:	add    %edi,%edi
  1f1a59:	cmp    $0xfeffffff,%edi
  1f1a5f:	ja     1f1a86 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x226>
  1f1a61:	lea    0x4(%rdx),%rdi
  1f1a65:	imul   0x10(%rsp),%rdi
  1f1a6b:	cmp    (%rsp),%rdi
  1f1a6f:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1a75:	vpblendd $0x1,%xmm0,%xmm2,%xmm0
  1f1a7b:	mov    0x8(%rsp),%r10
  1f1a80:	vmovq  %xmm0,(%r10,%rdi,8)
  1f1a86:	cmp    $0x5,%rcx
  1f1a8a:	je     1f1910 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xb0>
  1f1a90:	vmovd  0x7c(%rsp),%xmm0
  1f1a96:	vmovd  %xmm0,%edi
  1f1a9a:	add    %edi,%edi
  1f1a9c:	cmp    $0xfeffffff,%edi
  1f1aa2:	ja     1f1ac9 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x269>
  1f1aa4:	lea    0x5(%rdx),%rdi
  1f1aa8:	imul   0x10(%rsp),%rdi
  1f1aae:	cmp    (%rsp),%rdi
  1f1ab2:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1ab8:	vpblendd $0x1,%xmm0,%xmm2,%xmm0
  1f1abe:	mov    0x8(%rsp),%r10
  1f1ac3:	vmovq  %xmm0,(%r10,%rdi,8)
  1f1ac9:	cmp    $0x6,%rcx
  1f1acd:	je     1f1910 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xb0>
  1f1ad3:	vmovd  0x80(%rsp),%xmm0
  1f1adc:	vmovd  %xmm0,%edi
  1f1ae0:	add    %edi,%edi
  1f1ae2:	cmp    $0xfeffffff,%edi
  1f1ae8:	ja     1f1b0f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x2af>
  1f1aea:	lea    0x6(%rdx),%rdi
  1f1aee:	imul   0x10(%rsp),%rdi
  1f1af4:	cmp    (%rsp),%rdi
  1f1af8:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1afe:	vpblendd $0x1,%xmm0,%xmm2,%xmm0
  1f1b04:	mov    0x8(%rsp),%r10
  1f1b09:	vmovq  %xmm0,(%r10,%rdi,8)
  1f1b0f:	cmp    $0x7,%rcx
  1f1b13:	je     1f1910 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xb0>
  1f1b19:	vmovd  0x84(%rsp),%xmm0
  1f1b22:	vmovd  %xmm0,%ecx
  1f1b26:	add    %ecx,%ecx
  1f1b28:	cmp    $0xfeffffff,%ecx
  1f1b2e:	ja     1f1910 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xb0>
  1f1b34:	lea    0x7(%rdx),%rdi
  1f1b38:	imul   0x10(%rsp),%rdi
  1f1b3e:	cmp    (%rsp),%rdi
  1f1b42:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1b48:	vpblendd $0x1,%xmm0,%xmm2,%xmm0
  1f1b4e:	mov    0x8(%rsp),%rcx
  1f1b53:	vmovq  %xmm0,(%rcx,%rdi,8)
  1f1b58:	jmp    1f1910 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0xb0>
  1f1b5d:	lea    0x0(,%rbp,4),%r9
  1f1b65:	shlq   $0x3,0x28(%rsp)
  1f1b6b:	xor    %ecx,%ecx
  1f1b6d:	mov    $0x8,%r11d
  1f1b73:	mov    0xb7276(%rip),%rdx        # 2a8df0 <_DYNAMIC+0xa00>
  1f1b7a:	vpxor  %xmm3,%xmm3,%xmm3
  1f1b7e:	vxorps %xmm4,%xmm4,%xmm4
  1f1b82:	mov    %rbx,0x30(%rsp)
  1f1b87:	jmp    1f1bae <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x34e>
  1f1b89:	vpblendd $0x1,%xmm0,%xmm3,%xmm0
  1f1b8f:	mov    0x8(%rsp),%rax
  1f1b94:	vmovq  %xmm0,(%rax,%rdi,8)
  1f1b99:	nopl   0x0(%rax)
  1f1ba0:	add    0x50(%rsp),%rcx
  1f1ba5:	cmp    %rbx,%rcx
  1f1ba8:	jae    1f1f34 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6d4>
  1f1bae:	sub    %rcx,%rbx
  1f1bb1:	cmp    $0x8,%rbx
  1f1bb5:	mov    $0x8,%eax
  1f1bba:	mov    %rbx,0x18(%rsp)
  1f1bbf:	cmovb  %rbx,%rax
  1f1bc3:	mov    %rax,0x50(%rsp)
  1f1bc8:	mov    0x48(%rsp),%rax
  1f1bcd:	mov    %rcx,0x58(%rsp)
  1f1bd2:	lea    (%rcx,%rax,1),%r13
  1f1bd6:	mov    %r13,%rdi
  1f1bd9:	neg    %rdi
  1f1bdc:	mov    0x38(%rsp),%rax
  1f1be1:	lea    (%rax,%r13,4),%rsi
  1f1be5:	mov    0x40(%rsp),%rax
  1f1bea:	lea    (%rax,%r13,4),%r8
  1f1bee:	xor    %r10d,%r10d
  1f1bf1:	mov    0x20(%rsp),%rbx
  1f1bf6:	vxorps %xmm5,%xmm5,%xmm5
  1f1bfa:	vxorps %xmm6,%xmm6,%xmm6
  1f1bfe:	xchg   %ax,%ax
  1f1c00:	cmp    %r10,0x28(%rsp)
  1f1c05:	je     1f1cf3 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x493>
  1f1c0b:	cmp    %r14,%r13
  1f1c0e:	ja     1f1f49 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6e9>
  1f1c14:	lea    (%r14,%rdi,1),%rax
  1f1c18:	cmp    $0x8,%rax
  1f1c1c:	cmovae %r11,%rax
  1f1c20:	shl    $0x8,%eax
  1f1c23:	mov    (%rdx,%rax,1),%r12
  1f1c27:	mov    %r8,%rax
  1f1c2a:	lea    0x3(%rip),%rcx        # 1f1c34 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x3d4>
  1f1c31:	jmp    *%r12
  1f1c34:	cmp    %r15,%r13
  1f1c37:	ja     1f1f62 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x702>
  1f1c3d:	vmovaps %ymm0,%ymm2
  1f1c41:	dec    %rbx
  1f1c44:	lea    (%r15,%rdi,1),%rax
  1f1c48:	cmp    $0x8,%rax
  1f1c4c:	cmovae %r11,%rax
  1f1c50:	shl    $0x8,%eax
  1f1c53:	mov    (%rdx,%rax,1),%r12
  1f1c57:	mov    %rsi,%rax
  1f1c5a:	lea    0x3(%rip),%rcx        # 1f1c64 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x404>
  1f1c61:	jmp    *%r12
  1f1c64:	mov    0x60(%rsp),%rax
  1f1c69:	vbroadcastss (%rax,%r10,1),%ymm1
  1f1c6f:	vmulps %ymm1,%ymm2,%ymm7
  1f1c73:	vmulps %ymm1,%ymm0,%ymm0
  1f1c77:	vfmsub213ps %ymm7,%ymm2,%ymm1
  1f1c7c:	vmulps %ymm4,%ymm2,%ymm2
  1f1c80:	vaddps %ymm0,%ymm2,%ymm0
  1f1c84:	vaddps %ymm0,%ymm1,%ymm0
  1f1c88:	vaddps %ymm0,%ymm7,%ymm1
  1f1c8c:	vsubps %ymm7,%ymm1,%ymm2
  1f1c90:	vsubps %ymm2,%ymm1,%ymm8
  1f1c94:	vsubps %ymm8,%ymm7,%ymm7
  1f1c99:	vsubps %ymm2,%ymm0,%ymm0
  1f1c9d:	vaddps %ymm7,%ymm0,%ymm0
  1f1ca1:	vaddps %ymm1,%ymm5,%ymm2
  1f1ca5:	vsubps %ymm5,%ymm2,%ymm7
  1f1ca9:	vsubps %ymm7,%ymm2,%ymm8
  1f1cad:	vsubps %ymm8,%ymm5,%ymm5
  1f1cb2:	vsubps %ymm7,%ymm1,%ymm1
  1f1cb6:	vaddps %ymm5,%ymm1,%ymm1
  1f1cba:	vaddps %ymm0,%ymm6,%ymm0
  1f1cbe:	vaddps %ymm0,%ymm1,%ymm0
  1f1cc2:	vaddps %ymm0,%ymm2,%ymm5
  1f1cc6:	vsubps %ymm2,%ymm5,%ymm1
  1f1cca:	vsubps %ymm1,%ymm5,%ymm6
  1f1cce:	vsubps %ymm6,%ymm2,%ymm2
  1f1cd2:	vsubps %ymm1,%ymm0,%ymm0
  1f1cd6:	vaddps %ymm2,%ymm0,%ymm6
  1f1cda:	add    %rbp,%r13
  1f1cdd:	sub    %rbp,%rdi
  1f1ce0:	add    %r9,%rsi
  1f1ce3:	add    %r9,%r8
  1f1ce6:	add    $0x8,%r10
  1f1cea:	test   %rbx,%rbx
  1f1ced:	jne    1f1c00 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x3a0>
  1f1cf3:	vaddps %ymm5,%ymm6,%ymm0
  1f1cf7:	mov    0x808(%rdx),%rsi
  1f1cfe:	lea    0x68(%rsp),%rax
  1f1d03:	lea    0x2(%rip),%rcx        # 1f1d0c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x4ac>
  1f1d0a:	jmp    *%rsi
  1f1d0c:	mov    0x30(%rsp),%rbx
  1f1d11:	mov    0x58(%rsp),%rcx
  1f1d16:	cmp    %rcx,%rbx
  1f1d19:	je     1f1ba0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x340>
  1f1d1f:	vmovd  0x68(%rsp),%xmm0
  1f1d25:	vmovd  %xmm0,%eax
  1f1d29:	add    %eax,%eax
  1f1d2b:	cmp    $0xfeffffff,%eax
  1f1d30:	ja     1f1d55 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x4f5>
  1f1d32:	mov    %rcx,%rdi
  1f1d35:	imul   0x10(%rsp),%rdi
  1f1d3b:	cmp    (%rsp),%rdi
  1f1d3f:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1d45:	vpblendd $0x1,%xmm0,%xmm3,%xmm0
  1f1d4b:	mov    0x8(%rsp),%rax
  1f1d50:	vmovq  %xmm0,(%rax,%rdi,8)
  1f1d55:	cmpq   $0x1,0x18(%rsp)
  1f1d5b:	je     1f1ba0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x340>
  1f1d61:	vmovd  0x6c(%rsp),%xmm0
  1f1d67:	vmovd  %xmm0,%eax
  1f1d6b:	add    %eax,%eax
  1f1d6d:	cmp    $0xfeffffff,%eax
  1f1d72:	ja     1f1d98 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x538>
  1f1d74:	lea    0x1(%rcx),%rdi
  1f1d78:	imul   0x10(%rsp),%rdi
  1f1d7e:	cmp    (%rsp),%rdi
  1f1d82:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1d88:	vpblendd $0x1,%xmm0,%xmm3,%xmm0
  1f1d8e:	mov    0x8(%rsp),%rax
  1f1d93:	vmovq  %xmm0,(%rax,%rdi,8)
  1f1d98:	cmpq   $0x2,0x18(%rsp)
  1f1d9e:	je     1f1ba0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x340>
  1f1da4:	vmovd  0x70(%rsp),%xmm0
  1f1daa:	vmovd  %xmm0,%eax
  1f1dae:	add    %eax,%eax
  1f1db0:	cmp    $0xfeffffff,%eax
  1f1db5:	ja     1f1ddb <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x57b>
  1f1db7:	lea    0x2(%rcx),%rdi
  1f1dbb:	imul   0x10(%rsp),%rdi
  1f1dc1:	cmp    (%rsp),%rdi
  1f1dc5:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1dcb:	vpblendd $0x1,%xmm0,%xmm3,%xmm0
  1f1dd1:	mov    0x8(%rsp),%rax
  1f1dd6:	vmovq  %xmm0,(%rax,%rdi,8)
  1f1ddb:	cmpq   $0x3,0x18(%rsp)
  1f1de1:	je     1f1ba0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x340>
  1f1de7:	vmovd  0x74(%rsp),%xmm0
  1f1ded:	vmovd  %xmm0,%eax
  1f1df1:	add    %eax,%eax
  1f1df3:	cmp    $0xfeffffff,%eax
  1f1df8:	ja     1f1e1e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x5be>
  1f1dfa:	lea    0x3(%rcx),%rdi
  1f1dfe:	imul   0x10(%rsp),%rdi
  1f1e04:	cmp    (%rsp),%rdi
  1f1e08:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1e0e:	vpblendd $0x1,%xmm0,%xmm3,%xmm0
  1f1e14:	mov    0x8(%rsp),%rax
  1f1e19:	vmovq  %xmm0,(%rax,%rdi,8)
  1f1e1e:	cmpq   $0x4,0x18(%rsp)
  1f1e24:	je     1f1ba0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x340>
  1f1e2a:	vmovd  0x78(%rsp),%xmm0
  1f1e30:	vmovd  %xmm0,%eax
  1f1e34:	add    %eax,%eax
  1f1e36:	cmp    $0xfeffffff,%eax
  1f1e3b:	ja     1f1e61 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x601>
  1f1e3d:	lea    0x4(%rcx),%rdi
  1f1e41:	imul   0x10(%rsp),%rdi
  1f1e47:	cmp    (%rsp),%rdi
  1f1e4b:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1e51:	vpblendd $0x1,%xmm0,%xmm3,%xmm0
  1f1e57:	mov    0x8(%rsp),%rax
  1f1e5c:	vmovq  %xmm0,(%rax,%rdi,8)
  1f1e61:	cmpq   $0x5,0x18(%rsp)
  1f1e67:	je     1f1ba0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x340>
  1f1e6d:	vmovd  0x7c(%rsp),%xmm0
  1f1e73:	vmovd  %xmm0,%eax
  1f1e77:	add    %eax,%eax
  1f1e79:	cmp    $0xfeffffff,%eax
  1f1e7e:	ja     1f1ea4 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x644>
  1f1e80:	lea    0x5(%rcx),%rdi
  1f1e84:	imul   0x10(%rsp),%rdi
  1f1e8a:	cmp    (%rsp),%rdi
  1f1e8e:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1e94:	vpblendd $0x1,%xmm0,%xmm3,%xmm0
  1f1e9a:	mov    0x8(%rsp),%rax
  1f1e9f:	vmovq  %xmm0,(%rax,%rdi,8)
  1f1ea4:	cmpq   $0x6,0x18(%rsp)
  1f1eaa:	je     1f1ba0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x340>
  1f1eb0:	vmovd  0x80(%rsp),%xmm0
  1f1eb9:	vmovd  %xmm0,%eax
  1f1ebd:	add    %eax,%eax
  1f1ebf:	cmp    $0xfeffffff,%eax
  1f1ec4:	ja     1f1ee6 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x686>
  1f1ec6:	lea    0x6(%rcx),%rdi
  1f1eca:	imul   0x10(%rsp),%rdi
  1f1ed0:	cmp    (%rsp),%rdi
  1f1ed4:	jae    1f1f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x6c0>
  1f1ed6:	vpblendd $0x1,%xmm0,%xmm3,%xmm0
  1f1edc:	mov    0x8(%rsp),%rax
  1f1ee1:	vmovq  %xmm0,(%rax,%rdi,8)
  1f1ee6:	cmpq   $0x7,0x18(%rsp)
  1f1eec:	je     1f1ba0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x340>
  1f1ef2:	vmovd  0x84(%rsp),%xmm0
  1f1efb:	vmovd  %xmm0,%eax
  1f1eff:	add    %eax,%eax
  1f1f01:	cmp    $0xfeffffff,%eax
  1f1f06:	ja     1f1ba0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x340>
  1f1f0c:	lea    0x7(%rcx),%rdi
  1f1f10:	imul   0x10(%rsp),%rdi
  1f1f16:	cmp    (%rsp),%rdi
  1f1f1a:	jb     1f1b89 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_ordered::RefineZonalSpectrumF32>>+0x329>
  1f1f20:	lea    0xad961(%rip),%rdx        # 29f888 <anon.f9c2054a2fea40374d729b63dc314184.8.llvm.12431860669452260197+0x378>
  1f1f27:	mov    (%rsp),%rsi
  1f1f2b:	vzeroupper
  1f1f2e:	call   *0xb671c(%rip)        # 2a8650 <_DYNAMIC+0x260>
  1f1f34:	add    $0x468,%rsp
  1f1f3b:	pop    %rbx
  1f1f3c:	pop    %r12
  1f1f3e:	pop    %r13
  1f1f40:	pop    %r14
  1f1f42:	pop    %r15
  1f1f44:	pop    %rbp
  1f1f45:	vzeroupper
  1f1f48:	ret
  1f1f49:	lea    0xad920(%rip),%rcx        # 29f870 <anon.f9c2054a2fea40374d729b63dc314184.8.llvm.12431860669452260197+0x360>
  1f1f50:	mov    %r13,%rdi
  1f1f53:	mov    %r14,%rsi
  1f1f56:	mov    %r14,%rdx
  1f1f59:	vzeroupper
  1f1f5c:	call   *0xb673e(%rip)        # 2a86a0 <_DYNAMIC+0x2b0>
  1f1f62:	lea    0xad8ef(%rip),%rcx        # 29f858 <anon.f9c2054a2fea40374d729b63dc314184.8.llvm.12431860669452260197+0x348>
  1f1f69:	mov    %r13,%rdi
  1f1f6c:	mov    %r15,%rsi
  1f1f6f:	mov    %r15,%rdx
  1f1f72:	vzeroupper
  1f1f75:	call   *0xb6725(%rip)        # 2a86a0 <_DYNAMIC+0x2b0>
  1f1f7b:	int3
  1f1f7c:	int3
  1f1f7d:	int3
  1f1f7e:	int3
  1f1f7f:	int3
