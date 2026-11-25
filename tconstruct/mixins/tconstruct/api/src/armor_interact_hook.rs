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
//! sound conditions), unit-tested below. The server receiver runs the outcomes through the carried-stack
//! exchange surface: the worn piece and the off hand take whole-stack write-backs (`ServerPlayer::set_stack`
//! over the carried numbering), the belt's changed main slots the same, and the inventory menu opens
//! through the item-menu surface (`tool_container_menu`), which writes back what the player leaves in it.
//! The minimap's selection write-back rides the same surface (it changes the worn piece's own data, which
//! no event offers a write for).
//!
//! The zoom keybind is wired too (ZoomModule.startInteract/stopInteract, the spyglass style only —
//! the scope binds no armor interact hook): the client key tracking sets the FOV keybind flag on the
//! helmet key's press and clears it on its release (see fov_hook), and the server receiver plays the
//! spyglass use and stop sounds. The receiver has no position or dimension to play one at, so it queues
//! the sound for the player and the player tick — which carries both — plays it on its next tick.
//!
//! Three details an earlier lane left open were re-checked against the pinned original for
//! issue #1159, and all three are reproduced now. The equip sound is the original's
//! `SoundEvents.ARMOR_EQUIP_GENERIC` (`minecraft:item.armor.equip_generic`, played by both
//! ShieldStrapModule.startInteract and ToolBeltModule.startInteract at volume and pitch 1.0 —
//! and the id the host itself falls back to when a piece names no equip sound of its own);
//! a packet receiver names no position or dimension, so the sound is queued and the player
//! tick below plays it at the wearer's place, a tick later at most. The minimap's selection
//! message is sent (MinimapModule.onInventorySelect / onDisableSelection), as a chat line in
//! the player's language where the original uses the action bar — the reduction sleeves_hook
//! documents for its own selection messages — and the cycle follows
//! InventorySelectionModule.selectNext exactly, disabled position included. The spectator
//! skip is the receiver's first check: InteractionHandler wraps both startArmorInteract and
//! stopArmorInteract in `if (!player.isSpectator())`, and the receiver reads the flag the
//! player tick below records for every player (a player who has not ticked yet counts as no
//! spectator, as a freshly joined player would read in the original).
//!
//! One form needs no exchange and does run: SelfDestructiveModifier primes its wearer from the
//! helmet key (shift held — the packet's tooltip key is Shift exactly then, which is the
//! original's isShiftKeyDown check) and takes the effect off on release; the effect's own last
//! tick explodes on the player tick below, and taking the helmet off removes it there too
//! (SelfDestructiveModifier.onUnequip). The primed sound plays on the effect's first tick,
//! where the tick reports the wearer's place.
//!
//! The tank armor forms run here too, for the same reason: SlurpingModule's and
//! SpittingModule's armor timers change only the worn piece's own persistent data (which the
//! player handle writes at the armor slots) and the wearer's effects, so the receiver starts
//! and stops them through tank_hook on the plain tooltip key, ahead of the exchange drop.
use crate::{break_speed_hook::with_held_tool, modifier_modules::ZoomStyle, tool_container_menu, tool_inventory::{slot_of, stack_of, with_snapshot_items, MenuKey, ToolInventory}, tool_stack::{self, InteractPlan}};
use pluto_lib::fov_api_v1::FovRegistry;
use pluto_lib::{player_api::ServerPlayer, ActionTarget, ClientPlayNetworking, EntityAction, ExplosionInteraction, I18n, Identifier, ItemSlot, Nbt, NbtCompound, PlayerKeyEvents, ServerLivingEntityEvents, ServerPlayNetworking, StackData, World};
use std::collections::BTreeSet;
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

/// The message a minimap selection step shows (MinimapModule's two selection callbacks).
#[derive(Debug, PartialEq)]
pub(crate) enum Selection {
    /// onDisableSelection: the selection landed on the disabled position.
    Disabled,
    /// onInventorySelect: the newly selected stack and its slot (shown 1-based, with its map id).
    Selected { stack: ItemSlot, index: usize },
}

