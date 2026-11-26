//! Part builder block entity, window and result-slot ownership over PlutoLib's public API.
//! SlimeKnights (MIT; see LICENSE) defines the recipes and PartBuilderBlockEntity's behavior.
//! Slot 0 holds the material, slot 1 the pattern and slot 2 the result. Pattern buttons are published
//! to the window as menu data: slot 0 the selected button (-1 for none), slot 1 the button count,
//! slot 2 unused, slot 3 which of the material and pattern slots hold an item, and
//! slots 4.. one entry per button (the part's index plus one), so the client needs no recipes.
//! The window is drawn as the original's screen draws it: the `part_builder.png` background, the
//! pattern buttons in a four-by-four grid with the original's faces (the hovered button gets the
//! hover face) and pattern sprites, the scrollbar, and the empty-slot icons. The list scrolls with
//! the mouse wheel and by clicking above or below the scrollbar handle; the offset is the window's
//! own client-side state, as a menu button is the only road back to the server. The side panel
//! (PartInfoPanelScreen) is drawn from the window's synced stacks and the client's material
//! registry: the staged material's name, its stat blocks and its traits; the material value and
//! pattern cost lines are not drawn, as they are computed from the material recipes, which only
//! the server holds.
use std::sync::{Arc, Mutex};

use crate::{
    commands, common, material_recipes::{LeftoverSource, Recipes as MaterialRecipes},
    materials::Definitions,
    part_builder_recipes::{Context, Inventory, Recipes},
    station_menu::{draw_info_panel_sized, item_display_name, material_display_name, modifier_display_name, take_with},
    tables::input_consumption::consume_input,
    tool_parts,
};
use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, I18n, Identifier, ItemSlot, MenuAccess, NbtCompound, NbtCompoundExt, Nbt,
    ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, SlotAction,
};

pub(crate) const MATERIAL_SLOT: usize = 0;
pub(crate) const PATTERN_SLOT: usize = 1;
pub(crate) const RESULT_SLOT: usize = 2;
const SLOT_COUNT: usize = 3;
/// Menu data: selected button, button count, unused, filled slots, then one entry per button.
const DATA_FILLED: usize = 3;
const HEADER: usize = 4;
pub(crate) const MAX_BUTTONS: usize = 60;
pub(crate) const DATA_SLOTS: usize = HEADER + MAX_BUTTONS;
const NAME: &str = "part_builder";
/// The selected pattern, saved in the block entity's data.
const SELECTED: &str = "pattern";
/// Paging directions the scrollbar track's click geometry answers (handled on the client, where
/// the scroll offset lives; never sent as menu buttons).
const SCROLL_UP: i32 = 1000;
const SCROLL_DOWN: i32 = 1001;
/// The window's textures: its own sheet and the shared icon sheet.
const BACKGROUND: &str = "tconstruct:gui/part_builder";
const ICONS: &str = "tconstruct:gui/icons";
/// Button grid on the window: top left, size of a button and columns (`PartBuilderScreen`).
const GRID: (f32, f32, f32) = (51.0, 15.0, 18.0);
const COLUMNS: usize = 4;
/// Buttons visible at once: four columns by four rows.
const VISIBLE: usize = 16;

/// The largest scroll offset for a button count: whole rows past the visible four, in buttons.
fn max_offset(count: usize) -> usize {
    count.div_ceil(COLUMNS).saturating_sub(VISIBLE / COLUMNS) * COLUMNS
}

/// The window's current scroll offset, clamped to what a button count allows.
fn scroll_offset(state: &Mutex<usize>, count: usize) -> usize {
    state.lock().map(|held| (*held).min(max_offset(count))).unwrap_or(0)
}

/// The button at a window point, as its absolute index in the button list.
fn button_at(at: [f32; 2], offset: usize, count: usize) -> Option<usize> {
    let (x, y) = (at[0] - GRID.0, at[1] - GRID.1);
    if x < 0.0 || y < 0.0 { return None; }
    let (column, row) = ((x / GRID.2) as usize, (y / GRID.2) as usize);
    if column >= COLUMNS { return None; }
    let index = offset + row * COLUMNS + column;
    (index < count).then_some(index)
}

