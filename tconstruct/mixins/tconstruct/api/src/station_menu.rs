//! The tinker station family's block entities, windows and result-slot ownership over PlutoLib's
//! public API: the tinker station (three inputs) and the two anvils (five inputs), which share the
//! station crafting of `tables::station`. SlimeKnights (MIT; see LICENSE) defines the recipes and
//! the slot layouts; this registers where they apply.
//!
//! Slot layouts (station_layouts) drive the window: the selected layout — a station's main layout
//! until a tab is picked — positions the tool and input slots through the dynamic-slot callback,
//! with the ticker publishing the active layout and its positions as menu data (slot 0 the active
//! tab, then an x,y pair per slot, -1 for a slot the layout leaves out). The layout tabs sit in a
//! sidebar left of the window, as the original's screen draws them; a station only offers the
//! layouts its input count can host (the four-input layouts belong to the anvils), so the ticker
//! publishes the offered tabs packed — a count and, per packed position, the tab's full index —
//! and the draw, click and tooltip callbacks all read that packing. The side info panels (the
//! staged tool's materials, its modifiers, the layout's components) are drawn from the window's
//! own synced stacks, a slot a layout leaves out but that still holds an item is drawn in the
//! original's still-filled row, and the rename field at the window's top takes the client's typed
//! characters through PlutoLib's text input.
//!
//! What does not carry over, and why:
//! - A layout's slot filters cannot be enforced: PlutoLib's slot rule sees one stack at a time with
//!   no layout context. A wrong item in a slot simply crafts nothing, where the original refuses it.
//! - A slot a layout leaves out is hidden here (its item shows in the still-filled row); the
//!   original keeps it dormant and take-out-able. Switching back to a layout that shows the slot
//!   reaches its contents again.
//! - The original's window also carries the player's armor and offhand slots. They are declared
//!   (at the original's places, after the result slot) and drawn from the opening player's live
//!   equipment, but they take no clicks: the host's menu model binds container slots to the
//!   opened block entity or to linked block containers only, so swapping a worn piece still
//!   happens in the inventory window.
//! - The neighbouring tables' tabs above the window are carried (see `tabbed_tables`): a tab
//!   switches the open menu to that table. The station family shows no side inventory in the
//!   original, and none here. A LAN guest's tab press does not switch (see `tabbed_tables`).
//! - The armor stand preview is drawn through PlutoLib's living preview (the host's own
//!   screen-preview path for a stand, the smithing screen's), wearing or holding the staged
//!   result. It stands at the original's anchor; the original turns it with the mouse and
//!   drops it when the layout list grows past six rows, neither of which is carried here.
//! - The rename field edits on the client, but the name is never applied to the crafted tool: a
//!   screen has no channel that carries text to the block entity (menu buttons carry an id alone,
//!   and a stack's name is not a field a slot carries), where the original's field writes the
//!   result's name as it is typed.
//!
//! The window itself is drawn as the original's screen draws it: the `tinker.png` background, the
//! active layout's icon large under its translucent cover, a translucent background and an opaque
//! border at every shown slot, the pattern icon of every empty slot, and the layout tabs in a
//! six-column strip on the left with the original's button faces (the hovered tab gets the hover
//! face), beams and decorations.
use crate::{
    common, station_layout_order::TAB_ORDER, station_layouts::{self, Layout},
    tables::{lazy_result::LazyResultContainer, station::{self, Resolver}},
    tool_definitions, tool_parts,
    tool_station::{Context, Crafted, Outcome},
};
use std::sync::{Arc, Mutex};

use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, EquipmentSlotKind, I18n, Identifier, ItemSlot, LivingPreview, MenuAccess, Nbt, NbtCompound, NbtCompoundExt,
    ScreenCanvas, ScreenDraw, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, SlotAction, SlotClick,
};

