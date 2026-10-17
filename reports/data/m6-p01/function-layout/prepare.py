from pathlib import Path
import difflib
import hashlib
import re
import subprocess

repo = Path(__import__("subprocess").check_output(["git","rev-parse","--show-toplevel"], text=True).strip())
source = repo / 'target/m6-p01-function-layout/source'

def committed(path: str) -> str:
    return subprocess.check_output(['git', 'show', f'HEAD:{path}'], cwd=repo, text=True)

# The P01 trait entry points are the only mod.rs runtime additions. Drop the
# test-only M1 profiler module from this release-only source copy.
mod = (repo / 'src/legendre/mod.rs').read_text()
profile_module = '#[cfg(test)]\nmod route_profile;\n\n'
assert mod.count(profile_module) == 1
(source / 'src/legendre/mod.rs').write_text(mod.replace(profile_module, '', 1))

# Add only the scalar transition implementation to the committed paired
# recurrence source, placing that implementation at the end of the file.
current_pair = (repo / 'src/legendre/scalar_paired.rs').read_text()
start_marker = '''#[expect(\n    clippy::too_many_arguments,\n    reason = "The transition kernel receives coefficient factors and caller-owned lane scratch explicitly."\n)]\npub(crate) fn try_synthesize_order_scalar_transition_dispatched('''
end_marker = '\n#[expect(\n    clippy::too_many_arguments,\n    reason = "The transition kernel receives coefficient factors and caller-owned lane scratch explicitly."\n)]\npub(crate) fn try_synthesize_order_pair_transition_dispatched('
start = current_pair.index(start_marker)
end = current_pair.index(end_marker, start)
p01_block = current_pair[start:end].rstrip()
# Remove test-only M1 route-counter hooks from the inserted P01 block.
p01_block = re.sub(
    r'(?m)^[ \t]*#\[cfg\(test\)\]\n[ \t]*super::route_profile::[^\n]*\n',
    '',
    p01_block,
)
p01_block = re.sub(
    r'(?m)^[ \t]*#\[cfg\(test\)\]\n[ \t]*let profile_started = super::route_profile::start\(\);\n',
    '',
    p01_block,
)
p01_block = re.sub(
    r'(?m)^[ \t]*#\[cfg\(test\)\]\n[ \t]*\{\n(?:[ \t]*super::route_profile::[^\n]*\n)+[ \t]*\}\n',
    '',
    p01_block,
)
assert 'route_profile' not in p01_block
assert 'try_synthesize_order_scalar_transition_dispatched' not in committed('src/legendre/scalar_paired.rs')
base_pair = committed('src/legendre/scalar_paired.rs').rstrip()
(source / 'src/legendre/scalar_paired.rs').write_text(base_pair + '\n\n' + p01_block + '\n')

# Start from P01's scalar-only transition call site, but restore the committed
# visitor coefficient arithmetic so this copy excludes P02's reuse change.
transform = (repo / 'src/transform.rs').read_text()
current_coeffs = '''                                        let coefficient_even =\n                                            recurrence.fast_spin_first[pair_index];\n                                        let coefficient_odd =\n                                            recurrence.fast_spin_second[pair_index];'''
baseline_coeffs = '''                                        let coefficient_offset = l - m;\n                                        let next_even_coefficient = if l + 2 <= lmax {\n                                            alm[coefficient_start + coefficient_offset + 2]\n                                        } else {\n                                            Complex::new(T::zero(), T::zero())\n                                        };\n                                        let coefficient_even = (alm\n                                            [coefficient_start + coefficient_offset]\n                                            * recurrence.epsilon[l + 1]\n                                            + next_even_coefficient * recurrence.epsilon[l + 2])\n                                            * recurrence.spin_alpha[pair_index];\n                                        let coefficient_odd = if l < lmax {\n                                            alm[coefficient_start + coefficient_offset + 1]\n                                                * recurrence.spin_alpha[pair_index]\n                                        } else {\n                                            Complex::new(T::zero(), T::zero())\n                                        };'''
assert transform.count(current_coeffs) == 2
(source / 'src/transform.rs').write_text(transform.replace(current_coeffs, baseline_coeffs))

changed = [
    'src/legendre/mod.rs',
    'src/legendre/scalar_paired.rs',
    'src/transform.rs',
]
patch = []
for path in changed:
    before = committed(path).splitlines(keepends=True)
    after = (source / path).read_text().splitlines(keepends=True)
    patch.extend(difflib.unified_diff(before, after, fromfile='a/' + path, tofile='b/' + path))
(source.parent / 'source.patch').write_text(''.join(patch))

with (source.parent / 'source.sha256').open('w') as out:
    for path in changed:
        digest = hashlib.sha256((source / path).read_bytes()).hexdigest()
        out.write(f'{digest}  {path}\n')
    digest = hashlib.sha256((source.parent / 'source.patch').read_bytes()).hexdigest()
    out.write(f'{digest}  source.patch\n')
print(f'P01 block lines: {p01_block.count(chr(10)) + 1}')
print(f'P02 fallback coefficient replacements: {transform.count(current_coeffs)} -> 0')
print(f'Source patch: {source.parent / "source.patch"}')
