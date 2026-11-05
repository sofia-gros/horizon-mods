use super::{tconstruct_dreambound as rule, Game};

/// Worn stock armor with live source traits. No saved derived bonus.
fn levels(game: &Game<'_>) -> Option<[i32; 4]> {
    // A remote client needs an actor-specific authoritative frame, not a local
    // resource snapshot borrowed from another actor or dimension.
    if game.remote.is_some() {
        return None;
    }
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !rule::known(super::tconstruct_modifier_snapshot::highest_definition(
        raw,
        "tconstruct:tinkering/modifiers/dreambound",
    )) {
        return None;
    }
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let worn = tags.get("tconstruct:modifiable/armor/worn")?;
    let mut levels = [0; 4];
    for (index, stack) in game
        .armor
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.as_ref().map(|s| (i, s)))
    {
        if rule::armor_slot(&stack.item) != Some(index) || !worn.contains(stack.item.as_ref()) {
            continue;
        }
        if stack.count != 1 || stack.wire_patch().is_some() {
            return None;
        }
        // PERCENT AttributeModule uses EquipmentChange hooks, not the held
        // attribute hook. Its onEquip/onUnequip have no brokenness filter.
        let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
        levels[index] =
            super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, rule::ID)?;
    }
    Some(levels)
}

/// One native MULTIPLY_TOTAL attribute per source equipment slot. Keep the
/// IDs outside the host's reset list and explicitly remove absent bonuses.
pub(super) fn refresh(game: &mut Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let levels = levels(game).unwrap_or([0; 4]);
    let Some(player) = game.player.as_mut() else {
        return;
    };
    for (index, level) in levels.into_iter().enumerate() {
        let id = format!("tconstruct:dreambound_gravity/{index}");
        let amount = rule::gravity(level).unwrap_or(0.0);
        let _ = player.set_modifier(
            "minecraft:gravity",
            &id,
            amount,
            "add_multiplied_total",
            amount != 0.0,
        );
    }
}