/// Menu data: slot 0 the active tab's packed position (see the header: the offered tabs are
/// packed per station), then an x,y pair per station slot (the tool, then the inputs), then one
/// pattern-sprite code per station slot (`station_layouts::pattern_sprite_code`), then a bitmask
/// of the slots holding an item, then the packed tab count and one full tab index per packed
/// position, then an x,y pair per station slot for the still-filled row (-1 for a slot that is
/// shown or empty). The pattern codes and the bitmask carry what the original's screen reads from
/// the menu itself: the pattern an empty slot shows, and whether the slot is empty.
const DATA_TAB: usize = 0;
const DATA_POSITIONS: usize = 1;
/// The result slot's fixed position (`LazyResultSlot` sits at (114, 38) in every station window).
const RESULT_POSITION: (i32, i32) = (114, 38);
/// The player's own slots the window carries after the result slot, at the original's places
/// (`TinkerStationContainerMenu`: an armor slot per `ArmorItem.Type` at (152, 20 + ordinal * 18)
/// — head, chest, legs, feet — and the offhand at (132, 74)). They are equipment-sourced slots:
/// drawn from the opening player's live equipment, not the block entity's items.
const EQUIPMENT_POSITIONS: [[f32; 2]; 5] = [[152.0, 20.0], [152.0, 38.0], [152.0, 56.0], [152.0, 74.0], [132.0, 74.0]];
/// The equipment pieces at `EQUIPMENT_POSITIONS`, in the same order.
const EQUIPMENT_KINDS: [EquipmentSlotKind; 5] = [EquipmentSlotKind::Head, EquipmentSlotKind::Chest, EquipmentSlotKind::Legs, EquipmentSlotKind::Feet, EquipmentSlotKind::OffHand];
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
/// The still-filled row (TinkerStationScreen): the first hidden-but-filled slot's item sits at
/// (112, 62) and each next one 18px further left.
const STILL_FILLED: (i32, i32) = (112, 62);
/// The rename field: its box sprite at (79, 5), the text itself at (82, 8), at most 50 characters
/// (TinkerStationScreen's EditBox).
const TEXT_BOX_AT: (f32, f32) = (79.0, 5.0);
const NAME_MAX: usize = 50;
/// How many inputs each TAB_ORDER layout needs, in TAB_ORDER order, from the shipped layout data;
/// the ticker prefers the loaded layouts' own counts and falls back to this table before the
/// first reload. Only the anvils (five inputs) host the eight four-input layouts.
const TAB_INPUT_NEED: &[usize] = &[
    2, 3, 3, 3, 3, 3, 3, 3, 2, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 2,
];

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
    let modifier_repair_recipes = common::modifier_repair_recipes().and_then(|recipes| recipes.read().ok());
    let damaging_recipes = common::damaging_recipes().and_then(|recipes| recipes.read().ok());
    let dyeing_recipes = common::dyeing_recipes().and_then(|recipes| recipes.read().ok());
    let resolver = Resolver { kit_amount, recipes: recipes.as_deref().filter(|recipes| recipes.loaded),
        modifier_recipes: modifier_recipes.as_deref().filter(|recipes| recipes.loaded),
        overslime_recipes: overslime_recipes.as_deref().filter(|recipes| recipes.loaded),
        modifier_repair_recipes: modifier_repair_recipes.as_deref().filter(|recipes| recipes.loaded),
        damaging_recipes: damaging_recipes.as_deref().filter(|recipes| recipes.loaded),
        dyeing_recipes: dyeing_recipes.as_deref().filter(|recipes| recipes.loaded), in_tag: &in_tag };
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

/// The packed tab position at a window point, from the geometry alone.
fn packed_tab_at(at: [f32; 2]) -> Option<usize> {
    let (x, y) = (at[0] - TAB_LEFT, at[1] - TAB_TOP);
    if x < 0.0 || y < 0.0 { return None; }
    let (column, row) = ((x / TAB_STEP) as usize, (y / TAB_STEP) as usize);
    if column >= TAB_COLUMNS || x % TAB_STEP >= TAB_SIZE || y % TAB_STEP >= TAB_SIZE { return None; }
    Some(row * TAB_COLUMNS + column)
}

