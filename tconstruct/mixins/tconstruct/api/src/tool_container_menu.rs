//! The worn tool's container menu (InventoryMenuModule and ToolContainerMenu in the original):
//! opening the armor piece's inventory screen from the armor keybind, and writing back what the
//! player leaves in it. SlimeKnights (MIT; see LICENSE).
//!
//! The shape: the piece's inventory opens in the smallest tool-container menu type that holds it
//! (one per row count, `tconstruct:tool_container_1` to `tconstruct:tool_container_12`), titled with
//! the piece's name, through the item-menu surface (`PlayerOps::open_item_menu`) — a native-slots
//! menu of the mod menu type, not a block's. The extra data marks an item menu for the client
//! screen and names the worn slot's carried number, so the close knows which piece to write back
//! to. The menu types are registered here as windows and as synthetic block entity types (their
//! block id is the menu type itself, so `block_of_menu` resolves for the screen events, although
//! no block of that id exists in the world).
//!
//! What the close does, and where it differs from the original: the player moves items in the
//! menu by vanilla's click rules — the host's slot rules are vanilla, so the tool's blacklist and
//! module filters cannot be enforced at placement (issue #1159, checked against the pinned host:
//! this menu is served by the host's LAN container engine, whose placement check consults no mod
//! rule, and the one slot-rule hook the host offers feeds only the game menu engine's mod slots
//! and carries no player or tool identity to judge a per-tool filter by). They are enforced at
//! close instead: the
//! kept stacks go into the worn piece's inventory only where the tool takes them, and a stack the
//! tool does not take stays in the menu's slots and comes back to the player with the close, as
//! vanilla's `clearContainer` gives a crafting grid's items back. The menu is padded to its type's
//! full slot count (the host's menu type fixes the count), so a tool with fewer slots shows empty
//! slots past its inventory: anything left in those comes back to the player too, never into the
//! tool. The original's ToolContainerMenu is a single menu type with no slot cap at all; the
//! port's per-row types are registered up to twelve rows, so a tool with more than 108 inventory
//! slots opens nothing (the largest inventory the shipped modifier data can build is well under
//! half that). The worn piece is read again at
//! close and written back whole, so any damage or change it took while the menu was open is kept;
//! when the worn slot no longer holds the same piece, nothing is taken and the kept stacks come
//! back to the player instead.
//!
//! The client half rides the item-menu surface in PlutoLib: the return hook on `open_native_screen`
//! opens the screen from the menu type's own layout when the extra data marks an item menu (the
//! block path returns early for one, since no block at the stand-in position carries the menu).
use crate::{break_speed_hook::with_held_tool, tool_inventory::{with_snapshot_items, ToolInventory}, tool_stack};
use pluto_lib::{
    player_api::ServerPlayer,
    block_entity_api_v1::{BlockEntityType, BlockEntityTypeRegistry},
    screen_handler_api_v1::{NativeMenuClosed, NativeMenuEvents, NativeMenuInputClosed, ScreenCanvas, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout},
    I18n, Identifier, ItemSlot, StackData,
};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, Once};

/// The item menu's extra-data magic: the versioned mark the client screen reads to tell an item
/// menu from a block's (`PlayerOps::open_item_menu` documents the contract).
const ITEM_MENU_MAGIC: &[u8] = b"PLUTO\x01";

/// The window's sheet (`ToolContainerScreen`'s TEXTURE) and its segment geometry
/// (`ToolContainerMenu`: UI_START 4 + TITLE_SIZE 13 puts the repeating part at 17; the player
/// inventory segment starts at 17 + 6 rows and is 96 high).
const TOOL_INVENTORY: &str = "tconstruct:gui/tool_inventory";
const REPEAT_START: f32 = 17.0;
const PLAYER_SEGMENT_Y: f32 = REPEAT_START + 6.0 * 18.0;
const PLAYER_SEGMENT_H: f32 = 96.0;

