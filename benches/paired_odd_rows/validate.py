#!/usr/bin/env python3
"""One-shot numerical checks for the paired-row FFT prototype."""

from __future__ import annotations

import argparse
import csv
import math
import pathlib
import struct
import subprocess
import tempfile


ODD_LENGTHS = (129, 255, 257, 361, 497, 509, 511, 513, 515)
EVEN_CONTROLS = (256, 378, 500, 512, 540)
FOCUS_ODD = (361, 497, 509, 511, 513)
TOLERANCE = 2.0e-12


def values(path: pathlib.Path) -> list[float]:
    data = path.read_bytes()
    if len(data) % 8:
        raise RuntimeError(f"{path} is not an f64 array")
    return list(struct.unpack("<" + "d" * (len(data) // 8), data))


def compare(reference: list[float], actual: list[float]) -> tuple[float, float]:
    if len(reference) != len(actual):
        raise RuntimeError(f"output lengths differ: {len(reference)} != {len(actual)}")
    error2 = math.fsum((x - y) ** 2 for x, y in zip(reference, actual))
    norm2 = math.fsum(x * x for x in reference)
    max_abs = max((abs(x - y) for x, y in zip(reference, actual)), default=0.0)
    scale = max(1.0, max((abs(x) for x in reference), default=0.0))
    relative_l2 = math.sqrt(error2 / norm2) if norm2 else math.sqrt(error2)
    return relative_l2, max_abs / scale


def checked_compare(reference: list[float], actual: list[float], label: str) -> tuple[float, float]:
    result = compare(reference, actual)
    if result[0] > TOLERANCE or result[1] > TOLERANCE:
        raise RuntimeError(f"{label}: relative_l2={result[0]:.4e}, scaled_max={result[1]:.4e}")
    return result


def real_dft(samples: list[float], n: int, modes: int) -> list[float]:
    result: list[float] = []
    for k in range(modes):
        real_terms = []
        imag_terms = []
        for j, sample in enumerate(samples):
            angle = 2.0 * math.pi * k * j / n
            real_terms.append(sample * math.cos(angle))
            imag_terms.append(-sample * math.sin(angle))
        result.extend((math.fsum(real_terms), math.fsum(imag_terms)))
    result[1] = 0.0
    if n % 2 == 0 and modes > n // 2:
        result[2 * (n // 2) + 1] = 0.0
    return result


def real_inverse_series(spectrum: list[float], n: int, bins: int) -> list[float]:
    result = []
    highest = n // 2
    for j in range(n):
        terms = [spectrum[0]]
        for k in range(1, highest + 1):
            re = spectrum[2 * k]
            im = spectrum[2 * k + 1]
            angle = 2.0 * math.pi * k * j / n
            if n % 2 == 0 and k == highest:
                terms.append(re * (-1.0 if j % 2 else 1.0))
            else:
                terms.append(2.0 * (re * math.cos(angle) - im * math.sin(angle)))
        result.append(math.fsum(terms))
    assert bins == n // 2 + 1
    return result


def invoke(command: list[str], cpus: str | None = None) -> None:
    if cpus is not None:
        command = ["taskset", "-c", cpus, *command]
    subprocess.run(command, check=True, stdout=subprocess.DEVNULL)


def rust_once(
    binary: pathlib.Path,
    route: str,
    operation: str,
    n: int,
    rows: int,
    threads: int,
    shape: str,
    fixture: pathlib.Path,
    output: pathlib.Path,
    cpus: str,
) -> None:
    invoke(
        [
            str(binary),
            "--once",
            route,
            operation,
            str(n),
            str(rows),
            str(threads),
            shape,
            str(fixture),
            str(output),
        ],
        cpus,
    )


def ducc_once(
    binary: pathlib.Path,
    operation: str,
    n: int,
    rows: int,
    threads: int,
    shape: str,
    fixture: pathlib.Path,
    output: pathlib.Path,
    cpus: str,
) -> None:
    invoke(
        [
            str(binary),
            operation,
            str(n),
            str(rows),
            str(threads),
            shape,
            str(fixture),
            str(output),
            "--once",
        ],
        cpus,
    )


def run_case(
    rust: pathlib.Path,
    ducc: pathlib.Path,
    temp: pathlib.Path,
    n: int,
    rows: int,
    threads: int,
    operation: str,
    shape: str,
    routes: tuple[str, ...],
    q1_cpus: str,
    q8_cpus: str,
    records: list[dict[str, object]],
) -> dict[str, pathlib.Path]:
    label = f"{operation}-n{n}-r{rows}-q{threads}-{shape}"
    fixture = temp / f"{label}.input"
    invoke(
        [str(rust), "--write-fixture", operation, str(n), str(rows), shape, str(fixture)]
    )
    cpus = q1_cpus if threads == 1 else q8_cpus
    outputs: dict[str, pathlib.Path] = {}
    for route in routes:
        output = temp / f"{label}-{route}.output"
        if route == "ducc-real":
            ducc_once(ducc, operation, n, rows, threads, shape, fixture, output, cpus)
        else:
            rust_once(rust, route, operation, n, rows, threads, shape, fixture, output, cpus)
        outputs[route] = output

    reference = values(outputs["rust-real"])
    for route, output in outputs.items():
        actual = values(output)
        relative_l2, scaled_max = checked_compare(reference, actual, f"{label} {route}")
        records.append(
            {
                "operation": operation,
                "n": n,
                "rows": rows,
                "threads": threads,
                "shape": shape,
                "route": route,
                "relative_l2_vs_rust_real": f"{relative_l2:.12e}",
                "scaled_max_vs_rust_real": f"{scaled_max:.12e}",
            }
        )

    if operation == "forward" and shape == "full":
        samples = values(fixture)
        modes = n // 2 + 1
        output = reference
        for row in range(min(2, rows)):
            actual = output[row * modes * 2 : (row + 1) * modes * 2]
            expected = real_dft(samples[row * n : (row + 1) * n], n, modes)
            relative_l2, scaled_max = checked_compare(expected, actual, f"{label} direct DFT row {row}")
            records.append(
                {
                    "operation": operation,
                    "n": n,
                    "rows": rows,
                    "threads": threads,
                    "shape": shape,
                    "route": f"direct-dft-row-{row}",
                    "relative_l2_vs_rust_real": f"{relative_l2:.12e}",
                    "scaled_max_vs_rust_real": f"{scaled_max:.12e}",
                }
            )

    if operation == "inverse":
        spectra = values(fixture)
        bins = n // 2 + 1
        expected = []
        for row in range(min(2, rows)):
            packed = spectra[row * bins * 2 : (row + 1) * bins * 2]
            expected.extend(real_inverse_series(packed, n, bins))
        actual = reference[: len(expected)]
        relative_l2, scaled_max = checked_compare(expected, actual, f"{label} direct inverse series")
        records.append(
            {
                "operation": operation,
                "n": n,
                "rows": rows,
                "threads": threads,
                "shape": shape,
                "route": "direct-inverse-series",
                "relative_l2_vs_rust_real": f"{relative_l2:.12e}",
                "scaled_max_vs_rust_real": f"{scaled_max:.12e}",
            }
        )
    return outputs


def roundtrip(
    rust: pathlib.Path,
    ducc: pathlib.Path,
    temp: pathlib.Path,
    n: int,
    rows: int,
    q1_cpus: str,
    records: list[dict[str, object]],
) -> None:
    forward_fixture = temp / f"roundtrip-n{n}.map"
    invoke([str(rust), "--write-fixture", "forward", str(n), str(rows), "full", str(forward_fixture)])
    forward_output = temp / f"roundtrip-n{n}.spectrum"
    rust_once(rust, "rust-real", "forward", n, rows, 1, "full", forward_fixture, forward_output, q1_cpus)
    expected = [value * n for value in values(forward_fixture)]
    inverse_routes = ("rust-real", "rust-pair", "ducc-real")
    for route in inverse_routes:
        output = temp / f"roundtrip-n{n}-{route}.map"
        if route == "ducc-real":
            ducc_once(ducc, "inverse", n, rows, 1, "full", forward_output, output, q1_cpus)
        else:
            rust_once(rust, route, "inverse", n, rows, 1, "full", forward_output, output, q1_cpus)
        relative_l2, scaled_max = checked_compare(expected, values(output), f"roundtrip n={n} {route}")
        records.append(
            {
                "operation": "roundtrip",
                "n": n,
                "rows": rows,
                "threads": 1,
                "shape": "full",
                "route": route,
                "relative_l2_vs_n_times_map": f"{relative_l2:.12e}",
                "scaled_max_vs_n_times_map": f"{scaled_max:.12e}",
            }
        )
    if n == 361:
        output = temp / "roundtrip-n361-rust-pair-mixed.map"
        rust_once(rust, "rust-pair-mixed", "inverse", n, rows, 1, "full", forward_output, output, q1_cpus)
        relative_l2, scaled_max = checked_compare(expected, values(output), "roundtrip n=361 rust-pair-mixed")
        records.append(
            {
                "operation": "roundtrip",
                "n": n,
                "rows": rows,
                "threads": 1,
                "shape": "full",
                "route": "rust-pair-mixed",
                "relative_l2_vs_n_times_map": f"{relative_l2:.12e}",
                "scaled_max_vs_n_times_map": f"{scaled_max:.12e}",
            }
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-bin", type=pathlib.Path, required=True)
    parser.add_argument("--ducc-bin", type=pathlib.Path, required=True)
    parser.add_argument("--output-csv", type=pathlib.Path, required=True)
    parser.add_argument("--q1-cpus", default="0")
    parser.add_argument("--q8-cpus", default="0-7")
    parser.add_argument("--quick", action="store_true", help="validate N=361 only")
    args = parser.parse_args()

    lengths = (361,) if args.quick else (*ODD_LENGTHS, *EVEN_CONTROLS)
    routes = ("rust-real", "rust-c2c", "rust-pair", "ducc-real")
    records: list[dict[str, object]] = []
    with tempfile.TemporaryDirectory(prefix="paired-odd-rows-validate-") as directory:
        temp = pathlib.Path(directory)
        for n in lengths:
            run_case(
                args.rust_bin,
                args.ducc_bin,
                temp,
                n,
                9,
                1,
                "forward",
                "full",
                routes,
                args.q1_cpus,
                args.q8_cpus,
                records,
            )
            inverse_routes = routes + (("rust-pair-mixed",) if n == 361 else ())
            run_case(
                args.rust_bin,
                args.ducc_bin,
                temp,
                n,
                9,
                1,
                "inverse",
                "full",
                inverse_routes,
                args.q1_cpus,
                args.q8_cpus,
                records,
            )
            run_case(
                args.rust_bin,
                args.ducc_bin,
                temp,
                n,
                9,
                8,
                "forward",
                "full",
                ("rust-real", "rust-pair"),
                args.q1_cpus,
                args.q8_cpus,
                records,
            )
            run_case(
                args.rust_bin,
                args.ducc_bin,
                temp,
                n,
                9,
                8,
                "inverse",
                "full",
                ("rust-real", "rust-pair"),
                args.q1_cpus,
                args.q8_cpus,
                records,
            )
            run_case(
                args.rust_bin,
                args.ducc_bin,
                temp,
                n,
                9,
                1,
                "forward",
                "reduced",
                ("rust-real", "rust-c2c", "rust-pair", "ducc-real"),
                args.q1_cpus,
                args.q8_cpus,
                records,
            )
            run_case(
                args.rust_bin,
                args.ducc_bin,
                temp,
                n,
                9,
                1,
                "inverse",
                "reduced",
                inverse_routes,
                args.q1_cpus,
                args.q8_cpus,
                records,
            )
            if n in FOCUS_ODD:
                run_case(
                    args.rust_bin,
                    args.ducc_bin,
                    temp,
                    n,
                    9,
                    1,
                    "inverse",
                    "polar",
                    inverse_routes,
                    args.q1_cpus,
                    args.q8_cpus,
                    records,
                )
            run_case(
                args.rust_bin,
                args.ducc_bin,
                temp,
                n,
                8,
                1,
                "forward",
                "full",
                ("rust-real", "rust-pair"),
                args.q1_cpus,
                args.q8_cpus,
                records,
            )
            run_case(
                args.rust_bin,
                args.ducc_bin,
                temp,
                n,
                8,
                1,
                "inverse",
                "full",
                ("rust-real", "rust-pair"),
                args.q1_cpus,
                args.q8_cpus,
                records,
            )
            roundtrip(args.rust_bin, args.ducc_bin, temp, n, 9, args.q1_cpus, records)
            print(f"validated N={n}", flush=True)

    args.output_csv.parent.mkdir(parents=True, exist_ok=True)
    with args.output_csv.open("w", newline="") as output:
        fieldnames = list(dict.fromkeys(key for row in records for key in row))
        writer = csv.DictWriter(output, fieldnames=fieldnames, lineterminator="\n")
        writer.writeheader()
        writer.writerows(records)
    print(f"wrote {args.output_csv} ({len(records)} checks)")


if __name__ == "__main__":
    main()
