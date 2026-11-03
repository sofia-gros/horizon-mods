fn tconstruct_slime_leaves_place(level: &GameLevel<'_>, state: BlockStateId, context: &PlacementContext, answer: &mut Option<Option<BlockStateId>>) {
    super::tconstruct_slime_leaves::placement(level, context.pos, state, answer);
}
