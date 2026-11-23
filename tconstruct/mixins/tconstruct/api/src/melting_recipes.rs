//! Melting, alloying, melting fuel and entity melting recipes, from SlimeKnights MeltingRecipe, OreMeltingRecipe,
//! DamageableMeltingRecipe, MaterialMeltingRecipe, MeltingFuel, EntityMeltingRecipe and AlloyRecipe (MIT; see LICENSE).
//! Decoding follows each LOADER (field names, defaults and minimums); the arithmetic follows the Java methods.
//! Nothing here is connected to a block entity or the recipe manager yet.
#![allow(dead_code)]
use crate::{
    fluid_ingredient::{id_of, int_field, only_keys, FluidIngredient, FluidOutput, FluidTags, InTag},
    fluid_values::{scale_output, OreRateType},
    material_recipes::Ingredient,
    tool_materials::MaterialVariant,
};
pub(crate) use crate::fluid_values::OreRate;
use pluto_lib::Identifier;
use serde_json::{Map, Value};

/// The keys every recipe object may carry besides its own fields.
pub(crate) fn recipe_keys(fields: &Map<String, Value>, own: &[&str]) -> Result<(), String> {
    let mut allowed = vec!["type", "conditions", "group"];
    allowed.extend_from_slice(own);
    only_keys(fields, "recipe", &allowed)
}

fn byproduct_list(fields: &Map<String, Value>, extra: &[&str]) -> Result<Vec<(FluidOutput, Map<String, Value>)>, String> {
    match fields.get("byproducts") {
        None => Ok(Vec::new()),
        Some(Value::Array(list)) => list.iter().map(|value| {
            let output = FluidOutput::decode(value, extra)?;
            Ok((output, value.as_object().cloned().unwrap_or_default()))
        }).collect(),
        Some(_) => Err("byproducts must be an array".into()),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MeltingKind {
    /// `tconstruct:melting`.
    Plain,
    /// `tconstruct:ore_melting`: the result and byproducts grow with the container's ore rate.
    Ore { rate: OreRateType, byproduct_rates: Vec<OreRateType> },
    /// `tconstruct:damagable_melting`: the result scales with the item's remaining durability, in `unit_size` steps.
    Damageable { unit_size: i32, byproduct_sizes: Vec<i32> },
}

/// A recipe melting one ingredient into a fluid.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Melting {
    pub id: Identifier,
    pub input: Ingredient,
    pub output: FluidOutput,
    /// Celsius.
    pub temperature: i32,
    /// In 1/5th seconds at the reference rate; a slot needs ten times this much progress.
    pub time: i32,
    pub byproducts: Vec<FluidOutput>,
    pub kind: MeltingKind,
}

impl Melting {
    pub(crate) fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        let kind_name = fields.get("type").and_then(Value::as_str).ok_or("recipe needs a type")?;
        let (own, result_extra, byproduct_extra): (&[&str], &[&str], &[&str]) = match kind_name {
            "tconstruct:ore_melting" => (&["rate"], &[], &["rate"]),
            "tconstruct:damagable_melting" => (&[], &["unit_size"], &["unit_size"]),
            _ => (&[], &[], &[]),
        };
        recipe_keys(fields, &[&["ingredient", "result", "temperature", "time", "byproducts"], own].concat())?;
        let result = fields.get("result").ok_or("result missing")?;
        let byproducts = byproduct_list(fields, byproduct_extra)?;
        let kind = match kind_name {
            "tconstruct:melting" => MeltingKind::Plain,
            "tconstruct:ore_melting" => MeltingKind::Ore {
                rate: rate_type(fields.get("rate").ok_or("rate missing")?)?,
                byproduct_rates: byproducts.iter().map(|(_, own)| own.get("rate").map_or(Ok(OreRateType::Default), rate_type)).collect::<Result<_, _>>()?,
            },
            "tconstruct:damagable_melting" => MeltingKind::Damageable {
                unit_size: int_field(result.as_object().ok_or("result must be an object")?, "unit_size", 1, Some(1))?,
                byproduct_sizes: byproducts.iter().map(|(_, own)| int_field(own, "unit_size", 1, Some(1))).collect::<Result<_, _>>()?,
            },
            other => return Err(format!("not a melting recipe type {other}")),
        };
        Ok(Self {
            id: id.clone(),
            input: Ingredient::decode(fields.get("ingredient").ok_or("ingredient missing")?)?,
            output: FluidOutput::decode(result, result_extra)?,
            temperature: int_field(fields, "temperature", 0, None)?,
            time: int_field(fields, "time", 1, None)?,
            byproducts: byproducts.into_iter().map(|(output, _)| output).collect(),
            kind,
        })
    }

    /// `matches`: the item (and whether it has a crafting remainder) satisfies the ingredient.
    pub(crate) fn matches(&self, item: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, has_remainder: bool) -> bool {
        self.input.matches_with_remainder(item, in_tag, has_remainder)
    }

    /// getOutput's amount for a container with `ore_rate`; `damage` is the item's (damage, max damage) when it has durability.
    pub(crate) fn output_amount(&self, ore_rate: &OreRate, damage: Option<(i32, i32)>) -> i32 {
        match (&self.kind, damage) {
            (MeltingKind::Ore { rate, .. }, _) => ore_rate.apply(*rate, self.output.amount),
            (MeltingKind::Damageable { unit_size, .. }, Some((damage, max))) if max > 0 => scale_output(self.output.amount, damage, max, *unit_size),
            _ => self.output.amount,
        }
    }

    /// handleByproducts' amounts, parallel to `byproducts`. An ore's byproducts use the foundry byproduct rate whichever
    /// container melts it; a damageable item's scale with its damage by their own unit size.
    pub(crate) fn byproduct_amounts(&self, byproduct_rate: &OreRate, damage: Option<(i32, i32)>) -> Vec<i32> {
        self.byproducts.iter().enumerate().map(|(index, byproduct)| match (&self.kind, damage) {
            (MeltingKind::Ore { rate, byproduct_rates }, _) => byproduct_rate.apply(byproduct_rates[index].or_else(*rate), byproduct.amount),
            (MeltingKind::Damageable { unit_size, byproduct_sizes }, Some((damage, max))) if max > 0 =>
                scale_output(byproduct.amount, damage, max, byproduct_sizes.get(index).copied().unwrap_or(*unit_size)),
            _ => byproduct.amount,
        }).collect()
    }
}

