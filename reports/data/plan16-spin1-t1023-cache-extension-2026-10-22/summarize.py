#!/usr/bin/env python3
"""Emit sanitized Plan 16 T1023 cache A/B and stage summaries.

Raw native benchmark JSON and instrumented logs stay local because they contain
host scheduling/frequency diagnostics. Invoke with three comma-separated grid
inputs (for example GL=gl.json) and profile logs as positional options.
"""
from __future__ import annotations
import argparse
import csv
import json
import pathlib
import statistics


def median(values):
    values = [v for v in values if v is not None]
    return statistics.median(values) if values else None


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--gl", type=pathlib.Path, required=True)
    p.add_argument("--gl-reduced", type=pathlib.Path, required=True,
                   help="controlled GL candidate retaining the 17 MiB budget")
    p.add_argument("--cc", type=pathlib.Path, required=True)
    p.add_argument("--f1", type=pathlib.Path, required=True)
    p.add_argument("--cc-profile", type=pathlib.Path, required=True)
    p.add_argument("--f1-profile", type=pathlib.Path, required=True)
    p.add_argument("--out", type=pathlib.Path, required=True)
    a = p.parse_args()
    docs = {grid: json.loads(getattr(a, grid.lower()).read_text()) for grid in ("GL", "CC", "F1")}
    reduced_doc = json.loads(a.gl_reduced.read_text())
    a.out.mkdir(parents=True, exist_ok=True)
    fields = ["grid", "lmax", "mmax", "ntheta", "nphi", "operation", "threads",
              "baseline_ms", "baseline_p10_ms", "baseline_p90_ms",
              "candidate_ms", "candidate_p10_ms", "candidate_p90_ms",
              "ducc_ms", "ducc_p10_ms", "ducc_p90_ms",
              "candidate_baseline_pair_ratios", "candidate_ducc_pair_ratios",
              "relative_l2_candidate_ducc", "relative_l2_candidate_baseline",
              "candidate_first_ms", "candidate_setup_ms", "baseline_first_ms", "baseline_setup_ms",
              "candidate_batches", "baseline_batches", "ducc_batches",
              "candidate_wait_fraction_median", "baseline_wait_fraction_median", "ducc_wait_fraction_median",
              "all_finite", "repeat_consistent", "reset_consistent"]
    with (a.out / "results.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        for grid, doc in docs.items():
            summaries = doc["summaries"]
            comparisons = doc["comparisons"]
            case_ops = sorted({(c["case_id"], c["operation"], c["threads"]) for c in comparisons})
            for case_id, operation, threads in case_ops:
                group = {s["backend"]: s for s in summaries if s["case_id"] == case_id and
                         s["operation"] == operation and s["threads"] == threads}
                cs = [c for c in comparisons if c["case_id"] == case_id and c["operation"] == operation and c["threads"] == threads]
                case = cs[0]
                base_pairs, cand_pairs, ducc_pairs = (group[b]["pair_medians_ms"] for b in ("baseline", "candidate", "ducc"))
                host_wait = {}
                for backend in ("baseline", "candidate", "ducc"):
                    host_wait[backend] = median([r["host_diagnostics"]["wait_fraction"] for r in doc["raw"]
                                                 if r["case_id"] == case_id and r["operation"] == operation and
                                                 r["threads"] == threads and r["backend"] == backend])
                row = {k: None for k in fields}
                row.update({"grid": grid, "lmax": case["lmax"], "mmax": case["mmax"],
                            "ntheta": case["ntheta"], "nphi": case["nphi"], "operation": operation, "threads": threads})
                for backend in ("baseline", "candidate", "ducc"):
                    summary = group[backend]
                    row[f"{backend}_ms"] = summary["latency"]["median"]
                    row[f"{backend}_p10_ms"] = summary["latency"]["p10"]
                    row[f"{backend}_p90_ms"] = summary["latency"]["p90"]
                    row[f"{backend}_batches"] = summary["latency"]["count"]
                row["candidate_baseline_pair_ratios"] = ";".join(f"{c/b:.6f}" for c, b in zip(cand_pairs, base_pairs))
                row["candidate_ducc_pair_ratios"] = ";".join(f"{c/d:.6f}" for c, d in zip(cand_pairs, ducc_pairs))
                row["relative_l2_candidate_ducc"] = max(c["candidate_vs_ducc"]["relative_l2"] for c in cs)
                row["relative_l2_candidate_baseline"] = max(c["candidate_vs_baseline"]["relative_l2"] for c in cs)
                row["candidate_first_ms"] = median(group["candidate"]["first_call_ms"])
                row["candidate_setup_ms"] = median(group["candidate"]["setup_ms"])
                row["baseline_first_ms"] = median(group["baseline"]["first_call_ms"])
                row["baseline_setup_ms"] = median(group["baseline"]["setup_ms"])
                for backend in ("baseline", "candidate", "ducc"):
                    row[f"{backend}_wait_fraction_median"] = host_wait[backend]
                row["all_finite"] = all(c["all_finite"] for c in cs)
                row["repeat_consistent"] = all(all(c["repeat_consistent"].values()) for c in cs)
                row["reset_consistent"] = all(all(c["reset_consistent"].values()) for c in cs)
                writer.writerow(row)

    import re
    phase_rows = []
    for grid, path in (("CC", a.cc_profile), ("F1", a.f1_profile)):
        phases, totals = {}, []
        for line in path.read_text().splitlines():
            if "PROFILE_TOTAL," in line:
                tail = line.split("PROFILE_TOTAL,", 1)[1].split(",")
                totals.append(float(tail[-1]))
            elif "PROFILE_PHASE," in line:
                tail = line.split("PROFILE_PHASE,", 1)[1].split(",")
                name, ms = tail[-2], float(tail[-1])
                phases.setdefault(name, []).append(ms)
        phases["TOTAL transform"] = totals
        for name, values in phases.items():
            values.sort()
            lo = values[max(0, int(.1 * (len(values) - 1)))]
            hi = values[min(len(values) - 1, int(.9 * (len(values) - 1)))]
            phase_rows.append({"grid": grid, "lmax": 1023, "mmax": 1023, "threads": 8,
                               "phase": name, "median_ms": median(values), "p10_ms": lo,
                               "p90_ms": hi, "n": len(values)})
    with (a.out / "analysis-phase-profile.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=["grid", "lmax", "mmax", "threads", "phase", "median_ms", "p10_ms", "p90_ms", "n"])
        writer.writeheader()
        writer.writerows(phase_rows)

    target_case = "GL-T1023-m255-n1024-exact-p512-even-smooth"
    def summary(doc, backend):
        return next(s for s in doc["summaries"] if s["case_id"] == target_case and
                    s["operation"] == "synthesis" and s["threads"] == 8 and
                    s["backend"] == backend)
    reduced_rows = []
    for variant, doc, backend in (("baseline 720/17 MiB", docs["GL"], "baseline"),
                                  ("candidate 1023/17 MiB", reduced_doc, "candidate"),
                                  ("candidate 1023/34 MiB", docs["GL"], "candidate"),
                                  ("pinned DUCC", docs["GL"], "ducc")):
        s = summary(doc, backend)
        reduced_rows.append({"variant": variant, "median_ms": s["latency"]["median"],
                             "p10_ms": s["latency"]["p10"], "p90_ms": s["latency"]["p90"],
                             "pair_medians_ms": ";".join(f"{v:.6f}" for v in s["pair_medians_ms"]),
                             "samples": s["latency"]["count"]})
    with (a.out / "reduced-cache-budget-ablation.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(reduced_rows[0]))
        writer.writeheader()
        writer.writerows(reduced_rows)

    route_rows = []
    for variant, grid, mmax, nbytes, coefficient_count, workers in [
        ("production baseline", "GL", 255, 0, 0, 0),
        ("production baseline", "GL", 1023, 0, 0, 0),
        ("candidate 17 MiB", "GL", 255, 8_396_800, 524_800, 8),
        ("candidate 17 MiB", "GL", 1023, 0, 0, 0),
        ("candidate 34 MiB", "GL", 255, 8_396_800, 524_800, 8),
        ("candidate 34 MiB", "GL", 1023, 33_587_200, 2_099_200, 8),
        ("candidate 34 MiB", "CC", 255, 8_396_800, 524_800, 8),
        ("candidate 34 MiB", "CC", 1023, 33_587_200, 2_099_200, 8),
        ("candidate 34 MiB", "F1", 255, 0, 0, 0),
        ("candidate 34 MiB", "F1", 1023, 0, 0, 0),
    ]:
        route_rows.append({"variant": variant, "grid": grid, "lmax": 1023, "mmax": mmax,
                           "coefficient_bytes": nbytes, "coefficient_complex_values": coefficient_count,
                           "order_major_scratch_workers": workers, "cache_active": nbytes > 0})
    with (a.out / "route-evidence.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(route_rows[0]))
        writer.writeheader()
        writer.writerows(route_rows)

    metadata = {
        "baseline_source": "79e28abb",
        "candidate_source_base": "79e28abb; only cache MAX_LMAX=1023 and MAX_BYTES=34 MiB changed",
        "ducc_revision": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "toolchain": "rustc 1.99.1 / LLVM 23.1.1; GCC 15.2.0",
        "rust_flags": "-C target-cpu=x86-64-v3 -Aunexpected_cfgs",
        "ducc_sht_flags": "-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3; driver -fno-fast-math",
        "baseline_binary_sha256": "efb7a5e6be9897aed7eef1ba4432b6628d174d89c07bca8e79d8bb6f3015a1db",
        "candidate_binary_sha256": "6a8e41ba2637cee9a1c68ded25e19c5be02f7e6ba9ae64a0abd464c3ffb85656",
        "ducc_binary_sha256": "e3908ae1ce2502b1a7b9c9ede1442d85d4f977efbb62adc39542482d2352f61d",
        "host": "AMD Ryzen 9 5950X; q1 CPU 0, q8 CPUs 0-7 within L3 0-7,16-23",
        "protocol": "three interleaved process pairs; calibrated batches targeting 25 ms; plans/workspaces reused; first-call/setup reported separately",
        "cases": "T1023 GL/CC/F1 exact latitude, full mmax=1023 and reduced mmax=255, q1/q8, analysis/synthesis",
        "source_variants": "baseline 720/17 MiB; reduced candidate 1023/17 MiB; final candidate 1023/34 MiB",
        "raw_host_logs": "not included; per-run wait/frequency details remain local; CSV contains only wait-fraction medians",
        "complete": all(len(d["raw"]) == 72 and len(d["comparisons"]) == 24 for d in docs.values()),
        "numerical_all_finite_repeat_reset": all(all(c["all_finite"] and all(c["repeat_consistent"].values()) and all(c["reset_consistent"].values()) for c in d["comparisons"]) for d in docs.values()),
    }
    (a.out / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps(metadata, indent=2))

if __name__ == "__main__":
    main()
