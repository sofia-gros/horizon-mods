//! What items and fluids are worth as materials (`MaterialRecipe`, `MaterialFluidRecipe` and `MaterialMeltingRecipe`): the Part Builder takes
//! an item's material and value from the first; casting a part from a fluid uses the second; melting a part gives the fluid of the third.
use std::collections::HashMap;

use serde_json::Value;

use crate::condition::{self, ConditionContext};
use crate::materials::{MaterialId, MaterialVariantId};
use crate::resource::ResourceId;
use crate::smeltery::alloy::{AlloyOutput, FluidMatch};
use crate::smeltery::melting::{accepts_item, ingredient_matches, ItemMatch};

/// `INGOTS_PER_REPAIR`: how many ingots' worth of material a full repair takes.
pub const INGOTS_PER_REPAIR: f32 = 3.0;

fn conditions_hold(json: &Value, conditions: &dyn ConditionContext) -> Result<bool, String> {
    for c in json.get("conditions").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
        if !condition::test(c, conditions)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// An item worth some of a material: `value` units of it, `needed` units to make a part from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialRecipe {
    pub id: ResourceId,
    pub ingredient: Vec<ItemMatch>,
    pub material: MaterialVariantId,
    /// What one item is worth, in units.
    pub value: i32,
    /// How many units a part takes.
    pub needed: i32,
    /// What is given back when the item is worth more than one unit (a block gives its ingot back).
    pub leftover: Option<(ItemMatch, u32)>,
}

impl MaterialRecipe {
    /// Reads a `tconstruct:material` file. `Ok(None)` when its data conditions are not met.
    pub fn from_json(id: ResourceId, json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:material") {
            return Err("not a material recipe".to_owned());
        }
        if !conditions_hold(json, conditions)? {
            return Ok(None);
        }
        let ingredient = ingredient_matches(json.get("ingredient").ok_or("a material recipe needs an 'ingredient'")?)?;
        let material = MaterialVariantId::try_parse(json.get("material").and_then(Value::as_str).ok_or("a material recipe needs a 'material'")?).ok_or("bad material id")?;
        let positive = |key: &str| json.get(key).and_then(Value::as_i64).map_or(Ok(1), |v| if v >= 1 { Ok(v as i32) } else { Err(format!("'{key}' must be at least 1")) });
        let (value, needed) = (positive("value")?, positive("needed")?);
        // the leftover only counts when the item is worth more than one unit
        let leftover = match json.get("leftover") {
            Some(leftover) if value > 1 => {
                // an item id written out, or an object with an item or a tag and a count
                match leftover {
                    Value::String(name) => Some((ItemMatch::Item(ResourceId::try_parse(name).ok_or("bad leftover item id")?), 1)),
                    other => {
                        let item = ingredient_matches(other)?.into_iter().next().ok_or("the leftover names no item")?;
                        Some((item, other.get("count").and_then(Value::as_u64).unwrap_or(1) as u32))
                    }
                }
            }
            _ => None,
        };
        Ok(Some(Self { id, ingredient, material, value, needed, leftover }))
    }
}

/// The material recipes, found by item (`MaterialRecipeCache`).
pub struct MaterialRecipes {
    recipes: Vec<MaterialRecipe>,
}

impl MaterialRecipes {
    /// The recipes with a value; the first recipe loaded for an item wins when several accept it.
    pub fn new(recipes: Vec<MaterialRecipe>) -> Self {
        Self { recipes: recipes.into_iter().filter(|r| r.value > 0).collect() }
    }

    /// `findRecipe`: the recipe whose ingredient accepts the item, if its material is known.
    pub fn find(&self, item: &ResourceId, tag_items: &dyn Fn(&ResourceId) -> Vec<ResourceId>) -> Option<&MaterialRecipe> {
        self.recipes.iter().find(|r| accepts_item(&r.ingredient, item, tag_items))
    }

