//! Part Builder recipes (`PartRecipe` and `ItemPartRecipe`): a pattern and enough of a material make a part (or, for an item recipe, another
//! item) of a cost in units; a material worth more than the cost gives more than one part.
use serde_json::Value;

use super::material::MaterialRecipe;
use crate::condition::{self, ConditionContext};
use crate::materials::MaterialVariantId;
use crate::resource::ResourceId;
use crate::smeltery::melting::{accepts_item, ingredient_matches, ItemMatch};

/// The tag of the patterns that count when a recipe names none (`DEFAULT_PATTERNS`).
pub const DEFAULT_PATTERN_TAG: &str = "tconstruct:patterns/default";

fn pattern_items(json: &Value) -> Result<Vec<ItemMatch>, String> {
    match json.get("pattern_item") {
        Some(items) => ingredient_matches(items),
        None => Ok(vec![ItemMatch::Tag(ResourceId::try_parse(DEFAULT_PATTERN_TAG).expect("a valid tag id"))]),
    }
}

fn conditions_hold(json: &Value, conditions: &dyn ConditionContext) -> Result<bool, String> {
    for c in json.get("conditions").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
        if !condition::test(c, conditions)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// A recipe for a material part (`tconstruct:part_builder`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartRecipe {
    pub id: ResourceId,
    /// The pattern the player picks to make this part.
    pub pattern: ResourceId,
    pub pattern_item: Vec<ItemMatch>,
    /// The units of material one part costs.
    pub cost: i32,
    /// Whether a material that cannot be crafted may be used.
    pub allow_uncraftable: bool,
    /// The part item, which takes the material.
    pub result: ResourceId,
    pub result_count: i32,
}

impl PartRecipe {
    pub fn from_json(id: ResourceId, json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:part_builder") {
            return Err("not a part builder recipe".to_owned());
        }
        if !conditions_hold(json, conditions)? {
            return Ok(None);
        }
        let result = json.get("result").ok_or("a part recipe needs a 'result'")?;
        Ok(Some(Self {
            id,
            pattern: ResourceId::try_parse(json.get("pattern").and_then(Value::as_str).ok_or("a part recipe needs a 'pattern'")?).ok_or("bad pattern id")?,
            pattern_item: pattern_items(json)?,
            cost: json.get("cost").and_then(Value::as_i64).filter(|c| *c >= 1).ok_or("a part recipe needs a 'cost' of at least 1")? as i32,
            allow_uncraftable: json.get("allow_uncraftable").and_then(Value::as_bool).unwrap_or(false),
            result: ResourceId::try_parse(result.get("item").and_then(Value::as_str).ok_or("the result needs an 'item'")?).ok_or("bad result item id")?,
            result_count: result.get("count").and_then(Value::as_i64).filter(|c| *c >= 1).unwrap_or(1) as i32,
        }))
    }

    /// `matches`: the material may be used for this part (it is craftable unless the recipe allows otherwise, and the part takes it) and the
    /// stack has enough items.
    pub fn matches(&self, material: &MaterialRecipe, stack_count: u32, craftable: bool, part_takes_material: bool) -> bool {
        (self.allow_uncraftable || craftable) && part_takes_material && stack_count as i32 >= material.items_used(self.cost)
    }

    /// `partialMatch`: the pattern is the right one, and, when there is a material item, it is not already this part and the part could
    /// take the material.
    pub fn partial_match(&self, pattern_stack: &ResourceId, stack: Option<(&ResourceId, Option<(&MaterialRecipe, bool, bool)>)>, tag_items: &dyn Fn(&ResourceId) -> Vec<ResourceId>) -> bool {
        if !accepts_item(&self.pattern_item, pattern_stack, tag_items) {
            return false;
        }
        match stack {
            None => true,
            Some((item, _)) if *item == self.result => false,
            Some((_, None)) => false,
            Some((_, Some((_, craftable, takes)))) => (self.allow_uncraftable || craftable) && takes,
        }
    }

    /// `assemble`: how many parts one go gives: the output count, times how many times the material's value covers the cost when the
    /// item has no leftover to give back.
    pub fn parts_made(&self, material: &MaterialRecipe) -> i32 {
        if material.leftover.is_none() && material.value > self.cost {
            self.result_count * material.value / self.cost
        } else {
            self.result_count
        }
    }
}

