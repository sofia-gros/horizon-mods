use super::{tconstruct_padded as rule, Game};
use horizon_client::mobs::{Damage, DamageKind, Who};

fn level(game: &Game<'_>) -> Option<i32> {
    if game.remote.is_some() {
        return None;
    }
    let stack = game.inventory.get(game.selected)?.as_ref()?;
    // A first complete primary-mob path, not generic modifier dispatch.
    if stack.item.as_ref() != "tconstruct:dagger"
        || stack.count != 1
        || stack.wire_patch().is_some()
    {
        return None;
    }
    if game.held() != Some(stack) {
        return None;
    }
    super::tconstruct_source_melee_shape::intact(stack)?;
    if game.player.as_ref()?.auto_spin_attack_dmg > 0.0 {
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
        .get("tconstruct:modifiable/melee")?
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
    if !rule::known(super::tconstruct_modifier_snapshot::highest_definition(
        raw,
        "tconstruct:tinkering/modifiers/padded",
    )) {
        return None;
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let level =
        super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, rule::ID)?;
    if super::tconstruct_source_material_traits::level_with_upgrade(
        stack,
        &traits,
        "tconstruct:no_melee",
    )? > 0
    {
        return None;
    }
    // Other source knockback modules need ordered dispatch, not this shortcut.
    let mut ids: std::collections::BTreeSet<String> = traits.levels.keys().cloned().collect();
    if let Some(upgrades) = stack
        .data()?
        .other("minecraft:custom_data")?
        .get("tic_upgrades")
    {
        let horizon_world::nbt::Tag::Compound(upgrades) = upgrades else {
            return None;
        };
        ids.extend(upgrades.keys().map(|id| id.to_string_lossy().to_string()));
    }
    for id in ids {
        if id == rule::ID
            || super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, &id)?
                == 0
        {
            continue;
        }
        let (namespace, name) = id.split_once(':')?;
        let bytes = super::tconstruct_modifier_snapshot::highest_definition(
            raw,
            &format!("{namespace}:tinkering/modifiers/{name}"),
        )?;
        let definition: serde_json::Value = serde_json::from_slice(bytes).ok()?;
        if definition
            .get("modules")
            .and_then(|v| v.as_array())
            .is_some_and(|modules| {
                modules
                    .iter()
                    .any(|m| m.get("type").and_then(|v| v.as_str()) == Some("tconstruct:knockback"))
            })
        {
            return None;
        }
    }
    (level > 0).then_some(level)
}

pub(super) fn begin(game: &mut Game<'_>, index: usize) {
    let actor = game.lan_acting;
    let candidate = (|| {
        let level = level(game)?;
        let mob = game.mobs.get(index)?;
        // The newer host's Sulfur Cube has a different launch implementation.
        if mob.kind.id.trim_start_matches("minecraft:") == "sulfur_cube" {
            return None;
        }
        if !mob.attrs.knockback_resistance.is_finite()
            || !(0.0..=1.0).contains(&mob.attrs.knockback_resistance)
        {
            return None;
        }
        Some((mob.id, level, None, None))
    })();
    // A sentinel masks an outer context during unsupported nested attacks.
    game.world
        .chunks
        .__horizon_mixins_tconstruct_padded_context_mut()
        .push((actor, candidate));
}

pub(super) fn extra(game: &mut Game<'_>, value: (f32, bool)) -> (f32, bool) {
    let actor = game.lan_acting;
    // Source does not consume vanilla Knockback enchantments. Leave that mixed
    // host case unchanged rather than claim ordered source hook compatibility.
    if value.0 != if value.1 { 0.5 } else { 0.0 } {
        return value;
    }
    let attack = game
        .player
        .as_mut()
        .and_then(|p| p.attribute("minecraft:attack_knockback").ok())
        .unwrap_or(0.0) as f32;
    let Some((active, Some((_, level, total, _)))) = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_padded_context_mut()
        .last_mut()
    else {
        return value;
    };
    if *active != actor || !attack.is_finite() || attack < 0.0 {
        return value;
    }
    // ToolAttackContext starts with attribute/2, then living base, then sprint.
    let Some(next) = rule::reduce(attack / 2.0 + 0.4_f32 + value.0, *level) else {
        return value;
    };
    *total = Some(next);
    let (suppress, remainder) = rule::split(next);
    (if suppress { 0.0 } else { remainder }, value.1)
}

pub(super) fn before_hurt(game: &mut Game<'_>, index: usize, source: &Damage) {
    if source.kind != DamageKind::PlayerAttack || source.attacker != Some(Who::Player) {
        return;
    }
    let Some(mob) = game.mobs.get(index) else {
        return;
    };
    let (id, resistance) = (mob.id, mob.attrs.knockback_resistance);
    let actor = game.lan_acting;
    let Some((active, Some((target, _, Some(total), previous)))) = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_padded_context_mut()
        .last_mut()
    else {
        return;
    };
    if *active != actor || *target != id || !rule::split(*total).0 || previous.is_some() {
        return;
    }
    // Temporarily suppress default hurt knockback, as ToolAttackUtil does.
    // The operation wrapper restores the previous resistance before pushing.
    *previous = Some(resistance);
    game.mobs[index].attrs.knockback_resistance = 1.0;
}

pub(super) fn after_hurt(game: &mut Game<'_>, index: usize, source: &Damage, landed: bool) {
    let actor = game.lan_acting;
    let Some(id) = game.mobs.get(index).map(|mob| mob.id) else {
        return;
    };
    let Some((active, Some((target, _, total, previous)))) = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_padded_context_mut()
        .last_mut()
    else {
        return;
    };
    if *active != actor || *target != id {
        return;
    }
    let (total, previous) = (total.take(), previous.take());
    if let Some(previous) = previous {
        game.mobs[index].attrs.knockback_resistance = previous;
    }
    if !landed || source.kind != DamageKind::PlayerAttack || source.attacker != Some(Who::Player) {
        return;
    }
    let Some(total) = total else {
        return;
    };
    let (suppress, remainder) = rule::split(total);
    if suppress && previous.is_some() && remainder > 0.0 {
        let (x, z) = game.extra_knockback_direction();
        game.mobs[index].knockback(f64::from(remainder), x, z, source);
        game.stop_sprint_after_extra_knockback();
    }
    // The source post-hit push slows the attacker whenever its remainder is
    // positive. The host's extra-push path already clears sprint in that case.
    if remainder > 0.0 {
        if let Some(player) = game.player.as_mut() {
            player.velocity.x *= 0.6;
            player.velocity.z *= 0.6;
        }
    }
}

pub(super) fn end(game: &mut Game<'_>) {
    let actor = game.lan_acting;
    let contexts = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_padded_context_mut();
    if contexts.last().is_none_or(|(active, _)| *active != actor) {
        return;
    }
    if let Some((_, Some((id, _, _, Some(previous))))) = contexts.pop() {
        if let Some(mob) = game.mobs.iter_mut().find(|mob| mob.id == id) {
            mob.attrs.knockback_resistance = previous;
        }
    }
}