/// The tab button at a window point, as a button id: the packed position the geometry gives,
/// resolved through the menu data's packing to the tab's full index (the main layout first).
fn tab_at(at: [f32; 2], data: &[i32], tabs_at: usize, map_at: usize) -> Option<i32> {
    let packed = packed_tab_at(at)?;
    let count = data.get(tabs_at).copied().unwrap_or(0) as usize;
    if packed >= count { return None; }
    let full = data.get(map_at + packed).copied().unwrap_or(0);
    Some(full + MAIN_BUTTON)
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

/// Draws the window as `TinkerStationScreen` does: the `tinker.png` background, the rename field,
/// the active layout's icon large under the translucent item cover, a translucent background and
/// an opaque border at every shown slot, the pattern of every empty slot, the still-filled row,
/// the tab strip with its beams, decorations and button faces (the hovered tab gets the hover
/// face), and the two side info panels. `metal` is the anvils' style; the tinker station's is
/// wood (the original picks by input count: metal above three).
#[allow(clippy::too_many_arguments)]
fn draw_station(draw: &mut ScreenDraw<'_>, metal: bool, input_count: usize, icons_at: usize, filled_at: usize, tabs_at: usize, map_at: usize, dormant_at: usize, tabbed_at: usize, name: &str) {
    let canvas = &mut *draw.canvas;
    let (ox, oy) = (draw.origin[0], draw.origin[1]);
    let data = draw.data;
    let active = data.get(DATA_TAB).copied().unwrap_or(0);
    let active_full = data.get(map_at + active as usize).copied().unwrap_or(0);
    canvas.sprite(TINKER, [0.0, 0.0, 176.0, 184.0], [ox, oy, 176.0, 184.0]);
    // The neighbouring tables' tabs above the window (see `tabbed_tables`).
    crate::tabbed_tables::draw_tabs(canvas, [ox, oy], data, tabbed_at, ICONS);
    // The rename field (its text is the client's own; see the header for why it goes no further).
    canvas.sprite(TINKER, [0.0, 244.0, 90.0, 12.0], [ox + TEXT_BOX_AT.0, oy + TEXT_BOX_AT.1, 90.0, 12.0]);
    if !name.is_empty() {
        canvas.text(name, [ox + TEXT_BOX_AT.0 + 3.0, oy + TEXT_BOX_AT.1 + 3.0], 0xFF3F_3F3F, false);
    }
    if let Some(icon) = TAB_ICONS.get(active_full as usize) { draw_icon(canvas, icon, [ox + 12.5, oy + 22.0], 59.2); }
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
    // The still-filled row: an item in a slot the active layout leaves out, drawn where the
    // original draws it (the dynamic-slot callback never positions these; they are not slots).
    for slot in 0..=input_count {
        let (x, y) = (data.get(dormant_at + slot * 2).copied().unwrap_or(-1), data.get(dormant_at + slot * 2 + 1).copied().unwrap_or(-1));
        if x < 0 || y < 0 { continue; }
        if let Some(item) = draw.items.get(slot).cloned().flatten() {
            canvas.item(&item, [ox + x as f32, oy + y as f32], 16.0);
        }
    }
    let beam_v = if metal { 209.0 } else { 202.0 };
    canvas.sprite(TINKER, [0.0, beam_v, 2.0, 7.0], [ox + TAB_LEFT - 2.0, oy, 2.0, 7.0]);
    canvas.sprite(TINKER, [2.0, beam_v, 129.0, 7.0], [ox + TAB_LEFT, oy, 128.0, 7.0]);
    canvas.sprite(TINKER, [131.0, beam_v, 2.0, 7.0], [ox + TAB_LEFT + 128.0, oy, 2.0, 7.0]);
    let (decoration_u, face_v) = if metal { (36.0, 198.0) } else { (18.0, 180.0) };
    let count = data.get(tabs_at).copied().unwrap_or(0).max(0) as usize;
    let hovered = packed_tab_at(draw.mouse);
    for index in 0..count {
        let (x, y) = (ox + TAB_LEFT + (index % TAB_COLUMNS) as f32 * TAB_STEP, oy + TAB_TOP + (index / TAB_COLUMNS) as f32 * TAB_STEP);
        canvas.sprite(TINKER, [decoration_u, 198.0, 18.0, 2.0], [x, y - 2.0, 18.0, 2.0]);
        if index + TAB_COLUMNS < count { canvas.sprite(TINKER, [decoration_u, 196.0, 18.0, 2.0], [x, y + TAB_SIZE, 18.0, 2.0]); }
        let face_u = if index as i32 == active { 144.0 } else if hovered == Some(index) { 162.0 } else { 180.0 };
        canvas.sprite(ICONS, [face_u, face_v, 18.0, 18.0], [x, y, 18.0, 18.0]);
        let full = data.get(map_at + index).copied().unwrap_or(0) as usize;
        if let Some(icon) = TAB_ICONS.get(full) { draw_icon(canvas, icon, [x + 1.0, y + 1.0], 16.0); }
    }
    // The side info panels, right of the window (the tinker panel at the original's offset, the
    // modifier panel one panel-height below it).
    let ((caption, lines), (caption2, lines2)) = panel_content(draw.items, input_count + 1, active_full as usize);
    draw_info_panel(canvas, [ox + 178.0, oy + 15.0], &caption, &lines);
    draw_info_panel(canvas, [ox + 178.0, oy + 15.0 + PANEL_SIZE.1 + 4.0], &caption2, &lines2);
    // The armor stand preview left of the window, wearing or holding what the station would
    // produce (the staged result, else the tool in the tool slot), as the original's screen
    // draws it (TinkerStationScreen: setupArmorStandPreview(-55, 195, 35); the box's foot
    // stands at the anchor's y, its head 70 above it).
    let shown = draw.items.get(input_count + 1).cloned().flatten().or_else(|| draw.items.first().cloned().flatten());
    if let Some(item) = shown {
        canvas.living([ox - 55.0, oy + 125.0, ox - 5.0, oy + 195.0], &LivingPreview::armor_stand(item));
    }
}

/// An item's display name: its `item.` (or `block.`) translation, else its path as words.
pub(crate) fn item_display_name(item: &Identifier) -> String {
    for prefix in ["item", "block"] {
        let key = format!("{prefix}.{}.{}", item.namespace(), item.path());
        let text = I18n::translate(&key);
        if text != key { return text; }
    }
    words(item.path())
}

/// A material's display name: its `material.` translation, else its path as words.
pub(crate) fn material_display_name(material: &Identifier) -> String {
    let key = format!("material.{}.{}", material.namespace(), material.path());
    let text = I18n::translate(&key);
    if text != key { return text; }
    words(material.path())
}

/// A modifier's display name: its `modifier.` translation, else its path as words.
pub(crate) fn modifier_display_name(modifier: &Identifier) -> String {
    let key = format!("modifier.{}.{}", modifier.namespace(), modifier.path().replace('/', "."));
    let text = I18n::translate(&key);
    if text != key { return text; }
    words(modifier.path())
}

/// An id path as capitalized words (`broad_axe` to "Broad Axe").
fn words(path: &str) -> String {
    path.split(['_', '/']).map(|word| {
        let mut chars = word.chars();
        chars.next().map_or(String::new(), |first| first.to_uppercase().chain(chars).collect())
    }).collect::<Vec<_>>().join(" ")
}

/// The side info panel's size (the original's `InfoPanelScreen` is 126x83).
pub(crate) const PANEL_SIZE: (f32, f32) = (126.0, 83.0);

/// Draws one side info panel at a window point: a flat parchment panel with a border, the caption,
/// then the lines wrapped to the panel's width. Shared with the modifier worktable's panels.
pub(crate) fn draw_info_panel(canvas: &mut dyn ScreenCanvas, at: [f32; 2], caption: &str, lines: &[String]) {
    draw_info_panel_sized(canvas, at, PANEL_SIZE, caption, lines);
}

/// [`draw_info_panel`] at a caller-chosen size (the part builder's panel is the window's height).
pub(crate) fn draw_info_panel_sized(canvas: &mut dyn ScreenCanvas, at: [f32; 2], size: (f32, f32), caption: &str, lines: &[String]) {
    let (w, h) = size;
    canvas.fill([at[0], at[1], w, h], 0xFFC6_C6C6);
    canvas.fill([at[0], at[1], w, 1.0], 0xFF55_5555);
    canvas.fill([at[0], at[1] + h - 1.0, w, 1.0], 0xFF55_5555);
    canvas.fill([at[0], at[1], 1.0, h], 0xFF55_5555);
    canvas.fill([at[0] + w - 1.0, at[1], 1.0, h], 0xFF55_5555);
    canvas.text(caption, [at[0] + 8.0, at[1] + 6.0], 0xFF3F_3F3F, false);
    let mut y = at[1] + 20.0;
    for line in lines {
        let mut rest = line.as_str();
        loop {
            if y + 10.0 > at[1] + h - 3.0 { return; }
            if canvas.text_width(rest) <= 110.0 {
                canvas.text(rest, [at[0] + 8.0, y], 0xFF3F_3F3F, false);
                y += 10.0;
                break;
            }
            // Wrap at the last space that fits; a word longer than the panel is cut by characters.
            let mut cut = 0;
            for (index, _) in rest.char_indices() {
                if canvas.text_width(&rest[..index]) > 110.0 { break; }
                if rest.as_bytes()[index] == b' ' { cut = index; }
            }
            if cut == 0 {
                cut = rest.char_indices().take_while(|(index, _)| canvas.text_width(&rest[..*index]) <= 110.0).last().map_or(1, |(index, _)| index);
            }
            canvas.text(rest[..cut].trim_end(), [at[0] + 8.0, y], 0xFF3F_3F3F, false);
            y += 10.0;
            rest = rest[cut..].trim_start();
            if rest.is_empty() { break; }
        }
    }
}

/// What the station's two side panels show, read from the window's own synced stacks: a staged or
/// resulted tool gives its name, its part materials and its modifiers; anything else gives the
/// active layout's name and the staged stacks as the components list. The original's panels read
/// the same stacks through the menu; its error text has no counterpart here (a failed preview is
/// silent in this port), and its layout description and per-slot component names live in layout
/// data the client does not hold, so the staged stacks stand in for them.
fn panel_content(items: &[Option<ItemSlot>], result_slot: usize, active_full: usize) -> ((String, Vec<String>), (String, Vec<String>)) {
    let components = I18n::translate("gui.tconstruct.tinker_station.components");
    let staged: Vec<String> = items.iter().take(result_slot).filter_map(|slot| slot.as_ref().map(|stack| item_display_name(&stack.item))).collect();
    let subject = items.get(result_slot).cloned().flatten().or_else(|| items.first().cloned().flatten());
    let Some(subject) = subject else {
        return ((tab_name(active_full), Vec::new()), (components, staged));
    };
    let data = &subject.custom_data;
    if crate::tool_stack::is_initialized(data) {
        let lines: Vec<String> = crate::tool_stack::materials(data, true).as_slice().iter()
            .filter_map(|variant| variant.material_id()).map(|material| material_display_name(&material)).collect();
        let modifiers: Vec<String> = crate::tool_stack::upgrades(data).entries().iter().filter(|entry| entry.level > 0)
            .map(|entry| {
                let name = modifier_display_name(&entry.id);
                if entry.level > 1 { format!("{name} {}", entry.level) } else { name }
            }).collect();
        return ((item_display_name(&subject.item), lines), (I18n::translate("gui.tconstruct.tinker_station.modifiers"), modifiers));
    }
    ((item_display_name(&subject.item), Vec::new()), (components, staged))
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
/// tool slot and the result (the tinker station's three, the anvils' five — TinkerStationBlock's
/// slot counts of 4 and 6 include the tool slot itself).
fn register_station(name: &'static str, input_count: usize) {
    let id = Identifier::new("tconstruct", name).expect("valid station id");
    let result_slot = input_count + 1;
    let slot_count = result_slot + 1;
    // Menu data past the positions: the slots' icon codes, then the filled bitmask, then the
    // packed tab count and map, then the still-filled positions.
    let icons_at = DATA_POSITIONS + (input_count + 1) * 2;
    let filled_at = icons_at + input_count + 1;
    let tabs_at = filled_at + 1;
    let map_at = tabs_at + 1;
    let dormant_at = map_at + TAB_ICONS.len();
    // The tabbed tables' words (the neighbouring-table strip) follow the still-filled row.
    let tabbed_at = dormant_at + (input_count + 1) * 2;
    let metal = input_count > 3;
    // The rename field's text: the window's own client-side state, shared by its draw and its
    // text-input callbacks (see the header for why it never leaves the client).
    let field: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let mut layout = ScreenLayout::new(176.0, 184.0).inventory_at(8.0, 102.0).with_own_background();
    // The tool slot and the inputs are in the menu; the selected layout positions them.
    for _ in 0..input_count + 1 { layout = layout.dynamic_slot(); }
    layout = layout.slot(RESULT_POSITION.0 as f32, RESULT_POSITION.1 as f32);
    // The player's armor and offhand slots follow the result slot, as the original's menu adds
    // them after it.
    for (position, kind) in EQUIPMENT_POSITIONS.iter().zip(EQUIPMENT_KINDS) {
        layout = layout.equipment_slot(position[0], position[1], kind);
    }
    let draw_field = field.clone();
    let char_field = field.clone();
    let key_field = field.clone();
    let mut window = ScreenHandlerType::new(id.clone(), layout)
        .on_slot_rule(move |rule| { if rule.slot == result_slot { rule.may_place = false; } })
        .exclude_from_pick_all(result_slot)
        .on_dynamic_slots(move |state| {
            let main = station_layouts::baked_main(name);
            let fallback: Vec<Option<[f32; 2]>> = main.positions(&main, input_count).into_iter()
                .map(|position| position.map(|(x, y)| [x as f32, y as f32])).chain([Some([RESULT_POSITION.0 as f32, RESULT_POSITION.1 as f32])])
                .chain(EQUIPMENT_POSITIONS.map(Some)).collect();
            if state.data.len() < DATA_POSITIONS + (input_count + 1) * 2 { return Some(fallback); }
            let mut positions: Vec<Option<[f32; 2]>> = (0..=input_count).map(|slot| {
                let (x, y) = (state.data[DATA_POSITIONS + slot * 2], state.data[DATA_POSITIONS + slot * 2 + 1]);
                (x >= 0 && y >= 0).then_some([x as f32, y as f32])
            }).collect();
            positions.push(Some([RESULT_POSITION.0 as f32, RESULT_POSITION.1 as f32]));
            // The equipment slots never move: the dynamic answer must cover every declared
            // slot or the whole answer is refused.
            positions.extend(EQUIPMENT_POSITIONS.map(Some));
            Some(positions)
        })
        .on_draw(move |draw| {
            let name = draw_field.lock().map(|text| text.clone()).unwrap_or_default();
            draw_station(draw, metal, input_count, icons_at, filled_at, tabs_at, map_at, dormant_at, tabbed_at, &name);
        })
        .on_char(move |c| {
            if c.is_control() { return false; }
            if let Ok(mut text) = char_field.lock() {
                if text.chars().count() < NAME_MAX { text.push(c); }
            }
            true
        })
        .on_key(move |key| {
            match key {
                "backspace" | "delete" => {
                    if let Ok(mut text) = key_field.lock() { text.pop(); }
                    true
                }
                // Enter commits in the original; here the name has nowhere to go (see the
                // header), so the key is simply the field's.
                "enter" | "numpad_enter" => true,
                _ => false,
            }
        })
        .on_click(move |press| {
            if press.right { return None; }
            crate::tabbed_tables::tab_click(press.at, press.data, tabbed_at)
                .or_else(|| tab_at(press.at, press.data, tabs_at, map_at))
        })
        .on_tooltip(move |tip| {
            if let Some(name) = crate::tabbed_tables::tab_tooltip(tip.at, tip.data, tabbed_at) {
                tip.lines.push(name);
                return;
            }
            if let Some(button) = tab_at(tip.at, tip.data, tabs_at, map_at) {
                tip.lines.push(tab_name((button - MAIN_BUTTON) as usize));
            }
        })
        .on_button(move |press| {
            if crate::tabbed_tables::press_tab(press) { return; }
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
            let main = common::station_layouts().and_then(|layouts| layouts.read().ok().map(|layouts| layouts.main_for(name)))
                .unwrap_or_else(|| station_layouts::baked_main(name));
            let mut layout = active_layout(name, selected.as_deref());
            // The tabs this station offers: the main layout and every layout its input count
            // can host, packed in tab order. A selected layout this station cannot host (an
            // anvil's, named by an older save) falls back to the main layout.
            let layouts = common::station_layouts().and_then(|layouts| layouts.read().ok());
            let need = |full: usize| -> usize {
                if full == 0 { return 0; }
                if let (Some(layouts), Ok(id)) = (&layouts, Identifier::parse(&format!("tconstruct:{}", TAB_ORDER[full - 1]))) {
                    if let Some(layout) = layouts.get(&id) { return layout.input_slots.len(); }
                }
                TAB_INPUT_NEED[full - 1]
            };
            let packed: Vec<usize> = (0..TAB_ICONS.len()).filter(|&full| need(full) <= input_count).collect();
            let mut active_full = tab_value(&layout) as usize;
            if !packed.contains(&active_full) {
                layout = main.clone();
                active_full = 0;
            }
            let active_packed = packed.iter().position(|&full| full == active_full).unwrap_or(0);
            tick.set_menu_data(DATA_TAB, active_packed as i32);
            tick.set_menu_data(tabs_at, packed.len() as i32);
            for (index, full) in packed.iter().enumerate() {
                tick.set_menu_data(map_at + index, *full as i32);
            }
            let positions = layout.positions(&main, input_count);
            for (slot, position) in positions.iter().enumerate() {
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
            // The still-filled row: slots the active layout leaves out but that hold an item,
            // in slot order, right to left from (112, 62).
            let mut row = 0;
            for slot in 0..=input_count {
                let hidden_filled = positions.get(slot).copied().flatten().is_none() && tick.items.get(slot).is_some_and(|item| item.is_some());
                let (x, y) = if hidden_filled {
                    let at = (STILL_FILLED.0 - 18 * row, STILL_FILLED.1);
                    row += 1;
                    at
                } else { (-1, -1) };
                tick.set_menu_data(dormant_at + slot * 2, x);
                tick.set_menu_data(dormant_at + slot * 2 + 1, y);
            }
            // The neighbouring tables' strip (tabs only; the station family has no side
            // inventory in the original).
            crate::tabbed_tables::publish(tick, tabbed_at, false);
        }),
    );
}

/// Registers the station family. Called once from the product's hooks.
pub(crate) fn register() {
    register_station("tinker_station", 3);
    register_station("tinkers_anvil", 5);
    register_station("scorched_anvil", 5);
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
        // Menu data with every tab offered, packed in order (the anvil's packing).
        let (tabs_at, map_at) = (100, 101);
        let mut data = vec![0i32; 130];
        data[tabs_at] = TAB_ICONS.len() as i32;
        for full in 0..TAB_ICONS.len() { data[map_at + full] = full as i32; }
        assert_eq!(tab_at([TAB_LEFT + 1.0, TAB_TOP + 1.0], &data, tabs_at, map_at), Some(MAIN_BUTTON));
        assert_eq!(tab_at([TAB_LEFT + TAB_STEP + 1.0, TAB_TOP + 1.0], &data, tabs_at, map_at), Some(MAIN_BUTTON + 1));
        assert_eq!(tab_at([TAB_LEFT + 1.0, TAB_TOP + TAB_STEP + 1.0], &data, tabs_at, map_at), Some(MAIN_BUTTON + 6));
        assert_eq!(tab_at([TAB_LEFT + TAB_SIZE + 2.0, TAB_TOP + 1.0], &data, tabs_at, map_at), None, "the spacing between buttons is not a tab");
        assert_eq!(tab_at([0.0, 0.0], &data, tabs_at, map_at), None, "inside the window is not a tab");
        assert_eq!(TAB_ICONS.len(), TAB_ORDER.len() + 1);
        assert_eq!(TAB_INPUT_NEED.len(), TAB_ORDER.len(), "one input need per tab layout");
        assert_eq!(tab_value(&station_layouts::baked_main("tinker_station")), 0);
        // The tinker station's packing (three inputs) leaves the eight four-input layouts out:
        // packed position 11 is then the longbow's neighbour javelin-free row's plate armor.
        let station_packed: Vec<usize> = (0..TAB_ICONS.len()).filter(|&full| full == 0 || TAB_INPUT_NEED[full - 1] <= 3).collect();
        assert_eq!(station_packed.len(), 13, "main plus the twelve layouts of at most three inputs");
        assert!(station_packed.iter().all(|&full| full == 0 || TAB_INPUT_NEED[full - 1] <= 3));
    }
}
