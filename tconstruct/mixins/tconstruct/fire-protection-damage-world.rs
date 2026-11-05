use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
const ID: &str = "tconstruct:fire_protection";

fn piece(game: &super::Game<'_>, stack: &ItemStack, worn: bool) -> Option<(f32,bool,bool)> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let maximum = match custom.get("tic_max_damage") { Some(Tag::Int(n)) if *n > 0 => *n as u32, _ => return None };
    let flag = match custom.get("tic_broken") { None | Some(Tag::Byte(0)) => false, Some(Tag::Byte(1)) => true, _ => return None };
    let raw = match custom.get("tic_upgrades") {
        None => 0,
        Some(upgrades @ Tag::Compound(_)) => match upgrades.get(ID) { None => 0, Some(Tag::Int(raw)) => *raw, _ => return None },
        _ => return None,
    };
    let traits = super::tconstruct_source_material_traits::levels(&game.world,stack)?;
    let level = super::tconstruct_source_protection::effective_level(traits.levels.get(ID).copied().unwrap_or(0),raw)?;
    Some((level, flag || stack.damage >= maximum, worn))
}

fn wanted(game: &super::Game<'_>) -> Option<Vec<(String,f32)>> {
    if !super::tconstruct_source_protection::supported_definition(&game.world, ID) { return None; }
    let (source_types, vanilla_fire) = game.world.chunks.__horizon_mixins_tconstruct_fire_protection_damage_types().as_ref()?.as_ref().ok()?;
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let belongs = |stack:&ItemStack, tag:&str| tags.get(tag).is_some_and(|set|set.contains(stack.item.as_ref()));
    let mut pieces = Vec::new();
    // Source EquipmentSlot order: hands first, then feet through head.
    for stack in [game.inventory.get(game.selected).and_then(Option::as_ref), game.offhand.as_ref()].into_iter().flatten() {
        if matches!(stack.item.as_ref(), "tconstruct:travelers_shield" | "tconstruct:plate_shield" | "tconstruct:battlesign")
            && belongs(stack,"tconstruct:modifiable/held") { pieces.push(piece(game,stack,false)?); }
    }
    for (slot, stack) in game.armor.iter().enumerate().rev().filter_map(|(slot,s)|s.as_ref().map(|s|(slot,s))) {
        if super::tconstruct_fire_duration::armor_slot(&stack.item) == Some(slot)
            && belongs(stack,"tconstruct:modifiable/armor/worn") { pieces.push(piece(game,stack,true)?); }
    }
    let (source, broken_vanilla) = super::tconstruct_fire_protection_damage::points(&pieces)?;
    let mut table = Vec::new();
    for (_, definition) in game.motion.health.damage.entries() {
        let mut points = 0.0;
        if source_types.contains(&definition.name) { points += source; }
        if vanilla_fire.contains(&definition.name) { points += broken_vanilla; }
        if points > 0.0 { table.push((definition.name.clone(),points)); }
    }
    Some(table)
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    let table = if game.remote.is_none() { wanted(game).unwrap_or_default() } else { Vec::new() };
    if let Some(player) = game.player.as_mut() {
        *player.status.__horizon_mixins_tconstruct_fire_protection_points_mut() = table;
    }
}
