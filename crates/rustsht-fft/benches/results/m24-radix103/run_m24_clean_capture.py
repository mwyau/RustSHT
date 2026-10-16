#!/usr/bin/env python3
"""Capture alternating M24/M21 process pairs with per-run host telemetry."""

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


def uptime_seconds() -> float | None:
    try:
        return float(Path("/proc/uptime").read_text().split()[0])
    except (OSError, ValueError, IndexError):
        return None


def cpu_model() -> str | None:
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.lower().startswith("model name"):
                return line.split(":", 1)[1].strip()
    except OSError:
        return None
    return None


def append_record(stream: Any, record: dict[str, Any]) -> None:
    stream.write(json.dumps(record, separators=(",", ":")) + "\n")
    stream.flush()
    os.fsync(stream.fileno())


def run_process(
    side: dict[str, Any], pair: int, order: int, cpu: str, cwd: Path,
    common_env: dict[str, str], output: Any,
) -> int:
    command = [str(part) for part in side["command"]]
    full_command = ["taskset", "-c", cpu, *command]
    env = os.environ.copy()
    env.update(common_env)
    env.update({str(k): str(v) for k, v in side.get("env", {}).items()})
    started = utc_now()
    start_ns = time.perf_counter_ns()
    load_before = os.getloadavg()
    uptime_before = uptime_seconds()
    process = subprocess.Popen(
        full_command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    samples = []
    while True:
        try:
            stdout, stderr = process.communicate(timeout=3)
            break
        except subprocess.TimeoutExpired:
            samples.append({"utc": utc_now(), "load_average": os.getloadavg()})
    elapsed_ns = time.perf_counter_ns() - start_ns
    append_record(output, {
        "record": "process",
        "started_utc": started,
        "pair": pair,
        "order": order,
        "label": str(side["label"]),
        "pid": process.pid,
        "cpu_affinity": cpu,
        "load_average_before": load_before,
        "load_average_during": samples,
        "load_average_after": os.getloadavg(),
        "uptime_before_seconds": uptime_before,
        "uptime_after_seconds": uptime_seconds(),
        "elapsed_wall_ns": elapsed_ns,
        "exit_code": process.returncode,
        "command": command,
        "executable_sha256": sha256_file(command[0]),
        "stdout": stdout,
        "stderr": stderr,
    })
    return int(process.returncode or 0)


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: run_m24_clean_capture.py CONFIG.json OUTPUT.jsonl", file=sys.stderr)
        return 2
    config_path = Path(sys.argv[1]).resolve()
    output_path = Path(sys.argv[2]).resolve()
    config = json.loads(config_path.read_text())
    pairs = int(config["pairs"])
    if pairs < 1:
        raise ValueError("pairs must be positive")
    cpu = str(config["cpu"])
    cwd = Path(config.get("cwd", config_path.parent)).resolve()
    common_env = {str(k): str(v) for k, v in config.get("env", {}).items()}
    sides = (config["a"], config["b"])
    for side in sides:
        if not side.get("label") or not side.get("command"):
            raise ValueError("each side needs a label and a nonempty command")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    if output_path.exists():
        raise FileExistsError(f"refusing to overwrite {output_path}")
    with output_path.open("x") as output:
        append_record(output, {
            "record": "environment",
            "captured_utc": utc_now(),
            "pairs": pairs,
            "cpu_affinity": cpu,
            "cwd": str(cwd),
            "system": platform.platform(),
            "python": platform.python_version(),
            "cpu_model": cpu_model(),
            "uptime_seconds": uptime_seconds(),
            "load_average": os.getloadavg(),
            "metadata": config.get("metadata", {}),
        })
        for pair in range(1, pairs + 1):
            order = sides if pair % 2 else tuple(reversed(sides))
            for order_index, side in enumerate(order, start=1):
                status = run_process(side, pair, order_index, cpu, cwd, common_env, output)
                if status != 0:
                    print(f"{side['label']} failed in pair {pair}; raw output is {output_path}", file=sys.stderr)
                    return status
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
