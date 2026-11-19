//! Connects the protection modules to PlutoLib's damage event: the damage a player takes while wearing Tinkers armor is
//! reduced (or raised) by the armor's protection modifiers, as ToolEvents.livingHurt does in the original.
//! The event fires after vanilla's armor and enchantment protection. The enchantment points counted are those of the enchantments the
//! armor's own modifiers give (the stack carries them, so the game applied them); enchantments a player added are taken as 0 points,
//! so armor that also carries those is protected by them twice. Modifiers that ignore the enchantment's protection take its points off again.
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
    // Damage to capacity runs after the protection (ModifierHooks.MODIFY_DAMAGE follows MODIFY_HURT): each piece's bars take what they can of
    // the damage left, and the fills reach the pieces on the wearer's next tick.
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| { with_source(hit, |source| fill_capacity(hit, source)); });
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
    run(&DamageSource { damage_type: &damage_type, indirect: hit.damage.attacker != hit.damage.direct, tags: &tags, attacker_in_tag: &attacker_in_tag });
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

/// Protection: the damage after the armor's protection modules (and the damage a piece blocks altogether).
fn protect(hit: &DamageModification, source: &DamageSource) {
    static LOGGED: AtomicUsize = AtomicUsize::new(0);
    // A piece that blocks this kind of damage (long fall, frost walker, rugged) takes all of it away.
    if hit.armor.iter().any(|(_, piece)| with_held_tool(piece, |env, stack| tool_stack::blocks_damage(&stack.custom_data, env, source)).unwrap_or(false)) {
        hit.amount.set(0.0);
        return;
    }
    // The points the game's enchantment protection already took off for the enchantments the pieces' modifiers give.
    let levels: Vec<_> = hit.armor.iter().filter_map(|(_, piece)| with_held_tool(piece, |env, stack| tool_stack::enchantments(&stack.custom_data, env)).ok()).collect();
    let vanilla = crate::protection::host_enchantment_points(&levels, source);
    // Each piece's modifiers see the protection so far, feet to head, starting from the enchantment points.
    let mut total = vanilla;
    let mut cap_added = 0.0;
    for (_, piece) in &hit.armor {
        if let Ok((value, cap)) = with_held_tool(piece, |env, stack| (tool_stack::protection(&stack.custom_data, env, &source, total), tool_stack::protection_cap(&stack.custom_data, env))) { total = value; cap_added += cap; }
    }
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
