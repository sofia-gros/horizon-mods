//! What the Tinker Station makes of the stacks in its slots, as a function that does not touch the game: the slots go in, the result and
//! what it consumes come out. Two operations so far: building a tool from parts, and upgrading a tool with a modifier recipe's items.
use pluto_lib::{Nbt, NbtCompoundExt, StackData};
use tconstruct_library::materials::MaterialId;
use tconstruct_library::resource::ResourceId;

use crate::tools::{library, Ingredient, ToolLibrary};

/// What a station would make: the result stack and, for each slot, how many of its items it takes.
#[derive(Clone, Debug, PartialEq)]
pub struct StationPlan {
    pub result: StackData,
    pub consumed: Vec<(usize, u32)>,
}

/// Decides whether a stack is in an item tag.
pub type InTag<'a> = &'a dyn Fn(&StackData, &str) -> bool;

fn is_tool(stack: &StackData) -> bool {
    stack.custom_data.contains_key("tconstruct:stats")
}

impl ToolLibrary {
    /// The plan for the stacks in a station's slots (None for an empty slot): an upgrade when a tool is among them, else a tool built of the
    /// parts. Err with the reason when nothing can be made.
    pub fn station_plan(&self, slots: &[Option<StackData>], in_tag: InTag<'_>) -> Result<StationPlan, String> {
        match slots.iter().position(|s| s.as_ref().is_some_and(is_tool)) {
            Some(tool) => self.upgrade_plan(slots, tool, in_tag),
            None => self.build_plan(slots),
        }
    }

    fn build_plan(&self, slots: &[Option<StackData>]) -> Result<StationPlan, String> {
        let mut found = Vec::new(); // (slot, part item, material)
        for (slot, stack) in slots.iter().enumerate() {
            let Some(stack) = stack else { continue };
            let Some(Nbt::String(material)) = stack.custom_data.get("Material") else { continue };
            let (Some(item), Some(material)) = (ResourceId::try_parse(&stack.item), MaterialId::try_parse(material)) else { continue };
            if tconstruct_library::tools::definition::part_stat_type(&item).is_none() {
                continue;
            }
            found.push((slot, item, material));
        }
        let available: Vec<(ResourceId, MaterialId)> = found.iter().map(|(_, item, material)| (item.clone(), material.clone())).collect();
        let (tool, used) = self.tool_for_parts(&available).ok_or("no tool can be made of these parts")?;
        let materials = used.iter().map(|i| found[*i].2.clone()).collect();
        let result = self.make(&tool, materials, &[])?;
        Ok(StationPlan { result, consumed: used.iter().map(|i| (found[*i].0, 1)).collect() })
    }

    fn upgrade_plan(&self, slots: &[Option<StackData>], tool_slot: usize, in_tag: InTag<'_>) -> Result<StationPlan, String> {
        let tool = slots[tool_slot].as_ref().ok_or("no tool")?;
        let mut modifiers: Vec<_> = self.recipes.keys().collect();
        modifiers.sort_by_key(|m| m.to_string());
        let mut first_error = String::from("no modifier recipe fits these items");
        for modifier in modifiers {
            // the items still free in each slot; a recipe takes from them only when all of its inputs can be paid
            let mut left: Vec<u32> = slots.iter().enumerate().map(|(i, s)| if i == tool_slot { 0 } else { s.as_ref().map_or(0, |s| s.count) }).collect();
            let start = left.clone();
            let mut pay = |inputs: &[(Ingredient, u32)]| -> Result<(), String> {
                let mut trial = left.clone();
                for (what, need) in inputs {
                    let mut need = *need;
                    for (i, stack) in slots.iter().enumerate() {
                        let Some(stack) = stack else { continue };
                        let matches = match what {
                            Ingredient::Item(item) => stack.item == *item,
                            Ingredient::Tag(tag) => in_tag(stack, tag.trim_start_matches('#')),
                        };
                        if matches && trial[i] > 0 && need > 0 {
                            let take = trial[i].min(need);
                            trial[i] -= take;
                            need -= take;
                        }
                    }
                    if need > 0 {
                        return Err(format!("{need} more of {what:?} needed"));
                    }
                }
                left = trial;
                Ok(())
            };
            match self.upgrade_paying(tool, &modifier.to_string(), &mut pay) {
                Ok(result) => {
                    let mut consumed: Vec<(usize, u32)> = start.iter().zip(&left).enumerate().filter(|(_, (a, b))| a != b).map(|(i, (a, b))| (i, a - b)).collect();
                    consumed.insert(0, (tool_slot, 1));
                    return Ok(StationPlan { result, consumed });
                }
                Err(error) => {
                    // the first modifier whose recipe the items nearly match is the most useful reason; keep the first error otherwise
                    if first_error.starts_with("no modifier") && !error.contains("recipe gives") {
                        first_error = error;
                    }
                }
            }
        }
        Err(first_error)
    }
}

