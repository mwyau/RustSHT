# Full-transform RustSHT/DUCC parity benchmark (2026-10-20)

Pinned DUCC revision: `80388904b8cfc32400ba84af8a6a45aabfe60f86` (DUCC 0.42.0).
Hardware: AMD Ryzen 9 5950X 16-Core Processor; 16 physical cores and 32 logical CPUs visible; 96.0 GiB RAM; x86-64 features `avx2, fma`.
Builds: Rust `rustc 1.99.0 (b940084d7 2026-09-28)` with `-C target-cpu=x86-64-v3 -Aunexpected_cfgs`, Pulp 0.22.3; DUCC `g++ (Ubuntu 15.2.0-16ubuntu1) 15.2.0` with `-O3 -ffast-math -ffp-contract=fast -march=x86-64-v3`.
Runtime SIMD: Rust worker reports `V3(V3 { sse: Sse, sse2: Sse2, fxsr: Fxsr, sse3: Sse3, ssse3: Ssse3, sse4_1: Sse4_1, sse4_2: Sse4_2, popcnt: Popcnt, avx: Avx, avx2: Avx2, bmi1: Bmi1, bmi2: Bmi2, fma: Fma, lzcnt: Lzcnt })`; DUCC is compiled for x86-64-v3 (AVX2/FMA, four f64 lanes).

## Scope and method

Measured 214 GL scalar configurations using RustSHT's native `ShtPlan` and pinned DUCC's native `analysis_2d`/`synthesis_2d` C++ API. Each transform used all coefficients in its permitted `l=0..lmax, m=0..min(l,mmax)` triangle. Maps and coefficients were generated once per configuration and reused by both workers. `phi0=0.23`; `ntheta=lmax+1`; longitude counts cover odd minimum sampling and even FFT-smooth sampling.

Analysis and synthesis were each timed at q1 and q8. Single-thread processes were pinned to one allowed physical core; q8 used eight physical cores. Rust reused its plan and caller-owned workspace. DUCC's public 2D functions have no persistent plan/workspace object, so the adapter reuses all input/output buffers and reports its internal per-call preparation as part of transform latency. DUCC `mstart` mapped its indexing into a leading-offset view of the same compact coefficient body; no per-call coefficient conversion was timed.

Each backend/case/operation/thread-count collected 3 interleaved independent process pairs with up to three out-of-sample calibration batches and 10 measured repeated-operation batches per process when each operation was below 100 ms. Target was 25 ms and each measured short-operation batch was checked against the requested 20–100 ms range. Operations above 100 ms were measured individually five times per process. First-call and setup costs are separate from warmed samples. The runner records median, p10/p90, range, paired per-batch ratios and output error. See `reports/data/ducc-full-transform-parity-2026-10-20/results.json` and `.csv` for all cases and samples.

## Results

The table reports warmed native calls; ratios below one indicate lower RustSHT latency. Relative L2 uses the DUCC output norm, with near-zero values excluded only from the separately reported max-relative metric. No fixed parity threshold was applied.

