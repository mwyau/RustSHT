#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
data_dir="reports/data/atmospheric-resolution-2026-10-15/profiles"
metadata="$data_dir/synthesis-phase-profile-metadata.txt"
log="$data_dir/synthesis-phase-profile.log"
test_binary="target/release/deps/rustsht-159837ac1c3bd1f7"

taskset -c 8-15 env TMPDIR=target/rustsht-test-tmp cargo test --release --lib --no-run >/dev/null

{
    printf 'Measurement: ignored high-resolution synthesis phase diagnostic\n'
    printf 'Source HEAD: '
    git rev-parse HEAD
    printf 'Tracked worktree diff SHA-256: '
    git diff --binary -- src/transform.rs src/spin_transform.rs src/workspace.rs src/transform/tests.rs | sha256sum
    printf 'Branch: '
    git branch --show-current
    printf 'Working tree status:\n'
    git status --short
    printf 'Test binary SHA-256 (after the release no-run build):\n'
    sha256sum "$test_binary"
    printf 'Compiler:\n'
    rustc -vV
    cargo -V
    printf 'CPU model and topology:\n'
    lscpu | rg 'Model name|CPU\(s\)|Thread|Core|Socket|Flags'
    printf 'Benchmark affinity: CPUs 8-15 (taskset applied to cargo test process)\n'
    printf 'Build: release profile; no RUSTFLAGS or target-cpu override\n'
    printf 'DUCC reference in related report: ducc0 0.41.0, revision c1ed01b8fda69d2a378190f0915f1a19762dddbf\n'
    printf 'Host load:\n'
    uptime
    printf 'Concurrent compute processes:\n'
    ps -eo pid,psr,pcpu,comm,args | rg 'cargo|rustc|rustsht|pytest' || true
} > "$metadata"

taskset -c 8-15 env TMPDIR=target/rustsht-test-tmp cargo test --release --lib m5_synthesis_phase_profile -- --ignored --test-threads=1 --nocapture 2>&1 | tee "$log"
