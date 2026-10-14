# Adaptive Legendre selector recheck at current `main`

## Source audit

The selector must be evaluated against commit `3369014`. The earlier adaptive
strategy report contains the broad table-versus-recurrence study; the 2026-10-14
refresh covers the intervening spin-1 synthesis table gates. Those measurements
precede two recurrence changes that can alter the relative costs: the fused Q/U
spin-1 analysis reducer in `614db91` and staged synthesis coefficient
factorization in `3369014`. Both scalar and spin-1 Legendre tables are lazy `OnceLock` values. A scalar plan
does not build a table during construction. STANDARD spin-1 synthesis and
analysis can share one lazy table when their operation-specific gates accept
it. Table support is restricted to f64 GL, `63 <= lmax <= 255` (spin 1 stores
the degree extension through `lmax+1`), near-full `mmax`, at most twice the
minimum GL latitude count, and a 34 MiB payload. Input-range checks keep unsafe
direct products on the scaled recurrence. Scalar selection reads the active
Rayon pool at call time. Spin-1 has separate synthesis and analysis gates; its
synthesis table also requires a single-thread pool, exact GL dimensions and
the measured polar sine range. GRAD_ONLY, DERIV1, f32, CC and F1 use their
existing recurrence or specialized paths. The `sht_bench` reference at `1328f83feb5dc95b3e6708b51228d954820ff15a` uses
full atmospheric GL grids (`2F x 4F`) and CC grids with `nlat=2L+1`,
`nlon=4L`. RustSHT's minimum exact GL grids have different dimensions. Its
README and result plots establish machine and backend context; they do not
provide matched recurrence/table comparisons. Its process-isolated thread
cells, setup-excluded transform timing, and retained samples are suitable
methodological references. SHTns `sht_auto`/`sht_gauss` and `sht_gauss_fly`
select Gaussian grids, while `sht_quick_init` avoids the setup timing search. The reviewed SHTns source measures operation-specific on-the-fly variants and
checks accuracy. This supports operation-specific policy and measured
accuracy, but its setup-time search is not used as a RustSHT plan policy.

## Experiment plan

Before any selector or table-construction production change:

1. Build automatic, forced-recurrence and forced-table benchmark binaries
   from the same `3369014` source in a detached temporary worktree. Put the
   forcing controls only in that worktree and remove it after collecting data. Keep numerical guards, latitude geometry and table-size eligibility intact.
2. Recheck cells whose recurrence cost changed: scalar synthesis around the
   table boundary; spin-1 STANDARD synthesis at T127/T144/T159/T175/T191 q1;
   spin-1 STANDARD analysis at T85/T127/T159/T191 for q1/q8 and representative
   q2/q4/q16 pools. Use full `mmax` first, then the previously measured reduced
   `mmax` boundaries if an operation changes direction.
3. Compare exact-minimum grids and the matching atmospheric GL dimensions at
   T63/T85/T127/T159/T191 where the table bound permits. Preserve existing
   CC/F1, f32, GRAD_ONLY and DERIV1 controls. Each cell records `ntheta`,
   `nphi`, `lmax`, `mmax`, operation, thread count, process order and raw
   samples.
4. Separate table plan construction, first-use build, workspace allocation,
   first transform and warm transform. Use five timed calls per process, paired
   alternating order, CPU affinity and one, two, four, eight and sixteen
   threads where the selector differs. Measure resident memory and calculate
   reuse totals at 1, 5, 10, 25, 50, 100 and 1,000 calls.
5. Compare numerical outputs against installed DUCC 0.41.0 and existing
   analytic, adjoint, polar, equatorial and extreme-input tests. Accept a
   selector change only if a repeated whole-operation gain exceeds process
   variation after first-use cost and retained memory are included. Otherwise
   keep the selector and record the evidence. The first implementation will be benchmark-only. No production selector,
public API, FFT code or table budget changes until these comparisons finish.
