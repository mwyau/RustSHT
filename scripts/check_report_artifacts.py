#!/usr/bin/env python3
"""Reject local environment and process captures in staged repository files."""

from __future__ import annotations

import re
import subprocess
import sys

RAW_NAMES = re.compile(
    r"(?:\.(?:preflight|postflight|during|concurrent|command|time|stdout|stderr)(?:\.txt)?|"
    r"\.warnings(?:\.txt)?|\.running-[^/]*|"
    r"\.(?:status|refusal|interference|stability-check-v3|v2-stability|postflight-failed)|"
    r"(?:process|environment|host)-snapshot[^/]*|"
    r"(?:^|/)(?:preflight[^/]*|[^/]*preflight[^/]*)\.(?:log|txt|csv|json|jsonl|stdout|stderr|time|command)$|"
    r"(?:^|/)[^/]*(?:process-list|process-snapshot|concurrent-processes|environment-snapshot|environment-dump|host-state-snapshot|cpu-pressure-snapshot|cpu-frequency-snapshot|cpu-affinity-snapshot|thread-placement-snapshot)[^/]*\.(?:log|txt|csv|json|jsonl|stdout|stderr|time|command)$|"
    r"(?:^|/)(?:isolation-preflight|scalar-check-preflight)\.log$|"
    r"(?:(?:^|/)[^/]*(?:build|compile|test|pytest|cargo-test|focused-tests)[^/]*\.(?:log|stdout|stderr)(?:\.txt)?)|"
    r"(?:^|/)[^/]*(?:baseline-build|candidate-build|focused-tests)\.log$|"
    r"(?:^|/)host\.txt|(?:^|/)reports/data/m6-m2-spin1-fused-visitor/candidate-source\.tar\.gz)$",
    re.IGNORECASE,
)
CODE_SUFFIXES = {".py", ".sh", ".rs"}
ALWAYS_FORBIDDEN = [
    (re.compile(rb"/(?:home|root|Users|tmp|var/tmp|private/tmp)/(?!<)[A-Za-z0-9_.-]+"), "absolute local home or temporary path"),
    (re.compile(rb"(?i)(?:^|[\s\"'`])/(?:[^/\s]+/)*(?:RustSHT|rustsht)(?:/|\s|$)"), "absolute local project/worktree path"),
    (re.compile(rb"/dev" + rb"/shm(?:/|\b)"), "host temporary-filesystem path"),
    (re.compile(rb"^Linux\s+\S+.*GNU/Linux\s*$", re.MULTILINE), "host or kernel identity"),
]
CAPTURE_PATTERNS = [
    (re.compile(rb"^some avg10=[^\r\n]*(?:avg60=|avg300=|total=)", re.MULTILINE), "raw CPU PSI capture"),
    (re.compile(rb"^(?:Cpus_allowed_list|voluntary_ctxt_switches|nonvoluntary_ctxt_switches):", re.MULTILINE), "thread-placement or context-switch capture"),
    (re.compile(rb"^(?:Command being timed:|Elapsed \(wall clock\) time:|Maximum resident set size \(kbytes\):)", re.MULTILINE), "raw GNU time output"),
    (re.compile(rb"^(?:On-line CPU\(s\) list|NUMA node[0-9]+ CPU\(s\)|Allowed CPU list):", re.MULTILINE), "raw CPU topology or affinity"),
    (re.compile(rb"^(?:CPU MHz|scaling_cur_freq|cpu[0-9]+_frequency):", re.MULTILINE | re.IGNORECASE), "live CPU-frequency capture"),
    (re.compile(rb"^processor\s*:\s*\d+", re.MULTILINE | re.IGNORECASE), "raw CPU information snapshot"),
    (re.compile(rb"^MemTotal:\s*\d+\s+kB\s*$", re.MULTILINE), "raw memory information snapshot"),
    (re.compile(rb"^Hostname:", re.MULTILINE | re.IGNORECASE), "host identity capture"),
    (re.compile(rb"^(?:HOME|PWD|USER|LOGNAME|SHELL|HOSTNAME|PATH)=", re.MULTILINE), "process environment dump"),
    (re.compile(rb"^(?:host(?:name|_name|_id)|user(?:name)?|logname)\s*[:=]", re.MULTILINE | re.IGNORECASE), "host or user identity capture"),
    (re.compile(rb"^(?:actual_affinity|requested_affinity|requested_taskset|wrapper_pid|observed_pid|thread_status_pid)=", re.MULTILINE), "raw process or placement capture"),
    (re.compile(rb"^(?:loadavg(?:_[^=]*)?|cpu_pressure|cgroup_cpu_max)=", re.MULTILINE | re.IGNORECASE), "live host-pressure capture"),
    (re.compile(rb"(?:taskset\s+-c\s*[\"']?\d+(?:-\d+)?(?:,\d+(?:-\d+)?)*|(?:affinity|cpu_set|cpu_ids)\s*[:=]\s*[\"']?\d+(?:-\d+)?(?:,\d+(?:-\d+)?)*|(?:affinity|placement)[^\r\n]{0,60}\b(?:CPU|core)\s*\d+)", re.IGNORECASE), "machine-specific CPU placement"),
    (re.compile(rb"^\s*PID\s+PPID\s+(?:TID\s+)?STAT\s+PSR\b", re.MULTILINE), "raw process table"),
    (re.compile(rb"^\s*UID\s+PID\s+PPID\s+C\s+STIME\s+TTY\s+TIME\s+CMD\b", re.MULTILINE), "raw process table"),
    (re.compile(rb"(?m)^\s*(?:Name|State|Tgid|Ngid|Pid|PPid|TracerPid|Uid|Gid|FDSize|Groups|NStgid|NSpid|NSpgid|NSsid|Threads|Cpus_allowed_list|voluntary_ctxt_switches|nonvoluntary_ctxt_switches):"), "raw /proc process state"),
    (re.compile(rb"(?m)^\s*(?:some avg10=|full avg10=|loadavg:|cpu\.max=|cpu\.stat:|cpuset\.cpus=)"), "raw host-pressure snapshot"),
    (re.compile(rb"^\s*PID\s+USER\s+PR\s+NI\s+VIRT\b", re.MULTILINE), "raw process table"),
    (re.compile(rb"(?m)^\s*(?:User time \(seconds\)|System time \(seconds\)|Percent of CPU this job got|Average total size \(kbytes\)|Average resident set size \(kbytes\)|Major \(requiring I/O\) page faults|Minor \(reclaiming a frame\) page faults|Voluntary context switches|Involuntary context switches|Swaps|File system inputs|File system outputs|Exit status):"), "raw GNU time output"),
    (re.compile(rb"(?m)^\s*(?:Compiling\s+[^\n]+\(|Finished `[^`]+` profile|running [0-9]+ tests?\b|test [^\n]+ \.\.\. (?:ok|FAILED)|Doc-tests )"), "raw build or test console output"),
    (re.compile(rb"(?m)^test result: (?:ok|FAILED)\b|^=+ .*\b(?:passed|failed|error)\b.*=+$"), "raw test summary output"),
    (re.compile(rb"(?i)\b(?:pid|ppid|tid)\s*[=: ]\s*[0-9]+"), "process identifier"),
    (re.compile(rb"(?m)^\s*(?:PID\s+USER\s+PR\s+NI\s+VIRT|PID\s+PPID\s+(?:TID\s+)?STAT\s+PSR)\b"), "raw process table"),
    (re.compile(rb"^\s*(?:top\s+-\s+\d|Tasks:\s+\d|%Cpu\(s\):)", re.MULTILINE), "raw top snapshot"),
    (re.compile(rb"^(?:cpu\.max=|cpu\.stat:|cgroup_cpu_stat:|cpuset_effective=)", re.MULTILINE), "raw cgroup snapshot"),
    (re.compile(rb"(?i)(?:reports/data|crates/rustsht-fft/benches/results)/[A-Za-z0-9_./$-]*(?:\.(?:preflight|postflight|during|concurrent|command|time|stdout|stderr)(?:\.txt)?|(?:preflight|postflight|process-list|process-snapshot|environment-snapshot|host-state-snapshot|cpu-pressure-snapshot|cpu-frequency-snapshot|thread-placement-snapshot)[^/]*\.(?:log|txt|csv|json|jsonl|stdout|stderr|time|command))\b"), "raw capture path reference"),
    (re.compile(rb"(?i)\"(?:hostname|host_name|username|user_name|logname|home|pwd|cwd|working_directory|temp_dir|tmpdir|pid|ppid|tid|process_id|wrapper_pid|observed_pid|command_line|cmdline|environment_dump|env_dump|actual_affinity|requested_affinity|requested_taskset|cpu_affinity|cpu_ids|cpu_id|worker_cpu_ids|thread_placement|preflight_snapshot|postflight_snapshot|during_snapshot|process_snapshot|process_list|concurrent_processes|competing_processes|host_state|machine_state|proc_snapshot|cpu_psi|some_avg10|full_avg10|psi_avg10|cpu_pressure|cpu_pressure_avg10|current_frequency|scaling_cur_freq|frequency_samples|frequency_snapshot|context_switches|involuntary_context_switches|voluntary_context_switches|nvcsw|nivcsw|loadavg|cpu_load|external_cpu_load|cpu_percent|cpu_utilization|cgroup_cpu_stat|cgroup_cpu_max|cpuset_effective|major_page_faults|minor_page_faults|page_faults)\"\s*:", re.IGNORECASE), "host or process snapshot field"),
    (re.compile(rb"(?im)^.*\b(?:some|full)\s+avg10\s*=\s*\d+(?:\.\d+)?(?:%|\b).*$"), "exact CPU PSI capture"),
    (re.compile(rb"(?i)(?:^|[.!?]\s*)[^.!?\r\n]{0,180}\b(?:CPU\s+)?PSI\b[^.!?\r\n]{0,100}\b(?:some\s+avg10|avg10|preflight|postflight|during[- ]run|periodic reading)\b[^.!?\r\n]{0,80}(?:\b(?:was|were|reached|rose|peaked|remained|stayed|ranged|measured|recorded|observed)\b[^.!?\r\n]{0,30}|avg10\s*[:=]?\s*)\d+(?:\.\d+)?%?"), "exact transient CPU PSI reading"),
    (re.compile(rb"(?im)^.*\b(?:PSI|avg10)\b.*\d+(?:\.\d+)?%\s*(?:x|\xc3\x97)\s*\d+.*$"), "CPU PSI sample series"),
    (re.compile(rb"(?i)(?:^|[.!?]\s*)[^.!?\r\n]{0,100}\b(?:live|current|sampled|during[- ]run|observed|aggregate|ranged|varied)\b[^.!?\r\n]{0,60}\bfrequency\b[^\r\n]{0,150}\d+(?:\.\d+)?\s*(?:GHz|MHz)[^\r\n]{0,80}\d+(?:\.\d+)?\s*(?:GHz|MHz)"), "transient CPU-frequency samples"),
    (re.compile(rb"(?i)(?:^|[.!?]\s*)[^.!?\r\n]{0,100}\b(?:CPU[- ]?)?frequency\b[^.!?\r\n]{0,80}\b(?:during[- ]run|sampled|live|current|observed|ranged|varied)\b[^\r\n]{0,100}\d+(?:\.\d+)?\s*(?:GHz|MHz)"), "transient CPU-frequency sample"),
    (re.compile(rb"(?i)(?:^|[.!?]\s*)[^.!?\r\n]{0,80}\b(?:during[- ]run|sampled|live|current|observed|ranged|varied)\b[^.!?\r\n]{0,80}\b(?:CPU[- ]?)?frequency\b[^\r\n]{0,120}\d+(?:\.\d+)?\s*(?:GHz|MHz)"), "transient CPU-frequency sample"),
    (re.compile(rb"(?i)\b(?:involuntary|voluntary)\s+context[- ]switch(?:es)?\b[^.!?\r\n]*\bcounts?\b[^.!?\r\n]*\b\d[\d,]*\b"), "exact context-switch count"),
    (re.compile(rb"(?i)(?:^|[.!?]\s*)[^.!?\r\n]{0,120}\b(?:external|background|other-process|nonbenchmark|host)\b[^.!?\r\n]{0,80}\bCPU\s+(?:activity|load|use)\b[^.!?\r\n]{0,80}\d+(?:\.\d+)?%"), "transient external CPU activity"),
    (re.compile(rb"\"command\"\s*:\s*\[[^\]]*(?:taskset|/usr/bin/time|/home/|/tmp/)", re.IGNORECASE), "local process command transcript"),
]
BENCHMARK_LOG_HEADER = re.compile(
    rb"(?im)^(?:spin(?:-\d+)?\s+T\d+|spin=\d+\s+T\d+|T\d+\s+GL\s+\d+x\d+).*"
)
BENCHMARK_LOG_SAMPLES = re.compile(
    rb"^[A-Za-z][A-Za-z0-9_ -]*(?:raw_)?samples_ms\s*[:=]|"
    rb"^.*\b\d+\s+(?:within-process\s+)?samples?,\s*median\b",
    re.IGNORECASE | re.MULTILINE,
)