| lmax | mmax | grid | operation | threads | Rust ms [p10,p90] | DUCC ms [p10,p90] | Rust/DUCC | paired ratio [p10,p90] | excess ms | relative L2 |
|---:|---:|---|---|---:|---:|---:|---:|---:|---:|---:|
| 0 | 0 | GL 1×1 odd_minimum | analysis | 1 | 0.0116019 [0.0112143,0.0121311] | 0.000995506 [0.00098701,0.00100859] | 11.6543 | 11.1266–12.2975 | 0.0106064 | 0.000e+00 |
| 0 | 0 | GL 1×1 odd_minimum | analysis | 8 | 0.00612666 [0.00602561,0.00637024] | 0.00102205 [0.0010031,0.00102669] | 5.9945 | 5.8931–6.2097 | 0.00510461 | 0.000e+00 |
| 0 | 0 | GL 1×1 odd_minimum | synthesis | 1 | 0.0116576 [0.0110298,0.0124428] | 0.000874294 [0.00086995,0.000882307] | 13.3337 | 12.6702–14.2879 | 0.0107833 | 0.000e+00 |
| 0 | 0 | GL 1×1 odd_minimum | synthesis | 8 | 0.00614753 [0.0060431,0.00625472] | 0.000901566 [0.000884323,0.00091804] | 6.8187 | 6.6437–6.9367 | 0.00524596 | 0.000e+00 |
| 0 | 0 | GL 1×2 even_smooth | analysis | 1 | 0.011883 [0.0113655,0.0125134] | 0.00104025 [0.00103361,0.00108348] | 11.4233 | 10.7738–11.8559 | 0.0108428 | 0.000e+00 |
| 0 | 0 | GL 1×2 even_smooth | analysis | 8 | 0.00606527 [0.00596647,0.00626133] | 0.00104312 [0.00103239,0.00104869] | 5.8145 | 5.7261–6.0169 | 0.00502214 | 0.000e+00 |
| 0 | 0 | GL 1×2 even_smooth | synthesis | 1 | 0.0116677 [0.0109741,0.0119857] | 0.000918936 [0.000907169,0.000941925] | 12.6970 | 11.8790–13.1488 | 0.0107488 | 0.000e+00 |
| 0 | 0 | GL 1×2 even_smooth | synthesis | 8 | 0.00614805 [0.0060277,0.0062985] | 0.000936056 [0.000919146,0.000982475] | 6.5680 | 6.2053–6.7812 | 0.00521199 | 0.000e+00 |
| 4 | 4 | GL 5×9 odd_minimum | analysis | 1 | 0.012328 [0.0119877,0.0128329] | 0.00208952 [0.00207579,0.00210851] | 5.8999 | 5.6929–6.1054 | 0.0102385 | 4.010e-16 |
| 4 | 4 | GL 5×9 odd_minimum | analysis | 8 | 0.00816252 [0.00805437,0.00824701] | 0.0213026 [0.0210475,0.0214976] | 0.3832 | 0.3776–0.3880 | -0.0131401 | 4.010e-16 |
| 4 | 4 | GL 5×9 odd_minimum | synthesis | 1 | 0.0124398 [0.0120716,0.0133153] | 0.00193448 [0.00192464,0.0019505] | 6.4306 | 6.2712–6.8265 | 0.0105053 | 2.954e-16 |
| 4 | 4 | GL 5×9 odd_minimum | synthesis | 8 | 0.00833094 [0.00823979,0.0083886] | 0.0128859 [0.0127161,0.0129934] | 0.6465 | 0.6373–0.6569 | -0.00455496 | 2.954e-16 |
| 4 | 4 | GL 5×10 even_smooth | analysis | 1 | 0.0125 [0.012019,0.0128838] | 0.00210299 [0.0020604,0.00226886] | 5.9439 | 5.3202–6.1756 | 0.010397 | 3.087e-16 |
| 4 | 4 | GL 5×10 even_smooth | analysis | 8 | 0.00823906 [0.00808828,0.00832819] | 0.0212385 [0.0210598,0.0223173] | 0.3879 | 0.3740–0.3917 | -0.0129994 | 3.087e-16 |
| 4 | 4 | GL 5×10 even_smooth | synthesis | 1 | 0.012435 [0.0121557,0.0126653] | 0.00191544 [0.00190415,0.00192643] | 6.4919 | 6.3466–6.6318 | 0.0105195 | 2.749e-16 |
| 4 | 4 | GL 5×10 even_smooth | synthesis | 8 | 0.00833255 [0.00819081,0.00841078] | 0.0128506 [0.0126058,0.0134982] | 0.6484 | 0.6229–0.6584 | -0.004518 | 2.749e-16 |
| 8 | 8 | GL 9×17 odd_minimum | analysis | 1 | 0.0144302 [0.0139095,0.0146943] | 0.00408853 [0.00403401,0.00413376] | 3.5294 | 3.3938–3.6065 | 0.0103417 | 8.438e-16 |
| 8 | 8 | GL 9×17 odd_minimum | analysis | 8 | 0.0115795 [0.0114932,0.0117028] | 0.0239793 [0.023663,0.0245476] | 0.4829 | 0.4700–0.4925 | -0.0123999 | 8.438e-16 |
| 8 | 8 | GL 9×17 odd_minimum | synthesis | 1 | 0.0144068 [0.0139786,0.0148364] | 0.00414352 [0.00412645,0.00421137] | 3.4769 | 3.3795–3.5463 | 0.0102633 | 5.316e-16 |
| 8 | 8 | GL 9×17 odd_minimum | synthesis | 8 | 0.0120182 [0.0118918,0.0121092] | 0.0238876 [0.023395,0.0242865] | 0.5031 | 0.4959–0.5099 | -0.0118694 | 5.316e-16 |
| 8 | 8 | GL 9×18 even_smooth | analysis | 1 | 0.0144661 [0.0139961,0.014831] | 0.00337755 [0.00335971,0.00340971] | 4.2830 | 4.1633–4.3819 | 0.0110886 | 8.925e-16 |
| 8 | 8 | GL 9×18 even_smooth | analysis | 8 | 0.0115071 [0.0113226,0.0116589] | 0.0250332 [0.0236645,0.0256156] | 0.4597 | 0.4470–0.4840 | -0.013526 | 8.925e-16 |
| 8 | 8 | GL 9×18 even_smooth | synthesis | 1 | 0.0140872 [0.0137505,0.0144456] | 0.00338434 [0.00335797,0.00341361] | 4.1624 | 4.0625–4.2930 | 0.0107028 | 5.377e-16 |
| 8 | 8 | GL 9×18 even_smooth | synthesis | 8 | 0.0117083 [0.0115522,0.0118443] | 0.0238221 [0.0227643,0.0284765] | 0.4915 | 0.4031–0.5100 | -0.0121138 | 5.377e-16 |
| 16 | 16 | GL 17×33 odd_minimum | analysis | 1 | 0.0207813 [0.0201948,0.0214379] | 0.00808273 [0.00800418,0.00832634] | 2.5711 | 2.5112–2.6148 | 0.0126985 | 1.366e-15 |
| 16 | 16 | GL 17×33 odd_minimum | analysis | 8 | 0.0131473 [0.0126685,0.0137157] | 0.0267911 [0.0264475,0.0340766] | 0.4907 | 0.3865–0.5072 | -0.0136438 | 1.366e-15 |
| 16 | 16 | GL 17×33 odd_minimum | synthesis | 1 | 0.0194076 [0.0192321,0.0199719] | 0.00816831 [0.00808323,0.00832719] | 2.3760 | 2.3157–2.4456 | 0.0112392 | 1.494e-15 |
| 16 | 16 | GL 17×33 odd_minimum | synthesis | 8 | 0.0145402 [0.0141489,0.0149774] | 0.0255546 [0.025258,0.0260047] | 0.5690 | 0.5528–0.5844 | -0.0110144 | 1.494e-15 |
| 16 | 16 | GL 17×36 even_smooth | analysis | 1 | 0.0208621 [0.020519,0.0215942] | 0.00694375 [0.00686084,0.00725611] | 3.0044 | 2.8585–3.1133 | 0.0139183 | 1.386e-15 |
| 16 | 16 | GL 17×36 even_smooth | analysis | 8 | 0.0134501 [0.0131405,0.0138518] | 0.0263058 [0.0261652,0.0266177] | 0.5113 | 0.5001–0.5261 | -0.0128556 | 1.386e-15 |
| 16 | 16 | GL 17×36 even_smooth | synthesis | 1 | 0.0184718 [0.0183468,0.0188727] | 0.00702416 [0.00693241,0.00717476] | 2.6297 | 2.5743–2.6807 | 0.0114476 | 1.947e-15 |
| 16 | 16 | GL 17×36 even_smooth | synthesis | 8 | 0.013556 [0.0134256,0.0136862] | 0.0253193 [0.0249346,0.0283067] | 0.5354 | 0.4806–0.5458 | -0.0117634 | 1.947e-15 |
| 32 | 32 | GL 33×65 odd_minimum | analysis | 1 | 0.0437588 [0.0431846,0.044176] | 0.0215613 [0.0214474,0.0217] | 2.0295 | 2.0016–2.0456 | 0.0221975 | 1.581e-15 |
| 32 | 32 | GL 33×65 odd_minimum | analysis | 8 | 0.0216121 [0.0208287,0.0220466] | 0.0309222 [0.0306101,0.0315166] | 0.6989 | 0.6702–0.7116 | -0.00931017 | 1.581e-15 |
| 32 | 32 | GL 33×65 odd_minimum | synthesis | 1 | 0.0379249 [0.037645,0.0382574] | 0.0212616 [0.0211089,0.0217503] | 1.7837 | 1.7364–1.8055 | 0.0166633 | 5.976e-15 |
| 32 | 32 | GL 33×65 odd_minimum | synthesis | 8 | 0.0238688 [0.0235202,0.0243475] | 0.0299881 [0.0297441,0.0303754] | 0.7959 | 0.7822–0.8184 | -0.00611932 | 5.976e-15 |
| 32 | 32 | GL 33×70 even_smooth | analysis | 1 | 0.0439868 [0.043616,0.0447066] | 0.0218044 [0.0215867,0.0219042] | 2.0173 | 1.9912–2.0605 | 0.0221824 | 1.535e-15 |
| 32 | 32 | GL 33×70 even_smooth | analysis | 8 | 0.0215291 [0.0213011,0.0217441] | 0.031433 [0.0309784,0.0326302] | 0.6849 | 0.6546–0.6952 | -0.00990388 | 1.535e-15 |
| 32 | 32 | GL 33×70 even_smooth | synthesis | 1 | 0.03431 [0.0339827,0.0346164] | 0.0221221 [0.0219867,0.0223119] | 1.5509 | 1.5374–1.5660 | 0.0121879 | 4.304e-15 |
| 32 | 32 | GL 33×70 even_smooth | synthesis | 8 | 0.0197171 [0.0195134,0.0201537] | 0.0299387 [0.0297121,0.0302864] | 0.6586 | 0.6480–0.6706 | -0.0102216 | 4.304e-15 |
| 255 | 255 | GL 256×511 odd_minimum | analysis | 1 | 1.93789 [1.92622,1.96701] | 2.09244 [2.02745,2.24928] | 0.9261 | 0.8675–0.9711 | -0.154545 | 1.536e-14 |
| 255 | 255 | GL 256×511 odd_minimum | analysis | 8 | 0.796948 [0.771105,1.01223] | 0.333659 [0.328223,0.349284] | 2.3885 | 2.2690–3.0678 | 0.463288 | 1.721e-14 |
| 255 | 255 | GL 256×511 odd_minimum | synthesis | 1 | 5.83109 [5.80011,6.10716] | 2.2094 [2.19535,2.28065] | 2.6392 | 2.5918–2.7772 | 3.62169 | 8.962e-14 |
| 255 | 255 | GL 256×511 odd_minimum | synthesis | 8 | 2.38647 [2.34644,2.41689] | 0.351687 [0.34233,0.369436] | 6.7858 | 6.4382–7.0288 | 2.03478 | 8.962e-14 |
| 255 | 255 | GL 256×512 even_smooth | analysis | 1 | 1.52316 [1.51132,1.5487] | 1.20827 [1.19325,1.61253] | 1.2606 | 0.9345–1.2854 | 0.314888 | 1.513e-14 |
| 255 | 255 | GL 256×512 even_smooth | analysis | 8 | 0.709565 [0.69132,0.793014] | 0.221484 [0.220346,0.227898] | 3.2037 | 3.1013–3.5722 | 0.488081 | 1.666e-14 |
| 255 | 255 | GL 256×512 even_smooth | synthesis | 1 | 4.94396 [4.92777,5.11251] | 1.36345 [1.35917,1.44568] | 3.6261 | 3.4207–3.7468 | 3.58051 | 8.976e-14 |
| 255 | 255 | GL 256×512 even_smooth | synthesis | 8 | 1.48934 [1.45822,1.53696] | 0.232455 [0.228542,0.242483] | 6.4070 | 6.1223–6.6267 | 1.25689 | 8.976e-14 |
| 256 | 256 | GL 257×513 odd_minimum | analysis | 1 | 1.72628 [1.70988,1.79872] | 1.3275 [1.30011,1.37465] | 1.3004 | 1.2595–1.3573 | 0.398778 | 1.573e-14 |
| 256 | 256 | GL 257×513 odd_minimum | analysis | 8 | 0.394965 [0.39174,0.424307] | 0.238928 [0.237069,0.246783] | 1.6531 | 1.6310–1.7416 | 0.156037 | 1.573e-14 |
| 256 | 256 | GL 257×513 odd_minimum | synthesis | 1 | 1.99214 [1.98081,2.3285] | 1.45654 [1.44168,1.51388] | 1.3677 | 1.3132–1.6158 | 0.535595 | 1.280e-14 |
| 256 | 256 | GL 257×513 odd_minimum | synthesis | 8 | 0.48339 [0.477944,0.495949] | 0.247352 [0.244104,0.258907] | 1.9543 | 1.8982–2.0045 | 0.236038 | 1.280e-14 |
| 256 | 256 | GL 257×540 even_smooth | analysis | 1 | 1.61255 [1.60515,1.66383] | 1.26301 [1.24014,1.31693] | 1.2767 | 1.2356–1.3056 | 0.349536 | 1.564e-14 |
| 256 | 256 | GL 257×540 even_smooth | analysis | 8 | 0.374144 [0.370888,0.392879] | 0.230213 [0.228341,0.238388] | 1.6252 | 1.5916–1.6751 | 0.143931 | 1.564e-14 |
| 256 | 256 | GL 257×540 even_smooth | synthesis | 1 | 1.64479 [1.63613,1.82307] | 1.44813 [1.39029,1.45755] | 1.1358 | 1.1258–1.2579 | 0.196658 | 1.266e-14 |
| 256 | 256 | GL 257×540 even_smooth | synthesis | 8 | 0.419341 [0.415022,0.43735] | 0.234594 [0.233434,0.241366] | 1.7875 | 1.7361–1.8617 | 0.184746 | 1.266e-14 |
| 511 | 511 | GL 512×1023 odd_minimum | analysis | 1 | 8.59197 [8.53766,8.88791] | 6.39074 [6.34442,7.40494] | 1.3444 | 1.1553–1.3854 | 2.20123 | 2.785e-14 |
| 511 | 511 | GL 512×1023 odd_minimum | analysis | 8 | 1.70405 [1.6726,1.87559] | 0.993103 [0.976315,1.01986] | 1.7159 | 1.6801–1.9175 | 0.710945 | 2.785e-14 |
| 511 | 511 | GL 512×1023 odd_minimum | synthesis | 1 | 11.1501 [11.1048,11.8665] | 8.28201 [7.99003,8.39538] | 1.3463 | 1.3242–1.4216 | 2.86808 | 3.599e-14 |
| 511 | 511 | GL 512×1023 odd_minimum | synthesis | 8 | 2.25951 [2.24096,2.50987] | 1.16589 [1.15969,1.21496] | 1.9380 | 1.8580–2.1561 | 1.09362 | 3.599e-14 |
| 511 | 511 | GL 512×1024 even_smooth | analysis | 1 | 6.80086 [6.74284,7.24388] | 5.87648 [5.83007,8.12951] | 1.1573 | 0.8407–1.2242 | 0.924379 | 2.795e-14 |
| 511 | 511 | GL 512×1024 even_smooth | analysis | 8 | 1.34201 [1.32952,1.38557] | 0.909197 [0.901914,0.941697] | 1.4760 | 1.4211–1.5239 | 0.432817 | 2.795e-14 |
| 511 | 511 | GL 512×1024 even_smooth | synthesis | 1 | 8.30535 [8.24014,8.79207] | 7.60511 [7.53743,8.02004] | 1.0921 | 1.0339–1.1417 | 0.700246 | 3.486e-14 |
| 511 | 511 | GL 512×1024 even_smooth | synthesis | 8 | 1.93019 [1.91588,1.96766] | 1.11697 [1.10866,1.13014] | 1.7281 | 1.7059–1.7582 | 0.813221 | 3.486e-14 |
| 512 | 512 | GL 513×1025 odd_minimum | analysis | 1 | 8.73049 [8.6086,9.96033] | 6.52751 [6.43148,6.64007] | 1.3375 | 1.3071–1.5144 | 2.20298 | 2.594e-14 |
| 512 | 512 | GL 513×1025 odd_minimum | analysis | 8 | 1.63299 [1.5701,1.75233] | 1.00186 [0.99548,1.00657] | 1.6300 | 1.5671–1.7459 | 0.631126 | 2.594e-14 |
| 512 | 512 | GL 513×1025 odd_minimum | synthesis | 1 | 11.0424 [10.9791,11.2125] | 8.24495 [8.14291,8.43896] | 1.3393 | 1.3038–1.3641 | 2.7974 | 2.970e-14 |
| 512 | 512 | GL 513×1025 odd_minimum | synthesis | 8 | 2.28082 [2.25911,2.39361] | 1.20886 [1.19651,1.23561] | 1.8868 | 1.8431–1.9780 | 1.07196 | 2.970e-14 |
| 512 | 512 | GL 513×1050 even_smooth | analysis | 1 | 7.34063 [7.29457,7.86203] | 6.00382 [5.94097,6.25092] | 1.2227 | 1.1751–1.2923 | 1.33681 | 2.592e-14 |
| 512 | 512 | GL 513×1050 even_smooth | analysis | 8 | 1.40278 [1.3817,1.5547] | 0.939794 [0.921744,0.993169] | 1.4926 | 1.4220–1.5837 | 0.462981 | 2.592e-14 |
| 512 | 512 | GL 513×1050 even_smooth | synthesis | 1 | 8.74723 [8.70236,9.82705] | 7.82422 [7.70335,7.96716] | 1.1180 | 1.0983–1.2567 | 0.92301 | 3.225e-14 |
| 512 | 512 | GL 513×1050 even_smooth | synthesis | 8 | 1.9721 [1.94937,2.00534] | 1.14979 [1.14125,1.17055] | 1.7152 | 1.6810–1.7613 | 0.822309 | 3.225e-14 |
| 513 | 513 | GL 514×1027 odd_minimum | analysis | 1 | 11.6905 [11.5793,12.0288] | 7.88274 [7.73933,8.28023] | 1.4830 | 1.4113–1.5272 | 3.80775 | 2.673e-14 |
| 513 | 513 | GL 514×1027 odd_minimum | analysis | 8 | 2.01341 [1.98181,2.26073] | 1.16589 [1.15924,1.19422] | 1.7269 | 1.6915–1.9132 | 0.847524 | 2.673e-14 |
| 513 | 513 | GL 514×1027 odd_minimum | synthesis | 1 | 14.0891 [13.9949,14.3215] | 9.6948 [9.59582,11.3704] | 1.4533 | 1.2380–1.4778 | 4.39427 | 2.515e-14 |
| 513 | 513 | GL 514×1027 odd_minimum | synthesis | 8 | 2.76323 [2.73994,2.89622] | 1.40232 [1.39308,1.44056] | 1.9705 | 1.9340–2.0674 | 1.36091 | 2.515e-14 |
| 513 | 513 | GL 514×1050 even_smooth | analysis | 1 | 7.34475 [7.26143,7.47156] | 5.94934 [5.91466,6.13405] | 1.2345 | 1.1986–1.2444 | 1.39541 | 2.648e-14 |
| 513 | 513 | GL 514×1050 even_smooth | analysis | 8 | 1.39575 [1.38294,1.47884] | 0.926384 [0.918861,0.975691] | 1.5067 | 1.4293–1.5671 | 0.469371 | 2.648e-14 |
| 513 | 513 | GL 514×1050 even_smooth | synthesis | 1 | 8.8213 [8.73685,9.17797] | 8.00391 [7.7752,8.25901] | 1.1021 | 1.0636–1.1433 | 0.817383 | 2.868e-14 |
| 513 | 513 | GL 514×1050 even_smooth | synthesis | 8 | 2.00478 [1.96052,2.06287] | 1.14463 [1.13783,1.15806] | 1.7515 | 1.7078–1.8075 | 0.860147 | 2.868e-14 |
| 720 | 1 | GL 721×3 odd_minimum | analysis | 1 | 0.101572 [0.100248,0.102455] | 0.132764 [0.131869,0.133736] | 0.7651 | 0.7555–0.7736 | -0.0311919 | 5.277e-14 |
| 720 | 1 | GL 721×3 odd_minimum | analysis | 8 | 0.0784856 [0.0774614,0.0801554] | 0.131369 [0.130228,0.134358] | 0.5974 | 0.5793–0.6148 | -0.0528834 | 5.277e-14 |
| 720 | 1 | GL 721×3 odd_minimum | synthesis | 1 | 0.143119 [0.142599,0.143983] | 0.14961 [0.14853,0.153113] | 0.9566 | 0.9352–0.9653 | -0.00649008 | 1.077e-13 |
| 720 | 1 | GL 721×3 odd_minimum | synthesis | 8 | 0.14388 [0.142949,0.151755] | 0.131469 [0.129438,0.142338] | 1.0944 | 1.0417–1.1142 | 0.0124111 | 1.077e-13 |
| 720 | 1 | GL 721×4 even_smooth | analysis | 1 | 0.106459 [0.106002,0.10734] | 0.134085 [0.133108,0.135185] | 0.7940 | 0.7906–0.7991 | -0.0276263 | 6.121e-14 |
| 720 | 1 | GL 721×4 even_smooth | analysis | 8 | 0.0873277 [0.0862406,0.0886845] | 0.131927 [0.13051,0.134666] | 0.6619 | 0.6422–0.6755 | -0.0445998 | 6.121e-14 |
| 720 | 1 | GL 721×4 even_smooth | synthesis | 1 | 0.141554 [0.14112,0.142486] | 0.151123 [0.150026,0.156498] | 0.9367 | 0.9087–0.9445 | -0.00956931 | 1.841e-13 |
| 720 | 1 | GL 721×4 even_smooth | synthesis | 8 | 0.142488 [0.141282,0.142803] | 0.127078 [0.126358,0.127764] | 1.1213 | 1.1101–1.1293 | 0.0154091 | 1.841e-13 |
| 720 | 180 | GL 721×361 odd_minimum | analysis | 1 | 7.80035 [7.7749,7.89586] | 6.01065 [5.98632,6.03152] | 1.2978 | 1.2925–1.3163 | 1.78971 | 4.891e-14 |
| 720 | 180 | GL 721×361 odd_minimum | analysis | 8 | 1.31111 [1.29232,1.33229] | 0.940098 [0.927687,0.951941] | 1.3947 | 1.3711–1.4246 | 0.371013 | 4.891e-14 |
| 720 | 180 | GL 721×361 odd_minimum | synthesis | 1 | 10.9738 [10.9343,11.0458] | 8.53418 [8.46054,8.6384] | 1.2859 | 1.2651–1.3089 | 2.43966 | 5.195e-14 |
| 720 | 180 | GL 721×361 odd_minimum | synthesis | 8 | 2.33989 [2.31996,2.35839] | 1.25243 [1.24208,1.2604] | 1.8683 | 1.8505–1.9021 | 1.08746 | 5.195e-14 |
| 720 | 180 | GL 721×378 even_smooth | analysis | 1 | 6.37945 [6.31445,6.40674] | 5.86401 [5.81307,5.88651] | 1.0879 | 1.0745–1.1004 | 0.515436 | 4.909e-14 |
| 720 | 180 | GL 721×378 even_smooth | analysis | 8 | 1.10678 [1.09909,1.11885] | 0.954508 [0.910117,1.002] | 1.1595 | 1.1070–1.2153 | 0.152274 | 4.909e-14 |
| 720 | 180 | GL 721×378 even_smooth | synthesis | 1 | 8.98433 [8.97028,9.02898] | 8.25881 [8.15491,8.42037] | 1.0878 | 1.0674–1.1031 | 0.725523 | 4.707e-14 |
| 720 | 180 | GL 721×378 even_smooth | synthesis | 8 | 2.08741 [2.07249,2.15785] | 1.22187 [1.20415,1.25349] | 1.7084 | 1.6822–1.7569 | 0.865543 | 4.707e-14 |
| 720 | 720 | GL 721×1441 odd_minimum | analysis | 1 | 22.0535 [21.9586,22.1775] | 22.1456 [21.8938,22.3064] | 0.9958 | 0.9902–1.0099 | -0.0920818 | 4.630e-14 |
| 720 | 720 | GL 721×1441 odd_minimum | analysis | 8 | 4.01913 [3.97338,4.10302] | 3.17166 [3.14555,3.22128] | 1.2672 | 1.2412–1.2822 | 0.84747 | 4.630e-14 |
| 720 | 720 | GL 721×1441 odd_minimum | synthesis | 1 | 28.1459 [28.064,28.5548] | 27.483 [27.2492,27.8074] | 1.0241 | 1.0149–1.0371 | 0.662902 | 4.448e-14 |
| 720 | 720 | GL 721×1441 odd_minimum | synthesis | 8 | 5.71627 [5.67652,5.78445] | 3.87159 [3.84651,3.8958] | 1.4765 | 1.4619–1.4965 | 1.84469 | 4.448e-14 |
| 720 | 720 | GL 721×1458 even_smooth | analysis | 1 | 16.1498 [16.0771,16.2797] | 14.4771 [14.341,14.6267] | 1.1155 | 1.1044–1.1266 | 1.67279 | 4.632e-14 |
| 720 | 720 | GL 721×1458 even_smooth | analysis | 8 | 2.94423 [2.90587,2.98319] | 2.11833 [2.10541,2.14134] | 1.3899 | 1.3710–1.4172 | 0.825895 | 4.632e-14 |
| 720 | 720 | GL 721×1458 even_smooth | synthesis | 1 | 20.3774 [20.3144,20.5796] | 18.7669 [18.5886,18.9425] | 1.0858 | 1.0769–1.0959 | 1.61049 | 4.405e-14 |
| 720 | 720 | GL 721×1458 even_smooth | synthesis | 8 | 4.59441 [4.54999,4.81814] | 2.68371 [2.66466,2.6972] | 1.7120 | 1.6927–1.7963 | 1.9107 | 4.405e-14 |
| 721 | 721 | GL 722×1443 odd_minimum | analysis | 1 | 22.4384 [22.2569,22.6313] | 14.8108 [14.688,14.9386] | 1.5150 | 1.4994–1.5341 | 7.62751 | 4.292e-14 |
| 721 | 721 | GL 722×1443 odd_minimum | analysis | 8 | 3.98243 [3.94404,4.03548] | 2.16514 [2.15662,2.18086] | 1.8393 | 1.8178–1.8636 | 1.81729 | 4.292e-14 |
| 721 | 721 | GL 722×1443 odd_minimum | synthesis | 1 | 28.5245 [28.4686,28.6939] | 19.7736 [19.5187,20.1247] | 1.4426 | 1.4236–1.4621 | 8.75085 | 4.954e-14 |
| 721 | 721 | GL 722×1443 odd_minimum | synthesis | 8 | 5.8267 [5.78358,6.21662] | 2.79429 [2.78075,2.81258] | 2.0852 | 2.0606–2.2225 | 3.03241 | 4.954e-14 |
| 721 | 721 | GL 722×1458 even_smooth | analysis | 1 | 16.36 [16.2593,16.6368] | 14.5267 [14.3307,14.6515] | 1.1262 | 1.1179–1.1443 | 1.83322 | 4.310e-14 |
| 721 | 721 | GL 722×1458 even_smooth | analysis | 8 | 2.96462 [2.91497,3.15815] | 2.12219 [2.11115,2.13911] | 1.3970 | 1.3790–1.4860 | 0.842431 | 4.310e-14 |
| 721 | 721 | GL 722×1458 even_smooth | synthesis | 1 | 20.4531 [20.3647,20.6061] | 18.8525 [18.7247,19.1639] | 1.0849 | 1.0656–1.0972 | 1.60062 | 4.958e-14 |
| 721 | 721 | GL 722×1458 even_smooth | synthesis | 8 | 4.60188 [4.56385,4.82475] | 2.68795 [2.67423,2.70218] | 1.7120 | 1.6926–1.8054 | 1.91394 | 4.958e-14 |
| 1024 | 1024 | GL 1025×2049 odd_minimum | analysis | 1 | 62.594 [62.3521,63.6795] | 71.197 [70.907,71.4477] | 0.8792 | 0.8742–0.8915 | -8.60298 | 5.465e-14 |
| 1024 | 1024 | GL 1025×2049 odd_minimum | analysis | 8 | 10.7479 [10.5713,13.4666] | 10.0342 [9.94328,10.2848] | 1.0711 | 1.0557–1.2873 | 0.713712 | 5.465e-14 |
| 1024 | 1024 | GL 1025×2049 odd_minimum | synthesis | 1 | 80.6782 [80.213,81.7175] | 85.6628 [84.989,86.8792] | 0.9418 | 0.9221–0.9570 | -4.98463 | 5.298e-14 |
| 1024 | 1024 | GL 1025×2049 odd_minimum | synthesis | 8 | 15.9621 [15.8244,16.1619] | 11.8827 [11.8106,12.1587] | 1.3433 | 1.3150–1.3637 | 4.07942 | 5.298e-14 |
| 1024 | 1024 | GL 1025×2058 even_smooth | analysis | 1 | 39.6537 [39.4539,39.9914] | 34.9265 [34.6165,35.2054] | 1.1353 | 1.1248–1.1490 | 4.72729 | 5.474e-14 |
| 1024 | 1024 | GL 1025×2058 even_smooth | analysis | 8 | 7.42145 [7.29779,7.60562] | 5.04831 [4.99587,5.08047] | 1.4701 | 1.4510–1.5045 | 2.37314 | 5.474e-14 |
| 1024 | 1024 | GL 1025×2058 even_smooth | synthesis | 1 | 53.2101 [52.7179,54.2587] | 49.4631 [49.072,50.5219] | 1.0758 | 1.0516–1.0970 | 3.74699 | 5.279e-14 |
| 1024 | 1024 | GL 1025×2058 even_smooth | synthesis | 8 | 12.6495 [12.5237,12.8165] | 7.0167 [6.98252,7.07199] | 1.8028 | 1.7800–1.8276 | 5.63279 | 5.279e-14 |
| 1536 | 1536 | GL 1537×3073 odd_minimum | analysis | 1 | 143.804 [143.288,144.599] | 188.705 [187.332,189.663] | 0.7621 | 0.7563–0.7693 | -44.9011 | 9.849e-14 |
| 1536 | 1536 | GL 1537×3073 odd_minimum | analysis | 8 | 25.8197 [25.1757,26.8512] | 29.27 [28.7851,30.0484] | 0.8821 | 0.8633–0.9148 | -3.45032 | 9.849e-14 |
| 1536 | 1536 | GL 1537×3073 odd_minimum | synthesis | 1 | 196.905 [196.528,197.32] | 241.045 [237.054,243.8] | 0.8169 | 0.8061–0.8313 | -44.1398 | 1.807e-13 |
| 1536 | 1536 | GL 1537×3073 odd_minimum | synthesis | 8 | 44.0528 [43.8124,44.4245] | 35.8243 [35.0748,36.3575] | 1.2297 | 1.2094–1.2537 | 8.22846 | 1.807e-13 |
| 1536 | 1536 | GL 1537×3136 even_smooth | analysis | 1 | 114.125 [113.452,115.689] | 115.676 [114.381,117.202] | 0.9866 | 0.9744–1.0042 | -1.55053 | 9.877e-14 |
| 1536 | 1536 | GL 1537×3136 even_smooth | analysis | 8 | 22.4877 [22.1225,24.1137] | 20.4695 [19.9026,23.6372] | 1.0986 | 0.9616–1.1938 | 2.01817 | 9.877e-14 |
| 1536 | 1536 | GL 1537×3136 even_smooth | synthesis | 1 | 159.2 [158.724,163.776] | 166.296 [164.44,167.26] | 0.9573 | 0.9516–0.9888 | -7.09567 | 1.755e-13 |
| 1536 | 1536 | GL 1537×3136 even_smooth | synthesis | 8 | 39.6156 [39.2525,40.8332] | 26.7789 [26.3459,27.3861] | 1.4794 | 1.4483–1.5268 | 12.8367 | 1.755e-13 |
| 2048 | 512 | GL 2049×1050 even_smooth | analysis | 1 | 277.61 [273.672,287.401] | 98.4001 [97.8842,102.894] | 2.8212 | 2.7347–2.8955 | 179.21 | 1.261e-13 |
| 2048 | 512 | GL 2049×1050 even_smooth | analysis | 8 | 41.4816 [41.1107,43.0748] | 13.8136 [13.6503,14.4899] | 3.0029 | 2.8887–3.0972 | 27.668 | 1.261e-13 |
| 2048 | 512 | GL 2049×1050 even_smooth | synthesis | 1 | 171.291 [168.043,182.147] | 157.008 [154.589,160.086] | 1.0910 | 1.0700–1.1717 | 14.283 | 1.411e-13 |
| 2048 | 512 | GL 2049×1050 even_smooth | synthesis | 8 | 37.9936 [37.3617,39.0432] | 21.5451 [21.4605,21.8432] | 1.7634 | 1.7218–1.8139 | 16.4485 | 1.411e-13 |
| 2048 | 2048 | GL 2049×4116 even_smooth | analysis | 1 | 824.803 [817.356,867.789] | 249.042 [247.964,297.222] | 3.3119 | 2.7852–3.4879 | 575.761 | 1.181e-13 |
| 2048 | 2048 | GL 2049×4116 even_smooth | analysis | 8 | 134.036 [133.637,134.499] | 41.1791 [40.2437,42.4241] | 3.2550 | 3.1675–3.3282 | 92.8571 | 1.181e-13 |
| 2048 | 2048 | GL 2049×4116 even_smooth | synthesis | 1 | 357.592 [355.348,362.952] | 361.299 [352.694,364.614] | 0.9897 | 0.9745–1.0169 | -3.7069 | 1.195e-13 |
| 2048 | 2048 | GL 2049×4116 even_smooth | synthesis | 8 | 86.1311 [85.6077,86.834] | 57.5482 [56.0607,62.0003] | 1.4967 | 1.3858–1.5462 | 28.5828 | 1.195e-13 |
| 3072 | 3072 | GL 3073×6174 even_smooth | analysis | 1 | 6068.32 [6055.34,6107.86] | 723.528 [720.422,731.519] | 8.3871 | 8.3044–8.4480 | 5344.79 | 1.961e-13 |
| 3072 | 3072 | GL 3073×6174 even_smooth | analysis | 8 | 868.699 [858.969,880.833] | 116.79 [114.648,120.316] | 7.4381 | 7.3049–7.5629 | 751.909 | 1.961e-13 |
| 3072 | 3072 | GL 3073×6174 even_smooth | synthesis | 1 | 1118.87 [1113.46,1137.5] | 1135.38 [1096.64,1141.18] | 0.9855 | 0.9786–1.0195 | -16.514 | 2.114e-13 |
| 3072 | 3072 | GL 3073×6174 even_smooth | synthesis | 8 | 262.684 [262.039,264.343] | 175.605 [172.593,181.967] | 1.4959 | 1.4443–1.5239 | 87.0786 | 2.114e-13 |