/// A click on the scrollbar track, as a paging button id: above the handle pages up, below it
/// pages down, on the handle nothing. `None` anywhere else, and whenever every button fits.
fn scroll_at(at: [f32; 2], offset: usize, count: usize) -> Option<i32> {
    if count <= VISIBLE { return None; }
    if !(126.0..138.0).contains(&at[0]) || !(15.0..87.0).contains(&at[1]) { return None; }
    let handle_top = 15.0 + 59.0 * (offset / COLUMNS) as f32 / (max_offset(count) / COLUMNS) as f32;
    if at[1] < handle_top { Some(SCROLL_UP) } else if at[1] >= handle_top + 15.0 { Some(SCROLL_DOWN) } else { None }
}

/// What the table would craft for the current slots and selection.
pub(crate) struct Computation {
    /// The sorted pattern buttons, as the part index and pattern.
    pub buttons: Vec<Identifier>,
    pub selected: Option<usize>,
    pub crafted: Option<Crafted>,
}

pub(crate) struct Crafted {
    pub result: ItemSlot,
    pub items_used: u32,
    pub leftover: Option<ItemSlot>,
    pub keeps_pattern: bool,
}

/// The patterns that are not parts: the cast shapes the part builder presses into sand and red sand casts (`smeltery/casts/*/builder_*`).
const CAST_PATTERNS: &[&str] = &["ingot", "nugget", "gem", "rod", "plate", "gear", "coin", "wire"];

/// The remaining patterns the damageable recycling results are keyed by (casting shapes that are
/// neither parts nor builder casts); they only ever appear as recycling buttons.
const RECYCLE_PATTERNS: &[&str] = &["block", "crystal", "scale", "shard"];

/// The patterns in a fixed order the window and the server agree on: the material items, then the cast shapes,
/// then the remaining recycling shapes.
pub(crate) fn pattern_order() -> Vec<Identifier> {
    tool_parts::material_items().into_keys().chain(CAST_PATTERNS.iter().chain(RECYCLE_PATTERNS).map(|name| Identifier::new("tconstruct", name).expect("valid pattern id"))).collect()
}

/// Runs `read` over the loaded registries and recipes; `None` while any has not loaded.
fn with_context<R>(read: impl FnOnce(&Context, &Recipes, &MaterialRecipes, &dyn Fn(&Identifier, &Identifier) -> bool, &dyn Fn(&LeftoverSource) -> Option<Identifier>) -> R) -> Option<R> {
    let materials = common::register().read().ok()?;
    if !materials.server_loaded { return None; }
    let part_recipes = common::part_builder_recipes()?.read().ok()?;
    let material_recipes = common::material_recipes()?.read().ok()?;
    if !part_recipes.loaded || !material_recipes.loaded { return None; }
    let item_tags = common::item_snapshot();
    let in_tag = |item: &Identifier, tag: &Identifier| item_tags.as_deref().and_then(|tags| tags.get_tag(tag)).is_some_and(|members| members.contains(item));
    // A tag leftover resolves to the first member of the tag (the host has no mod preference list).
    let first_of = |source: &LeftoverSource| match source {
        LeftoverSource::Item(item) => Some(item.clone()),
        LeftoverSource::Tag(tag) => item_tags.as_deref().and_then(|tags| tags.get_tag(tag)).and_then(|members| members.iter().next().cloned()),
    };
    let parts = tool_parts::material_items();
    let ctx = Context { materials: &materials as &Definitions, parts: &parts, in_tag: &in_tag };
    Some(read(&ctx, &part_recipes, &material_recipes, &in_tag, &first_of))
}

