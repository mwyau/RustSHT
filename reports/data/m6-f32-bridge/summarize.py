#!/usr/bin/env python3
"""Summarize the M6 native-f32 and widened-f64 benchmark screen."""

from __future__ import annotations

import csv
import json
import os
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "raw"


def median_from_stdout(text: str, name: str) -> float:
    match = re.search(rf"{name}_samples_ms=[^\n]*median_ms=([0-9.]+)", text)
    if match is None:
        raise ValueError(f"missing {name} median")
    return float(match.group(1))


def peak_rss_kib(path: Path) -> int:
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", path.read_text())
    if match is None:
        raise ValueError(f"missing RSS in {path}")
    return int(match.group(1))


ducc_by_cell: dict[str, dict] = {}
for threads in (1, 8):
    document = json.loads((RAW / f"ducc-q{threads}.json").read_text())
    ducc_by_cell.update(document["comparison"])

rows: list[dict[str, str | int | float]] = []
for lmax in (127, 511, 1023):
    for spin in (0, 1):
        for threads in (1, 8):
            cell_key = f"T{lmax}-s{spin}-q{threads}"
            cell = ducc_by_cell[cell_key]
            for operation in ("synthesis", "analysis"):
                stem = f"t{lmax}-s{spin}-q{threads}-{operation}"
                stdout = (RAW / f"{stem}.stdout").read_text()
                timing_path = RAW / f"{stem}.time"
                if "Exit status: 0" not in timing_path.read_text():
                    raise ValueError(f"failed Rust process: {timing_path}")
                native = median_from_stdout(stdout, "native_f32")
                bridge = median_from_stdout(stdout, "f32_through_f64_bridge")
                f64 = median_from_stdout(stdout, "native_f64_rounded_inputs")
                channels = cell["operations"][operation]
                native_error = max(
                    channel["native_f32_vs_ducc_f32"]["relative_l2"]
                    for channel in channels
                )
                bridge_error = max(
                    channel["bridge_f32_vs_ducc_f32"]["relative_l2"]
                    for channel in channels
                )
                bridge_to_ducc64_error = max(
                    channel["bridge_f32_vs_ducc_f64_cast_to_f32"]["relative_l2"]
                    for channel in channels
                )
                ducc_f32 = cell["ducc_timing"][operation]["f32_median_ms"]
                ducc_f64 = cell["ducc_timing"][operation]["f64_median_ms"]
                bridge_temp_match = re.search(
                    r"bridge_temporary_input_output_bytes=(\d+)", stdout
                )
                if bridge_temp_match is None:
                    raise ValueError(f"missing temporary-array estimate in {stem}")
                bridge_temp_mib = int(bridge_temp_match.group(1)) / 2**20
                rows.append(
                    {
                        "lmax": lmax,
                        "spin": spin,
                        "threads": threads,
                        "operation": operation,
                        "native_f32_ms": native,
                        "bridge_f32_ms": bridge,
                        "native_f64_rounded_ms": f64,
                        "ducc_f32_ms": ducc_f32,
                        "ducc_f64_ms": ducc_f64,
                        "native_over_bridge": native / bridge,
                        "bridge_over_ducc_f32": bridge / ducc_f32,
                        "bridge_over_ducc_f64": bridge / ducc_f64,
                        "native_max_rel_l2_vs_ducc_f32": native_error,
                        "bridge_max_rel_l2_vs_ducc_f32": bridge_error,
                        "bridge_max_rel_l2_vs_ducc_f64_cast": bridge_to_ducc64_error,
                        "bridge_temp_arrays_mib": bridge_temp_mib,
                        "comparison_process_peak_rss_mib": peak_rss_kib(timing_path) / 1024,
                    }
                )

with (HERE / "summary.csv").open("w", newline="") as handle:
    writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)

