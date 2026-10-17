// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

//! Test-only per-worker synthesis accounting. One unit is one latitude lane
//! evaluated at one degree pair. Discarded transition work is recorded apart
//! from committed projections. No counters are compiled into library builds.

use std::cell::RefCell;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Counts {
    pub fused_ns: u128,
    pub visitor_ns: u128,
    pub transition_ns: u128,
    pub coefficients_ns: u128,
    pub factors_ns: u128,
    pub transition_calls: u64,
    pub preparation_failures: u64,
    pub fused: u64,
    pub visitor_normal: u64,
    pub visitor_scaled: u64,
    pub transition_prefix: u64,
    pub transition_fused: u64,
    pub discarded_prefix: u64,
    pub discarded_fused: u64,
    pub completed_transitions: u64,
    pub declined_transitions: u64,
    pending_prefix: u64,
    pending_fused: u64,
}

thread_local! {
    static COUNTS: RefCell<Option<Counts>> = const { RefCell::new(None) };
}

fn update(f: impl FnOnce(&mut Counts)) {
    COUNTS.with_borrow_mut(|counts| {
        if let Some(counts) = counts {
            f(counts);
        }
    });
}

pub(super) enum Phase {
    Fused,
    Visitor,
    Transition,
    Coefficients,
    Factors,
}

pub(super) fn start() -> Option<Instant> {
    COUNTS.with_borrow(|counts| counts.is_some().then(Instant::now))
}

pub(super) fn finish(start: Option<Instant>, phase: Phase) {
    if let Some(start) = start {
        let elapsed = start.elapsed().as_nanos();
        update(|counts| match phase {
            Phase::Fused => counts.fused_ns += elapsed,
            Phase::Visitor => counts.visitor_ns += elapsed,
            Phase::Transition => counts.transition_ns += elapsed,
            Phase::Coefficients => counts.coefficients_ns += elapsed,
            Phase::Factors => counts.factors_ns += elapsed,
        });
    }
}

pub(super) fn transition_call() {
    update(|counts| counts.transition_calls += 1);
}

pub(super) fn preparation_failure() {
    update(|counts| counts.preparation_failures += 1);
}

pub(super) fn fused(lanes: usize, pairs: usize) {
    update(|counts| counts.fused += (lanes * pairs) as u64);
}

pub(super) fn visitor(lanes: usize, normal: bool) {
    update(|counts| {
        if normal {
            counts.visitor_normal += lanes as u64;
        } else {
            counts.visitor_scaled += lanes as u64;
        }
    });
}

pub(super) fn transition_begin() {
    update(|counts| {
        counts.pending_prefix = 0;
        counts.pending_fused = 0;
    });
}

pub(super) fn transition_prefix(lanes: usize) {
    update(|counts| counts.pending_prefix += lanes as u64);
}

pub(super) fn transition_fused(lanes: usize, pairs: usize) {
    update(|counts| counts.pending_fused += (lanes * pairs) as u64);
}

pub(super) fn transition_end(success: bool) {
    update(|counts| {
        if success {
            counts.transition_prefix += counts.pending_prefix;
            counts.transition_fused += counts.pending_fused;
            counts.completed_transitions += 1;
        } else {
            counts.discarded_prefix += counts.pending_prefix;
            counts.discarded_fused += counts.pending_fused;
            counts.declined_transitions += 1;
        }
        counts.pending_prefix = 0;
        counts.pending_fused = 0;
    });
}

#[cfg(feature = "parallel")]
#[test]
#[ignore = "opt-in M6 scalar synthesis route accounting"]
fn m6_scalar_synthesis_routes() {
    use crate::{Complex64, GridKind, GridSpec, HarmonicSpec, ShtPlan};
    use std::time::Instant;

    let lmax: usize = std::env::var("RUSTSHT_ROUTE_LMAX")
        .unwrap_or_else(|_| "511".into())
        .parse()
        .unwrap();
    let threads: usize = std::env::var("RUSTSHT_ROUTE_THREADS")
        .unwrap_or_else(|_| "1".into())
        .parse()
        .unwrap();
    assert!(lmax >= 511, "profile the recurrence path");
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    let plan = ShtPlan::new(
        GridSpec::new(GridKind::GaussLegendre, lmax + 1, 2 * (lmax + 1), 0.37).unwrap(),
        HarmonicSpec::scalar(lmax, lmax).unwrap(),
    )
    .unwrap();
    let mut workspace = pool.install(|| plan.workspace()).unwrap();
    let mut alm = vec![Complex64::new(0.0, 0.0); plan.harmonics().nalm()];
    for m in 0..=lmax {
        for l in m..=lmax {
            alm[plan.harmonics().index(l, m).unwrap()] = Complex64::new(
                ((l * 7 + m * 11) % 19) as f64 - 9.0,
                if m == 0 {
                    0.0
                } else {
                    ((l * 13 + m * 5) % 23) as f64 - 11.0
                },
            ) * 1e-5;
        }
    }
    let mut map = vec![0.0; (lmax + 1) * 2 * (lmax + 1)];
    pool.install(|| plan.synthesis_into(&alm, &mut map, &mut workspace))
        .unwrap();
    let start = Instant::now();
    pool.install(|| plan.synthesis_into(&alm, &mut map, &mut workspace))
        .unwrap();
    let disabled = start.elapsed();
    let reference = map.clone();
    pool.broadcast(|_| COUNTS.with_borrow_mut(|counts| *counts = Some(Counts::default())));
    let start = Instant::now();
    pool.install(|| plan.synthesis_into(&alm, &mut map, &mut workspace))
        .unwrap();
    let enabled = start.elapsed();
    assert_eq!(map, reference, "accounting must preserve every map sample");
    let workers = pool.broadcast(|_| COUNTS.with_borrow_mut(Option::take));
    let mut total = Counts::default();
    for (worker, counts) in workers.into_iter().enumerate() {
        let counts = counts.unwrap();
        println!("worker={worker} {counts:?}");
        total.fused_ns += counts.fused_ns;
        total.visitor_ns += counts.visitor_ns;
        total.transition_ns += counts.transition_ns;
        total.coefficients_ns += counts.coefficients_ns;
        total.factors_ns += counts.factors_ns;
        total.transition_calls += counts.transition_calls;
        total.preparation_failures += counts.preparation_failures;
        total.fused += counts.fused;
        total.visitor_normal += counts.visitor_normal;
        total.visitor_scaled += counts.visitor_scaled;
        total.transition_prefix += counts.transition_prefix;
        total.transition_fused += counts.transition_fused;
        total.discarded_prefix += counts.discarded_prefix;
        total.discarded_fused += counts.discarded_fused;
        total.completed_transitions += counts.completed_transitions;
        total.declined_transitions += counts.declined_transitions;
    }
    let committed = total.fused
        + total.visitor_normal
        + total.visitor_scaled
        + total.transition_prefix
        + total.transition_fused;
    let potential = (lmax + 1) * (0..=lmax).map(|m| (lmax - m + 2) / 2).sum::<usize>();
    assert!(committed > 0 && committed <= potential as u64);
    println!(
        "M6 routes L={lmax} q={threads} potential={potential} committed={committed} disabled_ms={:.3} enabled_ms={:.3} {total:?}",
        disabled.as_secs_f64() * 1000.0,
        enabled.as_secs_f64() * 1000.0
    );
}
