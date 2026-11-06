fn pluto_world_fluid_tick(
    level: &mut crate::game::GameLevel<'_>,
    ticks: &mut horizon_world::level::ticks::Ticks,
    pos: horizon_core::position::BlockPos,
    state: horizon_world::registry::BlockStateId,
    handled: &mut bool,
) {
    crate::game::pluto_world_fluid::tick(level, ticks, pos, state, handled);
}
