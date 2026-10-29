// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Native RustSHT f32 worker for matched DUCC full-transform measurements.

use std::alloc::{GlobalAlloc, Layout, System};
use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use pulp::{Simd, WithSimd};
use rayon::prelude::*;
use rustsht::{Complex32, Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan, ShtWorkspace};

struct CountingAllocator;

static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static REALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

fn account_added_bytes(bytes: usize) {
    ALLOCATED_BYTES.fetch_add(bytes, Ordering::Relaxed);
    let live = LIVE_BYTES.fetch_add(bytes, Ordering::Relaxed) + bytes;
    let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
    while live > peak {
        match PEAK_BYTES.compare_exchange_weak(peak, live, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => peak = actual,
        }
    }
}

fn reset_allocation_counts() {
    let live = LIVE_BYTES.load(Ordering::Relaxed);
    PEAK_BYTES.store(live, Ordering::Relaxed);
    ALLOC_CALLS.store(0, Ordering::Relaxed);
    DEALLOC_CALLS.store(0, Ordering::Relaxed);
    REALLOC_CALLS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            account_added_bytes(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            account_added_bytes(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        DEALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() {
            REALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            if new_size >= layout.size() {
                account_added_bytes(new_size - layout.size());
            } else {
                LIVE_BYTES.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        resized
    }
}

struct F32LaneCount;

impl WithSimd for F32LaneCount {
    type Output = usize;

    fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
        S::F32_LANES
    }
}

struct F64LaneCount;

impl WithSimd for F64LaneCount {
    type Output = usize;

    fn with_simd<S: Simd>(self, _simd: S) -> Self::Output {
        S::F64_LANES
    }
}

fn read_real(path: &str, count: usize) -> Result<Vec<f32>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() != count * 4 {
        return Err(format!("{path} has {} bytes; expected {}", bytes.len(), count * 4).into());
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect())
}

fn read_complex(path: &str, count: usize) -> Result<Vec<Complex32>, Box<dyn Error>> {
    let values = read_real(path, count * 2)?;
    Ok(values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Complex32::new(pair[0], pair[1]))
        .collect())
}

fn write_real(path: &str, values: &[f32]) -> Result<(), Box<dyn Error>> {
    let mut file = fs::File::create(path)?;
    for value in values {
        file.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn write_complex(path: &str, values: &[Complex32]) -> Result<(), Box<dyn Error>> {
    let mut file = fs::File::create(path)?;
    for value in values {
        file.write_all(&value.re.to_le_bytes())?;
        file.write_all(&value.im.to_le_bytes())?;
    }
    Ok(())
}

fn same_real(left: &[f32], right: &[f32]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.to_bits() == b.to_bits())
}

fn same_complex(left: &[Complex32], right: &[Complex32]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.re.to_bits() == b.re.to_bits() && a.im.to_bits() == b.im.to_bits())
}

fn checksum_real(values: &[f32]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let stride = (values.len() / 64).max(1);
    values
        .iter()
        .step_by(stride)
        .take(64)
        .map(|&value| value as f64)
        .sum()
}

fn checksum_complex(values: &[Complex32]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let stride = (values.len() / 64).max(1);
    values
        .iter()
        .step_by(stride)
        .take(64)
        .map(|value| (value.re + value.im) as f64)
        .sum()
}

fn json_f64_array(values: &[f64]) -> String {
    let mut result = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            result.push(',');
        }
        result.push_str(&format!("{value:.17}"));
    }
    result.push(']');
    result
}

struct F64Internal {
    plan: ShtPlan<f64>,
    workspace: ShtWorkspace<f64>,
    threads: usize,
    map0: Vec<f64>,
    map1: Vec<f64>,
    alm0: Vec<Complex64>,
    alm1: Vec<Complex64>,
    map_out0: Vec<f64>,
    map_out1: Vec<f64>,
    alm_out0: Vec<Complex64>,
    alm_out1: Vec<Complex64>,
}

