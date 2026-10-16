# Interrupted M23 native DUCC capture

`m23-ducc-native-six-pairs.jsonl` contains three complete alternating pairs,
not six. A separate RustSHT test process started during this capture and drove
host load above the level used for the other M23 comparisons. The capture was
stopped before the remaining pairs to avoid using mixed-load timing as final
evidence. No summary is derived from these incomplete records. The file is
preserved; the six-pair retry uses a separate configuration and output path.
