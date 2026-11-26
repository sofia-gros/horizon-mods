//! The three Tinkers chests (AbstractChestBlockEntity and its item handlers, SlimeKnights, MIT; see
//! LICENSE): the tinkers chest holds 64 slots of 16 of anything; the part chest and the cast chest
//! are scaling chests — 256 slots behind a display size that grows as they fill, one kind of item
//! per slot (parts: items of `tconstruct:chest_parts`, 8 per slot; casts: items of `tconstruct:casts`,
//! 4 per slot). Right-clicking a scaling chest with a valid item deposits it (ChestBlock.use with
//! canInsert); the tinkers chest always opens its window instead (its canInsert is false). All
//! three chests share the one tabbed menu in the original, so their windows carry the
//! neighbouring tables' tabs (see `tabbed_tables`).
//!
//! What does not carry over, and why:
//! - The one-kind-per-slot rule spans slots; PlutoLib's slot rule sees one stack at a time, so tag
//!   validity and the per-slot limit are enforced on every placement, but a second slot of an item
//!   the chest already holds is not refused. Hopper insertion (on_accepts) has the same limit.
//! - The tinkers chest's dye colour is seeded from the placed stack's display tag at placement
//!   (TinkersChestBlockItem.updateCustomBlockEntityTag, through the placement-data hook); no
//!   renderer tints it here in any case. A broken cast chest drops its contents as items rather
//!   than keeping them in the dropped chest's data (the original's TinkerData copy).
//! - The window scrolls its slots a page at a time through the slider (the row offset is the
//!   block entity's, published as menu data, and the dynamic slots follow it). The mouse wheel
//!   cannot scroll it: PlutoLib's wheel callback is a client-side hearing with no path to the
//!   server state the slot positions resolve from, so the wheel is left to the game.
use crate::common;
use pluto_lib::{
    ActionResult, BlockEntityType, BlockEntityTypeRegistry, Identifier, ItemSlot, Nbt, NbtCompound,
    ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout,
};

/// The visible grid: eight columns of four rows (the original's scaling module is 162 wide
/// and 72 high: `ScalingChestScreen` over `DynamicContainerScreen`'s defaults plus one row).
const COLUMNS: usize = 8;
const ROWS: usize = 4;
const PAGE: usize = COLUMNS * ROWS;
/// The scaling chests' underlying size (`ScalingChestItemHandler.DEFAULT_MAX`).
const SCALING_SLOTS: usize = 256;
/// The tinkers chest's size and per-slot limit (TinkersChestItemHandler).
const TINKERS_SLOTS: usize = 64;
/// The saved scroll offset, in rows.
const ROW_KEY: &str = "row";
/// The slider: its track's left, top, width and height (right of the grid, where
/// `DynamicContainerScreen.updateSlider` puts it: the module's right edge, its full height).
const SLIDER: (f32, f32, f32, f32) = (157.0, 17.0, 12.0, 72.0);
/// The slider handle's height (the generic sheet's slider element is a fixed 10x15).
const HANDLE_H: f32 = 15.0;
/// The window's background (`TinkerChestScreen` draws BLANK_BACK_PLUS_1) and the module sheet.
const BACKGROUND: &str = "tconstruct:gui/blank_extra_row";
/// The icons sheet, as the station windows name it (the tab strip's texture).
const ICONS: &str = "tconstruct:gui/icons";
/// The menu-data base of the tabbed tables' words: the chest's own words are 0 (the display
/// size) and 1 (the scroll row), so the strip's follow at 2 (see `tabbed_tables`).
const TABBED_AT: usize = 2;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Tinkers,
    Parts,
    Casts,
}

impl Kind {
    fn slots(self) -> usize { match self { Kind::Tinkers => TINKERS_SLOTS, _ => SCALING_SLOTS } }
    /// The per-slot limit of the chest's item handler.
    fn limit(self) -> u32 { match self { Kind::Tinkers => 16, Kind::Parts => 8, Kind::Casts => 4 } }
    /// The tag an item must be in to belong in the chest, if any.
    fn tag(self) -> Option<Identifier> {
        match self {
            Kind::Tinkers => None,
            Kind::Parts => Some(Identifier::new("tconstruct", "chest_parts").expect("valid tag")),
            Kind::Casts => Some(Identifier::new("tconstruct", "casts").expect("valid tag")),
        }
    }
    /// Whether the chest takes the item at all (its handler's validity, without the cross-slot rule).
    fn accepts(self, item: &Identifier) -> bool {
        match self.tag() {
            None => true,
            Some(tag) => common::item_snapshot().as_deref()
                .and_then(|tags| tags.get_tag(&tag))
                .is_some_and(|members| members.contains(item)),
        }
    }
}

