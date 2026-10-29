// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later
//! Scratch-only warmed stage attribution for the f32-I/O/f64-internal adapter.
use std::{error::Error, fs, hint::black_box, time::Instant};
use rustsht::{Complex32, Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};
use rayon::prelude::*;

fn real(path:&str,n:usize)->Result<Vec<f32>,Box<dyn Error>> { let b=fs::read(path)?; if b.len()!=4*n{return Err("bad real fixture length".into())} Ok(b.as_chunks::<4>().0.iter().map(|x|f32::from_le_bytes(*x)).collect()) }
fn complex(path:&str,n:usize)->Result<Vec<Complex32>,Box<dyn Error>> { let v=real(path,2*n)?; Ok(v.as_chunks::<2>().0.iter().map(|x|Complex32::new(x[0],x[1])).collect()) }
fn promote_real(a:&[f32],b:&mut[f64],p:bool){if p{b.par_chunks_mut(8192).zip(a.par_chunks(8192)).for_each(|(y,x)|for (a,b) in x.iter().zip(y){*b=f64::from(*a)})}else{for(x,y)in a.iter().zip(b){*y=f64::from(*x)}}}
fn promote_complex(a:&[Complex32],b:&mut[Complex64],p:bool){if p{b.par_chunks_mut(8192).zip(a.par_chunks(8192)).for_each(|(y,x)|for (a,b) in x.iter().zip(y){*b=Complex64::new(f64::from(a.re),f64::from(a.im))})}else{for(x,y)in a.iter().zip(b){*y=Complex64::new(f64::from(x.re),f64::from(x.im))}}}
fn narrow_real(a:&[f64],b:&mut[f32],p:bool){if p{b.par_chunks_mut(8192).zip(a.par_chunks(8192)).for_each(|(y,x)|for (a,b) in x.iter().zip(y){*b=*a as f32})}else{for(x,y)in a.iter().zip(b){*y=*x as f32}}}
fn narrow_complex(a:&[Complex64],b:&mut[Complex32],p:bool){if p{b.par_chunks_mut(8192).zip(a.par_chunks(8192)).for_each(|(y,x)|for (a,b) in x.iter().zip(y){*b=Complex32::new(a.re as f32,a.im as f32)})}else{for(x,y)in a.iter().zip(b){*y=Complex32::new(x.re as f32,x.im as f32)}}}
fn med(v:&mut[f64])->f64 {v.sort_by(f64::total_cmp);v[v.len()/2]}
fn main()->Result<(),Box<dyn Error>>{
 let a=std::env::args().collect::<Vec<_>>(); if a.len()!=12{return Err("usage: OP SPIN THREADS NTHETA NPHI LMAX MMAX MAP0 MAP1 ALM0 ALM1".into())}
 let op=&a[1];let spin=a[2].parse::<usize>()?;let q=a[3].parse::<usize>()?;let nt=a[4].parse::<usize>()?;let np=a[5].parse::<usize>()?;let l=a[6].parse::<usize>()?;let m=a[7].parse::<usize>()?;
 let nmap=nt*np;let nalm=(m+1)*(l+1)-m*(m+1)/2;let map0=real(&a[8],nmap)?;let map1=real(&a[9],nmap)?;let alm0=complex(&a[10],nalm)?;let alm1=complex(&a[11],nalm)?;
 rayon::ThreadPoolBuilder::new().num_threads(q).build_global()?;let plan=ShtPlan::<f64>::new(GridSpec::new(GridKind::GaussLegendre,nt,np,0.23)?,HarmonicSpec::new(l,m,spin)?)?;let mut ws=plan.workspace()?;plan.prepare_workspace(&mut ws,q)?;
 let analysis=op=="analysis";let mut f_map0=vec![0.0;nmap];let mut f_map1=if spin==1{vec![0.0;nmap]}else{Vec::new()};let mut f_alm0=vec![Complex64::new(0.0,0.0);nalm];let mut f_alm1=if spin==1{vec![Complex64::new(0.0,0.0);nalm]}else{Vec::new()};let mut o_map0=vec![0.0;nmap];let mut o_map1=if spin==1{vec![0.0;nmap]}else{Vec::new()};let mut out_map32_0=vec![0.0_f32;nmap];let mut out_map32_1=if spin==1{vec![0.0_f32;nmap]}else{Vec::new()};let mut o_alm0=vec![Complex32::new(0.0,0.0);nalm];let mut o_alm1=if spin==1{vec![Complex32::new(0.0,0.0);nalm]}else{Vec::new()};
 let p=q>1;let mut pre=Vec::new();let mut trans=Vec::new();let mut post=Vec::new();let mut whole=Vec::new();
 for rep in 0..63 {let w=Instant::now();let t=Instant::now();if analysis{promote_real(&map0,&mut f_map0,p);if spin==1{promote_real(&map1,&mut f_map1,p)}}else{promote_complex(&alm0,&mut f_alm0,p);if spin==1{promote_complex(&alm1,&mut f_alm1,p)}}let x=t.elapsed().as_secs_f64()*1e3;let t=Instant::now();match(op.as_str(),spin){("analysis",0)=>plan.analysis_into(&f_map0,&mut f_alm0,&mut ws)?,("analysis",1)=>plan.analysis_spin_into(&f_map0,&f_map1,&mut f_alm0,&mut f_alm1,&mut ws)?,("synthesis",0)=>plan.synthesis_into(&f_alm0,&mut o_map0,&mut ws)?,("synthesis",1)=>plan.synthesis_spin_into(&f_alm0,&f_alm1,&mut o_map0,&mut o_map1,&mut ws)?,_=>return Err("unsupported operation".into())}let y=t.elapsed().as_secs_f64()*1e3;let t=Instant::now();if analysis{narrow_complex(&f_alm0,&mut o_alm0,p);if spin==1{narrow_complex(&f_alm1,&mut o_alm1,p)}}else{narrow_real(&o_map0,&mut out_map32_0,p);if spin==1{narrow_real(&o_map1,&mut out_map32_1,p)}}let z=t.elapsed().as_secs_f64()*1e3;black_box((&out_map32_0,&out_map32_1,&o_alm0,&o_alm1));if rep>=3{pre.push(x);trans.push(y);post.push(z);whole.push(w.elapsed().as_secs_f64()*1e3)}}
 println!("{{\"op\":\"{op}\",\"spin\":{spin},\"threads\":{q},\"repetitions\":60,\"pre_conversion_median_ms\":{:.6},\"f64_transform_median_ms\":{:.6},\"writeback_median_ms\":{:.6},\"whole_profile_median_ms\":{:.6},\"conversion_buffer_bytes\":{}}}",med(&mut pre),med(&mut trans),med(&mut post),med(&mut whole),(f_map0.capacity()+f_map1.capacity()+o_map0.capacity()+o_map1.capacity())*8+(f_alm0.capacity()+f_alm1.capacity())*16);
 Ok(())
}
