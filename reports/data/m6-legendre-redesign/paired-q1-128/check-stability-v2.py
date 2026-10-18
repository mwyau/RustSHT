#!/usr/bin/env python3
import math
import re
import statistics
import sys
from pathlib import Path

path = Path(sys.argv[1])
text = path.read_text()
failures = []
rows = []
for name in ("synthesis_rust_plan", "synthesis_ducc", "synthesis_shtns_adapter"):
    match = re.search(rf"^{name}_raw_samples_ms=([^\n]+)", text, re.M)
    if not match:
        failures.append(f"missing raw samples: {name}")
        continue
    values = [float(v) for v in match.group(1).split(";", 1)[0].split(",")]
    if len(values) != 5 or not all(math.isfinite(v) and v > 0 for v in values):
        failures.append(f"expected five positive finite samples for {name}, got {values}")
        continue
    median = statistics.median(values)
    mean = statistics.mean(values)
    cv = 100.0 * statistics.pstdev(values) / mean
    first = statistics.median(values[:3])
    final = statistics.median(values[3:])
    drift = 100.0 * abs(first - final) / median
    passed = cv <= 5.0 and drift <= 5.0
    rows.append((name, values, median, cv, first, final, drift, passed))
    if not passed:
        failures.append(f"{name} failed stability: CV={cv:.3f}% drift={drift:.3f}%")

out = path.with_suffix(".v2-stability")
with out.open("w") as f:
    for name, values, median, cv, first, final, drift, passed in rows:
        f.write(
            f"{name}: samples_ms={','.join(f'{v:.3f}' for v in values)} "
            f"median_ms={median:.3f} cv_pct={cv:.4f} "
            f"first3_median_ms={first:.3f} final2_median_ms={final:.3f} "
            f"drift_pct={drift:.4f} status={'PASS' if passed else 'FAIL'}\n"
        )
    f.write("overall=" + ("FAIL" if failures else "PASS") + "\n")
    for failure in failures:
        f.write("failure=" + failure + "\n")
print(out.read_text(), end="")
sys.exit(1 if failures else 0)
