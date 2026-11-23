//! The armor keybind interact (InteractionHandler.startArmorInteract, ToolClientEvents and
//! TinkerControlPacket in the original): pressing the helmet key (default `key.keyboard.z`) or the
//! leggings key (default `key.keyboard.i`) runs the worn piece's armor-interact modules with the tooltip
//! key held at the time — a shield strap rotates its inventory through the off hand, a tool belt swaps
//! its inventory with the main inventory, a minimap cycles its selected map, and an inventory menu
//! module opens the piece's screen. SlimeKnights (MIT; see LICENSE).
//!
//! What is bound here: the client half in full (the key event, the tooltip key from the held modifier
//! keys, the packet on the original's keys — the original's keys are rebindable key bindings, which this
//! host has no mod key bindings for, so the defaults are read directly) and the server logic in full
//! (`interact`: the merger's order, both swaps, the minimap's selection cycle, the blacklist rules, the
//! sound conditions), unit-tested below. The minimap's selection write-back rides the same walled
//! surface as the swaps (it changes the worn piece's own data, which no event offers a write for).
//!
//! What cannot run yet, with the evidence: executing a swap on the server needs to replace stacks in
//! places whose item changes — the off hand takes the strap inventory's first stack, hotbar slots take
//! belt stacks. The player handle writes a slot back only when it still holds the same item
//! (ServerPlayer::update_stack), whole-stack write-backs of carried places exist only on the ammo and
//! item-wear events (CarriedEdit), and no event fires either for a keybind packet. Reading the worn
//! pieces is not the problem (the player tick reports them, and the handle reads armor at slots 100 to
//! 103 and the off hand at -106); the exchange itself has no surface. Opening the inventory menu is
//! walled the same way: screens are keyed by block and open when a block is used, and no call opens one
//! for an item. So the server receiver drops the strap, belt and menu forms with a one-time log until
//! such a surface exists; `interact` is ready to be called by it unchanged.
//!
//! One form needs no exchange and does run: SelfDestructiveModifier primes its wearer from the
//! helmet key (shift held — the packet's tooltip key is Shift exactly then, which is the
//! original's isShiftKeyDown check) and takes the effect off on release; the effect's own last
//! tick explodes on the player tick below, and taking the helmet off removes it there too
//! (SelfDestructiveModifier.onUnequip). The primed sound plays on the effect's first tick,
//! where the tick reports the wearer's place.
use crate::{break_speed_hook::with_held_tool, tool_inventory::{MenuKey, ToolInventory}, tool_stack::{self, InteractPlan}};
use pluto_lib::{ActionTarget, ClientPlayNetworking, EntityAction, ExplosionInteraction, Identifier, ItemSlot, Nbt, NbtCompound, PlayerKeyEvents, ServerLivingEntityEvents, ServerPlayNetworking, StackData, World};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The channel the client hint travels on.
fn channel() -> Identifier { Identifier::parse("tconstruct:armor_interact/v1").expect("valid channel") }

/// Packet slots: the helmet key works the head piece, the leggings key the legs piece.
const SLOT_HEAD: u8 = 0;
const SLOT_LEGS: u8 = 1;

/// The packet: [slot, tooltip key, start]. The original's packet also carries no more than this (its
/// start/stop kinds name the slot the same way).
fn encode(slot: u8, key: MenuKey, start: bool) -> [u8; 3] {
    let key = match key { MenuKey::Normal => 0, MenuKey::Shift => 1, MenuKey::Control => 2, MenuKey::Alt => 3, MenuKey::Any => 0 };
    [slot, key, u8::from(start)]
}

fn decode(data: &[u8]) -> Option<(u8, MenuKey, bool)> {
    let [slot, key, start] = data else { return None; };
    if *slot > SLOT_LEGS { return None; }
    let key = match key { 0 => MenuKey::Normal, 1 => MenuKey::Shift, 2 => MenuKey::Control, 3 => MenuKey::Alt, _ => return None };
    Some((*slot, key, *start != 0))
}

/// TooltipKey.getTooltipKey: shift wins over control, control over alt.
fn tooltip_key(shift: bool, control: bool, alt: bool) -> MenuKey {
    if shift { MenuKey::Shift } else if control { MenuKey::Control } else if alt { MenuKey::Alt } else { MenuKey::Normal }
}

