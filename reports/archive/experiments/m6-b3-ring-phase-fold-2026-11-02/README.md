# B3 ring-scale phase fold — no promotion

Date: 2026-11-02. Rust parent and candidate base: `ead3f5232d47d7fedffaabd218cbfc56feeb6297`; pinned DUCC: `42ce8ab5c5d79e0e2fd9af4a09c2ea4693520e1b`.

The candidate folded each reflected ring's real scale into the common complex phase used by Q and U. It preserves the transform's mathematical scaling and does not change the Legendre recurrence. Two 15-pair GL T720 full/reduced q1/q8 screens were combined (30 paired process observations per cell). Only reduced q1 shows a small resolved gain; the candidate remains slower than DUCC in all four cells, so this experiment was not promoted.

| Cell | Candidate / parent, 95% interval | Candidate / DUCC, 95% interval | Interpretation |
|---|---:|---:|---|
| Full q1 | 0.9918 [0.9836, 1.0000] | 1.0961 [1.0876, 1.1047] | Parent comparison INCONCLUSIVE; DUCC GAP |
| Full q8 | 0.9954 [0.9770, 1.0141] | 1.1410 [1.1205, 1.1618] | Parent comparison INCONCLUSIVE; DUCC GAP |
| Reduced q1 | 0.9944 [0.9905, 0.9984] | 1.0880 [1.0839, 1.0922] | About 0.56% gain; DUCC GAP |
| Reduced q8 | 0.9941 [0.9848, 1.0034] | 1.1075 [1.0987, 1.1164] | Parent comparison INCONCLUSIVE; DUCC GAP |

Candidate/parent coefficient output relative L2 error was at most `1.08e-15`; maximum absolute error was `4.22e-17` full and `8.41e-17` reduced. The original 15-pair results and independent 15-pair repeat are preserved as `paired-results.json` and `raw/repeat-results.json`. Runner scripts and console logs with local paths remain in the local worktree. The repeat JSON SHA-256 is `1fd1f2fb211d28e57e625ba6e52da2bc76ce0ab5cfcf5bd58eec45cd668b5bd5`.

Exact binaries: parent `c33798bec2ed2df6385929da776a75045e3313ca76b328fda3507d5e551a1d97`, candidate `16c2174eb3e534781ffc4015403fb0cbbb99cee9b6ffd44d6c64bbe59436f6b5`, DUCC `b646183e9df8aac030f796e2357b8d92e3b4a194d21be98b460eae043ab63443`. CPU: AMD Ryzen 9 5950X. Rust 1.99.1/LLVM 23.1.1, `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`; DUCC g++ 15.2.0, `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3` for SHT, driver `-fno-fast-math`. q1 used one pinned worker; q8 used eight, with a separate monitoring worker. Exact logical CPU indices are retained only in local run data.

The ring-phase patch, paired records, and fixture hashes are committed. The local runner scripts and console logs remain untracked because they contain machine-specific paths. The result is a diagnostic candidate only; no production source changed.
