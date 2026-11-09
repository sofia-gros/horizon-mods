//! Melting (`MeltingRecipe` and `MeltingModule`): an item in a heating structure's slot warms up while the structure is hot enough and,
//! when its time is up, turns into a fluid that goes to the tank. The slot keeps its progress while the item is the same, loses it when
//! the item changes, cools off when the structure is too cold, and waits (`NO_SPACE`) when the tank has no room.
use serde_json::Value;

use super::alloy::{AlloyOutput, FluidMatch};
use crate::condition::{self, ConditionContext};
use crate::resource::ResourceId;

/// The slot has finished heating but the tank has no room for the result.
pub const NO_SPACE: i32 = -1;

/// What an item must be: one item, or any item of a tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemMatch {
    Item(ResourceId),
    Tag(ResourceId),
}

/// How the recipe gives its fluid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeltingKind {
    /// The same amount every time.
    Plain,
    /// A tool or armor piece: the amount follows the durability left, in steps of `unit_size` (one for each byproduct as well).
    Damagable { unit_size: i32, byproduct_sizes: Vec<i32> },
    /// An ore: the amount follows the ore rate of the structure for this kind of ore, and so does each byproduct (by its own kind, or the
    /// ore's when it names none).
    Ore { rate: OreRateType, byproduct_rates: Vec<OreRateType> },
}

/// The kind of ore a recipe melts (`OreRateType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OreRateType {
    None,
    Metal,
    Gem,
    /// Only for a byproduct: the ore's own kind.
    Default,
}

impl OreRateType {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "none" => Some(Self::None),
            "metal" => Some(Self::Metal),
            "gem" => Some(Self::Gem),
            "default" => Some(Self::Default),
            _ => None,
        }
    }

    /// `orElse`.
    pub fn or_else(self, other: Self) -> Self {
        if self == Self::Default { other } else { self }
    }
}

/// How much more than one ingot or gem an ore gives, for one kind of structure (`Config.OreRate`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OreRate {
    /// Nuggets per metal ore unit; 9 would give one ingot.
    pub nuggets_per_metal: i32,
    /// Shards per gem ore unit; 4 would give one gem.
    pub shards_per_gem: i32,
}

impl OreRate {
    /// The melter and the smeltery.
    pub const MELTER: Self = Self { nuggets_per_metal: 12, shards_per_gem: 8 };
    pub const SMELTERY: Self = Self::MELTER;
    pub const FOUNDRY: Self = Self { nuggets_per_metal: 9, shards_per_gem: 4 };
    pub const FOUNDRY_BYPRODUCT: Self = Self { nuggets_per_metal: 3, shards_per_gem: 4 };

    /// `applyOreBoost`.
    pub fn apply(&self, rate: OreRateType, amount: i32) -> i32 {
        match rate {
            OreRateType::Metal => amount * self.nuggets_per_metal / 9,
            OreRateType::Gem => amount * self.shards_per_gem / 4,
            _ => amount,
        }
    }
}

/// `DamageableMeltingRecipe.scaleOutput`: `amount` scaled by the durability left, never below one unit, in whole units.
pub fn scale_output(amount: i32, damage: i32, max_damage: i32, unit_size: i32) -> i32 {
    let mut scaled = (amount as i64 * (max_damage - damage) as i64 / max_damage as i64) as i32;
    if scaled <= unit_size {
        scaled = unit_size.max(1);
    } else if unit_size > 1 {
        scaled -= scaled % unit_size;
    }
    scaled
}

/// A melting recipe: an ingredient, the fluid it gives, the temperature it needs and the time it takes (the original's units; the module
/// counts ten times that many ticks of heat).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeltingRecipe {
    pub id: ResourceId,
    /// Any one of these items (the data writes one ingredient or a list of them).
    pub ingredient: Vec<ItemMatch>,
    pub result: AlloyOutput,
    /// Fluids that come out besides the result.
    pub byproducts: Vec<AlloyOutput>,
    pub temperature: i32,
    pub time: i32,
    pub kind: MeltingKind,
}

