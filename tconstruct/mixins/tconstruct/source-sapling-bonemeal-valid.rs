fn tconstruct_sapling_bonemeal_valid(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, valid: &mut Option<bool>) {
    if !super::tconstruct_sapling_tree::sapling(level, state) { return; }
    let stage = level.registry.state(state).and_then(|block| block.properties.get("stage")).map(String::as_str);
    *valid = Some(super::tconstruct_slime_tree_shape::bonemeal_ready(stage, ||
        super::tconstruct_sapling_tree::growth_chunks_loaded(level, pos)));
}
