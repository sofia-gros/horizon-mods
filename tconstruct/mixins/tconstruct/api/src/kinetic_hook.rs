//! Connects the kinetic module to PlutoLib's damage event: when a player wearing Tinkers armor is hurt
//! by an entity, the worn pieces' kinetic levels feed the insatiable (armor) effect, as
//! KineticModule's OnAttackedModifierHook does in the original. The original tracks which single slot
//! is in charge through equip changes (SlotInChargeModule) and that slot's level is the total over all
//! equipped pieces; a hit reports every worn piece, so the total is worked out over them here, as the
//! port works out armor levels. The effect is the original's own, which the game does not know until a
//! mod effect registration exists; the host skips an effect it does not know, as it does the other
//! Tinkers effects the bound modules name.
use crate::{break_speed_hook::with_held_tool, tool_stack};
use pluto_lib::{ActionTarget, EntityAction, ServerLivingEntityEvents};

/// The insatiable effect for armor, whose level the wearer's attack damage follows.
const EFFECT: &str = "tconstruct:insatiable_armor";

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
        // OnAttackedModifierHook runs for damage an entity caused; the event names a negative attacker
        // for damage with none (a fall, fire, drowning).
        if hit.damage.attacker < 0 || hit.armor.is_empty() { return; }
        let mut total = 0;
        for (_, piece) in &hit.armor {
            if let Ok(level) = with_held_tool(piece, |env, stack| tool_stack::kinetic_level(&stack.custom_data, env)) { total += level; }
        }
        // The wearer's current amplifier of the effect, as of their last tick (-1 when they lack it).
        let current = hit.target_state.as_ref().map_or(-1, |state| state.effect_levels.iter()
            .find(|(id, _)| id == EFFECT).map_or(-1, |(_, amplifier)| *amplifier));
        if let Some(grant) = crate::wearer_modules::Kinetic::grant(total, current) {
            hit.actions.borrow_mut().push(EntityAction::AddEffect { target: ActionTarget::Victim, effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration });
        }
    });
}
