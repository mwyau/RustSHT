#!/usr/bin/env python3
import re
from pathlib import Path
from statistics import median

root = Path(__file__).resolve().parent
rows = []
for threads in (1, 8):
    for pair in range(1, 6):
        stem = f"spin-0-t1023-q{threads}-{pair}"
        native = (root / f"{stem}-native.log").read_text()
        ducc = (root / f"{stem}-ducc.log").read_text()
        rust_ms = float(re.search(r"^analysis_rayon_ms=([0-9.]+)", native, re.M).group(1))
        ducc_line = next(line for line in ducc.splitlines() if line.startswith("analysis_ducc_run_medians_ms="))
        ducc_ms = float(re.search(r"median=([0-9.]+)", ducc_line).group(1))
        ratio = rust_ms / ducc_ms
        rows.append((threads, pair, rust_ms, ducc_ms, ratio))

with (root / "paired-results.tsv").open("w") as stream:
    stream.write("threads\tpair\trust_ms\tducc_ms\trust_ducc_ratio\n")
    for row in rows:
        stream.write("\t".join([str(row[0]), str(row[1]), *(f"{value:.3f}" for value in row[2:])]) + "\n")

for threads in (1, 8):
    selected = [row for row in rows if row[0] == threads]
    rust = [row[2] for row in selected]
    ducc = [row[3] for row in selected]
    ratios = [row[4] for row in selected]
    print(
        f"q{threads}: Rust median={median(rust):.3f} ms range={min(rust):.3f}..{max(rust):.3f}; "
        f"DUCC median={median(ducc):.3f} ms range={min(ducc):.3f}..{max(ducc):.3f}; "
        f"paired ratio median={median(ratios):.2f}x range={min(ratios):.2f}..{max(ratios):.2f}x"
    )
