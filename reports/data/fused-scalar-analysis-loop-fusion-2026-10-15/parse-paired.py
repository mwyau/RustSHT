#!/usr/bin/env python3
"""Extract process medians and paired changes from the raw benchmark log."""

from pathlib import Path
import re
import statistics


data_dir = Path(__file__).resolve().parent
log_path = data_dir / "paired-process-logs.txt"
samples_path = data_dir / "paired-process-samples.tsv"
summary_path = data_dir / "paired-summary.tsv"

records = []
pair = None
current = None
for line in log_path.read_text().splitlines():
    if line.startswith("pair="):
        match = re.fullmatch(r"pair=(\d+) spin=(\d+) T(\d+) q(\d+)", line)
        if match is None:
            raise ValueError(f"invalid pair header: {line}")
        pair = tuple(map(int, match.groups()))
    elif line.startswith("variant="):
        match = re.fullmatch(
            r"variant=(baseline|candidate) spin=(\d+) T(\d+) q(\d+) cpus=(\S+)",
            line,
        )
        if match is None or pair is None:
            raise ValueError(f"invalid process header: {line}")
        variant, spin, lmax, threads, cpus = match.groups()
        current = {
            "pair": pair[0],
            "spin": int(spin),
            "lmax": int(lmax),
            "threads": int(threads),
            "cpus": cpus,
            "variant": variant,
        }
        if pair[1:] != (int(spin), int(lmax), int(threads)):
            raise ValueError(f"pair/process mismatch: {pair} {line}")
    elif line.startswith("analysis_rayon_ms="):
        if current is None:
            raise ValueError("analysis median without process header")
        current["median_ms"] = float(line.split("=", 1)[1])
    elif line.startswith("analysis_rayon_samples_ms=["):
        if current is None:
            raise ValueError("analysis samples without process header")
        values = line.split("=[", 1)[1].removesuffix("]")
        current["samples_ms"] = values
    elif line.startswith("process_elapsed_s="):
        if current is None:
            raise ValueError("process timing without process header")
        match = re.fullmatch(r"process_elapsed_s=([0-9.]+) peak_rss_kib=(\d+)", line)
        if match is None:
            raise ValueError(f"invalid process timing: {line}")
        current["process_elapsed_s"], current["peak_rss_kib"] = match.groups()
        if not {"median_ms", "samples_ms"} <= current.keys():
            raise ValueError(f"incomplete process record: {current}")
        records.append(current)
        current = None

if current is not None:
    raise ValueError(f"incomplete final process record: {current}")
if len(records) != 40:
    raise ValueError(f"expected 40 process records, found {len(records)}")

with samples_path.open("w") as output:
    output.write(
        "pair\tspin\tlmax\tthreads\tcpus\tvariant\tmedian_ms\t"
        "samples_ms\tprocess_elapsed_s\tpeak_rss_kib\n"
    )
    for record in records:
        output.write(
            "\t".join(
                str(record[key])
                for key in (
                    "pair",
                    "spin",
                    "lmax",
                    "threads",
                    "cpus",
                    "variant",
                    "median_ms",
                    "samples_ms",
                    "process_elapsed_s",
                    "peak_rss_kib",
                )
            )
            + "\n"
        )

summary = []
for lmax in (511, 1023):
    for threads in (1, 8):
        baseline_by_pair = {
            row["pair"]: row
            for row in records
            if row["lmax"] == lmax
            and row["threads"] == threads
            and row["variant"] == "baseline"
        }
        candidate_by_pair = {
            row["pair"]: row
            for row in records
            if row["lmax"] == lmax
            and row["threads"] == threads
            and row["variant"] == "candidate"
        }
        if baseline_by_pair.keys() != candidate_by_pair.keys() or len(baseline_by_pair) != 5:
            raise ValueError(f"incomplete pairs at T{lmax} q{threads}")
        changes = [
            100.0
            * (
                float(candidate_by_pair[index]["median_ms"])
                / float(baseline_by_pair[index]["median_ms"])
                - 1.0
            )
            for index in sorted(baseline_by_pair)
        ]
        baseline = [float(row["median_ms"]) for row in baseline_by_pair.values()]
        candidate = [float(row["median_ms"]) for row in candidate_by_pair.values()]
        rss = [int(row["peak_rss_kib"]) for row in candidate_by_pair.values()]
        summary.append(
            (
                lmax,
                threads,
                statistics.median(baseline),
                statistics.median(candidate),
                statistics.median(changes),
                min(changes),
                max(changes),
                statistics.median(rss),
                min(rss),
                max(rss),
            )
        )

with summary_path.open("w") as output:
    output.write(
        "lmax\tthreads\tbaseline_median_ms\tcandidate_median_ms\t"
        "paired_change_median_pct\tpaired_change_min_pct\tpaired_change_max_pct\t"
        "candidate_peak_rss_median_kib\tcandidate_peak_rss_min_kib\t"
        "candidate_peak_rss_max_kib\n"
    )
    for row in summary:
        fields = [
            f"{value:.3f}" if isinstance(value, float) else str(value)
            for value in row
        ]
        output.write("\t".join(fields) + "\n")
