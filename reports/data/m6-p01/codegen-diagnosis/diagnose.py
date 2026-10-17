#!/usr/bin/env python3
"""Extract retained P01 binaries' spin-1 symbol and disassembly evidence."""

from __future__ import annotations

import hashlib
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
BINARIES = {
    "baseline": ROOT / "target/m6-p01-baseline-bench",
    "default-p01": ROOT / "target/m6-p01-candidate-bench",
    "ordered-p01": ROOT / "target/m6-p01-link-order-bench",
}
PATTERNS = {
    "spin-ring-group": "EXACT:<rustsht::plan::ShtPlan>::synthesize_spin_ring_group",
    "spin1-scalar-block": "EXACT:<rustsht::plan::ShtPlan>::synthesize_spin1_scalar_block",
    "spin1-safe-lane-helper": "try_synthesize_spin1_transition_safe_lanes::<f64>",
    "spin1-v4-kernel": "pulp::x86::v4::V4>::vectorize::imp::<<pulp::x86::v4::V4 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar::ForEachYlm<&mut <rustsht::plan::ShtPlan>::synthesize_spin1_scalar_block",
    "spin1-v3-kernel": "pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar::ForEachYlm<&mut <rustsht::plan::ShtPlan>::synthesize_spin1_scalar_block",
    "p01-dispatch": "try_synthesize_order_scalar_transition_dispatched",
    "p01-v4-kernel": "pulp::x86::v4::V4>::vectorize::imp::<<pulp::x86::v4::V4 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::StagedSynthesisScalarTransition>>",
    "p01-v3-kernel": "pulp::x86::v3::V3>::vectorize::imp::<<pulp::x86::v3::V3 as pulp::Simd>::vectorize::Impl<rustsht::legendre::scalar_paired::StagedSynthesisScalarTransition>>",
}
DISASSEMBLE = (
    "spin1-scalar-block",
    "spin1-safe-lane-helper",
    "spin1-v4-kernel",
    "spin1-v3-kernel",
)


def run(*args: str) -> str:
    return subprocess.check_output(args, text=True)


def symbols(binary: Path) -> list[tuple[int, int, str]]:
    rows = []
    for line in run("nm", "-S", "-nC", str(binary)).splitlines():
        match = re.match(r"^([0-9a-f]+)\s+([0-9a-f]+)\s+\S\s+(.+)$", line)
        if match:
            rows.append((int(match[1], 16), int(match[2], 16), match[3]))
    return rows


def text_section(binary: Path) -> tuple[int, int, int]:
    output = run("readelf", "-SW", str(binary))
    match = re.search(
        r"\[\s*\d+\]\s+\.text\s+PROGBITS\s+([0-9a-f]+)\s+([0-9a-f]+)\s+([0-9a-f]+)",
        output,
    )
    if not match:
        raise RuntimeError(f"could not find .text section in {binary}")
    return tuple(int(value, 16) for value in match.groups())


def find_symbol(rows: list[tuple[int, int, str]], pattern: str) -> tuple[int, int, str]:
    if pattern.startswith("EXACT:"):
        matches = [row for row in rows if pattern.removeprefix("EXACT:") == row[2]]
    else:
        matches = [row for row in rows if pattern in row[2]]
    if len(matches) != 1:
        raise RuntimeError(f"expected one match for {pattern!r}, found {len(matches)}")
    return matches[0]


def disassemble(binary: Path, start: int, size: int) -> str:
    return run(
        "objdump",
        "-dC",
        "-j",
        ".text",
        f"--start-address={start:#x}",
        f"--stop-address={start + size:#x}",
        str(binary),
    )


def normalized_instruction_digest(disassembly: str) -> tuple[str, int]:
    instructions = []
    for line in disassembly.splitlines():
        match = re.match(r"^\s*[0-9a-f]+:\s+(?:[0-9a-f]{2}\s+)+\s*(.*)$", line)
        if not match:
            continue
        instruction = match[1].split("#", 1)[0].strip()
        instruction = re.sub(r"-?0x[0-9a-f]+\(%rip\)", "RIPREL(%rip)", instruction)
        instruction = re.sub(
            r"\b[0-9a-f]+\s+(<[^>]+>)",
            lambda m: re.sub(r"^<[^>]+\+0x([0-9a-f]+)>$", r"<local+0x\1>", m[1]),
            instruction,
        )
        instructions.append(instruction)
    payload = "\n".join(instructions).encode()
    return hashlib.sha256(payload).hexdigest(), len(instructions)


def main() -> None:
    section_rows = []
    symbol_rows = []
    disassembly_parts = []
    normalized_rows = []

    for label, binary in BINARIES.items():
        data = binary.read_bytes()
        binary_hash = hashlib.sha256(data).hexdigest()
        text_address, text_offset, text_size = text_section(binary)
        section_rows.append(
            f"{label}\t{binary}\t{binary_hash}\t.text_va={text_address:#x}\t"
            f".text_file_offset={text_offset:#x}\t.text_size={text_size:#x}"
        )
        rows = symbols(binary)
        found = {}
        for key, pattern in PATTERNS.items():
            try:
                found[key] = find_symbol(rows, pattern)
            except RuntimeError:
                if key.startswith("p01-") and label == "baseline":
                    continue
                raise
            address, size, name = found[key]
            symbol_rows.append(
                f"{label}\t{key}\t{address:#x}\t{size:#x}\t"
                f"text_offset={address - text_address:#x}\t{name}"
            )

        for key in DISASSEMBLE:
            address, size, name = found[key]
            body = disassemble(binary, address, size)
            disassembly_parts.append(
                f"\n===== {label}: {key} at {address:#x}, size {size:#x} =====\n{body}"
            )
            digest, count = normalized_instruction_digest(body)
            normalized_rows.append(f"{label}\t{key}\t{size:#x}\t{count}\t{digest}")

    (OUT / "binary-sections.tsv").write_text("\n".join(section_rows) + "\n")
    (OUT / "spin1-p01-symbols.tsv").write_text("\n".join(symbol_rows) + "\n")
    (OUT / "spin1-hot-path-disassembly.txt").write_text("".join(disassembly_parts))
    (OUT / "normalized-disassembly.tsv").write_text("\n".join(normalized_rows) + "\n")


if __name__ == "__main__":
    main()
