//! Part builder block entity, window and result-slot ownership over PlutoLib's public API.
//! SlimeKnights (MIT; see LICENSE) defines the recipes and PartBuilderBlockEntity's behavior.
//! Slot 0 holds the material, slot 1 the pattern and slot 2 the result. Pattern buttons are published
//! to the window as menu data: slot 0 the selected button (-1 for none), slot 1 the button count and
//! slots 2.. one entry per button (the part's index plus one), so the client needs no recipes.
//! Window textures, scrolling, the material-value readout and retexturing are presentation work
//! that is not ported yet.
use crate::{
    common, material_recipes::{LeftoverSource, Recipes as MaterialRecipes},
    materials::Definitions,
    part_builder_recipes::{Context, Inventory, Recipes},
    station_menu::take_with,
    tables::input_consumption::consume_input,
    tool_parts,
};
use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, Identifier, ItemSlot, MenuAccess, NbtCompound, NbtCompoundExt, Nbt,
    ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, SlotAction,
};

pub(crate) const MATERIAL_SLOT: usize = 0;
pub(crate) const PATTERN_SLOT: usize = 1;
pub(crate) const RESULT_SLOT: usize = 2;
const SLOT_COUNT: usize = 3;
/// Menu data: selected button, button count, then one entry per button.
const HEADER: usize = 2;
pub(crate) const MAX_BUTTONS: usize = 30;
pub(crate) const DATA_SLOTS: usize = HEADER + MAX_BUTTONS;
const NAME: &str = "part_builder";
/// The selected pattern, saved in the block entity's data.
const SELECTED: &str = "pattern";
/// Button grid on the window: top left, size of a button and columns.
const GRID: (f32, f32, f32) = (34.0, 18.0, 18.0);
const COLUMNS: usize = 5;

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

/// The patterns in a fixed order the window and the server agree on: the material items, then the cast shapes.
pub(crate) fn pattern_order() -> Vec<Identifier> {
    tool_parts::material_items().into_keys().chain(CAST_PATTERNS.iter().map(|name| Identifier::new("tconstruct", name).expect("valid pattern id"))).collect()
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

/// What the slots and the selected pattern craft (slots beyond the first two are ignored).
pub(crate) fn compute(items: &[Option<ItemSlot>], selected: Option<&str>) -> Option<Computation> {
    if let Some(recycling) = recycle(items, selected) { return Some(recycling); }
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

fn publish(tick: &mut pluto_lib::BlockEntityTick<'_>, computation: &Computation) {
    let order = pattern_order();
    tick.set_menu_data(0, computation.selected.map_or(-1, |index| index as i32));
    let shown = computation.buttons.len().min(MAX_BUTTONS);
    tick.set_menu_data(1, shown as i32);
    for (index, pattern) in computation.buttons.iter().take(shown).enumerate() {
        tick.set_menu_data(HEADER + index, order.iter().position(|known| known == pattern).map_or(0, |position| position as i32 + 1));
    }
}

/// A readable name for a pattern button's tooltip.
fn display_name(pattern: &Identifier) -> String {
    pattern.path().split('_').map(|word| {
        let mut chars = word.chars();
        chars.next().map_or(String::new(), |first| first.to_uppercase().chain(chars).collect())
    }).collect::<Vec<_>>().join(" ")
}

/// Registers the block entity type and the window. Called once from the product's hooks.
pub(crate) fn register() {
    let id = Identifier::new("tconstruct", NAME).expect("valid part builder id");
    let layout = ScreenLayout::new(176.0, 214.0).inventory_at(8.0, 132.0)
        .slot(8.0, 24.0).slot(8.0, 44.0).slot(148.0, 34.0);
    let button_at = |at: [f32; 2], count: usize| -> Option<usize> {
        let (x, y) = (at[0] - GRID.0, at[1] - GRID.1);
        if x < 0.0 || y < 0.0 { return None; }
        let (column, row) = ((x / GRID.2) as usize, (y / GRID.2) as usize);
        let index = row * COLUMNS + column;
        (column < COLUMNS && index < count).then_some(index)
    };
    let mut window = ScreenHandlerType::new(id.clone(), layout)
        .on_slot_rule(|rule| { if rule.slot == RESULT_SLOT { rule.may_place = false; } })
        .exclude_from_pick_all(RESULT_SLOT)
        .on_draw(move |draw| {
            let selected = draw.data.first().copied().unwrap_or(-1);
            let count = draw.data.get(1).copied().unwrap_or(0).max(0) as usize;
            for index in 0..count.min(MAX_BUTTONS) {
                let (x, y) = (draw.origin[0] + GRID.0 + (index % COLUMNS) as f32 * GRID.2, draw.origin[1] + GRID.1 + (index / COLUMNS) as f32 * GRID.2);
                let color = if selected == index as i32 { 0xFF80C0FF } else { 0xFF8B8B8B };
                draw.canvas.fill([x, y, GRID.2 - 1.0, GRID.2 - 1.0], color);
                draw.canvas.text(&(index + 1).to_string(), [x + 5.0, y + 5.0], 0x202020, false);
            }
        })
        .on_click(move |press| {
            let count = press.data.get(1).copied().unwrap_or(0).max(0) as usize;
            (!press.right).then(|| button_at(press.at, count.min(MAX_BUTTONS))).flatten().map(|index| index as i32 + 1)
        })
        .on_tooltip(move |tip| {
            let count = tip.data.get(1).copied().unwrap_or(0).max(0) as usize;
            if let Some(index) = button_at(tip.at, count.min(MAX_BUTTONS)) {
                let order = pattern_order();
                if let Some(pattern) = tip.data.get(HEADER + index).copied().filter(|value| *value > 0).and_then(|value| order.get(value as usize - 1)) {
                    tip.lines.push(display_name(pattern));
                }
            }
        })
        .on_button(|press| {
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
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(id).item_slots(SLOT_COUNT as u32).menu("tconstruct:part_builder").ticker(|tick| {
            let selected = selected_of(tick.data);
            let (computation, _) = refresh(tick.items, selected.as_deref());
            if let Some(computation) = computation { publish(tick, &computation); }
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
        assert_eq!(pattern_order().len(), 36, "27 tool parts, the repair kit and the eight cast shapes");
    }
}
