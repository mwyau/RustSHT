#!/usr/bin/env python3
"""Short protected f64 T1023 spin-1 synthesis control for two Rust workers."""
from __future__ import annotations

import hashlib
import json
import os
import pathlib
import subprocess
import sys
import types

import numpy as np

source = pathlib.Path(os.environ["RUSTSHT_SOURCE_ROOT"]).resolve()
sys.path.insert(0, str(source / "scripts"))
# This benchmark helper is used only for fixture generation and worker args.
# Stub the shared benchmark module so this script also runs on Python 3.11.
sys.modules["benchmark_ducc_full_transform_parity"] = types.ModuleType(
    "benchmark_ducc_full_transform_parity"
)
import benchmark_ducc_spin1_native as bench  # noqa: E402

parent = pathlib.Path(os.environ["RUSTSHT_PARENT_BIN"]).resolve()
candidate = pathlib.Path(os.environ["RUSTSHT_CANDIDATE_BIN"]).resolve()
out_path = pathlib.Path(os.environ["RUSTSHT_RESULT_JSON"]).resolve()
scratch = pathlib.Path(os.environ["RUSTSHT_EVIDENCE_DIR"]).resolve()
scratch.mkdir(parents=True, exist_ok=True)

case = next(
    item
    for item in bench.make_cases([1023], ["GL"], False)
    if item["mmax"] == 1023
    and item["ntheta"] == 1024
    and item["longitude_mode"] == "even-smooth"
)
fixture = bench.make_fixture(case, scratch)
records = []

for threads, cpus in ((1, [2]), (8, list(range(8)))):
    for pair in range(3):
        order = ["parent", "candidate"] if pair % 2 == 0 else ["candidate", "parent"]
        runs = {}
        for name in order:
            binary = parent if name == "parent" else candidate
            outputs = {
                key: scratch / f"{name}-q{threads}-p{pair}-{key}.bin"
                for key in ("map0", "map1", "e", "b")
            }
            command = bench.worker_command(
                binary, case, fixture, "synthesis", threads, outputs, 25.0
            )
            completed = subprocess.run(
                ["taskset", "-c", ",".join(map(str, cpus)), *command],
                check=True,
                capture_output=True,
                text=True,
            )
            runs[name] = {"result": json.loads(completed.stdout), "outputs": outputs}

        parent_values = np.concatenate(
            [bench.output_values(runs["parent"]["outputs"][key], "real") for key in ("map0", "map1")]
        )
        candidate_values = np.concatenate(
            [bench.output_values(runs["candidate"]["outputs"][key], "real") for key in ("map0", "map1")]
        )
        parent_ms = float(np.median(runs["parent"]["result"]["batch_ms"]))
        candidate_ms = float(np.median(runs["candidate"]["result"]["batch_ms"]))
        records.append(
            {
                "threads": threads,
                "pair": pair,
                "order": order,
                "parent_ms": parent_ms,
                "candidate_ms": candidate_ms,
                "candidate_parent_ratio": candidate_ms / parent_ms,
                "outputs_exact": bool(np.array_equal(parent_values, candidate_values)),
                "checks": {
                    name: {
                        "repeat": runs[name]["result"]["repeat_consistent"],
                        "reset": runs[name]["result"]["reset_consistent"],
                    }
                    for name in order
                },
            }
        )
        print(
            f"q{threads} pair {pair + 1}: {parent_ms:.3f}/{candidate_ms:.3f} ms; "
            f"ratio={candidate_ms / parent_ms:.4f}; exact={records[-1]['outputs_exact']}",
            flush=True,
        )

summary = {}
for threads in (1, 8):
    group = [record for record in records if record["threads"] == threads]
    summary[f"q{threads}"] = {
        "parent_median_ms": float(np.median([record["parent_ms"] for record in group])),
        "candidate_median_ms": float(np.median([record["candidate_ms"] for record in group])),
        "candidate_parent_paired_ratio_median": float(
            np.median([record["candidate_parent_ratio"] for record in group])
        ),
    }

payload = {
    "metadata": {
        "case": case,
        "fixture_sha256": fixture["sha256"],
        "parent_binary_sha256": hashlib.sha256(parent.read_bytes()).hexdigest(),
        "candidate_binary_sha256": hashlib.sha256(candidate.read_bytes()).hexdigest(),
        "pairs_per_q": 3,
        "target_ms": 25.0,
        "affinity": {"q1": [2], "q8": list(range(8))},
    },
    "summary": summary,
    "records": records,
}
out_path.write_text(json.dumps(payload, indent=2) + "\n")