    /// `getRecipes(material)`: the recipes for a material, ordered by value then by units needed.
    pub fn for_material(&self, material: &MaterialVariantId) -> Vec<&MaterialRecipe> {
        let mut found: Vec<&MaterialRecipe> = self.recipes.iter().filter(|r| &r.material == material).collect();
        found.sort_by_key(|r| (r.value, r.needed));
        found
    }

    /// The materials that have a recipe, by id (the variants of a material are counted with it).
    pub fn known_materials(&self) -> HashMap<MaterialId, usize> {
        let mut counts = HashMap::new();
        for r in &self.recipes {
            *counts.entry(r.material.id().clone()).or_insert(0) += 1;
        }
        counts
    }
}

/// Casting a part from a fluid (`MaterialFluidRecipe`): `fluid` makes `output` material when the casting is at least `temperature` hot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialFluidRecipe {
    pub fluid: FluidMatch,
    pub fluid_amount: i32,
    pub output: MaterialVariantId,
    pub temperature: i32,
}

impl MaterialFluidRecipe {
    pub fn from_json(json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:material_fluid") {
            return Err("not a material fluid recipe".to_owned());
        }
        if !conditions_hold(json, conditions)? {
            return Ok(None);
        }
        let fluid = json.get("fluid").ok_or("a material fluid recipe needs a 'fluid'")?;
        let fluid_match = if let Some(name) = fluid.get("fluid").and_then(Value::as_str) {
            FluidMatch::Fluid(ResourceId::try_parse(name).ok_or("bad fluid id")?)
        } else {
            FluidMatch::Tag(ResourceId::try_parse(fluid.get("tag").and_then(Value::as_str).ok_or("the fluid needs 'fluid' or 'tag'")?).ok_or("bad fluid tag id")?)
        };
        Ok(Some(Self {
            fluid: fluid_match,
            fluid_amount: fluid.get("amount").and_then(Value::as_i64).filter(|a| *a >= 1).ok_or("the fluid needs a positive 'amount'")? as i32,
            output: MaterialVariantId::try_parse(json.get("output").and_then(Value::as_str).ok_or("a material fluid recipe needs an 'output'")?).ok_or("bad material id")?,
            temperature: json.get("temperature").and_then(Value::as_i64).filter(|t| *t >= 0).ok_or("a material fluid recipe needs a 'temperature'")? as i32,
        }))
    }
}

/// Melting a part (`MaterialMeltingRecipe`): a part of `input` material melts into `result` at `temperature`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialMeltingRecipe {
    pub input: MaterialVariantId,
    pub result: AlloyOutput,
    pub temperature: i32,
}

impl MaterialMeltingRecipe {
    pub fn from_json(json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:material_melting") {
            return Err("not a material melting recipe".to_owned());
        }
        if !conditions_hold(json, conditions)? {
            return Ok(None);
        }
        let result = json.get("result").ok_or("a material melting recipe needs a 'result'")?;
        let fluid = if let Some(name) = result.get("fluid").and_then(Value::as_str) {
            FluidMatch::Fluid(ResourceId::try_parse(name).ok_or("bad fluid id")?)
        } else {
            FluidMatch::Tag(ResourceId::try_parse(result.get("tag").and_then(Value::as_str).ok_or("the result needs 'fluid' or 'tag'")?).ok_or("bad fluid tag id")?)
        };
        Ok(Some(Self {
            input: MaterialVariantId::try_parse(json.get("input").and_then(Value::as_str).ok_or("a material melting recipe needs an 'input'")?).ok_or("bad material id")?,
            result: AlloyOutput { fluid, amount: result.get("amount").and_then(Value::as_i64).filter(|a| *a >= 1).ok_or("the result needs a positive 'amount'")? as i32 },
            temperature: json.get("temperature").and_then(Value::as_i64).filter(|t| *t >= 0).ok_or("a material melting recipe needs a 'temperature'")? as i32,
        }))
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

