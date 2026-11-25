fn pluto_block_scheduled_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    super::pluto_block_level::scheduled_tick(level, ticks, pos, state, handled);
}