Across 856 warm rows, RustSHT was faster in 292 where the paired p10–p90 ratio interval stayed below 1, slower in 552 where it stayed above 1, and inconclusive in 12 where the interval crossed 1. This classification follows observed variation rather than a fixed parity band.
### Cross-bandlimit findings

For even-smooth full-mmax GL, q1 RustSHT was near DUCC at T1536 (analysis 0.99×; synthesis 0.96×). At T2048, analysis rose to 3.31× q1 and 3.25× q8, while synthesis was 0.99× q1 and 1.50× q8. At T3072, analysis was 8.39× q1 (5344.8 ms excess) and 7.44× q8 (751.9 ms excess); synthesis was 0.99× q1 and 1.50× q8.
The large high-bandlimit excess is specific to the complete analysis call in these cases: synthesis remains near DUCC at q1, while q8 synthesis is about 1.5× slower. T1536 q1 is near parity, so the measured analysis gap grows sharply in the next sampled size range. End-to-end timings include FFT and preparation; they do not isolate recurrence as the cause. The next comparison should separate analysis recurrence/accumulation from FFT and output work at the high-bandlimit cases.
For operations below 100 ms, the median per-row fraction of measured batches in the requested 20–100 ms window was 100.0%; the lowest row fraction was 38.3%. 836 short-operation rows collected 30 batches per backend. One row median was just below the 20-ms lower bound; its timing distribution is retained without adjustment. Calibration attempts are recorded separately in each process sample.
Maximum measured random-output relative L2 error was 2.536e-13; maximum absolute error was 1.071e-08.
The out-of-timing checks covered 428 configurations. Exact-zero/reset consistency held for 428/428; all edge-case outputs were finite for 428/428. Maximum Rust/DUCC roundtrip relative L2 was 6.273e-13; maximum selected-basis recovery relative L2 was 5.153e-12; maximum adjoint identity relative residual was 3.427e-12.
Available host-state summary: frequency median/p10/p90 4.467/1.746/4.766 GHz; scheduler wait median/p90 0.00319/0.2781; involuntary-switch median/max 95/141251. This indicates intermittent scheduling/frequency interference despite low median scheduler wait. No samples were dropped; paired distributions retain the resulting uncertainty.

