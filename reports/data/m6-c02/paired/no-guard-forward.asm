
target/m6-c02-no-guard-bench:     file format elf64-x86-64


Disassembly of section .text:

0000000000103b50 <<rustsht::fft::RealFftPlan>::forward>:
  103b50:	push   %rbp
  103b51:	push   %r15
  103b53:	push   %r14
  103b55:	push   %r13
  103b57:	push   %r12
  103b59:	push   %rbx
  103b5a:	push   %rax
  103b5b:	mov    0x38(%rsi),%rax
  103b5f:	cmp    %rax,%rcx
  103b62:	jne    103bef <<rustsht::fft::RealFftPlan>::forward+0x9f>
  103b68:	mov    %rcx,%r12
  103b6b:	shr    $1,%r12
  103b6e:	lea    0x1(%r12),%rax
  103b73:	cmp    %rax,%r9
  103b76:	jne    103c16 <<rustsht::fft::RealFftPlan>::forward+0xc6>
  103b7c:	mov    %r8,%r13
  103b7f:	mov    0x40(%rsp),%rax
  103b84:	mov    0x40(%rsi),%rbx
  103b88:	mov    0x10(%rax),%r8
  103b8c:	cmp    %rbx,%r8
  103b8f:	jne    103c3d <<rustsht::fft::RealFftPlan>::forward+0xed>
  103b95:	mov    0x28(%rax),%r8
  103b99:	mov    0x48(%rsi),%r10
  103b9d:	cmp    %r10,%r8
  103ba0:	jne    103c4b <<rustsht::fft::RealFftPlan>::forward+0xfb>
  103ba6:	cmpq   $0xffffffffffffffff,(%rsi)
  103baa:	je     103c76 <<rustsht::fft::RealFftPlan>::forward+0x126>
  103bb0:	mov    %r13,(%rsp)
  103bb4:	mov    %r9,%r15
  103bb7:	shl    $0x3,%rcx
  103bbb:	mov    %rcx,%r10
  103bbe:	mov    0x8(%rax),%r14
  103bc2:	and    $0xfffffffffffffff0,%r10
  103bc6:	je     103d31 <<rustsht::fft::RealFftPlan>::forward+0x1e1>
  103bcc:	add    $0xfffffffffffffff0,%rcx
  103bd0:	shr    $0x4,%rcx
  103bd4:	cmp    %rcx,%rbx
  103bd7:	cmovb  %rbx,%rcx
  103bdb:	cmp    $0x4,%rcx
  103bdf:	jae    103c9f <<rustsht::fft::RealFftPlan>::forward+0x14f>
  103be5:	xor    %ecx,%ecx
  103be7:	mov    %rdx,%r9
  103bea:	jmp    103cf2 <<rustsht::fft::RealFftPlan>::forward+0x1a2>
  103bef:	movq   $0xc,(%rdi)
  103bf6:	mov    %rax,0x8(%rdi)
  103bfa:	mov    %rcx,0x10(%rdi)
  103bfe:	lea    -0xd97db(%rip),%rax        # 2a42a <anon.751df2bd59b83090bba343a15bf9562d.78.llvm.13745496055053099804+0x62>
  103c05:	mov    %rax,0x18(%rdi)
  103c09:	movq   $0xe,0x20(%rdi)
  103c11:	jmp    103fa0 <<rustsht::fft::RealFftPlan>::forward+0x450>
  103c16:	movq   $0xc,(%rdi)
  103c1d:	mov    %rax,0x8(%rdi)
  103c21:	mov    %r9,0x10(%rdi)
  103c25:	lea    -0xd98d4(%rip),%rax        # 2a358 <anon.60a84c4e5ed5f0b049e31ead5ba5dea3.50.llvm.13788710948842874667+0x274>
  103c2c:	mov    %rax,0x18(%rdi)
  103c30:	movq   $0x11,0x20(%rdi)
  103c38:	jmp    103fa0 <<rustsht::fft::RealFftPlan>::forward+0x450>
  103c3d:	lea    -0xd9850(%rip),%rax        # 2a3f4 <anon.751df2bd59b83090bba343a15bf9562d.78.llvm.13745496055053099804+0x2c>
  103c44:	mov    $0x1c,%ecx
  103c49:	jmp    103c5a <<rustsht::fft::RealFftPlan>::forward+0x10a>
  103c4b:	lea    -0xd9842(%rip),%rax        # 2a410 <anon.751df2bd59b83090bba343a15bf9562d.78.llvm.13745496055053099804+0x48>
  103c52:	mov    $0x1a,%ecx
  103c57:	mov    %r10,%rbx
  103c5a:	movq   $0xc,(%rdi)
  103c61:	mov    %rbx,0x8(%rdi)
  103c65:	mov    %r8,0x10(%rdi)
  103c69:	mov    %rax,0x18(%rdi)
  103c6d:	mov    %rcx,0x20(%rdi)
  103c71:	jmp    103fa0 <<rustsht::fft::RealFftPlan>::forward+0x450>
  103c76:	mov    %r9,%r14
  103c79:	mov    0x8(%rax),%r15
  103c7d:	cmp    %rbx,%rcx
  103c80:	cmovae %rbx,%rcx
  103c84:	test   %rcx,%rcx
  103c87:	je     103f47 <<rustsht::fft::RealFftPlan>::forward+0x3f7>
  103c8d:	cmp    $0x4,%rcx
  103c91:	jae    103eb6 <<rustsht::fft::RealFftPlan>::forward+0x366>
  103c97:	xor    %r9d,%r9d
  103c9a:	jmp    103f26 <<rustsht::fft::RealFftPlan>::forward+0x3d6>
  103c9f:	movabs $0xffffffffffffffc,%r9
  103ca9:	or     $0x2,%r9
  103cad:	and    %r9,%rcx
  103cb0:	mov    %rcx,%r9
  103cb3:	shl    $0x4,%r9
  103cb7:	add    %rdx,%r9
  103cba:	mov    $0x10,%r11d
  103cc0:	mov    %rcx,%r12
  103cc3:	data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
  103cd0:	movups -0x10(%rdx,%r11,1),%xmm0
  103cd6:	movupd (%rdx,%r11,1),%xmm1
  103cdc:	movups %xmm0,-0x10(%r14,%r11,1)
  103ce2:	movupd %xmm1,(%r14,%r11,1)
  103ce8:	add    $0x20,%r11
  103cec:	add    $0xfffffffffffffffe,%r12
  103cf0:	jne    103cd0 <<rustsht::fft::RealFftPlan>::forward+0x180>
  103cf2:	add    %r10,%rdx
  103cf5:	mov    %rcx,%r10
  103cf8:	shl    $0x4,%r10
  103cfc:	add    %r14,%r10
  103cff:	dec    %rcx
  103d02:	data16 data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
  103d10:	inc    %rcx
  103d13:	cmp    %rcx,%rbx
  103d16:	je     103fda <<rustsht::fft::RealFftPlan>::forward+0x48a>
  103d1c:	movups (%r9),%xmm0
  103d20:	add    $0x10,%r9
  103d24:	movups %xmm0,(%r10)
  103d28:	add    $0x10,%r10
  103d2c:	cmp    %rdx,%r9
  103d2f:	jne    103d10 <<rustsht::fft::RealFftPlan>::forward+0x1c0>
  103d31:	mov    %rdi,%rbp
  103d34:	mov    0x18(%rsi),%rcx
  103d38:	mov    %rsi,%r12
  103d3b:	mov    0x20(%rsi),%r9
  103d3f:	mov    0x10(%r9),%rdx
  103d43:	dec    %rdx
  103d46:	and    $0xfffffffffffffff0,%rdx
  103d4a:	lea    (%rcx,%rdx,1),%rdi
  103d4e:	add    $0x10,%rdi
  103d52:	mov    0x20(%rax),%rcx
  103d56:	mov    %r14,%rsi
  103d59:	mov    %rbx,%rdx
  103d5c:	call   *0x38(%r9)
  103d60:	test   %rbx,%rbx
  103d63:	je     103fed <<rustsht::fft::RealFftPlan>::forward+0x49d>
  103d69:	mov    %r15,%rsi
  103d6c:	mov    %r15,%r11
  103d6f:	shl    $0x4,%r11
  103d73:	mov    0x8(%r12),%r15
  103d78:	mov    0x10(%r12),%rcx
  103d7d:	xor    %r13d,%r13d
  103d80:	movapd -0xe1108(%rip),%xmm0        # 22c80 <anon.6dee47363b5293c3e832a61301d0e1ca.5.llvm.16897343711761133054+0x240>
  103d88:	xorpd  %xmm1,%xmm1
  103d8c:	xor    %r12d,%r12d
  103d8f:	xor    %r8d,%r8d
  103d92:	mov    %rbp,%rdi
  103d95:	mov    (%rsp),%r10
  103d99:	jmp    103e4a <<rustsht::fft::RealFftPlan>::forward+0x2fa>
  103d9e:	xchg   %ax,%ax
  103da0:	xor    %edx,%edx
  103da2:	div    %rbx
  103da5:	shl    $0x4,%r9
  103da9:	mov    %r13,%rax
  103dac:	inc    %rax
  103daf:	cmp    %rbx,%rax
  103db2:	mov    $0x0,%r13d
  103db8:	cmovne %rax,%r13
  103dbc:	inc    %r8
  103dbf:	shl    $0x4,%rdx
  103dc3:	movupd (%r14,%r9,1),%xmm2
  103dc9:	movupd (%r14,%rdx,1),%xmm3
  103dcf:	movupd (%r15,%r12,1),%xmm4
  103dd5:	movapd %xmm2,%xmm5
  103dd9:	addpd  %xmm3,%xmm5
  103ddd:	subpd  %xmm3,%xmm2
  103de1:	movapd %xmm2,%xmm3
  103de5:	movsd  %xmm5,%xmm3
  103de9:	mulpd  %xmm0,%xmm3
  103ded:	movapd %xmm4,%xmm6
  103df1:	mulpd  %xmm1,%xmm6
  103df5:	shufpd $0x1,%xmm4,%xmm4
  103dfa:	mulpd  %xmm0,%xmm4
  103dfe:	movapd %xmm6,%xmm7
  103e02:	addpd  %xmm4,%xmm7
  103e06:	subpd  %xmm4,%xmm6
  103e0a:	movapd %xmm6,%xmm4
  103e0e:	movsd  %xmm7,%xmm4
  103e12:	unpckhpd %xmm5,%xmm5
  103e16:	shufpd $0x1,%xmm7,%xmm6
  103e1b:	mulpd  %xmm5,%xmm6
  103e1f:	unpcklpd %xmm2,%xmm2
  103e23:	mulpd  %xmm4,%xmm2
  103e27:	movapd %xmm2,%xmm4
  103e2b:	subpd  %xmm6,%xmm4
  103e2f:	addpd  %xmm6,%xmm2
  103e33:	movsd  %xmm4,%xmm2
  103e37:	addpd  %xmm3,%xmm2
  103e3b:	movupd %xmm2,(%r10,%r12,1)
  103e41:	add    $0x10,%r12
  103e45:	cmp    %r12,%r11
  103e48:	je     103e99 <<rustsht::fft::RealFftPlan>::forward+0x349>
  103e4a:	cmp    %r8,%rcx
  103e4d:	je     103fc7 <<rustsht::fft::RealFftPlan>::forward+0x477>
  103e53:	mov    %r8,%rax
  103e56:	or     %rbx,%rax
  103e59:	shr    $0x20,%rax
  103e5d:	je     103e70 <<rustsht::fft::RealFftPlan>::forward+0x320>
  103e5f:	mov    %r13,%r9
  103e62:	jmp    103e7a <<rustsht::fft::RealFftPlan>::forward+0x32a>
  103e64:	data16 data16 cs nopw 0x0(%rax,%rax,1)
  103e70:	mov    %r8d,%eax
  103e73:	xor    %edx,%edx
  103e75:	div    %ebx
  103e77:	mov    %edx,%r9d
  103e7a:	mov    %rbx,%rax
  103e7d:	sub    %r9,%rax
  103e80:	mov    %rax,%rdx
  103e83:	or     %rbx,%rdx
  103e86:	shr    $0x20,%rdx
  103e8a:	jne    103da0 <<rustsht::fft::RealFftPlan>::forward+0x250>
  103e90:	xor    %edx,%edx
  103e92:	div    %ebx
  103e94:	jmp    103da5 <<rustsht::fft::RealFftPlan>::forward+0x255>
  103e99:	movq   $0x0,0x8(%r10)
  103ea1:	cmp    %rsi,%rbx
  103ea4:	jae    103ffa <<rustsht::fft::RealFftPlan>::forward+0x4aa>
  103eaa:	shl    $0x4,%rbx
  103eae:	add    %rbx,%r10
  103eb1:	jmp    103f91 <<rustsht::fft::RealFftPlan>::forward+0x441>
  103eb6:	movabs $0xffffffffffffffc,%r9
  103ec0:	and    %rcx,%r9
  103ec3:	lea    0x0(,%rcx,8),%r10
  103ecb:	and    $0xffffffffffffffe0,%r10
  103ecf:	xor    %r11d,%r11d
  103ed2:	pxor   %xmm0,%xmm0
  103ed6:	cs nopw 0x0(%rax,%rax,1)
  103ee0:	movdqu (%rdx,%r11,1),%xmm1
  103ee6:	movdqu 0x10(%rdx,%r11,1),%xmm2
  103eed:	movq   %xmm1,%xmm3
  103ef1:	punpckhqdq %xmm0,%xmm1
  103ef5:	movdqu %xmm1,0x10(%r15,%r11,2)
  103efc:	movdqu %xmm3,(%r15,%r11,2)
  103f02:	movq   %xmm2,%xmm1
  103f06:	punpckhqdq %xmm0,%xmm2
  103f0a:	movdqu %xmm2,0x30(%r15,%r11,2)
  103f11:	movdqu %xmm1,0x20(%r15,%r11,2)
  103f18:	add    $0x20,%r11
  103f1c:	cmp    %r11,%r10
  103f1f:	jne    103ee0 <<rustsht::fft::RealFftPlan>::forward+0x390>
  103f21:	cmp    %r9,%rcx
  103f24:	je     103f47 <<rustsht::fft::RealFftPlan>::forward+0x3f7>
  103f26:	mov    %r9,%r10
  103f29:	shl    $0x4,%r10
  103f2d:	add    %r15,%r10
  103f30:	movq   (%rdx,%r9,8),%xmm0
  103f36:	inc    %r9
  103f39:	movdqu %xmm0,(%r10)
  103f3e:	add    $0x10,%r10
  103f42:	cmp    %r9,%rcx
  103f45:	jne    103f30 <<rustsht::fft::RealFftPlan>::forward+0x3e0>
  103f47:	mov    %rdi,%rbp
  103f4a:	mov    0x8(%rsi),%rcx
  103f4e:	mov    0x10(%rsi),%r9
  103f52:	mov    0x10(%r9),%rdx
  103f56:	dec    %rdx
  103f59:	and    $0xfffffffffffffff0,%rdx
  103f5d:	lea    (%rcx,%rdx,1),%rdi
  103f61:	add    $0x10,%rdi
  103f65:	mov    0x20(%rax),%rcx
  103f69:	mov    %r15,%rsi
  103f6c:	mov    %rbx,%rdx
  103f6f:	call   *0x38(%r9)
  103f73:	cmp    %rbx,%r12
  103f76:	jae    103fb2 <<rustsht::fft::RealFftPlan>::forward+0x462>
  103f78:	shl    $0x4,%r14
  103f7c:	mov    %r13,%rdi
  103f7f:	mov    %r15,%rsi
  103f82:	mov    %r14,%rdx
  103f85:	call   *0x18e855(%rip)        # 2927e0 <memcpy@GLIBC_2.14>
  103f8b:	mov    %r13,%r10
  103f8e:	mov    %rbp,%rdi
  103f91:	movq   $0x0,0x8(%r10)
  103f99:	movq   $0xffffffffffffffff,(%rdi)
  103fa0:	mov    %rdi,%rax
  103fa3:	add    $0x8,%rsp
  103fa7:	pop    %rbx
  103fa8:	pop    %r12
  103faa:	pop    %r13
  103fac:	pop    %r14
  103fae:	pop    %r15
  103fb0:	pop    %rbp
  103fb1:	ret
  103fb2:	lea    0x17fd5f(%rip),%rcx        # 283d18 <anon.751df2bd59b83090bba343a15bf9562d.82.llvm.13745496055053099804+0x228>
  103fb9:	xor    %edi,%edi
  103fbb:	mov    %r14,%rsi
  103fbe:	mov    %rbx,%rdx
  103fc1:	call   *0x18e891(%rip)        # 292858 <_DYNAMIC+0x348>
  103fc7:	lea    0x17fd1a(%rip),%rdx        # 283ce8 <anon.751df2bd59b83090bba343a15bf9562d.82.llvm.13745496055053099804+0x1f8>
  103fce:	mov    %rcx,%rdi
  103fd1:	mov    %rcx,%rsi
  103fd4:	call   *0x18e7f6(%rip)        # 2927d0 <_DYNAMIC+0x2c0>
  103fda:	lea    0x17fd1f(%rip),%rdx        # 283d00 <anon.751df2bd59b83090bba343a15bf9562d.82.llvm.13745496055053099804+0x210>
  103fe1:	mov    %rbx,%rdi
  103fe4:	mov    %rbx,%rsi
  103fe7:	call   *0x18e7e3(%rip)        # 2927d0 <_DYNAMIC+0x2c0>
  103fed:	lea    0x17fcdc(%rip),%rdi        # 283cd0 <anon.751df2bd59b83090bba343a15bf9562d.82.llvm.13745496055053099804+0x1e0>
  103ff4:	call   *0x18e76e(%rip)        # 292768 <_DYNAMIC+0x258>
  103ffa:	lea    0x17fcb7(%rip),%rdx        # 283cb8 <anon.751df2bd59b83090bba343a15bf9562d.82.llvm.13745496055053099804+0x1c8>
  104001:	mov    %rbx,%rdi
  104004:	call   *0x18e7c6(%rip)        # 2927d0 <_DYNAMIC+0x2c0>

Disassembly of section .init:

Disassembly of section .fini:

Disassembly of section .plt:
