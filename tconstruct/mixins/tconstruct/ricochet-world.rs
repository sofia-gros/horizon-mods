use super::{Game, tconstruct_ricochet as rules};

pub(super) fn multiplier(game: &Game<'_>) -> Option<f64> {
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !rules::known(super::tconstruct_modifier_snapshot::highest_definition(
        raw,
        "tconstruct:tinkering/modifiers/ricochet",
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
    let mut levels = [0; 6];
    let hands = [
        game.inventory.get(game.selected).and_then(Option::as_ref),
        game.offhand.as_ref(),
    ];
    for (index, stack) in game
        .armor
        .iter()
        .map(Option::as_ref)
        .chain(hands)
        .enumerate()
    {
        let Some(stack) = stack else {
            continue;
        };
        let eligible = if index < 4 {
            rules::armor_slot(&stack.item) == Some(index) && worn.contains(stack.item.as_ref())
        } else {
            // The source attribute lists both hands with no held-tag condition.
            rules::held_shield(&stack.item)
        };
        if !eligible {
            continue;
        }
        if stack.count != 1 || stack.wire_patch().is_some() {
            return None;
        }
        // Default tooltip_style=attribute uses item attribute hooks, which
        // provide no modifiers from broken armor or held modifiable tools.
        if super::tconstruct_source_melee_shape::intact(stack).is_none() {
            continue;
        }
        let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
        levels[index] = super::tconstruct_source_material_traits::level_with_upgrade(
            stack,
            &traits,
            rules::ID,
        )?;
    }
    rules::multiplier(&levels)
}

pub(super) fn knockback(game: &Game<'_>, power: &mut f64) {
    if game.remote.is_some() {
        return;
    }
    if let Some(factor) = multiplier(game) {
        *power = rules::strength(*power, factor);
    }
}
