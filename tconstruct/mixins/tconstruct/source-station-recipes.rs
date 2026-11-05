// Source tool-building recipe data used by the station's shared transaction.
use horizon_items::items::ItemStack;
use serde_json::Value;

/// Match the vanilla item/tag ingredient forms used by modifier repair.
/// Unrecognized forms remain unknown instead of becoming a false match.
pub(super) fn repair_ingredient_matches(value: &Value, item: &str,
    tags: &std::collections::HashMap<String, std::collections::HashSet<String>>) -> Option<bool> {
    if let Some(entries) = value.as_array() {
        if entries.is_empty() { return None; }
        let mut matches = false;
        for entry in entries { matches |= repair_ingredient_matches(entry, item, tags)?; }
        return Some(matches);
    }
    let object = value.as_object()?;
    if let Some(kind) = object.get("type") {
        return match kind.as_str()? {
            "forge:difference" if object.len() == 3 => {
                let base = repair_ingredient_matches(object.get("base")?, item, tags)?;
                let removed = repair_ingredient_matches(object.get("subtracted")?, item, tags)?;
                Some(base && !removed)
            }
            "forge:intersection" if object.len() == 2 => {
                let children = object.get("children")?.as_array()?;
                if children.is_empty() { return None; }
                let mut matches = true;
                for child in children { matches &= repair_ingredient_matches(child, item, tags)?; }
                Some(matches)
            }
            _ => None,
        };
    }
    if object.len() != 1 { return None; }
    if let Some(id) = object.get("item") { return Some(id.as_str()? == item); }
    if let Some(tag) = object.get("tag") { return Some(tags.get(tag.as_str()?)?.contains(item)); }
    None
}

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
        Self::definition_parts(definition)
    }

    /// Part swapping is limited by the complete tool definition, not by the
    /// one replacement input or a tool-building recipe's parts override.
    pub(super) fn swap_parts_fit(definition: &[u8], input_capacity: usize) -> Option<bool> {
        if !matches!(input_capacity, 3 | 5) { return Some(false); }
        Some(Self::definition_parts(definition)?.len() <= input_capacity)
    }

    fn definition_parts(definition: &[u8]) -> Option<Vec<String>> {
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

/// Build in snapshot order after pack overrides have been applied. Keep bytes so
/// consumers continue to use the same validation and matching paths.
pub(super) fn building_recipe_index(recipes: &std::collections::BTreeMap<String, Vec<u8>>) -> Vec<Vec<u8>> {
    recipes.values().filter(|bytes| BuildingRecipe::parse(bytes).is_some()).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swapping_uses_total_definition_parts_not_replacement_count() {
        let definition = |count: usize| serde_json::to_vec(&serde_json::json!({"modules":[{
            "type":"tconstruct:part_stats", "parts":vec![serde_json::json!({"item":"tconstruct:tool_handle"});count]
        }]})).unwrap();
        for count in 1..=6 {
            assert_eq!(BuildingRecipe::swap_parts_fit(&definition(count), 3), Some(count <= 3));
            assert_eq!(BuildingRecipe::swap_parts_fit(&definition(count), 5), Some(count <= 5));
        }
        for capacity in [0,1,2,4,6,usize::MAX] {
            assert_eq!(BuildingRecipe::swap_parts_fit(&definition(1),capacity), Some(false));
        }
        // A one-input building override must not bypass a four-part tool's gate.
        let recipe = BuildingRecipe::parse(br#"{"type":"tconstruct:tool_building","result":"tconstruct:sledge_hammer","parts_override":["tconstruct:tool_handle"]}"#).unwrap();
        assert_eq!(recipe.parts(&definition(4)).unwrap().len(),1);
        assert_eq!(BuildingRecipe::swap_parts_fit(&definition(4),3),Some(false));
        assert_eq!(BuildingRecipe::swap_parts_fit(&definition(4),5),Some(true));
    }
    #[test]
    fn swapping_capacity_uses_valid_current_tool_parts_hook() {
        let explicit = br#"{"modules":[{"type":"tconstruct:tool_parts","hooks":["tconstruct:tool_parts"],"parts":["a","b","c","d"]}]}"#;
        assert_eq!(BuildingRecipe::swap_parts_fit(explicit,3),Some(false));
        assert_eq!(BuildingRecipe::swap_parts_fit(explicit,5),Some(true));
        assert_eq!(BuildingRecipe::swap_parts_fit(b"{",3),None);
        assert_eq!(BuildingRecipe::swap_parts_fit(br#"{"modules":[{"type":"other:parts","hooks":["tconstruct:tool_parts"],"parts":["a"]}]}"#,3),None);
        assert_eq!(BuildingRecipe::swap_parts_fit(br#"{"modules":[{"type":"tconstruct:tool_parts","parts":["a"]},{"type":"tconstruct:tool_parts","parts":["b"]}]}"#,5),None);
    }
    #[test]
    fn building_index_preserves_order_and_tracks_snapshot_replacement() {
        use std::collections::BTreeMap;
        let a = br#"{"type":"tconstruct:tool_building","result":"tconstruct:a"}"#.to_vec();
        let b = br#"{"type":"tconstruct:tool_building","result":"tconstruct:b"}"#.to_vec();
        let mut snapshot = BTreeMap::from([
            ("z".to_owned(), b.clone()), ("a".to_owned(), a.clone()),
            ("other".to_owned(), br#"{"type":"minecraft:crafting_shaped"}"#.to_vec()),
            ("invalid".to_owned(), b"{".to_vec()),
        ]);
        assert_eq!(building_recipe_index(&snapshot), vec![a.clone(), b.clone()]);
        snapshot.insert("a".into(), br#"{"type":"minecraft:crafting_shaped"}"#.to_vec());
        assert_eq!(building_recipe_index(&snapshot), vec![b]);
        assert_eq!(building_recipe_index(&BTreeMap::from([("a".into(), a.clone())])), vec![a]);
    }
    #[test]
    fn indexed_and_full_scans_choose_the_same_build_and_tool_recipe() {
        use std::collections::BTreeMap;
        let recipe = |item: &str, part: &str| format!(
            r#"{{"type":"tconstruct:tool_building","result":"{item}","parts_override":["{part}"]}}"#).into_bytes();
        let snapshot = BTreeMap::from([
            ("a".into(), br#"{"type":"minecraft:crafting_shaped"}"#.to_vec()),
            ("b".into(), recipe("tconstruct:pickaxe", "tconstruct:pick_head")),
            ("c".into(), recipe("tconstruct:pickaxe", "tconstruct:other_head")),
        ]);
        let indexed = building_recipe_index(&snapshot);
        let full: Vec<_> = snapshot.values().collect();
        let fast: Vec<_> = indexed.iter().collect();
        let select_build = |bytes: &[&Vec<u8>], slots: &[Option<ItemStack>]| {
            bytes.iter().filter_map(|bytes| BuildingRecipe::parse(bytes)).find_map(|recipe| {
                let parts = recipe.parts(b"{}").unwrap();
                recipe.match_inputs(&parts, parts.len(), slots, |_| Some("tconstruct:wood".into()), |_, _| true, |_, _| false)
            })
        };
        let select_tool = |bytes: &[&Vec<u8>]| bytes.iter().filter_map(|bytes| BuildingRecipe::parse(bytes))
            .find(|recipe| recipe.result == "tconstruct:pickaxe");
        // Build selection follows inputs. Swap and repair both select the first
        // recipe by result, including when multiple recipes share that result.
        for part in ["tconstruct:pick_head", "tconstruct:other_head", "tconstruct:missing"] {
            let slots = [Some(ItemStack::new(part, 1)), None, None];
            assert_eq!(select_build(&full, &slots), select_build(&fast, &slots));
        }
        assert_eq!(select_tool(&full), select_tool(&fast));
        assert_eq!(select_tool(&fast).unwrap().parts_override.unwrap(), vec!["tconstruct:pick_head"]);
    }

    #[test]
    fn modifier_repair_ingredients_resolve_set_forms_without_negating_unknowns() {
        let tags = std::collections::HashMap::from([("forge:foods".to_owned(),
            std::collections::HashSet::from(["tconstruct:bacon".to_owned(), "minecraft:apple".to_owned()]))]);
        let difference = serde_json::json!({"type":"forge:difference", "base":{"tag":"forge:foods"}, "subtracted":{"item":"minecraft:apple"}});
        assert_eq!(repair_ingredient_matches(&difference, "tconstruct:bacon", &tags), Some(true));
        assert_eq!(repair_ingredient_matches(&difference, "minecraft:apple", &tags), Some(false));
        let intersection = serde_json::json!({"type":"forge:intersection", "children":[{"tag":"forge:foods"}, {"item":"tconstruct:bacon"}]});
        assert_eq!(repair_ingredient_matches(&intersection, "tconstruct:bacon", &tags), Some(true));
        assert_eq!(repair_ingredient_matches(&intersection, "minecraft:apple", &tags), Some(false));
        let unknown = serde_json::json!({"type":"forge:difference", "base":{"tag":"forge:foods"}, "subtracted":{"type":"example:unknown"}});
        assert_eq!(repair_ingredient_matches(&unknown, "tconstruct:bacon", &tags), None);
    }
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
    fn four_part_recipe_respects_capacity_and_consumes_each_input_once() {
        let recipe = BuildingRecipe::parse(br#"{"type":"tconstruct:tool_building","result":"tconstruct:sledge_hammer"}"#).unwrap();
        let parts: Vec<String> = ["tconstruct:hammer_head", "tconstruct:tough_handle", "tconstruct:large_plate", "tconstruct:large_plate"]
            .into_iter().map(str::to_owned).collect();
        let mut inputs: Vec<_> = parts.iter().map(|part| Some(ItemStack::new(part, 1))).collect();
        inputs.push(None);
        let match_recipe = |slots: &[Option<ItemStack>]| recipe.match_inputs(&parts, 4, slots, |_| Some("tconstruct:iron".into()), |_, _| true, |_, _| false);
        assert!(match_recipe(&inputs[..3]).is_none());
        let matched = match_recipe(&inputs).unwrap();
        assert_eq!(matched.consumed_slots, [0, 1, 2, 3]);
        let mut carried = None;
        assert!(take_built_tool(&mut inputs, &mut carried, &matched, ItemStack::new("tconstruct:sledge_hammer", 1), 1));
        assert!(inputs.iter().all(Option::is_none));
        assert_eq!(carried.unwrap().item.as_ref(), "tconstruct:sledge_hammer");
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