/// Tool recycling: a tool in the material slot and a matching pattern offer the tool's parts as buttons. `None` when the slots are not a
/// recycling recipe (the ordinary part recipes then apply).
fn recycle(items: &[Option<ItemSlot>], selected: Option<&str>) -> Option<Computation> {
    let tool = items.get(MATERIAL_SLOT).cloned().flatten()?;
    let pattern = items.get(PATTERN_SLOT).cloned().flatten()?;
    if !tool.custom_data.contains_key(crate::tool_stack::TAG_MATERIALS) { return None; }
    let definitions = crate::tool_definitions::register().read().ok()?;
    let materials = common::register().read().ok()?;
    if !materials.server_loaded { return None; }
    let recycles = common::recycles()?.read().ok()?;
    if !recycles.loaded { return None; }
    let item_tags = common::item_snapshot();
    let in_tag = |item: &Identifier, tag: &Identifier| item_tags.as_deref().and_then(|tags| tags.get_tag(tag)).is_some_and(|members| members.contains(item));
    let recipe = recycles.find(&tool, &pattern.item, &in_tag)?;
    let definition_parts = definitions.data.get(&tool.item)?.tool_parts();
    let registry = tool_parts::registry();
    let order = pattern_order();
    let buttons: Vec<Identifier> = recipe.buttons(&tool, definition_parts, &registry, &materials).into_iter().filter(|part| order.contains(part)).collect();
    let selected_index = selected.and_then(|name| buttons.iter().position(|button| button.to_string() == name));
    let crafted = selected_index.and_then(|index| {
        let result = recipe.assemble(&tool, &buttons[index], definition_parts, &registry, &materials)?;
        let leftover = recipe.leftover(&tool, &buttons[index], definition_parts, &registry, &materials, &mut crate::part_recycling::random);
        let keeps_pattern = in_tag(&pattern.item, &Identifier::new("tconstruct", "patterns/reusable").expect("valid tag"));
        Some(Crafted { result, items_used: recipe.tools.amount, leftover, keeps_pattern })
    });
    Some(Computation { buttons, selected: selected_index, crafted })
}

/// Damageable recycling: an item with no materials of its own (a vanilla tool, a staff) in the material slot
/// and the recipe's pattern item offer its fixed results as buttons, scaled by the stack's remaining
/// durability. `None` when the slots are not such a recipe (the ordinary part recipes then apply).
fn damageable(items: &[Option<ItemSlot>], selected: Option<&str>) -> Option<Computation> {
    let tool = items.get(MATERIAL_SLOT).cloned().flatten()?;
    let pattern = items.get(PATTERN_SLOT).cloned().flatten()?;
    let recycles = common::damageable_recycles()?.read().ok()?;
    if !recycles.loaded() { return None; }
    let item_tags = common::item_snapshot();
    let in_tag = |item: &Identifier, tag: &Identifier| item_tags.as_deref().and_then(|tags| tags.get_tag(tag)).is_some_and(|members| members.contains(item));
    // The stack's own max damage: its component when it carries one (a Tinkers tool's durability), else the
    // item prototype's (a vanilla item's), else 0 for an item that cannot be damaged.
    let max_damage = tool.max_damage.or_else(|| common::item_prototypes().and_then(|prototypes| prototypes.default_stack(&tool.item)).and_then(|stack| stack.max_damage)).unwrap_or(0);
    let recipe = recycles.find(&tool, &pattern.item, max_damage, &in_tag)?;
    let first_of = |source: &crate::fluid_ingredient::Source| match source {
        crate::fluid_ingredient::Source::Id(item) => Some(item.clone()),
        crate::fluid_ingredient::Source::Tag(tag) => item_tags.as_deref().and_then(|tags| tags.get_tag(tag)).and_then(|members| members.iter().next().cloned()),
    };
    let order = pattern_order();
    let buttons: Vec<Identifier> = recipe.buttons().into_iter().filter(|part| order.contains(part)).collect();
    let selected_index = selected.and_then(|name| buttons.iter().position(|button| button.to_string() == name));
    let crafted = selected_index.and_then(|index| {
        let result = recipe.assemble(&buttons[index], &tool, max_damage, &first_of)?;
        let leftover = recipe.leftover(&buttons[index], &tool, max_damage, &first_of, &mut crate::part_recycling::random);
        let keeps_pattern = in_tag(&pattern.item, &Identifier::new("tconstruct", "patterns/reusable").expect("valid tag"));
        Some(Crafted { result, items_used: 1, leftover, keeps_pattern })
    });
    Some(Computation { buttons, selected: selected_index, crafted })
}

