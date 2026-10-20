#!/usr/bin/env python3
"""Compare matched scalar-table and T64-recurrence benchmark sweeps."""

from __future__ import annotations

import argparse
import csv
import json
import math
import pathlib
import re
import statistics


ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_BASELINE = ROOT / "reports/data/scalar-t64-recurrence-transition-2026-10-20/baseline"
DEFAULT_CANDIDATE = ROOT / "reports/data/scalar-t64-recurrence-transition-2026-10-20/candidate"
DEFAULT_OUTPUT = ROOT / "reports/data/scalar-t64-recurrence-transition-2026-10-20/comparison"
REPORT = ROOT / "reports/scalar-t64-recurrence-transition-2026-10-20.md"
DATA_ROOT = ROOT / "reports/data/scalar-t64-recurrence-transition-2026-10-20"
FOLLOWUP_SETS = (
    ("T63/T64 and T255/T256", DATA_ROOT / "baseline-boundary-check",
     DATA_ROOT / "candidate-boundary-check"),
    ("T127/T128", DATA_ROOT / "baseline-127-128-check",
     DATA_ROOT / "candidate-127-128-check"),
    ("T191/T223", DATA_ROOT / "baseline-191-223-check",
     DATA_ROOT / "candidate-191-223-check"),
)
FINAL_POLICY_DATA = DATA_ROOT / "final-production-policy"
DUCC_PIN = "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b"
BASELINE_HARNESS = "d68d8e48e982bc1e0ef347e95c744a703cbadf70"
PLOT_HARNESS = "949c1adff92c3837df8e763f27ec78d8e13f4e39"
BANDS = ((1, 32, "T1–T32"), (40, 63, "T40–T63"),
         (64, 127, "T64–T127"), (128, 255, "T128–T255"),
         (256, 256, "T256"))


def load_dataset(path: pathlib.Path) -> dict:
    data = json.loads((path / "results.json").read_text())
    if data["metadata"]["ducc_source_revision"] != DUCC_PIN:
        raise ValueError(f"{path}: unexpected DUCC revision")
    return data


def row_key(row: dict) -> tuple[str, str, int]:
    return row["case_id"], row["operation"], int(row["threads"])


def check_compatible(base: dict, candidate: dict) -> None:
    a, b = base["metadata"], candidate["metadata"]
    if a["ducc_source_revision"] != b["ducc_source_revision"]:
        raise ValueError("baseline and candidate use different DUCC revisions")
    if a["rustc"] != b["rustc"] or a["rust_flags"] != b["rust_flags"]:
        raise ValueError("baseline and candidate use different Rust toolchains or flags")
    if a["cpu_model"] != b["cpu_model"] or a["cpu_features"] != b["cpu_features"]:
        raise ValueError("baseline and candidate report different CPUs or feature sets")
    case_fields = ("case_id", "lmax", "mmax", "ntheta", "nphi", "longitude_mode", "input_sha256")
    left = [{key: case[key] for key in case_fields} for case in base["cases"]]
    right = [{key: case[key] for key in case_fields} for case in candidate["cases"]]
    if left != right:
        raise ValueError("baseline and candidate cases or deterministic input hashes differ")
    base_rows = {row_key(row): row for row in base["rows"]}
    candidate_rows = {row_key(row): row for row in candidate["rows"]}
    if base_rows.keys() != candidate_rows.keys():
        raise ValueError("baseline and candidate row keys differ")


def matched_rows(base: dict, candidate: dict) -> list[dict]:
    check_compatible(base, candidate)
    base_rows = {row_key(row): row for row in base["rows"]}
    candidate_rows = {row_key(row): row for row in candidate["rows"]}
    output = []
    for key in sorted(base_rows, key=lambda item: (
        base_rows[item]["lmax"], base_rows[item]["case_id"], item[1], item[2]
    )):
        before, after = base_rows[key], candidate_rows[key]
        row = {name: before[name] for name in (
            "case_id", "lmax", "mmax", "ntheta", "nphi", "longitude_mode", "operation", "threads"
        )}
        for label, source in (("baseline", before), ("candidate", after)):
            for field in ("rust_warm_ms", "rust_p10_ms", "rust_p90_ms", "rust_ducc_ratio",
                          "ducc_warm_ms", "ducc_p10_ms", "ducc_p90_ms", "rust_cold_first_ms",
                          "ducc_cold_first_ms", "rust_setup_ms", "ducc_setup_ms", "relative_l2",
                          "absolute_max", "paired_ratio_p10", "paired_ratio_p90",
                          "batch_elapsed_median_ms", "20_100ms_batch_fraction_for_short_ops"):
                row[f"{label}_{field}"] = source[field]
        row["candidate_over_baseline_rust"] = after["rust_warm_ms"] / before["rust_warm_ms"]
        row["warm_latency_change_percent"] = 100.0 * (row["candidate_over_baseline_rust"] - 1.0)
        row["candidate_rust_over_ducc"] = after["rust_ducc_ratio"]
        row["baseline_rust_over_ducc"] = before["rust_ducc_ratio"]
        output.append(row)
    return output


