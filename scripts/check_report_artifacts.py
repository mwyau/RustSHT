#!/usr/bin/env python3
"""Reject local environment and process captures in staged repository files."""

from __future__ import annotations

import re
import subprocess
import sys

RAW_NAMES = re.compile(
    r"(?:\.(?:preflight|postflight|during|concurrent|command|time)(?:\.txt)?|"
    r"\.stderr(?:\.txt)?|\.warnings(?:\.txt)?|\.running-[^/]*|"
    r"(?:process|environment|host)-snapshot[^/]*|"
    r"(?:(?:^|/)(?:build|test|pytest|cargo-test)[^/]*\.(?:stdout|stderr)(?:\.txt)?)|"
    r"(?:^|/)host\.txt)$",
    re.IGNORECASE,
)
CODE_SUFFIXES = {".py", ".sh", ".rs"}
ALWAYS_FORBIDDEN = [
    (re.compile(rb"/home/(?!<)[A-Za-z0-9_.-]+"), "absolute home-directory path"),
    (re.compile(rb"/tmp/(?!<)[A-Za-z0-9_.-]+"), "local temporary-directory path"),
    (re.compile(rb"/var/tmp/(?!<)[A-Za-z0-9_.-]+"), "local temporary-directory path"),
    (re.compile(rb"/dev" + rb"/shm(?:/|\b)"), "host temporary-filesystem path"),
    (re.compile(rb"^Linux\s+\S+.*GNU/Linux\s*$", re.MULTILINE), "host or kernel identity"),
]
CAPTURE_PATTERNS = [
    (re.compile(rb"^some avg10=[^\r\n]*(?:avg60=|avg300=|total=)", re.MULTILINE), "raw CPU PSI capture"),
    (re.compile(rb"^(?:Cpus_allowed_list|voluntary_ctxt_switches|nonvoluntary_ctxt_switches):", re.MULTILINE), "thread-placement or context-switch capture"),
    (re.compile(rb"^(?:Command being timed:|Elapsed \(wall clock\) time:|Maximum resident set size \(kbytes\):)", re.MULTILINE), "raw GNU time output"),
    (re.compile(rb"^(?:On-line CPU\(s\) list|NUMA node[0-9]+ CPU\(s\)|Allowed CPU list):", re.MULTILINE), "raw CPU topology or affinity"),
    (re.compile(rb"^(?:CPU MHz|CPU max MHz|CPU min MHz|scaling_cur_freq|cpu[0-9]+_frequency):", re.MULTILINE | re.IGNORECASE), "host model or live CPU-frequency capture"),
    (re.compile(rb"^(?:processor\s*:\s*\d+|vendor_id\s*:|cpu family\s*:|model name\s*:|stepping\s*:)", re.MULTILINE | re.IGNORECASE), "raw CPU information snapshot"),
    (re.compile(rb"^MemTotal:\s*\d+\s+kB\s*$", re.MULTILINE), "raw memory information snapshot"),
    (re.compile(rb"^(?:Hostname|Kernel release|Kernel version|Operating system):", re.MULTILINE | re.IGNORECASE), "host identity capture"),
    (re.compile(rb"^(?:HOME|PWD|USER|LOGNAME|SHELL|HOSTNAME|PATH)=", re.MULTILINE), "process environment dump"),
    (re.compile(rb"^(?:host(?:name|_name|_id)|user(?:name)?|logname)\s*[:=]", re.MULTILINE | re.IGNORECASE), "host or user identity capture"),
    (re.compile(rb"^(?:actual_affinity|requested_affinity|requested_taskset|wrapper_pid|observed_pid|thread_status_pid)=", re.MULTILINE), "raw process or placement capture"),
    (re.compile(rb"^(?:loadavg(?:_[^=]*)?|cpu_pressure|cgroup_cpu_max)=", re.MULTILINE | re.IGNORECASE), "live host-pressure capture"),
    (re.compile(rb"(?:taskset\s+-c\s*[\"']?\d+(?:-\d+)?(?:,\d+(?:-\d+)?)*|(?:affinity|cpu_set|cpu_ids)\s*[:=]\s*[\"']?\d+(?:-\d+)?(?:,\d+(?:-\d+)?)*|(?:affinity|placement)[^\r\n]{0,60}\b(?:CPU|core)\s*\d+)", re.IGNORECASE), "machine-specific CPU placement"),
    (re.compile(rb"^\s*PID\s+PPID\s+(?:TID\s+)?STAT\s+PSR\b", re.MULTILINE), "raw process table"),
    (re.compile(rb"^\s*PID\s+USER\s+PR\s+NI\s+VIRT\b", re.MULTILINE), "raw process table"),
    (re.compile(rb"(?m)^\s*(?:User time \(seconds\)|System time \(seconds\)|Percent of CPU this job got|Average total size \(kbytes\)|Average resident set size \(kbytes\)|Major \(requiring I/O\) page faults|Minor \(reclaiming a frame\) page faults|Voluntary context switches|Involuntary context switches|Swaps|File system inputs|File system outputs|Exit status):"), "raw GNU time output"),
    (re.compile(rb"(?m)^\s*(?:Compiling\s+[^\n]+\(|Finished `[^`]+` profile|running [0-9]+ tests?\b|test [^\n]+ \.\.\. (?:ok|FAILED)|Doc-tests )"), "raw build or test console output"),
    (re.compile(rb"(?m)^test result: (?:ok|FAILED)\b|^=+ .*\b(?:passed|failed|error)\b.*=+$"), "raw test summary output"),
    (re.compile(rb"(?i)\b(?:pid|ppid|tid)\s*[=: ]\s*[0-9]+"), "process identifier"),
    (re.compile(rb"(?m)^\s*(?:PID\s+USER\s+PR\s+NI\s+VIRT|PID\s+PPID\s+(?:TID\s+)?STAT\s+PSR)\b"), "raw process table"),
    (re.compile(rb"^\s*(?:top\s+-\s+\d|Tasks:\s+\d|%Cpu\(s\):)", re.MULTILINE), "raw top snapshot"),
    (re.compile(rb"^(?:cpu\.max=|cpu\.stat:|cgroup_cpu_stat:|cpuset_effective=)", re.MULTILINE), "raw cgroup snapshot"),
    (re.compile(rb"\"(?:hostname|host_name|cpu_model|cpu_frequency|actual_affinity|requested_affinity|process_snapshot)\"\s*:", re.IGNORECASE), "host or process snapshot field"),
    (re.compile(rb"\"command\"\s*:\s*\[[^\]]*(?:taskset|/usr/bin/time|/home/|/tmp/)", re.IGNORECASE), "local process command transcript"),
]


