fn pluto_world_fluid_neighbor(
    _value: (),
    level: &crate::game::GameLevel<'_>,
    ticks: &mut sunrise_world::level::ticks::Ticks,
    pos: sunrise_core::position::BlockPos,
    state: sunrise_world::registry::BlockStateId,
) {
    crate::game::pluto_world_fluid::neighbor(level, ticks, pos, state);
}
