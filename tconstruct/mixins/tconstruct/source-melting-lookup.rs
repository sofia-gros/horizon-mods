// Lookup uses one world's resolved tags and ordered datapack recipes. A fluid
// tag is accepted only when it identifies one still fluid; output tags are
// accepted only when their choice is unambiguous. This avoids guessing a
// missing mod's fluid or a namespace preference.
use std::collections::{BTreeMap, HashMap, HashSet};
use serde_json::Value;
use super::tconstruct_source_melting_recipes::{AlloyRecipe, CastingRecipe, FluidFuel, FluidSpec, FoundryRecipe, MeltingRecipe, Output};

pub(super) struct Catalog<'a> {
    pub recipes: &'a BTreeMap<String, Vec<u8>>,
    pub item_tags: &'a HashMap<String, HashSet<String>>,
    pub fluid_tags: &'a HashMap<String, HashSet<String>>,
    pub known_items: &'a HashSet<String>,
    pub known_fluids: &'a HashSet<String>,
}
impl Catalog<'_> {
    fn matches_fluid(&self, spec: &FluidSpec, fluid: &str) -> bool {
        if !self.known_fluids.contains(fluid) { return false; }
        if spec.tagged { self.fluid_tags.get(&spec.id).is_some_and(|members| members.contains(fluid)) }
        else { spec.id == fluid }
    }
    fn resolved_fluid(&self, spec: &FluidSpec) -> Option<String> {
        if !spec.tagged { return self.known_fluids.contains(&spec.id).then(|| spec.id.clone()); }
        let members = self.fluid_tags.get(&spec.id)?;
        let mut still = members.iter().filter(|id| self.known_fluids.contains(*id)
            && !id.split_once(':').is_some_and(|(_, path)| path.starts_with("flowing_")));
        let id = still.next()?.clone();
        still.next().is_none().then_some(id)
    }
    fn output(&self, result: &Output) -> Option<String> {
        match result {
            Output::Item(id) => self.known_items.contains(id).then(|| id.clone()),
            Output::Tag(tag) => {
                let mut values = self.item_tags.get(tag)?.iter().filter(|id| self.known_items.contains(*id));
                let value = values.next()?.clone();
                values.next().is_none().then_some(value)
            }
        }
    }
    pub(super) fn fluid_fuel(&self, fluid: &str, available: u32, conditions: &impl Fn(&[Value]) -> bool) -> Option<FluidFuel> {
        for bytes in self.recipes.values() {
            let Some(fuel) = FluidFuel::parse(bytes, conditions) else { continue; };
            if available >= fuel.fluid.amount && self.matches_fluid(&fuel.fluid, fluid) { return Some(fuel); }
        }
        None
    }
    /// Match source ingredients in order, using each stored fluid at most once.
    /// Conditions and tags are evaluated against this world's snapshots.
    pub(super) fn alloy(&self, fluids: &[(String, u32)], capacity: u32, temperature: u32, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, Vec<(String, u32)>, String, u32)> {
        let total = fluids.iter().try_fold(0u32, |sum, (_, amount)| sum.checked_add(*amount))?;
        for (id, bytes) in self.recipes {
            let Some(recipe) = AlloyRecipe::parse(bytes, conditions) else { continue; };
            if recipe.temperature > temperature { continue; }
            let Some(output) = self.resolved_fluid(&recipe.output) else { continue; };
            let mut used = HashSet::new();
            let mut inputs = Vec::with_capacity(recipe.inputs.len());
            for ingredient in &recipe.inputs {
                let matched = fluids.iter().enumerate().find(|(index, (fluid, amount))|
                    !used.contains(index) && *amount >= ingredient.amount && self.matches_fluid(ingredient, fluid));
                let Some((index, (fluid, _))) = matched else { break; };
                used.insert(index);
                inputs.push((fluid.clone(), ingredient.amount));
            }
            if inputs.len() == recipe.inputs.len() {
                let removed = inputs.iter().try_fold(0u32, |sum, (_, amount)| sum.checked_add(*amount))?;
                if recipe.output.amount.saturating_sub(removed) > capacity.saturating_sub(total) { continue; }
                return Some((id.clone(), inputs, output, recipe.output.amount));
            }
        }
        None
    }
    pub(super) fn melting(&self, item: &str, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, MeltingRecipe, String)> {
        if !self.known_items.contains(item) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found: Option<(String, MeltingRecipe, String)> = None;
        for (id, bytes) in self.recipes {
            let Some(recipe) = MeltingRecipe::parse(bytes, conditions) else { continue; };
            if !recipe.input.matches(item, &in_tag) { continue; }
            let Some(fluid) = self.resolved_fluid(&recipe.output) else { continue; };
            if let Some((_, previous, previous_fluid)) = &found {
                if previous.temperature != recipe.temperature || previous.time != recipe.time
                    || previous.output.amount != recipe.output.amount || previous_fluid != &fluid { return None; }
            } else { found = Some((id.clone(), recipe, fluid)); }
        }
        found
    }
    pub(super) fn foundry_melting(&self, item: &str, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
        if !self.known_items.contains(item) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found: Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> = None;
        for (id, bytes) in self.recipes {
            let Some(recipe) = FoundryRecipe::parse(bytes, conditions) else { continue; };
            if !recipe.melting.input.matches(item, &in_tag) { continue; }
            let Some(primary) = self.resolved_fluid(&recipe.melting.output) else { continue; };
            let Some(byproducts) = recipe.byproducts.iter().map(|fluid|
                self.resolved_fluid(fluid).map(|id| (id, fluid.amount))).collect::<Option<Vec<_>>>() else { continue; };
            if let Some((_, previous, old_primary, old_byproducts)) = &found {
                if previous.melting.temperature != recipe.melting.temperature || previous.melting.time != recipe.melting.time
                    || previous.melting.output.amount != recipe.melting.output.amount || old_primary != &primary
                    || old_byproducts != &byproducts { return None; }
            } else { found = Some((id.clone(), recipe, primary, byproducts)); }
        }
        found
    }
    pub(super) fn casting(&self, basin: bool, input: Option<&str>, fluid: &str, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, CastingRecipe, String)> {
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found: Option<(String, CastingRecipe, String)> = None;
        for (id, bytes) in self.recipes {
            let Some(recipe) = CastingRecipe::parse(bytes, conditions) else { continue; };
            if recipe.basin != basin || !self.matches_fluid(&recipe.fluid, fluid) { continue; }
            let matched = match (&recipe.cast, input) {
                (None, None) => true,
                (Some(cast), Some(item)) if self.known_items.contains(item) => cast.matches(item, &in_tag),
                _ => false,
            };
            if !matched { continue; }
            let Some(output) = self.output(&recipe.output) else { continue; };
            if let Some((_, previous, previous_output)) = &found {
                if previous.fluid.amount != recipe.fluid.amount || previous.cooling_time != recipe.cooling_time
                    || previous.consumes_cast != recipe.consumes_cast || previous.switch_slots != recipe.switch_slots
                    || previous_output != &output { return None; }
            } else { found = Some((id.clone(), recipe, output)); }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foundry_resolves_primary_and_byproduct_tags() {
        let recipe = br#"{"type":"tconstruct:ore_melting","ingredient":{"tag":"forge:ores/copper"},"result":{"amount":180,"tag":"forge:molten_copper"},"temperature":500,"time":125,"rate":"metal","byproducts":[{"amount":90,"tag":"forge:molten_gold"}]}"#;
        let recipes = [("ore".into(), recipe.to_vec())].into_iter().collect();
        let item_tags = [("forge:ores/copper".into(), ["minecraft:copper_ore".into()].into_iter().collect())].into_iter().collect();
        let fluid_tags = [("forge:molten_copper".into(), ["tconstruct:molten_copper".into()].into_iter().collect()),
            ("forge:molten_gold".into(), ["tconstruct:molten_gold".into()].into_iter().collect())].into_iter().collect();
        let known_items = ["minecraft:copper_ore".into()].into_iter().collect();
        let known_fluids = ["tconstruct:molten_copper".into(), "tconstruct:molten_gold".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
        let (_, recipe, primary, byproducts) = catalog.foundry_melting("minecraft:copper_ore", &|_| true).unwrap();
        assert_eq!((primary.as_str(), recipe.melting.output.amount), ("tconstruct:molten_copper", 180));
        assert_eq!(byproducts, vec![("tconstruct:molten_gold".into(), 30)]);
    }
    #[test]
    fn alloy_resolves_source_tags_and_temperature() {
        let recipes = [
            ("a-overflow".into(), br#"{"type":"tconstruct:alloy","inputs":[{"amount":270,"tag":"forge:copper"},{"amount":90,"tag":"forge:tin"}],"result":{"amount":361,"tag":"forge:bronze"},"temperature":700}"#.to_vec()),
            ("bronze".into(), br#"{"type":"tconstruct:alloy","inputs":[{"amount":270,"tag":"forge:copper"},{"amount":90,"tag":"forge:tin"}],"result":{"amount":360,"tag":"forge:bronze"},"temperature":700}"#.to_vec())
        ].into_iter().collect();
        let item_tags = HashMap::new(); let items = HashSet::new();
        let fluids = ["tconstruct:copper".into(), "tconstruct:tin".into(), "tconstruct:bronze".into()].into_iter().collect();
        let tags = [("forge:copper".into(), ["tconstruct:copper".into()].into_iter().collect()), ("forge:tin".into(), ["tconstruct:tin".into()].into_iter().collect()), ("forge:bronze".into(), ["tconstruct:bronze".into()].into_iter().collect())].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        let stored = [("tconstruct:copper".into(), 270), ("tconstruct:tin".into(), 90)];
        assert!(catalog.alloy(&stored, 360, 699, &|_| true).is_none());
        let (id, inputs, output, amount) = catalog.alloy(&stored, 360, 700, &|_| true).unwrap();
        assert_eq!(id, "bronze");
        assert_eq!((inputs.len(), output.as_str(), amount), (2, "tconstruct:bronze", 360));
        assert!(catalog.alloy(&stored[..1], 360, 700, &|_| true).is_none());
    }
    #[test]
    fn fluid_fuel_requires_known_fluid_and_enough_amount() {
        let recipes = [("lava".into(), br#"{"type":"tconstruct:melting_fuel","duration":100,"fluid":{"amount":50,"fluid":"minecraft:lava"},"rate":10,"temperature":1000}"#.to_vec())].into_iter().collect();
        let item_tags = HashMap::new(); let items = HashSet::new(); let tags = HashMap::new();
        let fluids = ["minecraft:lava".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        assert!(catalog.fluid_fuel("minecraft:lava", 49, &|_| true).is_none());
        assert_eq!(catalog.fluid_fuel("minecraft:lava", 50, &|_| true).unwrap().duration, 100);
        assert!(catalog.fluid_fuel("minecraft:water", 50, &|_| true).is_none());
    }
    #[test]
    fn copper_round_trip_uses_resolved_tags_and_rejects_ambiguous_outputs() {
        let recipes = [
            ("melting".into(), br#"{"type":"tconstruct:melting","ingredient":{"tag":"forge:ingots/copper"},"result":{"amount":90,"tag":"forge:molten_copper"},"temperature":500,"time":50}"#.to_vec()),
            ("casting".into(), br#"{"type":"tconstruct:casting_table","cast":{"tag":"tconstruct:casts/multi_use/ingot"},"cooling_time":50,"fluid":{"amount":90,"tag":"forge:molten_copper"},"result":{"tag":"forge:ingots/copper"}}"#.to_vec()),
        ].into_iter().collect();
        let items: HashSet<String> = ["minecraft:copper_ingot".into(), "tconstruct:ingot_cast".into()].into_iter().collect();
        let fluids: HashSet<String> = ["tconstruct:molten_copper".into(), "tconstruct:flowing_molten_copper".into()].into_iter().collect();
        let mut item_tags: HashMap<String, HashSet<String>> = [
            ("forge:ingots/copper".into(), ["minecraft:copper_ingot".into()].into_iter().collect()),
            ("tconstruct:casts/multi_use/ingot".into(), ["tconstruct:ingot_cast".into()].into_iter().collect()),
        ].into_iter().collect();
        let fluid_tags = [("forge:molten_copper".into(), fluids.clone())].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
        assert_eq!(catalog.melting("minecraft:copper_ingot", &|_| true).unwrap().2, "tconstruct:molten_copper");
        assert_eq!(catalog.casting(false, Some("tconstruct:ingot_cast"), "tconstruct:molten_copper", &|_| true).unwrap().2, "minecraft:copper_ingot");
        assert!(catalog.casting(false, None, "tconstruct:molten_copper", &|_| true).is_none());
        item_tags.get_mut("forge:ingots/copper").unwrap().insert("other:copper_ingot".into());
        let mut ambiguous_items = items.clone(); ambiguous_items.insert("other:copper_ingot".into());
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &ambiguous_items, known_fluids: &fluids };
        assert!(catalog.casting(false, Some("tconstruct:ingot_cast"), "tconstruct:molten_copper", &|_| true).is_none());
    }
}

/// Resolve the source pack's tank-fluid tags without registering flowing
/// world fluids. These IDs describe contents held by the melter; world-fluid
/// placement remains separate. Only source TConstruct IDs and the native
/// water/lava families are admitted; unrelated mods are not assumed present.
pub(super) fn tank_fluid_tags(
    raw: &BTreeMap<String, Vec<Vec<u8>>>,
    source_fluids: &HashSet<String>,
) -> Result<(HashSet<String>, HashMap<String, HashSet<String>>), String> {
    let mut fluids: HashSet<String> = ["minecraft:empty", "minecraft:water", "minecraft:flowing_water", "minecraft:lava", "minecraft:flowing_lava"]
        .into_iter().map(str::to_owned).collect();
    // Only IDs from the retained upstream catalog are virtual tank contents;
    // a datapack tag cannot invent a registered fluid by itself.
    fluids.extend(source_fluids.iter().filter(|id| id.starts_with("tconstruct:")).cloned());
    let tags = super::tconstruct_source_tags::resolve_registry(raw, &fluids)?;
    Ok((fluids, tags))
}

#[cfg(test)]
mod fluid_tag_tests {
    use super::*;
    #[test]
    fn resolves_source_copper_without_assuming_other_mod_fluids() {
        let raw = [
            ("forge:molten_copper".into(), vec![br##"{"values":["#tconstruct:molten_copper"]}"##.to_vec()]),
            ("tconstruct:molten_copper".into(), vec![br#"{"values":["tconstruct:molten_copper","tconstruct:flowing_molten_copper"]}"#.to_vec()]),
            ("forge:external".into(), vec![br#"{"values":["other:fluid"]}"#.to_vec()]),
        ].into_iter().collect();
        let source_fluids = ["tconstruct:molten_copper".into(), "tconstruct:flowing_molten_copper".into()].into_iter().collect();
        let (fluids, tags) = tank_fluid_tags(&raw, &source_fluids).unwrap();
        assert!(fluids.contains("tconstruct:molten_copper"));
        assert!(!fluids.contains("other:fluid"));
        assert!(tags["forge:molten_copper"].contains("tconstruct:molten_copper"));
        assert!(!tags.contains_key("forge:external"));
    }
}