def main() -> int:
    all_mode = sys.argv[1:] == ["--all"]
    name_command = (
        ["git", "ls-files", "-z"]
        if all_mode
        else ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR", "-z"]
    )
    names = subprocess.run(
        name_command,
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
        if all_mode:
            try:
                data = open(name, "rb").read()
            except (FileNotFoundError, IsADirectoryError):
                continue
        else:
            staged = subprocess.run(
                ["git", "show", f":{name}"],
                check=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
            )
            if staged.returncode:
                continue
            data = staged.stdout
        if (
            name.lower().endswith(".log")
            and BENCHMARK_LOG_HEADER.search(data)
            and BENCHMARK_LOG_SAMPLES.search(data)
        ):
            failures.append(f"{name}: raw benchmark process output; convert measurements to structured data")
            continue
        for pattern, label in ALWAYS_FORBIDDEN:
            if pattern.search(data):
                failures.append(f"{name}: {label}")
                break
        else:
            suffix = "." + name.rsplit(".", 1)[-1].lower() if "." in name else ""
            for pattern, label in CAPTURE_PATTERNS:
                # Reproducible collector code may read these sources. Captured values
                # and local paths remain prohibited in all staged files.
                if name == ".gitignore" and label == "raw capture path reference":
                    continue
                if suffix in CODE_SUFFIXES and label in {
                    "raw CPU PSI capture",
                    "thread-placement or context-switch capture",
                    "raw GNU time output",
                    "raw CPU topology or affinity",
                    "live CPU-frequency capture",
                    "live host-pressure capture",
                    "raw process table",
                    "raw top snapshot",
                    "raw cgroup snapshot",
                    "host or process snapshot field",
                    "exact CPU PSI capture",
                    "exact transient CPU PSI reading",
                    "CPU PSI sample series",
                    "transient CPU-frequency samples",
                    "transient CPU-frequency sample",
                    "exact context-switch count",
                    "transient external CPU activity",
                    "raw capture path reference",
                }:
                    continue
                if pattern.search(data):
                    failures.append(f"{name}: {label}")
                    break
    if failures:
        scope = "Repository files" if all_mode else "Staged files"
        print(f"{scope} contain local environment or process artifacts:", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        print("Keep raw captures local and untracked; commit sanitized numerical evidence.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
