# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
"""Independent manifest checks for the native scalar parity runner."""

import importlib.util
import json
from pathlib import Path

import numpy as np
import pytest

from scripts import benchmark_ducc_full_transform_parity as benchmark


_AUDIT_SPEC = importlib.util.spec_from_file_location(
    "cc_audit_manifest",
    Path(__file__).resolve().parents[1]
    / "reports/data/ducc-scalar-f64-cc-full-sweep-2026-10-22/audit_manifest.py",
)
assert _AUDIT_SPEC is not None and _AUDIT_SPEC.loader is not None
cc_audit_manifest = importlib.util.module_from_spec(_AUDIT_SPEC)
_AUDIT_SPEC.loader.exec_module(cc_audit_manifest)


HIGH_LMAX = (
    257, 258, 300, 320, 384, 511, 512, 513, 640, 720, 721, 768,
    896, 1023, 1024, 1025, 1280, 1536, 1792, 2047, 2048, 2049, 2560,
    3072,
)
ODD_LMAX = frozenset((
    0, 1, 2, 3, 4, 8, 16, 32, 48, 62, 63, 64, 65, 127, 128, 159,
    191, 223, 248, 254, 255, 256, 257, 511, 512, 513, 720, 721,
    1023, 1024, 1025, 2047, 2048, 2049, 2560, 3072,
))


def independent_even_smooth_count(mmax: int) -> int:
    candidate = 2 * mmax + 2
    while True:
        remainder = candidate
        for factor in (2, 3, 5, 7):
            while remainder % factor == 0:
                remainder //= factor
        if remainder == 1:
            return candidate
        candidate += 2