/// ShieldStrapModule.startInteract's rotation: the first slot's stack becomes the new off hand, the rest
/// shift back by one, the old off hand lands in the last slot. None when the off hand may not go into the
/// strap (blacklisted) — the module then does not answer and the merger moves on. The second return is the
/// equip sound's condition (either stack non-empty).
fn strap_rotate(inventory: &ToolInventory, persistent: &mut NbtCompound, offhand: Option<ItemSlot>, blacklisted: &dyn Fn(&ItemSlot) -> bool) -> Option<(Option<ItemSlot>, bool)> {
    if let Some(off) = &offhand { if blacklisted(off) { return None; } }
    let slots = inventory.slots();
    if slots == 0 { return None; }
    let first = inventory.stack(persistent, 0);
    for slot in 1..slots {
        let stack = inventory.stack(persistent, slot);
        inventory.set_stack(persistent, slot - 1, stack.as_ref());
    }
    inventory.set_stack(persistent, slots - 1, offhand.as_ref());
    let sound = first.is_some() || offhand.is_some();
    Some((first, sound))
}

/// ToolBeltModule.startInteract's exchange: slot for slot with the main inventory (at most its 36 slots),
/// skipping blacklisted stacks. Returns the equip sound's condition, keeping the original's quirk: the
/// flag is assigned per swapped slot, so the last swapped slot decides it.
fn belt_swap(inventory: &ToolInventory, persistent: &mut NbtCompound, main: &mut [Option<ItemSlot>], blacklisted: &dyn Fn(&ItemSlot) -> bool) -> bool {
    let slots = inventory.slots().min(main.len());
    let mut did_change = false;
    for slot in 0..slots {
        let original = main[slot].clone();
        if original.as_ref().is_none_or(|stack| !blacklisted(stack)) {
            let belt_item = inventory.stack(persistent, slot);
            main[slot] = belt_item.clone();
            inventory.set_stack(persistent, slot, original.as_ref());
            did_change = original.is_some() || belt_item.is_some();
        }
    }
    did_change
}

/// MinimapModule.startInteract's selection cycle (InventorySelectionModule.selectNext): the selection
/// moves to the next slot holding a stack, wrapping around the inventory, and the new selection is
/// written to the persistent data. None when no other slot holds a stack — the selection then stays
/// where it was, and the module does not answer.
fn select_next(inventory: &ToolInventory, persistent: &mut NbtCompound) -> Option<usize> {
    let slots = inventory.slots();
    if slots == 0 { return None; }
    let current = persistent.get("tconstruct:minimap_selected").and_then(Nbt::as_i64).unwrap_or(0).max(0) as usize % slots;
    for step in 1..slots {
        let next = (current + step) % slots;
        if inventory.stack(persistent, next).is_some() {
            persistent.insert("tconstruct:minimap_selected".to_owned(), Nbt::Int(next as i32));
            return Some(next);
        }
    }
    None
}

/// What an armor interact did, in the original's terms.
pub(crate) enum Outcome {
    /// No module answered the key (or the piece is not a built Tinkers armor tool).
    Nothing,
    /// A shield strap rotated: the piece with its new inventory, the new off hand, and the sound condition.
    Strap { piece: StackData, offhand: Option<ItemSlot>, sound: bool },
    /// A tool belt swapped: the piece, the main inventory's changed slots, and the sound condition.
    Belt { piece: StackData, main: Vec<(usize, Option<ItemSlot>)>, sound: bool },
    /// A minimap cycled its selection: the piece with the new selection in its data, and the slot now
    /// selected (MinimapModule.onInventorySelect shows the selection message for it).
    Select { piece: StackData, selected: usize },
    /// An inventory menu module answered the key: the piece's inventory screen is what would open.
    Menu,
}

