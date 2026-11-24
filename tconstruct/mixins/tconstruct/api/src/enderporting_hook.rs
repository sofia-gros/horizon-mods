//! EnderportingModifier (SlimeKnights, MIT; see LICENSE): the holder trades places with what the
//! tool touches. A harvested block pulls the holder onto it and a melee hit swaps the attacker and
//! the mob it landed on; the projectile half rides the flight profile (flight_hook's impact pass).
//! Every move the callbacks can test goes through the original's landing test as the ender slime's
//! teleport states it (slime_entities::teleport_lands: the blocks the body would occupy are free of
//! solids and liquids); the melee swap needs no test, as its destinations are the two parties' own
//! standing places. A landed harvest or swap wears the tool by 2, as the original calls
//! ToolDamageUtil.damageTool with 2 there.
//!
//! The harvest half is finishHarvest and the plant harvest in one: an area tool's extra blocks
//! ride the break without events of their own, so the centre block's after-break is the one that
//! is heard, which is where both of the original's hooks put the holder. Its `canHarvest`
//! condition is the break itself — the after-break callback only runs for a block the tool did
//! harvest. The launch wear (the shoot telling does not tell a multishot's primary arrow from its
//! extras, and the original charges only the primary) is wired in flight_hook's shot pass, which
//! counts the volley itself; the fuse teleport and the fishing rod's wear ride the flight's fuse
//! and impact passes, queuing here for the owner's next player tick when the reach cannot move
//! them at once (a player: the host has no call that moves a player); the monster-held swap rides
//! the damage event in monster_melee. The plant harvest teleports through the same queue, keyed by
//! the player's uuid (the use telling carries no entity id).
//!
//! Not wired: the draw-speed cooldown the fishing rod's wear carries (updateFishingRod's
//! applyCooldown: the cooldown surface is the sleeves item cooldown, not yet merged — the wear
//! lands, the cooldown waits); a thrown tool's enderporting hit (the flight's
//! thrown-tool branch answers the hit itself before the enderporting block runs). The monster-held
//! swap's wear by 2 rides the mob-stack-write surface (an `EntityAction::EditStackData` on the
//! attacker's main hand, queued with the swap).
use crate::{break_speed_hook::with_held_tool, common, slime_entities, tool_stack};
use pluto_lib::{ActionTarget, DamageModification, EntityAction, Identifier, PlayerBlockBreakEvents, PlayerTick, ServerLivingEntityEvents, StackSlot, UseItemOnBlockContext, World};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a queued teleport or rod wear waits for its owner's next player tick: the tick that
/// follows is the one that lands it, and a longer gap means the owner is gone.
const QUEUE_LIFETIME: Duration = Duration::from_secs(5);

/// Teleports queued for their owner's next player tick, by the owner's entity id: (dimension,
/// position, queued at). The fuse teleport's owner is named by entity id (the flight telling
/// carries no uuid); a player owner whose direct teleport the reach refused is the one that waits
/// here.
static PENDING_FUSE_TELEPORTS: Mutex<HashMap<u64, (String, [f64; 3], Instant)>> = Mutex::new(HashMap::new());
/// Teleports queued for their player's next tick, by the player's uuid: (dimension, position,
/// queued at). The plant harvest's owner is named by uuid (the use telling carries no entity id).
static PENDING_PLANT_TELEPORTS: Mutex<HashMap<u128, (String, [f64; 3], Instant)>> = Mutex::new(HashMap::new());
/// Rod wear queued for its owner's next player tick, by the owner's entity id: (amount, queued at).
static PENDING_ROD_WEAR: Mutex<HashMap<u64, (u32, Instant)>> = Mutex::new(HashMap::new());

/// The wear actions of a landed enderport: the tool in the main hand takes `amount` damage.
fn wear(held: &pluto_lib::StackData, amount: i32) -> Option<EntityAction> {
    let edits = with_held_tool(held, |_env, stack| tool_stack::damage_edits(&stack.custom_data, amount)).ok()?;
    (!edits.is_empty()).then(|| EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: held.item.clone(), edits })
}

/// Whether the tool in hand carries enderporting.
fn carries(held: &pluto_lib::StackData) -> bool {
    matches!(with_held_tool(held, |env, stack| tool_stack::enderporting(&stack.custom_data, env)), Ok(true))
}

