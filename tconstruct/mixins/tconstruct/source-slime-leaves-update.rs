fn tconstruct_slime_leaves_update(level: &GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, neighbor: BlockStateId, new: &mut Option<BlockStateId>) {
    super::tconstruct_slime_leaves::update_shape(level, ticks, pos, state, neighbor, new);
}
