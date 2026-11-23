//! The modifier worktable's modifier-removal recipe (`tconstruct:remove_modifier`), from SlimeKnights
//! ModifierRemovalRecipe/AbstractWorktableRecipe (MIT; see LICENSE). The block, its modifier buttons
//! and the other worktable recipe types are separate.
use crate::{
    material_recipes::Ingredient,
    materials::Conditions,
    modifier_entries::List,
    modifier_modules::CausePredicate,
    modifier_recipes::{check_match_facts, Sized},
    tool_stack::TAG_UPGRADES,
};
use pluto_lib::{Identifier, ItemSlot, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = crate::modifier_recipes::FOLDER;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Removal {
    pub id: Identifier,
    pub name: String,
    pub tools: Sized,
    pub inputs: Vec<Sized>,
    /// Items handed to the player when the recipe is taken.
    pub leftovers: Vec<Identifier>,
    pub predicate: CausePredicate,
}

impl Removal {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let inputs = fields.get("inputs").and_then(Value::as_array).filter(|inputs| !inputs.is_empty()).ok_or("inputs must be a nonempty array")?
            .iter().map(Sized::decode).collect::<Result<Vec<_>, _>>()?;
        // The default tool requirement is anything modifiable.
        let tools = match fields.get("tools") {
            None => Sized { ingredient: Ingredient::Tag(Identifier::new("tconstruct", "modifiable").expect("valid tag")), amount: 1 },
            Some(value) => Sized::decode(value)?,
        };
        let leftovers = match fields.get("leftovers") {
            None => Vec::new(),
            Some(Value::Array(list)) => list.iter().map(|item| match item {
                Value::String(item) => Identifier::parse(item).map_err(|e| e.to_string()),
                _ => Err("a leftover must be an item ID string".to_owned()),
            }).collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err("leftovers must be an array".into()),
        };
        let predicate = CausePredicate::decode(fields.get("modifier_predicate"))?.ok_or("unsupported modifier predicate")?;
        let name = fields.get("name").and_then(Value::as_str).unwrap_or("modifiers").to_owned();
        Ok(Self { id: id.clone(), name, tools, inputs, leftovers, predicate })
    }

    /// matches: the tool qualifies (and is stacked high enough) and the inputs are exactly this recipe's.
    pub fn matches(&self, tool: &(Identifier, u32), inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.matches_facts(tool, inputs, &[], in_tag)
    }

    /// matches with the inputs' stack facts (what fluid each input holds, for fluid container ingredients).
    pub fn matches_facts(&self, tool: &(Identifier, u32), inputs: &[Option<(Identifier, u32)>], facts: &[crate::material_recipes::StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.tools.test(tool, in_tag) && check_match_facts(&self.inputs, inputs, facts, in_tag).is_some()
    }

    /// getModifierOptions: the tool's upgrades this recipe may remove. `modifier_in_tag` answers modifier tags.
    pub fn options(&self, tool: &ItemSlot, modifier_in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Vec<Identifier> {
        List::from_nbt(tool.custom_data.get(TAG_UPGRADES)).entries().iter()
            .filter(|entry| self.predicate.matches(Some(&entry.id), modifier_in_tag)).map(|entry| entry.id.clone()).collect()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Removals { recipes: Vec<Removal>, pub problems: Vec<String>, pub loaded: bool }

impl Removals {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Removal>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:remove_modifier") { return Ok(None); }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                }
                Removal::decode(id, value).map(Some)
            })();
            match outcome {
                Ok(Some(recipe)) => result.recipes.push(recipe),
                Ok(None) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }
    pub fn len(&self) -> usize { self.recipes.len() }
    pub fn find(&self, tool: &(Identifier, u32), inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Removal> {
        self.find_facts(tool, inputs, &[], in_tag)
    }
    /// `find` with the inputs' stack facts (what fluid each input holds).
    pub fn find_facts(&self, tool: &(Identifier, u32), inputs: &[Option<(Identifier, u32)>], facts: &[crate::material_recipes::StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Removal> {
        self.recipes.iter().find(|recipe| recipe.matches_facts(tool, inputs, facts, in_tag))
    }
}

/// Reload listener, shaped like the other recipe listeners.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Removals>>, String> {
    let state = Arc::new(RwLock::new(Removals::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "worktable_removals").unwrap(), FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "worktable removals poisoned")?;
            match context {
                Ok(context) => {
                    *state = Removals::load(files, &context);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { *state = Removals { problems: vec![error.clone()], ..Removals::default() }; Err(error) }
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("worktable removals poisoned") = Removals::default());
    Ok(state)
}

/// The extraction recipe (`tconstruct:extract_modifier`), the original's ExtractModifierRecipe
/// (SlimeKnights, MIT; see LICENSE): a removal whose taken modifier level is handed back on a
/// modifier crystal. Its options are the predicate's modifiers with an effective level above 0
/// (a partial first level of an incremental modifier does not extract, which keeps the recipe
/// from minting crystals), and its predicates may name slot types, decided through the modifier
/// recipe lookup as ModifierRecipeLookup decides them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Extraction {
    pub id: Identifier,
    pub name: String,
    pub tools: Sized,
    pub inputs: Vec<Sized>,
    pub leftovers: Vec<Identifier>,
    pub predicate: CausePredicate,
}

impl Extraction {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let inputs = fields.get("inputs").and_then(Value::as_array).filter(|inputs| !inputs.is_empty()).ok_or("inputs must be a nonempty array")?
            .iter().map(Sized::decode).collect::<Result<Vec<_>, _>>()?;
        let tools = match fields.get("tools") {
            None => Sized { ingredient: Ingredient::Tag(Identifier::new("tconstruct", "modifiable").expect("valid tag")), amount: 1 },
            Some(value) => Sized::decode(value)?,
        };
        let leftovers = match fields.get("leftovers") {
            None => Vec::new(),
            Some(Value::Array(list)) => list.iter().map(|item| match item {
                Value::String(item) => Identifier::parse(item).map_err(|e| e.to_string()),
                _ => Err("a leftover must be an item ID string".to_owned()),
            }).collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err("leftovers must be an array".into()),
        };
        let predicate = CausePredicate::decode(fields.get("modifier_predicate"))?.ok_or("unsupported modifier predicate")?;
        let name = fields.get("name").and_then(Value::as_str).unwrap_or("modifiers").to_owned();
        Ok(Self { id: id.clone(), name, tools, inputs, leftovers, predicate })
    }

    /// matches: the tool qualifies (and is stacked high enough) and the inputs are exactly this recipe's.
    pub fn matches_facts(&self, tool: &(Identifier, u32), inputs: &[Option<(Identifier, u32)>], facts: &[crate::material_recipes::StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.tools.test(tool, in_tag) && check_match_facts(&self.inputs, inputs, facts, in_tag).is_some()
    }

    /// getModifierOptions: the tool's upgrades the predicate allows, at an effective level above
    /// 0. `slot_member` answers the predicate's slot types through the recipe lookup.
    pub fn options(&self, tool: &ItemSlot, modifier_in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, slot_member: &dyn Fn(&str, &Identifier) -> bool) -> Vec<Identifier> {
        List::from_nbt(tool.custom_data.get(TAG_UPGRADES)).entries().iter()
            .filter(|entry| entry.int_effective_level() > 0 && self.predicate.matches_slotted(Some(&entry.id), modifier_in_tag, slot_member))
            .map(|entry| entry.id.clone()).collect()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Extractions { recipes: Vec<Extraction>, pub problems: Vec<String>, pub loaded: bool }

impl Extractions {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Extraction>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:extract_modifier") { return Ok(None); }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                }
                Extraction::decode(id, value).map(Some)
            })();
            match outcome {
                Ok(Some(recipe)) => result.recipes.push(recipe),
                Ok(None) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }
    pub fn len(&self) -> usize { self.recipes.len() }
    /// The first recipe whose tool requirement and inputs match, in id order.
    pub fn find_facts(&self, tool: &(Identifier, u32), inputs: &[Option<(Identifier, u32)>], facts: &[crate::material_recipes::StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Extraction> {
        self.recipes.iter().find(|recipe| recipe.matches_facts(tool, inputs, facts, in_tag))
    }
}

/// Reload listener for the extraction recipes, shaped like the removals'.
pub(crate) fn register_extractions<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Extractions>>, String> {
    let state = Arc::new(RwLock::new(Extractions::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "worktable_extractions").unwrap(), FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "worktable extractions poisoned")?;
            match context {
                Ok(context) => {
                    *state = Extractions::load(files, &context);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { *state = Extractions { problems: vec![error.clone()], ..Extractions::default() }; Err(error) }
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("worktable extractions poisoned") = Extractions::default());
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct Flags;
    impl Conditions for Flags {
        type Parsed = bool;
        fn decode(&self, value: &Value) -> Result<bool, String> { value.as_bool().ok_or("condition must be boolean".into()) }
        fn test(&self, value: &bool) -> Result<bool, String> { Ok(*value) }
    }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn sponge() -> Value {
        json!({"type":"tconstruct:remove_modifier","inputs":[{"item":"minecraft:wet_sponge"}],"leftovers":["minecraft:sponge"],
            "modifier_predicate":{"type":"mantle:inverted","inverted_type":"tconstruct:tag","tag":"tconstruct:remove_blacklist"},
            "name":"modifiers","tools":{"tag":"tconstruct:modifiable"}})
    }
    #[test]
    fn the_sponge_recipe_matches_a_modifiable_tool_and_offers_non_blacklisted_modifiers() {
        let files = vec![(id("test:sponge"), sponge()), (id("test:fluid"), json!({"type":"tconstruct:remove_modifier","inputs":[{"ingredient":[{"type":"mantle:fluid_container","amount":1000,"display":{"item":"a:venom_bucket"},"tag":"a:venom"}]}]})),
            (id("test:off"), { let mut v = sponge(); v["conditions"] = json!([false]); v })];
        let recipes = Removals::load(&files, &Flags);
        assert_eq!((recipes.len(), recipes.problems.len()), (2, 0), "{:?}", recipes.problems);
        let in_tag = |item: &Identifier, tag: &Identifier| tag.path() == "modifiable" && item.path() == "pickaxe";
        let pickaxe = (id("tconstruct:pickaxe"), 1);
        let wet = [Some((id("minecraft:wet_sponge"), 1)), None];
        let recipe = recipes.find(&pickaxe, &wet, &in_tag).expect("the sponge recipe matches");
        assert_eq!(recipe.leftovers, vec![id("minecraft:sponge")]);
        assert!(recipes.find(&(id("minecraft:stick"), 1), &wet, &in_tag).is_none(), "the tool must be modifiable");
        assert!(recipes.find(&pickaxe, &[Some((id("minecraft:dirt"), 1))], &in_tag).is_none());

        let mut tool = ItemSlot::new(pickaxe.0.clone(), 1);
        let list = List::default().with_modifier(id("test:sharp"), 1).unwrap().with_modifier(id("test:locked"), 1).unwrap();
        tool.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
        let blacklisted = |modifier: &Identifier, tag: &Identifier| tag.path() == "remove_blacklist" && modifier.path() == "locked";
        assert_eq!(recipe.options(&tool, &blacklisted), vec![id("test:sharp")]);

        // The fluid removal matches only a container actually holding enough of the fluid.
        let tags = crate::fluid_ingredient::FluidTags::load(&[(id("a:venom"), json!({"values": ["a:venom"]}))]);
        let venom = id("a:venom");
        let bucket = [Some((id("a:venom_bucket"), 1)), None];
        let facts = |amount: i32| [crate::material_recipes::StackFacts { fluid: Some((&venom, amount)), fluid_tags: Some(&tags), ..Default::default() }, Default::default()];
        assert_eq!(recipes.find_facts(&pickaxe, &bucket, &facts(1000), &in_tag).map(|recipe| recipe.id.clone()), Some(id("test:fluid")));
        assert!(recipes.find_facts(&pickaxe, &bucket, &facts(999), &in_tag).is_none(), "not enough venom");
        assert!(recipes.find(&pickaxe, &bucket, &in_tag).is_none(), "a container whose contents are unknown matches nothing");
    }
}
