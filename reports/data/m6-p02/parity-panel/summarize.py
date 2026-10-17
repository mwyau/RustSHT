#!/usr/bin/env python3
"""Summarize the partial ordinary-kernel P02 screen."""

from __future__ import annotations

import csv
import random
import re
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
NAME = re.compile(r"t(?P<lmax>\d+)-s(?P<spin>\d+)-q1-pair(?P<pair>\d+)-(?P<variant>baseline|candidate)\.stdout$")
SAMPLES = re.compile(r"synthesis_rayon_samples_ms=\[([^]]+)\]")
RSS = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
EXIT = re.compile(r"Exit status:\s*(\d+)")
PSI = re.compile(r"^some avg10=([0-9.]+)", re.MULTILINE)
BOOTSTRAP_REPLICATES = 50_000
BOOTSTRAP_SEED = 2_026_101_7


def linear_quantile(values: list[float], probability: float) -> float:
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    low = int(position)
    high = min(low + 1, len(ordered) - 1)
    fraction = position - low
    return ordered[low] * (1 - fraction) + ordered[high] * fraction


preflight: dict[str, float] = {}
current: str | None = None
for line in (HERE / "preflight-processes.log").read_text().splitlines():
    run = re.search(r"--- run=(\S+)", line)
    if run:
        current = run.group(1)
    pressure = PSI.match(line)
    if current is not None and pressure:
        preflight[current] = float(pressure.group(1))

processes: dict[tuple[int, int, int, str], dict] = {}
for path in HERE.glob("t*-s*-q1-pair*-*.stdout"):
    match = NAME.fullmatch(path.name)
    assert match, path
    meta = match.groupdict()
    lmax = int(meta["lmax"])
    spin = int(meta["spin"])
    pair = int(meta["pair"])
    variant = meta["variant"]
    stem = path.stem
    output = path.read_text()
    config = output.splitlines()[0]
    assert f"GL {lmax + 1}x{2 * (lmax + 1)}" in config
    assert "requested_only=true" in config
    assert "phi0=0.37" in config or "phi0" not in config
    sample_match = SAMPLES.search(output)
    assert sample_match, path
    samples = [float(value.strip()) for value in sample_match.group(1).split(",")]
    assert len(samples) == 5, (path, samples)
    timing = path.with_suffix(".time").read_text()
    rss_match = RSS.search(timing)
    exit_match = EXIT.search(timing)
    assert rss_match and exit_match and exit_match.group(1) == "0", path
    pressure = preflight.get(stem)
    assert pressure is not None, stem
    key = (lmax, spin, pair, variant)
    assert key not in processes, key
    processes[key] = {
        "samples": samples,
        "median": statistics.median(samples),
        "rss_kib": int(rss_match.group(1)),
        "psi_some_avg10": pressure,
    }

process_rows: list[dict] = []
summary_rows: list[dict] = []
rng = random.Random(BOOTSTRAP_SEED)
cells = sorted({key[:2] for key in processes})
for lmax, spin in cells:
    pairs = sorted({key[2] for key in processes if key[:2] == (lmax, spin)})
    ratios: list[float] = []
    base_medians: list[float] = []
    candidate_medians: list[float] = []
    for pair in pairs:
        base = processes[(lmax, spin, pair, "baseline")]
        candidate = processes[(lmax, spin, pair, "candidate")]
        ratio = candidate["median"] / base["median"]
        ratios.append(ratio)
        base_medians.append(base["median"])
        candidate_medians.append(candidate["median"])
        process_rows.append(
            {
                "lmax": lmax,
                "spin": spin,
                "pair": pair,
                "baseline_first": str(pair % 2 == 1).lower(),
                "baseline_samples_ms": ";".join(f"{value:.3f}" for value in base["samples"]),
                "candidate_samples_ms": ";".join(f"{value:.3f}" for value in candidate["samples"]),
                "baseline_median_ms": f"{base['median']:.6f}",
                "candidate_median_ms": f"{candidate['median']:.6f}",
                "candidate_over_baseline": f"{ratio:.8f}",
                "baseline_psi_some_avg10": f"{base['psi_some_avg10']:.2f}",
                "candidate_psi_some_avg10": f"{candidate['psi_some_avg10']:.2f}",
                "baseline_rss_kib": base["rss_kib"],
                "candidate_rss_kib": candidate["rss_kib"],
            }
        )
    bootstrap = [
        statistics.median(rng.choices(ratios, k=len(ratios)))
        for _ in range(BOOTSTRAP_REPLICATES)
    ]
    summary_rows.append(
        {
            "lmax": lmax,
            "spin": spin,
            "threads": 1,
            "operation": "synthesis",
            "n_pairs": len(ratios),
            "pairs": ";".join(map(str, pairs)),
            "baseline_median_ms": f"{statistics.median(base_medians):.6f}",
            "candidate_median_ms": f"{statistics.median(candidate_medians):.6f}",
            "median_paired_ratio": f"{statistics.median(ratios):.8f}",
            "bootstrap95_low": f"{linear_quantile(bootstrap, 0.025):.8f}",
            "bootstrap95_high": f"{linear_quantile(bootstrap, 0.975):.8f}",
            "paired_ratio_min": f"{min(ratios):.8f}",
            "paired_ratio_max": f"{max(ratios):.8f}",
            "psi_rule": "run-one refuses CPU PSI some avg10 >= 8.0%",
        }
    )


def write_csv(path: Path, rows: list[dict]) -> None:
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


write_csv(HERE / "process-pairs.csv", process_rows)
write_csv(HERE / "summary.csv", summary_rows)
print(f"wrote {len(process_rows)} process pairs and {len(summary_rows)} summary rows")