fn rate_type(value: &Value) -> Result<OreRateType, String> {
    value.as_str().and_then(OreRateType::parse).ok_or_else(|| format!("invalid ore rate {value}"))
}

/// MaterialMeltingRecipe: every castable part of a material melts into its material's fluid, scaled by the part's casting cost.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaterialMelting {
    pub id: Identifier,
    pub input: MaterialVariant,
    pub temperature: i32,
    pub result: FluidOutput,
    pub byproducts: Vec<FluidOutput>,
}

impl MaterialMelting {
    pub(crate) fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        recipe_keys(fields, &["input", "temperature", "result", "byproducts"])?;
        Ok(Self {
            id: id.clone(),
            input: MaterialVariant::parse(fields.get("input").and_then(Value::as_str).ok_or("input must be a material ID")?).ok_or("invalid material variant")?,
            temperature: int_field(fields, "temperature", 1, None)?,
            result: FluidOutput::decode(fields.get("result").ok_or("result missing")?, &[])?,
            byproducts: byproduct_list(fields, &[])?.into_iter().map(|(output, _)| output).collect(),
        })
    }
    /// matches: a part with a casting cost whose material the input matches.
    pub(crate) fn matches(&self, cost: i32, material: &MaterialVariant) -> bool {
        self.input != MaterialVariant::unknown() && cost != 0 && self.input.matches(material)
    }
    /// getTime: for the amount the part gives.
    pub(crate) fn time(&self, cost: i32) -> i32 { crate::fluid_values::calc_time_for_amount(self.temperature, self.result.amount.wrapping_mul(cost)) }
    pub(crate) fn output_amount(&self, cost: i32) -> i32 { self.result.amount.wrapping_mul(cost) }
    pub(crate) fn byproduct_amounts(&self, cost: i32) -> Vec<i32> { self.byproducts.iter().map(|byproduct| byproduct.amount.wrapping_mul(cost)).collect() }
}