/// InventorySelectionModule.selectNext, as MinimapModule.startInteract runs it: the selection
/// advances by one, wrapping past the last slot onto the disabled position (one past the end),
/// skipping empty slots; the disabled position itself is a landing spot, not skipped. The new
/// selection is written to the persistent data and the message for it returned; None when the
/// selection cannot move (already at the disabled position with nothing before it to land on)
/// — the module then does not answer.
fn select_next(inventory: &ToolInventory, persistent: &mut NbtCompound) -> Option<Selection> {
    let slots = inventory.slots();
    let current = persistent.get("tconstruct:minimap_selected").and_then(Nbt::as_i64).unwrap_or(0).max(0) as usize;
    let mut next = (current + 1) % (slots + 1);
    while next < slots && inventory.stack(persistent, next).is_none() { next += 1; }
    if next == current { return None; }
    let selection = if next == slots { Selection::Disabled } else { Selection::Selected { stack: inventory.stack(persistent, next)?, index: next } };
    persistent.insert("tconstruct:minimap_selected".to_owned(), Nbt::Int(next as i32));
    Some(selection)
}

/// The minimap selection message keys (MinimapModule.onInventorySelect / onDisableSelection).
const MINIMAP_DISABLED_KEY: &str = "modifier.tconstruct.minimap.disabled";
const MINIMAP_SELECTED_KEY: &str = "modifier.tconstruct.minimap.selected";

/// The action-bar texts of the original, in the player's language, for the chat line
/// `apply_outcome` sends: the message surface PlutoLib offers is ServerPlayer::send_message
/// (a chat line, where the original's displayClientMessage(..., true) is the action bar), and
/// the selected stack's hover name is approximated by its item name's translation — the same
/// two reductions sleeves_hook documents for its selection messages. The selected message's
/// arguments are the original's own: the stack's name, its map id, and the slot shown 1-based.
fn selection_text(player: &ServerPlayer, selection: &Selection) -> String {
    I18n::use_language(Some(&player.language()));
    let text = match selection {
        Selection::Disabled => I18n::translate(MINIMAP_DISABLED_KEY),
        Selection::Selected { stack, index } => {
            let name = I18n::translate(&format!("item.{}", stack.item));
            let map = stack.map_id.unwrap_or(0).to_string();
            I18n::translate_args(MINIMAP_SELECTED_KEY, &[&name, &map, &(index + 1).to_string()])
        }
    };
    I18n::use_language(None);
    text
}

