use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use super::{Game, tconstruct_crystalstrike as equipment, tconstruct_magic_protection as rule};

fn level(game: &Game<'_>, stack: &ItemStack) -> Option<f32> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let maximum = match custom.get("tic_max_damage") {
        Some(Tag::Int(n)) if *n > 0 => *n as u32, _ => return None,
    };
    if stack.damage >= maximum || !matches!(custom.get("tic_broken"), None | Some(Tag::Byte(0))) {
        return None;
    }
    let raw = match custom.get("tic_upgrades") {
        None => 0,
        Some(upgrades @ Tag::Compound(_)) => match upgrades.get(rule::ID) {
            None => 0, Some(Tag::Int(raw)) => *raw, _ => return None,
        },
        _ => return None,
    };
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    rule::effective_level(traits.levels.get(rule::ID).copied().unwrap_or(0), raw)
}

/// Resolve fresh for every incoming effect. No equipment cache or saved bonus.
pub(super) fn amount(game: &Game<'_>) -> Option<f64> {
    let snapshot = game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?.as_ref().ok()?;
    if !rule::known(super::tconstruct_modifier_snapshot::highest_definition(
        snapshot, "tconstruct:tinkering/modifiers/magic_protection")) { return None; }
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?.as_ref().ok()?;
    let belongs = |stack: &ItemStack, tag: &str| {
        tags.get(tag).is_some_and(|items| items.contains(stack.item.as_ref()))
    };
    let mut levels = Vec::with_capacity(6);
    for (slot, stack) in game.armor.iter().enumerate()
        .filter_map(|(slot, stack)| stack.as_ref().map(|stack| (slot, stack))) {
        if equipment::armor_slot(&stack.item) == Some(slot)
            && belongs(stack, "tconstruct:modifiable/armor/worn") {
            if let Some(level) = level(game, stack) { levels.push(level); }
        }
    }
    // Source held_tag is modifiable/held, not modifiable/armor/held. Support
    // stock shield classes in either hand without requiring active blocking.
    for stack in [game.inventory.get(game.selected).and_then(Option::as_ref), game.offhand.as_ref()]
        .into_iter().flatten() {
        if equipment::held_shield(&stack.item) && belongs(stack, "tconstruct:modifiable/held") {
            if let Some(level) = level(game, stack) { levels.push(level); }
        }
    }
    rule::amount(&levels)
}