/// Queues the owner's teleport to where their projectile's fuse ended, for their next player tick
/// (the flight pass calls this when the reach could not move them at once).
pub(crate) fn queue_fuse_teleport(owner: u64, dimension: &str, position: [f64; 3]) {
    if let Ok(mut queue) = PENDING_FUSE_TELEPORTS.lock() {
        queue.insert(owner, (dimension.to_owned(), position, Instant::now()));
    }
}

/// Queues wear of `amount` on the owner's fishing rod, for their next player tick (the flight pass
/// calls this when a bobber's teleport landed).
pub(crate) fn queue_rod_wear(owner: u64, amount: u32) {
    if amount == 0 { return; }
    if let Ok(mut queue) = PENDING_ROD_WEAR.lock() {
        let held = queue.get(&owner).map(|(held, _)| *held).unwrap_or(0);
        queue.insert(owner, (held + amount, Instant::now()));
    }
}

/// Takes the teleport queued for `key` in `queue`, when the ticking player is in the dimension it
/// names (a player in another dimension keeps the teleport for their return, until it expires).
fn take_queued_teleport<K: Eq + std::hash::Hash>(queue: &Mutex<HashMap<K, (String, [f64; 3], Instant)>>, key: &K, dimension: &str) -> Option<[f64; 3]> {
    let mut queue = queue.lock().ok()?;
    queue.retain(|_, (_, _, at)| at.elapsed() < QUEUE_LIFETIME);
    let (queued_dimension, dest, _) = queue.get(key)?.clone();
    if queued_dimension != dimension { return None; }
    queue.remove(key).map(|(_, dest, _)| dest)
}

/// EnderportingModifier.afterHarvest for a plant harvest: when `harvested` blocks were harvested
/// with a tool carrying enderporting, queue the holder's teleport onto the clicked block's centre
/// for their next player tick (the use telling carries no entity id, so the queue is keyed by the
/// player's uuid). The wear by 2 is the caller's (the harvest hook's own wear, which finds the
/// tool's slot). Returns whether the teleport was queued.
pub(crate) fn plant_harvest(used: &UseItemOnBlockContext, held: &pluto_lib::StackData, harvested: i32) -> bool {
    if harvested <= 0 || !carries(held) { return false; }
    let dest = [f64::from(used.pos[0]) + 0.5, f64::from(used.pos[1]), f64::from(used.pos[2]) + 0.5];
    if !slime_entities::teleport_lands(&World::new(&used.dimension), dest) { return false; }
    if let Ok(mut queue) = PENDING_PLANT_TELEPORTS.lock() {
        queue.insert(used.player.uuid, (used.dimension.clone(), dest, Instant::now()));
        true
    } else {
        false
    }
}

/// EnderportingModifier's monster-held form: a monster's melee hit with an enderporting weapon
/// trades the attacker and the victim (the original's MonsterMeleeHit hook). The hit's positions
/// are the parties' own standing places, so no landing test is needed (the player form's
/// precedent). The wear by 2 rides the mob-stack-write surface with the swap, as an
/// `EntityAction::EditStackData` on the attacker's main hand. Returns whether the swap was queued.
pub(crate) fn monster_swap(hit: &DamageModification) -> bool {
    let Some(held) = hit.attacker_main_hand.as_ref() else { return false; };
    if !carries(held) { return false; }
    let (Some(attacker), Some(target)) = (hit.attacker_position, hit.target_position) else { return false; };
    hit.actions.borrow_mut().push(EntityAction::Teleport { target: ActionTarget::Attacker, position: target });
    hit.actions.borrow_mut().push(EntityAction::Teleport { target: ActionTarget::Victim, position: attacker });
    if let Some(action) = wear(held, 2) { hit.actions.borrow_mut().push(action); }
    true
}

