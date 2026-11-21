//! Modifier worktable block entity, window and result-slot ownership over PlutoLib's public API.
//! SlimeKnights (MIT; see LICENSE) defines ModifierWorktableBlockEntity and the recipes. Slot 0 is the
//! tool, slots 1..=2 the inputs and slot 3 the result. The modifier buttons are published as menu
//! data (slot 0 the selected button, -1 for none, slot 1 the button count); they are numbered because
//! modifier names are not known to the window. Only the removal recipe is wired here; textures,
//! scrolling, the recipe description and the other worktable recipe types are not ported yet.
use crate::{
    common, modifier_removal,
    station_menu::take_with,
    tables::input_consumption::consume_input,
    tool_definitions, tool_parts,
    tool_station::{definition_for, Context},
    worktable_recipes::Removals,
};
use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, FluidItems, FluidStack, I18n, Identifier, ItemSlot, MenuAccess, Nbt, NbtCompound, NbtCompoundExt,
    ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, SlotAction,
};

pub(crate) const TOOL_SLOT: usize = 0;
const INPUT_COUNT: usize = 2;
pub(crate) const RESULT_SLOT: usize = TOOL_SLOT + INPUT_COUNT + 1;
const SLOT_COUNT: usize = RESULT_SLOT + 1;
const MAX_BUTTONS: usize = 16;
/// Menu data: slot 0 the selected button, slot 1 the button count, then one entry per button.
pub(crate) const HEADER: usize = 2;
pub(crate) const DATA_SLOTS: usize = HEADER + MAX_BUTTONS;
const NAME: &str = "modifier_worktable";
const SELECTED: &str = "modifier";
/// Button grid: left, top and size.
const GRID: (f32, f32, f32) = (54.0, 18.0, 18.0);
const COLUMNS: usize = 4;

pub(crate) struct Computation {
    pub buttons: Vec<Identifier>,
    pub selected: Option<usize>,
    pub result: Option<ItemSlot>,
    pub leftovers: Vec<Identifier>,
}

/// Runs `read` over the loaded registries; `None` while any has not loaded.
fn with_context<R>(read: impl FnOnce(&Context, &Removals, &modifier_removal::Salvages, &dyn Fn(&Identifier, &Identifier) -> bool, &dyn Fn(&Identifier, &Identifier) -> bool) -> R) -> Option<R> {
    let definitions = tool_definitions::register().read().ok()?;
    let materials = common::register().read().ok()?;
    let modifiers = common::server_modifiers()?.read().ok()?;
    if !materials.server_loaded || !modifiers.definitions_loaded || definitions.data.is_empty() { return None; }
    let removals = common::removals()?.read().ok()?;
    let salvages = common::salvages()?.read().ok()?;
    if !removals.loaded || !salvages.loaded { return None; }
    let tools: Vec<(Identifier, &tool_definitions::Data)> = definitions.data.iter().map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |id: &Identifier| tool_parts::tool_stack_size(id);
    let item_tags = common::item_snapshot();
    let ctx = Context { materials: &materials, modifiers: &modifiers, item_tags_loaded: item_tags.is_some(), item_tags: item_tags.as_deref(), tools: &tools, item_max: &item_max };
    let in_tag = |item: &Identifier, tag: &Identifier| crate::tool_station::item_in_tag(item_tags.as_deref(), &tools, item, tag);
    let modifier_in_tag = |modifier: &Identifier, tag: &Identifier| modifiers.reverse_tags.get(modifier).is_some_and(|tags| tags.contains(tag));
    Some(read(&ctx, &removals, &salvages, &in_tag, &modifier_in_tag))
}

/// Every modifier by its stable sorted position, the order button data indexes: both sides of a local world read the
/// same registry, so the position names the modifier. (A remote client has no registry yet and shows numbers.)
fn modifier_order() -> Vec<Identifier> {
    common::server_modifiers().and_then(|registry| registry.read().ok()).map(|registry| registry.all_ids()).unwrap_or_default()
}

/// A modifier's display name: its translation (`modifier.<namespace>.<path>`) or, untranslated, its path as words.
fn display_name(modifier: &Identifier) -> String {
    let key = format!("modifier.{}.{}", modifier.namespace(), modifier.path().replace('/', "."));
    let text = I18n::translate(&key);
    if text != key { return text; }
    modifier.path().split(['_', '/']).map(|word| {
        let mut chars = word.chars();
        chars.next().map_or(String::new(), |first| first.to_uppercase().chain(chars).collect())
    }).collect::<Vec<_>>().join(" ")
}

