# SPDX-FileCopyrightText: 2026 Albert M. W. Yau
# SPDX-License-Identifier: GPL-2.0-or-later
"""Independent manifest checks for the native scalar parity runner."""

import json

import pytest

from scripts import benchmark_ducc_full_transform_parity as benchmark


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
        "reset_consistency_status": "accepted",
    }
    assert benchmark.dense_sweep_measurement_pass(
        {"CC-T4-m4-even_smooth"},
        {"CC-T4-m4-even_smooth"},
        [],
        {"case|analysis|q1": accepted_group},
    )