impl F64Internal {
    fn new(
        plan: ShtPlan<f64>,
        workspace: ShtWorkspace<f64>,
        operation: &str,
        spin: usize,
        threads: usize,
        map_count: usize,
        nalm: usize,
    ) -> Self {
        let analysis = operation == "analysis";
        Self {
            plan,
            workspace,
            threads,
            map0: if analysis {
                vec![0.0; map_count]
            } else {
                Vec::new()
            },
            map1: if analysis && spin == 1 {
                vec![0.0; map_count]
            } else {
                Vec::new()
            },
            alm0: if !analysis {
                vec![Complex64::new(0.0, 0.0); nalm]
            } else {
                Vec::new()
            },
            alm1: if !analysis && spin == 1 {
                vec![Complex64::new(0.0, 0.0); nalm]
            } else {
                Vec::new()
            },
            map_out0: if !analysis {
                vec![0.0; map_count]
            } else {
                Vec::new()
            },
            map_out1: if !analysis && spin == 1 {
                vec![0.0; map_count]
            } else {
                Vec::new()
            },
            alm_out0: if analysis {
                vec![Complex64::new(0.0, 0.0); nalm]
            } else {
                Vec::new()
            },
            alm_out1: if analysis && spin == 1 {
                vec![Complex64::new(0.0, 0.0); nalm]
            } else {
                Vec::new()
            },
        }
    }

    fn buffer_bytes(&self) -> usize {
        self.map0.capacity() * std::mem::size_of::<f64>()
            + self.map1.capacity() * std::mem::size_of::<f64>()
            + self.alm0.capacity() * std::mem::size_of::<Complex64>()
            + self.alm1.capacity() * std::mem::size_of::<Complex64>()
            + self.map_out0.capacity() * std::mem::size_of::<f64>()
            + self.map_out1.capacity() * std::mem::size_of::<f64>()
            + self.alm_out0.capacity() * std::mem::size_of::<Complex64>()
            + self.alm_out1.capacity() * std::mem::size_of::<Complex64>()
    }
}

fn promote_real(input: &[f32], output: &mut [f64], parallel: bool) {
    if parallel {
        output
            .par_chunks_mut(8192)
            .zip(input.par_chunks(8192))
            .for_each(|(targets, sources)| {
                for (source, target) in sources.iter().zip(targets) {
                    *target = f64::from(*source);
                }
            });
    } else {
        for (source, target) in input.iter().zip(output) {
            *target = f64::from(*source);
        }
    }
}

fn promote_complex(input: &[Complex32], output: &mut [Complex64], parallel: bool) {
    if parallel {
        output
            .par_chunks_mut(8192)
            .zip(input.par_chunks(8192))
            .for_each(|(targets, sources)| {
                for (source, target) in sources.iter().zip(targets) {
                    *target = Complex64::new(f64::from(source.re), f64::from(source.im));
                }
            });
    } else {
        for (source, target) in input.iter().zip(output) {
            *target = Complex64::new(f64::from(source.re), f64::from(source.im));
        }
    }
}

fn narrow_real(input: &[f64], output: &mut [f32], parallel: bool) {
    if parallel {
        output
            .par_chunks_mut(8192)
            .zip(input.par_chunks(8192))
            .for_each(|(targets, sources)| {
                for (source, target) in sources.iter().zip(targets) {
                    *target = *source as f32;
                }
            });
    } else {
        for (source, target) in input.iter().zip(output) {
            *target = *source as f32;
        }
    }
}

