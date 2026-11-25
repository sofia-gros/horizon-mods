//! SleevesModule (GeneralInteractionModifierHook) and InventorySelectionModule, from SlimeKnights
//! (MIT; see LICENSE): an empty-hand use while wearing a chest piece with the sleeves modifier
//! either uses the selected stack of the modifier's own inventory (the sibling inventory module's
//! slots, whose filter admits the `tconstruct:throwable` tag) or, crouching or with nothing usable
//! selected, cycles the selection kept in the piece's persistent data under
//! `tconstruct:sleeves_selected` — one position past the last slot is the disabled position, and
//! cycling skips empty slots. The display half already exists: hud.rs draws the selected stored
//! item beside the offhand slot from the same keys.
//!
//! The trigger exists now: PlutoLib's `AfterUseCallback` fires when a use finished without the
//! game consuming it (an empty-hand right click the world interaction did not take), carrying
//! the crouch state and the eye and look the throw aims by. `register` answers it for a
//! non-spectator wearing a chest piece with the sleeves modifier, with the selected item's
//! cooldown as the gate (the host seam already excluded spectators and consumed uses). The
//! entity-click path of the original's takeover is covered by the same callback: a click on
//! an entity the game's own interaction did not take is reported through it too — the woven
//! entity seam reports the clicks that arrive as entity packets, and the local player's
//! click falls through to the hand's use, which the hand seam reports — so an empty-hand
//! click on an entity throws or cycles exactly as a click on air does. The original runs
//! the entity's interaction first and the takeover only when it took nothing
//! (InteractionHandler.afterEntityInteract); both seams keep that order.
//!
//! Reductions kept honest:
//! - Cooldowns: the original checks the selected item's cooldown before using it, and the
//!   takeover's dispatch checks the piece's own item before any chestplate hook runs
//!   (InteractionHandler, at the head of `afterEntityInteract` and inside
//!   `onChestplateUse`). Both gates are wired: `register` reads them through
//!   `ServerPlayer::is_item_on_cooldown` — the piece's own item first, then the selected
//!   item's — and `execute` starts the selected item's cooldown for a thrown stack
//!   (`THROW_COOLDOWN_TICKS`, the held shuriken's half-second `use_cooldown`). A chest
//!   piece has no use of its own that starts a cooldown, so the piece gate passes in
//!   practice unless another system put the piece's item on cooldown; it is checked
//!   anyway, as the original checks it.
//! - Only this port's own throwables are thrown: the sibling inventory's filter admits any
//!   `tconstruct:throwable`-tagged stack, and the original swaps it into the hand and calls its
//!   use, whatever it is. A generic "use any stack as if held" verb is deliberately not built
//!   (the shipped data patterns the slots for the shuriken, the port's own throwable); a foreign
//!   tagged stack makes the throw decline, and `execute` then leaves the piece untouched.
//! - The selection messages go as chat lines in the player's language (see `message_text`),
//!   where the original shows them on the action bar; the HUD already reflects the selection
//!   either way.
//! - The interaction gate (ToolHooks.INTERACTION.canInteract) belongs to the caller, which
//!   owns the player; the takeover's preconditions (an unconsumed use, an empty hand, a
//!   non-spectator) are the host seam's, and the piece must be a whole, unbroken Tinkers
//!   tool, as every consumer through `with_held_tool` requires.
use crate::{break_speed_hook::with_held_tool, throwing_hook, tool_inventory::ToolInventory, tool_stack};
use pluto_lib::{ActionResult, AfterUseCallback, I18n, ItemSlot, Nbt, NbtCompound, ServerPlayer, StackData, StackSlot};

/// SleevesModule.SELECTED_SLOT: the selected slot in the piece's persistent data.
const SELECTED: &str = "tconstruct:sleeves_selected";
/// The cooldown a sleeves throw starts on the thrown item (the held shuriken's
/// `use_cooldown`: half a second).
const THROW_COOLDOWN_TICKS: i32 = 10;
/// The action-bar message keys (SleevesModule.DISABLED and SELECTED).
const DISABLED_KEY: &str = "modifier.tconstruct.sleeves.disabled";
const SELECTED_KEY: &str = "modifier.tconstruct.sleeves.selected";

/// The message a selection cycle shows (InventorySelectionModule's two callbacks).
pub(crate) enum Message {
    /// onDisableSelection: the selection landed on the disabled position.
    Disabled,
    /// onInventorySelect: the newly selected stack and its slot (shown 1-based).
    Selected { stack: ItemSlot, index: usize },
}

