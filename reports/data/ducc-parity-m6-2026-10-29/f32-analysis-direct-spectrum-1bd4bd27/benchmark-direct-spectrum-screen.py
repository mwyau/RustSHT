#!/usr/bin/env python3
import hashlib
import json
import os
import pathlib
import random
import re
import statistics
import struct
import subprocess


PARENT = pathlib.Path(os.environ["M6_PARENT_SOURCE"])
PARENT_TARGET = pathlib.Path(os.environ["M6_PARENT_TARGET"])
CANDIDATE = pathlib.Path(os.environ["M6_CANDIDATE_SOURCE"])
CANDIDATE_TARGET = pathlib.Path(os.environ["M6_CANDIDATE_TARGET"])
DUCC = pathlib.Path(os.environ["M6_DUCC_BINARY"])
FIXTURES = pathlib.Path(os.environ["M6_FIXTURES"])
OUTPUT = pathlib.Path(os.environ["M6_OUTPUT_ROOT"])
PAIRS = int(os.environ.get("M6_PAIRS", "3"))
START_PAIR = int(os.environ.get("M6_START_PAIR", "1"))
CASE_FILTER = set(filter(None, os.environ.get("M6_CASE_FILTER", "").split(",")))
OUTPUT.mkdir(parents=True, exist_ok=True)

CASES = [
    {"tag": "reduced_q1", "fixture": "reduced", "threads": 1, "nphi": 378, "mmax": 180},
    {"tag": "reduced_q8", "fixture": "reduced", "threads": 8, "nphi": 378, "mmax": 180},
    {"tag": "full_q1", "fixture": "full", "threads": 1, "nphi": 1458, "mmax": 720},
    {"tag": "full_q8", "fixture": "full", "threads": 8, "nphi": 1458, "mmax": 720},
]
if CASE_FILTER:
    CASES = [case for case in CASES if case["tag"] in CASE_FILTER]
previous_path = OUTPUT / "paired-results.json"
previous_rows = json.loads(previous_path.read_text()) if START_PAIR > 1 and previous_path.exists() else []
selected_tags = {case["tag"] for case in CASES}
rows = [row for row in previous_rows if row["case"]["tag"] not in selected_tags]