/// MeltingFuel: a fluid (or, with no fluid, the solid fuel) that heats a melter or smeltery.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MeltingFuel {
    pub id: Identifier,
    /// None for the solid fuel recipe, which takes its burn time from the item.
    pub fluid: Option<FluidIngredient>,
    pub duration: i32,
    pub temperature: i32,
    pub rate: i32,
}

impl MeltingFuel {
    pub(crate) fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        recipe_keys(fields, &["fluid", "duration", "temperature", "rate"])?;
        let fluid = fields.get("fluid").map(FluidIngredient::decode).transpose()?;
        let duration = match fields.get("duration") { None => 0, Some(_) => int_field(fields, "duration", 1, None)? };
        if fluid.is_some() && duration == 0 { return Err("Missing JSON field duration".into()); }
        Ok(Self { id: id.clone(), fluid, duration, temperature: int_field(fields, "temperature", 1, None)?, rate: int_field(fields, "rate", 1, None)? })
    }
    pub(crate) fn matches(&self, fluid: &Identifier, in_tag: InTag) -> bool { self.fluid.as_ref().is_some_and(|input| input.matches(fluid, in_tag)) }
    /// getAmount: how much of the fluid one burn takes.
    pub(crate) fn amount(&self, fluid: &Identifier, in_tag: InTag) -> i32 { self.fluid.as_ref().map_or(0, |input| input.amount(fluid, in_tag)) }
}

/// Mantle's EntityIngredient: entity types, an entity type tag, or an array of those.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EntityIngredient { Types(Vec<Identifier>), Tag(Identifier), Any(Vec<EntityIngredient>) }

impl EntityIngredient {
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        match value {
            Value::Array(members) if !members.is_empty() => Ok(Self::Any(members.iter().map(Self::decode).collect::<Result<_, _>>()?)),
            Value::Object(fields) => {
                only_keys(fields, "entity ingredient", &["type", "types", "tag"])?;
                match (fields.get("type"), fields.get("types"), fields.get("tag")) {
                    (Some(single), None, None) => Ok(Self::Types(vec![id_of(single)?])),
                    (None, Some(Value::Array(types)), None) if !types.is_empty() => Ok(Self::Types(types.iter().map(id_of).collect::<Result<_, _>>()?)),
                    (None, None, Some(tag)) => Ok(Self::Tag(id_of(tag)?)),
                    _ => Err("an entity ingredient names exactly one of type, types or tag".into()),
                }
            }
            _ => Err("entity ingredient must be an object or a nonempty array".into()),
        }
    }
    pub(crate) fn matches(&self, entity: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        match self {
            Self::Types(types) => types.contains(entity),
            Self::Tag(tag) => in_tag(entity, tag),
            Self::Any(members) => members.iter().any(|member| member.matches(entity, in_tag)),
        }
    }
}

/// EntityMeltingRecipe: hurting an entity in the smeltery for `damage` yields the fluid.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EntityMelting { pub id: Identifier, pub entity: EntityIngredient, pub output: FluidOutput, pub damage: i32 }

impl EntityMelting {
    pub(crate) fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        recipe_keys(fields, &["entity", "result", "damage"])?;
        Ok(Self {
            id: id.clone(),
            entity: EntityIngredient::decode(fields.get("entity").ok_or("entity missing")?)?,
            output: FluidOutput::decode(fields.get("result").ok_or("result missing")?, &[])?,
            damage: int_field(fields, "damage", 1, Some(2))?,
        })
    }
}

