#!/usr/bin/env python3
"""Summarize retained T2047 q8 RustSHT, DUCC and SHTns samples."""

from __future__ import annotations

import csv
import re
import statistics
from pathlib import Path


ROOT = Path(__file__).resolve().parent


def percentile(values: list[float], probability: float) -> float:
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = position - lower
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction


def require(pattern: str, text: str) -> str:
    match = re.search(pattern, text)
    if not match:
        raise ValueError(f"missing {pattern!r}")
    return match.group(1)


rows: list[dict[str, str | int | float]] = []
for output in sorted(ROOT.glob("t2047-s*-q8-rep*.stdout")):
    match = re.fullmatch(r"t2047-s([01])-q8-rep(\d+)\.stdout", output.name)
    assert match, output.name
    spin, replicate = map(int, match.groups())
    text = output.read_text()
    time_text = output.with_suffix(".time").read_text()
    if "Exit status: 0" not in time_text or "Swaps: 0" not in time_text:
        raise ValueError(f"failed or swapped benchmark process: {output}")
    if "T2047 GL 2048x4096" not in text or "threads=8 runs=3 scope=into" not in text:
        raise ValueError(f"unexpected benchmark configuration: {output}")
    if "synthesis_error=" not in text or "analysis_error=" not in text:
        raise ValueError(f"missing Rust/DUCC numerical comparison: {output}")

    errors = {
        "synthesis": require(r"synthesis_error=[^\n]*?rel_l2=(\S+)", text),
        "analysis": require(r"analysis_error=[^\n]*?rel_l2=(\S+)", text),
    }
    shtns_errors = {
        "synthesis": require(r"synthesis_shtns_ducc_error=[^\n]*?rel_l2=(\S+)", text),
        "analysis": require(r"analysis_shtns_ducc_error=[^\n]*?rel_l2=(\S+)", text),
    }
    common = {
        "lmax": 2047,
        "spin": spin,
        "threads": 8,
        "replicate": int(replicate),
        "sample_count": 15,
        "max_rss_kib": int(require(r"Maximum resident set size \(kbytes\): (\d+)", time_text)),
    }
    for operation in ("synthesis", "analysis"):
        for backend, key in (
            ("rust", f"{operation}_rust_plan_raw_samples_ms"),
            ("ducc", f"{operation}_ducc_raw_samples_ms"),
            ("shtns_adapter", f"{operation}_shtns_adapter_raw_samples_ms"),
        ):
            raw = require(rf"{key}=([^\n]+)", text)
            samples = [float(value) for value in re.findall(r"\d+(?:\.\d+)?", raw)]
            if len(samples) != 15:
                raise ValueError(f"expected 15 samples for {output} {key}; got {len(samples)}")
            rows.append(
                {
                    **common,
                    "operation": operation,
                    "backend": backend,
                    "median_ms": statistics.median(samples),
                    "p10_ms": percentile(samples, 0.10),
                    "p90_ms": percentile(samples, 0.90),
                    "min_ms": min(samples),
                    "max_ms": max(samples),
                    "rust_ducc_rel_l2": errors[operation],
                    "shtns_ducc_rel_l2": shtns_errors[operation],
                }
            )

if not rows:
    raise ValueError("no T2047 q8 raw benchmark outputs found")
fields = list(rows[0])
with (ROOT / "summary.csv").open("w", newline="") as destination:
    writer = csv.DictWriter(destination, fieldnames=fields, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)

lookup = {
    (int(row["spin"]), str(row["operation"]), str(row["backend"])): row
    for row in rows
}
lines = [
    "# Matched T2047 q8 RustSHT, DUCC and SHTns results",
    "",
    "Each backend/operation has 15 samples (three groups of five) from one fresh process per spin. Times are medians of the retained raw samples; p10-p90 values and individual samples are in `summary.csv` and `*.stdout`. Rust and DUCC used reusable plans and caller-provided outputs. The SHTns adapter timing includes input preservation and E/B conversion.",
    "",
    "| spin | Rust S | DUCC S | Rust/DUCC | SHTns S | Rust/SHTns | Rust A | DUCC A | Rust/DUCC | SHTns A | Rust/SHTns | Rust peak RSS |",
    "|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
]
for spin in (0, 1):
    get = lambda op, backend: lookup[(spin, op, backend)]
    rs, ds, ss = (get("synthesis", backend)["median_ms"] for backend in ("rust", "ducc", "shtns_adapter"))
    ra, da, sa = (get("analysis", backend)["median_ms"] for backend in ("rust", "ducc", "shtns_adapter"))
    rss = max(int(get(op, "rust")["max_rss_kib"]) for op in ("synthesis", "analysis")) / 1024
    lines.append(
        f"| {spin} | {rs:.3f} | {ds:.3f} | {rs / ds:.2f}x | {ss:.3f} | {rs / ss:.2f}x | {ra:.3f} | {da:.3f} | {ra / da:.2f}x | {sa:.3f} | {ra / sa:.2f}x | {rss:.1f} MiB |"
    )
lines.extend(
    [
        "",
        "Full-array errors were checked before timing in each process. This is an M1 reference baseline, not a P01 candidate measurement. Raw output, `/usr/bin/time -v` records, and preflight snapshots are retained beside this summary.",
    ]
)
(ROOT / "summary.md").write_text("\n".join(lines) + "\n")
print("\n".join(lines))
