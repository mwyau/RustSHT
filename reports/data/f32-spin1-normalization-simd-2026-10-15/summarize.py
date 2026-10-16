#!/usr/bin/env python3
import re
from pathlib import Path
from statistics import median

root = Path(__file__).resolve().parent
keys = {
    "synthesis_q1": "synthesis_one_thread_ms",
    "synthesis_q8": "synthesis_rayon_ms",
    "analysis_q1": "analysis_one_thread_ms",
    "analysis_q8": "analysis_rayon_ms",
}
data = {}
for path in root.glob("*pair-*-*.log"):
    match = re.fullmatch(r"(repeat-)?pair-(\d+)-(baseline|candidate)\.log", path.name)
    if not match:
        continue
    screen = "repeat" if match.group(1) else "first"
    pair, side = int(match.group(2)), match.group(3)
    values = {}
    for line in path.read_text().splitlines():
        if "_ms=" in line:
            key, value = line.split("=", 1)
            values[key] = float(value)
    rss = int(re.search(r"max_rss_kb=(\d+)", path.with_suffix(".time").read_text()).group(1))
    for key, source in keys.items():
        data[(screen, pair, side, key)] = (values[source], rss)

screens = sorted({key[0] for key in data})
rows = []
for screen in screens:
    pairs = sorted({key[1] for key in data if key[0] == screen})
    for pair in pairs:
        for key in keys:
            b, brss = data[(screen, pair, "baseline", key)]
            c, crss = data[(screen, pair, "candidate", key)]
            rows.append([screen, pair, key, b, c, (c / b - 1) * 100, brss, crss])
        for thread in ("q1", "q8"):
            s, a = f"synthesis_{thread}", f"analysis_{thread}"
            b = data[(screen, pair, "baseline", s)][0] + data[(screen, pair, "baseline", a)][0]
            c = data[(screen, pair, "candidate", s)][0] + data[(screen, pair, "candidate", a)][0]
            brss = data[(screen, pair, "baseline", a)][1]
            crss = data[(screen, pair, "candidate", a)][1]
            rows.append([screen, pair, f"combined_{thread}", b, c, (c / b - 1) * 100, brss, crss])

with (root / "paired-results.tsv").open("w") as stream:
    stream.write("screen\tpair\tcase\tbaseline_ms\tcandidate_ms\tchange_pct\tbaseline_rss_kb\tcandidate_rss_kb\n")
    for row in rows:
        stream.write("\t".join([str(row[0]), str(row[1]), row[2], *(f"{v:.3f}" for v in row[3:6]), str(row[6]), str(row[7])]) + "\n")

for screen in screens:
    print(f"[{screen}]")
    for key in [*keys, "combined_q1", "combined_q8"]:
        case = [row for row in rows if row[0] == screen and row[2] == key]
        baseline = [row[3] for row in case]
        candidate = [row[4] for row in case]
        changes = [row[5] for row in case]
        print(f"{key}: baseline median={median(baseline):.3f} ms; candidate median={median(candidate):.3f} ms; paired change median={median(changes):+.2f}% range={min(changes):+.2f}%..{max(changes):+.2f}%")
    for side in ("baseline", "candidate"):
        rss = [data[(screen, pair, side, "analysis_q1")][1]
               for pair in sorted({key[1] for key in data if key[0] == screen})]
        print(f"{side} process peak RSS median={median(rss):.0f} KiB range={min(rss)}..{max(rss)} KiB")