/// One AlloyRecipe input: the fluid and amount to take, or a catalyst that must be present but is not used up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AlloyInput { pub fluid: FluidIngredient, pub catalyst: bool }

impl AlloyInput {
    /// The compact form is the fluid ingredient itself (with an optional `catalyst` beside it); `match` names the ingredient otherwise.
    fn decode(value: &Value) -> Result<Self, String> {
        match value {
            Value::Object(fields) if fields.contains_key("match") => {
                only_keys(fields, "alloy input", &["match", "catalyst"])?;
                Ok(Self { fluid: FluidIngredient::decode(&fields["match"])?, catalyst: bool_field(fields, "catalyst")? })
            }
            Value::Object(fields) => {
                let mut plain = fields.clone();
                plain.remove("catalyst");
                Ok(Self { fluid: FluidIngredient::decode(&Value::Object(plain))?, catalyst: bool_field(fields, "catalyst")? })
            }
            other => Ok(Self { fluid: FluidIngredient::decode(other)?, catalyst: false }),
        }
    }
}

fn bool_field(fields: &Map<String, Value>, key: &str) -> Result<bool, String> {
    match fields.get(key) { None => Ok(false), Some(Value::Bool(value)) => Ok(*value), Some(_) => Err(format!("{key} must be a boolean")) }
}

/// The tank an alloy recipe reads and changes: the smeltery's tank, or an alloyer's side tanks and output tank.
pub(crate) trait AlloyTank {
    fn tanks(&self) -> usize;
    /// The fluid and amount in a tank, None when it is empty.
    fn fluid_in_tank(&self, tank: usize) -> Option<(Identifier, i32)>;
    fn temperature(&self) -> i32;
    /// Whether `output` fits after `removed` of the inputs is taken out.
    fn can_fit(&self, output: &(Identifier, i32), removed: i32) -> bool;
    /// Takes up to `amount` of `fluid` out of the tank, returning what was taken. A smeltery drains the fluid wherever it is;
    /// an alloyer drains the side tank at the index. Indexes are those of `fluid_in_tank` before any drain of this recipe.
    fn drain(&mut self, tank: usize, fluid: &Identifier, amount: i32) -> i32;
    /// Adds the output, returning what was added.
    fn fill(&mut self, fluid: &Identifier, amount: i32) -> i32;
}

/// AlloyRecipe: at least two inputs, mixed into the output at or above the temperature.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Alloy { pub id: Identifier, pub inputs: Vec<AlloyInput>, pub output: FluidOutput, pub temperature: i32 }

