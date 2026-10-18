#!/usr/bin/env python3
import math
import re
import statistics
import sys
from pathlib import Path

path = Path(sys.argv[1])
text = path.read_text()
rows = re.findall(
    r"^M6 fast-route sample L=(\d+) q=(\d+) sample=(\d+) elapsed_ns=(\d+) FastRouteCounts \{ ([^}]+) \}$",
    text,
    re.M,
)
errors = []
if "running 1 test" not in text:
    errors.append("test harness did not report exactly one running test")
if "test result: ok. 1 passed; 0 failed; 0 ignored;" not in text:
    errors.append("ignored route test did not pass")
if len(rows) != 5:
    errors.append(f"expected 5 elapsed_ns samples, got {len(rows)}")

samples = []
for index, (lmax, threads, sample_index, elapsed_ns, raw_counts) in enumerate(rows):
    if int(sample_index) != index:
        errors.append(f"expected sample index {index}, got {sample_index}")
    counts = {key: int(value) for key, value in re.findall(r"(\w+): (\d+)", raw_counts)}
    if int(threads) != 1:
        errors.append(f"expected q1, got q={threads}")
    samples.append((int(elapsed_ns), counts))

if len(samples) == 5:
    values = [elapsed for elapsed, _ in samples]
    median = statistics.median(values)
    cv = 100.0 * statistics.pstdev(values) / statistics.mean(values)
    first = statistics.median(values[:3])
    final = statistics.median(values[3:])
    drift = 100.0 * abs(first - final) / median
    if not all(math.isfinite(v) and v > 0 for v in values):
        errors.append("elapsed_ns samples must be positive finite integers")
    if cv > 5.0:
        errors.append(f"elapsed_ns CV exceeds 5%: {cv:.4f}%")
    if drift > 5.0:
        errors.append(f"first3/final2 drift exceeds 5%: {drift:.4f}%")
else:
    values = []
    median = cv = first = final = drift = float("nan")

out = path.with_suffix(".stability")
with out.open("w") as f:
    f.write(f"samples_elapsed_ns={','.join(str(v) for v in values)}\n")
    f.write(f"median_elapsed_ns={median:.3f}\n")
    f.write(f"cv_pct={cv:.4f}\n")
    f.write(f"first3_median_ns={first:.3f}\n")
    f.write(f"final2_median_ns={final:.3f}\n")
    f.write(f"drift_pct={drift:.4f}\n")
    f.write("route_timer_medians_ns:\n")
    if samples:
        for key in ("transition_elapsed_ns", "prefix_elapsed_ns", "suffix_elapsed_ns", "fallback_elapsed_ns", "tile_elapsed_ns"):
            med = statistics.median(counts.get(key, 0) for _, counts in samples)
            f.write(f"  {key}={med:.0f}\n")
    f.write("gate=" + ("FAIL" if errors else "PASS") + "\n")
    for error in errors:
        f.write("failure=" + error + "\n")
print(out.read_text(), end="")
sys.exit(1 if errors else 0)
