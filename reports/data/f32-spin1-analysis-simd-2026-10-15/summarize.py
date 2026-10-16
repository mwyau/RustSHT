#!/usr/bin/env python3
import re
from pathlib import Path
from statistics import median

root = Path(__file__).resolve().parent
patterns = {
    "synthesis_q1": "synthesis_one_thread_ms",
    "synthesis_q8": "synthesis_rayon_ms",
    "analysis_q1": "analysis_one_thread_ms",
    "analysis_q8": "analysis_rayon_ms",
}
records = []
for path in sorted(root.glob("*pair-*-*.log")):
    match = re.fullmatch(r"(repeat-)?pair-(\d+)-(baseline|candidate)\.log", path.name)
    if not match:
        continue
    screen = "repeat" if match.group(1) else "first"
    pair = int(match.group(2))
    side = match.group(3)
    values = {}
    for line in path.read_text().splitlines():
        if "_ms=" in line:
            key, value = line.split("=", 1)
            values[key] = float(value)
    time_text = path.with_suffix(".time").read_text()
    rss = int(re.search(r"max_rss_kb=(\d+)", time_text).group(1))
    for key, source_key in patterns.items():
        records.append((screen, pair, side, key, values[source_key], rss))

lookup = {(screen, pair, side, key): (value, rss)
          for screen, pair, side, key, value, rss in records}
rows = []
for screen in ["first", "repeat"]:
    for pair in range(1, 6):
        for key in patterns:
            base, base_rss = lookup[(screen, pair, "baseline", key)]
            cand, cand_rss = lookup[(screen, pair, "candidate", key)]
            rows.append((screen, pair, key, base, cand, (cand / base - 1.0) * 100.0, base_rss, cand_rss))

out = root / "paired-results.tsv"
with out.open("w") as stream:
    stream.write("screen\tpair\tcase\tbaseline_ms\tcandidate_ms\tchange_pct\tbaseline_rss_kb\tcandidate_rss_kb\n")
    for row in rows:
        stream.write("\t".join([str(row[0]), str(row[1]), row[2], *(f"{v:.3f}" for v in row[3:6]), str(row[6]), str(row[7])]) + "\n")

for screen in ["first", "repeat"]:
    print(f"[{screen}]")
    for key in patterns:
        case = [row for row in rows if row[0] == screen and row[2] == key]
        base = [row[3] for row in case]
        cand = [row[4] for row in case]
        changes = [row[5] for row in case]
        print(f"{key}: baseline median={median(base):.3f} ms; candidate median={median(cand):.3f} ms; paired change median={median(changes):+.2f}% range={min(changes):+.2f}%..{max(changes):+.2f}%")
    rss = [lookup[(screen, pair, side, "analysis_q1")][1]
           for pair in range(1, 6) for side in ("baseline", "candidate")]
    print(f"process peak RSS median={median(rss):.0f} KiB range={min(rss)}..{max(rss)} KiB")
