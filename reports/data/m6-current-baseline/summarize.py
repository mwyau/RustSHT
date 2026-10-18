#!/usr/bin/env python3
"""Summarize retained M6 Rust/DUCC/SHTns raw benchmark samples."""

from __future__ import annotations

import csv
import re
import statistics
from pathlib import Path


ROOT = Path(__file__).resolve().parent
ROWS: list[dict[str, str | int | float]] = []


def percentile(values: list[float], probability: float) -> float:
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = position - lower
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction


def elapsed_seconds(text: str) -> float:
    parts = text.split(":")
    if len(parts) == 3:
        return int(parts[0]) * 3600 + int(parts[1]) * 60 + float(parts[2])
    if len(parts) == 2:
        return int(parts[0]) * 60 + float(parts[1])
    return float(text)


def value_after(pattern: str, text: str) -> str:
    match = re.search(pattern, text)
    if not match:
        raise ValueError(f"missing {pattern!r}")
    return match.group(1)


for output in sorted(ROOT.glob("t*-q*-rep*.stdout")):
    match = re.fullmatch(r"t(\d+)-s(\d+)-q(\d+)-rep(\d+)\.stdout", output.name)
    assert match, output.name
    lmax, spin, threads, replicate = map(int, match.groups())
    text = output.read_text()
    time_text = output.with_suffix(".time").read_text()
    if "Exit status: 0" not in time_text:
        raise ValueError(f"failed benchmark process: {output}")
    if "runs=3" not in text or "geometry=GL" in text:
        if " runs=3 " not in text:
            raise ValueError(f"unexpected run configuration: {output}")

    rust_error = {}
    shtns_error = {}
    for operation in ("synthesis", "analysis"):
        rust_error[operation] = value_after(
            rf"{operation}_error=.*?rel_l2=([^ ]+)", text
        )
        shtns_error[operation] = value_after(
            rf"{operation}_shtns_ducc_error=.*?rel_l2=([^ ]+)", text
        )

    common = {
        "lmax": lmax,
        "spin": spin,
        "threads": threads,
        "replicate": replicate,
        "sample_count": 15,
        "wall_s": elapsed_seconds(
            value_after(r"Elapsed \(wall clock\) time .*?: (\S+)", time_text)
        ),
        "process_cpu_percent": float(value_after(r"Percent of CPU this job got: (\d+)%", time_text)),
        "max_rss_kib": int(value_after(r"Maximum resident set size \(kbytes\): (\d+)", time_text)),
    }

    for operation in ("synthesis", "analysis"):
        backends = {
            "rust": f"{operation}_rust_plan_raw_samples_ms",
            "ducc": f"{operation}_ducc_raw_samples_ms",
            "shtns_adapter": f"{operation}_shtns_adapter_raw_samples_ms",
        }
        samples_by_backend: dict[str, list[float]] = {}
        for backend, key in backends.items():
            raw = value_after(rf"{key}=([^\n]+)", text)
            samples = [float(value) for value in re.findall(r"\d+(?:\.\d+)?", raw)]
            if len(samples) != 15:
                raise ValueError(f"expected 15 samples for {output} {key}, found {len(samples)}")
            samples_by_backend[backend] = samples
            ROWS.append(
                {
                    **common,
                    "operation": operation,
                    "backend": backend,
                    "median_ms": statistics.median(samples),
                    "p10_ms": percentile(samples, 0.10),
                    "p90_ms": percentile(samples, 0.90),
                    "min_ms": min(samples),
                    "max_ms": max(samples),
                    "rust_ducc_rel_l2": rust_error[operation],
                    "shtns_ducc_rel_l2": shtns_error[operation],
                }
            )


fields = list(ROWS[0])
with (ROOT / "summary.csv").open("w", newline="") as destination:
    writer = csv.DictWriter(destination, fieldnames=fields)
    writer.writeheader()
    writer.writerows(ROWS)

configs = sorted({(int(row["lmax"]), int(row["spin"]), int(row["threads"])) for row in ROWS})
lines = [
    "# Matched RustSHT, DUCC and SHTns baseline",
    "",
    "Medians pool the 15 retained raw samples (three groups of five); p10-p90 captures spread. Rust and DUCC measurements use reusable plans and caller-provided output buffers. SHTns is the documented adapter scope, including input preservation and E/B potential conversion.",
    "",
    "| T | spin | threads | Rust S | DUCC S | Rust/DUCC | SHTns S | Rust A | DUCC A | Rust/DUCC | SHTns A | Rust peak RSS |",
    "|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
]
for lmax, spin, threads in configs:
    lookup = {
        (row["operation"], row["backend"]): row
        for row in ROWS
        if (int(row["lmax"]), int(row["spin"]), int(row["threads"])) == (lmax, spin, threads)
    }
    synth_rust = lookup[("synthesis", "rust")]
    synth_ducc = lookup[("synthesis", "ducc")]
    synth_shtns = lookup[("synthesis", "shtns_adapter")]
    analysis_rust = lookup[("analysis", "rust")]
    analysis_ducc = lookup[("analysis", "ducc")]
    analysis_shtns = lookup[("analysis", "shtns_adapter")]
    lines.append(
        f"| {lmax} | {spin} | {threads} | {synth_rust['median_ms']:.3f} | {synth_ducc['median_ms']:.3f} | {synth_rust['median_ms'] / synth_ducc['median_ms']:.2f}x | {synth_shtns['median_ms']:.3f} | {analysis_rust['median_ms']:.3f} | {analysis_ducc['median_ms']:.3f} | {analysis_rust['median_ms'] / analysis_ducc['median_ms']:.2f}x | {analysis_shtns['median_ms']:.3f} | {synth_rust['max_rss_kib'] / 1024:.1f} MiB |"
    )
lines.extend(
    [
        "",
        "Relative L2 errors were checked in every process before timing. Per-process numerical errors, timing samples, and aggregate RSS are summarized in structured outputs; raw process and GNU-time captures remain local.",
    ]
)
(ROOT / "summary.md").write_text("\n".join(lines) + "\n")
print("\n".join(lines))
