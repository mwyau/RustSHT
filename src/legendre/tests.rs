use super::{
    SCALAR_ANALYSIS_PAIR_TILE_SIZE, SCALAR_ORDER_BATCH_SIZE, ShtScalar, recurrence_scratch_len,
};

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

#[test]
fn scalar_analysis_pair_tiles_are_precision_specific_and_simd_aligned() {
    assert_eq!(
        <f64 as ShtScalar>::ANALYSIS_PAIR_TILE_SIZE,
        SCALAR_ANALYSIS_PAIR_TILE_SIZE
    );
    let f32_tile = <f32 as ShtScalar>::ANALYSIS_PAIR_TILE_SIZE;
    let f32_lanes = <f32 as ShtScalar>::scalar_paired_analysis_lane_width(pulp::Arch::new());
    assert_eq!(f32_tile, 32);
    assert!(f32_tile <= SCALAR_ANALYSIS_PAIR_TILE_SIZE);
    assert_eq!(f32_tile % f32_lanes, 0);
}
