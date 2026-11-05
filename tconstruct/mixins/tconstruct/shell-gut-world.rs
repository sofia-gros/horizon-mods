use super::{tconstruct_shell_gut as rule, Game};

pub(super) fn immune(game: &Game<'_>, effect: &str, amplifier: i32) -> bool {
    if game.remote.is_some() || rule::threshold(effect, &[]).is_none() {
        return false;
    }
    levels(game).is_some_and(|levels| rule::immune(effect, amplifier, &levels))
}

/// Resolve immunity at application time. Equipment changes, wear and reload
/// take effect without saved counters or retroactively curing active effects.
fn levels(game: &Game<'_>) -> Option<[i32; 4]> {
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !rule::known(super::tconstruct_modifier_snapshot::highest_definition(
        raw,
        "tconstruct:tinkering/modifiers/shell_gut",
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
        // Unlike percent attributes, EffectImmunityModule excludes broken gear.
        // Unknown/missing durability is conservatively ineligible as well.
        if super::tconstruct_source_melee_shape::intact(stack).is_none() {
            continue;
        }
        let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
        levels[index] =
            super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, rule::ID)?;
    }
    Some(levels)
}
