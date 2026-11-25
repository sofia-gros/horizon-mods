// Reuse the native cached-collision face analysis, including its empty result
// for dynamic shapes. Do not substitute sturdy-face or full-block flags.
pub fn full_faces(shape: &horizon_core::geometry::VoxelShape) -> Option<u8> {
    super::analyse_shape(shape).ok().map(|info| info.full_faces)
}
