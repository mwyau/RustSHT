#!/usr/bin/env bash
set -euo pipefail
script=$(git rev-parse --show-toplevel)/reports/data/m6-legendre-redesign/prototype-screen/run-one.sh
# Pair order alternates by size: B/C at T511, C/B at T1023, B/C at T2047.
for entry in baseline:511:t511 candidate:511:t511 candidate:1023:t1023 baseline:1023:t1023 baseline:2047:t2047 candidate:2047:t2047; do
  version=${entry%%:*}
  rest=${entry#*:}
  lmax=${rest%%:*}
  run_id=${rest##*:}
  "$script" "$version" "$lmax" "$run_id"
done