fn narrow_complex(input: &[Complex64], output: &mut [Complex32], parallel: bool) {
    if parallel {
        output
            .par_chunks_mut(8192)
            .zip(input.par_chunks(8192))
            .for_each(|(targets, sources)| {
                for (source, target) in sources.iter().zip(targets) {
                    *target = Complex32::new(source.re as f32, source.im as f32);
                }
            });
    } else {
        for (source, target) in input.iter().zip(output) {
            *target = Complex32::new(source.re as f32, source.im as f32);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_once(
    operation: &str,
    spin: usize,
    internal: &mut F64Internal,
    map0: &[f32],
    map1: &[f32],
    alm0: &[Complex32],
    alm1: &[Complex32],
    map_out0: &mut [f32],
    map_out1: &mut [f32],
    alm_out0: &mut [Complex32],
    alm_out1: &mut [Complex32],
) -> Result<(), rustsht::ShtError> {
    let parallel_conversions = internal.threads > 1;
    match (operation, spin) {
        ("analysis", 0) => {
            promote_real(map0, &mut internal.map0, parallel_conversions);
            internal.plan.analysis_into(
                &internal.map0,
                &mut internal.alm_out0,
                &mut internal.workspace,
            )?;
            narrow_complex(&internal.alm_out0, alm_out0, parallel_conversions);
            Ok(())
        }
        ("analysis", 1) => {
            promote_real(map0, &mut internal.map0, parallel_conversions);
            promote_real(map1, &mut internal.map1, parallel_conversions);
            internal.plan.analysis_spin_into(
                &internal.map0,
                &internal.map1,
                &mut internal.alm_out0,
                &mut internal.alm_out1,
                &mut internal.workspace,
            )?;
            narrow_complex(&internal.alm_out0, alm_out0, parallel_conversions);
            narrow_complex(&internal.alm_out1, alm_out1, parallel_conversions);
            Ok(())
        }
        ("synthesis", 0) => {
            promote_complex(alm0, &mut internal.alm0, parallel_conversions);
            internal.plan.synthesis_into(
                &internal.alm0,
                &mut internal.map_out0,
                &mut internal.workspace,
            )?;
            narrow_real(&internal.map_out0, map_out0, parallel_conversions);
            Ok(())
        }
        ("synthesis", 1) => {
            promote_complex(alm0, &mut internal.alm0, parallel_conversions);
            promote_complex(alm1, &mut internal.alm1, parallel_conversions);
            internal.plan.synthesis_spin_into(
                &internal.alm0,
                &internal.alm1,
                &mut internal.map_out0,
                &mut internal.map_out1,
                &mut internal.workspace,
            )?;
            narrow_real(&internal.map_out0, map_out0, parallel_conversions);
            narrow_real(&internal.map_out1, map_out1, parallel_conversions);
            Ok(())
        }
        _ => unreachable!("worker only supports scalar spin 0 and STANDARD spin 1"),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 18 || args[1] != "--worker" {
        return Err("usage: --worker OP SPIN GRID THREADS NTHETA NPHI LMAX MMAX PHI0 MAP0 MAP1 ALM0 ALM1 OUT0 OUT1 TARGET_MS".into());
    }
    let operation = args[2].as_str();
    if operation != "analysis" && operation != "synthesis" {
        return Err("operation must be analysis or synthesis".into());
    }
    let spin: usize = args[3].parse()?;
    if spin > 1 {
        return Err("spin must be 0 or 1".into());
    }
    let grid_kind = match args[4].as_str() {
        "GL" => GridKind::GaussLegendre,
        "CC" => GridKind::ClenshawCurtis,
        "F1" => GridKind::Fejer1,
        other => return Err(format!("unknown grid {other}").into()),
    };
    let threads: usize = args[5].parse()?;
    let ntheta: usize = args[6].parse()?;
    let nphi: usize = args[7].parse()?;
    let lmax: usize = args[8].parse()?;
    let mmax: usize = args[9].parse()?;
    let phi0: f64 = args[10].parse()?;
    let target_ms: f64 = args[17].parse()?;
    let selected_arch = pulp::Arch::new();
    let selected_arch_name = format!("{selected_arch:?}");
    let f32_lanes = selected_arch.dispatch(F32LaneCount);
    let f64_lanes = selected_arch.dispatch(F64LaneCount);
    let map_count = ntheta * nphi;
    let nalm = (mmax + 1) * (lmax + 1) - mmax * (mmax + 1) / 2;

    let read_started = Instant::now();
    let map0 = read_real(&args[11], map_count)?;
    let map1 = read_real(&args[12], map_count)?;
    let alm0 = read_complex(&args[13], nalm)?;
    let alm1 = read_complex(&args[14], nalm)?;
    let fixture_load_ms = read_started.elapsed().as_secs_f64() * 1e3;
    let mut map_out0 = vec![f32::NAN; map_count];
    let mut map_out1 = vec![f32::NAN; map_count];
    let mut alm_out0 = vec![Complex32::new(f32::NAN, f32::NAN); nalm];
    let mut alm_out1 = vec![Complex32::new(f32::NAN, f32::NAN); nalm];

    let pool_started = Instant::now();
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()?;
    if rayon::current_num_threads() != threads {
        return Err(format!(
            "Rayon global pool has {} threads, expected {threads}",
            rayon::current_num_threads()
        )
        .into());
    }
    let pool_setup_ms = pool_started.elapsed().as_secs_f64() * 1e3;

    let live_bytes_before_plan = LIVE_BYTES.load(Ordering::Relaxed);
    let setup_started = Instant::now();
    let grid = GridSpec::new(grid_kind, ntheta, nphi, phi0)?;
    let harmonics = HarmonicSpec::new(lmax, mmax, spin)?;
    let plan = ShtPlan::<f64>::new(grid, harmonics)?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let mut internal = F64Internal::new(plan, workspace, operation, spin, threads, map_count, nalm);
    let internal_buffer_bytes = internal.buffer_bytes();
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1e3;

    if std::env::var_os("RUSTSHT_ALLOC_REPORT").is_some() {
        reset_allocation_counts();
        run_once(
            operation,
            spin,
            &mut internal,
            &map0,
            &map1,
            &alm0,
            &alm1,
            &mut map_out0,
            &mut map_out1,
            &mut alm_out0,
            &mut alm_out1,
        )?;
        let warm_alloc_calls = ALLOC_CALLS.load(Ordering::Relaxed);
        let warm_realloc_calls = REALLOC_CALLS.load(Ordering::Relaxed);
        let warm_allocated_bytes = ALLOCATED_BYTES.load(Ordering::Relaxed);
        let live_bytes_after_warm = LIVE_BYTES.load(Ordering::Relaxed);

        reset_allocation_counts();
        run_once(
            operation,
            spin,
            &mut internal,
            &map0,
            &map1,
            &alm0,
            &alm1,
            &mut map_out0,
            &mut map_out1,
            &mut alm_out0,
            &mut alm_out1,
        )?;
        let steady_alloc_calls = ALLOC_CALLS.load(Ordering::Relaxed);
        let steady_dealloc_calls = DEALLOC_CALLS.load(Ordering::Relaxed);
        let steady_realloc_calls = REALLOC_CALLS.load(Ordering::Relaxed);
        let steady_allocated_bytes = ALLOCATED_BYTES.load(Ordering::Relaxed);
        let live_bytes_after_steady = LIVE_BYTES.load(Ordering::Relaxed);
        let peak_bytes_during_steady = PEAK_BYTES.load(Ordering::Relaxed);
        reset_allocation_counts();

        let plan_workspace_and_adapter_live_bytes =
            live_bytes_after_warm.saturating_sub(live_bytes_before_plan);
        let plan_workspace_estimate_bytes =
            plan_workspace_and_adapter_live_bytes.saturating_sub(internal_buffer_bytes);
        eprintln!(
            "F64_ADAPTER_ALLOC_META q={threads} T{lmax} m{mmax} grid={ntheta}x{nphi} reusable_conversion_buffer_bytes={internal_buffer_bytes} plan_workspace_and_adapter_live_bytes={plan_workspace_and_adapter_live_bytes} plan_workspace_estimate_bytes={plan_workspace_estimate_bytes} warm_alloc_calls={warm_alloc_calls} warm_realloc_calls={warm_realloc_calls} warm_allocated_bytes={warm_allocated_bytes} steady_alloc_calls={steady_alloc_calls} steady_dealloc_calls={steady_dealloc_calls} steady_realloc_calls={steady_realloc_calls} steady_allocated_bytes={steady_allocated_bytes} steady_live_delta_bytes={} steady_peak_extra_bytes={}",
            live_bytes_after_steady as i128 - live_bytes_after_warm as i128,
            peak_bytes_during_steady.saturating_sub(live_bytes_after_warm),
        );
        return Ok(());
    }

    let first_started = Instant::now();
    run_once(
        operation,
        spin,
        &mut internal,
        &map0,
        &map1,
        &alm0,
        &alm1,
        &mut map_out0,
        &mut map_out1,
        &mut alm_out0,
        &mut alm_out1,
    )?;
    let first_ms = first_started.elapsed().as_secs_f64() * 1e3;
    let first_maps = (map_out0.clone(), map_out1.clone());
    let first_alms = (alm_out0.clone(), alm_out1.clone());

    let warmup_started = Instant::now();
    run_once(
        operation,
        spin,
        &mut internal,
        &map0,
        &map1,
        &alm0,
        &alm1,
        &mut map_out0,
        &mut map_out1,
        &mut alm_out0,
        &mut alm_out1,
    )?;
    let warmup_ms = warmup_started.elapsed().as_secs_f64() * 1e3;
    let repeat_consistent = if operation == "analysis" {
        same_complex(&first_alms.0, &alm_out0)
            && (spin == 0 || same_complex(&first_alms.1, &alm_out1))
    } else {
        same_real(&first_maps.0, &map_out0) && (spin == 0 || same_real(&first_maps.1, &map_out1))
    };

    if operation == "analysis" {
        alm_out0.fill(Complex32::new(f32::NAN, f32::NAN));
        alm_out1.fill(Complex32::new(f32::NAN, f32::NAN));
    } else {
        map_out0.fill(f32::NAN);
        map_out1.fill(f32::NAN);
    }
    run_once(
        operation,
        spin,
        &mut internal,
        &map0,
        &map1,
        &alm0,
        &alm1,
        &mut map_out0,
        &mut map_out1,
        &mut alm_out0,
        &mut alm_out1,
    )?;
    let reset_consistent = if operation == "analysis" {
        same_complex(&first_alms.0, &alm_out0)
            && (spin == 0 || same_complex(&first_alms.1, &alm_out1))
    } else {
        same_real(&first_maps.0, &map_out0) && (spin == 0 || same_real(&first_maps.1, &map_out1))
    };

    let individually_timed = warmup_ms >= 100.0;
    let mut repetitions = if individually_timed {
        1
    } else {
        (target_ms / warmup_ms.max(0.000001))
            .ceil()
            .clamp(1.0, 2_000_000.0) as usize
    };
    if !individually_timed {
        for _ in 0..3 {
            let started = Instant::now();
            for _ in 0..repetitions {
                run_once(
                    operation,
                    spin,
                    &mut internal,
                    &map0,
                    &map1,
                    &alm0,
                    &alm1,
                    &mut map_out0,
                    &mut map_out1,
                    &mut alm_out0,
                    &mut alm_out1,
                )?;
            }
            let elapsed = started.elapsed().as_secs_f64() * 1e3;
            if (20.0..=100.0).contains(&elapsed) {
                break;
            }
            repetitions = (repetitions as f64 * target_ms / elapsed.max(0.000001))
                .ceil()
                .clamp(1.0, 2_000_000.0) as usize;
        }
    }
    let batch_count = if individually_timed { 5 } else { 10 };
    let mut batch_ms = Vec::with_capacity(batch_count);
    let mut batch_total_ms = Vec::with_capacity(batch_count);
    let mut output_checksum = 0.0;
    for _ in 0..batch_count {
        let started = Instant::now();
        for _ in 0..repetitions {
            run_once(
                operation,
                spin,
                &mut internal,
                &map0,
                &map1,
                &alm0,
                &alm1,
                &mut map_out0,
                &mut map_out1,
                &mut alm_out0,
                &mut alm_out1,
            )?;
        }
        let elapsed = started.elapsed().as_secs_f64() * 1e3;
        batch_total_ms.push(elapsed);
        batch_ms.push(elapsed / repetitions as f64);
        output_checksum += if operation == "analysis" {
            checksum_complex(&alm_out0)
                + if spin == 0 {
                    0.0
                } else {
                    checksum_complex(&alm_out1)
                }
        } else {
            checksum_real(&map_out0)
                + if spin == 0 {
                    0.0
                } else {
                    checksum_real(&map_out1)
                }
        };
        black_box(output_checksum);
    }

    if operation == "analysis" {
        write_complex(&args[15], &alm_out0)?;
        if spin == 1 {
            write_complex(&args[16], &alm_out1)?;
        }
    } else {
        write_real(&args[15], &map_out0)?;
        if spin == 1 {
            write_real(&args[16], &map_out1)?;
        }
    }
    let finite_outputs = if operation == "analysis" {
        alm_out0
            .iter()
            .all(|v| v.re.is_finite() && v.im.is_finite())
            && (spin == 0
                || alm_out1
                    .iter()
                    .all(|v| v.re.is_finite() && v.im.is_finite()))
    } else {
        map_out0.iter().all(|v| v.is_finite())
            && (spin == 0 || map_out1.iter().all(|v| v.is_finite()))
    };
    println!(
        "{{\"backend\":\"rustsht_internal_f64\",\"internal_precision\":\"f64\",\"reusable_conversion_buffer_bytes\":{internal_buffer_bytes},\"operation\":\"{operation}\",\"spin\":{spin},\"grid\":\"{}\",\"threads\":{threads},\"lmax\":{lmax},\"mmax\":{mmax},\"simd_arch\":\"{selected_arch_name}\",\"f32_lanes\":{f32_lanes},\"f64_lanes\":{f64_lanes},\"first_ms\":{first_ms:.17},\"warmup_ms\":{warmup_ms:.17},\"pool_setup_ms\":{pool_setup_ms:.17},\"setup_ms\":{setup_ms:.17},\"fixture_load_ms\":{fixture_load_ms:.17},\"repetitions_per_batch\":{repetitions},\"batch_ms\":{},\"batch_total_ms\":{},\"output_checksum\":{output_checksum:.17},\"finite_outputs\":{finite_outputs},\"repeat_consistent\":{repeat_consistent},\"reset_consistent\":{reset_consistent}}}",
        args[4],
        json_f64_array(&batch_ms),
        json_f64_array(&batch_total_ms),
    );
    if !finite_outputs || !repeat_consistent || !reset_consistent {
        return Err("nonfinite output or output repeat/reset mismatch".into());
    }
    Ok(())
}
