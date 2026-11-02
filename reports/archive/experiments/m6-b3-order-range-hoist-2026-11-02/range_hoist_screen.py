#!/usr/bin/env python3
"""Three-pair GL T720 B3 parent/candidate/DUCC screen using the existing worker."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile

import numpy as np


ROOT = next(parent for parent in Path(__file__).resolve().parents if (parent / "Cargo.toml").is_file())
sys.path.insert(0, str(ROOT / "scripts"))
import benchmark_ducc_full_transform_parity as common

FIXTURES = Path(os.environ["M6_B3_FIXTURES"])
DUCC_INPUTS = Path(os.environ["M6_B3_DUCC_INPUTS"])
BINARIES = {
    name: Path(os.environ[f"M6_B3_{name.upper()}_BIN"])
    for name in ("parent", "candidate", "ducc")
}
Q8_CPUS = tuple(common.physical_cpus()[:8])
Q1_CPUS = Q8_CPUS[:1]
CONTROLLER_CPU = common.physical_cpus()[8]
TARGET_MS = 25.0
PAIR_COUNT = int(os.environ.get("M6_B3_PAIRS", "3"))
RESULT_NAME = os.environ.get("M6_B3_RESULTS", "range-hoist-screen.json")
RESULT_PATH = Path(os.environ.get("M6_B3_RESULTS_PATH", RESULT_NAME))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def command(backend: str, mmax: int, threads: int, pair: int, outdir: Path) -> list[str]:
    reduced = mmax == 180
    geometry = "fixture-180" if reduced else "fixture-720"
    zero_alms = DUCC_INPUTS / ("reduced-zero-alms.bin" if reduced else "full-zero-alms.bin")
    nphi = 378 if reduced else 1458
    base = outdir / f"{backend}-m{mmax}-q{threads}-p{pair}"
    output_map0 = base.with_suffix(".map0")
    output_map1 = base.with_suffix(".map1")
    output_e = base.with_suffix(".e")
    output_b = base.with_suffix(".b")
    return [
        str(BINARIES[backend]), "--worker", "analysis", "GL", str(threads), "721",
        str(nphi), "720", str(mmax), "0.23",
        str(FIXTURES / geometry / "map0.bin"),
        str(FIXTURES / geometry / "map1.bin"), str(zero_alms), str(zero_alms),
        str(output_map0), str(output_map1), str(output_e), str(output_b),
        f"{TARGET_MS:g}",
    ]


def run(backend: str, mmax: int, threads: int, pair: int, outdir: Path) -> dict:
    cpus = Q1_CPUS if threads == 1 else Q8_CPUS
    parsed, diagnostics = common.run_pinned(
        command(backend, mmax, threads, pair, outdir), list(cpus), CONTROLLER_CPU
    )
    if not parsed.get("repeat_consistent") or not parsed.get("reset_consistent"):
        raise RuntimeError(f"worker repeat/reset check failed: {backend}: {parsed}")
    parsed.update({"backend": backend, "pair": pair, "mmax": mmax, "threads": threads})
    parsed["host_diagnostics"] = diagnostics
    return parsed


def summarize_outputs(outdir: Path, mmax: int, threads: int, pair: int) -> dict:
    stem = f"-m{mmax}-q{threads}-p{pair}"
    parent = np.concatenate([
        np.fromfile(outdir / f"parent{stem}.e", dtype="<c16"),
        np.fromfile(outdir / f"parent{stem}.b", dtype="<c16"),
    ])
    candidate = np.concatenate([
        np.fromfile(outdir / f"candidate{stem}.e", dtype="<c16"),
        np.fromfile(outdir / f"candidate{stem}.b", dtype="<c16"),
    ])
    delta = candidate - parent
    result = {
        "parent_candidate_bitwise_equal": bool(np.array_equal(candidate, parent)),
        "parent_candidate_relative_l2": float(np.linalg.norm(delta) / np.linalg.norm(parent)),
        "parent_candidate_max_abs": float(np.max(np.abs(delta))),
    }
    for backend in ("parent", "candidate", "ducc"):
        for suffix in (".map0", ".map1", ".e", ".b"):
            (outdir / f"{backend}{stem}{suffix}").unlink(missing_ok=True)
    return result


def main() -> None:
    for path in BINARIES.values():
        if not path.is_file():
            raise FileNotFoundError(path)
    allowed_cpus = sorted(os.sched_getaffinity(0))
    profile = {
        "rust_source_parent": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip(),
        "ducc_source": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "cpu": common.cpu_model(),
        "rust_toolchain": common.command_output(["rustc", "-Vv"], cwd=ROOT),
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs; release; parallel",
        "ducc_flags": "-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math",
        "q1_cpus": list(Q1_CPUS),
        "q8_cpus": list(Q8_CPUS),
        "monitor_cpu": CONTROLLER_CPU,
        "allowed_cpus": allowed_cpus,
        "q8_topology": [common.topology(cpu) for cpu in Q8_CPUS],
        "target_batch_ms": TARGET_MS,
        "pair_count": PAIR_COUNT,
        "binaries": {name: {"path": str(path), "sha256": sha256(path)} for name, path in BINARIES.items()},
        "candidate_patch_sha256": sha256(ROOT / "scheduler-probe/range-hoist-v2.patch"),
        "fixtures": {},
        "cells": [],
    }
    for mmax in (720, 180):
        geometry = "fixture-720" if mmax == 720 else "fixture-180"
        profile["fixtures"][str(mmax)] = {
            name: sha256(FIXTURES / geometry / f"{name}.bin") for name in ("map0", "map1")
        }
        for threads in (1, 8):
            runs = []
            orders = (
                ("parent", "candidate", "ducc"),
                ("ducc", "candidate", "parent"),
                ("candidate", "parent", "ducc"),
            )
            with tempfile.TemporaryDirectory(prefix="m6-b3-range-hoist-") as tmp:
                outdir = Path(tmp)
                for pair in range(PAIR_COUNT):
                    order = orders[pair % len(orders)]
                    pair_runs = {}
                    for backend in order:
                        record = run(backend, mmax, threads, pair, outdir)
                        pair_runs[backend] = record
                        runs.append(record)
                    output_check = summarize_outputs(outdir, mmax, threads, pair)
                    ratios = {
                        "candidate_over_parent": statistics.median(pair_runs["candidate"]["batch_ms"])
                        / statistics.median(pair_runs["parent"]["batch_ms"]),
                        "candidate_over_ducc": statistics.median(pair_runs["candidate"]["batch_ms"])
                        / statistics.median(pair_runs["ducc"]["batch_ms"]),
                        "parent_over_ducc": statistics.median(pair_runs["parent"]["batch_ms"])
                        / statistics.median(pair_runs["ducc"]["batch_ms"]),
                    }
                    profile["cells"].append({
                        "mmax": mmax,
                        "threads": threads,
                        "pair": pair,
                        "order": list(order),
                        "ratios": ratios,
                        "outputs": output_check,
                        "runs": pair_runs,
                    })
                    print(
                        f"M{mmax} q{threads} pair{pair}: C/P={ratios['candidate_over_parent']:.4f} "
                        f"C/D={ratios['candidate_over_ducc']:.4f} P/D={ratios['parent_over_ducc']:.4f} "
                        f"output_bitwise={output_check['parent_candidate_bitwise_equal']}",
                        flush=True,
                    )
                    RESULT_PATH.write_text(json.dumps(profile, indent=2) + "\n")
    print(f"saved {RESULT_PATH}")


if __name__ == "__main__":
    main()
