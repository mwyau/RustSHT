#!/usr/bin/env python3
"""Summarize the isolated P01 versus P02 synthesis comparison."""

import csv
import random
import re
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
NAME = re.compile(
    r"t(?P<lmax>\d+)-s(?P<spin>\d+)-q1-pair(?P<pair>\d+)-"
    r"(?P<variant>p01|p02)\.stdout$"
)
SAMPLES = re.compile(r"synthesis_rayon_samples_ms=\[([^]]+)\]")
RSS = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
EXIT = re.compile(r"Exit status:\s*(\d+)")


def read(path):
    match = NAME.fullmatch(path.name)
    assert match, path
    meta = {key: int(value) if key != "variant" else value
            for key, value in match.groupdict().items()}
    text = path.read_text()
    config = text.splitlines()[0]
    ntheta = meta["lmax"] + 1
    nphi = 2 * ntheta
    assert f"GL {ntheta}x{nphi}" in config
    assert "requested_only=true" in config and "phi0=0.37" in config
    samples = SAMPLES.search(text)
    assert samples
    values = [float(value) for value in samples.group(1).split(",")]
    assert len(values) == 5
    timing = path.with_suffix(".time").read_text()
    rss = RSS.search(timing)
    exit_status = EXIT.search(timing)
    assert rss and exit_status and exit_status.group(1) == "0"
    return {**meta, "values": values, "rss": int(rss.group(1))}


processes = {}
for path in HERE.glob("t*-s*-q1-pair*-*.stdout"):
    process = read(path)
    key = (process["lmax"], process["spin"], process["pair"], process["variant"])
    assert key not in processes
    processes[key] = process

cells = sorted({key[:2] for key in processes})
expected_cells = {(511, 0), (511, 1), (1023, 0), (2047, 0)}
assert set(cells) == expected_cells, (cells, expected_cells)
pair_rows = []
summary_rows = []
rng = random.Random(20261017)


def quantile(values, p):
    values = sorted(values)
    position = (len(values) - 1) * p
    lower = int(position)
    upper = min(lower + 1, len(values) - 1)
    fraction = position - lower
    return values[lower] * (1 - fraction) + values[upper] * fraction


for lmax, spin in cells:
    pairs = sorted({key[2] for key in processes if key[:2] == (lmax, spin)})
    assert pairs == list(range(1, 7)), (lmax, spin, pairs)
    ratios = []
    p01_medians = []
    p02_medians = []
    rss_deltas = []
    p01_samples = []
    p02_samples = []
    for pair in pairs:
        p01 = processes[(lmax, spin, pair, "p01")]
        p02 = processes[(lmax, spin, pair, "p02")]
        p01_median = statistics.median(p01["values"])
        p02_median = statistics.median(p02["values"])
        ratio = p02_median / p01_median
        ratios.append(ratio)
        p01_medians.append(p01_median)
        p02_medians.append(p02_median)
        p01_samples.extend(p01["values"])
        p02_samples.extend(p02["values"])
        rss_delta = p02["rss"] - p01["rss"]
        rss_deltas.append(rss_delta)
        pair_rows.append(
            {
                "lmax": lmax,
                "spin": spin,
                "threads": 1,
                "operation": "synthesis",
                "pair": pair,
                "p01_first": pair % 2 == 1,
                "p01_samples_ms": ";".join(f"{value:.3f}" for value in p01["values"]),
                "p02_samples_ms": ";".join(f"{value:.3f}" for value in p02["values"]),
                "p01_median_ms": f"{p01_median:.6f}",
                "p02_median_ms": f"{p02_median:.6f}",
                "p02_over_p01": f"{ratio:.8f}",
                "p01_rss_kib": p01["rss"],
                "p02_rss_kib": p02["rss"],
                "rss_delta_kib": rss_delta,
            }
        )
    bootstrap = [statistics.median(rng.choice(ratios) for _ in ratios)
                 for _ in range(50_000)]
    summary_rows.append(
        {
            "lmax": lmax,
            "spin": spin,
            "threads": 1,
            "operation": "synthesis",
            "n_pairs": 6,
            "p01_median_ms": f"{statistics.median(p01_medians):.6f}",
            "p02_median_ms": f"{statistics.median(p02_medians):.6f}",
            "median_paired_ratio": f"{statistics.median(ratios):.8f}",
            "bootstrap95_low": f"{quantile(bootstrap, 0.025):.8f}",
            "bootstrap95_high": f"{quantile(bootstrap, 0.975):.8f}",
            "paired_ratio_min": f"{min(ratios):.8f}",
            "paired_ratio_max": f"{max(ratios):.8f}",
            "p01_sample_min_ms": f"{min(p01_samples):.3f}",
            "p01_sample_max_ms": f"{max(p01_samples):.3f}",
            "p02_sample_min_ms": f"{min(p02_samples):.3f}",
            "p02_sample_max_ms": f"{max(p02_samples):.3f}",
            "median_rss_delta_kib": int(statistics.median(rss_deltas)),
        }
    )

for name, rows in (("process-pairs.csv", pair_rows), ("summary.csv", summary_rows)):
    with (HERE / name).open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
print(f"wrote {len(pair_rows)} synthesis-pair rows and {len(summary_rows)} summaries")
