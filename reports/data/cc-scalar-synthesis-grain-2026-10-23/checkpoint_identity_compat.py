#!/usr/bin/env python3
"""Run the native parity harness around its null build-record checkpoint bug."""

import importlib.util
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "scripts/benchmark_ducc_full_transform_parity.py"
SPEC = importlib.util.spec_from_file_location("rustsht_benchmark_runner", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load benchmark runner: {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

original_checkpoint_identity = MODULE.checkpoint_identity


def checkpoint_identity_compat(metadata, *args, **kwargs):
    if metadata.get("binary_build_record") is None:
        metadata = dict(metadata)
        metadata["binary_build_record"] = {}
    return original_checkpoint_identity(metadata, *args, **kwargs)


MODULE.checkpoint_identity = checkpoint_identity_compat
MODULE.main()
