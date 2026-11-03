fn tconstruct_slime_grass_meal_valid(
    level: &GameLevel<'_>,
    pos: BlockPos,
    state: BlockStateId,
    valid: &mut Option<bool>,
) {
    super::tconstruct_slime_grass_meal::valid(level, pos, state, valid);
}
