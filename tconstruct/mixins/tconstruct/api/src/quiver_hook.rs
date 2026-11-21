//! Connects the quiver modifiers to PlutoLib's ammo event (the LivingGetProjectileEvent surface): when a
//! player shoots a tool holding a quiver module, the tool's own inventory supplies the projectile and is
//! shrunk by one, as BowAmmoModifierHook runs the weapon's entries in the original. SlimeKnights (MIT; see
//! LICENSE).
//!
//! Limits of the surface, kept honest:
//! - A mob shooter reports no carried stacks, so there is nowhere to shrink the quiver from: a mob's quiver
//!   stays silent (the original shrinks the mob's tool the same way, but this event cannot write it back).
//! - The event names no creative flag, so a creative player's quiver is drawn from like anyone's when the
//!   game itself found no standard ammo (in creative the game's own pick is a plain arrow, so a bulk quiver
//!   that checks the standard arrows stays silent there anyway).
//! - The trick quiver's selection cycling rides the tool's use (TrickQuiverModule.onToolUse): the in-air use
//!   context carries no stack at all, and the on-block one names neither the hand used nor the player's
//!   crouch state the cycling keys on, so the selection cannot be cycled from here; the stored selection is
//!   honored exactly as it stands.
use crate::{break_speed_hook::with_held_tool, common, tool_stack};
use pluto_lib::{CarriedEdit, Identifier, ProjectileEvents};

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ProjectileEvents::AMMO.register(|request| {
        // A mob's stacks are not reported, so its quiver cannot be shrunk; only players draw.
        if request.player.is_none() { return; }
        let Some(items) = common::item_snapshot() else { return; };
        let in_tag = |item: &Identifier, tag: &Identifier| items.get_tag(tag).is_some_and(|members| members.contains(item));
        let Ok(drawn) = with_held_tool(&request.weapon, |env, tool| {
            let mut data = tool.custom_data.clone();
            tool_stack::draw_quiver_ammo(&mut data, env, request.vanilla.as_ref(), &in_tag).map(|ammo| (ammo, data))
        }) else { return; };
        let Some((ammo, data)) = drawn else { return; };
        // The weapon the game named must be found among the carried stacks to write the shrunken quiver back.
        let Some(carried) = request.carried.iter().find(|carried| {
            carried.stack.item.to_string() == request.weapon.item && carried.stack.count == request.weapon.count
                && carried.stack.custom_data == request.weapon.custom_data
        }) else { return; };
        let mut edited = carried.stack.clone();
        edited.custom_data = data;
        request.edits.borrow_mut().push(CarriedEdit { slot: carried.slot, stack: edited });
        *request.supplied.borrow_mut() = Some(ammo);
    });
}
