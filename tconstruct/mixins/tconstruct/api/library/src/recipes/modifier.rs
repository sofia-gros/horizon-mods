//! Recipes that add a modifier at the Tinker Station (`ModifierRecipe` and its kin): the items it takes, the slots it costs, the tools it goes on
//! and the levels it gives. Six kinds: one level at a time (`modifier`), an amount of one item for each level (`incremental_modifier`), levels
//! with their own slots (`multilevel_modifier` and `multilevel_incremental_modifier`), a choice among variants (`swappable_modifier`) and
//! repairing overslime (`overslime_modifier`).
use std::collections::BTreeMap;

use serde_json::Value;

use crate::condition::{self, ConditionContext};
use crate::modifiers::ModifierId;
use crate::resource::ResourceId;
use crate::smeltery::melting::{ingredient_matches, ItemMatch};

/// A range of modifier levels, both ends included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelRange {
    pub min: i32,
    pub max: i32,
}

impl LevelRange {
    pub fn contains(&self, level: i32) -> bool {
        self.min <= level && level <= self.max
    }

    /// A number is that level exactly; an object has `min` (default 1) and `max` (default no limit).
    fn from_json(json: &Value) -> Result<Self, String> {
        match json {
            Value::Number(n) => {
                let level = n.as_i64().ok_or("a level must be a whole number")? as i32;
                Ok(Self { min: level, max: level })
            }
            Value::Object(o) => Ok(Self { min: o.get("min").and_then(Value::as_i64).unwrap_or(1) as i32, max: o.get("max").and_then(Value::as_i64).map_or(i32::MAX, |m| m as i32) }),
            _ => Err("a level must be a number or an object with 'min' and 'max'".to_owned()),
        }
    }
}

/// One input slot: any of these items, and how many (`amount_needed`; 1 when the recipe does not say).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    pub item: Vec<ItemMatch>,
    pub amount: u32,
}