def rust(case, pair, backend):
    source, target, route = (
        (PARENT, PARENT_TARGET, "f32_fft_f64_recurrence")
        if backend == "parent"
        else (CANDIDATE, CANDIDATE_TARGET, "f32_fft_f64_recurrence_direct_spectra")
    )
    out = OUTPUT / case["tag"] / f"pair-{pair:02d}-{backend}.f32c"
    out.parent.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.update(
        {
            "CARGO_TARGET_DIR": str(target),
            "M6_SCALAR_NPHI": str(case["nphi"]),
            "M6_SCALAR_MMAX": str(case["mmax"]),
            "M6_SCALAR_THREADS": str(case["threads"]),
            "M6_SCALAR_ROUTE": route,
            "M6_SCALAR_RUNS": "7",
            "M6_SCALAR_MAP": str(FIXTURES / f"{case['fixture']}-map0.f32"),
            "M6_SCALAR_OUTPUT": str(out),
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
    proc = subprocess.run(command, cwd=source, env=env, text=True, capture_output=True, check=True)
    line = next((line for line in proc.stdout.splitlines() if '"route"' in line), None)
    if line is None:
        raise RuntimeError(f"test worker emitted no route summary:\n{proc.stdout}\n{proc.stderr}")
    line = line[line.index("{"):]
    line = re.sub(r'"peak_rss_kib":Some\((\d+)\)', r'"peak_rss_kib":\1', line)
    summary = json.loads(line)
    return {
        "backend": backend,
        "elapsed_ms": summary["median_ms"],
        "output": str(out),
        "sha256": hashlib.sha256(out.read_bytes()).hexdigest(),
        "finite": summary["finite"],
        "reset": summary["poison_reset"],
        "samples_ms": summary["samples_ms"],
    }


def ducc(case, pair):
    fixture = case["fixture"]
    out0 = OUTPUT / case["tag"] / f"pair-{pair:02d}-ducc-0.f32c"
    out1 = OUTPUT / case["tag"] / f"pair-{pair:02d}-ducc-1.f32c"
    out0.parent.mkdir(parents=True, exist_ok=True)
    command = [
        str(DUCC),
        "--worker",
        "analysis",
        "0",
        "GL",
        str(case["threads"]),
        "721",
        str(case["nphi"]),
        "720",
        str(case["mmax"]),
        "0.23",
        str(FIXTURES / f"{fixture}-map0.f32"),
        str(FIXTURES / f"{fixture}-map1.f32"),
        str(FIXTURES / f"{fixture}-alm0.f32c"),
        str(FIXTURES / f"{fixture}-alm1.f32c"),
        str(out0),
        str(out1),
        "25",
    ]
    proc = subprocess.run(command, text=True, capture_output=True, check=True)
    summary = json.loads(proc.stdout.strip().splitlines()[-1])
    return {
        "backend": "ducc",
        "elapsed_ms": statistics.median(summary["batch_ms"]),
        "output": str(out0),
        "sha256": hashlib.sha256(out0.read_bytes()).hexdigest(),
        "finite": summary["finite_outputs"],
        "reset": summary["reset_consistent"],
        "samples_ms": summary["batch_ms"],
    }


def values(path):
    raw = pathlib.Path(path).read_bytes()
    return struct.unpack(f"<{len(raw) // 4}f", raw)


def error(reference, candidate):
    ref = values(reference)
    got = values(candidate)
    assert len(ref) == len(got)
    diff = [a - b for a, b in zip(got, ref)]
    return {
        "relative_l2": (sum(x * x for x in diff) / sum(x * x for x in ref)) ** 0.5,
        "max_absolute": max(map(abs, diff), default=0.0),
    }


orders = [
    ["parent", "candidate", "ducc"],
    ["ducc", "parent", "candidate"],
    ["candidate", "ducc", "parent"],
]
for case_index, case in enumerate(CASES):
    pair_rows = [
        row for row in previous_rows
        if row["case"]["tag"] == case["tag"] and row["pair"] < START_PAIR
    ]
    rows.extend(pair_rows)
    for pair in range(START_PAIR, PAIRS + 1):
        order = orders[(pair - 1) % len(orders)]
        os.sched_setaffinity(0, set(range(case["threads"])))
        results = {}
        for backend in order:
            results[backend] = ducc(case, pair) if backend == "ducc" else rust(case, pair, backend)
        row = {
            "case": case,
            "pair": pair,
            "order": order,
            "times_ms": {name: results[name]["elapsed_ms"] for name in results},
            "candidate_parent_ratio": results["candidate"]["elapsed_ms"] / results["parent"]["elapsed_ms"],
            "candidate_ducc_ratio": results["candidate"]["elapsed_ms"] / results["ducc"]["elapsed_ms"],
            "outputs": {name: {"sha256": results[name]["sha256"], "finite": results[name]["finite"], "reset": results[name]["reset"]} for name in results},
            "candidate_vs_parent": error(results["parent"]["output"], results["candidate"]["output"]),
            "candidate_vs_ducc": error(results["ducc"]["output"], results["candidate"]["output"]),
        }
        for result in results.values():
            pathlib.Path(result["output"]).unlink(missing_ok=True)
        pair_rows.append(row)
        rows.append(row)
        print(json.dumps(row, sort_keys=True), flush=True)
    summary = {
        "case": case,
        "pairs": pair_rows,
        "candidate_parent_paired_median": statistics.median(row["candidate_parent_ratio"] for row in pair_rows),
        "candidate_ducc_paired_median": statistics.median(row["candidate_ducc_ratio"] for row in pair_rows),
        "candidate_parent_faster": sum(row["candidate_parent_ratio"] < 1.0 for row in pair_rows),
    }
    (OUTPUT / f"{case['tag']}-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({key: summary[key] for key in ("case", "pairs", "candidate_parent_paired_median", "candidate_ducc_paired_median", "candidate_parent_faster") if key != "pairs"}, sort_keys=True), flush=True)
(OUTPUT / "paired-results.json").write_text(json.dumps(rows, indent=2) + "\n")
