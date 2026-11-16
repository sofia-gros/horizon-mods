//! Connects the conditional melee damage modifiers to PlutoLib's damage event: when a player hits with a
//! Tinkers tool, its modifiers change the damage as MeleeDamageModifierHook does in the original. The event
//! reports the damage after the game worked it out (before armor), not the attack attribute alone, so the
//! `base_damage` variable is the player's base attack of 1 plus the tool's attack damage stat.
use crate::{break_speed_hook::with_held_tool, common, tool_stack, tool_stats::Stat};
use pluto_lib::{Identifier, ServerLivingEntityEvents};

const PLAYER_ATTACK: &str = "minecraft:player_attack";

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
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
        if let Ok(damage) = changed {
            if damage.is_finite() && damage >= 0.0 && damage != amount { hit.amount.set(damage); }
        }
    });
}
