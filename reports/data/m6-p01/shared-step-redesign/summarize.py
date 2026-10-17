#!/usr/bin/env python3
"""Summarize the retained shared-transition-step P01 screen."""

from __future__ import annotations

import csv
import random
import re
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
NAME = re.compile(
    r"t(?P<lmax>\d+)-s(?P<spin>\d+)-q1-pair(?P<pair>\d+)-"
    r"(?P<variant>[a-z0-9-]+)\.stdout$"
)
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
    assert sample_match, (path, output)
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
        "config": config,
    }

comparisons = [
    (1023, 0, "baseline", "candidate", range(1, 7), "T1023 scalar against pre-P01 baseline"),
    (511, 1, "baseline", "candidate", range(1, 7), "T511 spin-1 against pre-P01 baseline"),
    (511, 1, "p01-default", "p01-shared-step", range(1, 7), "T511 spin-1 redesigned versus original P01"),
]
process_rows: list[dict] = []
summary_rows: list[dict] = []
rng = random.Random(BOOTSTRAP_SEED)

for lmax, spin, control, candidate, pairs, label in comparisons:
    ratios: list[float] = []
    included: list[int] = []
    for pair in pairs:
        base = processes[(lmax, spin, pair, control)]
        cand = processes[(lmax, spin, pair, candidate)]
        base_pressure_ok = base["psi_some_avg10"] < 8.0
        cand_pressure_ok = cand["psi_some_avg10"] < 8.0
        pressure_ok = base_pressure_ok and cand_pressure_ok
        ratio = cand["median"] / base["median"]
        if pressure_ok:
            ratios.append(ratio)
            included.append(pair)
        process_rows.append(
            {
                "comparison": label,
                "lmax": lmax,
                "spin": spin,
                "pair": pair,
                "control_variant": control,
                "candidate_variant": candidate,
                "control_samples_ms": ";".join(f"{v:.3f}" for v in base["samples"]),
                "candidate_samples_ms": ";".join(f"{v:.3f}" for v in cand["samples"]),
                "control_median_ms": f"{base['median']:.6f}",
                "candidate_median_ms": f"{cand['median']:.6f}",
                "candidate_over_control": f"{ratio:.8f}",
                "control_psi_some_avg10": f"{base['psi_some_avg10']:.2f}",
                "candidate_psi_some_avg10": f"{cand['psi_some_avg10']:.2f}",
                "included_in_summary": str(pressure_ok).lower(),
                "control_rss_kib": base["rss_kib"],
                "candidate_rss_kib": cand["rss_kib"],
            }
        )

    assert ratios, label
    bootstrap = [
        statistics.median(rng.choices(ratios, k=len(ratios)))
        for _ in range(BOOTSTRAP_REPLICATES)
    ]
    summary_rows.append(
        {
            "comparison": label,
            "lmax": lmax,
            "spin": spin,
            "n_pairs_recorded": len(list(pairs)),
            "n_pairs_included": len(ratios),
            "included_pairs": ";".join(map(str, included)),
            "median_paired_ratio": f"{statistics.median(ratios):.8f}",
            "bootstrap95_low": f"{linear_quantile(bootstrap, 0.025):.8f}",
            "bootstrap95_high": f"{linear_quantile(bootstrap, 0.975):.8f}",
            "paired_ratio_min": f"{min(ratios):.8f}",
            "paired_ratio_max": f"{max(ratios):.8f}",
            "psi_exclusion_rule": "include pair only when both preflight some avg10 < 8.0%",
        }
    )


def write_csv(path: Path, rows: list[dict]) -> None:
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


write_csv(HERE / "process-pairs.csv", process_rows)
write_csv(HERE / "summary.csv", summary_rows)
print(f"wrote {len(process_rows)} paired rows and {len(summary_rows)} comparison rows")
