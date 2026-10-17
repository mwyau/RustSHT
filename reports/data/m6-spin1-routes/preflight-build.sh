#!/usr/bin/env bash
set -euo pipefail
date -u '+timestamp=%Y-%m-%dT%H:%M:%SZ'
uptime
cat /proc/pressure/cpu
lscpu
printf '\nprocesses by CPU use:\n'
ps -eo pid,ppid,psr,pcpu,pmem,etimes,stat,comm,args --sort=-pcpu | head -n 25
printf '\nmatching compiler and benchmark processes:\n'
ps -eo pid,pcpu,stat,comm,args | awk '$4 == "rustc" || $4 == "cargo" || $4 == "cc1" || $4 == "cc1plus" || $4 == "make" || $4 == "cmake" || $4 == "ninja" || $4 ~ /^m1b_bench/ || $4 ~ /^time_SHT/ || $4 ~ /^ducc/ {print}'
