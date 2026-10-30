#!/usr/bin/env python3
"""Run the standalone DUCC f32-input / f64-recurrence trace over GL T720."""

import argparse
import pathlib
import struct
import subprocess


def main():
    package = pathlib.Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True, type=pathlib.Path)
    parser.add_argument("--theta-file", type=pathlib.Path,
                        default=package / "data/gl-t720-theta.bits")
    parser.add_argument("--output", type=pathlib.Path,
                        default=package / "data/ducc-modes.csv")
    parser.add_argument("--smoke", action="store_true",
                        help="run only m=8,l=720,ring=1,real")
    args = parser.parse_args()

    theta_bits = [line.strip() for line in args.theta_file.read_text().splitlines()]
    rings = [0, 1, 5, 360, 715, 719, 720]
    degrees = [600, 650, 700, 719, 720]
    cases = [(0, 720, "re")] + [
        (m, degree, part)
        for m in (1, 2, 5, 8, 16)
        for degree in degrees
        for part in ("re", "im")
    ]
    selected_rings = rings
    if args.smoke:
        cases = [(8, 720, "re")]
        selected_rings = [1]

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w") as output:
        output.write("DUCC_MODE_TRACE,1,DUCC_0.42.0,AVX2_double_lanes=4\n")
        for case_index, (order, degree, part) in enumerate(cases, 1):
            re, im = ("1", "0") if part == "re" else ("0", "1")
            for ring in selected_rings:
                bits = theta_bits[ring]
                theta = struct.unpack(">d", bytes.fromhex(bits))[0]
                cmd = [
                    str(args.binary), str(order), str(degree),
                    format(theta, ".17g"), re, im, str(ring),
                ]
                result = subprocess.run(
                    cmd, text=True, capture_output=True, check=True
                )
                output.write(
                    f"DUCC_CASE,m={order},l={degree},ring={ring},"
                    f"part={part},theta={bits}\n"
                )
                output.write(result.stdout)
            if case_index % 25 == 0:
                print(f"{case_index}/{len(cases)} coefficient cases", flush=True)
    print(args.output)


if __name__ == "__main__":
    main()
