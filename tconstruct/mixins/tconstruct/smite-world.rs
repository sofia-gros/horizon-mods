use horizon_world::nbt::Tag;

pub(super) fn bonus(game: &super::Game<'_>, index: usize) -> Option<f32> {
    if game.remote.is_some() || game.player.as_ref()?.auto_spin_attack_dmg > 0.0 { return None; }
    let mob = game.mobs.get(index)?;
    if !mob.alive() || !mob.kind.flags.undead { return None; }
    let stack = game.inventory.get(game.selected)?.as_ref()?;
    if game.held() != Some(stack) || stack.count != 1 || stack.wire_patch().is_some()
        || !matches!(stack.item.as_ref(), "tconstruct:pickaxe" | "tconstruct:sledge_hammer" | "tconstruct:vein_hammer" | "tconstruct:hand_axe" | "tconstruct:broad_axe" | "tconstruct:mattock" | "tconstruct:pickadze" | "tconstruct:excavator" | "tconstruct:kama" | "tconstruct:scythe" | "tconstruct:sword" | "tconstruct:cleaver" | "tconstruct:dagger" | "tconstruct:battlesign" | "tconstruct:war_pick") { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let max = match custom.get("tic_max_damage") { Some(Tag::Int(max)) if *max > 0 => *max as u32, _ => return None };
    if stack.damage >= max || !matches!(custom.get("tic_broken"), None | Some(Tag::Byte(0))) { return None; }
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:modifiable/melee")?.contains(stack.item.as_ref())
        || !super::tconstruct_source_smite::supported_definition(&game.world) { return None; }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    if super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, "tconstruct:no_melee")? != 0 { return None; }
    let amount = match custom.get("tic_upgrades") {
        None => 0,
        Some(upgrades @ Tag::Compound(_)) => match upgrades.get("tconstruct:smite") {
            None => 0, Some(Tag::Int(amount)) => *amount, _ => return None,
        },
        _ => return None,
    };
    let level = super::tconstruct_source_smite::effective_level(traits.levels.get("tconstruct:smite").copied().unwrap_or(0), amount)?;
    if level <= 0.0 { return None; }
    let multiplier = match custom.get("tic_smite_melee_multiplier") { Some(Tag::Float(bits)) => f32::from_bits(*bits), _ => return None };
    if !multiplier.is_finite() || multiplier < 0.0 { return None; }
    let flat = 2.0 * level * multiplier;
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
