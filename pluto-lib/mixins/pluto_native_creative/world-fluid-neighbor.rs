fn pluto_world_fluid_neighbor(
    _value: (),
    level: &crate::game::GameLevel<'_>,
    ticks: &mut horizon_world::level::ticks::Ticks,
    pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId,
) {
    crate::game::pluto_world_fluid::neighbor(level, ticks, pos, state);
}
