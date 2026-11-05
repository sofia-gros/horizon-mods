use super::{Game, tconstruct_balm as balm, tconstruct_crystalstrike as rule};

/// Four worn slots plus selected mainhand/offhand Crystalstrike shields.
/// Balm remains worn-only; its two held entries stay zero.
fn levels(game: &Game<'_>, knockback: bool, id: &str) -> Option<[i32; 6]> {
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    let (path, known): (&str, fn(Option<&[u8]>) -> bool) = match id {
        rule::ID => ("tconstruct:tinkering/modifiers/crystalstrike", rule::known),
        balm::ID => ("tconstruct:tinkering/modifiers/balm_of_sssss", balm::known),
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
    let mut levels = [0; 6];
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
        // AttributeModule's EquipmentChange hooks have no brokenness check.
        // ArmorLevelModule expressly excludes broken pieces for knockback.
        if knockback && super::tconstruct_source_ranged_wear::is_broken(stack) {
            continue;
        }
        let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
        levels[index] =
            super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, id)?;
    }
    if id == rule::ID {
        let held_tag = tags.get("tconstruct:modifiable/armor/held");
        let hands = [
            game.inventory.get(game.selected).and_then(Option::as_ref),
            game.offhand.as_ref(),
        ];
        for (hand, stack) in hands
            .into_iter()
            .enumerate()
            .filter_map(|(hand, stack)| stack.map(|stack| (hand, stack)))
        {
            if !rule::held_shield(&stack.item) {
                continue;
            }
            if stack.count != 1 || stack.wire_patch().is_some() {
                return None;
            }
            // Percent AttributeModule accepts both hands, even while broken.
            // Only ArmorLevelModule needs the held tag and an intact tool.
            if knockback
                && (!held_tag.is_some_and(|tag| tag.contains(stack.item.as_ref()))
                    || super::tconstruct_source_melee_shape::intact(stack).is_none())
            {
                continue;
            }
            let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
            levels[4 + hand] =
                super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, id)?;
        }
    }
    Some(levels)
}

pub(super) fn attack_speed(game: &Game<'_>, value: f64) -> f64 {
    let Some(levels) = levels(game, false, rule::ID) else {
        return value;
    };
    levels
        .into_iter()
        .fold(value, |value, level| {
            value * (1.0 + rule::amount(level).unwrap_or(0.0))
        })
        .clamp(0.0, 1024.0)
}

/// Attribute operation ordering: Balm and maximum Magic Protection base
/// additions, then every Crystalstrike total, then one clamp and duration cast.
pub(super) fn multiplier(game: &Game<'_>) -> f64 {
    let balm = levels(game, false, balm::ID).unwrap_or([0; 6]);
    let totals = levels(game, false, rule::ID)
        .unwrap_or([0; 6])
        .map(|level| rule::amount(level).unwrap_or(0.0));
    let magic = super::tconstruct_magic_protection_world::amount(game).unwrap_or(0.0);
    balm::multiplier_with_base(&balm, &[magic], &totals).unwrap_or(1.0)
}

/// Keep /attribute consistent with the combat consumer's live calculation.
/// Own transient IDs, so removal and definition reload do not leave a bonus.
pub(super) fn refresh(game: &mut Game<'_>) {
    let levels = levels(game, false, rule::ID).unwrap_or([0; 6]);
    let Some(player) = game.player.as_mut() else {
        return;
    };
    for (index, level) in levels.into_iter().enumerate() {
        let id = if index < 4 {
            format!("tconstruct:crystalstrike_worn/{index}")
        } else {
            format!("tconstruct:crystalstrike_held/{}", index - 4)
        };
        let amount = rule::amount(level).unwrap_or(0.0);
        let _ = player.set_modifier(
            "minecraft:attack_speed",
            &id,
            amount,
            "add_multiplied_total",
            level > 0,
        );
    }
}

pub(super) fn effect_duration(game: &Game<'_>, effect: &str, ticks: i32) -> i32 {
    if game.remote.is_some() || !rule::curable_bad_effect(effect) {
        return ticks;
    }
    rule::duration(ticks, multiplier(game)).unwrap_or(ticks)
}

pub(super) fn knockback(game: &Game<'_>, x: &mut f64, z: &mut f64) {
    if game.remote.is_some() || !x.is_finite() || !z.is_finite() {
        return;
    }
    let Some(level) = levels(game, true, rule::ID)
        .and_then(|levels| levels.into_iter().try_fold(0_i32, i32::checked_add))
    else {
        return;
    };
    let squared = *x * *x + *z * *z;
    // Leave the host's random-direction fallback and invalid vectors alone.
    if squared <= 0.0 || !squared.is_finite() {
        return;
    }
    use horizon_core::geometry::trig;
    let angle = trig::atan2(*x, *z);
    let Some(delta) = rule::angle_delta(angle, level) else {
        return;
    };
    // Vec3.yRot takes a float angle. Source 1.20.1 sine/cosine index arithmetic
    // is float, unlike the current host's double-index wrapper.
    let table = trig::sine_table();
    let sin = f64::from(table[((delta * 10430.378_f32) as i32 & 65535) as usize]);
    let cos = f64::from(table[((delta * 10430.378_f32 + 16384.0_f32) as i32 & 65535) as usize]);
    let (old_x, old_z) = (*x, *z);
    *x = old_x * cos + old_z * sin;
    *z = old_z * cos - old_x * sin;
}
