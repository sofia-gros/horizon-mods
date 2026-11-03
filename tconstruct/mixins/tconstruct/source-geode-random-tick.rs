fn tconstruct_geode_random_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    super::tconstruct_native_hook_bodies::tconstruct_geode_random_tick(level, ticks, pos, state, handled);
}
