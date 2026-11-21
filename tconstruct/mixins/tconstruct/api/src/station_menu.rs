//! The tinker station family's block entities, windows and result-slot ownership over PlutoLib's
//! public API: the tinker station (four inputs) and the two anvils (six inputs), which share the
//! station crafting of `tables::station`. SlimeKnights (MIT; see LICENSE) defines the recipes and
//! the slot layouts; this registers where they apply.
//!
//! Slot layouts (station_layouts) drive the window: the selected layout — a station's main layout
//! until a tab is picked — positions the tool and input slots through the dynamic-slot callback,
//! with the ticker publishing the active layout and its positions as menu data (slot 0 the active
//! tab, then an x,y pair per slot, -1 for a slot the layout leaves out). The layout tabs sit in a
//! sidebar left of the window, as the original's screen draws them.
//!
//! What does not carry over, and why:
//! - A layout's slot filters cannot be enforced: PlutoLib's slot rule sees one stack at a time with
//!   no layout context. A wrong item in a slot simply crafts nothing, where the original refuses it.
//! - A slot a layout leaves out is hidden here; the original keeps it dormant (items can be taken
//!   out but not put in). Switching back to a layout that shows the slot reaches its contents again.
//! - The original's window also carries the player's armor and offhand slots and the neighbouring
//!   chests as side inventories; a menu here can reach neither the player's equipment slots nor a
//!   neighbour's items.
//! - The station texture and the layouts' ghost slot icons are presentation the canvas cannot do
//!   (it fills and writes text; it cannot draw item icons), so tabs are numbered squares, as the
//!   part builder's pattern buttons are.
use crate::{
    common, station_layout_order::TAB_ORDER, station_layouts::{self, Layout},
    tables::{lazy_result::LazyResultContainer, station::{self, Resolver}},
    tool_definitions, tool_parts,
    tool_station::{Context, Crafted, Outcome},
};
use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, I18n, Identifier, ItemSlot, MenuAccess, Nbt, NbtCompound, NbtCompoundExt,
    ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, SlotAction, SlotClick,
};

/// Menu data: slot 0 the active tab (0 for the main layout, else the tab's position in TAB_ORDER
/// plus one), then an x,y pair per station slot (the tool, then the inputs).
const DATA_TAB: usize = 0;
const DATA_POSITIONS: usize = 1;
/// The result slot's fixed position (`LazyResultSlot` sits at (114, 38) in every station window).
const RESULT_POSITION: (i32, i32) = (114, 38);
/// The selected layout's name in the block entity's data; absent is the main layout.
const SELECTED: &str = "layout";
/// The tab sidebar: two columns of squares left of the window (left, top, size).
const TABS: (f32, f32, f32) = (-46.0, 18.0, 18.0);
const TAB_COLUMNS: usize = 2;
/// Button ids: 1 is the main layout, 2.. the TAB_ORDER layouts in order.
const MAIN_BUTTON: i32 = 1;

/// Builds the read-only recipe context from the loaded server registries and runs `read`.
/// `None` while any registry has not loaded, so a station never crafts from partial state.
pub(crate) fn with_context<R>(read: impl FnOnce(&Context, &Resolver) -> R) -> Option<R> {
    let kit_amount = crate::config::repair_kit_amount().ok()?;
    let definitions = tool_definitions::register().read().ok()?;
    let materials = common::register().read().ok()?;
    let modifiers = common::server_modifiers()?.read().ok()?;
    if !materials.server_loaded || !modifiers.definitions_loaded || definitions.data.is_empty() { return None; }
    // Every tool definition can take a modifier; building only considers those with parts.
    let tools: Vec<(Identifier, &tool_definitions::Data)> = definitions.data.iter().map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |id: &Identifier| tool_parts::tool_stack_size(id);
    let item_tags = common::item_snapshot();
    let ctx = Context { materials: &materials, modifiers: &modifiers, item_tags_loaded: item_tags.is_some(), item_tags: item_tags.as_deref(), tools: &tools, item_max: &item_max };
    // Material recipes may be absent (not loaded): inputs then simply never resolve to materials.
    let recipes = common::material_recipes().and_then(|recipes| recipes.read().ok());
    let in_tag = |item: &Identifier, tag: &Identifier| crate::tool_station::item_in_tag(item_tags.as_deref(), &tools, item, tag);
    let modifier_recipes = common::modifier_recipes().and_then(|recipes| recipes.read().ok());
    let overslime_recipes = common::overslime_recipes().and_then(|recipes| recipes.read().ok());
    let resolver = Resolver { kit_amount, recipes: recipes.as_deref().filter(|recipes| recipes.loaded),
        modifier_recipes: modifier_recipes.as_deref().filter(|recipes| recipes.loaded),
        overslime_recipes: overslime_recipes.as_deref().filter(|recipes| recipes.loaded), in_tag: &in_tag };
    Some(read(&ctx, &resolver))
}

