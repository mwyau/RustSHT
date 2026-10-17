"""Summarize the complete P01 linker-order acceptance matrix."""

from __future__ import annotations

import csv
import random
import re
import statistics
from pathlib import Path


HERE = Path(__file__).resolve().parent
NAME = re.compile(
    r"t(?P<lmax>\d+)-s(?P<spin>\d+)-q(?P<threads>\d+)-"
    r"pair(?P<pair>\d+)-(?P<variant>baseline|ordered)\.stdout$"
)
SAMPLES = re.compile(r"(?P<op>synthesis|analysis)_rayon_samples_ms=\[([^]]+)\]")
RSS = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
EXIT = re.compile(r"Exit status:\s*(\d+)")
BOOTSTRAP_REPLICATES = 50_000
BOOTSTRAP_SEED = 2_026_101_7
EXPECTED_CELLS = {
    (511, 0, 1),
    (511, 1, 1),
    (1023, 0, 1),
    (2047, 0, 1),
    (1023, 0, 8),
    (2047, 0, 8),
}


def linear_quantile(values: list[float], probability: float) -> float:
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    low = int(position)
    high = min(low + 1, len(ordered) - 1)
    fraction = position - low
    return ordered[low] * (1 - fraction) + ordered[high] * fraction


def read_process(path: Path) -> dict:
    match = NAME.fullmatch(path.name)
    assert match, path
    metadata = {
        key: int(value) if key != "variant" else value
        for key, value in match.groupdict().items()
    }
    output = path.read_text()
    config = output.splitlines()[0]
    expected_geometry = f"GL {metadata['lmax'] + 1}x{2 * (metadata['lmax'] + 1)}"
    assert expected_geometry in config and "requested_only=true" in config and "phi0=0.37" in config, (path, config)
    samples: dict[str, list[float]] = {}
    for sample_match in SAMPLES.finditer(output):
        samples[sample_match.group("op")] = [
            float(value.strip()) for value in sample_match.group(2).split(",")
        ]
    assert set(samples) == {"synthesis", "analysis"}, (path, samples)
    assert all(len(values) == 5 for values in samples.values()), (path, samples)
    timing = path.with_suffix(".time").read_text()
    rss_match = RSS.search(timing)
    exit_match = EXIT.search(timing)
    assert rss_match and exit_match and exit_match.group(1) == "0", (path, timing)
    return {
        **metadata,
        "samples": samples,
        "rss_kib": int(rss_match.group(1)),
        "config": config,
    }


processes = {}
for stdout in HERE.glob("t*-s*-q*-pair*-*.stdout"):
    process = read_process(stdout)
    key = (
        process["lmax"],
        process["spin"],
        process["threads"],
        process["pair"],
        process["variant"],
    )
    assert key not in processes, key
    processes[key] = process

cells = sorted({key[:3] for key in processes})
assert set(cells) == EXPECTED_CELLS, cells

process_rows = []
summary_rows = []
rng = random.Random(BOOTSTRAP_SEED)
for cell in cells:
    lmax, spin, threads = cell
    pair_ids = sorted({key[3] for key in processes if key[:3] == cell})
    assert pair_ids == [1, 2, 3, 4, 5, 6], (cell, pair_ids)
    for operation in ("synthesis", "analysis"):
        ratios = []
        base_medians = []
        ordered_medians = []
        base_samples = []
        ordered_samples = []
        rss_deltas = []
        for pair in pair_ids:
            baseline = processes[(*cell, pair, "baseline")]
            ordered = processes[(*cell, pair, "ordered")]
            base_values = baseline["samples"][operation]
            ordered_values = ordered["samples"][operation]
            base_median = statistics.median(base_values)
            ordered_median = statistics.median(ordered_values)
            ratio = ordered_median / base_median
            ratios.append(ratio)
            base_medians.append(base_median)
            ordered_medians.append(ordered_median)
            base_samples.extend(base_values)
            ordered_samples.extend(ordered_values)
            rss_delta = ordered["rss_kib"] - baseline["rss_kib"]
            rss_deltas.append(rss_delta)
            process_rows.append(
                {
                    "lmax": lmax,
                    "spin": spin,
                    "threads": threads,
                    "operation": operation,
                    "pair": pair,
                    "baseline_first": "true" if pair % 2 == 1 else "false",
                    "baseline_samples_ms": ";".join(f"{v:.3f}" for v in base_values),
                    "ordered_samples_ms": ";".join(f"{v:.3f}" for v in ordered_values),
                    "baseline_median_ms": f"{base_median:.6f}",
                    "ordered_median_ms": f"{ordered_median:.6f}",
                    "ordered_over_baseline": f"{ratio:.8f}",
                    "baseline_rss_kib": baseline["rss_kib"],
                    "ordered_rss_kib": ordered["rss_kib"],
                    "rss_delta_kib": rss_delta,
                }
            )
        boot = []
        for _ in range(BOOTSTRAP_REPLICATES):
            boot.append(statistics.median(rng.choice(ratios) for _ in ratios))
        summary_rows.append(
            {
                "lmax": lmax,
                "spin": spin,
                "threads": threads,
                "operation": operation,
                "n_pairs": len(ratios),
                "baseline_median_ms": f"{statistics.median(base_medians):.6f}",
                "ordered_median_ms": f"{statistics.median(ordered_medians):.6f}",
                "median_paired_ratio": f"{statistics.median(ratios):.8f}",
                "bootstrap95_low": f"{linear_quantile(boot, 0.025):.8f}",
                "bootstrap95_high": f"{linear_quantile(boot, 0.975):.8f}",
                "paired_ratio_min": f"{min(ratios):.8f}",
                "paired_ratio_max": f"{max(ratios):.8f}",
                "baseline_sample_min_ms": f"{min(base_samples):.3f}",
                "baseline_sample_max_ms": f"{max(base_samples):.3f}",
                "ordered_sample_min_ms": f"{min(ordered_samples):.3f}",
                "ordered_sample_max_ms": f"{max(ordered_samples):.3f}",
                "median_baseline_rss_kib": int(
                    statistics.median(
                        processes[(*cell, pair, "baseline")]["rss_kib"]
                        for pair in pair_ids
                    )
                ),
                "median_ordered_rss_kib": int(
                    statistics.median(
                        processes[(*cell, pair, "ordered")]["rss_kib"]
                        for pair in pair_ids
                    )
                ),
                "median_paired_rss_delta_kib": int(statistics.median(rss_deltas)),
            }
        )


def write_csv(path: Path, rows: list[dict]) -> None:
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


write_csv(HERE / "summary.csv", summary_rows)
write_csv(HERE / "process-pairs.csv", process_rows)
print(f"wrote {len(summary_rows)} operation summaries and {len(process_rows)} paired rows")
