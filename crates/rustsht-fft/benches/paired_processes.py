#!/usr/bin/env python3
"""Run two benchmark commands in alternating process order and save raw output."""

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


def sha256_file(path: str) -> str | None:
    executable = Path(path)
    if not executable.is_file():
        return None
    digest = hashlib.sha256()
    with executable.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds")


def run_command(
    command: list[str],
    *,
    label: str,
    pair: int,
    order: int,
    cpu: str | None,
    cwd: Path,
    extra_env: dict[str, str],
) -> dict[str, Any]:
    env = os.environ.copy()
    env.update(extra_env)
    full_command = command
    if cpu is not None:
        full_command = ["taskset", "-c", cpu, *command]

    load_before = os.getloadavg()
    started = utc_now()
    start_time = time.perf_counter_ns()
    process = subprocess.Popen(
        full_command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    stdout, stderr = process.communicate()
    elapsed_ns = time.perf_counter_ns() - start_time
    executable = command[0]
    return {
        "record": "process",
        "started_utc": started,
        "pair": pair,
        "order": order,
        "label": label,
        "pid": process.pid,
        "cpu_affinity": cpu,
        "load_average_before": load_before,
        "elapsed_wall_ns": elapsed_ns,
        "exit_code": process.returncode,
        "command": command,
        "executable_sha256": sha256_file(executable),
        "stdout": stdout,
        "stderr": stderr,
    }


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: paired_processes.py CONFIG.json OUTPUT.jsonl", file=sys.stderr)
        return 2

    config_path = Path(sys.argv[1]).resolve()
    output_path = Path(sys.argv[2])
    config = json.loads(config_path.read_text())
    pairs = int(config["pairs"])
    if pairs < 1:
        raise ValueError("pairs must be positive")
    cpu = config.get("cpu")
    cwd = Path(config.get("cwd", config_path.parent)).resolve()
    common_env = {str(k): str(v) for k, v in config.get("env", {}).items()}
    side_a = config["a"]
    side_b = config["b"]
    for side in (side_a, side_b):
        if not side.get("label") or not side.get("command"):
            raise ValueError("each side needs a label and a nonempty command")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w") as output:
        metadata = {
            "record": "environment",
            "captured_utc": utc_now(),
            "pairs": pairs,
            "cpu_affinity": cpu,
            "cwd": str(cwd),
            "system": platform.platform(),
            "python": platform.python_version(),
            "metadata": config.get("metadata", {}),
        }
        output.write(json.dumps(metadata, separators=(",", ":")) + "\n")
        output.flush()

        for pair in range(1, pairs + 1):
            order = (side_a, side_b) if pair % 2 else (side_b, side_a)
            for order_index, side in enumerate(order, start=1):
                record = run_command(
                    [str(part) for part in side["command"]],
                    label=str(side["label"]),
                    pair=pair,
                    order=order_index,
                    cpu=cpu,
                    cwd=cwd,
                    extra_env={
                        **common_env,
                        **{str(k): str(v) for k, v in side.get("env", {}).items()},
                    },
                )
                output.write(json.dumps(record, separators=(",", ":")) + "\n")
                output.flush()
                if record["exit_code"] != 0:
                    print(
                        f"{record['label']} failed in pair {pair} with exit "
                        f"status {record['exit_code']}; raw output is in {output_path}",
                        file=sys.stderr,
                    )
                    return int(record["exit_code"] or 1)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
