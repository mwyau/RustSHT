# Exponent-epoch scalar synthesis q8 screen, protocol v1

## Purpose

Measure the frozen 128-ring f64 GL scalar candidate against the same clean
baseline at T1023 and T2047 with eight workers. The matched q1 screen passed
for both bandlimits. The T511 q1 control is unresolved because its process
order and output labels were not reliable; this q8 screen cannot resolve that
control or accept the candidate.

## Sources and workload

Use the exact baseline and candidate extensions, reference libraries and
`examples/python_perf_baseline.py` harness identified in the [q1 protocol](exponent-epoch-q1-screen-v1-protocol.md).
Do not rebuild either extension or change its environment during acquisition.

Each process runs f64 GL scalar `STANDARD` synthesis into a reusable output
map, full `mmax`, `phi0=0.37`, one warm-up and five timed calls per backend.
Use `(L+1) × 2(L+1)` with `L=1023` or `2047`. Set both `RAYON_NUM_THREADS`
and the harness thread argument to 8. DUCC and SHTns also use eight threads.
The CPU-set policy is eight distinct physical cores, one logical CPU per core,
with no SMT sibling CPUs. Record the actual worker-thread placement locally.

Run six independent baseline/candidate pairs at T1023, followed by six at
T2047. Alternate baseline/candidate for pair 1, candidate/baseline for pair
2, and continue alternating. Do not start T2047 if a T1023 process fails a
validity gate. Do not run compilation or another benchmark during acquisition.

The workload command template is:

```sh
taskset -c "${RUSTSHT_CPUSET:?eight physical CPUs}" \
  env PYTHONPATH="$PYTHONPATH" \
  RAYON_NUM_THREADS=8 RUSTSHT_PERF_RAW=1 \
  RUSTSHT_BENCH_PHI0=0.37 OPENBLAS_NUM_THREADS=1 \
  RUSTSHT_SHTNS_LIB="$RUSTSHT_SHTNS_LIB" \
  "$PYTHON" examples/python_perf_baseline.py \
  "$L" 0 8 GL 1 into synthesis all STANDARD \
  "$((2*(L+1)))" "$((L+1))"
```

Set `PYTHONPATH` to the selected RustSHT package and the pinned DUCC package;
the harness loads the SHTns library through `RUSTSHT_SHTNS_LIB`. The local
runner also records preflight, during-run and postflight validity evidence.
It must reject any pre-existing output label before launching this command.

Every output label must be new. The runner must refuse to start if any file
with that label already exists; it must not overwrite or delete previous
captures. Use distinct labels for each bandlimit, pair and variant. Preserve
all captures locally.

## Validity and decision rules

- Reject preflight or postflight CPU PSI `some avg10` above 2.00%. Report the
  exact preflight and postflight `avg10` values for each process. Values from
  0.50% through 2.00% are warnings.
- Require population CV at most 5% and first-three/final-two median drift at
  most 5% for each RustSHT, DUCC and SHTns five-call series. Do not remove or
  retry samples.
- Review affinity, worker placement, CPU-frequency behavior, involuntary
  context switches, cgroup throttling, concurrent CPU-intensive work and PSI
  after every process. Clear interference makes a process inconclusive even
  when PSI is below 2.00%.
- Require candidate relative-L2 error against DUCC at most `2e-11` and
  maximum absolute error at most `2e-12` in every process.
- For each bandlimit, the upper endpoint of the paired-ratio 95% bootstrap
  interval must be at most 1.02. At least one bandlimit must have median ratio
  at most 0.90 and interval upper endpoint below 1.00. Use 50,000 resamples,
  seed `20261017`, the median per resample, and linearly interpolated 2.5th
  and 97.5th percentiles.
- A passing q8 screen authorizes separate T511 scalar and spin-1 regression
  controls. Candidate acceptance still requires the unresolved T511 q1
  control, complete numerical and fallback checks, bounded memory, and the
  full regression matrix.

The q1 screen's surviving T511 outputs remain local diagnostics. Do not use
them to calculate q8 results or repair the earlier q1 pairing.
