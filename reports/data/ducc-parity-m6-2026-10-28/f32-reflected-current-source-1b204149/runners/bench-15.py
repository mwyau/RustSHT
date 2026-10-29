#!/usr/bin/env python3
"""Fifteen interleaved parent/candidate/DUCC comparisons in all T720 cells."""

import hashlib
import importlib.util
import json
import os
import pathlib
import random
import statistics
import subprocess
import sys

ROOT = pathlib.Path(os.environ["M6_EXPERIMENT_ROOT"])
REPO = pathlib.Path(os.environ["M6_REPO_ROOT"])
PARENT = ROOT / "target-parent/release/examples/ducc_f32_full_transform_parity"
CANDIDATE = ROOT / "target-candidate/release/examples/ducc_f32_full_transform_parity"
DUCC = ROOT / "ducc-build/ducc_f32_full_transform_parity"
OUT = ROOT / "results-15-v2"
SCRATCH = ROOT / "runs-15-v2"
sys.path.insert(0, str(REPO / "scripts"))
spec = importlib.util.spec_from_file_location("bench", REPO / "scripts/benchmark_ducc_f32_native.py")
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)
ORDERS = (("parent", "ducc", "candidate"), ("ducc", "candidate", "parent"), ("candidate", "parent", "ducc"))


def sha(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def bootstrap_median_ci(values, seed):
    samples_rng = random.Random(seed)
    medians = []
    for _ in range(20000):
        sample = [values[samples_rng.randrange(len(values))] for _ in values]
        medians.append(statistics.median(sample))
    return [bench.percentile(medians, 0.025), bench.percentile(medians, 0.975)]


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    SCRATCH.mkdir(parents=True, exist_ok=True)
    parent_sha = subprocess.check_output(["git", "-C", str(ROOT / "parent"), "rev-parse", "HEAD"], text=True).strip()
    patch = subprocess.check_output(["git", "-C", str(ROOT / "candidate"), "diff", "--binary"], text=False)
    (OUT / "candidate.patch").write_bytes(patch)
    rng = random.Random(0x7A720)
    all_cases = []
    for threads in (1, 8):
        cpus = "0" if threads == 1 else "0,2,4,6,8,10,12,14"
        for mmax, nphi in ((720, 1458), (180, 378)):
            case = {"grid": "GL", "ntheta": 721, "nphi": nphi, "lmax": 720,
                    "mmax": mmax, "longitude": "even", "spin": 0,
                    "operation": "synthesis", "threads": threads}
            case_id = f"GL-T720-M{mmax}-synthesis-q{threads}"
            fixture = bench.make_fixture(case, SCRATCH / "fixtures" / case_id)
            case_out = OUT / case_id
            case_out.mkdir(parents=True, exist_ok=True)
            pairs = []
            for pair in range(1, 16):
                order = ORDERS[(pair - 1) % 3]
                if rng.random() < 0.5:
                    order = tuple(reversed(order))
                outputs = {}
                for backend in order:
                    binary = {"parent": PARENT, "candidate": CANDIDATE, "ducc": DUCC}[backend]
                    result = bench.run_worker(binary, backend, case, fixture,
                        SCRATCH / case_id / f"pair-{pair}" / backend, 25.0, cpus)
                    outputs[backend] = result
                parent_ms = statistics.median(outputs["parent"]["batch_ms"])
                candidate_ms = statistics.median(outputs["candidate"]["batch_ms"])
                ducc_ms = statistics.median(outputs["ducc"]["batch_ms"])
                path_groups = {backend: [pathlib.Path(p) for p in outputs[backend]["outputs"]]
                               for backend in outputs}
                errors = {
                    "candidate_vs_parent": bench.error_metrics(path_groups["parent"], path_groups["candidate"]),
                    "parent_vs_ducc": bench.error_metrics(path_groups["ducc"], path_groups["parent"]),
                    "candidate_vs_ducc": bench.error_metrics(path_groups["ducc"], path_groups["candidate"]),
                }
                pair_result = {
                    "pair": pair, "order": order, "parent_ms": parent_ms,
                    "candidate_ms": candidate_ms, "ducc_ms": ducc_ms,
                    "candidate_parent": candidate_ms / parent_ms,
                    "candidate_ducc": candidate_ms / ducc_ms,
                    "parent_ducc": parent_ms / ducc_ms,
                    "outputs": errors,
                    "workers": {backend: {k: v for k, v in outputs[backend].items() if k != "outputs"}
                                for backend in outputs},
                }
                for paths in path_groups.values():
                    for path in paths:
                        path.unlink(missing_ok=True)
                pairs.append(pair_result)
                print(f"completed {case_id} pair {pair}", flush=True)
                (case_out / f"pair-{pair:02}.json").write_text(json.dumps(pair_result, indent=2) + "\n")
            cell = {
                "case": case, "case_id": case_id, "pairs": pairs,
                "parent_median_ms": statistics.median(p["parent_ms"] for p in pairs),
                "candidate_median_ms": statistics.median(p["candidate_ms"] for p in pairs),
                "ducc_median_ms": statistics.median(p["ducc_ms"] for p in pairs),
                "candidate_parent_median": statistics.median(p["candidate_parent"] for p in pairs),
                "candidate_parent_bootstrap_95": bootstrap_median_ci(
                    [p["candidate_parent"] for p in pairs], 0xF32A + threads + mmax),
                "candidate_ducc_median": statistics.median(p["candidate_ducc"] for p in pairs),
                "candidate_parent_wins": sum(p["candidate_parent"] < 1 for p in pairs),
            }
            all_cases.append(cell)
            print(f"completed {case_id}", flush=True)
    manifest = {
        "parent_sha": parent_sha, "candidate_parent_sha": parent_sha,
        "candidate_patch_sha256": hashlib.sha256(patch).hexdigest(),
        "parent_binary_sha256": sha(PARENT), "candidate_binary_sha256": sha(CANDIDATE),
        "ducc_source_sha": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "ducc_version": "0.42.0", "ducc_binary_module": str(ROOT / "ducc-install"),
        "ducc_flags": "-O3 -ffast-math -march=x86-64-v3 -ffp-contract=fast",
        "rust_flags": "-C target-cpu=x86-64-v3", "target_ms": 25.0,
        "q1_affinity": "0", "q8_affinity": "0,2,4,6,8,10,12,14",
        "fixture": "scripts/benchmark_ducc_f32_native.py make_fixture, phi0=0.23",
        "pairs_per_cell": 15, "process_order_balanced": True, "cases": all_cases,
    }
    (OUT / "results.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps([{k: v for k, v in row.items() if k.endswith("median_ms") or k.endswith("median") or k.endswith("wins") or k == "case_id"}
                      for row in all_cases], indent=2))


if __name__ == "__main__":
    main()