/// ArmorInteractModifierHook over a worn piece, as InteractionHandler.startArmorInteract runs it: the
/// entries' modules in merger order (see tool_stack::armor_interact_plan), the first that answers the key
/// wins. `main` is the wearer's main inventory (36 slots); `blacklisted` is
/// ToolInventoryCapability.isBlacklisted. The spectator skip and the modifiable-armor tag check of the
/// original belong to the caller, which owns the player.
pub(crate) fn interact(piece: &StackData, key: MenuKey, offhand: Option<ItemSlot>, main: &[Option<ItemSlot>], blacklisted: &dyn Fn(&ItemSlot) -> bool) -> Outcome {
    let Ok(outcome) = with_held_tool(piece, |env, tool| {
        let plan = tool_stack::armor_interact_plan(&tool.custom_data, env);
        let mut data = tool.custom_data.clone();
        let mut main = main.to_vec();
        for step in plan {
            match step {
                InteractPlan::Strap { keys, inventory } if keys.contains(&key) => {
                    let rotated = tool_stack::with_persistent(&mut data, |persistent| strap_rotate(&inventory, persistent, offhand.clone(), blacklisted));
                    if let Some((new_offhand, sound)) = rotated {
                        let mut piece = piece.clone();
                        piece.custom_data = data;
                        return Outcome::Strap { piece, offhand: new_offhand, sound };
                    }
                }
                InteractPlan::Belt { keys, inventory } if keys.contains(&key) => {
                    let before = main.clone();
                    let sound = tool_stack::with_persistent(&mut data, |persistent| belt_swap(&inventory, persistent, &mut main, blacklisted));
                    let changed: Vec<(usize, Option<ItemSlot>)> = main.iter().enumerate()
                        .filter(|(slot, now)| before.get(*slot) != Some(now))
                        .map(|(slot, now)| (slot, now.clone())).collect();
                    let mut piece = piece.clone();
                    piece.custom_data = data;
                    return Outcome::Belt { piece, main: changed, sound };
                }
                InteractPlan::MinimapSelect { inventory } if matches!(key, MenuKey::Normal | MenuKey::Control) => {
                    // MinimapModule.startInteract answers the normal and control tooltip keys.
                    let selected = tool_stack::with_persistent(&mut data, |persistent| select_next(&inventory, persistent));
                    if let Some(selected) = selected {
                        let mut piece = piece.clone();
                        piece.custom_data = data;
                        return Outcome::Select { piece, selected };
                    }
                }
                InteractPlan::Menu { key: menu_key } if menu_key.opens_on_keybind(key) => return Outcome::Menu,
                _ => {}
            }
        }
        Outcome::Nothing
    }) else { return Outcome::Nothing; };
    outcome
}

/// The wearer's self-destructive level from the head piece the player tick last reported
/// (SelfDestructiveModifier's tool), 0 when the helmet carries no such modifier.
fn self_destructive_level(player: u128) -> i32 {
    let Some((_, head)) = pluto_lib::entity_events_v1::equipment_of(player).into_iter().find(|(slot, _)| slot == "head") else { return 0; };
    with_held_tool(&head, |env, tool| tool_stack::modifier_level(&tool.custom_data, env, "tconstruct:self_destructive")).unwrap_or(0)
}

/// SelfDestructiveModifier.startInteract: shift with the helmet key primes the wearer — the
/// self-destructing effect for 30 ticks at twice the modifier's level.
fn self_destructive_start(player: u128) {
    let level = self_destructive_level(player);
    if level <= 0 { return; }
    let wearer = pluto_lib::player_api::ServerPlayer { uuid: player, name: String::new() };
    wearer.add_effect(&pluto_lib::player_api::EffectInstance::new("tconstruct:self_destructing", 30).amplifier((2 * level).clamp(0, 255) as u8));
}

/// SelfDestructiveModifier.stopInteract: the key's release takes the effect off.
fn self_destructive_stop(player: u128) {
    if self_destructive_level(player) <= 0 { return; }
    let wearer = pluto_lib::player_api::ServerPlayer { uuid: player, name: String::new() };
    wearer.remove_effect("tconstruct:self_destructing");
}

/// The players whose primed sound has played for the effect they currently carry, by uuid.
static PRIMED: Mutex<BTreeSet<u128>> = Mutex::new(BTreeSet::new());

