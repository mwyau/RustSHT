
target/release/examples/m1b_bench:     file format elf64-x86-64


Disassembly of section .text:

00000000001e4a90 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>>:
  1e4a90:	55                   	push   %rbp
  1e4a91:	41 57                	push   %r15
  1e4a93:	41 56                	push   %r14
  1e4a95:	41 55                	push   %r13
  1e4a97:	41 54                	push   %r12
  1e4a99:	53                   	push   %rbx
  1e4a9a:	48 81 ec 68 01 00 00 	sub    $0x168,%rsp
  1e4aa1:	48 8b 97 b0 00 00 00 	mov    0xb0(%rdi),%rdx
  1e4aa8:	48 8b 8f b8 00 00 00 	mov    0xb8(%rdi),%rcx
  1e4aaf:	b0 01                	mov    $0x1,%al
  1e4ab1:	48 39 ca             	cmp    %rcx,%rdx
  1e4ab4:	0f 87 a6 0f 00 00    	ja     1e5a60 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xfd0>
  1e4aba:	48 89 4c 24 58       	mov    %rcx,0x58(%rsp)
  1e4abf:	48 b8 fc ff ff ff ff 	movabs $0xffffffffffffffc,%rax
  1e4ac6:	ff ff 0f 
  1e4ac9:	48 8b 4f 38          	mov    0x38(%rdi),%rcx
  1e4acd:	48 8b 77 48          	mov    0x48(%rdi),%rsi
  1e4ad1:	4c 8b 57 58          	mov    0x58(%rdi),%r10
  1e4ad5:	48 8b 6f 68          	mov    0x68(%rdi),%rbp
  1e4ad9:	48 89 94 24 10 01 00 	mov    %rdx,0x110(%rsp)
  1e4ae0:	00 
  1e4ae1:	49 89 cb             	mov    %rcx,%r11
  1e4ae4:	49 c1 eb 02          	shr    $0x2,%r11
  1e4ae8:	89 ca                	mov    %ecx,%edx
  1e4aea:	83 e2 03             	and    $0x3,%edx
  1e4aed:	48 89 54 24 48       	mov    %rdx,0x48(%rsp)
  1e4af2:	48 89 ca             	mov    %rcx,%rdx
  1e4af5:	48 21 c2             	and    %rax,%rdx
  1e4af8:	48 89 54 24 70       	mov    %rdx,0x70(%rsp)
  1e4afd:	48 89 f3             	mov    %rsi,%rbx
  1e4b00:	48 c1 eb 02          	shr    $0x2,%rbx
  1e4b04:	89 f2                	mov    %esi,%edx
  1e4b06:	83 e2 03             	and    $0x3,%edx
  1e4b09:	48 89 54 24 38       	mov    %rdx,0x38(%rsp)
  1e4b0e:	48 89 f2             	mov    %rsi,%rdx
  1e4b11:	48 21 c2             	and    %rax,%rdx
  1e4b14:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
  1e4b19:	4d 89 d4             	mov    %r10,%r12
  1e4b1c:	49 c1 ec 02          	shr    $0x2,%r12
  1e4b20:	44 89 d2             	mov    %r10d,%edx
  1e4b23:	83 e2 03             	and    $0x3,%edx
  1e4b26:	48 89 54 24 30       	mov    %rdx,0x30(%rsp)
  1e4b2b:	4c 89 d2             	mov    %r10,%rdx
  1e4b2e:	48 21 c2             	and    %rax,%rdx
  1e4b31:	48 89 54 24 78       	mov    %rdx,0x78(%rsp)
  1e4b36:	48 89 ea             	mov    %rbp,%rdx
  1e4b39:	48 c1 ea 02          	shr    $0x2,%rdx
  1e4b3d:	41 89 e8             	mov    %ebp,%r8d
  1e4b40:	41 83 e0 03          	and    $0x3,%r8d
  1e4b44:	4c 89 44 24 28       	mov    %r8,0x28(%rsp)
  1e4b49:	49 89 e8             	mov    %rbp,%r8
  1e4b4c:	49 21 c0             	and    %rax,%r8
  1e4b4f:	4c 89 44 24 68       	mov    %r8,0x68(%rsp)
  1e4b54:	4c 8b 77 08          	mov    0x8(%rdi),%r14
  1e4b58:	4d 89 f1             	mov    %r14,%r9
  1e4b5b:	49 c1 e9 02          	shr    $0x2,%r9
  1e4b5f:	45 89 f0             	mov    %r14d,%r8d
  1e4b62:	41 83 e0 03          	and    $0x3,%r8d
  1e4b66:	4c 89 44 24 40       	mov    %r8,0x40(%rsp)
  1e4b6b:	4d 89 f0             	mov    %r14,%r8
  1e4b6e:	49 21 c0             	and    %rax,%r8
  1e4b71:	4c 89 84 24 80 00 00 	mov    %r8,0x80(%rsp)
  1e4b78:	00 
  1e4b79:	49 bf fe ff ff ff ff 	movabs $0x3fffffffffffffe,%r15
  1e4b80:	ff ff 03 
  1e4b83:	4d 21 fb             	and    %r15,%r11
  1e4b86:	49 c1 e3 05          	shl    $0x5,%r11
  1e4b8a:	4c 8b 47 30          	mov    0x30(%rdi),%r8
  1e4b8e:	4d 01 c3             	add    %r8,%r11
  1e4b91:	4c 89 9c 24 c8 00 00 	mov    %r11,0xc8(%rsp)
  1e4b98:	00 
  1e4b99:	48 89 8c 24 e8 00 00 	mov    %rcx,0xe8(%rsp)
  1e4ba0:	00 
  1e4ba1:	49 89 cb             	mov    %rcx,%r11
  1e4ba4:	49 c1 eb 03          	shr    $0x3,%r11
  1e4ba8:	4c 21 fb             	and    %r15,%rbx
  1e4bab:	48 c1 e3 05          	shl    $0x5,%rbx
  1e4baf:	4c 8b 6f 40          	mov    0x40(%rdi),%r13
  1e4bb3:	4c 01 eb             	add    %r13,%rbx
  1e4bb6:	48 89 9c 24 c0 00 00 	mov    %rbx,0xc0(%rsp)
  1e4bbd:	00 
  1e4bbe:	48 89 b4 24 e0 00 00 	mov    %rsi,0xe0(%rsp)
  1e4bc5:	00 
  1e4bc6:	48 c1 ee 03          	shr    $0x3,%rsi
  1e4bca:	4c 89 f9             	mov    %r15,%rcx
  1e4bcd:	4d 21 fc             	and    %r15,%r12
  1e4bd0:	49 c1 e4 05          	shl    $0x5,%r12
  1e4bd4:	4c 8b 7f 50          	mov    0x50(%rdi),%r15
  1e4bd8:	4d 01 fc             	add    %r15,%r12
  1e4bdb:	4c 89 a4 24 b8 00 00 	mov    %r12,0xb8(%rsp)
  1e4be2:	00 
  1e4be3:	4c 89 94 24 d8 00 00 	mov    %r10,0xd8(%rsp)
  1e4bea:	00 
  1e4beb:	49 c1 ea 03          	shr    $0x3,%r10
  1e4bef:	48 21 ca             	and    %rcx,%rdx
  1e4bf2:	48 89 cb             	mov    %rcx,%rbx
  1e4bf5:	48 c1 e2 05          	shl    $0x5,%rdx
  1e4bf9:	48 8b 4f 60          	mov    0x60(%rdi),%rcx
  1e4bfd:	48 01 ca             	add    %rcx,%rdx
  1e4c00:	48 89 94 24 b0 00 00 	mov    %rdx,0xb0(%rsp)
  1e4c07:	00 
  1e4c08:	48 89 ac 24 d0 00 00 	mov    %rbp,0xd0(%rsp)
  1e4c0f:	00 
  1e4c10:	48 c1 ed 03          	shr    $0x3,%rbp
  1e4c14:	49 21 d9             	and    %rbx,%r9
  1e4c17:	49 c1 e1 05          	shl    $0x5,%r9
  1e4c1b:	4c 8b 27             	mov    (%rdi),%r12
  1e4c1e:	4d 01 e1             	add    %r12,%r9
  1e4c21:	4c 89 8c 24 a8 00 00 	mov    %r9,0xa8(%rsp)
  1e4c28:	00 
  1e4c29:	4c 89 b4 24 f0 00 00 	mov    %r14,0xf0(%rsp)
  1e4c30:	00 
  1e4c31:	49 c1 ee 03          	shr    $0x3,%r14
  1e4c35:	4c 89 b4 24 90 00 00 	mov    %r14,0x90(%rsp)
  1e4c3c:	00 
  1e4c3d:	4c 39 de             	cmp    %r11,%rsi
  1e4c40:	4c 89 9c 24 30 01 00 	mov    %r11,0x130(%rsp)
  1e4c47:	00 
  1e4c48:	48 89 b4 24 28 01 00 	mov    %rsi,0x128(%rsp)
  1e4c4f:	00 
  1e4c50:	4c 0f 42 de          	cmovb  %rsi,%r11
  1e4c54:	4d 39 da             	cmp    %r11,%r10
  1e4c57:	4c 89 94 24 20 01 00 	mov    %r10,0x120(%rsp)
  1e4c5e:	00 
  1e4c5f:	4d 0f 42 da          	cmovb  %r10,%r11
  1e4c63:	4c 39 dd             	cmp    %r11,%rbp
  1e4c66:	48 89 ac 24 18 01 00 	mov    %rbp,0x118(%rsp)
  1e4c6d:	00 
  1e4c6e:	4c 0f 42 dd          	cmovb  %rbp,%r11
  1e4c72:	4c 89 9c 24 58 01 00 	mov    %r11,0x158(%rsp)
  1e4c79:	00 
  1e4c7a:	48 8b 54 24 48       	mov    0x48(%rsp),%rdx
  1e4c7f:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
  1e4c84:	48 39 d0             	cmp    %rdx,%rax
  1e4c87:	48 0f 42 d0          	cmovb  %rax,%rdx
  1e4c8b:	48 8b 47 10          	mov    0x10(%rdi),%rax
  1e4c8f:	48 89 84 24 50 01 00 	mov    %rax,0x150(%rsp)
  1e4c96:	00 
  1e4c97:	48 8b 47 18          	mov    0x18(%rdi),%rax
  1e4c9b:	48 89 84 24 00 01 00 	mov    %rax,0x100(%rsp)
  1e4ca2:	00 
  1e4ca3:	48 8b 47 20          	mov    0x20(%rdi),%rax
  1e4ca7:	48 89 84 24 48 01 00 	mov    %rax,0x148(%rsp)
  1e4cae:	00 
  1e4caf:	48 8b 47 28          	mov    0x28(%rdi),%rax
  1e4cb3:	48 89 84 24 f8 00 00 	mov    %rax,0xf8(%rsp)
  1e4cba:	00 
  1e4cbb:	4c 8b 77 70          	mov    0x70(%rdi),%r14
  1e4cbf:	48 8b 5f 78          	mov    0x78(%rdi),%rbx
  1e4cc3:	4c 8b 8f a8 00 00 00 	mov    0xa8(%rdi),%r9
  1e4cca:	48 8b 87 a0 00 00 00 	mov    0xa0(%rdi),%rax
  1e4cd1:	48 8b b7 98 00 00 00 	mov    0x98(%rdi),%rsi
  1e4cd8:	48 89 74 24 08       	mov    %rsi,0x8(%rsp)
  1e4cdd:	48 8b af 90 00 00 00 	mov    0x90(%rdi),%rbp
  1e4ce4:	48 8b b7 80 00 00 00 	mov    0x80(%rdi),%rsi
  1e4ceb:	48 8b bf 88 00 00 00 	mov    0x88(%rdi),%rdi
  1e4cf2:	4c 8b 54 24 30       	mov    0x30(%rsp),%r10
  1e4cf7:	4c 39 d2             	cmp    %r10,%rdx
  1e4cfa:	49 0f 43 d2          	cmovae %r10,%rdx
  1e4cfe:	4c 8b 54 24 40       	mov    0x40(%rsp),%r10
  1e4d03:	4c 39 d2             	cmp    %r10,%rdx
  1e4d06:	49 0f 43 d2          	cmovae %r10,%rdx
  1e4d0a:	4c 8b 54 24 28       	mov    0x28(%rsp),%r10
  1e4d0f:	4c 39 d2             	cmp    %r10,%rdx
  1e4d12:	49 0f 43 d2          	cmovae %r10,%rdx
  1e4d16:	48 89 94 24 40 01 00 	mov    %rdx,0x140(%rsp)
  1e4d1d:	00 
  1e4d1e:	c5 d1 57 ed          	vxorpd %xmm5,%xmm5,%xmm5
  1e4d22:	48 c7 84 24 08 01 00 	movq   $0x0,0x108(%rsp)
  1e4d29:	00 00 00 00 00 
  1e4d2e:	4c 8b 94 24 10 01 00 	mov    0x110(%rsp),%r10
  1e4d35:	00 
  1e4d36:	45 31 db             	xor    %r11d,%r11d
  1e4d39:	48 89 5c 24 20       	mov    %rbx,0x20(%rsp)
  1e4d3e:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
  1e4d43:	4c 89 4c 24 60       	mov    %r9,0x60(%rsp)
  1e4d48:	4c 89 d2             	mov    %r10,%rdx
  1e4d4b:	4c 01 da             	add    %r11,%rdx
  1e4d4e:	0f 82 0a 0d 00 00    	jb     1e5a5e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xfce>
  1e4d54:	4c 8b 54 24 58       	mov    0x58(%rsp),%r10
  1e4d59:	4c 39 d2             	cmp    %r10,%rdx
  1e4d5c:	0f 87 fc 0c 00 00    	ja     1e5a5e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xfce>
  1e4d62:	49 89 c1             	mov    %rax,%r9
  1e4d65:	48 89 f0             	mov    %rsi,%rax
  1e4d68:	48 8d 72 02          	lea    0x2(%rdx),%rsi
  1e4d6c:	4c 39 d6             	cmp    %r10,%rsi
  1e4d6f:	48 89 bc 24 88 00 00 	mov    %rdi,0x88(%rsp)
  1e4d76:	00 
  1e4d77:	48 89 b4 24 60 01 00 	mov    %rsi,0x160(%rsp)
  1e4d7e:	00 
  1e4d7f:	0f 86 0b 01 00 00    	jbe    1e4e90 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x400>
  1e4d85:	4c 8b 5c 24 08       	mov    0x8(%rsp),%r11
  1e4d8a:	4c 89 de             	mov    %r11,%rsi
  1e4d8d:	48 c1 ee 03          	shr    $0x3,%rsi
  1e4d91:	48 8b bc 24 58 01 00 	mov    0x158(%rsp),%rdi
  1e4d98:	00 
  1e4d99:	48 39 fe             	cmp    %rdi,%rsi
  1e4d9c:	48 0f 43 f7          	cmovae %rdi,%rsi
  1e4da0:	48 8b bc 24 90 00 00 	mov    0x90(%rsp),%rdi
  1e4da7:	00 
  1e4da8:	48 39 f7             	cmp    %rsi,%rdi
  1e4dab:	48 0f 42 f7          	cmovb  %rdi,%rsi
  1e4daf:	48 85 f6             	test   %rsi,%rsi
  1e4db2:	0f 84 9b 02 00 00    	je     1e5053 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x5c3>
  1e4db8:	c5 c9 57 f6          	vxorpd %xmm6,%xmm6,%xmm6
  1e4dbc:	bf 20 00 00 00       	mov    $0x20,%edi
  1e4dc1:	c4 41 29 57 d2       	vxorpd %xmm10,%xmm10,%xmm10
  1e4dc6:	c4 41 39 57 c0       	vxorpd %xmm8,%xmm8,%xmm8
  1e4dcb:	c4 41 11 57 ed       	vxorpd %xmm13,%xmm13,%xmm13
  1e4dd0:	c4 41 19 57 e4       	vxorpd %xmm12,%xmm12,%xmm12
  1e4dd5:	c4 41 31 57 c9       	vxorpd %xmm9,%xmm9,%xmm9
  1e4dda:	c5 c1 57 ff          	vxorpd %xmm7,%xmm7,%xmm7
  1e4dde:	c4 41 21 57 db       	vxorpd %xmm11,%xmm11,%xmm11
  1e4de3:	66 66 66 66 2e 0f 1f 	data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
  1e4dea:	84 00 00 00 00 00 
  1e4df0:	c5 fd 10 54 3d e0    	vmovupd -0x20(%rbp,%rdi,1),%ymm2
  1e4df6:	c5 7d 10 74 3d 00    	vmovupd 0x0(%rbp,%rdi,1),%ymm14
  1e4dfc:	c4 41 6d 59 7c 38 e0 	vmulpd -0x20(%r8,%rdi,1),%ymm2,%ymm15
  1e4e03:	c4 c1 6d 59 4c 3d e0 	vmulpd -0x20(%r13,%rdi,1),%ymm2,%ymm1
  1e4e0a:	c5 85 58 ff          	vaddpd %ymm7,%ymm15,%ymm7
  1e4e0e:	c5 35 58 c9          	vaddpd %ymm1,%ymm9,%ymm9
  1e4e12:	c4 c1 6d 59 4c 3f e0 	vmulpd -0x20(%r15,%rdi,1),%ymm2,%ymm1
  1e4e19:	c4 41 7d 10 7c 3c e0 	vmovupd -0x20(%r12,%rdi,1),%ymm15
  1e4e20:	c4 c1 7d 10 04 3c    	vmovupd (%r12,%rdi,1),%ymm0
  1e4e26:	c5 85 59 c9          	vmulpd %ymm1,%ymm15,%ymm1
  1e4e2a:	c5 1d 58 e1          	vaddpd %ymm1,%ymm12,%ymm12
  1e4e2e:	c5 ed 59 4c 39 e0    	vmulpd -0x20(%rcx,%rdi,1),%ymm2,%ymm1
  1e4e34:	c5 85 59 c9          	vmulpd %ymm1,%ymm15,%ymm1
  1e4e38:	c5 15 58 e9          	vaddpd %ymm1,%ymm13,%ymm13
  1e4e3c:	c4 c1 0d 59 0c 38    	vmulpd (%r8,%rdi,1),%ymm14,%ymm1
  1e4e42:	c4 c1 0d 59 54 3d 00 	vmulpd 0x0(%r13,%rdi,1),%ymm14,%ymm2
  1e4e49:	c5 3d 58 c1          	vaddpd %ymm1,%ymm8,%ymm8
  1e4e4d:	c5 2d 58 d2          	vaddpd %ymm2,%ymm10,%ymm10
  1e4e51:	c4 c1 0d 59 0c 3f    	vmulpd (%r15,%rdi,1),%ymm14,%ymm1
  1e4e57:	c5 f5 59 c8          	vmulpd %ymm0,%ymm1,%ymm1
  1e4e5b:	c5 cd 58 f1          	vaddpd %ymm1,%ymm6,%ymm6
  1e4e5f:	c5 8d 59 0c 39       	vmulpd (%rcx,%rdi,1),%ymm14,%ymm1
  1e4e64:	c5 fd 59 c1          	vmulpd %ymm1,%ymm0,%ymm0
  1e4e68:	c5 25 58 d8          	vaddpd %ymm0,%ymm11,%ymm11
  1e4e6c:	48 83 c7 40          	add    $0x40,%rdi
  1e4e70:	48 ff ce             	dec    %rsi
  1e4e73:	0f 85 77 ff ff ff    	jne    1e4df0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x360>
  1e4e79:	41 f6 c3 04          	test   $0x4,%r11b
  1e4e7d:	0f 85 00 02 00 00    	jne    1e5083 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x5f3>
  1e4e83:	e9 b4 02 00 00       	jmp    1e513c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x6ac>
  1e4e88:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  1e4e8f:	00 
  1e4e90:	4c 8b 9c 24 08 01 00 	mov    0x108(%rsp),%r11
  1e4e97:	00 
  1e4e98:	4c 3b 9c 24 00 01 00 	cmp    0x100(%rsp),%r11
  1e4e9f:	00 
  1e4ea0:	4c 8b 54 24 08       	mov    0x8(%rsp),%r10
  1e4ea5:	0f 83 4e 0d 00 00    	jae    1e5bf9 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1169>
  1e4eab:	4c 3b 9c 24 f8 00 00 	cmp    0xf8(%rsp),%r11
  1e4eb2:	00 
  1e4eb3:	0f 83 5b 0d 00 00    	jae    1e5c14 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1184>
  1e4eb9:	48 8b b4 24 50 01 00 	mov    0x150(%rsp),%rsi
  1e4ec0:	00 
  1e4ec1:	c4 a2 7d 19 34 de    	vbroadcastsd (%rsi,%r11,8),%ymm6
  1e4ec7:	48 8b b4 24 48 01 00 	mov    0x148(%rsp),%rsi
  1e4ece:	00 
  1e4ecf:	c4 a2 7d 19 3c de    	vbroadcastsd (%rsi,%r11,8),%ymm7
  1e4ed5:	49 c1 ea 03          	shr    $0x3,%r10
  1e4ed9:	4c 8b 74 24 60       	mov    0x60(%rsp),%r14
  1e4ede:	0f 84 22 04 00 00    	je     1e5306 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x876>
  1e4ee4:	48 c1 ef 03          	shr    $0x3,%rdi
  1e4ee8:	4c 89 f6             	mov    %r14,%rsi
  1e4eeb:	48 c1 ee 03          	shr    $0x3,%rsi
  1e4eef:	c4 41 39 57 c0       	vxorpd %xmm8,%xmm8,%xmm8
  1e4ef4:	45 31 db             	xor    %r11d,%r11d
  1e4ef7:	31 db                	xor    %ebx,%ebx
  1e4ef9:	c4 41 21 57 db       	vxorpd %xmm11,%xmm11,%xmm11
  1e4efe:	c4 41 31 57 c9       	vxorpd %xmm9,%xmm9,%xmm9
  1e4f03:	c4 41 01 57 ff       	vxorpd %xmm15,%xmm15,%xmm15
  1e4f08:	c4 41 11 57 ed       	vxorpd %xmm13,%xmm13,%xmm13
  1e4f0d:	c4 41 19 57 e4       	vxorpd %xmm12,%xmm12,%xmm12
  1e4f12:	c4 41 29 57 d2       	vxorpd %xmm10,%xmm10,%xmm10
  1e4f17:	c4 41 09 57 f6       	vxorpd %xmm14,%xmm14,%xmm14
  1e4f1c:	0f 1f 40 00          	nopl   0x0(%rax)
  1e4f20:	48 39 9c 24 90 00 00 	cmp    %rbx,0x90(%rsp)
  1e4f27:	00 
  1e4f28:	0f 84 a3 0b 00 00    	je     1e5ad1 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1041>
  1e4f2e:	48 39 9c 24 30 01 00 	cmp    %rbx,0x130(%rsp)
  1e4f35:	00 
  1e4f36:	0f 84 67 0b 00 00    	je     1e5aa3 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1013>
  1e4f3c:	48 39 9c 24 28 01 00 	cmp    %rbx,0x128(%rsp)
  1e4f43:	00 
  1e4f44:	0f 84 2b 0b 00 00    	je     1e5a75 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xfe5>
  1e4f4a:	48 39 9c 24 20 01 00 	cmp    %rbx,0x120(%rsp)
  1e4f51:	00 
  1e4f52:	0f 84 af 0b 00 00    	je     1e5b07 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1077>
  1e4f58:	48 39 9c 24 18 01 00 	cmp    %rbx,0x118(%rsp)
  1e4f5f:	00 
  1e4f60:	0f 84 86 0b 00 00    	je     1e5aec <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x105c>
  1e4f66:	48 39 df             	cmp    %rbx,%rdi
  1e4f69:	0f 84 4f 0b 00 00    	je     1e5abe <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x102e>
  1e4f6f:	48 39 de             	cmp    %rbx,%rsi
  1e4f72:	0f 84 18 0b 00 00    	je     1e5a90 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1000>
  1e4f78:	48 ff c3             	inc    %rbx
  1e4f7b:	c4 a1 7d 10 44 1d 00 	vmovupd 0x0(%rbp,%r11,1),%ymm0
  1e4f82:	c4 a1 7d 10 54 1d 20 	vmovupd 0x20(%rbp,%r11,1),%ymm2
  1e4f89:	c4 81 7d 59 0c 18    	vmulpd (%r8,%r11,1),%ymm0,%ymm1
  1e4f8f:	c4 81 7d 10 1c 1c    	vmovupd (%r12,%r11,1),%ymm3
  1e4f95:	c5 2d 58 d1          	vaddpd %ymm1,%ymm10,%ymm10
  1e4f99:	c4 81 7d 59 4c 1d 00 	vmulpd 0x0(%r13,%r11,1),%ymm0,%ymm1
  1e4fa0:	c4 81 7d 10 64 1c 20 	vmovupd 0x20(%r12,%r11,1),%ymm4
  1e4fa7:	c5 1d 58 e1          	vaddpd %ymm1,%ymm12,%ymm12
  1e4fab:	c4 81 7d 59 0c 1f    	vmulpd (%r15,%r11,1),%ymm0,%ymm1
  1e4fb1:	c5 e5 59 c9          	vmulpd %ymm1,%ymm3,%ymm1
  1e4fb5:	c5 15 58 e9          	vaddpd %ymm1,%ymm13,%ymm13
  1e4fb9:	c4 a1 7d 59 0c 19    	vmulpd (%rcx,%r11,1),%ymm0,%ymm1
  1e4fbf:	c5 e5 59 c9          	vmulpd %ymm1,%ymm3,%ymm1
  1e4fc3:	c5 05 58 f9          	vaddpd %ymm1,%ymm15,%ymm15
  1e4fc7:	c4 81 6d 59 4c 18 20 	vmulpd 0x20(%r8,%r11,1),%ymm2,%ymm1
  1e4fce:	c5 35 58 c9          	vaddpd %ymm1,%ymm9,%ymm9
  1e4fd2:	c4 81 6d 59 4c 1d 20 	vmulpd 0x20(%r13,%r11,1),%ymm2,%ymm1
  1e4fd9:	c5 25 58 d9          	vaddpd %ymm1,%ymm11,%ymm11
  1e4fdd:	c4 81 6d 59 4c 1f 20 	vmulpd 0x20(%r15,%r11,1),%ymm2,%ymm1
  1e4fe4:	c5 dd 59 c9          	vmulpd %ymm1,%ymm4,%ymm1
  1e4fe8:	c5 3d 58 c1          	vaddpd %ymm1,%ymm8,%ymm8
  1e4fec:	c4 a1 6d 59 4c 19 20 	vmulpd 0x20(%rcx,%r11,1),%ymm2,%ymm1
  1e4ff3:	c5 dd 59 c9          	vmulpd %ymm1,%ymm4,%ymm1
  1e4ff7:	c5 e5 59 db          	vmulpd %ymm3,%ymm3,%ymm3
  1e4ffb:	c4 e2 cd a8 df       	vfmadd213pd %ymm7,%ymm6,%ymm3
  1e5000:	c4 a2 fd a8 1c 18    	vfmadd213pd (%rax,%r11,1),%ymm0,%ymm3
  1e5006:	c4 81 7d 11 1c 19    	vmovupd %ymm3,(%r9,%r11,1)
  1e500c:	c5 dd 59 c4          	vmulpd %ymm4,%ymm4,%ymm0
  1e5010:	c4 e2 cd a8 c7       	vfmadd213pd %ymm7,%ymm6,%ymm0
  1e5015:	c4 a2 ed a8 44 18 20 	vfmadd213pd 0x20(%rax,%r11,1),%ymm2,%ymm0
  1e501c:	c5 0d 58 f1          	vaddpd %ymm1,%ymm14,%ymm14
  1e5020:	c4 81 7d 11 44 19 20 	vmovupd %ymm0,0x20(%r9,%r11,1)
  1e5027:	49 83 c3 40          	add    $0x40,%r11
  1e502b:	49 39 da             	cmp    %rbx,%r10
  1e502e:	0f 85 ec fe ff ff    	jne    1e4f20 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x490>
  1e5034:	f6 44 24 08 04       	testb  $0x4,0x8(%rsp)
  1e5039:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  1e503e:	0f 85 fa 02 00 00    	jne    1e533e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x8ae>
  1e5044:	48 8b 5c 24 70       	mov    0x70(%rsp),%rbx
  1e5049:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
  1e504e:	e9 09 04 00 00       	jmp    1e545c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x9cc>
  1e5053:	c4 41 21 57 db       	vxorpd %xmm11,%xmm11,%xmm11
  1e5058:	c5 c1 57 ff          	vxorpd %xmm7,%xmm7,%xmm7
  1e505c:	c4 41 31 57 c9       	vxorpd %xmm9,%xmm9,%xmm9
  1e5061:	c4 41 19 57 e4       	vxorpd %xmm12,%xmm12,%xmm12
  1e5066:	c4 41 11 57 ed       	vxorpd %xmm13,%xmm13,%xmm13
  1e506b:	c4 41 39 57 c0       	vxorpd %xmm8,%xmm8,%xmm8
  1e5070:	c4 41 29 57 d2       	vxorpd %xmm10,%xmm10,%xmm10
  1e5075:	c5 c9 57 f6          	vxorpd %xmm6,%xmm6,%xmm6
  1e5079:	41 f6 c3 04          	test   $0x4,%r11b
  1e507d:	0f 84 b9 00 00 00    	je     1e513c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x6ac>
  1e5083:	f6 84 24 f0 00 00 00 	testb  $0x4,0xf0(%rsp)
  1e508a:	04 
  1e508b:	0f 84 f2 0c 00 00    	je     1e5d83 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x12f3>
  1e5091:	f6 84 24 e8 00 00 00 	testb  $0x4,0xe8(%rsp)
  1e5098:	04 
  1e5099:	0f 84 d0 0c 00 00    	je     1e5d6f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x12df>
  1e509f:	f6 84 24 e0 00 00 00 	testb  $0x4,0xe0(%rsp)
  1e50a6:	04 
  1e50a7:	0f 84 ae 0c 00 00    	je     1e5d5b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x12cb>
  1e50ad:	f6 84 24 d8 00 00 00 	testb  $0x4,0xd8(%rsp)
  1e50b4:	04 
  1e50b5:	0f 84 8c 0c 00 00    	je     1e5d47 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x12b7>
  1e50bb:	f6 84 24 d0 00 00 00 	testb  $0x4,0xd0(%rsp)
  1e50c2:	04 
  1e50c3:	0f 84 6a 0c 00 00    	je     1e5d33 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x12a3>
  1e50c9:	4c 89 de             	mov    %r11,%rsi
  1e50cc:	48 c1 ee 02          	shr    $0x2,%rsi
  1e50d0:	48 bf fe ff ff ff ff 	movabs $0x3fffffffffffffe,%rdi
  1e50d7:	ff ff 03 
  1e50da:	48 21 fe             	and    %rdi,%rsi
  1e50dd:	48 c1 e6 05          	shl    $0x5,%rsi
  1e50e1:	48 01 ee             	add    %rbp,%rsi
  1e50e4:	c5 fd 10 06          	vmovupd (%rsi),%ymm0
  1e50e8:	48 8b b4 24 a8 00 00 	mov    0xa8(%rsp),%rsi
  1e50ef:	00 
  1e50f0:	c5 fd 10 0e          	vmovupd (%rsi),%ymm1
  1e50f4:	48 8b b4 24 c8 00 00 	mov    0xc8(%rsp),%rsi
  1e50fb:	00 
  1e50fc:	c5 fd 59 16          	vmulpd (%rsi),%ymm0,%ymm2
  1e5100:	c5 c5 58 fa          	vaddpd %ymm2,%ymm7,%ymm7
  1e5104:	48 8b b4 24 c0 00 00 	mov    0xc0(%rsp),%rsi
  1e510b:	00 
  1e510c:	c5 fd 59 16          	vmulpd (%rsi),%ymm0,%ymm2
  1e5110:	c5 35 58 ca          	vaddpd %ymm2,%ymm9,%ymm9
  1e5114:	48 8b b4 24 b8 00 00 	mov    0xb8(%rsp),%rsi
  1e511b:	00 
  1e511c:	c5 fd 59 16          	vmulpd (%rsi),%ymm0,%ymm2
  1e5120:	c5 f5 59 d2          	vmulpd %ymm2,%ymm1,%ymm2
  1e5124:	c5 1d 58 e2          	vaddpd %ymm2,%ymm12,%ymm12
  1e5128:	48 8b b4 24 b0 00 00 	mov    0xb0(%rsp),%rsi
  1e512f:	00 
  1e5130:	c5 fd 59 06          	vmulpd (%rsi),%ymm0,%ymm0
  1e5134:	c5 f5 59 c0          	vmulpd %ymm0,%ymm1,%ymm0
  1e5138:	c5 15 58 e8          	vaddpd %ymm0,%ymm13,%ymm13
  1e513c:	c4 c1 25 58 c5       	vaddpd %ymm13,%ymm11,%ymm0
  1e5141:	c5 9d 58 ce          	vaddpd %ymm6,%ymm12,%ymm1
  1e5145:	c4 c1 2d 58 d1       	vaddpd %ymm9,%ymm10,%ymm2
  1e514a:	c5 bd 58 f7          	vaddpd %ymm7,%ymm8,%ymm6
  1e514e:	44 89 de             	mov    %r11d,%esi
  1e5151:	c4 e3 7d 19 f7 01    	vextractf128 $0x1,%ymm6,%xmm7
  1e5157:	c5 c9 58 f7          	vaddpd %xmm7,%xmm6,%xmm6
  1e515b:	c5 c9 c6 fe 01       	vshufpd $0x1,%xmm6,%xmm6,%xmm7
  1e5160:	c5 4b 58 c7          	vaddsd %xmm7,%xmm6,%xmm8
  1e5164:	c4 e3 7d 19 d6 01    	vextractf128 $0x1,%ymm2,%xmm6
  1e516a:	c5 e9 58 d6          	vaddpd %xmm6,%xmm2,%xmm2
  1e516e:	c5 e9 c6 f2 01       	vshufpd $0x1,%xmm2,%xmm2,%xmm6
  1e5173:	c5 6b 58 d6          	vaddsd %xmm6,%xmm2,%xmm10
  1e5177:	c4 e3 7d 19 ca 01    	vextractf128 $0x1,%ymm1,%xmm2
  1e517d:	c5 f1 58 ca          	vaddpd %xmm2,%xmm1,%xmm1
  1e5181:	c5 f1 c6 d1 01       	vshufpd $0x1,%xmm1,%xmm1,%xmm2
  1e5186:	c5 73 58 ca          	vaddsd %xmm2,%xmm1,%xmm9
  1e518a:	c4 e3 7d 19 c1 01    	vextractf128 $0x1,%ymm0,%xmm1
  1e5190:	c5 f9 58 c1          	vaddpd %xmm1,%xmm0,%xmm0
  1e5194:	c5 f9 c6 c8 01       	vshufpd $0x1,%xmm0,%xmm0,%xmm1
  1e5199:	c5 7b 58 d9          	vaddsd %xmm1,%xmm0,%xmm11
  1e519d:	83 e6 03             	and    $0x3,%esi
  1e51a0:	0f 84 f1 05 00 00    	je     1e5797 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xd07>
  1e51a6:	48 8d 7e ff          	lea    -0x1(%rsi),%rdi
  1e51aa:	4c 8b 9c 24 40 01 00 	mov    0x140(%rsp),%r11
  1e51b1:	00 
  1e51b2:	4c 39 df             	cmp    %r11,%rdi
  1e51b5:	49 0f 43 fb          	cmovae %r11,%rdi
  1e51b9:	39 7c 24 48          	cmp    %edi,0x48(%rsp)
  1e51bd:	0f 84 cc 0a 00 00    	je     1e5c8f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x11ff>
  1e51c3:	39 7c 24 38          	cmp    %edi,0x38(%rsp)
  1e51c7:	0f 84 aa 0a 00 00    	je     1e5c77 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x11e7>
  1e51cd:	39 7c 24 30          	cmp    %edi,0x30(%rsp)
  1e51d1:	4c 8b 5c 24 08       	mov    0x8(%rsp),%r11
  1e51d6:	0f 84 83 0a 00 00    	je     1e5c5f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x11cf>
  1e51dc:	39 7c 24 40          	cmp    %edi,0x40(%rsp)
  1e51e0:	0f 84 61 0a 00 00    	je     1e5c47 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x11b7>
  1e51e6:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  1e51eb:	39 7c 24 28          	cmp    %edi,0x28(%rsp)
  1e51ef:	0f 84 3a 0a 00 00    	je     1e5c2f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x119f>
  1e51f5:	4c 89 df             	mov    %r11,%rdi
  1e51f8:	49 bb fc ff ff ff ff 	movabs $0xffffffffffffffc,%r11
  1e51ff:	ff ff 0f 
  1e5202:	4c 21 df             	and    %r11,%rdi
  1e5205:	c5 fb 10 44 fd 00    	vmovsd 0x0(%rbp,%rdi,8),%xmm0
  1e520b:	4c 8b 5c 24 70       	mov    0x70(%rsp),%r11
  1e5210:	c4 81 7b 59 0c d8    	vmulsd (%r8,%r11,8),%xmm0,%xmm1
  1e5216:	4c 8b 74 24 50       	mov    0x50(%rsp),%r14
  1e521b:	c4 81 7b 59 54 f5 00 	vmulsd 0x0(%r13,%r14,8),%xmm0,%xmm2
  1e5222:	c5 3b 58 c1          	vaddsd %xmm1,%xmm8,%xmm8
  1e5226:	c5 2b 58 d2          	vaddsd %xmm2,%xmm10,%xmm10
  1e522a:	48 8b 54 24 78       	mov    0x78(%rsp),%rdx
  1e522f:	c4 c1 7b 59 0c d7    	vmulsd (%r15,%rdx,8),%xmm0,%xmm1
  1e5235:	48 8b 9c 24 80 00 00 	mov    0x80(%rsp),%rbx
  1e523c:	00 
  1e523d:	c4 c1 7b 10 14 dc    	vmovsd (%r12,%rbx,8),%xmm2
  1e5243:	c5 f3 59 ca          	vmulsd %xmm2,%xmm1,%xmm1
  1e5247:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
  1e524c:	c4 a1 7b 59 04 f1    	vmulsd (%rcx,%r14,8),%xmm0,%xmm0
  1e5252:	c5 33 58 c9          	vaddsd %xmm1,%xmm9,%xmm9
  1e5256:	c5 eb 59 c0          	vmulsd %xmm0,%xmm2,%xmm0
  1e525a:	c5 23 58 d8          	vaddsd %xmm0,%xmm11,%xmm11
  1e525e:	83 fe 01             	cmp    $0x1,%esi
  1e5261:	0f 84 21 05 00 00    	je     1e5788 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xcf8>
  1e5267:	c5 fb 10 44 fd 08    	vmovsd 0x8(%rbp,%rdi,8),%xmm0
  1e526d:	c4 81 7b 59 4c d8 08 	vmulsd 0x8(%r8,%r11,8),%xmm0,%xmm1
  1e5274:	4c 8b 74 24 50       	mov    0x50(%rsp),%r14
  1e5279:	c4 81 7b 59 54 f5 08 	vmulsd 0x8(%r13,%r14,8),%xmm0,%xmm2
  1e5280:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
  1e5285:	c5 3b 58 c1          	vaddsd %xmm1,%xmm8,%xmm8
  1e5289:	c5 2b 58 d2          	vaddsd %xmm2,%xmm10,%xmm10
  1e528d:	c4 c1 7b 59 4c d7 08 	vmulsd 0x8(%r15,%rdx,8),%xmm0,%xmm1
  1e5294:	c4 c1 7b 10 54 dc 08 	vmovsd 0x8(%r12,%rbx,8),%xmm2
  1e529b:	c5 f3 59 ca          	vmulsd %xmm2,%xmm1,%xmm1
  1e529f:	c4 a1 7b 59 44 f1 08 	vmulsd 0x8(%rcx,%r14,8),%xmm0,%xmm0
  1e52a6:	c5 33 58 c9          	vaddsd %xmm1,%xmm9,%xmm9
  1e52aa:	c5 eb 59 c0          	vmulsd %xmm0,%xmm2,%xmm0
  1e52ae:	c5 23 58 d8          	vaddsd %xmm0,%xmm11,%xmm11
  1e52b2:	83 fe 02             	cmp    $0x2,%esi
  1e52b5:	0f 84 cd 04 00 00    	je     1e5788 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xcf8>
  1e52bb:	c5 fb 10 44 fd 10    	vmovsd 0x10(%rbp,%rdi,8),%xmm0
  1e52c1:	c4 81 7b 59 4c d8 10 	vmulsd 0x10(%r8,%r11,8),%xmm0,%xmm1
  1e52c8:	c5 3b 58 c1          	vaddsd %xmm1,%xmm8,%xmm8
  1e52cc:	48 8b 74 24 50       	mov    0x50(%rsp),%rsi
  1e52d1:	c4 c1 7b 59 4c f5 10 	vmulsd 0x10(%r13,%rsi,8),%xmm0,%xmm1
  1e52d8:	c4 c1 7b 59 54 d7 10 	vmulsd 0x10(%r15,%rdx,8),%xmm0,%xmm2
  1e52df:	c5 2b 58 d1          	vaddsd %xmm1,%xmm10,%xmm10
  1e52e3:	c4 c1 7b 10 4c dc 10 	vmovsd 0x10(%r12,%rbx,8),%xmm1
  1e52ea:	c5 eb 59 d1          	vmulsd %xmm1,%xmm2,%xmm2
  1e52ee:	c5 33 58 ca          	vaddsd %xmm2,%xmm9,%xmm9
  1e52f2:	c4 a1 7b 59 44 f1 10 	vmulsd 0x10(%rcx,%r14,8),%xmm0,%xmm0
  1e52f9:	c5 f3 59 c0          	vmulsd %xmm0,%xmm1,%xmm0
  1e52fd:	c5 23 58 d8          	vaddsd %xmm0,%xmm11,%xmm11
  1e5301:	e9 82 04 00 00       	jmp    1e5788 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xcf8>
  1e5306:	c4 41 09 57 f6       	vxorpd %xmm14,%xmm14,%xmm14
  1e530b:	c4 41 29 57 d2       	vxorpd %xmm10,%xmm10,%xmm10
  1e5310:	c4 41 19 57 e4       	vxorpd %xmm12,%xmm12,%xmm12
  1e5315:	c4 41 11 57 ed       	vxorpd %xmm13,%xmm13,%xmm13
  1e531a:	c4 41 01 57 ff       	vxorpd %xmm15,%xmm15,%xmm15
  1e531f:	c4 41 31 57 c9       	vxorpd %xmm9,%xmm9,%xmm9
  1e5324:	c4 41 21 57 db       	vxorpd %xmm11,%xmm11,%xmm11
  1e5329:	c4 41 39 57 c0       	vxorpd %xmm8,%xmm8,%xmm8
  1e532e:	f6 44 24 08 04       	testb  $0x4,0x8(%rsp)
  1e5333:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  1e5338:	0f 84 06 fd ff ff    	je     1e5044 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x5b4>
  1e533e:	f6 84 24 f0 00 00 00 	testb  $0x4,0xf0(%rsp)
  1e5345:	04 
  1e5346:	4c 8b 5c 24 70       	mov    0x70(%rsp),%r11
  1e534b:	0f 84 ce 09 00 00    	je     1e5d1f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x128f>
  1e5351:	f6 84 24 e8 00 00 00 	testb  $0x4,0xe8(%rsp)
  1e5358:	04 
  1e5359:	48 8b b4 24 88 00 00 	mov    0x88(%rsp),%rsi
  1e5360:	00 
  1e5361:	0f 84 a4 09 00 00    	je     1e5d0b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x127b>
  1e5367:	f6 84 24 e0 00 00 00 	testb  $0x4,0xe0(%rsp)
  1e536e:	04 
  1e536f:	0f 84 82 09 00 00    	je     1e5cf7 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1267>
  1e5375:	f6 84 24 d8 00 00 00 	testb  $0x4,0xd8(%rsp)
  1e537c:	04 
  1e537d:	0f 84 60 09 00 00    	je     1e5ce3 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1253>
  1e5383:	f6 84 24 d0 00 00 00 	testb  $0x4,0xd0(%rsp)
  1e538a:	04 
  1e538b:	0f 84 3e 09 00 00    	je     1e5ccf <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x123f>
  1e5391:	40 f6 c6 04          	test   $0x4,%sil
  1e5395:	0f 84 20 09 00 00    	je     1e5cbb <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x122b>
  1e539b:	41 f6 c6 04          	test   $0x4,%r14b
  1e539f:	0f 84 02 09 00 00    	je     1e5ca7 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1217>
  1e53a5:	4c 89 f7             	mov    %r14,%rdi
  1e53a8:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
  1e53ad:	4d 89 f2             	mov    %r14,%r10
  1e53b0:	49 c1 ea 02          	shr    $0x2,%r10
  1e53b4:	48 c1 ee 02          	shr    $0x2,%rsi
  1e53b8:	48 c1 ef 02          	shr    $0x2,%rdi
  1e53bc:	48 bb fe ff ff ff ff 	movabs $0x3fffffffffffffe,%rbx
  1e53c3:	ff ff 03 
  1e53c6:	49 21 da             	and    %rbx,%r10
  1e53c9:	49 c1 e2 05          	shl    $0x5,%r10
  1e53cd:	49 01 ea             	add    %rbp,%r10
  1e53d0:	48 21 de             	and    %rbx,%rsi
  1e53d3:	48 c1 e6 05          	shl    $0x5,%rsi
  1e53d7:	48 01 c6             	add    %rax,%rsi
  1e53da:	48 21 df             	and    %rbx,%rdi
  1e53dd:	c4 c1 7d 10 02       	vmovupd (%r10),%ymm0
  1e53e2:	4c 8b 94 24 c8 00 00 	mov    0xc8(%rsp),%r10
  1e53e9:	00 
  1e53ea:	c4 c1 7d 59 0a       	vmulpd (%r10),%ymm0,%ymm1
  1e53ef:	48 c1 e7 05          	shl    $0x5,%rdi
  1e53f3:	c5 2d 58 d1          	vaddpd %ymm1,%ymm10,%ymm10
  1e53f7:	4c 8b 94 24 c0 00 00 	mov    0xc0(%rsp),%r10
  1e53fe:	00 
  1e53ff:	c4 c1 7d 59 0a       	vmulpd (%r10),%ymm0,%ymm1
  1e5404:	4c 8b 94 24 a8 00 00 	mov    0xa8(%rsp),%r10
  1e540b:	00 
  1e540c:	c4 c1 7d 10 12       	vmovupd (%r10),%ymm2
  1e5411:	c5 1d 58 e1          	vaddpd %ymm1,%ymm12,%ymm12
  1e5415:	4c 8b 94 24 b8 00 00 	mov    0xb8(%rsp),%r10
  1e541c:	00 
  1e541d:	c4 c1 7d 59 0a       	vmulpd (%r10),%ymm0,%ymm1
  1e5422:	c5 ed 59 c9          	vmulpd %ymm1,%ymm2,%ymm1
  1e5426:	c5 15 58 e9          	vaddpd %ymm1,%ymm13,%ymm13
  1e542a:	4c 8b 94 24 b0 00 00 	mov    0xb0(%rsp),%r10
  1e5431:	00 
  1e5432:	c4 c1 7d 59 0a       	vmulpd (%r10),%ymm0,%ymm1
  1e5437:	4c 01 cf             	add    %r9,%rdi
  1e543a:	c5 ed 59 c9          	vmulpd %ymm1,%ymm2,%ymm1
  1e543e:	c5 05 58 f9          	vaddpd %ymm1,%ymm15,%ymm15
  1e5442:	c5 ed 59 ca          	vmulpd %ymm2,%ymm2,%ymm1
  1e5446:	c4 e2 cd a8 cf       	vfmadd213pd %ymm7,%ymm6,%ymm1
  1e544b:	c4 e2 fd a8 0e       	vfmadd213pd (%rsi),%ymm0,%ymm1
  1e5450:	c5 fd 11 0f          	vmovupd %ymm1,(%rdi)
  1e5454:	48 8b 54 24 10       	mov    0x10(%rsp),%rdx
  1e5459:	4c 89 db             	mov    %r11,%rbx
  1e545c:	c4 c1 0d 58 c7       	vaddpd %ymm15,%ymm14,%ymm0
  1e5461:	c4 c1 3d 58 cd       	vaddpd %ymm13,%ymm8,%ymm1
  1e5466:	c4 c1 25 58 d4       	vaddpd %ymm12,%ymm11,%ymm2
  1e546b:	c4 41 35 58 c2       	vaddpd %ymm10,%ymm9,%ymm8
  1e5470:	45 89 f2             	mov    %r14d,%r10d
  1e5473:	c4 43 7d 19 c1 01    	vextractf128 $0x1,%ymm8,%xmm9
  1e5479:	c4 41 39 58 c1       	vaddpd %xmm9,%xmm8,%xmm8
  1e547e:	c4 41 39 c6 c8 01    	vshufpd $0x1,%xmm8,%xmm8,%xmm9
  1e5484:	c4 41 3b 58 c1       	vaddsd %xmm9,%xmm8,%xmm8
  1e5489:	c4 c3 7d 19 d1 01    	vextractf128 $0x1,%ymm2,%xmm9
  1e548f:	c5 b1 58 d2          	vaddpd %xmm2,%xmm9,%xmm2
  1e5493:	c5 69 c6 ca 01       	vshufpd $0x1,%xmm2,%xmm2,%xmm9
  1e5498:	c5 33 58 d2          	vaddsd %xmm2,%xmm9,%xmm10
  1e549c:	c4 e3 7d 19 ca 01    	vextractf128 $0x1,%ymm1,%xmm2
  1e54a2:	c5 f1 58 ca          	vaddpd %xmm2,%xmm1,%xmm1
  1e54a6:	c5 f1 c6 d1 01       	vshufpd $0x1,%xmm1,%xmm1,%xmm2
  1e54ab:	c5 73 58 ca          	vaddsd %xmm2,%xmm1,%xmm9
  1e54af:	c4 e3 7d 19 c1 01    	vextractf128 $0x1,%ymm0,%xmm1
  1e54b5:	c5 f9 58 c1          	vaddpd %xmm1,%xmm0,%xmm0
  1e54b9:	c5 f9 c6 c8 01       	vshufpd $0x1,%xmm0,%xmm0,%xmm1
  1e54be:	c5 7b 58 d9          	vaddsd %xmm1,%xmm0,%xmm11
  1e54c2:	41 83 e2 03          	and    $0x3,%r10d
  1e54c6:	0f 84 9b 04 00 00    	je     1e5967 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xed7>
  1e54cc:	48 83 7c 24 40 00    	cmpq   $0x0,0x40(%rsp)
  1e54d2:	4c 8b 74 24 60       	mov    0x60(%rsp),%r14
  1e54d7:	0f 84 f1 06 00 00    	je     1e5bce <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x113e>
  1e54dd:	48 83 7c 24 48 00    	cmpq   $0x0,0x48(%rsp)
  1e54e3:	0f 84 cd 06 00 00    	je     1e5bb6 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1126>
  1e54e9:	48 83 7c 24 38 00    	cmpq   $0x0,0x38(%rsp)
  1e54ef:	0f 84 a9 06 00 00    	je     1e5b9e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x110e>
  1e54f5:	48 83 7c 24 30 00    	cmpq   $0x0,0x30(%rsp)
  1e54fb:	0f 84 85 06 00 00    	je     1e5b86 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10f6>
  1e5501:	48 83 7c 24 28 00    	cmpq   $0x0,0x28(%rsp)
  1e5507:	0f 84 61 06 00 00    	je     1e5b6e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10de>
  1e550d:	48 8b b4 24 88 00 00 	mov    0x88(%rsp),%rsi
  1e5514:	00 
  1e5515:	83 e6 03             	and    $0x3,%esi
  1e5518:	0f 84 3d 06 00 00    	je     1e5b5b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10cb>
  1e551e:	44 89 f7             	mov    %r14d,%edi
  1e5521:	83 e7 03             	and    $0x3,%edi
  1e5524:	0f 84 1e 06 00 00    	je     1e5b48 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10b8>
  1e552a:	48 8b 54 24 08       	mov    0x8(%rsp),%rdx
  1e552f:	49 bb fc ff ff ff ff 	movabs $0xffffffffffffffc,%r11
  1e5536:	ff ff 0f 
  1e5539:	4c 21 da             	and    %r11,%rdx
  1e553c:	4c 89 94 24 38 01 00 	mov    %r10,0x138(%rsp)
  1e5543:	00 
  1e5544:	4c 8b 94 24 88 00 00 	mov    0x88(%rsp),%r10
  1e554b:	00 
  1e554c:	4d 21 da             	and    %r11,%r10
  1e554f:	48 89 94 24 98 00 00 	mov    %rdx,0x98(%rsp)
  1e5556:	00 
  1e5557:	c5 fb 10 44 d5 00    	vmovsd 0x0(%rbp,%rdx,8),%xmm0
  1e555d:	c4 c1 7b 59 0c d8    	vmulsd (%r8,%rbx,8),%xmm0,%xmm1
  1e5563:	4d 21 de             	and    %r11,%r14
  1e5566:	c5 3b 58 c1          	vaddsd %xmm1,%xmm8,%xmm8
  1e556a:	4c 8b 5c 24 50       	mov    0x50(%rsp),%r11
  1e556f:	c4 81 7b 59 4c dd 00 	vmulsd 0x0(%r13,%r11,8),%xmm0,%xmm1
  1e5576:	4c 8b 9c 24 80 00 00 	mov    0x80(%rsp),%r11
  1e557d:	00 
  1e557e:	c4 81 7b 10 14 dc    	vmovsd (%r12,%r11,8),%xmm2
  1e5584:	c5 2b 58 d1          	vaddsd %xmm1,%xmm10,%xmm10
  1e5588:	48 8b 54 24 78       	mov    0x78(%rsp),%rdx
  1e558d:	c4 c1 7b 59 0c d7    	vmulsd (%r15,%rdx,8),%xmm0,%xmm1
  1e5593:	c5 eb 59 c9          	vmulsd %xmm1,%xmm2,%xmm1
  1e5597:	c5 33 58 c9          	vaddsd %xmm1,%xmm9,%xmm9
  1e559b:	4c 8b 5c 24 68       	mov    0x68(%rsp),%r11
  1e55a0:	c4 a1 7b 59 0c d9    	vmulsd (%rcx,%r11,8),%xmm0,%xmm1
  1e55a6:	c5 eb 59 c9          	vmulsd %xmm1,%xmm2,%xmm1
  1e55aa:	c5 23 58 d9          	vaddsd %xmm1,%xmm11,%xmm11
  1e55ae:	c5 eb 59 ca          	vmulsd %xmm2,%xmm2,%xmm1
  1e55b2:	c4 e2 c9 a9 cf       	vfmadd213sd %xmm7,%xmm6,%xmm1
  1e55b7:	4c 89 94 24 a0 00 00 	mov    %r10,0xa0(%rsp)
  1e55be:	00 
  1e55bf:	c4 a2 f9 a9 0c d0    	vfmadd213sd (%rax,%r10,8),%xmm0,%xmm1
  1e55c5:	4c 8b 94 24 38 01 00 	mov    0x138(%rsp),%r10
  1e55cc:	00 
  1e55cd:	c4 81 7b 11 0c f1    	vmovsd %xmm1,(%r9,%r14,8)
  1e55d3:	41 83 fa 01          	cmp    $0x1,%r10d
  1e55d7:	0f 84 a6 01 00 00    	je     1e5783 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xcf3>
  1e55dd:	83 7c 24 40 01       	cmpl   $0x1,0x40(%rsp)
  1e55e2:	4c 8b 5c 24 70       	mov    0x70(%rsp),%r11
  1e55e7:	0f 84 e1 05 00 00    	je     1e5bce <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x113e>
  1e55ed:	83 7c 24 48 01       	cmpl   $0x1,0x48(%rsp)
  1e55f2:	0f 84 be 05 00 00    	je     1e5bb6 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1126>
  1e55f8:	83 7c 24 38 01       	cmpl   $0x1,0x38(%rsp)
  1e55fd:	0f 84 9b 05 00 00    	je     1e5b9e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x110e>
  1e5603:	83 7c 24 30 01       	cmpl   $0x1,0x30(%rsp)
  1e5608:	0f 84 78 05 00 00    	je     1e5b86 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10f6>
  1e560e:	83 7c 24 28 01       	cmpl   $0x1,0x28(%rsp)
  1e5613:	0f 84 55 05 00 00    	je     1e5b6e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10de>
  1e5619:	83 fe 01             	cmp    $0x1,%esi
  1e561c:	0f 84 39 05 00 00    	je     1e5b5b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10cb>
  1e5622:	83 ff 01             	cmp    $0x1,%edi
  1e5625:	0f 84 1d 05 00 00    	je     1e5b48 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10b8>
  1e562b:	48 8b 94 24 98 00 00 	mov    0x98(%rsp),%rdx
  1e5632:	00 
  1e5633:	c5 fb 10 44 d5 08    	vmovsd 0x8(%rbp,%rdx,8),%xmm0
  1e5639:	c4 81 7b 59 4c d8 08 	vmulsd 0x8(%r8,%r11,8),%xmm0,%xmm1
  1e5640:	c5 3b 58 c1          	vaddsd %xmm1,%xmm8,%xmm8
  1e5644:	4d 89 f3             	mov    %r14,%r11
  1e5647:	4c 8b 74 24 50       	mov    0x50(%rsp),%r14
  1e564c:	c4 81 7b 59 4c f5 08 	vmulsd 0x8(%r13,%r14,8),%xmm0,%xmm1
  1e5653:	48 8b 9c 24 80 00 00 	mov    0x80(%rsp),%rbx
  1e565a:	00 
  1e565b:	c4 c1 7b 10 54 dc 08 	vmovsd 0x8(%r12,%rbx,8),%xmm2
  1e5662:	c5 2b 58 d1          	vaddsd %xmm1,%xmm10,%xmm10
  1e5666:	48 8b 54 24 78       	mov    0x78(%rsp),%rdx
  1e566b:	c4 c1 7b 59 4c d7 08 	vmulsd 0x8(%r15,%rdx,8),%xmm0,%xmm1
  1e5672:	c5 eb 59 c9          	vmulsd %xmm1,%xmm2,%xmm1
  1e5676:	c5 33 58 c9          	vaddsd %xmm1,%xmm9,%xmm9
  1e567a:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
  1e567f:	c4 a1 7b 59 4c f1 08 	vmulsd 0x8(%rcx,%r14,8),%xmm0,%xmm1
  1e5686:	c5 eb 59 c9          	vmulsd %xmm1,%xmm2,%xmm1
  1e568a:	c5 23 58 d9          	vaddsd %xmm1,%xmm11,%xmm11
  1e568e:	c5 eb 59 ca          	vmulsd %xmm2,%xmm2,%xmm1
  1e5692:	c4 e2 c9 a9 cf       	vfmadd213sd %xmm7,%xmm6,%xmm1
  1e5697:	48 8b 94 24 a0 00 00 	mov    0xa0(%rsp),%rdx
  1e569e:	00 
  1e569f:	c4 e2 f9 a9 4c d0 08 	vfmadd213sd 0x8(%rax,%rdx,8),%xmm0,%xmm1
  1e56a6:	c4 81 7b 11 4c d9 08 	vmovsd %xmm1,0x8(%r9,%r11,8)
  1e56ad:	41 83 fa 02          	cmp    $0x2,%r10d
  1e56b1:	0f 84 cc 00 00 00    	je     1e5783 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xcf3>
  1e56b7:	83 7c 24 40 02       	cmpl   $0x2,0x40(%rsp)
  1e56bc:	4c 8b 54 24 70       	mov    0x70(%rsp),%r10
  1e56c1:	0f 84 07 05 00 00    	je     1e5bce <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x113e>
  1e56c7:	83 7c 24 48 02       	cmpl   $0x2,0x48(%rsp)
  1e56cc:	0f 84 e4 04 00 00    	je     1e5bb6 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1126>
  1e56d2:	83 7c 24 38 02       	cmpl   $0x2,0x38(%rsp)
  1e56d7:	0f 84 c1 04 00 00    	je     1e5b9e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x110e>
  1e56dd:	83 7c 24 30 02       	cmpl   $0x2,0x30(%rsp)
  1e56e2:	0f 84 9e 04 00 00    	je     1e5b86 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10f6>
  1e56e8:	83 7c 24 28 02       	cmpl   $0x2,0x28(%rsp)
  1e56ed:	0f 84 7b 04 00 00    	je     1e5b6e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10de>
  1e56f3:	83 fe 02             	cmp    $0x2,%esi
  1e56f6:	0f 84 5f 04 00 00    	je     1e5b5b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10cb>
  1e56fc:	83 ff 02             	cmp    $0x2,%edi
  1e56ff:	0f 84 43 04 00 00    	je     1e5b48 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10b8>
  1e5705:	48 8b 94 24 98 00 00 	mov    0x98(%rsp),%rdx
  1e570c:	00 
  1e570d:	c5 fb 10 44 d5 10    	vmovsd 0x10(%rbp,%rdx,8),%xmm0
  1e5713:	48 8b b4 24 80 00 00 	mov    0x80(%rsp),%rsi
  1e571a:	00 
  1e571b:	c4 c1 7b 10 4c f4 10 	vmovsd 0x10(%r12,%rsi,8),%xmm1
  1e5722:	c5 f3 59 d1          	vmulsd %xmm1,%xmm1,%xmm2
  1e5726:	c4 e2 c9 a9 d7       	vfmadd213sd %xmm7,%xmm6,%xmm2
  1e572b:	48 8b 94 24 a0 00 00 	mov    0xa0(%rsp),%rdx
  1e5732:	00 
  1e5733:	c4 e2 f9 a9 54 d0 10 	vfmadd213sd 0x10(%rax,%rdx,8),%xmm0,%xmm2
  1e573a:	c4 81 7b 59 74 d0 10 	vmulsd 0x10(%r8,%r10,8),%xmm0,%xmm6
  1e5741:	48 8b 74 24 50       	mov    0x50(%rsp),%rsi
  1e5746:	c4 c1 7b 59 7c f5 10 	vmulsd 0x10(%r13,%rsi,8),%xmm0,%xmm7
  1e574d:	48 8b 54 24 78       	mov    0x78(%rsp),%rdx
  1e5752:	c4 41 7b 59 64 d7 10 	vmulsd 0x10(%r15,%rdx,8),%xmm0,%xmm12
  1e5759:	48 8b 74 24 68       	mov    0x68(%rsp),%rsi
  1e575e:	c5 fb 59 44 f1 10    	vmulsd 0x10(%rcx,%rsi,8),%xmm0,%xmm0
  1e5764:	c4 81 7b 11 54 d9 10 	vmovsd %xmm2,0x10(%r9,%r11,8)
  1e576b:	c5 3b 58 c6          	vaddsd %xmm6,%xmm8,%xmm8
  1e576f:	c5 2b 58 d7          	vaddsd %xmm7,%xmm10,%xmm10
  1e5773:	c5 9b 59 d1          	vmulsd %xmm1,%xmm12,%xmm2
  1e5777:	c5 33 58 ca          	vaddsd %xmm2,%xmm9,%xmm9
  1e577b:	c5 f3 59 c0          	vmulsd %xmm0,%xmm1,%xmm0
  1e577f:	c5 23 58 d8          	vaddsd %xmm0,%xmm11,%xmm11
  1e5783:	4c 8b 54 24 58       	mov    0x58(%rsp),%r10
  1e5788:	48 8b 5c 24 20       	mov    0x20(%rsp),%rbx
  1e578d:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
  1e5792:	48 8b 54 24 10       	mov    0x10(%rsp),%rdx
  1e5797:	48 89 d7             	mov    %rdx,%rdi
  1e579a:	48 2b bc 24 10 01 00 	sub    0x110(%rsp),%rdi
  1e57a1:	00 
  1e57a2:	48 39 df             	cmp    %rbx,%rdi
  1e57a5:	0f 83 8a 03 00 00    	jae    1e5b35 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10a5>
  1e57ab:	48 89 fe             	mov    %rdi,%rsi
  1e57ae:	48 c1 e6 04          	shl    $0x4,%rsi
  1e57b2:	c4 41 7b 11 04 36    	vmovsd %xmm8,(%r14,%rsi,1)
  1e57b8:	c4 41 7b 11 54 36 08 	vmovsd %xmm10,0x8(%r14,%rsi,1)
  1e57bf:	4c 39 d2             	cmp    %r10,%rdx
  1e57c2:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1e57c7:	73 1d                	jae    1e57e6 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xd56>
  1e57c9:	48 ff c7             	inc    %rdi
  1e57cc:	48 39 df             	cmp    %rbx,%rdi
  1e57cf:	0f 83 11 04 00 00    	jae    1e5be6 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1156>
  1e57d5:	48 c1 e7 04          	shl    $0x4,%rdi
  1e57d9:	c4 41 7b 11 0c 3e    	vmovsd %xmm9,(%r14,%rdi,1)
  1e57df:	c4 41 7b 11 5c 3e 08 	vmovsd %xmm11,0x8(%r14,%rdi,1)
  1e57e6:	4c 39 94 24 60 01 00 	cmp    %r10,0x160(%rsp)
  1e57ed:	00 
  1e57ee:	0f 87 6a 02 00 00    	ja     1e5a5e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xfce>
  1e57f4:	48 c1 ee 02          	shr    $0x2,%rsi
  1e57f8:	c5 fb 10 35 20 e1 e3 	vmovsd -0x1c1ee0(%rip),%xmm6        # 23920 <anon.1d01855f4677b114ff07ecd502d0759a.8.llvm.5874686521581109774+0x1d0>
  1e57ff:	ff 
  1e5800:	c4 e2 7d 19 3d a7 e0 	vbroadcastsd -0x1c1f59(%rip),%ymm7        # 238b0 <anon.1d01855f4677b114ff07ecd502d0759a.8.llvm.5874686521581109774+0x160>
  1e5807:	e3 ff 
  1e5809:	0f 84 8c 00 00 00    	je     1e589b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe0b>
  1e580f:	48 8b 7c 24 60       	mov    0x60(%rsp),%rdi
  1e5814:	48 c1 ef 02          	shr    $0x2,%rdi
  1e5818:	45 31 d2             	xor    %r10d,%r10d
  1e581b:	45 31 db             	xor    %r11d,%r11d
  1e581e:	66 90                	xchg   %ax,%ax
  1e5820:	4c 39 df             	cmp    %r11,%rdi
  1e5823:	0f 84 f9 02 00 00    	je     1e5b22 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x1092>
  1e5829:	c4 e2 7d 19 05 f6 df 	vbroadcastsd -0x1c200a(%rip),%ymm0        # 23828 <anon.1d01855f4677b114ff07ecd502d0759a.8.llvm.5874686521581109774+0xd8>
  1e5830:	e3 ff 
  1e5832:	c4 a1 7d 54 4c 15 00 	vandpd 0x0(%rbp,%r10,1),%ymm0,%ymm1
  1e5839:	c4 81 7d 54 04 11    	vandpd (%r9,%r10,1),%ymm0,%ymm0
  1e583f:	c5 f5 c2 d1 00       	vcmpeqpd %ymm1,%ymm1,%ymm2
  1e5844:	c5 fd c2 d8 00       	vcmpeqpd %ymm0,%ymm0,%ymm3
  1e5849:	c5 f5 5f e0          	vmaxpd %ymm0,%ymm1,%ymm4
  1e584d:	c4 e3 75 4b cc 30    	vblendvpd %ymm3,%ymm4,%ymm1,%ymm1
  1e5853:	c4 e3 7d 4b c1 20    	vblendvpd %ymm2,%ymm1,%ymm0,%ymm0
  1e5859:	c5 fd c2 cf 12       	vcmple_oqpd %ymm7,%ymm0,%ymm1
  1e585e:	c5 fd c2 d5 00       	vcmpeqpd %ymm5,%ymm0,%ymm2
  1e5863:	c4 e3 7d 4b c7 20    	vblendvpd %ymm2,%ymm7,%ymm0,%ymm0
  1e5869:	c4 e2 55 37 c9       	vpcmpgtq %ymm1,%ymm5,%ymm1
  1e586e:	c5 f5 db c0          	vpand  %ymm0,%ymm1,%ymm0
  1e5872:	c4 e3 7d 39 c1 01    	vextracti128 $0x1,%ymm0,%xmm1
  1e5878:	c5 f9 5d c1          	vminpd %xmm1,%xmm0,%xmm0
  1e587c:	c5 f9 c6 c8 01       	vshufpd $0x1,%xmm0,%xmm0,%xmm1
  1e5881:	c5 fb 5d c1          	vminsd %xmm1,%xmm0,%xmm0
  1e5885:	c5 f9 2e f0          	vucomisd %xmm0,%xmm6
  1e5889:	0f 87 cb 01 00 00    	ja     1e5a5a <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xfca>
  1e588f:	49 ff c3             	inc    %r11
  1e5892:	49 83 c2 20          	add    $0x20,%r10
  1e5896:	4c 39 de             	cmp    %r11,%rsi
  1e5899:	75 85                	jne    1e5820 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xd90>
  1e589b:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1e58a0:	83 e6 03             	and    $0x3,%esi
  1e58a3:	48 8b 7c 24 60       	mov    0x60(%rsp),%rdi
  1e58a8:	83 e7 03             	and    $0x3,%edi
  1e58ab:	48 39 f7             	cmp    %rsi,%rdi
  1e58ae:	48 0f 42 f7          	cmovb  %rdi,%rsi
  1e58b2:	48 85 f6             	test   %rsi,%rsi
  1e58b5:	74 71                	je     1e5928 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe98>
  1e58b7:	48 8b 7c 24 08       	mov    0x8(%rsp),%rdi
  1e58bc:	49 bb fc ff ff ff ff 	movabs $0xffffffffffffffc,%r11
  1e58c3:	ff ff 0f 
  1e58c6:	4c 21 df             	and    %r11,%rdi
  1e58c9:	4c 8b 54 24 60       	mov    0x60(%rsp),%r10
  1e58ce:	4d 21 da             	and    %r11,%r10
  1e58d1:	c5 fb 10 44 fd 00    	vmovsd 0x0(%rbp,%rdi,8),%xmm0
  1e58d7:	c5 fb 12 15 49 df e3 	vmovddup -0x1c20b7(%rip),%xmm2        # 23828 <anon.1d01855f4677b114ff07ecd502d0759a.8.llvm.5874686521581109774+0xd8>
  1e58de:	ff 
  1e58df:	c5 f9 54 c2          	vandpd %xmm2,%xmm0,%xmm0
  1e58e3:	c4 81 7b 10 0c d1    	vmovsd (%r9,%r10,8),%xmm1
  1e58e9:	c5 f1 54 ca          	vandpd %xmm2,%xmm1,%xmm1
  1e58ed:	c5 fb 5f d1          	vmaxsd %xmm1,%xmm0,%xmm2
  1e58f1:	c5 f3 c2 c9 03       	vcmpunordsd %xmm1,%xmm1,%xmm1
  1e58f6:	c4 e3 69 4b d0 10    	vblendvpd %xmm1,%xmm0,%xmm2,%xmm2
  1e58fc:	c5 fb 10 05 ac df e3 	vmovsd -0x1c2054(%rip),%xmm0        # 238b0 <anon.1d01855f4677b114ff07ecd502d0759a.8.llvm.5874686521581109774+0x160>
  1e5903:	ff 
  1e5904:	c5 f9 2e c2          	vucomisd %xmm2,%xmm0
  1e5908:	72 12                	jb     1e591c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe8c>
  1e590a:	c5 f9 57 c0          	vxorpd %xmm0,%xmm0,%xmm0
  1e590e:	c5 f9 2e d0          	vucomisd %xmm0,%xmm2
  1e5912:	75 02                	jne    1e5916 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe86>
  1e5914:	7b 79                	jnp    1e598f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xeff>
  1e5916:	c5 f9 2e d6          	vucomisd %xmm6,%xmm2
  1e591a:	73 73                	jae    1e598f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xeff>
  1e591c:	45 31 db             	xor    %r11d,%r11d
  1e591f:	41 39 f3             	cmp    %esi,%r11d
  1e5922:	0f 82 32 01 00 00    	jb     1e5a5a <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xfca>
  1e5928:	4c 8d 52 01          	lea    0x1(%rdx),%r10
  1e592c:	48 ff 84 24 08 01 00 	incq   0x108(%rsp)
  1e5933:	00 
  1e5934:	41 bb 01 00 00 00    	mov    $0x1,%r11d
  1e593a:	48 89 ee             	mov    %rbp,%rsi
  1e593d:	48 8b 7c 24 08       	mov    0x8(%rsp),%rdi
  1e5942:	4c 89 cd             	mov    %r9,%rbp
  1e5945:	4c 8b 4c 24 60       	mov    0x60(%rsp),%r9
  1e594a:	4c 89 4c 24 08       	mov    %r9,0x8(%rsp)
  1e594f:	48 3b 54 24 58       	cmp    0x58(%rsp),%rdx
  1e5954:	4c 8b 8c 24 88 00 00 	mov    0x88(%rsp),%r9
  1e595b:	00 
  1e595c:	0f 82 e1 f3 ff ff    	jb     1e4d43 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x2b3>
  1e5962:	e9 f7 00 00 00       	jmp    1e5a5e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xfce>
  1e5967:	4c 8b 54 24 58       	mov    0x58(%rsp),%r10
  1e596c:	48 8b 5c 24 20       	mov    0x20(%rsp),%rbx
  1e5971:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
  1e5976:	48 89 d7             	mov    %rdx,%rdi
  1e5979:	48 2b bc 24 10 01 00 	sub    0x110(%rsp),%rdi
  1e5980:	00 
  1e5981:	48 39 df             	cmp    %rbx,%rdi
  1e5984:	0f 82 21 fe ff ff    	jb     1e57ab <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xd1b>
  1e598a:	e9 a6 01 00 00       	jmp    1e5b35 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x10a5>
  1e598f:	83 fe 01             	cmp    $0x1,%esi
  1e5992:	74 94                	je     1e5928 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe98>
  1e5994:	c5 fb 10 44 fd 08    	vmovsd 0x8(%rbp,%rdi,8),%xmm0
  1e599a:	c5 fb 12 15 86 de e3 	vmovddup -0x1c217a(%rip),%xmm2        # 23828 <anon.1d01855f4677b114ff07ecd502d0759a.8.llvm.5874686521581109774+0xd8>
  1e59a1:	ff 
  1e59a2:	c5 f9 54 c2          	vandpd %xmm2,%xmm0,%xmm0
  1e59a6:	c4 81 7b 10 4c d1 08 	vmovsd 0x8(%r9,%r10,8),%xmm1
  1e59ad:	c5 f1 54 ca          	vandpd %xmm2,%xmm1,%xmm1
  1e59b1:	c5 fb 5f d1          	vmaxsd %xmm1,%xmm0,%xmm2
  1e59b5:	c5 f3 c2 c9 03       	vcmpunordsd %xmm1,%xmm1,%xmm1
  1e59ba:	c4 e3 69 4b d0 10    	vblendvpd %xmm1,%xmm0,%xmm2,%xmm2
  1e59c0:	41 bb 01 00 00 00    	mov    $0x1,%r11d
  1e59c6:	c5 fb 10 05 e2 de e3 	vmovsd -0x1c211e(%rip),%xmm0        # 238b0 <anon.1d01855f4677b114ff07ecd502d0759a.8.llvm.5874686521581109774+0x160>
  1e59cd:	ff 
  1e59ce:	c5 f9 2e c2          	vucomisd %xmm2,%xmm0
  1e59d2:	0f 82 47 ff ff ff    	jb     1e591f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe8f>
  1e59d8:	c5 f9 57 c0          	vxorpd %xmm0,%xmm0,%xmm0
  1e59dc:	c5 f9 2e d0          	vucomisd %xmm0,%xmm2
  1e59e0:	75 02                	jne    1e59e4 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xf54>
  1e59e2:	7b 0a                	jnp    1e59ee <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xf5e>
  1e59e4:	c5 f9 2e d6          	vucomisd %xmm6,%xmm2
  1e59e8:	0f 82 31 ff ff ff    	jb     1e591f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe8f>
  1e59ee:	83 fe 02             	cmp    $0x2,%esi
  1e59f1:	0f 84 31 ff ff ff    	je     1e5928 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe98>
  1e59f7:	c5 fb 10 44 fd 10    	vmovsd 0x10(%rbp,%rdi,8),%xmm0
  1e59fd:	c5 fb 12 15 23 de e3 	vmovddup -0x1c21dd(%rip),%xmm2        # 23828 <anon.1d01855f4677b114ff07ecd502d0759a.8.llvm.5874686521581109774+0xd8>
  1e5a04:	ff 
  1e5a05:	c5 f9 54 c2          	vandpd %xmm2,%xmm0,%xmm0
  1e5a09:	c4 81 7b 10 4c d1 10 	vmovsd 0x10(%r9,%r10,8),%xmm1
  1e5a10:	c5 f1 54 ca          	vandpd %xmm2,%xmm1,%xmm1
  1e5a14:	c5 fb 5f d1          	vmaxsd %xmm1,%xmm0,%xmm2
  1e5a18:	c5 f3 c2 c9 03       	vcmpunordsd %xmm1,%xmm1,%xmm1
  1e5a1d:	c4 e3 69 4b d0 10    	vblendvpd %xmm1,%xmm0,%xmm2,%xmm2
  1e5a23:	41 bb 02 00 00 00    	mov    $0x2,%r11d
  1e5a29:	c5 fb 10 05 7f de e3 	vmovsd -0x1c2181(%rip),%xmm0        # 238b0 <anon.1d01855f4677b114ff07ecd502d0759a.8.llvm.5874686521581109774+0x160>
  1e5a30:	ff 
  1e5a31:	c5 f9 2e c2          	vucomisd %xmm2,%xmm0
  1e5a35:	0f 82 e4 fe ff ff    	jb     1e591f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe8f>
  1e5a3b:	c5 f9 57 c0          	vxorpd %xmm0,%xmm0,%xmm0
  1e5a3f:	c5 f9 2e d0          	vucomisd %xmm0,%xmm2
  1e5a43:	75 06                	jne    1e5a4b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xfbb>
  1e5a45:	0f 8b dd fe ff ff    	jnp    1e5928 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe98>
  1e5a4b:	c5 f9 2e d6          	vucomisd %xmm6,%xmm2
  1e5a4f:	0f 82 ca fe ff ff    	jb     1e591f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe8f>
  1e5a55:	e9 ce fe ff ff       	jmp    1e5928 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xe98>
  1e5a5a:	31 c0                	xor    %eax,%eax
  1e5a5c:	eb 02                	jmp    1e5a60 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xfd0>
  1e5a5e:	b0 01                	mov    $0x1,%al
  1e5a60:	48 81 c4 68 01 00 00 	add    $0x168,%rsp
  1e5a67:	5b                   	pop    %rbx
  1e5a68:	41 5c                	pop    %r12
  1e5a6a:	41 5d                	pop    %r13
  1e5a6c:	41 5e                	pop    %r14
  1e5a6e:	41 5f                	pop    %r15
  1e5a70:	5d                   	pop    %rbp
  1e5a71:	c5 f8 77             	vzeroupper
  1e5a74:	c3                   	ret
  1e5a75:	48 8d 15 0c 73 0a 00 	lea    0xa730c(%rip),%rdx        # 28cd88 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x8d0>
  1e5a7c:	48 8b bc 24 28 01 00 	mov    0x128(%rsp),%rdi
  1e5a83:	00 
  1e5a84:	48 89 fe             	mov    %rdi,%rsi
  1e5a87:	c5 f8 77             	vzeroupper
  1e5a8a:	ff 15 30 07 0b 00    	call   *0xb0730(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5a90:	48 8d 15 51 73 0a 00 	lea    0xa7351(%rip),%rdx        # 28cde8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x930>
  1e5a97:	48 89 f7             	mov    %rsi,%rdi
  1e5a9a:	c5 f8 77             	vzeroupper
  1e5a9d:	ff 15 1d 07 0b 00    	call   *0xb071d(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5aa3:	48 8d 15 c6 72 0a 00 	lea    0xa72c6(%rip),%rdx        # 28cd70 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x8b8>
  1e5aaa:	48 8b bc 24 30 01 00 	mov    0x130(%rsp),%rdi
  1e5ab1:	00 
  1e5ab2:	48 89 fe             	mov    %rdi,%rsi
  1e5ab5:	c5 f8 77             	vzeroupper
  1e5ab8:	ff 15 02 07 0b 00    	call   *0xb0702(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5abe:	48 8d 15 0b 73 0a 00 	lea    0xa730b(%rip),%rdx        # 28cdd0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x918>
  1e5ac5:	48 89 fe             	mov    %rdi,%rsi
  1e5ac8:	c5 f8 77             	vzeroupper
  1e5acb:	ff 15 ef 06 0b 00    	call   *0xb06ef(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5ad1:	48 8d 15 80 72 0a 00 	lea    0xa7280(%rip),%rdx        # 28cd58 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x8a0>
  1e5ad8:	48 8b bc 24 90 00 00 	mov    0x90(%rsp),%rdi
  1e5adf:	00 
  1e5ae0:	48 89 fe             	mov    %rdi,%rsi
  1e5ae3:	c5 f8 77             	vzeroupper
  1e5ae6:	ff 15 d4 06 0b 00    	call   *0xb06d4(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5aec:	48 8d 15 c5 72 0a 00 	lea    0xa72c5(%rip),%rdx        # 28cdb8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x900>
  1e5af3:	48 8b bc 24 18 01 00 	mov    0x118(%rsp),%rdi
  1e5afa:	00 
  1e5afb:	48 89 fe             	mov    %rdi,%rsi
  1e5afe:	c5 f8 77             	vzeroupper
  1e5b01:	ff 15 b9 06 0b 00    	call   *0xb06b9(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5b07:	48 8d 15 92 72 0a 00 	lea    0xa7292(%rip),%rdx        # 28cda0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x8e8>
  1e5b0e:	48 8b bc 24 20 01 00 	mov    0x120(%rsp),%rdi
  1e5b15:	00 
  1e5b16:	48 89 fe             	mov    %rdi,%rsi
  1e5b19:	c5 f8 77             	vzeroupper
  1e5b1c:	ff 15 9e 06 0b 00    	call   *0xb069e(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5b22:	48 8d 15 d7 6c 0a 00 	lea    0xa6cd7(%rip),%rdx        # 28c800 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x348>
  1e5b29:	48 89 fe             	mov    %rdi,%rsi
  1e5b2c:	c5 f8 77             	vzeroupper
  1e5b2f:	ff 15 8b 06 0b 00    	call   *0xb068b(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5b35:	48 8d 15 6c 7f 0a 00 	lea    0xa7f6c(%rip),%rdx        # 28daa8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x15f0>
  1e5b3c:	48 89 de             	mov    %rbx,%rsi
  1e5b3f:	c5 f8 77             	vzeroupper
  1e5b42:	ff 15 78 06 0b 00    	call   *0xb0678(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5b48:	48 8d 15 f1 71 0a 00 	lea    0xa71f1(%rip),%rdx        # 28cd40 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x888>
  1e5b4f:	48 89 fe             	mov    %rdi,%rsi
  1e5b52:	c5 f8 77             	vzeroupper
  1e5b55:	ff 15 65 06 0b 00    	call   *0xb0665(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5b5b:	48 8d 15 c6 71 0a 00 	lea    0xa71c6(%rip),%rdx        # 28cd28 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x870>
  1e5b62:	48 89 f7             	mov    %rsi,%rdi
  1e5b65:	c5 f8 77             	vzeroupper
  1e5b68:	ff 15 52 06 0b 00    	call   *0xb0652(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5b6e:	48 8d 15 9b 71 0a 00 	lea    0xa719b(%rip),%rdx        # 28cd10 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x858>
  1e5b75:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
  1e5b7a:	48 89 fe             	mov    %rdi,%rsi
  1e5b7d:	c5 f8 77             	vzeroupper
  1e5b80:	ff 15 3a 06 0b 00    	call   *0xb063a(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5b86:	48 8d 15 6b 71 0a 00 	lea    0xa716b(%rip),%rdx        # 28ccf8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x840>
  1e5b8d:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
  1e5b92:	48 89 fe             	mov    %rdi,%rsi
  1e5b95:	c5 f8 77             	vzeroupper
  1e5b98:	ff 15 22 06 0b 00    	call   *0xb0622(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5b9e:	48 8d 15 3b 71 0a 00 	lea    0xa713b(%rip),%rdx        # 28cce0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x828>
  1e5ba5:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
  1e5baa:	48 89 fe             	mov    %rdi,%rsi
  1e5bad:	c5 f8 77             	vzeroupper
  1e5bb0:	ff 15 0a 06 0b 00    	call   *0xb060a(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5bb6:	48 8d 15 0b 71 0a 00 	lea    0xa710b(%rip),%rdx        # 28ccc8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x810>
  1e5bbd:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
  1e5bc2:	48 89 fe             	mov    %rdi,%rsi
  1e5bc5:	c5 f8 77             	vzeroupper
  1e5bc8:	ff 15 f2 05 0b 00    	call   *0xb05f2(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5bce:	48 8d 15 db 70 0a 00 	lea    0xa70db(%rip),%rdx        # 28ccb0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x7f8>
  1e5bd5:	48 8b 7c 24 40       	mov    0x40(%rsp),%rdi
  1e5bda:	48 89 fe             	mov    %rdi,%rsi
  1e5bdd:	c5 f8 77             	vzeroupper
  1e5be0:	ff 15 da 05 0b 00    	call   *0xb05da(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5be6:	48 8d 15 d3 7e 0a 00 	lea    0xa7ed3(%rip),%rdx        # 28dac0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x1608>
  1e5bed:	48 89 de             	mov    %rbx,%rsi
  1e5bf0:	c5 f8 77             	vzeroupper
  1e5bf3:	ff 15 c7 05 0b 00    	call   *0xb05c7(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5bf9:	48 8d 15 78 7e 0a 00 	lea    0xa7e78(%rip),%rdx        # 28da78 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x15c0>
  1e5c00:	4c 89 df             	mov    %r11,%rdi
  1e5c03:	48 8b b4 24 00 01 00 	mov    0x100(%rsp),%rsi
  1e5c0a:	00 
  1e5c0b:	c5 f8 77             	vzeroupper
  1e5c0e:	ff 15 ac 05 0b 00    	call   *0xb05ac(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5c14:	48 8d 15 75 7e 0a 00 	lea    0xa7e75(%rip),%rdx        # 28da90 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x15d8>
  1e5c1b:	4c 89 df             	mov    %r11,%rdi
  1e5c1e:	48 8b b4 24 f8 00 00 	mov    0xf8(%rsp),%rsi
  1e5c25:	00 
  1e5c26:	c5 f8 77             	vzeroupper
  1e5c29:	ff 15 91 05 0b 00    	call   *0xb0591(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5c2f:	48 8d 15 1a 73 0a 00 	lea    0xa731a(%rip),%rdx        # 28cf50 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0xa98>
  1e5c36:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
  1e5c3b:	48 89 fe             	mov    %rdi,%rsi
  1e5c3e:	c5 f8 77             	vzeroupper
  1e5c41:	ff 15 79 05 0b 00    	call   *0xb0579(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5c47:	48 8d 15 ea 72 0a 00 	lea    0xa72ea(%rip),%rdx        # 28cf38 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0xa80>
  1e5c4e:	48 8b 7c 24 40       	mov    0x40(%rsp),%rdi
  1e5c53:	48 89 fe             	mov    %rdi,%rsi
  1e5c56:	c5 f8 77             	vzeroupper
  1e5c59:	ff 15 61 05 0b 00    	call   *0xb0561(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5c5f:	48 8d 15 ba 72 0a 00 	lea    0xa72ba(%rip),%rdx        # 28cf20 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0xa68>
  1e5c66:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
  1e5c6b:	48 89 fe             	mov    %rdi,%rsi
  1e5c6e:	c5 f8 77             	vzeroupper
  1e5c71:	ff 15 49 05 0b 00    	call   *0xb0549(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5c77:	48 8d 15 8a 72 0a 00 	lea    0xa728a(%rip),%rdx        # 28cf08 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0xa50>
  1e5c7e:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
  1e5c83:	48 89 fe             	mov    %rdi,%rsi
  1e5c86:	c5 f8 77             	vzeroupper
  1e5c89:	ff 15 31 05 0b 00    	call   *0xb0531(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5c8f:	48 8d 15 5a 72 0a 00 	lea    0xa725a(%rip),%rdx        # 28cef0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0xa38>
  1e5c96:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
  1e5c9b:	48 89 fe             	mov    %rdi,%rsi
  1e5c9e:	c5 f8 77             	vzeroupper
  1e5ca1:	ff 15 19 05 0b 00    	call   *0xb0519(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5ca7:	48 8d 15 ea 6f 0a 00 	lea    0xa6fea(%rip),%rdx        # 28cc98 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x7e0>
  1e5cae:	31 ff                	xor    %edi,%edi
  1e5cb0:	31 f6                	xor    %esi,%esi
  1e5cb2:	c5 f8 77             	vzeroupper
  1e5cb5:	ff 15 05 05 0b 00    	call   *0xb0505(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5cbb:	48 8d 15 be 6f 0a 00 	lea    0xa6fbe(%rip),%rdx        # 28cc80 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x7c8>
  1e5cc2:	31 ff                	xor    %edi,%edi
  1e5cc4:	31 f6                	xor    %esi,%esi
  1e5cc6:	c5 f8 77             	vzeroupper
  1e5cc9:	ff 15 f1 04 0b 00    	call   *0xb04f1(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5ccf:	48 8d 15 92 6f 0a 00 	lea    0xa6f92(%rip),%rdx        # 28cc68 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x7b0>
  1e5cd6:	31 ff                	xor    %edi,%edi
  1e5cd8:	31 f6                	xor    %esi,%esi
  1e5cda:	c5 f8 77             	vzeroupper
  1e5cdd:	ff 15 dd 04 0b 00    	call   *0xb04dd(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5ce3:	48 8d 15 66 6f 0a 00 	lea    0xa6f66(%rip),%rdx        # 28cc50 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x798>
  1e5cea:	31 ff                	xor    %edi,%edi
  1e5cec:	31 f6                	xor    %esi,%esi
  1e5cee:	c5 f8 77             	vzeroupper
  1e5cf1:	ff 15 c9 04 0b 00    	call   *0xb04c9(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5cf7:	48 8d 15 3a 6f 0a 00 	lea    0xa6f3a(%rip),%rdx        # 28cc38 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x780>
  1e5cfe:	31 ff                	xor    %edi,%edi
  1e5d00:	31 f6                	xor    %esi,%esi
  1e5d02:	c5 f8 77             	vzeroupper
  1e5d05:	ff 15 b5 04 0b 00    	call   *0xb04b5(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5d0b:	48 8d 15 0e 6f 0a 00 	lea    0xa6f0e(%rip),%rdx        # 28cc20 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x768>
  1e5d12:	31 ff                	xor    %edi,%edi
  1e5d14:	31 f6                	xor    %esi,%esi
  1e5d16:	c5 f8 77             	vzeroupper
  1e5d19:	ff 15 a1 04 0b 00    	call   *0xb04a1(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5d1f:	48 8d 15 e2 6e 0a 00 	lea    0xa6ee2(%rip),%rdx        # 28cc08 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x750>
  1e5d26:	31 ff                	xor    %edi,%edi
  1e5d28:	31 f6                	xor    %esi,%esi
  1e5d2a:	c5 f8 77             	vzeroupper
  1e5d2d:	ff 15 8d 04 0b 00    	call   *0xb048d(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5d33:	48 8d 15 9e 71 0a 00 	lea    0xa719e(%rip),%rdx        # 28ced8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0xa20>
  1e5d3a:	31 ff                	xor    %edi,%edi
  1e5d3c:	31 f6                	xor    %esi,%esi
  1e5d3e:	c5 f8 77             	vzeroupper
  1e5d41:	ff 15 79 04 0b 00    	call   *0xb0479(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5d47:	48 8d 15 72 71 0a 00 	lea    0xa7172(%rip),%rdx        # 28cec0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0xa08>
  1e5d4e:	31 ff                	xor    %edi,%edi
  1e5d50:	31 f6                	xor    %esi,%esi
  1e5d52:	c5 f8 77             	vzeroupper
  1e5d55:	ff 15 65 04 0b 00    	call   *0xb0465(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5d5b:	48 8d 15 46 71 0a 00 	lea    0xa7146(%rip),%rdx        # 28cea8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x9f0>
  1e5d62:	31 ff                	xor    %edi,%edi
  1e5d64:	31 f6                	xor    %esi,%esi
  1e5d66:	c5 f8 77             	vzeroupper
  1e5d69:	ff 15 51 04 0b 00    	call   *0xb0451(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5d6f:	48 8d 15 1a 71 0a 00 	lea    0xa711a(%rip),%rdx        # 28ce90 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x9d8>
  1e5d76:	31 ff                	xor    %edi,%edi
  1e5d78:	31 f6                	xor    %esi,%esi
  1e5d7a:	c5 f8 77             	vzeroupper
  1e5d7d:	ff 15 3d 04 0b 00    	call   *0xb043d(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
  1e5d83:	48 8d 15 ee 70 0a 00 	lea    0xa70ee(%rip),%rdx        # 28ce78 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x9c0>
  1e5d8a:	31 ff                	xor    %edi,%edi
  1e5d8c:	31 f6                	xor    %esi,%esi
  1e5d8e:	c5 f8 77             	vzeroupper
  1e5d91:	ff 15 29 04 0b 00    	call   *0xb0429(%rip)        # 2961c0 <_DYNAMIC+0x2c0>
