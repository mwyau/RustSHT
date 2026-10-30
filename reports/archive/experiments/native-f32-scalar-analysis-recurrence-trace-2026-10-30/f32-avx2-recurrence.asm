
target/release/deps/rustsht-b61016143b4165b2:     file format elf64-x86-64


Disassembly of section .text:

000000000024c9c0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::analysis::FusedScalarAnalysisPairTyped<f32>>>+0x22b0>:
  24c9c0:	c4 c1 7c 10 54 08 a0 	vmovups -0x60(%r8,%rcx,1),%ymm2
  24c9c7:	c4 e2 7d a8 d1       	vfmadd213ps %ymm1,%ymm0,%ymm2
  24c9cc:	c4 c1 7c 10 5c 0a a0 	vmovups -0x60(%r10,%rcx,1),%ymm3
  24c9d3:	c4 c2 6d a8 5c 0c a0 	vfmadd213ps -0x60(%r12,%rcx,1),%ymm2,%ymm3
  24c9da:	c5 fc 11 5c 0d a0    	vmovups %ymm3,-0x60(%rbp,%rcx,1)
  24c9e0:	c4 c1 7c 10 54 08 c0 	vmovups -0x40(%r8,%rcx,1),%ymm2
  24c9e7:	c4 e2 7d a8 d1       	vfmadd213ps %ymm1,%ymm0,%ymm2
  24c9ec:	c4 c1 7c 10 5c 0a c0 	vmovups -0x40(%r10,%rcx,1),%ymm3
  24c9f3:	c4 c2 6d a8 5c 0c c0 	vfmadd213ps -0x40(%r12,%rcx,1),%ymm2,%ymm3
  24c9fa:	c5 fc 11 5c 0d c0    	vmovups %ymm3,-0x40(%rbp,%rcx,1)
  24ca00:	c4 c1 7c 10 54 08 e0 	vmovups -0x20(%r8,%rcx,1),%ymm2
  24ca07:	c4 e2 7d a8 d1       	vfmadd213ps %ymm1,%ymm0,%ymm2
  24ca0c:	c4 c1 7c 10 5c 0a e0 	vmovups -0x20(%r10,%rcx,1),%ymm3
  24ca13:	c4 c2 6d a8 5c 0c e0 	vfmadd213ps -0x20(%r12,%rcx,1),%ymm2,%ymm3
  24ca1a:	c5 fc 11 5c 0d e0    	vmovups %ymm3,-0x20(%rbp,%rcx,1)
  24ca20:	48 83 c0 04          	add    $0x4,%rax
  24ca24:	c4 c1 7c 10 14 08    	vmovups (%r8,%rcx,1),%ymm2
  24ca2a:	c4 e2 7d a8 d1       	vfmadd213ps %ymm1,%ymm0,%ymm2
  24ca2f:	c4 c1 7c 10 1c 0a    	vmovups (%r10,%rcx,1),%ymm3
  24ca35:	c4 c2 6d a8 1c 0c    	vfmadd213ps (%r12,%rcx,1),%ymm2,%ymm3
  24ca3b:	c5 fc 11 5c 0d 00    	vmovups %ymm3,0x0(%rbp,%rcx,1)
  24ca41:	48 83 e9 80          	sub    $0xffffffffffffff80,%rcx
  24ca45:	48 39 c2             	cmp    %rax,%rdx
  24ca48:	0f 85 72 ff ff ff    	jne    24c9c0 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::analysis::FusedScalarAnalysisPairTyped<f32>>>+0x22b0>
  24ca4e:	48 83 bc 24 60 02 00 	cmpq   $0x0,0x260(%rsp)
  24ca55:	00 00
  24ca57:	74 3a                	je     24ca93 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::analysis::FusedScalarAnalysisPairTyped<f32>>>+0x2383>
  24ca59:	48 c1 e0 05          	shl    $0x5,%rax
  24ca5d:	48 8d 0c 28          	lea    (%rax,%rbp,1),%rcx
  24ca61:	49 8d 14 04          	lea    (%r12,%rax,1),%rdx
  24ca65:	49 8d 34 02          	lea    (%r10,%rax,1),%rsi
  24ca69:	4c 01 c0             	add    %r8,%rax
  24ca6c:	31 ff                	xor    %edi,%edi
  24ca6e:	66 90                	xchg   %ax,%ax
  24ca70:	c5 fc 10 14 38       	vmovups (%rax,%rdi,1),%ymm2
  24ca75:	c4 e2 7d a8 d1       	vfmadd213ps %ymm1,%ymm0,%ymm2
  24ca7a:	c5 fc 10 1c 3e       	vmovups (%rsi,%rdi,1),%ymm3
  24ca7f:	c4 e2 6d a8 1c 3a    	vfmadd213ps (%rdx,%rdi,1),%ymm2,%ymm3
  24ca85:	c5 fc 11 1c 39       	vmovups %ymm3,(%rcx,%rdi,1)
  24ca8a:	48 83 c7 20          	add    $0x20,%rdi
  24ca8e:	49 39 fb             	cmp    %rdi,%r11
  24ca91:	75 dd                	jne    24ca70 <<pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::analysis::FusedScalarAnalysisPairTyped<f32>>>+0x2360>