fn fluid_output(json: &Value) -> Result<AlloyOutput, String> {
    let fluid = if let Some(name) = json.get("fluid").and_then(Value::as_str) {
        FluidMatch::Fluid(ResourceId::try_parse(name).ok_or("bad fluid id")?)
    } else {
        FluidMatch::Tag(ResourceId::try_parse(json.get("tag").and_then(Value::as_str).ok_or("a result needs 'fluid' or 'tag'")?).ok_or("bad fluid tag id")?)
    };
    Ok(AlloyOutput { fluid, amount: json.get("amount").and_then(Value::as_i64).filter(|a| *a > 0).ok_or("the result needs a positive 'amount'")? as i32 })
}

/// The items an ingredient accepts: an item, a tag, a list of those, or `tconstruct:no_container` around one (the container remainder is
/// the recipe's business, not the match's).
pub(crate) fn ingredient_matches(ingredient: &Value) -> Result<Vec<ItemMatch>, String> {
    match ingredient {
        Value::Array(list) => Ok(list.iter().map(ingredient_matches).collect::<Result<Vec<_>, _>>()?.into_iter().flatten().collect()),
        Value::Object(object) => {
            if let Some(kind) = object.get("type").and_then(Value::as_str) {
                match kind {
                    "tconstruct:no_container" => {
                        if let Some(inner) = object.get("match") {
                            return ingredient_matches(inner);
                        }
                    }
                    other => return Err(format!("unsupported ingredient type {other}")),
                }
            }
            if let Some(item) = object.get("item").and_then(Value::as_str) {
                return Ok(vec![ItemMatch::Item(ResourceId::try_parse(item).ok_or("bad item id")?)]);
            }
            let tag = object.get("tag").and_then(Value::as_str).ok_or("the ingredient needs 'item' or 'tag'")?;
            Ok(vec![ItemMatch::Tag(ResourceId::try_parse(tag).ok_or("bad item tag id")?)])
        }
        _ => Err("an ingredient must be an object or a list".to_owned()),
    }
}

impl MeltingRecipe {
    /// Reads a `tconstruct:melting` file. `Ok(None)` when its data conditions are not met.
    pub fn from_json(id: ResourceId, json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        let type_name = json.get("type").and_then(Value::as_str).unwrap_or("");
        // Forge's conditional recipe: the wrapper's own conditions, then the first entry whose conditions hold is the recipe
        if type_name == "forge:conditional" {
            for c in json.get("conditions").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
                if !condition::test(c, conditions)? {
                    return Ok(None);
                }
            }
            for entry in json.get("recipes").and_then(Value::as_array).ok_or("a conditional recipe needs 'recipes'")? {
                let mut holds = true;
                for c in entry.get("conditions").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
                    holds &= condition::test(c, conditions)?;
                }
                if holds {
                    return Self::from_json(id, entry.get("recipe").ok_or("a conditional entry needs a 'recipe'")?, conditions);
                }
            }
            return Ok(None);
        }
        if !matches!(type_name, "tconstruct:melting" | "tconstruct:damagable_melting" | "tconstruct:ore_melting") {
            return Err("not a melting recipe".to_owned());
        }
        for c in json.get("conditions").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
            if !condition::test(c, conditions)? {
                return Ok(None);
            }
        }
        let ingredient = json.get("ingredient").ok_or("a melting recipe needs an 'ingredient'")?;
        let ingredient = ingredient_matches(ingredient)?;
        let byproducts = match json.get("byproducts").and_then(Value::as_array) {
            Some(list) => list.iter().map(fluid_output).collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        let temperature = json.get("temperature").and_then(Value::as_i64).filter(|t| *t >= 0).ok_or("a melting recipe needs a 'temperature'")? as i32;
        let time = json.get("time").and_then(Value::as_i64).filter(|t| *t >= 1).ok_or("a melting recipe needs a 'time' of at least 1")? as i32;
        let kind = match type_name {
            "tconstruct:damagable_melting" => {
                let unit = |v: &Value| v.get("unit_size").and_then(Value::as_i64).filter(|u| *u >= 1).unwrap_or(1) as i32;
                let unit_size = unit(json.get("result").ok_or("a melting recipe needs a 'result'")?);
                let byproduct_sizes = json.get("byproducts").and_then(Value::as_array).map(|list| list.iter().map(unit).collect()).unwrap_or_default();
                MeltingKind::Damagable { unit_size, byproduct_sizes }
            }
            "tconstruct:ore_melting" => {
                let rate_of = |v: Option<&Value>, default: OreRateType| match v.and_then(Value::as_str) {
                    None => Ok(default),
                    Some(name) => OreRateType::from_name(name).ok_or_else(|| format!("unknown ore rate '{name}'")),
                };
                let rate = rate_of(json.get("rate"), OreRateType::Metal)?;
                let byproduct_rates = json
                    .get("byproducts")
                    .and_then(Value::as_array)
                    .map(|list| list.iter().map(|b| rate_of(b.get("rate"), OreRateType::Default)).collect::<Result<Vec<_>, _>>())
                    .transpose()?
                    .unwrap_or_default();
                MeltingKind::Ore { rate, byproduct_rates }
            }
            _ => MeltingKind::Plain,
        };
        Ok(Some(Self { id, ingredient, result: fluid_output(json.get("result").ok_or("a melting recipe needs a 'result'")?)?, byproducts, temperature, time, kind }))
    }
}