/// What an armor interact did, in the original's terms.
pub(crate) enum Outcome {
    /// No module answered the key (or the piece is not a built Tinkers armor tool).
    Nothing,
    /// A shield strap rotated: the piece with its new inventory, the new off hand, and the sound condition.
    Strap { piece: StackData, offhand: Option<ItemSlot>, sound: bool },
    /// A tool belt swapped: the piece, the main inventory's changed slots, and the sound condition.
    Belt { piece: StackData, main: Vec<(usize, Option<ItemSlot>)>, sound: bool },
    /// A minimap cycled its selection: the piece with the new selection in its data, and the
    /// selection message to show for where it landed.
    Select { piece: StackData, selection: Selection },
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
                    let selection = tool_stack::with_persistent(&mut data, |persistent| select_next(&inventory, persistent));
                    if let Some(selection) = selection {
                        let mut piece = piece.clone();
                        piece.custom_data = data;
                        return Outcome::Select { piece, selection };
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
    let wearer = ServerPlayer { uuid: player, name: String::new() };
    wearer.add_effect(&pluto_lib::player_api::EffectInstance::new("tconstruct:self_destructing", 30).amplifier((2 * level).clamp(0, 255) as u8));
}

/// SelfDestructiveModifier.stopInteract: the key's release takes the effect off.
fn self_destructive_stop(player: u128) {
    if self_destructive_level(player) <= 0 { return; }
    let wearer = ServerPlayer { uuid: player, name: String::new() };
    wearer.remove_effect("tconstruct:self_destructing");
}

/// The head piece's zoom style, if it carries a zoom module (ZoomModule's keybind half).
fn head_zoom_style(player: u128) -> Option<ZoomStyle> {
    let (_, head) = pluto_lib::entity_events_v1::equipment_of(player).into_iter().find(|(slot, _)| slot == "head")?;
    crate::fov_hook::zoom_style_of(&head)
}

/// The keybind zoom sounds the receiver asked for, by player: the receiver has no position or
/// dimension to play one at, so the player tick — which carries both — plays them on its next
/// tick (at most a tick late, unnoticeable for a key press).
static ZOOM_SOUNDS: Mutex<Vec<(u128, &'static str)>> = Mutex::new(Vec::new());

fn queue_zoom_sound(player: u128, sound: &'static str) {
    if let Ok(mut sounds) = ZOOM_SOUNDS.lock() {
        sounds.push((player, sound));
    }
}

fn take_zoom_sound(player: u128) -> Option<&'static str> {
    let mut sounds = ZOOM_SOUNDS.lock().ok()?;
    let at = sounds.iter().position(|(who, _)| *who == player)?;
    Some(sounds.remove(at).1)
}

/// The armor equip sound the strap and belt play when their sound condition holds:
/// `SoundEvents.ARMOR_EQUIP_GENERIC` in both modules of the pinned original, whose registry
/// name is this id (the host's own generic fallback in player_equip agrees).
const EQUIP_SOUND: &str = "minecraft:item.armor.equip_generic";

/// The equip sounds a keybind interact queued, by player: a packet receiver names no position or
/// dimension, so the player tick below plays them at the wearer's place, a tick later at most.
static PENDING_SOUNDS: Mutex<Vec<(u128, &'static str)>> = Mutex::new(Vec::new());

fn queue_sound(player: u128) {
    if let Ok(mut pending) = PENDING_SOUNDS.lock() {
        pending.push((player, EQUIP_SOUND));
    }
}

/// Runs one interact outcome through the carried-stack exchange surface: the worn piece takes the
/// whole-stack write-back first (it anchors the swap), then the off hand or the belt's changed
/// main slots. The menu opens through the item-menu surface; a piece that is not a built tool
/// opens nothing there.
fn apply_outcome(player: &ServerPlayer, carried: i32, piece: &StackData, outcome: Outcome) {
    match outcome {
        Outcome::Nothing => {}
        Outcome::Strap { piece, offhand, sound } => {
            let _ = player.set_stack(carried, Some(&piece));
            let offhand = offhand.as_ref().map(|slot| stack_of(slot));
            let _ = player.set_stack(-106, offhand.as_ref());
            if sound { queue_sound(player.uuid); }
        }
        Outcome::Belt { piece, main, sound } => {
            let _ = player.set_stack(carried, Some(&piece));
            for (slot, stack) in main {
                let stack = stack.as_ref().map(|slot| stack_of(slot));
                let _ = player.set_stack(slot as i32, stack.as_ref());
            }
            if sound { queue_sound(player.uuid); }
        }
        Outcome::Select { piece, selection } => {
            let _ = player.set_stack(carried, Some(&piece));
            player.send_message(&selection_text(player, &selection));
        }
        Outcome::Menu => {
            let _ = tool_container_menu::open_tool_menu(player, piece, carried);
        }
    }
}

/// The packet's interact, from the worn piece down: the piece, the off hand and the main inventory
/// as the named reads see them, the blacklist from the item snapshot, then the outcome applied.
/// A worn piece that is not a built Tinkers armor tool answers nothing (`interact` says so).
fn apply_interact(player: u128, slot: u8, key: MenuKey) {
    let wearer = ServerPlayer { uuid: player, name: String::new() };
    let carried = if slot == SLOT_HEAD { 103 } else { 101 };
    let Some(piece) = wearer.stack_named(carried) else { return; };
    let offhand = wearer.stack_named(-106).and_then(|stack| slot_of(&stack));
    let main: Vec<Option<ItemSlot>> = (0..36).map(|slot| wearer.stack_named(slot).and_then(|stack| slot_of(&stack))).collect();
    let outcome = with_snapshot_items(|items| interact(&piece, key, offhand, &main, &|stack| items.is_blacklisted(stack)));
    apply_outcome(&wearer, carried, &piece, outcome);
}

/// The players whose primed sound has played for the effect they currently carry, by uuid.
static PRIMED: Mutex<BTreeSet<u128>> = Mutex::new(BTreeSet::new());

/// The players the player tick last reported as spectators, by uuid: the receiver's
/// spectator skip reads it (InteractionHandler checks `player.isSpectator()` on the packet,
/// and the tick is the surface that carries the flag; the packet and the tick name the same
/// players, and the flag changes at most a tick before the receiver sees it).
static SPECTATORS: Mutex<BTreeSet<u128>> = Mutex::new(BTreeSet::new());

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
                // The keybind zoom's half of the state: held until the key is released (the FOV
                // callback answers it only while the head piece carries a spyglass zoom).
                FovRegistry::set_keybind_zoom(press.pressed);
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
        // InteractionHandler's spectator skip wraps both halves of the handler, so a
        // spectator's keypress runs nothing — not even the stop half's effect removal.
        if SPECTATORS.lock().map(|spectators| spectators.contains(&player)).unwrap_or(false) { return; }
        if !start {
            // SelfDestructiveModifier.stopInteract is the one stop half with work to do.
            if slot == SLOT_HEAD { self_destructive_stop(player); }
            // The tank armor forms stop here too (the slurp and spit timers on the helmet and
            // the leggings): a stop with no live timer is a no-op.
            crate::tank_hook::armor_stop(player, slot);
            // ZoomModule.stopInteract: letting go of the keybind zoom plays its stop sound (the
            // spyglass style only — the scope binds no armor interact hook).
            if slot == SLOT_HEAD && head_zoom_style(player) == Some(ZoomStyle::Spyglass) {
                queue_zoom_sound(player, "minecraft:item.spyglass.stop_using");
            }
            return;
        }
        // SelfDestructiveModifier.startInteract runs on the packet (see the module notes): the
        // tooltip key is Shift exactly when shift was held.
        if slot == SLOT_HEAD && key == MenuKey::Shift && self_destructive_level(player) > 0 {
            self_destructive_start(player);
            return;
        }
        // FireballModule.startInteract runs on the packet too: the helmet key with the normal
        // tooltip key shoots a fireball from the worn helmet (see fireball_hook).
        if slot == SLOT_HEAD && key == MenuKey::Normal && crate::fireball_hook::on_armor_interact(player) {
            return;
        }
        // The tank armor forms start on the plain tooltip key: the slurp and spit timers of
        // SlurpingModule and SpittingModule (their startInteract).
        if key == MenuKey::Normal && crate::tank_hook::armor_start(player, slot) {
            return;
        }
        // ZoomModule.startInteract: the helmet key's press plays the spyglass use sound (the
        // spyglass style only); the zoom itself is the client's keybind flag (see fov_hook).
        if slot == SLOT_HEAD && head_zoom_style(player) == Some(ZoomStyle::Spyglass) {
            queue_zoom_sound(player, "minecraft:item.spyglass.use");
            return;
        }
        // Anything else the key asks for runs through the carried-stack exchange surface.
        apply_interact(player, slot, key);
    }) {
        eprintln!("Tinkers: armor interact receiver not registered: {error}");
    }
    // SelfDestructiveEffect.applyEffectTick and SelfDestructiveModifier.onUnequip, over the
    // player tick: the primed sound on the effect's first tick, the explosion on its last,
    // and the removal when the helmet that primes it is no longer worn. Queued equip sounds
    // play here too, at the wearer's place (the packet receiver names none).
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        // The spectator record the receiver's skip reads.
        if tick.spectator {
            let _ = SPECTATORS.lock().map(|mut spectators| spectators.insert(tick.player));
        } else {
            let _ = SPECTATORS.lock().map(|mut spectators| spectators.remove(&tick.player));
        }
        // A keybind zoom sound the receiver queued for this player: the tick carries the position
        // and dimension the receiver never had, so it plays here, on the next tick.
        if let Some(sound) = take_zoom_sound(tick.player) {
            World::new(&tick.dimension).play_sound(sound, tick.position, 1.0, 1.0);
        }
        if let Ok(mut pending) = PENDING_SOUNDS.lock() {
            let mut index = 0;
            while index < pending.len() {
                if pending[index].0 == tick.player {
                    let (_, sound) = pending.remove(index);
                    World::new(&tick.dimension).play_sound(sound, tick.position, 1.0, 1.0);
                } else {
                    index += 1;
                }
            }
        }
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
    fn applying_a_strap_outcome_writes_the_piece_and_the_off_hand() {
        use pluto_lib::player_api::{with_ops, MemoryPlayerOps};
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 7, name: "Alex".into() };
        let piece = StackData { item: "tconstruct:helmet".into(), count: 1, ..StackData::default() };
        ops.carried.insert((7, 103), piece.clone());
        let outcome = Outcome::Strap { piece: piece.clone(), offhand: Some(stack("minecraft:shield", 1)), sound: true };
        with_ops(&mut ops, || apply_outcome(&player, 103, &piece, outcome));
        assert_eq!(ops.carried.get(&(7, 103)), Some(&piece), "the rotated piece is worn again");
        assert_eq!(ops.carried.get(&(7, -106)).map(|stack| (stack.item.as_str(), stack.count)), Some(("minecraft:shield", 1)), "the first strap slot is the new off hand");
        assert!(PENDING_SOUNDS.lock().map(|pending| pending.iter().any(|(uuid, _)| *uuid == 7)).unwrap_or(false), "the equip sound waits for the tick");
        PENDING_SOUNDS.lock().map(|mut pending| pending.retain(|(uuid, _)| *uuid != 7)).unwrap();
    }

    #[test]
    fn applying_a_belt_outcome_writes_the_piece_and_each_changed_slot() {
        use pluto_lib::player_api::{with_ops, MemoryPlayerOps};
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 8, name: "Alex".into() };
        let piece = StackData { item: "tconstruct:leggings".into(), count: 1, ..StackData::default() };
        let outcome = Outcome::Belt {
            piece: piece.clone(),
            main: vec![(0, Some(stack("tconstruct:pickaxe", 1))), (2, None)],
            sound: true,
        };
        with_ops(&mut ops, || apply_outcome(&player, 101, &piece, outcome));
        assert_eq!(ops.carried.get(&(8, 101)), Some(&piece));
        assert_eq!(ops.carried.get(&(8, 0)).map(|stack| stack.item.as_str()), Some("tconstruct:pickaxe"), "the belt's tool is in the hotbar");
        assert_eq!(ops.carried.get(&(8, 2)), None, "an emptied slot is cleared, not left holding the old stack");
        PENDING_SOUNDS.lock().map(|mut pending| pending.retain(|(uuid, _)| *uuid != 8)).unwrap();
    }

    #[test]
    fn applying_a_selection_outcome_writes_the_piece_and_sends_the_message() {
        use pluto_lib::player_api::{with_ops, MemoryPlayerOps};
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 9, name: "Alex".into() };
        let mut piece = StackData { item: "tconstruct:helmet".into(), count: 1, ..StackData::default() };
        piece.custom_data.insert("tconstruct:minimap_selected".to_owned(), Nbt::Int(2));
        let outcome = Outcome::Select { piece: piece.clone(), selection: Selection::Selected { stack: stack("minecraft:filled_map", 1), index: 2 } };
        with_ops(&mut ops, || apply_outcome(&player, 103, &piece, outcome));
        assert_eq!(ops.carried.get(&(9, 103)), Some(&piece), "the piece carries the new selection");
        assert_eq!(ops.carried.get(&(9, -106)), None, "the off hand is untouched");
        assert_eq!(ops.told.len(), 1, "the selection message is sent");
        assert_eq!(ops.told[0].0, 9, "to the wearer");
    }

    #[test]
    fn applying_nothing_writes_nothing_and_a_menu_for_an_unloaded_tool_opens_nothing() {
        use pluto_lib::player_api::{with_ops, MemoryPlayerOps};
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 10, name: "Alex".into() };
        let piece = StackData { item: "tconstruct:helmet".into(), count: 1, ..StackData::default() };
        with_ops(&mut ops, || {
            apply_outcome(&player, 103, &piece, Outcome::Nothing);
            apply_outcome(&player, 103, &piece, Outcome::Menu);
        });
        assert!(ops.carried.is_empty(), "no outcome, no write");
        assert!(ops.opened_item_menus.is_empty(), "the menu needs a built tool, and the registries are not loaded in this test");
    }

    #[test]
    fn the_minimap_selection_cycles_through_the_disabled_position_and_wraps() {
        let module = Inventory::decode(json!({"slots": {"flat": 3}}).as_object().unwrap()).unwrap().unwrap();
        let part = Part::new(module, Entry::new(id("tconstruct:minimap"), 1), true, true);
        let inventory = ToolInventory::new(vec![part]);
        let mut persistent = NbtCompound::new();
        inventory.set_stack(&mut persistent, 0, Some(&stack("minecraft:filled_map", 1)));
        inventory.set_stack(&mut persistent, 2, Some(&stack("minecraft:filled_map", 1)));
        // From the first map to the next stored one, skipping the empty slot between.
        assert_eq!(select_next(&inventory, &mut persistent), Some(Selection::Selected { stack: stack("minecraft:filled_map", 1), index: 2 }));
        assert_eq!(persistent.get("tconstruct:minimap_selected").and_then(Nbt::as_i64), Some(2), "the selection is written");
        // Past the last slot is the disabled position: a landing spot, not skipped.
        assert_eq!(select_next(&inventory, &mut persistent), Some(Selection::Disabled));
        assert_eq!(persistent.get("tconstruct:minimap_selected").and_then(Nbt::as_i64), Some(3), "the disabled position is one past the end");
        // And from there the cycle wraps to the first stored map.
        assert_eq!(select_next(&inventory, &mut persistent), Some(Selection::Selected { stack: stack("minecraft:filled_map", 1), index: 0 }));
        // A single stored map: from its own slot the cycle still moves, onto the disabled position.
        let mut single = NbtCompound::new();
        inventory.set_stack(&mut single, 1, Some(&stack("minecraft:filled_map", 1)));
        single.insert("tconstruct:minimap_selected".to_owned(), Nbt::Int(1));
        assert_eq!(select_next(&inventory, &mut single), Some(Selection::Disabled));
        // An empty inventory already at the disabled position cannot move: no answer.
        let mut empty = NbtCompound::new();
        empty.insert("tconstruct:minimap_selected".to_owned(), Nbt::Int(3));
        assert_eq!(select_next(&inventory, &mut empty), None);
        // But from a slot position it advances onto the disabled position, as the original does.
        assert_eq!(select_next(&inventory, &mut NbtCompound::new()), Some(Selection::Disabled));
    }

    #[test]
    fn a_queued_keybind_zoom_sound_plays_once_for_its_player_only() {
        queue_zoom_sound(7, "minecraft:item.spyglass.use");
        queue_zoom_sound(9, "minecraft:item.spyglass.stop_using");
        assert_eq!(take_zoom_sound(9), Some("minecraft:item.spyglass.stop_using"), "the other player's sound is untouched");
        assert_eq!(take_zoom_sound(7), Some("minecraft:item.spyglass.use"));
        assert_eq!(take_zoom_sound(7), None, "a queued sound plays once");
        assert_eq!(take_zoom_sound(9), None);
    }
}
