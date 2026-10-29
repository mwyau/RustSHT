
<candidate>/release/examples/ducc_f32_full_transform_parity:     file format elf64-x86-64


Disassembly of section .text:

00000000002081e0 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x760>:
  2081e0:	80 7c 24 74 00       	cmp    BYTE PTR [rsp+0x74],0x0
  2081e5:	48 89 54 24 48       	mov    QWORD PTR [rsp+0x48],rdx
  2081ea:	0f 84 4d 05 00 00    	je     20873d <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0xcbd>
  2081f0:	4b 8d 0c f4          	lea    rcx,[r12+r14*8]
  2081f4:	48 8b 09             	mov    rcx,QWORD PTR [rcx]
  2081f7:	41 b1 01             	mov    r9b,0x1
  2081fa:	4c 39 d5             	cmp    rbp,r10
  2081fd:	48 8b 7c 24 40       	mov    rdi,QWORD PTR [rsp+0x40]
  208202:	4c 8b 84 24 d8 00 00 	mov    r8,QWORD PTR [rsp+0xd8]
  208209:	00
  20820a:	74 16                	je     208222 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x7a2>
  20820c:	48 89 ee             	mov    rsi,rbp
  20820f:	49 39 0c 34          	cmp    QWORD PTR [r12+rsi*1],rcx
  208213:	41 0f 94 c1          	sete   r9b
  208217:	75 09                	jne    208222 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x7a2>
  208219:	49 39 f0             	cmp    r8,rsi
  20821c:	48 8d 76 08          	lea    rsi,[rsi+0x8]
  208220:	75 ed                	jne    20820f <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x78f>
  208222:	31 f6                	xor    esi,esi
  208224:	4f 8d 04 f4          	lea    r8,[r12+r14*8]
  208228:	4d 8b 04 f0          	mov    r8,QWORD PTR [r8+rsi*8]
  20822c:	49 ff c0             	inc    r8
  20822f:	49 ba 68 66 66 66 66 	movabs r10,0xfe66666666666668
  208236:	66 66 fe
  208239:	4d 39 d0             	cmp    r8,r10
  20823c:	72 39                	jb     208277 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x7f7>
  20823e:	4e 8d 04 b5 00 00 00 	lea    r8,[r14*4+0x0]
  208245:	00
  208246:	4d 01 e8             	add    r8,r13
  208249:	c4 c1 79 6e 04 b0    	vmovd  xmm0,DWORD PTR [r8+rsi*4]
  20824f:	c4 c1 79 7e c0       	vmovd  r8d,xmm0
  208254:	45 01 c0             	add    r8d,r8d
  208257:	41 81 f8 ff ff ff fe 	cmp    r8d,0xfeffffff
  20825e:	77 17                	ja     208277 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x7f7>
  208260:	c5 a9 db c0          	vpand  xmm0,xmm10,xmm0
  208264:	c5 78 2e d8          	vucomiss xmm11,xmm0
  208268:	72 0d                	jb     208277 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x7f7>
  20826a:	48 ff c6             	inc    rsi
  20826d:	48 39 f0             	cmp    rax,rsi
  208270:	75 b2                	jne    208224 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x7a4>
  208272:	e9 53 01 00 00       	jmp    2083ca <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x94a>
  208277:	48 39 c6             	cmp    rsi,rax
  20827a:	0f 83 4a 01 00 00    	jae    2083ca <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x94a>
  208280:	4c 89 f6             	mov    rsi,r14
  208283:	45 84 c9             	test   r9b,r9b
  208286:	0f 84 42 03 00 00    	je     2085ce <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0xb4e>
  20828c:	48 6b c9 50          	imul   rcx,rcx,0x50
  208290:	0f 80 da 08 00 00    	jo     208b70 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x10f0>
  208296:	4c 89 f6             	mov    rsi,r14
  208299:	48 83 f9 b0          	cmp    rcx,0xffffffffffffffb0
  20829d:	0f 85 2b 03 00 00    	jne    2085ce <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0xb4e>
  2082a3:	c5 f8 29 8c 24 90 00 	vmovaps XMMWORD PTR [rsp+0x90],xmm1
  2082aa:	00 00
  2082ac:	c5 f8 29 94 24 a0 00 	vmovaps XMMWORD PTR [rsp+0xa0],xmm2
  2082b3:	00 00
  2082b5:	4a 8d 0c b5 00 00 00 	lea    rcx,[r14*4+0x0]
  2082bc:	00
  2082bd:	48 8b 74 24 58       	mov    rsi,QWORD PTR [rsp+0x58]
  2082c2:	48 8b 7c 24 40       	mov    rdi,QWORD PTR [rsp+0x40]
  2082c7:	c4 c1 7e 6f 44 0d 00 	vmovdqu ymm0,YMMWORD PTR [r13+rcx*1+0x0]
  2082ce:	c5 f5 72 d0 17       	vpsrld ymm1,ymm0,0x17
  2082d3:	c5 95 db c9          	vpand  ymm1,ymm13,ymm1
  2082d7:	c5 8d 76 d1          	vpcmpeqd ymm2,ymm14,ymm1
  2082db:	c4 e2 7d 0e d2       	vtestps ymm2,ymm2
  2082e0:	0f 85 d3 02 00 00    	jne    2085b9 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0xb39>
  2082e6:	c5 f5 66 94 24 10 02 	vpcmpgtd ymm2,ymm1,YMMWORD PTR [rsp+0x210]
  2082ed:	00 00
  2082ef:	c5 fe 6f 9c 24 f0 01 	vmovdqu ymm3,YMMWORD PTR [rsp+0x1f0]
  2082f6:	00 00
  2082f8:	c5 e5 fa d9          	vpsubd ymm3,ymm3,ymm1
  2082fc:	c5 7c 10 bc 24 d0 01 	vmovups ymm15,YMMWORD PTR [rsp+0x1d0]
  208303:	00 00
  208305:	c4 c3 65 4a df 20    	vblendvps ymm3,ymm3,ymm15,ymm2
  20830b:	c4 e2 7d 58 25 bc 7e 	vpbroadcastd ymm4,DWORD PTR [rip+0xffffffffffe17ebc]        # 201d0 <__abi_tag+0x1fed4>
  208312:	e1 ff
  208314:	c4 e2 65 3b e4       	vpminud ymm4,ymm3,ymm4
  208319:	c4 e2 7d 58 2d 7e 79 	vpbroadcastd ymm5,DWORD PTR [rip+0xffffffffffe1797e]        # 1fca0 <__abi_tag+0x1f9a4>
  208320:	e1 ff
  208322:	c5 fd db ed          	vpand  ymm5,ymm0,ymm5
  208326:	c4 e2 7d 58 35 29 80 	vpbroadcastd ymm6,DWORD PTR [rip+0xffffffffffe18029]        # 20358 <__abi_tag+0x2005c>
  20832d:	e1 ff
  20832f:	c5 d5 eb ee          	vpor   ymm5,ymm5,ymm6
  208333:	c4 e2 55 45 f4       	vpsrlvd ymm6,ymm5,ymm4
  208338:	c5 fd db bc 24 30 02 	vpand  ymm7,ymm0,YMMWORD PTR [rsp+0x230]
  20833f:	00 00
  208341:	c4 62 05 47 c4       	vpsllvd ymm8,ymm15,ymm4
  208346:	c4 41 31 ef c9       	vpxor  xmm9,xmm9,xmm9
  20834b:	c5 b5 76 c9          	vpcmpeqd ymm1,ymm9,ymm1
  20834f:	c4 41 35 76 c9       	vpcmpeqd ymm9,ymm9,ymm9
  208354:	c4 41 3d fe c9       	vpaddd ymm9,ymm8,ymm9
  208359:	c4 c1 3d 72 d0 01    	vpsrld ymm8,ymm8,0x1
  20835f:	c5 b5 db ed          	vpand  ymm5,ymm9,ymm5
  208363:	c4 62 3d 3f cd       	vpmaxud ymm9,ymm8,ymm5
  208368:	c4 41 3d 76 c9       	vpcmpeqd ymm9,ymm8,ymm9
  20836d:	c5 bd 76 ed          	vpcmpeqd ymm5,ymm8,ymm5
  208371:	c5 cd db ed          	vpand  ymm5,ymm6,ymm5
  208375:	c4 c1 55 df e9       	vpandn ymm5,ymm5,ymm9
  20837a:	c4 41 31 ef c9       	vpxor  xmm9,xmm9,xmm9
  20837f:	c4 c1 55 df ef       	vpandn ymm5,ymm5,ymm15
  208384:	c5 cd fe ed          	vpaddd ymm5,ymm6,ymm5
  208388:	c5 e5 76 dc          	vpcmpeqd ymm3,ymm3,ymm4
  20838c:	c5 e5 db dd          	vpand  ymm3,ymm3,ymm5
  208390:	c5 c5 eb db          	vpor   ymm3,ymm7,ymm3
  208394:	c4 e3 65 4a cf 10    	vblendvps ymm1,ymm3,ymm7,ymm1
  20839a:	c4 e2 7d 58 1d 95 7a 	vpbroadcastd ymm3,DWORD PTR [rip+0xffffffffffe17a95]        # 1fe38 <__abi_tag+0x1fb3c>
  2083a1:	e1 ff
  2083a3:	c5 fd fe c3          	vpaddd ymm0,ymm0,ymm3
  2083a7:	c4 e3 75 4a c0 20    	vblendvps ymm0,ymm1,ymm0,ymm2
  2083ad:	c5 fc 11 04 0f       	vmovups YMMWORD PTR [rdi+rcx*1],ymm0
  2083b2:	48 83 c1 20          	add    rcx,0x20
  2083b6:	48 ff ce             	dec    rsi
  2083b9:	0f 85 08 ff ff ff    	jne    2082c7 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0x847>
  2083bf:	b1 01                	mov    cl,0x1
  2083c1:	89 4c 24 0c          	mov    DWORD PTR [rsp+0xc],ecx
  2083c5:	e9 db 04 00 00       	jmp    2088a5 <_RINvNtNtNtCs7F2oEptmHw6_7rustsht8legendre14scalar_ordered6kernel27prepare_scaled_prefix_groupfNtNtNtCs6aRYI14iRvw_4pulp3x862v32V3EB8_+0xe25>
