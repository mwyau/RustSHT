
<experiment-scratch>/target-candidate/release/examples/ducc_f32_full_transform_parity:     file format elf64-x86-64


Disassembly of section .text:

0000000000179bf0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>>:
  179bf0:	55                   	push   %rbp
  179bf1:	41 57                	push   %r15
  179bf3:	41 56                	push   %r14
  179bf5:	41 55                	push   %r13
  179bf7:	41 54                	push   %r12
  179bf9:	53                   	push   %rbx
  179bfa:	48 81 ec a8 00 00 00 	sub    $0xa8,%rsp
  179c01:	49 bf f8 ff ff ff ff 	movabs $0x1ffffffffffffff8,%r15
  179c08:	ff ff 1f
  179c0b:	4c 8b 67 08          	mov    0x8(%rdi),%r12
  179c0f:	4c 8b b7 90 00 00 00 	mov    0x90(%rdi),%r14
  179c16:	48 8b 97 a0 00 00 00 	mov    0xa0(%rdi),%rdx
  179c1d:	45 89 e3             	mov    %r12d,%r11d
  179c20:	41 83 e3 07          	and    $0x7,%r11d
  179c24:	b8 04 00 00 00       	mov    $0x4,%eax
  179c29:	48 89 04 24          	mov    %rax,(%rsp)
  179c2d:	4d 85 f6             	test   %r14,%r14
  179c30:	74 48                	je     179c7a <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x8a>
  179c32:	48 8b b7 98 00 00 00 	mov    0x98(%rdi),%rsi
  179c39:	48 89 f1             	mov    %rsi,%rcx
  179c3c:	48 c1 e9 03          	shr    $0x3,%rcx
  179c40:	89 f0                	mov    %esi,%eax
  179c42:	83 e0 07             	and    $0x7,%eax
  179c45:	4c 21 fe             	and    %r15,%rsi
  179c48:	4d 8d 14 b6          	lea    (%r14,%rsi,4),%r10
  179c4c:	4c 89 e3             	mov    %r12,%rbx
  179c4f:	48 c1 eb 03          	shr    $0x3,%rbx
  179c53:	48 85 d2             	test   %rdx,%rdx
  179c56:	74 3e                	je     179c96 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa6>
  179c58:	4c 8b 8f a8 00 00 00 	mov    0xa8(%rdi),%r9
  179c5f:	4c 89 ce             	mov    %r9,%rsi
  179c62:	48 c1 ee 03          	shr    $0x3,%rsi
  179c66:	45 89 c8             	mov    %r9d,%r8d
  179c69:	41 83 e0 07          	and    $0x7,%r8d
  179c6d:	4d 21 f9             	and    %r15,%r9
  179c70:	4e 8d 0c 8a          	lea    (%rdx,%r9,4),%r9
  179c74:	48 89 14 24          	mov    %rdx,(%rsp)
  179c78:	eb 27                	jmp    179ca1 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xb1>
  179c7a:	31 c0                	xor    %eax,%eax
  179c7c:	41 ba 04 00 00 00    	mov    $0x4,%r10d
  179c82:	31 c9                	xor    %ecx,%ecx
  179c84:	41 be 04 00 00 00    	mov    $0x4,%r14d
  179c8a:	4c 89 e3             	mov    %r12,%rbx
  179c8d:	48 c1 eb 03          	shr    $0x3,%rbx
  179c91:	48 85 d2             	test   %rdx,%rdx
  179c94:	75 c2                	jne    179c58 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x68>
  179c96:	31 f6                	xor    %esi,%esi
  179c98:	41 b9 04 00 00 00    	mov    $0x4,%r9d
  179c9e:	45 31 c0             	xor    %r8d,%r8d
  179ca1:	48 31 d9             	xor    %rbx,%rcx
  179ca4:	48 31 de             	xor    %rbx,%rsi
  179ca7:	48 09 ce             	or     %rcx,%rsi
  179caa:	4c 31 d8             	xor    %r11,%rax
  179cad:	48 09 f0             	or     %rsi,%rax
  179cb0:	4d 31 d8             	xor    %r11,%r8
  179cb3:	49 09 c0             	or     %rax,%r8
  179cb6:	74 17                	je     179ccf <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xdf>
  179cb8:	31 c0                	xor    %eax,%eax
  179cba:	48 81 c4 a8 00 00 00 	add    $0xa8,%rsp
  179cc1:	5b                   	pop    %rbx
  179cc2:	41 5c                	pop    %r12
  179cc4:	41 5d                	pop    %r13
  179cc6:	41 5e                	pop    %r14
  179cc8:	41 5f                	pop    %r15
  179cca:	5d                   	pop    %rbp
  179ccb:	c5 f8 77             	vzeroupper
  179cce:	c3                   	ret
  179ccf:	4c 89 a4 24 90 00 00 	mov    %r12,0x90(%rsp)
  179cd6:	00
  179cd7:	4c 89 4c 24 08       	mov    %r9,0x8(%rsp)
  179cdc:	4c 89 9c 24 98 00 00 	mov    %r11,0x98(%rsp)
  179ce3:	00
  179ce4:	4c 89 54 24 30       	mov    %r10,0x30(%rsp)
  179ce9:	48 8b 47 58          	mov    0x58(%rdi),%rax
  179ced:	48 89 44 24 48       	mov    %rax,0x48(%rsp)
  179cf2:	48 8b 47 68          	mov    0x68(%rdi),%rax
  179cf6:	48 89 44 24 38       	mov    %rax,0x38(%rsp)
  179cfb:	48 8b 47 78          	mov    0x78(%rdi),%rax
  179cff:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
  179d04:	48 8b 07             	mov    (%rdi),%rax
  179d07:	48 89 44 24 58       	mov    %rax,0x58(%rsp)
  179d0c:	48 8b 47 50          	mov    0x50(%rdi),%rax
  179d10:	48 89 44 24 50       	mov    %rax,0x50(%rsp)
  179d15:	48 8b 47 60          	mov    0x60(%rdi),%rax
  179d19:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
  179d1e:	48 8b 47 70          	mov    0x70(%rdi),%rax
  179d22:	48 89 44 24 10       	mov    %rax,0x10(%rsp)
  179d27:	4c 8b bf 88 00 00 00 	mov    0x88(%rdi),%r15
  179d2e:	48 8b 47 10          	mov    0x10(%rdi),%rax
  179d32:	48 89 44 24 68       	mov    %rax,0x68(%rsp)
  179d37:	48 8b 47 18          	mov    0x18(%rdi),%rax
  179d3b:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
  179d40:	48 8b 47 20          	mov    0x20(%rdi),%rax
  179d44:	48 89 44 24 60       	mov    %rax,0x60(%rsp)
  179d49:	48 8b 47 28          	mov    0x28(%rdi),%rax
  179d4d:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
  179d52:	48 8b 6f 30          	mov    0x30(%rdi),%rbp
  179d56:	48 89 f8             	mov    %rdi,%rax
  179d59:	48 8b 7f 38          	mov    0x38(%rdi),%rdi
  179d5d:	4c 2b b8 80 00 00 00 	sub    0x80(%rax),%r15
  179d64:	48 8b 50 40          	mov    0x40(%rax),%rdx
  179d68:	48 8b 48 48          	mov    0x48(%rax),%rcx
  179d6c:	49 83 c7 02          	add    $0x2,%r15
  179d70:	49 d1 ef             	shr    $1,%r15
  179d73:	48 85 db             	test   %rbx,%rbx
  179d76:	0f 84 91 02 00 00    	je     17a00d <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x41d>
  179d7c:	48 8b 44 24 48       	mov    0x48(%rsp),%rax
  179d81:	48 c1 e8 03          	shr    $0x3,%rax
  179d85:	4c 8b 5c 24 38       	mov    0x38(%rsp),%r11
  179d8a:	49 c1 eb 03          	shr    $0x3,%r11
  179d8e:	4c 8b 44 24 40       	mov    0x40(%rsp),%r8
  179d93:	49 c1 e8 03          	shr    $0x3,%r8
  179d97:	4d 85 ff             	test   %r15,%r15
  179d9a:	0f 84 0e 02 00 00    	je     179fae <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x3be>
  179da0:	45 31 e4             	xor    %r12d,%r12d
  179da3:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  179da7:	49 39 c4             	cmp    %rax,%r12
  179daa:	0f 84 ba 07 00 00    	je     17a56a <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x97a>
  179db0:	49 8d 74 24 01       	lea    0x1(%r12),%rsi
  179db5:	48 89 b4 24 a0 00 00 	mov    %rsi,0xa0(%rsp)
  179dbc:	00
  179dbd:	4d 89 e2             	mov    %r12,%r10
  179dc0:	49 c1 e2 05          	shl    $0x5,%r10
  179dc4:	48 8b 74 24 58       	mov    0x58(%rsp),%rsi
  179dc9:	c4 21 7c 10 04 16    	vmovups (%rsi,%r10,1),%ymm8
  179dcf:	48 8b 74 24 50       	mov    0x50(%rsp),%rsi
  179dd4:	c4 21 7c 10 1c 16    	vmovups (%rsi,%r10,1),%ymm11
  179dda:	c4 c1 3c 59 c8       	vmulps %ymm8,%ymm8,%ymm1
  179ddf:	c5 fc 11 4c 24 70    	vmovups %ymm1,0x70(%rsp)
  179de5:	c4 41 18 57 e4       	vxorps %xmm12,%xmm12,%xmm12
  179dea:	45 31 c9             	xor    %r9d,%r9d
  179ded:	c5 d0 57 ed          	vxorps %xmm5,%xmm5,%xmm5
  179df1:	c5 c8 57 f6          	vxorps %xmm6,%xmm6,%xmm6
  179df5:	c5 c0 57 ff          	vxorps %xmm7,%xmm7,%xmm7
  179df9:	c4 41 28 57 d2       	vxorps %xmm10,%xmm10,%xmm10
  179dfe:	eb 58                	jmp    179e58 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x268>
  179e00:	c4 41 7c 28 fb       	vmovaps %ymm11,%ymm15
  179e05:	c5 f8 28 d4          	vmovaps %xmm4,%xmm2
  179e09:	c4 c2 3d a8 e6       	vfmadd213ps %ymm14,%ymm8,%ymm4
  179e0e:	c5 f8 28 d9          	vmovaps %xmm1,%xmm3
  179e12:	c4 c2 3d a8 cd       	vfmadd213ps %ymm13,%ymm8,%ymm1
  179e17:	c4 62 25 b8 d4       	vfmadd231ps %ymm4,%ymm11,%ymm10
  179e1c:	c4 e2 25 b8 f9       	vfmadd231ps %ymm1,%ymm11,%ymm7
  179e21:	c5 fa 5c ca          	vsubss %xmm2,%xmm0,%xmm1
  179e25:	c4 e2 7d 18 c9       	vbroadcastss %xmm1,%ymm1
  179e2a:	c4 c2 3d a8 ce       	vfmadd213ps %ymm14,%ymm8,%ymm1
  179e2f:	c5 fa 5c d3          	vsubss %xmm3,%xmm0,%xmm2
  179e33:	c4 e2 7d 18 d2       	vbroadcastss %xmm2,%ymm2
  179e38:	c4 c2 3d a8 d5       	vfmadd213ps %ymm13,%ymm8,%ymm2
  179e3d:	c4 e2 25 b8 f1       	vfmadd231ps %ymm1,%ymm11,%ymm6
  179e42:	c4 e2 25 b8 ea       	vfmadd231ps %ymm2,%ymm11,%ymm5
  179e47:	49 89 f1             	mov    %rsi,%r9
  179e4a:	c4 41 7c 28 df       	vmovaps %ymm15,%ymm11
  179e4f:	49 39 f7             	cmp    %rsi,%r15
  179e52:	0f 84 08 01 00 00    	je     179f60 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x370>
  179e58:	4c 39 cf             	cmp    %r9,%rdi
  179e5b:	0f 84 87 06 00 00    	je     17a4e8 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x8f8>
  179e61:	4c 39 c9             	cmp    %r9,%rcx
  179e64:	0f 84 91 06 00 00    	je     17a4fb <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x90b>
  179e6a:	49 8d 71 01          	lea    0x1(%r9),%rsi
  179e6e:	c4 a2 7d 18 24 ca    	vbroadcastss (%rdx,%r9,8),%ymm4
  179e74:	c4 22 7d 18 74 cd 00 	vbroadcastss 0x0(%rbp,%r9,8),%ymm14
  179e7b:	c4 a2 7d 18 4c ca 04 	vbroadcastss 0x4(%rdx,%r9,8),%ymm1
  179e82:	c4 22 7d 18 6c cd 04 	vbroadcastss 0x4(%rbp,%r9,8),%ymm13
  179e89:	4c 39 fe             	cmp    %r15,%rsi
  179e8c:	0f 83 6e ff ff ff    	jae    179e00 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x210>
  179e92:	4c 3b 4c 24 28       	cmp    0x28(%rsp),%r9
  179e97:	0f 83 8c 06 00 00    	jae    17a529 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x939>
  179e9d:	4c 3b 4c 24 20       	cmp    0x20(%rsp),%r9
  179ea2:	0f 83 69 06 00 00    	jae    17a511 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x921>
  179ea8:	4c 8b 6c 24 68       	mov    0x68(%rsp),%r13
  179ead:	c4 82 7d 18 54 8d 00 	vbroadcastss 0x0(%r13,%r9,4),%ymm2
  179eb4:	4c 8b 6c 24 60       	mov    0x60(%rsp),%r13
  179eb9:	c4 02 7d 18 7c 8d 00 	vbroadcastss 0x0(%r13,%r9,4),%ymm15
  179ec0:	c4 62 6d b8 7c 24 70 	vfmadd231ps 0x70(%rsp),%ymm2,%ymm15
  179ec7:	c4 42 25 a8 fc       	vfmadd213ps %ymm12,%ymm11,%ymm15
  179ecc:	c4 e2 7d 18 15 db 62 	vbroadcastss -0x159d25(%rip),%ymm2        # 201b0 <__abi_tag+0x1feb4>
  179ed3:	ea ff
  179ed5:	c5 24 54 e2          	vandps %ymm2,%ymm11,%ymm12
  179ed9:	c5 84 54 d2          	vandps %ymm2,%ymm15,%ymm2
  179edd:	c4 e2 7d 18 1d 9a 60 	vbroadcastss -0x159f66(%rip),%ymm3        # 1ff80 <__abi_tag+0x1fc84>
  179ee4:	ea ff
  179ee6:	c5 9c c2 db 12       	vcmple_oqps %ymm3,%ymm12,%ymm3
  179eeb:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  179eef:	c5 1c c2 c8 00       	vcmpeqps %ymm0,%ymm12,%ymm9
  179ef4:	c4 e2 7d 18 05 93 62 	vbroadcastss -0x159d6d(%rip),%ymm0        # 20190 <__abi_tag+0x1fe94>
  179efb:	ea ff
  179efd:	c5 1c c2 e0 1d       	vcmpge_oqps %ymm0,%ymm12,%ymm12
  179f02:	c4 41 1c 56 c9       	vorps  %ymm9,%ymm12,%ymm9
  179f07:	c5 b4 54 db          	vandps %ymm3,%ymm9,%ymm3
  179f0b:	c4 62 7d 18 0d 6c 60 	vbroadcastss -0x159f94(%rip),%ymm9        # 1ff80 <__abi_tag+0x1fc84>
  179f12:	ea ff
  179f14:	c4 41 6c c2 c9 12    	vcmple_oqps %ymm9,%ymm2,%ymm9
  179f1a:	c5 b4 54 db          	vandps %ymm3,%ymm9,%ymm3
  179f1e:	c4 41 30 57 c9       	vxorps %xmm9,%xmm9,%xmm9
  179f23:	c5 34 c2 ca 00       	vcmpeqps %ymm2,%ymm9,%ymm9
  179f28:	c5 ec c2 d0 1d       	vcmpge_oqps %ymm0,%ymm2,%ymm2
  179f2d:	c5 b4 56 d2          	vorps  %ymm2,%ymm9,%ymm2
  179f31:	c5 e4 54 d2          	vandps %ymm2,%ymm3,%ymm2
  179f35:	c5 fd 76 c0          	vpcmpeqd %ymm0,%ymm0,%ymm0
  179f39:	c4 e2 7d 17 d0       	vptest %ymm0,%ymm2
  179f3e:	c4 41 7c 28 e3       	vmovaps %ymm11,%ymm12
  179f43:	c5 f9 ef c0          	vpxor  %xmm0,%xmm0,%xmm0
  179f47:	0f 82 b8 fe ff ff    	jb     179e05 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x215>
  179f4d:	e9 66 fd ff ff       	jmp    179cb8 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xc8>
  179f52:	66 66 66 66 66 2e 0f 	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
  179f59:	1f 84 00 00 00 00 00
  179f60:	4d 39 dc             	cmp    %r11,%r12
  179f63:	0f 84 17 06 00 00    	je     17a580 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x990>
  179f69:	48 8b 74 24 18       	mov    0x18(%rsp),%rsi
  179f6e:	c4 21 7c 11 14 16    	vmovups %ymm10,(%rsi,%r10,1)
  179f74:	4d 39 c4             	cmp    %r8,%r12
  179f77:	0f 84 19 06 00 00    	je     17a596 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9a6>
  179f7d:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
  179f82:	c4 a1 7c 11 3c 16    	vmovups %ymm7,(%rsi,%r10,1)
  179f88:	c4 81 7c 11 34 16    	vmovups %ymm6,(%r14,%r10,1)
  179f8e:	48 8b 34 24          	mov    (%rsp),%rsi
  179f92:	c4 a1 7c 11 2c 16    	vmovups %ymm5,(%rsi,%r10,1)
  179f98:	48 8b b4 24 a0 00 00 	mov    0xa0(%rsp),%rsi
  179f9f:	00
  179fa0:	49 89 f4             	mov    %rsi,%r12
  179fa3:	48 39 de             	cmp    %rbx,%rsi
  179fa6:	0f 85 fb fd ff ff    	jne    179da7 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x1b7>
  179fac:	eb 5f                	jmp    17a00d <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x41d>
  179fae:	31 f6                	xor    %esi,%esi
  179fb0:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  179fb4:	45 31 c9             	xor    %r9d,%r9d
  179fb7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  179fbe:	00 00
  179fc0:	4c 39 c8             	cmp    %r9,%rax
  179fc3:	0f 84 a1 05 00 00    	je     17a56a <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x97a>
  179fc9:	4d 39 cb             	cmp    %r9,%r11
  179fcc:	0f 84 ae 05 00 00    	je     17a580 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x990>
  179fd2:	4c 8b 54 24 18       	mov    0x18(%rsp),%r10
  179fd7:	c4 c1 7c 11 04 32    	vmovups %ymm0,(%r10,%rsi,1)
  179fdd:	4d 39 c8             	cmp    %r9,%r8
  179fe0:	0f 84 b0 05 00 00    	je     17a596 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9a6>
  179fe6:	49 ff c1             	inc    %r9
  179fe9:	4c 8b 54 24 10       	mov    0x10(%rsp),%r10
  179fee:	c4 c1 7c 11 04 32    	vmovups %ymm0,(%r10,%rsi,1)
  179ff4:	c4 c1 7c 11 04 36    	vmovups %ymm0,(%r14,%rsi,1)
  179ffa:	4c 8b 14 24          	mov    (%rsp),%r10
  179ffe:	c4 c1 7c 11 04 32    	vmovups %ymm0,(%r10,%rsi,1)
  17a004:	48 83 c6 20          	add    $0x20,%rsi
  17a008:	4c 39 cb             	cmp    %r9,%rbx
  17a00b:	75 b3                	jne    179fc0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x3d0>
  17a00d:	b0 01                	mov    $0x1,%al
  17a00f:	4c 8b a4 24 98 00 00 	mov    0x98(%rsp),%r12
  17a016:	00
  17a017:	4d 85 e4             	test   %r12,%r12
  17a01a:	0f 84 8a 02 00 00    	je     17a2aa <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x6ba>
  17a020:	48 8b 5c 24 48       	mov    0x48(%rsp),%rbx
  17a025:	41 89 d9             	mov    %ebx,%r9d
  17a028:	41 83 e1 07          	and    $0x7,%r9d
  17a02c:	4c 8b 54 24 38       	mov    0x38(%rsp),%r10
  17a031:	45 89 d0             	mov    %r10d,%r8d
  17a034:	41 83 e0 07          	and    $0x7,%r8d
  17a038:	48 be f8 ff ff ff ff 	movabs $0x1ffffffffffffff8,%rsi
  17a03f:	ff ff 1f
  17a042:	49 21 f2             	and    %rsi,%r10
  17a045:	4c 8b 74 24 40       	mov    0x40(%rsp),%r14
  17a04a:	45 89 f3             	mov    %r14d,%r11d
  17a04d:	41 83 e3 07          	and    $0x7,%r11d
  17a051:	49 21 f6             	and    %rsi,%r14
  17a054:	4c 8b 6c 24 18       	mov    0x18(%rsp),%r13
  17a059:	4e 8d 14 95 00 00 00 	lea    0x0(,%r10,4),%r10
  17a060:	00
  17a061:	4d 01 ea             	add    %r13,%r10
  17a064:	4c 89 54 24 70       	mov    %r10,0x70(%rsp)
  17a069:	4c 8b 54 24 10       	mov    0x10(%rsp),%r10
  17a06e:	4f 8d 14 b2          	lea    (%r10,%r14,4),%r10
  17a072:	4c 89 14 24          	mov    %r10,(%rsp)
  17a076:	4d 85 ff             	test   %r15,%r15
  17a079:	0f 84 32 02 00 00    	je     17a2b1 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x6c1>
  17a07f:	48 8b 84 24 90 00 00 	mov    0x90(%rsp),%rax
  17a086:	00
  17a087:	48 21 f0             	and    %rsi,%rax
  17a08a:	48 21 f3             	and    %rsi,%rbx
  17a08d:	48 8b 74 24 58       	mov    0x58(%rsp),%rsi
  17a092:	48 8d 04 86          	lea    (%rsi,%rax,4),%rax
  17a096:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
  17a09b:	48 8b 44 24 50       	mov    0x50(%rsp),%rax
  17a0a0:	48 8d 04 98          	lea    (%rax,%rbx,4),%rax
  17a0a4:	48 89 44 24 10       	mov    %rax,0x10(%rsp)
  17a0a9:	31 db                	xor    %ebx,%ebx
  17a0ab:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  17a0af:	48 8b 74 24 30       	mov    0x30(%rsp),%rsi
  17a0b4:	4c 39 cb             	cmp    %r9,%rbx
  17a0b7:	0f 84 1f 05 00 00    	je     17a5dc <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9ec>
  17a0bd:	4c 8d 73 01          	lea    0x1(%rbx),%r14
  17a0c1:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
  17a0c6:	c5 fa 10 0c 98       	vmovss (%rax,%rbx,4),%xmm1
  17a0cb:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
  17a0d0:	c5 fa 10 2c 98       	vmovss (%rax,%rbx,4),%xmm5
  17a0d5:	c5 f2 59 e1          	vmulss %xmm1,%xmm1,%xmm4
  17a0d9:	c4 41 18 57 e4       	vxorps %xmm12,%xmm12,%xmm12
  17a0de:	31 c0                	xor    %eax,%eax
  17a0e0:	c4 41 00 57 ff       	vxorps %xmm15,%xmm15,%xmm15
  17a0e5:	c5 e0 57 db          	vxorps %xmm3,%xmm3,%xmm3
  17a0e9:	c5 e8 57 d2          	vxorps %xmm2,%xmm2,%xmm2
  17a0ed:	c5 c0 57 ff          	vxorps %xmm7,%xmm7,%xmm7
  17a0f1:	eb 5b                	jmp    17a14e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x55e>
  17a0f3:	66 66 66 66 2e 0f 1f 	data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
  17a0fa:	84 00 00 00 00 00
  17a100:	c5 78 28 f5          	vmovaps %xmm5,%xmm14
  17a104:	c5 f8 28 d1          	vmovaps %xmm1,%xmm2
  17a108:	c4 c2 01 a9 d4       	vfmadd213ss %xmm12,%xmm15,%xmm2
  17a10d:	c4 c2 51 a9 d2       	vfmadd213ss %xmm10,%xmm5,%xmm2
  17a112:	c5 f8 28 d9          	vmovaps %xmm1,%xmm3
  17a116:	c4 c2 11 a9 db       	vfmadd213ss %xmm11,%xmm13,%xmm3
  17a11b:	c4 c2 51 a9 d9       	vfmadd213ss %xmm9,%xmm5,%xmm3
  17a120:	c4 41 7a 5c ff       	vsubss %xmm15,%xmm0,%xmm15
  17a125:	c4 42 71 a9 fc       	vfmadd213ss %xmm12,%xmm1,%xmm15
  17a12a:	c4 42 51 a9 f8       	vfmadd213ss %xmm8,%xmm5,%xmm15
  17a12f:	c4 41 7a 5c e5       	vsubss %xmm13,%xmm0,%xmm12
  17a134:	c4 42 71 a9 e3       	vfmadd213ss %xmm11,%xmm1,%xmm12
  17a139:	c4 62 51 a9 e6       	vfmadd213ss %xmm6,%xmm5,%xmm12
  17a13e:	4c 89 d0             	mov    %r10,%rax
  17a141:	c5 78 29 f5          	vmovaps %xmm14,%xmm5
  17a145:	4d 39 d7             	cmp    %r10,%r15
  17a148:	0f 84 1c 01 00 00    	je     17a26a <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x67a>
  17a14e:	48 39 c7             	cmp    %rax,%rdi
  17a151:	0f 84 00 04 00 00    	je     17a557 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x967>
  17a157:	48 39 c1             	cmp    %rax,%rcx
  17a15a:	0f 84 e1 03 00 00    	je     17a541 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x951>
  17a160:	c5 78 28 d2          	vmovaps %xmm2,%xmm10
  17a164:	c5 78 28 cb          	vmovaps %xmm3,%xmm9
  17a168:	c4 41 78 28 c7       	vmovaps %xmm15,%xmm8
  17a16d:	c5 78 29 e6          	vmovaps %xmm12,%xmm6
  17a171:	4c 8d 50 01          	lea    0x1(%rax),%r10
  17a175:	c5 7a 10 64 c5 00    	vmovss 0x0(%rbp,%rax,8),%xmm12
  17a17b:	c5 7a 10 5c c5 04    	vmovss 0x4(%rbp,%rax,8),%xmm11
  17a181:	c5 7a 10 3c c2       	vmovss (%rdx,%rax,8),%xmm15
  17a186:	c5 7a 10 6c c2 04    	vmovss 0x4(%rdx,%rax,8),%xmm13
  17a18c:	4d 39 fa             	cmp    %r15,%r10
  17a18f:	0f 83 6b ff ff ff    	jae    17a100 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x510>
  17a195:	48 3b 44 24 28       	cmp    0x28(%rsp),%rax
  17a19a:	0f 83 24 04 00 00    	jae    17a5c4 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9d4>
  17a1a0:	48 3b 44 24 20       	cmp    0x20(%rsp),%rax
  17a1a5:	0f 83 01 04 00 00    	jae    17a5ac <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9bc>
  17a1ab:	4c 8b 6c 24 68       	mov    0x68(%rsp),%r13
  17a1b0:	c4 41 7a 10 74 85 00 	vmovss 0x0(%r13,%rax,4),%xmm14
  17a1b7:	4c 8b 6c 24 60       	mov    0x60(%rsp),%r13
  17a1bc:	c4 42 59 a9 74 85 00 	vfmadd213ss 0x0(%r13,%rax,4),%xmm4,%xmm14
  17a1c3:	c5 f8 2e e8          	vucomiss %xmm0,%xmm5
  17a1c7:	75 02                	jne    17a1cb <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x5db>
  17a1c9:	7b 40                	jnp    17a20b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x61b>
  17a1cb:	c5 f9 7e e8          	vmovd  %xmm5,%eax
  17a1cf:	01 c0                	add    %eax,%eax
  17a1d1:	3d ff ff ff fe       	cmp    $0xfeffffff,%eax
  17a1d6:	0f 87 dc fa ff ff    	ja     179cb8 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xc8>
  17a1dc:	c4 e2 79 18 15 cb 5f 	vbroadcastss -0x15a035(%rip),%xmm2        # 201b0 <__abi_tag+0x1feb4>
  17a1e3:	ea ff
  17a1e5:	c5 d0 54 d2          	vandps %xmm2,%xmm5,%xmm2
  17a1e9:	31 c0                	xor    %eax,%eax
  17a1eb:	c5 f8 2e 15 9d 5f ea 	vucomiss -0x15a063(%rip),%xmm2        # 20190 <__abi_tag+0x1fe94>
  17a1f2:	ff
  17a1f3:	0f 82 c1 fa ff ff    	jb     179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a1f9:	c5 fa 10 1d 7f 5d ea 	vmovss -0x15a281(%rip),%xmm3        # 1ff80 <__abi_tag+0x1fc84>
  17a200:	ff
  17a201:	c5 f8 2e da          	vucomiss %xmm2,%xmm3
  17a205:	0f 82 af fa ff ff    	jb     179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a20b:	c4 62 51 a9 f7       	vfmadd213ss %xmm7,%xmm5,%xmm14
  17a210:	c5 78 2e f0          	vucomiss %xmm0,%xmm14
  17a214:	75 0b                	jne    17a221 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x631>
  17a216:	7a 09                	jp     17a221 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x631>
  17a218:	c5 f8 28 fd          	vmovaps %xmm5,%xmm7
  17a21c:	e9 e3 fe ff ff       	jmp    17a104 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x514>
  17a221:	c5 79 7e f0          	vmovd  %xmm14,%eax
  17a225:	01 c0                	add    %eax,%eax
  17a227:	3d ff ff ff fe       	cmp    $0xfeffffff,%eax
  17a22c:	0f 87 86 fa ff ff    	ja     179cb8 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xc8>
  17a232:	c4 e2 79 18 15 75 5f 	vbroadcastss -0x15a08b(%rip),%xmm2        # 201b0 <__abi_tag+0x1feb4>
  17a239:	ea ff
  17a23b:	c5 88 54 d2          	vandps %xmm2,%xmm14,%xmm2
  17a23f:	31 c0                	xor    %eax,%eax
  17a241:	c5 f8 2e 15 47 5f ea 	vucomiss -0x15a0b9(%rip),%xmm2        # 20190 <__abi_tag+0x1fe94>
  17a248:	ff
  17a249:	0f 82 6b fa ff ff    	jb     179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a24f:	c5 fa 10 1d 29 5d ea 	vmovss -0x15a2d7(%rip),%xmm3        # 1ff80 <__abi_tag+0x1fc84>
  17a256:	ff
  17a257:	c5 f8 2e da          	vucomiss %xmm2,%xmm3
  17a25b:	c5 f8 28 fd          	vmovaps %xmm5,%xmm7
  17a25f:	0f 83 9f fe ff ff    	jae    17a104 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x514>
  17a265:	e9 50 fa ff ff       	jmp    179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a26a:	4c 39 c3             	cmp    %r8,%rbx
  17a26d:	0f 84 95 03 00 00    	je     17a608 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa18>
  17a273:	48 8b 44 24 70       	mov    0x70(%rsp),%rax
  17a278:	c5 fa 11 14 98       	vmovss %xmm2,(%rax,%rbx,4)
  17a27d:	4c 39 db             	cmp    %r11,%rbx
  17a280:	0f 84 6c 03 00 00    	je     17a5f2 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa02>
  17a286:	48 8b 04 24          	mov    (%rsp),%rax
  17a28a:	c5 fa 11 1c 98       	vmovss %xmm3,(%rax,%rbx,4)
  17a28f:	c5 7a 11 3c 9e       	vmovss %xmm15,(%rsi,%rbx,4)
  17a294:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  17a299:	c5 7a 11 24 98       	vmovss %xmm12,(%rax,%rbx,4)
  17a29e:	4c 89 f3             	mov    %r14,%rbx
  17a2a1:	4d 39 e6             	cmp    %r12,%r14
  17a2a4:	0f 85 0a fe ff ff    	jne    17a0b4 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x4c4>
  17a2aa:	b0 01                	mov    $0x1,%al
  17a2ac:	e9 09 fa ff ff       	jmp    179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a2b1:	4d 85 c9             	test   %r9,%r9
  17a2b4:	48 8b 4c 24 30       	mov    0x30(%rsp),%rcx
  17a2b9:	0f 84 1d 03 00 00    	je     17a5dc <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9ec>
  17a2bf:	4d 85 c0             	test   %r8,%r8
  17a2c2:	0f 84 40 03 00 00    	je     17a608 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa18>
  17a2c8:	48 8b 54 24 70       	mov    0x70(%rsp),%rdx
  17a2cd:	c7 02 00 00 00 00    	movl   $0x0,(%rdx)
  17a2d3:	4d 85 db             	test   %r11,%r11
  17a2d6:	0f 84 16 03 00 00    	je     17a5f2 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa02>
  17a2dc:	48 8b 14 24          	mov    (%rsp),%rdx
  17a2e0:	c7 02 00 00 00 00    	movl   $0x0,(%rdx)
  17a2e6:	c7 01 00 00 00 00    	movl   $0x0,(%rcx)
  17a2ec:	48 8b 54 24 08       	mov    0x8(%rsp),%rdx
  17a2f1:	c7 02 00 00 00 00    	movl   $0x0,(%rdx)
  17a2f7:	41 83 fc 01          	cmp    $0x1,%r12d
  17a2fb:	0f 84 b9 f9 ff ff    	je     179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a301:	41 83 f9 01          	cmp    $0x1,%r9d
  17a305:	0f 84 d1 02 00 00    	je     17a5dc <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9ec>
  17a30b:	41 83 f8 01          	cmp    $0x1,%r8d
  17a30f:	0f 84 f3 02 00 00    	je     17a608 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa18>
  17a315:	48 8b 54 24 70       	mov    0x70(%rsp),%rdx
  17a31a:	c7 42 04 00 00 00 00 	movl   $0x0,0x4(%rdx)
  17a321:	41 83 fb 01          	cmp    $0x1,%r11d
  17a325:	0f 84 c7 02 00 00    	je     17a5f2 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa02>
  17a32b:	48 8b 14 24          	mov    (%rsp),%rdx
  17a32f:	c7 42 04 00 00 00 00 	movl   $0x0,0x4(%rdx)
  17a336:	c7 41 04 00 00 00 00 	movl   $0x0,0x4(%rcx)
  17a33d:	48 8b 54 24 08       	mov    0x8(%rsp),%rdx
  17a342:	c7 42 04 00 00 00 00 	movl   $0x0,0x4(%rdx)
  17a349:	41 83 fc 02          	cmp    $0x2,%r12d
  17a34d:	0f 84 67 f9 ff ff    	je     179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a353:	41 83 f9 02          	cmp    $0x2,%r9d
  17a357:	0f 84 7f 02 00 00    	je     17a5dc <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9ec>
  17a35d:	41 83 f8 02          	cmp    $0x2,%r8d
  17a361:	0f 84 a1 02 00 00    	je     17a608 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa18>
  17a367:	48 8b 54 24 70       	mov    0x70(%rsp),%rdx
  17a36c:	c7 42 08 00 00 00 00 	movl   $0x0,0x8(%rdx)
  17a373:	41 83 fb 02          	cmp    $0x2,%r11d
  17a377:	0f 84 75 02 00 00    	je     17a5f2 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa02>
  17a37d:	48 8b 14 24          	mov    (%rsp),%rdx
  17a381:	c7 42 08 00 00 00 00 	movl   $0x0,0x8(%rdx)
  17a388:	c7 41 08 00 00 00 00 	movl   $0x0,0x8(%rcx)
  17a38f:	48 8b 54 24 08       	mov    0x8(%rsp),%rdx
  17a394:	c7 42 08 00 00 00 00 	movl   $0x0,0x8(%rdx)
  17a39b:	41 83 fc 03          	cmp    $0x3,%r12d
  17a39f:	0f 84 15 f9 ff ff    	je     179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a3a5:	41 83 f9 03          	cmp    $0x3,%r9d
  17a3a9:	0f 84 2d 02 00 00    	je     17a5dc <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9ec>
  17a3af:	41 83 f8 03          	cmp    $0x3,%r8d
  17a3b3:	0f 84 4f 02 00 00    	je     17a608 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa18>
  17a3b9:	48 8b 54 24 70       	mov    0x70(%rsp),%rdx
  17a3be:	c7 42 0c 00 00 00 00 	movl   $0x0,0xc(%rdx)
  17a3c5:	41 83 fb 03          	cmp    $0x3,%r11d
  17a3c9:	0f 84 23 02 00 00    	je     17a5f2 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa02>
  17a3cf:	48 8b 14 24          	mov    (%rsp),%rdx
  17a3d3:	c7 42 0c 00 00 00 00 	movl   $0x0,0xc(%rdx)
  17a3da:	c7 41 0c 00 00 00 00 	movl   $0x0,0xc(%rcx)
  17a3e1:	48 8b 54 24 08       	mov    0x8(%rsp),%rdx
  17a3e6:	c7 42 0c 00 00 00 00 	movl   $0x0,0xc(%rdx)
  17a3ed:	41 83 fc 04          	cmp    $0x4,%r12d
  17a3f1:	0f 84 c3 f8 ff ff    	je     179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a3f7:	41 83 f9 04          	cmp    $0x4,%r9d
  17a3fb:	0f 84 db 01 00 00    	je     17a5dc <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9ec>
  17a401:	41 83 f8 04          	cmp    $0x4,%r8d
  17a405:	0f 84 fd 01 00 00    	je     17a608 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa18>
  17a40b:	48 8b 54 24 70       	mov    0x70(%rsp),%rdx
  17a410:	c7 42 10 00 00 00 00 	movl   $0x0,0x10(%rdx)
  17a417:	41 83 fb 04          	cmp    $0x4,%r11d
  17a41b:	0f 84 d1 01 00 00    	je     17a5f2 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa02>
  17a421:	48 8b 14 24          	mov    (%rsp),%rdx
  17a425:	c7 42 10 00 00 00 00 	movl   $0x0,0x10(%rdx)
  17a42c:	c7 41 10 00 00 00 00 	movl   $0x0,0x10(%rcx)
  17a433:	48 8b 54 24 08       	mov    0x8(%rsp),%rdx
  17a438:	c7 42 10 00 00 00 00 	movl   $0x0,0x10(%rdx)
  17a43f:	41 83 fc 05          	cmp    $0x5,%r12d
  17a443:	0f 84 71 f8 ff ff    	je     179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a449:	41 83 f9 05          	cmp    $0x5,%r9d
  17a44d:	0f 84 89 01 00 00    	je     17a5dc <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9ec>
  17a453:	41 83 f8 05          	cmp    $0x5,%r8d
  17a457:	0f 84 ab 01 00 00    	je     17a608 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa18>
  17a45d:	48 8b 54 24 70       	mov    0x70(%rsp),%rdx
  17a462:	c7 42 14 00 00 00 00 	movl   $0x0,0x14(%rdx)
  17a469:	41 83 fb 05          	cmp    $0x5,%r11d
  17a46d:	0f 84 7f 01 00 00    	je     17a5f2 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa02>
  17a473:	48 8b 14 24          	mov    (%rsp),%rdx
  17a477:	c7 42 14 00 00 00 00 	movl   $0x0,0x14(%rdx)
  17a47e:	c7 41 14 00 00 00 00 	movl   $0x0,0x14(%rcx)
  17a485:	48 8b 54 24 08       	mov    0x8(%rsp),%rdx
  17a48a:	c7 42 14 00 00 00 00 	movl   $0x0,0x14(%rdx)
  17a491:	41 83 fc 06          	cmp    $0x6,%r12d
  17a495:	0f 84 1f f8 ff ff    	je     179cba <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xca>
  17a49b:	41 83 f9 06          	cmp    $0x6,%r9d
  17a49f:	0f 84 37 01 00 00    	je     17a5dc <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x9ec>
  17a4a5:	41 83 f8 06          	cmp    $0x6,%r8d
  17a4a9:	0f 84 59 01 00 00    	je     17a608 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa18>
  17a4af:	48 8b 44 24 70       	mov    0x70(%rsp),%rax
  17a4b4:	c7 40 18 00 00 00 00 	movl   $0x0,0x18(%rax)
  17a4bb:	41 83 fb 06          	cmp    $0x6,%r11d
  17a4bf:	0f 84 2d 01 00 00    	je     17a5f2 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0xa02>
  17a4c5:	48 8b 04 24          	mov    (%rsp),%rax
  17a4c9:	c7 40 18 00 00 00 00 	movl   $0x0,0x18(%rax)
  17a4d0:	c7 41 18 00 00 00 00 	movl   $0x0,0x18(%rcx)
  17a4d7:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  17a4dc:	c7 40 18 00 00 00 00 	movl   $0x0,0x18(%rax)
  17a4e3:	e9 c2 fd ff ff       	jmp    17a2aa <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::synthesis::FusedSynthesisBlock<f32, true, true, true>>>+0x6ba>
  17a4e8:	48 8d 15 19 f7 10 00 	lea    0x10f719(%rip),%rdx        # 289c08 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0x108>
  17a4ef:	48 89 fe             	mov    %rdi,%rsi
  17a4f2:	c5 f8 77             	vzeroupper
  17a4f5:	ff 15 cd 94 11 00    	call   *0x1194cd(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a4fb:	48 8d 15 1e f7 10 00 	lea    0x10f71e(%rip),%rdx        # 289c20 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0x120>
  17a502:	48 89 cf             	mov    %rcx,%rdi
  17a505:	48 89 ce             	mov    %rcx,%rsi
  17a508:	c5 f8 77             	vzeroupper
  17a50b:	ff 15 b7 94 11 00    	call   *0x1194b7(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a511:	48 8d 15 38 f7 10 00 	lea    0x10f738(%rip),%rdx        # 289c50 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0x150>
  17a518:	4c 89 cf             	mov    %r9,%rdi
  17a51b:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
  17a520:	c5 f8 77             	vzeroupper
  17a523:	ff 15 9f 94 11 00    	call   *0x11949f(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a529:	48 8d 15 08 f7 10 00 	lea    0x10f708(%rip),%rdx        # 289c38 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0x138>
  17a530:	4c 89 cf             	mov    %r9,%rdi
  17a533:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
  17a538:	c5 f8 77             	vzeroupper
  17a53b:	ff 15 87 94 11 00    	call   *0x119487(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a541:	48 8d 15 30 f6 10 00 	lea    0x10f630(%rip),%rdx        # 289b78 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0x78>
  17a548:	48 89 cf             	mov    %rcx,%rdi
  17a54b:	48 89 ce             	mov    %rcx,%rsi
  17a54e:	c5 f8 77             	vzeroupper
  17a551:	ff 15 71 94 11 00    	call   *0x119471(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a557:	48 8d 15 02 f6 10 00 	lea    0x10f602(%rip),%rdx        # 289b60 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0x60>
  17a55e:	48 89 fe             	mov    %rdi,%rsi
  17a561:	c5 f8 77             	vzeroupper
  17a564:	ff 15 5e 94 11 00    	call   *0x11945e(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a56a:	48 8d 15 4f f6 10 00 	lea    0x10f64f(%rip),%rdx        # 289bc0 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0xc0>
  17a571:	48 89 c7             	mov    %rax,%rdi
  17a574:	48 89 c6             	mov    %rax,%rsi
  17a577:	c5 f8 77             	vzeroupper
  17a57a:	ff 15 48 94 11 00    	call   *0x119448(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a580:	48 8d 15 51 f6 10 00 	lea    0x10f651(%rip),%rdx        # 289bd8 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0xd8>
  17a587:	4c 89 df             	mov    %r11,%rdi
  17a58a:	4c 89 de             	mov    %r11,%rsi
  17a58d:	c5 f8 77             	vzeroupper
  17a590:	ff 15 32 94 11 00    	call   *0x119432(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a596:	48 8d 15 53 f6 10 00 	lea    0x10f653(%rip),%rdx        # 289bf0 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0xf0>
  17a59d:	4c 89 c7             	mov    %r8,%rdi
  17a5a0:	4c 89 c6             	mov    %r8,%rsi
  17a5a3:	c5 f8 77             	vzeroupper
  17a5a6:	ff 15 1c 94 11 00    	call   *0x11941c(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a5ac:	48 8d 15 f5 f5 10 00 	lea    0x10f5f5(%rip),%rdx        # 289ba8 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0xa8>
  17a5b3:	48 89 c7             	mov    %rax,%rdi
  17a5b6:	48 8b 74 24 20       	mov    0x20(%rsp),%rsi
  17a5bb:	c5 f8 77             	vzeroupper
  17a5be:	ff 15 04 94 11 00    	call   *0x119404(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a5c4:	48 8d 15 c5 f5 10 00 	lea    0x10f5c5(%rip),%rdx        # 289b90 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0x90>
  17a5cb:	48 89 c7             	mov    %rax,%rdi
  17a5ce:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
  17a5d3:	c5 f8 77             	vzeroupper
  17a5d6:	ff 15 ec 93 11 00    	call   *0x1193ec(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a5dc:	48 8d 15 35 f5 10 00 	lea    0x10f535(%rip),%rdx        # 289b18 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0x18>
  17a5e3:	4c 89 cf             	mov    %r9,%rdi
  17a5e6:	4c 89 ce             	mov    %r9,%rsi
  17a5e9:	c5 f8 77             	vzeroupper
  17a5ec:	ff 15 d6 93 11 00    	call   *0x1193d6(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a5f2:	48 8d 15 4f f5 10 00 	lea    0x10f54f(%rip),%rdx        # 289b48 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0x48>
  17a5f9:	4c 89 df             	mov    %r11,%rdi
  17a5fc:	4c 89 de             	mov    %r11,%rsi
  17a5ff:	c5 f8 77             	vzeroupper
  17a602:	ff 15 c0 93 11 00    	call   *0x1193c0(%rip)        # 2939c8 <_DYNAMIC+0x260>
  17a608:	48 8d 15 21 f5 10 00 	lea    0x10f521(%rip),%rdx        # 289b30 <anon.5121fecd2351c5ba2e97336f14d76e83.21.llvm.7576789630240191199+0x30>
  17a60f:	4c 89 c7             	mov    %r8,%rdi
  17a612:	4c 89 c6             	mov    %r8,%rsi
  17a615:	c5 f8 77             	vzeroupper
  17a618:	ff 15 aa 93 11 00    	call   *0x1193aa(%rip)        # 2939c8 <_DYNAMIC+0x260>

Disassembly of section .init:

Disassembly of section .fini:

Disassembly of section .plt:
