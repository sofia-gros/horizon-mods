fn tconstruct_sapling_survive(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, survives: &mut Option<bool>) {
    super::tconstruct_native_hook_bodies::tconstruct_sapling_survive(level, pos, state, survives);
}
