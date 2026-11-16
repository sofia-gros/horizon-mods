//! Connects the conditional melee damage modifiers to PlutoLib's damage event: when a player hits with a
//! Tinkers tool, its modifiers change the damage as MeleeDamageModifierHook does in the original. The event
//! reports the damage after the game worked it out (before armor), not the attack attribute alone, so the
//! `base_damage` variable is the player's base attack of 1 plus the tool's attack damage stat.
use crate::{break_speed_hook::with_held_tool, common, tool_stack, tool_stats::Stat};
use pluto_lib::{Identifier, ServerLivingEntityEvents};
use std::sync::atomic::{AtomicUsize, Ordering};

const PLAYER_ATTACK: &str = "minecraft:player_attack";

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    // The first few hits are logged with what the hook saw, so a hit it leaves alone is visible in the game log.
    static LOGGED: AtomicUsize = AtomicUsize::new(0);
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
        let log = LOGGED.fetch_add(1, Ordering::Relaxed) < 8;
        if log {
            eprintln!("Tinkers: damage {} of {} on {} (stack: {:?})", hit.damage.damage_type, hit.amount.get(), hit.damage.kind, hit.attacker_stack.as_ref().map(|stack| stack.item.as_str()));
        }
        let Some(held) = hit.attacker_stack.as_ref() else { return; };
        if hit.damage.damage_type != PLAYER_ATTACK { return; }
        let amount = hit.amount.get();
        // The target's entity type tags come from the entity type registry captured at the last reload.
        let entities = common::entity_type_snapshot();
        let target = Identifier::parse(&hit.damage.kind).ok();
        let target_in_tag = |tag: &Identifier| match (&entities, &target) {
            (Some(entities), Some(target)) => entities.get_tag(tag).is_some_and(|members| members.contains(target)),
            _ => false,
        };
        let changed = with_held_tool(held, |env, stack| {
            let base = 1.0 + tool_stack::stat(&stack.custom_data, Stat::AttackDamage);
            tool_stack::melee_damage(&stack.custom_data, env, base, amount, &target_in_tag)
        });
        if log {
            let undead = Identifier::parse("minecraft:undead").ok();
            eprintln!("Tinkers: melee {:?}, entity types captured: {}, undead tag known: {}, target in undead: {}",
                changed, entities.is_some(), undead.as_ref().is_some_and(|tag| entities.as_ref().is_some_and(|e| e.get_tag(tag).is_some())), undead.as_ref().is_some_and(|tag| target_in_tag(tag)));
        }
        if let Ok(damage) = changed {
            if damage.is_finite() && damage >= 0.0 && damage != amount { hit.amount.set(damage); }
        }
    });
}
