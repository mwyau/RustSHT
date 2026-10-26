// SPDX-FileCopyrightText: 2026 Albert M. W. Yau
// SPDX-License-Identifier: GPL-2.0-or-later

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::time::Instant;

use rayon::prelude::*;

use crate::error::ShtError;
use crate::legendre::ShtScalar;
use crate::workspace::RecurrenceWorkspace;

/// Runs dynamically queued independent harmonic-order jobs on recurrence workers.
///
/// Test builds can opt selected workers into queue and job timing. Production
/// builds do not read the profiling predicate or create timers.
pub(crate) fn run_dynamic_order_jobs<T, J, P, F>(
    jobs: VecDeque<J>,
    recurrence: &mut [RecurrenceWorkspace<T>],
    profile_enabled: P,
    run_job: F,
) -> Result<(), ShtError>
where
    T: ShtScalar,
    J: Send,
    P: Fn(&RecurrenceWorkspace<T>) -> bool + Sync,
    F: Fn(&mut RecurrenceWorkspace<T>, J, bool, u128) -> Result<(), ShtError> + Sync,
{
    #[cfg(not(test))]
    let _ = &profile_enabled;

    let jobs = Mutex::new(jobs);
    let failed = AtomicBool::new(false);
    let failure = Mutex::new(None::<ShtError>);
    recurrence.par_iter_mut().for_each(|worker| {
        loop {
            if failed.load(Ordering::Relaxed) {
                break;
            }
            #[cfg(test)]
            let profile_worker = profile_enabled(worker);
            #[cfg(not(test))]
            let profile_worker = false;
            #[cfg(test)]
            let queue_lock_started = profile_worker.then(Instant::now);
            let job = jobs
                .lock()
                .expect("dynamic analysis order queue is not poisoned")
                .pop_front();
            #[cfg(test)]
            let queue_lock_ns = queue_lock_started
                .map(|started| started.elapsed().as_nanos())
                .unwrap_or(0);
            #[cfg(not(test))]
            let queue_lock_ns = 0;
            let Some(job) = job else {
                break;
            };
            if let Err(error) = run_job(worker, job, profile_worker, queue_lock_ns) {
                failed.store(true, Ordering::Relaxed);
                let mut first_error = failure
                    .lock()
                    .expect("dynamic analysis error slot is not poisoned");
                first_error.get_or_insert(error);
                break;
            }
        }
    });

    if failed.load(Ordering::Relaxed) {
        return Err(failure
            .into_inner()
            .expect("dynamic analysis error slot is not poisoned")
            .expect("a failed analysis job records its error"));
    }
    Ok(())
}

/// Chooses the order boundary with the closest total coefficient work on both sides.
#[cfg(feature = "parallel")]
pub(crate) fn weighted_order_midpoint(lmax: usize, m_start: usize, m_end: usize) -> usize {
    debug_assert!(m_start < m_end);
    debug_assert!(m_end - 1 <= lmax);
    debug_assert!(m_end - m_start > 1);

    let total_work = (m_start..m_end)
        .map(|m| (lmax - m + 1) as u128)
        .sum::<u128>();
    let mut left_work = 0_u128;
    let mut best_mid = m_start + 1;
    let mut best_difference = u128::MAX;
    for m in m_start..(m_end - 1) {
        left_work += (lmax - m + 1) as u128;
        let right_work = total_work - left_work;
        let difference = left_work.abs_diff(right_work);
        if difference < best_difference {
            best_difference = difference;
            best_mid = m + 1;
        }
    }
    best_mid
}