/// Draws the tool window's background from its sheet, as `ToolContainerScreen.renderBg` builds
/// it: the top segment up to the slot rows (a single stretch while they fit in six rows, the
/// six-row chunk repeated past that), the player inventory segment below them, and a slot-row
/// strip under every row of tool slots (the segments themselves carry no slot frames). The
/// layout is one pixel taller than the original's window, so the last segment stretches by
/// that pixel to close the foot. The original's crafting grid, fluid tank bar, per-slot
/// patterns, offhand shield and selected-slot highlight have no counterpart: this menu holds
/// none of those slots, and the patterns and the worn slot never reach the client draw.
fn draw_tool_background(canvas: &mut dyn ScreenCanvas, origin: [f32; 2], rows: usize) {
    let (x, y) = (origin[0], origin[1]);
    let slot_background = REPEAT_START + rows as f32 * 18.0;
    if slot_background < PLAYER_SEGMENT_Y {
        canvas.sprite(TOOL_INVENTORY, [0.0, 0.0, 176.0, slot_background], [x, y, 176.0, slot_background]);
    } else {
        canvas.sprite(TOOL_INVENTORY, [0.0, 0.0, 176.0, PLAYER_SEGMENT_Y], [x, y, 176.0, PLAYER_SEGMENT_Y]);
        let mut offset = PLAYER_SEGMENT_Y;
        let mut remaining = slot_background - PLAYER_SEGMENT_Y;
        while remaining > 6.0 * 18.0 {
            canvas.sprite(TOOL_INVENTORY, [0.0, REPEAT_START, 176.0, 6.0 * 18.0], [x, y + offset, 176.0, 6.0 * 18.0]);
            offset += 6.0 * 18.0;
            remaining -= 6.0 * 18.0;
        }
        if remaining > 0.0 {
            canvas.sprite(TOOL_INVENTORY, [0.0, REPEAT_START, 176.0, remaining], [x, y + offset, 176.0, remaining]);
        }
    }
    canvas.sprite(TOOL_INVENTORY, [0.0, PLAYER_SEGMENT_Y, 176.0, PLAYER_SEGMENT_H], [x, y + slot_background, 176.0, PLAYER_SEGMENT_H + 1.0]);
    for row in 0..rows {
        canvas.sprite(TOOL_INVENTORY, [0.0, 238.0, 162.0, 18.0], [x + 7.0, y + REPEAT_START + row as f32 * 18.0, 162.0, 18.0]);
    }
}

/// The tool container menu types, one per row count: a tool's inventory is shown in the smallest
/// that holds it, padded with empty slots to the type's full count. Twelve rows are registered:
/// the original's single menu type has no cap, and twelve rows (108 slots) cover any inventory
/// the shipped modifier data can build with room to spare.
const MENU_KINDS: [&str; 12] = [
    "tconstruct:tool_container_1",
    "tconstruct:tool_container_2",
    "tconstruct:tool_container_3",
    "tconstruct:tool_container_4",
    "tconstruct:tool_container_5",
    "tconstruct:tool_container_6",
    "tconstruct:tool_container_7",
    "tconstruct:tool_container_8",
    "tconstruct:tool_container_9",
    "tconstruct:tool_container_10",
    "tconstruct:tool_container_11",
    "tconstruct:tool_container_12",
];

/// The menu type for a tool inventory of `slots` slots, or None past twelve rows (108 slots). An
/// inventory with no slots takes the one-row type.
fn menu_kind_for(slots: usize) -> Option<&'static str> {
    if slots > 108 { return None; }
    Some(MENU_KINDS[slots.saturating_sub(1) / 9])
}

/// A tool container menu open for a worn piece: the worn slot's carried number, the piece's item
/// (which identifies it at close), and the tool's inventory layout (which persistent slots the
/// kept stacks go back into).
struct OpenToolMenu {
    worn: i32,
    item: String,
    inventory: ToolInventory,
}

/// The tool container menus open now, by (player, window).
static OPEN: LazyLock<Mutex<HashMap<(u128, i32), OpenToolMenu>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Opens the worn piece's inventory screen for the player (InventoryMenuModule.startInteract):
/// the smallest container menu type that holds the tool's inventory, titled with the piece's
/// name. True when the menu opened; false for a piece that is not a built tool, or a tool with
/// more than 108 inventory slots.
pub(crate) fn open_tool_menu(player: &ServerPlayer, piece: &StackData, worn: i32) -> bool {
    let Ok(Some((inventory, items))) = with_held_tool(piece, |env, tool| {
        let inventory = tool_stack::inventory(&tool.custom_data, env);
        let mut data = tool.custom_data.clone();
        let items = tool_stack::with_persistent(&mut data, |persistent| {
            (0..inventory.slots()).map(|slot| inventory.stack(persistent, slot)).collect::<Vec<_>>()
        });
        Some((inventory, items))
    }) else { return false; };
    open_menu_for(player, piece, inventory, items, worn)
}

