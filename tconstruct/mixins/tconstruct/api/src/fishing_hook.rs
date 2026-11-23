//! Connects the fishing module to PlutoLib's fishing reach. The Tinkers rod sits on the vanilla rod's
//! template, but the game's rod use is gated to the vanilla item, so the rod never even vanilla-casts:
//! a use of the rod is answered here instead (`FishingModule.onToolUse`). The profile is the module's
//! own reading of the tool — the cast action from the fishing module, the conditional sea luck, lure,
//! velocity, accuracy and projectile damage stats, the grapple and drill actions for the grapple kind,
//! the item hook action for collecting — and PlutoLib's fishing tick casts or retrieves with it.
//!
//! The rod's wear comes back the other way: the game side reports each cast's, hit's and retrieve's
//! cost, and the next use of the rod pays it into the hand's stack through the tool damage system
//! (`ToolDamageUtil.damageAnimated`), so the wear lands at most one use late. The retrieve's 1-tick
//! cooldown is not applied (a use one tick early retrieves a hair sooner; nothing else reads it).
//! The port has no knockback stat, so the profile's knockback is the module-computed melee knockback
//! at no base strength, which is what a rod without knockback modifiers carries anyway.
use crate::{break_speed_hook::with_held_tool, entity_facts, tool_damage_hook, tool_slot, tool_stack, tool_stats::Stat};
use pluto_lib::{ActionResult, Fishing, FishingProfile, FishingRegistry, Grapple, Identifier, ItemSlot, StackData, UseItemCallback, UseItemContext};

/// The rod item: the one tool the fishing module's cast belongs to.
const ROD: &str = "tconstruct:fishing_rod";

/// The cast profile of a rod stack (`FishingModule.onToolUse`'s reading of the tool): None when the
/// stack is not a whole tool carrying the fishing module (the module grants the cast action).
fn profile(stack: &StackData) -> Option<FishingProfile> {
    with_held_tool(stack, |env, tool| {
        let nbt = &tool.custom_data;
        if !tool_stack::can_perform_action(nbt, env, "fishing_rod_cast") {
            return None;
        }
        // The caster's state tags and the entities a conditional stat could look at are not known to
        // the use event, so a conditional stat that needs them does not apply (the edible reading
        // makes the same choice).
        let no_state = |_: &Identifier| false;
        let no_entities = entity_facts::Entities { attacker: None, target: None };
        let stat = |stat: Stat| tool_stack::conditional_stat(nbt, env, stat, &no_state, &no_entities);
        let accuracy = stat(Stat::Accuracy);
        let grapple = if tool_stack::can_perform_action(nbt, env, "grapple_hook") {
            if tool_stack::can_perform_action(nbt, env, "drill_attack") {
                Grapple::Drill
            } else {
                Grapple::Dash
            }
        } else {
            Grapple::None
        };
        Some(FishingProfile {
            luck: stat(Stat::SeaLuck) as i32,
            lure: stat(Stat::Lure) as i32,
            velocity: stat(Stat::Velocity),
            // ModifierUtil.getInaccuracy: 3 * (1 / accuracy - 1); a stack with no accuracy reading
            // casts true.
            inaccuracy: if accuracy > 0.0 { 3.0 * (1.0 / accuracy - 1.0) } else { 0.0 },
            power: stat(Stat::ProjectileDamage),
            knockback: tool_stack::melee_knockback(nbt, env, 0.0, &no_state),
            grapple,
            collecting: tool_stack::can_perform_action(nbt, env, "item_hook"),
        })
    })
    .ok()
    .flatten()
}

/// Whether two readings of a stack are the same stack (item, count, damage and data).
fn same_stack(left: &StackData, right: &StackData) -> bool {
    left.item == right.item && left.count == right.count && left.damage == right.damage && left.custom_data == right.custom_data
}

/// Pays the wear the game side has reported for the rod into the hand that used it: the selected slot
/// when its stack is the used one, else the offhand (carried slot -106). Wear for a rod no longer in
/// either hand is dropped — the rod that earned it is gone from the hands.
fn pay_wear(context: &UseItemContext) {
    let Some(player) = &context.player else { return };
    let wear = Fishing::take_wear(player);
    if wear == 0 {
        return;
    }
    let selected = player.selected_slot().unwrap_or(0);
    let hand = [selected, -106]
        .into_iter()
        .find(|slot| player.stack(*slot).as_ref().is_some_and(|carried| same_stack(carried, &context.stack)));
    let Some(hand) = hand else { return };
    let Some(carried) = player.stack(hand) else { return };
    let Ok(item) = Identifier::parse(&carried.item) else { return };
    let mut slot = ItemSlot::new(item, carried.count);
    slot.damage = carried.damage;
    slot.custom_data = carried.custom_data.clone();
    let damaged = with_held_tool(&carried, |env, _tool| {
        tool_slot::damage_slot(&mut slot, env, wear as i32, true, true, None, &mut tool_damage_hook::random)
    });
    if damaged.is_ok() {
        let back = StackData { item: slot.item.to_string(), count: slot.count, damage: slot.damage, custom_data: slot.custom_data.clone(), ..carried };
        player.update_stack(hand, &back);
    }
}

/// Registers the rod's cast profile and the use callback. Called once from the product's hooks.
pub(crate) fn register() {
    if let Ok(rod) = Identifier::new("tconstruct", "fishing_rod") {
        FishingRegistry::register(rod, profile);
    }
    UseItemCallback::EVENT.register(|context| {
        if context.item != ROD {
            return ActionResult::Pass;
        }
        let Some(player) = &context.player else { return ActionResult::Pass };
        pay_wear(context);
        let Some(profile) = FishingRegistry::profile(&context.stack) else { return ActionResult::Pass };
        Fishing::use_rod(player, profile);
        ActionResult::Success
    });
}
