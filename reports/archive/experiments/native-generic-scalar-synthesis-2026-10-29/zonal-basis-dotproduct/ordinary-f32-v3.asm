
candidate ELF: file format elf64-x86-64


Disassembly of section .text:

00000000001eac60 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>>:
  1eac60:	push   %rbp
  1eac61:	push   %r15
  1eac63:	push   %r14
  1eac65:	push   %r13
  1eac67:	push   %r12
  1eac69:	push   %rbx
  1eac6a:	sub    $0x28,%rsp
  1eac6e:	cmp    %rsi,%rcx
  1eac71:	seta   %al
  1eac74:	mov    %rdx,%rcx
  1eac77:	sub    %r8,%rcx
  1eac7a:	setbe  %sil
  1eac7e:	or     %al,%sil
  1eac81:	mov    $0x1,%al
  1eac83:	jne    1ead27 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0xc7>
  1eac89:	mov    0x60(%rsp),%rsi
  1eac8e:	mov    0x78(%rdi),%r10
  1eac92:	mov    %rsi,%r11
  1eac95:	sub    %r9,%r11
  1eac98:	jb     1eaf59 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x2f9>
  1eac9e:	cmp    %r10,%rsi
  1eaca1:	ja     1eaf59 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x2f9>
  1eaca7:	cmp    $0x8,%r11
  1eacab:	jne    1ead25 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0xc5>
  1eacad:	mov    0x88(%rdi),%r10
  1eacb4:	cmp    %r10,%rsi
  1eacb7:	ja     1eaf6c <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x30c>
  1eacbd:	mov    0x98(%rdi),%r10
  1eacc4:	cmp    %r10,%rsi
  1eacc7:	ja     1eaf7f <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x31f>
  1eaccd:	mov    0xc8(%rdi),%r10
  1eacd4:	cmp    %r10,%rsi
  1eacd7:	ja     1eaf92 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x332>
  1eacdd:	mov    0xd8(%rdi),%r10
  1eace4:	cmp    %r10,%rsi
  1eace7:	ja     1eafa5 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x345>
  1eaced:	mov    0xe8(%rdi),%r10
  1eacf4:	cmp    %r10,%rsi
  1eacf7:	ja     1eafb8 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x358>
  1eacfd:	mov    0xf8(%rdi),%r10
  1ead04:	cmp    %r10,%rsi
  1ead07:	ja     1eafcb <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x36b>
  1ead0d:	cmp    0x28(%rdi),%rdx
  1ead11:	ja     1ead25 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0xc5>
  1ead13:	cmp    0x38(%rdi),%rdx
  1ead17:	ja     1ead25 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0xc5>
  1ead19:	cmp    0x48(%rdi),%rdx
  1ead1d:	ja     1ead25 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0xc5>
  1ead1f:	cmp    0x58(%rdi),%rdx
  1ead23:	jbe    1ead39 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0xd9>
  1ead25:	xor    %eax,%eax
  1ead27:	add    $0x28,%rsp
  1ead2b:	pop    %rbx
  1ead2c:	pop    %r12
  1ead2e:	pop    %r13
  1ead30:	pop    %r14
  1ead32:	pop    %r15
  1ead34:	pop    %rbp
  1ead35:	vzeroupper
  1ead38:	ret
  1ead39:	mov    0x70(%rdi),%rdx
  1ead3d:	mov    0x20(%rdi),%r14
  1ead41:	mov    0x30(%rdi),%r15
  1ead45:	mov    0x40(%rdi),%r12
  1ead49:	mov    0x50(%rdi),%r13
  1ead4d:	vmovups (%rdx,%r9,4),%ymm1
  1ead53:	mov    0x80(%rdi),%rdx
  1ead5a:	mov    %rdx,(%rsp)
  1ead5e:	vmovups (%rdx,%r9,4),%ymm0
  1ead64:	mov    0x90(%rdi),%rsi
  1ead6b:	vmovups (%rsi,%r9,4),%ymm2
  1ead71:	mov    0xc0(%rdi),%r10
  1ead78:	vmovups (%r10,%r9,4),%ymm10
  1ead7e:	mov    0xd0(%rdi),%r11
  1ead85:	vmovups (%r11,%r9,4),%ymm9
  1ead8b:	mov    0xe0(%rdi),%rbx
  1ead92:	vmovups (%rbx,%r9,4),%ymm8
  1ead98:	mov    0xf0(%rdi),%rdi
  1ead9f:	vmovups (%rdi,%r9,4),%ymm7
  1eada5:	mov    %rcx,%rbp
  1eada8:	shr    $1,%rbp
  1eadab:	mov    %r14,0x20(%rsp)
  1eadb0:	mov    %r15,0x18(%rsp)
  1eadb5:	mov    %r12,0x10(%rsp)
  1eadba:	mov    %r13,0x8(%rsp)
  1eadbf:	je     1eaea0 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x240>
  1eadc5:	lea    (%r14,%r8,4),%r14
  1eadc9:	add    $0x4,%r14
  1eadcd:	lea    (%r15,%r8,4),%r15
  1eadd1:	add    $0x4,%r15
  1eadd5:	lea    (%r12,%r8,8),%r12
  1eadd9:	add    $0xc,%r12
  1eaddd:	lea    0xc(%r13,%r8,8),%r13
  1eade2:	xor    %edx,%edx
  1eade4:	data16 data16 cs nopw 0x0(%rax,%rax,1)
  1eadf0:	vmovaps %ymm2,%ymm11
  1eadf4:	vbroadcastss -0x4(%r14,%rdx,1),%ymm13
  1eadfb:	vbroadcastss -0x4(%r15,%rdx,1),%ymm14
  1eae02:	vbroadcastss (%r14,%rdx,1),%ymm12
  1eae08:	vbroadcastss -0xc(%r12,%rdx,2),%ymm3
  1eae0f:	vfmadd213ps %ymm10,%ymm2,%ymm3
  1eae14:	vbroadcastss -0x8(%r12,%rdx,2),%ymm4
  1eae1b:	vfmadd213ps %ymm9,%ymm2,%ymm4
  1eae20:	vbroadcastss -0xc(%r13,%rdx,2),%ymm5
  1eae27:	vfmadd213ps %ymm8,%ymm2,%ymm5
  1eae2c:	vbroadcastss -0x8(%r13,%rdx,2),%ymm6
  1eae33:	vfmadd213ps %ymm7,%ymm2,%ymm6
  1eae38:	vbroadcastss (%r15,%rdx,1),%ymm2
  1eae3e:	vfmadd231ps %ymm13,%ymm1,%ymm14
  1eae43:	vbroadcastss -0x4(%r12,%rdx,2),%ymm7
  1eae4a:	vfmadd231ps %ymm14,%ymm11,%ymm0
  1eae4f:	vbroadcastss (%r12,%rdx,2),%ymm8
  1eae55:	vfmadd231ps %ymm7,%ymm0,%ymm3
  1eae5a:	vbroadcastss -0x4(%r13,%rdx,2),%ymm7
  1eae61:	vfmadd231ps %ymm8,%ymm0,%ymm4
  1eae66:	vbroadcastss 0x0(%r13,%rdx,2),%ymm8
  1eae6d:	vfmadd231ps %ymm7,%ymm0,%ymm5
  1eae72:	vfmadd231ps %ymm8,%ymm0,%ymm6
  1eae77:	vfmadd231ps %ymm12,%ymm1,%ymm2
  1eae7c:	vfmadd213ps %ymm11,%ymm0,%ymm2
  1eae81:	add    $0x8,%rdx
  1eae85:	vmovaps %ymm6,%ymm7
  1eae89:	vmovaps %ymm5,%ymm8
  1eae8d:	vmovaps %ymm4,%ymm9
  1eae91:	vmovaps %ymm3,%ymm10
  1eae95:	dec    %rbp
  1eae98:	jne    1eadf0 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x190>
  1eae9e:	jmp    1eaeb0 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x250>
  1eaea0:	vmovaps %ymm10,%ymm3
  1eaea4:	vmovaps %ymm9,%ymm4
  1eaea8:	vmovaps %ymm8,%ymm5
  1eaeac:	vmovaps %ymm7,%ymm6
  1eaeb0:	mov    %rcx,%rdx
  1eaeb3:	and    $0xfffffffffffffffe,%rdx
  1eaeb7:	cmp    %rcx,%rdx
  1eaeba:	je     1eaf2c <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0x2cc>
  1eaebc:	mov    0x20(%rsp),%rcx
  1eaec1:	lea    (%rcx,%r8,4),%rcx
  1eaec5:	mov    0x18(%rsp),%r14
  1eaeca:	lea    (%r14,%r8,4),%r14
  1eaece:	mov    0x10(%rsp),%r15
  1eaed3:	lea    (%r15,%r8,8),%r15
  1eaed7:	mov    0x8(%rsp),%r12
  1eaedc:	lea    (%r12,%r8,8),%r8
  1eaee0:	vbroadcastss (%rcx,%rdx,4),%ymm7
  1eaee6:	vbroadcastss (%r14,%rdx,4),%ymm8
  1eaeec:	vbroadcastss (%r15,%rdx,8),%ymm9
  1eaef2:	vbroadcastss 0x4(%r15,%rdx,8),%ymm10
  1eaef9:	vbroadcastss (%r8,%rdx,8),%ymm11
  1eaeff:	vbroadcastss 0x4(%r8,%rdx,8),%ymm12
  1eaf06:	vfmadd231ps %ymm9,%ymm2,%ymm3
  1eaf0b:	vfmadd231ps %ymm10,%ymm2,%ymm4
  1eaf10:	vfmadd231ps %ymm11,%ymm2,%ymm5
  1eaf15:	vfmadd231ps %ymm12,%ymm2,%ymm6
  1eaf1a:	vfmadd213ps %ymm8,%ymm7,%ymm1
  1eaf1f:	vfmadd213ps %ymm0,%ymm2,%ymm1
  1eaf24:	vmovaps %ymm2,%ymm0
  1eaf28:	vmovaps %ymm1,%ymm2
  1eaf2c:	mov    (%rsp),%rcx
  1eaf30:	vmovups %ymm0,(%rcx,%r9,4)
  1eaf36:	vmovups %ymm2,(%rsi,%r9,4)
  1eaf3c:	vmovups %ymm3,(%r10,%r9,4)
  1eaf42:	vmovups %ymm4,(%r11,%r9,4)
  1eaf48:	vmovups %ymm5,(%rbx,%r9,4)
  1eaf4e:	vmovups %ymm6,(%rdi,%r9,4)
  1eaf54:	jmp    1ead27 <rustsht::legendre::scalar_ordered::kernel::synthesize_ordinary_suffix_vector::<f32, pulp::x86::v3::V3>+0xc7>
  1eaf59:	lea    0xb44f0(%rip),%rcx        # 29f450 <anon.f09c3a3007160cecd1a5798135a651ae.40.llvm.11416580622004461126+0x848>
  1eaf60:	mov    %r9,%rdi
  1eaf63:	mov    %r10,%rdx
  1eaf66:	call   *0xbd734(%rip)        # 2a86a0 <_DYNAMIC+0x2b0>
  1eaf6c:	lea    0xb444d(%rip),%rcx        # 29f3c0 <anon.f09c3a3007160cecd1a5798135a651ae.40.llvm.11416580622004461126+0x7b8>
  1eaf73:	mov    %r9,%rdi
  1eaf76:	mov    %r10,%rdx
  1eaf79:	call   *0xbd721(%rip)        # 2a86a0 <_DYNAMIC+0x2b0>
  1eaf7f:	lea    0xb4452(%rip),%rcx        # 29f3d8 <anon.f09c3a3007160cecd1a5798135a651ae.40.llvm.11416580622004461126+0x7d0>
  1eaf86:	mov    %r9,%rdi
  1eaf89:	mov    %r10,%rdx
  1eaf8c:	call   *0xbd70e(%rip)        # 2a86a0 <_DYNAMIC+0x2b0>
  1eaf92:	lea    0xb4457(%rip),%rcx        # 29f3f0 <anon.f09c3a3007160cecd1a5798135a651ae.40.llvm.11416580622004461126+0x7e8>
  1eaf99:	mov    %r9,%rdi
  1eaf9c:	mov    %r10,%rdx
  1eaf9f:	call   *0xbd6fb(%rip)        # 2a86a0 <_DYNAMIC+0x2b0>
  1eafa5:	lea    0xb445c(%rip),%rcx        # 29f408 <anon.f09c3a3007160cecd1a5798135a651ae.40.llvm.11416580622004461126+0x800>
  1eafac:	mov    %r9,%rdi
  1eafaf:	mov    %r10,%rdx
  1eafb2:	call   *0xbd6e8(%rip)        # 2a86a0 <_DYNAMIC+0x2b0>
  1eafb8:	lea    0xb4461(%rip),%rcx        # 29f420 <anon.f09c3a3007160cecd1a5798135a651ae.40.llvm.11416580622004461126+0x818>
  1eafbf:	mov    %r9,%rdi
  1eafc2:	mov    %r10,%rdx
  1eafc5:	call   *0xbd6d5(%rip)        # 2a86a0 <_DYNAMIC+0x2b0>
  1eafcb:	lea    0xb4466(%rip),%rcx        # 29f438 <anon.f09c3a3007160cecd1a5798135a651ae.40.llvm.11416580622004461126+0x830>
  1eafd2:	mov    %r9,%rdi
  1eafd5:	mov    %r10,%rdx
  1eafd8:	call   *0xbd6c2(%rip)        # 2a86a0 <_DYNAMIC+0x2b0>
  1eafde:	int3
  1eafdf:	int3
