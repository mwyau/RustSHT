import pathlib, sys
root = pathlib.Path(sys.argv.pop(1))
sys.path.insert(0, str(root / 'scripts'))
import benchmark_ducc_spin1_native as bench
original = bench.make_cases
bench.make_cases = lambda lmax_values, grids, smoke: [
    case for case in original(lmax_values, grids, smoke)
    if case['latitude_mode'] == 'exact'
]
sys.argv = [str(root / 'scripts' / 'benchmark_ducc_spin1_native.py'), *sys.argv[1:]]
raise SystemExit(bench.main())
