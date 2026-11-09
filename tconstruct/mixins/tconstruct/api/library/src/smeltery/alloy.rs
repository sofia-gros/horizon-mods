//! Alloying (`AlloyRecipe`): fluids in a tank set become another fluid when their amounts and the temperature allow it.
//!
//! A recipe lists inputs (a fluid or a fluid tag with an amount; a catalyst is needed but not used up), an output and the temperature the
//! structure must have. The tank is described by [`AlloyTank`]; a recipe matches when every input finds its own tank, and can run when
//! each of those tanks holds enough and the output fits.
use serde_json::Value;

use crate::condition::{self, ConditionContext};
use crate::resource::ResourceId;

/// What an input accepts: one fluid, or any fluid of a tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FluidMatch {
    Fluid(ResourceId),
    Tag(ResourceId),
}

/// One input of an alloy recipe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlloyIngredient {
    pub fluid: FluidMatch,
    /// Millibuckets used for each run of the recipe.
    pub amount: i32,
    /// A catalyst must be present in this amount but is not used up.
    pub catalyst: bool,
}

/// A fluid with an amount, as a tank holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fluid {
    pub id: ResourceId,
    pub amount: i32,
}

/// What the recipe makes: a fluid, or the first fluid of a tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlloyOutput {
    pub fluid: FluidMatch,
    pub amount: i32,
}

/// The tanks an alloy recipe works on (`IAlloyTank` and `IMutableAlloyTank`).
pub trait AlloyTank {
    fn tanks(&self) -> usize;
    /// The fluid in tank `index`, None when it is empty.
    fn fluid(&self, index: usize) -> Option<Fluid>;
    fn temperature(&self) -> i32;
    /// Whether `fluid` of `amount` fits after `drained` millibuckets have been taken out.
    fn can_fit(&self, fluid: &ResourceId, amount: i32, drained: i32) -> bool;
    /// Takes up to `amount` from tank `index`; returns how much was taken.
    fn drain(&mut self, index: usize, amount: i32) -> i32;
    /// Puts the fluid in; returns how much went in.
    fn fill(&mut self, fluid: &ResourceId, amount: i32) -> i32;
}

