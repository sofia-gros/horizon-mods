fn tconstruct_gauge_update(level: &crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId, direction: horizon_core::geometry::Direction,
    new: &mut Option<horizon_world::registry::BlockStateId>) {
    crate::game::tconstruct_source_gauge::update(level, pos, state, direction, new);
}