/// What a sleeves use did, in the original's terms (InteractionResult).
pub(crate) enum Outcome {
    /// PASS: no sleeves module answered, or one of the original's pass cases (a cooling-down
    /// selection, the disabled position with nothing selected, a selection that cannot move).
    Nothing,
    /// SUCCESS by cycling: the piece with its new selection in its data, and the message for it.
    Cycled { piece: StackData, message: Message },
    /// The selected stack was used: the piece with the use's remainder stored back, and the one
    /// stack that left the inventory.
    Used { piece: StackData, thrown: ItemSlot },
}

/// `use_stored`'s outcome, before the piece around it is rebuilt.
enum Stored { Nothing, Cycled(Message), Used(ItemSlot) }

/// The selected slot, 0 when none was ever written.
fn selected(persistent: &NbtCompound) -> usize {
    crate::modifier_entries::nbt_int(persistent.get(SELECTED)).max(0) as usize
}

/// InventorySelectionModule.selectNext: the selection advances by one, wrapping past the last
/// slot onto the disabled position (one past the end), skipping empty slots; the disabled
/// position itself is a landing spot, not skipped. The new selection is written to the
/// persistent data and the message for it returned; None when the selection cannot move (an
/// empty inventory already at the disabled position) — the module then does not answer.
fn select_next(inventory: &ToolInventory, persistent: &mut NbtCompound) -> Option<Message> {
    let slots = inventory.slots();
    let current = selected(persistent);
    let mut next = (current + 1) % (slots + 1);
    while next < slots && inventory.stack(persistent, next).is_none() { next += 1; }
    if next == current { return None; }
    let message = if next == slots { Message::Disabled } else { Message::Selected { stack: inventory.stack(persistent, next)?, index: next } };
    persistent.insert(SELECTED.to_owned(), Nbt::Int(next as i32));
    Some(message)
}

/// SleevesModule.onToolUse over the modifier's inventory in a piece's persistent data.
/// `crouching` is the player's sneak state (the original's isCrouching branch); `cooling_down`
/// whether the selected stack's item is on the player's cooldown, which the caller reads
/// through `ServerPlayer::is_item_on_cooldown`.
fn use_stored(inventory: &ToolInventory, persistent: &mut NbtCompound, crouching: bool, cooling_down: bool) -> Stored {
    let slots = inventory.slots();
    if !crouching {
        let selected = selected(persistent);
        if selected < slots {
            if let Some(stack) = inventory.stack(persistent, selected) {
                // A selection on cooldown passes without toggling (the original returns PASS).
                if cooling_down { return Stored::Nothing; }
                // The use: one of the stack leaves, and the remainder is stored back where the
                // stack was (the result stack of the port's throwing flow, which spends one).
                let mut rest = stack.clone();
                rest.count -= 1;
                inventory.set_stack(persistent, selected, (rest.count > 0).then_some(&rest));
                let mut thrown = stack;
                thrown.count = 1;
                return Stored::Used(thrown);
            }
        }
        // Nothing usable is selected; at the disabled position the use passes outright, and
        // anywhere else it falls through to the slot toggle.
        if selected == slots { return Stored::Nothing; }
    }
    match select_next(inventory, persistent) {
        Some(message) => Stored::Cycled(message),
        None => Stored::Nothing,
    }
}

/// SleevesModule.onToolUse over a worn piece: its sleeves modifier's inventory, read and written
/// in the piece's own persistent data.
pub(crate) fn on_tool_use(piece: &StackData, crouching: bool, cooling_down: bool) -> Outcome {
    let Ok(outcome) = with_held_tool(piece, |env, tool| {
        let Some(inventory) = tool_stack::sleeves_inventory(&tool.custom_data, env) else { return Outcome::Nothing };
        let mut data = tool.custom_data.clone();
        let stored = tool_stack::with_persistent(&mut data, |persistent| use_stored(&inventory, persistent, crouching, cooling_down));
        match stored {
            Stored::Nothing => Outcome::Nothing,
            Stored::Cycled(message) => { let mut piece = piece.clone(); piece.custom_data = data; Outcome::Cycled { piece, message } }
            Stored::Used(thrown) => { let mut piece = piece.clone(); piece.custom_data = data; Outcome::Used { piece, thrown } }
        }
    }) else { return Outcome::Nothing; };
    outcome
}

