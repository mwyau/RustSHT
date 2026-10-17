#!/usr/bin/env python3
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__import__("subprocess").check_output(["git","rev-parse","--show-toplevel"], text=True).strip())
OUT = ROOT / "reports/data/m6-m1-scheduling"
PYTHON = ROOT / ".venv/bin/python"
DUCC = ROOT / "target/m6-references/ducc-build"
TMP = ROOT / "target/tmp"
CPUS = os.environ["RUSTSHT_CPUSET"]
THREADS = 8
L = 2047
NT = 2048
NP = 4096
RUNNER = OUT / "python_perf_baseline.py"

VARIANTS = {
    "baseline": ROOT / "target/m6-m1-scheduling",
    "candidate": ROOT / "target/m6-m1-scheduling-candidate",
}
ORDER = [
    ["baseline", "candidate"] if pair % 2 == 1 else ["candidate", "baseline"]
    for pair in range(1, 9)
]


def capture_preflight(stem: str) -> bool:
    timestamp = datetime.now(timezone.utc).isoformat()
    pressure = Path("/proc/pressure/cpu").read_text()
    uptime = subprocess.run(["uptime"], text=True, capture_output=True, check=True).stdout.strip()
    affinity = subprocess.run(
        ["taskset", "-pc", str(os.getpid())],
        text=True,
        capture_output=True,
        check=True,
    ).stdout.strip()
    processes = subprocess.run(
        [
            "ps",
            "-eo",
            "pid,ppid,psr,pcpu,pmem,etime,stat,comm,args",
            "--sort=-pcpu",
        ],
        text=True,
        capture_output=True,
        check=True,
    ).stdout

    some = re.search(r"^some .*?avg10=([0-9.]+).*?avg60=([0-9.]+)", pressure, re.M)
    full = re.search(r"^full .*?avg10=([0-9.]+)", pressure, re.M)
    psi_ok = bool(some and full and float(some.group(1)) <= 0.5 and float(some.group(2)) <= 1.0 and float(full.group(1)) <= 0.05)

    active = []
    for line in processes.splitlines()[1:]:
        fields = line.split(None, 8)
        if len(fields) < 9:
            continue
        pid, _, _, cpu, _, _, _, comm, command = fields
        if int(pid) == os.getpid():
            continue
        lower = f"{comm} {command}".lower()
        if any(
            marker in lower
            for marker in (
                "cargo build",
                "cargo test",
                "rustc",
                "cc1plus",
                "cc1 ",
                "cmake --build",
                "m1b_bench",
                "python_perf_baseline.py",
                "time_sht",
                "ducc benchmark",
            )
        ):
            active.append(line)
            continue
        try:
            if float(cpu) >= 15.0:
                active.append(line)
        except ValueError:
            pass

    allowed = psi_ok and not active
    content = [
        f"timestamp={timestamp}",
        f"requested_affinity={CPUS}",
        f"preflight_process_affinity={affinity}",
        f"psi_allowed={allowed}",
        f"psi_gate=some_avg10<=0.5,some_avg60<=1.0,full_avg10<=0.05; no matching build/benchmark or >=15% CPU process",
        f"uptime={uptime}",
        "pressure:",
        pressure.rstrip(),
        "active_or_heavy_processes:",
        *(active or ["none"]),
        "process_snapshot_top_35:",
        "\n".join(processes.splitlines()[:36]),
    ]
    (OUT / f"{stem}.preflight").write_text("\n".join(content) + "\n")
    return allowed


def run_one(pair: int, position: int, variant: str, suffix: str) -> None:
    stem = f"pair{pair:02d}-{variant}{suffix}"
    if not capture_preflight(stem):
        raise SystemExit(f"preflight refused {stem}; no process launched")

    source = VARIANTS[variant]
    env = {
        **os.environ,
        "TMPDIR": str(TMP),
        "PYTHONPATH": f"{source / 'python'}:{DUCC}",
        "RAYON_NUM_THREADS": str(THREADS),
        "RUSTSHT_PERF_RAW": "1",
    }
    command = [
        "taskset",
        "-c",
        CPUS,
        "/usr/bin/time",
        "-v",
        "-o",
        str(OUT / f"{stem}.time"),
        "env",
        f"TMPDIR={TMP}",
        f"PYTHONPATH={source / 'python'}:{DUCC}",
        f"RAYON_NUM_THREADS={THREADS}",
        "RUSTSHT_PERF_RAW=1",
        str(PYTHON),
        str(source / "examples/python_perf_baseline.py"),
        str(L),
        "0",
        str(THREADS),
        "GL",
        "1",
        "into",
        "synthesis",
        "both",
        "STANDARD",
        str(NP),
        str(NT),
    ]
    (OUT / f"{stem}.command").write_text(" ".join(command) + "\n")
    with (OUT / f"{stem}.stdout").open("w") as output:
        result = subprocess.run(command, env=env, stdout=output, stderr=subprocess.STDOUT)
    if result.returncode != 0:
        raise SystemExit(f"{stem} exited {result.returncode}; stop screen")
    print(f"completed {stem}", flush=True)


def main() -> None:
    start_pair = int(sys.argv[1]) if len(sys.argv) > 1 else 1
    end_pair = int(sys.argv[2]) if len(sys.argv) > 2 else 8
    suffix = sys.argv[3] if len(sys.argv) > 3 else ""
    if suffix and not suffix.startswith("-"):
        suffix = f"-{suffix}"
    if not (1 <= start_pair <= end_pair <= len(ORDER)):
        raise SystemExit("pair range must be within 1..8")
    for pair in range(start_pair, end_pair + 1):
        for position, variant in enumerate(ORDER[pair - 1], start=1):
            run_one(pair, position, variant, suffix)


if __name__ == "__main__":
    main()
