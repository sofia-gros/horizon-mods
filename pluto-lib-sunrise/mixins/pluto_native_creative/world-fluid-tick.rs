fn pluto_world_fluid_tick(
    level: &mut crate::game::GameLevel<'_>,
    ticks: &mut sunrise_world::level::ticks::Ticks,
    pos: sunrise_core::position::BlockPos,
    state: sunrise_world::registry::BlockStateId,
    handled: &mut bool,
) {
    crate::game::pluto_world_fluid::tick(level, ticks, pos, state, handled);
}
