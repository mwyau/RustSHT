use super::{SCALAR_ANALYSIS_PAIR_TILE_SIZE, SCALAR_ORDER_BATCH_SIZE, recurrence_scratch_len};

#[allow(unexpected_cfgs)]
#[test]
fn scalar_analysis_pair_tile_matches_build_configuration() {
    assert_eq!(SCALAR_ANALYSIS_PAIR_TILE_SIZE, 256);
}

#[test]
fn recurrence_scratch_covers_scalar_analysis_pair_tile() {
    let expected = SCALAR_ORDER_BATCH_SIZE.max(SCALAR_ANALYSIS_PAIR_TILE_SIZE);
    assert_eq!(recurrence_scratch_len(721), expected);
    assert!(recurrence_scratch_len(0) >= SCALAR_ANALYSIS_PAIR_TILE_SIZE);
}