/// The result the staged slots would give, with nothing consumed.
fn preview_of(items: &[Option<ItemSlot>], input_count: usize) -> Option<ItemSlot> {
    let menu = SlotsView(items);
    with_context(|ctx, resolver| station::preview(&menu, ctx, input_count, resolver)).and_then(Result::ok).and_then(|outcome| match outcome {
        Outcome::Crafted(Crafted { result, .. }) => Some(result), _ => None,
    })
}

/// Keeps the result slot equal to the current preview. Returns whether anything changed.
pub(crate) fn refresh_result(items: &mut Vec<Option<ItemSlot>>, input_count: usize) -> bool {
    let result_slot = input_count + 1;
    if items.len() <= result_slot { return false; }
    let next = preview_of(&items[..result_slot], input_count);
    if items[result_slot] == next { return false; }
    items[result_slot] = next;
    true
}

/// A read-only MenuAccess over plain slots, enough for previews (nothing else is ever asked).
struct SlotsView<'a>(&'a [Option<ItemSlot>]);
impl MenuAccess for SlotsView<'_> {
    fn slot_count(&self) -> usize { self.0.len() }
    fn stack(&self, slot: usize) -> Option<ItemSlot> { self.0.get(slot).cloned().flatten() }
    fn set_stack(&mut self, _: usize, _: Option<ItemSlot>) -> bool { false }
    fn swap_stack(&self, _: i32) -> Option<ItemSlot> { None }
    fn set_swap_stack(&mut self, _: i32, _: Option<ItemSlot>) -> bool { false }
    fn carried(&self) -> Option<ItemSlot> { None }
    fn set_carried(&mut self, _: Option<ItemSlot>) {}
    fn same_item_and_components(&self, _: &ItemSlot, _: &ItemSlot) -> bool { false }
    fn max_stack_count(&self, stack: &ItemSlot) -> u32 { stack.count }
    fn move_stack(&mut self, _: &mut ItemSlot, _: usize, _: usize, _: bool) -> bool { false }
    fn crafting_remainder(&self, _: &ItemSlot) -> Option<ItemSlot> { None }
    fn give_stack(&mut self, _: ItemSlot) {}
    fn drop_stack(&mut self, _: &ItemSlot) {}
}

/// Crafts once on the staged menu: consumes inputs and the tool, restages the next preview into the
/// result slot, and returns what was crafted. Nothing happens (and `None` returns) without a match.
pub(crate) fn craft_once(menu: &mut dyn MenuAccess, input_count: usize) -> Option<ItemSlot> {
    let result_slot = input_count + 1;
    let mut lazy = LazyResultContainer::default();
    let crafted = with_context(|ctx, resolver| station::take(menu, &mut lazy, ctx, input_count, resolver))?;
    let crafted = match crafted { Ok(Some(crafted)) => crafted, Ok(None) => return None, Err(error) => { eprintln!("Tinkers: station craft failed: {error}"); return None; } };
    let items: Vec<Option<ItemSlot>> = (0..result_slot).map(|slot| menu.stack(slot)).collect();
    menu.set_stack(result_slot, preview_of(&items, input_count));
    Some(crafted)
}

/// Result-slot clicks for any table: `craft` consumes the inputs and returns what was crafted.
pub(crate) fn take_with(click: &mut SlotClick<'_>, result_slot: usize, craft: &dyn Fn(&mut dyn MenuAccess) -> Option<ItemSlot>) {
    let menu = &mut *click.menu;
    match click.action {
        SlotAction::Pickup => {
            let Some(preview) = menu.stack(result_slot) else { return; };
            // The result stacks onto a compatible cursor only when all of it fits.
            if let Some(cursor) = menu.carried() {
                let room = menu.max_stack_count(&cursor).saturating_sub(cursor.count);
                if !menu.same_item_and_components(&cursor, &preview) || room < preview.count { return; }
            }
            let Some(crafted) = craft(menu) else { return; };
            let merged = match menu.carried() {
                Some(mut cursor) => { cursor.count += crafted.count; cursor }
                None => crafted,
            };
            menu.set_carried(Some(merged));
        }
        SlotAction::QuickMove => {
            // Craft repeatedly until the inputs run out; whatever the inventory cannot hold goes to the player.
            for _ in 0..64 {
                let Some(mut crafted) = craft(menu) else { break; };
                let end = menu.slot_count();
                menu.move_stack(&mut crafted, result_slot + 1, end, true);
                if crafted.count > 0 { menu.give_stack(crafted); }
            }
        }
        SlotAction::Swap => {
            // A hotbar or offhand key: only into an empty target, like a plain result slot.
            if menu.swap_stack(click.button).is_some() { return; }
            if let Some(crafted) = craft(menu) {
                if !menu.set_swap_stack(click.button, Some(crafted.clone())) { menu.give_stack(crafted); }
            }
        }
        SlotAction::Throw => {
            if let Some(crafted) = craft(menu) { menu.drop_stack(&crafted); }
        }
    }
}