/// Registers the client key tracking, the packet send and the server receiver. Called once from the
/// product's hooks.
pub(crate) fn register() {
    static KEYS: Mutex<(bool, bool, bool)> = Mutex::new((false, false, false));
    PlayerKeyEvents::EVENT.register(|press| {
        let Ok(mut keys) = KEYS.lock() else { return; };
        match press.key.as_str() {
            "key.keyboard.left.shift" | "key.keyboard.right.shift" => keys.0 = press.pressed,
            "key.keyboard.left.control" | "key.keyboard.right.control" => keys.1 = press.pressed,
            "key.keyboard.left.alt" | "key.keyboard.right.alt" => keys.2 = press.pressed,
            "key.keyboard.z" => {
                let _ = ClientPlayNetworking::send(&channel(), &encode(SLOT_HEAD, tooltip_key(keys.0, keys.1, keys.2), press.pressed));
            }
            "key.keyboard.i" => {
                let _ = ClientPlayNetworking::send(&channel(), &encode(SLOT_LEGS, tooltip_key(keys.0, keys.1, keys.2), press.pressed));
            }
            _ => {}
        }
    });
    if let Err(error) = ServerPlayNetworking::register_global_receiver(channel(), |player, data| {
        let Some((slot, key, start)) = decode(data) else { return; };
        if !start {
            // SelfDestructiveModifier.stopInteract is the one stop half with work to do.
            if slot == SLOT_HEAD { self_destructive_stop(player); }
            return;
        }
        // SelfDestructiveModifier.startInteract runs on the packet (see the module notes): the
        // tooltip key is Shift exactly when shift was held.
        if slot == SLOT_HEAD && key == MenuKey::Shift && self_destructive_level(player) > 0 {
            self_destructive_start(player);
            return;
        }
        // The exchange the packet asks for has no server surface yet (see the module notes).
        static LOGGED: AtomicBool = AtomicBool::new(false);
        if !LOGGED.swap(true, Ordering::Relaxed) {
            eprintln!("Tinkers: an armor interact key was pressed, but no server surface can exchange a worn piece's stacks yet; the interact was dropped");
        }
    }) {
        eprintln!("Tinkers: armor interact receiver not registered: {error}");
    }
    // SelfDestructiveEffect.applyEffectTick and SelfDestructiveModifier.onUnequip, over the
    // player tick: the primed sound on the effect's first tick, the explosion on its last,
    // and the removal when the helmet that primes it is no longer worn.
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        let Some((_, amplifier, left)) = tick.effects.iter().find(|(id, _, _)| id == "tconstruct:self_destructing") else {
            let _ = PRIMED.lock().map(|mut primed| primed.remove(&tick.player));
            return;
        };
        let world = World::new(&tick.dimension);
        if PRIMED.lock().map(|mut primed| primed.insert(tick.player)).unwrap_or(false) {
            world.play_sound("minecraft:entity.creeper.primed", tick.position, 1.0, 0.5);
        }
        let helmet_keeps_it = tick.armor.iter().find(|(slot, _)| slot == "head")
            .is_some_and(|(_, head)| with_held_tool(head, |env, tool| tool_stack::modifier_level(&tool.custom_data, env, "tconstruct:self_destructive") > 0).unwrap_or(false));
        if !helmet_keeps_it {
            tick.remove_effects.borrow_mut().push("tconstruct:self_destructing".to_owned());
            return;
        }
        if *left == 1 {
            // The effect's level is the explosion's radius; the blast is a mob-interaction
            // explosion (the port's block interaction), and the wearer takes the self-destruct hit.
            world.explode(tick.position, (*amplifier + 1) as f32, false, ExplosionInteraction::Block);
            tick.actions.borrow_mut().push(EntityAction::Hurt { target: ActionTarget::Attacker, amount: 99999.0, damage_type: "tconstruct:self_destruct".to_owned() });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{modifier_entries::Entry, tool_inventory::{Inventory, Part}};
    use pluto_lib::Identifier;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn stack(item: &str, count: u32) -> ItemSlot { ItemSlot::new(id(item), count) }

    #[test]
    fn the_packet_round_trips_and_rejects_bad_bytes() {
        assert_eq!(decode(&encode(SLOT_LEGS, MenuKey::Control, true)), Some((SLOT_LEGS, MenuKey::Control, true)));
        assert_eq!(decode(&encode(SLOT_HEAD, MenuKey::Normal, false)), Some((SLOT_HEAD, MenuKey::Normal, false)));
        assert_eq!(decode(&[2, 0, 1]), None, "no third slot");
        assert_eq!(decode(&[0, 9, 1]), None, "no ninth key");
        assert_eq!(decode(&[0, 0]), None, "short packet");
    }

    #[test]
    fn the_tooltip_key_prefers_shift_then_control_then_alt() {
        assert_eq!(tooltip_key(true, true, true), MenuKey::Shift);
        assert_eq!(tooltip_key(false, true, true), MenuKey::Control);
        assert_eq!(tooltip_key(false, false, true), MenuKey::Alt);
        assert_eq!(tooltip_key(false, false, false), MenuKey::Normal);
    }

    fn strap_inventory() -> (ToolInventory, NbtCompound) {
        let module = Inventory::decode(json!({"slots": {"flat": 3}}).as_object().unwrap()).unwrap().unwrap();
        let part = Part::new(module, Entry::new(id("tconstruct:shield_strap"), 1), true, true);
        let inventory = ToolInventory::new(vec![part]);
        let mut persistent = NbtCompound::new();
        inventory.set_stack(&mut persistent, 0, Some(&stack("minecraft:shield", 1)));
        inventory.set_stack(&mut persistent, 2, Some(&stack("minecraft:apple", 2)));
        (inventory, persistent)
    }

    #[test]
    fn a_strap_rotation_moves_the_first_slot_to_the_hand_and_the_hand_to_the_last_slot() {
        let (inventory, mut persistent) = strap_inventory();
        let (offhand, sound) = strap_rotate(&inventory, &mut persistent, Some(stack("minecraft:torch", 5)), &|_| false).expect("rotation");
        assert_eq!(offhand, Some(stack("minecraft:shield", 1)), "the first slot is the new off hand");
        assert!(sound, "both stacks were non-empty");
        assert_eq!(inventory.stack(&persistent, 0), None, "the second (empty) slot shifted back");
        assert_eq!(inventory.stack(&persistent, 1), Some(stack("minecraft:apple", 2)), "the third slot shifted back");
        assert_eq!(inventory.stack(&persistent, 2), Some(stack("minecraft:torch", 5)), "the old off hand is last");
    }

    #[test]
    fn a_blacklisted_off_hand_declines_the_strap() {
        let (inventory, mut persistent) = strap_inventory();
        let blacklisted = |stack: &ItemSlot| stack.item == id("minecraft:shulker_box");
        assert!(strap_rotate(&inventory, &mut persistent, Some(stack("minecraft:shulker_box", 1)), &blacklisted).is_none());
        assert_eq!(inventory.stack(&persistent, 0), Some(stack("minecraft:shield", 1)), "nothing moved");
    }

    #[test]
    fn a_belt_swap_exchanges_slots_and_skips_blacklisted_stacks() {
        let module = Inventory::decode(json!({"slots": {"flat": 3}}).as_object().unwrap()).unwrap().unwrap();
        let part = Part::new(module, Entry::new(id("tconstruct:tool_belt"), 1), true, true);
        let inventory = ToolInventory::new(vec![part]);
        let mut persistent = NbtCompound::new();
        inventory.set_stack(&mut persistent, 0, Some(&stack("tconstruct:pickaxe", 1)));
        inventory.set_stack(&mut persistent, 2, Some(&stack("tconstruct:sword", 1)));
        let mut main: Vec<Option<ItemSlot>> = vec![Some(stack("minecraft:dirt", 4)), Some(stack("minecraft:shulker_box", 1)), None];
        let blacklisted = |stack: &ItemSlot| stack.item == id("minecraft:shulker_box");
        assert!(belt_swap(&inventory, &mut persistent, &mut main, &blacklisted), "slots changed");
        assert_eq!(main[0], Some(stack("tconstruct:pickaxe", 1)), "the belt's tool is in the hotbar");
        assert_eq!(inventory.stack(&persistent, 0), Some(stack("minecraft:dirt", 4)), "the dirt went into the belt");
        assert_eq!(main[1], Some(stack("minecraft:shulker_box", 1)), "a blacklisted stack stays");
        assert_eq!(inventory.stack(&persistent, 1), None, "and its belt slot is untouched");
        assert_eq!(main[2], Some(stack("tconstruct:sword", 1)), "an empty slot takes the belt's stack");
        assert_eq!(inventory.stack(&persistent, 2), None);
    }

    #[test]
    fn the_minimap_selection_cycles_to_the_next_stored_map_and_wraps() {
        let module = Inventory::decode(json!({"slots": {"flat": 3}}).as_object().unwrap()).unwrap().unwrap();
        let part = Part::new(module, Entry::new(id("tconstruct:minimap"), 1), true, true);
        let inventory = ToolInventory::new(vec![part]);
        let mut persistent = NbtCompound::new();
        inventory.set_stack(&mut persistent, 0, Some(&stack("minecraft:filled_map", 1)));
        inventory.set_stack(&mut persistent, 2, Some(&stack("minecraft:filled_map", 1)));
        assert_eq!(select_next(&inventory, &mut persistent), Some(2), "from the first map to the next stored one");
        assert_eq!(persistent.get("tconstruct:minimap_selected").and_then(Nbt::as_i64), Some(2), "the selection is written");
        assert_eq!(select_next(&inventory, &mut persistent), Some(0), "and around again");
        // A single stored map leaves the selection where it is: the module does not answer.
        let mut single = NbtCompound::new();
        inventory.set_stack(&mut single, 1, Some(&stack("minecraft:filled_map", 1)));
        single.insert("tconstruct:minimap_selected".to_owned(), Nbt::Int(1));
        assert_eq!(select_next(&inventory, &mut single), None);
        // An empty inventory answers nothing either.
        assert_eq!(select_next(&inventory, &mut NbtCompound::new()), None);
    }
}
