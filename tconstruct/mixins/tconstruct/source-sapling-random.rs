fn tconstruct_sapling_random(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    use horizon_core::random::RandomSource;
    if !super::tconstruct_sapling_tree::sapling(level,state) {return;}
    *handled=true;
    let Some(y)=pos.y.checked_add(1) else {return;};
    if horizon_world::level::raw_brightness(level, BlockPos::new(pos.x,y,pos.z), level.sky_darken()) >= 9
        && matches!(level.world.random.next_int_bound(7),Ok(0)) {
        super::tconstruct_sapling_tree::advance(level,ticks,pos,state);
    }
}