fn selected_of(data: &NbtCompound) -> Option<String> { data.get_string(SELECTED).map(str::to_owned) }

/// The layout a station window is showing: the selected one when it is a loaded tab layout, else
/// the block's main layout (the loaded one, or the baked layout before the first reload).
fn active_layout(block: &str, selected: Option<&str>) -> Layout {
    let main = common::station_layouts().and_then(|layouts| layouts.read().ok().map(|layouts| layouts.main_for(block)))
        .unwrap_or_else(|| station_layouts::baked_main(block));
    let Some(selected) = selected else { return main };
    let Ok(id) = Identifier::parse(selected) else { return main };
    if id == main.name { return main; }
    common::station_layouts().and_then(|layouts| layouts.read().ok())
        .and_then(|layouts| layouts.get(&id).filter(|layout| !layout.is_main()).cloned())
        .unwrap_or(main)
}

/// The menu data value naming the active tab: 0 for the main layout, else the tab's index plus one.
fn tab_value(layout: &Layout) -> i32 {
    if layout.is_main() { return 0; }
    TAB_ORDER.iter().position(|name| layout.name.path() == *name).map_or(0, |index| index as i32 + 1)
}

/// The tab button at a window point, as (button id, tab index or none for the main layout).
fn tab_at(at: [f32; 2]) -> Option<i32> {
    let (x, y) = (at[0] - TABS.0, at[1] - TABS.1);
    if x < 0.0 || y < 0.0 { return None; }
    let (column, row) = ((x / TABS.2) as usize, (y / TABS.2) as usize);
    if column >= TAB_COLUMNS { return None; }
    let index = row * TAB_COLUMNS + column;
    (index <= TAB_ORDER.len()).then_some(index as i32 + MAIN_BUTTON)
}

/// A tab's display name: its item translation when it has one, else its path as words.
fn tab_name(index: usize) -> String {
    if index == 0 { return I18n::translate("gui.tconstruct.tinker_station"); }
    let Some(name) = TAB_ORDER.get(index - 1) else { return String::new() };
    let key = format!("item.tconstruct.{name}");
    let text = I18n::translate(&key);
    if text != key { return text; }
    name.split('_').map(|word| {
        let mut chars = word.chars();
        chars.next().map_or(String::new(), |first| first.to_uppercase().chain(chars).collect())
    }).collect::<Vec<_>>().join(" ")
}