impl Alloy {
    pub(crate) fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        recipe_keys(fields, &["inputs", "result", "temperature"])?;
        let inputs = fields.get("inputs").and_then(Value::as_array).ok_or("inputs must be an array")?;
        if inputs.len() < 2 { return Err("inputs must have at least 2 entries".into()); }
        Ok(Self {
            id: id.clone(),
            inputs: inputs.iter().map(AlloyInput::decode).collect::<Result<_, _>>()?,
            output: FluidOutput::decode(fields.get("result").ok_or("result missing")?, &[])?,
            temperature: int_field(fields, "temperature", 1, None)?,
        })
    }

    /// The index of the first unused tank holding a fluid the ingredient names (and, when `check_size`, enough of it).
    fn find_match(ingredient: &FluidIngredient, tank: &dyn AlloyTank, used: &mut [bool], check_size: bool, in_tag: InTag) -> Option<usize> {
        for index in 0..tank.tanks() {
            if used[index] { continue; }
            let Some((fluid, amount)) = tank.fluid_in_tank(index) else { continue; };
            if if check_size { ingredient.test(&fluid, amount, in_tag) } else { ingredient.matches(&fluid, in_tag) } {
                used[index] = true;
                return Some(index);
            }
        }
        None
    }

    /// Empty tanks count as used, so they are never matched.
    fn used_marks(tank: &dyn AlloyTank) -> Vec<bool> { (0..tank.tanks()).map(|index| tank.fluid_in_tank(index).is_none()).collect() }

    /// matches: every input has a fluid of its kind somewhere, whatever the amounts.
    pub(crate) fn matches(&self, tank: &dyn AlloyTank, in_tag: InTag) -> bool {
        let mut used = Self::used_marks(tank);
        self.inputs.iter().all(|input| Self::find_match(&input.fluid, tank, &mut used, false, in_tag).is_some())
    }

    /// The tanks to drain and by how much, or None when the temperature is too low or an input is missing or short.
    fn plan(&self, tank: &dyn AlloyTank, in_tag: InTag) -> Option<(Vec<(usize, Identifier, i32)>, i32)> {
        if tank.temperature() < self.temperature { return None; }
        let mut used = Self::used_marks(tank);
        let mut drains = Vec::new();
        let mut total = 0;
        for input in &self.inputs {
            let index = Self::find_match(&input.fluid, tank, &mut used, true, in_tag)?;
            if !input.catalyst {
                let (fluid, _) = tank.fluid_in_tank(index).expect("a matched tank is not empty");
                let amount = input.fluid.amount(&fluid, in_tag);
                total += amount;
                drains.push((index, fluid, amount));
            }
        }
        Some((drains, total))
    }

    fn output_stack(&self, tags: &FluidTags) -> Option<(Identifier, i32)> { self.output.resolve(tags).map(|fluid| (fluid, self.output.amount)) }

    /// canPerform: hot enough, every input present in its amount, and the output fits.
    pub(crate) fn can_perform(&self, tank: &dyn AlloyTank, tags: &FluidTags) -> bool {
        let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
        let (Some((_, total)), Some(output)) = (self.plan(tank, &in_tag), self.output_stack(tags)) else { return false; };
        tank.can_fit(&output, total)
    }

    /// performRecipe: drains each non-catalyst input by its amount, then adds the output. Returns whether it ran.
    pub(crate) fn perform(&self, tank: &mut dyn AlloyTank, tags: &FluidTags) -> bool {
        let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
        let (Some((drains, total)), Some(output)) = (self.plan(tank, &in_tag), self.output_stack(tags)) else { return false; };
        if !tank.can_fit(&output, total) { return false; }
        for (index, fluid, amount) in drains { tank.drain(index, &fluid, amount); }
        tank.fill(&output.0, output.1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn fields(value: Value) -> Map<String, Value> { value.as_object().unwrap().clone() }
    fn tags() -> FluidTags {
        FluidTags::load(&[
            (id("forge:molten_gold"), json!({"values": ["tconstruct:molten_gold"]})),
            (id("tconstruct:molten_debris"), json!({"values": ["tconstruct:molten_debris"]})),
            (id("forge:molten_netherite"), json!({"values": ["tconstruct:molten_netherite"]})),
        ])
    }

    /// The smeltery's tank: one entry per fluid, a shared capacity (SmelteryTank + SmelteryAlloyTank).
    struct Smeltery { fluids: Vec<(Identifier, i32)>, capacity: i32, temperature: i32 }
    impl Smeltery { fn space(&self) -> i32 { self.capacity - self.fluids.iter().map(|(_, amount)| amount).sum::<i32>() } }
    impl AlloyTank for Smeltery {
        fn tanks(&self) -> usize { self.fluids.len() }
        fn fluid_in_tank(&self, tank: usize) -> Option<(Identifier, i32)> { self.fluids.get(tank).cloned() }
        fn temperature(&self) -> i32 { self.temperature }
        fn can_fit(&self, output: &(Identifier, i32), removed: i32) -> bool { output.1 - removed <= self.space() }
        // SmelteryAlloyTank.drain ignores the tank index and drains the fluid, so a drained tank leaving the list is harmless.
        fn drain(&mut self, _: usize, fluid: &Identifier, amount: i32) -> i32 {
            let Some(entry) = self.fluids.iter_mut().find(|(own, _)| own == fluid) else { return 0 };
            let taken = amount.min(entry.1);
            entry.1 -= taken;
            self.fluids.retain(|(_, amount)| *amount > 0);
            taken
        }
        fn fill(&mut self, fluid: &Identifier, amount: i32) -> i32 {
            match self.fluids.iter_mut().find(|(own, _)| own == fluid) { Some(entry) => entry.1 += amount, None => self.fluids.push((fluid.clone(), amount)) }
            amount
        }
    }

    #[test]
    fn netherite_takes_the_amounts_of_each_input_and_leaves_what_is_left() {
        let alloy = Alloy::decode(&id("tconstruct:netherite"), &fields(json!({"type": "tconstruct:alloy", "temperature": 1250,
            "inputs": [{"amount": 40, "tag": "tconstruct:molten_debris"}, {"amount": 40, "tag": "forge:molten_gold"}],
            "result": {"amount": 10, "tag": "forge:molten_netherite"}}))).unwrap();
        let tags = tags();
        let mut tank = Smeltery { fluids: vec![(id("tconstruct:molten_gold"), 100), (id("tconstruct:molten_debris"), 90)], capacity: 1000, temperature: 1250 };
        assert!(alloy.matches(&tank, &|fluid, tag| tags.contains(fluid, tag)) && alloy.can_perform(&tank, &tags));
        // order of the tanks does not matter: the gold tank is first yet matches the second input
        assert!(alloy.perform(&mut tank, &tags));
        assert_eq!(tank.fluids, vec![(id("tconstruct:molten_gold"), 60), (id("tconstruct:molten_debris"), 50), (id("tconstruct:molten_netherite"), 10)]);
        // a second run needs 40 of each again: 20 gold and 10 debris are left, and a third finds too little
        assert!(alloy.perform(&mut tank, &tags));
        assert_eq!(tank.fluids, vec![(id("tconstruct:molten_gold"), 20), (id("tconstruct:molten_debris"), 10), (id("tconstruct:molten_netherite"), 20)]);
        assert!(alloy.matches(&tank, &|fluid, tag| tags.contains(fluid, tag)), "the right fluids are there, in too small amounts");
        assert!(!alloy.can_perform(&tank, &tags) && !alloy.perform(&mut tank, &tags));
        // too cold stops it
        tank.fluids = vec![(id("tconstruct:molten_gold"), 100), (id("tconstruct:molten_debris"), 100)];
        tank.temperature = 1249;
        assert!(!alloy.can_perform(&tank, &tags));
        tank.temperature = 1250;
        assert!(alloy.can_perform(&tank, &tags));
        // an alloy that gains volume needs the room for the net gain only: 30 made from 20 is 10 more
        let gain = Alloy::decode(&id("a:gain"), &fields(json!({"type": "tconstruct:alloy", "temperature": 1,
            "inputs": [{"amount": 10, "fluid": "a:one"}, {"amount": 10, "fluid": "a:two"}], "result": {"amount": 30, "fluid": "a:three"}}))).unwrap();
        let mut small = Smeltery { fluids: vec![(id("a:one"), 10), (id("a:two"), 10)], capacity: 25, temperature: 1 };
        assert!(!gain.can_perform(&small, &FluidTags::default()) && !gain.perform(&mut small, &FluidTags::default()));
        small.capacity = 30;
        assert!(gain.perform(&mut small, &FluidTags::default()));
        assert_eq!(small.fluids, vec![(id("a:three"), 30)]);
    }

    #[test]
    fn a_catalyst_must_be_present_in_full_but_is_not_consumed() {
        let obsidian = Alloy::decode(&id("tconstruct:obsidian"), &fields(json!({"type": "tconstruct:alloy", "temperature": 1000,
            "inputs": [{"amount": 250, "catalyst": true, "fluid": "minecraft:water"}, {"amount": 100, "fluid": "minecraft:lava"}],
            "result": {"amount": 100, "fluid": "tconstruct:molten_obsidian"}}))).unwrap();
        assert!(obsidian.inputs[0].catalyst && !obsidian.inputs[1].catalyst);
        let tags = FluidTags::default();
        let mut tank = Smeltery { fluids: vec![(id("minecraft:lava"), 300), (id("minecraft:water"), 249)], capacity: 2000, temperature: 1000 };
        assert!(obsidian.matches(&tank, &|fluid, tag| tags.contains(fluid, tag)));
        assert!(!obsidian.perform(&mut tank, &tags), "249 of water is not enough catalyst");
        tank.fluids[1].1 = 250;
        assert!(obsidian.perform(&mut tank, &tags));
        assert_eq!(tank.fluids, vec![(id("minecraft:lava"), 200), (id("minecraft:water"), 250), (id("tconstruct:molten_obsidian"), 100)]);
    }

    #[test]
    fn alloys_need_two_inputs_and_the_compact_and_match_forms_decode_alike() {
        let one = json!({"type": "tconstruct:alloy", "temperature": 5, "inputs": [{"amount": 1, "fluid": "a:b"}], "result": {"amount": 1, "fluid": "a:c"}});
        assert!(Alloy::decode(&id("a:x"), &fields(one)).is_err());
        let both = |first: Value| Alloy::decode(&id("a:x"), &fields(json!({"type": "tconstruct:alloy", "temperature": 5,
            "inputs": [first, {"match": {"amount": 1, "fluid": "a:d"}, "catalyst": true}], "result": {"amount": 1, "fluid": "a:c"}}))).unwrap();
        assert_eq!(both(json!({"amount": 2, "fluid": "a:b"})), both(json!({"match": {"amount": 2, "fluid": "a:b"}})));
        assert!(both(json!({"amount": 2, "fluid": "a:b"})).inputs[1].catalyst);
    }

    #[test]
    fn ore_and_damageable_melting_scale_their_output_and_byproducts_by_the_source_rules() {
        let ore = Melting::decode(&id("a:ore"), &fields(json!({"type": "tconstruct:ore_melting", "temperature": 800, "time": 100, "rate": "metal",
            "ingredient": {"tag": "forge:ores/iron"}, "result": {"amount": 90, "fluid": "a:iron"},
            "byproducts": [{"amount": 90, "fluid": "a:debris", "rate": "gem"}, {"amount": 90, "fluid": "a:other"}]}))).unwrap();
        // 12 nuggets per ore in the smeltery: 90 * 12 / 9; the foundry byproduct rate is 3 nuggets (gem 4 shards) and the second byproduct inherits metal
        assert_eq!(ore.output_amount(&OreRate::SMELTERY, None), 120);
        assert_eq!(ore.output_amount(&OreRate::FOUNDRY, None), 90);
        assert_eq!(ore.byproduct_amounts(&OreRate::FOUNDRY_BYPRODUCT, None), vec![90, 30]);
        let tool = Melting::decode(&id("a:pick"), &fields(json!({"type": "tconstruct:damagable_melting", "temperature": 1450, "time": 137,
            "ingredient": {"tag": "a:picks"}, "result": {"amount": 300, "fluid": "a:diamond", "unit_size": 25},
            "byproducts": [{"amount": 90, "fluid": "a:debris", "unit_size": 10}, {"amount": 50, "fluid": "a:slag"}]}))).unwrap();
        assert_eq!(tool.output_amount(&OreRate::SMELTERY, Some((67, 100))), 75);
        assert_eq!(tool.byproduct_amounts(&OreRate::FOUNDRY_BYPRODUCT, Some((67, 100))), vec![20, 16], "90 * 33 / 100 = 29 rounds down to whole 10s; the second byproduct has unit size 1");
        assert_eq!(tool.output_amount(&OreRate::SMELTERY, Some((67, 0))), 300, "an item with no durability gives the whole amount");
    }
}
