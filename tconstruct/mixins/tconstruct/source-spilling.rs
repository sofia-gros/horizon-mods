//! Source melee spilling: one server-side attack owns one pending context.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::HashSet;

pub(super) fn begin(game: &mut super::Game<'_>, index: usize) {
    if game.remote.is_some() { return; }
    let actor = game.lan_acting;
    let entry = game.mobs.get(index).zip(game.held()).and_then(|(mob, stack)| {
        let custom = stack.data()?.other("minecraft:custom_data")?;
        if !matches!(custom.get("tic_upgrades")?.get("tconstruct:spilling"), Some(Tag::Int(n)) if *n > 0) { return None; }
        Some((mob.id, stack.clone()))
    });
    let contexts = game.world.chunks.__horizon_mixins_tconstruct_spilling_context_mut();
    let pending = contexts.entry(actor).or_default();
    // A nested attack can replace the log's last record. Do not associate it
    // with an enclosing attack even when both happen to hit the same target.
    for parent in pending.iter_mut() { *parent = None; }
    pending.push(entry);
}

pub(super) fn end(game: &mut super::Game<'_>) {
    if game.remote.is_some() { return; }
    let actor = game.lan_acting;
    let contexts = game.world.chunks.__horizon_mixins_tconstruct_spilling_context_mut();
    let Some(entries) = contexts.get_mut(&actor) else { return; };
    let entry = entries.pop();
    if entries.is_empty() { contexts.remove(&actor); }
    let Some(Some((target, mut before))) = entry else { return; };
    let Some(record) = game.player.as_ref().and_then(|p| p.attack_log.last()) else { return; };
    if record.id != target || !record.landed || record.dealt <= 0.0 || record.scale <= 0.9 { return; }
    let Some(stack) = game.held().cloned() else { return; };
    // Wear from this hit may change damage; all other components must still match.
    before.damage = stack.damage;
    if before.count != stack.count || !before.same_item_same_components(&stack) || stack.count != 1 || stack.wire_patch().is_some() || super::tconstruct_source_melee_shape::intact(&stack).is_none() { return; }
    let Some(index) = game.mobs.iter().position(|mob| mob.id == target && mob.alive()) else { return; };
    // The host has no complete LivingEntity.canBeAffected API; these ordinary
    // vanilla species have no special status-effect immunity overrides.
    if !matches!(game.mobs[index].kind.id.as_str(), "minecraft:pig" | "minecraft:cow" | "minecraft:sheep" | "minecraft:chicken" | "pig" | "cow" | "sheep" | "chicken") { return; }
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return; };
    if matches!(custom.get("tic_broken"), Some(Tag::Byte(n)) if *n != 0) { return; }
    let Some(Tag::Int(level)) = custom.get("tic_upgrades").and_then(|upgrades| upgrades.get("tconstruct:spilling")) else { return; };
    if *level <= 0 { return; }
    let Some(Tag::Int(capacity)) = custom.get("tic_tank_capacity") else { return; };
    if *capacity <= 0 { return; }
    let Some(Some(fluid)) = super::pluto_stack_fluid::read(&stack, "tic_persistent", "tconstruct:tank_fluid") else { return; };
    let capacity = *capacity as u32;
    if fluid.amount > capacity || ((game.lan.is_some() || game.lan_acting_uuid.is_some()) && fluid.tag.is_some()) { return; }
    let Some(recipe) = super::tconstruct_source_slurping::recipe(game, &fluid.id) else { return; };
    let strength = (*level as f32).min(fluid.amount as f32 / recipe.amount as f32);
    if strength <= 0.0 { return; }
    let mut planned = Vec::new();
    let mut ids = HashSet::new();
    for effect in recipe.entity {
        let super::tconstruct_source_fluid_effects::EntityEffect::Mob(effect) = effect else { return; };
        let Some(known) = horizon_client::brewing::effect(&effect.id) else { return; };
        if !ids.insert(known.id) || matches!(known.id, "minecraft:instant_health" | "minecraft:instant_damage") { return; }
        let existing = game.mobs[index].extra.effects.iter().find(|entry| entry.0 == known.id).map(|entry| (entry.1, entry.2));
        let Some((duration, scale)) = super::tconstruct_source_fluid_effects::mob_duration(&effect, strength, *level as f32, existing) else { continue; };
        let used = ((recipe.amount as f32 * scale.min(strength)).ceil() as u32).min(fluid.amount);
        if used > 0 { planned.push((known.id, duration, effect.amplifier, used)); }
    }
    if planned.is_empty() { return; }
    // Validate every possible remainder before applying any effect.
    let mut remainders = Vec::new();
    for (_, _, _, used) in &planned {
        let mut fluid = fluid.clone(); fluid.amount -= *used;
        let Some(drained) = super::pluto_stack_fluid::write(&stack, "tic_persistent", "tconstruct:tank_fluid", (fluid.amount > 0).then_some(&fluid), capacity) else { return; };
        remainders.push(drained);
    }
    let mut consumed = None;
    for ((id, duration, amplifier, used), drained) in planned.into_iter().zip(remainders) {
        let mob = &mut game.mobs[index];
        let before = mob.extra.effects.iter().find(|entry| entry.0 == id).copied();
        mob.add_effect(id, duration, amplifier);
        let after = mob.extra.effects.iter().find(|entry| entry.0 == id).copied();
        if before != after && consumed.as_ref().is_none_or(|(amount, _)| used > *amount) { consumed = Some((used, drained)); }
    }
    if !game.creative { if let Some((_, drained)) = consumed { *game.held_mut() = Some(drained); } }
}
