use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
const ID: &str = "tconstruct:rugged";

fn supported(game: &super::Game<'_>) -> bool {
    game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw|raw.as_ref().ok())
        .and_then(|raw|raw.0.get("tconstruct:tinkering/modifiers/rugged"))
        .and_then(|layers|layers.last())
        .and_then(|bytes|serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .is_some_and(|value|super::tconstruct_source_rugged::supported_value(&value))
}

fn level(game: &super::Game<'_>, stack: &ItemStack) -> Option<(i32,bool)> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let maximum = match custom.get("tic_max_damage") { Some(Tag::Int(n)) if *n > 0 => *n as u32, _ => return None };
    let broken = match custom.get("tic_broken") { None | Some(Tag::Byte(0)) => false, Some(Tag::Byte(1)) => true, _ => return None };
    let traits = super::tconstruct_source_material_traits::levels(&game.world,stack)?;
    let level = super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,ID)?;
    Some((level,broken || stack.damage >= maximum))
}

fn wanted(game: &super::Game<'_>) -> Option<Vec<String>> {
    if !supported(game) { return None; }
    let damage_tags = game.world.chunks.__horizon_mixins_tconstruct_protection_damage_types().as_ref()?.as_ref().ok()?;
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let belongs = |stack:&ItemStack, tag:&str|tags.get(tag).is_some_and(|types|types.contains(stack.item.as_ref()));
    let mut levels = Vec::new();
    for stack in [game.inventory.get(game.selected).and_then(Option::as_ref),game.offhand.as_ref()].into_iter().flatten() {
        if matches!(stack.item.as_ref(),"tconstruct:travelers_shield" | "tconstruct:plate_shield" | "tconstruct:battlesign")
            && belongs(stack,"tconstruct:modifiable/held") { levels.push(level(game,stack)?); }
    }
    for (slot,stack) in game.armor.iter().enumerate().filter_map(|(slot,stack)|stack.as_ref().map(|stack|(slot,stack))) {
        if super::tconstruct_fire_duration::armor_slot(&stack.item) == Some(slot)
            && belongs(stack,"tconstruct:modifiable/armor/worn") { levels.push(level(game,stack)?); }
    }
    super::tconstruct_source_rugged::blocked_types(&levels,damage_tags)
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    let blocked = if game.remote.is_none() { wanted(game).unwrap_or_default() } else { Vec::new() };
    if let Some(player) = game.player.as_mut() {
        *player.status.__horizon_mixins_tconstruct_rugged_blocked_types_mut() = blocked;
    }
}
