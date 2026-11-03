// Source SlimeGrassBlock and SlimeNyliumBlock random ticks.
fn tconstruct_slimy_grass_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    use horizon_core::{geometry::Direction, random::RandomSource};
    use horizon_world::level::data::state_flags::{CAN_OCCLUDE, SHAPE_OCCLUSION};
    let Some(name) = level.registry.state(state).map(|v| v.name.clone()) else { return; };
    let Some(prefix) = name.strip_prefix("tconstruct:").and_then(|s| s.strip_suffix("_slime_grass")) else { return; };
    let Some((foliage, soil)) = prefix.split_once('_') else { return; };
    if !["earth", "sky", "ichor", "ender", "blood"].contains(&foliage) || !["earth", "sky", "ichor", "ender", "vanilla"].contains(&soil) { return; }
    *handled = true;
    let nether = matches!(foliage, "ichor" | "blood");
    if !nether {
        // Source prevents loading a missing chunk anywhere within three blocks.
        for dx in [-3, 3] { for dz in [-3, 3] {
            let (Some(x), Some(z)) = (pos.x.checked_add(dx), pos.z.checked_add(dz)) else { return; };
            if !level.world.loaded(x, z) { return; }
        }}
    }
    let Some(y) = pos.y.checked_add(1) else { return; };
    let above = BlockPos::new(pos.x, y, pos.z);
    let behaviors = level.behaviours;
    let valid = |level: &GameLevel<'_>, state: BlockStateId, at: BlockPos, nylium: bool| -> bool {
        let Some(y) = at.y.checked_add(1) else { return false; };
        let above = level.block_state(BlockPos::new(at.x, y, at.z));
        let info = level.registry.state(above);
        if !nylium {
            if info.is_some_and(|v| v.name == "minecraft:snow" && v.properties.get("layers").is_some_and(|n| n == "1")) { return true; }
            if behaviors.fluid(above).is_full() { return false; }
        }
        let occludes = |s| behaviors.facts(s).flags & (CAN_OCCLUDE | SHAPE_OCCLUSION) == (CAN_OCCLUDE | SHAPE_OCCLUSION);
        let from_full = occludes(state) && behaviors.shape_info(state).full_faces & (1 << Direction::Up as u8) != 0;
        let to_full = occludes(above) && behaviors.shape_info(above).full_faces & (1 << Direction::Down as u8) != 0;
        let attenuation = if from_full || to_full { 16 } else { i32::from(behaviors.facts(above).dampening) };
        attenuation < 15
    };
    if !valid(level, state, pos, nether) {
        let dirt = if soil == "vanilla" { "minecraft:dirt".to_owned() } else { format!("tconstruct:{soil}_slime_dirt") };
        if let Some(next) = level.registry.default_state(&dirt) { behaviors.set_block(level, ticks, pos, next, 3); }
        return;
    }
    if nether || horizon_world::level::raw_brightness(level, above, level.sky_darken()) < 9 { return; }
    for _ in 0..4 {
        let Ok(dx) = level.world.random.next_int_bound(3) else { return; };
        let Ok(dy) = level.world.random.next_int_bound(5) else { return; };
        let Ok(dz) = level.world.random.next_int_bound(3) else { return; };
        let (Some(x), Some(y), Some(z)) = (pos.x.checked_add(dx-1), pos.y.checked_add(dy-3), pos.z.checked_add(dz-1)) else { continue; };
        let at = BlockPos::new(x,y,z);
        let Some(soil_name) = level.registry.state(level.block_state(at)).map(|v| v.name.as_str()) else { continue; };
        let kind = match soil_name { "minecraft:dirt" => "vanilla", "tconstruct:earth_slime_dirt" => "earth", "tconstruct:sky_slime_dirt" => "sky", "tconstruct:ichor_slime_dirt" => "ichor", "tconstruct:ender_slime_dirt" => "ender", _ => continue };
        let Some(default) = level.registry.default_state(&format!("tconstruct:{foliage}_{kind}_slime_grass")) else { continue; };
        let Some(y) = y.checked_add(1) else { continue; };
        let top = level.block_state(BlockPos::new(x,y,z));
        if !valid(level, default, at, false) || matches!(behaviors.fluid(top).kind(), horizon_world::fluid::FluidKind::Water | horizon_world::fluid::FluidKind::FlowingWater) { continue; }
        let snowy = level.registry.state(top).is_some_and(|s| s.name == "minecraft:snow");
        let Some(mut next) = level.registry.state(default).cloned() else { continue; };
        let Some(property) = next.properties.get_mut("snowy") else { continue; };
        *property = snowy.to_string();
        let Ok(next) = level.registry.resolve(&next) else { continue; };
        behaviors.set_block(level, ticks, at, next, 3);
    }
}
