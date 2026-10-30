#!/usr/bin/env python3
"""Replay the exact-f32 seed substitution on top of the saved plan-time cos^2 trace."""
from __future__ import annotations

import csv
import gzip
import io
import json
import sys
from decimal import Decimal
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PACKAGE / "tracers"))
import controlled_f32_replay as replay  # noqa: E402
import seed_only_replay as seed  # noqa: E402

D = Decimal
COS2_PACKAGE = PACKAGE.parent / "cos2-plan-time-control-2026-10-30"


def metrics(rows):
    errors = [row["error"] for row in rows]
    references = [abs(row["reference"]) for row in rows]
    return {
        "relative_l2": format(
            (sum(value * value for value in errors) /
             sum(value * value for value in references)).sqrt(), ".20g"
        ),
        "rms_absolute": format(
            (sum(value * value for value in errors) / len(errors)).sqrt(), ".20g"
        ),
        "max_absolute": format(max(errors), ".20g"),
    }


def outcomes(candidate, baseline):
    other = {(row["m"], row["ring"], row["component"]): row["error"] for row in baseline}
    counts = {"improved": 0, "worsened": 0, "unchanged": 0}
    for row in candidate:
        old = other[row["m"], row["ring"], row["component"]]
        counts["improved" if row["error"] < old else "worsened" if row["error"] > old else "unchanged"] += 1
    return counts


def write_gzip_csv(path: Path, rows):
    with path.open("wb") as raw_stream:
        with gzip.GzipFile(fileobj=raw_stream, mode="wb", filename="", mtime=0, compresslevel=9) as compressed:
            with io.TextIOWrapper(compressed, newline="") as stream:
                writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
                writer.writeheader()
                writer.writerows(rows)


