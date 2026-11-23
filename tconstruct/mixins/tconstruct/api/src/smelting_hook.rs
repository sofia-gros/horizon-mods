//! Connects the smelting modules' cooking to the hits and shots that power it: a melee hit cooks
//! by the damage it deals (SmeltingModule.afterMeleeHit, read off the damage event's dealt
//! estimate as lifesteal reads it), and a shot cooks by the launcher's projectile damage stat
//! (onProjectileLaunch, whose arrow power is that stat for a tool's own arrows). The block
//! harvest cooking rides block_break_hook (finishHarvest) and the plant harvest cooking
//! harvest_hook (afterHarvest). The original's remaining surfaces — damage taken while wearing
//! the tool as armor, a launcher's hits, shearing, a monster's held tool and a sling's launch —
//! are not connected: their flows carry no consumer of this shape yet, and the cooking engine
//! (tool_stack::cook_smelting) is ready for them.
use crate::{break_speed_hook::with_held_tool, tool_stack, tool_stats::Stat};
use pluto_lib::{ActionTarget, EntityAction, ProjectileEvents, ServerLivingEntityEvents, StackSlot};

const PLAYER_ATTACK: &str = "minecraft:player_attack";

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
        let Some(held) = hit.attacker_stack.as_ref() else { return; };
        if hit.damage.damage_type != PLAYER_ATTACK { return; }
        let amount = hit.amount.get();
        let dealt = hit.target_state.as_ref().and_then(|state| state.health).map_or(amount, |health| amount.min(health));
        if dealt <= 0.0 { return; }
        let Ok(edits) = with_held_tool(held, |env, stack| tool_stack::smelting_edits(&stack.custom_data, env, dealt)) else { return; };
        if edits.is_empty() { return; }
        hit.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: held.item.clone(), edits });
    });
    ProjectileEvents::SHOOT_BOW.register(|shot| {
        for (hand, slot) in [(&shot.main_hand, StackSlot::MainHand), (&shot.off_hand, StackSlot::OffHand)] {
            let Some(stack) = hand else { continue };
            let Ok(edits) = with_held_tool(stack, |env, tool| tool_stack::smelting_edits(&tool.custom_data, env, tool_stack::stat(&tool.custom_data, Stat::ProjectileDamage))) else { continue };
            if edits.is_empty() { continue; }
            shot.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot, item: stack.item.clone(), edits });
            return;
        }
    });
}
