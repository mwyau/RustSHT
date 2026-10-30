
target/release/deps/rustsht-7d36680db74a2dd3:     file format elf64-x86-64


Disassembly of section .text:

000000000075a9c0 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>>:
  75a9c0:	31 c9                	xor    %ecx,%ecx
  75a9c2:	48 83 7f 48 08       	cmpq   $0x8,0x48(%rdi)
  75a9c7:	0f 85 79 03 00 00    	jne    75ad46 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x386>
  75a9cd:	48 83 7f 68 08       	cmpq   $0x8,0x68(%rdi)
  75a9d2:	0f 85 6e 03 00 00    	jne    75ad46 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x386>
  75a9d8:	48 83 bf 88 00 00 00 	cmpq   $0x8,0x88(%rdi)
  75a9df:	08
  75a9e0:	0f 85 60 03 00 00    	jne    75ad46 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x386>
  75a9e6:	4c 8b 47 08          	mov    0x8(%rdi),%r8
  75a9ea:	4c 3b 47 18          	cmp    0x18(%rdi),%r8
  75a9ee:	0f 85 50 03 00 00    	jne    75ad44 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x384>
  75a9f4:	4c 3b 47 28          	cmp    0x28(%rdi),%r8
  75a9f8:	0f 85 46 03 00 00    	jne    75ad44 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x384>
  75a9fe:	4c 3b 47 38          	cmp    0x38(%rdi),%r8
  75aa02:	0f 85 3c 03 00 00    	jne    75ad44 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x384>
  75aa08:	48 83 bf e8 00 00 00 	cmpq   $0x8,0xe8(%rdi)
  75aa0f:	08
  75aa10:	0f 85 2e 03 00 00    	jne    75ad44 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x384>
  75aa16:	53                   	push   %rbx
  75aa17:	48 83 ec 40          	sub    $0x40,%rsp
  75aa1b:	4c 8b 57 40          	mov    0x40(%rdi),%r10
  75aa1f:	4c 8b 5f 60          	mov    0x60(%rdi),%r11
  75aa23:	48 8b 9f 80 00 00 00 	mov    0x80(%rdi),%rbx
  75aa2a:	48 8b 0f             	mov    (%rdi),%rcx
  75aa2d:	4c 8b 4f 10          	mov    0x10(%rdi),%r9
  75aa31:	48 8b 57 20          	mov    0x20(%rdi),%rdx
  75aa35:	48 8b 77 30          	mov    0x30(%rdi),%rsi
  75aa39:	48 8b 87 e0 00 00 00 	mov    0xe0(%rdi),%rax
  75aa40:	c4 41 7c 10 3a       	vmovups (%r10),%ymm15
  75aa45:	c4 c1 7c 10 0b       	vmovups (%r11),%ymm1
  75aa4a:	c5 fc 10 1b          	vmovups (%rbx),%ymm3
  75aa4e:	80 bf f0 00 00 00 01 	cmpb   $0x1,0xf0(%rdi)
  75aa55:	0f 85 ee 02 00 00    	jne    75ad49 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x389>
  75aa5b:	c5 fc 11 4c 24 20    	vmovups %ymm1,0x20(%rsp)
  75aa61:	c4 41 28 57 d2       	vxorps %xmm10,%xmm10,%xmm10
  75aa66:	c5 d0 57 ed          	vxorps %xmm5,%xmm5,%xmm5
  75aa6a:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  75aa6e:	c5 c0 57 ff          	vxorps %xmm7,%xmm7,%xmm7
  75aa72:	4d 85 c0             	test   %r8,%r8
  75aa75:	0f 84 36 02 00 00    	je     75acb1 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x2f1>
  75aa7b:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  75aa7f:	c5 84 59 c0          	vmulps %ymm0,%ymm15,%ymm0
  75aa83:	c5 fc 11 04 24       	vmovups %ymm0,(%rsp)
  75aa88:	31 ff                	xor    %edi,%edi
  75aa8a:	c4 41 18 57 e4       	vxorps %xmm12,%xmm12,%xmm12
  75aa8f:	c4 41 10 57 ed       	vxorps %xmm13,%xmm13,%xmm13
  75aa94:	c4 41 20 57 db       	vxorps %xmm11,%xmm11,%xmm11
  75aa99:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  75aaa0:	c4 62 7d 18 34 be    	vbroadcastss (%rsi,%rdi,4),%ymm14
  75aaa6:	c5 7c 29 fa          	vmovaps %ymm15,%ymm2
  75aaaa:	c5 0c 59 fb          	vmulps %ymm3,%ymm14,%ymm15
  75aaae:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  75aab2:	c5 e4 59 c0          	vmulps %ymm0,%ymm3,%ymm0
  75aab6:	c4 c1 24 59 ce       	vmulps %ymm14,%ymm11,%ymm1
  75aabb:	c4 42 65 aa f7       	vfmsub213ps %ymm15,%ymm3,%ymm14
  75aac0:	c5 fc 58 c9          	vaddps %ymm1,%ymm0,%ymm1
  75aac4:	c5 8c 58 c9          	vaddps %ymm1,%ymm14,%ymm1
  75aac8:	c5 04 58 f1          	vaddps %ymm1,%ymm15,%ymm14
  75aacc:	c4 41 2c 58 ce       	vaddps %ymm14,%ymm10,%ymm9
  75aad1:	c4 c1 34 5c f2       	vsubps %ymm10,%ymm9,%ymm6
  75aad6:	c5 34 5c c6          	vsubps %ymm6,%ymm9,%ymm8
  75aada:	c4 41 2c 5c c0       	vsubps %ymm8,%ymm10,%ymm8
  75aadf:	c5 8c 5c f6          	vsubps %ymm6,%ymm14,%ymm6
  75aae3:	c5 bc 58 f6          	vaddps %ymm6,%ymm8,%ymm6
  75aae7:	c4 41 0c 5c c7       	vsubps %ymm15,%ymm14,%ymm8
  75aaec:	c4 41 0c 5c d0       	vsubps %ymm8,%ymm14,%ymm10
  75aaf1:	c4 41 04 5c d2       	vsubps %ymm10,%ymm15,%ymm10
  75aaf6:	c5 7c 28 fa          	vmovaps %ymm2,%ymm15
  75aafa:	c4 c1 74 5c c8       	vsubps %ymm8,%ymm1,%ymm1
  75aaff:	c5 ac 58 c9          	vaddps %ymm1,%ymm10,%ymm1
  75ab03:	c5 c4 58 c9          	vaddps %ymm1,%ymm7,%ymm1
  75ab07:	c5 cc 58 c9          	vaddps %ymm1,%ymm6,%ymm1
  75ab0b:	c5 34 58 d1          	vaddps %ymm1,%ymm9,%ymm10
  75ab0f:	c4 c1 2c 5c f1       	vsubps %ymm9,%ymm10,%ymm6
  75ab14:	c5 ac 5c fe          	vsubps %ymm6,%ymm10,%ymm7
  75ab18:	c5 b4 5c ff          	vsubps %ymm7,%ymm9,%ymm7
  75ab1c:	c5 f4 5c ce          	vsubps %ymm6,%ymm1,%ymm1
  75ab20:	c5 f4 58 ff          	vaddps %ymm7,%ymm1,%ymm7
  75ab24:	c4 e2 7d 18 0c ba    	vbroadcastss (%rdx,%rdi,4),%ymm1
  75ab2a:	c5 e4 59 f1          	vmulps %ymm1,%ymm3,%ymm6
  75ab2e:	c5 24 59 c1          	vmulps %ymm1,%ymm11,%ymm8
  75ab32:	c4 e2 65 aa ce       	vfmsub213ps %ymm6,%ymm3,%ymm1
  75ab37:	c5 bc 58 c0          	vaddps %ymm0,%ymm8,%ymm0
  75ab3b:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  75ab3f:	c5 cc 58 c8          	vaddps %ymm0,%ymm6,%ymm1
  75ab43:	c5 5c 58 c1          	vaddps %ymm1,%ymm4,%ymm8
  75ab47:	c5 3c 5c cc          	vsubps %ymm4,%ymm8,%ymm9
  75ab4b:	c4 41 3c 5c f1       	vsubps %ymm9,%ymm8,%ymm14
  75ab50:	c4 c1 5c 5c e6       	vsubps %ymm14,%ymm4,%ymm4
  75ab55:	c4 62 7d 18 34 b9    	vbroadcastss (%rcx,%rdi,4),%ymm14
  75ab5b:	c4 41 74 5c c9       	vsubps %ymm9,%ymm1,%ymm9
  75ab60:	c5 b4 58 e4          	vaddps %ymm4,%ymm9,%ymm4
  75ab64:	c5 74 5c ce          	vsubps %ymm6,%ymm1,%ymm9
  75ab68:	c4 c1 74 5c c9       	vsubps %ymm9,%ymm1,%ymm1
  75ab6d:	c5 cc 5c c9          	vsubps %ymm1,%ymm6,%ymm1
  75ab71:	c4 c1 7c 5c c1       	vsubps %ymm9,%ymm0,%ymm0
  75ab76:	c5 fc 58 c1          	vaddps %ymm1,%ymm0,%ymm0
  75ab7a:	c5 d4 58 c0          	vaddps %ymm0,%ymm5,%ymm0
  75ab7e:	c5 dc 58 c0          	vaddps %ymm0,%ymm4,%ymm0
  75ab82:	c5 bc 58 e0          	vaddps %ymm0,%ymm8,%ymm4
  75ab86:	c4 c1 5c 5c c8       	vsubps %ymm8,%ymm4,%ymm1
  75ab8b:	c5 dc 5c e9          	vsubps %ymm1,%ymm4,%ymm5
  75ab8f:	c5 bc 5c ed          	vsubps %ymm5,%ymm8,%ymm5
  75ab93:	c5 fc 5c c1          	vsubps %ymm1,%ymm0,%ymm0
  75ab97:	c5 fc 58 ed          	vaddps %ymm5,%ymm0,%ymm5
  75ab9b:	c4 c2 7d 18 04 b9    	vbroadcastss (%r9,%rdi,4),%ymm0
  75aba1:	c5 8c 59 ca          	vmulps %ymm2,%ymm14,%ymm1
  75aba5:	c5 e8 57 d2          	vxorps %xmm2,%xmm2,%xmm2
  75aba9:	c5 8a 59 f2          	vmulss %xmm2,%xmm14,%xmm6
  75abad:	c4 62 05 aa f1       	vfmsub213ps %ymm1,%ymm15,%ymm14
  75abb2:	c4 e2 7d 18 f6       	vbroadcastss %xmm6,%ymm6
  75abb7:	c5 cc 58 34 24       	vaddps (%rsp),%ymm6,%ymm6
  75abbc:	c5 8c 58 f6          	vaddps %ymm6,%ymm14,%ymm6
  75abc0:	c5 74 58 c6          	vaddps %ymm6,%ymm1,%ymm8
  75abc4:	c5 3c 5c c9          	vsubps %ymm1,%ymm8,%ymm9
  75abc8:	c4 41 3c 5c f1       	vsubps %ymm9,%ymm8,%ymm14
  75abcd:	c4 c1 74 5c ce       	vsubps %ymm14,%ymm1,%ymm1
  75abd2:	c4 c1 4c 5c f1       	vsubps %ymm9,%ymm6,%ymm6
  75abd7:	c5 cc 58 c9          	vaddps %ymm1,%ymm6,%ymm1
  75abdb:	c5 bc 58 f0          	vaddps %ymm0,%ymm8,%ymm6
  75abdf:	c4 41 4c 5c c8       	vsubps %ymm8,%ymm6,%ymm9
  75abe4:	c4 41 4c 5c f1       	vsubps %ymm9,%ymm6,%ymm14
  75abe9:	c4 41 3c 5c c6       	vsubps %ymm14,%ymm8,%ymm8
  75abee:	c4 c1 7c 5c c1       	vsubps %ymm9,%ymm0,%ymm0
  75abf3:	c5 bc 58 c0          	vaddps %ymm0,%ymm8,%ymm0
  75abf7:	c5 e8 57 d2          	vxorps %xmm2,%xmm2,%xmm2
  75abfb:	c5 f4 58 ca          	vaddps %ymm2,%ymm1,%ymm1
  75abff:	c5 fc 58 c1          	vaddps %ymm1,%ymm0,%ymm0
  75ac03:	c5 cc 58 c8          	vaddps %ymm0,%ymm6,%ymm1
  75ac07:	c5 74 5c c6          	vsubps %ymm6,%ymm1,%ymm8
  75ac0b:	c4 41 74 5c c8       	vsubps %ymm8,%ymm1,%ymm9
  75ac10:	c4 c1 4c 5c f1       	vsubps %ymm9,%ymm6,%ymm6
  75ac15:	c4 c1 7c 5c c0       	vsubps %ymm8,%ymm0,%ymm0
  75ac1a:	c5 fc 58 c6          	vaddps %ymm6,%ymm0,%ymm0
  75ac1e:	c5 e4 59 f1          	vmulps %ymm1,%ymm3,%ymm6
  75ac22:	c5 7c 28 c3          	vmovaps %ymm3,%ymm8
  75ac26:	c4 62 75 aa c6       	vfmsub213ps %ymm6,%ymm1,%ymm8
  75ac2b:	c5 a4 59 c9          	vmulps %ymm1,%ymm11,%ymm1
  75ac2f:	c5 e4 59 c0          	vmulps %ymm0,%ymm3,%ymm0
  75ac33:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  75ac37:	c5 bc 58 c0          	vaddps %ymm0,%ymm8,%ymm0
  75ac3b:	c5 cc 58 c8          	vaddps %ymm0,%ymm6,%ymm1
  75ac3f:	c5 74 5c c6          	vsubps %ymm6,%ymm1,%ymm8
  75ac43:	c4 41 74 5c c8       	vsubps %ymm8,%ymm1,%ymm9
  75ac48:	c4 c1 4c 5c f1       	vsubps %ymm9,%ymm6,%ymm6
  75ac4d:	c4 c1 7c 5c c0       	vsubps %ymm8,%ymm0,%ymm0
  75ac52:	c5 fc 58 c6          	vaddps %ymm6,%ymm0,%ymm0
  75ac56:	c5 94 58 c0          	vaddps %ymm0,%ymm13,%ymm0
  75ac5a:	c5 9c 58 f1          	vaddps %ymm1,%ymm12,%ymm6
  75ac5e:	c5 4c 5c c1          	vsubps %ymm1,%ymm6,%ymm8
  75ac62:	c4 41 4c 5c c8       	vsubps %ymm8,%ymm6,%ymm9
  75ac67:	c4 c1 74 5c c9       	vsubps %ymm9,%ymm1,%ymm1
  75ac6c:	c4 41 1c 5c c0       	vsubps %ymm8,%ymm12,%ymm8
  75ac71:	c5 bc 58 c9          	vaddps %ymm1,%ymm8,%ymm1
  75ac75:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  75ac79:	c5 cc 58 c8          	vaddps %ymm0,%ymm6,%ymm1
  75ac7d:	c5 74 5c c6          	vsubps %ymm6,%ymm1,%ymm8
  75ac81:	c4 41 74 5c c8       	vsubps %ymm8,%ymm1,%ymm9
  75ac86:	c4 c1 4c 5c f1       	vsubps %ymm9,%ymm6,%ymm6
  75ac8b:	c4 c1 7c 5c c0       	vsubps %ymm8,%ymm0,%ymm0
  75ac90:	c5 fc 58 c6          	vaddps %ymm6,%ymm0,%ymm0
  75ac94:	c5 7c 28 e3          	vmovaps %ymm3,%ymm12
  75ac98:	c4 41 7c 28 eb       	vmovaps %ymm11,%ymm13
  75ac9d:	48 ff c7             	inc    %rdi
  75aca0:	c5 fc 28 d9          	vmovaps %ymm1,%ymm3
  75aca4:	c5 7c 28 d8          	vmovaps %ymm0,%ymm11
  75aca8:	49 39 f8             	cmp    %rdi,%r8
  75acab:	0f 85 ef fd ff ff    	jne    75aaa0 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0xe0>
  75acb1:	c5 fc 10 4c 24 20    	vmovups 0x20(%rsp),%ymm1
  75acb7:	c5 ac 59 d1          	vmulps %ymm1,%ymm10,%ymm2
  75acbb:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  75acbf:	c5 ac 59 c0          	vmulps %ymm0,%ymm10,%ymm0
  75acc3:	c4 62 75 aa d2       	vfmsub213ps %ymm2,%ymm1,%ymm10
  75acc8:	c5 f4 59 cf          	vmulps %ymm7,%ymm1,%ymm1
  75accc:	c5 fc 58 c1          	vaddps %ymm1,%ymm0,%ymm0
  75acd0:	c5 ac 58 c0          	vaddps %ymm0,%ymm10,%ymm0
  75acd4:	c5 ec 58 c8          	vaddps %ymm0,%ymm2,%ymm1
  75acd8:	c5 f4 5c da          	vsubps %ymm2,%ymm1,%ymm3
  75acdc:	c5 f4 5c f3          	vsubps %ymm3,%ymm1,%ymm6
  75ace0:	c5 ec 5c d6          	vsubps %ymm6,%ymm2,%ymm2
  75ace4:	c5 fc 5c c3          	vsubps %ymm3,%ymm0,%ymm0
  75ace8:	c5 fc 58 c2          	vaddps %ymm2,%ymm0,%ymm0
  75acec:	c5 d4 58 c0          	vaddps %ymm0,%ymm5,%ymm0
  75acf0:	c5 dc 58 d1          	vaddps %ymm1,%ymm4,%ymm2
  75acf4:	c5 ec 5c dc          	vsubps %ymm4,%ymm2,%ymm3
  75acf8:	c5 ec 5c eb          	vsubps %ymm3,%ymm2,%ymm5
  75acfc:	c5 dc 5c e5          	vsubps %ymm5,%ymm4,%ymm4
  75ad00:	c5 f4 5c cb          	vsubps %ymm3,%ymm1,%ymm1
  75ad04:	c5 f4 58 cc          	vaddps %ymm4,%ymm1,%ymm1
  75ad08:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  75ad0c:	c5 ec 58 c8          	vaddps %ymm0,%ymm2,%ymm1
  75ad10:	c5 f4 5c da          	vsubps %ymm2,%ymm1,%ymm3
  75ad14:	c5 f4 5c e3          	vsubps %ymm3,%ymm1,%ymm4
  75ad18:	c5 ec 5c d4          	vsubps %ymm4,%ymm2,%ymm2
  75ad1c:	c5 fc 5c c3          	vsubps %ymm3,%ymm0,%ymm0
  75ad20:	c5 fc 58 c2          	vaddps %ymm2,%ymm0,%ymm0
  75ad24:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  75ad28:	48 8b 0d 99 4e 12 00 	mov    0x124e99(%rip),%rcx        # 87fbc8 <_DYNAMIC+0x550>
  75ad2f:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  75ad36:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 75ad3f <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x37f>
  75ad3d:	ff e2                	jmp    *%rdx
  75ad3f:	e9 9b 03 00 00       	jmp    75b0df <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x71f>
  75ad44:	31 c9                	xor    %ecx,%ecx
  75ad46:	89 c8                	mov    %ecx,%eax
  75ad48:	c3                   	ret
  75ad49:	80 bf f1 00 00 00 00 	cmpb   $0x0,0xf1(%rdi)
  75ad50:	0f 84 a1 01 00 00    	je     75aef7 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x537>
  75ad56:	80 bf f2 00 00 00 00 	cmpb   $0x0,0xf2(%rdi)
  75ad5d:	0f 84 b6 01 00 00    	je     75af19 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x559>
  75ad63:	31 c9                	xor    %ecx,%ecx
  75ad65:	48 83 7f 58 08       	cmpq   $0x8,0x58(%rdi)
  75ad6a:	0f 85 f7 03 00 00    	jne    75b167 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7a7>
  75ad70:	48 83 7f 78 08       	cmpq   $0x8,0x78(%rdi)
  75ad75:	0f 85 ec 03 00 00    	jne    75b167 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7a7>
  75ad7b:	48 83 bf 98 00 00 00 	cmpq   $0x8,0x98(%rdi)
  75ad82:	08
  75ad83:	0f 85 de 03 00 00    	jne    75b167 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7a7>
  75ad89:	4c 39 87 a8 00 00 00 	cmp    %r8,0xa8(%rdi)
  75ad90:	0f 85 74 04 00 00    	jne    75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75ad96:	4c 39 87 b8 00 00 00 	cmp    %r8,0xb8(%rdi)
  75ad9d:	0f 85 67 04 00 00    	jne    75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75ada3:	4c 39 87 c8 00 00 00 	cmp    %r8,0xc8(%rdi)
  75adaa:	0f 85 5a 04 00 00    	jne    75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75adb0:	4c 39 87 d8 00 00 00 	cmp    %r8,0xd8(%rdi)
  75adb7:	0f 85 4d 04 00 00    	jne    75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75adbd:	48 8b 4f 70          	mov    0x70(%rdi),%rcx
  75adc1:	c5 fc 10 01          	vmovups (%rcx),%ymm0
  75adc5:	c5 fc 11 04 24       	vmovups %ymm0,(%rsp)
  75adca:	4d 85 c0             	test   %r8,%r8
  75adcd:	0f 84 9f 03 00 00    	je     75b172 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7b2>
  75add3:	4c 8b 5f 50          	mov    0x50(%rdi),%r11
  75add7:	48 8b 9f 90 00 00 00 	mov    0x90(%rdi),%rbx
  75adde:	48 8b 8f a0 00 00 00 	mov    0xa0(%rdi),%rcx
  75ade5:	4c 8b 8f b0 00 00 00 	mov    0xb0(%rdi),%r9
  75adec:	4c 8b 97 c0 00 00 00 	mov    0xc0(%rdi),%r10
  75adf3:	48 8b bf d0 00 00 00 	mov    0xd0(%rdi),%rdi
  75adfa:	c4 c1 7c 10 03       	vmovups (%r11),%ymm0
  75adff:	c5 7c 10 13          	vmovups (%rbx),%ymm10
  75ae03:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  75ae07:	45 31 db             	xor    %r11d,%r11d
  75ae0a:	c5 c0 57 ff          	vxorps %xmm7,%xmm7,%xmm7
  75ae0e:	c4 41 38 57 c0       	vxorps %xmm8,%xmm8,%xmm8
  75ae13:	c5 c8 57 f6          	vxorps %xmm6,%xmm6,%xmm6
  75ae17:	c5 7c 29 fa          	vmovaps %ymm15,%ymm2
  75ae1b:	c5 7c 28 cf          	vmovaps %ymm7,%ymm9
  75ae1f:	c5 7c 29 d7          	vmovaps %ymm10,%ymm7
  75ae23:	c4 22 7d 18 14 9a    	vbroadcastss (%rdx,%r11,4),%ymm10
  75ae29:	c4 22 7d 18 1c 9e    	vbroadcastss (%rsi,%r11,4),%ymm11
  75ae2f:	c4 c2 65 b8 e2       	vfmadd231ps %ymm10,%ymm3,%ymm4
  75ae34:	c4 c2 45 b8 e2       	vfmadd231ps %ymm10,%ymm7,%ymm4
  75ae39:	c4 c2 65 b8 f3       	vfmadd231ps %ymm11,%ymm3,%ymm6
  75ae3e:	c4 c2 45 b8 f3       	vfmadd231ps %ymm11,%ymm7,%ymm6
  75ae43:	c4 22 7d 18 14 99    	vbroadcastss (%rcx,%r11,4),%ymm10
  75ae49:	c4 02 7d 18 1c 99    	vbroadcastss (%r9,%r11,4),%ymm11
  75ae4f:	c5 2c 59 e2          	vmulps %ymm2,%ymm10,%ymm12
  75ae53:	c5 7c 28 ea          	vmovaps %ymm2,%ymm13
  75ae57:	c4 42 2d aa ec       	vfmsub213ps %ymm12,%ymm10,%ymm13
  75ae5c:	c4 41 1c 58 f3       	vaddps %ymm11,%ymm12,%ymm14
  75ae61:	c4 41 0c 5c fc       	vsubps %ymm12,%ymm14,%ymm15
  75ae66:	c4 c1 0c 5c ef       	vsubps %ymm15,%ymm14,%ymm5
  75ae6b:	c5 9c 5c ed          	vsubps %ymm5,%ymm12,%ymm5
  75ae6f:	c4 02 7d 18 24 9a    	vbroadcastss (%r10,%r11,4),%ymm12
  75ae75:	c4 41 24 5c df       	vsubps %ymm15,%ymm11,%ymm11
  75ae7a:	c4 22 7d 18 3c 9f    	vbroadcastss (%rdi,%r11,4),%ymm15
  75ae80:	c5 a4 58 ed          	vaddps %ymm5,%ymm11,%ymm5
  75ae84:	c5 94 58 ed          	vaddps %ymm5,%ymm13,%ymm5
  75ae88:	c5 1c 59 d8          	vmulps %ymm0,%ymm12,%ymm11
  75ae8c:	c4 42 6d b8 dc       	vfmadd231ps %ymm12,%ymm2,%ymm11
  75ae91:	c4 42 7d b8 da       	vfmadd231ps %ymm10,%ymm0,%ymm11
  75ae96:	c4 41 04 58 d3       	vaddps %ymm11,%ymm15,%ymm10
  75ae9b:	c5 ac 58 ed          	vaddps %ymm5,%ymm10,%ymm5
  75ae9f:	c5 0c 59 d3          	vmulps %ymm3,%ymm14,%ymm10
  75aea3:	c5 7c 28 db          	vmovaps %ymm3,%ymm11
  75aea7:	c4 42 0d aa da       	vfmsub213ps %ymm10,%ymm14,%ymm11
  75aeac:	c4 41 3c 58 e2       	vaddps %ymm10,%ymm8,%ymm12
  75aeb1:	c4 41 1c 5c ea       	vsubps %ymm10,%ymm12,%ymm13
  75aeb6:	c4 41 1c 5c fd       	vsubps %ymm13,%ymm12,%ymm15
  75aebb:	c4 41 2c 5c d7       	vsubps %ymm15,%ymm10,%ymm10
  75aec0:	c4 41 3c 5c c5       	vsubps %ymm13,%ymm8,%ymm8
  75aec5:	c4 41 3c 58 c2       	vaddps %ymm10,%ymm8,%ymm8
  75aeca:	c4 41 24 58 c0       	vaddps %ymm8,%ymm11,%ymm8
  75aecf:	c4 42 45 b8 c6       	vfmadd231ps %ymm14,%ymm7,%ymm8
  75aed4:	c4 62 65 b8 c5       	vfmadd231ps %ymm5,%ymm3,%ymm8
  75aed9:	c4 41 34 58 d0       	vaddps %ymm8,%ymm9,%ymm10
  75aede:	49 ff c3             	inc    %r11
  75aee1:	c5 7c 28 c3          	vmovaps %ymm3,%ymm8
  75aee5:	c5 7c 29 e3          	vmovaps %ymm12,%ymm3
  75aee9:	4d 39 d8             	cmp    %r11,%r8
  75aeec:	0f 85 29 ff ff ff    	jne    75ae1b <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x45b>
  75aef2:	e9 83 02 00 00       	jmp    75b17a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7ba>
  75aef7:	4d 85 c0             	test   %r8,%r8
  75aefa:	0f 84 ee 00 00 00    	je     75afee <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x62e>
  75af00:	49 83 f8 01          	cmp    $0x1,%r8
  75af04:	0f 85 f1 00 00 00    	jne    75affb <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x63b>
  75af0a:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  75af0e:	31 ff                	xor    %edi,%edi
  75af10:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  75af14:	e9 66 01 00 00       	jmp    75b07f <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x6bf>
  75af19:	4d 85 c0             	test   %r8,%r8
  75af1c:	0f 84 99 01 00 00    	je     75b0bb <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x6fb>
  75af22:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  75af26:	31 ff                	xor    %edi,%edi
  75af28:	c4 41 38 57 c0       	vxorps %xmm8,%xmm8,%xmm8
  75af2d:	c5 d0 57 ed          	vxorps %xmm5,%xmm5,%xmm5
  75af31:	c5 c8 57 f6          	vxorps %xmm6,%xmm6,%xmm6
  75af35:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  75af39:	c5 fc 28 fd          	vmovaps %ymm5,%ymm7
  75af3d:	c5 7c 29 c5          	vmovaps %ymm8,%ymm5
  75af41:	c4 62 7d 18 04 b9    	vbroadcastss (%rcx,%rdi,4),%ymm8
  75af47:	c4 62 7d 18 0c be    	vbroadcastss (%rsi,%rdi,4),%ymm9
  75af4d:	c4 62 7d 18 14 ba    	vbroadcastss (%rdx,%rdi,4),%ymm10
  75af53:	c4 42 7d 18 1c b9    	vbroadcastss (%r9,%rdi,4),%ymm11
  75af59:	48 ff c7             	inc    %rdi
  75af5c:	c4 c2 65 b8 c1       	vfmadd231ps %ymm9,%ymm3,%ymm0
  75af61:	c4 c2 55 b8 c1       	vfmadd231ps %ymm9,%ymm5,%ymm0
  75af66:	c4 c2 65 b8 e2       	vfmadd231ps %ymm10,%ymm3,%ymm4
  75af6b:	c4 c2 55 b8 e2       	vfmadd231ps %ymm10,%ymm5,%ymm4
  75af70:	c4 41 04 59 c8       	vmulps %ymm8,%ymm15,%ymm9
  75af75:	c4 42 05 aa c1       	vfmsub213ps %ymm9,%ymm15,%ymm8
  75af7a:	c4 41 34 58 d3       	vaddps %ymm11,%ymm9,%ymm10
  75af7f:	c4 41 2c 5c e1       	vsubps %ymm9,%ymm10,%ymm12
  75af84:	c4 41 2c 5c ec       	vsubps %ymm12,%ymm10,%ymm13
  75af89:	c4 41 34 5c cd       	vsubps %ymm13,%ymm9,%ymm9
  75af8e:	c4 41 24 5c dc       	vsubps %ymm12,%ymm11,%ymm11
  75af93:	c4 41 24 58 c9       	vaddps %ymm9,%ymm11,%ymm9
  75af98:	c4 41 3c 58 c1       	vaddps %ymm9,%ymm8,%ymm8
  75af9d:	c5 2c 59 cb          	vmulps %ymm3,%ymm10,%ymm9
  75afa1:	c5 7c 28 db          	vmovaps %ymm3,%ymm11
  75afa5:	c4 42 2d aa d9       	vfmsub213ps %ymm9,%ymm10,%ymm11
  75afaa:	c5 34 58 e6          	vaddps %ymm6,%ymm9,%ymm12
  75afae:	c4 41 1c 5c e9       	vsubps %ymm9,%ymm12,%ymm13
  75afb3:	c4 41 1c 5c f5       	vsubps %ymm13,%ymm12,%ymm14
  75afb8:	c4 41 34 5c ce       	vsubps %ymm14,%ymm9,%ymm9
  75afbd:	c4 c1 4c 5c f5       	vsubps %ymm13,%ymm6,%ymm6
  75afc2:	c5 b4 58 f6          	vaddps %ymm6,%ymm9,%ymm6
  75afc6:	c5 a4 58 f6          	vaddps %ymm6,%ymm11,%ymm6
  75afca:	c4 c2 55 b8 f2       	vfmadd231ps %ymm10,%ymm5,%ymm6
  75afcf:	c4 c2 65 b8 f0       	vfmadd231ps %ymm8,%ymm3,%ymm6
  75afd4:	c5 44 58 c6          	vaddps %ymm6,%ymm7,%ymm8
  75afd8:	c5 fc 28 f3          	vmovaps %ymm3,%ymm6
  75afdc:	c5 7c 29 e3          	vmovaps %ymm12,%ymm3
  75afe0:	49 39 f8             	cmp    %rdi,%r8
  75afe3:	0f 85 50 ff ff ff    	jne    75af39 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x579>
  75afe9:	e9 d5 00 00 00       	jmp    75b0c3 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x703>
  75afee:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  75aff2:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  75aff6:	e9 a2 00 00 00       	jmp    75b09d <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x6dd>
  75affb:	4d 89 c2             	mov    %r8,%r10
  75affe:	49 83 e2 fe          	and    $0xfffffffffffffffe,%r10
  75b002:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  75b006:	31 ff                	xor    %edi,%edi
  75b008:	c5 d0 57 ed          	vxorps %xmm5,%xmm5,%xmm5
  75b00c:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  75b010:	c5 fc 28 f3          	vmovaps %ymm3,%ymm6
  75b014:	c4 e2 7d 18 1c b9    	vbroadcastss (%rcx,%rdi,4),%ymm3
  75b01a:	c4 e2 7d 18 3c be    	vbroadcastss (%rsi,%rdi,4),%ymm7
  75b020:	c4 e2 4d a8 f8       	vfmadd213ps %ymm0,%ymm6,%ymm7
  75b025:	c4 62 7d 18 04 ba    	vbroadcastss (%rdx,%rdi,4),%ymm8
  75b02b:	c4 62 4d a8 c4       	vfmadd213ps %ymm4,%ymm6,%ymm8
  75b030:	c4 c2 7d 18 04 b9    	vbroadcastss (%r9,%rdi,4),%ymm0
  75b036:	c4 e2 05 b8 c3       	vfmadd231ps %ymm3,%ymm15,%ymm0
  75b03b:	c4 e2 4d b8 e8       	vfmadd231ps %ymm0,%ymm6,%ymm5
  75b040:	c4 62 7d 18 4c b9 04 	vbroadcastss 0x4(%rcx,%rdi,4),%ymm9
  75b047:	c4 e2 7d 18 44 be 04 	vbroadcastss 0x4(%rsi,%rdi,4),%ymm0
  75b04e:	c4 e2 55 a8 c7       	vfmadd213ps %ymm7,%ymm5,%ymm0
  75b053:	c4 e2 7d 18 64 ba 04 	vbroadcastss 0x4(%rdx,%rdi,4),%ymm4
  75b05a:	c4 c2 55 a8 e0       	vfmadd213ps %ymm8,%ymm5,%ymm4
  75b05f:	c4 c2 7d 18 5c b9 04 	vbroadcastss 0x4(%r9,%rdi,4),%ymm3
  75b066:	48 83 c7 02          	add    $0x2,%rdi
  75b06a:	c4 c2 05 b8 d9       	vfmadd231ps %ymm9,%ymm15,%ymm3
  75b06f:	c4 e2 55 a8 de       	vfmadd213ps %ymm6,%ymm5,%ymm3
  75b074:	49 39 fa             	cmp    %rdi,%r10
  75b077:	75 97                	jne    75b010 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x650>
  75b079:	41 f6 c0 01          	test   $0x1,%r8b
  75b07d:	74 1e                	je     75b09d <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x6dd>
  75b07f:	c5 fc 28 d0          	vmovaps %ymm0,%ymm2
  75b083:	c5 fc 28 ec          	vmovaps %ymm4,%ymm5
  75b087:	c4 e2 7d 18 04 be    	vbroadcastss (%rsi,%rdi,4),%ymm0
  75b08d:	c4 e2 65 a8 c2       	vfmadd213ps %ymm2,%ymm3,%ymm0
  75b092:	c4 e2 7d 18 24 ba    	vbroadcastss (%rdx,%rdi,4),%ymm4
  75b098:	c4 e2 65 a8 e5       	vfmadd213ps %ymm5,%ymm3,%ymm4
  75b09d:	c4 e2 75 a8 c4       	vfmadd213ps %ymm4,%ymm1,%ymm0
  75b0a2:	48 8b 0d 1f 4b 12 00 	mov    0x124b1f(%rip),%rcx        # 87fbc8 <_DYNAMIC+0x550>
  75b0a9:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  75b0b0:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 75b0b9 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x6f9>
  75b0b7:	ff e2                	jmp    *%rdx
  75b0b9:	eb 24                	jmp    75b0df <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x71f>
  75b0bb:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  75b0bf:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  75b0c3:	c4 e2 75 a8 c4       	vfmadd213ps %ymm4,%ymm1,%ymm0
  75b0c8:	48 8b 0d f9 4a 12 00 	mov    0x124af9(%rip),%rcx        # 87fbc8 <_DYNAMIC+0x550>
  75b0cf:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  75b0d6:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 75b0df <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x71f>
  75b0dd:	ff e2                	jmp    *%rdx
  75b0df:	8b 08                	mov    (%rax),%ecx
  75b0e1:	01 c9                	add    %ecx,%ecx
  75b0e3:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b0e9:	0f 87 1b 01 00 00    	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b0ef:	8b 48 04             	mov    0x4(%rax),%ecx
  75b0f2:	01 c9                	add    %ecx,%ecx
  75b0f4:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b0fa:	0f 87 0a 01 00 00    	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b100:	8b 48 08             	mov    0x8(%rax),%ecx
  75b103:	01 c9                	add    %ecx,%ecx
  75b105:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b10b:	0f 87 f9 00 00 00    	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b111:	8b 48 0c             	mov    0xc(%rax),%ecx
  75b114:	01 c9                	add    %ecx,%ecx
  75b116:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b11c:	0f 87 e8 00 00 00    	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b122:	8b 48 10             	mov    0x10(%rax),%ecx
  75b125:	01 c9                	add    %ecx,%ecx
  75b127:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b12d:	0f 87 d7 00 00 00    	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b133:	8b 48 14             	mov    0x14(%rax),%ecx
  75b136:	01 c9                	add    %ecx,%ecx
  75b138:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b13e:	0f 87 c6 00 00 00    	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b144:	8b 48 18             	mov    0x18(%rax),%ecx
  75b147:	01 c9                	add    %ecx,%ecx
  75b149:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b14f:	0f 87 b5 00 00 00    	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b155:	8b 40 1c             	mov    0x1c(%rax),%eax
  75b158:	01 c0                	add    %eax,%eax
  75b15a:	3d ff ff ff fe       	cmp    $0xfeffffff,%eax
  75b15f:	0f 87 a5 00 00 00    	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b165:	b1 01                	mov    $0x1,%cl
  75b167:	48 83 c4 40          	add    $0x40,%rsp
  75b16b:	5b                   	pop    %rbx
  75b16c:	89 c8                	mov    %ecx,%eax
  75b16e:	c5 f8 77             	vzeroupper
  75b171:	c3                   	ret
  75b172:	c5 c8 57 f6          	vxorps %xmm6,%xmm6,%xmm6
  75b176:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  75b17a:	c4 e2 4d a8 cc       	vfmadd213ps %ymm4,%ymm6,%ymm1
  75b17f:	c5 fc 10 04 24       	vmovups (%rsp),%ymm0
  75b184:	c4 e2 4d a8 c1       	vfmadd213ps %ymm1,%ymm6,%ymm0
  75b189:	48 8b 0d 38 4a 12 00 	mov    0x124a38(%rip),%rcx        # 87fbc8 <_DYNAMIC+0x550>
  75b190:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  75b197:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 75b1a0 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7e0>
  75b19e:	ff e2                	jmp    *%rdx
  75b1a0:	8b 08                	mov    (%rax),%ecx
  75b1a2:	01 c9                	add    %ecx,%ecx
  75b1a4:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b1aa:	77 5e                	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b1ac:	8b 48 04             	mov    0x4(%rax),%ecx
  75b1af:	01 c9                	add    %ecx,%ecx
  75b1b1:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b1b7:	77 51                	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b1b9:	8b 48 08             	mov    0x8(%rax),%ecx
  75b1bc:	01 c9                	add    %ecx,%ecx
  75b1be:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b1c4:	77 44                	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b1c6:	8b 48 0c             	mov    0xc(%rax),%ecx
  75b1c9:	01 c9                	add    %ecx,%ecx
  75b1cb:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b1d1:	77 37                	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b1d3:	8b 48 10             	mov    0x10(%rax),%ecx
  75b1d6:	01 c9                	add    %ecx,%ecx
  75b1d8:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b1de:	77 2a                	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b1e0:	8b 48 14             	mov    0x14(%rax),%ecx
  75b1e3:	01 c9                	add    %ecx,%ecx
  75b1e5:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b1eb:	77 1d                	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b1ed:	8b 48 18             	mov    0x18(%rax),%ecx
  75b1f0:	01 c9                	add    %ecx,%ecx
  75b1f2:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  75b1f8:	77 10                	ja     75b20a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x84a>
  75b1fa:	8b 40 1c             	mov    0x1c(%rax),%eax
  75b1fd:	01 c0                	add    %eax,%eax
  75b1ff:	3d ff ff ff fe       	cmp    $0xfeffffff,%eax
  75b204:	0f 86 5b ff ff ff    	jbe    75b165 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7a5>
  75b20a:	31 c9                	xor    %ecx,%ecx
  75b20c:	48 83 c4 40          	add    $0x40,%rsp
  75b210:	5b                   	pop    %rbx
  75b211:	89 c8                	mov    %ecx,%eax
  75b213:	c5 f8 77             	vzeroupper
  75b216:	c3                   	ret
  75b217:	cc                   	int3
  75b218:	cc                   	int3
  75b219:	cc                   	int3
  75b21a:	cc                   	int3
  75b21b:	cc                   	int3
  75b21c:	cc                   	int3
  75b21d:	cc                   	int3
  75b21e:	cc                   	int3
  75b21f:	cc                   	int3
