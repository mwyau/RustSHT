#!/usr/bin/env python3
"""Separate first-call Legendre stage and worker diagnostics."""

import hashlib
import json
import os
import pathlib
import subprocess

ROOT = pathlib.Path(os.environ["M6_EXPERIMENT_ROOT"])
OUT = ROOT / "stage-profiles-v2"
OUT.mkdir(parents=True, exist_ok=True)
RUST = {
    "parent": ROOT / "target-profile-parent/release/examples/ducc_f32_full_transform_parity",
    "candidate": ROOT / "target-profile-candidate/release/examples/ducc_f32_full_transform_parity",
}
DUCC = ROOT / "ducc-worker.py"


def parse_fields(line):
    parts = line.split("\t")
    return {part.split("=", 1)[0]: part.split("=", 1)[1] for part in parts[1:] if "=" in part}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    rows = []
    for threads in (1, 8):
        cpus = "0" if threads == 1 else "0,2,4,6,8,10,12,14"
        for mmax, nphi in ((720, 1458), (180, 378)):
            case_id = f"GL-T720-M{mmax}-synthesis-q{threads}"
            fixture = ROOT / "runs-15/fixtures" / case_id
            for backend in ("parent", "candidate", "ducc"):
                output_path = OUT / f"{case_id}-{backend}.f32"
                args = [
                    "--worker", "synthesis", "0", "GL", str(threads), "721", str(nphi),
                    "720", str(mmax), "0.23", str(fixture / "map0.f32"),
                    str(fixture / "map1.f32"), str(fixture / "alm0.f32"),
                    str(fixture / "alm1.f32"), str(output_path),
                    str(OUT / "unused1.f32"), "25",
                ]
                env = os.environ.copy()
                env["RAYON_NUM_THREADS"] = str(threads)
                env["M6_PROFILE_LEGENDRE"] = "1"
                env["M6_PROFILE_EXIT_AFTER_FIRST"] = "1"
                if backend == "candidate" and threads > 1:
                    env["M6_PROFILE_ORDER_JOBS"] = "1"
                    env["M6_PROFILE_ORDER_LINES"] = "1"
                else:
                    env.pop("M6_PROFILE_ORDER_JOBS", None)
                    env.pop("M6_PROFILE_ORDER_LINES", None)
                if backend == "ducc":
                    env["PYTHONPATH"] = str(ROOT / "ducc-profile-install")
                    env["M6_COUNT_DUCC_UPDATES"] = "1"
                    binary = DUCC
                else:
                    env.pop("M6_COUNT_DUCC_UPDATES", None)
                    binary = RUST[backend]
                completed = subprocess.run(
                    ["taskset", "-c", cpus, str(binary), *args],
                    env=env, text=True, capture_output=True, timeout=240,
                )
                if completed.returncode:
                    raise RuntimeError(
                        f"{backend} failed in {case_id}:\n{completed.stderr}\n{completed.stdout}"
                    )
                if output_path.exists():
                    output_sha = sha(output_path)
                else:
                    output_sha = None
                record = {
                    "case_id": case_id, "backend": backend, "affinity": cpus,
                    "returncode": completed.returncode,
                    "stdout": completed.stdout, "stderr": completed.stderr,
                    "output_sha256": output_sha,
                }
                record["rust_workers"] = [parse_fields(line) for line in completed.stderr.splitlines()
                    if line.startswith("M6_RUST_LEGENDRE_PROFILE\t")]
                record["rust_stage"] = [parse_fields(line) for line in completed.stderr.splitlines()
                    if line.startswith("M6_RUST_LEGENDRE_STAGE_WALL\t")]
                record["reflected_route"] = [parse_fields(line) for line in completed.stderr.splitlines()
                    if line.startswith("M6_RUST_REFLECTED_ROUTE\t")]
                record["ducc_stage"] = [parse_fields(line) for line in completed.stderr.splitlines()
                    if line.startswith("M6_DUCC_LEGENDRE_PROFILE\t")]
                record["order_jobs"] = [parse_fields(line) for line in completed.stderr.splitlines()
                    if line.startswith("M6_ORDER_JOB\t")]
                rows.append(record)
                (OUT / f"{case_id}-{backend}.json").write_text(json.dumps(record, indent=2) + "\n")
                print(json.dumps({
                    "case_id": case_id, "backend": backend,
                    "rust_stage": record["rust_stage"],
                    "ducc_stage": record["ducc_stage"],
                    "workers": record["rust_workers"],
                    "order_jobs": len(record["order_jobs"]),
                }), flush=True)
    (OUT / "results.json").write_text(json.dumps({
        "parent_sha": "49295a89f2d13d8b02c2cbfa8566a8c5c21c6a80",
        "ducc_sha": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "ducc_version": "0.42.0", "rust_flags": "-C target-cpu=x86-64-v3",
        "ducc_flags": "-O3 -ffast-math -march=x86-64-v3 -ffp-contract=fast",
        "method": "one first-call Legendre stage, separate profile binaries, not timed production",
        "rows": rows,
    }, indent=2) + "\n")


if __name__ == "__main__":
    main()
