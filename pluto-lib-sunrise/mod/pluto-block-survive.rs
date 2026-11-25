fn pluto_block_survive(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, survives: &mut Option<bool>) {
    super::pluto_block_level::survives(level, pos, state, survives);
}
