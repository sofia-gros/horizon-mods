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
//! - The side info panels and the armor stand preview do not carry over (the screen surface has no
//!   text panels and no entities), nor does the rename text field (no text input), nor a tab's
//!   hover face (the draw callback is not told where the cursor is).
//!
//! The window itself is drawn as the original's screen draws it: the `tinker.png` background, the
//! active layout's icon large under its translucent cover, a translucent background and an opaque
//! border at every shown slot, the pattern icon of every empty slot, and the layout tabs in a
//! six-column strip on the left with the original's button faces, beams and decorations.
use crate::{
    common, station_layout_order::TAB_ORDER, station_layouts::{self, Layout},
    tables::{lazy_result::LazyResultContainer, station::{self, Resolver}},
    tool_definitions, tool_parts,
    tool_station::{Context, Crafted, Outcome},
};
use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, I18n, Identifier, ItemSlot, MenuAccess, Nbt, NbtCompound, NbtCompoundExt,
    ScreenCanvas, ScreenDraw, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, SlotAction, SlotClick,
};

/// Menu data: slot 0 the active tab (0 for the main layout, else the tab's position in TAB_ORDER
/// plus one), then an x,y pair per station slot (the tool, then the inputs), then one pattern-sprite
/// code per station slot (`station_layouts::pattern_sprite_code`), then a bitmask of the slots
/// holding an item. The last two carry what the original's screen reads from the menu itself: the
/// pattern an empty slot shows, and whether the slot is empty.
const DATA_TAB: usize = 0;
const DATA_POSITIONS: usize = 1;
/// The result slot's fixed position (`LazyResultSlot` sits at (114, 38) in every station window).
const RESULT_POSITION: (i32, i32) = (114, 38);
/// The selected layout's name in the block entity's data; absent is the main layout.
const SELECTED: &str = "layout";
/// The window's textures: the station sheet and the shared icon sheet, both 256x256.
const TINKER: &str = "tconstruct:gui/tinker";
const ICONS: &str = "tconstruct:gui/icons";
const SHEET: [f32; 2] = [256.0, 256.0];
/// The layout tabs left of the window, as the original's buttons widget places them: 18px buttons
/// with 4px spacing in six columns, the widget's top at the window top plus its beams (9px), its
/// right edge 2px left of the window (`TinkerStationButtonsWidget`: width(6) = 6*18 + 5*4 = 128).
const TAB_COLUMNS: usize = 6;
const TAB_LEFT: f32 = -130.0;
const TAB_TOP: f32 = 9.0;
const TAB_SIZE: f32 = 18.0;
const TAB_STEP: f32 = 22.0;
/// Button ids: 1 is the main layout, 2.. the TAB_ORDER layouts in order.
const MAIN_BUTTON: i32 = 1;