/// What a recipe of this kind adds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// One level, from the levels the recipe covers.
    Single,
    /// An amount of one item makes a level: `needed_per_level` units, each item worth `amount_per_item`.
    Incremental { needed_per_level: i32, amount_per_item: i32 },
    /// Levels (or ranges) each with their own slot cost.
    Multilevel { levels: Vec<(LevelRange, BTreeMap<String, i32>)> },
    MultilevelIncremental { needed_per_level: i32, amount_per_item: i32, levels: Vec<(LevelRange, BTreeMap<String, i32>)> },
    /// The modifier has variants (`crystalshot` with `amethyst`); the recipe picks one.
    Swappable { variant: String },
    /// Restores overslime by `restore_amount` for each item.
    Overslime { restore_amount: i32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModifierRecipe {
    pub id: ResourceId,
    pub kind: Kind,
    pub inputs: Vec<Input>,
    /// The modifier added (for an overslime recipe, the overslime modifier is implied and this is None).
    pub result: Option<ModifierId>,
    /// The levels the recipe can give (the recipe's `level`, or all up to its `max`).
    pub levels: LevelRange,
    /// The slots one level costs (for kinds without per-level slots).
    pub slots: BTreeMap<String, i32>,
    pub tools: Vec<ItemMatch>,
    pub allow_crystal: bool,
    pub check_trait_level: bool,
}

fn slots_of(json: &Value) -> Result<BTreeMap<String, i32>, String> {
    match json.get("slots") {
        None => Ok(BTreeMap::new()),
        Some(Value::Object(o)) => o.iter().map(|(k, v)| Ok((k.clone(), v.as_i64().ok_or("a slot count must be a number")? as i32))).collect(),
        Some(_) => Err("'slots' must be an object".to_owned()),
    }
}

fn level_slots(json: &Value) -> Result<Vec<(LevelRange, BTreeMap<String, i32>)>, String> {
    json.get("levels")
        .and_then(Value::as_array)
        .ok_or("this recipe needs 'levels'")?
        .iter()
        .map(|entry| Ok((LevelRange::from_json(entry.get("level").ok_or("a level entry needs a 'level'")?)?, slots_of(entry)?)))
        .collect()
}

impl ModifierRecipe {
    /// Reads a modifier recipe of any of the six kinds. `Ok(None)` when its data conditions are not met.
    pub fn from_json(id: ResourceId, json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        let type_name = json.get("type").and_then(Value::as_str).unwrap_or("");
        if !matches!(type_name, "tconstruct:modifier" | "tconstruct:incremental_modifier" | "tconstruct:multilevel_modifier" | "tconstruct:multilevel_incremental_modifier" | "tconstruct:swappable_modifier" | "tconstruct:overslime_modifier") {
            return Err("not a modifier recipe".to_owned());
        }
        for c in json.get("conditions").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
            if !condition::test(c, conditions)? {
                return Ok(None);
            }
        }
        let positive = |key: &str| json.get(key).and_then(Value::as_i64).filter(|v| *v >= 1).map(|v| v as i32);
        let kind = match type_name {
            "tconstruct:modifier" => Kind::Single,
            "tconstruct:incremental_modifier" => Kind::Incremental { needed_per_level: positive("needed_per_level").ok_or("needs 'needed_per_level'")?, amount_per_item: positive("amount_per_item").unwrap_or(1) },
            "tconstruct:multilevel_modifier" => Kind::Multilevel { levels: level_slots(json)? },
            "tconstruct:multilevel_incremental_modifier" => Kind::MultilevelIncremental { needed_per_level: positive("needed_per_level").ok_or("needs 'needed_per_level'")?, amount_per_item: positive("amount_per_item").unwrap_or(1), levels: level_slots(json)? },
            "tconstruct:swappable_modifier" => Kind::Swappable { variant: json.get("result").and_then(|r| r.get("value")).and_then(Value::as_str).ok_or("a swappable recipe needs a result 'value'")?.to_owned() },
            _ => Kind::Overslime { restore_amount: positive("restore_amount").ok_or("needs 'restore_amount'")? },
        };
        let inputs = match (json.get("inputs"), json.get("input"), json.get("ingredient")) {
            (Some(list), _, _) => list
                .as_array()
                .ok_or("'inputs' must be a list")?
                .iter()
                .map(|i| Ok(Input { item: ingredient_matches(i.get("ingredient").unwrap_or(i))?, amount: i.get("amount_needed").and_then(Value::as_u64).unwrap_or(1) as u32 }))
                .collect::<Result<Vec<_>, String>>()?,
            (None, Some(one), _) | (None, None, Some(one)) => vec![Input { item: ingredient_matches(one)?, amount: 1 }],
            _ => return Err("a modifier recipe needs its items".to_owned()),
        };
        let result_name = match json.get("result") {
            Some(Value::String(name)) => Some(name.as_str()),
            Some(object) => object.get("name").and_then(Value::as_str),
            None => None,
        };
        let result = match (&kind, result_name) {
            (Kind::Overslime { .. }, _) => None,
            (_, Some(name)) => Some(ModifierId::try_parse(name).ok_or("bad modifier id")?),
            (_, None) => return Err("a modifier recipe needs a 'result'".to_owned()),
        };
        let levels = match json.get("level") {
            Some(l) => LevelRange::from_json(l)?,
            None => LevelRange { min: 1, max: i32::MAX },
        };
        let tools = ingredient_matches(json.get("tools").ok_or("a modifier recipe needs 'tools'")?)?;
        Ok(Some(Self {
            id,
            kind,
            inputs,
            result,
            levels,
            slots: slots_of(json)?,
            tools,
            allow_crystal: json.get("allow_crystal").and_then(Value::as_bool).unwrap_or(true),
            check_trait_level: json.get("check_trait_level").and_then(Value::as_bool).unwrap_or(true),
        }))
    }

    /// The slots the recipe costs for the modifier at `level`: per level for the multilevel kinds, else the recipe's own.
    pub fn slots_for_level(&self, level: i32) -> BTreeMap<String, i32> {
        match &self.kind {
            Kind::Multilevel { levels } | Kind::MultilevelIncremental { levels, .. } => levels.iter().find(|(range, _)| range.contains(level)).map(|(_, slots)| slots.clone()).unwrap_or_default(),
            _ => self.slots.clone(),
        }
    }

    /// The items an incremental recipe needs for one level, as whole items.
    pub fn items_per_level(&self) -> Option<i32> {
        match self.kind {
            Kind::Incremental { needed_per_level, amount_per_item } | Kind::MultilevelIncremental { needed_per_level, amount_per_item, .. } => Some((needed_per_level + amount_per_item.max(1) - 1) / amount_per_item.max(1)),
            _ => None,
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

    fn load(json: serde_json::Value) -> ModifierRecipe {
        ModifierRecipe::from_json(id("a:r"), &json, &EmptyContext).unwrap().unwrap()
    }

    #[test]
    fn a_single_level_recipe_has_its_items_slots_tools_and_levels() {
        let recipe = load(json!({"type": "tconstruct:modifier", "inputs": [{"item": "a:x"}, {"tag": "forge:string"}], "level": 1, "result": "tconstruct:ambidextrous", "slots": {"abilities": 1}, "tools": {"tag": "tconstruct:modifiable/melee"}}));
        assert_eq!((recipe.inputs.len(), recipe.levels, recipe.slots_for_level(1)["abilities"]), (2, LevelRange { min: 1, max: 1 }, 1));
        assert!(recipe.levels.contains(1) && !recipe.levels.contains(2));
        assert_eq!(recipe.result.map(|m| m.to_string()), Some("tconstruct:ambidextrous".to_owned()));
    }

    #[test]
    fn an_incremental_recipe_needs_whole_items_for_each_level() {
        let recipe = load(json!({"type": "tconstruct:incremental_modifier", "input": {"tag": "forge:gems/quartz"}, "needed_per_level": 36, "amount_per_item": 4, "level": {"max": 5}, "result": "tconstruct:sharpness", "slots": {"upgrades": 1}, "tools": {"tag": "tconstruct:modifiable/melee"}}));
        assert_eq!(recipe.items_per_level(), Some(9));
        assert!(recipe.levels.contains(5) && !recipe.levels.contains(6));
        let rounded = load(json!({"type": "tconstruct:incremental_modifier", "input": {"item": "a:x"}, "needed_per_level": 10, "amount_per_item": 4, "result": "a:m", "tools": {"item": "a:t"}}));
        assert_eq!(rounded.items_per_level(), Some(3), "10 units at 4 an item take 3 items");
    }

    #[test]
    fn multilevel_recipes_cost_different_slots_at_different_levels() {
        let recipe = load(json!({"type": "tconstruct:multilevel_modifier", "inputs": [{"amount_needed": 4, "item": "a:slime"}], "levels": [{"level": 1, "slots": {"abilities": 1}}, {"level": {"min": 2, "max": 3}, "slots": {"upgrades": 1}}], "result": "tconstruct:reflecting", "tools": {"item": "a:t"}}));
        assert_eq!(recipe.inputs[0].amount, 4);
        assert_eq!((recipe.slots_for_level(1).get("abilities"), recipe.slots_for_level(3).get("upgrades"), recipe.slots_for_level(4).len()), (Some(&1), Some(&1), 0));
        let free = load(json!({"type": "tconstruct:multilevel_incremental_modifier", "input": {"item": "minecraft:feather"}, "needed_per_level": 12, "levels": [{"level": 1, "slots": {"upgrades": 1}}, {"level": 2}], "result": "tconstruct:feather_fall", "tools": {"item": "a:boots"}}));
        assert!(free.slots_for_level(2).is_empty(), "level 2 costs no slot");
        assert_eq!(free.items_per_level(), Some(12));
    }

    #[test]
    fn swappable_and_overslime_recipes_read_their_own_fields() {
        let swap = load(json!({"type": "tconstruct:swappable_modifier", "inputs": [{"item": "a:x"}], "result": {"name": "tconstruct:crystalshot", "value": "amethyst"}, "slots": {"abilities": 1}, "tools": {"item": "a:bow"}}));
        assert_eq!(swap.kind, Kind::Swappable { variant: "amethyst".to_owned() });
        assert_eq!(swap.result.map(|m| m.to_string()), Some("tconstruct:crystalshot".to_owned()));
        let slime = load(json!({"type": "tconstruct:overslime_modifier", "ingredient": {"item": "minecraft:slime_ball"}, "restore_amount": 20, "tools": {"tag": "tconstruct:modifiable/durability"}}));
        assert_eq!((slime.kind, slime.result), (Kind::Overslime { restore_amount: 20 }, None));
        assert!(ModifierRecipe::from_json(id("a:b"), &json!({"type": "tconstruct:material"}), &EmptyContext).is_err());
        assert_eq!(ModifierRecipe::from_json(id("a:g"), &json!({"type": "tconstruct:modifier", "conditions": [{"type": "forge:false"}]}), &EmptyContext), Ok(None));
    }
}
