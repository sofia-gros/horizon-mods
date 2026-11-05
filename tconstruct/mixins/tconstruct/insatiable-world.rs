use super::{Game, tconstruct_insatiable as rules};
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;

fn selected<'g>(game: &'g Game<'_>) -> Option<&'g ItemStack> {
    game.inventory.get(game.selected)?.as_ref()
}

fn level(game: &Game<'_>, stack: &ItemStack) -> Option<i32> {
    if stack.count != 1
        || stack.wire_patch().is_some()
        || super::tconstruct_source_melee_shape::intact(stack).is_none()
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
        .get("tconstruct:modifiable/melee")?
        .contains(&*stack.item)
    {
        return None;
    }
    // Insatiable is a static Java modifier, not a bundled JSON module. A
    // replacement definition is outside this implementation's known contract.
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    if raw
        .0
        .contains_key("tconstruct:tinkering/modifiers/insatiable")
    {
        return None;
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let level = super::tconstruct_source_material_traits::level_with_upgrade(
        stack,
        &traits,
        "tconstruct:insatiable",
    )?;
    (level > 0).then_some(level)
}

pub(super) fn begin(game: &mut Game<'_>, index: usize) {
    if game.remote.is_some() {
        return;
    }
    let actor = game.lan_acting;
    let entry = (|| {
        if game
            .player
            .as_ref()
            .is_none_or(|player| player.auto_spin_attack_dmg > 0.0)
        {
            return None;
        }
        if game
            .world
            .chunks
            .__horizon_mixins_tconstruct_melee_active()
            .contains(&actor)
        {
            return None;
        }
        let stack = selected(game)?;
        level(game, stack)?;
        Some((
            game.mobs.get(index)?.id,
            stack.clone(),
            rules::charged(game.attack_strength_scale()),
        ))
    })();
    let contexts = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_insatiable_context_mut();
    // Ambiguous recursive attacks must not credit an older attack scope.
    for (owner, pending) in contexts.iter_mut() {
        if *owner == actor {
            *pending = None;
        }
    }
    contexts.push((actor, entry));
}

pub(super) fn end(game: &mut Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let actor = game.lan_acting;
    let contexts = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_insatiable_context_mut();
    if contexts.last().is_some_and(|(owner, _)| *owner == actor) {
        contexts.pop();
    }
}

fn same_tool(before: &ItemStack, now: &ItemStack) -> bool {
    let mut before = before.clone();
    before.damage = now.damage;
    before.count == now.count && before.same_item_same_components(now)
}

// Called after native melee damage: source criticals add a fraction of BASE
// damage only, so this conditional bonus is cooldown-scaled, not crit-scaled.
pub(super) fn damage(game: &Game<'_>, scale: f32) -> f32 {
    let bonus = (|| {
        if game.remote.is_some() || !scale.is_finite() {
            return None;
        }
        let (owner, pending) = game
            .world
            .chunks
            .__horizon_mixins_tconstruct_insatiable_context()
            .last()?;
        if *owner != game.lan_acting {
            return None;
        }
        let (_, before, _) = pending.as_ref()?;
        let stack = selected(game)?;
        if !same_tool(before, stack) {
            return None;
        }
        let level = level(game, stack)?;
        // Existing projection stores the shared conditional ATTACK_DAMAGE
        // multiplier under this legacy Raging field, including Emerald.
        let custom = stack.data()?.other("minecraft:custom_data")?;
        let Tag::Float(bits) = custom.get("tic_raging_melee_multiplier")? else {
            return None;
        };
        let mut flat = rules::bonus(
            game.potions.effects.amplifier(rules::EFFECT),
            level,
            f32::from_bits(*bits),
        )?;
        let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
        let raw = game
            .world
            .chunks
            .__horizon_mixins_tconstruct_modifier_resource_snapshot()
            .as_ref()?
            .as_ref()
            .ok()?;
        let wear_level = |id| {
            if super::tconstruct_modifier_snapshot::known_melee_wear(Some(raw), id) {
                traits.levels.get(id).copied().unwrap_or(0)
            } else {
                0
            }
        };
        let jagged = wear_level("tconstruct:jagged");
        let stonebound = wear_level("tconstruct:stonebound");
        if jagged != 0 || stonebound != 0 {
            flat += super::tconstruct_source_material_effects::wear_damage_bonus(
                flat,
                stack.damage,
                traits.durability_multiplier,
                jagged,
                stonebound,
            );
        }
        let scaled = flat * (0.2 + scale * scale * 0.8);
        scaled.is_finite().then_some(scaled)
    })();
    bonus.unwrap_or(0.0)
}

pub(super) fn hit(game: &mut Game<'_>, index: usize, landed: bool) {
    if game.remote.is_some() {
        return;
    }
    let actor = game.lan_acting;
    let contexts = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_insatiable_context_mut();
    let Some((owner, pending)) = contexts.last_mut() else {
        return;
    };
    if *owner != actor {
        return;
    }
    let Some((target, before, charged)) = pending.take() else {
        return;
    };
    if !landed || !charged || game.mobs.get(index).is_none_or(|mob| mob.id != target) {
        return;
    }
    let Some(stack) = selected(game) else {
        return;
    };
    if !same_tool(&before, stack)
        || level(game, stack).is_none()
        || game
            .player
            .as_ref()
            .is_none_or(|player| player.health.dead_or_dying())
    {
        return;
    }
    let amplifier = rules::next_amplifier(game.potions.effects.amplifier(rules::EFFECT));
    let mut effect =
        horizon_client::effects::EffectInstance::new(rules::EFFECT, rules::DURATION, amplifier);
    effect.visible = false; // source: particles off, HUD icon on
    game.add_player_effect(effect);
}