/// Which fluids belong to a tag.
pub trait FluidTags {
    fn members(&self, tag: &ResourceId) -> Vec<ResourceId>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlloyRecipe {
    pub id: ResourceId,
    pub inputs: Vec<AlloyIngredient>,
    pub output: AlloyOutput,
    pub temperature: i32,
}

fn fluid_match(json: &Value) -> Result<FluidMatch, String> {
    if let Some(name) = json.get("fluid").and_then(Value::as_str) {
        return Ok(FluidMatch::Fluid(ResourceId::try_parse(name).ok_or("bad fluid id")?));
    }
    let tag = json.get("tag").and_then(Value::as_str).ok_or("a fluid needs 'fluid' or 'tag'")?;
    Ok(FluidMatch::Tag(ResourceId::try_parse(tag).ok_or("bad fluid tag id")?))
}

impl AlloyRecipe {
    /// Reads a recipe file. `Ok(None)` when its data conditions are not met (the recipe is skipped, as the original skips it).
    pub fn from_json(id: ResourceId, json: &Value, conditions: &dyn ConditionContext) -> Result<Option<Self>, String> {
        // Forge's conditional recipe: the first entry whose conditions hold is the recipe
        if json.get("type").and_then(Value::as_str) == Some("forge:conditional") {
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
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:alloy") {
            return Err("not an alloy recipe".to_owned());
        }
        if let Some(list) = json.get("conditions").and_then(Value::as_array) {
            for c in list {
                if !condition::test(c, conditions)? {
                    return Ok(None);
                }
            }
        }
        let inputs = json
            .get("inputs")
            .and_then(Value::as_array)
            .ok_or("an alloy needs 'inputs'")?
            .iter()
            .map(|i| {
                Ok(AlloyIngredient {
                    fluid: fluid_match(i)?,
                    amount: i.get("amount").and_then(Value::as_i64).filter(|a| *a > 0).ok_or("an input needs a positive 'amount'")? as i32,
                    catalyst: i.get("catalyst").and_then(Value::as_bool).unwrap_or(false),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if inputs.len() < 2 {
            return Err("an alloy needs at least two inputs".to_owned());
        }
        let result = json.get("result").ok_or("an alloy needs a 'result'")?;
        let output = AlloyOutput { fluid: fluid_match(result)?, amount: result.get("amount").and_then(Value::as_i64).filter(|a| *a > 0).ok_or("the result needs a positive 'amount'")? as i32 };
        let temperature = json.get("temperature").and_then(Value::as_i64).filter(|t| *t >= 1).ok_or("an alloy needs a 'temperature' of at least 1")? as i32;
        Ok(Some(Self { id, inputs, output, temperature }))
    }

    fn accepts(&self, ingredient: &AlloyIngredient, fluid: &ResourceId, tags: &dyn FluidTags) -> bool {
        match &ingredient.fluid {
            FluidMatch::Fluid(id) => id == fluid,
            FluidMatch::Tag(tag) => tags.members(tag).contains(fluid),
        }
    }

    /// The tank each input would take its fluid from, each tank once; None when some input finds none. `check_size` also asks for enough fluid.
    fn assign(&self, tank: &dyn AlloyTank, tags: &dyn FluidTags, check_size: bool) -> Option<Vec<usize>> {
        let mut used: Vec<bool> = (0..tank.tanks()).map(|i| tank.fluid(i).is_none()).collect();
        let mut found = Vec::new();
        for ingredient in &self.inputs {
            let index = (0..tank.tanks()).find(|&i| {
                !used[i] && tank.fluid(i).is_some_and(|f| self.accepts(ingredient, &f.id, tags) && (!check_size || f.amount >= ingredient.amount))
            })?;
            used[index] = true;
            found.push(index);
        }
        Some(found)
    }

    /// The fluid the recipe makes, with a tag resolved to its first member.
    pub fn output_fluid(&self, tags: &dyn FluidTags) -> Option<ResourceId> {
        match &self.output.fluid {
            FluidMatch::Fluid(id) => Some(id.clone()),
            FluidMatch::Tag(tag) => tags.members(tag).into_iter().next(),
        }
    }

    /// `matches`: every input finds a tank with a fluid of its kind, whatever the amounts.
    pub fn matches(&self, tank: &dyn AlloyTank, tags: &dyn FluidTags) -> bool {
        self.assign(tank, tags, false).is_some()
    }

    /// The amount drained for one run, when the recipe can run now (hot enough, enough of each fluid, room for the result).
    fn plan(&self, tank: &dyn AlloyTank, tags: &dyn FluidTags) -> Option<Vec<(usize, i32)>> {
        if tank.temperature() < self.temperature {
            return None;
        }
        let tanks = self.assign(tank, tags, true)?;
        let drains: Vec<(usize, i32)> = self.inputs.iter().zip(&tanks).filter(|(i, _)| !i.catalyst).map(|(i, t)| (*t, i.amount)).collect();
        let drained: i32 = drains.iter().map(|(_, a)| a).sum();
        let output = self.output_fluid(tags)?;
        tank.can_fit(&output, self.output.amount, drained).then_some(drains)
    }

    /// `canPerform`.
    pub fn can_perform(&self, tank: &dyn AlloyTank, tags: &dyn FluidTags) -> bool {
        self.plan(tank, tags).is_some()
    }

    /// `performRecipe`: drains the inputs (not the catalysts) and fills the output. False when it could not run.
    pub fn perform(&self, tank: &mut dyn AlloyTank, tags: &dyn FluidTags) -> bool {
        let Some(drains) = self.plan(tank, tags) else { return false };
        let Some(output) = self.output_fluid(tags) else { return false };
        for (index, amount) in drains {
            tank.drain(index, amount);
        }
        tank.fill(&output, self.output.amount);
        true
    }
}

/// `SingleAlloyingModule.findRecipe`: the first recipe that can run now and whose result fits the tank (the melter and the alloyer
/// have one tank, so the recipe that works on it is the one used).
pub fn find_single<'a>(recipes: &'a [AlloyRecipe], tank: &dyn AlloyTank, tags: &dyn FluidTags) -> Option<&'a AlloyRecipe> {
    recipes.iter().find(|r| r.can_perform(tank, tags))
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
    impl FluidTags for Tags {
        fn members(&self, tag: &ResourceId) -> Vec<ResourceId> {
            match tag.to_string().as_str() {
                "forge:molten_copper" => vec![id("tconstruct:molten_copper")],
                "forge:molten_tin" => vec![id("tconstruct:molten_tin")],
                "forge:molten_bronze" => vec![id("tconstruct:molten_bronze")],
                _ => Vec::new(),
            }
        }
    }

    struct Tank {
        fluids: Vec<Option<Fluid>>,
        temperature: i32,
        capacity: i32,
    }
    impl Tank {
        fn of(fluids: &[(&str, i32)], temperature: i32, capacity: i32) -> Self {
            Self { fluids: fluids.iter().map(|(f, a)| Some(Fluid { id: id(f), amount: *a })).collect(), temperature, capacity }
        }
        fn amount(&self, fluid: &str) -> i32 {
            self.fluids.iter().flatten().filter(|f| f.id == id(fluid)).map(|f| f.amount).sum()
        }
    }
    impl AlloyTank for Tank {
        fn tanks(&self) -> usize {
            self.fluids.len()
        }
        fn fluid(&self, index: usize) -> Option<Fluid> {
            self.fluids[index].clone()
        }
        fn temperature(&self) -> i32 {
            self.temperature
        }
        fn can_fit(&self, _: &ResourceId, amount: i32, drained: i32) -> bool {
            self.fluids.iter().flatten().map(|f| f.amount).sum::<i32>() - drained + amount <= self.capacity
        }
        fn drain(&mut self, index: usize, amount: i32) -> i32 {
            let Some(f) = self.fluids[index].as_mut() else { return 0 };
            let taken = f.amount.min(amount);
            f.amount -= taken;
            if f.amount == 0 {
                self.fluids[index] = None;
            }
            taken
        }
        fn fill(&mut self, fluid: &ResourceId, amount: i32) -> i32 {
            match self.fluids.iter_mut().flatten().find(|f| f.id == *fluid) {
                Some(f) => f.amount += amount,
                None => self.fluids.push(Some(Fluid { id: fluid.clone(), amount })),
            }
            amount
        }
    }

    fn bronze() -> AlloyRecipe {
        let json = json!({"type": "tconstruct:alloy", "inputs": [{"amount": 270, "tag": "forge:molten_copper"}, {"amount": 90, "tag": "forge:molten_tin"}],
            "result": {"amount": 360, "tag": "forge:molten_bronze"}, "temperature": 700});
        AlloyRecipe::from_json(id("tconstruct:smeltery/alloys/molten_bronze"), &json, &EmptyContext).unwrap().unwrap()
    }

    #[test]
    fn copper_and_tin_make_bronze_when_hot_enough_and_the_amounts_are_there() {
        let recipe = bronze();
        let mut tank = Tank::of(&[("tconstruct:molten_copper", 300), ("tconstruct:molten_tin", 100)], 800, 4000);
        assert!(recipe.matches(&tank, &Tags) && recipe.can_perform(&tank, &Tags));
        assert!(recipe.perform(&mut tank, &Tags));
        assert_eq!((tank.amount("tconstruct:molten_copper"), tank.amount("tconstruct:molten_tin"), tank.amount("tconstruct:molten_bronze")), (30, 10, 360));
    }

    #[test]
    fn a_cold_tank_too_little_fluid_or_no_room_stop_the_recipe() {
        let recipe = bronze();
        assert!(!recipe.can_perform(&Tank::of(&[("tconstruct:molten_copper", 300), ("tconstruct:molten_tin", 100)], 600, 4000), &Tags), "too cold");
        let thin = Tank::of(&[("tconstruct:molten_copper", 100), ("tconstruct:molten_tin", 100)], 800, 4000);
        assert!(recipe.matches(&thin, &Tags) && !recipe.can_perform(&thin, &Tags), "matches by kind but not enough copper");
        assert!(!recipe.can_perform(&Tank::of(&[("tconstruct:molten_copper", 300), ("tconstruct:molten_tin", 100)], 800, 399), &Tags), "the result would overfill the tank by 1");
    }

    #[test]
    fn a_catalyst_is_needed_but_not_used_up() {
        let json = json!({"type": "tconstruct:alloy", "inputs": [{"amount": 250, "catalyst": true, "fluid": "minecraft:water"}, {"amount": 100, "fluid": "minecraft:lava"}],
            "result": {"amount": 100, "fluid": "tconstruct:molten_obsidian"}, "temperature": 1000});
        let recipe = AlloyRecipe::from_json(id("tconstruct:obsidian"), &json, &EmptyContext).unwrap().unwrap();
        let mut tank = Tank::of(&[("minecraft:water", 300), ("minecraft:lava", 150)], 1100, 4000);
        assert!(recipe.perform(&mut tank, &Tags));
        assert_eq!((tank.amount("minecraft:water"), tank.amount("minecraft:lava"), tank.amount("tconstruct:molten_obsidian")), (300, 50, 100));
        assert!(!recipe.can_perform(&Tank::of(&[("minecraft:water", 100), ("minecraft:lava", 150)], 1100, 4000), &Tags), "water short of the catalyst amount");
    }

    #[test]
    fn the_single_alloying_module_runs_the_first_recipe_that_can_run() {
        let obsidian = AlloyRecipe::from_json(id("a:obsidian"), &json!({"type": "tconstruct:alloy", "inputs": [{"amount": 250, "catalyst": true, "fluid": "minecraft:water"}, {"amount": 100, "fluid": "minecraft:lava"}],
            "result": {"amount": 100, "fluid": "tconstruct:molten_obsidian"}, "temperature": 1000}), &EmptyContext).unwrap().unwrap();
        let recipes = [bronze(), obsidian];
        let hot = Tank::of(&[("minecraft:water", 300), ("minecraft:lava", 150)], 1100, 4000);
        assert_eq!(find_single(&recipes, &hot, &Tags).map(|r| r.id.to_string()), Some("a:obsidian".to_owned()));
        let cold = Tank::of(&[("minecraft:water", 300), ("minecraft:lava", 150)], 900, 4000);
        assert!(find_single(&recipes, &cold, &Tags).is_none());
    }

    #[test]
    fn two_inputs_never_take_the_same_tank() {
        let json = json!({"type": "tconstruct:alloy", "inputs": [{"amount": 10, "tag": "forge:molten_copper"}, {"amount": 10, "tag": "forge:molten_copper"}],
            "result": {"amount": 20, "fluid": "tconstruct:molten_bronze"}, "temperature": 100});
        let recipe = AlloyRecipe::from_json(id("tconstruct:twice"), &json, &EmptyContext).unwrap().unwrap();
        assert!(!recipe.matches(&Tank::of(&[("tconstruct:molten_copper", 500)], 500, 4000), &Tags));
    }

    #[test]
    fn conditions_skip_a_recipe_and_bad_data_is_an_error() {
        let gated = json!({"type": "tconstruct:alloy", "conditions": [{"type": "mantle:tag_filled", "tag": "forge:ingots/tin"}],
            "inputs": [{"amount": 1, "fluid": "a:b"}, {"amount": 1, "fluid": "a:c"}], "result": {"amount": 1, "fluid": "a:d"}, "temperature": 1});
        assert_eq!(AlloyRecipe::from_json(id("a:x"), &gated, &EmptyContext), Ok(None), "no tin ingot tag members, so the recipe is skipped");
        let one = json!({"type": "tconstruct:alloy", "inputs": [{"amount": 1, "fluid": "a:b"}], "result": {"amount": 1, "fluid": "a:d"}, "temperature": 1});
        assert!(AlloyRecipe::from_json(id("a:y"), &one, &EmptyContext).is_err());
    }
}