def main() -> int:
    names = subprocess.run(
        ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR", "-z"],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout.split(b"\0")
    failures: list[str] = []
    for raw_name in names:
        if not raw_name:
            continue
        name = raw_name.decode("utf-8", "surrogateescape")
        if RAW_NAMES.search(name):
            failures.append(f"{name}: raw capture filename")
            continue
        staged = subprocess.run(
            ["git", "show", f":{name}"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        if staged.returncode:
            continue
        data = staged.stdout
        for pattern, label in ALWAYS_FORBIDDEN:
            if pattern.search(data):
                failures.append(f"{name}: {label}")
                break
        else:
            suffix = "." + name.rsplit(".", 1)[-1].lower() if "." in name else ""
            for pattern, label in CAPTURE_PATTERNS:
                # Reproducible collector code may read these sources. Captured values
                # and local paths remain prohibited in all staged files.
                if suffix in CODE_SUFFIXES and label in {
                    "raw CPU PSI capture",
                    "thread-placement or context-switch capture",
                    "raw GNU time output",
                    "raw CPU topology or affinity",
                    "host model or live CPU-frequency capture",
                    "live host-pressure capture",
                    "raw process table",
                    "raw top snapshot",
                    "raw cgroup snapshot",
                    "host or process snapshot field",
                }:
                    continue
                if pattern.search(data):
                    failures.append(f"{name}: {label}")
                    break
    if failures:
        print("Staged files contain local environment or process artifacts:", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        print("Keep raw captures local and untracked; commit sanitized numerical evidence.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
