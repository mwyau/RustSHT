
target/release/deps/rustsht-7d36680db74a2dd3:     file format elf64-x86-64


Disassembly of section .text:

000000000072cf60 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_>:
  72cf60:	31 c0                	xor    %eax,%eax
  72cf62:	48 83 7f 48 08       	cmpq   $0x8,0x48(%rdi)
  72cf67:	0f 85 65 03 00 00    	jne    72d2d2 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x372>
  72cf6d:	48 83 7f 58 08       	cmpq   $0x8,0x58(%rdi)
  72cf72:	0f 85 5a 03 00 00    	jne    72d2d2 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x372>
  72cf78:	48 83 7f 68 08       	cmpq   $0x8,0x68(%rdi)
  72cf7d:	0f 85 4f 03 00 00    	jne    72d2d2 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x372>
  72cf83:	48 8b 77 08          	mov    0x8(%rdi),%rsi
  72cf87:	48 3b 77 18          	cmp    0x18(%rdi),%rsi
  72cf8b:	0f 85 3f 03 00 00    	jne    72d2d0 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x370>
  72cf91:	48 3b 77 28          	cmp    0x28(%rdi),%rsi
  72cf95:	0f 85 35 03 00 00    	jne    72d2d0 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x370>
  72cf9b:	48 3b 77 38          	cmp    0x38(%rdi),%rsi
  72cf9f:	0f 85 2b 03 00 00    	jne    72d2d0 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x370>
  72cfa5:	48 83 7f 78 08       	cmpq   $0x8,0x78(%rdi)
  72cfaa:	0f 85 20 03 00 00    	jne    72d2d0 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x370>
  72cfb0:	53                   	push   %rbx
  72cfb1:	48 83 ec 60          	sub    $0x60,%rsp
  72cfb5:	4c 8b 57 40          	mov    0x40(%rdi),%r10
  72cfb9:	4c 8b 5f 50          	mov    0x50(%rdi),%r11
  72cfbd:	48 8b 5f 60          	mov    0x60(%rdi),%rbx
  72cfc1:	4c 8b 07             	mov    (%rdi),%r8
  72cfc4:	4c 8b 4f 10          	mov    0x10(%rdi),%r9
  72cfc8:	48 8b 4f 20          	mov    0x20(%rdi),%rcx
  72cfcc:	48 8b 57 30          	mov    0x30(%rdi),%rdx
  72cfd0:	48 8b 47 70          	mov    0x70(%rdi),%rax
  72cfd4:	c4 c1 7c 10 12       	vmovups (%r10),%ymm2
  72cfd9:	c4 c1 7c 10 0b       	vmovups (%r11),%ymm1
  72cfde:	c5 7c 10 23          	vmovups (%rbx),%ymm12
  72cfe2:	80 bf 80 00 00 00 00 	cmpb   $0x0,0x80(%rdi)
  72cfe9:	0f 84 e4 02 00 00    	je     72d2d3 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x373>
  72cfef:	c5 fc 11 4c 24 20    	vmovups %ymm1,0x20(%rsp)
  72cff5:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  72cff9:	c5 e0 57 db          	vxorps %xmm3,%xmm3,%xmm3
  72cffd:	c4 41 38 57 c0       	vxorps %xmm8,%xmm8,%xmm8
  72d002:	c5 c8 57 f6          	vxorps %xmm6,%xmm6,%xmm6
  72d006:	48 85 f6             	test   %rsi,%rsi
  72d009:	c5 fc 11 14 24       	vmovups %ymm2,(%rsp)
  72d00e:	0f 84 29 02 00 00    	je     72d23d <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x2dd>
  72d014:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  72d018:	c5 fc 59 04 24       	vmulps (%rsp),%ymm0,%ymm0
  72d01d:	c5 fc 11 44 24 40    	vmovups %ymm0,0x40(%rsp)
  72d023:	31 ff                	xor    %edi,%edi
  72d025:	c4 41 00 57 ff       	vxorps %xmm15,%xmm15,%xmm15
  72d02a:	c4 41 28 57 d2       	vxorps %xmm10,%xmm10,%xmm10
  72d02f:	c4 41 20 57 db       	vxorps %xmm11,%xmm11,%xmm11
  72d034:	c5 e8 57 d2          	vxorps %xmm2,%xmm2,%xmm2
  72d038:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  72d03f:	00
  72d040:	c4 41 7c 28 eb       	vmovaps %ymm11,%ymm13
  72d045:	c4 41 7c 28 f2       	vmovaps %ymm10,%ymm14
  72d04a:	c4 41 7c 28 dc       	vmovaps %ymm12,%ymm11
  72d04f:	c4 41 7c 28 d7       	vmovaps %ymm15,%ymm10
  72d054:	c4 62 7d 18 24 ba    	vbroadcastss (%rdx,%rdi,4),%ymm12
  72d05a:	c4 41 24 59 fc       	vmulps %ymm12,%ymm11,%ymm15
  72d05f:	c5 a4 59 c2          	vmulps %ymm2,%ymm11,%ymm0
  72d063:	c4 c1 2c 59 cc       	vmulps %ymm12,%ymm10,%ymm1
  72d068:	c4 42 25 aa e7       	vfmsub213ps %ymm15,%ymm11,%ymm12
  72d06d:	c5 fc 58 c9          	vaddps %ymm1,%ymm0,%ymm1
  72d071:	c5 9c 58 c9          	vaddps %ymm1,%ymm12,%ymm1
  72d075:	c5 04 58 e1          	vaddps %ymm1,%ymm15,%ymm12
  72d079:	c4 41 3c 58 cc       	vaddps %ymm12,%ymm8,%ymm9
  72d07e:	c4 c1 34 5c e8       	vsubps %ymm8,%ymm9,%ymm5
  72d083:	c5 b4 5c fd          	vsubps %ymm5,%ymm9,%ymm7
  72d087:	c5 bc 5c ff          	vsubps %ymm7,%ymm8,%ymm7
  72d08b:	c5 9c 5c ed          	vsubps %ymm5,%ymm12,%ymm5
  72d08f:	c5 d4 58 ef          	vaddps %ymm7,%ymm5,%ymm5
  72d093:	c4 c1 1c 5c ff       	vsubps %ymm15,%ymm12,%ymm7
  72d098:	c5 1c 5c c7          	vsubps %ymm7,%ymm12,%ymm8
  72d09c:	c4 41 04 5c c0       	vsubps %ymm8,%ymm15,%ymm8
  72d0a1:	c5 f4 5c cf          	vsubps %ymm7,%ymm1,%ymm1
  72d0a5:	c5 bc 58 c9          	vaddps %ymm1,%ymm8,%ymm1
  72d0a9:	c5 cc 58 c9          	vaddps %ymm1,%ymm6,%ymm1
  72d0ad:	c5 d4 58 c9          	vaddps %ymm1,%ymm5,%ymm1
  72d0b1:	c5 34 58 c1          	vaddps %ymm1,%ymm9,%ymm8
  72d0b5:	c4 c1 3c 5c e9       	vsubps %ymm9,%ymm8,%ymm5
  72d0ba:	c5 bc 5c f5          	vsubps %ymm5,%ymm8,%ymm6
  72d0be:	c5 b4 5c f6          	vsubps %ymm6,%ymm9,%ymm6
  72d0c2:	c5 f4 5c cd          	vsubps %ymm5,%ymm1,%ymm1
  72d0c6:	c5 f4 58 f6          	vaddps %ymm6,%ymm1,%ymm6
  72d0ca:	c4 e2 7d 18 0c b9    	vbroadcastss (%rcx,%rdi,4),%ymm1
  72d0d0:	c5 a4 59 e9          	vmulps %ymm1,%ymm11,%ymm5
  72d0d4:	c5 ac 59 f9          	vmulps %ymm1,%ymm10,%ymm7
  72d0d8:	c4 e2 25 aa cd       	vfmsub213ps %ymm5,%ymm11,%ymm1
  72d0dd:	c5 fc 58 c7          	vaddps %ymm7,%ymm0,%ymm0
  72d0e1:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  72d0e5:	c5 d4 58 c8          	vaddps %ymm0,%ymm5,%ymm1
  72d0e9:	c5 dc 58 f9          	vaddps %ymm1,%ymm4,%ymm7
  72d0ed:	c5 44 5c cc          	vsubps %ymm4,%ymm7,%ymm9
  72d0f1:	c4 41 44 5c e1       	vsubps %ymm9,%ymm7,%ymm12
  72d0f6:	c4 c1 5c 5c e4       	vsubps %ymm12,%ymm4,%ymm4
  72d0fb:	c4 42 7d 18 24 b8    	vbroadcastss (%r8,%rdi,4),%ymm12
  72d101:	c4 41 74 5c c9       	vsubps %ymm9,%ymm1,%ymm9
  72d106:	c5 b4 58 e4          	vaddps %ymm4,%ymm9,%ymm4
  72d10a:	c5 74 5c cd          	vsubps %ymm5,%ymm1,%ymm9
  72d10e:	c4 c1 74 5c c9       	vsubps %ymm9,%ymm1,%ymm1
  72d113:	c5 d4 5c c9          	vsubps %ymm1,%ymm5,%ymm1
  72d117:	c4 c1 7c 5c c1       	vsubps %ymm9,%ymm0,%ymm0
  72d11c:	c5 fc 58 c1          	vaddps %ymm1,%ymm0,%ymm0
  72d120:	c5 e4 58 c0          	vaddps %ymm0,%ymm3,%ymm0
  72d124:	c5 dc 58 c0          	vaddps %ymm0,%ymm4,%ymm0
  72d128:	c5 c4 58 e0          	vaddps %ymm0,%ymm7,%ymm4
  72d12c:	c5 dc 5c cf          	vsubps %ymm7,%ymm4,%ymm1
  72d130:	c5 dc 5c d9          	vsubps %ymm1,%ymm4,%ymm3
  72d134:	c5 c4 5c db          	vsubps %ymm3,%ymm7,%ymm3
  72d138:	c5 fc 5c c1          	vsubps %ymm1,%ymm0,%ymm0
  72d13c:	c5 fc 58 db          	vaddps %ymm3,%ymm0,%ymm3
  72d140:	c4 c2 7d 18 04 b9    	vbroadcastss (%r9,%rdi,4),%ymm0
  72d146:	c5 fc 10 3c 24       	vmovups (%rsp),%ymm7
  72d14b:	c5 9c 59 cf          	vmulps %ymm7,%ymm12,%ymm1
  72d14f:	c5 d0 57 ed          	vxorps %xmm5,%xmm5,%xmm5
  72d153:	c5 9a 59 ed          	vmulss %xmm5,%xmm12,%xmm5
  72d157:	c4 62 45 aa e1       	vfmsub213ps %ymm1,%ymm7,%ymm12
  72d15c:	c4 e2 7d 18 ed       	vbroadcastss %xmm5,%ymm5
  72d161:	c5 d4 58 6c 24 40    	vaddps 0x40(%rsp),%ymm5,%ymm5
  72d167:	c5 9c 58 ed          	vaddps %ymm5,%ymm12,%ymm5
  72d16b:	c5 f4 58 fd          	vaddps %ymm5,%ymm1,%ymm7
  72d16f:	c5 44 5c c9          	vsubps %ymm1,%ymm7,%ymm9
  72d173:	c4 41 44 5c e1       	vsubps %ymm9,%ymm7,%ymm12
  72d178:	c4 c1 74 5c cc       	vsubps %ymm12,%ymm1,%ymm1
  72d17d:	c4 c1 54 5c e9       	vsubps %ymm9,%ymm5,%ymm5
  72d182:	c5 d4 58 c9          	vaddps %ymm1,%ymm5,%ymm1
  72d186:	c5 fc 58 ef          	vaddps %ymm7,%ymm0,%ymm5
  72d18a:	c5 54 5c cf          	vsubps %ymm7,%ymm5,%ymm9
  72d18e:	c4 41 54 5c e1       	vsubps %ymm9,%ymm5,%ymm12
  72d193:	c4 c1 44 5c fc       	vsubps %ymm12,%ymm7,%ymm7
  72d198:	c4 c1 7c 5c c1       	vsubps %ymm9,%ymm0,%ymm0
  72d19d:	c5 fc 58 c7          	vaddps %ymm7,%ymm0,%ymm0
  72d1a1:	c5 f4 58 ca          	vaddps %ymm2,%ymm1,%ymm1
  72d1a5:	c5 fc 58 c1          	vaddps %ymm1,%ymm0,%ymm0
  72d1a9:	c5 d4 58 c8          	vaddps %ymm0,%ymm5,%ymm1
  72d1ad:	c5 f4 5c fd          	vsubps %ymm5,%ymm1,%ymm7
  72d1b1:	c5 74 5c cf          	vsubps %ymm7,%ymm1,%ymm9
  72d1b5:	c4 c1 54 5c e9       	vsubps %ymm9,%ymm5,%ymm5
  72d1ba:	c5 fc 5c c7          	vsubps %ymm7,%ymm0,%ymm0
  72d1be:	c5 fc 58 c5          	vaddps %ymm5,%ymm0,%ymm0
  72d1c2:	c5 a4 59 e9          	vmulps %ymm1,%ymm11,%ymm5
  72d1c6:	c5 7c 29 df          	vmovaps %ymm11,%ymm7
  72d1ca:	c4 e2 75 aa fd       	vfmsub213ps %ymm5,%ymm1,%ymm7
  72d1cf:	c5 ac 59 c9          	vmulps %ymm1,%ymm10,%ymm1
  72d1d3:	c5 a4 59 c0          	vmulps %ymm0,%ymm11,%ymm0
  72d1d7:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  72d1db:	c5 c4 58 c0          	vaddps %ymm0,%ymm7,%ymm0
  72d1df:	c5 d4 58 c8          	vaddps %ymm0,%ymm5,%ymm1
  72d1e3:	c5 f4 5c fd          	vsubps %ymm5,%ymm1,%ymm7
  72d1e7:	c5 74 5c cf          	vsubps %ymm7,%ymm1,%ymm9
  72d1eb:	c4 c1 54 5c e9       	vsubps %ymm9,%ymm5,%ymm5
  72d1f0:	c5 fc 5c c7          	vsubps %ymm7,%ymm0,%ymm0
  72d1f4:	c5 fc 58 c5          	vaddps %ymm5,%ymm0,%ymm0
  72d1f8:	c5 8c 58 c0          	vaddps %ymm0,%ymm14,%ymm0
  72d1fc:	c5 94 58 e9          	vaddps %ymm1,%ymm13,%ymm5
  72d200:	c5 d4 5c f9          	vsubps %ymm1,%ymm5,%ymm7
  72d204:	c5 54 5c cf          	vsubps %ymm7,%ymm5,%ymm9
  72d208:	c4 c1 74 5c c9       	vsubps %ymm9,%ymm1,%ymm1
  72d20d:	c5 94 5c ff          	vsubps %ymm7,%ymm13,%ymm7
  72d211:	c5 c4 58 c9          	vaddps %ymm1,%ymm7,%ymm1
  72d215:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  72d219:	c5 54 58 e0          	vaddps %ymm0,%ymm5,%ymm12
  72d21d:	c5 9c 5c cd          	vsubps %ymm5,%ymm12,%ymm1
  72d221:	c5 9c 5c f9          	vsubps %ymm1,%ymm12,%ymm7
  72d225:	c5 d4 5c ef          	vsubps %ymm7,%ymm5,%ymm5
  72d229:	c5 fc 5c c1          	vsubps %ymm1,%ymm0,%ymm0
  72d22d:	c5 7c 58 fd          	vaddps %ymm5,%ymm0,%ymm15
  72d231:	48 ff c7             	inc    %rdi
  72d234:	48 39 fe             	cmp    %rdi,%rsi
  72d237:	0f 85 03 fe ff ff    	jne    72d040 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0xe0>
  72d23d:	c5 fc 10 4c 24 20    	vmovups 0x20(%rsp),%ymm1
  72d243:	c5 bc 59 d1          	vmulps %ymm1,%ymm8,%ymm2
  72d247:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  72d24b:	c5 bc 59 c0          	vmulps %ymm0,%ymm8,%ymm0
  72d24f:	c4 62 75 aa c2       	vfmsub213ps %ymm2,%ymm1,%ymm8
  72d254:	c5 f4 59 ce          	vmulps %ymm6,%ymm1,%ymm1
  72d258:	c5 fc 58 c1          	vaddps %ymm1,%ymm0,%ymm0
  72d25c:	c5 bc 58 c0          	vaddps %ymm0,%ymm8,%ymm0
  72d260:	c5 ec 58 c8          	vaddps %ymm0,%ymm2,%ymm1
  72d264:	c5 f4 5c ea          	vsubps %ymm2,%ymm1,%ymm5
  72d268:	c5 f4 5c f5          	vsubps %ymm5,%ymm1,%ymm6
  72d26c:	c5 ec 5c d6          	vsubps %ymm6,%ymm2,%ymm2
  72d270:	c5 fc 5c c5          	vsubps %ymm5,%ymm0,%ymm0
  72d274:	c5 fc 58 c2          	vaddps %ymm2,%ymm0,%ymm0
  72d278:	c5 e4 58 c0          	vaddps %ymm0,%ymm3,%ymm0
  72d27c:	c5 dc 58 d1          	vaddps %ymm1,%ymm4,%ymm2
  72d280:	c5 ec 5c dc          	vsubps %ymm4,%ymm2,%ymm3
  72d284:	c5 ec 5c eb          	vsubps %ymm3,%ymm2,%ymm5
  72d288:	c5 dc 5c e5          	vsubps %ymm5,%ymm4,%ymm4
  72d28c:	c5 f4 5c cb          	vsubps %ymm3,%ymm1,%ymm1
  72d290:	c5 f4 58 cc          	vaddps %ymm4,%ymm1,%ymm1
  72d294:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  72d298:	c5 ec 58 c8          	vaddps %ymm0,%ymm2,%ymm1
  72d29c:	c5 f4 5c da          	vsubps %ymm2,%ymm1,%ymm3
  72d2a0:	c5 f4 5c e3          	vsubps %ymm3,%ymm1,%ymm4
  72d2a4:	c5 ec 5c d4          	vsubps %ymm4,%ymm2,%ymm2
  72d2a8:	c5 fc 5c c3          	vsubps %ymm3,%ymm0,%ymm0
  72d2ac:	c5 fc 58 c2          	vaddps %ymm2,%ymm0,%ymm0
  72d2b0:	c5 f4 58 c0          	vaddps %ymm0,%ymm1,%ymm0
  72d2b4:	48 8b 0d 85 c7 14 00 	mov    0x14c785(%rip),%rcx        # 879a40 <_DYNAMIC+0x550>
  72d2bb:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  72d2c2:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 72d2cb <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x36b>
  72d2c9:	ff e2                	jmp    *%rdx
  72d2cb:	e9 e8 00 00 00       	jmp    72d3b8 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x458>
  72d2d0:	31 c0                	xor    %eax,%eax
  72d2d2:	c3                   	ret
  72d2d3:	48 85 f6             	test   %rsi,%rsi
  72d2d6:	74 15                	je     72d2ed <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x38d>
  72d2d8:	48 83 fe 01          	cmp    $0x1,%rsi
  72d2dc:	75 1c                	jne    72d2fa <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x39a>
  72d2de:	c5 e0 57 db          	vxorps %xmm3,%xmm3,%xmm3
  72d2e2:	31 ff                	xor    %edi,%edi
  72d2e4:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  72d2e8:	e9 91 00 00 00       	jmp    72d37e <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x41e>
  72d2ed:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  72d2f1:	c5 e0 57 db          	vxorps %xmm3,%xmm3,%xmm3
  72d2f5:	e9 a2 00 00 00       	jmp    72d39c <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x43c>
  72d2fa:	49 89 f2             	mov    %rsi,%r10
  72d2fd:	49 83 e2 fe          	and    $0xfffffffffffffffe,%r10
  72d301:	c5 e0 57 db          	vxorps %xmm3,%xmm3,%xmm3
  72d305:	31 ff                	xor    %edi,%edi
  72d307:	c5 d8 57 e4          	vxorps %xmm4,%xmm4,%xmm4
  72d30b:	c5 f8 57 c0          	vxorps %xmm0,%xmm0,%xmm0
  72d30f:	c5 7c 29 e5          	vmovaps %ymm12,%ymm5
  72d313:	c4 c2 7d 18 34 b8    	vbroadcastss (%r8,%rdi,4),%ymm6
  72d319:	c4 e2 7d 18 3c ba    	vbroadcastss (%rdx,%rdi,4),%ymm7
  72d31f:	c4 e2 1d a8 f8       	vfmadd213ps %ymm0,%ymm12,%ymm7
  72d324:	c4 62 7d 18 04 b9    	vbroadcastss (%rcx,%rdi,4),%ymm8
  72d32a:	c4 62 1d a8 c3       	vfmadd213ps %ymm3,%ymm12,%ymm8
  72d32f:	c4 c2 7d 18 04 b9    	vbroadcastss (%r9,%rdi,4),%ymm0
  72d335:	c4 e2 6d b8 c6       	vfmadd231ps %ymm6,%ymm2,%ymm0
  72d33a:	c4 e2 1d b8 e0       	vfmadd231ps %ymm0,%ymm12,%ymm4
  72d33f:	c4 c2 7d 18 74 b8 04 	vbroadcastss 0x4(%r8,%rdi,4),%ymm6
  72d346:	c4 e2 7d 18 44 ba 04 	vbroadcastss 0x4(%rdx,%rdi,4),%ymm0
  72d34d:	c4 e2 5d a8 c7       	vfmadd213ps %ymm7,%ymm4,%ymm0
  72d352:	c4 e2 7d 18 5c b9 04 	vbroadcastss 0x4(%rcx,%rdi,4),%ymm3
  72d359:	c4 c2 5d a8 d8       	vfmadd213ps %ymm8,%ymm4,%ymm3
  72d35e:	c4 42 7d 18 64 b9 04 	vbroadcastss 0x4(%r9,%rdi,4),%ymm12
  72d365:	48 83 c7 02          	add    $0x2,%rdi
  72d369:	c4 62 6d b8 e6       	vfmadd231ps %ymm6,%ymm2,%ymm12
  72d36e:	c4 62 5d a8 e5       	vfmadd213ps %ymm5,%ymm4,%ymm12
  72d373:	49 39 fa             	cmp    %rdi,%r10
  72d376:	75 97                	jne    72d30f <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x3af>
  72d378:	40 f6 c6 01          	test   $0x1,%sil
  72d37c:	74 1e                	je     72d39c <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x43c>
  72d37e:	c5 fc 28 d0          	vmovaps %ymm0,%ymm2
  72d382:	c5 fc 28 e3          	vmovaps %ymm3,%ymm4
  72d386:	c4 e2 7d 18 04 ba    	vbroadcastss (%rdx,%rdi,4),%ymm0
  72d38c:	c4 e2 1d a8 c2       	vfmadd213ps %ymm2,%ymm12,%ymm0
  72d391:	c4 e2 7d 18 1c b9    	vbroadcastss (%rcx,%rdi,4),%ymm3
  72d397:	c4 e2 1d a8 dc       	vfmadd213ps %ymm4,%ymm12,%ymm3
  72d39c:	c4 e2 75 a8 c3       	vfmadd213ps %ymm3,%ymm1,%ymm0
  72d3a1:	48 8b 0d 98 c6 14 00 	mov    0x14c698(%rip),%rcx        # 879a40 <_DYNAMIC+0x550>
  72d3a8:	48 8b 91 08 08 00 00 	mov    0x808(%rcx),%rdx
  72d3af:	48 8d 0d 02 00 00 00 	lea    0x2(%rip),%rcx        # 72d3b8 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x458>
  72d3b6:	ff e2                	jmp    *%rdx
  72d3b8:	8b 08                	mov    (%rax),%ecx
  72d3ba:	01 c9                	add    %ecx,%ecx
  72d3bc:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  72d3c2:	77 5a                	ja     72d41e <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x4be>
  72d3c4:	8b 48 04             	mov    0x4(%rax),%ecx
  72d3c7:	01 c9                	add    %ecx,%ecx
  72d3c9:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  72d3cf:	77 4d                	ja     72d41e <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x4be>
  72d3d1:	8b 48 08             	mov    0x8(%rax),%ecx
  72d3d4:	01 c9                	add    %ecx,%ecx
  72d3d6:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  72d3dc:	77 40                	ja     72d41e <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x4be>
  72d3de:	8b 48 0c             	mov    0xc(%rax),%ecx
  72d3e1:	01 c9                	add    %ecx,%ecx
  72d3e3:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  72d3e9:	77 33                	ja     72d41e <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x4be>
  72d3eb:	8b 48 10             	mov    0x10(%rax),%ecx
  72d3ee:	01 c9                	add    %ecx,%ecx
  72d3f0:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  72d3f6:	77 26                	ja     72d41e <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x4be>
  72d3f8:	8b 48 14             	mov    0x14(%rax),%ecx
  72d3fb:	01 c9                	add    %ecx,%ecx
  72d3fd:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  72d403:	77 19                	ja     72d41e <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x4be>
  72d405:	8b 48 18             	mov    0x18(%rax),%ecx
  72d408:	01 c9                	add    %ecx,%ecx
  72d40a:	81 f9 ff ff ff fe    	cmp    $0xfeffffff,%ecx
  72d410:	77 0c                	ja     72d41e <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x4be>
  72d412:	8b 40 1c             	mov    0x1c(%rax),%eax
  72d415:	01 c0                	add    %eax,%eax
  72d417:	3d ff ff ff fe       	cmp    $0xfeffffff,%eax
  72d41c:	76 0b                	jbe    72d429 <_RINvXs0_NtNtNtCs1mtnvNSEw2f_7rustsht8legendre14scalar_ordered5testsNtB6_22OrderedF32VectorReplayNtCs6aRYI14iRvw_4pulp8WithSimd9with_simdNtNtNtB1y_3x862v32V3EBc_+0x4c9>
  72d41e:	31 c0                	xor    %eax,%eax
  72d420:	48 83 c4 60          	add    $0x60,%rsp
  72d424:	5b                   	pop    %rbx
  72d425:	c5 f8 77             	vzeroupper
  72d428:	c3                   	ret
  72d429:	b0 01                	mov    $0x1,%al
  72d42b:	48 83 c4 60          	add    $0x60,%rsp
  72d42f:	5b                   	pop    %rbx
  72d430:	c5 f8 77             	vzeroupper
  72d433:	c3                   	ret
