// Bodies of native-hook adapters live outside the host hook-point file.
use super::{GameLevel, Ticks, BlockPos, BlockStateId, LevelAccess};

pub(super) fn tconstruct_casting_comparator_present(level: &crate::game::GameLevel<'_>, state: horizon_world::registry::BlockStateId, output: &mut bool) {
    if matches!(&*level.facts.get(state).name, "tconstruct:seared_casting_tank" | "tconstruct:seared_fluid_cannon") { *output = true; }
}

pub(super) fn tconstruct_geode_random_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    use horizon_core::random::RandomSource;
    use horizon_world::fluid::FluidKind;
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
    let air = level.behaviours.is_air(old);
    let full_water = !air && super::tconstruct_source_geode_growth::can_start(false, old_name == "minecraft:water", fluid.is_full());
    let Some(next) = super::tconstruct_source_geode_growth::next(&source, old_name, air, full_water) else { return; };
    let Some(default) = level.registry.default_state(&next) else { return; };
    let Some(mut next_state) = level.registry.state(default).cloned() else { return; };
    next_state.properties.insert("facing".into(), facing.into());
    next_state.properties.insert("waterlogged".into(), (fluid.kind() == FluidKind::Water).to_string().into());
    let Ok(next_state) = level.registry.resolve(&next_state) else { return; };
    let behaviours = level.behaviours;
    behaviours.set_block(level, ticks, target, next_state, 3);
}

pub(super) fn tconstruct_sapling_random(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    use horizon_core::random::RandomSource;
    if !super::tconstruct_sapling_tree::sapling(level,state) {return;}
    *handled=true;
    let Some(y)=pos.y.checked_add(1) else {return;};
    if horizon_world::level::raw_brightness(level, BlockPos::new(pos.x,y,pos.z), level.sky_darken()) >= 9
        && matches!(level.world.random.next_int_bound(7),Ok(0)) {
        super::tconstruct_sapling_tree::advance(level,ticks,pos,state);
    }
}

pub(super) fn tconstruct_slime_leaves_random(level: &mut GameLevel<'_>, state: BlockStateId, handled: &mut bool) {
    let Some(block) = level.registry.state(state) else { return; };
    if matches!(block.name.as_str(), "tconstruct:earth_slime_leaves" | "tconstruct:sky_slime_leaves" | "tconstruct:ender_slime_leaves")
        && level.behaviours.tag("tconstruct:slimy_logs").is_none() {
        *handled = true;
    }
}

// Source SlimeGrassBlock and SlimeNyliumBlock random ticks.
pub(super) fn tconstruct_slimy_grass_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
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
        if !valid(level, default, at, false) { continue; }
        // The live source tag snapshot is world-scoped. If it is unavailable
        // or unresolved, do not spread speculatively.
        let tags = level.world.chunks.__horizon_mixins_tconstruct_resolved_fluid_tags()
            .as_ref().and_then(|result| result.as_ref().ok());
        let id = behaviors.fluid(top).kind().name();
        let is_water = super::tconstruct_source_tags::resolved_member(tags, "minecraft:water", id)
            .or_else(|| {
                let raw = level.world.chunks.__horizon_mixins_tconstruct_fluid_tag_snapshot().as_ref()?;
                let known = horizon_world::fluid::FluidKind::ALL.into_iter()
                    .map(|kind| kind.name().to_owned()).collect();
                super::tconstruct_source_tags::local_tag_member(raw, &known, "minecraft:water", id)
            });
        let Some(is_water) = is_water else { continue; };
        if is_water { continue; }
        let snowy = level.registry.state(top).is_some_and(|s| s.name == "minecraft:snow");
        let Some(mut next) = level.registry.state(default).cloned() else { continue; };
        let Some(property) = next.properties.get_mut("snowy") else { continue; };
        *property = snowy.to_string();
        let Ok(next) = level.registry.resolve(&next) else { continue; };
        behaviors.set_block(level, ticks, at, next, 3);
    }
}

pub(super) fn tconstruct_sapling_survive(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, survives: &mut Option<bool>) {
    if !super::tconstruct_sapling_tree::sapling(level,state) {return;}
    let Some(y)=pos.y.checked_sub(1) else {*survives=Some(false);return;};
    let ground=level.block_state(BlockPos::new(pos.x,y,pos.z));
    let Some(name)=level.registry.state(ground).map(|s|s.name.as_str()) else {*survives=Some(false);return;};
    let dirt=matches!(name,"tconstruct:earth_slime_dirt"|"tconstruct:sky_slime_dirt"|"tconstruct:ichor_slime_dirt"|"tconstruct:ender_slime_dirt");
    let grass=name.strip_prefix("tconstruct:").and_then(|v|v.strip_suffix("_slime_grass")).and_then(|v|v.split_once('_')).is_some_and(|(foliage,soil)| ["earth","sky","ichor","ender","blood"].contains(&foliage) && ["earth","sky","ichor","ender","vanilla"].contains(&soil));
    *survives=Some(dirt||grass);
}

// Run after native cancellation handlers and before actual block removal.
// Area recursion spends only on its first allowed call.
pub(super) fn tconstruct_overwield_harvest_start(game: &mut crate::game::Game<'_>, canceled: bool) {
    if canceled { return; }
    use horizon_world::nbt::Tag;
    let actor = game.lan_acting;
    let active = game.world.chunks.__horizon_mixins_tconstruct_area_active().contains(&actor);
    if active && game.world.chunks.__horizon_mixins_tconstruct_area_overwield_spent().contains(&actor) { return; }
    let Some(stack) = game.inventory[game.selected].as_ref() else { return; };
    if crate::game::tconstruct_source_ranged_wear::is_broken(stack) { return; }
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return; };
    if !matches!(custom.get("tic_max_damage"), Some(Tag::Int(max)) if *max > 0 && stack.damage < *max as u32) { return; }
    let Some(tags) = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|result| result.as_ref().ok()) else { return; };
    let Some(broad_tag) = tags.get("tconstruct:modifiable/broad") else { return; };
    let Some(harvest_tag) = tags.get("tconstruct:modifiable/harvest") else { return; };
    let broad = broad_tag.contains(stack.item.as_ref());
    let harvest = harvest_tag.contains(stack.item.as_ref());
    if !broad && !harvest { return; }
    let Some(Tag::List(traits)) = custom.get("tic_mining_traits") else { return; };
    let level = traits.iter().find_map(|entry| match (entry.get("id"), entry.get("level")) {
        (Some(Tag::String(id)), Some(Tag::Int(level))) if id.to_string_lossy() == "tconstruct:overwield" => Some((*level).max(0) as u32),
        _ => None,
    }).unwrap_or(0);
    if level == 0 { return; }
    if let Some(held) = game.inventory[game.selected].as_mut() {
        crate::game::tconstruct_source_overslime::spend(held, level.saturating_mul(if broad { 5 } else { 1 }));
    }
    if active { game.world.chunks.__horizon_mixins_tconstruct_area_overwield_spent_mut().insert(actor); }
}
