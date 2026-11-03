fn tconstruct_sapling_bonemeal_valid(level: &GameLevel<'_>, state: BlockStateId, valid: &mut Option<bool>) {
    if super::tconstruct_sapling_tree::sapling(level,state) {*valid=Some(true);}
}
