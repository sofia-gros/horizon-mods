fn tconstruct_geode_random_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    use horizon_core::random::RandomSource;
    use horizon_world::fluid::FluidState;
    let Some(source) = level.registry.state(state).map(|s| s.name.clone()) else { return; };
    if !matches!(source.as_str(), "tconstruct:budding_sky_slime_crystal" | "tconstruct:budding_earth_slime_crystal" | "tconstruct:budding_ichor_slime_crystal" | "tconstruct:budding_ender_slime_crystal") { return; }
    *handled = true;
    if !matches!(level.world.random.next_int_bound(5), Ok(0)) { return; }
    let Ok(index) = level.world.random.next_int_bound(6) else { return; };
    let (offset, facing) = super::tconstruct_source_geode_growth::DIRECTIONS[index as usize];
    let (Some(x), Some(y), Some(z)) = (pos.x.checked_add(offset[0]), pos.y.checked_add(offset[1]), pos.z.checked_add(offset[2])) else { return; };
    if !level.world.loaded(x, z) { return; }
    let target = BlockPos::new(x, y, z);
    let old = level.block_state(target);
    let Some(old_name) = level.registry.state(old).map(|s| s.name.as_str()) else { return; };
    let fluid = level.behaviours.fluid(old);
    let source_water = old_name == "minecraft:water" && fluid == FluidState::WATER;
    let Some(next) = super::tconstruct_source_geode_growth::next(&source, old_name, level.behaviours.is_air(old), source_water) else { return; };
    let Some(default) = level.registry.default_state(&next) else { return; };
    let Some(mut next_state) = level.registry.state(default).cloned() else { return; };
    next_state.properties.insert("facing".into(), facing.into());
    next_state.properties.insert("waterlogged".into(), (fluid == FluidState::WATER).to_string().into());
    let Ok(next_state) = level.registry.resolve(&next_state) else { return; };
    let behaviours = level.behaviours;
    behaviours.set_block(level, ticks, target, next_state, 3);
}
