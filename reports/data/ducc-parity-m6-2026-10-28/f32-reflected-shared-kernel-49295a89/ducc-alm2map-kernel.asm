
<experiment-scratch>/ducc-callgrind-build/ducc_f32_full_transform_parity:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .plt.sec:

Disassembly of section .text:

000000000067cbc0 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]>:
  67cbc0:	55                   	push   %rbp
  67cbc1:	49 89 cb             	mov    %rcx,%r11
  67cbc4:	49 89 d2             	mov    %rdx,%r10
  67cbc7:	49 8d 43 02          	lea    0x2(%r11),%rax
  67cbcb:	48 89 e5             	mov    %rsp,%rbp
  67cbce:	41 55                	push   %r13
  67cbd0:	41 54                	push   %r12
  67cbd2:	4d 89 c4             	mov    %r8,%r12
  67cbd5:	53                   	push   %rbx
  67cbd6:	48 89 f3             	mov    %rsi,%rbx
  67cbd9:	48 83 e4 e0          	and    $0xffffffffffffffe0,%rsp
  67cbdd:	48 8b 4d 10          	mov    0x10(%rbp),%rcx
  67cbe1:	49 39 c1             	cmp    %rax,%r9
  67cbe4:	0f 82 a3 01 00 00    	jb     67cd8d <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x1cd>
  67cbea:	48 85 c9             	test   %rcx,%rcx
  67cbed:	0f 84 69 01 00 00    	je     67cd5c <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x19c>
  67cbf3:	4d 89 c5             	mov    %r8,%r13
  67cbf6:	4c 89 da             	mov    %r11,%rdx
  67cbf9:	49 c1 e5 04          	shl    $0x4,%r13
  67cbfd:	48 c1 e2 04          	shl    $0x4,%rdx
  67cc01:	49 01 f5             	add    %rsi,%r13
  67cc04:	48 89 ce             	mov    %rcx,%rsi
  67cc07:	4c 01 d2             	add    %r10,%rdx
  67cc0a:	48 c1 e6 05          	shl    $0x5,%rsi
  67cc0e:	48 01 fe             	add    %rdi,%rsi
  67cc11:	0f 1f 40 00          	nopl   0x0(%rax)
  67cc15:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
  67cc1c:	00 00 00 00
  67cc20:	c4 c2 7d 19 55 08    	vbroadcastsd 0x8(%r13),%ymm2
  67cc26:	c4 62 7d 19 3a       	vbroadcastsd (%rdx),%ymm15
  67cc2b:	48 89 f8             	mov    %rdi,%rax
  67cc2e:	c4 62 7d 19 72 08    	vbroadcastsd 0x8(%rdx),%ymm14
  67cc34:	c4 62 7d 19 6a 10    	vbroadcastsd 0x10(%rdx),%ymm13
  67cc3a:	c5 fd 29 54 24 e0    	vmovapd %ymm2,-0x20(%rsp)
  67cc40:	c4 c2 7d 19 55 10    	vbroadcastsd 0x10(%r13),%ymm2
  67cc46:	c4 62 7d 19 62 18    	vbroadcastsd 0x18(%rdx),%ymm12
  67cc4c:	c4 62 7d 19 5a 20    	vbroadcastsd 0x20(%rdx),%ymm11
  67cc52:	c4 62 7d 19 52 28    	vbroadcastsd 0x28(%rdx),%ymm10
  67cc58:	c5 fd 29 54 24 c0    	vmovapd %ymm2,-0x40(%rsp)
  67cc5e:	c4 c2 7d 19 55 18    	vbroadcastsd 0x18(%r13),%ymm2
  67cc64:	c4 62 7d 19 4a 30    	vbroadcastsd 0x30(%rdx),%ymm9
  67cc6a:	c4 62 7d 19 42 38    	vbroadcastsd 0x38(%rdx),%ymm8
  67cc70:	c4 c2 7d 19 7d 00    	vbroadcastsd 0x0(%r13),%ymm7
  67cc76:	c5 fd 29 54 24 a0    	vmovapd %ymm2,-0x60(%rsp)
  67cc7c:	0f 1f 40 00          	nopl   0x0(%rax)
  67cc80:	c5 7d 29 ec          	vmovapd %ymm13,%ymm4
  67cc84:	c5 7d 29 fe          	vmovapd %ymm15,%ymm6
  67cc88:	c5 7d 29 f5          	vmovapd %ymm14,%ymm5
  67cc8c:	48 83 c0 20          	add    $0x20,%rax
  67cc90:	c5 fd 28 90 e0 27 00 	vmovapd 0x27e0(%rax),%ymm2
  67cc97:	00
  67cc98:	c5 fd 28 44 24 e0    	vmovapd -0x20(%rsp),%ymm0
  67cc9e:	c5 7d 29 e3          	vmovapd %ymm12,%ymm3
  67cca2:	c5 fd 28 88 e0 1f 00 	vmovapd 0x1fe0(%rax),%ymm1
  67cca9:	00
  67ccaa:	c4 e2 f5 a8 a0 e0 3f 	vfmadd213pd 0x3fe0(%rax),%ymm1,%ymm4
  67ccb1:	00 00
  67ccb3:	c4 e2 f5 a8 b0 e0 2f 	vfmadd213pd 0x2fe0(%rax),%ymm1,%ymm6
  67ccba:	00 00
  67ccbc:	c4 e2 c5 b8 c2       	vfmadd231pd %ymm2,%ymm7,%ymm0
  67ccc1:	c4 e2 f5 a8 80 e0 17 	vfmadd213pd 0x17e0(%rax),%ymm1,%ymm0
  67ccc8:	00 00
  67ccca:	c4 e2 f5 a8 a8 e0 37 	vfmadd213pd 0x37e0(%rax),%ymm1,%ymm5
  67ccd1:	00 00
  67ccd3:	c4 e2 f5 a8 98 e0 47 	vfmadd213pd 0x47e0(%rax),%ymm1,%ymm3
  67ccda:	00 00
  67ccdc:	c4 e2 b5 b8 e0       	vfmadd231pd %ymm0,%ymm9,%ymm4
  67cce1:	c4 e2 a5 b8 f0       	vfmadd231pd %ymm0,%ymm11,%ymm6
  67cce6:	c5 fd 29 80 e0 17 00 	vmovapd %ymm0,0x17e0(%rax)
  67cced:	00
  67ccee:	c4 e2 ad b8 e8       	vfmadd231pd %ymm0,%ymm10,%ymm5
  67ccf3:	c4 e2 bd b8 d8       	vfmadd231pd %ymm0,%ymm8,%ymm3
  67ccf8:	c5 fd 29 a0 e0 3f 00 	vmovapd %ymm4,0x3fe0(%rax)
  67ccff:	00
  67cd00:	c5 fd 28 64 24 a0    	vmovapd -0x60(%rsp),%ymm4
  67cd06:	c4 e2 dd 98 54 24 c0 	vfmadd132pd -0x40(%rsp),%ymm4,%ymm2
  67cd0d:	c5 fd 29 b0 e0 2f 00 	vmovapd %ymm6,0x2fe0(%rax)
  67cd14:	00
  67cd15:	c5 fd 29 a8 e0 37 00 	vmovapd %ymm5,0x37e0(%rax)
  67cd1c:	00
  67cd1d:	c5 fd 29 98 e0 47 00 	vmovapd %ymm3,0x47e0(%rax)
  67cd24:	00
  67cd25:	c4 e2 f5 98 c2       	vfmadd132pd %ymm2,%ymm1,%ymm0
  67cd2a:	c5 fd 29 80 e0 1f 00 	vmovapd %ymm0,0x1fe0(%rax)
  67cd31:	00
  67cd32:	48 39 c6             	cmp    %rax,%rsi
  67cd35:	0f 85 45 ff ff ff    	jne    67cc80 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0xc0>
  67cd3b:	4d 8d 43 04          	lea    0x4(%r11),%r8
  67cd3f:	49 83 c3 06          	add    $0x6,%r11
  67cd43:	49 83 c4 02          	add    $0x2,%r12
  67cd47:	48 83 c2 40          	add    $0x40,%rdx
  67cd4b:	49 83 c5 20          	add    $0x20,%r13
  67cd4f:	4d 39 d9             	cmp    %r11,%r9
  67cd52:	72 3c                	jb     67cd90 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x1d0>
  67cd54:	4d 89 c3             	mov    %r8,%r11
  67cd57:	e9 c4 fe ff ff       	jmp    67cc20 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x60>
  67cd5c:	49 8d 43 06          	lea    0x6(%r11),%rax
  67cd60:	49 39 c1             	cmp    %rax,%r9
  67cd63:	72 1d                	jb     67cd82 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x1c2>
  67cd65:	4d 8d 43 08          	lea    0x8(%r11),%r8
  67cd69:	49 83 c3 0a          	add    $0xa,%r11
  67cd6d:	49 83 c4 04          	add    $0x4,%r12
  67cd71:	4d 39 d9             	cmp    %r11,%r9
  67cd74:	72 1a                	jb     67cd90 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x1d0>
  67cd76:	4d 89 c3             	mov    %r8,%r11
  67cd79:	49 8d 43 06          	lea    0x6(%r11),%rax
  67cd7d:	49 39 c1             	cmp    %rax,%r9
  67cd80:	73 e3                	jae    67cd65 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x1a5>
  67cd82:	48 8d 65 e8          	lea    -0x18(%rbp),%rsp
  67cd86:	5b                   	pop    %rbx
  67cd87:	41 5c                	pop    %r12
  67cd89:	41 5d                	pop    %r13
  67cd8b:	5d                   	pop    %rbp
  67cd8c:	c3                   	ret
  67cd8d:	4d 89 d8             	mov    %r11,%r8
  67cd90:	4d 39 c1             	cmp    %r8,%r9
  67cd93:	0f 82 fb 00 00 00    	jb     67ce94 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x2d4>
  67cd99:	48 85 c9             	test   %rcx,%rcx
  67cd9c:	0f 84 f2 00 00 00    	je     67ce94 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x2d4>
  67cda2:	49 c1 e4 04          	shl    $0x4,%r12
  67cda6:	4c 89 c0             	mov    %r8,%rax
  67cda9:	48 c1 e1 05          	shl    $0x5,%rcx
  67cdad:	48 c1 e0 04          	shl    $0x4,%rax
  67cdb1:	4c 01 e3             	add    %r12,%rbx
  67cdb4:	48 8d 14 0f          	lea    (%rdi,%rcx,1),%rdx
  67cdb8:	49 8d 34 02          	lea    (%r10,%rax,1),%rsi
  67cdbc:	0f 1f 40 00          	nopl   0x0(%rax)
  67cdc0:	c4 e2 7d 19 3e       	vbroadcastsd (%rsi),%ymm7
  67cdc5:	c4 e2 7d 19 76 08    	vbroadcastsd 0x8(%rsi),%ymm6
  67cdcb:	48 89 f8             	mov    %rdi,%rax
  67cdce:	c4 e2 7d 19 6e 10    	vbroadcastsd 0x10(%rsi),%ymm5
  67cdd4:	c4 e2 7d 19 66 18    	vbroadcastsd 0x18(%rsi),%ymm4
  67cdda:	c4 e2 7d 19 1b       	vbroadcastsd (%rbx),%ymm3
  67cddf:	c4 e2 7d 19 53 08    	vbroadcastsd 0x8(%rbx),%ymm2
  67cde5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
  67cdec:	00 00 00 00
  67cdf0:	c5 fd 28 80 00 20 00 	vmovapd 0x2000(%rax),%ymm0
  67cdf7:	00
  67cdf8:	c5 fd 28 cf          	vmovapd %ymm7,%ymm1
  67cdfc:	48 83 c0 20          	add    $0x20,%rax
  67ce00:	c4 e2 fd a8 88 e0 2f 	vfmadd213pd 0x2fe0(%rax),%ymm0,%ymm1
  67ce07:	00 00
  67ce09:	c5 fd 29 88 e0 2f 00 	vmovapd %ymm1,0x2fe0(%rax)
  67ce10:	00
  67ce11:	c5 fd 28 ce          	vmovapd %ymm6,%ymm1
  67ce15:	c4 e2 fd a8 88 e0 37 	vfmadd213pd 0x37e0(%rax),%ymm0,%ymm1
  67ce1c:	00 00
  67ce1e:	c5 fd 29 88 e0 37 00 	vmovapd %ymm1,0x37e0(%rax)
  67ce25:	00
  67ce26:	c5 fd 28 cd          	vmovapd %ymm5,%ymm1
  67ce2a:	c4 e2 fd a8 88 e0 3f 	vfmadd213pd 0x3fe0(%rax),%ymm0,%ymm1
  67ce31:	00 00
  67ce33:	c5 fd 29 88 e0 3f 00 	vmovapd %ymm1,0x3fe0(%rax)
  67ce3a:	00
  67ce3b:	c5 fd 28 cc          	vmovapd %ymm4,%ymm1
  67ce3f:	c4 e2 fd a8 88 e0 47 	vfmadd213pd 0x47e0(%rax),%ymm0,%ymm1
  67ce46:	00 00
  67ce48:	c5 fd 29 88 e0 47 00 	vmovapd %ymm1,0x47e0(%rax)
  67ce4f:	00
  67ce50:	c5 fd 28 cb          	vmovapd %ymm3,%ymm1
  67ce54:	c4 e2 ed 98 88 e0 27 	vfmadd132pd 0x27e0(%rax),%ymm2,%ymm1
  67ce5b:	00 00
  67ce5d:	c4 e2 fd a8 88 e0 17 	vfmadd213pd 0x17e0(%rax),%ymm0,%ymm1
  67ce64:	00 00
  67ce66:	c5 fd 29 80 e0 17 00 	vmovapd %ymm0,0x17e0(%rax)
  67ce6d:	00
  67ce6e:	c5 fd 29 88 e0 1f 00 	vmovapd %ymm1,0x1fe0(%rax)
  67ce75:	00
  67ce76:	48 39 c2             	cmp    %rax,%rdx
  67ce79:	0f 85 71 ff ff ff    	jne    67cdf0 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x230>
  67ce7f:	49 83 c0 02          	add    $0x2,%r8
  67ce83:	48 83 c6 20          	add    $0x20,%rsi
  67ce87:	48 83 c3 10          	add    $0x10,%rbx
  67ce8b:	4d 39 c1             	cmp    %r8,%r9
  67ce8e:	0f 83 2c ff ff ff    	jae    67cdc0 <ducc0::detail_sht::detail_sht_inner_loop::alm2map_kernel(ducc0::detail_sht::detail_sht_inner_loop::s0data_v&, std::vector<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2, std::allocator<ducc0::detail_sht::detail_sht_inner_loop::Ylmgen::dbl2> > const&, std::complex<double> const*, unsigned long, unsigned long, unsigned long, unsigned long) [clone .isra.0]+0x200>
  67ce94:	c5 f8 77             	vzeroupper
  67ce97:	48 8d 65 e8          	lea    -0x18(%rbp),%rsp
  67ce9b:	5b                   	pop    %rbx
  67ce9c:	41 5c                	pop    %r12
  67ce9e:	41 5d                	pop    %r13
  67cea0:	5d                   	pop    %rbp
  67cea1:	c3                   	ret

Disassembly of section .fini:
