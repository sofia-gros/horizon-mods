//! Modifier worktable block entity, window and result-slot ownership over PlutoLib's public API.
//! SlimeKnights (MIT; see LICENSE) defines ModifierWorktableBlockEntity and the recipes. Slot 0 is the
//! tool, slots 1..=2 the inputs and slot 3 the result. The modifier buttons are published as menu
//! data (slot 0 the selected button, -1 for none, slot 1 the button count, then each button's
//! modifier by its position in the sorted registry); the window draws them on the original's button
//! faces at the original's grid, sixteen at a time, scrolled by the wheel and the slider (the scroll
//! is the window's own, as a menu button press is the only thing a click can send). The side panels
//! carry the tool's materials and modifiers, or the selected modifier's name and description. Two
//! recipe types are wired: the modifier removal
//! recipe, and the enchantment converting recipe (enchantment_converting) — an enchanted book or an
//! enchantable tool gives up one enchantment as a modifier crystal and comes back one level poorer.
//! A stack counts as enchantable here when it is not one of this port's tools (the original's own
//! tools answer false; no PlutoLib API reports another item's answer, and an unenchanted stack
//! offers no options either way). What is not ported: the buttons' per-modifier icons — the
//! original's icon mapping (`tinkering/modifier_icons.json`) and its `gui/modifiers` sprites are
//! not among the shipped assets, and the modifier definitions carry no icon — and the armor stand
//! preview (the host draws no entities in a screen; see the station's note).
use crate::{
    common, enchantment_converting, modifier_removal,
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
/// Every button the ticker publishes; the window shows sixteen at a time and scrolls.
const MAX_BUTTONS: usize = 60;
/// Menu data: slot 0 the selected button, slot 1 the button count, then one entry per button.
pub(crate) const HEADER: usize = 2;
pub(crate) const DATA_SLOTS: usize = HEADER + MAX_BUTTONS;
const NAME: &str = "modifier_worktable";
const SELECTED: &str = "modifier";
const BACKGROUND: &str = "tconstruct:gui/worktable";
/// Button grid: left, top and size (ModifierWorktableScreen's MODIFIER_LEFT/TOP/SIZE).
const GRID: (f32, f32, f32) = (28.0, 15.0, 18.0);
const COLUMNS: usize = 4;
/// How many buttons the grid shows at once: four columns of four.
const VISIBLE: usize = 16;
/// The slider: its track's left and top, the handle's travel and size.
const SLIDER: (f32, f32) = (103.0, 15.0);
const SLIDER_TRAVEL: f32 = 59.0;

pub(crate) struct Computation {
    pub buttons: Vec<Identifier>,
    pub selected: Option<usize>,
    pub result: Option<ItemSlot>,
    pub leftovers: Vec<Identifier>,
    /// The converting recipe and the chosen (modifier, level), when the buttons are a converting
    /// recipe's options rather than a removal's: the craft then pays per level and hands the
    /// disenchanted stack back.
    pub conversion: Option<(enchantment_converting::Recipe, (Identifier, i32))>,
}

/// Runs `read` over the loaded registries; `None` while any has not loaded.
fn with_context<R>(read: impl FnOnce(&Context, &Removals, &enchantment_converting::Conversions, &modifier_removal::Salvages, &dyn Fn(&Identifier, &Identifier) -> bool, &dyn Fn(&Identifier, &Identifier) -> bool, &dyn Fn(Option<&str>, &Identifier) -> bool) -> R) -> Option<R> {
    let definitions = tool_definitions::register().read().ok()?;
    let materials = common::register().read().ok()?;
    let modifiers = common::server_modifiers()?.read().ok()?;
    if !materials.server_loaded || !modifiers.definitions_loaded || definitions.data.is_empty() { return None; }
    let removals = common::removals()?.read().ok()?;
    let conversions = common::conversions()?.read().ok()?;
    let salvages = common::salvages()?.read().ok()?;
    let modifier_recipes = common::modifier_recipes()?.read().ok()?;
    if !removals.loaded || !conversions.loaded || !salvages.loaded || !modifier_recipes.loaded { return None; }
    let tools: Vec<(Identifier, &tool_definitions::Data)> = tool_definitions::item_pairs(&definitions);
    let item_max = |id: &Identifier| tool_parts::tool_stack_size(id);
    let item_tags = common::item_snapshot();
    let ctx = Context { materials: &materials, modifiers: &modifiers, item_tags_loaded: item_tags.is_some(), item_tags: item_tags.as_deref(), tools: &tools, item_max: &item_max };
    let in_tag = |item: &Identifier, tag: &Identifier| crate::tool_station::item_in_tag(item_tags.as_deref(), &tools, item, tag);
    let modifier_in_tag = |modifier: &Identifier, tag: &Identifier| modifiers.reverse_tags.get(modifier).is_some_and(|tags| tags.contains(tag));
    let recipe_modifier = |slot: Option<&str>, modifier: &Identifier| modifier_recipes.is_recipe_modifier(slot, modifier);
    Some(read(&ctx, &removals, &conversions, &salvages, &in_tag, &modifier_in_tag, &recipe_modifier))
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
    with_context(|ctx, removals, conversions, salvages, in_tag, modifier_in_tag, recipe_modifier| {
        let empty = Computation { buttons: Vec::new(), selected: None, result: None, leftovers: Vec::new(), conversion: None };
        let Some(tool) = tool.as_ref() else { return empty; };
        if let Some(recipe) = removals.find_facts(&(tool.item.clone(), tool.count), &inputs, &facts, in_tag) {
            let buttons: Vec<Identifier> = recipe.options(tool, modifier_in_tag).into_iter().take(MAX_BUTTONS).collect();
            let selected_index = selected.and_then(|name| buttons.iter().position(|button| button.to_string() == name));
            let result = selected_index.and_then(|index| {
                let definition = definition_for(ctx, tool)?;
                let env = ctx.environment(&tool.item, definition);
                modifier_removal::remove(tool, &env, &buttons[index], salvages, in_tag).ok()?.ok()
            });
            return Computation { buttons, selected: selected_index, result, leftovers: recipe.leftovers.clone(), conversion: None };
        }
        // Enchantment converting: the tool slot holds an enchanted book or an enchantable stack whose
        // enchantments the map turns into crystal options.
        let enchantable = definition_for(ctx, tool).is_none();
        let Some(recipe) = conversions.find(&tool.item, enchantable, &inputs, &facts, in_tag) else { return empty; };
        let enchantments = tool.enchantments.clone().unwrap_or_default();
        let options = recipe.options(&enchantments, &ctx.modifiers.enchantment_map, modifier_in_tag, recipe_modifier);
        let buttons: Vec<Identifier> = options.iter().map(|(modifier, _)| modifier.clone()).take(MAX_BUTTONS).collect();
        let selected_index = selected.and_then(|name| buttons.iter().position(|button| button.to_string() == name));
        let (result, conversion) = match selected_index {
            Some(index) => (recipe.result(&options[index], &inputs, &facts, in_tag), Some((recipe.clone(), options[index].clone()))),
            None => (None, None),
        };
        Computation { buttons, selected: selected_index, result, leftovers: Vec::new(), conversion }
    })
}

fn selected_of(data: &NbtCompound) -> Option<String> { data.get_string(SELECTED).map(str::to_owned) }

/// The slots a converting craft pays, per ingredient: the assignment `Recipe::result` itself checks —
/// a distinct slot holding the ingredient at amount × level (the original consumes the inputs once
/// per level converted). Returns (slot, amount) pairs in ingredient order.
fn conversion_payment(recipe: &enchantment_converting::Recipe, level: i32, inputs: &[Option<(Identifier, u32)>],
                      facts: &[crate::material_recipes::StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<Vec<(usize, u32)>> {
    let level = u32::try_from(level.max(1)).ok()?;
    let mut used = vec![false; inputs.len()];
    let mut payment = Vec::with_capacity(recipe.inputs.len());
    for ingredient in &recipe.inputs {
        let needed = ingredient.amount.saturating_mul(level);
        let slot = (0..inputs.len()).find(|&slot| !used[slot] && inputs[slot].as_ref().is_some_and(|stack| {
            stack.1 >= needed && ingredient.ingredient.matches_facts(&stack.0, in_tag, &facts.get(slot).copied().unwrap_or_default())
        }))?;
        used[slot] = true;
        payment.push((slot, needed));
    }
    Some(payment)
}

/// A converting craft: pays the inputs once per level, takes one tool, hands the disenchanted stack
/// back (the original's giveItem return) and restages the crystal. Returns what was made.
fn craft_converting(menu: &mut dyn MenuAccess, selected: Option<&str>, recipe: &enchantment_converting::Recipe, chosen: &(Identifier, i32)) -> Option<ItemSlot> {
    let items: Vec<Option<ItemSlot>> = (0..SLOT_COUNT).map(|slot| menu.stack(slot)).collect();
    let tool = items[TOOL_SLOT].clone()?;
    let result = compute(&items, selected)?.result?;
    let inputs: Vec<Option<(Identifier, u32)>> = (1..=INPUT_COUNT).map(|slot| items[slot].as_ref().map(|stack| (stack.item.clone(), stack.count))).collect();
    let data = common::smeltery_data().and_then(|data| data.read().ok());
    let held = input_contents(&items);
    let facts = input_facts(&held, data.as_deref());
    let payment = with_context(|_, _, _, _, in_tag, _, _| conversion_payment(recipe, chosen.1, &inputs, &facts, in_tag)).flatten()?;
    for (index, amount) in payment {
        let slot = index + 1;
        let stack = menu.stack(slot);
        let container = stack.as_ref().and_then(|stack| menu.crafting_remainder(stack));
        let consumption = consume_input(stack.as_ref(), amount, container)?;
        if !menu.set_stack(slot, consumption.slot) { return None; }
        if let Some(give) = consumption.give { menu.give_stack(give); }
    }
    // shrinkToolSlotBy is 1 for this recipe type: one tool is converted per craft.
    let consumption = consume_input(Some(&tool), 1, None)?;
    if !menu.set_stack(TOOL_SLOT, consumption.slot) { return None; }
    // The disenchanted return: the same stack one level poorer in the converted enchantment — a book
    // left with none becomes a plain book (its custom name does not survive the change of item, as a
    // slot carries no name).
    let returned = with_context(|ctx, _, _, _, _, _, _| {
        recipe.unenchanted_return(&tool.item, &tool.enchantments.clone().unwrap_or_default(), &ctx.modifiers.enchantment_map, &chosen.0)
    }).flatten();
    if let Some(returned) = returned {
        let mut stack = tool.clone();
        stack.count = 1;
        if returned.item != tool.item { stack = ItemSlot::new(returned.item.clone(), 1); }
        stack.enchantments = (!returned.enchantments.is_empty()).then_some(returned.enchantments);
        menu.give_stack(stack);
    }
    let items: Vec<Option<ItemSlot>> = (0..SLOT_COUNT).map(|slot| menu.stack(slot)).collect();
    menu.set_stack(RESULT_SLOT, compute(&items, selected).and_then(|computation| computation.result));
    Some(result)
}

/// Crafts once on the staged menu: pays the inputs, hands back the leftovers, takes one tool and
/// restages the result. Returns what was made.
fn craft_once(menu: &mut dyn MenuAccess, selected: Option<&str>) -> Option<ItemSlot> {
    let items: Vec<Option<ItemSlot>> = (0..SLOT_COUNT).map(|slot| menu.stack(slot)).collect();
    let computation = compute(&items, selected)?;
    let result = computation.result?;
    if let Some((recipe, chosen)) = &computation.conversion {
        return craft_converting(menu, selected, recipe, chosen);
    }
    // The recipe's input amounts, in slot order (each ingredient takes the first matching slot).
    let amounts = with_context(|_, removals, _, _, in_tag, _, _| {
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

/// The furthest the button window can scroll, in buttons: whole rows past the visible sixteen.
fn max_offset(count: usize) -> usize { count.div_ceil(COLUMNS).saturating_sub(VISIBLE / COLUMNS) * COLUMNS }

/// The window's scroll offset for a button count, clamped to what the count allows.
fn scroll_offset(state: &std::sync::Mutex<usize>, count: usize) -> usize {
    state.lock().map(|held| *held).unwrap_or(0).min(max_offset(count))
}

/// The button under a window point, through the scroll offset.
fn button_at(at: [f32; 2], count: usize, offset: usize) -> Option<usize> {
    let (x, y) = (at[0] - GRID.0, at[1] - GRID.1);
    if x < 0.0 || y < 0.0 { return None; }
    let (column, row) = ((x / GRID.2) as usize, (y / GRID.2) as usize);
    if column >= COLUMNS || row >= VISIBLE / COLUMNS { return None; }
    let index = offset + row * COLUMNS + column;
    (index < count).then_some(index)
}

/// The side panels' contents: the result (or the tool) with its materials, then the selected
/// modifier's name and description — or, with a tool in hand and nothing selected, its modifiers;
/// with nothing staged, the table's own info line.
fn panels(items: &[Option<ItemSlot>], selected_modifier: Option<&Identifier>) -> ((String, Vec<String>), (String, Vec<String>)) {
    let subject = items.get(RESULT_SLOT).cloned().flatten().or_else(|| items.get(TOOL_SLOT).cloned().flatten());
    let mut first = (String::new(), Vec::new());
    if let Some(subject) = &subject {
        let data = &subject.custom_data;
        first = if crate::tool_stack::is_initialized(data) {
            (crate::station_menu::item_display_name(&subject.item), crate::tool_stack::materials(data, true).as_slice().iter()
                .filter_map(|variant| variant.material_id()).map(|material| crate::station_menu::material_display_name(&material)).collect())
        } else {
            (crate::station_menu::item_display_name(&subject.item), Vec::new())
        };
    }
    if let Some(modifier) = selected_modifier {
        let mut lines = vec![display_name(modifier)];
        let key = format!("modifier.{}.{}.description", modifier.namespace(), modifier.path().replace('/', "."));
        let text = I18n::translate(&key);
        if text != key { lines.push(text); }
        return (first, (I18n::translate("gui.tconstruct.tinker_station.modifiers"), lines));
    }
    if let Some(subject) = &subject {
        if crate::tool_stack::is_initialized(&subject.custom_data) {
            let modifiers: Vec<String> = crate::tool_stack::upgrades(&subject.custom_data).entries().iter().filter(|entry| entry.level > 0)
                .map(|entry| {
                    let name = crate::station_menu::modifier_display_name(&entry.id);
                    if entry.level > 1 { format!("{name} {}", entry.level) } else { name }
                }).collect();
            return (first, (I18n::translate("gui.tconstruct.tinker_station.modifiers"), modifiers));
        }
    }
    let info = I18n::translate("gui.tconstruct.modifier_worktable.info");
    (first, (String::new(), if info == "gui.tconstruct.modifier_worktable.info" { Vec::new() } else { vec![info] }))
}

/// Registers the block entity type and the window. Called once from the product's hooks.
pub(crate) fn register() {
    let id = Identifier::new("tconstruct", NAME).expect("valid worktable id");
    let layout = ScreenLayout::new(176.0, 184.0).inventory_at(8.0, 102.0).with_own_background()
        .slot(8.0, 24.0).slot(8.0, 46.0).slot(8.0, 68.0).slot(148.0, 34.0);
    // The scroll is the window's own state, shared by its draw, click, tooltip and wheel: a menu
    // button press is the only thing a click can send, and scrolling sends nothing.
    let scroll = std::sync::Arc::new(std::sync::Mutex::new(0usize));
    let scroll_draw = scroll.clone();
    let scroll_tip = scroll.clone();
    let scroll_click = scroll.clone();
    let scroll_wheel = scroll.clone();
    let mut window = ScreenHandlerType::new(id.clone(), layout)
        .on_slot_rule(|rule| { if rule.slot == RESULT_SLOT { rule.may_place = false; } })
        .exclude_from_pick_all(RESULT_SLOT)
        .on_draw(move |draw| {
            let selected = draw.data.first().copied().unwrap_or(-1);
            let count = draw.data.get(1).copied().unwrap_or(0).max(0) as usize;
            draw.canvas.sprite(BACKGROUND, [0.0, 0.0, 176.0, 184.0], [draw.origin[0], draw.origin[1], 176.0, 184.0]);
            // the pattern ghosts in the empty tool and input slots (the port's pattern item is
            // one item for every pattern, so the three glyphs of the original draw as it)
            if let Ok(pattern) = Identifier::new("tconstruct", "pattern") {
                for (slot, at) in [(TOOL_SLOT, [8.0, 24.0]), (1, [8.0, 46.0]), (2, [8.0, 68.0])] {
                    if draw.items.get(slot).cloned().flatten().is_none() {
                        draw.canvas.item(&ItemSlot::new(pattern.clone(), 1), [draw.origin[0] + at[0] + 1.0, draw.origin[1] + at[1] + 1.0], 16.0);
                    }
                }
            }
            let offset = scroll_offset(&scroll_draw, count);
            for visible in 0..VISIBLE {
                let index = offset + visible;
                if index >= count { break; }
                let (cell_x, cell_y) = (GRID.0 + (visible % COLUMNS) as f32 * GRID.2, GRID.1 + (visible / COLUMNS) as f32 * GRID.2);
                let hovered = draw.mouse[0] >= cell_x && draw.mouse[0] < cell_x + GRID.2 && draw.mouse[1] >= cell_y && draw.mouse[1] < cell_y + GRID.2;
                let v = if selected == index as i32 { 33.0 } else if hovered { 51.0 } else { 15.0 };
                draw.canvas.sprite(BACKGROUND, [176.0, v, 18.0, 18.0], [draw.origin[0] + cell_x, draw.origin[1] + cell_y, 18.0, 18.0]);
            }
            let max = max_offset(count);
            let progress = if max > 0 { offset as f32 / max as f32 } else { 0.0 };
            draw.canvas.sprite(BACKGROUND, [if max > 0 { 176.0 } else { 188.0 }, 0.0, 12.0, 15.0],
                [draw.origin[0] + SLIDER.0, draw.origin[1] + SLIDER.1 + SLIDER_TRAVEL * progress, 12.0, 15.0]);
            let order = modifier_order();
            let selected_modifier = (selected >= 0).then(|| draw.data.get(HEADER + selected as usize).copied().filter(|value| *value > 0)
                .and_then(|value| order.get(value as usize - 1))).flatten();
            let (first, second) = panels(draw.items, selected_modifier);
            crate::station_menu::draw_info_panel(draw.canvas, [draw.origin[0] + 178.0, draw.origin[1] + 11.0], &first.0, &first.1);
            crate::station_menu::draw_info_panel(draw.canvas, [draw.origin[0] + 178.0, draw.origin[1] + 98.0], &second.0, &second.1);
        })
        .on_scroll(move |event| {
            let count = event.data.get(1).copied().unwrap_or(0).max(0) as usize;
            if count <= VISIBLE { return false; }
            let Ok(mut held) = scroll_wheel.lock() else { return false };
            let current = (*held).min(max_offset(count));
            let next = if event.delta[1] > 0.0 { current.saturating_sub(COLUMNS) } else { (current + COLUMNS).min(max_offset(count)) };
            let changed = next != current;
            *held = next;
            changed
        })
        .on_tooltip(move |tip| {
            let count = tip.data.get(1).copied().unwrap_or(0).max(0) as usize;
            if let Some(index) = button_at(tip.at, count, scroll_offset(&scroll_tip, count)) {
                let order = modifier_order();
                if let Some(modifier) = tip.data.get(HEADER + index).copied().filter(|value| *value > 0).and_then(|value| order.get(value as usize - 1)) {
                    tip.lines.push(display_name(modifier));
                }
            }
        })
        .on_click(move |press| {
            let count = press.data.get(1).copied().unwrap_or(0).max(0) as usize;
            // the slider track pages the window a row at a time, on this side only
            if !press.right && press.at[0] >= SLIDER.0 && press.at[0] < SLIDER.0 + 12.0 && press.at[1] >= SLIDER.1 && press.at[1] < SLIDER.1 + 74.0 {
                if let Ok(mut held) = scroll_click.lock() {
                    let current = (*held).min(max_offset(count));
                    let handle = SLIDER.1 + SLIDER_TRAVEL * if max_offset(count) > 0 { current as f32 / max_offset(count) as f32 } else { 0.0 };
                    *held = if press.at[1] < handle { current.saturating_sub(COLUMNS) } else { (current + COLUMNS).min(max_offset(count)) };
                }
                return None;
            }
            (!press.right).then(|| button_at(press.at, count, scroll_offset(&scroll_click, count))).flatten().map(|index| index as i32 + 1)
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

    #[test]
    fn the_button_grid_sits_at_the_original_place_and_scrolls_by_rows() {
        assert_eq!(max_offset(16), 0, "sixteen buttons fit the grid");
        assert_eq!(max_offset(17), 4, "one more row scrolls in");
        assert_eq!(max_offset(60), 44);
        // the first cell is the first button; the offset shifts the window a row at a time
        assert_eq!(button_at([GRID.0 + 1.0, GRID.1 + 1.0], 40, 0), Some(0));
        assert_eq!(button_at([GRID.0 + 1.0, GRID.1 + 1.0], 40, 4), Some(4));
        assert_eq!(button_at([GRID.0 + 19.0, GRID.1 + 19.0], 40, 0), Some(5), "second row, second column");
        assert_eq!(button_at([GRID.0 + 1.0, GRID.1 + 73.0], 40, 0), None, "below the four visible rows");
        assert_eq!(button_at([GRID.0 + 1.0, GRID.1 + 1.0], 3, 4), None, "scrolled past the end of a short list");
    }
}
