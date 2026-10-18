import hashlib
import numpy as np

for label, baseline_path, candidate_path in [
    ("T2047 q8", "target/m6-m2-spin1-fused-visitor/baseline-t2047-q8.npy", "target/m6-m2-spin1-fused-visitor/candidate-t2047-q8.npy"),
    ("T511 q1", "target/m6-m2-spin1-fused-visitor/baseline-t511-q1.npy", "target/m6-m2-spin1-fused-visitor/candidate-t511-q1.npy"),
]:
    baseline = np.load(baseline_path)
    candidate = np.load(candidate_path)
    equal = np.array_equal(baseline, candidate)
    max_abs = float(np.max(np.abs(baseline - candidate)))
    print(f"{label}: bitwise_equal={equal} max_abs_difference={max_abs:.3e}")
    print(f"  baseline_sha256={hashlib.sha256(baseline.tobytes(order='C')).hexdigest()}")
    print(f"  candidate_sha256={hashlib.sha256(candidate.tobytes(order='C')).hexdigest()}")
    if not equal:
        raise SystemExit(1)
