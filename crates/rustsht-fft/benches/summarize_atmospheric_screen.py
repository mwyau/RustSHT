#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: BSD-3-Clause
"""Write a directional table from the M22 atmospheric-length screen."""

from __future__ import annotations

import argparse
import csv
import io
import json
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("input", type=Path)
    args = parser.parse_args()

    records = [json.loads(line) for line in args.input.read_text().splitlines()]
    if not records or records[0].get("record") != "environment":
        raise SystemExit(f"missing environment record in {args.input}")
    rows: list[dict[str, str | int | float]] = []
    for record in records[1:]:
        if record.get("exit_code") != 0:
            raise SystemExit(f"nonzero process status at N={record.get('length')}")
        operation_rows = csv.DictReader(io.StringIO(record["stdout"]))
        for operation in operation_rows:
            if not operation.get("mode"):
                continue
            rows.append(
                {
                    "length": int(operation["n"]),
                    "truncations": ";".join(record["truncations"]),
                    "kinds": ";".join(record["kinds"]),
                    "operation": operation["mode"],
                    "single_row_ns": float(operation["public_operation_ns"]),
                    "plan_ns": float(operation["plan_ns"]),
                    "workspace_ns": float(operation["workspace_ns"]),
                    "workspace_bytes": int(operation["workspace_bytes"]),
                    "checksum": float(operation["checksum"]),
                    "process_wall_ns": int(record["elapsed_wall_ns"]),
                    "load_1m_before": float(record["load_average_before"][0]),
                }
            )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


if __name__ == "__main__":
    main()