def independent_cc_manifest() -> set[tuple[str, int, int, int, int, str]]:
    expected = set()
    for lmax in (*range(257), *HIGH_LMAX):
        for mmax in {lmax, lmax // 4}:
            modes = ["even_smooth"]
            if lmax in ODD_LMAX:
                modes.append("odd_minimum")
            for mode in modes:
                nphi = (
                    2 * mmax + 1
                    if mode == "odd_minimum"
                    else independent_even_smooth_count(mmax)
                )
                expected.add((
                    f"CC-T{lmax}-m{mmax}-{mode}",
                    lmax,
                    mmax,
                    lmax + 2,
                    nphi,
                    mode,
                ))
    return expected


SYNTHETIC_CC_CASE = {
    "case_id": "CC-T1-m0-even_smooth",
    "grid": "CC",
    "lmax": 1,
    "mmax": 0,
    "ntheta": 3,
    "nphi": 2,
    "longitude_mode": "even_smooth",
}


def synthetic_metric(relative_l2: float = 1e-14, count: int = 0) -> dict:
    return {
        "absolute_max": relative_l2,
        "reference_peak": 1.0,
        "relative_l2": relative_l2,
        "max_relative_above_1e-12_peak": relative_l2,
        "near_zero_reference_count": count,
        "metrics_finite": True,
    }


def synthetic_exact_zero_metric(count: int) -> dict:
    return {
        "absolute_max": 0.0,
        "reference_peak": 0.0,
        "relative_l2": 0.0,
        "max_relative_above_1e-12_peak": 0.0,
        "near_zero_reference_count": count,
        "metrics_finite": True,
    }


def accepted_validation() -> dict:
    nmap = SYNTHETIC_CC_CASE["ntheta"] * SYNTHETIC_CC_CASE["nphi"]
    nalm = 2
    basis = []
    for degree, order in benchmark.selected_modes(
        SYNTHETIC_CC_CASE["lmax"], SYNTHETIC_CC_CASE["mmax"]
    ):
        basis.append({
            "l": degree,
            "m": order,
            "map_rust_ducc": synthetic_metric(),
            "analysis_of_synthesis_rust": synthetic_metric(),
            "analysis_of_synthesis_ducc": synthetic_metric(),
        })
    smallest_subnormal = np.nextafter(0.0, 1.0)
    subnormal_metric = {
        "absolute_max": 3 * smallest_subnormal,
        "reference_peak": 0.0,
        "relative_l2": None,
        "max_relative_above_1e-12_peak": 0.0,
        "near_zero_reference_count": nmap,
        "metrics_finite": False,
    }
    return {
        **SYNTHETIC_CC_CASE,
        "finite_outputs": True,
        "finite_flags": {
            backend: {
                "zero_finite": True,
                "extreme_finite": True,
                "subnormal_finite": True,
            }
            for backend in ("rustsht", "ducc")
        },
        "reset_consistent": {"rustsht": True, "ducc": True},
        "roundtrip": {
            backend: synthetic_metric()
            for backend in ("rustsht", "ducc", "rust_ducc")
        },
        "basis": basis,
        "adjoints": {
            backend: {
                "synthesis_relative_residual": 1e-14,
                "analysis_relative_residual": 1e-14,
            }
            for backend in ("rustsht", "ducc")
        },
        "edge_cases": {
            "zero": {
                "exact_zero": True,
                "map_rust_ducc": synthetic_exact_zero_metric(nmap),
                "alm_rust_ducc": synthetic_exact_zero_metric(nalm),
            },
            "extreme": {
                "map_rust_ducc": synthetic_metric(),
                "alm_rust_ducc": synthetic_metric(),
            },
            "subnormal": {
                "map_rust_ducc": subnormal_metric,
                "alm_rust_ducc": {
                    **subnormal_metric,
                    "near_zero_reference_count": nalm,
                },
            },
        },
    }


def complete_synthetic_group(batch_duration_ms: float,
                             validation: dict | None) -> dict:
    samples = []
    comparisons = []
    for pair in range(benchmark.PROCESS_PAIRS):
        for backend in ("rustsht", "ducc"):
            samples.append({
                **SYNTHETIC_CC_CASE,
                "operation": "analysis",
                "threads": 1,
                "process_pair": pair,
                "backend": backend,
                "warmup_ms": 10.0,
                "batch_ms": [batch_duration_ms] * benchmark.BATCHES_PER_PROCESS,
                "batch_total_ms": [batch_duration_ms] * benchmark.BATCHES_PER_PROCESS,
                "reset_consistent": True,
            })
        comparisons.append({
            **SYNTHETIC_CC_CASE,
            "operation": "analysis",
            "threads": 1,
            "process_pair": pair,
            "finite_outputs": True,
            **synthetic_metric(),
            "numerical_status": "accepted",
        })
    saved_validation = None if validation is None else {
        **validation, "case_id": SYNTHETIC_CC_CASE["case_id"], "threads": 1,
    }
    coverage = benchmark.dense_sweep_coverage(
        [SYNTHETIC_CC_CASE], samples, comparisons, [], "CC",
        [] if saved_validation is None else [saved_validation],
    )
    return coverage["measurement_groups"][
        f"{SYNTHETIC_CC_CASE['case_id']}|analysis|q1"
    ]


def test_cc_dense_cases_match_independent_floor_quarter_manifest() -> None:
    cases = benchmark.make_cases(False, dense_sweep=True, grid="CC")
    observed = {
        (
            case["case_id"],
            case["lmax"],
            case["mmax"],
            case["ntheta"],
            case["nphi"],
            case["longitude_mode"],
        )
        for case in cases
    }
    expected = independent_cc_manifest()

    assert len(expected) == 632
    assert len(cases) == 632
    assert len(observed) == len(cases)
    assert observed == expected
    assert {
        case["mmax"] for case in cases if case["lmax"] in (1, 2, 3)
    } == {0, 1, 2, 3}


def test_dense_gl_keeps_the_historical_low_degree_order_contract() -> None:
    cases = benchmark.make_cases(False, dense_sweep=True, grid="GL")
    orders_by_degree = {}
    for case in cases:
        orders_by_degree.setdefault(case["lmax"], set()).add(case["mmax"])

    assert orders_by_degree[0] == {0}
    assert orders_by_degree[1] == {1}
    assert orders_by_degree[2] == {1, 2}
    assert orders_by_degree[3] == {1, 3}


def test_legacy_execution_protocol_uses_declared_binary_source() -> None:
    legacy = {
        "production_source_revision": "a" * 40,
        "rust_source_revision": "b" * 40,
    }
    assert benchmark.execution_protocol(legacy)["binary_source_revision"] == "a" * 40


def test_skipped_build_requires_matching_binary_build_identity(tmp_path) -> None:
    record = {
        "metadata": {
            "production_source_revision": "a" * 40,
            "rust_binary_sha256": "1" * 64,
            "ducc_binary_sha256": "2" * 64,
            "ducc_source_revision": "3" * 40,
            "rustc": "rustc test",
            "cxx": "g++ test",
            "rust_flags": "rust flags",
            "ducc_flags": "DUCC flags",
        }
    }
    path = tmp_path / "build-record.json"
    path.write_text(json.dumps(record))
    actual = dict(record["metadata"])

    verified = benchmark.validate_binary_build_record(path, actual, "a" * 40)
    assert verified["binary_source_revision"] == "a" * 40
    assert verified["rust_binary_sha256"] == "1" * 64

    actual["rust_binary_sha256"] = "9" * 64
    with pytest.raises(ValueError, match="rust_binary_sha256"):
        benchmark.validate_binary_build_record(path, actual, "a" * 40)


def test_sweep_acceptance_is_independent_of_optional_stage_profile() -> None:
    accepted_group = {
        "status": "complete",
        "numerical_status": "accepted",
        "timing_quality_status": "qualified",
        "duration_window_status": "within_target",
        "reset_consistency_status": "accepted",
    }
    case_id = "CC-T4-m4-even_smooth"
    accepted_groups = {
        f"{case_id}|{operation}|q{threads}": dict(accepted_group)
        for operation in ("analysis", "synthesis")
        for threads in (1, 8)
    }
    assert benchmark.dense_sweep_measurement_pass(
        {case_id}, {case_id}, [], accepted_groups
    )


def test_merged_sample_export_strips_local_host_telemetry() -> None:
    samples = [{"backend": "rustsht", "batch_ms": [1.0], "host_diagnostics": {"wait": 0.2}}]
    raw_shard = benchmark.samples_for_result_export(
        samples, {"retain_local_host_telemetry": True}
    )
    aggregate = benchmark.samples_for_result_export(
        samples, {"retain_local_host_telemetry": True, "merged_shards": [{"name": "one"}]}
    )

    assert raw_shard is samples
    assert aggregate == [{"backend": "rustsht", "batch_ms": [1.0]}]
    historical = [{
        "payload": {
            "samples": [{"backend": "ducc", "host_diagnostics": {"wait": 0.2}}],
            "host_samples": [{"wait": 0.2}],
        }
    }]
    assert benchmark.strip_local_host_telemetry(historical) == [{
        "payload": {"samples": [{"backend": "ducc"}]}
    }]


def test_complex_relative_l2_uses_magnitude_squared() -> None:
    reference = np.array([2.0 + 1.0j, 1.0 - 0.2j], dtype=np.complex128)
    actual = reference.copy()
    actual[0] += 1e-8 - 2e-8j

    metrics = benchmark.error_metrics(actual, reference)
    expected = np.linalg.norm(actual - reference) / np.linalg.norm(reference)

    assert metrics["relative_l2"] == pytest.approx(expected, rel=1e-14)
    assert metrics["reference_peak"] == pytest.approx(np.abs(reference).max())


def test_zero_reference_subnormal_comparison_has_a_tight_absolute_gate() -> None:
    smallest_subnormal = np.nextafter(0.0, 1.0)
    reference = np.zeros(3, dtype=np.complex128)
    actual = np.array([3 * smallest_subnormal, 0.0, 0.0], dtype=np.complex128)
    metric = benchmark.error_metrics(actual, reference)

    assert benchmark.zero_reference_subnormal_comparison_acceptable(metric, 3)
    assert cc_audit_manifest.zero_reference_subnormal_metric_acceptable(metric, 3)
    assert not benchmark.zero_reference_subnormal_comparison_acceptable(metric, 2)
    assert not cc_audit_manifest.zero_reference_subnormal_metric_acceptable(metric, 2)

    too_large = benchmark.error_metrics(
        np.array([2 * np.finfo(np.float64).tiny, 0.0, 0.0], dtype=np.complex128),
        reference,
    )
    assert not benchmark.zero_reference_subnormal_comparison_acceptable(too_large, 3)
    assert not cc_audit_manifest.zero_reference_subnormal_metric_acceptable(too_large, 3)


def test_runner_and_independent_auditor_accept_the_same_complete_validation() -> None:
    validation = accepted_validation()

    assert benchmark.saved_validation_acceptance(validation, SYNTHETIC_CC_CASE) == (
        "accepted", []
    )
    assert cc_audit_manifest.validation_reasons(validation, SYNTHETIC_CC_CASE) == []


def test_zero_reference_exact_edge_match_is_accepted() -> None:
    validation = accepted_validation()
    exact_match = synthetic_exact_zero_metric(
        SYNTHETIC_CC_CASE["ntheta"] * SYNTHETIC_CC_CASE["nphi"]
    )
    validation["edge_cases"]["subnormal"]["map_rust_ducc"] = exact_match

    assert benchmark.saved_validation_acceptance(validation, SYNTHETIC_CC_CASE)[0] == "accepted"
    assert cc_audit_manifest.validation_reasons(validation, SYNTHETIC_CC_CASE) == []


@pytest.mark.parametrize("missing_or_invalid", [
    "roundtrip_backend",
    "basis_metric",
    "adjoint",
    "edge_metric",
    "reset",
])
def test_runner_and_auditor_reject_incomplete_or_nonfinite_validation(
    missing_or_invalid: str,
) -> None:
    validation = accepted_validation()
    if missing_or_invalid == "roundtrip_backend":
        del validation["roundtrip"]["ducc"]
    elif missing_or_invalid == "basis_metric":
        validation["basis"][0]["map_rust_ducc"]["relative_l2"] = float("nan")
    elif missing_or_invalid == "adjoint":
        validation["adjoints"]["rustsht"]["analysis_relative_residual"] = float("nan")
    elif missing_or_invalid == "edge_metric":
        del validation["edge_cases"]["extreme"]["alm_rust_ducc"]
    else:
        validation["reset_consistent"]["ducc"] = False

    runner_status, runner_reasons = benchmark.saved_validation_acceptance(
        validation, SYNTHETIC_CC_CASE
    )
    audit_status, audit_reasons = cc_audit_manifest.audited_numerical_status(
        True, [], validation, "accepted", SYNTHETIC_CC_CASE
    )

    assert runner_status == audit_status == "rejected"
    assert runner_reasons
    assert audit_reasons


def test_three_complete_process_pairs_without_validation_remain_incomplete() -> None:
    group = complete_synthetic_group(30.0, None)
    runner_status, runner_reasons = benchmark.saved_validation_acceptance(None)
    audit_status, audit_reasons = cc_audit_manifest.audited_numerical_status(
        True, [], None, "accepted", SYNTHETIC_CC_CASE
    )

    assert group["status"] == "complete"
    assert group["numerical_status"] == "incomplete"
    assert group["timing_quality_status"] == "qualified"
    assert runner_status == audit_status == "incomplete"
    assert runner_reasons == audit_reasons == ["missing_validation"]
    assert not benchmark.dense_sweep_measurement_pass(
        {SYNTHETIC_CC_CASE["case_id"]}, {SYNTHETIC_CC_CASE["case_id"]}, [],
        {"synthetic": group},
    )
    audited_group = {
        "status": "complete",
        "numerical_status": audit_status,
        "timing_quality_status": "qualified",
        "duration_window_status": "within_target",
        "reset_consistency_status": "accepted",
    }
    assert not cc_audit_manifest.independent_full_sweep_pass(
        [audited_group], [], [], [], [], [], [], {}, {"source": {"sha"}}, True
    )


def test_random_numeric_record_requires_all_finite_metrics() -> None:
    record = {
        **SYNTHETIC_CC_CASE,
        "operation": "analysis",
        "finite_outputs": True,
        **synthetic_metric(),
    }

    assert benchmark.random_comparison_failures(record) == []
    assert cc_audit_manifest.random_numeric_reasons(record) == []
    del record["reference_peak"]
    assert benchmark.random_comparison_failures(record)
    assert cc_audit_manifest.random_numeric_reasons(record)


def test_complete_batch_counts_keep_out_of_window_status_as_a_caveat() -> None:
    group = complete_synthetic_group(5.0, accepted_validation())

    assert group["status"] == "complete"
    assert group["numerical_status"] == "accepted"
    assert group["timing_quality_status"] == "qualified"
    assert group["duration_window_status"] == "partially_outside_target"
    assert not benchmark.dense_sweep_measurement_pass(
        {SYNTHETIC_CC_CASE["case_id"]}, {SYNTHETIC_CC_CASE["case_id"]}, [],
        {"synthetic": group},
    )
    audited_group = {
        "status": "complete",
        "numerical_status": "accepted",
        "timing_quality_status": "qualified",
        "duration_window_status": group["duration_window_status"],
        "reset_consistency_status": "accepted",
    }
    assert not cc_audit_manifest.independent_full_sweep_pass(
        [audited_group], [], [], [], [], ["synthetic"], [], {}, {"source": {"sha"}}, True
    )


def test_validation_exception_checkpoint_is_retried_with_old_samples_preserved() -> None:
    failed_payload = {
        "samples": [{"backend": "rustsht", "batch_ms": [1.0]}],
        "comparisons": [],
        "validations": [],
        "failures": [{"failure_class": "validation_error"}],
        "host_samples": [],
    }
    exception_checkpoint = benchmark.checkpoint_pair_state(
        None, failed_payload, False, "error", validation_required=True
    )
    assert not benchmark.checkpoint_pair_is_complete(exception_checkpoint, True)
    history = benchmark.checkpoint_history_records(
        {"pairs": {"0": exception_checkpoint}}, SYNTHETIC_CC_CASE["case_id"],
        "analysis", 1,
    )
    assert len(history) == 1
    assert history[0]["validation_status"] == "error"
    assert history[0]["payload"]["samples"] == failed_payload["samples"]

    rerun_checkpoint = benchmark.checkpoint_pair_state(
        exception_checkpoint,
        {
            "samples": [{"backend": "ducc", "batch_ms": [2.0]}],
            "validations": [{"acceptance_status": "accepted"}],
        },
        True,
        "accepted",
        validation_required=True,
    )
    assert benchmark.checkpoint_pair_is_complete(rerun_checkpoint, True)
    assert rerun_checkpoint["attempt_history"][0]["payload"]["samples"] == (
        failed_payload["samples"]
    )
    legacy_complete_without_validation = {"complete": True, "payload": {}}
    assert not benchmark.checkpoint_pair_is_complete(
        legacy_complete_without_validation, validation_required=True
    )
    status_without_validation_payload = {
        "complete": True,
        "validation_status": "accepted",
        "payload": {"validations": []},
    }
    assert not benchmark.checkpoint_pair_is_complete(
        status_without_validation_payload, validation_required=True
    )


def test_nonfinite_sidecar_values_do_not_drop_validation_or_failure_records(tmp_path) -> None:
    sidecar = tmp_path / "results.json"
    benchmark.write_json_sidecar(sidecar, {
        "validation": [{"relative_l2": float("nan"), "case_id": "synthetic"}],
        "failures": [{"failure_class": "validation_error", "metric": float("inf")}],
        "samples": [{"batch_ms": [1.0]}],
    })
    restored = json.loads(sidecar.read_text())

    assert restored["validation"] == [{"relative_l2": None, "case_id": "synthetic"}]
    assert restored["failures"] == [{"failure_class": "validation_error", "metric": None}]
    assert restored["samples"] == [{"batch_ms": [1.0]}]