/// Opens the menu for a piece whose inventory is already derived (the testable core of
/// [`open_tool_menu`]): the items padded to the menu type's full slot count, the title from the
/// piece's item name, the extra data marking an item menu and naming the worn slot.
fn open_menu_for(player: &ServerPlayer, piece: &StackData, inventory: ToolInventory, items: Vec<Option<ItemSlot>>, worn: i32) -> bool {
    let Some(kind) = menu_kind_for(inventory.slots()) else { return false; };
    let slots = MENU_KINDS.iter().position(|name| *name == kind).map_or(0, |row| (row + 1) * 9);
    let mut padded = items;
    padded.resize(slots, None);
    let title = Identifier::parse(&piece.item).map_or_else(
        |_| piece.item.clone(),
        |item| I18n::translate_in(&player.language(), &format!("item.{}.{}", item.namespace(), item.path())),
    );
    let mut extra = ITEM_MENU_MAGIC.to_vec();
    extra.push(worn as u8);
    let Some(window) = player.open_item_menu(kind, &title, padded, extra) else { return false; };
    if let Ok(mut open) = OPEN.lock() {
        open.insert((player.uuid, window), OpenToolMenu { worn, item: piece.item.clone(), inventory });
    }
    true
}

/// Writes back what the player left in a tool container menu (the input-closed event, which the
/// host fires before the close event for the same window): the kept stacks go into the worn
/// piece's inventory — only the slots the tool actually has, and only stacks its inventory
/// takes — and the piece is written back whole. A stack the tool does not take, or a slot past
/// its inventory, stays in the menu's slots and comes back to the player with the close.
fn write_back(event: &NativeMenuInputClosed) {
    let Some((worn, item, inventory)) = OPEN.lock().ok().and_then(|open| open.get(&(event.player, event.window)).map(|open| (open.worn, open.item.clone(), open.inventory.clone()))) else { return; };
    let wearer = ServerPlayer { uuid: event.player, name: String::new() };
    let Some(mut piece) = wearer.stack_named(worn) else { return; };
    if piece.item != item { return; }
    let tool_slots = inventory.slots();
    let mut slots = event.slots.borrow_mut();
    with_snapshot_items(|items| {
        tool_stack::with_persistent(&mut piece.custom_data, |persistent| {
            for (index, kept) in slots.iter_mut().enumerate() {
                if index >= tool_slots { continue; }
                let Some(stack) = kept.as_ref() else { continue; };
                if !inventory.is_item_valid(items, index, stack) { continue; }
                let taken = kept.take();
                inventory.set_stack(persistent, index, taken.as_ref());
            }
        });
    });
    let _ = wearer.set_stack(worn, Some(&piece));
}

/// Forgets a tool container menu that closed, whatever closed it.
fn forget(event: &NativeMenuClosed) {
    if let Ok(mut open) = OPEN.lock() {
        open.remove(&(event.player, event.window));
    }
}

