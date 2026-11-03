fn tconstruct_slime_plant_survive(
    level: &GameLevel<'_>,
    pos: BlockPos,
    state: BlockStateId,
    survives: &mut Option<bool>,
) {
    let Some(name) = level.registry.state(state).map(|s| s.name.as_str()) else {
        return;
    };
    if !super::tconstruct_slime_plant_support::is_plant(name) {
        return;
    }
    let Some(y) = pos.y.checked_sub(1) else {
        *survives = Some(false);
        return;
    };
    let ground = level.block_state(BlockPos::new(pos.x, y, pos.z));
    *survives = Some(
        level
            .registry
            .state(ground)
            .is_some_and(|s| super::tconstruct_slime_plant_support::supports(&s.name)),
    );
}
