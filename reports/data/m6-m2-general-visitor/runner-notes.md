# Benchmark runner notes

The first run-screen.sh attempt stopped in preflight because taskset -pc
was given a CPU list instead of a PID. It launched no benchmark process. The
probe now runs taskset -c "${RUSTSHT_CPUSET:?set RUSTSHT_CPUSET}" sh -c 'taskset -pc $$'; it confirmed affinity to
one CPU core before the accepted screen. The 36 accepted benchmark processes completed with /usr/bin/time exit status
0. The first summarizer revision expected phi0 in the first output line,
but m1b_bench records phi0=0.37 as a source constant and omits it from the
timing header. The summarizer was corrected to validate geometry, mode,
operation and requested-only configuration; the source constant is recorded
in the summary. No samples were rerun to repair either tooling issue.
