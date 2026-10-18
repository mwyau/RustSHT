#!/usr/bin/env bash
set -euo pipefail
script=$(git rev-parse --show-toplevel)/reports/data/m6-legendre-redesign/tile128-screen/run-one.sh
"$script" baseline
"$script" candidate
