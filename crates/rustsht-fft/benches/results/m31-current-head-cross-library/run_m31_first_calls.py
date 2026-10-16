#!/usr/bin/env python3
"""Capture first and warmed calls for the M31 current-head SHT FFT targets."""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path

COMPLEX = (146, 150, 300, 256, 512, 722, 726, 1024, 1442, 1452, 2904)
REAL = (257, 723, 1025, 1443, 2049, 4097)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: run_m31_first_calls.py BINARY OUTPUT.jsonl CPU", file=sys.stderr)
        return 2
    binary = Path(sys.argv[1]).resolve()
    output = Path(sys.argv[2]).resolve()
    cpu = sys.argv[3]
    if not binary.is_file():
        raise FileNotFoundError(binary)
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        raise FileExistsError(output)
    operations = [
        *((mode, n) for n in COMPLEX for mode in ("c2c_forward", "c2c_inverse")),
        *((mode, n) for n in REAL for mode in ("r2c", "c2r")),
    ]
    with output.open("x") as stream:
        environment = {
            "record": "environment",
            "captured_utc": dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds"),
            "source": "RustSHT-FFT M30 current head cc6a626aede65c7bfdc9ebb0ab91cca5972fab94",
            "binary": str(binary),
            "binary_sha256": sha256(binary),
            "cpu_affinity": cpu,
            "cpu_model": next(
                (line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines() if line.lower().startswith("model name")),
                None,
            ),
            "system": platform.platform(),
            "operations": len(operations),
            "method": "One fresh process per operation; plan and workspace constructed before first public transform; first transform followed by seven warmed calls",
        }
        stream.write(json.dumps(environment, separators=(",", ":")) + "\n")
        stream.flush()
        os.fsync(stream.fileno())
        for mode, n in operations:
            command = [str(binary), "bench-native-first-call", mode, str(n)]
            started = time.perf_counter_ns()
            process = subprocess.Popen(
                ["taskset", "-c", cpu, *command],
                cwd=Path.cwd(),
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            stdout, stderr = process.communicate()
            record = {
                "record": "process",
                "mode": mode,
                "n": n,
                "pid": process.pid,
                "cpu_affinity": cpu,
                "elapsed_wall_ns": time.perf_counter_ns() - started,
                "exit_code": process.returncode,
                "command": command,
                "executable_sha256": sha256(binary),
                "stdout": stdout,
                "stderr": stderr,
            }
            stream.write(json.dumps(record, separators=(",", ":")) + "\n")
            stream.flush()
            os.fsync(stream.fileno())
            if process.returncode:
                print(f"{mode} N={n} failed; raw output is {output}", file=sys.stderr)
                return process.returncode
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