/// What the slots and the selected pattern craft (slots beyond the first two are ignored).
pub(crate) fn compute(items: &[Option<ItemSlot>], selected: Option<&str>) -> Option<Computation> {
    if let Some(recycling) = recycle(items, selected) { return Some(recycling); }
    if let Some(recycling) = damageable(items, selected) { return Some(recycling); }
    let pattern_stack = items.get(PATTERN_SLOT).cloned().flatten();
    let material_stack = items.get(MATERIAL_SLOT).cloned().flatten();
    with_context(|ctx, recipes, material_recipes, in_tag, first_of| {
        let Some(pattern_stack) = pattern_stack.as_ref() else { return Computation { buttons: Vec::new(), selected: None, crafted: None }; };
        let material = material_stack.as_ref().and_then(|stack| {
            let instrument = crate::material_recipes::instrument_of(stack);
            material_recipes.find_facts(&stack.item, in_tag, &crate::material_recipes::StackFacts { instrument: instrument.as_ref(), ..Default::default() })
        });
        let inv = Inventory { pattern_item: Some(&pattern_stack.item), stack: material_stack.as_ref().map(|stack| (&stack.item, stack.count)), material };
        let order = pattern_order();
        // A recipe whose pattern is neither a part nor a cast shape has no button: the window could not name it.
        let buttons: Vec<Identifier> = recipes.buttons(&inv, ctx).into_iter().map(|(pattern, _)| pattern).filter(|pattern| order.contains(pattern)).collect();
        let selected_index = selected.and_then(|name| buttons.iter().position(|button| button.to_string() == name));
        let crafted = selected_index.and_then(|index| {
            let recipe = recipes.find(&buttons[index], &inv, ctx)?;
            let result = recipe.assemble(&inv);
            let items_used = u32::try_from(recipe.items_used(&inv)).unwrap_or(0);
            let leftover = material.and_then(|recipe_for_material| recipe_for_material.leftover_for(recipe.cost)).and_then(|(source, count)| {
                first_of(source).map(|item| ItemSlot::new(item, count))
            });
            let keeps_pattern = in_tag(&pattern_stack.item, &Identifier::new("tconstruct", "patterns/reusable").expect("valid tag"));
            Some(Crafted { result, items_used, leftover, keeps_pattern })
        });
        Computation { buttons, selected: selected_index, crafted }
    })
}

fn selected_of(data: &NbtCompound) -> Option<String> { data.get_string(SELECTED).map(str::to_owned) }

/// Keeps the result slot and the published menu data in step with the slots. Returns whether the slots changed.
pub(crate) fn refresh(items: &mut Vec<Option<ItemSlot>>, selected: Option<&str>) -> (Option<Computation>, bool) {
    if items.len() < SLOT_COUNT { return (None, false); }
    let computation = compute(items, selected);
    let next = computation.as_ref().and_then(|computation| computation.crafted.as_ref()).map(|crafted| crafted.result.clone());
    let changed = items[RESULT_SLOT] != next;
    items[RESULT_SLOT] = next;
    (computation, changed)
}

/// Crafts once on the staged menu: pays the leftover and the inputs, restages the result, returns what was made.
fn craft_once(menu: &mut dyn MenuAccess, selected: Option<&str>) -> Option<ItemSlot> {
    let items: Vec<Option<ItemSlot>> = (0..SLOT_COUNT).map(|slot| menu.stack(slot)).collect();
    let crafted = compute(&items, selected)?.crafted?;
    if let Some(leftover) = crafted.leftover.clone() { menu.give_stack(leftover); }
    for (slot, amount) in [(MATERIAL_SLOT, crafted.items_used), (PATTERN_SLOT, if crafted.keeps_pattern { 0 } else { 1 })] {
        if amount == 0 { continue; }
        let stack = menu.stack(slot);
        let container = stack.as_ref().and_then(|stack| menu.crafting_remainder(stack));
        let consumption = consume_input(stack.as_ref(), amount, container)?;
        if !menu.set_stack(slot, consumption.slot) { return None; }
        if let Some(give) = consumption.give { menu.give_stack(give); }
    }
    let items: Vec<Option<ItemSlot>> = (0..SLOT_COUNT).map(|slot| menu.stack(slot)).collect();
    menu.set_stack(RESULT_SLOT, compute(&items, selected).and_then(|computation| computation.crafted).map(|crafted| crafted.result));
    Some(crafted.result)
}