impl MeltingRecipe {
    /// The fluid one melting gives: the result, scaled for an ore by the structure's ore rate.
    pub fn output_amount(&self, ore_rate: &OreRate) -> i32 {
        match &self.kind {
            MeltingKind::Ore { rate, .. } => ore_rate.apply(*rate, self.result.amount),
            _ => self.result.amount,
        }
    }

    /// The amounts of the byproducts of one melting; an ore's follow the byproduct rate of the structure.
    pub fn byproduct_amounts(&self, byproduct_rate: &OreRate) -> Vec<i32> {
        match &self.kind {
            MeltingKind::Ore { rate, byproduct_rates } => self
                .byproducts
                .iter()
                .enumerate()
                .map(|(i, b)| byproduct_rate.apply(byproduct_rates.get(i).copied().unwrap_or(OreRateType::Default).or_else(*rate), b.amount))
                .collect(),
            _ => self.byproducts.iter().map(|b| b.amount).collect(),
        }
    }
}

/// Finds the recipe for an item (the recipe manager).
pub trait MeltingRecipes {
    fn recipe_for(&self, item: &ResourceId) -> Option<MeltingRecipe>;
}

/// One slot of a heating structure.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeltingSlot {
    pub item: Option<(ResourceId, u32)>,
    pub current_time: i32,
    pub required_time: i32,
    pub required_temp: i32,
}

/// Whether the tank took the result of a recipe.
pub type Output<'a> = &'a mut dyn FnMut(&MeltingRecipe) -> bool;

impl MeltingSlot {
    fn reset(&mut self) {
        self.current_time = 0;
        self.required_time = 0;
        self.required_temp = 0;
    }

    /// `setStack`: progress is kept while the item stays the same, and the time and temperature follow the recipe of the new item.
    pub fn set_stack(&mut self, stack: Option<(ResourceId, u32)>, recipes: &dyn MeltingRecipes) {
        match &stack {
            None => self.reset(),
            Some((item, _)) if self.item.as_ref().is_none_or(|(old, _)| old != item) => self.current_time = 0,
            _ => {}
        }
        self.item = stack;
        let found = self.item.as_ref().and_then(|(item, _)| recipes.recipe_for(item));
        self.required_time = found.as_ref().map_or(0, |r| r.time * 10);
        self.required_temp = found.map_or(0, |r| r.temperature);
    }

    /// `canHeatItem`: there is an item with a recipe, it is not done waiting for room, and the structure is hot enough.
    pub fn can_heat(&mut self, temperature: i32) -> bool {
        if self.required_time > 0 {
            if self.item.is_none() {
                self.reset();
                return false;
            }
            return self.current_time != NO_SPACE && temperature >= self.required_temp;
        }
        false
    }

    /// `heatItem`: one tick of heating at `rate`; when the time is up the result goes to the tank through `output`.
    pub fn heat(&mut self, temperature: i32, rate: i32, recipes: &dyn MeltingRecipes, output: Output<'_>) {
        if self.current_time == NO_SPACE || self.can_heat(temperature) {
            if self.current_time == NO_SPACE || self.current_time >= self.required_time {
                if self.finish(recipes, output) {
                    self.reset();
                }
            } else {
                self.current_time += rate;
            }
        }
    }

    /// `coolItem`: a slot waiting for room keeps trying; a warm one loses 5 each tick.
    pub fn cool(&mut self, recipes: &dyn MeltingRecipes, output: Output<'_>) {
        if self.current_time == NO_SPACE {
            if self.finish(recipes, output) {
                self.reset();
            }
        } else if self.current_time > 0 && self.required_time > 0 {
            self.current_time -= 5;
        }
    }