/// The stack the piece's selection names, if any: the selected slot's stack, or None for an
/// empty slot or the disabled position. Pure over the inventory and the persistent data, so
/// the cooldown gate can read it without touching the game.
pub(crate) fn selected_stack_item(inventory: &ToolInventory, persistent: &NbtCompound) -> Option<ItemSlot> {
    let slots = inventory.slots();
    let selected = selected(persistent);
    if selected >= slots {
        return None;
    }
    inventory.stack(persistent, selected)
}

/// The selected stack of a worn piece, if the piece is a whole, unbroken Tinkers tool with a
/// sleeves inventory (None otherwise).
fn selected_item(piece: &StackData) -> Option<ItemSlot> {
    let Ok(selected) = with_held_tool(piece, |env, tool| {
        let Some(inventory) = tool_stack::sleeves_inventory(&tool.custom_data, env) else { return None };
        let mut data = tool.custom_data.clone();
        tool_stack::with_persistent(&mut data, |persistent| selected_stack_item(&inventory, persistent))
    }) else { return None };
    selected
}

/// One sleeves use by a player wearing `piece` in the chest slot, end to end: the core above,
/// the thrown stack through the port's throwing flow, the piece written back to the chest slot
/// and the selection message sent. False when nothing was used or cycled, or when the throw
/// could not be summoned — the piece is then left as it was, rather than eating the stored stack.
pub(crate) fn execute(player: &ServerPlayer, dimension: &str, eye: [f64; 3], look: [f64; 3], piece: &StackData, crouching: bool, creative: bool, cooling_down: bool) -> bool {
    match on_tool_use(piece, crouching, cooling_down) {
        Outcome::Nothing => false,
        Outcome::Cycled { piece, message } => {
            if !player.set_equipment(StackSlot::Chest, Some(&piece)) { return false; }
            player.send_message(&message_text(player, &message));
            true
        }
        Outcome::Used { piece, thrown } => {
            if !throwing_hook::throw_stored_shuriken(player, dimension, eye, look, &thrown) { return false; }
            // A creative use spends nothing, as the held flow keeps its stack (throwing_hook).
            if !creative { player.set_equipment(StackSlot::Chest, Some(&piece)); }
            // The throw starts the selected item's cooldown, even in creative: the held
            // shuriken's release starts the same cooldown through its `used` mark.
            player.start_item_cooldown(&thrown.item.to_string(), THROW_COOLDOWN_TICKS);
            true
        }
    }
}

/// Registers the sleeves answer on the finished use: a player wearing a chest piece with the
/// sleeves modifier uses the selected stack (or cycles the selection crouching), gated on
/// the piece's own item cooldown first and then the selected item's — the original's two
/// checks, the piece's at the takeover's dispatch (InteractionHandler) and the selected
/// item's inside SleevesModule. Spectators and consumed uses never reach here: the host
/// seams gate both, on the hand's use and on the entity interaction alike. The piece must
/// carry the sleeves modifier — `on_tool_use` answers nothing without it — and the chest
/// piece is the one the tick memory last saw worn (see `equipment_of`).
pub(crate) fn register() {
    AfterUseCallback::EVENT.register(|context| {
        let Some(piece) = chest_piece(&context.player) else { return ActionResult::Pass };
        // The takeover's own gate comes first: the original checks the piece's item
        // cooldown before any chestplate hook runs, on every path into the takeover.
        if context.player.is_item_on_cooldown(&piece.item) { return ActionResult::Pass; }
        let cooling_down = selected_item(&piece)
            .is_some_and(|selected| context.player.is_item_on_cooldown(&selected.item.to_string()));
        if execute(&context.player, &context.dimension, context.eye, context.look, &piece, context.crouching, context.creative, cooling_down) {
            ActionResult::Success
        } else {
            ActionResult::Pass
        }
    });
}

/// The chest piece the player wears, as of their last tick, if any.
fn chest_piece(player: &ServerPlayer) -> Option<StackData> {
    pluto_lib::entity_events_v1::equipment_of(player.uuid)
        .into_iter()
        .find(|(slot, _)| slot == "chest")
        .map(|(_, stack)| stack)
}

