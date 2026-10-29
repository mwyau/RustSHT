#!/usr/bin/env python3
"""Matched f32 adjacent and bandlimit controls for the isolated Stage A patch."""
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
PARENT = ROOT / "target-parent/release/examples/ducc_f32_full_transform_parity"
CANDIDATE = ROOT / "target-candidate/release/examples/ducc_f32_full_transform_parity"
DUCC = ROOT / "ducc-build/ducc_f32_full_transform_parity"
OUT = ROOT / "controls-f32"
RUNS = ROOT / "runs-controls-f32"
sys.path.insert(0, str(REPO / "scripts"))
spec = importlib.util.spec_from_file_location("f32bench", REPO / "scripts/benchmark_ducc_f32_native.py")
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)


def sha256(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def bootstrap(values, seed):
    rng = random.Random(seed)
    medians = []
    for _ in range(20000):
        medians.append(statistics.median(values[rng.randrange(len(values))] for _ in values))
    medians.sort()
    return [medians[int(0.025 * (len(medians) - 1))], medians[int(0.975 * (len(medians) - 1))]]


def cases():
    values = []
    for mmax, nphi in ((720, 1458), (180, 378)):
        for threads in (1, 8):
            count = 15 if threads == 8 else 3
            values.append(({"grid":"GL","ntheta":721,"nphi":nphi,"lmax":720,"mmax":mmax,
                            "longitude":"even","spin":0,"operation":"analysis","threads":threads},count))
    for operation in ("analysis", "synthesis"):
        for threads in (1, 8):
            values.append(({"grid":"GL","ntheta":721,"nphi":1458,"lmax":720,"mmax":720,
                            "longitude":"even","spin":1,"operation":operation,"threads":threads},3))
    values.extend([
        ({"grid":"GL","ntheta":256,"nphi":512,"lmax":255,"mmax":255,
          "longitude":"even","spin":0,"operation":"synthesis","threads":1},3),
        ({"grid":"GL","ntheta":1024,"nphi":2048,"lmax":1023,"mmax":1023,
          "longitude":"even","spin":0,"operation":"synthesis","threads":8},3),
    ])
    return values


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    RUNS.mkdir(parents=True, exist_ok=True)
    rng = random.Random(0xA51A2026)
    orderings = (("parent","candidate","ducc"),("ducc","candidate","parent"),("candidate","parent","ducc"))
    rows = []
    for case_index,(case,pair_count) in enumerate(cases()):
        cid = f"GL-T{case['lmax']}-M{case['mmax']}-s{case['spin']}-{case['operation']}-q{case['threads']}"
        fixture = bench.make_fixture(case, RUNS / "fixtures" / cid)
        fixture_hashes = {name:sha256(path) for name,path in fixture.items()}
        pairs = []
        cpus = "0" if case["threads"] == 1 else "0,2,4,6,8,10,12,14"
        for index in range(1,pair_count+1):
            order = list(orderings[(index-1)%3])
            if rng.random() < 0.5: order.reverse()
            bins = {"parent":PARENT,"candidate":CANDIDATE,"ducc":DUCC}
            workers = {}
            for backend in order:
                workers[backend] = bench.run_worker(bins[backend],backend,case,fixture,
                    RUNS / cid / f"pair-{index:02}" / backend,25.0,cpus)
            times = {name:statistics.median(workers[name]["batch_ms"]) for name in bins}
            paths = {name:[pathlib.Path(x) for x in workers[name]["outputs"]] for name in bins}
            pair = {
                "pair":index,"order":order,"times_ms":times,
                "candidate_parent_ratio":times["candidate"]/times["parent"],
                "candidate_ducc_ratio":times["candidate"]/times["ducc"],
                "errors":{
                    "candidate_vs_parent":bench.error_metrics(paths["parent"],paths["candidate"]),
                    "candidate_vs_ducc":bench.error_metrics(paths["ducc"],paths["candidate"]),
                    "parent_vs_ducc":bench.error_metrics(paths["ducc"],paths["parent"]),
                },
                "workers":{name:{k:workers[name][k] for k in
                    ("backend","first_ms","warmup_ms","setup_ms","pool_setup_ms","repetitions_per_batch",
                     "batch_ms","batch_total_ms","process_elapsed_s","finite_outputs","repeat_consistent",
                     "reset_consistent","simd_arch","f32_lanes") if k in workers[name]} for name in bins},
            }
            for output_paths in paths.values():
                for path in output_paths: path.unlink(missing_ok=True)
            pairs.append(pair)
            print(f"completed {cid} pair {index}",flush=True)
        cp = [p["candidate_parent_ratio"] for p in pairs]
        cd = [p["candidate_ducc_ratio"] for p in pairs]
        cell = {
            "case_id":cid,"case":case,"pairs":pair_count,
            "parent_median_ms":statistics.median(p["times_ms"]["parent"] for p in pairs),
            "candidate_median_ms":statistics.median(p["times_ms"]["candidate"] for p in pairs),
            "ducc_median_ms":statistics.median(p["times_ms"]["ducc"] for p in pairs),
            "candidate_parent_median":statistics.median(cp),"candidate_parent_range":[min(cp),max(cp)],
            "candidate_parent_95_ci":bootstrap(cp,0x6F00+case_index) if pair_count>=15 else None,
            "candidate_ducc_median":statistics.median(cd),"candidate_ducc_range":[min(cd),max(cd)],
            "all_checks":all(all(p["workers"][name].get(k) is True for k in
                ("finite_outputs","repeat_consistent","reset_consistent")) for p in pairs for name in ("parent","candidate","ducc")),
            "fixture_sha256":fixture_hashes,"pairs_data":pairs,
        }
        rows.append(cell)
        (OUT/f"{cid}.json").write_text(json.dumps(cell,indent=2)+"\n")
        print(json.dumps({k:cell[k] for k in ("case_id","pairs","parent_median_ms","candidate_median_ms","ducc_median_ms",
            "candidate_parent_median","candidate_parent_95_ci","candidate_ducc_median","all_checks")}),flush=True)
    manifest={
        "source_parent_sha":"3d46caaf89a8fcd3796fe43c49aa8ef82e481cad",
        "candidate_patch_sha256":"f62628d42c21db6bada73ee6b0d0ad5001b67e319ed5370c7b77fc2e01b3af31",
        "runner_sha256":sha256(pathlib.Path(__file__)),
        "ducc_sha":"42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "rust_flags":"--locked --release --features parallel; -C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "ducc_flags":"SHT: -O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math",
        "fixture_source":"scripts/benchmark_ducc_f32_native.py::make_fixture, phi0=0.23",
        "affinity":"one CPU q1; eight distinct physical cores q8, exact worker pins in runner source",
        "warm_target_ms":25,"parent_binary_sha256":sha256(PARENT),"candidate_binary_sha256":sha256(CANDIDATE),
        "ducc_f32_binary_sha256":sha256(DUCC),"rows":rows,
    }
    (OUT/"results.json").write_text(json.dumps(manifest,indent=2)+"\n")


if __name__ == "__main__": main()
