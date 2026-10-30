
target/release/deps/rustsht-7d36680db74a2dd3:     file format elf64-x86-64


Disassembly of section .text:

00000000004aea00 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>>:
  4aea00:	31 c9                	xor    %ecx,%ecx
  4aea02:	48 83 7f 48 08       	cmpq   $0x8,0x48(%rdi)
  4aea07:	0f 85 7f 03 00 00    	jne    4aed8c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x38c>
  4aea0d:	48 83 7f 68 08       	cmpq   $0x8,0x68(%rdi)
  4aea12:	0f 85 74 03 00 00    	jne    4aed8c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x38c>
  4aea18:	48 83 bf 88 00 00 00 	cmpq   $0x8,0x88(%rdi)
  4aea1f:	08
  4aea20:	0f 85 66 03 00 00    	jne    4aed8c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x38c>
  4aea26:	4c 8b 47 08          	mov    0x8(%rdi),%r8
  4aea2a:	4c 3b 47 18          	cmp    0x18(%rdi),%r8
  4aea2e:	0f 85 56 03 00 00    	jne    4aed8a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x38a>
  4aea34:	4c 3b 47 28          	cmp    0x28(%rdi),%r8
  4aea38:	0f 85 4c 03 00 00    	jne    4aed8a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x38a>
  4aea3e:	4c 3b 47 38          	cmp    0x38(%rdi),%r8
  4aea42:	0f 85 42 03 00 00    	jne    4aed8a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x38a>
  4aea48:	48 83 bf 08 01 00 00 	cmpq   $0x8,0x108(%rdi)
  4aea4f:	08
  4aea50:	0f 85 34 03 00 00    	jne    4aed8a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x38a>
  4aea56:	53                   	push   %rbx
  4aea57:	48 83 ec 40          	sub    $0x40,%rsp
  4aea5b:	4c 8b 57 40          	mov    0x40(%rdi),%r10
  4aea5f:	4c 8b 5f 60          	mov    0x60(%rdi),%r11
  4aea63:	48 8b 9f 80 00 00 00 	mov    0x80(%rdi),%rbx
  4aea6a:	48 8b 0f             	mov    (%rdi),%rcx
  4aea6d:	4c 8b 4f 10          	mov    0x10(%rdi),%r9
  4aea71:	48 8b 57 20          	mov    0x20(%rdi),%rdx
  4aea75:	48 8b 77 30          	mov    0x30(%rdi),%rsi
  4aea79:	48 8b 87 00 01 00 00 	mov    0x100(%rdi),%rax
  4aea80:	c4 41 7c 10 3a       	vmovups (%r10),%ymm15
  4aea85:	c4 c1 7c 10 03       	vmovups (%r11),%ymm0
  4aea8a:	c5 fc 10 13          	vmovups (%rbx),%ymm2
  4aea8e:	80 bf 10 01 00 00 01 	cmpb   $0x1,0x110(%rdi)
  4aea95:	0f 85 f4 02 00 00    	jne    4aed8f <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x38f>
  4aea9b:	c5 fc 11 44 24 20    	vmovups %ymm0,0x20(%rsp)
  4aeaa1:	c4 41 30 57 c9       	vxorps %xmm9,%xmm9,%xmm9
  4aeaa6:	c4 41 28 57 d2       	vxorps %xmm10,%xmm10,%xmm10
  4aeaab:	c5 e0 57 db          	vxorps %xmm3,%xmm3,%xmm3
  4aeaaf:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  4aeab3:	4d 85 c0             	test   %r8,%r8
  4aeab6:	0f 84 2b 02 00 00    	je     4aece7 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x2e7>
  4aeabc:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  4aeac0:	c5 84 59 c0          	vmulps %ymm0,%ymm15,%ymm0
  4aeac4:	c5 fc 11 04 24       	vmovups %ymm0,(%rsp)
  4aeac9:	31 ff                	xor    %edi,%edi
  4aeacb:	c4 41 18 57 e4       	vxorps %xmm12,%xmm12,%xmm12
  4aead0:	c4 41 10 57 ed       	vxorps %xmm13,%xmm13,%xmm13
  4aead5:	c4 41 20 57 db       	vxorps %xmm11,%xmm11,%xmm11
  4aeada:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  4aeae0:	c4 62 7d 18 34 be    	vbroadcastss (%rsi,%rdi,4),%ymm14
  4aeae6:	c5 7c 29 fd          	vmovaps %ymm15,%ymm5
  4aeaea:	c5 0c 59 fa          	vmulps %ymm2,%ymm14,%ymm15
  4aeaee:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  4aeaf2:	c5 ec 59 c8          	vmulps %ymm0,%ymm2,%ymm1
  4aeaf6:	c4 41 24 59 c6       	vmulps %ymm14,%ymm11,%ymm8
  4aeafb:	c4 42 6d aa f7       	vfmsub213ps %ymm15,%ymm2,%ymm14
  4aeb00:	c5 3c 58 c1          	vaddps %ymm1,%ymm8,%ymm8
  4aeb04:	c4 41 0c 58 c0       	vaddps %ymm8,%ymm14,%ymm8
  4aeb09:	c4 41 04 58 f0       	vaddps %ymm8,%ymm15,%ymm14
  4aeb0e:	c4 c1 2c 58 c6       	vaddps %ymm14,%ymm10,%ymm0
  4aeb13:	c4 c1 7c 5c f2       	vsubps %ymm10,%ymm0,%ymm6
  4aeb18:	c5 fc 5c fe          	vsubps %ymm6,%ymm0,%ymm7
  4aeb1c:	c5 ac 5c ff          	vsubps %ymm7,%ymm10,%ymm7
  4aeb20:	c5 8c 5c f6          	vsubps %ymm6,%ymm14,%ymm6
  4aeb24:	c5 cc 58 f7          	vaddps %ymm7,%ymm6,%ymm6
  4aeb28:	c4 c1 0c 5c ff       	vsubps %ymm15,%ymm14,%ymm7
  4aeb2d:	c5 0c 5c d7          	vsubps %ymm7,%ymm14,%ymm10
  4aeb31:	c4 41 04 5c d2       	vsubps %ymm10,%ymm15,%ymm10
  4aeb36:	c5 7c 28 fd          	vmovaps %ymm5,%ymm15
  4aeb3a:	c5 bc 5c ff          	vsubps %ymm7,%ymm8,%ymm7
  4aeb3e:	c5 ac 58 ff          	vaddps %ymm7,%ymm10,%ymm7
  4aeb42:	c5 b4 58 ff          	vaddps %ymm7,%ymm9,%ymm7
  4aeb46:	c5 cc 58 f7          	vaddps %ymm7,%ymm6,%ymm6
  4aeb4a:	c5 7c 58 d6          	vaddps %ymm6,%ymm0,%ymm10
  4aeb4e:	c5 ac 5c f8          	vsubps %ymm0,%ymm10,%ymm7
  4aeb52:	c5 2c 5c c7          	vsubps %ymm7,%ymm10,%ymm8
  4aeb56:	c4 c1 7c 5c c0       	vsubps %ymm8,%ymm0,%ymm0
  4aeb5b:	c5 cc 5c f7          	vsubps %ymm7,%ymm6,%ymm6
  4aeb5f:	c5 4c 58 c8          	vaddps %ymm0,%ymm6,%ymm9
  4aeb63:	c4 e2 7d 18 04 ba    	vbroadcastss (%rdx,%rdi,4),%ymm0
  4aeb69:	c5 ec 59 f0          	vmulps %ymm0,%ymm2,%ymm6
  4aeb6d:	c5 a4 59 f8          	vmulps %ymm0,%ymm11,%ymm7
  4aeb71:	c4 e2 6d aa c6       	vfmsub213ps %ymm6,%ymm2,%ymm0
  4aeb76:	c5 f4 58 cf          	vaddps %ymm7,%ymm1,%ymm1
  4aeb7a:	c5 fc 58 c1          	vaddps %ymm1,%ymm0,%ymm0
  4aeb7e:	c5 cc 58 c8          	vaddps %ymm0,%ymm6,%ymm1
  4aeb82:	c5 dc 58 f9          	vaddps %ymm1,%ymm4,%ymm7
  4aeb86:	c5 44 5c c4          	vsubps %ymm4,%ymm7,%ymm8
  4aeb8a:	c4 41 44 5c f0       	vsubps %ymm8,%ymm7,%ymm14
  4aeb8f:	c4 c1 5c 5c e6       	vsubps %ymm14,%ymm4,%ymm4
  4aeb94:	c4 62 7d 18 34 b9    	vbroadcastss (%rcx,%rdi,4),%ymm14
  4aeb9a:	c4 41 74 5c c0       	vsubps %ymm8,%ymm1,%ymm8
  4aeb9f:	c5 bc 58 e4          	vaddps %ymm4,%ymm8,%ymm4
  4aeba3:	c5 74 5c c6          	vsubps %ymm6,%ymm1,%ymm8
  4aeba7:	c4 c1 74 5c c8       	vsubps %ymm8,%ymm1,%ymm1
  4aebac:	c5 cc 5c c9          	vsubps %ymm1,%ymm6,%ymm1
  4aebb0:	c4 c1 7c 5c c0       	vsubps %ymm8,%ymm0,%ymm0
  4aebb5:	c5 fc 58 c1          	vaddps %ymm1,%ymm0,%ymm0
  4aebb9:	c5 e4 58 c0          	vaddps %ymm0,%ymm3,%ymm0
  4aebbd:	c5 dc 58 c0          	vaddps %ymm0,%ymm4,%ymm0
  4aebc1:	c5 c4 58 e0          	vaddps %ymm0,%ymm7,%ymm4
  4aebc5:	c5 dc 5c cf          	vsubps %ymm7,%ymm4,%ymm1
  4aebc9:	c5 dc 5c e9          	vsubps %ymm1,%ymm4,%ymm5
  4aebcd:	c5 c4 5c ed          	vsubps %ymm5,%ymm7,%ymm5
  4aebd1:	c5 fc 5c c1          	vsubps %ymm1,%ymm0,%ymm0
  4aebd5:	c5 fc 58 dd          	vaddps %ymm5,%ymm0,%ymm3
  4aebd9:	c4 c2 7d 18 04 b9    	vbroadcastss (%r9,%rdi,4),%ymm0
  4aebdf:	c4 c1 04 59 ce       	vmulps %ymm14,%ymm15,%ymm1
  4aebe4:	c5 d0 57 ed          	vxorps %xmm5,%xmm5,%xmm5
  4aebe8:	c5 8a 59 f5          	vmulss %xmm5,%xmm14,%xmm6
  4aebec:	c4 62 05 aa f1       	vfmsub213ps %ymm1,%ymm15,%ymm14
  4aebf1:	c4 e2 7d 18 f6       	vbroadcastss %xmm6,%ymm6
  4aebf6:	c5 cc 58 34 24       	vaddps (%rsp),%ymm6,%ymm6
  4aebfb:	c5 8c 58 f6          	vaddps %ymm6,%ymm14,%ymm6
  4aebff:	c5 f4 58 fe          	vaddps %ymm6,%ymm1,%ymm7
  4aec03:	c5 44 5c c1          	vsubps %ymm1,%ymm7,%ymm8
  4aec07:	c4 41 44 5c f0       	vsubps %ymm8,%ymm7,%ymm14
  4aec0c:	c4 c1 74 5c ce       	vsubps %ymm14,%ymm1,%ymm1
  4aec11:	c4 c1 4c 5c f0       	vsubps %ymm8,%ymm6,%ymm6
  4aec16:	c5 cc 58 c9          	vaddps %ymm1,%ymm6,%ymm1
  4aec1a:	c5 fc 58 f7          	vaddps %ymm7,%ymm0,%ymm6
  4aec1e:	c5 4c 5c c7          	vsubps %ymm7,%ymm6,%ymm8
  4aec22:	c4 41 4c 5c f0       	vsubps %ymm8,%ymm6,%ymm14
  4aec27:	c4 c1 44 5c fe       	vsubps %ymm14,%ymm7,%ymm7
  4aec2c:	c4 c1 7c 5c c0       	vsubps %ymm8,%ymm0,%ymm0
  4aec31:	c5 fc 58 c7          	vaddps %ymm7,%ymm0,%ymm0
  4aec35:	c5 d0 57 ed          	vxorps %xmm5,%xmm5,%xmm5
  4aec39:	c5 f4 58 cd          	vaddps %ymm5,%ymm1,%ymm1
  4aec3d:	c5 fc 58 c1          	vaddps %ymm1,%ymm0,%ymm0
  4aec41:	c5 cc 58 c8          	vaddps %ymm0,%ymm6,%ymm1
  4aec45:	c5 f4 5c fe          	vsubps %ymm6,%ymm1,%ymm7
  4aec49:	c5 74 5c c7          	vsubps %ymm7,%ymm1,%ymm8
  4aec4d:	c4 c1 4c 5c f0       	vsubps %ymm8,%ymm6,%ymm6
  4aec52:	c5 fc 5c c7          	vsubps %ymm7,%ymm0,%ymm0
  4aec56:	c5 fc 58 c6          	vaddps %ymm6,%ymm0,%ymm0
  4aec5a:	c5 ec 59 f1          	vmulps %ymm1,%ymm2,%ymm6
  4aec5e:	c5 fc 28 fa          	vmovaps %ymm2,%ymm7
  4aec62:	c4 e2 75 aa fe       	vfmsub213ps %ymm6,%ymm1,%ymm7
  4aec67:	c5 a4 59 c9          	vmulps %ymm1,%ymm11,%ymm1
  4aec6b:	c5 ec 59 c0          	vmulps %ymm0,%ymm2,%ymm0
  4aec6f:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  4aec73:	c5 c4 58 c0          	vaddps %ymm0,%ymm7,%ymm0
  4aec77:	c5 cc 58 c8          	vaddps %ymm0,%ymm6,%ymm1
  4aec7b:	c5 f4 5c fe          	vsubps %ymm6,%ymm1,%ymm7
  4aec7f:	c5 74 5c c7          	vsubps %ymm7,%ymm1,%ymm8
  4aec83:	c4 c1 4c 5c f0       	vsubps %ymm8,%ymm6,%ymm6
  4aec88:	c5 fc 5c c7          	vsubps %ymm7,%ymm0,%ymm0
  4aec8c:	c5 fc 58 c6          	vaddps %ymm6,%ymm0,%ymm0
  4aec90:	c5 94 58 c0          	vaddps %ymm0,%ymm13,%ymm0
  4aec94:	c5 9c 58 f1          	vaddps %ymm1,%ymm12,%ymm6
  4aec98:	c5 cc 5c f9          	vsubps %ymm1,%ymm6,%ymm7
  4aec9c:	c5 4c 5c c7          	vsubps %ymm7,%ymm6,%ymm8
  4aeca0:	c4 c1 74 5c c8       	vsubps %ymm8,%ymm1,%ymm1
  4aeca5:	c5 9c 5c ff          	vsubps %ymm7,%ymm12,%ymm7
  4aeca9:	c5 c4 58 c9          	vaddps %ymm1,%ymm7,%ymm1
  4aecad:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  4aecb1:	c5 cc 58 c8          	vaddps %ymm0,%ymm6,%ymm1
  4aecb5:	c5 f4 5c fe          	vsubps %ymm6,%ymm1,%ymm7
  4aecb9:	c5 74 5c c7          	vsubps %ymm7,%ymm1,%ymm8
  4aecbd:	c4 c1 4c 5c f0       	vsubps %ymm8,%ymm6,%ymm6
  4aecc2:	c5 fc 5c c7          	vsubps %ymm7,%ymm0,%ymm0
  4aecc6:	c5 fc 58 c6          	vaddps %ymm6,%ymm0,%ymm0
  4aecca:	c5 7c 28 e2          	vmovaps %ymm2,%ymm12
  4aecce:	c4 41 7c 28 eb       	vmovaps %ymm11,%ymm13
  4aecd3:	48 ff c7             	inc    %rdi
  4aecd6:	c5 fc 28 d1          	vmovaps %ymm1,%ymm2
  4aecda:	c5 7c 28 d8          	vmovaps %ymm0,%ymm11
  4aecde:	49 39 f8             	cmp    %rdi,%r8
  4aece1:	0f 85 f9 fd ff ff    	jne    4aeae0 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0xe0>
  4aece7:	c5 fc 10 44 24 20    	vmovups 0x20(%rsp),%ymm0
  4aeced:	c5 ac 59 d0          	vmulps %ymm0,%ymm10,%ymm2
  4aecf1:	c5 f0 57 c9          	vxorps %xmm1,%xmm1,%xmm1
  4aecf5:	c5 ac 59 c9          	vmulps %ymm1,%ymm10,%ymm1
  4aecf9:	c4 62 7d aa d2       	vfmsub213ps %ymm2,%ymm0,%ymm10
  4aecfe:	c5 b4 59 c0          	vmulps %ymm0,%ymm9,%ymm0
  4aed02:	c5 fc 58 c1          	vaddps %ymm1,%ymm0,%ymm0
  4aed06:	c5 ac 58 c0          	vaddps %ymm0,%ymm10,%ymm0
  4aed0a:	c5 ec 58 c8          	vaddps %ymm0,%ymm2,%ymm1
  4aed0e:	c5 f4 5c ea          	vsubps %ymm2,%ymm1,%ymm5
  4aed12:	c5 f4 5c f5          	vsubps %ymm5,%ymm1,%ymm6
  4aed16:	c5 ec 5c d6          	vsubps %ymm6,%ymm2,%ymm2
  4aed1a:	c5 fc 5c c5          	vsubps %ymm5,%ymm0,%ymm0
  4aed1e:	c5 fc 58 c2          	vaddps %ymm2,%ymm0,%ymm0
  4aed22:	c5 e4 58 c0          	vaddps %ymm0,%ymm3,%ymm0
  4aed26:	c5 dc 58 d1          	vaddps %ymm1,%ymm4,%ymm2
  4aed2a:	c5 ec 5c dc          	vsubps %ymm4,%ymm2,%ymm3
  4aed2e:	c5 ec 5c eb          	vsubps %ymm3,%ymm2,%ymm5
  4aed32:	c5 dc 5c e5          	vsubps %ymm5,%ymm4,%ymm4
  4aed36:	c5 f4 5c cb          	vsubps %ymm3,%ymm1,%ymm1
  4aed3a:	c5 f4 58 cc          	vaddps %ymm4,%ymm1,%ymm1
  4aed3e:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  4aed42:	c5 ec 58 c8          	vaddps %ymm0,%ymm2,%ymm1
  4aed46:	c5 f4 5c da          	vsubps %ymm2,%ymm1,%ymm3
  4aed4a:	c5 f4 5c e3          	vsubps %ymm3,%ymm1,%ymm4
  4aed4e:	c5 ec 5c d4          	vsubps %ymm4,%ymm2,%ymm2
  4aed52:	c5 fc 5c c3          	vsubps %ymm3,%ymm0,%ymm0
  4aed56:	c5 fc 58 c2          	vaddps %ymm2,%ymm0,%ymm0
  4aed5a:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  4aed5e:	48 8b 0d bb 2a 3d 00 	mov    0x3d2abb(%rip),%rcx        # 881820 <_DYNAMIC+0x548>
  4aed65:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  4aed6c:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 4aed75 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x375>
  4aed73:	ff e2                	jmp    *%rdx
  4aed75:	8b 08                	mov    (%rax),%ecx
  4aed77:	01 c9                	add    %ecx,%ecx
  4aed79:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4aed7f:	0f 87 87 05 00 00    	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4aed85:	e9 bb 03 00 00       	jmp    4af145 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x745>
  4aed8a:	31 c9                	xor    %ecx,%ecx
  4aed8c:	89 c8                	mov    %ecx,%eax
  4aed8e:	c3                   	ret
  4aed8f:	80 bf 11 01 00 00 00 	cmpb   $0x0,0x111(%rdi)
  4aed96:	0f 84 07 01 00 00    	je     4aeea3 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x4a3>
  4aed9c:	80 bf 13 01 00 00 00 	cmpb   $0x0,0x113(%rdi)
  4aeda3:	0f 84 1c 01 00 00    	je     4aeec5 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x4c5>
  4aeda9:	4c 39 87 e8 00 00 00 	cmp    %r8,0xe8(%rdi)
  4aedb0:	0f 85 56 05 00 00    	jne    4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4aedb6:	31 c9                	xor    %ecx,%ecx
  4aedb8:	48 83 bf 98 00 00 00 	cmpq   $0x8,0x98(%rdi)
  4aedbf:	08
  4aedc0:	0f 85 f7 03 00 00    	jne    4af1bd <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7bd>
  4aedc6:	48 83 7f 78 08       	cmpq   $0x8,0x78(%rdi)
  4aedcb:	0f 85 ec 03 00 00    	jne    4af1bd <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7bd>
  4aedd1:	4c 39 87 f8 00 00 00 	cmp    %r8,0xf8(%rdi)
  4aedd8:	0f 85 df 03 00 00    	jne    4af1bd <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7bd>
  4aedde:	48 8b 4f 70          	mov    0x70(%rdi),%rcx
  4aede2:	4d 85 c0             	test   %r8,%r8
  4aede5:	0f 84 e8 04 00 00    	je     4af2d3 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x8d3>
  4aedeb:	4c 8b 97 90 00 00 00 	mov    0x90(%rdi),%r10
  4aedf2:	4c 8b 8f e0 00 00 00 	mov    0xe0(%rdi),%r9
  4aedf9:	48 8b bf f0 00 00 00 	mov    0xf0(%rdi),%rdi
  4aee00:	c4 c1 7c 10 3a       	vmovups (%r10),%ymm7
  4aee05:	c5 f0 57 c9          	vxorps %xmm1,%xmm1,%xmm1
  4aee09:	45 31 d2             	xor    %r10d,%r10d
  4aee0c:	c5 e0 57 db          	vxorps %xmm3,%xmm3,%xmm3
  4aee10:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  4aee14:	c5 d0 57 ed          	vxorps %xmm5,%xmm5,%xmm5
  4aee18:	c5 7c 28 c5          	vmovaps %ymm5,%ymm8
  4aee1c:	c5 fc 28 f4          	vmovaps %ymm4,%ymm6
  4aee20:	c5 fc 28 e7          	vmovaps %ymm7,%ymm4
  4aee24:	c5 fc 28 ea          	vmovaps %ymm2,%ymm5
  4aee28:	c4 82 7d 18 3c 91    	vbroadcastss (%r9,%r10,4),%ymm7
  4aee2e:	c4 a2 7d 18 14 96    	vbroadcastss (%rsi,%r10,4),%ymm2
  4aee34:	c4 22 7d 18 0c 92    	vbroadcastss (%rdx,%r10,4),%ymm9
  4aee3a:	c4 22 7d 18 14 97    	vbroadcastss (%rdi,%r10,4),%ymm10
  4aee40:	49 ff c2             	inc    %r10
  4aee43:	c4 e2 55 b8 ca       	vfmadd231ps %ymm2,%ymm5,%ymm1
  4aee48:	c4 e2 5d b8 ca       	vfmadd231ps %ymm2,%ymm4,%ymm1
  4aee4d:	c4 c2 55 b8 d9       	vfmadd231ps %ymm9,%ymm5,%ymm3
  4aee52:	c4 c2 5d b8 d9       	vfmadd231ps %ymm9,%ymm4,%ymm3
  4aee57:	c5 54 59 cf          	vmulps %ymm7,%ymm5,%ymm9
  4aee5b:	c5 7c 28 dd          	vmovaps %ymm5,%ymm11
  4aee5f:	c4 42 45 aa d9       	vfmsub213ps %ymm9,%ymm7,%ymm11
  4aee64:	c4 c1 3c 58 d1       	vaddps %ymm9,%ymm8,%ymm2
  4aee69:	c4 41 6c 5c e1       	vsubps %ymm9,%ymm2,%ymm12
  4aee6e:	c4 41 6c 5c ec       	vsubps %ymm12,%ymm2,%ymm13
  4aee73:	c4 41 34 5c cd       	vsubps %ymm13,%ymm9,%ymm9
  4aee78:	c4 41 3c 5c c4       	vsubps %ymm12,%ymm8,%ymm8
  4aee7d:	c4 41 3c 58 c1       	vaddps %ymm9,%ymm8,%ymm8
  4aee82:	c4 41 24 58 c0       	vaddps %ymm8,%ymm11,%ymm8
  4aee87:	c4 62 5d b8 c7       	vfmadd231ps %ymm7,%ymm4,%ymm8
  4aee8c:	c4 42 55 b8 c2       	vfmadd231ps %ymm10,%ymm5,%ymm8
  4aee91:	c5 bc 58 fe          	vaddps %ymm6,%ymm8,%ymm7
  4aee95:	4d 39 d0             	cmp    %r10,%r8
  4aee98:	0f 85 7a ff ff ff    	jne    4aee18 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x418>
  4aee9e:	e9 38 04 00 00       	jmp    4af2db <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x8db>
  4aeea3:	4d 85 c0             	test   %r8,%r8
  4aeea6:	0f 84 ba 01 00 00    	je     4af066 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x666>
  4aeeac:	49 83 f8 01          	cmp    $0x1,%r8
  4aeeb0:	0f 85 bd 01 00 00    	jne    4af073 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x673>
  4aeeb6:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  4aeeba:	31 ff                	xor    %edi,%edi
  4aeebc:	c5 f0 57 c9          	vxorps %xmm1,%xmm1,%xmm1
  4aeec0:	e9 32 02 00 00       	jmp    4af0f7 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x6f7>
  4aeec5:	80 bf 12 01 00 00 00 	cmpb   $0x0,0x112(%rdi)
  4aeecc:	0f 84 f6 02 00 00    	je     4af1c8 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7c8>
  4aeed2:	31 c9                	xor    %ecx,%ecx
  4aeed4:	48 83 7f 58 08       	cmpq   $0x8,0x58(%rdi)
  4aeed9:	0f 85 de 02 00 00    	jne    4af1bd <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7bd>
  4aeedf:	48 83 7f 78 08       	cmpq   $0x8,0x78(%rdi)
  4aeee4:	0f 85 d3 02 00 00    	jne    4af1bd <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7bd>
  4aeeea:	48 83 bf 98 00 00 00 	cmpq   $0x8,0x98(%rdi)
  4aeef1:	08
  4aeef2:	0f 85 c5 02 00 00    	jne    4af1bd <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7bd>
  4aeef8:	4c 39 87 a8 00 00 00 	cmp    %r8,0xa8(%rdi)
  4aeeff:	0f 85 07 04 00 00    	jne    4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4aef05:	4c 39 87 b8 00 00 00 	cmp    %r8,0xb8(%rdi)
  4aef0c:	0f 85 fa 03 00 00    	jne    4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4aef12:	4c 39 87 c8 00 00 00 	cmp    %r8,0xc8(%rdi)
  4aef19:	0f 85 ed 03 00 00    	jne    4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4aef1f:	4c 39 87 d8 00 00 00 	cmp    %r8,0xd8(%rdi)
  4aef26:	0f 85 e0 03 00 00    	jne    4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4aef2c:	48 8b 4f 70          	mov    0x70(%rdi),%rcx
  4aef30:	c5 fc 10 09          	vmovups (%rcx),%ymm1
  4aef34:	c5 fc 11 0c 24       	vmovups %ymm1,(%rsp)
  4aef39:	4d 85 c0             	test   %r8,%r8
  4aef3c:	0f 84 d7 03 00 00    	je     4af319 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x919>
  4aef42:	4c 8b 5f 50          	mov    0x50(%rdi),%r11
  4aef46:	48 8b 9f 90 00 00 00 	mov    0x90(%rdi),%rbx
  4aef4d:	48 8b 8f a0 00 00 00 	mov    0xa0(%rdi),%rcx
  4aef54:	4c 8b 8f b0 00 00 00 	mov    0xb0(%rdi),%r9
  4aef5b:	4c 8b 97 c0 00 00 00 	mov    0xc0(%rdi),%r10
  4aef62:	48 8b bf d0 00 00 00 	mov    0xd0(%rdi),%rdi
  4aef69:	c4 c1 7c 10 2b       	vmovups (%r11),%ymm5
  4aef6e:	c5 7c 10 13          	vmovups (%rbx),%ymm10
  4aef72:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  4aef76:	45 31 db             	xor    %r11d,%r11d
  4aef79:	c5 c0 57 ff          	vxorps %xmm7,%xmm7,%xmm7
  4aef7d:	c4 41 38 57 c0       	vxorps %xmm8,%xmm8,%xmm8
  4aef82:	c5 c8 57 f6          	vxorps %xmm6,%xmm6,%xmm6
  4aef86:	c5 7c 29 fb          	vmovaps %ymm15,%ymm3
  4aef8a:	c5 7c 28 cf          	vmovaps %ymm7,%ymm9
  4aef8e:	c5 7c 29 d7          	vmovaps %ymm10,%ymm7
  4aef92:	c4 22 7d 18 14 9a    	vbroadcastss (%rdx,%r11,4),%ymm10
  4aef98:	c4 22 7d 18 1c 9e    	vbroadcastss (%rsi,%r11,4),%ymm11
  4aef9e:	c4 c2 6d b8 e2       	vfmadd231ps %ymm10,%ymm2,%ymm4
  4aefa3:	c4 c2 45 b8 e2       	vfmadd231ps %ymm10,%ymm7,%ymm4
  4aefa8:	c4 c2 6d b8 f3       	vfmadd231ps %ymm11,%ymm2,%ymm6
  4aefad:	c4 c2 45 b8 f3       	vfmadd231ps %ymm11,%ymm7,%ymm6
  4aefb2:	c4 22 7d 18 14 99    	vbroadcastss (%rcx,%r11,4),%ymm10
  4aefb8:	c4 02 7d 18 1c 99    	vbroadcastss (%r9,%r11,4),%ymm11
  4aefbe:	c5 2c 59 e3          	vmulps %ymm3,%ymm10,%ymm12
  4aefc2:	c5 7c 28 eb          	vmovaps %ymm3,%ymm13
  4aefc6:	c4 42 2d aa ec       	vfmsub213ps %ymm12,%ymm10,%ymm13
  4aefcb:	c4 41 1c 58 f3       	vaddps %ymm11,%ymm12,%ymm14
  4aefd0:	c4 41 0c 5c fc       	vsubps %ymm12,%ymm14,%ymm15
  4aefd5:	c4 c1 0c 5c cf       	vsubps %ymm15,%ymm14,%ymm1
  4aefda:	c5 9c 5c c9          	vsubps %ymm1,%ymm12,%ymm1
  4aefde:	c4 02 7d 18 24 9a    	vbroadcastss (%r10,%r11,4),%ymm12
  4aefe4:	c4 41 24 5c df       	vsubps %ymm15,%ymm11,%ymm11
  4aefe9:	c4 22 7d 18 3c 9f    	vbroadcastss (%rdi,%r11,4),%ymm15
  4aefef:	c5 a4 58 c9          	vaddps %ymm1,%ymm11,%ymm1
  4aeff3:	c5 94 58 c9          	vaddps %ymm1,%ymm13,%ymm1
  4aeff7:	c5 1c 59 dd          	vmulps %ymm5,%ymm12,%ymm11
  4aeffb:	c4 42 65 b8 dc       	vfmadd231ps %ymm12,%ymm3,%ymm11
  4af000:	c4 42 55 b8 da       	vfmadd231ps %ymm10,%ymm5,%ymm11
  4af005:	c4 41 04 58 d3       	vaddps %ymm11,%ymm15,%ymm10
  4af00a:	c5 ac 58 c9          	vaddps %ymm1,%ymm10,%ymm1
  4af00e:	c5 0c 59 d2          	vmulps %ymm2,%ymm14,%ymm10
  4af012:	c5 7c 28 da          	vmovaps %ymm2,%ymm11
  4af016:	c4 42 0d aa da       	vfmsub213ps %ymm10,%ymm14,%ymm11
  4af01b:	c4 41 3c 58 e2       	vaddps %ymm10,%ymm8,%ymm12
  4af020:	c4 41 1c 5c ea       	vsubps %ymm10,%ymm12,%ymm13
  4af025:	c4 41 1c 5c fd       	vsubps %ymm13,%ymm12,%ymm15
  4af02a:	c4 41 2c 5c d7       	vsubps %ymm15,%ymm10,%ymm10
  4af02f:	c4 41 3c 5c c5       	vsubps %ymm13,%ymm8,%ymm8
  4af034:	c4 41 3c 58 c2       	vaddps %ymm10,%ymm8,%ymm8
  4af039:	c4 41 24 58 c0       	vaddps %ymm8,%ymm11,%ymm8
  4af03e:	c4 42 45 b8 c6       	vfmadd231ps %ymm14,%ymm7,%ymm8
  4af043:	c4 62 6d b8 c1       	vfmadd231ps %ymm1,%ymm2,%ymm8
  4af048:	c4 41 34 58 d0       	vaddps %ymm8,%ymm9,%ymm10
  4af04d:	49 ff c3             	inc    %r11
  4af050:	c5 7c 28 c2          	vmovaps %ymm2,%ymm8
  4af054:	c5 7c 29 e2          	vmovaps %ymm12,%ymm2
  4af058:	4d 39 d8             	cmp    %r11,%r8
  4af05b:	0f 85 29 ff ff ff    	jne    4aef8a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x58a>
  4af061:	e9 bb 02 00 00       	jmp    4af321 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x921>
  4af066:	c5 f0 57 c9          	vxorps %xmm1,%xmm1,%xmm1
  4af06a:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  4af06e:	e9 a2 00 00 00       	jmp    4af115 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x715>
  4af073:	4d 89 c2             	mov    %r8,%r10
  4af076:	49 83 e2 fe          	and    $0xfffffffffffffffe,%r10
  4af07a:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  4af07e:	31 ff                	xor    %edi,%edi
  4af080:	c5 d0 57 ed          	vxorps %xmm5,%xmm5,%xmm5
  4af084:	c5 f0 57 c9          	vxorps %xmm1,%xmm1,%xmm1
  4af088:	c5 fc 28 f2          	vmovaps %ymm2,%ymm6
  4af08c:	c4 e2 7d 18 14 b9    	vbroadcastss (%rcx,%rdi,4),%ymm2
  4af092:	c4 e2 7d 18 3c be    	vbroadcastss (%rsi,%rdi,4),%ymm7
  4af098:	c4 e2 4d a8 f9       	vfmadd213ps %ymm1,%ymm6,%ymm7
  4af09d:	c4 62 7d 18 04 ba    	vbroadcastss (%rdx,%rdi,4),%ymm8
  4af0a3:	c4 62 4d a8 c4       	vfmadd213ps %ymm4,%ymm6,%ymm8
  4af0a8:	c4 c2 7d 18 0c b9    	vbroadcastss (%r9,%rdi,4),%ymm1
  4af0ae:	c4 e2 05 b8 ca       	vfmadd231ps %ymm2,%ymm15,%ymm1
  4af0b3:	c4 e2 4d b8 e9       	vfmadd231ps %ymm1,%ymm6,%ymm5
  4af0b8:	c4 62 7d 18 4c b9 04 	vbroadcastss 0x4(%rcx,%rdi,4),%ymm9
  4af0bf:	c4 e2 7d 18 4c be 04 	vbroadcastss 0x4(%rsi,%rdi,4),%ymm1
  4af0c6:	c4 e2 55 a8 cf       	vfmadd213ps %ymm7,%ymm5,%ymm1
  4af0cb:	c4 e2 7d 18 64 ba 04 	vbroadcastss 0x4(%rdx,%rdi,4),%ymm4
  4af0d2:	c4 c2 55 a8 e0       	vfmadd213ps %ymm8,%ymm5,%ymm4
  4af0d7:	c4 c2 7d 18 54 b9 04 	vbroadcastss 0x4(%r9,%rdi,4),%ymm2
  4af0de:	48 83 c7 02          	add    $0x2,%rdi
  4af0e2:	c4 c2 05 b8 d1       	vfmadd231ps %ymm9,%ymm15,%ymm2
  4af0e7:	c4 e2 55 a8 d6       	vfmadd213ps %ymm6,%ymm5,%ymm2
  4af0ec:	49 39 fa             	cmp    %rdi,%r10
  4af0ef:	75 97                	jne    4af088 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x688>
  4af0f1:	41 f6 c0 01          	test   $0x1,%r8b
  4af0f5:	74 1e                	je     4af115 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x715>
  4af0f7:	c5 fc 28 d9          	vmovaps %ymm1,%ymm3
  4af0fb:	c5 fc 28 ec          	vmovaps %ymm4,%ymm5
  4af0ff:	c4 e2 7d 18 0c be    	vbroadcastss (%rsi,%rdi,4),%ymm1
  4af105:	c4 e2 6d a8 cb       	vfmadd213ps %ymm3,%ymm2,%ymm1
  4af10a:	c4 e2 7d 18 24 ba    	vbroadcastss (%rdx,%rdi,4),%ymm4
  4af110:	c4 e2 6d a8 e5       	vfmadd213ps %ymm5,%ymm2,%ymm4
  4af115:	c4 e2 7d a8 cc       	vfmadd213ps %ymm4,%ymm0,%ymm1
  4af11a:	48 8b 0d ff 26 3d 00 	mov    0x3d26ff(%rip),%rcx        # 881820 <_DYNAMIC+0x548>
  4af121:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  4af128:	c5 fc 28 c1          	vmovaps %ymm1,%ymm0
  4af12c:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 4af135 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x735>
  4af133:	ff e2                	jmp    *%rdx
  4af135:	8b 08                	mov    (%rax),%ecx
  4af137:	01 c9                	add    %ecx,%ecx
  4af139:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4af13f:	0f 87 c7 01 00 00    	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4af145:	8b 48 04             	mov    0x4(%rax),%ecx
  4af148:	01 c9                	add    %ecx,%ecx
  4af14a:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4af150:	0f 87 b6 01 00 00    	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4af156:	8b 48 08             	mov    0x8(%rax),%ecx
  4af159:	01 c9                	add    %ecx,%ecx
  4af15b:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4af161:	0f 87 a5 01 00 00    	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4af167:	8b 48 0c             	mov    0xc(%rax),%ecx
  4af16a:	01 c9                	add    %ecx,%ecx
  4af16c:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4af172:	0f 87 94 01 00 00    	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4af178:	8b 48 10             	mov    0x10(%rax),%ecx
  4af17b:	01 c9                	add    %ecx,%ecx
  4af17d:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4af183:	0f 87 83 01 00 00    	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4af189:	8b 48 14             	mov    0x14(%rax),%ecx
  4af18c:	01 c9                	add    %ecx,%ecx
  4af18e:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4af194:	0f 87 72 01 00 00    	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4af19a:	8b 48 18             	mov    0x18(%rax),%ecx
  4af19d:	01 c9                	add    %ecx,%ecx
  4af19f:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4af1a5:	0f 87 61 01 00 00    	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4af1ab:	8b 40 1c             	mov    0x1c(%rax),%eax
  4af1ae:	01 c0                	add    %eax,%eax
  4af1b0:	3d ff ff ff fe       	cmp    $0xfeffffff,%eax
  4af1b5:	0f 87 51 01 00 00    	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4af1bb:	b1 01                	mov    $0x1,%cl
  4af1bd:	48 83 c4 40          	add    $0x40,%rsp
  4af1c1:	5b                   	pop    %rbx
  4af1c2:	89 c8                	mov    %ecx,%eax
  4af1c4:	c5 f8 77             	vzeroupper
  4af1c7:	c3                   	ret
  4af1c8:	4d 85 c0             	test   %r8,%r8
  4af1cb:	0f 84 c9 00 00 00    	je     4af29a <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x89a>
  4af1d1:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  4af1d5:	31 ff                	xor    %edi,%edi
  4af1d7:	c4 41 38 57 c0       	vxorps %xmm8,%xmm8,%xmm8
  4af1dc:	c5 d0 57 ed          	vxorps %xmm5,%xmm5,%xmm5
  4af1e0:	c5 c8 57 f6          	vxorps %xmm6,%xmm6,%xmm6
  4af1e4:	c5 f0 57 c9          	vxorps %xmm1,%xmm1,%xmm1
  4af1e8:	c5 fc 28 fd          	vmovaps %ymm5,%ymm7
  4af1ec:	c5 7c 29 c5          	vmovaps %ymm8,%ymm5
  4af1f0:	c4 62 7d 18 04 b9    	vbroadcastss (%rcx,%rdi,4),%ymm8
  4af1f6:	c4 62 7d 18 0c be    	vbroadcastss (%rsi,%rdi,4),%ymm9
  4af1fc:	c4 62 7d 18 14 ba    	vbroadcastss (%rdx,%rdi,4),%ymm10
  4af202:	c4 42 7d 18 1c b9    	vbroadcastss (%r9,%rdi,4),%ymm11
  4af208:	48 ff c7             	inc    %rdi
  4af20b:	c4 c2 6d b8 c9       	vfmadd231ps %ymm9,%ymm2,%ymm1
  4af210:	c4 c2 55 b8 c9       	vfmadd231ps %ymm9,%ymm5,%ymm1
  4af215:	c4 c2 6d b8 e2       	vfmadd231ps %ymm10,%ymm2,%ymm4
  4af21a:	c4 c2 55 b8 e2       	vfmadd231ps %ymm10,%ymm5,%ymm4
  4af21f:	c4 41 04 59 c8       	vmulps %ymm8,%ymm15,%ymm9
  4af224:	c4 42 05 aa c1       	vfmsub213ps %ymm9,%ymm15,%ymm8
  4af229:	c4 41 34 58 d3       	vaddps %ymm11,%ymm9,%ymm10
  4af22e:	c4 41 2c 5c e1       	vsubps %ymm9,%ymm10,%ymm12
  4af233:	c4 41 2c 5c ec       	vsubps %ymm12,%ymm10,%ymm13
  4af238:	c4 41 34 5c cd       	vsubps %ymm13,%ymm9,%ymm9
  4af23d:	c4 41 24 5c dc       	vsubps %ymm12,%ymm11,%ymm11
  4af242:	c4 41 24 58 c9       	vaddps %ymm9,%ymm11,%ymm9
  4af247:	c4 41 3c 58 c1       	vaddps %ymm9,%ymm8,%ymm8
  4af24c:	c5 2c 59 ca          	vmulps %ymm2,%ymm10,%ymm9
  4af250:	c5 7c 28 da          	vmovaps %ymm2,%ymm11
  4af254:	c4 42 2d aa d9       	vfmsub213ps %ymm9,%ymm10,%ymm11
  4af259:	c5 34 58 e6          	vaddps %ymm6,%ymm9,%ymm12
  4af25d:	c4 41 1c 5c e9       	vsubps %ymm9,%ymm12,%ymm13
  4af262:	c4 41 1c 5c f5       	vsubps %ymm13,%ymm12,%ymm14
  4af267:	c4 41 34 5c ce       	vsubps %ymm14,%ymm9,%ymm9
  4af26c:	c4 c1 4c 5c f5       	vsubps %ymm13,%ymm6,%ymm6
  4af271:	c5 b4 58 f6          	vaddps %ymm6,%ymm9,%ymm6
  4af275:	c5 a4 58 f6          	vaddps %ymm6,%ymm11,%ymm6
  4af279:	c4 c2 55 b8 f2       	vfmadd231ps %ymm10,%ymm5,%ymm6
  4af27e:	c4 c2 6d b8 f0       	vfmadd231ps %ymm8,%ymm2,%ymm6
  4af283:	c5 44 58 c6          	vaddps %ymm6,%ymm7,%ymm8
  4af287:	c5 fc 28 f2          	vmovaps %ymm2,%ymm6
  4af28b:	c5 7c 29 e2          	vmovaps %ymm12,%ymm2
  4af28f:	49 39 f8             	cmp    %rdi,%r8
  4af292:	0f 85 50 ff ff ff    	jne    4af1e8 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x7e8>
  4af298:	eb 08                	jmp    4af2a2 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x8a2>
  4af29a:	c5 f0 57 c9          	vxorps %xmm1,%xmm1,%xmm1
  4af29e:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  4af2a2:	c4 e2 7d a8 cc       	vfmadd213ps %ymm4,%ymm0,%ymm1
  4af2a7:	48 8b 0d 72 25 3d 00 	mov    0x3d2572(%rip),%rcx        # 881820 <_DYNAMIC+0x548>
  4af2ae:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  4af2b5:	c5 fc 28 c1          	vmovaps %ymm1,%ymm0
  4af2b9:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 4af2c2 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x8c2>
  4af2c0:	ff e2                	jmp    *%rdx
  4af2c2:	8b 08                	mov    (%rax),%ecx
  4af2c4:	01 c9                	add    %ecx,%ecx
  4af2c6:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4af2cc:	77 3e                	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4af2ce:	e9 72 fe ff ff       	jmp    4af145 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x745>
  4af2d3:	c5 e0 57 db          	vxorps %xmm3,%xmm3,%xmm3
  4af2d7:	c5 f0 57 c9          	vxorps %xmm1,%xmm1,%xmm1
  4af2db:	c4 e2 75 a8 c3       	vfmadd213ps %ymm3,%ymm1,%ymm0
  4af2e0:	c4 e2 75 b8 01       	vfmadd231ps (%rcx),%ymm1,%ymm0
  4af2e5:	48 8b 0d 34 25 3d 00 	mov    0x3d2534(%rip),%rcx        # 881820 <_DYNAMIC+0x548>
  4af2ec:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  4af2f3:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 4af2fc <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x8fc>
  4af2fa:	ff e2                	jmp    *%rdx
  4af2fc:	8b 08                	mov    (%rax),%ecx
  4af2fe:	01 c9                	add    %ecx,%ecx
  4af300:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4af306:	0f 86 39 fe ff ff    	jbe    4af145 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x745>
  4af30c:	31 c9                	xor    %ecx,%ecx
  4af30e:	48 83 c4 40          	add    $0x40,%rsp
  4af312:	5b                   	pop    %rbx
  4af313:	89 c8                	mov    %ecx,%eax
  4af315:	c5 f8 77             	vzeroupper
  4af318:	c3                   	ret
  4af319:	c5 c8 57 f6          	vxorps %xmm6,%xmm6,%xmm6
  4af31d:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  4af321:	c4 e2 4d a8 c4       	vfmadd213ps %ymm4,%ymm6,%ymm0
  4af326:	c5 fc 10 0c 24       	vmovups (%rsp),%ymm1
  4af32b:	c4 e2 4d a8 c8       	vfmadd213ps %ymm0,%ymm6,%ymm1
  4af330:	48 8b 0d e9 24 3d 00 	mov    0x3d24e9(%rip),%rcx        # 881820 <_DYNAMIC+0x548>
  4af337:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  4af33e:	c5 fc 28 c1          	vmovaps %ymm1,%ymm0
  4af342:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 4af34b <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x94b>
  4af349:	ff e2                	jmp    *%rdx
  4af34b:	8b 08                	mov    (%rax),%ecx
  4af34d:	01 c9                	add    %ecx,%ecx
  4af34f:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  4af355:	77 b5                	ja     4af30c <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x90c>
  4af357:	e9 e9 fd ff ff       	jmp    4af145 <<rustsht::legendre::scalar_ordered::tests::OrderedF32VectorReplay as pulp::WithSimd>::with_simd::<pulp::x86::v3::V3>+0x745>
  4af35c:	cc                   	int3
  4af35d:	cc                   	int3
  4af35e:	cc                   	int3
  4af35f:	cc                   	int3