/// Registers one station block's block entity type and window: `input_count` inputs between the
/// tool slot and the result (TinkerStationBlock's slotCount of 4, the anvils' 6).
fn register_station(name: &'static str, input_count: usize) {
    let id = Identifier::new("tconstruct", name).expect("valid station id");
    let result_slot = input_count + 1;
    let slot_count = result_slot + 1;
    let mut layout = ScreenLayout::new(176.0, 184.0).inventory_at(8.0, 102.0);
    for _ in 0..input_count + 1 { layout = layout.hidden_slot(); }
    layout = layout.slot(RESULT_POSITION.0 as f32, RESULT_POSITION.1 as f32);
    let mut window = ScreenHandlerType::new(id.clone(), layout)
        .on_slot_rule(move |rule| { if rule.slot == result_slot { rule.may_place = false; } })
        .exclude_from_pick_all(result_slot)
        .on_dynamic_slots(move |state| {
            let main = station_layouts::baked_main(name);
            let fallback: Vec<Option<[f32; 2]>> = main.positions(&main, input_count).into_iter()
                .map(|position| position.map(|(x, y)| [x as f32, y as f32])).chain([Some([RESULT_POSITION.0 as f32, RESULT_POSITION.1 as f32])]).collect();
            if state.data.len() < DATA_POSITIONS + (input_count + 1) * 2 { return Some(fallback); }
            let mut positions: Vec<Option<[f32; 2]>> = (0..=input_count).map(|slot| {
                let (x, y) = (state.data[DATA_POSITIONS + slot * 2], state.data[DATA_POSITIONS + slot * 2 + 1]);
                (x >= 0 && y >= 0).then_some([x as f32, y as f32])
            }).collect();
            positions.push(Some([RESULT_POSITION.0 as f32, RESULT_POSITION.1 as f32]));
            Some(positions)
        })
        .on_draw(|draw| {
            let active = draw.data.get(DATA_TAB).copied().unwrap_or(0);
            for index in 0..=TAB_ORDER.len() {
                let (x, y) = (draw.origin[0] + TABS.0 + (index % TAB_COLUMNS) as f32 * TABS.2, draw.origin[1] + TABS.1 + (index / TAB_COLUMNS) as f32 * TABS.2);
                let selected = (index == 0 && active == 0) || (index > 0 && active == index as i32);
                let color = if selected { 0xFF80C0FF } else { 0xFF8B8B8B };
                draw.canvas.fill([x, y, TABS.2 - 1.0, TABS.2 - 1.0], color);
                draw.canvas.text(&(index + 1).to_string(), [x + 5.0, y + 5.0], 0x202020, false);
            }
        })
        .on_click(|press| (!press.right).then(|| tab_at(press.at)).flatten())
        .on_tooltip(|tip| {
            if let Some(button) = tab_at(tip.at) {
                tip.lines.push(tab_name((button - MAIN_BUTTON) as usize));
            }
        })
        .on_button(move |press| {
            if press.id == MAIN_BUTTON { press.data.remove(SELECTED); return; }
            if let Some(name) = TAB_ORDER.get((press.id - MAIN_BUTTON - 1) as usize) {
                press.data.insert(SELECTED.to_owned(), Nbt::String(format!("tconstruct:{name}")));
            }
        });
    for action in [SlotAction::Pickup, SlotAction::QuickMove, SlotAction::Swap, SlotAction::Throw] {
        window = window.on_slot_click(result_slot, action, move |click| {
            take_with(click, result_slot, &|menu| craft_once(menu, input_count));
        });
    }
    ScreenHandlerRegistry::register(window);
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(id).item_slots(slot_count as u32).menu(&format!("tconstruct:{name}")).ticker(move |tick| {
            refresh_result(tick.items, input_count);
            let selected = selected_of(tick.data);
            let layout = active_layout(name, selected.as_deref());
            let main = common::station_layouts().and_then(|layouts| layouts.read().ok().map(|layouts| layouts.main_for(name)))
                .unwrap_or_else(|| station_layouts::baked_main(name));
            tick.set_menu_data(DATA_TAB, tab_value(&layout));
            for (slot, position) in layout.positions(&main, input_count).into_iter().enumerate() {
                let (x, y) = position.unwrap_or((-1, -1));
                tick.set_menu_data(DATA_POSITIONS + slot * 2, x);
                tick.set_menu_data(DATA_POSITIONS + slot * 2 + 1, y);
            }
        }),
    );
}

/// Registers the station family. Called once from the product's hooks.
pub(crate) fn register() {
    register_station("tinker_station", 4);
    register_station("tinkers_anvil", 6);
    register_station("scorched_anvil", 6);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_result_slot_follows_the_preview_and_unloaded_registries_craft_nothing() {
        // With no registries loaded the context is absent: nothing previews, nothing changes.
        let mut items: Vec<Option<ItemSlot>> = vec![None; 6];
        assert!(!refresh_result(&mut items, 4));
        assert!(with_context(|_, _| ()).is_none());
        // A stale result with no inputs is cleared.
        items[5] = Some(ItemSlot::new(Identifier::parse("test:stale").unwrap(), 1));
        assert!(refresh_result(&mut items, 4));
        assert!(items[5].is_none());
        // Too few slots are left alone rather than indexed out of range.
        assert!(!refresh_result(&mut vec![None; 2], 4));
    }

    #[test]
    fn tabs_sit_in_two_columns_and_the_main_layout_is_first() {
        assert_eq!(tab_at([TABS.0 + 1.0, TABS.1 + 1.0]), Some(MAIN_BUTTON));
        assert_eq!(tab_at([TABS.0 + TABS.2 + 1.0, TABS.1 + 1.0]), Some(MAIN_BUTTON + 1));
        assert_eq!(tab_at([TABS.0 + 1.0, TABS.1 + TABS.2 + 1.0]), Some(MAIN_BUTTON + 2));
        assert_eq!(tab_at([0.0, 0.0]), None, "inside the window is not a tab");
        assert_eq!(tab_value(&station_layouts::baked_main("tinker_station")), 0);
    }
}
