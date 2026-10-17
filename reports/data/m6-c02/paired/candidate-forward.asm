
target/m6-c02-candidate-bench:     file format elf64-x86-64


Disassembly of section .text:

00000000000f7e10 <<rustsht::fft::RealFftPlan>::forward>:
   f7e10:	push   %rbp
   f7e11:	push   %r15
   f7e13:	push   %r14
   f7e15:	push   %r13
   f7e17:	push   %r12
   f7e19:	push   %rbx
   f7e1a:	push   %rax
   f7e1b:	mov    0x38(%rsi),%rax
   f7e1f:	cmp    %rax,%rcx
   f7e22:	jne    f7eaf <<rustsht::fft::RealFftPlan>::forward+0x9f>
   f7e28:	mov    %rcx,%r12
   f7e2b:	shr    $1,%r12
   f7e2e:	lea    0x1(%r12),%rax
   f7e33:	cmp    %rax,%r9
   f7e36:	jne    f7ed6 <<rustsht::fft::RealFftPlan>::forward+0xc6>
   f7e3c:	mov    %r8,%r13
   f7e3f:	mov    0x40(%rsp),%rax
   f7e44:	mov    0x40(%rsi),%rbx
   f7e48:	mov    0x10(%rax),%r8
   f7e4c:	cmp    %rbx,%r8
   f7e4f:	jne    f7efd <<rustsht::fft::RealFftPlan>::forward+0xed>
   f7e55:	mov    0x28(%rax),%r8
   f7e59:	mov    0x48(%rsi),%r10
   f7e5d:	cmp    %r10,%r8
   f7e60:	jne    f7f0b <<rustsht::fft::RealFftPlan>::forward+0xfb>
   f7e66:	cmpq   $0xffffffffffffffff,(%rsi)
   f7e6a:	je     f7f36 <<rustsht::fft::RealFftPlan>::forward+0x126>
   f7e70:	mov    %r13,(%rsp)
   f7e74:	mov    %r9,%r15
   f7e77:	shl    $0x3,%rcx
   f7e7b:	mov    %rcx,%r10
   f7e7e:	mov    0x8(%rax),%r14
   f7e82:	and    $0xfffffffffffffff0,%r10
   f7e86:	je     f7ff1 <<rustsht::fft::RealFftPlan>::forward+0x1e1>
   f7e8c:	add    $0xfffffffffffffff0,%rcx
   f7e90:	shr    $0x4,%rcx
   f7e94:	cmp    %rcx,%rbx
   f7e97:	cmovb  %rbx,%rcx
   f7e9b:	cmp    $0x4,%rcx
   f7e9f:	jae    f7f5f <<rustsht::fft::RealFftPlan>::forward+0x14f>
   f7ea5:	xor    %ecx,%ecx
   f7ea7:	mov    %rdx,%r9
   f7eaa:	jmp    f7fb2 <<rustsht::fft::RealFftPlan>::forward+0x1a2>
   f7eaf:	movq   $0xc,(%rdi)
   f7eb6:	mov    %rax,0x8(%rdi)
   f7eba:	mov    %rcx,0x10(%rdi)
   f7ebe:	lea    -0xcdc87(%rip),%rax        # 2a23e <anon.af498e3ca18112c3d24389f09b879115.78.llvm.7730762197114068069+0x62>
   f7ec5:	mov    %rax,0x18(%rdi)
   f7ec9:	movq   $0xe,0x20(%rdi)
   f7ed1:	jmp    f83f0 <<rustsht::fft::RealFftPlan>::forward+0x5e0>
   f7ed6:	movq   $0xc,(%rdi)
   f7edd:	mov    %rax,0x8(%rdi)
   f7ee1:	mov    %r9,0x10(%rdi)
   f7ee5:	lea    -0xcdd80(%rip),%rax        # 2a16c <anon.0f6e124a8ee63ec2128466979e645f81.79.llvm.13775430102626878459+0x158>
   f7eec:	mov    %rax,0x18(%rdi)
   f7ef0:	movq   $0x11,0x20(%rdi)
   f7ef8:	jmp    f83f0 <<rustsht::fft::RealFftPlan>::forward+0x5e0>
   f7efd:	lea    -0xcdcfc(%rip),%rax        # 2a208 <anon.af498e3ca18112c3d24389f09b879115.78.llvm.7730762197114068069+0x2c>
   f7f04:	mov    $0x1c,%ecx
   f7f09:	jmp    f7f1a <<rustsht::fft::RealFftPlan>::forward+0x10a>
   f7f0b:	lea    -0xcdcee(%rip),%rax        # 2a224 <anon.af498e3ca18112c3d24389f09b879115.78.llvm.7730762197114068069+0x48>
   f7f12:	mov    $0x1a,%ecx
   f7f17:	mov    %r10,%rbx
   f7f1a:	movq   $0xc,(%rdi)
   f7f21:	mov    %rbx,0x8(%rdi)
   f7f25:	mov    %r8,0x10(%rdi)
   f7f29:	mov    %rax,0x18(%rdi)
   f7f2d:	mov    %rcx,0x20(%rdi)
   f7f31:	jmp    f83f0 <<rustsht::fft::RealFftPlan>::forward+0x5e0>
   f7f36:	mov    %r9,%r14
   f7f39:	mov    0x8(%rax),%r15
   f7f3d:	cmp    %rbx,%rcx
   f7f40:	cmovae %rbx,%rcx
   f7f44:	test   %rcx,%rcx
   f7f47:	je     f8397 <<rustsht::fft::RealFftPlan>::forward+0x587>
   f7f4d:	cmp    $0x4,%rcx
   f7f51:	jae    f830d <<rustsht::fft::RealFftPlan>::forward+0x4fd>
   f7f57:	xor    %r9d,%r9d
   f7f5a:	jmp    f8376 <<rustsht::fft::RealFftPlan>::forward+0x566>
   f7f5f:	movabs $0xffffffffffffffc,%r9
   f7f69:	or     $0x2,%r9
   f7f6d:	and    %r9,%rcx
   f7f70:	mov    %rcx,%r9
   f7f73:	shl    $0x4,%r9
   f7f77:	add    %rdx,%r9
   f7f7a:	mov    $0x10,%r11d
   f7f80:	mov    %rcx,%r12
   f7f83:	data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   f7f90:	movups -0x10(%rdx,%r11,1),%xmm0
   f7f96:	movupd (%rdx,%r11,1),%xmm1
   f7f9c:	movups %xmm0,-0x10(%r14,%r11,1)
   f7fa2:	movupd %xmm1,(%r14,%r11,1)
   f7fa8:	add    $0x20,%r11
   f7fac:	add    $0xfffffffffffffffe,%r12
   f7fb0:	jne    f7f90 <<rustsht::fft::RealFftPlan>::forward+0x180>
   f7fb2:	add    %r10,%rdx
   f7fb5:	mov    %rcx,%r10
   f7fb8:	shl    $0x4,%r10
   f7fbc:	add    %r14,%r10
   f7fbf:	dec    %rcx
   f7fc2:	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
   f7fd0:	inc    %rcx
   f7fd3:	cmp    %rcx,%rbx
   f7fd6:	je     f842a <<rustsht::fft::RealFftPlan>::forward+0x61a>
   f7fdc:	movups (%r9),%xmm0
   f7fe0:	add    $0x10,%r9
   f7fe4:	movups %xmm0,(%r10)
   f7fe8:	add    $0x10,%r10
   f7fec:	cmp    %rdx,%r9
   f7fef:	jne    f7fd0 <<rustsht::fft::RealFftPlan>::forward+0x1c0>
   f7ff1:	mov    %rdi,%rbp
   f7ff4:	mov    0x18(%rsi),%rcx
   f7ff8:	mov    %rsi,%r12
   f7ffb:	mov    0x20(%rsi),%r9
   f7fff:	mov    0x10(%r9),%rdx
   f8003:	dec    %rdx
   f8006:	and    $0xfffffffffffffff0,%rdx
   f800a:	lea    (%rcx,%rdx,1),%rdi
   f800e:	add    $0x10,%rdi
   f8012:	mov    0x20(%rax),%rcx
   f8016:	mov    %r14,%rsi
   f8019:	mov    %rbx,%rdx
   f801c:	call   *0x38(%r9)
   f8020:	test   %rbx,%rbx
   f8023:	je     f843d <<rustsht::fft::RealFftPlan>::forward+0x62d>
   f8029:	mov    %r15,%rsi
   f802c:	mov    %r15,%r11
   f802f:	shl    $0x4,%r11
   f8033:	mov    0x8(%r12),%r15
   f8038:	mov    0x10(%r12),%rcx
   f803d:	xor    %r12d,%r12d
   f8040:	movapd -0xd53c8(%rip),%xmm0        # 22c80 <anon.6dee47363b5293c3e832a61301d0e1ca.5.llvm.16897343711761133054+0x240>
   f8048:	movapd -0xd5670(%rip),%xmm3        # 229e0 <__abi_tag+0x226e4>
   f8050:	movdqa -0xd4bf8(%rip),%xmm4        # 23460 <anon.6dee47363b5293c3e832a61301d0e1ca.5.llvm.16897343711761133054+0xa20>
   f8058:	movapd -0xd4c00(%rip),%xmm5        # 23460 <anon.6dee47363b5293c3e832a61301d0e1ca.5.llvm.16897343711761133054+0xa20>
   f8060:	xor    %r13d,%r13d
   f8063:	xor    %r8d,%r8d
   f8066:	mov    %rbp,%rdi
   f8069:	mov    (%rsp),%r10
   f806d:	jmp    f8099 <<rustsht::fft::RealFftPlan>::forward+0x289>
   f806f:	nop
   f8070:	movupd %xmm2,(%r10,%r13,1)
   f8076:	add    $0x10,%r13
   f807a:	mov    %r12,%rax
   f807d:	inc    %rax
   f8080:	cmp    %rbx,%rax
   f8083:	mov    $0x0,%r12d
   f8089:	cmovne %rax,%r12
   f808d:	inc    %r8
   f8090:	cmp    %r13,%r11
   f8093:	je     f82f0 <<rustsht::fft::RealFftPlan>::forward+0x4e0>
   f8099:	mov    %r8,%rax
   f809c:	or     %rbx,%rax
   f809f:	shr    $0x20,%rax
   f80a3:	je     f80b0 <<rustsht::fft::RealFftPlan>::forward+0x2a0>
   f80a5:	mov    %r12,%r9
   f80a8:	jmp    f80ba <<rustsht::fft::RealFftPlan>::forward+0x2aa>
   f80aa:	nopw   0x0(%rax,%rax,1)
   f80b0:	mov    %r8d,%eax
   f80b3:	xor    %edx,%edx
   f80b5:	div    %ebx
   f80b7:	mov    %edx,%r9d
   f80ba:	mov    %rbx,%rax
   f80bd:	sub    %r9,%rax
   f80c0:	mov    %rax,%rdx
   f80c3:	or     %rbx,%rdx
   f80c6:	shr    $0x20,%rdx
   f80ca:	je     f80e0 <<rustsht::fft::RealFftPlan>::forward+0x2d0>
   f80cc:	xor    %edx,%edx
   f80ce:	div    %rbx
   f80d1:	cmp    %r8,%rcx
   f80d4:	jne    f80ed <<rustsht::fft::RealFftPlan>::forward+0x2dd>
   f80d6:	jmp    f8417 <<rustsht::fft::RealFftPlan>::forward+0x607>
   f80db:	nopl   0x0(%rax,%rax,1)
   f80e0:	xor    %edx,%edx
   f80e2:	div    %ebx
   f80e4:	cmp    %r8,%rcx
   f80e7:	je     f8417 <<rustsht::fft::RealFftPlan>::forward+0x607>
   f80ed:	shl    $0x4,%r9
   f80f1:	add    %r14,%r9
   f80f4:	shl    $0x4,%rdx
   f80f8:	movupd (%r9),%xmm10
   f80fd:	movupd (%r14,%rdx,1),%xmm8
   f8103:	movapd %xmm10,%xmm9
   f8108:	addpd  %xmm8,%xmm9
   f810d:	movapd %xmm10,%xmm13
   f8112:	subpd  %xmm8,%xmm13
   f8117:	movapd %xmm13,%xmm12
   f811c:	movsd  %xmm9,%xmm12
   f8121:	movupd (%r15,%r13,1),%xmm7
   f8127:	movapd %xmm12,%xmm11
   f812c:	mulpd  %xmm0,%xmm11
   f8131:	movapd %xmm7,%xmm6
   f8135:	xorpd  %xmm1,%xmm1
   f8139:	mulpd  %xmm1,%xmm6
   f813d:	movapd %xmm7,%xmm2
   f8141:	shufpd $0x1,%xmm7,%xmm2
   f8146:	mulpd  %xmm0,%xmm2
   f814a:	movapd %xmm6,%xmm14
   f814f:	addpd  %xmm2,%xmm14
   f8154:	movapd %xmm6,%xmm15
   f8159:	subpd  %xmm2,%xmm15
   f815e:	movapd %xmm15,%xmm1
   f8163:	movsd  %xmm14,%xmm1
   f8168:	movapd %xmm9,%xmm2
   f816d:	unpckhpd %xmm9,%xmm2
   f8172:	shufpd $0x1,%xmm14,%xmm15
   f8178:	mulpd  %xmm2,%xmm15
   f817d:	movapd %xmm13,%xmm2
   f8182:	unpcklpd %xmm13,%xmm2
   f8187:	mulpd  %xmm1,%xmm2
   f818b:	movapd %xmm2,%xmm1
   f818f:	subpd  %xmm15,%xmm1
   f8194:	addpd  %xmm15,%xmm2
   f8199:	movsd  %xmm1,%xmm2
   f819d:	addpd  %xmm11,%xmm2
   f81a2:	movq   %xmm2,%rax
   f81a7:	shr    $0x34,%rax
   f81ab:	and    $0x7ff,%eax
   f81b0:	cmp    $0x7ff,%eax
   f81b5:	setb   %al
   f81b8:	movdqa %xmm2,%xmm1
   f81bc:	punpckhqdq %xmm2,%xmm1
   f81c0:	movq   %xmm1,%rdx
   f81c5:	shr    $0x34,%rdx
   f81c9:	and    $0x7ff,%edx
   f81cf:	cmp    $0x7ff,%edx
   f81d5:	setb   %dl
   f81d8:	test   %dl,%al
   f81da:	jne    f8070 <<rustsht::fft::RealFftPlan>::forward+0x260>
   f81e0:	movsd  %xmm13,%xmm9
   f81e5:	movapd %xmm10,%xmm14
   f81ea:	mulpd  %xmm0,%xmm14
   f81ef:	movapd %xmm8,%xmm2
   f81f4:	mulpd  -0xd55fc(%rip),%xmm2        # 22c00 <anon.6dee47363b5293c3e832a61301d0e1ca.5.llvm.16897343711761133054+0x1c0>
   f81fc:	movapd %xmm9,%xmm15
   f8201:	mulpd  %xmm0,%xmm15
   f8206:	andpd  %xmm3,%xmm12
   f820b:	pcmpeqd %xmm4,%xmm12
   f8210:	pshufd $0xb1,%xmm12,%xmm13
   f8216:	pand   %xmm12,%xmm13
   f821b:	andpd  %xmm3,%xmm10
   f8220:	movapd %xmm10,%xmm1
   f8225:	cmpneqpd %xmm5,%xmm1
   f822a:	cmpordpd %xmm5,%xmm10
   f8230:	andpd  %xmm1,%xmm10
   f8235:	andpd  %xmm3,%xmm8
   f823a:	movapd %xmm8,%xmm1
   f823f:	cmpneqpd %xmm5,%xmm1
   f8244:	cmpordpd %xmm5,%xmm8
   f824a:	andpd  %xmm1,%xmm8
   f824f:	andpd  %xmm10,%xmm8
   f8254:	pand   %xmm8,%xmm13
   f8259:	movapd %xmm14,%xmm1
   f825e:	addpd  %xmm2,%xmm1
   f8262:	andpd  %xmm13,%xmm1
   f8267:	andnpd %xmm11,%xmm13
   f826c:	orpd   %xmm1,%xmm13
   f8271:	andpd  %xmm3,%xmm9
   f8276:	pcmpeqd %xmm4,%xmm9
   f827b:	pshufd $0xb1,%xmm9,%xmm1
   f8281:	pand   %xmm9,%xmm1
   f8286:	pand   %xmm8,%xmm1
   f828b:	subpd  %xmm2,%xmm14
   f8290:	andpd  %xmm1,%xmm14
   f8295:	andnpd %xmm15,%xmm1
   f829a:	orpd   %xmm14,%xmm1
   f829f:	movapd %xmm6,%xmm2
   f82a3:	unpcklpd %xmm6,%xmm2
   f82a7:	movapd %xmm7,%xmm8
   f82ac:	unpckhpd %xmm7,%xmm8
   f82b1:	addpd  %xmm2,%xmm8
   f82b6:	mulpd  %xmm1,%xmm8
   f82bb:	pshufd $0x4e,%xmm1,%xmm1
   f82c0:	unpckhpd %xmm6,%xmm6
   f82c4:	unpcklpd %xmm7,%xmm7
   f82c8:	subpd  %xmm7,%xmm6
   f82cc:	mulpd  %xmm1,%xmm6
   f82d0:	movapd %xmm8,%xmm1
   f82d5:	subpd  %xmm6,%xmm1
   f82d9:	addpd  %xmm8,%xmm6
   f82de:	movsd  %xmm1,%xmm6
   f82e2:	addpd  %xmm13,%xmm6
   f82e7:	movapd %xmm6,%xmm2
   f82eb:	jmp    f8070 <<rustsht::fft::RealFftPlan>::forward+0x260>
   f82f0:	movq   $0x0,0x8(%r10)
   f82f8:	cmp    %rsi,%rbx
   f82fb:	jae    f844a <<rustsht::fft::RealFftPlan>::forward+0x63a>
   f8301:	shl    $0x4,%rbx
   f8305:	add    %rbx,%r10
   f8308:	jmp    f83e1 <<rustsht::fft::RealFftPlan>::forward+0x5d1>
   f830d:	movabs $0xffffffffffffffc,%r9
   f8317:	and    %rcx,%r9
   f831a:	lea    0x0(,%rcx,8),%r10
   f8322:	and    $0xffffffffffffffe0,%r10
   f8326:	xor    %r11d,%r11d
   f8329:	pxor   %xmm0,%xmm0
   f832d:	nopl   (%rax)
   f8330:	movdqu (%rdx,%r11,1),%xmm1
   f8336:	movdqu 0x10(%rdx,%r11,1),%xmm2
   f833d:	movq   %xmm1,%xmm3
   f8341:	punpckhqdq %xmm0,%xmm1
   f8345:	movdqu %xmm1,0x10(%r15,%r11,2)
   f834c:	movdqu %xmm3,(%r15,%r11,2)
   f8352:	movq   %xmm2,%xmm1
   f8356:	punpckhqdq %xmm0,%xmm2
   f835a:	movdqu %xmm2,0x30(%r15,%r11,2)
   f8361:	movdqu %xmm1,0x20(%r15,%r11,2)
   f8368:	add    $0x20,%r11
   f836c:	cmp    %r11,%r10
   f836f:	jne    f8330 <<rustsht::fft::RealFftPlan>::forward+0x520>
   f8371:	cmp    %r9,%rcx
   f8374:	je     f8397 <<rustsht::fft::RealFftPlan>::forward+0x587>
   f8376:	mov    %r9,%r10
   f8379:	shl    $0x4,%r10
   f837d:	add    %r15,%r10
   f8380:	movq   (%rdx,%r9,8),%xmm0
   f8386:	inc    %r9
   f8389:	movdqu %xmm0,(%r10)
   f838e:	add    $0x10,%r10
   f8392:	cmp    %r9,%rcx
   f8395:	jne    f8380 <<rustsht::fft::RealFftPlan>::forward+0x570>
   f8397:	mov    %rdi,%rbp
   f839a:	mov    0x8(%rsi),%rcx
   f839e:	mov    0x10(%rsi),%r9
   f83a2:	mov    0x10(%r9),%rdx
   f83a6:	dec    %rdx
   f83a9:	and    $0xfffffffffffffff0,%rdx
   f83ad:	lea    (%rcx,%rdx,1),%rdi
   f83b1:	add    $0x10,%rdi
   f83b5:	mov    0x20(%rax),%rcx
   f83b9:	mov    %r15,%rsi
   f83bc:	mov    %rbx,%rdx
   f83bf:	call   *0x38(%r9)
   f83c3:	cmp    %rbx,%r12
   f83c6:	jae    f8402 <<rustsht::fft::RealFftPlan>::forward+0x5f2>
   f83c8:	shl    $0x4,%r14
   f83cc:	mov    %r13,%rdi
   f83cf:	mov    %r15,%rsi
   f83d2:	mov    %r14,%rdx
   f83d5:	call   *0x19a5a5(%rip)        # 292980 <memcpy@GLIBC_2.14>
   f83db:	mov    %r13,%r10
   f83de:	mov    %rbp,%rdi
   f83e1:	movq   $0x0,0x8(%r10)
   f83e9:	movq   $0xffffffffffffffff,(%rdi)
   f83f0:	mov    %rdi,%rax
   f83f3:	add    $0x8,%rsp
   f83f7:	pop    %rbx
   f83f8:	pop    %r12
   f83fa:	pop    %r13
   f83fc:	pop    %r14
   f83fe:	pop    %r15
   f8400:	pop    %rbp
   f8401:	ret
   f8402:	lea    0x18b5cf(%rip),%rcx        # 2839d8 <anon.af498e3ca18112c3d24389f09b879115.82.llvm.7730762197114068069+0x228>
   f8409:	xor    %edi,%edi
   f840b:	mov    %r14,%rsi
   f840e:	mov    %rbx,%rdx
   f8411:	call   *0x19a5e1(%rip)        # 2929f8 <_DYNAMIC+0x348>
   f8417:	lea    0x18b58a(%rip),%rdx        # 2839a8 <anon.af498e3ca18112c3d24389f09b879115.82.llvm.7730762197114068069+0x1f8>
   f841e:	mov    %rcx,%rdi
   f8421:	mov    %rcx,%rsi
   f8424:	call   *0x19a546(%rip)        # 292970 <_DYNAMIC+0x2c0>
   f842a:	lea    0x18b58f(%rip),%rdx        # 2839c0 <anon.af498e3ca18112c3d24389f09b879115.82.llvm.7730762197114068069+0x210>
   f8431:	mov    %rbx,%rdi
   f8434:	mov    %rbx,%rsi
   f8437:	call   *0x19a533(%rip)        # 292970 <_DYNAMIC+0x2c0>
   f843d:	lea    0x18b54c(%rip),%rdi        # 283990 <anon.af498e3ca18112c3d24389f09b879115.82.llvm.7730762197114068069+0x1e0>
   f8444:	call   *0x19a4be(%rip)        # 292908 <_DYNAMIC+0x258>
   f844a:	lea    0x18b527(%rip),%rdx        # 283978 <anon.af498e3ca18112c3d24389f09b879115.82.llvm.7730762197114068069+0x1c8>
   f8451:	mov    %rbx,%rdi
   f8454:	call   *0x19a516(%rip)        # 292970 <_DYNAMIC+0x2c0>

Disassembly of section .init:

Disassembly of section .fini:

Disassembly of section .plt:
