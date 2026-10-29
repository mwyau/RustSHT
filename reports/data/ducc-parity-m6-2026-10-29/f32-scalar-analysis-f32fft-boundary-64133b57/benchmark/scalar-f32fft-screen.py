#!/usr/bin/env python3
"""Interleaved f32-FFT/f64-recurrence scalar-analysis screen."""
import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "source"
FIXTURES = ROOT / "fixtures"
RESULTS = ROOT / "results"
RUST = ["cargo", "test", "--release", "--features", "parallel", "--lib",
        "transform::tests::m6_profile_scalar_analysis_f32_fft_f64_recurrence",
        "--", "--exact", "--nocapture"]
DUCC = ROOT / "ducc-build/ducc_f32_spin_analysis_profile"

def median(values):
    values = sorted(values)
    return values[len(values) // 2]

def run_rust(case, pair, route):
    tag = case["tag"]
    output = RESULTS / f"{tag}-p{pair}-{route}.f32c"
    env = os.environ.copy()
    env.update({
        "M6_SCALAR_NPHI": str(case["nphi"]),
        "M6_SCALAR_MMAX": str(case["mmax"]),
        "M6_SCALAR_THREADS": str(case["threads"]),
        "M6_SCALAR_ROUTE": route,
        "M6_SCALAR_RUNS": "7",
        "M6_SCALAR_MAP": str(FIXTURES / f"{case['fixture']}-map0.f32"),
        "M6_SCALAR_OUTPUT": str(output),
    })
    process = subprocess.run(RUST, cwd=SOURCE, env=env, text=True,
                             capture_output=True, check=True)
    line = next(line for line in process.stdout.splitlines()
                if '"route"' in line)
    line = line[line.index("{"):]
    line = re.sub(r'"peak_rss_kib":Some\((\d+)\)', r'"peak_rss_kib":\1', line)
    summary = json.loads(line)
    return {"summary": summary, "elapsed_ms": summary["median_ms"],
            "output": str(output), "finite": summary["finite"],
            "reset": summary["poison_reset"]}

def run_ducc(case, pair):
    tag = case["tag"]
    output = RESULTS / f"{tag}-p{pair}-ducc.f32c"
    fixture = case["fixture"]
    command = [str(DUCC), "--worker", "analysis", "0", "GL",
               str(case["threads"]), "721", str(case["nphi"]), "720",
               str(case["mmax"]), "0.23",
               str(FIXTURES / f"{fixture}-map0.f32"),
               str(FIXTURES / f"{fixture}-map1.f32"),
               str(FIXTURES / f"{fixture}-alm0.f32c"),
               str(FIXTURES / f"{fixture}-alm1.f32c"),
               str(output), str(RESULTS / f"{tag}-unused.f32c"), "25"]
    process = subprocess.run(command, cwd=ROOT, text=True,
                             capture_output=True, check=True)
    summary = json.loads(next(line for line in process.stdout.splitlines()
                              if line.startswith('{"backend"')))
    return {"summary": summary, "elapsed_ms": median(summary["batch_ms"]),
            "output": str(output), "finite": summary["finite_outputs"],
            "reset": summary["reset_consistent"]}

cases = [
    {"tag": "reduced_q1", "fixture": "reduced", "threads": 1,
     "nphi": 378, "mmax": 180},
    {"tag": "reduced_q8", "fixture": "reduced", "threads": 8,
     "nphi": 378, "mmax": 180},
    {"tag": "full_q1", "fixture": "full", "threads": 1,
     "nphi": 1458, "mmax": 720},
    {"tag": "full_q8", "fixture": "full", "threads": 8,
     "nphi": 1458, "mmax": 720},
]
orders = [
    ["candidate", "native", "ducc"],
    ["ducc", "candidate", "native"],
    ["native", "ducc", "candidate"],
]
rows = []
for case in cases:
    q = case["threads"]
    os.sched_setaffinity(0, set(range(q)))
    case_rows = []
    for pair, order in enumerate(orders, 1):
        current = {}
        for backend in order:
            if backend == "ducc":
                result = run_ducc(case, pair)
            else:
                route = "f32_fft_f64_recurrence" if backend == "candidate" else "native"
                result = run_rust(case, pair, route)
            row = {"case": case["tag"], "pair": pair, "backend": backend,
                   **result}
            current[backend] = row
            case_rows.append(row)
        rust_candidate = current["candidate"]["elapsed_ms"]
        ducc = current["ducc"]["elapsed_ms"]
        print(json.dumps({"case": case["tag"], "pair": pair,
                          "ratio_candidate_ducc": rust_candidate / ducc,
                          "ratio_native_ducc": current["native"]["elapsed_ms"] / ducc,
                          "ordered_backends": order}, sort_keys=True), flush=True)
    rows.extend(case_rows)

RESULTS.mkdir(exist_ok=True)
(RESULTS / "scalar-f32fft-screen.json").write_text(json.dumps(rows, indent=2) + "\n")
