//! Connects the launcher modules to PlutoLib's projectile events: a shot with a tool whose modifiers
//! hurt the shooter for firing (DamageOnShootModule, on the shot event, which reports the launcher in
//! the shooter's hands) and a launcher that supplies its own ammo (InfinityModule, on the ammo event,
//! which the game asks before it fires). The shot event does not say which hand fired, so the main hand
//! is the launcher when it carries the module, else the off hand; a mob that shoots cannot be hurt back
//! or supplied, as the launch event takes no actions and a mob's request carries no stacks.
use crate::{break_speed_hook::with_held_tool, tool_slot, tool_stack};
use pluto_lib::{ActionTarget, AmmoRequest, BowShot, CarriedEdit, CarriedSlot, EntityAction, ItemSlot, ProjectileEvents, StackData, StackSlot};

/// DamageOnShootModule.onProjectileLaunch for the firing hand: the holder is hurt by each of the
/// launcher's modules, as the shot's actions land on the shooter.
fn damage_on_shoot(shot: &BowShot) {
    for hand in [&shot.main_hand, &shot.off_hand] {
        let Some(stack) = hand else { continue };
        let Ok(hurts) = with_held_tool(stack, |env, tool| tool_stack::damage_on_shoot(&tool.custom_data, env)) else { continue };
        if hurts.is_empty() { continue; }
        shot.actions.borrow_mut().extend(hurts.into_iter().map(|(damage_type, amount)| EntityAction::Hurt { target: ActionTarget::Attacker, damage_type, amount }));
        return;
    }
}

/// Whether a carried stack is the weapon the game reported (the same item, count, damage and data).
fn same_stack(stack: &ItemSlot, weapon: &StackData) -> bool {
    stack.item.to_string() == weapon.item && stack.count == weapon.count && stack.damage == weapon.damage && stack.custom_data == weapon.custom_data
}

/// InfinityModule as the ammo event asks: the supplied stack replaces the game's pick and nothing is
/// shrunk, so the shot's cost is the callback's own edit — the weapon among the carried stacks, worn by
/// the usage (shrinkAmmo). The request names no creative flag, so a creative shooter's tool wears as a
/// survival one does; with standard arrows checked, a creative pick already counts as standard ammo.
fn infinity(request: &AmmoRequest) {
    let Ok(supply) = with_held_tool(&request.weapon, |env, tool| tool_stack::infinity(&tool.custom_data, env, request.vanilla.is_some())) else { return };
    let Some((stack, usage, cause)) = supply else { return };
    request.supplied.borrow_mut().replace(stack);
    if usage <= 0 { return; }
    let Some(carried) = request.carried.iter().find(|carried| matches!(carried.slot, CarriedSlot::Equipment(StackSlot::MainHand | StackSlot::OffHand)) && same_stack(&carried.stack, &request.weapon)) else { return };
    let mut worn = carried.stack.clone();
    let damaged = with_held_tool(&request.weapon, |env, _tool| tool_slot::damage_slot(&mut worn, env, usage, true, true, Some(&cause), &mut crate::tool_damage_hook::random));
    if damaged.is_ok() { request.edits.borrow_mut().push(CarriedEdit { slot: carried.slot, stack: worn }); }
}

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    ProjectileEvents::SHOOT_BOW.register(damage_on_shoot);
    ProjectileEvents::AMMO.register(infinity);
}