### Setup and cold-start costs

The machine-readable rows retain per-process fixture loading, output-buffer preparation, Rust plan/workspace setup, first-call latency, warmed batches, actual batch wall times and the fraction of short-operation batches within 20–100 ms. Rust creates its plan and workspace before timing; first-call latency reflects the native cold numerical route and runtime initialization. DUCC has no reusable 2D plan at this API boundary, so its internal allocation and transform preparation remain in each call. Each worker verifies that repeated calls overwrite outputs consistently with the first call.

### Plots

- [Analysis ratio by lmax](data/ducc-full-transform-parity-2026-10-20/ratio-analysis-full-mmax.svg)
- [Synthesis ratio by lmax](data/ducc-full-transform-parity-2026-10-20/ratio-synthesis-full-mmax.svg)
- [Analysis absolute runtime](data/ducc-full-transform-parity-2026-10-20/runtime-analysis.svg)
- [Synthesis absolute runtime](data/ducc-full-transform-parity-2026-10-20/runtime-synthesis.svg)
- [Analysis q1/q8 speedup](data/ducc-full-transform-parity-2026-10-20/thread-speedup-analysis.svg)
- [Synthesis q1/q8 speedup](data/ducc-full-transform-parity-2026-10-20/thread-speedup-synthesis.svg)
- [Full/reduced mmax analysis](data/ducc-full-transform-parity-2026-10-20/full-reduced-mmax-analysis.svg)
- [Full/reduced mmax synthesis](data/ducc-full-transform-parity-2026-10-20/full-reduced-mmax-synthesis.svg)
- [Rust analysis first/warmed latency](data/ducc-full-transform-parity-2026-10-20/cold-warm-analysis-rust-q1.svg)