fn publish(tick: &mut pluto_lib::BlockEntityTick<'_>, computation: &Computation, filled: i32) {
    let order = pattern_order();
    tick.set_menu_data(0, computation.selected.map_or(-1, |index| index as i32));
    let shown = computation.buttons.len().min(MAX_BUTTONS);
    tick.set_menu_data(1, shown as i32);
    tick.set_menu_data(DATA_FILLED, filled);
    for (index, pattern) in computation.buttons.iter().take(shown).enumerate() {
        tick.set_menu_data(HEADER + index, order.iter().position(|known| known == pattern).map_or(0, |position| position as i32 + 1));
    }
}

/// A pattern button's tooltip name: the part item's localized name (the original's
/// `pattern.getDisplayName()`), with the path as words when it has no translation.
fn display_name(pattern: &Identifier) -> String {
    item_display_name(pattern)
}

/// The material the staged stack is, when the client can tell: a part stack names its material in
/// its own data, a tool stack in its material list. A raw material item's material comes from the
/// material recipes, which only the server holds, so it does not resolve here.
fn staged_material(items: &[Option<ItemSlot>]) -> Option<Identifier> {
    let stack = items.get(MATERIAL_SLOT).cloned().flatten()?;
    if let Some(Nbt::String(text)) = stack.custom_data.get("Material") {
        return crate::tool_materials::MaterialVariant::parse(text).and_then(|variant| variant.material_id());
    }
    if stack.custom_data.contains_key(crate::tool_stack::TAG_MATERIALS) {
        return crate::tool_stack::materials(&stack.custom_data, true).as_slice().first().and_then(|variant| variant.material_id());
    }
    None
}

/// What the side panel shows: the staged material's name with its stat blocks and traits from the
/// client's material registry (the selected part's name first when one is selected), or the
/// window's title and the builder's info text while no material resolves.
fn panel_content(items: &[Option<ItemSlot>], selected: Option<&Identifier>) -> (String, Vec<String>) {
    let Some(material) = staged_material(items) else {
        let info = I18n::translate("gui.tconstruct.part_builder.info");
        let lines = if info == "gui.tconstruct.part_builder.info" { Vec::new() } else { vec![info] };
        return (I18n::translate("block.tconstruct.part_builder"), lines);
    };
    let mut lines = Vec::new();
    if let Some(pattern) = selected {
        lines.push(item_display_name(pattern));
    }
    common::with_client_materials(|definitions| {
        let Some(definitions) = definitions else { return };
        for stat in ["head", "handle", "extra"] {
            let stat_id = Identifier::new("tconstruct", stat).expect("valid stat id");
            let Some(stats) = definitions.get_stats(&material, &stat_id) else { continue };
            lines.push(commands::stat_type_name(&stat_id));
            lines.extend(commands::localized_info(stats, &stat_id, 1.0));
            for entry in definitions.get_traits(&material, &stat_id).iter().flatten() {
                lines.push(modifier_display_name(&entry.id));
            }
        }
    });
    (material_display_name(&material), lines)
}