/// The display size of a scaling chest (`ScalingChestItemHandler`'s visual size): one past the last
/// filled slot plus one empty row slot to grow into, at least 1; the tinkers chest shows all its slots.
fn visual_size(kind: Kind, items: &[Option<ItemSlot>]) -> usize {
    if kind == Kind::Tinkers { return TINKERS_SLOTS; }
    let last_filled = items.iter().rposition(|slot| slot.as_ref().is_some_and(|stack| stack.count > 0));
    match last_filled {
        None => 1,
        Some(index) => (index + 2).min(kind.slots()),
    }
}

fn row_of(data: &NbtCompound) -> usize {
    match data.get(ROW_KEY) { Some(Nbt::Int(row)) => (*row).max(0) as usize, _ => 0 }
}

/// How many rows the display size fills, and how far it can scroll.
fn rows_total(visual: usize) -> usize { visual.div_ceil(COLUMNS) }
fn max_row(visual: usize) -> usize { rows_total(visual).saturating_sub(ROWS) }

/// The slider handle's height and top for a display size and scroll offset: the handle is the
/// sheet's fixed-size element, travelling the track by the scroll's share of its range.
fn handle(visual: usize, row: usize) -> (f32, f32) {
    let travel = SLIDER.3 - HANDLE_H;
    let progress = if max_row(visual) > 0 { row.min(max_row(visual)) as f32 / max_row(visual) as f32 } else { 0.0 };
    (HANDLE_H, SLIDER.1 + travel * progress)
}

/// Where a slot sits on the window under the current scroll and display size; `None` hides it.
fn slot_position(slot: usize, visual: usize, row: usize) -> Option<[f32; 2]> {
    if slot >= visual { return None; }
    let within = slot.checked_sub(row * COLUMNS)?;
    if within >= PAGE { return None; }
    Some([8.0 + (within % COLUMNS) as f32 * 18.0, 18.0 + (within / COLUMNS) as f32 * 18.0])
}

/// Deposits as much of `held` as the chest takes (ChestBlock.use through ItemHandlerHelper.insertItem):
/// first the slot already holding the item (the one slot of its kind), then the empty slots in order,
/// each up to the chest's limit. Returns what remains in hand. The caller has checked the item is one
/// the chest takes (the handler's validity); this is only the stacking.
fn deposit(kind: Kind, items: &mut [Option<ItemSlot>], held: ItemSlot) -> ItemSlot {
    let mut rest = held;
    let limit = kind.limit();
    let same = items.iter().position(|slot| slot.as_ref().is_some_and(|stack| stack.item == rest.item && stack.custom_data == rest.custom_data && stack.enchantments == rest.enchantments));
    // Whether some slot already holds this kind: an empty slot may take it only when none does, and
    // filling one empty slot makes the kind present for the rest of the deposit.
    let mut present = same.is_some();
    let order = same.into_iter().chain((0..items.len()).filter(move |index| Some(*index) != same));
    for index in order {
        if rest.count == 0 { break; }
        match &mut items[index] {
            Some(stack) => {
                if stack.item != rest.item || stack.custom_data != rest.custom_data || stack.enchantments != rest.enchantments { continue; }
                let room = limit.saturating_sub(stack.count);
                let moved = room.min(rest.count);
                stack.count += moved;
                rest.count -= moved;
            }
            slot @ None => {
                if present { continue; }
                let moved = limit.min(rest.count);
                if moved == 0 { continue; }
                let mut stack = rest.clone();
                stack.count = moved;
                *slot = Some(stack);
                present = true;
                rest.count -= moved;
            }
        }
    }
    rest
}

