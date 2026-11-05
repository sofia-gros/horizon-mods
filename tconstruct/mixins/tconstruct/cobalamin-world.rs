use super::{tconstruct_cobalamin as rule, Game};

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
        "tconstruct:tinkering/modifiers/cobalamin",
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

pub(super) fn attack_speed(game: &Game<'_>, speed: f64) -> f64 {
    levels(game)
        .and_then(|levels| rule::scale(speed, &levels, 0.05, 1024.0))
        .unwrap_or(speed)
}

/// Keep the attribute as a double until ToolEvents' final float cast. The
/// existing Haste bridge's f32 baseline is retained for compatibility.
pub(super) fn mining_speed(game: &Game<'_>, speed: f32, existing: f32) -> f32 {
    levels(game)
        .filter(|levels| levels.iter().any(|level| *level > 0))
        .and_then(|levels| rule::scale(f64::from(existing), &levels, 0.15, 100.0))
        .map(|multiplier| (f64::from(speed) * multiplier) as f32)
        .unwrap_or_else(|| {
            super::tconstruct_source_mining_rules::finish_mining_speed(speed, existing)
        })
}

/// Keep the visible attack-speed attribute in step with the combat consumer.
/// The native consumer independently evaluates mainhand and potion modifiers.
pub(super) fn refresh(game: &mut Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let levels = levels(game).unwrap_or([0; 4]);
    let Some(player) = game.player.as_mut() else {
        return;
    };
    for (index, level) in levels.into_iter().enumerate() {
        let id = format!("tconstruct:cobalamin_worn/{index}");
        let amount = rule::amount(level, 0.05).unwrap_or(0.0);
        let _ = player.set_modifier(
            "minecraft:attack_speed",
            &id,
            amount,
            "add_multiplied_total",
            level > 0,
        );
    }
}
