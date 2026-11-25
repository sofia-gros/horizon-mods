fn pluto_block_bonemeal_valid(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, valid: &mut Option<bool>) {
    super::pluto_block_level::bonemeal_valid(level, pos, state, valid);
}
