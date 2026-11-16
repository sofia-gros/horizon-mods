//! Connects the protection modules to PlutoLib's damage event: the damage a player takes while wearing Tinkers armor is
//! reduced (or raised) by the armor's protection modifiers, as ToolEvents.livingHurt does in the original.
//! The event fires after vanilla's armor and enchantment protection; vanilla enchantment protection is taken as 0 points,
//! so armor that also carries vanilla protection enchantments is protected by it twice.
use crate::{break_speed_hook::with_held_tool, damage_tags::{self, DamageTags}, protection::{damage_after_protection, protection_cap, DamageSource}, tool_stack};
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType, ServerLifecycleEvents, ServerLivingEntityEvents};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc, RwLock};

static TAGS: RwLock<Option<Arc<DamageTags>>> = RwLock::new(None);

/// The damage-type tags of the last reload, empty before the first.
fn tags() -> Arc<DamageTags> {
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
    static LOGGED: AtomicUsize = AtomicUsize::new(0);
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
        if hit.armor.is_empty() { return; }
        let Ok(damage_type) = Identifier::parse(&hit.damage.damage_type) else { return; };
        let tags = tags();
        let source = DamageSource { damage_type: &damage_type, indirect: hit.damage.attacker != hit.damage.direct, tags: &tags };
        // Each piece's modifiers see the protection so far, feet to head.
        let mut total = 0.0;
        let mut cap_added = 0.0;
        for (_, piece) in &hit.armor {
            if let Ok((value, cap)) = with_held_tool(piece, |env, stack| (tool_stack::protection(&stack.custom_data, env, &source, total), tool_stack::protection_cap(&stack.custom_data, env))) { total = value; cap_added += cap; }
        }
        if total == 0.0 { return; }
        let amount = hit.amount.get();
        let after = damage_after_protection(amount, 0.0, total, protection_cap(cap_added));
        if LOGGED.fetch_add(1, Ordering::Relaxed) < 4 {
            eprintln!("Tinkers: {} damage to {} with {} armor pieces: protection {total}, {amount} -> {after}", hit.damage.damage_type, hit.damage.kind, hit.armor.len());
        }
        if after.is_finite() && after >= 0.0 && after != amount { hit.amount.set(after); }
    });
}
