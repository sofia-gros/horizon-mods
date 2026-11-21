//! The three Tinkers chests (AbstractChestBlockEntity and its item handlers, SlimeKnights, MIT; see
//! LICENSE): the tinkers chest holds 64 slots of 16 of anything; the part chest and the cast chest
//! are scaling chests — 256 slots behind a display size that grows as they fill, one kind of item
//! per slot (parts: items of `tconstruct:chest_parts`, 8 per slot; casts: items of `tconstruct:casts`,
//! 4 per slot). Right-clicking a scaling chest with a valid item deposits it (ChestBlock.use with
//! canInsert); the tinkers chest always opens its window instead (its canInsert is false).
//!
//! What does not carry over, and why:
//! - The one-kind-per-slot rule spans slots; PlutoLib's slot rule sees one stack at a time, so tag
//!   validity and the per-slot limit are enforced on every placement, but a second slot of an item
//!   the chest already holds is not refused. Hopper insertion (on_accepts) has the same limit.
//! - The tinkers chest's dye colour is set by its block item at placement; there is no placement-data
//!   hook in PlutoLib's public API, so a placed chest keeps the default colour, and no renderer tints
//!   it here in any case. For the same reason a broken cast chest drops its contents as items rather
//!   than keeping them in the dropped chest's data (the original's TinkerData copy).
//! - The window pages its slots (64 to a page); the original's chest screen scrolls instead.
use crate::common;
use pluto_lib::{
    ActionResult, BlockEntityType, BlockEntityTypeRegistry, Identifier, ItemSlot, Nbt, NbtCompound,
    ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout,
};

/// Slots shown per window page, in eight columns.
const COLUMNS: usize = 8;
const ROWS: usize = 8;
const PAGE: usize = COLUMNS * ROWS;
/// The scaling chests' underlying size (`ScalingChestItemHandler.DEFAULT_MAX`).
const SCALING_SLOTS: usize = 256;
/// The tinkers chest's size and per-slot limit (TinkersChestItemHandler).
const TINKERS_SLOTS: usize = 64;
/// The saved window page.
const PAGE_KEY: &str = "page";
/// Page buttons on the window: previous and next.
const BUTTONS: (f32, f32, f32) = (136.0, 2.0, 18.0);

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

fn page_of(data: &NbtCompound) -> usize {
    match data.get(PAGE_KEY) { Some(Nbt::Int(page)) => (*page).max(0) as usize, _ => 0 }
}

fn pages(visual: usize) -> usize { visual.div_ceil(PAGE).max(1) }

/// Where a slot sits on the window under the current page and display size; `None` hides it.
fn slot_position(slot: usize, visual: usize, page: usize) -> Option<[f32; 2]> {
    if slot >= visual || slot / PAGE != page { return None; }
    let within = slot % PAGE;
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
    let mut layout = ScreenLayout::new(176.0, 252.0).inventory_at(8.0, 170.0);
    for _ in 0..slots { layout = layout.hidden_slot(); }
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
            let page = state.data.get(1).copied().unwrap_or(0).max(0) as usize;
            Some((0..slots).map(|slot| slot_position(slot, visual, page)).collect())
        })
        .on_draw(|draw| {
            let visual = draw.data.first().copied().unwrap_or(0).max(0) as usize;
            if pages(visual) <= 1 { return; }
            for (index, label) in ["<", ">"].into_iter().enumerate() {
                let x = draw.origin[0] + BUTTONS.0 + index as f32 * BUTTONS.2;
                let y = draw.origin[1] + BUTTONS.1;
                draw.canvas.fill([x, y, BUTTONS.2 - 2.0, 12.0], 0xFF8B8B8B);
                draw.canvas.text(label, [x + 5.0, y + 2.0], 0x202020, false);
            }
        })
        .on_click(|press| {
            let visual = press.data.first().copied().unwrap_or(0).max(0) as usize;
            if press.right || pages(visual) <= 1 { return None; }
            let (x, y) = (press.at[0] - BUTTONS.0, press.at[1] - BUTTONS.1);
            if y < 0.0 || y > 12.0 || x < 0.0 { return None; }
            let index = (x / BUTTONS.2) as usize;
            (index <= 1 && x < BUTTONS.2 * 2.0 - 2.0).then_some(index as i32 + 1)
        })
        .on_button(move |press| {
            let visual = visual_size(kind, press.items);
            let last = pages(visual) - 1;
            let page = page_of(press.data);
            let next = match press.id { 1 => page.saturating_sub(1), 2 => (page + 1).min(last), _ => return };
            press.data.insert(PAGE_KEY.to_owned(), Nbt::Int(next as i32));
        });
    ScreenHandlerRegistry::register(window);
    let mut entity = BlockEntityType::new(id.clone()).item_slots(slots as u32).menu(&format!("tconstruct:{name}"))
        .ticker(move |tick| {
            let visual = visual_size(kind, tick.items);
            let page = page_of(tick.data).min(pages(visual) - 1);
            if page_of(tick.data) != page { tick.data.insert(PAGE_KEY.to_owned(), Nbt::Int(page as i32)); }
            tick.set_menu_data(0, visual as i32);
            tick.set_menu_data(1, page as i32);
        });
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
    fn slots_page_in_eights_and_hide_outside_the_display_size() {
        assert_eq!(slot_position(0, 1, 0), Some([8.0, 18.0]));
        assert_eq!(slot_position(1, 1, 0), None, "past the display size");
        assert_eq!(slot_position(63, 64, 0), Some([8.0 + 7.0 * 18.0, 18.0 + 7.0 * 18.0]));
        assert_eq!(slot_position(64, 100, 0), None, "on the next page");
        assert_eq!(slot_position(64, 100, 1), Some([8.0, 18.0]));
        assert_eq!(pages(64), 1);
        assert_eq!(pages(65), 2);
        assert_eq!(pages(256), 4);
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
