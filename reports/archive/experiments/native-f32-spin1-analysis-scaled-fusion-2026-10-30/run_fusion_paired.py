#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
from benchmark_ducc_f32_native import error_metrics, make_fixture, run_worker

OUT = Path(os.environ["RUSTSHT_M6_OUT"])
TARGET_MS = 25.0
PHI0 = 0.23


def configured_binary(name: str) -> Path:
    variable = f"RUSTSHT_M6_{name.upper()}"
    value = os.environ.get(variable)
    if not value:
        raise SystemExit(f"set {variable} to the corresponding worker executable")
    path = Path(value)
    if not path.is_file():
        raise SystemExit(f"worker executable does not exist: {path}")
    return path


def sha(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def run_f64(binary: Path, case: dict, fixture: dict[str, Path], run_dir: Path,
            cpus: str) -> dict:
    run_dir.mkdir(parents=True, exist_ok=True)
    args = [
        str(binary), "--worker", "analysis", "1", str(case["threads"]),
        str(case["ntheta"]), str(case["nphi"]), str(case["lmax"]),
        str(case["mmax"]), repr(PHI0), str(fixture["map0"]),
        str(fixture["map1"]), str(fixture["alm0"]), str(fixture["alm1"]),
        repr(TARGET_MS),
    ]
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = str(case["threads"])
    started = time.monotonic()
    done = subprocess.run(["taskset", "-c", cpus, *args], text=True,
                          capture_output=True, env=env)
    if done.returncode:
        raise RuntimeError(f"f64 worker failed for {case}:\n{done.stderr}\n{done.stdout}")
    result = json.loads(done.stdout.strip().splitlines()[-1])
    result["process_elapsed_s"] = time.monotonic() - started
    return result


def main() -> None:
    binaries = {
        "parent_f32": configured_binary("parent_f32"),
        "candidate_f32": configured_binary("candidate_f32"),
        "parent_f64": configured_binary("parent_f64"),
        "candidate_f64": configured_binary("candidate_f64"),
        "ducc_f32": configured_binary("ducc_f32"),
    }
    OUT.mkdir(parents=True, exist_ok=True)
    rows = []
    cases = [
        {
            "grid": "GL", "ntheta": 721, "nphi": nphi, "lmax": 720,
            "mmax": mmax, "longitude": "even", "spin": 1,
            "operation": "analysis", "threads": threads,
        }
        for mmax, nphi in ((720, 1458), (180, 378))
        for threads in (1, 8)
    ]
    orders = [
        ["parent_f32", "ducc_f32", "parent_f64", "candidate_f32", "candidate_f64"],
        ["candidate_f64", "candidate_f32", "parent_f64", "ducc_f32", "parent_f32"],
        ["ducc_f32", "parent_f32", "candidate_f64", "parent_f64", "candidate_f32"],
    ]
    for case in cases:
        case_id = f"GL-T720-M{case['mmax']}-s1-analysis-q{case['threads']}"
        fixture = make_fixture(case, OUT / "fixtures" / case_id)
        cpus = "0" if case["threads"] == 1 else "0,1,2,3,4,5,6,7"
        pairs = []
        for pair_index, order in enumerate(orders, 1):
            results = {}
            for name in order:
                run_dir = OUT / "scratch" / case_id / f"pair-{pair_index}" / name
                if name.endswith("f64"):
                    results[name] = run_f64(binaries[name], case, fixture, run_dir, cpus)
                else:
                    results[name] = run_worker(
                        binaries[name], name, case, fixture, run_dir, TARGET_MS, cpus
                    )
            pair = {"pair": pair_index, "order": order}
            for name in ("parent_f32", "candidate_f32", "parent_f64", "candidate_f64", "ducc_f32"):
                pair[f"{name}_ms"] = statistics.median(results[name]["batch_ms"])
            pair["parent_f32_over_parent_f64"] = pair["parent_f32_ms"] / pair["parent_f64_ms"]
            pair["candidate_f32_over_candidate_f64"] = pair["candidate_f32_ms"] / pair["candidate_f64_ms"]
            pair["candidate_f32_over_parent_f32"] = pair["candidate_f32_ms"] / pair["parent_f32_ms"]
            pair["parent_f32_over_ducc"] = pair["parent_f32_ms"] / pair["ducc_f32_ms"]
            pair["candidate_f32_over_ducc"] = pair["candidate_f32_ms"] / pair["ducc_f32_ms"]
            for backend in ("parent_f32", "candidate_f32"):
                pair[f"{backend}_vs_ducc"] = error_metrics(
                    [Path(path) for path in results["ducc_f32"]["outputs"]],
                    [Path(path) for path in results[backend]["outputs"]],
                )
            pair["candidate_f32_simd_arch"] = results["candidate_f32"].get("simd_arch")
            pair["candidate_f32_lanes"] = results["candidate_f32"].get("f32_lanes")
            for result in results.values():
                for output in result.get("outputs", []):
                    Path(output).unlink(missing_ok=True)
            pairs.append(pair)
            print(f"{case_id} pair {pair_index} complete", flush=True)

        row = {
            "case_id": case_id,
            "fixture_sha256": {name: sha(path) for name, path in fixture.items()},
            "pairs": pairs,
        }
        for key in (
            "parent_f32_ms", "candidate_f32_ms", "parent_f64_ms", "candidate_f64_ms",
            "ducc_f32_ms", "parent_f32_over_parent_f64", "candidate_f32_over_candidate_f64",
            "candidate_f32_over_parent_f32", "parent_f32_over_ducc", "candidate_f32_over_ducc",
        ):
            row[f"median_{key}"] = statistics.median(pair[key] for pair in pairs)
        row["max_candidate_relative_l2_vs_ducc"] = max(
            pair["candidate_f32_vs_ducc"]["relative_l2"] for pair in pairs
        )
        row["max_candidate_absolute_vs_ducc"] = max(
            pair["candidate_f32_vs_ducc"]["max_absolute"] for pair in pairs
        )
        rows.append(row)

    result = {
        "purpose": "paired same-session source-matched spin-1 analysis fusion screen",
        "base_head": "9cb2788cd3f18c0f262e24dff6fac58236e5a18d",
        "candidate_head": "be39b56137f45ac9bfceda4ccc497109c70fd34d",
        "candidate_patch_sha256": "06f0fa7cb578b1e2ff0a1a784ff476e758894c8126b5207dd764b6525fa8b434",
        "ducc_revision": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "rustc_vv": subprocess.check_output(["rustc", "-Vv"], text=True),
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "cpu_model": subprocess.check_output(["lscpu"], text=True).split("Model name:")[1].splitlines()[0].strip(),
        "affinity": {"q1": "0", "q8": "0,1,2,3,4,5,6,7"},
        "binaries": {name: {"role": name, "sha256": sha(path)} for name, path in binaries.items()},
        "process_pairs": 3,
        "target_ms": TARGET_MS,
        "cases": rows,
    }
    (OUT / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(OUT / "result.json")


if __name__ == "__main__":
    main()