/// What each input slot's stack holds as a fluid container, as PlutoLib reports it.
fn input_contents(items: &[Option<ItemSlot>]) -> Vec<Option<FluidStack>> {
    (1..=INPUT_COUNT).map(|slot| items.get(slot).cloned().flatten().and_then(|stack| FluidItems::contents(&stack))).collect()
}

/// The input stacks' facts for recipe matching: their fluid contents, read against the smeltery's fluid
/// tags (without the smeltery data the tag form cannot match, as the contents then belong to no known tag).
fn input_facts<'a>(held: &'a [Option<FluidStack>], data: Option<&'a crate::smeltery_data::Data>) -> Vec<crate::material_recipes::StackFacts<'a>> {
    held.iter().map(|held| crate::material_recipes::StackFacts {
        fluid: held.as_ref().map(|held| (&held.fluid, held.amount as i32)),
        fluid_tags: data.map(|data| &data.tags),
        ..Default::default()
    }).collect()
}

/// What the slots and the selected modifier craft.
pub(crate) fn compute(items: &[Option<ItemSlot>], selected: Option<&str>) -> Option<Computation> {
    let tool = items.get(TOOL_SLOT).cloned().flatten();
    let inputs: Vec<Option<(Identifier, u32)>> = (1..=INPUT_COUNT).map(|slot| items.get(slot).cloned().flatten().map(|stack| (stack.item, stack.count))).collect();
    let data = common::smeltery_data().and_then(|data| data.read().ok());
    let held = input_contents(items);
    let facts = input_facts(&held, data.as_deref());
    with_context(|ctx, removals, salvages, in_tag, modifier_in_tag| {
        let empty = Computation { buttons: Vec::new(), selected: None, result: None, leftovers: Vec::new() };
        let Some(tool) = tool.as_ref() else { return empty; };
        let Some(recipe) = removals.find_facts(&(tool.item.clone(), tool.count), &inputs, &facts, in_tag) else { return empty; };
        let buttons: Vec<Identifier> = recipe.options(tool, modifier_in_tag).into_iter().take(MAX_BUTTONS).collect();
        let selected_index = selected.and_then(|name| buttons.iter().position(|button| button.to_string() == name));
        let result = selected_index.and_then(|index| {
            let definition = definition_for(ctx, tool)?;
            let env = ctx.environment(&tool.item, definition);
            modifier_removal::remove(tool, &env, &buttons[index], salvages, in_tag).ok()?.ok()
        });
        Computation { buttons, selected: selected_index, result, leftovers: recipe.leftovers.clone() }
    })
}

fn selected_of(data: &NbtCompound) -> Option<String> { data.get_string(SELECTED).map(str::to_owned) }

/// Crafts once on the staged menu: pays the inputs, hands back the leftovers, takes one tool and
/// restages the result. Returns what was made.
fn craft_once(menu: &mut dyn MenuAccess, selected: Option<&str>) -> Option<ItemSlot> {
    let items: Vec<Option<ItemSlot>> = (0..SLOT_COUNT).map(|slot| menu.stack(slot)).collect();
    let computation = compute(&items, selected)?;
    let result = computation.result?;
    // The recipe's input amounts, in slot order (each ingredient takes the first matching slot).
    let amounts = with_context(|_, removals, _, in_tag, _| {
        let tool = items[TOOL_SLOT].as_ref()?;
        let inputs: Vec<Option<(Identifier, u32)>> = (1..=INPUT_COUNT).map(|slot| items[slot].as_ref().map(|stack| (stack.item.clone(), stack.count))).collect();
        let data = common::smeltery_data().and_then(|data| data.read().ok());
        let held = input_contents(&items);
        let facts = input_facts(&held, data.as_deref());
        let recipe = removals.find_facts(&(tool.item.clone(), tool.count), &inputs, &facts, in_tag)?;
        let taken = crate::modifier_recipes::check_match_facts(&recipe.inputs, &inputs, &facts, in_tag)?;
        let mut amounts = vec![0u32; INPUT_COUNT];
        for (slot, ingredient) in taken.into_iter().zip(&recipe.inputs) { amounts[slot] += ingredient.amount; }
        Some(amounts)
    }).flatten()?;
    for item in &computation.leftovers { menu.give_stack(ItemSlot::new(item.clone(), 1)); }
    for (index, amount) in amounts.into_iter().enumerate() {
        if amount == 0 { continue; }
        let slot = index + 1;
        let stack = menu.stack(slot);
        let container = stack.as_ref().and_then(|stack| menu.crafting_remainder(stack));
        let consumption = consume_input(stack.as_ref(), amount, container)?;
        if !menu.set_stack(slot, consumption.slot) { return None; }
        if let Some(give) = consumption.give { menu.give_stack(give); }
    }
    // The tool is replaced by the result: one tool leaves the slot.
    let tool = menu.stack(TOOL_SLOT);
    let consumption = consume_input(tool.as_ref(), 1, None)?;
    if !menu.set_stack(TOOL_SLOT, consumption.slot) { return None; }
    let items: Vec<Option<ItemSlot>> = (0..SLOT_COUNT).map(|slot| menu.stack(slot)).collect();
    menu.set_stack(RESULT_SLOT, compute(&items, selected).and_then(|computation| computation.result));
    Some(result)
}

