//! What the Tinker Station makes of the stacks in its slots, as a function that does not touch the game: the slots go in, the result and
//! what it consumes come out. Two operations so far: building a tool from parts, and upgrading a tool with a modifier recipe's items.
use pluto_lib::{Nbt, StackData};
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
