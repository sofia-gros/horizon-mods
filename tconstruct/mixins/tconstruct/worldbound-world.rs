use super::{Game, tconstruct_worldbound as rules};
use horizon_items::items::ItemStack;

pub(super) fn protects(game: &Game<'_>, stack: &ItemStack) -> Option<bool> {
    if game.remote.is_some()
        || !stack.item.starts_with("tconstruct:")
        || stack.count != 1
        || stack.wire_patch().is_some()
    {
        return None;
    }
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !tags
        .get("tconstruct:modifiable")?
        .contains(stack.item.as_ref())
    {
        return None;
    }
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !rules::known(super::tconstruct_modifier_snapshot::highest_definition(
        raw,
        "tconstruct:tinkering/modifiers/worldbound",
    )) {
        return None;
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let level =
        super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, rules::ID)?;
    // Volatile flags remain active on broken tools and armor.
    Some(level > 0)
}

pub(super) fn prepare(game: &mut Game<'_>) {
    // Tick-scoped entity IDs, not markers written into the dropped stack.
    // Rebuild before the native tick takes the item vector out of Game.
    let protected = game
        .dropped
        .iter()
        .filter(|item| protects(game, &item.stack) == Some(true))
        .map(|item| item.id)
        .collect();
    *game
        .world
        .chunks
        .__horizon_mixins_tconstruct_worldbound_environment_items_mut() = protected;
}

pub(super) fn clear(game: &mut Game<'_>) {
    game.world
        .chunks
        .__horizon_mixins_tconstruct_worldbound_environment_items_mut()
        .clear();
}

pub(super) fn expire(game: &Game<'_>, stack: &ItemStack, extra_life: &mut i32) {
    if protects(game, stack) == Some(true) {
        *extra_life = rules::extend_life(*extra_life);
    }
}