/// The action-bar texts of the original, in the player's language, for the chat line
/// `execute` sends: the message surface PlutoLib offers is ServerPlayer::send_message (a chat
/// line, where the original's displayClientMessage(..., true) is the action bar), and the
/// selected stack's hover name is approximated by its item name's translation — a built tool's
/// true hover name is composed from its materials by the game, which no surface here returns.
fn message_text(player: &ServerPlayer, message: &Message) -> String {
    I18n::use_language(Some(&player.language()));
    let text = match message {
        Message::Disabled => I18n::translate(DISABLED_KEY),
        Message::Selected { stack, index } => {
            let name = I18n::translate(&format!("item.{}", stack.item));
            I18n::translate_args(SELECTED_KEY, &[&name, &(index + 1).to_string()])
        }
    };
    I18n::use_language(None);
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{modifier_entries::Entry, tool_inventory::{Inventory, Part}};
    use pluto_lib::Identifier;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn stack(item: &str, count: u32) -> ItemSlot { ItemSlot::new(id(item), count) }

    /// The shipped sleeves inventory (the sibling module of sleeves.json): three slots per
    /// level, a stack limit of 16, the throwable tag as its filter, the shuriken pattern.
    fn sleeves_inventory() -> ToolInventory {
        let module = Inventory::decode(json!({"filter": {"type": "mantle:tag", "tag": "tconstruct:throwable"}, "limit": {"flat": 16}, "pattern": "tconstruct:shuriken", "slots": {"each_level": 3}}).as_object().unwrap()).unwrap().unwrap();
        let part = Part::new(module, Entry::new(id("tconstruct:sleeves"), 1), true, true);
        ToolInventory::new(vec![part])
    }

    /// A level 1 sleeves inventory holding shurikens in every slot.
    fn stocked() -> (ToolInventory, NbtCompound) {
        let inventory = sleeves_inventory();
        let mut persistent = NbtCompound::new();
        inventory.set_stack(&mut persistent, 0, Some(&stack("tconstruct:shuriken", 2)));
        inventory.set_stack(&mut persistent, 1, Some(&stack("tconstruct:shuriken", 1)));
        inventory.set_stack(&mut persistent, 2, Some(&stack("tconstruct:shuriken", 4)));
        (inventory, persistent)
    }

    #[test]
    fn crouching_cycles_through_the_slots_to_the_disabled_position_and_around() {
        let (inventory, mut persistent) = stocked();
        // From the default selection (0): to 1, then to 2.
        for (index, count) in [(1, 1), (2, 4)] {
            let Stored::Cycled(Message::Selected { stack, index: at }) = use_stored(&inventory, &mut persistent, true, false) else { panic!("a cycle to a stocked slot") };
            assert_eq!(at, index);
            assert_eq!(stack.count, count, "the message names the newly selected stack");
            assert_eq!(selected(&persistent), index, "the selection is written");
        }
        // Then the disabled position, one past the last slot.
        let Stored::Cycled(Message::Disabled) = use_stored(&inventory, &mut persistent, true, false) else { panic!("a cycle to the disabled position") };
        assert_eq!(selected(&persistent), 3);
        // And around again to the first slot.
        let Stored::Cycled(Message::Selected { index: at, .. }) = use_stored(&inventory, &mut persistent, true, false) else { panic!("a cycle around") };
        assert_eq!(at, 0);
        assert_eq!(selected(&persistent), 0);
    }

    #[test]
    fn cycling_skips_empty_slots_and_an_empty_inventory_answers_nothing() {
        let inventory = sleeves_inventory();
        let mut persistent = NbtCompound::new();
        inventory.set_stack(&mut persistent, 2, Some(&stack("tconstruct:shuriken", 1)));
        // From 0 the next stocked slot is 2; the empty slots are skipped.
        let Stored::Cycled(Message::Selected { index: at, .. }) = use_stored(&inventory, &mut persistent, true, false) else { panic!("a cycle to the stocked slot") };
        assert_eq!(at, 2);
        // From there, only the disabled position remains before the wrap.
        let Stored::Cycled(Message::Disabled) = use_stored(&inventory, &mut persistent, true, false) else { panic!("a cycle to the disabled position") };
        // From the disabled position the wrap lands back on the one stocked slot.
        let Stored::Cycled(Message::Selected { index: at, .. }) = use_stored(&inventory, &mut persistent, true, false) else { panic!("a cycle around") };
        assert_eq!(at, 2);
        // An empty inventory already at the disabled position cannot move.
        let mut empty = NbtCompound::new();
        empty.insert(SELECTED.to_owned(), Nbt::Int(3));
        assert!(matches!(use_stored(&inventory, &mut empty, true, false), Stored::Nothing));
        assert_eq!(selected(&empty), 3, "the selection stays where it was");
    }

    #[test]
    fn a_stored_shuriken_is_thrown_one_at_a_time_and_the_remainder_stays() {
        let (inventory, mut persistent) = stocked();
        let Stored::Used(thrown) = use_stored(&inventory, &mut persistent, false, false) else { panic!("a throw") };
        assert_eq!(thrown, stack("tconstruct:shuriken", 1), "one shuriken leaves");
        assert_eq!(inventory.stack(&persistent, 0), Some(stack("tconstruct:shuriken", 1)), "the rest of the stack stays stored");
        assert_eq!(selected(&persistent), 0, "a throw does not move the selection");
        // The last one empties the slot.
        let Stored::Used(_) = use_stored(&inventory, &mut persistent, false, false) else { panic!("a throw") };
        assert_eq!(inventory.stack(&persistent, 0), None);
    }

    #[test]
    fn an_empty_selection_falls_through_to_cycling() {
        let inventory = sleeves_inventory();
        let mut persistent = NbtCompound::new();
        inventory.set_stack(&mut persistent, 1, Some(&stack("tconstruct:shuriken", 1)));
        // Slot 0 is selected and empty: the use cycles instead of throwing.
        let Stored::Cycled(Message::Selected { index: at, .. }) = use_stored(&inventory, &mut persistent, false, false) else { panic!("a cycle") };
        assert_eq!(at, 1);
        assert_eq!(selected(&persistent), 1);
    }

    #[test]
    fn the_disabled_position_and_a_cooling_down_selection_pass() {
        let (inventory, mut persistent) = stocked();
        persistent.insert(SELECTED.to_owned(), Nbt::Int(3));
        assert!(matches!(use_stored(&inventory, &mut persistent, false, false), Stored::Nothing), "the disabled position passes");
        assert_eq!(selected(&persistent), 3, "and does not toggle");
        // A stocked selection on cooldown passes without throwing or toggling.
        persistent.insert(SELECTED.to_owned(), Nbt::Int(0));
        assert!(matches!(use_stored(&inventory, &mut persistent, false, true), Stored::Nothing));
        assert_eq!(inventory.stack(&persistent, 0), Some(stack("tconstruct:shuriken", 2)), "nothing was thrown");
        assert_eq!(selected(&persistent), 0, "and the selection did not toggle");
    }

    #[test]
    fn the_takeover_gate_reads_the_pieces_own_item_cooldown() {
        use pluto_lib::MemoryPlayerOps;
        // InteractionHandler gates every chestplate interaction on the piece's own item
        // cooldown (`getCooldowns().isOnCooldown(chestplate.getItem())`), at the dispatch,
        // separately from the selected item's gate inside the module. The read is the one
        // `register` makes for the piece before anything else, over the memory backend.
        let piece = StackData { item: "tconstruct:plate_chestplate".into(), count: 1, ..Default::default() };
        let player = ServerPlayer { uuid: 9, name: "Alex".into() };
        let mut ops = MemoryPlayerOps::default();
        pluto_lib::with_ops(&mut ops, || {
            assert!(!player.is_item_on_cooldown(&piece.item), "a piece whose item never started a cooldown passes the gate");
            player.start_item_cooldown(&piece.item, 20);
            assert!(player.is_item_on_cooldown(&piece.item), "the piece's own item on cooldown closes the gate");
            assert!(!player.is_item_on_cooldown("tconstruct:shuriken"), "the selected item's cooldown is the other gate, not this one");
        });
    }

    #[test]
    fn the_selected_stack_item_is_the_cooldown_gate_item() {
        let (inventory, mut persistent) = stocked();
        // The default selection names the first stocked slot.
        assert_eq!(selected_stack_item(&inventory, &persistent), Some(stack("tconstruct:shuriken", 2)));
        // After cycling, the selection names the next stocked slot.
        let Stored::Cycled(_) = use_stored(&inventory, &mut persistent, true, false) else { panic!("a cycle") };
        assert_eq!(selected_stack_item(&inventory, &persistent), Some(stack("tconstruct:shuriken", 1)));
        // A selected slot that holds nothing names nothing, as does the disabled position.
        let bare = sleeves_inventory();
        let mut persistent = NbtCompound::new();
        bare.set_stack(&mut persistent, 2, Some(&stack("tconstruct:shuriken", 1)));
        assert_eq!(selected_stack_item(&bare, &persistent), None, "slot 0 is empty");
        persistent.insert(SELECTED.to_owned(), Nbt::Int(3));
        assert_eq!(selected_stack_item(&bare, &persistent), None, "the disabled position names nothing");
    }
}
