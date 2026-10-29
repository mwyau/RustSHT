00000000001dde00 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>>:
  1dde00:	55                   	push   %rbp
  1dde01:	48 89 e5             	mov    %rsp,%rbp
  1dde04:	41 57                	push   %r15
  1dde06:	41 56                	push   %r14
  1dde08:	41 55                	push   %r13
  1dde0a:	41 54                	push   %r12
  1dde0c:	53                   	push   %rbx
  1dde0d:	48 83 e4 e0          	and    $0xffffffffffffffe0,%rsp
  1dde11:	48 81 ec 80 02 00 00 	sub    $0x280,%rsp
  1dde18:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  1dde1d:	48 89 7c 24 08       	mov    %rdi,0x8(%rsp)
  1dde22:	48 89 f0             	mov    %rsi,%rax
  1dde25:	48 31 c8             	xor    %rcx,%rax
  1dde28:	49 31 c9             	xor    %rcx,%r9
  1dde2b:	41 b2 02             	mov    $0x2,%r10b
  1dde2e:	49 09 c1             	or     %rax,%r9
  1dde31:	0f 85 92 0c 00 00    	jne    1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1dde37:	4c 89 84 24 a8 00 00 	mov    %r8,0xa8(%rsp)
  1dde3e:	00
  1dde3f:	0f 57 c0             	xorps  %xmm0,%xmm0
  1dde42:	0f 29 84 24 b0 01 00 	movaps %xmm0,0x1b0(%rsp)
  1dde49:	00
  1dde4a:	0f 29 84 24 a0 01 00 	movaps %xmm0,0x1a0(%rsp)
  1dde51:	00
  1dde52:	0f 29 84 24 90 01 00 	movaps %xmm0,0x190(%rsp)
  1dde59:	00
  1dde5a:	0f 29 84 24 80 01 00 	movaps %xmm0,0x180(%rsp)
  1dde61:	00
  1dde62:	48 89 74 24 58       	mov    %rsi,0x58(%rsp)
  1dde67:	49 89 f4             	mov    %rsi,%r12
  1dde6a:	49 c1 ec 03          	shr    $0x3,%r12
  1dde6e:	48 8d 7c 24 20       	lea    0x20(%rsp),%rdi
  1dde73:	f3 0f 10 05 4d 0b e4 	movss  -0x1bf4b3(%rip),%xmm0        # 1e9c8 <__abi_tag+0x1e6cc>
  1dde7a:	ff
  1dde7b:	e8 70 74 00 00       	call   1e52f0 <core::core_arch::x86::avx::_mm256_set1_ps>
  1dde80:	0f 28 44 24 20       	movaps 0x20(%rsp),%xmm0
  1dde85:	0f 29 84 24 30 02 00 	movaps %xmm0,0x230(%rsp)
  1dde8c:	00
  1dde8d:	0f 28 44 24 30       	movaps 0x30(%rsp),%xmm0
  1dde92:	0f 29 84 24 20 02 00 	movaps %xmm0,0x220(%rsp)
  1dde99:	00
  1dde9a:	48 8d 7c 24 20       	lea    0x20(%rsp),%rdi
  1dde9f:	f3 0f 10 05 b9 0b e4 	movss  -0x1bf447(%rip),%xmm0        # 1ea60 <__abi_tag+0x1e764>
  1ddea6:	ff
  1ddea7:	e8 44 74 00 00       	call   1e52f0 <core::core_arch::x86::avx::_mm256_set1_ps>
  1ddeac:	0f 28 44 24 20       	movaps 0x20(%rsp),%xmm0
  1ddeb1:	0f 29 84 24 10 02 00 	movaps %xmm0,0x210(%rsp)
  1ddeb8:	00
  1ddeb9:	0f 28 44 24 30       	movaps 0x30(%rsp),%xmm0
  1ddebe:	0f 29 84 24 00 02 00 	movaps %xmm0,0x200(%rsp)
  1ddec5:	00
  1ddec6:	48 8d 7c 24 20       	lea    0x20(%rsp),%rdi
  1ddecb:	0f 57 c0             	xorps  %xmm0,%xmm0
  1ddece:	e8 1d 74 00 00       	call   1e52f0 <core::core_arch::x86::avx::_mm256_set1_ps>
  1dded3:	4d 85 e4             	test   %r12,%r12
  1dded6:	0f 84 6a 07 00 00    	je     1de646 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x846>
  1ddedc:	0f 28 44 24 20       	movaps 0x20(%rsp),%xmm0
  1ddee1:	0f 29 84 24 60 01 00 	movaps %xmm0,0x160(%rsp)
  1ddee8:	00
  1ddee9:	0f 28 44 24 30       	movaps 0x30(%rsp),%xmm0
  1ddeee:	0f 29 84 24 50 01 00 	movaps %xmm0,0x150(%rsp)
  1ddef5:	00
  1ddef6:	48 c7 44 24 18 00 00 	movq   $0x0,0x18(%rsp)
  1ddefd:	00 00
  1ddeff:	4d 89 e7             	mov    %r12,%r15
  1ddf02:	31 c9                	xor    %ecx,%ecx
  1ddf04:	48 8b 54 24 58       	mov    0x58(%rsp),%rdx
  1ddf09:	4c 89 a4 24 78 01 00 	mov    %r12,0x178(%rsp)
  1ddf10:	00
  1ddf11:	4c 8d 24 cd 00 00 00 	lea    0x0(,%rcx,8),%r12
  1ddf18:	00
  1ddf19:	48 ff c1             	inc    %rcx
  1ddf1c:	49 f7 df             	neg    %r15
  1ddf1f:	90                   	nop
  1ddf20:	49 8d 44 24 07       	lea    0x7(%r12),%rax
  1ddf25:	48 39 d0             	cmp    %rdx,%rax
  1ddf28:	0f 83 83 0b 00 00    	jae    1deab1 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcb1>
  1ddf2e:	48 89 8c 24 f8 00 00 	mov    %rcx,0xf8(%rsp)
  1ddf35:	00
  1ddf36:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  1ddf3b:	42 0f 10 04 a0       	movups (%rax,%r12,4),%xmm0
  1ddf40:	0f 29 84 24 e0 00 00 	movaps %xmm0,0xe0(%rsp)
  1ddf47:	00
  1ddf48:	42 0f 10 44 a0 10    	movups 0x10(%rax,%r12,4),%xmm0
  1ddf4e:	0f 29 84 24 d0 00 00 	movaps %xmm0,0xd0(%rsp)
  1ddf55:	00
  1ddf56:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
  1ddf5b:	42 0f 10 04 a0       	movups (%rax,%r12,4),%xmm0
  1ddf60:	0f 29 84 24 c0 00 00 	movaps %xmm0,0xc0(%rsp)
  1ddf67:	00
  1ddf68:	42 0f 10 44 a0 10    	movups 0x10(%rax,%r12,4),%xmm0
  1ddf6e:	0f 29 84 24 b0 00 00 	movaps %xmm0,0xb0(%rsp)
  1ddf75:	00
  1ddf76:	48 8d 9c 24 80 00 00 	lea    0x80(%rsp),%rbx
  1ddf7d:	00
  1ddf7e:	48 89 df             	mov    %rbx,%rdi
  1ddf81:	f3 0f 10 05 c7 08 e4 	movss  -0x1bf739(%rip),%xmm0        # 1e850 <__abi_tag+0x1e554>
  1ddf88:	ff
  1ddf89:	e8 62 73 00 00       	call   1e52f0 <core::core_arch::x86::avx::_mm256_set1_ps>
  1ddf8e:	4c 8d 74 24 20       	lea    0x20(%rsp),%r14
  1ddf93:	4c 89 f7             	mov    %r14,%rdi
  1ddf96:	be ff ff ff ff       	mov    $0xffffffff,%esi
  1ddf9b:	e8 70 d5 ff ff       	call   1db510 <core::core_arch::x86::avx::_mm256_set1_epi32>
  1ddfa0:	4c 8d 6c 24 60       	lea    0x60(%rsp),%r13
  1ddfa5:	4c 89 ef             	mov    %r13,%rdi
  1ddfa8:	48 89 de             	mov    %rbx,%rsi
  1ddfab:	4c 89 f2             	mov    %r14,%rdx
  1ddfae:	e8 ad d5 ff ff       	call   1db560 <core::core_arch::x86::avx2::_mm256_xor_si256>
  1ddfb3:	0f 28 84 24 b0 00 00 	movaps 0xb0(%rsp),%xmm0
  1ddfba:	00
  1ddfbb:	0f 29 44 24 30       	movaps %xmm0,0x30(%rsp)
  1ddfc0:	0f 28 84 24 c0 00 00 	movaps 0xc0(%rsp),%xmm0
  1ddfc7:	00
  1ddfc8:	0f 29 44 24 20       	movaps %xmm0,0x20(%rsp)
  1ddfcd:	48 89 df             	mov    %rbx,%rdi
  1ddfd0:	4c 89 ee             	mov    %r13,%rsi
  1ddfd3:	4c 89 f2             	mov    %r14,%rdx
  1ddfd6:	e8 f5 72 00 00       	call   1e52d0 <core::core_arch::x86::avx::_mm256_and_ps>
  1ddfdb:	0f 28 84 24 20 02 00 	movaps 0x220(%rsp),%xmm0
  1ddfe2:	00
  1ddfe3:	0f 29 44 24 30       	movaps %xmm0,0x30(%rsp)
  1ddfe8:	0f 28 84 24 30 02 00 	movaps 0x230(%rsp),%xmm0
  1ddfef:	00
  1ddff0:	0f 29 44 24 20       	movaps %xmm0,0x20(%rsp)
  1ddff5:	4c 89 ef             	mov    %r13,%rdi
  1ddff8:	48 89 de             	mov    %rbx,%rsi
  1ddffb:	4c 89 f2             	mov    %r14,%rdx
  1ddffe:	e8 dd 3c 00 00       	call   1e1ce0 <core::core_arch::x86::avx::_mm256_cmp_ps::<30>>
  1de003:	66 0f 6f 54 24 60    	movdqa 0x60(%rsp),%xmm2
  1de009:	66 0f 6f 5c 24 70    	movdqa 0x70(%rsp),%xmm3
  1de00f:	66 0f 6f c2          	movdqa %xmm2,%xmm0
  1de013:	66 0f eb c3          	por    %xmm3,%xmm0
  1de017:	66 0f ef c9          	pxor   %xmm1,%xmm1
  1de01b:	66 0f 76 c1          	pcmpeqd %xmm1,%xmm0
  1de01f:	0f 50 c0             	movmskps %xmm0,%eax
  1de022:	83 f0 0f             	xor    $0xf,%eax
  1de025:	75 39                	jne    1de060 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x260>
  1de027:	49 83 c4 08          	add    $0x8,%r12
  1de02b:	48 8b 8c 24 f8 00 00 	mov    0xf8(%rsp),%rcx
  1de032:	00
  1de033:	49 8d 04 0f          	lea    (%r15,%rcx,1),%rax
  1de037:	48 ff c0             	inc    %rax
  1de03a:	48 ff c1             	inc    %rcx
  1de03d:	48 83 f8 01          	cmp    $0x1,%rax
  1de041:	48 8b 54 24 58       	mov    0x58(%rsp),%rdx
  1de046:	f3 0f 10 2d 12 0a e4 	movss  -0x1bf5ee(%rip),%xmm5        # 1ea60 <__abi_tag+0x1e764>
  1de04d:	ff
  1de04e:	0f 85 cc fe ff ff    	jne    1ddf20 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x120>
  1de054:	e9 03 06 00 00       	jmp    1de65c <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x85c>
  1de059:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  1de060:	0f 28 84 24 d0 00 00 	movaps 0xd0(%rsp),%xmm0
  1de067:	00
  1de068:	0f 29 84 24 90 00 00 	movaps %xmm0,0x90(%rsp)
  1de06f:	00
  1de070:	0f 28 84 24 e0 00 00 	movaps 0xe0(%rsp),%xmm0
  1de077:	00
  1de078:	0f 29 84 24 80 00 00 	movaps %xmm0,0x80(%rsp)
  1de07f:	00
  1de080:	0f 28 84 24 00 02 00 	movaps 0x200(%rsp),%xmm0
  1de087:	00
  1de088:	0f 29 44 24 30       	movaps %xmm0,0x30(%rsp)
  1de08d:	0f 28 84 24 10 02 00 	movaps 0x210(%rsp),%xmm0
  1de094:	00
  1de095:	0f 29 44 24 20       	movaps %xmm0,0x20(%rsp)
  1de09a:	4c 8d 7c 24 60       	lea    0x60(%rsp),%r15
  1de09f:	4c 89 ff             	mov    %r15,%rdi
  1de0a2:	4c 8d ac 24 80 00 00 	lea    0x80(%rsp),%r13
  1de0a9:	00
  1de0aa:	4c 89 ee             	mov    %r13,%rsi
  1de0ad:	48 8d 5c 24 20       	lea    0x20(%rsp),%rbx
  1de0b2:	48 89 da             	mov    %rbx,%rdx
  1de0b5:	66 0f 7f 94 24 40 01 	movdqa %xmm2,0x140(%rsp)
  1de0bc:	00 00
  1de0be:	66 0f 7f 9c 24 30 01 	movdqa %xmm3,0x130(%rsp)
  1de0c5:	00 00
  1de0c7:	e8 14 72 00 00       	call   1e52e0 <core::core_arch::x86::avx::_mm256_mul_ps>
  1de0cc:	0f 28 44 24 60       	movaps 0x60(%rsp),%xmm0
  1de0d1:	0f 29 84 24 d0 01 00 	movaps %xmm0,0x1d0(%rsp)
  1de0d8:	00
  1de0d9:	0f 28 44 24 70       	movaps 0x70(%rsp),%xmm0
  1de0de:	0f 29 84 24 c0 01 00 	movaps %xmm0,0x1c0(%rsp)
  1de0e5:	00
  1de0e6:	0f 28 84 24 b0 00 00 	movaps 0xb0(%rsp),%xmm0
  1de0ed:	00
  1de0ee:	0f 29 84 24 90 00 00 	movaps %xmm0,0x90(%rsp)
  1de0f5:	00
  1de0f6:	0f 28 84 24 c0 00 00 	movaps 0xc0(%rsp),%xmm0
  1de0fd:	00
  1de0fe:	0f 29 84 24 80 00 00 	movaps %xmm0,0x80(%rsp)
  1de105:	00
  1de106:	0f 28 84 24 00 02 00 	movaps 0x200(%rsp),%xmm0
  1de10d:	00
  1de10e:	0f 29 44 24 30       	movaps %xmm0,0x30(%rsp)
  1de113:	0f 28 84 24 10 02 00 	movaps 0x210(%rsp),%xmm0
  1de11a:	00
  1de11b:	0f 29 44 24 20       	movaps %xmm0,0x20(%rsp)
  1de120:	4c 89 ff             	mov    %r15,%rdi
  1de123:	4c 89 ee             	mov    %r13,%rsi
  1de126:	48 89 da             	mov    %rbx,%rdx
  1de129:	e8 b2 71 00 00       	call   1e52e0 <core::core_arch::x86::avx::_mm256_mul_ps>
  1de12e:	0f 28 44 24 60       	movaps 0x60(%rsp),%xmm0
  1de133:	0f 29 84 24 f0 01 00 	movaps %xmm0,0x1f0(%rsp)
  1de13a:	00
  1de13b:	0f 28 44 24 70       	movaps 0x70(%rsp),%xmm0
  1de140:	0f 29 84 24 e0 01 00 	movaps %xmm0,0x1e0(%rsp)
  1de147:	00
  1de148:	0f 28 84 24 d0 00 00 	movaps 0xd0(%rsp),%xmm0
  1de14f:	00
  1de150:	0f 29 44 24 70       	movaps %xmm0,0x70(%rsp)
  1de155:	0f 28 84 24 e0 00 00 	movaps 0xe0(%rsp),%xmm0
  1de15c:	00
  1de15d:	0f 29 44 24 60       	movaps %xmm0,0x60(%rsp)
  1de162:	0f 28 84 24 50 01 00 	movaps 0x150(%rsp),%xmm0
  1de169:	00
  1de16a:	0f 29 44 24 30       	movaps %xmm0,0x30(%rsp)
  1de16f:	0f 28 84 24 60 01 00 	movaps 0x160(%rsp),%xmm0
  1de176:	00
  1de177:	0f 29 44 24 20       	movaps %xmm0,0x20(%rsp)
  1de17c:	4c 89 ef             	mov    %r13,%rdi
  1de17f:	4c 89 fe             	mov    %r15,%rsi
  1de182:	48 89 da             	mov    %rbx,%rdx
  1de185:	e8 36 3b 00 00       	call   1e1cc0 <core::core_arch::x86::avx::_mm256_cmp_ps::<0>>
  1de18a:	48 89 df             	mov    %rbx,%rdi
  1de18d:	be ff ff ff ff       	mov    $0xffffffff,%esi
  1de192:	e8 89 71 00 00       	call   1e5320 <core::core_arch::x86::avx::_mm256_set1_epi32>
  1de197:	4c 89 ff             	mov    %r15,%rdi
  1de19a:	4c 89 ee             	mov    %r13,%rsi
  1de19d:	48 89 da             	mov    %rbx,%rdx
  1de1a0:	e8 bb 71 00 00       	call   1e5360 <core::core_arch::x86::avx2::_mm256_xor_si256>
  1de1a5:	0f 28 84 24 c0 01 00 	movaps 0x1c0(%rsp),%xmm0
  1de1ac:	00
  1de1ad:	0f 29 84 24 10 01 00 	movaps %xmm0,0x110(%rsp)
  1de1b4:	00
  1de1b5:	0f 28 84 24 d0 01 00 	movaps 0x1d0(%rsp),%xmm0
  1de1bc:	00
  1de1bd:	0f 29 84 24 00 01 00 	movaps %xmm0,0x100(%rsp)
  1de1c4:	00
  1de1c5:	0f 28 84 24 50 01 00 	movaps 0x150(%rsp),%xmm0
  1de1cc:	00
  1de1cd:	0f 29 84 24 90 00 00 	movaps %xmm0,0x90(%rsp)
  1de1d4:	00
  1de1d5:	0f 28 84 24 60 01 00 	movaps 0x160(%rsp),%xmm0
  1de1dc:	00
  1de1dd:	0f 29 84 24 80 00 00 	movaps %xmm0,0x80(%rsp)
  1de1e4:	00
  1de1e5:	48 89 df             	mov    %rbx,%rdi
  1de1e8:	48 8d b4 24 00 01 00 	lea    0x100(%rsp),%rsi
  1de1ef:	00
  1de1f0:	4c 89 ea             	mov    %r13,%rdx
  1de1f3:	e8 c8 3a 00 00       	call   1e1cc0 <core::core_arch::x86::avx::_mm256_cmp_ps::<0>>
  1de1f8:	48 8d bc 24 40 02 00 	lea    0x240(%rsp),%rdi
  1de1ff:	00
  1de200:	4c 89 fe             	mov    %r15,%rsi
  1de203:	48 89 da             	mov    %rbx,%rdx
  1de206:	e8 45 71 00 00       	call   1e5350 <core::core_arch::x86::avx2::_mm256_and_si256>
  1de20b:	0f 28 84 24 b0 00 00 	movaps 0xb0(%rsp),%xmm0
  1de212:	00
  1de213:	0f 29 44 24 70       	movaps %xmm0,0x70(%rsp)
  1de218:	0f 28 84 24 c0 00 00 	movaps 0xc0(%rsp),%xmm0
  1de21f:	00
  1de220:	0f 29 44 24 60       	movaps %xmm0,0x60(%rsp)
  1de225:	0f 28 84 24 50 01 00 	movaps 0x150(%rsp),%xmm0
  1de22c:	00
  1de22d:	0f 29 44 24 30       	movaps %xmm0,0x30(%rsp)
  1de232:	0f 28 84 24 60 01 00 	movaps 0x160(%rsp),%xmm0
  1de239:	00
  1de23a:	0f 29 44 24 20       	movaps %xmm0,0x20(%rsp)
  1de23f:	4c 89 ef             	mov    %r13,%rdi
  1de242:	4c 89 fe             	mov    %r15,%rsi
  1de245:	48 89 da             	mov    %rbx,%rdx
  1de248:	e8 73 3a 00 00       	call   1e1cc0 <core::core_arch::x86::avx::_mm256_cmp_ps::<0>>
  1de24d:	48 89 df             	mov    %rbx,%rdi
  1de250:	be ff ff ff ff       	mov    $0xffffffff,%esi
  1de255:	e8 c6 70 00 00       	call   1e5320 <core::core_arch::x86::avx::_mm256_set1_epi32>
  1de25a:	4c 89 ff             	mov    %r15,%rdi
  1de25d:	4c 89 ee             	mov    %r13,%rsi
  1de260:	48 89 da             	mov    %rbx,%rdx
  1de263:	e8 f8 70 00 00       	call   1e5360 <core::core_arch::x86::avx2::_mm256_xor_si256>
  1de268:	0f 28 84 24 e0 01 00 	movaps 0x1e0(%rsp),%xmm0
  1de26f:	00
  1de270:	0f 29 84 24 10 01 00 	movaps %xmm0,0x110(%rsp)
  1de277:	00
  1de278:	0f 28 84 24 f0 01 00 	movaps 0x1f0(%rsp),%xmm0
  1de27f:	00
  1de280:	0f 29 84 24 00 01 00 	movaps %xmm0,0x100(%rsp)
  1de287:	00
  1de288:	0f 28 84 24 50 01 00 	movaps 0x150(%rsp),%xmm0
  1de28f:	00
  1de290:	0f 29 84 24 90 00 00 	movaps %xmm0,0x90(%rsp)
  1de297:	00
  1de298:	0f 28 84 24 60 01 00 	movaps 0x160(%rsp),%xmm0
  1de29f:	00
  1de2a0:	0f 29 84 24 80 00 00 	movaps %xmm0,0x80(%rsp)
  1de2a7:	00
  1de2a8:	48 89 df             	mov    %rbx,%rdi
  1de2ab:	4c 8d b4 24 00 01 00 	lea    0x100(%rsp),%r14
  1de2b2:	00
  1de2b3:	4c 89 f6             	mov    %r14,%rsi
  1de2b6:	4c 89 ea             	mov    %r13,%rdx
  1de2b9:	e8 02 3a 00 00       	call   1e1cc0 <core::core_arch::x86::avx::_mm256_cmp_ps::<0>>
  1de2be:	4c 89 f7             	mov    %r14,%rdi
  1de2c1:	4c 89 fe             	mov    %r15,%rsi
  1de2c4:	48 89 da             	mov    %rbx,%rdx
  1de2c7:	e8 84 70 00 00       	call   1e5350 <core::core_arch::x86::avx2::_mm256_and_si256>
  1de2cc:	4c 89 ef             	mov    %r13,%rdi
  1de2cf:	48 8d b4 24 40 02 00 	lea    0x240(%rsp),%rsi
  1de2d6:	00
  1de2d7:	4c 89 f2             	mov    %r14,%rdx
  1de2da:	e8 61 70 00 00       	call   1e5340 <core::core_arch::x86::avx2::_mm256_or_si256>
  1de2df:	48 89 df             	mov    %rbx,%rdi
  1de2e2:	be ff ff ff ff       	mov    $0xffffffff,%esi
  1de2e7:	e8 34 70 00 00       	call   1e5320 <core::core_arch::x86::avx::_mm256_set1_epi32>
  1de2ec:	4c 89 ff             	mov    %r15,%rdi
  1de2ef:	4c 89 ee             	mov    %r13,%rsi
  1de2f2:	48 89 da             	mov    %rbx,%rdx
  1de2f5:	e8 66 70 00 00       	call   1e5360 <core::core_arch::x86::avx2::_mm256_xor_si256>
  1de2fa:	0f 28 84 24 30 01 00 	movaps 0x130(%rsp),%xmm0
  1de301:	00
  1de302:	0f 29 44 24 30       	movaps %xmm0,0x30(%rsp)
  1de307:	0f 28 84 24 40 01 00 	movaps 0x140(%rsp),%xmm0
  1de30e:	00
  1de30f:	0f 29 44 24 20       	movaps %xmm0,0x20(%rsp)
  1de314:	4c 89 ef             	mov    %r13,%rdi
  1de317:	48 89 de             	mov    %rbx,%rsi
  1de31a:	4c 89 fa             	mov    %r15,%rdx
  1de31d:	e8 2e 70 00 00       	call   1e5350 <core::core_arch::x86::avx2::_mm256_and_si256>
  1de322:	66 0f 6f 8c 24 80 00 	movdqa 0x80(%rsp),%xmm1
  1de329:	00 00
  1de32b:	66 0f 7f 8c 24 40 01 	movdqa %xmm1,0x140(%rsp)
  1de332:	00 00
  1de334:	66 0f 6f 94 24 90 00 	movdqa 0x90(%rsp),%xmm2
  1de33b:	00 00
  1de33d:	66 0f 7f 94 24 30 01 	movdqa %xmm2,0x130(%rsp)
  1de344:	00 00
  1de346:	0f 28 84 24 d0 00 00 	movaps 0xd0(%rsp),%xmm0
  1de34d:	00
  1de34e:	0f 29 44 24 70       	movaps %xmm0,0x70(%rsp)
  1de353:	0f 28 84 24 e0 00 00 	movaps 0xe0(%rsp),%xmm0
  1de35a:	00
  1de35b:	0f 29 44 24 60       	movaps %xmm0,0x60(%rsp)
  1de360:	0f 28 84 24 c0 01 00 	movaps 0x1c0(%rsp),%xmm0
  1de367:	00
  1de368:	0f 29 84 24 90 00 00 	movaps %xmm0,0x90(%rsp)
  1de36f:	00
  1de370:	0f 28 84 24 d0 01 00 	movaps 0x1d0(%rsp),%xmm0
  1de377:	00
  1de378:	0f 29 84 24 80 00 00 	movaps %xmm0,0x80(%rsp)
  1de37f:	00
  1de380:	66 0f 7f 54 24 30    	movdqa %xmm2,0x30(%rsp)
  1de386:	66 0f 7f 4c 24 20    	movdqa %xmm1,0x20(%rsp)
  1de38c:	4c 89 f7             	mov    %r14,%rdi
  1de38f:	4c 89 fe             	mov    %r15,%rsi
  1de392:	4c 89 ea             	mov    %r13,%rdx
  1de395:	48 89 d9             	mov    %rbx,%rcx
  1de398:	e8 63 6f 00 00       	call   1e5300 <core::core_arch::x86::avx::_mm256_blendv_ps>
  1de39d:	0f 28 84 24 00 01 00 	movaps 0x100(%rsp),%xmm0
  1de3a4:	00
  1de3a5:	0f 29 84 24 e0 00 00 	movaps %xmm0,0xe0(%rsp)
  1de3ac:	00
  1de3ad:	0f 28 84 24 10 01 00 	movaps 0x110(%rsp),%xmm0
  1de3b4:	00
  1de3b5:	0f 29 84 24 d0 00 00 	movaps %xmm0,0xd0(%rsp)
  1de3bc:	00
  1de3bd:	0f 28 84 24 b0 00 00 	movaps 0xb0(%rsp),%xmm0
  1de3c4:	00
  1de3c5:	0f 29 44 24 70       	movaps %xmm0,0x70(%rsp)
  1de3ca:	0f 28 84 24 c0 00 00 	movaps 0xc0(%rsp),%xmm0
  1de3d1:	00
  1de3d2:	0f 29 44 24 60       	movaps %xmm0,0x60(%rsp)
  1de3d7:	0f 28 84 24 e0 01 00 	movaps 0x1e0(%rsp),%xmm0
  1de3de:	00
  1de3df:	0f 29 84 24 90 00 00 	movaps %xmm0,0x90(%rsp)
  1de3e6:	00
  1de3e7:	0f 28 84 24 f0 01 00 	movaps 0x1f0(%rsp),%xmm0
  1de3ee:	00
  1de3ef:	0f 29 84 24 80 00 00 	movaps %xmm0,0x80(%rsp)
  1de3f6:	00
  1de3f7:	0f 28 84 24 30 01 00 	movaps 0x130(%rsp),%xmm0
  1de3fe:	00
  1de3ff:	0f 29 44 24 30       	movaps %xmm0,0x30(%rsp)
  1de404:	0f 28 84 24 40 01 00 	movaps 0x140(%rsp),%xmm0
  1de40b:	00
  1de40c:	0f 29 44 24 20       	movaps %xmm0,0x20(%rsp)
  1de411:	4c 89 f7             	mov    %r14,%rdi
  1de414:	4c 89 fe             	mov    %r15,%rsi
  1de417:	4c 89 ea             	mov    %r13,%rdx
  1de41a:	48 89 d9             	mov    %rbx,%rcx
  1de41d:	e8 de 6e 00 00       	call   1e5300 <core::core_arch::x86::avx::_mm256_blendv_ps>
  1de422:	0f 28 84 24 00 01 00 	movaps 0x100(%rsp),%xmm0
  1de429:	00
  1de42a:	0f 29 84 24 c0 00 00 	movaps %xmm0,0xc0(%rsp)
  1de431:	00
  1de432:	0f 28 84 24 10 01 00 	movaps 0x110(%rsp),%xmm0
  1de439:	00
  1de43a:	0f 29 84 24 b0 00 00 	movaps %xmm0,0xb0(%rsp)
  1de441:	00
  1de442:	4c 89 ef             	mov    %r13,%rdi
  1de445:	be 01 00 00 00       	mov    $0x1,%esi
  1de44a:	e8 d1 6e 00 00       	call   1e5320 <core::core_arch::x86::avx::_mm256_set1_epi32>
  1de44f:	4c 89 ff             	mov    %r15,%rdi
  1de452:	31 f6                	xor    %esi,%esi
  1de454:	e8 c7 6e 00 00       	call   1e5320 <core::core_arch::x86::avx::_mm256_set1_epi32>
  1de459:	0f 28 84 24 30 01 00 	movaps 0x130(%rsp),%xmm0
  1de460:	00
  1de461:	0f 29 44 24 30       	movaps %xmm0,0x30(%rsp)
  1de466:	0f 28 84 24 40 01 00 	movaps 0x140(%rsp),%xmm0
  1de46d:	00
  1de46e:	0f 29 44 24 20       	movaps %xmm0,0x20(%rsp)
  1de473:	48 8d bc 24 80 01 00 	lea    0x180(%rsp),%rdi
  1de47a:	00
  1de47b:	4c 89 fe             	mov    %r15,%rsi
  1de47e:	4c 89 ea             	mov    %r13,%rdx
  1de481:	48 89 d9             	mov    %rbx,%rcx
  1de484:	e8 77 6e 00 00       	call   1e5300 <core::core_arch::x86::avx::_mm256_blendv_ps>
  1de489:	44 8b 94 24 80 01 00 	mov    0x180(%rsp),%r10d
  1de490:	00
  1de491:	45 85 d2             	test   %r10d,%r10d
  1de494:	4c 8b 9c 24 a8 00 00 	mov    0xa8(%rsp),%r11
  1de49b:	00
  1de49c:	48 ba ff ff ff ff ff 	movabs $0x7fffffffffffffff,%rdx
  1de4a3:	ff ff 7f
  1de4a6:	74 0a                	je     1de4b2 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x6b2>
  1de4a8:	4b 39 14 e3          	cmp    %rdx,(%r11,%r12,8)
  1de4ac:	0f 84 14 06 00 00    	je     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de4b2:	44 8b 8c 24 84 01 00 	mov    0x184(%rsp),%r9d
  1de4b9:	00
  1de4ba:	45 85 c9             	test   %r9d,%r9d
  1de4bd:	74 0b                	je     1de4ca <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x6ca>
  1de4bf:	4b 39 54 e3 08       	cmp    %rdx,0x8(%r11,%r12,8)
  1de4c4:	0f 84 fc 05 00 00    	je     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de4ca:	44 8b 84 24 88 01 00 	mov    0x188(%rsp),%r8d
  1de4d1:	00
  1de4d2:	45 85 c0             	test   %r8d,%r8d
  1de4d5:	f3 0f 10 2d 83 05 e4 	movss  -0x1bfa7d(%rip),%xmm5        # 1ea60 <__abi_tag+0x1e764>
  1de4dc:	ff
  1de4dd:	74 0b                	je     1de4ea <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x6ea>
  1de4df:	4b 39 54 e3 10       	cmp    %rdx,0x10(%r11,%r12,8)
  1de4e4:	0f 84 dc 05 00 00    	je     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de4ea:	8b bc 24 8c 01 00 00 	mov    0x18c(%rsp),%edi
  1de4f1:	85 ff                	test   %edi,%edi
  1de4f3:	74 0b                	je     1de500 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x700>
  1de4f5:	4b 39 54 e3 18       	cmp    %rdx,0x18(%r11,%r12,8)
  1de4fa:	0f 84 c6 05 00 00    	je     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de500:	8b b4 24 90 01 00 00 	mov    0x190(%rsp),%esi
  1de507:	85 f6                	test   %esi,%esi
  1de509:	74 0b                	je     1de516 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x716>
  1de50b:	4b 39 54 e3 20       	cmp    %rdx,0x20(%r11,%r12,8)
  1de510:	0f 84 b0 05 00 00    	je     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de516:	44 8b b4 24 94 01 00 	mov    0x194(%rsp),%r14d
  1de51d:	00
  1de51e:	45 85 f6             	test   %r14d,%r14d
  1de521:	74 0b                	je     1de52e <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x72e>
  1de523:	4b 39 54 e3 28       	cmp    %rdx,0x28(%r11,%r12,8)
  1de528:	0f 84 98 05 00 00    	je     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de52e:	8b 8c 24 98 01 00 00 	mov    0x198(%rsp),%ecx
  1de535:	85 c9                	test   %ecx,%ecx
  1de537:	74 0b                	je     1de544 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x744>
  1de539:	4b 39 54 e3 30       	cmp    %rdx,0x30(%r11,%r12,8)
  1de53e:	0f 84 82 05 00 00    	je     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de544:	8b 84 24 9c 01 00 00 	mov    0x19c(%rsp),%eax
  1de54b:	85 c0                	test   %eax,%eax
  1de54d:	74 0b                	je     1de55a <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x75a>
  1de54f:	4b 39 54 e3 38       	cmp    %rdx,0x38(%r11,%r12,8)
  1de554:	0f 84 6c 05 00 00    	je     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de55a:	45 85 d2             	test   %r10d,%r10d
  1de55d:	74 0b                	je     1de56a <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x76a>
  1de55f:	4b ff 04 e3          	incq   (%r11,%r12,8)
  1de563:	b2 01                	mov    $0x1,%dl
  1de565:	48 89 54 24 18       	mov    %rdx,0x18(%rsp)
  1de56a:	45 85 c9             	test   %r9d,%r9d
  1de56d:	48 8b 54 24 58       	mov    0x58(%rsp),%rdx
  1de572:	74 0d                	je     1de581 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x781>
  1de574:	4b ff 44 e3 08       	incq   0x8(%r11,%r12,8)
  1de579:	41 b1 01             	mov    $0x1,%r9b
  1de57c:	4c 89 4c 24 18       	mov    %r9,0x18(%rsp)
  1de581:	45 85 c0             	test   %r8d,%r8d
  1de584:	74 0d                	je     1de593 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x793>
  1de586:	4b ff 44 e3 10       	incq   0x10(%r11,%r12,8)
  1de58b:	41 b0 01             	mov    $0x1,%r8b
  1de58e:	4c 89 44 24 18       	mov    %r8,0x18(%rsp)
  1de593:	85 ff                	test   %edi,%edi
  1de595:	74 0d                	je     1de5a4 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x7a4>
  1de597:	4b ff 44 e3 18       	incq   0x18(%r11,%r12,8)
  1de59c:	40 b7 01             	mov    $0x1,%dil
  1de59f:	48 89 7c 24 18       	mov    %rdi,0x18(%rsp)
  1de5a4:	85 f6                	test   %esi,%esi
  1de5a6:	74 0d                	je     1de5b5 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x7b5>
  1de5a8:	4b ff 44 e3 20       	incq   0x20(%r11,%r12,8)
  1de5ad:	40 b6 01             	mov    $0x1,%sil
  1de5b0:	48 89 74 24 18       	mov    %rsi,0x18(%rsp)
  1de5b5:	45 85 f6             	test   %r14d,%r14d
  1de5b8:	74 0d                	je     1de5c7 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x7c7>
  1de5ba:	4b ff 44 e3 28       	incq   0x28(%r11,%r12,8)
  1de5bf:	40 b6 01             	mov    $0x1,%sil
  1de5c2:	48 89 74 24 18       	mov    %rsi,0x18(%rsp)
  1de5c7:	85 c9                	test   %ecx,%ecx
  1de5c9:	74 0c                	je     1de5d7 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x7d7>
  1de5cb:	4b ff 44 e3 30       	incq   0x30(%r11,%r12,8)
  1de5d0:	b1 01                	mov    $0x1,%cl
  1de5d2:	48 89 4c 24 18       	mov    %rcx,0x18(%rsp)
  1de5d7:	85 c0                	test   %eax,%eax
  1de5d9:	74 0c                	je     1de5e7 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x7e7>
  1de5db:	4b ff 44 e3 38       	incq   0x38(%r11,%r12,8)
  1de5e0:	b0 01                	mov    $0x1,%al
  1de5e2:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
  1de5e7:	48 8b 44 24 08       	mov    0x8(%rsp),%rax
  1de5ec:	0f 28 84 24 e0 00 00 	movaps 0xe0(%rsp),%xmm0
  1de5f3:	00
  1de5f4:	42 0f 11 04 a0       	movups %xmm0,(%rax,%r12,4)
  1de5f9:	0f 28 84 24 d0 00 00 	movaps 0xd0(%rsp),%xmm0
  1de600:	00
  1de601:	42 0f 11 44 a0 10    	movups %xmm0,0x10(%rax,%r12,4)
  1de607:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
  1de60c:	0f 28 84 24 b0 00 00 	movaps 0xb0(%rsp),%xmm0
  1de613:	00
  1de614:	42 0f 11 44 a0 10    	movups %xmm0,0x10(%rax,%r12,4)
  1de61a:	0f 28 84 24 c0 00 00 	movaps 0xc0(%rsp),%xmm0
  1de621:	00
  1de622:	42 0f 11 04 a0       	movups %xmm0,(%rax,%r12,4)
  1de627:	4c 8b bc 24 78 01 00 	mov    0x178(%rsp),%r15
  1de62e:	00
  1de62f:	48 8b 8c 24 f8 00 00 	mov    0xf8(%rsp),%rcx
  1de636:	00
  1de637:	4c 39 f9             	cmp    %r15,%rcx
  1de63a:	4c 0f 47 f9          	cmova  %rcx,%r15
  1de63e:	0f 82 cd f8 ff ff    	jb     1ddf11 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x111>
  1de644:	eb 16                	jmp    1de65c <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x85c>
  1de646:	48 c7 44 24 18 00 00 	movq   $0x0,0x18(%rsp)
  1de64d:	00 00
  1de64f:	48 8b 54 24 58       	mov    0x58(%rsp),%rdx
  1de654:	f3 0f 10 2d 04 04 e4 	movss  -0x1bfbfc(%rip),%xmm5        # 1ea60 <__abi_tag+0x1e764>
  1de65b:	ff
  1de65c:	89 d0                	mov    %edx,%eax
  1de65e:	83 e0 07             	and    $0x7,%eax
  1de661:	0f 84 ae 00 00 00    	je     1de715 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x915>
  1de667:	48 bf f8 ff ff ff ff 	movabs $0x1ffffffffffffff8,%rdi
  1de66e:	ff ff 1f
  1de671:	48 21 d7             	and    %rdx,%rdi
  1de674:	48 8b 4c 24 10       	mov    0x10(%rsp),%rcx
  1de679:	f3 0f 10 04 b9       	movss  (%rcx,%rdi,4),%xmm0
  1de67e:	0f 28 0d 9b 24 e4 ff 	movaps -0x1bdb65(%rip),%xmm1        # 20b20 <anon.44166ce10ff660448c7f2cfe9904e0b0.0.llvm.3986032740938256925+0xbe0>
  1de685:	0f 54 c8             	andps  %xmm0,%xmm1
  1de688:	0f 2e 0d 39 03 e4 ff 	ucomiss -0x1bfcc7(%rip),%xmm1        # 1e9c8 <__abi_tag+0x1e6cc>
  1de68f:	0f 86 8a 00 00 00    	jbe    1de71f <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x91f>
  1de695:	48 39 d7             	cmp    %rdx,%rdi
  1de698:	4c 8b 84 24 a8 00 00 	mov    0xa8(%rsp),%r8
  1de69f:	00
  1de6a0:	4c 8b 54 24 18       	mov    0x18(%rsp),%r10
  1de6a5:	0f 84 35 04 00 00    	je     1deae0 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xce0>
  1de6ab:	49 8b 0c f8          	mov    (%r8,%rdi,8),%rcx
  1de6af:	48 ff c1             	inc    %rcx
  1de6b2:	0f 80 0e 04 00 00    	jo     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de6b8:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de6bd:	f3 0f 10 1c be       	movss  (%rsi,%rdi,4),%xmm3
  1de6c2:	0f 28 cb             	movaps %xmm3,%xmm1
  1de6c5:	f3 0f 59 cd          	mulss  %xmm5,%xmm1
  1de6c9:	66 0f ef d2          	pxor   %xmm2,%xmm2
  1de6cd:	f3 0f c2 da 04       	cmpneqss %xmm2,%xmm3
  1de6d2:	0f 28 e1             	movaps %xmm1,%xmm4
  1de6d5:	f3 0f c2 e2 00       	cmpeqss %xmm2,%xmm4
  1de6da:	0f 54 e3             	andps  %xmm3,%xmm4
  1de6dd:	66 0f 7e e6          	movd   %xmm4,%esi
  1de6e1:	40 f6 c6 01          	test   $0x1,%sil
  1de6e5:	75 24                	jne    1de70b <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x90b>
  1de6e7:	f3 0f 59 c5          	mulss  %xmm5,%xmm0
  1de6eb:	0f 2e c2             	ucomiss %xmm2,%xmm0
  1de6ee:	74 1b                	je     1de70b <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x90b>
  1de6f0:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de6f5:	f3 0f 11 0c be       	movss  %xmm1,(%rsi,%rdi,4)
  1de6fa:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
  1de6ff:	f3 0f 11 04 be       	movss  %xmm0,(%rsi,%rdi,4)
  1de704:	49 89 0c f8          	mov    %rcx,(%r8,%rdi,8)
  1de708:	41 b2 01             	mov    $0x1,%r10b
  1de70b:	83 f8 01             	cmp    $0x1,%eax
  1de70e:	75 25                	jne    1de735 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x935>
  1de710:	e9 b4 03 00 00       	jmp    1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1de715:	4c 8b 54 24 18       	mov    0x18(%rsp),%r10
  1de71a:	e9 aa 03 00 00       	jmp    1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1de71f:	4c 8b 84 24 a8 00 00 	mov    0xa8(%rsp),%r8
  1de726:	00
  1de727:	4c 8b 54 24 18       	mov    0x18(%rsp),%r10
  1de72c:	83 f8 01             	cmp    $0x1,%eax
  1de72f:	0f 84 94 03 00 00    	je     1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1de735:	48 8b 4c 24 10       	mov    0x10(%rsp),%rcx
  1de73a:	f3 0f 10 44 b9 04    	movss  0x4(%rcx,%rdi,4),%xmm0
  1de740:	0f 28 0d d9 23 e4 ff 	movaps -0x1bdc27(%rip),%xmm1        # 20b20 <anon.44166ce10ff660448c7f2cfe9904e0b0.0.llvm.3986032740938256925+0xbe0>
  1de747:	0f 54 c8             	andps  %xmm0,%xmm1
  1de74a:	0f 2e 0d 77 02 e4 ff 	ucomiss -0x1bfd89(%rip),%xmm1        # 1e9c8 <__abi_tag+0x1e6cc>
  1de751:	76 6f                	jbe    1de7c2 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x9c2>
  1de753:	48 8d 4f 01          	lea    0x1(%rdi),%rcx
  1de757:	48 39 d1             	cmp    %rdx,%rcx
  1de75a:	0f 83 7d 03 00 00    	jae    1deadd <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcdd>
  1de760:	4d 8b 0c c8          	mov    (%r8,%rcx,8),%r9
  1de764:	49 ff c1             	inc    %r9
  1de767:	0f 80 59 03 00 00    	jo     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de76d:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de772:	f3 0f 10 5c be 04    	movss  0x4(%rsi,%rdi,4),%xmm3
  1de778:	0f 28 cb             	movaps %xmm3,%xmm1
  1de77b:	f3 0f 59 cd          	mulss  %xmm5,%xmm1
  1de77f:	0f 57 d2             	xorps  %xmm2,%xmm2
  1de782:	f3 0f c2 da 04       	cmpneqss %xmm2,%xmm3
  1de787:	0f 28 e1             	movaps %xmm1,%xmm4
  1de78a:	f3 0f c2 e2 00       	cmpeqss %xmm2,%xmm4
  1de78f:	0f 54 e3             	andps  %xmm3,%xmm4
  1de792:	66 0f 7e e6          	movd   %xmm4,%esi
  1de796:	40 f6 c6 01          	test   $0x1,%sil
  1de79a:	75 26                	jne    1de7c2 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x9c2>
  1de79c:	f3 0f 59 c5          	mulss  %xmm5,%xmm0
  1de7a0:	0f 2e c2             	ucomiss %xmm2,%xmm0
  1de7a3:	74 1d                	je     1de7c2 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0x9c2>
  1de7a5:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de7aa:	f3 0f 11 4c be 04    	movss  %xmm1,0x4(%rsi,%rdi,4)
  1de7b0:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
  1de7b5:	f3 0f 11 44 be 04    	movss  %xmm0,0x4(%rsi,%rdi,4)
  1de7bb:	4d 89 0c c8          	mov    %r9,(%r8,%rcx,8)
  1de7bf:	41 b2 01             	mov    $0x1,%r10b
  1de7c2:	83 f8 02             	cmp    $0x2,%eax
  1de7c5:	0f 84 fe 02 00 00    	je     1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1de7cb:	48 8b 4c 24 10       	mov    0x10(%rsp),%rcx
  1de7d0:	f3 0f 10 44 b9 08    	movss  0x8(%rcx,%rdi,4),%xmm0
  1de7d6:	0f 28 0d 43 23 e4 ff 	movaps -0x1bdcbd(%rip),%xmm1        # 20b20 <anon.44166ce10ff660448c7f2cfe9904e0b0.0.llvm.3986032740938256925+0xbe0>
  1de7dd:	0f 54 c8             	andps  %xmm0,%xmm1
  1de7e0:	0f 2e 0d e1 01 e4 ff 	ucomiss -0x1bfe1f(%rip),%xmm1        # 1e9c8 <__abi_tag+0x1e6cc>
  1de7e7:	76 6f                	jbe    1de858 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xa58>
  1de7e9:	48 8d 4f 02          	lea    0x2(%rdi),%rcx
  1de7ed:	48 39 d1             	cmp    %rdx,%rcx
  1de7f0:	0f 83 e7 02 00 00    	jae    1deadd <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcdd>
  1de7f6:	4d 8b 0c c8          	mov    (%r8,%rcx,8),%r9
  1de7fa:	49 ff c1             	inc    %r9
  1de7fd:	0f 80 c3 02 00 00    	jo     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de803:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de808:	f3 0f 10 5c be 08    	movss  0x8(%rsi,%rdi,4),%xmm3
  1de80e:	0f 28 cb             	movaps %xmm3,%xmm1
  1de811:	f3 0f 59 cd          	mulss  %xmm5,%xmm1
  1de815:	0f 57 d2             	xorps  %xmm2,%xmm2
  1de818:	f3 0f c2 da 04       	cmpneqss %xmm2,%xmm3
  1de81d:	0f 28 e1             	movaps %xmm1,%xmm4
  1de820:	f3 0f c2 e2 00       	cmpeqss %xmm2,%xmm4
  1de825:	0f 54 e3             	andps  %xmm3,%xmm4
  1de828:	66 0f 7e e6          	movd   %xmm4,%esi
  1de82c:	40 f6 c6 01          	test   $0x1,%sil
  1de830:	75 26                	jne    1de858 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xa58>
  1de832:	f3 0f 59 c5          	mulss  %xmm5,%xmm0
  1de836:	0f 2e c2             	ucomiss %xmm2,%xmm0
  1de839:	74 1d                	je     1de858 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xa58>
  1de83b:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de840:	f3 0f 11 4c be 08    	movss  %xmm1,0x8(%rsi,%rdi,4)
  1de846:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
  1de84b:	f3 0f 11 44 be 08    	movss  %xmm0,0x8(%rsi,%rdi,4)
  1de851:	4d 89 0c c8          	mov    %r9,(%r8,%rcx,8)
  1de855:	41 b2 01             	mov    $0x1,%r10b
  1de858:	83 f8 03             	cmp    $0x3,%eax
  1de85b:	0f 84 68 02 00 00    	je     1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1de861:	48 8b 4c 24 10       	mov    0x10(%rsp),%rcx
  1de866:	f3 0f 10 44 b9 0c    	movss  0xc(%rcx,%rdi,4),%xmm0
  1de86c:	0f 28 0d ad 22 e4 ff 	movaps -0x1bdd53(%rip),%xmm1        # 20b20 <anon.44166ce10ff660448c7f2cfe9904e0b0.0.llvm.3986032740938256925+0xbe0>
  1de873:	0f 54 c8             	andps  %xmm0,%xmm1
  1de876:	0f 2e 0d 4b 01 e4 ff 	ucomiss -0x1bfeb5(%rip),%xmm1        # 1e9c8 <__abi_tag+0x1e6cc>
  1de87d:	76 6f                	jbe    1de8ee <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xaee>
  1de87f:	48 8d 4f 03          	lea    0x3(%rdi),%rcx
  1de883:	48 39 d1             	cmp    %rdx,%rcx
  1de886:	0f 83 51 02 00 00    	jae    1deadd <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcdd>
  1de88c:	4d 8b 0c c8          	mov    (%r8,%rcx,8),%r9
  1de890:	49 ff c1             	inc    %r9
  1de893:	0f 80 2d 02 00 00    	jo     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de899:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de89e:	f3 0f 10 5c be 0c    	movss  0xc(%rsi,%rdi,4),%xmm3
  1de8a4:	0f 28 cb             	movaps %xmm3,%xmm1
  1de8a7:	f3 0f 59 cd          	mulss  %xmm5,%xmm1
  1de8ab:	0f 57 d2             	xorps  %xmm2,%xmm2
  1de8ae:	f3 0f c2 da 04       	cmpneqss %xmm2,%xmm3
  1de8b3:	0f 28 e1             	movaps %xmm1,%xmm4
  1de8b6:	f3 0f c2 e2 00       	cmpeqss %xmm2,%xmm4
  1de8bb:	0f 54 e3             	andps  %xmm3,%xmm4
  1de8be:	66 0f 7e e6          	movd   %xmm4,%esi
  1de8c2:	40 f6 c6 01          	test   $0x1,%sil
  1de8c6:	75 26                	jne    1de8ee <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xaee>
  1de8c8:	f3 0f 59 c5          	mulss  %xmm5,%xmm0
  1de8cc:	0f 2e c2             	ucomiss %xmm2,%xmm0
  1de8cf:	74 1d                	je     1de8ee <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xaee>
  1de8d1:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de8d6:	f3 0f 11 4c be 0c    	movss  %xmm1,0xc(%rsi,%rdi,4)
  1de8dc:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
  1de8e1:	f3 0f 11 44 be 0c    	movss  %xmm0,0xc(%rsi,%rdi,4)
  1de8e7:	4d 89 0c c8          	mov    %r9,(%r8,%rcx,8)
  1de8eb:	41 b2 01             	mov    $0x1,%r10b
  1de8ee:	83 f8 04             	cmp    $0x4,%eax
  1de8f1:	0f 84 d2 01 00 00    	je     1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1de8f7:	48 8b 4c 24 10       	mov    0x10(%rsp),%rcx
  1de8fc:	f3 0f 10 44 b9 10    	movss  0x10(%rcx,%rdi,4),%xmm0
  1de902:	0f 28 0d 17 22 e4 ff 	movaps -0x1bdde9(%rip),%xmm1        # 20b20 <anon.44166ce10ff660448c7f2cfe9904e0b0.0.llvm.3986032740938256925+0xbe0>
  1de909:	0f 54 c8             	andps  %xmm0,%xmm1
  1de90c:	0f 2e 0d b5 00 e4 ff 	ucomiss -0x1bff4b(%rip),%xmm1        # 1e9c8 <__abi_tag+0x1e6cc>
  1de913:	76 6f                	jbe    1de984 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xb84>
  1de915:	48 8d 4f 04          	lea    0x4(%rdi),%rcx
  1de919:	48 39 d1             	cmp    %rdx,%rcx
  1de91c:	0f 83 bb 01 00 00    	jae    1deadd <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcdd>
  1de922:	4d 8b 0c c8          	mov    (%r8,%rcx,8),%r9
  1de926:	49 ff c1             	inc    %r9
  1de929:	0f 80 97 01 00 00    	jo     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de92f:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de934:	f3 0f 10 5c be 10    	movss  0x10(%rsi,%rdi,4),%xmm3
  1de93a:	0f 28 cb             	movaps %xmm3,%xmm1
  1de93d:	f3 0f 59 cd          	mulss  %xmm5,%xmm1
  1de941:	0f 57 d2             	xorps  %xmm2,%xmm2
  1de944:	f3 0f c2 da 04       	cmpneqss %xmm2,%xmm3
  1de949:	0f 28 e1             	movaps %xmm1,%xmm4
  1de94c:	f3 0f c2 e2 00       	cmpeqss %xmm2,%xmm4
  1de951:	0f 54 e3             	andps  %xmm3,%xmm4
  1de954:	66 0f 7e e6          	movd   %xmm4,%esi
  1de958:	40 f6 c6 01          	test   $0x1,%sil
  1de95c:	75 26                	jne    1de984 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xb84>
  1de95e:	f3 0f 59 c5          	mulss  %xmm5,%xmm0
  1de962:	0f 2e c2             	ucomiss %xmm2,%xmm0
  1de965:	74 1d                	je     1de984 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xb84>
  1de967:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de96c:	f3 0f 11 4c be 10    	movss  %xmm1,0x10(%rsi,%rdi,4)
  1de972:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
  1de977:	f3 0f 11 44 be 10    	movss  %xmm0,0x10(%rsi,%rdi,4)
  1de97d:	4d 89 0c c8          	mov    %r9,(%r8,%rcx,8)
  1de981:	41 b2 01             	mov    $0x1,%r10b
  1de984:	83 f8 05             	cmp    $0x5,%eax
  1de987:	0f 84 3c 01 00 00    	je     1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1de98d:	48 8b 4c 24 10       	mov    0x10(%rsp),%rcx
  1de992:	f3 0f 10 44 b9 14    	movss  0x14(%rcx,%rdi,4),%xmm0
  1de998:	0f 28 0d 81 21 e4 ff 	movaps -0x1bde7f(%rip),%xmm1        # 20b20 <anon.44166ce10ff660448c7f2cfe9904e0b0.0.llvm.3986032740938256925+0xbe0>
  1de99f:	0f 54 c8             	andps  %xmm0,%xmm1
  1de9a2:	0f 2e 0d 1f 00 e4 ff 	ucomiss -0x1bffe1(%rip),%xmm1        # 1e9c8 <__abi_tag+0x1e6cc>
  1de9a9:	76 6f                	jbe    1dea1a <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xc1a>
  1de9ab:	48 8d 4f 05          	lea    0x5(%rdi),%rcx
  1de9af:	48 39 d1             	cmp    %rdx,%rcx
  1de9b2:	0f 83 25 01 00 00    	jae    1deadd <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcdd>
  1de9b8:	4d 8b 0c c8          	mov    (%r8,%rcx,8),%r9
  1de9bc:	49 ff c1             	inc    %r9
  1de9bf:	0f 80 01 01 00 00    	jo     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1de9c5:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1de9ca:	f3 0f 10 5c be 14    	movss  0x14(%rsi,%rdi,4),%xmm3
  1de9d0:	0f 28 cb             	movaps %xmm3,%xmm1
  1de9d3:	f3 0f 59 cd          	mulss  %xmm5,%xmm1
  1de9d7:	0f 57 d2             	xorps  %xmm2,%xmm2
  1de9da:	f3 0f c2 da 04       	cmpneqss %xmm2,%xmm3
  1de9df:	0f 28 e1             	movaps %xmm1,%xmm4
  1de9e2:	f3 0f c2 e2 00       	cmpeqss %xmm2,%xmm4
  1de9e7:	0f 54 e3             	andps  %xmm3,%xmm4
  1de9ea:	66 0f 7e e6          	movd   %xmm4,%esi
  1de9ee:	40 f6 c6 01          	test   $0x1,%sil
  1de9f2:	75 26                	jne    1dea1a <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xc1a>
  1de9f4:	f3 0f 59 c5          	mulss  %xmm5,%xmm0
  1de9f8:	0f 2e c2             	ucomiss %xmm2,%xmm0
  1de9fb:	74 1d                	je     1dea1a <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xc1a>
  1de9fd:	48 8b 74 24 08       	mov    0x8(%rsp),%rsi
  1dea02:	f3 0f 11 4c be 14    	movss  %xmm1,0x14(%rsi,%rdi,4)
  1dea08:	48 8b 74 24 10       	mov    0x10(%rsp),%rsi
  1dea0d:	f3 0f 11 44 be 14    	movss  %xmm0,0x14(%rsi,%rdi,4)
  1dea13:	4d 89 0c c8          	mov    %r9,(%r8,%rcx,8)
  1dea17:	41 b2 01             	mov    $0x1,%r10b
  1dea1a:	83 f8 06             	cmp    $0x6,%eax
  1dea1d:	0f 84 a6 00 00 00    	je     1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1dea23:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
  1dea28:	f3 0f 10 44 b8 18    	movss  0x18(%rax,%rdi,4),%xmm0
  1dea2e:	0f 28 0d eb 20 e4 ff 	movaps -0x1bdf15(%rip),%xmm1        # 20b20 <anon.44166ce10ff660448c7f2cfe9904e0b0.0.llvm.3986032740938256925+0xbe0>
  1dea35:	0f 54 c8             	andps  %xmm0,%xmm1
  1dea38:	0f 2e 0d 89 ff e3 ff 	ucomiss -0x1c0077(%rip),%xmm1        # 1e9c8 <__abi_tag+0x1e6cc>
  1dea3f:	0f 86 84 00 00 00    	jbe    1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1dea45:	48 8d 47 06          	lea    0x6(%rdi),%rax
  1dea49:	48 39 d0             	cmp    %rdx,%rax
  1dea4c:	0f 83 a0 00 00 00    	jae    1deaf2 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcf2>
  1dea52:	49 8b 0c c0          	mov    (%r8,%rax,8),%rcx
  1dea56:	48 ff c1             	inc    %rcx
  1dea59:	70 6b                	jo     1deac6 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc6>
  1dea5b:	48 8b 54 24 08       	mov    0x8(%rsp),%rdx
  1dea60:	f3 0f 10 5c ba 18    	movss  0x18(%rdx,%rdi,4),%xmm3
  1dea66:	0f 28 cb             	movaps %xmm3,%xmm1
  1dea69:	f3 0f 59 cd          	mulss  %xmm5,%xmm1
  1dea6d:	0f 57 d2             	xorps  %xmm2,%xmm2
  1dea70:	f3 0f c2 da 04       	cmpneqss %xmm2,%xmm3
  1dea75:	0f 28 e1             	movaps %xmm1,%xmm4
  1dea78:	f3 0f c2 e2 00       	cmpeqss %xmm2,%xmm4
  1dea7d:	0f 54 e3             	andps  %xmm3,%xmm4
  1dea80:	66 0f 7e e2          	movd   %xmm4,%edx
  1dea84:	f6 c2 01             	test   $0x1,%dl
  1dea87:	75 40                	jne    1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1dea89:	f3 0f 59 c5          	mulss  %xmm5,%xmm0
  1dea8d:	0f 2e c2             	ucomiss %xmm2,%xmm0
  1dea90:	74 37                	je     1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1dea92:	48 8b 54 24 08       	mov    0x8(%rsp),%rdx
  1dea97:	f3 0f 11 4c ba 18    	movss  %xmm1,0x18(%rdx,%rdi,4)
  1dea9d:	48 8b 54 24 10       	mov    0x10(%rsp),%rdx
  1deaa2:	f3 0f 11 44 ba 18    	movss  %xmm0,0x18(%rdx,%rdi,4)
  1deaa8:	49 89 0c c0          	mov    %rcx,(%r8,%rax,8)
  1deaac:	41 b2 01             	mov    $0x1,%r10b
  1deaaf:	eb 18                	jmp    1deac9 <rustsht::legendre::scalar_ordered::rescale_ordered_batch_f32::<pulp::x86::v3::V3, true, false>+0xcc9>
  1deab1:	49 8d 74 24 08       	lea    0x8(%r12),%rsi
  1deab6:	48 8d 0d 93 9b 09 00 	lea    0x99b93(%rip),%rcx        # 278650 <anon.f99f0da779ee2b57768039fdd9ea1592.25.llvm.17656861175786383952+0x48>
  1deabd:	4c 89 e7             	mov    %r12,%rdi
  1deac0:	ff 15 1a 0e 0a 00    	call   *0xa0e1a(%rip)        # 27f8e0 <_DYNAMIC+0x298>
  1deac6:	41 b2 02             	mov    $0x2,%r10b
  1deac9:	44 89 d0             	mov    %r10d,%eax
  1deacc:	b2 01                	mov    $0x1,%dl
  1deace:	48 8d 65 d8          	lea    -0x28(%rbp),%rsp
  1dead2:	5b                   	pop    %rbx
  1dead3:	41 5c                	pop    %r12
  1dead5:	41 5d                	pop    %r13
  1dead7:	41 5e                	pop    %r14
  1dead9:	41 5f                	pop    %r15
  1deadb:	5d                   	pop    %rbp
  1deadc:	c3                   	ret
  1deadd:	48 89 cf             	mov    %rcx,%rdi
  1deae0:	48 8d 15 39 9b 09 00 	lea    0x99b39(%rip),%rdx        # 278620 <anon.f99f0da779ee2b57768039fdd9ea1592.25.llvm.17656861175786383952+0x18>
  1deae7:	48 8b 74 24 58       	mov    0x58(%rsp),%rsi
  1deaec:	ff 15 b6 0d 0a 00    	call   *0xa0db6(%rip)        # 27f8a8 <_DYNAMIC+0x260>
  1deaf2:	48 89 c7             	mov    %rax,%rdi
  1deaf5:	48 8d 15 24 9b 09 00 	lea    0x99b24(%rip),%rdx        # 278620 <anon.f99f0da779ee2b57768039fdd9ea1592.25.llvm.17656861175786383952+0x18>
  1deafc:	48 8b 74 24 58       	mov    0x58(%rsp),%rsi
  1deb01:	ff 15 a1 0d 0a 00    	call   *0xa0da1(%rip)        # 27f8a8 <_DYNAMIC+0x260>
  1deb07:	cc                   	int3
  1deb08:	cc                   	int3
  1deb09:	cc                   	int3
  1deb0a:	cc                   	int3
  1deb0b:	cc                   	int3
  1deb0c:	cc                   	int3
  1deb0d:	cc                   	int3
  1deb0e:	cc                   	int3
  1deb0f:	cc                   	int3
