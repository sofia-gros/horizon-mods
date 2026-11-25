fn pluto_block_random_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    super::pluto_block_level::random_tick(level, ticks, pos, state, handled);
}