/// The plan for the stacks in a station's slots, over the library built into the mod.
pub fn plan(slots: &[Option<StackData>], in_tag: InTag<'_>) -> Result<StationPlan, String> {
    library().station_plan(slots, in_tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(item: &str, material: &str) -> Option<StackData> {
        let mut stack = StackData { item: item.to_owned(), count: 1, ..Default::default() };
        stack.custom_data.insert("Material".to_owned(), Nbt::String(material.to_owned()));
        Some(stack)
    }

    fn quartz(count: u32) -> Option<StackData> {
        Some(StackData { item: "minecraft:quartz".to_owned(), count, ..Default::default() })
    }

    fn tags(stack: &StackData, tag: &str) -> bool {
        tag == "forge:gems/quartz" && stack.item == "minecraft:quartz"
    }

    #[test]
    fn three_parts_build_a_pickaxe_and_each_is_consumed_once() {
        let slots = vec![part("tconstruct:pick_head", "tconstruct:iron"), part("tconstruct:tool_handle", "tconstruct:wood"), part("tconstruct:tool_binding", "tconstruct:flint"), None];
        let plan = plan(&slots, &tags).unwrap();
        assert_eq!(plan.result.item, "tconstruct:pickaxe");
        assert_eq!(plan.consumed, vec![(0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn a_tool_and_enough_quartz_make_a_sharper_tool_and_too_little_does_not() {
        let built = plan(&[part("tconstruct:pick_head", "tconstruct:iron"), part("tconstruct:tool_handle", "tconstruct:wood"), part("tconstruct:tool_binding", "tconstruct:flint")], &tags).unwrap().result;
        let rich = plan(&[Some(built.clone()), quartz(40), None], &tags).unwrap();
        assert_eq!(rich.consumed, vec![(0, 1), (1, 36)]);
        assert!(rich.result.custom_data.get("tconstruct:modifiers").is_some_and(|m| format!("{m:?}").contains("sharpness#1")));
        assert!(plan(&[Some(built), quartz(10)], &tags).is_err());
    }

    #[test]
    fn nothing_in_the_slots_makes_nothing() {
        assert!(plan(&[None, None], &tags).is_err());
    }
}

fn slot_to_stack(slot: &pluto_lib::ItemSlot) -> StackData {
    StackData { item: slot.item.to_string(), count: slot.count, damage: slot.damage, max_damage: slot.max_damage, custom_data: slot.custom_data.clone() }
}

fn stack_to_slot(stack: &StackData) -> Option<pluto_lib::ItemSlot> {
    Some(pluto_lib::ItemSlot { item: pluto_lib::Identifier::parse(&stack.item).ok()?, count: stack.count, damage: stack.damage, max_damage: stack.max_damage, custom_data: stack.custom_data.clone() })
}

/// The wooden station has a tool slot and three inputs; its separate result is last in the menu.
const RESULT_SLOT: usize = 4;

struct StationLayout {
    name: String,
    slots: [Option<[f32; 2]>; 4],
}

fn layouts() -> &'static [StationLayout] {
    static ALL: std::sync::OnceLock<Vec<StationLayout>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let mut layouts = Vec::new();
        for &(kind, id, json) in crate::tinkering_data::FILES {
            if kind != "station_layouts" { continue; }
            let json: serde_json::Value = serde_json::from_str(json).expect("generated station layout");
            let inputs = json["input_slots"].as_array().expect("layout inputs");
            if inputs.len() > 3 || id.ends_with(":scorched_anvil") { continue; }
            let position = |v: &serde_json::Value| Some([v["x"].as_f64()? as f32, v["y"].as_f64()? as f32]);
            let mut slots = [None; 4];
            slots[0] = position(&json["tool_slot"]);
            for (i, input) in inputs.iter().enumerate() { slots[i + 1] = position(input); }
            layouts.push((json["sortIndex"].as_i64().unwrap_or(0), id.to_owned(), StationLayout {
                name: id.trim_start_matches("tconstruct:").replace('_', " "), slots,
            }));
        }
        layouts.sort_by(|a, b| (a.1 != "tconstruct:tinker_station", a.0, &a.1).cmp(&(b.1 != "tconstruct:tinker_station", b.0, &b.1)));
        layouts.into_iter().map(|(_, _, layout)| layout).collect()
    })
}

fn slot_positions(selection: usize, occupied: u32) -> Vec<Option<[f32; 2]>> {
    let layout = &layouts()[selection.min(layouts().len() - 1)];
    let mut dormant = 0;
    let mut slots: Vec<_> = layout.slots.iter().enumerate().map(|(i, slot)| {
        if slot.is_some() { return *slot; }
        // TinkerStationScreen's still-filled row: do not strand an item when its layout is no longer active.
        let at = [112.0 - 18.0 * dormant as f32, 62.0];
        dormant += 1;
        (occupied & (1 << i) != 0).then_some(at)
    }).collect();
    slots.push(Some([114.0, 38.0]));
    slots
}

fn selected(data: &[i32]) -> usize {
    data.first().copied().unwrap_or(0).max(0) as usize
}

/// Every tick of a Tinker Station block entity: puts what the inputs make in the result slot, and when the player has taken the result,
/// takes the inputs it was made of.
fn station_tick(tick: &mut pluto_lib::BlockEntityTick<'_>) {
    if tick.items.len() <= RESULT_SLOT {
        tick.items.resize(RESULT_SLOT + 1, None);
    }
    let occupied = tick.items[..RESULT_SLOT].iter().enumerate().fold(0, |mask, (i, slot)| mask | if slot.is_some() { 1 << i } else { 0 });
    tick.set_menu_data(1, occupied);
    let selection = tick.data.get_int("layout", 0).max(0) as usize;
    tick.set_menu_data(0, selection.min(layouts().len() - 1) as i32);
    let inputs: Vec<Option<StackData>> = tick.items[..RESULT_SLOT].iter().map(|s| s.as_ref().map(slot_to_stack)).collect();
    let plan = if inputs[0].as_ref().is_some_and(is_tool) {
        library().upgrade_plan(&inputs, 0, &|stack, tag| library().in_forge_tag(&stack.item, tag))
    } else if inputs[0].is_none() {
        library().build_plan(&inputs[1..]).map(|mut plan| {
            for (slot, _) in &mut plan.consumed { *slot += 1; }
            plan
        })
    } else {
        Err("the tool slot contains no modifiable tool".to_owned())
    };
    let made = tick.data.get_int("made", 0) == 1;
    if made && tick.items[RESULT_SLOT].is_none() {
        // the result was taken: the inputs it was made of go
        if let Ok(plan) = plan {
            for (slot, count) in plan.consumed {
                if let Some(held) = tick.items[slot].as_mut() {
                    held.count = held.count.saturating_sub(count);
                    if held.count == 0 {
                        tick.items[slot] = None;
                    }
                }
            }
        }
        println!("tconstruct: station block took its inputs");
        tick.data.insert("made".to_owned(), Nbt::Int(0));
        return;
    }
    match plan.ok().and_then(|plan| stack_to_slot(&plan.result)) {
        Some(result) => {
            if tick.items[RESULT_SLOT].as_ref() != Some(&result) {
                println!("tconstruct: station block made {}", result.item);
                tick.items[RESULT_SLOT] = Some(result);
            }
            tick.data.insert("made".to_owned(), Nbt::Int(1));
        }
        None => {
            if made {
                tick.items[RESULT_SLOT] = None;
            }
            tick.data.insert("made".to_owned(), Nbt::Int(0));
        }
    }
}

/// Draws the Tinker Station's window: the GUI texture's 176 x 184 panel, a frame behind each slot and the two labels.
fn draw_window(draw: &mut pluto_lib::ScreenDraw<'_>) {
    let [x, y] = draw.origin;
    if !draw.canvas.sprite("tconstruct:gui/tinker_station", [0.0, 0.0, 176.0, 184.0], [x, y, 176.0, 184.0]) {
        draw.canvas.fill([x, y, 176.0, 184.0], 0xFFC6C6C6);
    }
    for [sx, sy] in slot_positions(selected(draw.data), draw.data.get(1).copied().unwrap_or(0) as u32).into_iter().flatten() {
        let (fx, fy) = (x + sx - 1.0, y + sy - 1.0);
        draw.canvas.fill([fx, fy, 18.0, 18.0], 0xFF373737);
        draw.canvas.fill([fx + 1.0, fy + 1.0, 17.0, 17.0], 0xFFFFFFFF);
        draw.canvas.fill([fx + 1.0, fy + 1.0, 16.0, 16.0], 0xFF8B8B8B);
    }
    for (i, _) in layouts().iter().enumerate() {
        let bx = x - 116.0 + (i % 6) as f32 * 18.0;
        let by = y + 8.0 + (i / 6) as f32 * 18.0;
        draw.canvas.fill([bx, by, 17.0, 17.0], if selected(draw.data) == i { 0xFF88BB88 } else { 0xFFAAAAAA });
        draw.canvas.text(&(i + 1).to_string(), [bx + 3.0, by + 4.0], 0x202020, false);
    }
    draw.canvas.text(&layouts()[selected(draw.data).min(layouts().len() - 1)].name, [x + 8.0, y + 6.0], 0x404040, false);
    draw.canvas.text("Inventory", [x + 8.0, y + 90.0], 0x404040, false);
}

/// Registers the Tinker Station's menu and block entity: PlutoLib owns both, the legacy station code is not involved.
pub fn register() {
    use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout};
    let id = Identifier::new("tconstruct", "tinker_station").unwrap();
    let mut layout = ScreenLayout::new(176.0, 184.0).with_own_background().with_own_labels().inventory_at(8.0, 102.0);
    layout.slots = slot_positions(0, 0);
    ScreenHandlerRegistry::register(ScreenHandlerType::new(id.clone(), layout).on_draw(draw_window)
        .on_dynamic_slots(|state| {
            let occupied = state.items.iter().take(RESULT_SLOT).enumerate().fold(0, |mask, (i, slot)| mask | if slot.is_some() { 1 << i } else { 0 });
            Some(slot_positions(selected(state.data), occupied))
        })
        .on_click(|click| {
            if click.right || click.at[0] < -116.0 || click.at[0] >= -8.0 || click.at[1] < 8.0 { return None; }
            let col = ((click.at[0] + 116.0) / 18.0) as usize;
            let row = ((click.at[1] - 8.0) / 18.0) as usize;
            let id = row * 6 + col;
            (id < layouts().len()).then_some(id as i32)
        })
        .on_button(|button| {
            if button.id >= 0 && (button.id as usize) < layouts().len() {
                button.data.insert("layout".to_owned(), Nbt::Int(button.id));
                button.set_data(0, button.id);
                println!("tconstruct: station layout {}", layouts()[button.id as usize].name);
            }
        })
        .on_slot_rule(|rule| {
        if rule.slot == RESULT_SLOT {
            rule.may_place = false; // the result slot only gives
        }
    }));
    BlockEntityTypeRegistry::register(BlockEntityType::new(id).item_slots((RESULT_SLOT + 1) as u32).menu("tconstruct:tinker_station").ticker(station_tick));
}

/// `/tconstruct station`: says what a station would make of the 36 main inventory stacks, without changing anything.
pub fn station_command(ctx: &pluto_lib::CommandContext) -> i32 {
    let slots: Vec<Option<StackData>> = (0..36).map(|slot| ctx.source.stack(slot)).collect();
    // a stack's item tags are not asked here (the menu that comes next has the host); recipes with a tag input do not match in this preview
    match plan(&slots, &|_, _| false) {
        Ok(plan) => {
            let line = format!("a station would make {} taking {} slots", plan.result.item, plan.consumed.len());
            ctx.source.send_message(&line);
            println!("tconstruct: {line}");
            1
        }
        Err(error) => {
            ctx.source.send_message(&error);
            println!("tconstruct: station refused: {error}");
            0
        }
    }
}