/// Registers the block entity type and the window. Called once from the product's hooks.
pub(crate) fn register() {
    let id = Identifier::new("tconstruct", NAME).expect("valid part builder id");
    let mut layout = ScreenLayout::new(176.0, 184.0).inventory_at(8.0, 102.0).with_own_background()
        .slot(29.0, 43.0).slot(8.0, 43.0).slot(148.0, 42.0);
    // The side inventory's linked range follows the window's own slots (see `tabbed_tables`).
    for _ in 0..crate::tabbed_tables::SIDE_SLOTS { layout = layout.linked_slot(); }
    // The button list's scroll offset: the window's own client-side state, shared by its draw,
    // click, tooltip and wheel callbacks (a menu button cannot carry it back from the server).
    let scroll: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let draw_scroll = scroll.clone();
    let click_scroll = scroll.clone();
    let tip_scroll = scroll.clone();
    let wheel_scroll = scroll.clone();
    let mut window = ScreenHandlerType::new(id.clone(), layout)
        .on_slot_rule(|rule| { if rule.slot == RESULT_SLOT { rule.may_place = false; } })
        .exclude_from_pick_all(RESULT_SLOT)
        .on_menu_links(|context| crate::tabbed_tables::links_answer(context.pos))
        .on_dynamic_slots(|state| {
            let mut positions = vec![Some([29.0, 43.0]), Some([8.0, 43.0]), Some([148.0, 42.0])];
            positions.extend(crate::tabbed_tables::side_positions(state.data, DATA_SLOTS));
            Some(positions)
        })
        .on_draw(move |draw| {
            let canvas = &mut *draw.canvas;
            let (ox, oy) = (draw.origin[0], draw.origin[1]);
            let selected = draw.data.first().copied().unwrap_or(-1);
            let count = draw.data.get(1).copied().unwrap_or(0).max(0) as usize;
            let offset = scroll_offset(&draw_scroll, count);
            canvas.sprite(BACKGROUND, [0.0, 0.0, 176.0, 184.0], [ox, oy, 176.0, 184.0]);
            // The scrollbar handle: the enabled face while buttons are hidden, the disabled one else.
            let scrollable = count > VISIBLE;
            let progress = if scrollable { (offset / COLUMNS) as f32 / (max_offset(count) / COLUMNS) as f32 } else { 0.0 };
            canvas.sprite(BACKGROUND, [if scrollable { 176.0 } else { 188.0 }, 0.0, 12.0, 15.0], [ox + 126.0, oy + 15.0 + 59.0 * progress, 12.0, 15.0]);
            // The empty slots' icons, as Mantle's empty-slot icons draw them over the slot.
            let filled = draw.data.get(DATA_FILLED).copied().unwrap_or(0);
            if filled & 1 == 0 { canvas.sprite(ICONS, [54.0, 234.0, 18.0, 18.0], [ox + 28.0, oy + 42.0, 18.0, 18.0]); }
            if filled & 2 == 0 { canvas.sprite(ICONS, [0.0, 216.0, 18.0, 18.0], [ox + 7.0, oy + 42.0, 18.0, 18.0]); }
            let order = pattern_order();
            let hovered = button_at(draw.mouse, offset, count);
            for cell in 0..VISIBLE {
                let index = offset + cell;
                if index >= count { break; }
                let (x, y) = (ox + GRID.0 + (cell % COLUMNS) as f32 * GRID.2, oy + GRID.1 + (cell / COLUMNS) as f32 * GRID.2);
                let face = if selected == index as i32 { 33.0 } else if hovered == Some(index) { 51.0 } else { 15.0 };
                canvas.sprite(BACKGROUND, [176.0, face, 18.0, 18.0], [x, y, 18.0, 18.0]);
                if let Some(sprite) = draw.data.get(HEADER + index).copied().filter(|value| *value > 0)
                    .and_then(|value| order.get(value as usize - 1))
                    .and_then(|pattern| crate::station_layouts::pattern_sprite_of(pattern)) {
                    canvas.atlas_sprite(sprite, [x + 1.0, y + 1.0, 16.0, 16.0], 0xFFFF_FFFF);
                }
            }
            // The side panel, right of the window at its full height.
            let selected_pattern = if selected >= 0 {
                draw.data.get(HEADER + selected as usize).copied().filter(|value| *value > 0).and_then(|value| order.get(value as usize - 1))
            } else { None };
            let (caption, lines) = panel_content(draw.items, selected_pattern);
            draw_info_panel_sized(canvas, [ox + 178.0, oy], (126.0, 184.0), &caption, &lines);
            // The neighbouring chest's side inventory and the tables' tabs (see `tabbed_tables`).
            crate::tabbed_tables::draw_side_panel(canvas, [ox, oy], draw.data, DATA_SLOTS);
            crate::tabbed_tables::draw_tabs(canvas, [ox, oy], draw.data, DATA_SLOTS, ICONS);
        })
        .on_click(move |press| {
            if press.right { return None; }
            if let Some(id) = crate::tabbed_tables::tab_click(press.at, press.data, DATA_SLOTS) { return Some(id); }
            let count = press.data.get(1).copied().unwrap_or(0).max(0) as usize;
            let offset = scroll_offset(&click_scroll, count);
            if let Some(scroll) = scroll_at(press.at, offset, count) {
                // A track click pages the list here on the client; nothing is sent.
                if let Ok(mut held) = click_scroll.lock() {
                    *held = if scroll == SCROLL_UP { offset.saturating_sub(VISIBLE) } else { (offset + VISIBLE).min(max_offset(count)) };
                }
                return None;
            }
            button_at(press.at, offset, count).map(|index| index as i32 + 1)
        })
        .on_scroll(move |roll| {
            let count = roll.data.get(1).copied().unwrap_or(0).max(0) as usize;
            if count <= VISIBLE { return false; }
            if let Ok(mut held) = wheel_scroll.lock() {
                let current = (*held).min(max_offset(count));
                *held = if roll.delta[1] > 0.0 { current.saturating_sub(COLUMNS) } else if roll.delta[1] < 0.0 { (current + COLUMNS).min(max_offset(count)) } else { current };
            }
            true
        })
        .on_tooltip(move |tip| {
            if let Some(name) = crate::tabbed_tables::tab_tooltip(tip.at, tip.data, DATA_SLOTS) {
                tip.lines.push(name);
                return;
            }
            let count = tip.data.get(1).copied().unwrap_or(0).max(0) as usize;
            let offset = scroll_offset(&tip_scroll, count);
            if let Some(index) = button_at(tip.at, offset, count) {
                let order = pattern_order();
                if let Some(pattern) = tip.data.get(HEADER + index).copied().filter(|value| *value > 0).and_then(|value| order.get(value as usize - 1)) {
                    tip.lines.push(display_name(pattern));
                }
            }
        })
        .on_button(|press| {
            if crate::tabbed_tables::press_tab(press) { return; }
            // The press names a button by its position in the sorted list; resolve it against the current slots.
            let Some(index) = usize::try_from(press.id - 1).ok() else { return; };
            let selected = selected_of(press.data);
            let Some(computation) = compute(press.items, selected.as_deref()) else { return; };
            match computation.buttons.get(index) {
                Some(pattern) => { press.data.insert(SELECTED.to_owned(), Nbt::String(pattern.to_string())); }
                None => { press.data.remove(SELECTED); }
            }
        });
    for action in [SlotAction::Pickup, SlotAction::QuickMove, SlotAction::Swap, SlotAction::Throw] {
        window = window.on_slot_click(RESULT_SLOT, action, |click| {
            let selected = selected_of(click.data);
            take_with(click, RESULT_SLOT, &|menu| craft_once(menu, selected.as_deref()));
        });
    }
    ScreenHandlerRegistry::register(window);
    // The registered slot count is the menu's whole slot space (PlutoLib's `menu` contract):
    // the block's own slots plus the linked range the menu declaration names, or the host
    // drops the menu for a size mismatch.
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(id).item_slots((SLOT_COUNT + crate::tabbed_tables::SIDE_SLOTS) as u32).menu("tconstruct:part_builder").on_place(crate::retextured::keep_texture).ticker(|tick| {
            // Keep the staged stacks' model data in step with their material data, as in the
            // station windows (a stack written outside the stations renders untextured otherwise).
            for slot in tick.items.iter_mut().flatten() {
                crate::tool_materials::heal_model(slot);
            }
            let selected = selected_of(tick.data);
            let (computation, _) = refresh(tick.items, selected.as_deref());
            if let Some(computation) = computation {
                let filled = tick.items.iter().take(2).enumerate()
                    .fold(0i32, |bits, (slot, item)| if item.is_some() { bits | (1 << slot) } else { bits });
                publish(tick, &computation, filled);
            }
            // The tabbed words follow this window's own (see `tabbed_tables`).
            crate::tabbed_tables::publish(tick, DATA_SLOTS, true);
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unloaded_registries_craft_and_publish_nothing() {
        let mut items: Vec<Option<ItemSlot>> = vec![None; SLOT_COUNT];
        items[RESULT_SLOT] = Some(ItemSlot::new(Identifier::parse("test:stale").unwrap(), 1));
        // Without loaded registries there is no computation, so the stale result is left for the next valid tick.
        let (computation, changed) = refresh(&mut items, None);
        assert!(computation.is_none());
        assert!(changed, "the stale result is cleared because there is nothing to craft");
        assert!(items[RESULT_SLOT].is_none());
        assert!(refresh(&mut vec![None; 1], None).0.is_none(), "too few slots are left alone");
        assert!(compute(&items, None).is_none());
    }

    #[test]
    fn pattern_buttons_get_readable_names() {
        assert_eq!(display_name(&Identifier::parse("tconstruct:pick_head").unwrap()), "Pick Head");
        assert_eq!(pattern_order().len(), 40, "27 tool parts, the repair kit, the eight cast shapes and the four remaining recycling shapes");
    }

    #[test]
    fn every_offered_pattern_has_a_window_sprite() {
        for pattern in pattern_order() {
            assert!(crate::station_layouts::pattern_sprite_code(&pattern) > 0, "{pattern}");
        }
    }

    #[test]
    fn every_station_and_chest_block_resolves_its_type_and_window() {
        // The menu-open gate (pluto-lib's right-click bridge) opens a block's window when
        // the block entity registry resolves the block to a type carrying a menu, and the
        // host's open chain then needs the block's window in the screen handler registry.
        // Step 2 found this window — alone among the stations — not opening in-game at
        // 564c574aa, though it opened before wave 3; the audit of the whole chain found
        // the mod side uniform and fault-free (the remaining suspects are host-runtime),
        // so the resolution is pinned here for the whole station/chest family.
        crate::station_menu::register();
        crate::crafting_station::register();
        register();
        crate::worktable_menu::register();
        crate::chests::register();
        for block in ["tconstruct:tinker_station", "tconstruct:tinkers_anvil", "tconstruct:scorched_anvil", "tconstruct:crafting_station", "tconstruct:part_builder", "tconstruct:modifier_worktable", "tconstruct:tinkers_chest", "tconstruct:part_chest", "tconstruct:cast_chest"] {
            assert!(BlockEntityTypeRegistry::get(block).is_some(), "{block} resolves to a block entity type");
            assert!(ScreenHandlerRegistry::get(block).is_some(), "{block} resolves to a window");
        }
    }

    #[test]
    fn buttons_address_the_scrolled_window() {
        assert_eq!(button_at([52.0, 16.0], 0, 40), Some(0));
        assert_eq!(button_at([52.0, 16.0], 16, 40), Some(16));
        assert_eq!(button_at([52.0 + 18.0 * 3.0, 16.0 + 18.0 * 3.0], 0, 40), Some(15));
        assert_eq!(button_at([52.0, 16.0], 0, 0), None);
        assert_eq!(button_at([10.0, 16.0], 0, 40), None, "left of the grid");
    }

    #[test]
    fn scrolling_pages_by_whole_rows() {
        assert_eq!(max_offset(16), 0);
        assert_eq!(max_offset(17), 4);
        assert_eq!(max_offset(40), 24);
        assert_eq!(scroll_at([130.0, 20.0], 0, 40), None, "on the handle");
        assert_eq!(scroll_at([130.0, 80.0], 0, 40), Some(SCROLL_DOWN));
        assert_eq!(scroll_at([130.0, 20.0], 24, 40), Some(SCROLL_UP));
        assert_eq!(scroll_at([100.0, 80.0], 0, 40), None, "outside the track");
        assert_eq!(scroll_at([130.0, 80.0], 0, 16), None, "nothing to scroll");
    }
}
