#!/usr/bin/env python3
"""Record successive baseline synthesis_into calls and host state."""

import csv
import hashlib
import json
import os
from pathlib import Path
import resource
import sys
import time

import numpy as np
import rustsht


L = 511
SPIN = 0
NTHETA = 512
NPHI = 1024
PHI0 = 0.37
THREADS = int(os.environ["RAYON_NUM_THREADS"])
MEASURED_CALLS = int(sys.argv[1])
OUT_CSV = Path(sys.argv[2])

nalm = sum(L - m + 1 for m in range(L + 1))
alm = np.zeros((1, nalm), dtype=np.complex128)
offset = 0
for m in range(L + 1):
    for ell in range(m, L + 1):
        if ell >= SPIN:
            re = ((ell * 7 + m * 11) % 19) - 9
            im = 0 if m == 0 else (((ell * 13 + m * 5) % 23) - 11)
            alm[0, offset] = (re + 1j * im) * 1e-5
        offset += 1

plan = rustsht.ShtPlan("GL", NTHETA, NPHI, L, L, spin=SPIN, phi0=PHI0)
output = np.empty((1, NTHETA, NPHI), dtype=np.float64)


def read_psi():
    try:
        return Path("/proc/pressure/cpu").read_text().strip().replace("\n", " | ")
    except OSError as exc:
        return f"unavailable:{exc}"


def read_cpu_state():
    result = {}
    for cpu in sorted(os.sched_getaffinity(0)):
        root = Path(f"/sys/devices/system/cpu/cpu{cpu}/cpufreq")
        values = {}
        for name in ("scaling_governor", "scaling_cur_freq", "cpuinfo_cur_freq"):
            try:
                values[name] = (root / name).read_text().strip()
            except OSError:
                values[name] = None
        result[str(cpu)] = values
    thermal = []
    for path in sorted(Path("/sys/class/thermal").glob("thermal_zone*/temp")):
        zone = path.parent
        try:
            name = (zone / "type").read_text().strip()
            value = path.read_text().strip()
            thermal.append({"zone": zone.name, "type": name, "temp": value})
        except OSError as exc:
            thermal.append({"zone": zone.name, "error": str(exc)})
    result["thermal"] = thermal or None
    return result


def read_threads():
    rows = []
    for task in sorted(Path("/proc/self/task").iterdir(), key=lambda item: int(item.name)):
        tid = int(task.name)
        row = {"tid": tid}
        try:
            row["comm"] = (task / "comm").read_text().strip()
            stat = (task / "stat").read_text()
            closing = stat.rfind(")")
            fields = stat[closing + 1 :].split()
            # /proc/<pid>/stat field 39 is the last CPU on which it ran.
            row["last_cpu"] = int(fields[36])
            status = (task / "status").read_text().splitlines()
            for line in status:
                if line.startswith("Cpus_allowed_list:"):
                    row["allowed_cpus"] = line.split(":", 1)[1].strip()
                    break
        except (OSError, ValueError, IndexError) as exc:
            row["snapshot_error"] = str(exc)
        rows.append(row)
    return rows


def usage():
    item = resource.getrusage(resource.RUSAGE_SELF)
    return {
        "voluntary_ctx": item.ru_nvcsw,
        "involuntary_ctx": item.ru_nivcsw,
        "user_s": item.ru_utime,
        "system_s": item.ru_stime,
    }


def run_call(index, phase, writer):
    cpu_before = read_cpu_state()
    psi_before = read_psi()
    usage_before = usage()
    started_ns = time.perf_counter_ns()
    plan.synthesis_into(alm, output)
    elapsed_ms = (time.perf_counter_ns() - started_ns) / 1e6
    usage_after = usage()
    psi_after = read_psi()
    cpu_after = read_cpu_state()
    threads_after = read_threads()
    writer.writerow(
        {
            "index": index,
            "phase": phase,
            "elapsed_ms": f"{elapsed_ms:.6f}",
            "voluntary_ctx_delta": usage_after["voluntary_ctx"] - usage_before["voluntary_ctx"],
            "involuntary_ctx_delta": usage_after["involuntary_ctx"] - usage_before["involuntary_ctx"],
            "voluntary_ctx_total": usage_after["voluntary_ctx"],
            "involuntary_ctx_total": usage_after["involuntary_ctx"],
            "psi_before": psi_before,
            "psi_after": psi_after,
            "cpu_before": json.dumps(cpu_before, sort_keys=True, separators=(",", ":")),
            "cpu_after": json.dumps(cpu_after, sort_keys=True, separators=(",", ":")),
            "threads_after": json.dumps(threads_after, sort_keys=True, separators=(",", ":")),
            "output_sha256": hashlib.sha256(output.tobytes()).hexdigest(),
        }
    )
    print(
        f"call={index} phase={phase} elapsed_ms={elapsed_ms:.6f} "
        f"involuntary_ctx_delta={usage_after['involuntary_ctx'] - usage_before['involuntary_ctx']} "
        f"psi_after={psi_after}"
    )


OUT_CSV.parent.mkdir(parents=True, exist_ok=True)
print(f"python={sys.version.split()[0]} numpy={np.__version__}")
print(f"extension={rustsht._rustsht.__file__}")
print(f"lmax={L} spin={SPIN} grid=GL {NTHETA}x{NPHI} phi0={PHI0} threads={THREADS}")
print(f"requested_affinity={sorted(os.sched_getaffinity(0))}")
print(f"alm_sha256={hashlib.sha256(alm.tobytes()).hexdigest()}")
print(f"psi_start={read_psi()}")
print(f"cpu_start={json.dumps(read_cpu_state(), sort_keys=True)}")

fields = [
    "index",
    "phase",
    "elapsed_ms",
    "voluntary_ctx_delta",
    "involuntary_ctx_delta",
    "voluntary_ctx_total",
    "involuntary_ctx_total",
    "psi_before",
    "psi_after",
    "cpu_before",
    "cpu_after",
    "threads_after",
    "output_sha256",
]
with OUT_CSV.open("w", newline="") as stream:
    writer = csv.DictWriter(stream, fieldnames=fields)
    writer.writeheader()
    run_call(0, "harness-correctness-call-equivalent", writer)
    run_call(1, "harness-untimed-warmup", writer)
    for call in range(MEASURED_CALLS):
        run_call(call + 2, "measured", writer)

print(f"psi_end={read_psi()}")
print(f"cpu_end={json.dumps(read_cpu_state(), sort_keys=True)}")
print(f"threads_end={json.dumps(read_threads(), sort_keys=True)}")
print(f"process_usage_end={json.dumps(usage(), sort_keys=True)}")