/// Registers one chest's block entity type and window. Called once from the product's hooks.
fn register_chest(name: &'static str, kind: Kind) {
    let id = Identifier::new("tconstruct", name).expect("valid chest id");
    let slots = kind.slots();
    let mut layout = ScreenLayout::new(176.0, 184.0).inventory_at(8.0, 102.0).with_own_background();
    // The whole grid is in the menu; the current page positions it.
    for _ in 0..slots { layout = layout.dynamic_slot(); }
    let window = ScreenHandlerType::new(id.clone(), layout)
        .on_slot_rule(move |rule| {
            rule.max_stack = kind.limit();
            if let Some(tag) = kind.tag() {
                let item = Identifier::parse(rule.item).ok();
                rule.may_place = item.as_ref().is_some_and(|item| {
                    common::item_snapshot().as_deref().and_then(|tags| tags.get_tag(&tag)).is_some_and(|members| members.contains(item))
                });
            }
        })
        .on_accepts(move |accept| kind.tag().is_none_or(|_| {
            Identifier::parse(accept.item).is_ok_and(|item| kind.accepts(&item))
        }))
        .on_dynamic_slots(move |state| {
            let visual = state.data.first().copied().unwrap_or(0).max(0) as usize;
            let row = state.data.get(1).copied().unwrap_or(0).max(0) as usize;
            Some((0..slots).map(|slot| slot_position(slot, visual, row)).collect())
        })
        .on_draw(|draw| {
            let (ox, oy) = (draw.origin[0], draw.origin[1]);
            // The window's own background, the original's blank sheet with its extra row.
            draw.canvas.sprite(BACKGROUND, [0.0, 0.0, 176.0, 184.0], [ox, oy, 176.0, 184.0]);
            let visual = draw.data.first().copied().unwrap_or(0).max(0) as usize;
            let row = draw.data.get(1).copied().unwrap_or(0).max(0) as usize;
            // The module's slot frames: a filled frame per shown slot, the empty frame for
            // the rest of a partial last row (`DynamicContainerScreen.renderBg`).
            let shown = visual.saturating_sub(row * COLUMNS).min(PAGE);
            let (full_rows, partial) = (shown / COLUMNS, shown % COLUMNS);
            for r in 0..ROWS {
                let cells = if r < full_rows { COLUMNS } else if r == full_rows { partial } else { break };
                for c in 0..COLUMNS {
                    let src = if c < cells { [7.0, 7.0, 18.0, 18.0] } else if r == full_rows && partial > 0 { [25.0, 7.0, 18.0, 18.0] } else { continue };
                    draw.canvas.sprite(crate::station_menu::GENERIC, src, [ox + 7.0 + c as f32 * 18.0, oy + 17.0 + r as f32 * 18.0, 18.0, 18.0]);
                }
            }
            // The slider, when the display size scrolls: the generic sheet's caps, its scaled
            // track and the fixed handle element (`GenericScreen`'s slider pieces).
            if max_row(visual) > 0 {
                let (height, top) = handle(visual, row);
                let (x, y) = (ox + SLIDER.0, oy + SLIDER.1);
                draw.canvas.sprite(crate::station_menu::GENERIC, [43.0, 7.0, 12.0, 1.0], [x, y, 12.0, 1.0]);
                draw.canvas.sprite(crate::station_menu::GENERIC, [43.0, 8.0, 12.0, 30.0], [x, y + 1.0, 12.0, SLIDER.3 - 2.0]);
                draw.canvas.sprite(crate::station_menu::GENERIC, [43.0, 38.0, 12.0, 1.0], [x, y + SLIDER.3 - 1.0, 12.0, 1.0]);
                draw.canvas.sprite(crate::station_menu::GENERIC, [7.0, 25.0, 10.0, 15.0], [x + 1.0, oy + top, 10.0, height]);
            }
            // The neighbouring tables' tabs above the window (see `tabbed_tables`).
            crate::tabbed_tables::draw_tabs(draw.canvas, draw.origin, draw.data, TABBED_AT, ICONS);
        })
        .on_click(|press| {
            if !press.right {
                if let Some(id) = crate::tabbed_tables::tab_click(press.at, press.data, TABBED_AT) { return Some(id); }
            }
            let visual = press.data.first().copied().unwrap_or(0).max(0) as usize;
            if press.right || max_row(visual) == 0 { return None; }
            if press.at[0] < SLIDER.0 || press.at[0] >= SLIDER.0 + SLIDER.2 || press.at[1] < SLIDER.1 || press.at[1] >= SLIDER.1 + SLIDER.3 { return None; }
            let row = press.data.get(1).copied().unwrap_or(0).max(0) as usize;
            let (height, top) = handle(visual, row);
            if press.at[1] < top { Some(1) } else if press.at[1] >= top + height { Some(2) } else { None }
        })
        .on_tooltip(|tip| {
            if let Some(name) = crate::tabbed_tables::tab_tooltip(tip.at, tip.data, TABBED_AT) {
                tip.lines.push(name);
            }
        })
        .on_button(move |press| {
            if crate::tabbed_tables::press_tab(press) { return; }
            let visual = visual_size(kind, press.items);
            let row = row_of(press.data);
            let next = match press.id { 1 => row.saturating_sub(ROWS), 2 => (row + ROWS).min(max_row(visual)), _ => return };
            press.data.insert(ROW_KEY.to_owned(), Nbt::Int(next as i32));
        });
    ScreenHandlerRegistry::register(window);
    let mut entity = BlockEntityType::new(id.clone()).item_slots(slots as u32).menu(&format!("tconstruct:{name}"))
        .ticker(move |tick| {
            let visual = visual_size(kind, tick.items);
            let row = row_of(tick.data).min(max_row(visual));
            if row_of(tick.data) != row { tick.data.insert(ROW_KEY.to_owned(), Nbt::Int(row as i32)); }
            tick.set_menu_data(0, visual as i32);
            tick.set_menu_data(1, row as i32);
            // The neighbouring tables' strip (tabs only; a chest shows no side inventory).
            crate::tabbed_tables::publish(tick, TABBED_AT, false);
        });
    if kind == Kind::Tinkers {
        // TinkersChestBlockItem.updateCustomBlockEntityTag: a chest item carrying a dye colour in its
        // display tag seeds the placed block entity's colour (TinkersChestBlockEntity's "color"); an
        // undyed item leaves the default colour unset, as the original leaves it unsaved.
        entity = entity.on_place(|place| {
            let Some(Nbt::Compound(display)) = place.stack.custom_data.get("display") else { return; };
            if display.contains_key("color") {
                let color = crate::modifier_entries::nbt_int(display.get("color"));
                place.data.insert("color".to_owned(), Nbt::Int(color));
            }
        });
    }
    if kind != Kind::Tinkers {
        // ChestBlock.use: a held item the chest takes is deposited instead of opening the window.
        entity = entity.on_use(move |used| {
            let Some(held) = used.held.take() else { return ActionResult::Pass };
            if used.sneaking || !kind.accepts(&held.item) { used.held = Some(held); return ActionResult::Pass; }
            let before = held.count;
            let rest = deposit(kind, used.items, held);
            if rest.count == before { used.held = Some(rest); return ActionResult::Pass; }
            used.held = (rest.count > 0).then_some(rest);
            ActionResult::Success
        });
    }
    BlockEntityTypeRegistry::register(entity);
}

