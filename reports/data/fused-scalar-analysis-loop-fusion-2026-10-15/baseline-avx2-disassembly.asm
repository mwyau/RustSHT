
target/release/examples/m1b_bench:     file format elf64-x86-64


Disassembly of section .text:

00000000001e7710 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>>:
  1e7710:	55                   	push   %rbp
  1e7711:	41 57                	push   %r15
  1e7713:	41 56                	push   %r14
  1e7715:	41 55                	push   %r13
  1e7717:	41 54                	push   %r12
  1e7719:	53                   	push   %rbx
  1e771a:	48 81 ec 48 01 00 00 	sub    $0x148,%rsp
  1e7721:	4c 8b b7 b0 00 00 00 	mov    0xb0(%rdi),%r14
  1e7728:	48 8b 8f b8 00 00 00 	mov    0xb8(%rdi),%rcx
  1e772f:	b0 01                	mov    $0x1,%al
  1e7731:	49 39 ce             	cmp    %rcx,%r14
  1e7734:	0f 87 07 0b 00 00    	ja     1e8241 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb31>
  1e773a:	48 89 4c 24 40       	mov    %rcx,0x40(%rsp)
  1e773f:	48 b9 fc ff ff ff ff 	movabs $0xffffffffffffffc,%rcx
  1e7746:	ff ff 0f 
  1e7749:	4c 8b 7f 38          	mov    0x38(%rdi),%r15
  1e774d:	4c 8b 67 48          	mov    0x48(%rdi),%r12
  1e7751:	4c 8b 4f 58          	mov    0x58(%rdi),%r9
  1e7755:	4c 89 8c 24 90 00 00 	mov    %r9,0x90(%rsp)
  1e775c:	00 
  1e775d:	4c 8b 57 68          	mov    0x68(%rdi),%r10
  1e7761:	4c 89 fa             	mov    %r15,%rdx
  1e7764:	48 c1 ea 02          	shr    $0x2,%rdx
  1e7768:	48 89 f8             	mov    %rdi,%rax
  1e776b:	44 89 fe             	mov    %r15d,%esi
  1e776e:	83 e6 03             	and    $0x3,%esi
  1e7771:	48 89 74 24 58       	mov    %rsi,0x58(%rsp)
  1e7776:	4c 89 ff             	mov    %r15,%rdi
  1e7779:	48 21 cf             	and    %rcx,%rdi
  1e777c:	48 89 7c 24 68       	mov    %rdi,0x68(%rsp)
  1e7781:	4c 89 e7             	mov    %r12,%rdi
  1e7784:	48 c1 ef 02          	shr    $0x2,%rdi
  1e7788:	44 89 e5             	mov    %r12d,%ebp
  1e778b:	83 e5 03             	and    $0x3,%ebp
  1e778e:	4d 89 e0             	mov    %r12,%r8
  1e7791:	49 21 c8             	and    %rcx,%r8
  1e7794:	4c 89 44 24 60       	mov    %r8,0x60(%rsp)
  1e7799:	4d 89 c8             	mov    %r9,%r8
  1e779c:	49 c1 e8 02          	shr    $0x2,%r8
  1e77a0:	44 89 ce             	mov    %r9d,%esi
  1e77a3:	83 e6 03             	and    $0x3,%esi
  1e77a6:	48 89 74 24 50       	mov    %rsi,0x50(%rsp)
  1e77ab:	49 21 c9             	and    %rcx,%r9
  1e77ae:	4c 89 8c 24 a8 00 00 	mov    %r9,0xa8(%rsp)
  1e77b5:	00 
  1e77b6:	4d 89 d1             	mov    %r10,%r9
  1e77b9:	49 c1 e9 02          	shr    $0x2,%r9
  1e77bd:	44 89 d6             	mov    %r10d,%esi
  1e77c0:	4c 89 94 24 88 00 00 	mov    %r10,0x88(%rsp)
  1e77c7:	00 
  1e77c8:	83 e6 03             	and    $0x3,%esi
  1e77cb:	48 89 74 24 48       	mov    %rsi,0x48(%rsp)
  1e77d0:	4c 89 d6             	mov    %r10,%rsi
  1e77d3:	48 21 ce             	and    %rcx,%rsi
  1e77d6:	48 89 b4 24 10 01 00 	mov    %rsi,0x110(%rsp)
  1e77dd:	00 
  1e77de:	48 8b 58 08          	mov    0x8(%rax),%rbx
  1e77e2:	48 89 de             	mov    %rbx,%rsi
  1e77e5:	48 c1 ee 02          	shr    $0x2,%rsi
  1e77e9:	41 89 db             	mov    %ebx,%r11d
  1e77ec:	41 83 e3 03          	and    $0x3,%r11d
  1e77f0:	49 89 da             	mov    %rbx,%r10
  1e77f3:	49 21 ca             	and    %rcx,%r10
  1e77f6:	4c 89 54 24 30       	mov    %r10,0x30(%rsp)
  1e77fb:	4c 89 b4 24 b0 00 00 	mov    %r14,0xb0(%rsp)
  1e7802:	00 
  1e7803:	49 be fe ff ff ff ff 	movabs $0x3fffffffffffffe,%r14
  1e780a:	ff ff 03 
  1e780d:	4c 21 f2             	and    %r14,%rdx
  1e7810:	48 c1 e2 05          	shl    $0x5,%rdx
  1e7814:	48 8b 48 30          	mov    0x30(%rax),%rcx
  1e7818:	48 01 ca             	add    %rcx,%rdx
  1e781b:	48 89 94 24 f0 00 00 	mov    %rdx,0xf0(%rsp)
  1e7822:	00 
  1e7823:	4c 21 f7             	and    %r14,%rdi
  1e7826:	48 c1 e7 05          	shl    $0x5,%rdi
  1e782a:	4c 8b 68 40          	mov    0x40(%rax),%r13
  1e782e:	4c 01 ef             	add    %r13,%rdi
  1e7831:	48 89 bc 24 e8 00 00 	mov    %rdi,0xe8(%rsp)
  1e7838:	00 
  1e7839:	4d 21 f0             	and    %r14,%r8
  1e783c:	49 c1 e0 05          	shl    $0x5,%r8
  1e7840:	48 8b 50 50          	mov    0x50(%rax),%rdx
  1e7844:	49 01 d0             	add    %rdx,%r8
  1e7847:	4c 89 84 24 e0 00 00 	mov    %r8,0xe0(%rsp)
  1e784e:	00 
  1e784f:	4d 21 f1             	and    %r14,%r9
  1e7852:	49 c1 e1 05          	shl    $0x5,%r9
  1e7856:	4c 8b 50 60          	mov    0x60(%rax),%r10
  1e785a:	4d 01 d1             	add    %r10,%r9
  1e785d:	4c 89 8c 24 d8 00 00 	mov    %r9,0xd8(%rsp)
  1e7864:	00 
  1e7865:	4c 89 df             	mov    %r11,%rdi
  1e7868:	48 89 74 24 20       	mov    %rsi,0x20(%rsp)
  1e786d:	4c 21 f6             	and    %r14,%rsi
  1e7870:	48 c1 e6 05          	shl    $0x5,%rsi
  1e7874:	4c 8b 08             	mov    (%rax),%r9
  1e7877:	4c 01 ce             	add    %r9,%rsi
  1e787a:	48 89 b4 24 d0 00 00 	mov    %rsi,0xd0(%rsp)
  1e7881:	00 
  1e7882:	4c 89 bc 24 00 01 00 	mov    %r15,0x100(%rsp)
  1e7889:	00 
  1e788a:	49 c1 ef 03          	shr    $0x3,%r15
  1e788e:	4c 89 a4 24 f8 00 00 	mov    %r12,0xf8(%rsp)
  1e7895:	00 
  1e7896:	49 c1 ec 03          	shr    $0x3,%r12
  1e789a:	4d 39 fc             	cmp    %r15,%r12
  1e789d:	4d 0f 43 e7          	cmovae %r15,%r12
  1e78a1:	48 8b b4 24 90 00 00 	mov    0x90(%rsp),%rsi
  1e78a8:	00 
  1e78a9:	48 c1 ee 03          	shr    $0x3,%rsi
  1e78ad:	4c 39 e6             	cmp    %r12,%rsi
  1e78b0:	49 0f 43 f4          	cmovae %r12,%rsi
  1e78b4:	4c 8b 84 24 88 00 00 	mov    0x88(%rsp),%r8
  1e78bb:	00 
  1e78bc:	49 c1 e8 03          	shr    $0x3,%r8
  1e78c0:	49 39 f0             	cmp    %rsi,%r8
  1e78c3:	4c 0f 43 c6          	cmovae %rsi,%r8
  1e78c7:	4c 89 84 24 38 01 00 	mov    %r8,0x138(%rsp)
  1e78ce:	00 
  1e78cf:	48 89 9c 24 08 01 00 	mov    %rbx,0x108(%rsp)
  1e78d6:	00 
  1e78d7:	48 c1 eb 03          	shr    $0x3,%rbx
  1e78db:	48 89 9c 24 30 01 00 	mov    %rbx,0x130(%rsp)
  1e78e2:	00 
  1e78e3:	4c 8b 74 24 58       	mov    0x58(%rsp),%r14
  1e78e8:	4c 39 f5             	cmp    %r14,%rbp
  1e78eb:	48 89 ac 24 80 00 00 	mov    %rbp,0x80(%rsp)
  1e78f2:	00 
  1e78f3:	4c 0f 42 f5          	cmovb  %rbp,%r14
  1e78f7:	48 8b 70 10          	mov    0x10(%rax),%rsi
  1e78fb:	48 89 b4 24 28 01 00 	mov    %rsi,0x128(%rsp)
  1e7902:	00 
  1e7903:	48 8b 70 18          	mov    0x18(%rax),%rsi
  1e7907:	48 89 b4 24 a0 00 00 	mov    %rsi,0xa0(%rsp)
  1e790e:	00 
  1e790f:	48 8b 70 20          	mov    0x20(%rax),%rsi
  1e7913:	48 89 b4 24 20 01 00 	mov    %rsi,0x120(%rsp)
  1e791a:	00 
  1e791b:	48 8b 70 28          	mov    0x28(%rax),%rsi
  1e791f:	48 89 b4 24 98 00 00 	mov    %rsi,0x98(%rsp)
  1e7926:	00 
  1e7927:	48 8b 70 70          	mov    0x70(%rax),%rsi
  1e792b:	48 89 b4 24 b8 00 00 	mov    %rsi,0xb8(%rsp)
  1e7932:	00 
  1e7933:	48 8b 70 78          	mov    0x78(%rax),%rsi
  1e7937:	48 89 74 24 28       	mov    %rsi,0x28(%rsp)
  1e793c:	4c 8b b8 a8 00 00 00 	mov    0xa8(%rax),%r15
  1e7943:	48 8b 98 a0 00 00 00 	mov    0xa0(%rax),%rbx
  1e794a:	4c 8b a0 98 00 00 00 	mov    0x98(%rax),%r12
  1e7951:	4c 8b 80 90 00 00 00 	mov    0x90(%rax),%r8
  1e7958:	4c 8b 98 80 00 00 00 	mov    0x80(%rax),%r11
  1e795f:	48 8b 80 88 00 00 00 	mov    0x88(%rax),%rax
  1e7966:	48 8b 74 24 50       	mov    0x50(%rsp),%rsi
  1e796b:	49 39 f6             	cmp    %rsi,%r14
  1e796e:	4c 0f 43 f6          	cmovae %rsi,%r14
  1e7972:	49 39 fe             	cmp    %rdi,%r14
  1e7975:	48 89 7c 24 08       	mov    %rdi,0x8(%rsp)
  1e797a:	4c 0f 43 f7          	cmovae %rdi,%r14
  1e797e:	4c 89 ff             	mov    %r15,%rdi
  1e7981:	48 8b 74 24 48       	mov    0x48(%rsp),%rsi
  1e7986:	49 39 f6             	cmp    %rsi,%r14
  1e7989:	4c 0f 43 f6          	cmovae %rsi,%r14
  1e798d:	4c 89 b4 24 18 01 00 	mov    %r14,0x118(%rsp)
  1e7994:	00 
  1e7995:	c5 fb 10 1d b3 be e3 	vmovsd -0x1c414d(%rip),%xmm3        # 23850 <anon.320a9f19b214853d1b1894d26beaba85.4.llvm.13199781604905473672+0x1c8>
  1e799c:	ff 
  1e799d:	c4 e2 7d 19 25 4a be 	vbroadcastsd -0x1c41b6(%rip),%ymm4        # 237f0 <anon.320a9f19b214853d1b1894d26beaba85.4.llvm.13199781604905473672+0x168>
  1e79a4:	e3 ff 
  1e79a6:	c5 d1 57 ed          	vxorpd %xmm5,%xmm5,%xmm5
  1e79aa:	48 c7 84 24 c0 00 00 	movq   $0x0,0xc0(%rsp)
  1e79b1:	00 00 00 00 00 
  1e79b6:	4c 8b b4 24 b0 00 00 	mov    0xb0(%rsp),%r14
  1e79bd:	00 
  1e79be:	45 31 ff             	xor    %r15d,%r15d
  1e79c1:	48 89 7c 24 78       	mov    %rdi,0x78(%rsp)
  1e79c6:	48 89 de             	mov    %rbx,%rsi
  1e79c9:	4c 89 db             	mov    %r11,%rbx
  1e79cc:	4d 89 f3             	mov    %r14,%r11
  1e79cf:	4d 01 fb             	add    %r15,%r11
  1e79d2:	0f 82 67 08 00 00    	jb     1e823f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb2f>
  1e79d8:	48 8b 7c 24 40       	mov    0x40(%rsp),%rdi
  1e79dd:	49 39 fb             	cmp    %rdi,%r11
  1e79e0:	0f 87 59 08 00 00    	ja     1e823f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb2f>
  1e79e6:	48 89 44 24 70       	mov    %rax,0x70(%rsp)
  1e79eb:	4c 89 e0             	mov    %r12,%rax
  1e79ee:	48 c1 e8 03          	shr    $0x3,%rax
  1e79f2:	48 8b bc 24 38 01 00 	mov    0x138(%rsp),%rdi
  1e79f9:	00 
  1e79fa:	48 39 f8             	cmp    %rdi,%rax
  1e79fd:	48 0f 43 c7          	cmovae %rdi,%rax
  1e7a01:	48 8b bc 24 30 01 00 	mov    0x130(%rsp),%rdi
  1e7a08:	00 
  1e7a09:	48 39 c7             	cmp    %rax,%rdi
  1e7a0c:	48 0f 42 c7          	cmovb  %rdi,%rax
  1e7a10:	48 85 c0             	test   %rax,%rax
  1e7a13:	0f 84 c7 00 00 00    	je     1e7ae0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x3d0>
  1e7a19:	c5 c9 57 f6          	vxorpd %xmm6,%xmm6,%xmm6
  1e7a1d:	41 be 20 00 00 00    	mov    $0x20,%r14d
  1e7a23:	c4 41 29 57 d2       	vxorpd %xmm10,%xmm10,%xmm10
  1e7a28:	c4 41 39 57 c0       	vxorpd %xmm8,%xmm8,%xmm8
  1e7a2d:	c4 41 11 57 ed       	vxorpd %xmm13,%xmm13,%xmm13
  1e7a32:	c4 41 19 57 e4       	vxorpd %xmm12,%xmm12,%xmm12
  1e7a37:	c4 41 31 57 c9       	vxorpd %xmm9,%xmm9,%xmm9
  1e7a3c:	c5 c1 57 ff          	vxorpd %xmm7,%xmm7,%xmm7
  1e7a40:	c4 41 21 57 db       	vxorpd %xmm11,%xmm11,%xmm11
  1e7a45:	48 8b bc 24 a8 00 00 	mov    0xa8(%rsp),%rdi
  1e7a4c:	00 
  1e7a4d:	0f 1f 00             	nopl   (%rax)
  1e7a50:	c4 01 7d 10 74 30 e0 	vmovupd -0x20(%r8,%r14,1),%ymm14
  1e7a57:	c4 01 7d 10 3c 30    	vmovupd (%r8,%r14,1),%ymm15
  1e7a5d:	c4 a1 0d 59 54 31 e0 	vmulpd -0x20(%rcx,%r14,1),%ymm14,%ymm2
  1e7a64:	c4 81 0d 59 4c 35 e0 	vmulpd -0x20(%r13,%r14,1),%ymm14,%ymm1
  1e7a6b:	c5 c5 58 fa          	vaddpd %ymm2,%ymm7,%ymm7
  1e7a6f:	c5 35 58 c9          	vaddpd %ymm1,%ymm9,%ymm9
  1e7a73:	c4 a1 0d 59 4c 32 e0 	vmulpd -0x20(%rdx,%r14,1),%ymm14,%ymm1
  1e7a7a:	c4 81 7d 10 54 31 e0 	vmovupd -0x20(%r9,%r14,1),%ymm2
  1e7a81:	c4 81 7d 10 04 31    	vmovupd (%r9,%r14,1),%ymm0
  1e7a87:	c5 f5 59 ca          	vmulpd %ymm2,%ymm1,%ymm1
  1e7a8b:	c5 1d 58 e1          	vaddpd %ymm1,%ymm12,%ymm12
  1e7a8f:	c4 81 0d 59 4c 32 e0 	vmulpd -0x20(%r10,%r14,1),%ymm14,%ymm1
  1e7a96:	c5 ed 59 c9          	vmulpd %ymm1,%ymm2,%ymm1
  1e7a9a:	c5 15 58 e9          	vaddpd %ymm1,%ymm13,%ymm13
  1e7a9e:	c4 a1 05 59 0c 31    	vmulpd (%rcx,%r14,1),%ymm15,%ymm1
  1e7aa4:	c4 81 05 59 54 35 00 	vmulpd 0x0(%r13,%r14,1),%ymm15,%ymm2
  1e7aab:	c5 3d 58 c1          	vaddpd %ymm1,%ymm8,%ymm8
  1e7aaf:	c5 2d 58 d2          	vaddpd %ymm2,%ymm10,%ymm10
  1e7ab3:	c4 a1 05 59 0c 32    	vmulpd (%rdx,%r14,1),%ymm15,%ymm1
  1e7ab9:	c5 f5 59 c8          	vmulpd %ymm0,%ymm1,%ymm1
  1e7abd:	c5 cd 58 f1          	vaddpd %ymm1,%ymm6,%ymm6
  1e7ac1:	c4 81 05 59 0c 32    	vmulpd (%r10,%r14,1),%ymm15,%ymm1
  1e7ac7:	c5 fd 59 c1          	vmulpd %ymm1,%ymm0,%ymm0
  1e7acb:	c5 25 58 d8          	vaddpd %ymm0,%ymm11,%ymm11
  1e7acf:	49 83 c6 40          	add    $0x40,%r14
  1e7ad3:	48 ff c8             	dec    %rax
  1e7ad6:	0f 85 74 ff ff ff    	jne    1e7a50 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x340>
  1e7adc:	eb 30                	jmp    1e7b0e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x3fe>
  1e7ade:	66 90                	xchg   %ax,%ax
  1e7ae0:	c4 41 21 57 db       	vxorpd %xmm11,%xmm11,%xmm11
  1e7ae5:	c5 c1 57 ff          	vxorpd %xmm7,%xmm7,%xmm7
  1e7ae9:	c4 41 31 57 c9       	vxorpd %xmm9,%xmm9,%xmm9
  1e7aee:	c4 41 19 57 e4       	vxorpd %xmm12,%xmm12,%xmm12
  1e7af3:	c4 41 11 57 ed       	vxorpd %xmm13,%xmm13,%xmm13
  1e7af8:	c4 41 39 57 c0       	vxorpd %xmm8,%xmm8,%xmm8
  1e7afd:	c4 41 29 57 d2       	vxorpd %xmm10,%xmm10,%xmm10
  1e7b02:	c5 c9 57 f6          	vxorpd %xmm6,%xmm6,%xmm6
  1e7b06:	48 8b bc 24 a8 00 00 	mov    0xa8(%rsp),%rdi
  1e7b0d:	00 
  1e7b0e:	4c 89 e5             	mov    %r12,%rbp
  1e7b11:	48 c1 ed 02          	shr    $0x2,%rbp
  1e7b15:	41 f6 c4 04          	test   $0x4,%r12b
  1e7b19:	4c 89 9c 24 c8 00 00 	mov    %r11,0xc8(%rsp)
  1e7b20:	00 
  1e7b21:	75 0d                	jne    1e7b30 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x420>
  1e7b23:	49 89 fe             	mov    %rdi,%r14
  1e7b26:	e9 bd 00 00 00       	jmp    1e7be8 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x4d8>
  1e7b2b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
  1e7b30:	f6 84 24 08 01 00 00 	testb  $0x4,0x108(%rsp)
  1e7b37:	04 
  1e7b38:	0f 84 cd 08 00 00    	je     1e840b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xcfb>
  1e7b3e:	f6 84 24 00 01 00 00 	testb  $0x4,0x100(%rsp)
  1e7b45:	04 
  1e7b46:	0f 84 ab 08 00 00    	je     1e83f7 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xce7>
  1e7b4c:	f6 84 24 f8 00 00 00 	testb  $0x4,0xf8(%rsp)
  1e7b53:	04 
  1e7b54:	0f 84 89 08 00 00    	je     1e83e3 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xcd3>
  1e7b5a:	f6 84 24 90 00 00 00 	testb  $0x4,0x90(%rsp)
  1e7b61:	04 
  1e7b62:	0f 84 67 08 00 00    	je     1e83cf <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xcbf>
  1e7b68:	f6 84 24 88 00 00 00 	testb  $0x4,0x88(%rsp)
  1e7b6f:	04 
  1e7b70:	0f 84 45 08 00 00    	je     1e83bb <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xcab>
  1e7b76:	49 89 fe             	mov    %rdi,%r14
  1e7b79:	48 89 e8             	mov    %rbp,%rax
  1e7b7c:	49 bb fe ff ff ff ff 	movabs $0x3fffffffffffffe,%r11
  1e7b83:	ff ff 03 
  1e7b86:	4c 21 d8             	and    %r11,%rax
  1e7b89:	48 c1 e0 05          	shl    $0x5,%rax
  1e7b8d:	4c 01 c0             	add    %r8,%rax
  1e7b90:	c5 fd 10 00          	vmovupd (%rax),%ymm0
  1e7b94:	48 8b 84 24 d0 00 00 	mov    0xd0(%rsp),%rax
  1e7b9b:	00 
  1e7b9c:	c5 fd 10 08          	vmovupd (%rax),%ymm1
  1e7ba0:	48 8b 84 24 f0 00 00 	mov    0xf0(%rsp),%rax
  1e7ba7:	00 
  1e7ba8:	c5 fd 59 10          	vmulpd (%rax),%ymm0,%ymm2
  1e7bac:	c5 c5 58 fa          	vaddpd %ymm2,%ymm7,%ymm7
  1e7bb0:	48 8b 84 24 e8 00 00 	mov    0xe8(%rsp),%rax
  1e7bb7:	00 
  1e7bb8:	c5 fd 59 10          	vmulpd (%rax),%ymm0,%ymm2
  1e7bbc:	c5 35 58 ca          	vaddpd %ymm2,%ymm9,%ymm9
  1e7bc0:	48 8b 84 24 e0 00 00 	mov    0xe0(%rsp),%rax
  1e7bc7:	00 
  1e7bc8:	c5 fd 59 10          	vmulpd (%rax),%ymm0,%ymm2
  1e7bcc:	c5 f5 59 d2          	vmulpd %ymm2,%ymm1,%ymm2
  1e7bd0:	c5 1d 58 e2          	vaddpd %ymm2,%ymm12,%ymm12
  1e7bd4:	48 8b 84 24 d8 00 00 	mov    0xd8(%rsp),%rax
  1e7bdb:	00 
  1e7bdc:	c5 fd 59 00          	vmulpd (%rax),%ymm0,%ymm0
  1e7be0:	c5 f5 59 c0          	vmulpd %ymm0,%ymm1,%ymm0
  1e7be4:	c5 15 58 e8          	vaddpd %ymm0,%ymm13,%ymm13
  1e7be8:	48 8b 44 24 20       	mov    0x20(%rsp),%rax
  1e7bed:	c4 c1 25 58 c5       	vaddpd %ymm13,%ymm11,%ymm0
  1e7bf2:	c5 9d 58 ce          	vaddpd %ymm6,%ymm12,%ymm1
  1e7bf6:	c4 c1 2d 58 d1       	vaddpd %ymm9,%ymm10,%ymm2
  1e7bfb:	c5 bd 58 f7          	vaddpd %ymm7,%ymm8,%ymm6
  1e7bff:	45 89 e7             	mov    %r12d,%r15d
  1e7c02:	41 83 e7 03          	and    $0x3,%r15d
  1e7c06:	4c 89 e7             	mov    %r12,%rdi
  1e7c09:	49 bb fc ff ff ff ff 	movabs $0xffffffffffffffc,%r11
  1e7c10:	ff ff 0f 
  1e7c13:	4c 21 df             	and    %r11,%rdi
  1e7c16:	c4 e3 7d 19 f7 01    	vextractf128 $0x1,%ymm6,%xmm7
  1e7c1c:	c5 c9 58 f7          	vaddpd %xmm7,%xmm6,%xmm6
  1e7c20:	c4 e3 7d 19 d7 01    	vextractf128 $0x1,%ymm2,%xmm7
  1e7c26:	c5 e9 58 d7          	vaddpd %xmm7,%xmm2,%xmm2
  1e7c2a:	c5 c9 7c fa          	vhaddpd %xmm2,%xmm6,%xmm7
  1e7c2e:	c4 e3 7d 19 ca 01    	vextractf128 $0x1,%ymm1,%xmm2
  1e7c34:	c5 f1 58 ca          	vaddpd %xmm2,%xmm1,%xmm1
  1e7c38:	c4 e3 7d 19 c2 01    	vextractf128 $0x1,%ymm0,%xmm2
  1e7c3e:	c5 f9 58 c2          	vaddpd %xmm2,%xmm0,%xmm0
  1e7c42:	c5 f1 7c f0          	vhaddpd %xmm0,%xmm1,%xmm6
  1e7c46:	4d 85 ff             	test   %r15,%r15
  1e7c49:	4c 89 7c 24 38       	mov    %r15,0x38(%rsp)
  1e7c4e:	48 89 7c 24 18       	mov    %rdi,0x18(%rsp)
  1e7c53:	0f 84 45 01 00 00    	je     1e7d9e <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x68e>
  1e7c59:	4d 8d 5f ff          	lea    -0x1(%r15),%r11
  1e7c5d:	48 8b 84 24 18 01 00 	mov    0x118(%rsp),%rax
  1e7c64:	00 
  1e7c65:	49 39 c3             	cmp    %rax,%r11
  1e7c68:	4c 0f 43 d8          	cmovae %rax,%r11
  1e7c6c:	44 39 5c 24 58       	cmp    %r11d,0x58(%rsp)
  1e7c71:	0f 84 2c 07 00 00    	je     1e83a3 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xc93>
  1e7c77:	44 39 9c 24 80 00 00 	cmp    %r11d,0x80(%rsp)
  1e7c7e:	00 
  1e7c7f:	0f 84 03 07 00 00    	je     1e8388 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xc78>
  1e7c85:	44 39 5c 24 50       	cmp    %r11d,0x50(%rsp)
  1e7c8a:	0f 84 e0 06 00 00    	je     1e8370 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xc60>
  1e7c90:	44 39 5c 24 08       	cmp    %r11d,0x8(%rsp)
  1e7c95:	0f 84 bd 06 00 00    	je     1e8358 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xc48>
  1e7c9b:	44 39 5c 24 48       	cmp    %r11d,0x48(%rsp)
  1e7ca0:	0f 84 9a 06 00 00    	je     1e8340 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xc30>
  1e7ca6:	c4 c1 7b 12 04 f8    	vmovddup (%r8,%rdi,8),%xmm0
  1e7cac:	48 8b 44 24 68       	mov    0x68(%rsp),%rax
  1e7cb1:	c5 fb 10 0c c1       	vmovsd (%rcx,%rax,8),%xmm1
  1e7cb6:	48 8b 44 24 60       	mov    0x60(%rsp),%rax
  1e7cbb:	c4 c1 71 16 4c c5 00 	vmovhpd 0x0(%r13,%rax,8),%xmm1,%xmm1
  1e7cc2:	c5 f9 59 c9          	vmulpd %xmm1,%xmm0,%xmm1
  1e7cc6:	c5 c1 58 f9          	vaddpd %xmm1,%xmm7,%xmm7
  1e7cca:	c4 a1 7b 10 0c f2    	vmovsd (%rdx,%r14,8),%xmm1
  1e7cd0:	48 8b 84 24 10 01 00 	mov    0x110(%rsp),%rax
  1e7cd7:	00 
  1e7cd8:	c4 c1 71 16 0c c2    	vmovhpd (%r10,%rax,8),%xmm1,%xmm1
  1e7cde:	c5 f9 59 c1          	vmulpd %xmm1,%xmm0,%xmm0
  1e7ce2:	4c 8b 5c 24 30       	mov    0x30(%rsp),%r11
  1e7ce7:	c4 81 7b 12 0c d9    	vmovddup (%r9,%r11,8),%xmm1
  1e7ced:	c5 f9 59 c1          	vmulpd %xmm1,%xmm0,%xmm0
  1e7cf1:	c5 c9 58 f0          	vaddpd %xmm0,%xmm6,%xmm6
  1e7cf5:	41 83 ff 01          	cmp    $0x1,%r15d
  1e7cf9:	0f 84 9a 00 00 00    	je     1e7d99 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x689>
  1e7cff:	c4 c1 7b 12 44 f8 08 	vmovddup 0x8(%r8,%rdi,8),%xmm0
  1e7d06:	4c 8b 5c 24 68       	mov    0x68(%rsp),%r11
  1e7d0b:	c4 a1 7b 10 4c d9 08 	vmovsd 0x8(%rcx,%r11,8),%xmm1
  1e7d12:	4c 8b 5c 24 60       	mov    0x60(%rsp),%r11
  1e7d17:	c4 81 71 16 4c dd 08 	vmovhpd 0x8(%r13,%r11,8),%xmm1,%xmm1
  1e7d1e:	4c 8b 5c 24 30       	mov    0x30(%rsp),%r11
  1e7d23:	c5 f9 59 c9          	vmulpd %xmm1,%xmm0,%xmm1
  1e7d27:	c5 c1 58 f9          	vaddpd %xmm1,%xmm7,%xmm7
  1e7d2b:	c4 a1 7b 10 4c f2 08 	vmovsd 0x8(%rdx,%r14,8),%xmm1
  1e7d32:	c4 c1 71 16 4c c2 08 	vmovhpd 0x8(%r10,%rax,8),%xmm1,%xmm1
  1e7d39:	c5 f9 59 c1          	vmulpd %xmm1,%xmm0,%xmm0
  1e7d3d:	c4 81 7b 12 4c d9 08 	vmovddup 0x8(%r9,%r11,8),%xmm1
  1e7d44:	c5 f9 59 c1          	vmulpd %xmm1,%xmm0,%xmm0
  1e7d48:	c5 c9 58 f0          	vaddpd %xmm0,%xmm6,%xmm6
  1e7d4c:	41 83 ff 02          	cmp    $0x2,%r15d
  1e7d50:	74 47                	je     1e7d99 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x689>
  1e7d52:	c4 c1 7b 12 44 f8 10 	vmovddup 0x10(%r8,%rdi,8),%xmm0
  1e7d59:	48 8b 7c 24 68       	mov    0x68(%rsp),%rdi
  1e7d5e:	c5 fb 10 4c f9 10    	vmovsd 0x10(%rcx,%rdi,8),%xmm1
  1e7d64:	48 8b 7c 24 60       	mov    0x60(%rsp),%rdi
  1e7d69:	c4 c1 71 16 4c fd 10 	vmovhpd 0x10(%r13,%rdi,8),%xmm1,%xmm1
  1e7d70:	c5 f9 59 c9          	vmulpd %xmm1,%xmm0,%xmm1
  1e7d74:	c5 c1 58 f9          	vaddpd %xmm1,%xmm7,%xmm7
  1e7d78:	c4 a1 7b 10 4c f2 10 	vmovsd 0x10(%rdx,%r14,8),%xmm1
  1e7d7f:	c4 c1 71 16 4c c2 10 	vmovhpd 0x10(%r10,%rax,8),%xmm1,%xmm1
  1e7d86:	c5 f9 59 c1          	vmulpd %xmm1,%xmm0,%xmm0
  1e7d8a:	c4 81 7b 12 4c d9 10 	vmovddup 0x10(%r9,%r11,8),%xmm1
  1e7d91:	c5 f9 59 c1          	vmulpd %xmm1,%xmm0,%xmm0
  1e7d95:	c5 c9 58 f0          	vaddpd %xmm0,%xmm6,%xmm6
  1e7d99:	48 8b 44 24 20       	mov    0x20(%rsp),%rax
  1e7d9e:	4c 8b bc 24 c8 00 00 	mov    0xc8(%rsp),%r15
  1e7da5:	00 
  1e7da6:	4d 89 fe             	mov    %r15,%r14
  1e7da9:	4c 2b b4 24 b0 00 00 	sub    0xb0(%rsp),%r14
  1e7db0:	00 
  1e7db1:	4c 3b 74 24 28       	cmp    0x28(%rsp),%r14
  1e7db6:	0f 83 36 05 00 00    	jae    1e82f2 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xbe2>
  1e7dbc:	4d 89 f3             	mov    %r14,%r11
  1e7dbf:	49 c1 e3 04          	shl    $0x4,%r11
  1e7dc3:	48 8b bc 24 b8 00 00 	mov    0xb8(%rsp),%rdi
  1e7dca:	00 
  1e7dcb:	c4 a1 79 11 3c 1f    	vmovupd %xmm7,(%rdi,%r11,1)
  1e7dd1:	48 8b 7c 24 40       	mov    0x40(%rsp),%rdi
  1e7dd6:	49 39 ff             	cmp    %rdi,%r15
  1e7dd9:	73 25                	jae    1e7e00 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x6f0>
  1e7ddb:	49 ff c6             	inc    %r14
  1e7dde:	4c 3b 74 24 28       	cmp    0x28(%rsp),%r14
  1e7de3:	0f 83 36 06 00 00    	jae    1e841f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xd0f>
  1e7de9:	49 c1 e6 04          	shl    $0x4,%r14
  1e7ded:	48 8b bc 24 b8 00 00 	mov    0xb8(%rsp),%rdi
  1e7df4:	00 
  1e7df5:	c4 a1 79 11 34 37    	vmovupd %xmm6,(%rdi,%r14,1)
  1e7dfb:	48 8b 7c 24 40       	mov    0x40(%rsp),%rdi
  1e7e00:	4d 8d 5f 02          	lea    0x2(%r15),%r11
  1e7e04:	49 39 fb             	cmp    %rdi,%r11
  1e7e07:	0f 87 32 04 00 00    	ja     1e823f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb2f>
  1e7e0d:	4c 8b 9c 24 c0 00 00 	mov    0xc0(%rsp),%r11
  1e7e14:	00 
  1e7e15:	4c 3b 9c 24 a0 00 00 	cmp    0xa0(%rsp),%r11
  1e7e1c:	00 
  1e7e1d:	0f 84 e7 04 00 00    	je     1e830a <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xbfa>
  1e7e23:	4c 89 a4 24 40 01 00 	mov    %r12,0x140(%rsp)
  1e7e2a:	00 
  1e7e2b:	4c 3b 9c 24 98 00 00 	cmp    0x98(%rsp),%r11
  1e7e32:	00 
  1e7e33:	0f 84 ec 04 00 00    	je     1e8325 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xc15>
  1e7e39:	4c 8b 74 24 78       	mov    0x78(%rsp),%r14
  1e7e3e:	48 8b bc 24 28 01 00 	mov    0x128(%rsp),%rdi
  1e7e45:	00 
  1e7e46:	c4 a2 7d 19 34 df    	vbroadcastsd (%rdi,%r11,8),%ymm6
  1e7e4c:	49 c1 ee 02          	shr    $0x2,%r14
  1e7e50:	48 8b bc 24 20 01 00 	mov    0x120(%rsp),%rdi
  1e7e57:	00 
  1e7e58:	c4 a2 7d 19 3c df    	vbroadcastsd (%rdi,%r11,8),%ymm7
  1e7e5e:	48 85 ed             	test   %rbp,%rbp
  1e7e61:	48 8b 7c 24 70       	mov    0x70(%rsp),%rdi
  1e7e66:	74 60                	je     1e7ec8 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x7b8>
  1e7e68:	49 89 ff             	mov    %rdi,%r15
  1e7e6b:	49 c1 ef 02          	shr    $0x2,%r15
  1e7e6f:	45 31 db             	xor    %r11d,%r11d
  1e7e72:	45 31 e4             	xor    %r12d,%r12d
  1e7e75:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
  1e7e7c:	00 00 00 00 
  1e7e80:	4c 39 e0             	cmp    %r12,%rax
  1e7e83:	0f 84 e3 03 00 00    	je     1e826c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb5c>
  1e7e89:	4d 39 e7             	cmp    %r12,%r15
  1e7e8c:	0f 84 f0 03 00 00    	je     1e8282 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb72>
  1e7e92:	4d 39 e6             	cmp    %r12,%r14
  1e7e95:	0f 84 bb 03 00 00    	je     1e8256 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb46>
  1e7e9b:	c4 81 7d 10 04 19    	vmovupd (%r9,%r11,1),%ymm0
  1e7ea1:	c5 fd 59 c0          	vmulpd %ymm0,%ymm0,%ymm0
  1e7ea5:	c4 e2 cd a8 c7       	vfmadd213pd %ymm7,%ymm6,%ymm0
  1e7eaa:	c4 81 7d 10 0c 18    	vmovupd (%r8,%r11,1),%ymm1
  1e7eb0:	c4 a2 fd a8 0c 1b    	vfmadd213pd (%rbx,%r11,1),%ymm0,%ymm1
  1e7eb6:	49 ff c4             	inc    %r12
  1e7eb9:	c4 a1 7d 11 0c 1e    	vmovupd %ymm1,(%rsi,%r11,1)
  1e7ebf:	49 83 c3 20          	add    $0x20,%r11
  1e7ec3:	4c 39 e5             	cmp    %r12,%rbp
  1e7ec6:	75 b8                	jne    1e7e80 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x770>
  1e7ec8:	48 8b 44 24 78       	mov    0x78(%rsp),%rax
  1e7ecd:	41 89 c7             	mov    %eax,%r15d
  1e7ed0:	41 83 e7 03          	and    $0x3,%r15d
  1e7ed4:	49 89 c3             	mov    %rax,%r11
  1e7ed7:	48 b8 fc ff ff ff ff 	movabs $0xffffffffffffffc,%rax
  1e7ede:	ff ff 0f 
  1e7ee1:	49 21 c3             	and    %rax,%r11
  1e7ee4:	4c 89 5c 24 10       	mov    %r11,0x10(%rsp)
  1e7ee9:	48 83 7c 24 38 00    	cmpq   $0x0,0x38(%rsp)
  1e7eef:	0f 84 1b 01 00 00    	je     1e8010 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x900>
  1e7ef5:	48 83 7c 24 08 00    	cmpq   $0x0,0x8(%rsp)
  1e7efb:	0f 84 d9 03 00 00    	je     1e82da <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xbca>
  1e7f01:	41 89 fc             	mov    %edi,%r12d
  1e7f04:	41 83 e4 03          	and    $0x3,%r12d
  1e7f08:	0f 84 b6 03 00 00    	je     1e82c4 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xbb4>
  1e7f0e:	4d 85 ff             	test   %r15,%r15
  1e7f11:	0f 84 97 03 00 00    	je     1e82ae <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb9e>
  1e7f17:	49 89 fb             	mov    %rdi,%r11
  1e7f1a:	49 21 c3             	and    %rax,%r11
  1e7f1d:	48 8b 44 24 30       	mov    0x30(%rsp),%rax
  1e7f22:	c4 c1 7b 10 04 c1    	vmovsd (%r9,%rax,8),%xmm0
  1e7f28:	c5 fb 59 c0          	vmulsd %xmm0,%xmm0,%xmm0
  1e7f2c:	c4 e2 c9 a9 c7       	vfmadd213sd %xmm7,%xmm6,%xmm0
  1e7f31:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
  1e7f36:	c4 c1 7b 10 0c f8    	vmovsd (%r8,%rdi,8),%xmm1
  1e7f3c:	c4 a2 f9 a9 0c db    	vfmadd213sd (%rbx,%r11,8),%xmm0,%xmm1
  1e7f42:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
  1e7f47:	c5 fb 11 0c fe       	vmovsd %xmm1,(%rsi,%rdi,8)
  1e7f4c:	48 8b 7c 24 70       	mov    0x70(%rsp),%rdi
  1e7f51:	83 7c 24 38 01       	cmpl   $0x1,0x38(%rsp)
  1e7f56:	0f 84 b4 00 00 00    	je     1e8010 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x900>
  1e7f5c:	83 7c 24 08 01       	cmpl   $0x1,0x8(%rsp)
  1e7f61:	0f 84 73 03 00 00    	je     1e82da <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xbca>
  1e7f67:	41 83 fc 01          	cmp    $0x1,%r12d
  1e7f6b:	0f 84 53 03 00 00    	je     1e82c4 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xbb4>
  1e7f71:	41 83 ff 01          	cmp    $0x1,%r15d
  1e7f75:	0f 84 33 03 00 00    	je     1e82ae <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb9e>
  1e7f7b:	c4 c1 7b 10 44 c1 08 	vmovsd 0x8(%r9,%rax,8),%xmm0
  1e7f82:	c5 fb 59 c0          	vmulsd %xmm0,%xmm0,%xmm0
  1e7f86:	c4 e2 c9 a9 c7       	vfmadd213sd %xmm7,%xmm6,%xmm0
  1e7f8b:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
  1e7f90:	c4 c1 7b 10 4c f8 08 	vmovsd 0x8(%r8,%rdi,8),%xmm1
  1e7f97:	48 8b 7c 24 70       	mov    0x70(%rsp),%rdi
  1e7f9c:	c4 a2 f9 a9 4c db 08 	vfmadd213sd 0x8(%rbx,%r11,8),%xmm0,%xmm1
  1e7fa3:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
  1e7fa8:	c5 fb 11 4c c6 08    	vmovsd %xmm1,0x8(%rsi,%rax,8)
  1e7fae:	48 8b 44 24 30       	mov    0x30(%rsp),%rax
  1e7fb3:	83 7c 24 38 02       	cmpl   $0x2,0x38(%rsp)
  1e7fb8:	74 56                	je     1e8010 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x900>
  1e7fba:	83 7c 24 08 02       	cmpl   $0x2,0x8(%rsp)
  1e7fbf:	0f 84 15 03 00 00    	je     1e82da <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xbca>
  1e7fc5:	41 83 fc 02          	cmp    $0x2,%r12d
  1e7fc9:	0f 84 f5 02 00 00    	je     1e82c4 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xbb4>
  1e7fcf:	41 83 ff 02          	cmp    $0x2,%r15d
  1e7fd3:	0f 84 d5 02 00 00    	je     1e82ae <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb9e>
  1e7fd9:	c4 c1 7b 10 44 c1 10 	vmovsd 0x10(%r9,%rax,8),%xmm0
  1e7fe0:	c5 fb 59 c0          	vmulsd %xmm0,%xmm0,%xmm0
  1e7fe4:	c4 e2 c9 a9 c7       	vfmadd213sd %xmm7,%xmm6,%xmm0
  1e7fe9:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
  1e7fee:	c4 c1 7b 10 4c c0 10 	vmovsd 0x10(%r8,%rax,8),%xmm1
  1e7ff5:	c4 a2 f9 a9 4c db 10 	vfmadd213sd 0x10(%rbx,%r11,8),%xmm0,%xmm1
  1e7ffc:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
  1e8001:	c5 fb 11 4c c6 10    	vmovsd %xmm1,0x10(%rsi,%rax,8)
  1e8007:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  1e800e:	00 00 
  1e8010:	48 85 ed             	test   %rbp,%rbp
  1e8013:	0f 84 81 00 00 00    	je     1e809a <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x98a>
  1e8019:	45 31 db             	xor    %r11d,%r11d
  1e801c:	45 31 e4             	xor    %r12d,%r12d
  1e801f:	90                   	nop
  1e8020:	4d 39 e6             	cmp    %r12,%r14
  1e8023:	0f 84 6f 02 00 00    	je     1e8298 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb88>
  1e8029:	c4 e2 7d 19 05 2e b7 	vbroadcastsd -0x1c48d2(%rip),%ymm0        # 23760 <anon.320a9f19b214853d1b1894d26beaba85.4.llvm.13199781604905473672+0xd8>
  1e8030:	e3 ff 
  1e8032:	c4 81 7d 54 0c 18    	vandpd (%r8,%r11,1),%ymm0,%ymm1
  1e8038:	c4 a1 7d 54 04 1e    	vandpd (%rsi,%r11,1),%ymm0,%ymm0
  1e803e:	c5 f5 c2 d1 00       	vcmpeqpd %ymm1,%ymm1,%ymm2
  1e8043:	c5 fd c2 f0 00       	vcmpeqpd %ymm0,%ymm0,%ymm6
  1e8048:	c5 f5 5f f8          	vmaxpd %ymm0,%ymm1,%ymm7
  1e804c:	c4 e3 75 4b cf 60    	vblendvpd %ymm6,%ymm7,%ymm1,%ymm1
  1e8052:	c4 e3 7d 4b c1 20    	vblendvpd %ymm2,%ymm1,%ymm0,%ymm0
  1e8058:	c5 fd c2 cc 12       	vcmple_oqpd %ymm4,%ymm0,%ymm1
  1e805d:	c5 fd c2 d5 00       	vcmpeqpd %ymm5,%ymm0,%ymm2
  1e8062:	c4 e3 7d 4b c4 20    	vblendvpd %ymm2,%ymm4,%ymm0,%ymm0
  1e8068:	c4 e2 55 37 c9       	vpcmpgtq %ymm1,%ymm5,%ymm1
  1e806d:	c5 f5 db c0          	vpand  %ymm0,%ymm1,%ymm0
  1e8071:	c4 e3 7d 39 c1 01    	vextracti128 $0x1,%ymm0,%xmm1
  1e8077:	c5 f9 5d c1          	vminpd %xmm1,%xmm0,%xmm0
  1e807b:	c5 f9 c6 c8 01       	vshufpd $0x1,%xmm0,%xmm0,%xmm1
  1e8080:	c5 fb 5d c1          	vminsd %xmm1,%xmm0,%xmm0
  1e8084:	c5 f9 2e d8          	vucomisd %xmm0,%xmm3
  1e8088:	0f 87 ad 01 00 00    	ja     1e823b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb2b>
  1e808e:	49 ff c4             	inc    %r12
  1e8091:	49 83 c3 20          	add    $0x20,%r11
  1e8095:	4c 39 e5             	cmp    %r12,%rbp
  1e8098:	75 86                	jne    1e8020 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x910>
  1e809a:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
  1e809f:	49 39 c7             	cmp    %rax,%r15
  1e80a2:	49 0f 42 c7          	cmovb  %r15,%rax
  1e80a6:	49 89 c7             	mov    %rax,%r15
  1e80a9:	48 85 c0             	test   %rax,%rax
  1e80ac:	48 8b 84 24 40 01 00 	mov    0x140(%rsp),%rax
  1e80b3:	00 
  1e80b4:	74 66                	je     1e811c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa0c>
  1e80b6:	49 89 fc             	mov    %rdi,%r12
  1e80b9:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
  1e80be:	c4 c1 7b 10 04 f8    	vmovsd (%r8,%rdi,8),%xmm0
  1e80c4:	c5 fb 12 15 94 b6 e3 	vmovddup -0x1c496c(%rip),%xmm2        # 23760 <anon.320a9f19b214853d1b1894d26beaba85.4.llvm.13199781604905473672+0xd8>
  1e80cb:	ff 
  1e80cc:	c5 f9 54 c2          	vandpd %xmm2,%xmm0,%xmm0
  1e80d0:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
  1e80d5:	c5 fb 10 0c fe       	vmovsd (%rsi,%rdi,8),%xmm1
  1e80da:	c5 f1 54 ca          	vandpd %xmm2,%xmm1,%xmm1
  1e80de:	c5 fb 5f d1          	vmaxsd %xmm1,%xmm0,%xmm2
  1e80e2:	c5 f3 c2 c9 03       	vcmpunordsd %xmm1,%xmm1,%xmm1
  1e80e7:	c4 e3 69 4b f0 10    	vblendvpd %xmm1,%xmm0,%xmm2,%xmm6
  1e80ed:	c5 fb 10 05 fb b6 e3 	vmovsd -0x1c4905(%rip),%xmm0        # 237f0 <anon.320a9f19b214853d1b1894d26beaba85.4.llvm.13199781604905473672+0x168>
  1e80f4:	ff 
  1e80f5:	c5 f9 2e c6          	vucomisd %xmm6,%xmm0
  1e80f9:	72 12                	jb     1e810d <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x9fd>
  1e80fb:	c5 f9 57 c0          	vxorpd %xmm0,%xmm0,%xmm0
  1e80ff:	c5 f9 2e f0          	vucomisd %xmm0,%xmm6
  1e8103:	75 02                	jne    1e8107 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x9f7>
  1e8105:	7b 4a                	jnp    1e8151 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa41>
  1e8107:	c5 f9 2e f3          	vucomisd %xmm3,%xmm6
  1e810b:	73 44                	jae    1e8151 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa41>
  1e810d:	45 31 db             	xor    %r11d,%r11d
  1e8110:	4c 89 e7             	mov    %r12,%rdi
  1e8113:	45 39 fb             	cmp    %r15d,%r11d
  1e8116:	0f 82 1f 01 00 00    	jb     1e823b <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb2b>
  1e811c:	48 8b ac 24 c8 00 00 	mov    0xc8(%rsp),%rbp
  1e8123:	00 
  1e8124:	4c 8d 75 01          	lea    0x1(%rbp),%r14
  1e8128:	48 ff 84 24 c0 00 00 	incq   0xc0(%rsp)
  1e812f:	00 
  1e8130:	41 bf 01 00 00 00    	mov    $0x1,%r15d
  1e8136:	4d 89 c3             	mov    %r8,%r11
  1e8139:	49 89 f0             	mov    %rsi,%r8
  1e813c:	4c 8b 64 24 78       	mov    0x78(%rsp),%r12
  1e8141:	48 3b 6c 24 40       	cmp    0x40(%rsp),%rbp
  1e8146:	0f 82 75 f8 ff ff    	jb     1e79c1 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0x2b1>
  1e814c:	e9 ee 00 00 00       	jmp    1e823f <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb2f>
  1e8151:	41 83 ff 01          	cmp    $0x1,%r15d
  1e8155:	4c 89 e7             	mov    %r12,%rdi
  1e8158:	74 c2                	je     1e811c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa0c>
  1e815a:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
  1e815f:	c4 c1 7b 10 44 f8 08 	vmovsd 0x8(%r8,%rdi,8),%xmm0
  1e8166:	c5 fb 12 15 f2 b5 e3 	vmovddup -0x1c4a0e(%rip),%xmm2        # 23760 <anon.320a9f19b214853d1b1894d26beaba85.4.llvm.13199781604905473672+0xd8>
  1e816d:	ff 
  1e816e:	c5 f9 54 c2          	vandpd %xmm2,%xmm0,%xmm0
  1e8172:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
  1e8177:	c5 fb 10 4c fe 08    	vmovsd 0x8(%rsi,%rdi,8),%xmm1
  1e817d:	c5 f1 54 ca          	vandpd %xmm2,%xmm1,%xmm1
  1e8181:	c5 fb 5f d1          	vmaxsd %xmm1,%xmm0,%xmm2
  1e8185:	c5 f3 c2 c9 03       	vcmpunordsd %xmm1,%xmm1,%xmm1
  1e818a:	c4 e3 69 4b f0 10    	vblendvpd %xmm1,%xmm0,%xmm2,%xmm6
  1e8190:	41 bb 01 00 00 00    	mov    $0x1,%r11d
  1e8196:	c5 fb 10 05 52 b6 e3 	vmovsd -0x1c49ae(%rip),%xmm0        # 237f0 <anon.320a9f19b214853d1b1894d26beaba85.4.llvm.13199781604905473672+0x168>
  1e819d:	ff 
  1e819e:	c5 f9 2e c6          	vucomisd %xmm6,%xmm0
  1e81a2:	0f 82 68 ff ff ff    	jb     1e8110 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa00>
  1e81a8:	c5 f9 57 c0          	vxorpd %xmm0,%xmm0,%xmm0
  1e81ac:	c5 f9 2e f0          	vucomisd %xmm0,%xmm6
  1e81b0:	4c 89 e7             	mov    %r12,%rdi
  1e81b3:	75 02                	jne    1e81b7 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xaa7>
  1e81b5:	7b 0a                	jnp    1e81c1 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xab1>
  1e81b7:	c5 f9 2e f3          	vucomisd %xmm3,%xmm6
  1e81bb:	0f 82 52 ff ff ff    	jb     1e8113 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa03>
  1e81c1:	41 83 ff 02          	cmp    $0x2,%r15d
  1e81c5:	0f 84 51 ff ff ff    	je     1e811c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa0c>
  1e81cb:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
  1e81d0:	c4 c1 7b 10 44 f8 10 	vmovsd 0x10(%r8,%rdi,8),%xmm0
  1e81d7:	c5 fb 12 15 81 b5 e3 	vmovddup -0x1c4a7f(%rip),%xmm2        # 23760 <anon.320a9f19b214853d1b1894d26beaba85.4.llvm.13199781604905473672+0xd8>
  1e81de:	ff 
  1e81df:	c5 f9 54 c2          	vandpd %xmm2,%xmm0,%xmm0
  1e81e3:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
  1e81e8:	c5 fb 10 4c fe 10    	vmovsd 0x10(%rsi,%rdi,8),%xmm1
  1e81ee:	c5 f1 54 ca          	vandpd %xmm2,%xmm1,%xmm1
  1e81f2:	c5 fb 5f d1          	vmaxsd %xmm1,%xmm0,%xmm2
  1e81f6:	c5 f3 c2 c9 03       	vcmpunordsd %xmm1,%xmm1,%xmm1
  1e81fb:	c4 e3 69 4b f0 10    	vblendvpd %xmm1,%xmm0,%xmm2,%xmm6
  1e8201:	41 bb 02 00 00 00    	mov    $0x2,%r11d
  1e8207:	c5 fb 10 05 e1 b5 e3 	vmovsd -0x1c4a1f(%rip),%xmm0        # 237f0 <anon.320a9f19b214853d1b1894d26beaba85.4.llvm.13199781604905473672+0x168>
  1e820e:	ff 
  1e820f:	c5 f9 2e c6          	vucomisd %xmm6,%xmm0
  1e8213:	0f 82 f7 fe ff ff    	jb     1e8110 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa00>
  1e8219:	c5 f9 57 c0          	vxorpd %xmm0,%xmm0,%xmm0
  1e821d:	c5 f9 2e f0          	vucomisd %xmm0,%xmm6
  1e8221:	4c 89 e7             	mov    %r12,%rdi
  1e8224:	75 06                	jne    1e822c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb1c>
  1e8226:	0f 8b f0 fe ff ff    	jnp    1e811c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa0c>
  1e822c:	c5 f9 2e f3          	vucomisd %xmm3,%xmm6
  1e8230:	0f 82 dd fe ff ff    	jb     1e8113 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa03>
  1e8236:	e9 e1 fe ff ff       	jmp    1e811c <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xa0c>
  1e823b:	31 c0                	xor    %eax,%eax
  1e823d:	eb 02                	jmp    1e8241 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::FusedScalarAnalysisPair>>+0xb31>
  1e823f:	b0 01                	mov    $0x1,%al
  1e8241:	48 81 c4 48 01 00 00 	add    $0x148,%rsp
  1e8248:	5b                   	pop    %rbx
  1e8249:	41 5c                	pop    %r12
  1e824b:	41 5d                	pop    %r13
  1e824d:	41 5e                	pop    %r14
  1e824f:	41 5f                	pop    %r15
  1e8251:	5d                   	pop    %rbp
  1e8252:	c5 f8 77             	vzeroupper
  1e8255:	c3                   	ret
  1e8256:	48 8d 15 d3 7a 0a 00 	lea    0xa7ad3(%rip),%rdx        # 28fd30 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x14a0>
  1e825d:	4c 89 f7             	mov    %r14,%rdi
  1e8260:	4c 89 f6             	mov    %r14,%rsi
  1e8263:	c5 f8 77             	vzeroupper
  1e8266:	ff 15 c4 01 0b 00    	call   *0xb01c4(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e826c:	48 8d 15 8d 7a 0a 00 	lea    0xa7a8d(%rip),%rdx        # 28fd00 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x1470>
  1e8273:	48 89 c7             	mov    %rax,%rdi
  1e8276:	48 89 c6             	mov    %rax,%rsi
  1e8279:	c5 f8 77             	vzeroupper
  1e827c:	ff 15 ae 01 0b 00    	call   *0xb01ae(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e8282:	48 8d 15 8f 7a 0a 00 	lea    0xa7a8f(%rip),%rdx        # 28fd18 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x1488>
  1e8289:	4c 89 ff             	mov    %r15,%rdi
  1e828c:	4c 89 fe             	mov    %r15,%rsi
  1e828f:	c5 f8 77             	vzeroupper
  1e8292:	ff 15 98 01 0b 00    	call   *0xb0198(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e8298:	48 8d 15 39 69 0a 00 	lea    0xa6939(%rip),%rdx        # 28ebd8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x348>
  1e829f:	4c 89 f7             	mov    %r14,%rdi
  1e82a2:	4c 89 f6             	mov    %r14,%rsi
  1e82a5:	c5 f8 77             	vzeroupper
  1e82a8:	ff 15 82 01 0b 00    	call   *0xb0182(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e82ae:	48 8d 15 33 7a 0a 00 	lea    0xa7a33(%rip),%rdx        # 28fce8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x1458>
  1e82b5:	4c 89 ff             	mov    %r15,%rdi
  1e82b8:	4c 89 fe             	mov    %r15,%rsi
  1e82bb:	c5 f8 77             	vzeroupper
  1e82be:	ff 15 6c 01 0b 00    	call   *0xb016c(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e82c4:	48 8d 15 05 7a 0a 00 	lea    0xa7a05(%rip),%rdx        # 28fcd0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x1440>
  1e82cb:	4c 89 e7             	mov    %r12,%rdi
  1e82ce:	4c 89 e6             	mov    %r12,%rsi
  1e82d1:	c5 f8 77             	vzeroupper
  1e82d4:	ff 15 56 01 0b 00    	call   *0xb0156(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e82da:	48 8d 15 d7 79 0a 00 	lea    0xa79d7(%rip),%rdx        # 28fcb8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x1428>
  1e82e1:	48 8b 7c 24 08       	mov    0x8(%rsp),%rdi
  1e82e6:	48 89 fe             	mov    %rdi,%rsi
  1e82e9:	c5 f8 77             	vzeroupper
  1e82ec:	ff 15 3e 01 0b 00    	call   *0xb013e(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e82f2:	48 8d 15 5f 79 0a 00 	lea    0xa795f(%rip),%rdx        # 28fc58 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x13c8>
  1e82f9:	4c 89 f7             	mov    %r14,%rdi
  1e82fc:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
  1e8301:	c5 f8 77             	vzeroupper
  1e8304:	ff 15 26 01 0b 00    	call   *0xb0126(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e830a:	48 8d 15 77 79 0a 00 	lea    0xa7977(%rip),%rdx        # 28fc88 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x13f8>
  1e8311:	48 8b bc 24 a0 00 00 	mov    0xa0(%rsp),%rdi
  1e8318:	00 
  1e8319:	48 89 fe             	mov    %rdi,%rsi
  1e831c:	c5 f8 77             	vzeroupper
  1e831f:	ff 15 0b 01 0b 00    	call   *0xb010b(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e8325:	48 8d 15 74 79 0a 00 	lea    0xa7974(%rip),%rdx        # 28fca0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x1410>
  1e832c:	48 8b bc 24 98 00 00 	mov    0x98(%rsp),%rdi
  1e8333:	00 
  1e8334:	48 89 fe             	mov    %rdi,%rsi
  1e8337:	c5 f8 77             	vzeroupper
  1e833a:	ff 15 f0 00 0b 00    	call   *0xb00f0(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e8340:	48 8d 15 e9 6d 0a 00 	lea    0xa6de9(%rip),%rdx        # 28f130 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x8a0>
  1e8347:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
  1e834c:	48 89 fe             	mov    %rdi,%rsi
  1e834f:	c5 f8 77             	vzeroupper
  1e8352:	ff 15 d8 00 0b 00    	call   *0xb00d8(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e8358:	48 8d 15 b9 6d 0a 00 	lea    0xa6db9(%rip),%rdx        # 28f118 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x888>
  1e835f:	48 8b 7c 24 08       	mov    0x8(%rsp),%rdi
  1e8364:	48 89 fe             	mov    %rdi,%rsi
  1e8367:	c5 f8 77             	vzeroupper
  1e836a:	ff 15 c0 00 0b 00    	call   *0xb00c0(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e8370:	48 8d 15 89 6d 0a 00 	lea    0xa6d89(%rip),%rdx        # 28f100 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x870>
  1e8377:	48 8b 7c 24 50       	mov    0x50(%rsp),%rdi
  1e837c:	48 89 fe             	mov    %rdi,%rsi
  1e837f:	c5 f8 77             	vzeroupper
  1e8382:	ff 15 a8 00 0b 00    	call   *0xb00a8(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e8388:	48 8d 15 59 6d 0a 00 	lea    0xa6d59(%rip),%rdx        # 28f0e8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x858>
  1e838f:	48 8b bc 24 80 00 00 	mov    0x80(%rsp),%rdi
  1e8396:	00 
  1e8397:	48 89 fe             	mov    %rdi,%rsi
  1e839a:	c5 f8 77             	vzeroupper
  1e839d:	ff 15 8d 00 0b 00    	call   *0xb008d(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e83a3:	48 8d 15 26 6d 0a 00 	lea    0xa6d26(%rip),%rdx        # 28f0d0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x840>
  1e83aa:	48 8b 7c 24 58       	mov    0x58(%rsp),%rdi
  1e83af:	48 89 fe             	mov    %rdi,%rsi
  1e83b2:	c5 f8 77             	vzeroupper
  1e83b5:	ff 15 75 00 0b 00    	call   *0xb0075(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e83bb:	48 8d 15 f6 6c 0a 00 	lea    0xa6cf6(%rip),%rdx        # 28f0b8 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x828>
  1e83c2:	31 ff                	xor    %edi,%edi
  1e83c4:	31 f6                	xor    %esi,%esi
  1e83c6:	c5 f8 77             	vzeroupper
  1e83c9:	ff 15 61 00 0b 00    	call   *0xb0061(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e83cf:	48 8d 15 ca 6c 0a 00 	lea    0xa6cca(%rip),%rdx        # 28f0a0 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x810>
  1e83d6:	31 ff                	xor    %edi,%edi
  1e83d8:	31 f6                	xor    %esi,%esi
  1e83da:	c5 f8 77             	vzeroupper
  1e83dd:	ff 15 4d 00 0b 00    	call   *0xb004d(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e83e3:	48 8d 15 9e 6c 0a 00 	lea    0xa6c9e(%rip),%rdx        # 28f088 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x7f8>
  1e83ea:	31 ff                	xor    %edi,%edi
  1e83ec:	31 f6                	xor    %esi,%esi
  1e83ee:	c5 f8 77             	vzeroupper
  1e83f1:	ff 15 39 00 0b 00    	call   *0xb0039(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e83f7:	48 8d 15 72 6c 0a 00 	lea    0xa6c72(%rip),%rdx        # 28f070 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x7e0>
  1e83fe:	31 ff                	xor    %edi,%edi
  1e8400:	31 f6                	xor    %esi,%esi
  1e8402:	c5 f8 77             	vzeroupper
  1e8405:	ff 15 25 00 0b 00    	call   *0xb0025(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e840b:	48 8d 15 46 6c 0a 00 	lea    0xa6c46(%rip),%rdx        # 28f058 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x7c8>
  1e8412:	31 ff                	xor    %edi,%edi
  1e8414:	31 f6                	xor    %esi,%esi
  1e8416:	c5 f8 77             	vzeroupper
  1e8419:	ff 15 11 00 0b 00    	call   *0xb0011(%rip)        # 298430 <_DYNAMIC+0x2c0>
  1e841f:	48 8d 15 4a 78 0a 00 	lea    0xa784a(%rip),%rdx        # 28fc70 <anon.0bae4e66a8b77356723805bdcd86bdb6.44.llvm.10768078440944648525+0x13e0>
  1e8426:	4c 89 f7             	mov    %r14,%rdi
  1e8429:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
  1e842e:	c5 f8 77             	vzeroupper
  1e8431:	ff 15 f9 ff 0a 00    	call   *0xafff9(%rip)        # 298430 <_DYNAMIC+0x2c0>
