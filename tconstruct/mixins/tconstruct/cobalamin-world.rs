use super::{tconstruct_cobalamin as rule, tconstruct_godspeed as godspeed, Game};

/// Shared Cobalamin/Godspeed worn-attribute bridge. No saved derived bonus.
fn levels(game: &Game<'_>, id: &str) -> Option<[i32; 4]> {
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
    let (path, known): (&str, fn(Option<&[u8]>) -> bool) = match id {
        rule::ID => ("tconstruct:tinkering/modifiers/cobalamin", rule::known),
        godspeed::ID => ("tconstruct:tinkering/modifiers/godspeed", godspeed::known),
        _ => return None,
    };
    if !known(super::tconstruct_modifier_snapshot::highest_definition(
        raw, path,
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
            super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, id)?;
    }
    Some(levels)
}

pub(super) fn attack_speed(game: &Game<'_>, speed: f64) -> f64 {
    let speed = levels(game, rule::ID)
        .and_then(|levels| rule::scale(speed, &levels, 0.05, 1024.0))
        .unwrap_or(speed);
    levels(game, godspeed::ID)
        .and_then(|levels| godspeed::scale(speed, &levels, godspeed::Attribute::Attack, 1024.0))
        .unwrap_or(speed)
}

/// Both source attributes remain doubles until ToolEvents' final float cast.
/// Retain the existing Haste bridge's float baseline for compatibility.
pub(super) fn mining_speed(game: &Game<'_>, speed: f32, existing: f32) -> f32 {
    let cobalamin = levels(game, rule::ID).filter(|levels| levels.iter().any(|level| *level > 0));
    let godspeed =
        levels(game, godspeed::ID).filter(|levels| levels.iter().any(|level| *level > 0));
    if cobalamin.is_none() && godspeed.is_none() {
        return super::tconstruct_source_mining_rules::finish_mining_speed(speed, existing);
    }
    let multiplier = f64::from(existing);
    let multiplier = cobalamin
        .and_then(|levels| rule::scale(multiplier, &levels, 0.15, 100.0))
        .unwrap_or(multiplier);
    let multiplier = godspeed
        .and_then(|levels| godspeed::scale(multiplier, &levels, godspeed::Attribute::Mining, 100.0))
        .unwrap_or(multiplier);
    (f64::from(speed) * multiplier) as f32
}

/// Refresh attack-speed display and the actual native movement attribute.
/// The native consumer independently evaluates mainhand and potion modifiers.
pub(super) fn refresh(game: &mut Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let cobalamin = levels(game, rule::ID).unwrap_or([0; 4]);
    let godspeed = levels(game, godspeed::ID).unwrap_or([0; 4]);
    let Some(player) = game.player.as_mut() else {
        return;
    };
    for (index, level) in cobalamin.into_iter().enumerate() {
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
    for (index, level) in godspeed.into_iter().enumerate() {
        for (attribute, field) in [
            ("minecraft:attack_speed", godspeed::Attribute::Attack),
            ("minecraft:movement_speed", godspeed::Attribute::Movement),
        ] {
            let id = format!("tconstruct:godspeed_worn/{index}");
            let amount = godspeed::amount(level, field).unwrap_or(0.0);
            let _ = player.set_modifier(attribute, &id, amount, "add_multiplied_total", level > 0);
        }
    }
}
