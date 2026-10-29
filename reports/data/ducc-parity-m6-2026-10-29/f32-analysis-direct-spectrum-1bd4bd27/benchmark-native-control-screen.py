#!/usr/bin/env python3
import hashlib
import json
import os
import pathlib
import random
import re
import statistics
import subprocess


PARENT = pathlib.Path(os.environ["M6_PARENT_SOURCE"])
PARENT_TARGET = pathlib.Path(os.environ["M6_PARENT_TARGET"])
CANDIDATE = pathlib.Path(os.environ["M6_CANDIDATE_SOURCE"])
CANDIDATE_TARGET = pathlib.Path(os.environ["M6_CANDIDATE_TARGET"])
FIXTURES = pathlib.Path(os.environ["M6_FIXTURES"])
OUTPUT = pathlib.Path(os.environ["M6_OUTPUT_ROOT"])
OUTPUT.mkdir(parents=True, exist_ok=True)
PAIRS = int(os.environ.get("M6_PAIRS", "3"))
START_PAIR = int(os.environ.get("M6_START_PAIR", "1"))
CASE_FILTER = set(filter(None, os.environ.get("M6_CASE_FILTER", "").split(",")))
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


def run(case, pair, backend):
    source, target = (PARENT, PARENT_TARGET) if backend == "parent" else (CANDIDATE, CANDIDATE_TARGET)
    output = OUTPUT / case["tag"] / f"pair-{pair:02d}-{backend}.f32c"
    output.parent.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.update(
        {
            "CARGO_TARGET_DIR": str(target),
            "M6_SCALAR_NPHI": str(case["nphi"]),
            "M6_SCALAR_MMAX": str(case["mmax"]),
            "M6_SCALAR_THREADS": str(case["threads"]),
            "M6_SCALAR_ROUTE": "native",
            "M6_SCALAR_RUNS": "7",
            "M6_SCALAR_MAP": str(FIXTURES / f"{case['fixture']}-map0.f32"),
            "M6_SCALAR_OUTPUT": str(output),
        }
    )
    command = [
        "cargo", "test", "--release", "--features", "parallel", "--lib",
        "transform::tests::m6_profile_scalar_analysis_f32_fft_f64_recurrence",
        "--", "--exact", "--nocapture",
    ]
    proc = subprocess.run(command, cwd=source, env=env, text=True, capture_output=True, check=True)
    line = next(line for line in proc.stdout.splitlines() if '"route"' in line)
    line = line[line.index("{"):]
    line = re.sub(r'"peak_rss_kib":Some\((\d+)\)', r'"peak_rss_kib":\1', line)
    summary = json.loads(line)
    result = {
        "elapsed_ms": summary["median_ms"],
        "samples_ms": summary["samples_ms"],
        "finite": summary["finite"],
        "reset": summary["poison_reset"],
        "sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
    }
    output.unlink(missing_ok=True)
    return result


rng = random.Random(0xF32C0A7 + START_PAIR)
for case in CASES:
    case_rows = [
        row for row in previous_rows
        if row["case"]["tag"] == case["tag"] and row["pair"] < START_PAIR
    ]
    rows.extend(case_rows)
    for pair in range(START_PAIR, PAIRS + 1):
        order = ["parent", "candidate"] if rng.randrange(2) == 0 else ["candidate", "parent"]
        os.sched_setaffinity(0, set(range(case["threads"])))
        results = {backend: run(case, pair, backend) for backend in order}
        row = {
            "case": case,
            "pair": pair,
            "order": order,
            "parent_ms": results["parent"]["elapsed_ms"],
            "candidate_ms": results["candidate"]["elapsed_ms"],
            "candidate_parent_ratio": results["candidate"]["elapsed_ms"] / results["parent"]["elapsed_ms"],
            "parent": {k: results["parent"][k] for k in ("finite", "reset", "sha256")},
            "candidate": {k: results["candidate"][k] for k in ("finite", "reset", "sha256")},
        }
        rows.append(row)
        print(json.dumps(row, sort_keys=True), flush=True)
    (OUTPUT / f"{case['tag']}-summary.json").write_text(
        json.dumps(
            {
                "case": case,
                "pairs": [r for r in rows if r["case"]["tag"] == case["tag"]],
                "candidate_parent_paired_median": statistics.median(
                    r["candidate_parent_ratio"] for r in rows if r["case"]["tag"] == case["tag"]
                ),
            },
            indent=2,
        ) + "\n"
    )
(OUTPUT / "paired-results.json").write_text(json.dumps(rows, indent=2) + "\n")
