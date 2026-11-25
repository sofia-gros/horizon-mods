fn pluto_block_placement(level: &GameLevel<'_>, state: BlockStateId, context: &PlacementContext, answer: &mut Option<Option<BlockStateId>>) {
    super::pluto_block_level::placement(level, state, context, answer);
}