/// Registers the three chests. Called once from the product's hooks.
pub(crate) fn register() {
    register_chest("tinkers_chest", Kind::Tinkers);
    register_chest("part_chest", Kind::Parts);
    register_chest("cast_chest", Kind::Casts);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: &str, count: u32) -> ItemSlot { ItemSlot::new(Identifier::parse(item).unwrap(), count) }

    #[test]
    fn the_scaling_display_size_trails_the_last_filled_slot_by_one() {
        let mut items: Vec<Option<ItemSlot>> = vec![None; SCALING_SLOTS];
        assert_eq!(visual_size(Kind::Parts, &items), 1, "an empty chest shows a single slot");
        items[0] = Some(stack("tconstruct:pick_head", 3));
        assert_eq!(visual_size(Kind::Parts, &items), 2);
        items[5] = Some(stack("tconstruct:pick_head", 1));
        assert_eq!(visual_size(Kind::Parts, &items), 7, "one past the last filled slot, plus the growth slot");
        items[255] = Some(stack("tconstruct:pick_head", 1));
        assert_eq!(visual_size(Kind::Parts, &items), 256, "capped at the chest's size");
        assert_eq!(visual_size(Kind::Tinkers, &vec![None; TINKERS_SLOTS]), 64, "the tinkers chest always shows all of its slots");
    }

    #[test]
    fn slots_scroll_in_rows_of_eight_and_hide_outside_the_display_size() {
        assert_eq!(slot_position(0, 1, 0), Some([8.0, 18.0]));
        assert_eq!(slot_position(1, 1, 0), None, "past the display size");
        assert_eq!(slot_position(31, 64, 0), Some([8.0 + 7.0 * 18.0, 18.0 + 3.0 * 18.0]), "the page's last slot");
        assert_eq!(slot_position(32, 64, 0), None, "below the visible rows");
        assert_eq!(slot_position(32, 100, 1), Some([8.0, 18.0 + 3.0 * 18.0]), "one row of scroll brings slot 32 to the last visible row");
        assert_eq!(slot_position(0, 100, 1), None, "scrolled off the top");
        assert_eq!(rows_total(64), 8);
        assert_eq!(max_row(32), 0, "four rows fit without scrolling");
        assert_eq!(max_row(64), 4);
        assert_eq!(max_row(65), 5);
        assert_eq!(max_row(256), 28);
        let (height, top) = handle(256, 28);
        assert_eq!(height, HANDLE_H, "the handle is the sheet's fixed-size element");
        assert!(top + height <= SLIDER.1 + SLIDER.3 + 0.01, "the handle stays in its track at the bottom");
    }

    #[test]
    fn a_deposit_fills_the_matching_slot_then_one_empty_slot_up_to_the_limit() {
        let mut items: Vec<Option<ItemSlot>> = vec![None; SCALING_SLOTS];
        items[3] = Some(stack("tconstruct:pick_head", 6));
        // Six in the slot and ten in hand: two top the slot up to the part limit of 8, the rest stays in hand
        // (no second slot of the same kind is started).
        let rest = deposit(Kind::Parts, &mut items, stack("tconstruct:pick_head", 10));
        assert_eq!(items[3].as_ref().map(|stack| stack.count), Some(8));
        assert_eq!(rest.count, 8);
        // A new kind takes the first empty slot, up to the limit.
        let rest = deposit(Kind::Parts, &mut items, stack("tconstruct:tool_handle", 12));
        assert_eq!(items[0].as_ref().map(|stack| stack.count), Some(8));
        assert_eq!(rest.count, 4);
    }
}