/// A recipe for an item made of a material (`tconstruct:item_part_builder`), such as copying an elytra with dragon scales.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemPartRecipe {
    pub id: ResourceId,
    /// The material the item is made from; None when the recipe takes nothing from the material slot.
    pub material: Option<MaterialVariantId>,
    pub pattern: ResourceId,
    pub pattern_item: Vec<ItemMatch>,
    pub cost: i32,
    pub result: ResourceId,
    pub result_count: i32,
}

impl ItemPartRecipe {
    pub fn from_json(id: ResourceId, json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:item_part_builder") {
            return Err("not an item part recipe".to_owned());
        }
        if !conditions_hold(json, conditions)? {
            return Ok(None);
        }
        let material = match json.get("material").and_then(Value::as_str) {
            Some(name) => Some(MaterialVariantId::try_parse(name).ok_or("bad material id")?).filter(|m| m.id().to_string() != "tconstruct:unknown"),
            None => None,
        };
        let cost = json.get("cost").and_then(Value::as_i64).unwrap_or(0) as i32;
        if cost == 0 && material.is_some() {
            return Err("Cost must be greater than zero if material is defined".to_owned());
        }
        let result = json.get("result").ok_or("an item part recipe needs a 'result'")?;
        let (item, count) = match result {
            Value::String(name) => (name.as_str(), 1),
            other => (other.get("item").and_then(Value::as_str).ok_or("the result needs an 'item'")?, other.get("count").and_then(Value::as_i64).unwrap_or(1) as i32),
        };
        Ok(Some(Self {
            id,
            material,
            pattern: ResourceId::try_parse(json.get("pattern").and_then(Value::as_str).ok_or("an item part recipe needs a 'pattern'")?).ok_or("bad pattern id")?,
            pattern_item: pattern_items(json)?,
            cost,
            result: ResourceId::try_parse(item).ok_or("bad result item id")?,
            result_count: count,
        }))
    }

    /// `matches`: without a material the material slot must be empty; with one, the slot holds that material in enough items.
    pub fn matches(&self, slot: Option<(&MaterialRecipe, u32)>) -> bool {
        match (&self.material, slot) {
            (None, None) => true,
            (None, Some(_)) => false,
            (Some(_), None) => false,
            (Some(wanted), Some((recipe, count))) => &recipe.material == wanted && count as i32 >= recipe.items_used(self.cost),
        }
    }

    /// `getItemsUsed`: nothing without a material.
    pub fn items_used(&self, slot: Option<&MaterialRecipe>) -> i32 {
        match (&self.material, slot) {
            (Some(_), Some(recipe)) => recipe.items_used(self.cost),
            _ => 0,
        }
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

    fn material(value: i32, leftover: bool) -> MaterialRecipe {
        let mut json = json!({"type": "tconstruct:material", "ingredient": {"item": "a:x"}, "material": "tconstruct:copper", "needed": 1, "value": value});
        if leftover {
            json["leftover"] = json!({"item": "a:ingot"});
        }
        MaterialRecipe::from_json(id("a:m"), &json, &EmptyContext).unwrap().unwrap()
    }

    fn pick_head() -> PartRecipe {
        let json = json!({"type": "tconstruct:part_builder", "cost": 2, "pattern": "tconstruct:pick_head", "pattern_item": [{"tag": "tconstruct:patterns/default"}, {"item": "tconstruct:pick_head_cast"}], "result": {"item": "tconstruct:pick_head"}});
        PartRecipe::from_json(id("a:p"), &json, &EmptyContext).unwrap().unwrap()
    }

    #[test]
    fn a_part_needs_a_craftable_material_the_part_takes_and_enough_items() {
        let recipe = pick_head();
        let ingot = material(1, false);
        assert!(recipe.matches(&ingot, 2, true, true));
        assert!(!recipe.matches(&ingot, 1, true, true), "two ingots are needed");
        assert!(!recipe.matches(&ingot, 5, false, true), "an uncraftable material is refused");
        assert!(!recipe.matches(&ingot, 5, true, false), "the part does not take this material");
        let mut allows = pick_head();
        allows.allow_uncraftable = true;
        assert!(allows.matches(&ingot, 2, false, true));
    }

    #[test]
    fn a_valuable_item_makes_several_parts_unless_it_has_a_leftover() {
        let recipe = pick_head();
        assert_eq!(recipe.parts_made(&material(9, false)), 4, "9 / 2 parts");
        assert_eq!(recipe.parts_made(&material(9, true)), 1, "a block with a leftover gives the rest back instead");
        assert_eq!(recipe.parts_made(&material(1, false)), 1);
    }

    #[test]
    fn the_pattern_decides_a_partial_match_and_the_default_patterns_are_a_tag() {
        let recipe = pick_head();
        let tags = |tag: &ResourceId| if tag.to_string() == DEFAULT_PATTERN_TAG { vec![id("tconstruct:pattern")] } else { Vec::new() };
        assert!(recipe.partial_match(&id("tconstruct:pattern"), None, &tags));
        assert!(recipe.partial_match(&id("tconstruct:pick_head_cast"), None, &tags));
        assert!(!recipe.partial_match(&id("minecraft:stick"), None, &tags));
        let copper = material(1, false);
        assert!(recipe.partial_match(&id("tconstruct:pattern"), Some((&id("a:ingot"), Some((&copper, true, true)))), &tags));
        assert!(!recipe.partial_match(&id("tconstruct:pattern"), Some((&id("tconstruct:pick_head"), None)), &tags), "already this part");
        assert!(!recipe.partial_match(&id("tconstruct:pattern"), Some((&id("a:stick"), None)), &tags), "not a material");
        let none = json!({"type": "tconstruct:part_builder", "cost": 1, "pattern": "a:b", "result": {"item": "a:c", "count": 2}});
        let defaulted = PartRecipe::from_json(id("a:d"), &none, &EmptyContext).unwrap().unwrap();
        assert_eq!((defaulted.result_count, defaulted.pattern_item), (2, vec![ItemMatch::Tag(id(DEFAULT_PATTERN_TAG))]));
    }

    #[test]
    fn an_item_recipe_takes_its_material_or_an_empty_slot() {
        let json = json!({"type": "tconstruct:item_part_builder", "cost": 32, "material": "tconstruct:copper", "pattern": "tconstruct:maille", "pattern_item": {"item": "minecraft:elytra"}, "result": "minecraft:elytra"});
        let recipe = ItemPartRecipe::from_json(id("a:e"), &json, &EmptyContext).unwrap().unwrap();
        let copper = material(1, false);
        assert!(recipe.matches(Some((&copper, 32))) && !recipe.matches(Some((&copper, 31))) && !recipe.matches(None));
        assert_eq!(recipe.items_used(Some(&copper)), 32);
        let bare = json!({"type": "tconstruct:item_part_builder", "pattern": "a:b", "result": "a:c"});
        let bare = ItemPartRecipe::from_json(id("a:f"), &bare, &EmptyContext).unwrap().unwrap();
        assert!(bare.matches(None) && !bare.matches(Some((&copper, 1))) && bare.items_used(None) == 0);
        let broken = json!({"type": "tconstruct:item_part_builder", "material": "tconstruct:copper", "pattern": "a:b", "result": "a:c"});
        assert!(ItemPartRecipe::from_json(id("a:g"), &broken, &EmptyContext).is_err(), "a material needs a cost");
    }
}
