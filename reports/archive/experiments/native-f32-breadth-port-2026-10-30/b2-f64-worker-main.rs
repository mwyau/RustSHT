use std::error::Error;
use std::fs;
use std::path::Path;
use std::hint::black_box;
use std::time::Instant;
use rustsht::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan, ShtWorkspace};

fn parse<T: std::str::FromStr>(s: &str) -> Result<T, Box<dyn Error>> where T::Err: Error + 'static { Ok(s.parse()?) }
fn write_f64(path: &Path, values: &[f64]) -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in values { bytes.extend_from_slice(&value.to_le_bytes()); }
    fs::write(path, bytes)?;
    Ok(())
}
fn read_f32(path: &str, n: usize) -> Result<Vec<f32>, Box<dyn Error>> {
    let b = fs::read(path)?;
    if b.len() != n * 4 { return Err(format!("{path}: {} bytes, expected {}", b.len(), n*4).into()); }
    Ok(b.as_chunks::<4>().0.iter().map(|x| f32::from_le_bytes(*x)).collect())
}
fn read_real64_from_f32(path: &str, n: usize) -> Result<Vec<f64>, Box<dyn Error>> {
    Ok(read_f32(path,n)?.into_iter().map(f64::from).collect())
}
fn read_complex64_from_f32(path: &str, n: usize) -> Result<Vec<Complex64>, Box<dyn Error>> {
    let x=read_f32(path,2*n)?;
    Ok(x.chunks_exact(2).map(|v|Complex64::new(f64::from(v[0]),f64::from(v[1]))).collect())
}
fn checksum_real(x:&[f64])->f64 { x.iter().step_by((x.len()/64).max(1)).take(64).sum() }
fn checksum_complex(x:&[Complex64])->f64 { x.iter().step_by((x.len()/64).max(1)).take(64).map(|v|v.re+v.im).sum() }
fn same_real(a:&[f64],b:&[f64])->bool { a.len()==b.len() && a.iter().zip(b).all(|(x,y)|x.to_bits()==y.to_bits()) }
fn same_complex(a:&[Complex64],b:&[Complex64])->bool { a.len()==b.len() && a.iter().zip(b).all(|(x,y)|x.re.to_bits()==y.re.to_bits()&&x.im.to_bits()==y.im.to_bits()) }
fn run_once(op:&str, spin:usize, p:&ShtPlan<f64>, m0:&[f64],m1:&[f64],a0:&[Complex64],a1:&[Complex64],o0:&mut[f64],o1:&mut[f64],c0:&mut[Complex64],c1:&mut[Complex64],w:&mut ShtWorkspace<f64>)->Result<(),rustsht::ShtError>{
 match (op,spin) { ("analysis",0)=>p.analysis_into(m0,c0,w), ("analysis",1)=>p.analysis_spin_into(m0,m1,c0,c1,w), ("synthesis",0)=>p.synthesis_into(a0,o0,w), ("synthesis",1)=>p.synthesis_spin_into(a0,a1,o0,o1,w), _=>unreachable!() }
}
fn main()->Result<(),Box<dyn Error>>{
 let a=std::env::args().collect::<Vec<_>>();
 if a.len()!=16||a[1]!="--worker" { return Err("--worker OP SPIN THREADS NTHETA NPHI LMAX MMAX PHI0 MAP0 MAP1 ALM0 ALM1 TARGET_MS".into()); }
 let op=&a[2]; let spin:usize=parse(&a[3])?; let threads:usize=parse(&a[4])?;
 let nt:usize=parse(&a[5])?; let np:usize=parse(&a[6])?; let lm:usize=parse(&a[7])?; let mm:usize=parse(&a[8])?; let phi:f64=parse(&a[9])?; let target:f64=parse(&a[14])?;
 let nmap=nt*np; let nalm=(mm+1)*(lm+1)-mm*(mm+1)/2;
 let m0=read_real64_from_f32(&a[10],nmap)?;let m1=read_real64_from_f32(&a[11],nmap)?;let a0=read_complex64_from_f32(&a[12],nalm)?;let a1=read_complex64_from_f32(&a[13],nalm)?;
 rayon::ThreadPoolBuilder::new().num_threads(threads).build_global()?;
 if rayon::current_num_threads()!=threads{return Err("rayon thread count mismatch".into());}
 let kind=GridKind::GaussLegendre; let grid=GridSpec::new(kind,nt,np,phi)?;let h=HarmonicSpec::new(lm,mm,spin)?;let p=ShtPlan::<f64>::new(grid,h)?;let mut w=p.workspace()?;p.prepare_workspace(&mut w,threads)?;
 let mut o0=vec![0.0;nmap];let mut o1=vec![0.0;nmap];let mut c0=vec![Complex64::new(0.0,0.0);nalm];let mut c1=vec![Complex64::new(0.0,0.0);nalm];
 let t=Instant::now();run_once(op,spin,&p,&m0,&m1,&a0,&a1,&mut o0,&mut o1,&mut c0,&mut c1,&mut w)?;let first=t.elapsed().as_secs_f64()*1e3;
 let f0=o0.clone();let f1=o1.clone();let fc0=c0.clone();let fc1=c1.clone();
 run_once(op,spin,&p,&m0,&m1,&a0,&a1,&mut o0,&mut o1,&mut c0,&mut c1,&mut w)?;
 let stable=if op=="analysis"{same_complex(&fc0,&c0)&&(spin==0||same_complex(&fc1,&c1))}else{same_real(&f0,&o0)&&(spin==0||same_real(&f1,&o1))};
 if !stable{return Err("warm calls differ".into());}
 let mut reps=if first>=100.0{1}else{((target/first.max(1e-6)).ceil() as usize).clamp(1,2_000_000)};
 if first<100.0{for _ in 0..3{let t=Instant::now();for _ in 0..reps{run_once(op,spin,&p,&m0,&m1,&a0,&a1,&mut o0,&mut o1,&mut c0,&mut c1,&mut w)?;black_box((&o0,&o1,&c0,&c1));}let ms=t.elapsed().as_secs_f64()*1e3;if (20.0..=100.0).contains(&ms){break;}reps=((reps as f64*target/ms.max(1e-6)).ceil() as usize).clamp(1,2_000_000);}}
 let batches=if first>=100.0{5}else{10};let mut times=Vec::with_capacity(batches);
 for _ in 0..batches{let t=Instant::now();for _ in 0..reps{run_once(op,spin,&p,&m0,&m1,&a0,&a1,&mut o0,&mut o1,&mut c0,&mut c1,&mut w)?;black_box((&o0,&o1,&c0,&c1));}times.push(t.elapsed().as_secs_f64()*1e3/reps as f64);}
 let finite=if op=="analysis"{c0.iter().all(|x|x.re.is_finite()&&x.im.is_finite())&&(spin==0||c1.iter().all(|x|x.re.is_finite()&&x.im.is_finite()))}else{o0.iter().all(|x|x.is_finite())&&(spin==0||o1.iter().all(|x|x.is_finite()))};
 let checksum=if op=="analysis"{checksum_complex(&c0)+(if spin==0{0.0}else{checksum_complex(&c1)})}else{checksum_real(&o0)+(if spin==0{0.0}else{checksum_real(&o1)})};
 if &a[15] != "-" { let output_dir=Path::new(&a[15]); fs::create_dir_all(output_dir)?; write_f64(&output_dir.join("map0.f64"),&o0)?; write_f64(&output_dir.join("map1.f64"),&o1)?; }
 println!("{{\"op\":\"{op}\",\"spin\":{spin},\"threads\":{threads},\"first_ms\":{first:.9},\"repetitions\":{reps},\"batch_ms\":{:?},\"finite\":{finite},\"stable\":{stable},\"checksum\":{checksum:.17},\"arch\":\"{:?}\"}}",times,pulp::Arch::new());
 Ok(())
}
