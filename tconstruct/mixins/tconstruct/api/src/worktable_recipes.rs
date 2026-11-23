//! The modifier worktable's modifier-removal recipe (`tconstruct:remove_modifier`), from SlimeKnights
//! ModifierRemovalRecipe/AbstractWorktableRecipe (MIT; see LICENSE). The block, its modifier buttons
//! and the other worktable recipe types are separate.
use crate::{
    material_recipes::Ingredient,
    materials::Conditions,
    modifier_entries::List,
    modifier_modules::CausePredicate,
    modifier_recipes::{check_match_facts, Sized},
    tool_stack::{TAG_MODIFIERS, TAG_PERSISTENT, TAG_UPGRADES},
};
use pluto_lib::{Identifier, ItemSlot, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
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

/// A tool's persistent data compound (ModDataNBT), empty when unset.
fn persistent_of(tool: &ItemSlot) -> NbtCompound {
    match tool.custom_data.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() }
}

/// A string list in the persistent data (ModifierSetWorktableRecipe's LIST_GETTER).
fn string_list(persistent: &NbtCompound, key: &str) -> Vec<String> {
    match persistent.get(key) {
        Some(Nbt::List(values)) => values.iter().filter_map(|value| match value { Nbt::String(text) => Some(text.clone()), _ => None }).collect(),
        _ => Vec::new(),
    }
}

fn nbt_string_list(list: Vec<String>) -> Nbt { Nbt::List(list.into_iter().map(Nbt::String).collect()) }

