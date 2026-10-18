# Current scalar and spin-1 analysis phase profile

## Method

The ignored release diagnostic timed five GL STANDARD analysis calls for
scalar and spin 1 at T511 and T1023, using explicit q1 and q6 Rayon pools. The T1023 inputs used `ntheta=1024`, `nphi=2049`, full `mmax=1023`,
`phi0=0.37`, and deterministic full-band maps. Plan and workspace creation
were outside the timers. The profile separates forward longitude FFTs,
recurrence and coefficient accumulation, and the remaining preparation
phases. No warm-up call was discarded. The current production source includes parallel scheduling of independent
longitude rows. The q6 FFT phase therefore covers the Rayon row path and its
private FFT workspaces. The q1 path retains the serial row loop. This is a
phase diagnostic, not a paired performance comparison. The run used rustc `1.99.0 (b940084d7 2026-09-28)`, LLVM `23.1.1`, and
`cargo test --release` on an x86-64 CPU with host topology omitted. The process used the available CPU set without explicit affinity. Pulp runtime
dispatch and the portable fallback were enabled; the diagnostic did not log
which SIMD implementation it selected. The cgroup reported no CPU quota. Load-average telemetry was retained only in local diagnostics. The phase timing and methodology are summarized here; raw output and host metadata remain local and untracked.

## Results

Times are medians across five calls, in milliseconds. Ranges are the minimum
and maximum call times. “Residual” is the per-call wall time minus the forward
FFT and recurrence/accumulation timers; it summarizes the remaining phases
and is not a separate timer.

| Operation | Threads | Whole call | Forward FFT | Recurrence and accumulation | Residual |
| --- | ---: | ---: | ---: | ---: | ---: |
| Scalar T1023 | q1 | `154.370 (154.261–155.983)` | `35.728 (35.658–37.379)` | `116.038 (115.846–116.092)` | `2.565` |
| Scalar T1023 | q6 | `33.656 (33.126–38.422)` | `7.017 (6.708–7.207)` | `24.190 (23.453–28.413)` | `2.614` |
| Spin 1 T1023 | q1 | `247.630 (247.167–248.423)` | `72.083 (71.496–72.344)` | `174.656 (173.936–175.377)` | `0.990` |
| Spin 1 T1023 | q6 | `55.720 (54.482–60.547)` | `14.098 (13.158–17.469)` | `40.564 (40.331–42.165)` | `1.034` |

At q6, recurrence and accumulation account for about 72% of the scalar wall
median and 73% of spin-1 wall time. Forward FFT accounts for about 21% and
25%, respectively. The current row scheduling has reduced the FFT share
relative to the pre-row-scheduling phase profile, but that comparison uses
separate diagnostic runs and does not quantify a paired change. The T1023
recurrence remains the largest measured phase for both operations at q1 and
q6. The phase sums match wall time within `0.06 ms` in every call. At q6, scalar
order-spectrum preparation took a median `2.125 ms`; spin-1 coefficient
initialization took `0.979 ms`. The remaining individual measured phases were
at or below `0.5 ms`.

## Interpretation

This diagnostic supports further investigation of the scalar recurrence and
coefficient-accumulation path. It does not identify a specific instruction or
recurrence step as the cause of the remaining scalar/DUCC gap, and it does not
justify a kernel change by itself. The existing leaf profile shows that
active-ring work is balanced closely while elapsed leaf times differ; its
work estimate omits per-order and recurrence-block costs. Any new scheduling
or kernel candidate still requires alternating end-to-end process pairs and
correctness checks.
