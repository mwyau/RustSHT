// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use rustsht::{Complex32, GridKind, GridSpec, HarmonicSpec, ShtPlan};

struct CountingAllocator;

static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static REALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

fn add_bytes(bytes: usize) {
    let live = LIVE_BYTES.fetch_add(bytes, Ordering::Relaxed) + bytes;
    let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
    while live > peak {
        match PEAK_BYTES.compare_exchange_weak(peak, live, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => peak = actual,
        }
    }
}

fn reset_counts() {
    PEAK_BYTES.store(LIVE_BYTES.load(Ordering::Relaxed), Ordering::Relaxed);
    ALLOC_CALLS.store(0, Ordering::Relaxed);
    DEALLOC_CALLS.store(0, Ordering::Relaxed);
    REALLOC_CALLS.store(0, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            add_bytes(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            add_bytes(layout.size());
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
                add_bytes(new_size - layout.size());
            } else {
                LIVE_BYTES.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        resized
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err("usage: m6_f32_spin1_internal_alloc MMAX NPHI THREADS".into());
    }
    let mmax = args[1].parse::<usize>()?;
    let nphi = args[2].parse::<usize>()?;
    let threads = args[3].parse::<usize>()?;
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()?;

    let harmonics = HarmonicSpec::new(720, mmax, 1)?;
    let sample_count = 721 * nphi;
    let coefficients = vec![Complex32::new(0.0, 0.0); harmonics.nalm()];
    let mut first = vec![0.0_f32; sample_count];
    let mut second = vec![0.0_f32; sample_count];
    let baseline_live = LIVE_BYTES.load(Ordering::Relaxed);
    reset_counts();

    let plan = ShtPlan::<f32>::new(
        GridSpec::new(GridKind::GaussLegendre, 721, nphi, 0.23)?,
        harmonics,
    )?;
    let mut workspace = plan.workspace()?;
    plan.prepare_workspace(&mut workspace, threads)?;
    let setup_added_bytes = LIVE_BYTES.load(Ordering::Relaxed) - baseline_live;
    let setup_peak_extra_bytes = PEAK_BYTES.load(Ordering::Relaxed) - baseline_live;

    for _ in 0..2 {
        plan.synthesis_spin_into(
            &coefficients,
            &coefficients,
            &mut first,
            &mut second,
            &mut workspace,
        )?;
    }
    let warmed_live_bytes = LIVE_BYTES.load(Ordering::Relaxed) - baseline_live;
    let warmed_peak_extra_bytes = PEAK_BYTES.load(Ordering::Relaxed) - baseline_live;
    reset_counts();
    plan.synthesis_spin_into(
        &coefficients,
        &coefficients,
        &mut first,
        &mut second,
        &mut workspace,
    )?;

    println!(
        "ALLOC_REPORT {{\"mmax\":{mmax},\"nphi\":{nphi},\"threads\":{threads},\
         \"plan_workspace_setup_live_bytes\":{},\"plan_workspace_setup_peak_extra_bytes\":{},\
         \"plan_workspace_warmed_live_bytes\":{},\
         \"plan_workspace_warmed_peak_extra_bytes\":{},\"steady_call_allocs\":{},\
         \"steady_call_deallocs\":{},\"steady_call_reallocs\":{},\
         \"steady_call_peak_extra_bytes\":{}}}",
        setup_added_bytes,
        setup_peak_extra_bytes,
        warmed_live_bytes,
        warmed_peak_extra_bytes,
        ALLOC_CALLS.load(Ordering::Relaxed),
        DEALLOC_CALLS.load(Ordering::Relaxed),
        REALLOC_CALLS.load(Ordering::Relaxed),
        PEAK_BYTES.load(Ordering::Relaxed) - LIVE_BYTES.load(Ordering::Relaxed),
    );
    Ok(())
}