/// One worktable recipe family's load over the shared folder, shaped like the removals'.
macro_rules! worktable_family {
    ($family:ident, $recipe:ident, $kind:literal, $listener:ident, $label:literal) => {
        #[derive(Clone, Debug, Default)]
        pub(crate) struct $family { recipes: Vec<$recipe>, pub problems: Vec<String>, pub loaded: bool }
        impl $family {
            pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
                let mut result = Self { loaded: true, ..Self::default() };
                let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                for (id, value) in sorted {
                    let outcome = (|| -> Result<Option<$recipe>, String> {
                        let fields = value.as_object().ok_or("recipe must be an object")?;
                        if fields.get("type").and_then(Value::as_str) != Some($kind) { return Ok(None); }
                        if let Some(list) = fields.get("conditions") {
                            let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                            for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                        }
                        $recipe::decode(id, value).map(Some)
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
            pub fn find_facts(&self, tool: &(Identifier, u32), inputs: &[Option<(Identifier, u32)>], facts: &[crate::material_recipes::StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&$recipe> {
                self.recipes.iter().find(|recipe| recipe.matches_facts(tool, inputs, facts, in_tag))
            }
        }
        pub(crate) fn $listener<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<$family>>, String> {
            let state = Arc::new(RwLock::new($family::default()));
            let output = state.clone();
            ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
                Identifier::new("tconstruct", $label).unwrap(), FOLDER, move |files| {
                    let context = context();
                    let mut state = output.write().map_err(|_| "worktable recipes poisoned")?;
                    match context {
                        Ok(context) => {
                            *state = $family::load(files, &context);
                            for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                            Ok(())
                        }
                        Err(error) => { *state = $family { problems: vec![error.clone()], ..$family::default() }; Err(error) }
                    }
                })?;
            let stopped = state.clone();
            ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("worktable recipes poisoned") = $family::default());
            Ok(state)
        }
    };
}

/// The modifier set recipe (`tconstruct:modifier_set_worktable`), the original's
/// ModifierSetWorktableRecipe (SlimeKnights, MIT; see LICENSE): adds a modifier to (or removes
/// it from) a named string set in the tool's persistent data — the invisible ink recipes are
/// its shipped instances. Options are the tool's upgrades (or every modifier, when traits are
/// allowed) the predicate allows whose set membership differs from the recipe's direction.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SetRecipe {
    pub id: Identifier,
    pub data_key: Identifier,
    pub add_to_set: bool,
    pub allow_traits: bool,
    pub tools: Ingredient,
    pub inputs: Vec<Sized>,
    pub predicate: CausePredicate,
}

impl SetRecipe {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let data_key = Identifier::parse(fields.get("data_key").and_then(Value::as_str).ok_or("data_key missing")?).map_err(|e| e.to_string())?;
        let add_to_set = fields.get("add_to_set").and_then(Value::as_bool).ok_or("add_to_set must be a boolean")?;
        let allow_traits = fields.get("allow_traits").and_then(Value::as_bool).unwrap_or(false);
        let tools = Ingredient::decode(fields.get("tools").ok_or("tools missing")?)?;
        let inputs = fields.get("inputs").and_then(Value::as_array).filter(|inputs| !inputs.is_empty()).ok_or("inputs must be a nonempty array")?
            .iter().map(Sized::decode).collect::<Result<Vec<_>, _>>()?;
        let predicate = CausePredicate::decode(fields.get("modifier_predicate"))?.ok_or("unsupported modifier predicate")?;
        Ok(Self { id: id.clone(), data_key, add_to_set, allow_traits, tools, inputs, predicate })
    }

    pub fn matches_facts(&self, tool: &(Identifier, u32), inputs: &[Option<(Identifier, u32)>], facts: &[crate::material_recipes::StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.tools.matches(&tool.0, in_tag) && check_match_facts(&self.inputs, inputs, facts, in_tag).is_some()
    }

    /// getModifierOptions: the predicate's modifiers whose set membership differs from the
    /// recipe's direction.
    pub fn options(&self, tool: &ItemSlot, modifier_in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Vec<Identifier> {
        let list = List::from_nbt(tool.custom_data.get(if self.allow_traits { TAG_MODIFIERS } else { TAG_UPGRADES }));
        let set = string_list(&persistent_of(tool), &self.data_key.to_string());
        list.entries().iter()
            .filter(|entry| self.predicate.matches(Some(&entry.id), modifier_in_tag) && set.contains(&entry.id.to_string()) != self.add_to_set)
            .map(|entry| entry.id.clone()).collect()
    }

    /// getResult: the tool copy with the set updated; no rebuild, as in the original.
    pub fn result(&self, tool: &ItemSlot, modifier: &Identifier) -> ItemSlot {
        let mut result = tool.clone();
        let mut persistent = persistent_of(tool);
        let key = self.data_key.to_string();
        let value = modifier.to_string();
        let mut list = string_list(&persistent, &key);
        let found = list.contains(&value);
        if !self.add_to_set && found { list.retain(|entry| entry != &value); }
        if !self.add_to_set && list.is_empty() {
            persistent.remove(&key);
        } else if !found && self.add_to_set {
            list.push(value);
            persistent.insert(key, nbt_string_list(list));
        } else if !self.add_to_set && found {
            persistent.insert(key, nbt_string_list(list));
        }
        result.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        result
    }
}

/// The sorting recipe (`tconstruct:modifier_sorting`), the original's ModifierSortingRecipe
/// (SlimeKnights, MIT; see LICENSE): swaps the selected upgrade with its neighbor — backwards
/// when the first input slot is empty, forwards otherwise — and consumes nothing.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Sorting {
    pub id: Identifier,
    pub inputs: Vec<Sized>,
}

impl Sorting {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        // The inputs list is compact: a single ingredient object or an array of them.
        let inputs = match fields.get("inputs") {
            Some(Value::Array(list)) if !list.is_empty() => list.iter().map(Sized::decode).collect::<Result<Vec<_>, _>>()?,
            Some(single @ Value::Object(_)) => vec![Sized::decode(single)?],
            _ => return Err("inputs must be an ingredient or a nonempty array".into()),
        };
        Ok(Self { id: id.clone(), inputs })
    }

    pub fn matches_facts(&self, tool: &(Identifier, u32), inputs: &[Option<(Identifier, u32)>], facts: &[crate::material_recipes::StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        let modifiable = Identifier::new("tconstruct", "modifiable").expect("valid tag");
        in_tag(&tool.0, &modifiable) && check_match_facts(&self.inputs, inputs, facts, in_tag).is_some()
    }

    /// getModifierOptions: the tool's upgrades, when there are at least two to reorder.
    pub fn options(&self, tool: &ItemSlot) -> Vec<Identifier> {
        let list = List::from_nbt(tool.custom_data.get(TAG_UPGRADES));
        if list.entries().len() < 2 { return Vec::new(); }
        list.entries().iter().map(|entry| entry.id.clone()).collect()
    }

    /// getResult: the tool copy with the selected entry swapped toward its neighbor; no
    /// rebuild or validation, as the modifiers themselves do not change.
    pub fn result(&self, tool: &ItemSlot, modifier: &Identifier, first_input_empty: bool) -> Option<ItemSlot> {
        let upgrades = List::from_nbt(tool.custom_data.get(TAG_UPGRADES));
        let entries = upgrades.entries();
        let to_move = entries.iter().position(|entry| &entry.id == modifier)?;
        let target = if first_input_empty {
            if to_move == 0 { entries.len() - 1 } else { to_move - 1 }
        } else {
            if to_move == entries.len() - 1 { 0 } else { to_move + 1 }
        };
        let mut result = tool.clone();
        result.custom_data.insert(TAG_UPGRADES.into(), upgrades.swapped(to_move, target).to_nbt());
        Some(result)
    }
}

/// The toggle interaction recipe (`tconstruct:toggle_interaction`), the original's
/// ToggleInteractionWorktableRecipe (SlimeKnights, MIT; see LICENSE): moves a dual-interaction
/// modifier between the tool's attack and interact modifier sets in the persistent data. A
/// modifier in either set is removed from both; otherwise it joins the attack set when the
/// tool's interaction gate lets it interact on the right click, and the interact set when not.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Toggle {
    pub id: Identifier,
    pub tools: Ingredient,
    pub inputs: Vec<Sized>,
}

/// InteractionSource.LEFT_CLICK's persistent key.
const ATTACK_KEY: &str = "tconstruct:attack_modifiers";
/// InteractionSource.RIGHT_CLICK's persistent key.
const INTERACT_KEY: &str = "tconstruct:interact_modifiers";

impl Toggle {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let tools = Ingredient::decode(fields.get("tools").ok_or("tools missing")?)?;
        let inputs = fields.get("inputs").and_then(Value::as_array).filter(|inputs| !inputs.is_empty()).ok_or("inputs must be a nonempty array")?
            .iter().map(Sized::decode).collect::<Result<Vec<_>, _>>()?;
        Ok(Self { id: id.clone(), tools, inputs })
    }

    pub fn matches_facts(&self, tool: &(Identifier, u32), inputs: &[Option<(Identifier, u32)>], facts: &[crate::material_recipes::StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.tools.matches(&tool.0, in_tag) && check_match_facts(&self.inputs, inputs, facts, in_tag).is_some()
    }

    /// getModifierOptions: the tool's modifiers in the dual interaction tag.
    pub fn options(&self, tool: &ItemSlot, modifier_in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Vec<Identifier> {
        let dual = Identifier::new("tconstruct", "dual_interaction").expect("valid tag");
        List::from_nbt(tool.custom_data.get(TAG_MODIFIERS)).entries().iter()
            .filter(|entry| modifier_in_tag(&entry.id, &dual)).map(|entry| entry.id.clone()).collect()
    }

    /// getResult: the tool copy with the modifier toggled between the two sets. `gate` is the
    /// tool definition's interaction gate, whose canInteract answers for the right click.
    pub fn result(&self, tool: &ItemSlot, modifier: &Identifier, gate: Option<&crate::tool_definitions::InteractionGate>, modifier_in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> ItemSlot {
        let mut result = tool.clone();
        let mut persistent = persistent_of(tool);
        let value = modifier.to_string();
        let mut left = string_list(&persistent, ATTACK_KEY);
        let mut right = string_list(&persistent, INTERACT_KEY);
        let in_left = left.contains(&value);
        let in_right = right.contains(&value);
        if in_left || in_right {
            left.retain(|entry| entry != &value);
            right.retain(|entry| entry != &value);
            if left.is_empty() { persistent.remove(ATTACK_KEY); } else { persistent.insert(ATTACK_KEY.into(), nbt_string_list(left)); }
            if right.is_empty() { persistent.remove(INTERACT_KEY); } else { persistent.insert(INTERACT_KEY.into(), nbt_string_list(right)); }
        } else {
            // ToolHooks.INTERACTION.canInteract(tool, modifier, RIGHT_CLICK).
            let can_right = match gate {
                Some(crate::tool_definitions::InteractionGate::DualOption) => !in_left,
                Some(crate::tool_definitions::InteractionGate::ToggleableSet(predicate)) => {
                    if predicate.matches(Some(modifier), modifier_in_tag) { !in_left } else { in_right }
                }
                None => false,
            };
            if can_right {
                left.push(value);
                persistent.insert(ATTACK_KEY.into(), nbt_string_list(left));
            } else {
                right.push(value);
                persistent.insert(INTERACT_KEY.into(), nbt_string_list(right));
            }
        }
        result.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        result
    }
}

worktable_family!(Sets, SetRecipe, "tconstruct:modifier_set_worktable", register_sets, "worktable_sets");
worktable_family!(Sortings, Sorting, "tconstruct:modifier_sorting", register_sortings, "worktable_sortings");
worktable_family!(Toggles, Toggle, "tconstruct:toggle_interaction", register_toggles, "worktable_toggles");

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