/// Registers the block entity type and the window. Called once from the product's hooks.
pub(crate) fn register() {
    let id = Identifier::new("tconstruct", NAME).expect("valid worktable id");
    let layout = ScreenLayout::new(176.0, 184.0).inventory_at(8.0, 102.0)
        .slot(8.0, 24.0).slot(8.0, 46.0).slot(8.0, 68.0).slot(148.0, 34.0);
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
        .on_draw(|draw| {
            let selected = draw.data.first().copied().unwrap_or(-1);
            let count = draw.data.get(1).copied().unwrap_or(0).max(0) as usize;
            for index in 0..count.min(MAX_BUTTONS) {
                let (x, y) = (draw.origin[0] + GRID.0 + (index % COLUMNS) as f32 * GRID.2, draw.origin[1] + GRID.1 + (index / COLUMNS) as f32 * GRID.2);
                let color = if selected == index as i32 { 0xFF80C0FF } else { 0xFF8B8B8B };
                draw.canvas.fill([x, y, GRID.2 - 1.0, GRID.2 - 1.0], color);
                draw.canvas.text(&(index + 1).to_string(), [x + 5.0, y + 5.0], 0x202020, false);
            }
        })
        .on_tooltip(move |tip| {
            let count = tip.data.get(1).copied().unwrap_or(0).max(0) as usize;
            if let Some(index) = button_at(tip.at, count.min(MAX_BUTTONS)) {
                let order = modifier_order();
                if let Some(modifier) = tip.data.get(HEADER + index).copied().filter(|value| *value > 0).and_then(|value| order.get(value as usize - 1)) {
                    tip.lines.push(display_name(modifier));
                }
            }
        })
        .on_click(move |press| {
            let count = press.data.get(1).copied().unwrap_or(0).max(0) as usize;
            (!press.right).then(|| button_at(press.at, count.min(MAX_BUTTONS))).flatten().map(|index| index as i32 + 1)
        })
        .on_button(|press| {
            let Some(index) = usize::try_from(press.id - 1).ok() else { return; };
            let selected = selected_of(press.data);
            let Some(computation) = compute(press.items, selected.as_deref()) else { return; };
            match computation.buttons.get(index) {
                Some(modifier) => { press.data.insert(SELECTED.to_owned(), Nbt::String(modifier.to_string())); }
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
        BlockEntityType::new(id).item_slots(SLOT_COUNT as u32).menu("tconstruct:modifier_worktable").ticker(|tick| {
            if tick.items.len() < SLOT_COUNT { return; }
            let selected = selected_of(tick.data);
            let Some(computation) = compute(tick.items, selected.as_deref()) else { return; };
            tick.items[RESULT_SLOT] = computation.result.clone();
            tick.set_menu_data(0, computation.selected.map_or(-1, |index| index as i32));
            tick.set_menu_data(1, computation.buttons.len() as i32);
            let order = modifier_order();
            for index in 0..MAX_BUTTONS {
                let value = computation.buttons.get(index).and_then(|button| order.iter().position(|id| id == button)).map_or(0, |position| position as i32 + 1);
                tick.set_menu_data(HEADER + index, value);
            }
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unloaded_registries_craft_nothing() {
        let items: Vec<Option<ItemSlot>> = vec![None; SLOT_COUNT];
        assert!(compute(&items, None).is_none());
    }
}
