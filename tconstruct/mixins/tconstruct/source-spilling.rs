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
    entries.pop();
    if entries.is_empty() { contexts.remove(&actor); }
}

// weapon_hit is called only for a landed attack, after the attack log is
// appended but before weapon_wear; the source also applies before tool wear.
pub(super) fn hit(game: &mut super::Game<'_>, index: usize) {
    if game.remote.is_some() { return; }
    let actor = game.lan_acting;
    let contexts = game.world.chunks.__horizon_mixins_tconstruct_spilling_context_mut();
    let Some(entry) = contexts.get_mut(&actor).and_then(|entries| entries.last_mut()).and_then(Option::take) else { return; };
    let (target, mut before) = entry;
    if game.mobs.get(index).is_none_or(|mob| mob.id != target) { return; }
    let Some(record) = game.player.as_ref().and_then(|p| p.attack_log.last()) else { return; };
    if record.id != target || !record.landed || record.dealt <= 0.0 || record.scale <= 0.9 { return; }
    let Some(stack) = game.held().cloned() else { return; };
    // Wear from this hit may change damage; all other components must still match.
    before.damage = stack.damage;
    if before.count != stack.count || !before.same_item_same_components(&stack) || stack.count != 1 || stack.wire_patch().is_some() || super::tconstruct_source_melee_shape::intact(&stack).is_none() { return; }
    let Some(index) = game.mobs.iter().position(|mob| mob.id == target && mob.alive()) else { return; };
    // Unknown modded entities and entities with special overrides remain
    // unavailable until their canBeAffected behavior is represented by the host.
    let mob = &game.mobs[index];
    let kind = mob.kind.id.strip_prefix("minecraft:").unwrap_or(&mob.kind.id);
    if !matches!(kind, "pig" | "cow" | "sheep" | "chicken" | "rabbit" | "horse" | "donkey" | "mule" | "llama" | "trader_llama" | "goat" | "panda" | "polar_bear" | "fox" | "wolf" | "cat" | "ocelot" | "parrot" | "turtle" | "frog" | "axolotl" | "squid" | "glow_squid" | "cod" | "salmon" | "tropical_fish" | "pufferfish" | "dolphin" | "bat" | "villager" | "wandering_trader" | "zombie" | "husk" | "drowned" | "zombie_villager" | "skeleton" | "stray" | "bogged" | "zombified_piglin" | "zoglin" | "phantom" | "creeper" | "enderman" | "endermite" | "silverfish" | "slime" | "magma_cube" | "witch" | "pillager" | "vindicator" | "piglin" | "piglin_brute" | "hoglin" | "ghast" | "blaze" | "strider" | "guardian" | "elder_guardian" | "evoker" | "vex" | "ravager") { return; }
    let undead = mob.kind.flags.undead;
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
        let effect = match effect {
            super::tconstruct_source_fluid_effects::EntityEffect::Mob(effect) => effect,
            // RestoreHungerFluidEffect applies only to players, not mobs.
            super::tconstruct_source_fluid_effects::EntityEffect::Hunger { .. } => continue,
            super::tconstruct_source_fluid_effects::EntityEffect::Remove(id) => {
                let Some(known) = horizon_client::brewing::effect(&id) else { return; };
                if strength >= 1.0 && game.mobs[index].extra.effects.iter().any(|entry| entry.0 == horizon_client_core::potions::namespaced(known.id)) { return; }
                continue;
            }
        };
        let Some(known) = horizon_client::brewing::effect(&effect.id) else { return; };
        let id = horizon_client_core::potions::namespaced(known.id);
        if !ids.insert(id) || matches!(id, "minecraft:instant_health" | "minecraft:instant_damage") { return; }
        // Vanilla undead reject poison and regeneration before addEffect.
        if undead && matches!(id, "minecraft:poison" | "minecraft:regeneration") { continue; }
        let existing = game.mobs[index].extra.effects.iter().find(|entry| entry.0 == id).map(|entry| (entry.1, entry.2));
        let Some((duration, scale)) = super::tconstruct_source_fluid_effects::mob_duration(&effect, strength, *level as f32, existing) else { continue; };
        let used = ((recipe.amount as f32 * scale.min(strength)).ceil() as u32).min(fluid.amount);
        if used > 0 { planned.push((id, duration, effect.amplifier, used)); }
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
