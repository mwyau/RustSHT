#!/usr/bin/env python3
import re
from pathlib import Path
from statistics import median

root = Path(__file__).resolve().parent
rows = []
for q in (1, 8):
    for pair in range(1, 6):
        native_path = root / f"pair-q{q}-{pair}-native.log"
        native_text = native_path.read_text()
        native = {}
        for line in native_text.splitlines():
            if line.startswith("analysis_rayon_ms="):
                key, value = line.split("=", 1)
                native[key] = float(value)
        native_rss = int(re.search(r"max_rss_kb=(\d+)", (root / f"pair-q{q}-{pair}-native.time").read_text()).group(1))

        ducc_path = root / f"pair-q{q}-{pair}-ducc.log"
        ducc_text = ducc_path.read_text()
        ducc_line = next(line for line in ducc_text.splitlines() if line.startswith("analysis_ducc_run_medians_ms="))
        ducc_value = float(re.search(r"median=([0-9.]+)", ducc_line).group(1))
        ducc_rss = int(re.search(r"max_rss_kb=(\d+)", (root / f"pair-q{q}-{pair}-ducc.time").read_text()).group(1))
        rust_value = native["analysis_rayon_ms"]
        rows.append((pair, q, rust_value, ducc_value, rust_value / ducc_value, (rust_value / ducc_value - 1.0) * 100.0, native_rss, ducc_rss))

out = root / "paired-results.tsv"
with out.open("w") as stream:
    stream.write("pair\tthreads\trust_ms\tducc_ms\trust_ducc_ratio\texcess_pct\trust_rss_kb\tducc_rss_kb\n")
    for row in rows:
        stream.write("\t".join([str(row[0]), str(row[1]), *(f"{value:.3f}" for value in row[2:6]), str(row[6]), str(row[7])]) + "\n")

for q in (1, 8):
    selected = [row for row in rows if row[1] == q]
    ratios = [row[4] for row in selected]
    changes = [row[5] for row in selected]
    rust = [row[2] for row in selected]
    ducc = [row[3] for row in selected]
    print(f"q{q}: RustSHT median={median(rust):.3f} ms range={min(rust):.3f}..{max(rust):.3f}; DUCC median={median(ducc):.3f} ms range={min(ducc):.3f}..{max(ducc):.3f}; paired ratio median={median(ratios):.2f}x range={min(ratios):.2f}..{max(ratios):.2f}x; excess={median(changes):+.1f}%")
