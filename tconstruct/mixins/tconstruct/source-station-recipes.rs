// Source tool-building recipe data used by the station's shared transaction.
use horizon_items::items::ItemStack;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BuildingRecipe {
    pub result: String,
    pub result_count: u32,
    pub layout: Option<String>,
    pub parts_override: Option<Vec<String>>,
    pub extra_requirements: Vec<Value>,
    pub extra_materials: Vec<String>,
}

impl BuildingRecipe {
    pub(super) fn parse(bytes: &[u8]) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        if value.get("type")?.as_str()? != "tconstruct:tool_building" { return None; }
        let strings = |v: &Value| v.as_array()?.iter().map(|item| Some(item.as_str()?.to_owned())).collect::<Option<Vec<_>>>();
        let count = match value.get("result_count") {
            Some(v) => u32::try_from(v.as_u64()?).ok().filter(|n| *n > 0)?,
            None => 1,
        };
        Some(Self {
            result: value.get("result")?.as_str()?.to_owned(),
            result_count: count,
            layout: value.get("slot_layout").and_then(Value::as_str).map(str::to_owned),
            parts_override: match value.get("parts_override") { Some(Value::Null) | None => None, Some(v) => Some(strings(v)?) },
            extra_requirements: match value.get("extra_requirements") { None => Vec::new(), Some(v) => v.as_array()?.clone() },
            extra_materials: match value.get("extra_materials") { None => Vec::new(), Some(v) => strings(v)? },
        })
    }


    /// Resolve the tool-parts hook or the recipe's explicit override.
    /// Other modules still require validation before producing a tool.
    pub(super) fn parts(&self, definition: &[u8]) -> Option<Vec<String>> {
        if let Some(parts) = &self.parts_override { return Some(parts.clone()); }
        let value: Value = serde_json::from_slice(definition).ok()?;
        let mut result = None;
        for module in value.get("modules")?.as_array()? {
            let kind = module.get("type")?.as_str()?;
            let hooks = match module.get("hooks") {
                None => None,
                Some(value) => Some(value.as_array()?.iter().map(Value::as_str).collect::<Option<Vec<_>>>()?),
            };
            let supplies = hooks.as_ref().map_or(matches!(kind, "tconstruct:part_stats" | "tconstruct:tool_parts"), |hooks| hooks.contains(&"tconstruct:tool_parts"));
            if !supplies { continue; }
            if !matches!(kind, "tconstruct:part_stats" | "tconstruct:tool_parts") || result.is_some() { return None; }
            let parts = module.get("parts")?.as_array()?.iter().map(|value| {
                value.as_str().or_else(|| value.get("item").and_then(Value::as_str)).map(str::to_owned)
            }).collect::<Option<Vec<_>>>()?;
            result = Some(parts);
        }
        Some(result.unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_parts_follow_recipe_and_reject_extra_inputs() {
        let recipe = BuildingRecipe::parse(br#"{"type":"tconstruct:tool_building","result":"tconstruct:pickaxe"}"#).unwrap();
        let definition = br#"{"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head","tconstruct:tool_handle"]},{"type":"tconstruct:base_stats","stats":{}}]}"#;
        let parts = recipe.parts(definition).unwrap();
        assert_eq!(parts, ["tconstruct:pick_head", "tconstruct:tool_handle"]);
        let mut slots = vec![Some(ItemStack::new(&parts[0], 1)), Some(ItemStack::new(&parts[1], 1)), None];
        let matched = |inputs: &[Option<ItemStack>]| recipe.match_inputs(&parts, 2, inputs, |_| Some("tconstruct:wood".into()), |_, _| true, |_, _| false);
        assert!(matched(&slots).is_some());
        slots.swap(0, 1);
        assert!(matched(&slots).is_none());
        slots.swap(0, 1);
        slots[2] = Some(ItemStack::new("minecraft:stone", 1));
        assert!(matched(&slots).is_none());
        slots[2] = None;
        let result = recipe.match_inputs(&parts, 2, &slots, |_| Some("tconstruct:wood".into()), |_, _| true, |_, _| false).unwrap();
        assert_eq!(result.consumed_slots, [0, 1]);
        assert_eq!(result.materials, ["tconstruct:wood", "tconstruct:wood"]);
        let before = slots.clone();
        let mut carried = Some(ItemStack::new("minecraft:stone", 1));
        assert!(!take_built_tool(&mut slots, &mut carried, &result, ItemStack::new("tconstruct:pickaxe", 1), 1));
        assert_eq!(slots, before);
        carried = None;
        assert!(take_built_tool(&mut slots, &mut carried, &result, ItemStack::new("tconstruct:pickaxe", 1), 1));
        assert!(slots.iter().all(Option::is_none));
        assert_eq!(&*carried.unwrap().item, "tconstruct:pickaxe");
        let arrow = BuildingRecipe::parse(br#"{"type":"tconstruct:tool_building","result":"tconstruct:arrow","result_count":4,"parts_override":[],"extra_requirements":[{"item":"minecraft:arrow"}],"extra_materials":["tconstruct:flint"]}"#).unwrap();
        let inputs = vec![None, Some(ItemStack::new("minecraft:arrow", 1)), None];
        let result = arrow.match_inputs(&[], 1, &inputs, |_| None, |_, _| false, |_, item| &*item.item == "minecraft:arrow").unwrap();
        assert_eq!((result.result_count, result.consumed_slots, result.materials), (4, vec![1], vec!["tconstruct:flint".to_owned()]));
    }

    #[test]
    fn multi_item_results_obey_capacity_before_consuming_parts() {
        let recipe = BuildingRecipe::parse(br#"{"type":"tconstruct:tool_building","result":"tconstruct:dagger","result_count":2}"#).unwrap();
        let parts = vec!["tconstruct:small_blade".to_owned(), "tconstruct:tool_handle".to_owned()];
        let mut slots = vec![Some(ItemStack::new(&parts[0], 1)), Some(ItemStack::new(&parts[1], 1)), None];
        let matched = recipe.match_inputs(&parts, 2, &slots, |_| Some("tconstruct:wood".into()), |_, _| true, |_, _| false).unwrap();
        let original = slots.clone();
        let mut carried = Some(ItemStack::new("tconstruct:dagger", 1));
        assert!(!take_built_tool(&mut slots, &mut carried, &matched, ItemStack::new("tconstruct:dagger", 2), 2));
        assert_eq!(slots, original);
        carried = None;
        assert!(take_built_tool(&mut slots, &mut carried, &matched, ItemStack::new("tconstruct:dagger", 2), 2));
        assert_eq!(carried.unwrap().count, 2);
        assert!(slots.iter().all(Option::is_none));
    }
}

/// Inputs selected for one authoritative tool-building transaction. A caller
/// validates the resulting tool and derives its stats before consuming them.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BuildingMatch {
    pub result: String,
    pub result_count: u32,
    pub materials: Vec<String>,
    pub consumed_slots: Vec<usize>,
}

/// Commit an already validated recipe and fully built output to the cursor.
/// Preview callers can use the same match without mutating the input slots.
pub(super) fn take_built_tool(
    inputs: &mut [Option<ItemStack>],
    carried: &mut Option<ItemStack>,
    matched: &BuildingMatch,
    output: ItemStack,
    max_stack: u32,
) -> bool {
    if output.item.as_ref() != matched.result || output.count != matched.result_count || output.count > max_stack {
        return false;
    }
    if matched.consumed_slots.iter().any(|&slot| inputs.get(slot).and_then(Option::as_ref).is_none_or(|stack| stack.count == 0)) {
        return false;
    }
    if let Some(existing) = carried.as_ref() {
        let mut compare = existing.clone();
        compare.count = output.count;
        if compare != output || existing.count.saturating_add(output.count) > max_stack { return false; }
    }
    for &slot in &matched.consumed_slots {
        let stack = inputs[slot].as_mut().unwrap();
        stack.count -= 1;
        if stack.count == 0 { inputs[slot] = None; }
    }
    if let Some(existing) = carried.as_mut() { existing.count += output.count; } else { *carried = Some(output); }
    true
}

impl BuildingRecipe {
    pub(super) fn match_inputs(
        &self,
        parts: &[String],
        material_count: usize,
        inputs: &[Option<ItemStack>],
        material: impl Fn(&ItemStack) -> Option<String>,
        accepts_part: impl Fn(&str, &str) -> bool,
        accepts_ingredient: impl Fn(&Value, &ItemStack) -> bool,
    ) -> Option<BuildingMatch> {
        let required = parts.len() + self.extra_requirements.len();
        if required == 0 || required > inputs.len() { return None; }
        let mut consumed_slots = Vec::with_capacity(required);
        let mut materials = Vec::with_capacity(parts.len() + self.extra_materials.len());
        if parts.is_empty() && required == 1 {
            for (index, input) in inputs.iter().enumerate() {
                if let Some(stack) = input {
                    if !consumed_slots.is_empty() || !accepts_ingredient(&self.extra_requirements[0], stack) { return None; }
                    consumed_slots.push(index);
                }
            }
            if consumed_slots.is_empty() { return None; }
        } else {
            for (index, part) in parts.iter().enumerate() {
                let stack = inputs[index].as_ref()?;
                if stack.item.as_ref() != part || stack.count == 0 { return None; }
                let identity = material(stack)?;
                if !accepts_part(part, &identity) { return None; }
                consumed_slots.push(index);
                if index < material_count { materials.push(identity); }
            }
            for (offset, ingredient) in self.extra_requirements.iter().enumerate() {
                let index = parts.len() + offset;
                let stack = inputs[index].as_ref()?;
                if stack.count == 0 || !accepts_ingredient(ingredient, stack) { return None; }
                consumed_slots.push(index);
            }
            if inputs[required..].iter().any(Option::is_some) { return None; }
        }
        materials.extend(self.extra_materials.iter().take(material_count.saturating_sub(parts.len())).cloned());
        Some(BuildingMatch { result: self.result.clone(), result_count: self.result_count, materials, consumed_slots })
    }
}