/// What a tab button shows: the layout's icon from the shipped layout data — the tool item for the
/// tool layouts, a pattern for the repair button and the two pattern-icon layouts.
enum TabIcon {
    Item(&'static str),
    Pattern(&'static str),
}

/// The tab icons in button order: the main layout first, then TAB_ORDER.
const TAB_ICONS: &[TabIcon] = &[
    TabIcon::Pattern("tconstruct:button_repair"),
    TabIcon::Item("tconstruct:dagger"), TabIcon::Item("tconstruct:sword"),
    TabIcon::Item("tconstruct:hand_axe"), TabIcon::Item("tconstruct:kama"),
    TabIcon::Item("tconstruct:mattock"), TabIcon::Item("tconstruct:pickadze"),
    TabIcon::Item("tconstruct:pickaxe"), TabIcon::Item("tconstruct:arrow"),
    TabIcon::Pattern("tconstruct:thrown_ammo"), TabIcon::Item("tconstruct:crossbow"),
    TabIcon::Item("tconstruct:fishing_rod"), TabIcon::Item("tconstruct:cleaver"),
    TabIcon::Item("tconstruct:broad_axe"), TabIcon::Item("tconstruct:excavator"),
    TabIcon::Item("tconstruct:scythe"), TabIcon::Item("tconstruct:sledge_hammer"),
    TabIcon::Item("tconstruct:vein_hammer"), TabIcon::Item("tconstruct:javelin"),
    TabIcon::Item("tconstruct:longbow"), TabIcon::Pattern("tconstruct:plate_armor"),
];

/// Builds the read-only recipe context from the loaded server registries and runs `read`.
/// `None` while any registry has not loaded, so a station never crafts from partial state.
pub(crate) fn with_context<R>(read: impl FnOnce(&Context, &Resolver) -> R) -> Option<R> {
    let kit_amount = crate::config::repair_kit_amount().ok()?;
    let definitions = tool_definitions::register().read().ok()?;
    let materials = common::register().read().ok()?;
    let modifiers = common::server_modifiers()?.read().ok()?;
    if !materials.server_loaded || !modifiers.definitions_loaded || definitions.data.is_empty() { return None; }
    // Every tool definition can take a modifier; building only considers those with parts.
    let tools: Vec<(Identifier, &tool_definitions::Data)> = tool_definitions::item_pairs(&definitions);
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

/// The tab button at a window point, as a button id (the main layout first).
fn tab_at(at: [f32; 2]) -> Option<i32> {
    let (x, y) = (at[0] - TAB_LEFT, at[1] - TAB_TOP);
    if x < 0.0 || y < 0.0 { return None; }
    let (column, row) = ((x / TAB_STEP) as usize, (y / TAB_STEP) as usize);
    if column >= TAB_COLUMNS || x % TAB_STEP >= TAB_SIZE || y % TAB_STEP >= TAB_SIZE { return None; }
    let index = row * TAB_COLUMNS + column;
    (index <= TAB_ORDER.len()).then_some(index as i32 + MAIN_BUTTON)
}

/// Draws one layout icon at a window point: a pattern from the block atlas, or a plain stack of the
/// layout's tool item (the original's `renderIcon`).
fn draw_icon(canvas: &mut dyn ScreenCanvas, icon: &TabIcon, at: [f32; 2], size: f32) {
    match icon {
        TabIcon::Pattern(pattern) => {
            let pattern = Identifier::parse(pattern).expect("valid tab pattern");
            if let Some(sprite) = station_layouts::pattern_sprite_of(&pattern) {
                canvas.atlas_sprite(sprite, [at[0], at[1], size, size], 0xFFFF_FFFF);
            }
        }
        TabIcon::Item(item) => {
            let stack = ItemSlot::new(Identifier::parse(item).expect("valid tab item"), 1);
            canvas.item(&stack, at, size);
        }
    }
}

/// Draws the window as `TinkerStationScreen` does: the `tinker.png` background, the active layout's
/// icon large under the translucent item cover, a translucent background and an opaque border at
/// every shown slot, the pattern of every empty slot, and the tab strip with its beams, decorations
/// and button faces. `metal` is the anvils' style; the tinker station's is wood (the original picks
/// by input count: metal above three). A tab's hover face is not drawn: the draw callback is not
/// told where the cursor is.
fn draw_station(draw: &mut ScreenDraw<'_>, metal: bool, input_count: usize, icons_at: usize, filled_at: usize) {
    let canvas = &mut *draw.canvas;
    let (ox, oy) = (draw.origin[0], draw.origin[1]);
    let data = draw.data;
    let active = data.get(DATA_TAB).copied().unwrap_or(0);
    canvas.sprite(TINKER, [0.0, 0.0, 176.0, 184.0], [ox, oy, 176.0, 184.0]);
    if let Some(icon) = TAB_ICONS.get(active as usize) { draw_icon(canvas, icon, [ox + 12.5, oy + 22.0], 59.2); }
    canvas.sprite_tinted(TINKER, [176.0, 18.0, 70.0, 64.0], SHEET, [ox + 7.0, oy + 18.0, 70.0, 64.0], 0xD1FF_FFFF);
    let position = |slot: usize| {
        let (x, y) = (data.get(DATA_POSITIONS + slot * 2).copied().unwrap_or(-1), data.get(DATA_POSITIONS + slot * 2 + 1).copied().unwrap_or(-1));
        (x >= 0 && y >= 0).then_some((x as f32, y as f32))
    };
    for slot in 0..=input_count {
        let Some((x, y)) = position(slot) else { continue; };
        canvas.sprite_tinted(TINKER, [176.0, 0.0, 18.0, 18.0], SHEET, [ox + x - 1.0, oy + y - 1.0, 18.0, 18.0], 0x47FF_FFFF);
    }
    let filled = data.get(filled_at).copied().unwrap_or(0);
    for slot in 0..=input_count {
        let Some((x, y)) = position(slot) else { continue; };
        canvas.sprite(TINKER, [194.0, 0.0, 18.0, 18.0], [ox + x - 1.0, oy + y - 1.0, 18.0, 18.0]);
        if filled & (1 << slot) == 0 {
            if let Some(sprite) = data.get(icons_at + slot).copied().and_then(station_layouts::pattern_sprite) {
                canvas.atlas_sprite(sprite, [ox + x, oy + y, 16.0, 16.0], 0xFFFF_FFFF);
            }
        }
    }
    let beam_v = if metal { 209.0 } else { 202.0 };
    canvas.sprite(TINKER, [0.0, beam_v, 2.0, 7.0], [ox + TAB_LEFT - 2.0, oy, 2.0, 7.0]);
    canvas.sprite(TINKER, [2.0, beam_v, 129.0, 7.0], [ox + TAB_LEFT, oy, 128.0, 7.0]);
    canvas.sprite(TINKER, [131.0, beam_v, 2.0, 7.0], [ox + TAB_LEFT + 128.0, oy, 2.0, 7.0]);
    let (decoration_u, face_v) = if metal { (36.0, 198.0) } else { (18.0, 180.0) };
    let count = TAB_ORDER.len() + 1;
    for index in 0..count {
        let (x, y) = (ox + TAB_LEFT + (index % TAB_COLUMNS) as f32 * TAB_STEP, oy + TAB_TOP + (index / TAB_COLUMNS) as f32 * TAB_STEP);
        canvas.sprite(TINKER, [decoration_u, 198.0, 18.0, 2.0], [x, y - 2.0, 18.0, 2.0]);
        if index < count - TAB_COLUMNS { canvas.sprite(TINKER, [decoration_u, 196.0, 18.0, 2.0], [x, y + TAB_SIZE, 18.0, 2.0]); }
        let face_u = if index as i32 == active { 144.0 } else { 180.0 };
        canvas.sprite(ICONS, [face_u, face_v, 18.0, 18.0], [x, y, 18.0, 18.0]);
        if let Some(icon) = TAB_ICONS.get(index) { draw_icon(canvas, icon, [x + 1.0, y + 1.0], 16.0); }
    }
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
    // Menu data past the positions: the slots' icon codes, then the filled bitmask.
    let icons_at = DATA_POSITIONS + (input_count + 1) * 2;
    let filled_at = icons_at + input_count + 1;
    let metal = input_count > 4;
    let mut layout = ScreenLayout::new(176.0, 184.0).inventory_at(8.0, 102.0).with_own_background();
    // The tool slot and the inputs are in the menu; the selected layout positions them.
    for _ in 0..input_count + 1 { layout = layout.dynamic_slot(); }
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
        .on_draw(move |draw| draw_station(draw, metal, input_count, icons_at, filled_at))
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
            // A stack can reach a slot with its material data but without the model data the item
            // definitions select on (a `/give` naming only `custom_data`, another mod's stack, an
            // older save): it then renders as the untextured base sprite. Keep the staged stacks
            // in step, as the original's model loader reads the material from the stack itself.
            for slot in tick.items.iter_mut().take(input_count + 1).flatten() {
                crate::tool_materials::heal_model(slot);
            }
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
            for (slot, layout_slot) in layout.slots(&main, input_count).into_iter().enumerate() {
                let code = layout_slot.and_then(|layout_slot| layout_slot.icon.as_ref()).map_or(0, station_layouts::pattern_sprite_code);
                tick.set_menu_data(icons_at + slot, code);
            }
            let filled = tick.items.iter().take(input_count + 1).enumerate()
                .fold(0i32, |bits, (slot, item)| if item.is_some() { bits | (1 << slot) } else { bits });
            tick.set_menu_data(filled_at, filled);
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
    fn tabs_sit_in_six_columns_and_the_main_layout_is_first() {
        assert_eq!(tab_at([TAB_LEFT + 1.0, TAB_TOP + 1.0]), Some(MAIN_BUTTON));
        assert_eq!(tab_at([TAB_LEFT + TAB_STEP + 1.0, TAB_TOP + 1.0]), Some(MAIN_BUTTON + 1));
        assert_eq!(tab_at([TAB_LEFT + 1.0, TAB_TOP + TAB_STEP + 1.0]), Some(MAIN_BUTTON + 6));
        assert_eq!(tab_at([TAB_LEFT + TAB_SIZE + 2.0, TAB_TOP + 1.0]), None, "the spacing between buttons is not a tab");
        assert_eq!(tab_at([0.0, 0.0]), None, "inside the window is not a tab");
        assert_eq!(TAB_ICONS.len(), TAB_ORDER.len() + 1);
        assert_eq!(tab_value(&station_layouts::baked_main("tinker_station")), 0);
    }
}
