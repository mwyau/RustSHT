#!/usr/bin/env python3
import re
from pathlib import Path
from statistics import median

root = Path(__file__).resolve().parent
rows = []
for spin in (0, 1):
    for threads in (1, 8):
        for pair in range(1, 6):
            stem = f"spin-{spin}-t1023-q{threads}-{pair}"
            native = (root / f"{stem}-native.log").read_text()
            ducc = (root / f"{stem}-ducc.log").read_text()
            rust_ms = float(re.search(r"^analysis_rayon_ms=([0-9.]+)", native, re.M).group(1))
            ducc_line = next(line for line in ducc.splitlines() if line.startswith("analysis_ducc_run_medians_ms="))
            ducc_ms = float(re.search(r"median=([0-9.]+)", ducc_line).group(1))
            rust_rss = int(re.search(r"max_rss_kb=(\d+)", (root / f"{stem}-native.time").read_text()).group(1))
            ducc_rss = int(re.search(r"max_rss_kb=(\d+)", (root / f"{stem}-ducc.time").read_text()).group(1))
            rows.append((spin, threads, pair, rust_ms, ducc_ms, rust_ms / ducc_ms, rust_rss, ducc_rss))

with (root / "paired-results.tsv").open("w") as stream:
    stream.write("spin\tthreads\tpair\trust_ms\tducc_ms\trust_ducc_ratio\trust_rss_kb\tducc_rss_kb\n")
    for row in rows:
        stream.write("\t".join([str(row[0]), str(row[1]), str(row[2]), *(f"{value:.3f}" for value in row[3:6]), str(row[6]), str(row[7])]) + "\n")

for spin in (0, 1):
    for threads in (1, 8):
        selected = [row for row in rows if row[0] == spin and row[1] == threads]
        rust = [row[3] for row in selected]
        ducc = [row[4] for row in selected]
        ratios = [row[5] for row in selected]
        print(
            f"spin {spin} q{threads}: Rust median={median(rust):.3f} ms "
            f"range={min(rust):.3f}..{max(rust):.3f}; DUCC median={median(ducc):.3f} ms "
            f"range={min(ducc):.3f}..{max(ducc):.3f}; paired ratio median={median(ratios):.2f}x "
            f"range={min(ratios):.2f}..{max(ratios):.2f}x"
        )