    fn finish(&mut self, recipes: &dyn MeltingRecipes, output: Output<'_>) -> bool {
        let Some(recipe) = self.item.as_ref().and_then(|(item, _)| recipes.recipe_for(item)) else { return true };
        if output(&recipe) {
            self.set_stack(None, recipes);
            return true;
        }
        self.current_time = NO_SPACE;
        false
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

    struct Recipes;
    impl MeltingRecipes for Recipes {
        fn recipe_for(&self, item: &ResourceId) -> Option<MeltingRecipe> {
            let json = json!({"type": "tconstruct:melting", "ingredient": {"item": "minecraft:iron_ingot"}, "result": {"amount": 90, "fluid": "tconstruct:molten_iron"}, "temperature": 800, "time": 6});
            (item.to_string() == "minecraft:iron_ingot").then(|| MeltingRecipe::from_json(id("tconstruct:iron"), &json, &EmptyContext).unwrap().unwrap())
        }
    }

    fn slot_with_iron() -> MeltingSlot {
        let mut slot = MeltingSlot::default();
        slot.set_stack(Some((id("minecraft:iron_ingot"), 1)), &Recipes);
        slot
    }

    #[test]
    fn an_item_with_a_recipe_gets_its_time_ten_times_over_and_its_temperature() {
        let slot = slot_with_iron();
        assert_eq!((slot.required_time, slot.required_temp), (60, 800));
        let mut stone = MeltingSlot::default();
        stone.set_stack(Some((id("minecraft:stone"), 1)), &Recipes);
        assert_eq!(stone.required_time, 0, "no recipe, nothing to heat");
    }

    #[test]
    fn a_conditional_wrapper_gives_the_first_entry_whose_conditions_hold() {
        let wrapper = json!({"type": "forge:conditional", "conditions": [{"type": "forge:true"}], "recipes": [
            {"conditions": [{"type": "forge:false"}], "recipe": {"type": "tconstruct:melting", "ingredient": {"item": "a:x"}, "result": {"amount": 1, "fluid": "a:first"}, "temperature": 1, "time": 1}},
            {"conditions": [{"type": "forge:true"}], "recipe": {"type": "tconstruct:melting", "ingredient": {"item": "a:x"}, "result": {"amount": 2, "fluid": "a:second"}, "temperature": 1, "time": 1}}]});
        let recipe = MeltingRecipe::from_json(id("a:w"), &wrapper, &EmptyContext).unwrap().unwrap();
        assert_eq!(recipe.result.amount, 2);
        let gated = json!({"type": "forge:conditional", "conditions": [{"type": "forge:false"}], "recipes": []});
        assert_eq!(MeltingRecipe::from_json(id("a:g"), &gated, &EmptyContext), Ok(None), "the wrapper's own conditions come first");
    }

    #[test]
    fn a_damaged_tool_gives_less_fluid_in_whole_units_and_never_less_than_one_unit() {
        // 300 for a new tool, in units of 25: half worn gives 150, a quarter left rounds down to whole units
        assert_eq!(scale_output(300, 0, 100, 25), 300);
        assert_eq!(scale_output(300, 50, 100, 25), 150);
        assert_eq!(scale_output(300, 63, 100, 25), 100, "111 rounds down to 100");
        assert_eq!(scale_output(300, 99, 100, 25), 25, "nearly broken still gives one unit");
        assert_eq!(scale_output(90, 99, 100, 1), 1);
    }

    #[test]
    fn damagable_and_ore_recipes_carry_their_extra_fields() {
        let damagable = json!({"type": "tconstruct:damagable_melting", "ingredient": {"tag": "a:tools"}, "result": {"amount": 300, "fluid": "a:molten", "unit_size": 25},
            "byproducts": [{"amount": 90, "fluid": "a:other", "unit_size": 10}], "temperature": 1450, "time": 137});
        let recipe = MeltingRecipe::from_json(id("a:d"), &damagable, &EmptyContext).unwrap().unwrap();
        assert_eq!(recipe.kind, MeltingKind::Damagable { unit_size: 25, byproduct_sizes: vec![10] });
        let ore = json!({"type": "tconstruct:ore_melting", "ingredient": {"item": "a:bud"}, "rate": "gem", "result": {"amount": 300, "fluid": "a:molten"}, "temperature": 950, "time": 194});
        assert_eq!(MeltingRecipe::from_json(id("a:o"), &ore, &EmptyContext).unwrap().unwrap().kind, MeltingKind::Ore { rate: OreRateType::Gem, byproduct_rates: vec![] });
    }

    #[test]
    fn an_ore_gives_more_than_an_ingot_in_the_melter_and_follows_the_foundry_rates_there() {
        let ore = json!({"type": "tconstruct:ore_melting", "ingredient": {"item": "a:raw"}, "rate": "metal", "result": {"amount": 90, "fluid": "a:molten"},
            "byproducts": [{"amount": 90, "fluid": "a:other"}, {"amount": 100, "fluid": "a:gem", "rate": "gem"}], "temperature": 800, "time": 100});
        let recipe = MeltingRecipe::from_json(id("a:r"), &ore, &EmptyContext).unwrap().unwrap();
        assert_eq!(recipe.output_amount(&OreRate::MELTER), 120, "12 nuggets, a third more than 9");
        assert_eq!(recipe.output_amount(&OreRate::FOUNDRY), 90);
        // the first byproduct has no rate of its own and takes the ore's (metal); the second names gem
        assert_eq!(recipe.byproduct_amounts(&OreRate::FOUNDRY_BYPRODUCT), vec![30, 100]);
        let plain = json!({"type": "tconstruct:melting", "ingredient": {"item": "a:x"}, "result": {"amount": 90, "fluid": "a:molten"}, "temperature": 1, "time": 1});
        assert_eq!(MeltingRecipe::from_json(id("a:p"), &plain, &EmptyContext).unwrap().unwrap().output_amount(&OreRate::MELTER), 90);
        assert_eq!(OreRate::MELTER.apply(OreRateType::None, 90), 90);
        assert!(MeltingRecipe::from_json(id("a:b"), &json!({"type": "tconstruct:ore_melting", "ingredient": {"item": "a:x"}, "rate": "lumpy", "result": {"amount": 1, "fluid": "a:m"}, "temperature": 1, "time": 1}), &EmptyContext).is_err());
    }

    #[test]
    fn heating_runs_to_the_required_time_then_the_result_goes_to_the_tank() {
        let mut slot = slot_with_iron();
        let mut taken = Vec::new();
        let mut take = |r: &MeltingRecipe| {
            taken.push(r.result.amount);
            true
        };
        for _ in 0..60 {
            slot.heat(900, 1, &Recipes, &mut take);
        }
        assert_eq!(slot.current_time, 60);
        slot.heat(900, 1, &Recipes, &mut take);
        assert_eq!(taken, vec![90]);
        assert_eq!(slot, MeltingSlot::default(), "the slot is empty and reset");
    }

    #[test]
    fn a_cold_structure_does_not_heat_and_a_warm_slot_cools_five_a_tick() {
        let mut slot = slot_with_iron();
        slot.heat(700, 1, &Recipes, &mut |_| true);
        assert_eq!(slot.current_time, 0);
        slot.heat(900, 20, &Recipes, &mut |_| true);
        slot.cool(&Recipes, &mut |_| true);
        assert_eq!(slot.current_time, 15);
    }

    #[test]
    fn a_full_tank_makes_the_slot_wait_and_it_finishes_when_room_appears() {
        let mut slot = slot_with_iron();
        slot.current_time = 60;
        slot.heat(900, 1, &Recipes, &mut |_| false);
        assert_eq!(slot.current_time, NO_SPACE);
        slot.cool(&Recipes, &mut |_| false);
        assert_eq!(slot.current_time, NO_SPACE, "still no room");
        slot.cool(&Recipes, &mut |_| true);
        assert!(slot.item.is_none() && slot.current_time == 0);
    }

    #[test]
    fn progress_stays_for_the_same_item_and_goes_for_another() {
        let mut slot = slot_with_iron();
        slot.heat(900, 30, &Recipes, &mut |_| true);
        slot.set_stack(Some((id("minecraft:iron_ingot"), 2)), &Recipes);
        assert_eq!(slot.current_time, 30, "more of the same item keeps the progress");
        slot.set_stack(Some((id("minecraft:stone"), 1)), &Recipes);
        assert_eq!(slot.current_time, 0);
    }
}
