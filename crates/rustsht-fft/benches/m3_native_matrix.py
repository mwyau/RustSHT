#!/usr/bin/env python3
"""Collect the M3 C2C target, nested-route, and control matrix from a bench binary."""

import csv
import hashlib
import io
import subprocess
import sys
from pathlib import Path

if len(sys.argv) != 3:
    raise SystemExit("usage: m3_native_matrix.py BENCH_BINARY all")

binary = str(Path(sys.argv[1]).resolve())
kind = sys.argv[2]
if kind != "all":
    raise SystemExit("expected matrix kind 'all'")
lengths = [
    ("c2c", 150, [1, 32, 73]),
    ("c2c", 300, [1, 32, 73]),
    ("c2c", 726, [1, 32, 361]),
    ("c2c", 1452, [1, 32, 721]),
    ("c2c", 2904, [1, 32, 721]),
    ("c2c", 240, [1, 32]),
    ("c2c", 484, [1, 32]),
    ("c2c", 1372, [1, 32]),
    ("c2c", 256, [1, 32, 128]),
    ("c2c", 512, [1, 32, 256]),
    ("c2c", 1024, [1, 32, 512]),
    ("real", 1452, [1, 32, 721]),
    ("real", 1026, [1, 32, 512]),
]
digest = hashlib.sha256(Path(binary).read_bytes()).hexdigest()
header = None
rows = []
logs = []
for mode, n, shapes in lengths:
    command = [binary, "bench-native-c2c" if mode == "c2c" else "bench-native-real", str(n), *(str(row) for row in shapes)]
    result = subprocess.run(command, text=True, capture_output=True)
    logs.append(f"COMMAND {command!r} binary_sha256={digest} exit={result.returncode}\n{result.stderr}")
    if result.returncode:
        sys.stderr.write("\n".join(logs))
        raise SystemExit(result.returncode)
    parsed = csv.reader(io.StringIO(result.stdout))
    invocation_header = next(parsed)
    if header is None:
        header = invocation_header
    elif header != invocation_header:
        raise RuntimeError("benchmark CSV header changed")
    rows.extend(list(parsed))
print(",".join(header))
for row in rows:
    print(",".join(row))
sys.stderr.write("\n".join(logs))
