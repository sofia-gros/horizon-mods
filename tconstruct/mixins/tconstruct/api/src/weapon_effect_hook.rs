//! Connects the weapon mob effect modifiers to PlutoLib's weapon-hit event: when a player hits a mob with a Tinkers tool, each of the
//! tool's weapon effect modules that lands puts its status effect on the mob (MobEffectModule.Weapon). Only effects the game itself
//! defines (the `minecraft` ones) can land; the original's own effects (venom, conductive, pierce) have no registration here.
use crate::{break_speed_hook::with_held_tool, common, melee_damage, melee_hook, tool_damage_hook, tool_stack};
use pluto_lib::{EffectGrant, Identifier, ServerLivingEntityEvents};

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
        let Some(held) = hit.weapon.as_ref() else { return; };
        let entities = common::entity_type_snapshot();
        let target = Identifier::parse(&hit.kind).ok();
        let in_tag = |tag: &Identifier| {
            if tag.namespace() == melee_damage::STATE_NAMESPACE { return melee_damage::state_in_tag(tag, hit.attacker_state.as_ref(), hit.target_state.as_ref()); }
            melee_hook::target_in_tag(entities.as_deref(), target.as_ref(), tag)
        };
        let Ok(grants) = with_held_tool(held, |env, stack| tool_stack::weapon_effects(&stack.custom_data, env, &in_tag, &mut tool_damage_hook::random)) else { return; };
        for (_, grant) in grants {
            // Effects that apply before the hit and after it both land here, before the damage; the game has no later mod call.
            if grant.effect.namespace() != "minecraft" { continue; }
            hit.effects.borrow_mut().push(EffectGrant { effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration });
        }
    });
}
