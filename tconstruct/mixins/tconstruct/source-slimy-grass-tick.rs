// Source SlimeGrassBlock and SlimeNyliumBlock random ticks.
fn tconstruct_slimy_grass_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    super::tconstruct_native_hook_bodies::tconstruct_slimy_grass_tick(level, ticks, pos, state, handled);
}
