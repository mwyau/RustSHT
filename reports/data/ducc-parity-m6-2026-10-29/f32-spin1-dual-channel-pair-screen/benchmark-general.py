#!/usr/bin/env python3
"""Matched public f32 spin-1 synthesis screen: parent, fused pair, native DUCC."""
import hashlib
import importlib.util
import json
import os
import pathlib
import random
import statistics
import sys

ROOT = pathlib.Path(os.environ["M6_EXPERIMENT_ROOT"])
REPO = pathlib.Path(os.environ["M6_REPO_ROOT"])
PARENT = pathlib.Path(os.environ["M6_PARENT_BINARY"])
CANDIDATE = pathlib.Path(os.environ["M6_CANDIDATE_BINARY"])
GENERAL = pathlib.Path(os.environ["M6_GENERAL_BINARY"])
DUCC = pathlib.Path(os.environ["M6_DUCC_BINARY"])
OUT = ROOT / os.environ.get("M6_RESULTS_DIR", "results-q8-screen")
SCRATCH = ROOT / os.environ.get("M6_RUNS_DIR", "runs-q8-screen")
sys.path.insert(0, str(REPO / "scripts"))
spec = importlib.util.spec_from_file_location("bench", REPO / "scripts/benchmark_ducc_f32_native.py")
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)


