#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: BSD-3-Clause
"""Screen deduplicated atmospheric real-FFT lengths with one-row timings."""

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
from typing import Any


def sha256_file(path: str) -> str:
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds")


def main() -> int:
    if len(sys.argv) != 3:
        print(
            "usage: screen_atmospheric_lengths.py CONFIG.json OUTPUT.jsonl",
            file=sys.stderr,
        )
        return 2

    config_path = Path(sys.argv[1]).resolve()
    output_path = Path(sys.argv[2])
    config = json.loads(config_path.read_text())
    binary = str(config["binary"])
    cpu = str(config["cpu"])
    cwd = Path(config["cwd"]).resolve()
    if not Path(binary).is_file():
        raise FileNotFoundError(binary)

    cases: dict[int, dict[str, Any]] = {}
    for item in config["truncations"]:
        for kind in ("minimum_odd", "minimum_even", "sampled", "fft_friendly"):
            length = int(item[kind])
            case = cases.setdefault(length, {"length": length, "truncations": [], "kinds": []})
            label = f"T{item['t']}"
            if label not in case["truncations"]:
                case["truncations"].append(label)
            if kind not in case["kinds"]:
                case["kinds"].append(kind)

    env = os.environ.copy()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w") as output:
        environment = {
            "record": "environment",
            "captured_utc": utc_now(),
            "source_revision": config["source_revision"],
            "binary": binary,
            "binary_sha256": sha256_file(binary),
            "cpu_affinity": cpu,
            "cwd": str(cwd),
            "system": platform.platform(),
            "python": platform.python_version(),
            "compiler": config["compiler"],
            "cpu_model": config["cpu_model"],
            "method": config["method"],
            "truncations": config["truncations"],
            "unique_lengths": len(cases),
        }
        output.write(json.dumps(environment, separators=(",", ":")) + "\n")
        output.flush()

        for case in cases.values():
            command = [binary, "--bench", "bench-native-real", str(case["length"]), "1"]
            load_before = os.getloadavg()
            started = utc_now()
            start = time.perf_counter_ns()
            process = subprocess.run(
                ["taskset", "-c", cpu, *command],
                cwd=cwd,
                env=env,
                text=True,
                capture_output=True,
                check=False,
            )
            record = {
                "record": "screen",
                "started_utc": started,
                "length": case["length"],
                "truncations": case["truncations"],
                "kinds": case["kinds"],
                "cpu_affinity": cpu,
                "load_average_before": load_before,
                "elapsed_wall_ns": time.perf_counter_ns() - start,
                "exit_code": process.returncode,
                "command": command,
                "stdout": process.stdout,
                "stderr": process.stderr,
            }
            output.write(json.dumps(record, separators=(",", ":")) + "\n")
            output.flush()
            if process.returncode != 0:
                print(
                    f"length {case['length']} failed; raw output is in {output_path}",
                    file=sys.stderr,
                )
                return process.returncode or 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
