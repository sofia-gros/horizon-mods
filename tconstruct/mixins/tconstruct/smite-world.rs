use horizon_world::nbt::Tag;
use horizon_items::items::ItemStack;
use super::tconstruct_source_material_traits::Traits;

pub(super) fn context<'a>(game: &'a super::Game<'_>) -> Option<(&'a ItemStack, Traits)> {
    if game.remote.is_some() || game.player.as_ref()?.auto_spin_attack_dmg > 0.0 { return None; }
    let stack = game.inventory.get(game.selected)?.as_ref()?;
    if game.held() != Some(stack) || stack.count != 1 || stack.wire_patch().is_some()
        || !matches!(stack.item.as_ref(), "tconstruct:pickaxe" | "tconstruct:sledge_hammer" | "tconstruct:vein_hammer" | "tconstruct:hand_axe" | "tconstruct:broad_axe" | "tconstruct:mattock" | "tconstruct:pickadze" | "tconstruct:excavator" | "tconstruct:kama" | "tconstruct:scythe" | "tconstruct:sword" | "tconstruct:cleaver" | "tconstruct:dagger" | "tconstruct:battlesign" | "tconstruct:war_pick") { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let max = match custom.get("tic_max_damage") { Some(Tag::Int(max)) if *max > 0 => *max as u32, _ => return None };
    if stack.damage >= max || !matches!(custom.get("tic_broken"), None | Some(Tag::Byte(0))) { return None; }
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:modifiable/melee")?.contains(stack.item.as_ref()) { return None; }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    if super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, "tconstruct:no_melee")? != 0 { return None; }
    Some((stack, traits))
}

fn modifier_level(stack: &ItemStack, traits: &Traits, id: &str) -> Option<f32> {
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let amount = match custom.get("tic_upgrades") {
        None => 0,
        Some(upgrades @ Tag::Compound(_)) => match upgrades.get(id) {
            None => 0, Some(Tag::Int(amount)) => *amount, _ => return None,
        },
        _ => return None,
    };
    super::tconstruct_source_smite::effective_level_with_needed(traits.levels.get(id).copied().unwrap_or(0), amount,
        if id == "tconstruct:bane_of_sssss" { 15 } else { 5 }).filter(|level| *level > 0.0)
}

fn bane_target(game: &super::Game<'_>, index: usize) -> bool {
    let Some(mob) = game.mobs.get(index) else { return false; };
    let kind = if mob.kind.id.contains(':') { mob.kind.id.clone() } else { format!("minecraft:{}", mob.kind.id) };
    super::tconstruct_source_smite::arthropod(&kind) || game.stations().enchantments.entity_tags
        .get("forge:creepers").is_some_and(|ids| ids.contains(&kind))
}

pub(super) fn bonus(game: &super::Game<'_>, index: usize) -> Option<f32> {
    let mob = game.mobs.get(index)?;
    if !mob.alive() { return None; }
    let (stack, traits) = context(game)?;
    let mut level = 0.0;
    for (id, target, supported) in [
        ("tconstruct:smite", mob.kind.flags.undead, super::tconstruct_source_smite::supported_definition(&game.world)),
        ("tconstruct:antiaquatic", super::tconstruct_source_smite::water_mob(&mob.kind.id), super::tconstruct_source_smite::supported_antiaquatic_definition(&game.world)),
        ("tconstruct:bane_of_sssss", bane_target(game,index), super::tconstruct_source_smite::supported_bane_definition(&game.world)),
    ] {
        if target && supported { level += modifier_level(stack, &traits, id).unwrap_or(0.0); }
    }
    if !level.is_finite() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let multiplier = match custom.get("tic_smite_melee_multiplier") { Some(Tag::Float(bits)) => f32::from_bits(*bits), _ => return None };
    if !multiplier.is_finite() || multiplier < 0.0 { return None; }
    let flat = 2.0 * level * multiplier
        + super::tconstruct_antitoxin_world::melee_bonus(game, stack, &traits, multiplier).unwrap_or(0.0)
        + super::tconstruct_flamestance_world::melee_bonus(game, stack, &traits, multiplier).unwrap_or(0.0);
    if flat == 0.0 || !flat.is_finite() { return None; }
    let raw = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref().and_then(|raw| raw.as_ref().ok());
    let wear_level = |id| {
        if super::tconstruct_modifier_snapshot::known_melee_wear(raw, id) { traits.levels.get(id).copied().unwrap_or(0) } else { 0 }
    };
    let jagged = wear_level("tconstruct:jagged");
    let stonebound = wear_level("tconstruct:stonebound");
    let adjusted = if jagged == 0 && stonebound == 0 { flat } else {
        flat + super::tconstruct_source_material_effects::wear_damage_bonus(flat, stack.damage,
            traits.durability_multiplier, jagged, stonebound)
    };
    adjusted.is_finite().then_some(adjusted)
}


pub(super) fn bane_hit(game: &mut super::Game<'_>, index: usize) {
    use horizon_core::random::RandomSource;
    let candidate = (|| {
        let mob = game.mobs.get(index)?;
        if !mob.alive() || !bane_target(game,index) || !super::tconstruct_source_smite::supported_bane_definition(&game.world) { return None; }
        let record = game.player.as_ref()?.attack_log.last()?;
        if record.id != mob.id || !record.landed || !record.scale.is_finite() || record.scale <= 0.9 { return None; }
        let (stack, traits) = context(game)?;
        modifier_level(stack, &traits, "tconstruct:bane_of_sssss")
    })();
    let Some(level) = candidate else { return; };
    let Some(ticks) = super::tconstruct_source_smite::bane_duration(level, game.world.random.next_float()) else { return; };
    let Some(mob) = game.mobs.get_mut(index) else { return; };
    let (new, entity) = (!mob.has_effect("minecraft:slowness"), mob.id as i32);
    mob.add_effect("minecraft:slowness", ticks, 3);
    game.mod_effect_added("minecraft:slowness", Some(entity), 3, new);
}
