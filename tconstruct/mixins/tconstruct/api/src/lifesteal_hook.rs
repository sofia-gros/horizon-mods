//! Connects the lifesteal modules to PlutoLib's damage event: when a player's critical melee hit with a Tinkers tool goes ahead, the tool's
//! lifesteal modules heal the player by a share of the damage (LifestealModule.afterMeleeHit). The event reports the damage after every other
//! module changed it, so this runs after the melee damage callback; the heal is asked as an action on the attacker, which lands on the
//! player's next tick.
use crate::{break_speed_hook::with_held_tool, common, lifesteal::is_critical, melee_hook::target_in_tag, tool_stack};
use pluto_lib::{ActionTarget, EntityAction, Identifier, ServerLivingEntityEvents};

/// Registers the callback. Called once from the product's hooks, after the melee damage callback.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
        let Some(held) = hit.attacker_stack.as_ref() else { return; };
        if hit.damage.damage_type != "minecraft:player_attack" || hit.damage.attacker < 0 { return; }
        let Some(attacker_state) = hit.attacker_state.as_ref().filter(|state| is_critical(state)) else { return; };
        // The damage dealt cannot pass the health the mob had.
        let amount = hit.amount.get();
        let dealt = hit.target_state.as_ref().and_then(|state| state.health).map_or(amount, |health| amount.min(health));
        let entities = common::entity_type_snapshot();
        let target = Identifier::parse(&hit.damage.kind).ok();
        let player = Identifier::parse("minecraft:player").ok();
        let target_tag = |tag: &Identifier| {
            if tag.namespace() == crate::melee_damage::STATE_NAMESPACE { return crate::melee_damage::state_in_tag(tag, Some(attacker_state), hit.target_state.as_ref()); }
            target_in_tag(entities.as_deref(), target.as_ref(), tag)
        };
        let attacker_tag = |tag: &Identifier| {
            if tag.namespace() == crate::melee_damage::STATE_NAMESPACE { return crate::melee_damage::state_in_tag(tag, Some(attacker_state), hit.target_state.as_ref()); }
            target_in_tag(entities.as_deref(), player.as_ref(), tag)
        };
        let blacklisted = Identifier::new("tconstruct", "necrotic_blacklist").is_ok_and(|tag| target_tag(&tag));
        let Ok(heal) = with_held_tool(held, |env, stack| tool_stack::lifesteal_heal(&stack.custom_data, env, dealt, blacklisted, &attacker_tag, &target_tag)) else { return; };
        if heal > 0.0 { hit.actions.borrow_mut().push(EntityAction::Heal { target: ActionTarget::Attacker, amount: heal }); }
    });
}
