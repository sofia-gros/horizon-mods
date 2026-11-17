//! Connects the effect immunity modifiers to PlutoLib's player tick: the wearer of armor with an immunity loses the effects it refuses, as
//! ModifierEvents.isPotionApplicable refuses them in the original (the game has no call to refuse an effect, so it is taken off at the tick).
use crate::{break_speed_hook::with_held_tool, effect_immunity::refuses, tool_stack};
use pluto_lib::ServerLivingEntityEvents;
use std::collections::BTreeMap;

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        if tick.effects.is_empty() { return; }
        // The counts of every worn piece add up for an effect; a broken piece adds nothing (with_held_tool leaves it alone).
        let mut counts: BTreeMap<String, i32> = BTreeMap::new();
        for (_, piece) in &tick.armor {
            let Ok(immunities) = with_held_tool(piece, |env, stack| tool_stack::effect_immunities(&stack.custom_data, env)) else { continue; };
            for (effect, count) in immunities { *counts.entry(effect.to_string()).or_insert(0) += count; }
        }
        for (id, amplifier, _) in &tick.effects {
            if counts.get(id).is_some_and(|count| refuses(*count, *amplifier)) { tick.remove_effects.borrow_mut().push(id.clone()); }
        }
    });
}
