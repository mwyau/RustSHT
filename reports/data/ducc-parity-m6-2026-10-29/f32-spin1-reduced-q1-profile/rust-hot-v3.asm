
<experiment>:     file format elf64-x86-64


Disassembly of section .text:

00000000001f6740 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>>:
  1f6740:	push   %rbp
  1f6741:	push   %r15
  1f6743:	push   %r14
  1f6745:	push   %r13
  1f6747:	push   %r12
  1f6749:	push   %rbx
  1f674a:	sub    $0x98,%rsp
  1f6751:	mov    0xc0(%rdi),%r13
  1f6758:	mov    (%rdi),%rax
  1f675b:	mov    %rax,0x70(%rsp)
  1f6760:	mov    0x8(%rdi),%rbx
  1f6764:	mov    0x10(%rdi),%rbp
  1f6768:	mov    0x18(%rdi),%r9
  1f676c:	mov    0x20(%rdi),%r11
  1f6770:	mov    0x28(%rdi),%rax
  1f6774:	mov    0x30(%rdi),%r14
  1f6778:	mov    0x38(%rdi),%rcx
  1f677c:	mov    0x40(%rdi),%r12
  1f6780:	mov    0x48(%rdi),%rsi
  1f6784:	mov    0x50(%rdi),%rdx
  1f6788:	mov    0x58(%rdi),%r8
  1f678c:	mov    %rbx,0x60(%rsp)
  1f6791:	shr    $0x2,%rbx
  1f6795:	cmp    %r9,%rax
  1f6798:	cmovb  %rax,%r9
  1f679c:	mov    0x60(%rdi),%r10
  1f67a0:	cmp    %r9,%rcx
  1f67a3:	cmovb  %rcx,%r9
  1f67a7:	mov    0x68(%rdi),%rax
  1f67ab:	cmp    %r9,%rsi
  1f67ae:	cmovb  %rsi,%r9
  1f67b2:	mov    0x70(%rdi),%rcx
  1f67b6:	mov    %rcx,0x68(%rsp)
  1f67bb:	cmp    %r9,%r8
  1f67be:	cmovb  %r8,%r9
  1f67c2:	mov    0x78(%rdi),%rcx
  1f67c6:	mov    %rcx,0x58(%rsp)
  1f67cb:	cmp    %r9,%rax
  1f67ce:	cmovb  %rax,%r9
  1f67d2:	mov    %r9,0x90(%rsp)
  1f67da:	mov    0x80(%rdi),%rax
  1f67e1:	mov    %rax,0x50(%rsp)
  1f67e6:	mov    0x88(%rdi),%rax
  1f67ed:	mov    %rax,0x30(%rsp)
  1f67f2:	mov    0x90(%rdi),%rax
  1f67f9:	mov    %rax,0x48(%rsp)
  1f67fe:	mov    0x98(%rdi),%rax
  1f6805:	mov    %rax,0x28(%rsp)
  1f680a:	mov    0xa0(%rdi),%rax
  1f6811:	mov    %rax,0x40(%rsp)
  1f6816:	mov    0xa8(%rdi),%rax
  1f681d:	mov    %rax,0x20(%rsp)
  1f6822:	mov    0xb0(%rdi),%rax
  1f6829:	mov    %rax,0x38(%rsp)
  1f682e:	mov    0xb8(%rdi),%rax
  1f6835:	mov    %rax,0x18(%rsp)
  1f683a:	mov    %rbx,0x10(%rsp)
  1f683f:	test   %rbx,%rbx
  1f6842:	je     1f6a91 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x351>
  1f6848:	mov    0x58(%rsp),%rax
  1f684d:	shr    $0x2,%rax
  1f6851:	mov    %rax,0x8(%rsp)
  1f6856:	mov    0x30(%rsp),%rax
  1f685b:	shr    $0x2,%rax
  1f685f:	mov    %rax,0x88(%rsp)
  1f6867:	mov    0x28(%rsp),%rax
  1f686c:	shr    $0x2,%rax
  1f6870:	mov    %rax,0x80(%rsp)
  1f6878:	mov    0x20(%rsp),%rax
  1f687d:	shr    $0x2,%rax
  1f6881:	mov    %rax,0x78(%rsp)
  1f6886:	mov    0x18(%rsp),%rdi
  1f688b:	shr    $0x2,%rdi
  1f688f:	mov    0x90(%rsp),%r9
  1f6897:	test   %r9,%r9
  1f689a:	je     1f6a18 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x2d8>
  1f68a0:	neg    %r9
  1f68a3:	xor    %r15d,%r15d
  1f68a6:	cs nopw 0x0(%rax,%rax,1)
  1f68b0:	cmp    0x8(%rsp),%r15
  1f68b5:	je     1f70ae <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x96e>
  1f68bb:	lea    0x1(%r15),%rbx
  1f68bf:	mov    %r15,%rsi
  1f68c2:	shl    $0x5,%rsi
  1f68c6:	mov    0x70(%rsp),%rax
  1f68cb:	vmovupd (%rax,%rsi,1),%ymm3
  1f68d0:	mov    0x68(%rsp),%rax
  1f68d5:	vmovupd (%rax,%rsi,1),%ymm7
  1f68da:	vmulpd %ymm3,%ymm3,%ymm4
  1f68de:	vxorpd %xmm5,%xmm5,%xmm5
  1f68e2:	mov    $0x1,%eax
  1f68e7:	xor    %ecx,%ecx
  1f68e9:	vxorpd %xmm0,%xmm0,%xmm0
  1f68ed:	vxorpd %xmm1,%xmm1,%xmm1
  1f68f1:	vxorpd %xmm2,%xmm2,%xmm2
  1f68f5:	vxorpd %xmm6,%xmm6,%xmm6
  1f68f9:	jmp    1f698f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x24f>
  1f68fe:	xchg   %ax,%ax
  1f6900:	vbroadcastsd 0x0(%rbp,%rcx,1),%ymm9
  1f6907:	vbroadcastsd (%r11,%rcx,1),%ymm8
  1f690d:	vfmadd231pd %ymm9,%ymm4,%ymm8
  1f6912:	vfmadd213pd %ymm5,%ymm7,%ymm8
  1f6917:	vmovapd %ymm7,%ymm5
  1f691b:	vbroadcastsd (%r12,%rcx,2),%ymm9
  1f6921:	vbroadcastsd (%r14,%rcx,2),%ymm10
  1f6927:	vfmadd231pd %ymm9,%ymm3,%ymm10
  1f692c:	vbroadcastsd 0x8(%r12,%rcx,2),%ymm9
  1f6933:	vbroadcastsd 0x8(%r14,%rcx,2),%ymm11
  1f693a:	vfmadd231pd %ymm9,%ymm3,%ymm11
  1f693f:	vbroadcastsd (%r10,%rcx,2),%ymm9
  1f6945:	vbroadcastsd (%rdx,%rcx,2),%ymm12
  1f694b:	vfmadd231pd %ymm9,%ymm3,%ymm12
  1f6950:	vbroadcastsd 0x8(%r10,%rcx,2),%ymm9
  1f6957:	vbroadcastsd 0x8(%rdx,%rcx,2),%ymm13
  1f695e:	vfmadd231pd %ymm9,%ymm3,%ymm13
  1f6963:	vfmadd231pd %ymm10,%ymm7,%ymm6
  1f6968:	vfmadd231pd %ymm11,%ymm7,%ymm2
  1f696d:	vfmadd231pd %ymm12,%ymm7,%ymm1
  1f6972:	vfmadd231pd %ymm13,%ymm7,%ymm0
  1f6977:	add    $0x8,%rcx
  1f697b:	lea    (%r9,%rax,1),%r8
  1f697f:	inc    %r8
  1f6982:	inc    %rax
  1f6985:	vmovapd %ymm8,%ymm7
  1f6989:	cmp    $0x1,%r8
  1f698d:	je     1f69b0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x270>
  1f698f:	cmp    %r13,%rax
  1f6992:	jb     1f6900 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x1c0>
  1f6998:	vmovapd %ymm7,%ymm8
  1f699c:	jmp    1f691b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x1db>
  1f69a1:	data16 data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
  1f69b0:	cmp    0x88(%rsp),%r15
  1f69b8:	je     1f70c6 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x986>
  1f69be:	mov    0x50(%rsp),%rax
  1f69c3:	vmovupd %ymm6,(%rax,%rsi,1)
  1f69c8:	cmp    0x80(%rsp),%r15
  1f69d0:	je     1f70e1 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9a1>
  1f69d6:	mov    0x48(%rsp),%rax
  1f69db:	vmovupd %ymm2,(%rax,%rsi,1)
  1f69e0:	cmp    0x78(%rsp),%r15
  1f69e5:	je     1f70fc <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9bc>
  1f69eb:	mov    0x40(%rsp),%rax
  1f69f0:	vmovupd %ymm1,(%rax,%rsi,1)
  1f69f5:	cmp    %rdi,%r15
  1f69f8:	je     1f7114 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9d4>
  1f69fe:	mov    0x38(%rsp),%rax
  1f6a03:	vmovupd %ymm0,(%rax,%rsi,1)
  1f6a08:	mov    %rbx,%r15
  1f6a0b:	cmp    0x10(%rsp),%rbx
  1f6a10:	jne    1f68b0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x170>
  1f6a16:	jmp    1f6a91 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x351>
  1f6a18:	xor    %eax,%eax
  1f6a1a:	vxorpd %xmm0,%xmm0,%xmm0
  1f6a1e:	xor    %ecx,%ecx
  1f6a20:	cmp    %rcx,0x8(%rsp)
  1f6a25:	je     1f70ae <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x96e>
  1f6a2b:	cmp    %rcx,0x88(%rsp)
  1f6a33:	je     1f70c6 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x986>
  1f6a39:	mov    0x50(%rsp),%rsi
  1f6a3e:	vmovupd %ymm0,(%rsi,%rax,1)
  1f6a43:	cmp    %rcx,0x80(%rsp)
  1f6a4b:	je     1f70e1 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9a1>
  1f6a51:	mov    0x48(%rsp),%rsi
  1f6a56:	vmovupd %ymm0,(%rsi,%rax,1)
  1f6a5b:	cmp    %rcx,0x78(%rsp)
  1f6a60:	je     1f70fc <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9bc>
  1f6a66:	mov    0x40(%rsp),%rsi
  1f6a6b:	vmovupd %ymm0,(%rsi,%rax,1)
  1f6a70:	cmp    %rcx,%rdi
  1f6a73:	je     1f7114 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9d4>
  1f6a79:	inc    %rcx
  1f6a7c:	mov    0x38(%rsp),%rsi
  1f6a81:	vmovupd %ymm0,(%rsi,%rax,1)
  1f6a86:	add    $0x20,%rax
  1f6a8a:	cmp    %rcx,0x10(%rsp)
  1f6a8f:	jne    1f6a20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x2e0>
  1f6a91:	mov    0x60(%rsp),%rsi
  1f6a96:	and    $0x3,%esi
  1f6a99:	je     1f7099 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x959>
  1f6a9f:	movabs $0xffffffffffffffc,%r9
  1f6aa9:	mov    0x58(%rsp),%r8
  1f6aae:	and    $0x3,%r8d
  1f6ab2:	mov    0x30(%rsp),%rax
  1f6ab7:	mov    %eax,%ecx
  1f6ab9:	and    $0x3,%ecx
  1f6abc:	and    %r9,%rax
  1f6abf:	mov    %rax,0x30(%rsp)
  1f6ac4:	mov    0x28(%rsp),%rdi
  1f6ac9:	mov    %edi,%eax
  1f6acb:	and    $0x3,%eax
  1f6ace:	mov    %rax,0x10(%rsp)
  1f6ad3:	and    %r9,%rdi
  1f6ad6:	mov    %rdi,0x28(%rsp)
  1f6adb:	mov    0x20(%rsp),%rbx
  1f6ae0:	mov    %ebx,%edi
  1f6ae2:	and    $0x3,%edi
  1f6ae5:	and    %r9,%rbx
  1f6ae8:	mov    %rbx,0x20(%rsp)
  1f6aed:	mov    0x18(%rsp),%rbx
  1f6af2:	mov    %ebx,%eax
  1f6af4:	and    $0x3,%eax
  1f6af7:	mov    %rax,0x8(%rsp)
  1f6afc:	and    %r9,%rbx
  1f6aff:	mov    %rbx,0x18(%rsp)
  1f6b04:	mov    0x90(%rsp),%rax
  1f6b0c:	test   %rax,%rax
  1f6b0f:	je     1f6d54 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x614>
  1f6b15:	test   %r8,%r8
  1f6b18:	je     1f7127 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9e7>
  1f6b1e:	mov    0x60(%rsp),%rbx
  1f6b23:	and    %r9,%rbx
  1f6b26:	mov    0x58(%rsp),%r15
  1f6b2b:	and    %r9,%r15
  1f6b2e:	mov    0x70(%rsp),%r9
  1f6b33:	mov    %rbx,0x60(%rsp)
  1f6b38:	vmovsd (%r9,%rbx,8),%xmm2
  1f6b3e:	mov    0x68(%rsp),%r9
  1f6b43:	mov    %r15,0x58(%rsp)
  1f6b48:	vmovsd (%r9,%r15,8),%xmm7
  1f6b4e:	vmulsd %xmm2,%xmm2,%xmm3
  1f6b52:	neg    %rax
  1f6b55:	vxorpd %xmm5,%xmm5,%xmm5
  1f6b59:	mov    $0x1,%r9d
  1f6b5f:	xor    %ebx,%ebx
  1f6b61:	vxorpd %xmm0,%xmm0,%xmm0
  1f6b65:	vxorpd %xmm1,%xmm1,%xmm1
  1f6b69:	vxorpd %xmm4,%xmm4,%xmm4
  1f6b6d:	vxorpd %xmm6,%xmm6,%xmm6
  1f6b71:	jmp    1f6bf5 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x4b5>
  1f6b76:	cs nopw 0x0(%rax,%rax,1)
  1f6b80:	vmovsd 0x0(%rbp,%rbx,1),%xmm8
  1f6b86:	vfmadd213sd (%r11,%rbx,1),%xmm3,%xmm8
  1f6b8c:	vfmadd213sd %xmm5,%xmm7,%xmm8
  1f6b91:	vmovapd %xmm7,%xmm5
  1f6b95:	vmovsd (%r12,%rbx,2),%xmm9
  1f6b9b:	vmovsd 0x8(%r12,%rbx,2),%xmm10
  1f6ba2:	vfmadd213sd (%r14,%rbx,2),%xmm2,%xmm9
  1f6ba8:	vfmadd231sd %xmm9,%xmm7,%xmm6
  1f6bad:	vfmadd213sd 0x8(%r14,%rbx,2),%xmm2,%xmm10
  1f6bb4:	vfmadd231sd %xmm10,%xmm7,%xmm4
  1f6bb9:	vmovsd (%r10,%rbx,2),%xmm9
  1f6bbf:	vmovsd 0x8(%r10,%rbx,2),%xmm10
  1f6bc6:	vfmadd213sd (%rdx,%rbx,2),%xmm2,%xmm9
  1f6bcc:	vfmadd231sd %xmm9,%xmm7,%xmm1
  1f6bd1:	vfmadd213sd 0x8(%rdx,%rbx,2),%xmm2,%xmm10
  1f6bd8:	vfmadd231sd %xmm10,%xmm7,%xmm0
  1f6bdd:	add    $0x8,%rbx
  1f6be1:	lea    (%rax,%r9,1),%r15
  1f6be5:	inc    %r15
  1f6be8:	inc    %r9
  1f6beb:	vmovapd %xmm8,%xmm7
  1f6bef:	cmp    $0x1,%r15
  1f6bf3:	je     1f6c00 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x4c0>
  1f6bf5:	cmp    %r13,%r9
  1f6bf8:	jb     1f6b80 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x440>
  1f6bfa:	vmovapd %xmm7,%xmm8
  1f6bfe:	jmp    1f6b95 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x455>
  1f6c00:	test   %rcx,%rcx
  1f6c03:	je     1f713d <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9fd>
  1f6c09:	mov    0x50(%rsp),%r9
  1f6c0e:	mov    0x30(%rsp),%rbx
  1f6c13:	vmovsd %xmm6,(%r9,%rbx,8)
  1f6c19:	cmpq   $0x0,0x10(%rsp)
  1f6c1f:	je     1f7153 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa13>
  1f6c25:	mov    0x48(%rsp),%r9
  1f6c2a:	mov    0x28(%rsp),%rbx
  1f6c2f:	vmovsd %xmm4,(%r9,%rbx,8)
  1f6c35:	test   %rdi,%rdi
  1f6c38:	je     1f716b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa2b>
  1f6c3e:	mov    0x40(%rsp),%r9
  1f6c43:	mov    0x20(%rsp),%rbx
  1f6c48:	vmovsd %xmm1,(%r9,%rbx,8)
  1f6c4e:	cmpq   $0x0,0x8(%rsp)
  1f6c54:	je     1f717e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa3e>
  1f6c5a:	mov    0x38(%rsp),%r9
  1f6c5f:	mov    0x18(%rsp),%rbx
  1f6c64:	vmovsd %xmm0,(%r9,%rbx,8)
  1f6c6a:	cmp    $0x1,%esi
  1f6c6d:	je     1f7099 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x959>
  1f6c73:	cmp    $0x1,%r8d
  1f6c77:	je     1f7127 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9e7>
  1f6c7d:	mov    0x70(%rsp),%r9
  1f6c82:	mov    0x60(%rsp),%rbx
  1f6c87:	vmovsd 0x8(%r9,%rbx,8),%xmm3
  1f6c8e:	mov    0x68(%rsp),%r9
  1f6c93:	mov    0x58(%rsp),%rbx
  1f6c98:	vmovsd 0x8(%r9,%rbx,8),%xmm7
  1f6c9f:	vmulsd %xmm3,%xmm3,%xmm4
  1f6ca3:	vxorpd %xmm5,%xmm5,%xmm5
  1f6ca7:	mov    $0x1,%r9d
  1f6cad:	xor    %ebx,%ebx
  1f6caf:	vxorpd %xmm0,%xmm0,%xmm0
  1f6cb3:	vxorpd %xmm1,%xmm1,%xmm1
  1f6cb7:	vxorpd %xmm2,%xmm2,%xmm2
  1f6cbb:	vxorpd %xmm6,%xmm6,%xmm6
  1f6cbf:	jmp    1f6d49 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x609>
  1f6cc4:	data16 data16 cs nopw 0x0(%rax,%rax,1)
  1f6cd0:	vmovsd 0x0(%rbp,%rbx,1),%xmm8
  1f6cd6:	vfmadd213sd (%r11,%rbx,1),%xmm4,%xmm8
  1f6cdc:	vfmadd213sd %xmm5,%xmm7,%xmm8
  1f6ce1:	vmovapd %xmm7,%xmm5
  1f6ce5:	vmovsd (%r12,%rbx,2),%xmm9
  1f6ceb:	vmovsd 0x8(%r12,%rbx,2),%xmm10
  1f6cf2:	vfmadd213sd (%r14,%rbx,2),%xmm3,%xmm9
  1f6cf8:	vfmadd231sd %xmm9,%xmm7,%xmm6
  1f6cfd:	vfmadd213sd 0x8(%r14,%rbx,2),%xmm3,%xmm10
  1f6d04:	vfmadd231sd %xmm10,%xmm7,%xmm2
  1f6d09:	vmovsd (%r10,%rbx,2),%xmm9
  1f6d0f:	vmovsd 0x8(%r10,%rbx,2),%xmm10
  1f6d16:	vfmadd213sd (%rdx,%rbx,2),%xmm3,%xmm9
  1f6d1c:	vfmadd231sd %xmm9,%xmm7,%xmm1
  1f6d21:	vfmadd213sd 0x8(%rdx,%rbx,2),%xmm3,%xmm10
  1f6d28:	vfmadd231sd %xmm10,%xmm7,%xmm0
  1f6d2d:	add    $0x8,%rbx
  1f6d31:	lea    (%rax,%r9,1),%r15
  1f6d35:	inc    %r15
  1f6d38:	inc    %r9
  1f6d3b:	vmovapd %xmm8,%xmm7
  1f6d3f:	cmp    $0x1,%r15
  1f6d43:	je     1f6ee2 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x7a2>
  1f6d49:	cmp    %r13,%r9
  1f6d4c:	jb     1f6cd0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x590>
  1f6d4e:	vmovapd %xmm7,%xmm8
  1f6d52:	jmp    1f6ce5 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x5a5>
  1f6d54:	test   %r8,%r8
  1f6d57:	je     1f7127 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9e7>
  1f6d5d:	test   %rcx,%rcx
  1f6d60:	je     1f713d <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9fd>
  1f6d66:	mov    0x50(%rsp),%rdx
  1f6d6b:	mov    0x30(%rsp),%r9
  1f6d70:	movq   $0x0,(%rdx,%r9,8)
  1f6d78:	cmpq   $0x0,0x10(%rsp)
  1f6d7e:	je     1f7153 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa13>
  1f6d84:	mov    0x48(%rsp),%rdx
  1f6d89:	mov    0x28(%rsp),%r9
  1f6d8e:	movq   $0x0,(%rdx,%r9,8)
  1f6d96:	test   %rdi,%rdi
  1f6d99:	je     1f716b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa2b>
  1f6d9f:	mov    0x40(%rsp),%rdx
  1f6da4:	mov    0x20(%rsp),%r9
  1f6da9:	movq   $0x0,(%rdx,%r9,8)
  1f6db1:	cmpq   $0x0,0x8(%rsp)
  1f6db7:	je     1f717e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa3e>
  1f6dbd:	mov    0x38(%rsp),%rdx
  1f6dc2:	mov    0x18(%rsp),%r9
  1f6dc7:	movq   $0x0,(%rdx,%r9,8)
  1f6dcf:	cmp    $0x1,%esi
  1f6dd2:	je     1f7099 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x959>
  1f6dd8:	cmp    $0x1,%r8d
  1f6ddc:	je     1f7127 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9e7>
  1f6de2:	cmp    $0x1,%ecx
  1f6de5:	je     1f713d <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9fd>
  1f6deb:	mov    0x50(%rsp),%rdx
  1f6df0:	mov    0x30(%rsp),%r9
  1f6df5:	movq   $0x0,0x8(%rdx,%r9,8)
  1f6dfe:	cmpl   $0x1,0x10(%rsp)
  1f6e03:	je     1f7153 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa13>
  1f6e09:	mov    0x48(%rsp),%rdx
  1f6e0e:	mov    0x28(%rsp),%r9
  1f6e13:	movq   $0x0,0x8(%rdx,%r9,8)
  1f6e1c:	cmp    $0x1,%edi
  1f6e1f:	je     1f716b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa2b>
  1f6e25:	mov    0x40(%rsp),%rdx
  1f6e2a:	mov    0x20(%rsp),%r9
  1f6e2f:	movq   $0x0,0x8(%rdx,%r9,8)
  1f6e38:	cmpl   $0x1,0x8(%rsp)
  1f6e3d:	je     1f717e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa3e>
  1f6e43:	mov    0x38(%rsp),%rdx
  1f6e48:	mov    0x18(%rsp),%r9
  1f6e4d:	movq   $0x0,0x8(%rdx,%r9,8)
  1f6e56:	cmp    $0x2,%esi
  1f6e59:	je     1f7099 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x959>
  1f6e5f:	cmp    $0x2,%r8d
  1f6e63:	je     1f7127 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9e7>
  1f6e69:	cmp    $0x2,%ecx
  1f6e6c:	je     1f713d <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9fd>
  1f6e72:	mov    0x50(%rsp),%rcx
  1f6e77:	mov    0x30(%rsp),%rdx
  1f6e7c:	movq   $0x0,0x10(%rcx,%rdx,8)
  1f6e85:	cmpl   $0x2,0x10(%rsp)
  1f6e8a:	je     1f7153 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa13>
  1f6e90:	mov    0x48(%rsp),%rax
  1f6e95:	mov    0x28(%rsp),%rcx
  1f6e9a:	movq   $0x0,0x10(%rax,%rcx,8)
  1f6ea3:	cmp    $0x2,%edi
  1f6ea6:	je     1f716b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa2b>
  1f6eac:	mov    0x40(%rsp),%rax
  1f6eb1:	mov    0x20(%rsp),%rcx
  1f6eb6:	movq   $0x0,0x10(%rax,%rcx,8)
  1f6ebf:	cmpl   $0x2,0x8(%rsp)
  1f6ec4:	je     1f717e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa3e>
  1f6eca:	mov    0x38(%rsp),%rax
  1f6ecf:	mov    0x18(%rsp),%rcx
  1f6ed4:	movq   $0x0,0x10(%rax,%rcx,8)
  1f6edd:	jmp    1f7099 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x959>
  1f6ee2:	cmp    $0x1,%ecx
  1f6ee5:	je     1f713d <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9fd>
  1f6eeb:	mov    0x50(%rsp),%r9
  1f6ef0:	mov    0x30(%rsp),%rbx
  1f6ef5:	vmovsd %xmm6,0x8(%r9,%rbx,8)
  1f6efc:	cmpl   $0x1,0x10(%rsp)
  1f6f01:	je     1f7153 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa13>
  1f6f07:	mov    0x48(%rsp),%r9
  1f6f0c:	mov    0x28(%rsp),%rbx
  1f6f11:	vmovsd %xmm2,0x8(%r9,%rbx,8)
  1f6f18:	cmp    $0x1,%edi
  1f6f1b:	je     1f716b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa2b>
  1f6f21:	mov    0x40(%rsp),%r9
  1f6f26:	mov    0x20(%rsp),%rbx
  1f6f2b:	vmovsd %xmm1,0x8(%r9,%rbx,8)
  1f6f32:	cmpl   $0x1,0x8(%rsp)
  1f6f37:	je     1f717e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa3e>
  1f6f3d:	mov    0x38(%rsp),%r9
  1f6f42:	mov    0x18(%rsp),%rbx
  1f6f47:	vmovsd %xmm0,0x8(%r9,%rbx,8)
  1f6f4e:	cmp    $0x2,%esi
  1f6f51:	je     1f7099 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x959>
  1f6f57:	cmp    $0x2,%r8d
  1f6f5b:	je     1f7127 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9e7>
  1f6f61:	mov    0x70(%rsp),%rsi
  1f6f66:	mov    0x60(%rsp),%r8
  1f6f6b:	vmovsd 0x10(%rsi,%r8,8),%xmm3
  1f6f72:	mov    0x68(%rsp),%rsi
  1f6f77:	mov    0x58(%rsp),%r8
  1f6f7c:	vmovsd 0x10(%rsi,%r8,8),%xmm7
  1f6f83:	vmulsd %xmm3,%xmm3,%xmm4
  1f6f87:	vxorpd %xmm5,%xmm5,%xmm5
  1f6f8b:	mov    $0x1,%esi
  1f6f90:	xor    %r8d,%r8d
  1f6f93:	vxorpd %xmm0,%xmm0,%xmm0
  1f6f97:	vxorpd %xmm1,%xmm1,%xmm1
  1f6f9b:	vxorpd %xmm2,%xmm2,%xmm2
  1f6f9f:	vxorpd %xmm6,%xmm6,%xmm6
  1f6fa3:	jmp    1f7026 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x8e6>
  1f6fa8:	nopl   0x0(%rax,%rax,1)
  1f6fb0:	vmovsd 0x0(%rbp,%r8,1),%xmm8
  1f6fb7:	vfmadd213sd (%r11,%r8,1),%xmm4,%xmm8
  1f6fbd:	vfmadd213sd %xmm5,%xmm7,%xmm8
  1f6fc2:	vmovapd %xmm7,%xmm5
  1f6fc6:	vmovsd (%r12,%r8,2),%xmm9
  1f6fcc:	vmovsd 0x8(%r12,%r8,2),%xmm10
  1f6fd3:	vfmadd213sd (%r14,%r8,2),%xmm3,%xmm9
  1f6fd9:	vfmadd231sd %xmm9,%xmm7,%xmm6
  1f6fde:	vfmadd213sd 0x8(%r14,%r8,2),%xmm3,%xmm10
  1f6fe5:	vfmadd231sd %xmm10,%xmm7,%xmm2
  1f6fea:	vmovsd (%r10,%r8,2),%xmm9
  1f6ff0:	vmovsd 0x8(%r10,%r8,2),%xmm10
  1f6ff7:	vfmadd213sd (%rdx,%r8,2),%xmm3,%xmm9
  1f6ffd:	vfmadd231sd %xmm9,%xmm7,%xmm1
  1f7002:	vfmadd213sd 0x8(%rdx,%r8,2),%xmm3,%xmm10
  1f7009:	vfmadd231sd %xmm10,%xmm7,%xmm0
  1f700e:	add    $0x8,%r8
  1f7012:	lea    (%rax,%rsi,1),%r9
  1f7016:	inc    %r9
  1f7019:	inc    %rsi
  1f701c:	vmovapd %xmm8,%xmm7
  1f7020:	cmp    $0x1,%r9
  1f7024:	je     1f7031 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x8f1>
  1f7026:	cmp    %r13,%rsi
  1f7029:	jb     1f6fb0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x870>
  1f702b:	vmovapd %xmm7,%xmm8
  1f702f:	jmp    1f6fc6 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x886>
  1f7031:	cmp    $0x2,%ecx
  1f7034:	je     1f713d <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0x9fd>
  1f703a:	mov    0x50(%rsp),%rcx
  1f703f:	mov    0x30(%rsp),%rdx
  1f7044:	vmovsd %xmm6,0x10(%rcx,%rdx,8)
  1f704a:	cmpl   $0x2,0x10(%rsp)
  1f704f:	je     1f7153 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa13>
  1f7055:	mov    0x48(%rsp),%rax
  1f705a:	mov    0x28(%rsp),%rcx
  1f705f:	vmovsd %xmm2,0x10(%rax,%rcx,8)
  1f7065:	cmp    $0x2,%edi
  1f7068:	je     1f716b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa2b>
  1f706e:	mov    0x40(%rsp),%rax
  1f7073:	mov    0x20(%rsp),%rcx
  1f7078:	vmovsd %xmm1,0x10(%rax,%rcx,8)
  1f707e:	cmpl   $0x2,0x8(%rsp)
  1f7083:	je     1f717e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedPreparedSynthesisPairBlock>>+0xa3e>
  1f7089:	mov    0x38(%rsp),%rax
  1f708e:	mov    0x18(%rsp),%rcx
  1f7093:	vmovsd %xmm0,0x10(%rax,%rcx,8)
  1f7099:	add    $0x98,%rsp
  1f70a0:	pop    %rbx
  1f70a1:	pop    %r12
  1f70a3:	pop    %r13
  1f70a5:	pop    %r14
  1f70a7:	pop    %r15
  1f70a9:	pop    %rbp
  1f70aa:	vzeroupper
  1f70ad:	ret
  1f70ae:	lea    0x14f9fb(%rip),%rdx        # 346ab0 <anon.90c8f467974d2f2a528245fe12e87011.81.llvm.16679231169477627807+0x4b0>
  1f70b5:	mov    0x8(%rsp),%rdi
  1f70ba:	mov    %rdi,%rsi
  1f70bd:	vzeroupper
  1f70c0:	call   *0x15bf62(%rip)        # 353028 <_DYNAMIC+0x2b8>
  1f70c6:	lea    0x14f9fb(%rip),%rdx        # 346ac8 <anon.90c8f467974d2f2a528245fe12e87011.81.llvm.16679231169477627807+0x4c8>
  1f70cd:	mov    0x88(%rsp),%rdi
  1f70d5:	mov    %rdi,%rsi
  1f70d8:	vzeroupper
  1f70db:	call   *0x15bf47(%rip)        # 353028 <_DYNAMIC+0x2b8>
  1f70e1:	lea    0x14f9f8(%rip),%rdx        # 346ae0 <anon.90c8f467974d2f2a528245fe12e87011.81.llvm.16679231169477627807+0x4e0>
  1f70e8:	mov    0x80(%rsp),%rdi
  1f70f0:	mov    %rdi,%rsi
  1f70f3:	vzeroupper
  1f70f6:	call   *0x15bf2c(%rip)        # 353028 <_DYNAMIC+0x2b8>
  1f70fc:	lea    0x14f9f5(%rip),%rdx        # 346af8 <anon.90c8f467974d2f2a528245fe12e87011.81.llvm.16679231169477627807+0x4f8>
  1f7103:	mov    0x78(%rsp),%rdi
  1f7108:	mov    %rdi,%rsi
  1f710b:	vzeroupper
  1f710e:	call   *0x15bf14(%rip)        # 353028 <_DYNAMIC+0x2b8>
  1f7114:	lea    0x14f9f5(%rip),%rdx        # 346b10 <anon.90c8f467974d2f2a528245fe12e87011.81.llvm.16679231169477627807+0x510>
  1f711b:	mov    %rdi,%rsi
  1f711e:	vzeroupper
  1f7121:	call   *0x15bf01(%rip)        # 353028 <_DYNAMIC+0x2b8>
  1f7127:	lea    0x14f90a(%rip),%rdx        # 346a38 <anon.90c8f467974d2f2a528245fe12e87011.81.llvm.16679231169477627807+0x438>
  1f712e:	mov    %r8,%rdi
  1f7131:	mov    %r8,%rsi
  1f7134:	vzeroupper
  1f7137:	call   *0x15beeb(%rip)        # 353028 <_DYNAMIC+0x2b8>
  1f713d:	lea    0x14f90c(%rip),%rdx        # 346a50 <anon.90c8f467974d2f2a528245fe12e87011.81.llvm.16679231169477627807+0x450>
  1f7144:	mov    %rcx,%rdi
  1f7147:	mov    %rcx,%rsi
  1f714a:	vzeroupper
  1f714d:	call   *0x15bed5(%rip)        # 353028 <_DYNAMIC+0x2b8>
  1f7153:	lea    0x14f90e(%rip),%rdx        # 346a68 <anon.90c8f467974d2f2a528245fe12e87011.81.llvm.16679231169477627807+0x468>
  1f715a:	mov    0x10(%rsp),%rdi
  1f715f:	mov    %rdi,%rsi
  1f7162:	vzeroupper
  1f7165:	call   *0x15bebd(%rip)        # 353028 <_DYNAMIC+0x2b8>
  1f716b:	lea    0x14f90e(%rip),%rdx        # 346a80 <anon.90c8f467974d2f2a528245fe12e87011.81.llvm.16679231169477627807+0x480>
  1f7172:	mov    %rdi,%rsi
  1f7175:	vzeroupper
  1f7178:	call   *0x15beaa(%rip)        # 353028 <_DYNAMIC+0x2b8>
  1f717e:	lea    0x14f913(%rip),%rdx        # 346a98 <anon.90c8f467974d2f2a528245fe12e87011.81.llvm.16679231169477627807+0x498>
  1f7185:	mov    0x8(%rsp),%rdi
  1f718a:	mov    %rdi,%rsi
  1f718d:	vzeroupper
  1f7190:	call   *0x15be92(%rip)        # 353028 <_DYNAMIC+0x2b8>
  1f7196:	int3
  1f7197:	int3
  1f7198:	int3
  1f7199:	int3
  1f719a:	int3
  1f719b:	int3
  1f719c:	int3
  1f719d:	int3
  1f719e:	int3
  1f719f:	int3
