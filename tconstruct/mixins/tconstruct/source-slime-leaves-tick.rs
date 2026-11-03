fn tconstruct_slime_leaves_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    super::tconstruct_slime_leaves::tick(level, ticks, pos, state, handled);
}
