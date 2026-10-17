#!/usr/bin/env python3
"""Parse test-only scalar synthesis route profiles into worker and case tables."""
import csv
import re
import statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parent
COUNT_RE = re.compile(r"worker=(\d+) Counts \{([^}]*)\}")
TOTAL_RE = re.compile(
    r"M6 routes L=(\d+) q=(\d+) potential=(\d+) committed=(\d+) "
    r"disabled_ms=([0-9.]+) enabled_ms=([0-9.]+) Counts \{([^}]*)\}"
)


def parse_fields(text):
    return {
        key.strip(): int(value)
        for key, value in re.findall(r"(\w+): (\d+)", text)
    }


def parse_file(path):
    text = path.read_text()
    total_match = TOTAL_RE.search(text)
    if not total_match:
        raise ValueError(f"missing aggregate line: {path}")
    lmax, threads, potential, committed, disabled_ms, enabled_ms, total_text = total_match.groups()
    workers = []
    for match in COUNT_RE.finditer(text):
        worker = int(match.group(1))
        fields = parse_fields(match.group(2))
        fields.update({"worker": worker, "lmax": int(lmax), "threads": int(threads)})
        fields["active_ns"] = fields.get("fused_ns", 0) + fields.get("visitor_ns", 0) + fields.get("transition_ns", 0)
        fields["route_units"] = (
            fields.get("fused", 0)
            + fields.get("visitor_normal", 0)
            + fields.get("visitor_scaled", 0)
            + fields.get("transition_prefix", 0)
            + fields.get("transition_fused", 0)
        )
        workers.append(fields)
    aggregate = parse_fields(total_text)
    if not workers:
        raise ValueError(f"missing worker lines: {path}")
    return {
        "lmax": int(lmax),
        "threads": int(threads),
        "potential": int(potential),
        "committed": int(committed),
        "disabled_ms": float(disabled_ms),
        "enabled_ms": float(enabled_ms),
        "aggregate": aggregate,
        "workers": sorted(workers, key=lambda worker: worker["worker"]),
    }


def write_csv(path, rows):
    if not rows:
        return
    with path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


profiles = []
for path in sorted(ROOT.glob("m6-routes-final-t*-q*.log")):
    profile = parse_file(path)
    profile["source_log"] = path.name
    profiles.append(profile)
if len(profiles) != 6:
    raise SystemExit(f"expected six route logs, found {len(profiles)}")

worker_rows = []
case_rows = []
for profile in profiles:
    for worker in profile["workers"]:
        worker_rows.append({
            "lmax": profile["lmax"],
            "threads": profile["threads"],
            "worker": worker["worker"],
            "route_units": worker["route_units"],
            "active_route_ms_sum": f"{worker['active_ns'] / 1e6:.3f}",
            "fused_pairs": worker.get("fused", 0),
            "fused_ms_sum": f"{worker.get('fused_ns', 0) / 1e6:.3f}",
            "visitor_normal_lanes": worker.get("visitor_normal", 0),
            "visitor_scaled_lanes": worker.get("visitor_scaled", 0),
            "visitor_ms_sum": f"{worker.get('visitor_ns', 0) / 1e6:.3f}",
            "transition_prefix_lanes": worker.get("transition_prefix", 0),
            "transition_fused_pairs": worker.get("transition_fused", 0),
            "transition_ms_sum": f"{worker.get('transition_ns', 0) / 1e6:.3f}",
            "discarded_prefix_lanes": worker.get("discarded_prefix", 0),
            "discarded_fused_pairs": worker.get("discarded_fused", 0),
            "transition_calls": worker.get("transition_calls", 0),
            "completed_transitions": worker.get("completed_transitions", 0),
            "declined_transitions": worker.get("declined_transitions", 0),
            "preparation_failures": worker.get("preparation_failures", 0),
        })

    workers = profile["workers"]
    nonidle_ns = [worker["active_ns"] for worker in workers if worker["active_ns"] > 0]
    aggregate = profile["aggregate"]
    route_ns = {
        name: aggregate.get(f"{name}_ns", 0)
        for name in ("fused", "visitor", "transition", "coefficients", "factors")
    }
    case_rows.append({
        "lmax": profile["lmax"],
        "threads": profile["threads"],
        "potential_pairs": profile["potential"],
        "committed_route_units": profile["committed"],
        "committed_pct_of_potential": f"{100 * profile['committed'] / profile['potential']:.2f}",
        "disabled_ms": f"{profile['disabled_ms']:.3f}",
        "enabled_ms": f"{profile['enabled_ms']:.3f}",
        "enabled_minus_disabled_pct": f"{100 * (profile['enabled_ms'] / profile['disabled_ms'] - 1):.2f}",
        "fused_pairs": aggregate.get("fused", 0),
        "visitor_normal_lanes": aggregate.get("visitor_normal", 0),
        "visitor_scaled_lanes": aggregate.get("visitor_scaled", 0),
        "transition_prefix_lanes": aggregate.get("transition_prefix", 0),
        "transition_fused_pairs": aggregate.get("transition_fused", 0),
        "discarded_prefix_lanes": aggregate.get("discarded_prefix", 0),
        "discarded_fused_pairs": aggregate.get("discarded_fused", 0),
        "transition_calls": aggregate.get("transition_calls", 0),
        "completed_transitions": aggregate.get("completed_transitions", 0),
        "declined_transitions": aggregate.get("declined_transitions", 0),
        "preparation_failures": aggregate.get("preparation_failures", 0),
        "fused_cpu_ms_sum": f"{route_ns['fused'] / 1e6:.3f}",
        "visitor_cpu_ms_sum": f"{route_ns['visitor'] / 1e6:.3f}",
        "transition_cpu_ms_sum": f"{route_ns['transition'] / 1e6:.3f}",
        "coefficients_cpu_ms_sum": f"{route_ns['coefficients'] / 1e6:.3f}",
        "factors_cpu_ms_sum": f"{route_ns['factors'] / 1e6:.3f}",
        "active_worker_route_ms_min": f"{min(nonidle_ns) / 1e6:.3f}",
        "active_worker_route_ms_max": f"{max(nonidle_ns) / 1e6:.3f}",
        "active_worker_route_max_min": f"{max(nonidle_ns) / min(nonidle_ns):.3f}",
        "idle_workers": sum(worker["active_ns"] == 0 for worker in workers),
        "source_log": profile["source_log"],
    })

write_csv(ROOT / "workers.csv", worker_rows)
write_csv(ROOT / "summary.csv", case_rows)
for row in case_rows:
    print(
        f"T{row['lmax']} q{row['threads']}: committed {row['committed_route_units']}/"
        f"{row['potential_pairs']} ({row['committed_pct_of_potential']}%); "
        f"transitions {row['completed_transitions']} complete/{row['declined_transitions']} declined; "
        f"route CPU sums F/V/T={row['fused_cpu_ms_sum']}/{row['visitor_cpu_ms_sum']}/"
        f"{row['transition_cpu_ms_sum']} ms; worker active route max/min="
        f"{row['active_worker_route_max_min']}x, idle={row['idle_workers']}"
    )
