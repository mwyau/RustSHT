import argparse
import hashlib
import json
import math
import statistics
from pathlib import Path
import sys
sys.path.insert(0, str(Path.cwd()))
from scripts.benchmark_ducc_f32_native import error_metrics, make_fixture, run_worker

root = Path.cwd()
parser = argparse.ArgumentParser(description="Run the recorded B1 GL T720 full-q1 screen")
parser.add_argument("--parent", required=True, type=Path)
parser.add_argument("--candidate", required=True, type=Path)
parser.add_argument("--ducc", required=True, type=Path)
parser.add_argument("--output", type=Path, default=Path("b1-screen-results"))
args = parser.parse_args()
out = args.output
out.mkdir(parents=True, exist_ok=True)
binaries = {"parent": args.parent, "candidate": args.candidate, "ducc": args.ducc}
orderings = (("parent", "candidate", "ducc"), ("ducc", "candidate", "parent"), ("candidate", "parent", "ducc"))
rows = []
for mmax, nphi in ((720, 1458),):
    for threads in (1,):
        case = {"grid": "GL", "ntheta": 721, "nphi": nphi, "lmax": 720, "mmax": mmax,
                "longitude": "even", "spin": 0, "operation": "analysis", "threads": threads}
        case_id = f"GL-T720-M{mmax}-analysis-q{threads}"
        fixture = make_fixture(case, out / "fixtures" / case_id)
        cpus = "2" if threads == 1 else "0,1,2,3,4,5,6,7"
        pair_rows = []
        samples = {name: [] for name in binaries}
        for pair, order in enumerate(orderings, 1):
            records = {}
            outputs = {}
            for name in order:
                record = run_worker(binaries[name], name, case, fixture,
                                    out / "runs" / case_id / f"pair-{pair}" / name, 25.0, cpus)
                records[name] = record
                samples[name].extend(record["batch_ms"])
                outputs[name] = [Path(path) for path in record["outputs"]]
            medians = {name: statistics.median(records[name]["batch_ms"]) for name in binaries}
            pair_rows.append({"pair": pair, "order": order, "median_ms": medians,
                              "candidate_parent": medians["candidate"] / medians["parent"],
                              "candidate_ducc": medians["candidate"] / medians["ducc"],
                              "parent_ducc": medians["parent"] / medians["ducc"],
                              "candidate_vs_parent_error": error_metrics(outputs["parent"], outputs["candidate"]),
                              "candidate_vs_ducc_error": error_metrics(outputs["ducc"], outputs["candidate"])})
            print(f"completed {case_id} pair {pair}", flush=True)
        row = {"case_id": case_id, "case": case, "cpus": cpus, "rows": pair_rows,
               "fixture_sha256": {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in fixture.values()},
               "binary_sha256": {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in binaries.items()},
               "aggregate_median_ms": {name: statistics.median(values) for name, values in samples.items()},
               "candidate_parent_geomean": math.exp(statistics.mean(math.log(p["candidate_parent"]) for p in pair_rows)),
               "candidate_ducc_geomean": math.exp(statistics.mean(math.log(p["candidate_ducc"]) for p in pair_rows)),
               "parent_ducc_geomean": math.exp(statistics.mean(math.log(p["parent_ducc"]) for p in pair_rows))}
        rows.append(row)
        print(json.dumps({k: row[k] for k in ("case_id", "aggregate_median_ms", "candidate_parent_geomean", "candidate_ducc_geomean", "parent_ducc_geomean", "rows")}, separators=(",", ":")), flush=True)
(out / "results.json").write_text(json.dumps(rows, indent=2) + "\n")
print(out / "results.json", flush=True)
