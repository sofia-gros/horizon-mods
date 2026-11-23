//! Connects the protection modules to PlutoLib's damage event: the damage a player takes while wearing Tinkers armor is
//! reduced (or raised) by the armor's protection modifiers, as ToolEvents.livingHurt does in the original.
//! The event fires after vanilla's armor and enchantment protection. The enchantment points counted are those of the enchantments the
//! armor carries in full — the ones its modifiers give (the stack carries them, so the game applied them) plus the ones a player added,
//! which the last slot sync recorded in the tool's data (tool_slot): the damage event reports no components, so a piece enchanted since
//! its last sync is counted without the new enchantments until it is next synced. Modifiers that ignore the enchantment's protection take its points off again.
use crate::{break_speed_hook::with_held_tool, common, damage_tags::{self, DamageTags}, protection::{damage_after_protection, protection_cap, DamageSource}, tool_stack};
use pluto_lib::{armor_stack_slot, ActionTarget, DamageModification, EntityAction, Identifier, ResourceManagerHelper, ResourceType, ServerLifecycleEvents, ServerLivingEntityEvents};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc, RwLock};

static TAGS: RwLock<Option<Arc<DamageTags>>> = RwLock::new(None);

/// The damage-type tags of the last reload, empty before the first.
pub(crate) fn tags() -> Arc<DamageTags> {
    TAGS.read().ok().and_then(|tags| tags.clone()).unwrap_or_default()
}

/// Declares the damage tag folder and publishes its tags on each reload; a failed reload keeps none.
pub(crate) fn register_tags() -> Result<(), String> {
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "damage_tags").map_err(|e| e.to_string())?, damage_tags::TAG_FOLDER,
        |_| {
            let stacks = ResourceManagerHelper::reload_json_stacks(damage_tags::TAG_FOLDER).ok_or("damage tag folder was not captured")?;
            let (loaded, problems) = DamageTags::load(&stacks);
            for problem in &problems { eprintln!("Tinkers: {problem}"); }
            *TAGS.write().map_err(|_| "damage tags poisoned")? = Some(Arc::new(loaded));
            Ok(())
        },
    )?;
    ServerLifecycleEvents::SERVER_STOPPED.register(|_| { if let Ok(mut tags) = TAGS.write() { *tags = None; } });
    Ok(())
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| { with_source(hit, |source| protect(hit, source)); });
    // Recurrent protection is a modify hurt module: it runs on the damage the protection left, before damage to capacity.
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| { with_source(hit, |source| recurrent(hit, source)); });
    // Damage to capacity runs after the protection (ModifierHooks.MODIFY_DAMAGE follows MODIFY_HURT): each piece's bars take what they can of
    // the damage left, and the fills reach the pieces on the wearer's next tick.
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| { with_source(hit, |source| fill_capacity(hit, source)); });
    // Restore lost health is the last of the modify damage modules (priority 10): it works on the damage that is left.
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| { with_source(hit, |source| restore_health(hit, source, false)); });
    // The same module bound to the modify hurt hook runs as the damage arrives, before armor: a share of the damage as it
    // came in comes back as regeneration. (ModifierHooks.MODIFY_HURT follows ALLOW_DAMAGE and precedes MODIFY_DAMAGE.)
    ServerLivingEntityEvents::MODIFY_HURT.register(|hit| { with_source(hit, |source| restore_health(hit, source, true)); });
}

/// RecurrentProtectionModule: the momentum the wearer already has comes off the damage, and a share of what is left
/// becomes their new momentum, granted as the momentum (armor) effect. The level is the total over the worn pieces,
/// as SlotInChargeModule totals them; the module's numbers are the first piece's, as a single slot is in charge.
/// The effect is the original's own, which the game does not know until a mod effect registration exists; the host
/// skips an effect it does not know, as it does the other Tinkers effects the bound modules name. The wearer's
/// momentum is read from the state of their last tick, as the rest of this hook reads them.
fn recurrent(hit: &DamageModification, source: &DamageSource) {
    if source.tags.contains_vanilla("bypasses_invulnerability", source.damage_type) { return; }
    let mut total = 0;
    let mut module = None;
    for (_, piece) in &hit.armor {
        let Ok(found) = with_held_tool(piece, |env, stack| tool_stack::recurrent_protection(&stack.custom_data, env)) else { continue };
        for (level, candidate) in found {
            total += level;
            if module.is_none() { module = Some(candidate); }
        }
    }
    let Some(module) = module else { return };
    let momentum = hit.target_state.as_ref().map_or(0, |state| state.effect_level(crate::wearer_modules::RecurrentProtection::EFFECT));
    let amount = hit.amount.get();
    let (after, grant) = module.apply(total, momentum, amount);
    if let Some((amplifier, duration)) = grant {
        hit.actions.borrow_mut().push(EntityAction::AddEffect { target: ActionTarget::Victim, effect: crate::wearer_modules::RecurrentProtection::EFFECT.to_owned(), amplifier, duration });
    }
    // The original lets the damage go below zero; the event takes none then.
    if after != amount && after.is_finite() { hit.amount.set(after.max(0.0)); }
}

