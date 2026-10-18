# spin0-t511-q8

Selected 12 completed paired synthesis runs. Per-process times are medians of five retained raw samples. Bootstrap resamples pair ratios with replacement, uses the median, 200000 draws, and seed 1109470273.

- Candidate/baseline median ratio: 0.891984
- Paired bootstrap 95% interval: [0.776221, 0.998472]
- Pair ratio range: [0.628262, 1.488840]
- Pairs above 1.02: 3 / 12
- Numeric bootstrap threshold: **pass** (control 95% upper <= 1.02).
- Measurement status: **inconclusive** due to severe within-process timing nonstationarity; the cell cannot serve as a control pass. Per-pair raw values and selected attempt IDs: [spin0-t511-q8-pairs.csv](spin0-t511-q8-pairs.csv). All selected stdout, GNU time and preflight records remain alongside the runner artifacts. See [the timing audit](spin0-t511-q8-audit.md) and [the preflight/process audit](spin0-t511-q8-preflight-audit.md).