/// The queued teleports and rod wear of the ticking player, applied on their tick.
fn drain_tick(tick: &PlayerTick) {
    if let Some(dest) = u64::try_from(tick.entity).ok().and_then(|owner| take_queued_teleport(&PENDING_FUSE_TELEPORTS, &owner, &tick.dimension)) {
        if slime_entities::teleport_lands(&World::new(&tick.dimension), dest) {
            tick.actions.borrow_mut().push(EntityAction::Teleport { target: ActionTarget::Attacker, position: dest });
        }
    }
    if let Some(dest) = take_queued_teleport(&PENDING_PLANT_TELEPORTS, &tick.player, &tick.dimension) {
        if slime_entities::teleport_lands(&World::new(&tick.dimension), dest) {
            tick.actions.borrow_mut().push(EntityAction::Teleport { target: ActionTarget::Attacker, position: dest });
        }
    }
    let amount = PENDING_ROD_WEAR.lock().ok().and_then(|mut queue| {
        queue.retain(|_, (_, at)| at.elapsed() < QUEUE_LIFETIME);
        let key = u64::try_from(tick.entity).ok()?;
        queue.remove(&key).map(|(amount, _)| amount)
    });
    let Some(amount) = amount else { return; };
    // The rod in the hands (main then offhand) that can cast a fishing rod and carries
    // enderporting: the original's updateFishingRod finds the rod the same way.
    for (name, stack) in &tick.hands {
        let slot = match name.as_str() { "mainhand" => StackSlot::MainHand, "offhand" => StackSlot::OffHand, _ => continue };
        let Ok(edits) = with_held_tool(stack, |env, tool| {
            if !tool_stack::can_perform_action(&tool.custom_data, env, "fishing_rod_cast") || !tool_stack::enderporting(&tool.custom_data, env) { return Vec::new(); }
            tool_stack::damage_edits(&tool.custom_data, i32::try_from(amount).unwrap_or(i32::MAX))
        }) else { continue; };
        if edits.is_empty() { continue; }
        tick.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot, item: stack.item.clone(), edits });
        return;
    }
}

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::PLAYER_TICK.register(drain_tick);
    // finishHarvest: a block harvested with a harvest tool that carries enderporting pulls the
    // holder onto the block's centre, and wears the tool by 2.
    PlayerBlockBreakEvents::AFTER.register(|broken| {
        let Some(held) = broken.held.as_ref() else { return; };
        let Some(items) = common::item_snapshot() else { return; };
        let (Ok(item), Ok(harvest)) = (Identifier::parse(&held.item), Identifier::new("tconstruct", "modifiable/harvest")) else { return; };
        if !items.get_tag(&harvest).is_some_and(|members| members.contains(&item)) { return; }
        if !carries(held) { return; }
        let dest = [f64::from(broken.pos[0]) + 0.5, f64::from(broken.pos[1]), f64::from(broken.pos[2]) + 0.5];
        let Some(world) = World::ticking() else { return; };
        if !slime_entities::teleport_lands(&world, dest) { return; }
        let mut actions = vec![EntityAction::Teleport { target: ActionTarget::Attacker, position: dest }];
        actions.extend(wear(held, 2));
        broken.actions.borrow_mut().extend(actions);
    });
    // afterMeleeHit: the attacker and the mob it hit trade places, and the tool wears by 2. An
    // extra attack is not told apart by the weapon-hit telling, so every direct hit swaps.
    ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
        let Some(held) = hit.weapon.as_ref() else { return; };
        if !carries(held) { return; }
        let (Some(attacker), Some(target)) = (hit.attacker_position, hit.target_position) else { return; };
        let mut actions = vec![
            EntityAction::Teleport { target: ActionTarget::Attacker, position: target },
            EntityAction::Teleport { target: ActionTarget::Victim, position: attacker },
        ];
        actions.extend(wear(held, 2));
        hit.actions.borrow_mut().extend(actions);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_teleports_wait_for_the_owners_tick_in_the_same_dimension() {
        let owner = 77u64;
        queue_fuse_teleport(owner, "minecraft:overworld", [1.0, 64.0, 2.0]);
        // Another dimension's tick does not take it.
        assert_eq!(take_queued_teleport(&PENDING_FUSE_TELEPORTS, &owner, "minecraft:the_nether"), None);
        assert_eq!(take_queued_teleport(&PENDING_FUSE_TELEPORTS, &owner, "minecraft:overworld"), Some([1.0, 64.0, 2.0]));
        assert_eq!(take_queued_teleport(&PENDING_FUSE_TELEPORTS, &owner, "minecraft:overworld"), None, "taken once");
    }

    #[test]
    fn queued_rod_wear_accumulates_for_one_owner() {
        let owner = 78u64;
        queue_rod_wear(owner, 10);
        queue_rod_wear(owner, 10);
        let amount = PENDING_ROD_WEAR.lock().unwrap().remove(&owner).map(|(amount, _)| amount);
        assert_eq!(amount, Some(20));
    }
}
