# Accuracy replay result

Status: passed focused test-only recurrence replay (208 outputs).

- Fixture: GL T720; orders 1–16; degrees 600–720; rings 0–5, 360, and 715–720.
- The f32 seed, geometry, recurrence tables, and prepared coefficient pairs were held fixed. A widened reference used those same f32 inputs and evaluated the leading contraction and recurrence in f64 with fused operations; all paths accumulated in f64 to isolate recurrence arithmetic.
- Ordinary f32 contraction and state relative RMS error: `1.644841959e-4`.
- Double-single state with f32-rounded leading contraction: `1.634344137e-4` versus the full reference; its state-only error against a same-lead reference was `5.907280717e-13`.
- Double-single leading contraction and state: `1.200190783e-12` relative RMS error; maximum absolute error `2.488634299e-14`.

Both the leading contraction and recurrence update need compensation on this fixture. This is a focused recurrence diagnostic, not an independent end-to-end transform check.
