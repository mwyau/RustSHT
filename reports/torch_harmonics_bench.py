"""Measure Torch-harmonics Legendre table setup, memory, and transform time.

Run from a checkout of the target torch-harmonics revision in an environment
with PyTorch installed:

    python reports/torch_harmonics_bench.py 720 0 synthesis GL cuda 1

Arguments are bandlimit, spin (0 or 1), operation, grid (GL/CC/F1), device
and Torch CPU thread count. The script emits one JSON record. It is an
optional research harness and is not imported by RustSHT.
"""

import gc
import json
import resource
import sys
import time
import warnings
from types import ModuleType

warnings.filterwarnings("ignore", message="No optimized kernels are available")


def disable_optional_acceleration_imports():
    """Stub optional extensions absent from source-only Torch-harmonics runs."""
    optional_features = {
        "attention_helpers": ("cuda_kernels_is_available", "optimized_kernels_is_available"),
        "disco_helpers": (
            "cuda_kernels_is_available",
            "optimized_kernels_is_available",
            "kpacked_sm90_kernels_is_available",
            "kpacked_sm100_kernels_is_available",
        ),
    }
    for module_name, functions in optional_features.items():
        try:
            __import__(module_name)
        except ModuleNotFoundError as error:
            if error.name != module_name:
                raise
            module = ModuleType(module_name)
            for function_name in functions:
                setattr(module, function_name, lambda: False)
            sys.modules[module_name] = module


def main():
    import torch

    disable_optional_acceleration_imports()
    import torch_harmonics as th

    bandlimit = int(sys.argv[1])
    spin = int(sys.argv[2])
    operation = sys.argv[3]
    grid_name = sys.argv[4]
    device_name = sys.argv[5] if len(sys.argv) > 5 else "cuda"
    thread_count = int(sys.argv[6]) if len(sys.argv) > 6 else 8
    if operation not in {"analysis", "synthesis"} or spin not in {0, 1}:
        raise ValueError("operation must be analysis/synthesis and spin must be 0/1")

    torch.set_num_threads(thread_count)
    torch.manual_seed(42)
    device = torch.device(device_name)
    if device.type == "cuda":
        torch.cuda.init()
        torch.cuda.synchronize()

    nlat = bandlimit + (2 if grid_name == "CC" else 1)
    nlon = 2 * bandlimit + 3
    grid_type = {"GL": "legendre-gauss", "CC": "equiangular", "F1": "fejer1"}[grid_name]
    grid = th.as_grid(grid_type, nlat=nlat, nlon=nlon)
    exclusive = bandlimit + 1
    transform_type = {
        (0, "analysis"): th.RealSHT,
        (0, "synthesis"): th.InverseRealSHT,
        (1, "analysis"): th.RealVectorSHT,
        (1, "synthesis"): th.InverseRealVectorSHT,
    }[(spin, operation)]

    if device.type == "cuda":
        torch.cuda.reset_peak_memory_stats()
    setup_start = time.perf_counter()
    module = transform_type(grid, lmax=exclusive, mmax=exclusive, norm="ortho", csphase=True)
    build_ms = (time.perf_counter() - setup_start) * 1000
    folded = getattr(module, "_extended_folded", False)
    weight_bytes = sum(buffer.numel() * buffer.element_size() for buffer in module.buffers())

    transfer_start = time.perf_counter()
    module = module.to(device)
    if device.type == "cuda":
        torch.cuda.synchronize()
    transfer_ms = (time.perf_counter() - transfer_start) * 1000
    table_allocated = torch.cuda.memory_allocated(device) if device.type == "cuda" else None
    peak_after_transfer = torch.cuda.max_memory_allocated(device) if device.type == "cuda" else None

    if operation == "analysis":
        shape = (nlat, nlon) if spin == 0 else (2, nlat, nlon)
        data = torch.randn(shape, dtype=torch.float64, device=device)
    else:
        shape = (exclusive, exclusive) if spin == 0 else (2, exclusive, exclusive)
        data = torch.randn(shape, dtype=torch.complex128, device=device)

    def invoke():
        return module(data)

    for _ in range(3):
        output = invoke()
    if device.type == "cuda":
        torch.cuda.synchronize()
        torch.cuda.reset_peak_memory_stats()

    iterations = max(3, min(24, 5000 // bandlimit))
    event_samples = []
    for _ in range(5):
        if device.type == "cuda":
            start_event = torch.cuda.Event(enable_timing=True)
            end_event = torch.cuda.Event(enable_timing=True)
            start_event.record()
        else:
            start_wall = time.perf_counter()
        for _ in range(iterations):
            output = invoke()
        if device.type == "cuda":
            end_event.record()
            end_event.synchronize()
            event_samples.append(start_event.elapsed_time(end_event) / iterations)
        else:
            event_samples.append((time.perf_counter() - start_wall) * 1000 / iterations)

    host_samples = []
    for _ in range(5):
        start = time.perf_counter()
        output = invoke()
        if device.type == "cuda":
            torch.cuda.synchronize()
        host_samples.append((time.perf_counter() - start) * 1000)

    peak_operation = torch.cuda.max_memory_allocated(device) if device.type == "cuda" else None
    result = {
        "torch": torch.__version__,
        "cuda_runtime": torch.version.cuda,
        "gpu": torch.cuda.get_device_name(device) if device.type == "cuda" else None,
        "device": device_name,
        "torch_threads": thread_count,
        "bandlimit": bandlimit,
        "spin": spin,
        "operation": operation,
        "grid": grid_name,
        "grid_shape": [nlat, nlon],
        "torch_spectral_shape": [exclusive, exclusive],
        "folded_projection": bool(folded),
        "weight_bytes": weight_bytes,
        "plan_build_ms": build_ms,
        "to_device_ms": transfer_ms,
        "device_allocated_after_transfer_bytes": table_allocated,
        "device_peak_after_transfer_bytes": peak_after_transfer,
        "device_peak_during_transform_bytes": peak_operation,
        "event_ms_per_transform": event_samples,
        "host_sync_ms_per_transform": host_samples,
        "iterations_per_event_sample": iterations,
        "max_rss_kib": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
        "output_abs_mean": float(output.abs().mean().item()),
    }
    print(json.dumps(result, sort_keys=True))
    del output, data, module, grid
    if device.type == "cuda":
        torch.cuda.synchronize()
        torch.cuda.empty_cache()
    gc.collect()


if __name__ == "__main__":
    main()
