//! Building a tool at the Tinker Station (`ToolBuildingRecipe`): the parts of the tool, each in its own slot in the tool's order, with any
//! extra items after them, make the tool; the materials of the parts become the tool's materials.
use serde_json::Value;

use crate::condition::{self, ConditionContext};
use crate::materials::MaterialVariantId;
use crate::resource::ResourceId;
use crate::smeltery::melting::{accepts_item, ingredient_matches, ItemMatch};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolBuildingRecipe {
    pub id: ResourceId,
    /// The tool item made.
    pub result: ResourceId,
    pub result_count: i32,
    /// The station layout to show; the tool's own when None.
    pub slot_layout: Option<ResourceId>,
    /// Items needed besides the parts, in the slots after them.
    pub extra_requirements: Vec<Vec<ItemMatch>>,
    /// Parts to use instead of the tool definition's.
    pub parts_override: Option<Vec<ResourceId>>,
    /// Materials for the tool's parts that are not made from slots.
    pub extra_materials: Vec<MaterialVariantId>,
}

impl ToolBuildingRecipe {
    /// Reads a `tconstruct:tool_building` file. `Ok(None)` when its data conditions are not met.
    pub fn from_json(id: ResourceId, json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:tool_building") {
            return Err("not a tool building recipe".to_owned());
        }
        for c in json.get("conditions").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
            if !condition::test(c, conditions)? {
                return Ok(None);
            }
        }
        let ids = |key: &str| -> Result<Option<Vec<ResourceId>>, String> {
            json.get(key)
                .map(|list| list.as_array().ok_or_else(|| format!("'{key}' must be a list"))?.iter().map(|v| v.as_str().and_then(ResourceId::try_parse).ok_or_else(|| format!("bad id in '{key}'"))).collect())
                .transpose()
        };
        Ok(Some(Self {
            id,
            result: ResourceId::try_parse(json.get("result").and_then(Value::as_str).ok_or("a tool building recipe needs a 'result'")?).ok_or("bad result id")?,
            result_count: json.get("result_count").and_then(Value::as_i64).filter(|c| *c >= 1).unwrap_or(1) as i32,
            slot_layout: json.get("slot_layout").and_then(Value::as_str).map(|l| ResourceId::try_parse(l).ok_or("bad slot layout id")).transpose()?,
            extra_requirements: json
                .get("extra_requirements")
                .and_then(Value::as_array)
                .map(|list| list.iter().map(ingredient_matches).collect::<Result<Vec<_>, _>>())
                .transpose()?
                .unwrap_or_default(),
            parts_override: ids("parts_override")?,
            extra_materials: json
                .get("extra_materials")
                .and_then(Value::as_array)
                .map(|list| list.iter().map(|m| m.as_str().and_then(MaterialVariantId::try_parse).ok_or("bad extra material id")).collect::<Result<Vec<_>, _>>())
                .transpose()?
                .unwrap_or_default(),
        }))
    }

    /// The parts of the tool: the recipe's own, else those of the tool definition (given by the caller).
    pub fn tool_parts<'a>(&'a self, definition_parts: &'a [ResourceId]) -> &'a [ResourceId] {
        self.parts_override.as_deref().unwrap_or(definition_parts)
    }

    /// `matches`: the tool slot is empty; the parts are in the first slots in order, then the extra requirements in the slots after (an
    /// empty slot where there are none left); a recipe of one extra item and no part takes exactly that one item anywhere.
    pub fn matches(&self, tool_slot_empty: bool, inputs: &[Option<ResourceId>], definition_parts: &[ResourceId], tag_items: &dyn Fn(&ResourceId) -> Vec<ResourceId>) -> bool {
        if !tool_slot_empty {
            return false;
        }
        let parts = self.tool_parts(definition_parts);
        let required = parts.len() + self.extra_requirements.len();
        if required == 0 || required > inputs.len() {
            return false;
        }
        if required == 1 && parts.is_empty() {
            let mut found = false;
            for item in inputs.iter().flatten() {
                if found || !accepts_item(&self.extra_requirements[0], item, tag_items) {
                    return false;
                }
                found = true;
            }
            return found;
        }
        for (i, part) in parts.iter().enumerate() {
            if inputs[i].as_ref() != Some(part) {
                return false;
            }
        }
        for (i, input) in inputs.iter().enumerate().skip(parts.len()) {
            match (self.extra_requirements.get(i - parts.len()), input) {
                (Some(wanted), Some(item)) => {
                    if !accepts_item(wanted, item, tag_items) {
                        return false;
                    }
                }
                (Some(_), None) => return false,
                (None, Some(_)) => return false,
                (None, None) => {}
            }
        }
        true
    }

    /// `getValidatedResult`, the materials: those of the part items in the first slots (up to the number the tool has), then the extra
    /// materials for the parts that are not made from slots.
    pub fn result_materials(&self, part_materials: &[MaterialVariantId], definition_parts: usize, material_count: usize) -> Vec<MaterialVariantId> {
        let parts = self.parts_override.as_ref().map_or(definition_parts, Vec::len);
        let mut materials = Vec::new();
        if material_count > 0 {
            materials.extend(part_materials.iter().take(parts.min(material_count)).cloned());
            let extra = material_count.saturating_sub(parts).min(self.extra_materials.len());
            materials.extend(self.extra_materials.iter().take(extra).cloned());
        }
        materials
    }

    /// The number of tools made: the recipe's count, at most a stack (`max_stack`).
    pub fn count(&self, max_stack: i32) -> i32 {
        self.result_count.min(max_stack)
    }

    /// `requiresAnvil`: four or more inputs need the anvil station.
    pub fn requires_anvil(&self, definition_parts: &[ResourceId]) -> bool {
        self.tool_parts(definition_parts).len() + self.extra_requirements.len() >= 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::EmptyContext;
    use serde_json::json;

    fn id(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    fn no_tags(_: &ResourceId) -> Vec<ResourceId> {
        Vec::new()
    }

    fn pickaxe_parts() -> Vec<ResourceId> {
        vec![id("tconstruct:pick_head"), id("tconstruct:tool_handle"), id("tconstruct:tool_binding")]
    }

    fn recipe(json: serde_json::Value) -> ToolBuildingRecipe {
        ToolBuildingRecipe::from_json(id("a:r"), &json, &EmptyContext).unwrap().unwrap()
    }

    #[test]
    fn the_parts_must_be_in_the_first_slots_in_the_tools_order() {
        let r = recipe(json!({"type": "tconstruct:tool_building", "result": "tconstruct:pickaxe"}));
        let parts = pickaxe_parts();
        let right = [Some(id("tconstruct:pick_head")), Some(id("tconstruct:tool_handle")), Some(id("tconstruct:tool_binding")), None, None];
        assert!(r.matches(true, &right, &parts, &no_tags));
        let swapped = [Some(id("tconstruct:tool_handle")), Some(id("tconstruct:pick_head")), Some(id("tconstruct:tool_binding")), None, None];
        assert!(!r.matches(true, &swapped, &parts, &no_tags), "the order counts");
        assert!(!r.matches(false, &right, &parts, &no_tags), "a tool in the tool slot is a different recipe");
        let extra = [Some(id("tconstruct:pick_head")), Some(id("tconstruct:tool_handle")), Some(id("tconstruct:tool_binding")), Some(id("minecraft:stick")), None];
        assert!(!r.matches(true, &extra, &parts, &no_tags), "an item beyond the recipe stops it");
        assert!(!r.matches(true, &right[..2], &parts, &no_tags), "too few slots for the parts");
    }

    #[test]
    fn extra_requirements_follow_the_parts_and_a_lone_item_recipe_takes_that_item_in_any_slot() {
        let with_extra = recipe(json!({"type": "tconstruct:tool_building", "result": "a:tool", "extra_requirements": [{"item": "minecraft:stick"}], "parts_override": ["a:head"]}));
        let inputs = [Some(id("a:head")), Some(id("minecraft:stick")), None];
        assert!(with_extra.matches(true, &inputs, &[], &no_tags));
        assert!(!with_extra.matches(true, &[Some(id("a:head")), None, None], &[], &no_tags), "the stick is missing");
        let lone = recipe(json!({"type": "tconstruct:tool_building", "result": "a:plate", "extra_requirements": [{"item": "a:scale"}], "parts_override": []}));
        assert!(lone.matches(true, &[None, None, Some(id("a:scale")), None, None], &[], &no_tags));
        assert!(!lone.matches(true, &[Some(id("a:scale")), Some(id("a:scale")), None, None, None], &[], &no_tags), "only one");
        assert!(!lone.matches(true, &[None, None, None], &[], &no_tags), "nothing in the slots");
    }

    #[test]
    fn the_tools_materials_come_from_the_parts_then_the_extra_materials_and_the_count_is_a_stack_at_most() {
        let r = recipe(json!({"type": "tconstruct:tool_building", "result": "a:tool", "result_count": 8, "extra_materials": ["tconstruct:iron", "tconstruct:gold"], "parts_override": ["a:head"]}));
        let from_parts = vec![MaterialVariantId::try_parse("tconstruct:wood").unwrap()];
        let materials = r.result_materials(&from_parts, 3, 3);
        assert_eq!(materials.iter().map(|m| m.to_string()).collect::<Vec<_>>(), vec!["tconstruct:wood", "tconstruct:iron", "tconstruct:gold"]);
        assert_eq!(r.result_materials(&from_parts, 3, 0), vec![]);
        assert_eq!((r.count(64), r.count(4)), (8, 4));
        assert!(!r.requires_anvil(&[]) && recipe(json!({"type": "tconstruct:tool_building", "result": "a:t"})).requires_anvil(&[id("a:1"), id("a:2"), id("a:3"), id("a:4")]));
    }
}
