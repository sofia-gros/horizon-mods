fn pluto_block_remove_head(game: &mut crate::game::Game<'_>, pos: horizon_core::position::BlockPos, old: horizon_world::registry::BlockStateId, side_effects: bool) {
    // The block entity is still in the world at the head of its removal: its type hears the
    // removal now, and what it keeps outside its slots is dropped before the removal goes on.
    crate::game::pluto_block_remove::dispatch(game, pos, old, side_effects);
}