## Measured scope and limitations

The largest completed degree is T3072. The sweep includes exhaustive lmax 0–32, dense low/intermediate sampling, required T255/T256/T511/T512/T513/T720/T721/T1024/T1536/T2048, and T3072 where resources permitted. f64 scalar GL is the full matrix; other precisions, spins, CC and F1 are outside this benchmark. Existing stage-level scalar-analysis measurements remain in `PERFORMANCE_STATUS.md` and the linked active profile; this run times complete native calls including FFT, Legendre work, packing/conversion and output writes.

The Rust/DUCC output comparison validates identical deterministic random maps and complete-triangle coefficient arrays in every timed configuration at q1 and q8. For each configuration and thread count, the first q1/q8 analysis process also runs out-of-timing roundtrip, zonal/intermediate/sectoral basis, zero, subnormal, extreme-finite and analysis/synthesis adjoint checks. Their per-case results are in `results.json`; neither set of checks is inferred from roundtrip alone.

## Reproduction

```sh
RUSTFLAGS='-C target-cpu=x86-64-v3 -Aunexpected_cfgs' cargo build --release --example ducc_full_transform_parity --features parallel
cmake -S reports/data/ducc-full-transform-parity-2026-10-20 -B "${DUCC_BUILD_DIR:-ducc-build}" -DDUCC_SRC="${DUCC_SRC:?set DUCC_SRC to the pinned checkout}" -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=g++
cmake --build "${DUCC_BUILD_DIR:-ducc-build}" --parallel 4
python3 scripts/benchmark_ducc_full_transform_parity.py --ducc-src "${DUCC_SRC:?set DUCC_SRC to the pinned checkout}" --skip-build
```