/// RestoreLostHealthModule: a share of the damage the wearer takes comes back as Regeneration, at a chance, and wears the piece. The effect and the
/// edits of the piece reach the wearer on their next tick; several pieces give the effect as the game adds them (the longer one stays).
fn restore_health(hit: &DamageModification, source: &DamageSource, hurt: bool) {
    let amount = hit.amount.get();
    if !(amount > 0.0) { return; }
    let mut actions = Vec::new();
    for (slot, piece) in &hit.armor {
        let Some(stack_slot) = armor_stack_slot(slot) else { continue };
        let Ok(restores) = with_held_tool(piece, |env, stack| tool_stack::restores(&stack.custom_data, env, source, amount, hurt, &mut crate::tool_damage_hook::random)) else { continue };
        for (modifier, restore) in restores {
            actions.push(EntityAction::AddEffect { target: ActionTarget::Victim, effect: "minecraft:regeneration".into(), amplifier: 0, duration: restore.duration });
            if restore.wear <= 0 { continue; }
            let edits = with_held_tool(piece, |env, stack| crate::wearer_modules::wear_edits(&stack.custom_data, env, &modifier, restore.wear, &mut crate::tool_damage_hook::random)).unwrap_or_default();
            if !edits.is_empty() { actions.push(EntityAction::EditStackData { target: ActionTarget::Victim, slot: stack_slot, item: piece.item.clone(), edits }); }
        }
    }
    hit.actions.borrow_mut().extend(actions);
}

/// Runs `run` on the damage a player's armor is asked about; nothing for an entity that wears no armor with custom data or a damage type that
/// does not parse.
fn with_source(hit: &DamageModification, run: impl FnOnce(&DamageSource)) {
    if hit.armor.is_empty() { return; }
    let Ok(damage_type) = Identifier::parse(&hit.damage.damage_type) else { return; };
    let tags = tags();
    // The attacker's entity type tags and what it was doing (the states of a hit by a mob come with the hit).
    let entities = common::entity_type_snapshot();
    let attacker_kind = hit.attacker_state.as_ref().and_then(|state| Identifier::parse(&state.kind).ok());
    let attacker_in_tag = |tag: &Identifier| {
        if tag.namespace() == crate::melee_damage::STATE_NAMESPACE { return crate::melee_damage::state_in_tag(tag, hit.attacker_state.as_ref(), hit.target_state.as_ref()); }
        crate::melee_hook::target_in_tag(entities.as_deref(), attacker_kind.as_ref(), tag)
    };
    // The wearer is the target of the hit, as PlutoLib reported them as of their last tick.
    let entities = crate::entity_facts::Entities { attacker: hit.attacker_state.as_ref(), target: hit.target_state.as_ref() };
    run(&DamageSource { damage_type: &damage_type, indirect: hit.damage.attacker != hit.damage.direct, tags: &tags, attacker_in_tag: &attacker_in_tag, entities });
}

/// Puts each worn piece's damage to capacity modules to work on the damage left, feet to head, and queues the edits that fill the bars.
fn fill_capacity(hit: &DamageModification, source: &DamageSource) {
    let mut amount = hit.amount.get();
    let mut edits = Vec::new();
    for (slot, piece) in &hit.armor {
        let Some(stack_slot) = armor_stack_slot(slot) else { continue };
        let Ok((left, piece_edits)) = with_held_tool(piece, |env, stack| tool_stack::damage_capacity(&stack.custom_data, env, source, amount)) else { continue };
        amount = left;
        if !piece_edits.is_empty() { edits.push(EntityAction::EditStackData { target: ActionTarget::Victim, slot: stack_slot, item: piece.item.clone(), edits: piece_edits }); }
    }
    if edits.is_empty() { return; }
    // The original lets the damage go below zero (the next piece sees that); the event takes none then.
    if amount.is_finite() { hit.amount.set(amount.max(0.0)); }
    hit.actions.borrow_mut().extend(edits);
}

/// Whether the damaged entity is one of the original's small armor victims: `forge:small_armor`, whose only
/// member is `#forge:slimes` — the two vanilla slimes and the three Tinkers ones, named outright because the
/// tag reaches the snapshot only when the game captured it. The tag is consulted too, for a data pack that
/// adds a member of its own.
fn small_armor_victim(kind: &str) -> bool {
    if matches!(kind, "minecraft:slime" | "minecraft:magma_cube" | "tconstruct:sky_slime" | "tconstruct:ender_slime" | "tconstruct:terracube") { return true; }
    let Ok(kind) = Identifier::parse(kind) else { return false };
    let Ok(tag) = Identifier::parse("forge:small_armor") else { return false };
    crate::melee_hook::target_in_tag(common::entity_type_snapshot().as_deref(), Some(&kind), &tag)
}