def write_comparison_csv(rows: list[dict], path: pathlib.Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def compiler_version(text: str) -> str:
    match = re.search(r"\) ([0-9]+(?:\.[0-9]+)+)$", text)
    return match.group(1) if match else text


def load_followups() -> tuple[list[dict], dict]:
    followups = []
    for label, baseline_path, candidate_path in FOLLOWUP_SETS:
        baseline = load_dataset(baseline_path)
        candidate = load_dataset(candidate_path)
        check_compatible(baseline, candidate)
        if compiler_version(baseline["metadata"]["cxx"]) != compiler_version(candidate["metadata"]["cxx"]):
            raise ValueError(f"{label}: baseline and candidate use different C++ compiler versions")
        rows = matched_rows(baseline, candidate)
        for row in rows:
            row["followup_set"] = label
        followups.append({"label": label, "baseline": baseline,
                          "candidate": candidate, "rows": rows})
    final_policy = load_dataset(FINAL_POLICY_DATA)
    if final_policy["metadata"]["rust_source_revision"] != "d6382614d1726e28a8fc0ae1208d3d15516fac39":
        raise ValueError("final policy data were not collected from the committed synthesis-only policy")
    if not final_policy["metadata"]["rust_worktree_clean"]:
        raise ValueError("final policy data were collected from a dirty source worktree")
    return followups, final_policy


def focused_summary(rows: list[dict], degree: int, operation: str, threads: int) -> list[dict]:
    return [row for row in rows if row["lmax"] == degree and row["mmax"] == degree
            and row["operation"] == operation and row["threads"] == threads]


def add_policy_followup_section(lines: list[str], output: pathlib.Path,
                                followups: list[dict], final_policy: dict) -> None:
    combined = [row for followup in followups for row in followup["rows"]]
    if combined:
        write_comparison_csv(combined, output / "focused-boundary-comparison.csv")
    try:
        output_link = output.resolve().relative_to((ROOT / "reports").resolve()).as_posix()
    except ValueError:
        output_link = output.name
    lines += [
        "",
        "## Focused boundary reruns and policy decision",
        "",
        "The full sweep's baseline and global T64 candidate ran at different host loads. To check the route changes with lower, more similar load, three focused baseline/candidate reruns used the same inputs, GCC 15.2, pinned DUCC v3, q1/q8 and the standard interleaved 30-batch protocol. Each run collected three independent process pairs. Rust and DUCC were interleaved within each source run; baseline and candidate source runs were sequential, not interleaved across revisions.",
        "",
        "| rerun | baseline run-queue median/p90 | candidate run-queue median/p90 | baseline/candidate median CPU GHz | baseline/candidate numerical rows |",
        "|---|---:|---:|---:|---:|",
    ]
    for item in followups:
        b, c = item["baseline"], item["candidate"]
        bh, ch = b["host_summary"], c["host_summary"]
        lines.append(
            f"| {item['label']} | {bh['runqueue_wait_fraction_median']:.1%}/{bh['runqueue_wait_fraction_p90']:.1%} | {ch['runqueue_wait_fraction_median']:.1%}/{ch['runqueue_wait_fraction_p90']:.1%} | {bh['frequency_ghz_median']:.3f}/{ch['frequency_ghz_median']:.3f} | {len(b['rows'])}/{len(c['rows'])} |"
        )
    lines += [
        "",
        "The table below reports full-order GL cases, geometric means over odd-minimum and even-smooth longitude layouts. `candidate/base` compares Rust warm latency; the Rust/DUCC ratios are from each source run. The candidate here is the tested global T64 policy, which forced recurrence for both analysis and synthesis.",
        "",
        "| lmax | operation | q | n | candidate/base Rust | baseline Rust/DUCC | candidate Rust/DUCC |",
        "|---:|---|---:|---:|---:|---:|---:|",
    ]
    focused_degrees = (63, 64, 127, 128, 191, 223, 255, 256)
    for degree in focused_degrees:
        focus_rows = [row for item in followups for row in item["rows"]]
        for operation in ("analysis", "synthesis"):
            for threads in (1, 8):
                sample = focused_summary(focus_rows, degree, operation, threads)
                if not sample:
                    continue
                lines.append(
                    f"| {degree} | {operation} | {threads} | {len(sample)} | {geometric_mean([row['candidate_over_baseline_rust'] for row in sample]):.3f} | {geometric_mean([row['baseline_rust_over_ducc'] for row in sample]):.3f} | {geometric_mean([row['candidate_rust_over_ducc'] for row in sample]):.3f} |"
                )
    lines += [
        "",
        "For full-order q1 analysis, switching table-selected blocks to recurrence raised Rust latency by 28–33% at T64, 24% at T127, 24–31% at T128, and 14–16% at T191. At T223 it was neutral within these samples. At T128, synthesis recurrence reduced full-order Rust latency by 43–49% at q1 and 69–75% at q8; it also improved table-selected q8 synthesis at T64 and T127. At T255, recurrence improved full-order synthesis at q1/q8 and q8 analysis, while q1 analysis already selected recurrence under the original exception. These outcomes reject one global T64 threshold for both operations.",
        "",
        "The retained production policy is operation-specific: f64 scalar synthesis uses recurrence from T64; analysis keeps the prior table/reuse selection and its oversampled/T255 exceptions. f32, spin-1, other grids and public APIs are unchanged. The final policy was checked from a clean worktree at commit `d6382614` over 104 rows at T63/T64/T127/T128/T255/T256, with full/reduced mmax, both longitude samplings, q1/q8, 312 numerical comparisons and maximum relative L2 error 2.353e-14.",
        "",
        "| lmax | operation | q | n | final-policy Rust/DUCC GM | Rust warm ms GM | DUCC warm ms GM | max relative L2 |",
        "|---:|---|---:|---:|---:|---:|---:|---:|",
    ]
    for degree in (63, 64, 127, 128, 255, 256):
        for operation in ("analysis", "synthesis"):
            for threads in (1, 8):
                sample = [row for row in final_policy["rows"]
                          if row["lmax"] == degree and row["mmax"] == degree
                          and row["operation"] == operation and row["threads"] == threads]
                if not sample:
                    continue
                lines.append(
                    f"| {degree} | {operation} | {threads} | {len(sample)} | {geometric_mean([row['rust_ducc_ratio'] for row in sample]):.3f} | {geometric_mean([row['rust_warm_ms'] for row in sample]):.6g} | {geometric_mean([row['ducc_warm_ms'] for row in sample]):.6g} | {max(row['relative_l2'] for row in sample):.3e} |"
                )
    final_host = final_policy["host_summary"]
    lines += [
        "",
        f"The final-policy run's median/p90 run-queue wait was {final_host['runqueue_wait_fraction_median']:.1%}/{final_host['runqueue_wait_fraction_p90']:.1%}; median CPU frequency was {final_host['frequency_ghz_median']:.2f} GHz. Retain q8 ratios as provisional where scheduling pressure was high.",
        "",
        f"Full focused samples: [matched boundary CSV](./{output_link}/focused-boundary-comparison.csv), [T63/T64 and T255/T256 baseline rerun](./{DATA_ROOT.relative_to(ROOT / 'reports').as_posix()}/baseline-boundary-check/results.json), [T127/T128 rerun](./{DATA_ROOT.relative_to(ROOT / 'reports').as_posix()}/baseline-127-128-check/results.json), [T191/T223 rerun](./{DATA_ROOT.relative_to(ROOT / 'reports').as_posix()}/baseline-191-223-check/results.json), [final synthesis-only policy run](./{DATA_ROOT.relative_to(ROOT / 'reports').as_posix()}/final-production-policy/results.json). Candidate counterparts are stored beside each baseline dataset.",
    ]


def geometric_mean(values: list[float]) -> float:
    positive = [value for value in values if value > 0.0 and math.isfinite(value)]
    return math.exp(statistics.mean(math.log(value) for value in positive))


def summarize(rows: list[dict], operation: str, threads: int, lo: int, hi: int) -> dict:
    sample = [row for row in rows if row["operation"] == operation and row["threads"] == threads
              and lo <= row["lmax"] <= hi]
    return {
        "n": len(sample),
        "base_am": statistics.mean(row["baseline_rust_ducc_ratio"] for row in sample),
        "base_gm": geometric_mean([row["baseline_rust_ducc_ratio"] for row in sample]),
        "candidate_am": statistics.mean(row["candidate_rust_over_ducc"] for row in sample),
        "candidate_gm": geometric_mean([row["candidate_rust_over_ducc"] for row in sample]),
        "improvement_gm": geometric_mean([row["candidate_over_baseline_rust"] for row in sample]),
    }


def svg_text(x: float, y: float, text: str, **attrs: str) -> str:
    escaped = (text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;"))
    extra = " ".join(f'{key.replace("_", "-")}="{value}"' for key, value in attrs.items())
    return f'<text x="{x:.1f}" y="{y:.1f}" {extra}>{escaped}</text>'


def write_chart(path: pathlib.Path, title: str, operation: str, base_rows: list[dict],
                candidate_rows: list[dict], metric: str, unit: str) -> None:
    width, height = 1580, 850
    left, right, top, bottom = 112, 45, 100, 220
    plot_w, plot_h = width - left - right, height - top - bottom
    xmax = max(int(row["lmax"]) for row in candidate_rows)
    xlogmax = math.log1p(xmax)
    xcoord = lambda degree: left + math.log1p(degree) / xlogmax * plot_w

    is_ratio = metric == "ratio"
    if is_ratio:
        values = [float(row["rust_ducc_ratio"]) for rows in (base_rows, candidate_rows)
                  for row in rows if row["operation"] == operation and row["threads"] in (1, 8)
                  and row["lmax"] > 0 and row["mmax"] == row["lmax"]
                  and row["longitude_mode"] == "even_smooth"]
        ymin = max(0.2, min(values) * 0.88)
        ymax = max(5.0, max(values) * 1.12)
    else:
        values = []
        for rows in (base_rows, candidate_rows):
            for row in rows:
                if (row["operation"] == operation and row["threads"] in (1, 8)
                        and row["lmax"] > 0 and row["mmax"] == row["lmax"]
                        and row["longitude_mode"] == "even_smooth"):
                    values.extend((float(row["rust_warm_ms"]), float(row["ducc_warm_ms"])))
        ymin = min(values) * 0.7
        ymax = max(values) * 1.4
    logmin, logmax = math.log(ymin), math.log(ymax)
    ycoord = lambda value: top + (logmax - math.log(value)) / (logmax - logmin) * plot_h

    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
             '<rect width="100%" height="100%" fill="#fff"/>',
             svg_text(left, 34, title, font_family="sans-serif", font_size="26", font_weight="bold"),
             svg_text(left, 63, "f64 · GL · full mmax · even-smooth nphi · warmed calls · q1/q8 · lower is faster",
                      font_family="sans-serif", font_size="15", fill="#4f5c65"),
             svg_text(xcoord(7), top + 18, "exhaustive T0–T32", font_family="sans-serif", font_size="13", fill="#586774"),
             svg_text(xcoord(45), top + 18, "sampled degrees from T40", font_family="sans-serif", font_size="13", fill="#586774")]
    for decade in range(math.floor(logmin / math.log(10)), math.ceil(logmax / math.log(10)) + 1):
        for scale in (1, 2, 5):
            value = scale * (10 ** decade)
            if not ymin <= value <= ymax:
                continue
            y = ycoord(value)
            parts.append(f'<line x1="{left}" y1="{y:.1f}" x2="{width-right}" y2="{y:.1f}" stroke="#e0e5e9"/>')
            parts.append(svg_text(left - 12, y + 4, f"{value:g}", text_anchor="end", font_family="sans-serif", font_size="12"))
    if is_ratio and ymin < 1 < ymax:
        y = ycoord(1)
        parts.append(f'<line x1="{left}" y1="{y:.1f}" x2="{width-right}" y2="{y:.1f}" stroke="#53616c" stroke-dasharray="7 4" stroke-width="1.5"/>')
    parts.append(f'<line x1="{left}" y1="{top}" x2="{left}" y2="{height-bottom}" stroke="#34404a"/>')
    parts.append(f'<line x1="{left}" y1="{height-bottom}" x2="{width-right}" y2="{height-bottom}" stroke="#34404a"/>')

    # Shading and labels make the change from exhaustive low-degree points to
    # the sampled bandlimit sweep visible without implying interpolation.
    x32 = xcoord(32)
    parts.append(f'<line x1="{x32:.1f}" y1="{top}" x2="{x32:.1f}" y2="{height-bottom}" stroke="#83909a" stroke-dasharray="4 4"/>')
    boundary_styles = ((63.5, "T63 | T64", "#916b24", 18),
                       (255.5, "T255 | T256", "#8f426c", 36))
    for degree, label, color, dy in boundary_styles:
        x = (xcoord(degree) if degree.is_integer() else (xcoord(math.floor(degree)) + xcoord(math.ceil(degree))) / 2)
        parts.append(f'<line x1="{x:.1f}" y1="{top}" x2="{x:.1f}" y2="{height-bottom}" stroke="{color}" stroke-dasharray="5 4"/>')
        anchor = "end" if x > width - right - 100 else "start"
        label_x = x - 9 if anchor == "end" else x + 9
        parts.append(svg_text(label_x, top + dy, label, text_anchor=anchor,
                              font_family="sans-serif", font_size="12", fill=color))

    degree_set = {int(row["lmax"]) for row in candidate_rows if row["lmax"] <= 32}
    degree_set.update({40, 48, 56, 62, 63, 64, 65, 72, 80, 96, 112, 126, 127, 128, 129,
                       144, 159, 176, 191, 208, 223, 240, 248, 254, 255, 256})
    degree_set = sorted(degree for degree in degree_set if degree <= xmax)
    for index, degree in enumerate(degree_set):
        x = xcoord(degree)
        parts.append(f'<line x1="{x:.1f}" y1="{height-bottom}" x2="{x:.1f}" y2="{height-bottom+5}" stroke="#7d8790"/>')
        parts.append(f'<text transform="translate({x:.1f} {height-bottom+64}) rotate(-67)" text-anchor="end" font-family="sans-serif" font-size="10" fill="#394651">{degree}</text>')

    colors = {"base_q1": "#155e99", "candidate_q1": "#18845b", "base_q8": "#bd3f48",
              "candidate_q8": "#dd7d20", "ducc_q1": "#35434d", "ducc_q8": "#777f85"}
    series = []
    for threads in (1, 8):
        for label, policy_key, rows in (("Original policy", "base", base_rows),
                                        ("T64 recurrence", "candidate", candidate_rows)):
            selected = [row for row in rows if row["operation"] == operation and row["threads"] == threads
                        and row["lmax"] > 0 and row["mmax"] == row["lmax"]
                        and row["longitude_mode"] == "even_smooth"]
            series.append((f"{label} q{threads}", f"{policy_key}_q{threads}",
                           selected, "rust_ducc_ratio" if is_ratio else "rust_warm_ms"))
    if not is_ratio:
        for threads in (1, 8):
            before = [row for row in base_rows if row["operation"] == operation and row["threads"] == threads
                      and row["lmax"] > 0 and row["mmax"] == row["lmax"]
                      and row["longitude_mode"] == "even_smooth"]
            after = [row for row in candidate_rows if row["operation"] == operation and row["threads"] == threads
                     and row["lmax"] > 0 and row["mmax"] == row["lmax"]
                     and row["longitude_mode"] == "even_smooth"]
            joined = {row["case_id"]: row for row in after}
            ducc_geo = []
            for row in before:
                other = joined[row["case_id"]]
                ducc_geo.append({**row, "ducc_shared_ms": math.sqrt(row["ducc_warm_ms"] * other["ducc_warm_ms"])})
            series.append((f"DUCC q{threads} (geomean of paired runs)", f"ducc_q{threads}",
                           ducc_geo, "ducc_shared_ms"))

    for name, key, points, field in series:
        points = sorted(points, key=lambda row: row["lmax"])
        coords = [(xcoord(row["lmax"]), ycoord(row[field])) for row in points]
        if len(coords) > 1:
            path_data = " ".join(f"{x:.1f},{y:.1f}" for x, y in coords)
            dash = ' stroke-dasharray="7 4"' if "q8" in key else ""
            parts.append(f'<polyline fill="none" stroke="{colors[key]}" stroke-width="2.5"{dash} points="{path_data}"/>')
        marker = "triangle" if "candidate" in key else "circle"
        for x, y in coords:
            if marker == "triangle":
                points_data = f"{x:.1f},{y-4:.1f} {x-3.7:.1f},{y+3.3:.1f} {x+3.7:.1f},{y+3.3:.1f}"
                parts.append(f'<polygon points="{points_data}" fill="{colors[key]}"/>')
            else:
                parts.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="2.6" fill="{colors[key]}"/>')
    for index, (name, key, _points, _field) in enumerate(series):
        legend_columns = 3
        column, row_number = index % legend_columns, index // legend_columns
        x = left + column * 465
        y = height - 86 + row_number * 23
        dash = ' stroke-dasharray="7 4"' if "q8" in key else ""
        parts.append(f'<line x1="{x}" y1="{y}" x2="{x+27}" y2="{y}" stroke="{colors[key]}" stroke-width="3"{dash}/>')
        parts.append(svg_text(x + 35, y + 4, name, font_family="sans-serif", font_size="12", fill="#24333f"))
    label = "RustSHT / DUCC runtime (log scale)" if is_ratio else "Warmed transform latency (ms, log scale)"
    parts.append(svg_text(left + plot_w / 2, height - 8, "lmax (log(1 + lmax) spacing; sampled degrees shown)",
                          text_anchor="middle", font_family="sans-serif", font_size="14"))
    parts.append(f'<text transform="translate(22 {top+plot_h/2:.1f}) rotate(-90)" text-anchor="middle" font-family="sans-serif" font-size="14">{label}</text>')
    parts.append("</svg>")
    path.write_text("\n".join(parts) + "\n")


def near_row(rows: list[dict], degree: int, operation: str, threads: int) -> dict | None:
    return next((row for row in rows if row["lmax"] == degree and row["mmax"] == degree
                 and row["longitude_mode"] == "even_smooth" and row["operation"] == operation
                 and row["threads"] == threads), None)


def pct(value: float) -> str:
    return f"{value:.1f}%"


def write_report(base: dict, candidate: dict, rows: list[dict], output: pathlib.Path,
                 report_path: pathlib.Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    write_comparison_csv(rows, output / "matched-comparison.csv")
    followups, final_policy = load_followups()
    for operation in ("analysis", "synthesis"):
        base_rows = base["rows"]
        candidate_rows = candidate["rows"]
        write_chart(output / f"ratio-{operation}.svg", f"Scalar {operation}: RustSHT / pinned DUCC",
                    operation, base_rows, candidate_rows, "ratio", "×")
        write_chart(output / f"runtime-{operation}.svg", f"Scalar {operation}: absolute runtime",
                    operation, base_rows, candidate_rows, "runtime", "ms")

    lines = [
        "# Scalar T64 recurrence dispatch experiment",
        "",
        "## Scope and source versions",
        "",
        f"Baseline benchmark checkout HEAD: `{base['metadata']['rust_source_revision']}` (scalar production source last changed at `980e8e4f`); candidate source commit: `{candidate['metadata']['rust_source_revision']}`.",
        f"Benchmark case/timing harness: `{BASELINE_HARNESS}`; plot-only x-axis adjustment: `{PLOT_HARNESS}` (no timing or case-selection code changed there).",
        f"Rust: `{candidate['metadata']['rustc']}` (rustc commit `58cf8f9c3`, LLVM 23.1.1) with `{candidate['metadata']['rust_flags']}`; runtime dispatch: Pulp V3 (four f64 lanes, AVX2/FMA).",
        f"Pinned DUCC: `{candidate['metadata']['ducc_source_revision']}` ({candidate['metadata']['ducc_version']}); `{candidate['metadata']['ducc_flags']}`.",
        f"Host: {candidate['metadata']['cpu_model']}; {candidate['metadata']['physical_cores_visible']} visible physical cores; CPU features {', '.join(candidate['metadata']['cpu_features'])}.",
        "",
        f"Both sweeps contain {len(base['cases'])} configurations at {len({case['lmax'] for case in base['cases']})} degrees, with {len(rows)} matched operation/thread rows. Every deterministic map and coefficient-array SHA-256 matches. Each row has 30 timing batches over three interleaved process pairs per backend. T0 remains in the machine results and numerical coverage, but is excluded from throughput summaries and charts.",
        "",
        "Baseline and candidate used separate runs of the same immutable DUCC v3 build; each Rust/DUCC ratio uses its DUCC timing from the corresponding run. Absolute runtime charts show the geometric mean of the two DUCC measurements as a shared reference. No Rust 1.99.0 or older-DUCC timings are mixed into this comparison.",
        "",
        "## Dispatch behavior",
        "",
        "This comparison first tests a global f64 spin-0 scalar GL rule: recurrence at T64 and above for both analysis and synthesis. That experimental rule is not the final production policy. f32 did not support the compact scalar table and is unchanged. Spin-1 retains its separate table selector and reuse policy.",
        "",
        "For full-order, exact-sampling GL, finite moderate inputs, default reuse, the pre-change route matrix is:",
        "",
        "| lmax | q1 S | q1 A | q8 S | q8 A |",
        "|---:|---|---|---|---|",
        "| 63 | recurrence | table | table | recurrence |",
        "| 64 | recurrence | table | table | table |",
        "| 127 | recurrence | table | table | table |",
        "| 128 | table | table | table | table |",
        "| 255 | table | recurrence | table | table |",
        "| 256 | recurrence | recurrence | recurrence | recurrence |",
        "",
        "The global-cutoff candidate keeps the T63 row and selects recurrence for all four T64 cells and every higher spin-0 bandlimit. Reduced mmax, over-resolved geometries, non-default reuse estimates, and unsafe table inputs can select recurrence below T64 under the baseline exceptions.",
        "",
        "## Warm Rust/DUCC latency ratios and Rust-only change",
        "",
        "Arithmetic/geometric means below are computed over matched configuration rows in the indicated lmax band, excluding T0. `candidate/base` is the geometric mean of the candidate-to-baseline Rust warm latency; values below 1 indicate candidate improvement. Counts are matched rows, not transform repetitions.",
        "",
        "| lmax band | operation | q | n | baseline ratio AM/GM | candidate ratio AM/GM | candidate/base GM |",
        "|---|---|---:|---:|---:|---:|---:|",
    ]
    for lo, hi, band_name in BANDS:
        for operation in ("analysis", "synthesis"):
            for threads in (1, 8):
                summary = summarize(rows, operation, threads, lo, hi)
                lines.append(f"| {band_name} | {operation} | {threads} | {summary['n']} | {summary['base_am']:.3f}/{summary['base_gm']:.3f} | {summary['candidate_am']:.3f}/{summary['candidate_gm']:.3f} | {summary['improvement_gm']:.3f} |")
    lines += ["", "### Longitude and order coverage", "",
              "These geometric-mean summaries separate the two longitude samplings and full versus quarter-order truncation; the matched CSV contains every configuration, including the additional mmax=1 cases.",
              "",
              "| selection | op | q | n | baseline Rust/DUCC GM | candidate Rust/DUCC GM | candidate/base Rust GM |",
              "|---|---|---:|---:|---:|---:|---:|"]
    for mode in ("odd_minimum", "even_smooth"):
        for operation in ("analysis", "synthesis"):
            for threads in (1, 8):
                sample = [row for row in rows if row["lmax"] > 0 and row["longitude_mode"] == mode
                          and row["operation"] == operation and row["threads"] == threads]
                lines.append(f"| {mode} | {operation} | {threads} | {len(sample)} | {geometric_mean([row['baseline_rust_ducc_ratio'] for row in sample]):.3f} | {geometric_mean([row['candidate_rust_over_ducc'] for row in sample]):.3f} | {geometric_mean([row['candidate_over_baseline_rust'] for row in sample]):.3f} |")
    for order, predicate in (("full mmax", lambda row: row["mmax"] == row["lmax"]),
                             ("quarter mmax", lambda row: row["mmax"] == max(1, row["lmax"] // 4))):
        for operation in ("analysis", "synthesis"):
            for threads in (1, 8):
                sample = [row for row in rows if row["lmax"] > 0 and predicate(row)
                          and row["operation"] == operation and row["threads"] == threads]
                lines.append(f"| {order} | {operation} | {threads} | {len(sample)} | {geometric_mean([row['baseline_rust_ducc_ratio'] for row in sample]):.3f} | {geometric_mean([row['candidate_rust_over_ducc'] for row in sample]):.3f} | {geometric_mean([row['candidate_over_baseline_rust'] for row in sample]):.3f} |")
    lines += ["", "### T63/T64 and T255/T256 warmed and first-call latency", "",
              "Full mmax, even-smooth GL. First-call values are the runner's first native call after plan/workspace setup; setup remains separate.",
              "",
              "| lmax | op | q | baseline/candidate Rust warm ms | baseline/candidate DUCC warm ms | candidate/base Rust | Rust first-call baseline/candidate ms | DUCC first-call baseline/candidate ms | Rust setup baseline/candidate ms | DUCC setup baseline/candidate ms | baseline/candidate Rust/DUCC |",
              "|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|"]
    for degree in (63, 64, 255, 256):
        for operation in ("analysis", "synthesis"):
            for threads in (1, 8):
                base_row = near_row(base["rows"], degree, operation, threads)
                candidate_row = near_row(candidate["rows"], degree, operation, threads)
                if base_row is None or candidate_row is None:
                    continue
                lines.append(
                    f"| {degree} | {operation} | {threads} | {base_row['rust_warm_ms']:.6g}/{candidate_row['rust_warm_ms']:.6g} | {base_row['ducc_warm_ms']:.6g}/{candidate_row['ducc_warm_ms']:.6g} | {candidate_row['rust_warm_ms']/base_row['rust_warm_ms']:.3f} | {base_row['rust_cold_first_ms']:.6g}/{candidate_row['rust_cold_first_ms']:.6g} | {base_row['ducc_cold_first_ms']:.6g}/{candidate_row['ducc_cold_first_ms']:.6g} | {base_row['rust_setup_ms']:.6g}/{candidate_row['rust_setup_ms']:.6g} | {base_row['ducc_setup_ms']:.6g}/{candidate_row['ducc_setup_ms']:.6g} | {base_row['rust_ducc_ratio']:.3f}/{candidate_row['rust_ducc_ratio']:.3f} |"
                )
    lines += ["", "## Paired-case extrema and numerical agreement", "",
              f"Maximum warm relative L2 errors versus DUCC were {max(row['baseline_relative_l2'] for row in rows):.3e} before and {max(row['candidate_relative_l2'] for row in rows):.3e} after the policy change. Maximum absolute errors were {max(row['baseline_absolute_max'] for row in rows):.3e} and {max(row['candidate_absolute_max'] for row in rows):.3e}.",
              f"Across {len(base['validation'])} baseline and {len(candidate['validation'])} candidate configuration validation suites, finite-output, roundtrip, basis-mode and adjoint checks are retained in the source datasets. The candidate's worst sampled relative numerical row is `{max(rows, key=lambda row: row['candidate_relative_l2'])['case_id']}`.",
              "",
              "The largest Rust-only latency reductions and increases are listed for diagnosis; timing overlap is not treated as a meaningful difference.",
              "",
              "| operation | q | strongest reduction (case, candidate/base) | largest increase (case, candidate/base) |",
              "|---|---:|---|---|"]
    for operation in ("analysis", "synthesis"):
        for threads in (1, 8):
            subset = [row for row in rows if row["operation"] == operation and row["threads"] == threads and row["lmax"] > 0]
            best = min(subset, key=lambda row: row["candidate_over_baseline_rust"])
            worst = max(subset, key=lambda row: row["candidate_over_baseline_rust"])
            lines.append(f"| {operation} | {threads} | `{best['case_id']}` {best['candidate_over_baseline_rust']:.3f}× | `{worst['case_id']}` {worst['candidate_over_baseline_rust']:.3f}× |")
    try:
        output_link = output.resolve().relative_to((ROOT / "reports").resolve()).as_posix()
    except ValueError:
        output_link = output.name
    lines += ["", "## Host diagnostics and interpretation", "",
              "The baseline run's host summary is:", "",
              f"- Run-queue wait median/p90: {base['host_summary']['runqueue_wait_fraction_median']:.1%}/{base['host_summary']['runqueue_wait_fraction_p90']:.1%}; median CPU frequency: {base['host_summary']['frequency_ghz_median']:.2f} GHz.",
              f"- Candidate run-queue wait median/p90: {candidate['host_summary']['runqueue_wait_fraction_median']:.1%}/{candidate['host_summary']['runqueue_wait_fraction_p90']:.1%}; median CPU frequency: {candidate['host_summary']['frequency_ghz_median']:.2f} GHz.",
              f"- Median of the per-row fraction of short batches inside 20–100 ms: baseline {statistics.median(row['baseline_20_100ms_batch_fraction_for_short_ops'] for row in rows if row['baseline_20_100ms_batch_fraction_for_short_ops'] is not None):.1%}; candidate {statistics.median(row['candidate_20_100ms_batch_fraction_for_short_ops'] for row in rows if row['candidate_20_100ms_batch_fraction_for_short_ops'] is not None):.1%}.",
              "",
              "Treat isolated short-degree and q8 movements in view of these host summaries and paired p10–p90 intervals. The table compares complete native operations, including FFT and setup-independent output production; it does not attribute a change to the Legendre recurrence alone."]
    add_policy_followup_section(lines, output, followups, final_policy)
    lines += ["", "## Figures", "",
              f"- [Analysis Rust/DUCC ratio](./{output_link}/ratio-analysis.svg)",
              f"- [Synthesis Rust/DUCC ratio](./{output_link}/ratio-synthesis.svg)",
              f"- [Analysis absolute runtime](./{output_link}/runtime-analysis.svg)",
              f"- [Synthesis absolute runtime](./{output_link}/runtime-synthesis.svg)",
              "- Each sweep also contains regenerated issue-style ratio, runtime, speedup, full/reduced-mmax and cold/warm figures.",
              "",
              "## Reproduction", "",
              "```sh",
              "RESULT_ROOT=${RESULT_ROOT:-reports/data/scalar-t64-recurrence-transition-2026-10-20/reproduction}",
              "git worktree add --detach \"${BASELINE_TREE:?set baseline worktree}\" d68d8e48e982bc1e0ef347e95c744a703cbadf70",
              "(cd \"$BASELINE_TREE\" && python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src \"${DUCC_SRC:?set pinned DUCC checkout}\" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --max-lmax 256 --results-dir \"$RESULT_ROOT/baseline\" --no-report)",
              "git worktree add --detach \"${CANDIDATE_TREE:?set candidate worktree}\" 0ffcddda3f020d7c7144301de4320e127c5167b5",
              "(cd \"$CANDIDATE_TREE\" && python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src \"$DUCC_SRC\" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --max-lmax 256 --results-dir \"$RESULT_ROOT/candidate\" --no-report)",
              "git worktree add --detach \"${FINAL_TREE:?set final-policy worktree}\" d6382614d1726e28a8fc0ae1208d3d15516fac39",
              "(cd \"$FINAL_TREE\" && python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src \"$DUCC_SRC\" --ducc-revision 42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b --lmax 63 --lmax 64 --lmax 127 --lmax 128 --lmax 255 --lmax 256 --results-dir \"$RESULT_ROOT/final-production-policy\" --no-report)",
              "python3 scripts/compare_scalar_t64_policy.py --baseline \"$RESULT_ROOT/baseline\" --candidate \"$RESULT_ROOT/candidate\" --output \"$RESULT_ROOT/comparison\" --report \"$RESULT_ROOT/report.md\"",
              "```", "",
              "`matched-comparison.csv` contains all 848 matched case/operation/thread rows and the per-case baseline, candidate, DUCC, improvement, ratio, cold-start and error measurements. The source `results.json` files retain process samples, batch distributions and numerical-suite outputs.", ""]
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text("\n".join(lines))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=pathlib.Path, default=DEFAULT_BASELINE)
    parser.add_argument("--candidate", type=pathlib.Path, default=DEFAULT_CANDIDATE)
    parser.add_argument("--output", type=pathlib.Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--report", type=pathlib.Path, default=REPORT)
    args = parser.parse_args()
    baseline = load_dataset(args.baseline)
    candidate = load_dataset(args.candidate)
    rows = matched_rows(baseline, candidate)
    write_report(baseline, candidate, rows, args.output, args.report)
    print(f"Compared {len(rows)} matched rows; report={args.report}; figures/data={args.output}")


if __name__ == "__main__":
    main()