/// Registers the tool container menu types, their windows and the close write-back. Called once
/// from the product's hooks.
pub(crate) fn register() {
    static EVENTS: Once = Once::new();
    EVENTS.call_once(|| {
        NativeMenuEvents::INPUT_CLOSED.register(|event| write_back(event));
        NativeMenuEvents::CLOSED.register(|event| forget(event));
    });
    for (row, kind) in MENU_KINDS.iter().enumerate() {
        let slots = (row + 1) * 9;
        let id = Identifier::parse(kind).expect("valid tool container id");
        let mut layout = ScreenLayout::new(176.0, 18.0 + (row + 1) as f32 * 18.0 + 96.0).inventory_at(8.0, 18.0 + (row + 1) as f32 * 18.0 + 13.0).with_own_background();
        for slot in 0..slots {
            layout = layout.slot(8.0 + (slot % 9) as f32 * 18.0, 18.0 + (slot / 9) as f32 * 18.0);
        }
        ScreenHandlerRegistry::register(ScreenHandlerType::new(id.clone(), layout)
            .on_draw(move |draw| draw_tool_background(draw.canvas, draw.origin, row + 1)));
        BlockEntityTypeRegistry::register(BlockEntityType::new(id).item_slots(slots as u32).menu(kind));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{modifier_entries::Entry, tool_inventory::{Inventory, Part}};
    use pluto_lib::player_api::{with_ops, MemoryPlayerOps};
    use pluto_lib::{Nbt, NbtCompound};
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn stack(item: &str, count: u32) -> ItemSlot { ItemSlot::new(id(item), count) }

    fn pockets() -> ToolInventory {
        let module = Inventory::decode(json!({"slots": {"flat": 3}}).as_object().unwrap()).unwrap().unwrap();
        ToolInventory::new(vec![Part::new(module, Entry::new(id("tconstruct:pocket"), 1), true, true)])
    }

    fn piece_with(inventory: &ToolInventory, stacks: &[Option<ItemSlot>]) -> StackData {
        let mut custom_data = NbtCompound::new();
        tool_stack::with_persistent(&mut custom_data, |persistent| {
            for (slot, stack) in stacks.iter().enumerate() {
                inventory.set_stack(persistent, slot, stack.as_ref());
            }
        });
        StackData { item: "tconstruct:helmet".into(), count: 1, custom_data, ..StackData::default() }
    }

    #[test]
    fn the_menu_type_is_the_smallest_that_holds_the_inventory() {
        assert_eq!(menu_kind_for(0), Some("tconstruct:tool_container_1"));
        assert_eq!(menu_kind_for(9), Some("tconstruct:tool_container_1"));
        assert_eq!(menu_kind_for(10), Some("tconstruct:tool_container_2"));
        assert_eq!(menu_kind_for(54), Some("tconstruct:tool_container_6"));
        assert_eq!(menu_kind_for(55), Some("tconstruct:tool_container_7"), "past six rows opens too (issue #1159)");
        assert_eq!(menu_kind_for(108), Some("tconstruct:tool_container_12"));
        assert_eq!(menu_kind_for(109), None, "past twelve rows the menu cannot open");
    }

    #[derive(Default)]
    struct Recorder { sprites: Vec<(String, [f32; 4], [f32; 4])> }
    impl ScreenCanvas for Recorder {
        fn fill(&mut self, _: [f32; 4], _: u32) {}
        fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4]) -> bool { self.sprites.push((name.to_owned(), src, dst)); true }
        fn sprite_tinted(&mut self, _: &str, _: [f32; 4], _: [f32; 2], _: [f32; 4], _: u32) -> bool { true }
        fn text(&mut self, _: &str, _: [f32; 2], _: u32, _: bool) {}
        fn text_width(&self, _: &str) -> f32 { 0.0 }
    }

    #[test]
    fn the_tool_background_stretches_one_segment_for_small_inventories_and_repeats_for_large_ones() {
        let mut canvas = Recorder::default();
        draw_tool_background(&mut canvas, [10.0, 20.0], 1);
        assert_eq!(canvas.sprites, vec![
            (TOOL_INVENTORY.to_owned(), [0.0, 0.0, 176.0, 35.0], [10.0, 20.0, 176.0, 35.0]),
            (TOOL_INVENTORY.to_owned(), [0.0, 125.0, 176.0, 96.0], [10.0, 55.0, 176.0, 97.0]),
            (TOOL_INVENTORY.to_owned(), [0.0, 238.0, 162.0, 18.0], [17.0, 37.0, 162.0, 18.0]),
        ], "one row: a single top segment, the player segment, one slot strip");
        let mut canvas = Recorder::default();
        draw_tool_background(&mut canvas, [0.0, 0.0], 12);
        assert_eq!(canvas.sprites.len(), 15, "twelve rows: top segment + one repeated chunk + player segment + twelve strips");
        assert_eq!(canvas.sprites[1], (TOOL_INVENTORY.to_owned(), [0.0, 17.0, 176.0, 108.0], [0.0, 125.0, 176.0, 108.0]));
        assert_eq!(canvas.sprites[2].2, [0.0, 233.0, 176.0, 97.0], "the player segment follows the repeated chunk");
    }

    #[test]
    fn opening_a_tool_menu_records_its_kind_items_and_worn_slot() {
        register();
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 1001, name: "Alex".into() };
        let inventory = pockets();
        let piece = piece_with(&inventory, &[Some(stack("minecraft:torch", 5)), None, Some(stack("minecraft:apple", 2))]);
        let opened = with_ops(&mut ops, || open_menu_for(&player, &piece, inventory, vec![Some(stack("minecraft:torch", 5)), None, Some(stack("minecraft:apple", 2))], 103));
        assert!(opened);
        assert_eq!(ops.opened_item_menus.len(), 1);
        let menu = &ops.opened_item_menus[0];
        assert_eq!((menu.player, menu.menu.as_str()), (1001, "tconstruct:tool_container_1"));
        assert_eq!(menu.items.len(), 9, "padded to the type's full slot count");
        assert_eq!(menu.items[0], Some(stack("minecraft:torch", 5)));
        assert_eq!(menu.items[2], Some(stack("minecraft:apple", 2)));
        assert_eq!(menu.extra, [b'P', b'L', b'U', b'T', b'O', 0x01, 103], "the magic, then the worn slot");
        // A second menu for the same player takes the next window, and closing forgets the first.
        let inventory = pockets();
        let piece = piece_with(&inventory, &[]);
        assert!(with_ops(&mut ops, || open_menu_for(&player, &piece, inventory, vec![], 101)));
        assert_eq!(ops.opened_item_menus.len(), 2);
        forget(&NativeMenuClosed { player: 1001, window: 1 });
        forget(&NativeMenuClosed { player: 1001, window: 2 });
    }

    #[test]
    fn closing_writes_back_only_what_the_tool_takes() {
        register();
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 1002, name: "Alex".into() };
        let inventory = pockets();
        let piece = piece_with(&inventory, &[Some(stack("minecraft:torch", 5)), None, None]);
        // The worn piece is what the close reads back: seed it where the named read looks.
        ops.carried.insert((1002, 103), piece.clone());
        assert!(with_ops(&mut ops, || open_menu_for(&player, &piece, inventory, vec![Some(stack("minecraft:torch", 5)), None, None], 103)));
        // The player swaps the torch for an apple, leaves a shulker box in a real slot (the tool
        // does not take it) and a stack in a padding slot past the tool's three.
        let event = NativeMenuInputClosed { player: 1002, window: 1, slots: std::cell::RefCell::new(vec![Some(stack("minecraft:apple", 2)), Some(stack("minecraft:shulker_box", 1)), None, Some(stack("minecraft:dirt", 4)), None, None, None, None, None]) };
        with_ops(&mut ops, || write_back(&event));
        let slots = event.slots.borrow();
        assert_eq!(slots[0], None, "the taken apple left the menu");
        assert_eq!(slots[1], Some(stack("minecraft:shulker_box", 1)), "a blacklisted stack stays for the player");
        assert_eq!(slots[3], Some(stack("minecraft:dirt", 4)), "a padding slot's stack stays too");
        drop(slots);
        let written = ops.carried.get(&(1002, 103)).expect("the piece was written back");
        let after = with_held_tool_free(&written);
        assert_eq!(after, Some(stack("minecraft:apple", 2)), "the apple is in the tool's first slot");
        forget(&NativeMenuClosed { player: 1002, window: 1 });
        // A close after the forget takes nothing more.
        let again = NativeMenuInputClosed { player: 1002, window: 1, slots: std::cell::RefCell::new(vec![Some(stack("minecraft:gold_ingot", 1))]) };
        with_ops(&mut ops, || write_back(&again));
        assert_eq!(again.slots.borrow()[0], Some(stack("minecraft:gold_ingot", 1)));
    }

    /// The first stack of the piece's inventory, read without a tool registry: the piece's
    /// persistent data is plain NBT, so the slot's compound is read straight from it.
    fn with_held_tool_free(piece: &StackData) -> Option<ItemSlot> {
        let persistent = match piece.custom_data.get(tool_stack::TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => return None };
        let inventory = pockets();
        inventory.stack(&persistent, 0)
    }

    #[test]
    fn closing_after_the_piece_was_swapped_takes_nothing() {
        register();
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 1003, name: "Alex".into() };
        let inventory = pockets();
        let piece = piece_with(&inventory, &[Some(stack("minecraft:torch", 5)), None, None]);
        ops.carried.insert((1003, 103), piece.clone());
        assert!(with_ops(&mut ops, || open_menu_for(&player, &piece, inventory, vec![Some(stack("minecraft:torch", 5)), None, None], 103)));
        // A different piece is worn now: the kept stacks stay for the player.
        ops.carried.insert((1003, 103), StackData { item: "minecraft:diamond_helmet".into(), count: 1, custom_data: NbtCompound::new(), ..StackData::default() });
        let event = NativeMenuInputClosed { player: 1003, window: 1, slots: std::cell::RefCell::new(vec![Some(stack("minecraft:apple", 2)), None, None, None, None, None, None, None, None]) };
        with_ops(&mut ops, || write_back(&event));
        assert_eq!(event.slots.borrow()[0], Some(stack("minecraft:apple", 2)));
        forget(&NativeMenuClosed { player: 1003, window: 1 });
    }
}
