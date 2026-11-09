//! Casting (`ItemCastingRecipe`): a casting table or basin turns a fluid, and a cast when the recipe wants one, into an item after a
//! cooling time. This is the recipe: what it accepts, what it gives, how much fluid it takes, how long it cools, and whether the cast is
//! used up.
use serde_json::Value;

use super::alloy::FluidMatch;
use super::melting::{ingredient_matches, ItemMatch};
use crate::condition::{self, ConditionContext};
use crate::resource::ResourceId;

/// The table makes small things (ingots, nuggets, gems), the basin makes blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastingKind {
    Table,
    Basin,
}

/// An item with a count; a tag stands for its first member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemOutput {
    pub item: ItemMatch,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastingRecipe {
    pub id: ResourceId,
    pub kind: CastingKind,
    pub group: String,
    /// What the cast slot must hold; None when the recipe wants an empty slot.
    pub cast: Option<Vec<ItemMatch>>,
    pub cast_consumed: bool,
    pub switch_slots: bool,
    pub fluid: FluidMatch,
    /// Millibuckets of the fluid one casting takes.
    pub fluid_amount: i32,
    pub result: ItemOutput,
    pub cooling_time: i32,
}

/// Filling a container with fluid (`ContainerFillingRecipe`): the container item takes `fluid_amount` of whatever fluid it is given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FillingRecipe {
    pub id: ResourceId,
    pub kind: CastingKind,
    pub container: ResourceId,
    pub fluid_amount: i32,
}

impl FillingRecipe {
    /// Reads a `tconstruct:table_filling` or `tconstruct:basin_filling` file.
    pub fn from_json(id: ResourceId, json: &Value) -> Result<Self, String> {
        let kind = match json.get("type").and_then(Value::as_str) {
            Some("tconstruct:table_filling") => CastingKind::Table,
            Some("tconstruct:basin_filling") => CastingKind::Basin,
            _ => return Err("not a filling recipe".to_owned()),
        };
        Ok(Self {
            id,
            kind,
            container: ResourceId::try_parse(json.get("container").and_then(Value::as_str).ok_or("a filling recipe needs a 'container'")?).ok_or("bad container id")?,
            fluid_amount: json.get("fluid_amount").and_then(Value::as_i64).filter(|a| *a >= 1).ok_or("a filling recipe needs a positive 'fluid_amount'")? as i32,
        })
    }
}

/// Copying a cast or template with fluid (`CastingTableDuplicationRecipe`): the item in the table is given back twice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicationRecipe {
    pub id: ResourceId,
    pub cast: Vec<ItemMatch>,
    pub fluid: FluidMatch,
    pub fluid_amount: i32,
    pub cooling_time: i32,
}

impl DuplicationRecipe {
    pub fn from_json(id: ResourceId, json: &Value) -> Result<Self, String> {
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:table_duplication") {
            return Err("not a duplication recipe".to_owned());
        }
        let (fluid, fluid_amount) = fluid_and_amount(json.get("fluid").ok_or("a duplication recipe needs a 'fluid'")?)?;
        Ok(Self {
            id,
            cast: ingredient_matches(json.get("cast").ok_or("a duplication recipe needs a 'cast'")?)?,
            fluid,
            fluid_amount,
            cooling_time: json.get("cooling_time").and_then(Value::as_i64).filter(|t| *t >= 1).ok_or("a duplication recipe needs a 'cooling_time'")? as i32,
        })
    }
}

/// Filling a bottle with a potion fluid (`PotionCastingRecipe`): the bottle becomes the result and the potion is carried over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PotionRecipe {
    pub id: ResourceId,
    pub bottle: Vec<ItemMatch>,
    pub fluid: FluidMatch,
    pub fluid_amount: i32,
    pub result: ResourceId,
}

impl PotionRecipe {
    pub fn from_json(id: ResourceId, json: &Value) -> Result<Self, String> {
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:casting_table_potion") {
            return Err("not a potion casting recipe".to_owned());
        }
        let (fluid, fluid_amount) = fluid_and_amount(json.get("fluid").ok_or("a potion recipe needs a 'fluid'")?)?;
        Ok(Self {
            id,
            bottle: ingredient_matches(json.get("bottle").ok_or("a potion recipe needs a 'bottle'")?)?,
            fluid,
            fluid_amount,
            result: ResourceId::try_parse(json.get("result").and_then(Value::as_str).ok_or("a potion recipe needs a 'result' item id")?).ok_or("bad result id")?,
        })
    }
}