    fn recipe(json: serde_json::Value) -> MaterialRecipe {
        MaterialRecipe::from_json(id("a:r"), &json, &EmptyContext).unwrap().unwrap()
    }

    fn no_tags(_: &ResourceId) -> Vec<ResourceId> {
        Vec::new()
    }

    #[test]
    fn an_item_is_worth_its_value_and_a_block_gives_its_ingot_back() {
        let ingot = recipe(json!({"type": "tconstruct:material", "ingredient": {"item": "a:copper_ingot"}, "material": "tconstruct:copper", "needed": 1, "value": 1}));
        let block = recipe(json!({"type": "tconstruct:material", "ingredient": {"item": "a:copper_block"}, "leftover": {"item": "a:copper_ingot", "count": 1}, "material": "tconstruct:copper", "needed": 1, "value": 9}));
        assert_eq!((ingot.value, block.value, block.leftover.clone()), (1, 9, Some((ItemMatch::Item(id("a:copper_ingot")), 1))));
        assert_eq!(ingot.leftover, None, "a one-unit item leaves nothing");
        let recipes = MaterialRecipes::new(vec![ingot, block]);
        assert_eq!(recipes.find(&id("a:copper_block"), &no_tags).map(|r| r.value), Some(9));
        assert!(recipes.find(&id("a:stick"), &no_tags).is_none());
    }

    #[test]
    fn the_recipes_of_a_material_are_ordered_by_value() {
        let a = recipe(json!({"type": "tconstruct:material", "ingredient": {"item": "a:block"}, "material": "tconstruct:copper", "needed": 1, "value": 9}));
        let b = recipe(json!({"type": "tconstruct:material", "ingredient": {"item": "a:ingot"}, "material": "tconstruct:copper", "needed": 1, "value": 1}));
        let c = recipe(json!({"type": "tconstruct:material", "ingredient": {"item": "a:oak"}, "material": "tconstruct:wood", "needed": 1, "value": 1}));
        let recipes = MaterialRecipes::new(vec![a, b, c]);
        let copper = MaterialVariantId::try_parse("tconstruct:copper").unwrap();
        assert_eq!(recipes.for_material(&copper).iter().map(|r| r.value).collect::<Vec<_>>(), vec![1, 9]);
        assert_eq!(recipes.known_materials().len(), 2);
    }

    #[test]
    fn a_variant_material_and_the_conditions_are_read() {
        let variant = recipe(json!({"type": "tconstruct:material", "ingredient": {"item": "a:x"}, "material": "tconstruct:copper#oxidized", "needed": 1, "value": 1}));
        assert!(variant.material.has_variant());
        let gated = json!({"type": "tconstruct:material", "conditions": [{"type": "mantle:tag_filled", "tag": "forge:ingots/aluminum"}], "ingredient": {"item": "a:x"}, "material": "tconstruct:aluminum"});
        assert_eq!(MaterialRecipe::from_json(id("a:g"), &gated, &EmptyContext), Ok(None));
    }

    #[test]
    fn fluid_and_melting_recipes_for_materials_read_their_fields() {
        let fluid = MaterialFluidRecipe::from_json(&json!({"type": "tconstruct:material_fluid", "fluid": {"amount": 90, "tag": "forge:molten_amethyst_bronze"}, "output": "tconstruct:amethyst_bronze", "temperature": 820}), &EmptyContext).unwrap().unwrap();
        assert_eq!((fluid.fluid_amount, fluid.temperature), (90, 820));
        let melting = MaterialMeltingRecipe::from_json(&json!({"type": "tconstruct:material_melting", "input": "tconstruct:aluminum", "result": {"amount": 90, "tag": "forge:molten_aluminum"}, "temperature": 425}), &EmptyContext).unwrap().unwrap();
        assert_eq!((melting.result.amount, melting.temperature), (90, 425));
        assert!(MaterialFluidRecipe::from_json(&json!({"type": "tconstruct:material"}), &EmptyContext).is_err());
    }
}