/// Protection: the damage after the armor's protection modules (and the damage a piece blocks altogether).
fn protect(hit: &DamageModification, source: &DamageSource) {
    static LOGGED: AtomicUsize = AtomicUsize::new(0);
    // A piece that blocks this kind of damage (long fall, frost walker, rugged) takes all of it away.
    if hit.armor.iter().any(|(_, piece)| with_held_tool(piece, |env, stack| tool_stack::blocks_damage(&stack.custom_data, env, source)).unwrap_or(false)) {
        hit.amount.set(0.0);
        return;
    }
    // The points the game's enchantment protection already took off for the enchantments the pieces carry, given and player-added alike.
    let levels: Vec<_> = hit.armor.iter().filter_map(|(_, piece)| with_held_tool(piece, |env, stack| tool_stack::carried_enchantments(&stack.custom_data, env)).ok()).collect();
    let vanilla = crate::protection::host_enchantment_points(&levels, source);
    // Each piece's modifiers see the protection so far, feet to head, starting from the enchantment points.
    let mut total = vanilla;
    let mut cap_added = 0.0;
    let mut spent = Vec::new();
    for (slot, piece) in &hit.armor {
        if let Ok((value, cap, edits)) = with_held_tool(piece, |env, stack| {
            let (value, edits) = tool_stack::protection_with_edits(&stack.custom_data, env, &source, total);
            (value, tool_stack::protection_cap(&stack.custom_data, env), edits)
        }) {
            total = value;
            cap_added += cap;
            // What the piece's modules spent (an overshield's overslime) comes off the piece on the wearer's next tick.
            if let (false, Some(stack_slot)) = (edits.is_empty(), armor_stack_slot(slot)) { spent.push(EntityAction::EditStackData { target: ActionTarget::Victim, slot: stack_slot, item: piece.item.clone(), edits }); }
        }
    }
    hit.actions.borrow_mut().extend(spent);
    // ToolEvents.getProtectionModifier: a small armor victim's accumulated protection counts fourfold.
    let total = if small_armor_victim(&hit.damage.kind) { crate::protection::small_armor_total(total) } else { total };
    let amount = hit.amount.get();
    let mut after = if total == 0.0 && vanilla == 0.0 { amount } else { damage_after_protection(amount, vanilla, total, protection_cap(cap_added)) };
    // The modules that adjust the damage the wearer takes run after the protection, each piece seeing what the last left; the wearer is
    // the target of the hit, with the health and surroundings PlutoLib reported for them.
    let entities = crate::entity_facts::Entities { attacker: hit.attacker_state.as_ref(), target: hit.target_state.as_ref() };
    for (_, piece) in &hit.armor {
        if after <= 0.0 { break; }
        if let Ok(next) = with_held_tool(piece, |env, stack| tool_stack::adjust_damage(&stack.custom_data, env, source, &entities, after)) { after = next; }
    }
    if total == 0.0 && vanilla == 0.0 && after == amount { return; }
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 4 {
        eprintln!("Tinkers: {} damage to {} with {} armor pieces: protection {total}, {amount} -> {after}", hit.damage.damage_type, hit.damage.kind, hit.armor.len());
    }
    if after.is_finite() && after >= 0.0 && after != amount { hit.amount.set(after); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_small_armor_victims_are_the_slimes() {
        for kind in ["minecraft:slime", "minecraft:magma_cube", "tconstruct:sky_slime", "tconstruct:ender_slime", "tconstruct:terracube"] {
            assert!(small_armor_victim(kind), "{kind}");
        }
        assert!(!small_armor_victim("minecraft:zombie"));
        assert!(!small_armor_victim("minecraft:player"));
        assert!(!small_armor_victim("not an id"));
    }

    #[test]
    fn a_small_armor_victims_protection_counts_fourfold_in_the_damage() {
        // 10 damage with 2 enchantment points the game already took and 5 accumulated: a normal victim's
        // damage runs through the 5; a small armor victim's 5 counts as 20, the protection cap.
        let normal = damage_after_protection(10.0, 2.0, 5.0, protection_cap(0.0));
        let small = damage_after_protection(10.0, 2.0, crate::protection::small_armor_total(5.0), protection_cap(0.0));
        assert!((normal - 10.0 / (1.0 - 2.0 / 25.0) * (1.0 - 5.0 / 25.0)).abs() < 1e-4, "{normal}");
        assert!((small - 10.0 / (1.0 - 2.0 / 25.0) * (1.0 - 20.0 / 25.0)).abs() < 1e-4, "{small}");
        assert!(small < normal);
    }
}
