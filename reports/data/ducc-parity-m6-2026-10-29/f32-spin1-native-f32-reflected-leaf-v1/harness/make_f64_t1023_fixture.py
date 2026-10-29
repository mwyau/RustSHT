from array import array
import argparse
import random
from pathlib import Path

def write_values(path, values):
    with Path(path).open('wb') as stream:
        array('d', values).tofile(stream)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    rng = random.Random(0x71A2026)
    map_count = 1024 * 2048
    for name in ('map0.f64', 'map1.f64'):
        write_values(args.output / name, (rng.uniform(-1.0, 1.0) for _ in range(map_count)))
    for name in ('e.f64', 'b.f64'):
        def coefficients():
            for m in range(1024):
                for ell in range(m, 1024):
                    if ell < 1:
                        yield 0.0
                        yield 0.0
                    else:
                        yield rng.uniform(-1.0, 1.0) / 1024.0
                        yield 0.0 if m == 0 else rng.uniform(-1.0, 1.0) / 1024.0
        write_values(args.output / name, coefficients())

if __name__ == '__main__':
    main()
