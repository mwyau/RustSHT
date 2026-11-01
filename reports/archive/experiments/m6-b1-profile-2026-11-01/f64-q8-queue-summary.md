# B1 f64 q8 order-queue timing summary

Source: `255e34d4a078c42ffa5de3616c73f6eca173f472`; pinned GL T720 full-order q8 analysis; eight Rayon workers; 721 order jobs per call.

Across 30 instrumented calls, the median queue-lock time was 42.084 microseconds. The median uninstrumented whole-call wall time from the adjacent control was 2.245 milliseconds, so queue locking was about 1.9% of that probe. Workers handled 83–91 jobs each. This argues against queue locking as the explanation for the larger f64 q8 gap; it does not measure all scheduling costs.

The original raw test log is retained only in the local diagnostic scratch. This committed summary is derived from that log.
