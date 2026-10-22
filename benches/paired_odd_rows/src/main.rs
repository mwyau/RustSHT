// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::alloc::{GlobalAlloc, Layout, System};
use std::env;
use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use num_complex::Complex64;
use rayon::ThreadPool;
use rayon::prelude::*;
use rustfft::algorithm::MixedRadix;
use rustfft::{Fft, FftPlanner};

mod profile;

#[path = "../../../src/error.rs"]
#[allow(dead_code)]
mod error;
#[path = "../../../src/fft.rs"]
#[allow(dead_code)]
mod real_fft;

use real_fft::{RealFftPlan, RealFftWorkspace};

type C = Complex64;

const BATCHES: usize = 30;
const TARGET_BATCH: Duration = Duration::from_millis(30);

static COUNT_ALLOCATIONS: AtomicBool = AtomicBool::new(false);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

struct CountingAllocator;

// The counter is enabled only around one warmed execution in this local
// benchmark. Allocation and deallocation still use the system allocator.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: the layout is forwarded unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the pointer was allocated by the same system allocator.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if COUNT_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: the layout is forwarded unchanged to the system allocator.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNT_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: the pointer and old layout are forwarded unchanged.
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    Forward,
    Inverse,
}

impl Operation {
    fn parse(value: &str) -> Self {
        match value {
            "forward" => Self::Forward,
            "inverse" => Self::Inverse,
            _ => panic!("operation must be forward or inverse"),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Inverse => "inverse",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Route {
    RustReal,
    RustC2c,
    RustPair,
    RustPairMixed,
}

impl Route {
    fn parse(value: &str) -> Self {
        match value {
            "rust-real" => Self::RustReal,
            "rust-c2c" => Self::RustC2c,
            "rust-pair" => Self::RustPair,
            "rust-pair-mixed" => Self::RustPairMixed,
            _ => panic!("route must be rust-real, rust-c2c, rust-pair, or rust-pair-mixed"),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::RustReal => "rust-real",
            Self::RustC2c => "rust-c2c",
            Self::RustPair => "rust-pair",
            Self::RustPairMixed => "rust-pair-mixed",
        }
    }

    const fn is_paired(self) -> bool {
        matches!(self, Self::RustPair | Self::RustPairMixed)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SpectrumShape {
    Full,
    Reduced,
    Polar,
}

impl SpectrumShape {
    fn parse(value: &str) -> Self {
        match value {
            "full" => Self::Full,
            "reduced" => Self::Reduced,
            "polar" => Self::Polar,
            _ => panic!("spectrum shape must be full, reduced, or polar"),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Reduced => "reduced",
            Self::Polar => "polar",
        }
    }
}

struct Plans {
    n: usize,
    bins: usize,
    modes: usize,
    real: RealFftPlan<f64>,
    c2c_forward: Arc<dyn Fft<f64>>,
    c2c_inverse: Arc<dyn Fft<f64>>,
    c2c_inverse_mixed: Option<Arc<dyn Fft<f64>>>,
    real_workspace_bytes: usize,
    c2c_workspace_bytes: usize,
    mixed_workspace_bytes: usize,
}

impl Plans {
    fn new(n: usize, modes: usize) -> Self {
        assert!(n >= 2);
        let bins = n / 2 + 1;
        assert!((1..=bins).contains(&modes));
        let real = RealFftPlan::new(n).expect("RustSHT real FFT plan");

        let mut planner = FftPlanner::<f64>::new();
        let real_transform_len = if n.is_multiple_of(2) { n / 2 } else { n };
        let real_forward = planner.plan_fft_forward(real_transform_len);
        let real_inverse = planner.plan_fft_inverse(real_transform_len);
        let real_mixed = if n == 19 * 19 {
            let radix19 = planner.plan_fft_inverse(19);
            Some(Arc::new(MixedRadix::new(Arc::clone(&radix19), radix19)) as Arc<dyn Fft<f64>>)
        } else {
            None
        };
        let real_scratch = real_forward
            .get_inplace_scratch_len()
            .max(real_inverse.get_inplace_scratch_len())
            .max(
                real_mixed
                    .as_ref()
                    .map_or(0, |plan| plan.get_inplace_scratch_len()),
            );
        let real_workspace_bytes = (real_transform_len + real_scratch) * size_of::<C>();

        let c2c_forward = planner.plan_fft_forward(n);
        let c2c_inverse = planner.plan_fft_inverse(n);
        let c2c_inverse_mixed = if n == 19 * 19 {
            let radix19 = planner.plan_fft_inverse(19);
            Some(Arc::new(MixedRadix::new(Arc::clone(&radix19), radix19)) as Arc<dyn Fft<f64>>)
        } else {
            None
        };
        let ordinary_scratch = c2c_forward
            .get_inplace_scratch_len()
            .max(c2c_inverse.get_inplace_scratch_len());
        let mixed_scratch = c2c_inverse_mixed.as_ref().map_or(ordinary_scratch, |plan| {
            ordinary_scratch.max(plan.get_inplace_scratch_len())
        });

        Self {
            n,
            bins,
            modes,
            real,
            c2c_forward,
            c2c_inverse,
            c2c_inverse_mixed,
            real_workspace_bytes,
            c2c_workspace_bytes: (n + ordinary_scratch) * size_of::<C>(),
            mixed_workspace_bytes: (n + mixed_scratch) * size_of::<C>(),
        }
    }

    fn inverse_for(&self, route: Route) -> &dyn Fft<f64> {
        if route == Route::RustPairMixed {
            self.c2c_inverse_mixed
                .as_deref()
                .unwrap_or(self.c2c_inverse.as_ref())
        } else {
            self.c2c_inverse.as_ref()
        }
    }

    fn workspace_bytes(&self, route: Route) -> usize {
        match route {
            Route::RustReal => self.real_workspace_bytes,
            Route::RustC2c | Route::RustPair => self.c2c_workspace_bytes,
            Route::RustPairMixed => self.mixed_workspace_bytes,
        }
    }
}

struct WorkerWorkspace {
    real: RealFftWorkspace<f64>,
    transform: Vec<C>,
    scratch: Vec<C>,
}

impl WorkerWorkspace {
    fn new(plans: &Plans) -> Self {
        let scratch_len = plans
            .c2c_forward
            .get_inplace_scratch_len()
            .max(plans.c2c_inverse.get_inplace_scratch_len())
            .max(
                plans
                    .c2c_inverse_mixed
                    .as_ref()
                    .map_or(0, |plan| plan.get_inplace_scratch_len()),
            );
        Self {
            real: plans.real.workspace().expect("RustSHT real FFT workspace"),
            transform: vec![C::new(0.0, 0.0); plans.n],
            scratch: vec![C::new(0.0, 0.0); scratch_len],
        }
    }
}

enum Fixture {
    Forward { samples: Vec<f64> },
    Inverse { spectra: Vec<C> },
}

struct Benchmark {
    operation: Operation,
    route: Route,
    shape: SpectrumShape,
    rows: usize,
    threads: usize,
    plans: Plans,
    fixture: Fixture,
    output_real: Vec<f64>,
    output_spectrum: Vec<C>,
    workspaces: Vec<WorkerWorkspace>,
    pool: ThreadPool,
}

impl Benchmark {
    fn new(
        operation: Operation,
        route: Route,
        n: usize,
        rows: usize,
        threads: usize,
        shape: SpectrumShape,
        input_path: &Path,
    ) -> Self {
        assert!(rows >= 2 && threads >= 1);
        let bins = n / 2 + 1;
        let max_order = match shape {
            SpectrumShape::Full | SpectrumShape::Polar => bins - 1,
            SpectrumShape::Reduced => (bins - 1) / 2,
        };
        let modes = max_order + 1;
        let plans = Plans::new(n, modes);
        let fixture = read_fixture(input_path, operation, n, rows, bins);
        let workspaces = (0..threads).map(|_| WorkerWorkspace::new(&plans)).collect();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("Rust row worker pool");
        let output_len = if operation == Operation::Inverse {
            rows * n
        } else {
            rows * modes
        };

        Self {
            operation,
            route,
            shape,
            rows,
            threads,
            plans,
            fixture,
            output_real: vec![
                0.0;
                if operation == Operation::Inverse {
                    output_len
                } else {
                    0
                }
            ],
            output_spectrum: vec![
                C::new(0.0, 0.0);
                if operation == Operation::Forward {
                    output_len
                } else {
                    0
                }
            ],
            workspaces,
            pool,
        }
    }

    fn execute(&mut self) {
        let pair_count = self.rows / 2;
        let even_rows = pair_count * 2;
        let pairs_per_worker = pair_count.div_ceil(self.threads).max(1);
        let rows_per_worker = pairs_per_worker * 2;
        let Benchmark {
            operation,
            route,
            shape,
            rows,
            threads,
            plans,
            fixture,
            output_real,
            output_spectrum,
            workspaces,
            pool,
            ..
        } = self;

        match (operation, fixture) {
            (Operation::Forward, Fixture::Forward { samples }) => {
                let chunk_len = rows_per_worker * plans.modes;
                if even_rows > 0 {
                    if *threads == 1 {
                        process_forward_rows(
                            *route,
                            *shape,
                            0,
                            even_rows,
                            plans,
                            samples,
                            &mut output_spectrum[..even_rows * plans.modes],
                            &mut workspaces[0],
                        );
                    } else {
                        pool.install(|| {
                            output_spectrum[..even_rows * plans.modes]
                                .par_chunks_mut(chunk_len)
                                .zip(workspaces.par_iter_mut())
                                .enumerate()
                                .for_each(|(group, (output, workspace))| {
                                    let row_start = group * rows_per_worker;
                                    let row_end = (row_start + rows_per_worker).min(even_rows);
                                    process_forward_rows(
                                        *route, *shape, row_start, row_end, plans, samples, output,
                                        workspace,
                                    );
                                });
                        });
                    }
                }
                if *rows > even_rows {
                    process_forward_rows(
                        Route::RustReal,
                        *shape,
                        even_rows,
                        *rows,
                        plans,
                        samples,
                        &mut output_spectrum[even_rows * plans.modes..],
                        &mut workspaces[0],
                    );
                }
            }
            (Operation::Inverse, Fixture::Inverse { spectra }) => {
                let chunk_len = rows_per_worker * plans.n;
                if even_rows > 0 {
                    if *threads == 1 {
                        process_inverse_rows(
                            *route,
                            *shape,
                            0,
                            even_rows,
                            plans,
                            spectra,
                            &mut output_real[..even_rows * plans.n],
                            &mut workspaces[0],
                        );
                    } else {
                        pool.install(|| {
                            output_real[..even_rows * plans.n]
                                .par_chunks_mut(chunk_len)
                                .zip(workspaces.par_iter_mut())
                                .enumerate()
                                .for_each(|(group, (output, workspace))| {
                                    let row_start = group * rows_per_worker;
                                    let row_end = (row_start + rows_per_worker).min(even_rows);
                                    process_inverse_rows(
                                        *route, *shape, row_start, row_end, plans, spectra, output,
                                        workspace,
                                    );
                                });
                        });
                    }
                }
                if *rows > even_rows {
                    process_inverse_rows(
                        Route::RustReal,
                        *shape,
                        even_rows,
                        *rows,
                        plans,
                        spectra,
                        &mut output_real[even_rows * plans.n..],
                        &mut workspaces[0],
                    );
                }
            }
            _ => panic!("fixture operation and benchmark operation differ"),
        }
    }

    fn checksum(&self) -> f64 {
        match self.operation {
            Operation::Forward => self
                .output_spectrum
                .iter()
                .map(|value| value.re + value.im)
                .sum(),
            Operation::Inverse => self.output_real.iter().sum(),
        }
    }

    fn write_output(&self, path: &Path) {
        let mut bytes = Vec::new();
        match self.operation {
            Operation::Forward => {
                bytes.reserve(self.output_spectrum.len() * 16);
                for value in &self.output_spectrum {
                    bytes.extend_from_slice(&value.re.to_le_bytes());
                    bytes.extend_from_slice(&value.im.to_le_bytes());
                }
            }
            Operation::Inverse => {
                bytes.reserve(self.output_real.len() * 8);
                for value in &self.output_real {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
        fs::write(path, bytes).expect("write row-pair benchmark output");
    }
}

fn process_forward_rows(
    route: Route,
    _shape: SpectrumShape,
    row_start: usize,
    row_end: usize,
    plans: &Plans,
    input: &[f64],
    output: &mut [C],
    workspace: &mut WorkerWorkspace,
) {
    let mut row = row_start;
    if route.is_paired() {
        while row + 1 < row_end {
            let local = row - row_start;
            let input_first = &input[row * plans.n..(row + 1) * plans.n];
            let input_second = &input[(row + 1) * plans.n..(row + 2) * plans.n];
            let split = (local + 1) * plans.modes;
            let (before, after) = output.split_at_mut(split);
            let first_output = &mut before[local * plans.modes..split];
            let second_output = &mut after[..plans.modes];
            pair_forward(
                plans,
                input_first,
                input_second,
                first_output,
                second_output,
                workspace,
            );
            row += 2;
        }
        if row < row_end {
            forward_single(
                Route::RustReal,
                plans,
                &input[row * plans.n..(row + 1) * plans.n],
                &mut output[(row - row_start) * plans.modes..(row - row_start + 1) * plans.modes],
                workspace,
            );
        }
        return;
    }

    while row < row_end {
        forward_single(
            route,
            plans,
            &input[row * plans.n..(row + 1) * plans.n],
            &mut output[(row - row_start) * plans.modes..(row - row_start + 1) * plans.modes],
            workspace,
        );
        row += 1;
    }
}

fn forward_single(
    route: Route,
    plans: &Plans,
    input: &[f64],
    output: &mut [C],
    workspace: &mut WorkerWorkspace,
) {
    match route {
        Route::RustReal => plans
            .real
            .forward_truncated(input, output, &mut workspace.real)
            .expect("RustSHT real forward"),
        Route::RustC2c => direct_forward(plans, input, output, workspace),
        Route::RustPair | Route::RustPairMixed => unreachable!("paired row path"),
    }
}

fn direct_forward(plans: &Plans, input: &[f64], output: &mut [C], workspace: &mut WorkerWorkspace) {
    for (slot, &value) in workspace.transform.iter_mut().zip(input) {
        *slot = C::new(value, 0.0);
    }
    plans
        .c2c_forward
        .process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
    output.copy_from_slice(&workspace.transform[..plans.modes]);
    output[0].im = 0.0;
    if plans.n.is_multiple_of(2) && plans.modes > plans.n / 2 {
        output[plans.n / 2].im = 0.0;
    }
}

fn pair_forward(
    plans: &Plans,
    first: &[f64],
    second: &[f64],
    first_output: &mut [C],
    second_output: &mut [C],
    workspace: &mut WorkerWorkspace,
) {
    let component_bound = f64::MAX / (8.0 * plans.n as f64);
    let mut safe = true;
    let mut first_scale: f64 = 0.0;
    let mut second_scale: f64 = 0.0;
    for j in 0..plans.n {
        safe &= pair_component_safe(first[j], component_bound)
            && pair_component_safe(second[j], component_bound);
        if first[j].is_finite() {
            first_scale = first_scale.max(first[j].abs());
        }
        if second[j].is_finite() {
            second_scale = second_scale.max(second[j].abs());
        }
        workspace.transform[j] = C::new(first[j], second[j]);
    }
    if !safe || !paired_scales_safe(first_scale, second_scale) {
        direct_forward(plans, first, first_output, workspace);
        direct_forward(plans, second, second_output, workspace);
        return;
    }
    plans
        .c2c_forward
        .process_with_scratch(&mut workspace.transform, &mut workspace.scratch);

    let mut subnormal_output = false;
    for k in 0..plans.modes {
        let value = workspace.transform[k];
        let mirror = workspace.transform[(plans.n - k) % plans.n].conj();
        let sum = value + mirror;
        let difference = value - mirror;
        first_output[k] = sum * 0.5;
        second_output[k] = C::new(difference.im * 0.5, -difference.re * 0.5);
        subnormal_output |=
            complex_has_subnormal(first_output[k]) || complex_has_subnormal(second_output[k]);
    }
    if subnormal_output {
        direct_forward(plans, first, first_output, workspace);
        direct_forward(plans, second, second_output, workspace);
        return;
    }
    first_output[0].im = 0.0;
    second_output[0].im = 0.0;
    if plans.n.is_multiple_of(2) && plans.modes > plans.n / 2 {
        first_output[plans.n / 2].im = 0.0;
        second_output[plans.n / 2].im = 0.0;
    }
}

fn process_inverse_rows(
    route: Route,
    _shape: SpectrumShape,
    row_start: usize,
    row_end: usize,
    plans: &Plans,
    input: &[C],
    output: &mut [f64],
    workspace: &mut WorkerWorkspace,
) {
    let mut row = row_start;
    if route.is_paired() {
        while row + 1 < row_end {
            let local = row - row_start;
            let first = &input[row * plans.bins..(row + 1) * plans.bins];
            let second = &input[(row + 1) * plans.bins..(row + 2) * plans.bins];
            let split = (local + 1) * plans.n;
            let (before, after) = output.split_at_mut(split);
            let first_output = &mut before[local * plans.n..split];
            let second_output = &mut after[..plans.n];
            pair_inverse(
                plans,
                route,
                first,
                second,
                first_output,
                second_output,
                workspace,
            );
            row += 2;
        }
        if row < row_end {
            inverse_single(
                Route::RustReal,
                plans,
                &input[row * plans.bins..(row + 1) * plans.bins],
                &mut output[(row - row_start) * plans.n..(row - row_start + 1) * plans.n],
                workspace,
            );
        }
        return;
    }

    while row < row_end {
        let local = row - row_start;
        inverse_single(
            route,
            plans,
            &input[row * plans.bins..(row + 1) * plans.bins],
            &mut output[local * plans.n..(local + 1) * plans.n],
            workspace,
        );
        row += 1;
    }
}

fn inverse_single(
    route: Route,
    plans: &Plans,
    input: &[C],
    output: &mut [f64],
    workspace: &mut WorkerWorkspace,
) {
    match route {
        Route::RustReal => plans
            .real
            .inverse_series(input, output, &mut workspace.real)
            .expect("RustSHT real inverse series"),
        Route::RustC2c => direct_inverse(plans, input, output, workspace),
        Route::RustPair | Route::RustPairMixed => unreachable!("paired row path"),
    }
}

fn direct_inverse(plans: &Plans, input: &[C], output: &mut [f64], workspace: &mut WorkerWorkspace) {
    check_real_edges(plans.n, input);
    workspace.transform.fill(C::new(0.0, 0.0));
    workspace.transform[0] = C::new(input[0].re, 0.0);
    let highest = plans.n / 2;
    for k in 1..=highest {
        workspace.transform[k] = input[k];
        if !(plans.n.is_multiple_of(2) && k == highest) {
            workspace.transform[plans.n - k] = input[k].conj();
        }
    }
    plans
        .c2c_inverse
        .process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
    for (sample, value) in output.iter_mut().zip(&workspace.transform) {
        *sample = value.re;
    }
}

fn pair_inverse(
    plans: &Plans,
    route: Route,
    first: &[C],
    second: &[C],
    first_output: &mut [f64],
    second_output: &mut [f64],
    workspace: &mut WorkerWorkspace,
) {
    check_real_edges(plans.n, first);
    check_real_edges(plans.n, second);
    workspace.transform.fill(C::new(0.0, 0.0));
    workspace.transform[0] = C::new(first[0].re, second[0].re);

    let component_bound = f64::MAX / (8.0 * plans.n as f64);
    let mut safe = complex_components_safe(first[0], component_bound)
        && complex_components_safe(second[0], component_bound);
    let mut first_scale = first[0].re.abs();
    let mut second_scale = second[0].re.abs();
    let highest = plans.n / 2;
    for k in 1..=highest {
        let x = first[k];
        let y = second[k];
        safe &= complex_components_safe(x, component_bound)
            && complex_components_safe(y, component_bound);
        if x.re.is_finite() && x.im.is_finite() {
            first_scale = first_scale.max(x.re.abs()).max(x.im.abs());
        }
        if y.re.is_finite() && y.im.is_finite() {
            second_scale = second_scale.max(y.re.abs()).max(y.im.abs());
        }
        if plans.n.is_multiple_of(2) && k == highest {
            workspace.transform[k] = C::new(x.re, y.re);
        } else {
            workspace.transform[k] = C::new(x.re - y.im, x.im + y.re);
            workspace.transform[plans.n - k] = C::new(x.re + y.im, -x.im + y.re);
        }
    }
    if !safe || !paired_scales_safe(first_scale, second_scale) {
        inverse_single(Route::RustReal, plans, first, first_output, workspace);
        inverse_single(Route::RustReal, plans, second, second_output, workspace);
        return;
    }

    plans
        .inverse_for(route)
        .process_with_scratch(&mut workspace.transform, &mut workspace.scratch);
    if workspace
        .transform
        .iter()
        .any(|value| value.re.is_subnormal() || value.im.is_subnormal())
    {
        inverse_single(Route::RustReal, plans, first, first_output, workspace);
        inverse_single(Route::RustReal, plans, second, second_output, workspace);
        return;
    }
    for ((first, second), value) in first_output
        .iter_mut()
        .zip(second_output)
        .zip(&workspace.transform)
    {
        *first = value.re;
        *second = value.im;
    }
}

fn pair_component_safe(value: f64, bound: f64) -> bool {
    value.is_finite() && !value.is_subnormal() && value.abs() <= bound
}

fn complex_components_safe(value: C, bound: f64) -> bool {
    pair_component_safe(value.re, bound) && pair_component_safe(value.im, bound)
}

fn complex_has_subnormal(value: C) -> bool {
    value.re.is_subnormal() || value.im.is_subnormal()
}

fn paired_scales_safe(first: f64, second: f64) -> bool {
    if first == 0.0 || second == 0.0 {
        return first == 0.0 && second == 0.0;
    }
    first.min(second) >= first.max(second) / 8.0
}

fn check_real_edges(n: usize, spectrum: &[C]) {
    assert_eq!(spectrum[0].im, 0.0, "non-real DC");
    if n.is_multiple_of(2) {
        assert_eq!(spectrum[n / 2].im, 0.0, "non-real Nyquist");
    }
}

fn spectrum_mode_limit(shape: SpectrumShape, row: usize, rows: usize, max_order: usize) -> usize {
    match shape {
        SpectrumShape::Full => max_order,
        SpectrumShape::Reduced => max_order / 2,
        SpectrumShape::Polar => {
            let theta = std::f64::consts::PI * (row as f64 + 0.5) / rows as f64;
            ((max_order as f64 * theta.sin().abs()).floor() as usize).min(max_order)
        }
    }
}

fn make_fixture(operation: Operation, n: usize, rows: usize, shape: SpectrumShape) -> Fixture {
    let bins = n / 2 + 1;
    let max_order = match shape {
        SpectrumShape::Full | SpectrumShape::Polar => bins - 1,
        SpectrumShape::Reduced => (bins - 1) / 2,
    };
    match operation {
        Operation::Forward => {
            let mut samples = vec![0.0; rows * n];
            for row in 0..rows {
                for j in 0..n {
                    let x = (row * n + j + 1) as f64;
                    samples[row * n + j] = 0.7 * (x * 0.017).sin() + 0.1 * (x * 0.003).cos();
                }
            }
            Fixture::Forward { samples }
        }
        Operation::Inverse => {
            let mut spectra = vec![C::new(0.0, 0.0); rows * bins];
            for row in 0..rows {
                let limit = spectrum_mode_limit(shape, row, rows, max_order);
                for k in 0..=limit {
                    let x = (row * bins + k + 1) as f64;
                    let mut value = C::new(0.7 * (x * 0.017).sin(), 0.4 * (x * 0.013).cos());
                    if k == 0 || (n.is_multiple_of(2) && k == n / 2) {
                        value.im = 0.0;
                    }
                    spectra[row * bins + k] = value;
                }
            }
            Fixture::Inverse { spectra }
        }
    }
}

fn write_fixture_file(path: &Path, fixture: &Fixture) {
    let mut bytes = Vec::new();
    match fixture {
        Fixture::Forward { samples } => {
            bytes.reserve(samples.len() * 8);
            for value in samples {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        Fixture::Inverse { spectra } => {
            bytes.reserve(spectra.len() * 16);
            for value in spectra {
                bytes.extend_from_slice(&value.re.to_le_bytes());
                bytes.extend_from_slice(&value.im.to_le_bytes());
            }
        }
    }
    fs::write(path, bytes).expect("write row-pair input fixture");
}

fn read_fixture(path: &Path, operation: Operation, n: usize, rows: usize, bins: usize) -> Fixture {
    let bytes = fs::read(path).expect("read row-pair input fixture");
    let expected_words = if operation == Operation::Forward {
        rows * n
    } else {
        rows * bins * 2
    };
    assert_eq!(bytes.len(), expected_words * 8, "fixture byte count");
    let values: Vec<f64> = bytes
        .chunks_exact(8)
        .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
        .collect();
    match operation {
        Operation::Forward => Fixture::Forward { samples: values },
        Operation::Inverse => Fixture::Inverse {
            spectra: values
                .chunks_exact(2)
                .map(|value| C::new(value[0], value[1]))
                .collect(),
        },
    }
}

fn elapsed_per_call<F: FnMut()>(calls: usize, mut operation: F) -> f64 {
    let start = Instant::now();
    for _ in 0..calls {
        operation();
        black_box(0x91a7_u64);
    }
    start.elapsed().as_secs_f64() * 1.0e3 / calls as f64
}

fn percentile(sorted: &[f64], numerator: usize, denominator: usize) -> f64 {
    sorted[(sorted.len() - 1) * numerator / denominator]
}

fn run_benchmark(
    route: Route,
    operation: Operation,
    n: usize,
    rows: usize,
    threads: usize,
    shape: SpectrumShape,
    input_path: &Path,
    output_path: &Path,
) {
    let mut bench = Benchmark::new(operation, route, n, rows, threads, shape, input_path);
    let first_ms = elapsed_per_call(1, || bench.execute());
    for _ in 0..8 {
        bench.execute();
    }

    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    COUNT_ALLOCATIONS.store(true, Ordering::Relaxed);
    bench.execute();
    COUNT_ALLOCATIONS.store(false, Ordering::Relaxed);
    let warm_allocations = ALLOCATION_COUNT.load(Ordering::Relaxed);

    let calibration_ms = elapsed_per_call(12, || bench.execute());
    let calls = ((TARGET_BATCH.as_secs_f64() * 1.0e3 / calibration_ms).round() as usize).max(1);
    let mut samples = Vec::with_capacity(BATCHES);
    for _ in 0..BATCHES {
        samples.push(elapsed_per_call(calls, || bench.execute()));
    }
    let mut ordered = samples.clone();
    ordered.sort_by(f64::total_cmp);
    let p10 = percentile(&ordered, 1, 10);
    let median = ordered[ordered.len() / 2];
    let p90 = percentile(&ordered, 9, 10);
    let row_pairs = rows as f64 / 2.0;
    let checksum = bench.checksum();
    assert!(checksum.is_finite(), "nonfinite benchmark checksum");
    bench.write_output(output_path);

    println!(
        "environment,route={},operation={},n={},rows={},modes={},shape={},threads={},batch_calls={},batches={},target_batch_ms={:.1},first_ms={first_ms:.6},calibration_ms={calibration_ms:.6},scratch_bytes_per_worker={},runtime_avx2={},runtime_fma={},mixed_radix_361_inverse={}",
        route.as_str(),
        operation.as_str(),
        n,
        rows,
        bench.plans.modes,
        shape.as_str(),
        threads,
        calls,
        BATCHES,
        TARGET_BATCH.as_secs_f64() * 1.0e3,
        bench.plans.workspace_bytes(route),
        std::is_x86_feature_detected!("avx2"),
        std::is_x86_feature_detected!("fma"),
        operation == Operation::Inverse
            && n == 361
            && (route == Route::RustPairMixed || route == Route::RustReal),
    );
    println!(
        "summary,p10_ms={p10:.6},median_ms={median:.6},p90_ms={p90:.6},us_per_row_pair={:.6},warm_allocations={warm_allocations},checksum={checksum:.12e}",
        median * 1000.0 / row_pairs,
    );
    for (index, sample) in samples.iter().enumerate() {
        println!("batch,{index},ms={sample:.6}");
    }
}

fn run_once(
    route: Route,
    operation: Operation,
    n: usize,
    rows: usize,
    threads: usize,
    shape: SpectrumShape,
    input_path: &Path,
    output_path: &Path,
) {
    let mut bench = Benchmark::new(operation, route, n, rows, threads, shape, input_path);
    bench.execute();
    let checksum = bench.checksum();
    assert!(checksum.is_finite(), "nonfinite one-shot checksum");
    bench.write_output(output_path);
    println!(
        "once,route={},operation={},n={n},rows={rows},threads={threads},shape={},checksum={checksum:.12e}",
        route.as_str(),
        operation.as_str(),
        shape.as_str(),
    );
}

fn usage() -> ! {
    panic!(
        "usage:\n  paired-odd-rows --write-fixture OPERATION N ROWS SHAPE PATH\n  paired-odd-rows ROUTE OPERATION N ROWS THREADS SHAPE INPUT OUTPUT"
    );
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() == 7 && args[1] == "--profile" {
        profile::run_profile(
            Route::parse(&args[2]),
            Operation::parse(&args[3]),
            args[4].parse().expect("FFT length"),
            SpectrumShape::parse(&args[5]),
            Path::new(&args[6]),
        );
        return;
    }
    if args.len() == 7 && args[1] == "--write-fixture" {
        let operation = Operation::parse(&args[2]);
        let n = args[3].parse().expect("FFT length");
        let rows = args[4].parse().expect("row count");
        let shape = SpectrumShape::parse(&args[5]);
        let fixture = make_fixture(operation, n, rows, shape);
        write_fixture_file(Path::new(&args[6]), &fixture);
        return;
    }
    if args.len() == 10 && args[1] == "--once" {
        run_once(
            Route::parse(&args[2]),
            Operation::parse(&args[3]),
            args[4].parse().expect("FFT length"),
            args[5].parse().expect("row count"),
            args[6].parse().expect("thread count"),
            SpectrumShape::parse(&args[7]),
            Path::new(&args[8]),
            Path::new(&args[9]),
        );
        return;
    }
    if args.len() != 9 {
        usage();
    }
    run_benchmark(
        Route::parse(&args[1]),
        Operation::parse(&args[2]),
        args[3].parse().expect("FFT length"),
        args[4].parse().expect("row count"),
        args[5].parse().expect("thread count"),
        SpectrumShape::parse(&args[6]),
        Path::new(&args[7]),
        Path::new(&args[8]),
    );
}