max_bridge_error = max(row["bridge_max_rel_l2_vs_ducc_f32"] for row in rows)
max_native_error = max(row["native_max_rel_l2_vs_ducc_f32"] for row in rows)
max_bridge64_error = max(row["bridge_max_rel_l2_vs_ducc_f64_cast"] for row in rows)
max_speedup = max(row["native_over_bridge"] for row in rows)
min_speedup = min(row["native_over_bridge"] for row in rows)
max_peak_rss = max(row["comparison_process_peak_rss_mib"] for row in rows)
source_linked_replay = (
    HERE.name == "replay-source-linked"
    or os.environ.get("RUSTSHT_M6_F32_SOURCE_LINKED") == "1"
)
lines = [
    "# M6 f32-to-f64 bridge screen",
    "",
    (
        "This is the source-linked replay and current timing table."
        if source_linked_replay
        else "This is the initial screen; the source-linked replay is the current timing table."
    ),
    "",
    "The private bridge widened public f32 coefficients or maps to f64, ran the",
    "f64 `ShtPlan`, and downcast outputs to f32. It beat native f32 in all 24",
    f"screen cells, with speedups from {min_speedup:.2f}x to {max_speedup:.2f}x. The bridge remains",
    "slower than DUCC f32 in most T511/T1023 cells; this screen does not establish",
    "f32 parity or accept a production change.",
    "",
    f"The maximum full-array relative-L2 error versus DUCC f32 was {max_bridge_error:.3e} for the",
    f"bridge and {max_native_error:.3e} for native f32. Bridge outputs differed from DUCC f64",
    f"cast to f32 by at most {max_bridge64_error:.3e}.",
    "",
    "Each Rust cell used one process, one warmup, and five within-process",
    "alternating samples for native f32, bridge, and f64 with the same f32-rounded",
    "inputs. DUCC f32 and f64 each used one warmup and five timed calls in a",
    "separate q-specific process. The Rust process retained both plans and",
    f"workspaces for comparison; its peak RSS of {max_peak_rss:.1f} MiB is not a per-mode",
    "memory measurement. The bridge's per-call temporary arrays occupy up to",
    f"{max(row['bridge_temp_arrays_mib'] for row in rows if row['lmax'] == 1023 and row['spin'] == 1):.1f} MiB at T1023 spin-1.",
    "",
    "The CSV contains all 24 GL STANDARD cells for T127/T511/T1023, scalar and",
    "spin-1, q1/q8, synthesis and analysis. DUCC measurements use version 0.41.0",
    "at source revision `c1ed01b8fda69d2a378190f0915f1a19762dddbf`.",
]
if source_linked_replay:
    lines.extend(
        [
            "",
            "This isolated rebuild used RustSHT revision",
            "`73493cf410809f8bbe0455f252b201188d0fc6a5` and the archived benchmark",
            "source. The release ELF SHA-256 matches all 24 matrix preflights:",
            "`2d5ddf6febd6ef9fd480a0c2cd97feb1f10f89ee0e003097cfa2b2be0ea6e4fb`.",
            "Clippy, build, preflight, and per-cell timing logs are retained in",
            "`raw/`. This fresh matrix is the current timing screen.",
            "",
            "A prior smoke without a recorded ELF hash and a replay using a",
            "different ELF (`aaf5808ed463632ac0926360265b3411f557eb2f8226f49510245a42c3bb54b6`)",
            "failed with `NonRealMZero { degree: 1 }`; both are excluded. The",
            "source-linked T127 spin-1 smoke and all 24 replay cells passed.",
            "The comparison remains diagnostic: it has five within-process",
            "samples, no independent process-pair repetitions, no odd-longitude",
            "or adjoint cases, and no per-mode RSS measurement.",
        ]
    )
else:
    lines.extend(
        [
            "",
            "These rows preserve the initial screen. A clean isolated rebuild",
            "of the archived source reproduced its recorded ELF hash and",
            "completed a fresh 24-cell matrix; see",
            "`replay-source-linked/summary.md` for the current timing screen.",
            "The initial and replay medians differ in some DUCC cells, so do not",
            "combine their timings. The source-linked replay is the current",
            "benchmark evidence. See `provenance.md` for methods and limits.",
        ]
    )
(HERE / "summary.md").write_text("\n".join(lines) + "\n")
print(f"wrote {len(rows)} rows; bridge speedup={min_speedup:.2f}x–{max_speedup:.2f}x")