def sha256(path):
    h = hashlib.sha256()
    with pathlib.Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    SCRATCH.mkdir(parents=True, exist_ok=True)
    pair_count = int(os.environ.get("M6_PAIRS", "3"))
    cases = []
    for threads in (1, 8):
        for mmax, nphi in ((720, 1458), (180, 378)):
            cases.append({"grid": "GL", "ntheta": 721, "nphi": nphi,
                          "lmax": 720, "mmax": mmax, "longitude": "even",
                          "spin": 1, "operation": "synthesis", "threads": threads})
    selected = set(os.environ.get("M6_CASES", "").split(","))
    if selected != {""}:
        cases = [c for c in cases if
                 f"GL-T720-M{c['mmax']}-s1-synthesis-q{c['threads']}" in selected]
    orderings = (("parent", "candidate", "general_f64", "ducc"),
                 ("ducc", "general_f64", "parent", "candidate"),
                 ("candidate", "ducc", "parent", "general_f64"))
    binaries = {"parent": PARENT, "candidate": CANDIDATE,
                "general_f64": GENERAL, "ducc": DUCC}
    rows = []
    for index, case in enumerate(cases):
        cid = f"GL-T720-M{case['mmax']}-s1-synthesis-q{case['threads']}"
        fixture = bench.make_fixture(case, SCRATCH / "fixtures" / cid)
        fixture_hashes = {name: sha256(path) for name, path in fixture.items()}
        pairs = []
        cpus = "0" if case["threads"] == 1 else "0,2,4,6,8,10,12,14"
        for pair_index in range(1, pair_count + 1):
            order = list(orderings[(pair_index - 1) % len(orderings)])
            if random.Random(0x5A17 + index * 19 + pair_index).random() < 0.5:
                order.reverse()
            workers = {}
            for backend in order:
                workers[backend] = bench.run_worker(
                    binaries[backend], backend, case, fixture,
                    SCRATCH / cid / f"pair-{pair_index:02}" / backend,
                    25.0, cpus)
            times = {name: statistics.median(workers[name]["batch_ms"])
                     for name in binaries}
            output_paths = {name: [pathlib.Path(path) for path in workers[name]["outputs"]]
                            for name in binaries}
            outputs = {name: [sha256(path) for path in output_paths[name]]
                       for name in binaries}
            errors = {
                "candidate_vs_ducc": bench.error_metrics(output_paths["ducc"], output_paths["candidate"]),
                "candidate_vs_parent": bench.error_metrics(output_paths["parent"], output_paths["candidate"]),
                "candidate_vs_general_f64": bench.error_metrics(output_paths["general_f64"], output_paths["candidate"]),
                "parent_vs_ducc": bench.error_metrics(output_paths["ducc"], output_paths["parent"]),
            }
            pair = {
                "pair": pair_index, "order": order, "times_ms": times,
                "candidate_parent_ratio": times["candidate"] / times["parent"],
                "candidate_general_ratio": times["candidate"] / times["general_f64"],
                "candidate_ducc_ratio": times["candidate"] / times["ducc"],
                "parent_ducc_ratio": times["parent"] / times["ducc"],
                "fixture_sha256": fixture_hashes, "output_sha256": outputs,
                "errors": errors,
                "workers": {name: {key: workers[name][key] for key in (
                    "backend", "first_ms", "warmup_ms", "setup_ms", "pool_setup_ms",
                    "fixture_load_ms", "repetitions_per_batch", "batch_ms", "batch_total_ms",
                    "process_elapsed_s", "output_checksum", "finite_outputs", "repeat_consistent",
                    "reset_consistent", "simd_arch", "f32_lanes", "telemetry") if key in workers[name]}
                    for name in binaries},
            }
            pairs.append(pair)
            for paths in output_paths.values():
                for path in paths:
                    path.unlink(missing_ok=True)
            (OUT / f"{cid}-pair-{pair_index:02}.json").write_text(
                json.dumps(pair, indent=2) + "\n")
            print(f"completed {cid} pair {pair_index}", flush=True)

        cp = [p["candidate_parent_ratio"] for p in pairs]
        cg = [p["candidate_general_ratio"] for p in pairs]
        cd = [p["candidate_ducc_ratio"] for p in pairs]
        row = {
            "case_id": cid, "case": case, "pairs": pair_count,
            "parent_median_ms": statistics.median(p["times_ms"]["parent"] for p in pairs),
            "candidate_median_ms": statistics.median(p["times_ms"]["candidate"] for p in pairs),
            "ducc_median_ms": statistics.median(p["times_ms"]["ducc"] for p in pairs),
            "candidate_parent_paired_median": statistics.median(cp),
            "candidate_parent_pair_range": [min(cp), max(cp)],
            "candidate_parent_faster_pairs": sum(value < 1.0 for value in cp),
            "general_f64_median_ms": statistics.median(p["times_ms"]["general_f64"] for p in pairs),
            "candidate_general_paired_median": statistics.median(cg),
            "candidate_general_pair_range": [min(cg), max(cg)],
            "candidate_ducc_paired_median": statistics.median(cd),
            "candidate_ducc_pair_range": [min(cd), max(cd)],
            "candidate_ducc_faster_pairs": sum(value < 1.0 for value in cd),
            "max_candidate_ducc_relative_l2": max(p["errors"]["candidate_vs_ducc"]["relative_l2"] for p in pairs),
            "max_candidate_ducc_absolute": max(p["errors"]["candidate_vs_ducc"]["max_absolute"] for p in pairs),
            "max_candidate_general_relative_l2": max(p["errors"]["candidate_vs_general_f64"]["relative_l2"] for p in pairs),
            "all_worker_checks": all(all(p["workers"][name].get(key) is True for key in
                                         ("finite_outputs", "repeat_consistent", "reset_consistent"))
                                      for p in pairs for name in binaries),
            "fixture_sha256": fixture_hashes, "pairs_data": pairs,
        }
        rows.append(row)
        (OUT / f"{cid}-summary.json").write_text(json.dumps(row, indent=2) + "\n")
        print(json.dumps({key: row[key] for key in (
            "case_id", "parent_median_ms", "candidate_median_ms", "ducc_median_ms",
            "candidate_parent_paired_median", "candidate_ducc_paired_median",
            "candidate_ducc_pair_range", "all_worker_checks")}), flush=True)
    manifest = {
        "source_parent_sha": os.environ["M6_SOURCE_PARENT_SHA"],
        "parent_adapter_patch_sha256": os.environ["M6_PARENT_PATCH_SHA256"],
        "candidate_patch_sha256": os.environ["M6_CANDIDATE_PATCH_SHA256"],
        "ducc_source_sha": bench.DUCC_SHA,
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "ducc_flags": "-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math",
        "compiler": os.environ.get("M6_COMPILER", "rustc 1.99.1; g++ 15.2.0"),
        "q1_affinity": "0", "q8_affinity": "0,2,4,6,8,10,12,14",
        "warm_target_ms": 25.0, "process_order_balanced": True,
        "pairs_per_cell": pair_count,
        "fixture": "scripts/benchmark_ducc_f32_native.py make_fixture; phi0=0.23",
        "parent_binary_sha256": sha256(PARENT),
        "candidate_binary_sha256": sha256(CANDIDATE),
        "general_f64_binary_sha256": sha256(GENERAL),
        "ducc_f32_binary_sha256": sha256(DUCC), "results": rows,
    }
    (OUT / "results.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