fn fluid_and_amount(fluid: &Value) -> Result<(FluidMatch, i32), String> {
    let kind = if let Some(name) = fluid.get("fluid").and_then(Value::as_str) {
        FluidMatch::Fluid(ResourceId::try_parse(name).ok_or("bad fluid id")?)
    } else {
        FluidMatch::Tag(ResourceId::try_parse(fluid.get("tag").and_then(Value::as_str).ok_or("the fluid needs 'fluid' or 'tag'")?).ok_or("bad fluid tag id")?)
    };
    Ok((kind, fluid.get("amount").and_then(Value::as_i64).filter(|a| *a >= 1).ok_or("the fluid needs a positive 'amount'")? as i32))
}

/// Which items and fluids belong to a tag.
pub trait CastingTags {
    fn items(&self, tag: &ResourceId) -> Vec<ResourceId>;
    fn fluids(&self, tag: &ResourceId) -> Vec<ResourceId>;
}

impl CastingRecipe {
    /// Reads a `tconstruct:casting_table` or `tconstruct:casting_basin` file. `Ok(None)` when its data conditions are not met.
    pub fn from_json(id: ResourceId, json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        let kind = match json.get("type").and_then(Value::as_str) {
            Some("tconstruct:casting_table") => CastingKind::Table,
            // the retextured basin casts the same way; the cast's texture goes onto the result (not carried)
            Some("tconstruct:casting_basin" | "tconstruct:retextured_casting_basin") => CastingKind::Basin,
            _ => return Err("not an item casting recipe".to_owned()),
        };
        for c in json.get("conditions").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
            if !condition::test(c, conditions)? {
                return Ok(None);
            }
        }
        let cast = match json.get("cast") {
            None => None,
            Some(cast) => Some(ingredient_matches(cast)?),
        };
        let fluid = json.get("fluid").ok_or("a casting recipe needs a 'fluid'")?;
        let fluid_match = if let Some(name) = fluid.get("fluid").and_then(Value::as_str) {
            FluidMatch::Fluid(ResourceId::try_parse(name).ok_or("bad fluid id")?)
        } else {
            FluidMatch::Tag(ResourceId::try_parse(fluid.get("tag").and_then(Value::as_str).ok_or("the fluid needs 'fluid' or 'tag'")?).ok_or("bad fluid tag id")?)
        };
        let result = json.get("result").ok_or("a casting recipe needs a 'result'")?;
        // the result is an item id written out, or an object with an item or a tag (and a count)
        let item = match result {
            Value::String(name) => ItemMatch::Item(ResourceId::try_parse(name).ok_or("bad result item id")?),
            other => ingredient_matches(other)?.into_iter().next().ok_or("the result names no item")?,
        };
        Ok(Some(Self {
            id,
            kind,
            group: json.get("group").and_then(Value::as_str).unwrap_or("").to_owned(),
            cast,
            cast_consumed: json.get("cast_consumed").and_then(Value::as_bool).unwrap_or(false),
            switch_slots: json.get("switch_slots").and_then(Value::as_bool).unwrap_or(false),
            fluid: fluid_match,
            fluid_amount: fluid.get("amount").and_then(Value::as_i64).filter(|a| *a >= 1).ok_or("the fluid needs a positive 'amount'")? as i32,
            result: ItemOutput { item, count: result.get("count").and_then(Value::as_u64).unwrap_or(1) as u32 },
            cooling_time: json.get("cooling_time").and_then(Value::as_i64).filter(|t| *t >= 1).ok_or("a casting recipe needs a 'cooling_time' of at least 1")? as i32,
        }))
    }

    fn item_in(&self, matches: &[ItemMatch], item: &ResourceId, tags: &dyn CastingTags) -> bool {
        matches.iter().any(|m| match m {
            ItemMatch::Item(id) => id == item,
            ItemMatch::Tag(tag) => tags.items(tag).contains(item),
        })
    }

    /// `matches`: the cast slot holds what the recipe wants (nothing, when it wants nothing) and the fluid is the recipe's.
    pub fn matches(&self, cast: Option<&ResourceId>, fluid: &ResourceId, tags: &dyn CastingTags) -> bool {
        let cast_ok = match (&self.cast, cast) {
            (None, None) => true,
            (Some(wanted), Some(item)) => self.item_in(wanted, item, tags),
            _ => false,
        };
        cast_ok
            && match &self.fluid {
                FluidMatch::Fluid(id) => id == fluid,
                FluidMatch::Tag(tag) => tags.fluids(tag).contains(fluid),
            }
    }

    /// The item the casting gives, with a tag resolved to its first member.
    pub fn result_item(&self, tags: &dyn CastingTags) -> Option<(ResourceId, u32)> {
        let id = match &self.result.item {
            ItemMatch::Item(id) => id.clone(),
            ItemMatch::Tag(tag) => tags.items(tag).into_iter().next()?,
        };
        Some((id, self.result.count))
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

    struct Tags;
    impl CastingTags for Tags {
        fn items(&self, tag: &ResourceId) -> Vec<ResourceId> {
            match tag.to_string().as_str() {
                "tconstruct:casts/multi_use/ingot" => vec![id("tconstruct:ingot_cast")],
                "forge:ingots/iron" => vec![id("minecraft:iron_ingot")],
                _ => Vec::new(),
            }
        }
        fn fluids(&self, tag: &ResourceId) -> Vec<ResourceId> {
            if tag.to_string() == "forge:molten_iron" { vec![id("tconstruct:molten_iron")] } else { Vec::new() }
        }
    }

    fn ingot_recipe() -> CastingRecipe {
        let json = json!({"type": "tconstruct:casting_table", "cast": {"tag": "tconstruct:casts/multi_use/ingot"}, "cooling_time": 60,
            "fluid": {"amount": 90, "tag": "forge:molten_iron"}, "result": {"tag": "forge:ingots/iron"}});
        CastingRecipe::from_json(id("tconstruct:iron_ingot"), &json, &EmptyContext).unwrap().unwrap()
    }

    #[test]
    fn a_cast_and_molten_iron_make_an_iron_ingot_and_the_cast_stays() {
        let recipe = ingot_recipe();
        assert_eq!((recipe.kind, recipe.fluid_amount, recipe.cooling_time, recipe.cast_consumed), (CastingKind::Table, 90, 60, false));
        assert!(recipe.matches(Some(&id("tconstruct:ingot_cast")), &id("tconstruct:molten_iron"), &Tags));
        assert_eq!(recipe.result_item(&Tags), Some((id("minecraft:iron_ingot"), 1)));
    }

    #[test]
    fn the_wrong_cast_no_cast_or_the_wrong_fluid_do_not_match() {
        let recipe = ingot_recipe();
        assert!(!recipe.matches(Some(&id("tconstruct:nugget_cast")), &id("tconstruct:molten_iron"), &Tags));
        assert!(!recipe.matches(None, &id("tconstruct:molten_iron"), &Tags), "a recipe with a cast needs one");
        assert!(!recipe.matches(Some(&id("tconstruct:ingot_cast")), &id("tconstruct:molten_gold"), &Tags));
    }

    #[test]
    fn a_recipe_without_a_cast_wants_an_empty_slot() {
        let json = json!({"type": "tconstruct:casting_basin", "cooling_time": 100, "fluid": {"amount": 810, "fluid": "tconstruct:molten_iron"},
            "result": {"item": "minecraft:iron_block"}, "cast_consumed": true});
        let recipe = CastingRecipe::from_json(id("tconstruct:iron_block"), &json, &EmptyContext).unwrap().unwrap();
        assert_eq!(recipe.kind, CastingKind::Basin);
        assert!(recipe.matches(None, &id("tconstruct:molten_iron"), &Tags));
        assert!(!recipe.matches(Some(&id("tconstruct:ingot_cast")), &id("tconstruct:molten_iron"), &Tags));
    }

    #[test]
    fn filling_duplication_and_potion_recipes_read_their_fields() {
        let filling = FillingRecipe::from_json(id("a:f"), &json!({"type": "tconstruct:basin_filling", "container": "tconstruct:gauge", "fluid_amount": 250})).unwrap();
        assert_eq!((filling.kind, filling.fluid_amount), (CastingKind::Basin, 250));
        let copy = DuplicationRecipe::from_json(id("a:d"), &json!({"type": "tconstruct:table_duplication", "cast": [{"item": "a:template"}, {"tag": "a:templates"}], "cooling_time": 186, "fluid": {"amount": 500, "tag": "a:molten"}})).unwrap();
        assert_eq!((copy.cast.len(), copy.fluid_amount, copy.cooling_time), (2, 500, 186));
        let potion = PotionRecipe::from_json(id("a:p"), &json!({"type": "tconstruct:casting_table_potion", "bottle": {"item": "minecraft:glass_bottle"}, "fluid": {"amount": 250, "tag": "forge:potion"}, "result": "minecraft:potion"})).unwrap();
        assert_eq!((potion.fluid_amount, potion.result.to_string().as_str()), (250, "minecraft:potion"));
        assert!(FillingRecipe::from_json(id("a:x"), &json!({"type": "tconstruct:table_filling", "container": "a:b"})).is_err());
    }

    #[test]
    fn missing_fields_are_errors_and_other_recipe_types_are_refused() {
        let no_time = json!({"type": "tconstruct:casting_table", "fluid": {"amount": 1, "fluid": "a:b"}, "result": {"item": "a:c"}});
        assert!(CastingRecipe::from_json(id("a:x"), &no_time, &EmptyContext).is_err());
        let filling = json!({"type": "tconstruct:table_filling"});
        assert!(CastingRecipe::from_json(id("a:y"), &filling, &EmptyContext).is_err());
        let _ = &filling;
    }
}