def main():
    output_dir = Path(sys.argv[1]) if len(sys.argv) == 2 else PACKAGE / "data" / "dense-band"
    if len(sys.argv) > 2:
        raise SystemExit("usage: seed_cos2_interaction.py [output-directory]")
    output_dir.mkdir(parents=True, exist_ok=True)

    dense_dir = PACKAGE / "data" / "dense-band"
    cos2_trace = COS2_PACKAGE / "data" / "dense-high-band" / "rust-native-f32-dense-trace.csv.gz"
    original_rec, original_pairs, original_outputs = replay.load_rust(
        dense_dir / "rust-f32-trace.csv.gz"
    )
    cos2_rec, cos2_pairs, cos2_outputs = replay.load_rust(cos2_trace)
    quad_rec, _, quad_outputs = replay.load_quad(dense_dir / "quad128-reference.csv.gz")
    if original_pairs != cos2_pairs:
        raise AssertionError("cos^2 candidate changed prepared coefficient pairs")

    norms = seed.rust_f32_sectoral_norms(16)
    case_outputs = {name: [] for name in ("baseline", "seed_only", "cos2_only", "seed_plus_cos2")}
    recurrence_rows = []
    changed_csq_cases = changed_csq_rows = 0
    seed_changes = 0
    cases = [(m, ring) for m in range(1, 17) for ring in (0, 1, 5, 360)]
    pairs = {
        key: tuple(replay.f32d(bits) for bits in values)
        for key, values in original_pairs.items()
    }

    for m, ring in cases:
        key = (m, ring)
        old_rows, new_rows = original_rec[key], cos2_rec[key]
        old_output, new_output = original_outputs[key], cos2_outputs[key]
        if set(old_rows) != set(new_rows):
            raise AssertionError(f"recurrence row mismatch m={m}, ring={ring}")
        if old_output["sin"] != new_output["sin"] or old_output["cos"] != new_output["cos"]:
            raise AssertionError(f"cos^2 candidate changed sin/cos m={m}, ring={ring}")
        if old_rows[m + 2][4] != new_rows[m + 2][4]:
            raise AssertionError(f"cos^2 candidate changed actual seed m={m}, ring={ring}")
        if any(old_rows[degree][1:3] != new_rows[degree][1:3] for degree in old_rows):
            raise AssertionError(f"cos^2 candidate changed a/b m={m}, ring={ring}")

        changed = [degree for degree in old_rows if old_rows[degree][0] != new_rows[degree][0]]
        changed_csq_rows += len(changed)
        changed_csq_cases += bool(changed)

        actual_seed = old_rows[m + 2][4]
        corrected_seed = seed.exact_product_seed_bits(
            old_output["sin"], replay.f32bits(norms[m]), m
        )
        seed_changes += actual_seed != corrected_seed

        baseline_states, _ = replay.rust_operands_replay(old_rows, m, actual_seed)
        cos2_states, _ = replay.rust_operands_replay(new_rows, m, actual_seed)
        seed_states = seed.replay_seed_with_traced_operands(old_rows, m, corrected_seed)
        combined_states = seed.replay_seed_with_traced_operands(new_rows, m, corrected_seed)
        reference_states = replay.state_from_trace(quad_rec[key], m)
        for degree in sorted(reference_states):
            actual_cos2 = cos2_states[degree]
            corrected_cos2 = combined_states[degree]
            target_state = reference_states[degree]
            recurrence_rows.append({
                "m": m,
                "ring": ring,
                "degree": degree,
                "cos2_native_f32_state": format(actual_cos2, ".40g"),
                "seed_plus_cos2_f32_state": format(corrected_cos2, ".40g"),
                "binary128_state": format(target_state, ".40g"),
                "cos2_absolute_state_error": format(abs(actual_cos2 - target_state), ".17g"),
                "combined_absolute_state_error": format(abs(corrected_cos2 - target_state), ".17g"),
                "state_delta_from_seed": format(corrected_cos2 - actual_cos2, ".17g"),
            })

        qout = quad_outputs[key]["decimal"]
        reference = {
            "north_re": qout["nr"],
            "north_im": qout["ni"],
            "south_re": qout["sr"],
            "south_im": qout["eveni"] - qout["c"] * qout["oddi"],
        }
        cosine = replay.f32d(old_output["cos"])
        state_paths = {
            "baseline": baseline_states,
            "seed_only": seed_states,
            "cos2_only": cos2_states,
            "seed_plus_cos2": combined_states,
        }
        for path_name, states in state_paths.items():
            values = replay.accumulate(states, pairs, m, cosine)
            for component, target in reference.items():
                value = values[component]
                case_outputs[path_name].append({
                    "m": m,
                    "ring": ring,
                    "component": component,
                    "value": value,
                    "reference": target,
                    "error": abs(value - target),
                })

    if len(cases) != 64 or len(recurrence_rows) != 22_816:
        raise AssertionError(f"unexpected dense interaction coverage: {len(cases)}, {len(recurrence_rows)}")
    if changed_csq_cases != 32 or changed_csq_rows != 11_392:
        raise AssertionError(f"unexpected cos^2 substitutions: {changed_csq_cases}, {changed_csq_rows}")

    output_rows = []
    for index, baseline in enumerate(case_outputs["baseline"]):
        row = {"m": baseline["m"], "ring": baseline["ring"], "component": baseline["component"]}
        for path_name in case_outputs:
            result = case_outputs[path_name][index]
            row[f"{path_name}_value"] = format(result["value"], ".40g")
            row[f"{path_name}_absolute_error"] = format(result["error"], ".20g")
        row["binary128_reference"] = format(baseline["reference"], ".40g")
        output_rows.append(row)

    write_gzip_csv(output_dir / "seed-cos2-interaction.csv.gz", output_rows)
    write_gzip_csv(output_dir / "seed-cos2-recurrence-growth.csv.gz", recurrence_rows)
    cos2_manifest = json.loads((COS2_PACKAGE / "data" / "dense-high-band" / "source-manifest.json").read_text())
    all_metrics = {name: metrics(rows) for name, rows in case_outputs.items()}
    combined_vs_cos2 = {
        key: format(
            (D(all_metrics["seed_plus_cos2"][key]) / D(all_metrics["cos2_only"][key]) - 1) * 100,
            ".12g",
        )
        for key in ("relative_l2", "rms_absolute", "max_absolute")
    }
    summary = {
        "scope": "diagnostic-only pre-FFT replay; no production source or performance binary changed",
        "candidate_source_sha": "45380d7e4bd849a33ffef7d91978ee18fd7e4193",
        "ducc_reference_sha": "42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b",
        "cases": len(cases),
        "outputs": len(output_rows),
        "recurrence_updates": len(recurrence_rows) - len(cases),
        "cos2_candidate_trace_sha256": seed.sha256(cos2_trace),
        "cos2_candidate_patch_sha256_uncompressed": cos2_manifest["candidate_patch_sha256_uncompressed"],
        "original_rust_trace_sha256": seed.sha256(dense_dir / "rust-f32-trace.csv.gz"),
        "binary128_reference_sha256": seed.sha256(dense_dir / "quad128-reference.csv.gz"),
        "cos2_changed_cases": changed_csq_cases,
        "cos2_changed_degree_rows": changed_csq_rows,
        "seed_changed_cases": seed_changes,
        "substitutions": {
            "baseline": "original Rust f32 seed, f32 cos^2, f32 a/b/FMA recurrence",
            "seed_only": "exactly rounded f32 sine^m * f32 sectoral norm seed; original f32 cos^2 and a/b",
            "cos2_only": "saved plan-time f64 cos^2 narrowed to f32; original f32 seed and a/b",
            "seed_plus_cos2": "both substitutions above; original f32 a/b and FMA recurrence",
            "all_paths": "same Rust f32 coefficient pairs and cosine; 100-digit Decimal accumulation/reflection isolates recurrence-state changes",
        },
        "metrics": all_metrics,
        "seed_plus_cos2_metric_change_vs_cos2_only_percent": combined_vs_cos2,
        "outcomes_vs_baseline": {
            name: outcomes(case_outputs[name], case_outputs["baseline"])
            for name in ("seed_only", "cos2_only", "seed_plus_cos2")
        },
        "seed_plus_cos2_outcomes_vs_cos2_only": outcomes(
            case_outputs["seed_plus_cos2"], case_outputs["cos2_only"]
        ),
    }
    (output_dir / "seed-cos2-interaction-summary.json").write_text(
        json.dumps(summary, indent=2) + "\n"
    )
    print(json.dumps({"metrics": summary["metrics"], "outcomes_vs_baseline": summary["outcomes_vs_baseline"],
                      "seed_plus_cos2_outcomes_vs_cos2_only": summary["seed_plus_cos2_outcomes_vs_cos2_only"]}, indent=2))


if __name__ == "__main__":
    main()
