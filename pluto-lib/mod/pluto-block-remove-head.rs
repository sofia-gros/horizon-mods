fn pluto_block_remove_head(level: &mut crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos, old: horizon_world::registry::BlockStateId, side_effects: bool) {
    // The block entity is still in the world at the head of its removal: capture what its
    // type keeps outside its slots now, and let the drain at the end of the level operation
    // run its removal hook over the capture (see pluto-block-remove).
    crate::game::pluto_block_remove::capture(level, pos, old, side_effects);
}
