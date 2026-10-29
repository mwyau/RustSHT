#!/usr/bin/env python3
"""Run the isolated v3 direct-spectrum capacity/allocation diagnostics."""
import json
import os
import pathlib
import re
import subprocess

SOURCE = pathlib.Path(os.environ["M6_SOURCE"])
TARGET = pathlib.Path(os.environ["M6_TARGET"])
FIXTURES = pathlib.Path(os.environ["M6_FIXTURES"])
OUTPUT = pathlib.Path(os.environ["M6_OUTPUT"])
OUTPUT.mkdir(parents=True, exist_ok=True)
os.environ["RUSTFLAGS"] = "-C target-cpu=x86-64-v3 -Aunexpected_cfgs"
CASES = [
    ("reduced_q1", "reduced", 1, 378, 180),
    ("reduced_q8", "reduced", 8, 378, 180),
    ("full_q1", "full", 1, 1458, 720),
    ("full_q8", "full", 8, 1458, 720),
]
rows = []
for tag, fixture, threads, nphi, mmax in CASES:
    os.sched_setaffinity(0, set(range(threads)))
    env = os.environ.copy()
    env.update(
        {
            "CARGO_TARGET_DIR": str(TARGET),
            "M6_SCALAR_NPHI": str(nphi),
            "M6_SCALAR_MMAX": str(mmax),
            "M6_SCALAR_THREADS": str(threads),
            "M6_SCALAR_ROUTE": "f32_fft_f64_recurrence_direct_spectra_profile",
            "M6_SCALAR_RUNS": "30",
            "M6_SCALAR_COUNT_ALLOC": "1",
            "M6_SCALAR_MAP": str(FIXTURES / f"{fixture}-map0.f32"),
            "M6_SCALAR_OUTPUT": str(OUTPUT / f"{tag}.f32c"),
        }
    )
    command = [
        "cargo",
        "test",
        "--release",
        "--features",
        "parallel",
        "--lib",
        "transform::tests::m6_profile_scalar_analysis_f32_fft_f64_recurrence",
        "--",
        "--exact",
        "--nocapture",
    ]
    proc = subprocess.run(command, cwd=SOURCE, env=env, text=True, capture_output=True, check=True)
    line = next(line for line in proc.stdout.splitlines() if '"route"' in line)
    line = line[line.index("{") :]
    line = re.sub(r'"peak_rss_kib":Some\((\d+)\)', r'"peak_rss_kib":\1', line)
    row = {"case": tag, **json.loads(line)}
    rows.append(row)
    print(json.dumps(row), flush=True)
(OUTPUT / "workspace-alloc-diagnostics.json").write_text(json.dumps(rows, indent=2) + "\n")
