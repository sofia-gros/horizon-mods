//! The `tconstruct:modifier` station recipe: add a modifier level to a tool for items and slots
//! (SlimeKnights ModifierRecipe/AbstractModifierRecipe/IncrementalModifierRecipe, MIT; see LICENSE).
//! Modifier crystals, swappable and the other modifier recipe types are not loaded here.
use crate::{
    material_recipes::Ingredient,
    modifier_entries::List,
    tool_slot,
    tool_stack::{rebuild_stats, try_validate, Environment, TAG_MODIFIERS, TAG_PERSISTENT, TAG_UPGRADES},
};
use crate::materials::Conditions;
use pluto_lib::{Identifier, ItemSlot, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::{Map, Value};
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = "recipes/tools/modifiers";

/// SizedIngredient: an ingredient and how many of it one application takes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Sized { pub ingredient: Ingredient, pub amount: u32 }

impl Sized {
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("input must be an object")?;
        let amount = match fields.get("amount_needed") {
            None => 1,
            Some(Value::Number(n)) => { let n = crate::materials::number_int(&n.to_string()); if n >= 1 { n as u32 } else { return Err("amount_needed must be at least 1".into()); } }
            Some(_) => return Err("amount_needed must be a number".into()),
        };
        // The wrapped form names the ingredient under "ingredient"; otherwise the object is the ingredient.
        let ingredient = Ingredient::decode(fields.get("ingredient").unwrap_or(value))?;
        Ok(Self { ingredient, amount })
    }
    /// SizedIngredient.test: the right item and at least the needed count.
    pub(crate) fn test(&self, stack: &(Identifier, u32), in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        stack.1 >= self.amount && self.ingredient.matches(&stack.0, in_tag)
    }
    /// The same test with the stack's facts (an instrument it carries, a fluid it holds).
    pub(crate) fn test_facts(&self, stack: &(Identifier, u32), in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, facts: &crate::material_recipes::StackFacts) -> bool {
        stack.1 >= self.amount && self.ingredient.matches_facts(&stack.0, in_tag, facts)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub id: Identifier,
    pub inputs: Vec<Sized>,
    pub tools: Ingredient,
    pub max_tool_size: u32,
    pub result: Identifier,
    pub min_level: i32,
    pub max_level: i32,
    /// The slot type and count this recipe spends.
    pub slots: Option<(String, i32)>,
    pub check_trait_level: bool,
    /// Whether a modifier crystal of this recipe's modifier can stand in for the inputs (the original's `allow_crystal`, true by default).
    pub allow_crystal: bool,
    pub kind: Kind,
    /// Per-level level ranges and slots of the multilevel recipe types (empty for the others).
    pub levels: Vec<LevelEntry>,
}

/// MultilevelModifierRecipe.LevelEntry: the levels it covers and the slots a level there spends.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LevelEntry { pub min: i32, pub max: i32, pub slots: Option<(String, i32)> }

fn slot_count(value: Option<&Value>) -> Result<Option<(String, i32)>, String> {
    match value.filter(|value| !value.is_null()) {
        None => Ok(None),
        Some(Value::Object(slots)) if slots.len() == 1 => {
            let (name, count) = slots.iter().next().expect("one entry");
            let Value::Number(count) = count else { return Err("slot count must be a number".into()); };
            let count = crate::materials::number_int(&count.to_string());
            if count < 1 { return Err("slot count must be at least 1".into()); }
            if !name.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_')) { return Err(format!("invalid modifier slot name {name}")); }
            Ok(Some((name.clone(), count)))
        }
        Some(_) => Err("slots must name exactly one slot type".into()),
    }
}

/// What differs between the single-step recipe and the incremental one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Kind {
    /// `tconstruct:modifier`: the listed inputs add one level.
    Single,
    /// `tconstruct:incremental_modifier`: one ingredient, each item worth `amount_per_item` toward
    /// the `needed_per_level` a level takes; partial progress is kept on the tool.
    Incremental { input: Ingredient, amount_per_item: i32, needed_per_level: i32, leftover: Option<Identifier> },
    /// `tconstruct:swappable_modifier`: one level, plus a variant string kept in the tool's
    /// persistent data under the modifier's ID; a recipe for another variant swaps it.
    Swappable { value: String },
}

fn positive(fields: &Map<String, Value>, key: &str, default: Option<i32>) -> Result<i32, String> {
    match fields.get(key) {
        None => default.ok_or_else(|| format!("{key} missing")),
        Some(Value::Number(n)) => { let n = crate::materials::number_int(&n.to_string()); if n >= 1 { Ok(n) } else { Err(format!("{key} must be at least 1")) } }
        Some(_) => Err(format!("{key} must be a number")),
    }
}

impl Recipe {
    fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        let kind_name = fields.get("type").and_then(Value::as_str);
        let multilevel = matches!(kind_name, Some("tconstruct:multilevel_modifier" | "tconstruct:multilevel_incremental_modifier"));
        let (inputs, kind) = if matches!(kind_name, Some("tconstruct:incremental_modifier" | "tconstruct:multilevel_incremental_modifier")) {
            let leftover = match fields.get("leftover") {
                None | Some(Value::Null) => None,
                Some(Value::String(text)) => Some(Identifier::parse(text).map_err(|e| e.to_string())?),
                Some(_) => return Err("leftover must be an item ID string".into()),
            };
            (Vec::new(), Kind::Incremental {
                input: Ingredient::decode(fields.get("input").ok_or("input missing")?)?,
                amount_per_item: positive(fields, "amount_per_item", Some(1))?,
                needed_per_level: positive(fields, "needed_per_level", None)?,
                leftover,
            })
        } else {
            let swappable = kind_name == Some("tconstruct:swappable_modifier");
            let inputs = fields.get("inputs").and_then(Value::as_array).filter(|inputs| !inputs.is_empty()).ok_or("inputs must be a nonempty array")?
                .iter().map(Sized::decode).collect::<Result<Vec<_>, _>>()?;
            (inputs, if swappable {
                // The variant formatter only changes how the variant is displayed.
                match fields.get("variant_formatter") {
                    None | Some(Value::Null) => {}
                    Some(Value::String(name)) if matches!(name.as_str(), "tconstruct:default" | "tconstruct:material" | "tconstruct:parameter") => {}
                    Some(_) => return Err("unknown variant_formatter".into()),
                }
                Kind::Swappable { value: String::new() }
            } else { Kind::Single })
        };
        // A swappable recipe names its modifier and variant in one "result" object.
        let (kind, result_value) = match (kind, fields.get("result")) {
            (Kind::Swappable { .. }, Some(Value::Object(result))) => {
                let value = result.get("value").and_then(Value::as_str).ok_or("result value must be a string")?.to_owned();
                (Kind::Swappable { value }, result.get("name").cloned().ok_or("result name missing")?)
            }
            (Kind::Swappable { .. }, _) => return Err("a swappable recipe's result must be an object with name and value".into()),
            (kind, result) => (kind, result.cloned().ok_or("result must be an ID string")?),
        };
        let tools = Ingredient::decode(fields.get("tools").ok_or("tools missing")?)?;
        let max_tool_size = match fields.get("max_tool_size") {
            None | Some(Value::Null) => 16,
            Some(Value::Number(n)) => { let n = crate::materials::number_int(&n.to_string()); if n >= 1 { n as u32 } else { return Err("max_tool_size must be at least 1".into()); } }
            Some(_) => return Err("max_tool_size must be a number".into()),
        };
        let result = Identifier::parse(result_value.as_str().ok_or("result must be an ID string")?).map_err(|e| e.to_string())?;
        let (min_level, max_level) = if matches!(kind, Kind::Swappable { .. }) { (1, 1) } else { crate::modifiers::range(fields.get("level"))? };
        let slots = slot_count(fields.get("slots"))?;
        let levels = if multilevel {
            fields.get("levels").and_then(Value::as_array).filter(|levels| !levels.is_empty()).ok_or("levels must be a nonempty array")?.iter().map(|entry| {
                let entry = entry.as_object().ok_or("a level entry must be an object")?;
                let (min, max) = crate::modifiers::range(Some(entry.get("level").ok_or("a level entry needs a level")?))?;
                Ok(LevelEntry { min, max, slots: slot_count(entry.get("slots"))? })
            }).collect::<Result<Vec<_>, String>>()?
        } else { Vec::new() };
        let (min_level, max_level) = levels.first().map_or((min_level, max_level), |first| (first.min, first.max));
        let check_trait_level = match fields.get("check_trait_level") { None => false, Some(Value::Bool(value)) => *value, Some(_) => return Err("check_trait_level must be boolean".into()) };
        let allow_crystal = match fields.get("allow_crystal") { None | Some(Value::Null) => true, Some(Value::Bool(value)) => *value, Some(_) => return Err("allow_crystal must be boolean".into()) };
        Ok(Self { id: id.clone(), inputs, tools, max_tool_size, result, min_level, max_level, slots, check_trait_level, allow_crystal, kind, levels })
    }

    /// matches(): the tool qualifies and the inputs are exactly this recipe's, with no extras.
    /// `inputs` holds each slot's item and count, None for an empty slot.
    pub fn matches(&self, tool: &Identifier, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        if !self.tools.matches(tool, in_tag) { return false; }
        match &self.kind {
            Kind::Single | Kind::Swappable { .. } => self.assign(inputs, in_tag).is_some(),
            // containsOnlyIngredient: at least one match and nothing else in the table.
            Kind::Incremental { input, .. } => {
                let stacks: Vec<&(Identifier, u32)> = inputs.iter().flatten().collect();
                !stacks.is_empty() && stacks.iter().all(|stack| input.matches(&stack.0, in_tag))
            }
        }
    }

    /// checkMatch: each ingredient takes the first unused matching slot (empty slots count as used),
    /// and nothing may be left over. Returns the slot taken by each ingredient.
    fn assign(&self, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<Vec<usize>> {
        check_match(&self.inputs, inputs, in_tag)
    }

    /// updateInputs: how many to take from each slot in slot order, and what to hand back.
    /// `before` is the tool as placed, `after` the crafted result.
    pub fn consumption(&self, before: &ItemSlot, after: &ItemSlot, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> (Vec<u32>, Vec<(Identifier, u32)>) {
        let mut consumed = vec![0; inputs.len()];
        let mut leftover = Vec::new();
        match &self.kind {
            Kind::Single | Kind::Swappable { .. } => if let Some(taken) = self.assign(inputs, in_tag) {
                for (slot, ingredient) in taken.into_iter().zip(&self.inputs) { consumed[slot] += ingredient.amount; }
            },
            Kind::Incremental { input, amount_per_item, needed_per_level, leftover: item } => {
                let entry = |tool: &ItemSlot| List::from_nbt(tool.custom_data.get(TAG_UPGRADES)).entry(&self.result);
                let (was, now) = (entry(before), entry(after));
                let need = was.needed();
                let amount = if need == 0 || need == *needed_per_level {
                    now.amount(*needed_per_level).wrapping_sub(was.amount(0))
                } else {
                    // The sizes differ: rescale into the input's terms and round up, consuming a bit too much.
                    let gcd = crate::modifier_entries::gcd(need, *needed_per_level);
                    let scale = need / gcd;
                    let used = now.amount(*needed_per_level).wrapping_mul(scale).wrapping_sub(was.amount(0).wrapping_mul(*needed_per_level) / gcd);
                    (used + scale - 1) / scale
                };
                let amount = amount.max(0);
                let mut items = amount / amount_per_item;
                let rest = amount % amount_per_item;
                if rest > 0 {
                    items += 1;
                    if let Some(item) = item { leftover.push((item.clone(), (amount_per_item - rest) as u32)); }
                }
                let mut remaining = items as u32;
                for (slot, stack) in inputs.iter().enumerate() {
                    let Some((id, count)) = stack else { continue };
                    if !input.matches(id, in_tag) { continue; }
                    let take = (*count).min(remaining);
                    consumed[slot] = take;
                    remaining -= take;
                }
            }
        }
        (consumed, leftover)
    }

    /// getValidatedResult: the tool with this modifier added, or the failure's message key
    /// (followed by space-separated arguments).
    pub fn apply(&self, tool: &ItemSlot, env: &Environment, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Result<Result<ItemSlot, String>, String> {
        self.apply_inner(tool, env, inputs, in_tag, false)
    }

    /// The same with a modifier crystal standing in for the inputs: it always adds a whole new level, even to a partly filled
    /// incremental modifier.
    pub fn apply_crystal(&self, tool: &ItemSlot, env: &Environment, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Result<Result<ItemSlot, String>, String> {
        self.apply_inner(tool, env, &[], in_tag, true)
    }

    fn apply_inner(&self, tool: &ItemSlot, env: &Environment, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, crystal: bool) -> Result<Result<ItemSlot, String>, String> {
        let nbt = &tool.custom_data;
        let list = List::from_nbt(nbt.get(if self.check_trait_level { TAG_MODIFIERS } else { TAG_UPGRADES }));
        let entry = list.entry(&self.result);
        // An incremental recipe continuing a partial level only checks that level; anything else adds a level.
        let continuing = !crystal && matches!(self.kind, Kind::Incremental { .. }) && entry.amount(0) > 0;
        let new_level = if continuing { entry.level } else { list.level(&self.result).wrapping_add(1) };
        let traits = if self.check_trait_level { ".traits" } else { "" };
        let persistent_of = |tool: &ItemSlot| match tool.custom_data.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
        // A swappable recipe adds the modifier only when absent; with it present only the variant changes.
        let swapping = matches!(self.kind, Kind::Swappable { .. }) && list.level(&self.result) > 0;
        if let (Kind::Swappable { value }, true) = (&self.kind, swapping) {
            if matches!(persistent_of(tool).get(&self.result.to_string()), Some(Nbt::String(current)) if current == value) {
                return Ok(Err(format!("recipe.tconstruct.swappable.already_present {} {value}", self.result)));
            }
        }
        // Multilevel recipes find the entry for this level; gaps and out-of-range levels fail like the bounds do.
        let level_entry = self.levels.iter().find(|entry| entry.min <= new_level && new_level <= entry.max);
        let (low, high) = match (self.levels.first(), self.levels.last()) { (Some(first), Some(last)) => (first.min, last.max), _ => (self.min_level, self.max_level) };
        if !swapping && (new_level < low || (new_level > high) || (!self.levels.is_empty() && level_entry.is_none())) {
            let (key, bound) = if new_level < low { ("min_level", low - 1) } else { ("max_level", high) };
            return Ok(Err(format!("recipe.tconstruct.modifier.{key}{traits} {} {bound}", self.result)));
        }
        let slots = if self.levels.is_empty() { self.slots.as_ref() } else { level_entry.and_then(|entry| entry.slots.as_ref()) };
        let spends_slots = !continuing && !swapping;
        let free = |name: &str| {
            let slots = |tag: &str| match nbt.get(tag) { Some(Nbt::Compound(data)) => crate::modifier_entries::nbt_int(data.get(name)), _ => 0 };
            slots(TAG_PERSISTENT).wrapping_add(slots(crate::tool_stack::TAG_VOLATILE))
        };
        if let Some((name, count)) = slots.filter(|_| spends_slots) {
            if free(name) < *count {
                return Ok(Err(if *count == 1 { format!("recipe.tconstruct.modifier.not_enough_slot slot_type.tconstruct.{name}") }
                    else { format!("recipe.tconstruct.modifier.not_enough_slots {count} slot_type.tconstruct.{name}") }));
            }
        }
        let mut copy = tool.clone();
        if let Some((name, count)) = slots.filter(|_| spends_slots) {
            // ToolDataNBT.addSlots(type, -count): a total of zero removes the key.
            let mut persistent = match copy.custom_data.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
            let total = crate::modifier_entries::nbt_int(persistent.get(name)).wrapping_sub(*count);
            if total == 0 { persistent.remove(name); } else { persistent.insert(name.clone(), Nbt::Int(total)); }
            copy.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        }
        if let Kind::Swappable { value } = &self.kind {
            let mut persistent = persistent_of(&copy);
            persistent.insert(self.result.to_string(), Nbt::String(value.clone()));
            copy.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        }
        // ToolStack.addModifier(id, 1): save the new upgrade list, then rebuild.
        let current = List::from_nbt(copy.custom_data.get(TAG_UPGRADES));
        let upgrades = match &self.kind {
            Kind::Swappable { .. } if swapping => current,
            Kind::Swappable { .. } => current.with_modifier(self.result.clone(), 1)?,
            Kind::Single => current.with_modifier(self.result.clone(), 1)?,
            Kind::Incremental { .. } if crystal => current.with_modifier(self.result.clone(), 1)?,
            // getAvailableAmount: every matching stack counts in full; addModifierAmount stops at the level.
            Kind::Incremental { input, amount_per_item, needed_per_level, .. } => {
                let available = inputs.iter().flatten().filter(|stack| input.matches(&stack.0, in_tag))
                    .fold(0i32, |sum, stack| sum.wrapping_add((stack.1 as i32).wrapping_mul(*amount_per_item)));
                current.add_amount(self.result.clone(), available, *needed_per_level)
            }
        };
        copy.custom_data.insert(TAG_UPGRADES.into(), upgrades.to_nbt());
        // An environment that is not ready cannot rebuild; the recipe then must not produce a stale tool.
        if !rebuild_stats(&mut copy.custom_data, env)? { return Err("the tool could not be rebuilt".into()); }
        if let Some(error) = try_validate(&copy.custom_data, env) { return Ok(Err(error)); }
        tool_slot::sync_visuals(&mut copy, env)?;
        copy.count = tool.count.min(self.max_tool_size);
        Ok(Ok(copy))
    }
}

/// ModifierRecipe.checkMatch: each ingredient takes the first unused matching slot (empty slots count
/// as used) and nothing may be left over. Returns the slot taken by each ingredient.
pub(crate) fn check_match(ingredients: &[Sized], inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<Vec<usize>> {
    check_match_facts(ingredients, inputs, &[], in_tag)
}

/// check_match with a facts entry per input slot (slots past the list read as a stack without any).
pub(crate) fn check_match_facts(ingredients: &[Sized], inputs: &[Option<(Identifier, u32)>], facts: &[crate::material_recipes::StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<Vec<usize>> {
    let mut used: Vec<bool> = inputs.iter().map(Option::is_none).collect();
    let mut taken = Vec::with_capacity(ingredients.len());
    for ingredient in ingredients {
        let slot = (0..inputs.len()).find(|&slot| !used[slot] && inputs[slot].as_ref().is_some_and(|stack| {
            ingredient.test_facts(stack, in_tag, &facts.get(slot).copied().unwrap_or_default())
        }))?;
        used[slot] = true;
        taken.push(slot);
    }
    used.iter().all(|used| *used).then_some(taken)
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recipes { recipes: Vec<Recipe>, pub problems: Vec<String>, pub loaded: bool }

impl Recipes {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Recipe>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if !matches!(fields.get("type").and_then(Value::as_str), Some("tconstruct:modifier" | "tconstruct:incremental_modifier" | "tconstruct:swappable_modifier" | "tconstruct:multilevel_modifier" | "tconstruct:multilevel_incremental_modifier")) { return Ok(None); }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                }
                Recipe::decode(id, fields).map(Some)
            })();
            match outcome {
                Ok(Some(recipe)) => result.recipes.push(recipe),
                Ok(None) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }
    /// The first recipe whose tool requirement and inputs match, in id order.
    pub fn find(&self, tool: &Identifier, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Recipe> {
        self.recipes.iter().find(|recipe| recipe.matches(tool, inputs, in_tag))
    }
    /// The recipe a modifier crystal applies for `modifier` on this tool: a single-step or incremental recipe that allows crystals.
    pub fn find_crystal(&self, tool: &Identifier, modifier: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Recipe> {
        self.recipes.iter().find(|recipe| recipe.allow_crystal && matches!(recipe.kind, Kind::Single | Kind::Incremental { .. }) && &recipe.result == modifier && recipe.tools.matches(tool, in_tag))
    }
    pub fn len(&self) -> usize { self.recipes.len() }
}

/// The crystal item and the NBT key holding its modifier.
pub(crate) const CRYSTAL_ITEM: (&str, &str) = ("tconstruct", "modifier_crystal");
pub(crate) const CRYSTAL_KEY: &str = "modifier";

/// matchesCrystal: the table holds exactly one stack and it is a crystal; returns the modifier it carries and its slot.
pub(crate) fn lone_crystal(stacks: &[Option<(Identifier, u32, Option<Identifier>)>]) -> Option<(usize, Identifier)> {
    let mut found = None;
    for (slot, stack) in stacks.iter().enumerate() {
        let Some((item, _, modifier)) = stack else { continue; };
        if found.is_some() || item.namespace() != CRYSTAL_ITEM.0 || item.path() != CRYSTAL_ITEM.1 { return None; }
        found = Some((slot, modifier.clone()?));
    }
    found
}

/// Reload listener, shaped like the material recipes': a missing condition context clears and fails.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Recipes>>, String> {
    let state = Arc::new(RwLock::new(Recipes::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "modifier_recipes").unwrap(), FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "modifier recipes poisoned")?;
            match context {
                Ok(context) => {
                    *state = Recipes::load(files, &context);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { *state = Recipes { problems: vec![error.clone()], ..Recipes::default() }; Err(error) }
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("modifier recipes poisoned") = Recipes::default());
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
    fn recipe(extra: Value) -> Recipe {
        let mut base = json!({"type":"tconstruct:modifier","inputs":[{"item":"minecraft:diamond"},{"item":"minecraft:stick","amount_needed":2}],
            "tools":{"tag":"tconstruct:modifiable/harvest"},"result":"test:sharp","slots":{"upgrades":1},"level":{"max":3}});
        base.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        Recipe::decode(&id("test:recipe"), base.as_object().unwrap()).unwrap()
    }
    fn stack(item: &str, count: u32) -> Option<(Identifier, u32)> { Some((id(item), count)) }
    #[test]
    fn inputs_must_match_exactly_with_no_leftovers_and_enough_of_each() {
        let r = recipe(json!({}));
        let in_tag = |item: &Identifier, tag: &Identifier| item.path() == "pickaxe" && tag.path() == "modifiable/harvest";
        let pickaxe = id("tconstruct:pickaxe");
        // Slot order does not matter; each ingredient takes the first unused matching slot.
        assert!(r.matches(&pickaxe, &[stack("minecraft:stick", 2), None, stack("minecraft:diamond", 1), None], &in_tag));
        assert!(!r.matches(&pickaxe, &[stack("minecraft:stick", 1), stack("minecraft:diamond", 1)], &in_tag), "the stick needs 2");
        assert!(!r.matches(&pickaxe, &[stack("minecraft:stick", 2), stack("minecraft:diamond", 1), stack("minecraft:dirt", 1)], &in_tag), "a leftover input blocks the recipe");
        assert!(!r.matches(&pickaxe, &[stack("minecraft:diamond", 1)], &in_tag), "a missing ingredient");
        assert!(!r.matches(&id("minecraft:stick"), &[stack("minecraft:stick", 2), stack("minecraft:diamond", 1)], &in_tag), "the tool must qualify");
        // Consumption takes each ingredient's amount from the slot it matched.
        let tool = ItemSlot::new(pickaxe.clone(), 1);
        assert_eq!(r.consumption(&tool, &tool, &[stack("minecraft:stick", 5), stack("minecraft:diamond", 3)], &in_tag), (vec![2, 1], vec![]));
    }
    fn incremental(extra: Value) -> Recipe {
        let mut base = json!({"type":"tconstruct:incremental_modifier","input":{"item":"test:shard"},"amount_per_item":2,"needed_per_level":5,
            "leftover":"test:dust","tools":{"tag":"test:tools"},"result":"test:grow","slots":{"upgrades":1},"level":{"max":3}});
        base.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        Recipe::decode(&id("test:inc"), base.as_object().unwrap()).unwrap()
    }
    fn with_upgrades(amount: i32, needed: i32) -> ItemSlot {
        let mut tool = ItemSlot::new(id("test:pick"), 1);
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().add_amount(id("test:grow"), amount, needed).to_nbt());
        tool
    }
    #[test]
    fn incremental_recipes_match_only_their_ingredient_and_count_partial_progress() {
        let r = incremental(json!({}));
        let in_tag = |_: &Identifier, tag: &Identifier| tag.path() == "tools";
        let pick = id("test:pick");
        assert!(r.matches(&pick, &[stack("test:shard", 1), None, stack("test:shard", 3)], &in_tag), "any number of the ingredient in any slots");
        assert!(!r.matches(&pick, &[stack("test:shard", 1), stack("minecraft:dirt", 1)], &in_tag), "a stray item blocks it");
        assert!(!r.matches(&pick, &[None, None], &in_tag), "needs at least one item");
        // From nothing, two shards (worth 4 of 5) become a partial level; 3 items round up and hand back 1 dust.
        let none = ItemSlot::new(pick.clone(), 1);
        let after = with_upgrades(4, 5);
        assert_eq!(r.consumption(&none, &after, &[stack("test:shard", 2)], &in_tag), (vec![2], vec![]));
        let after = with_upgrades(5, 5);
        assert_eq!(r.consumption(&none, &after, &[stack("test:shard", 3), stack("test:shard", 1)], &in_tag), (vec![3, 0], vec![(id("test:dust"), 1)]),
            "5 needed at 2 per item takes 3 items and gives back the 1 unused");
    }
    #[test]
    fn swappable_recipes_read_name_and_value_and_behave_like_single_step_recipes() {
        let base = json!({"type":"tconstruct:swappable_modifier","inputs":[{"item":"a:b"}],"tools":{"tag":"a:t"},"result":{"name":"test:shot","value":"amethyst"},"variant_formatter":"tconstruct:material","slots":{"abilities":1}});
        let r = Recipe::decode(&id("test:swap"), base.as_object().unwrap()).unwrap();
        assert_eq!((r.result.to_string(), r.min_level, r.max_level), ("test:shot".to_owned(), 1, 1));
        assert_eq!(r.kind, Kind::Swappable { value: "amethyst".into() });
        let in_tag = |_: &Identifier, _: &Identifier| true;
        assert!(r.matches(&id("x:tool"), &[stack("a:b", 1)], &in_tag));
        let mut bad = base.clone();
        bad["result"] = json!("test:shot");
        assert!(Recipe::decode(&id("test:bad"), bad.as_object().unwrap()).is_err(), "result must be name and value");
        bad = base.clone();
        bad["variant_formatter"] = json!("test:unknown");
        assert!(Recipe::decode(&id("test:bad"), bad.as_object().unwrap()).is_err());
    }
    #[test]
    fn multilevel_recipes_take_ranges_and_slots_per_level() {
        let base = json!({"type":"tconstruct:multilevel_modifier","inputs":[{"item":"a:b"}],"tools":{"tag":"a:t"},"result":"test:reflect",
            "levels":[{"level":1,"slots":{"abilities":1}},{"level":{"min":2,"max":3},"slots":{"upgrades":1}}]});
        let r = Recipe::decode(&id("test:m"), base.as_object().unwrap()).unwrap();
        assert_eq!((r.min_level, r.max_level, r.levels.len()), (1, 1, 2), "the first entry is the recipe's own range");
        assert_eq!(r.levels[1], LevelEntry { min: 2, max: 3, slots: Some(("upgrades".into(), 1)) });
        let mut bad = base.clone();
        bad["levels"] = json!([]);
        assert!(Recipe::decode(&id("test:bad"), bad.as_object().unwrap()).is_err());
        let inc = json!({"type":"tconstruct:multilevel_incremental_modifier","input":{"item":"a:b"},"needed_per_level":12,"tools":{"tag":"a:t"},"result":"test:fall",
            "levels":[{"level":1,"slots":{"upgrades":1}},{"level":2}]});
        let r = Recipe::decode(&id("test:i"), inc.as_object().unwrap()).unwrap();
        assert!(matches!(r.kind, Kind::Incremental { needed_per_level: 12, .. }) && r.levels[1].slots.is_none());
    }
    #[test]
    fn incremental_decode_validates_fields() {
        let bad = |extra: Value| {
            let mut base = json!({"type":"tconstruct:incremental_modifier","input":{"item":"a:b"},"needed_per_level":4,"tools":{"tag":"a:t"},"result":"a:r"});
            base.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            Recipe::decode(&id("test:bad"), base.as_object().unwrap()).is_err()
        };
        assert!(!bad(json!({})) && bad(json!({"needed_per_level":0})) && bad(json!({"amount_per_item":0})) && bad(json!({"leftover":3})) && bad(json!({"input":null})));
    }
    #[test]
    fn decode_accepts_shipped_shapes_and_rejects_malformed_recipes() {
        let r = recipe(json!({"level":2,"max_tool_size":4,"check_trait_level":true,"inputs":[{"ingredient":[{"item":"a:b"},{"item":"c:d"}],"amount_needed":3}]}));
        assert_eq!((r.min_level, r.max_level, r.max_tool_size, r.check_trait_level), (2, 2, 4, true));
        assert_eq!(r.inputs[0].amount, 3);
        let bad = |extra: Value| {
            let mut base = json!({"type":"tconstruct:modifier","inputs":[{"item":"a:b"}],"tools":{"tag":"a:b"},"result":"a:c"});
            base.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            Recipe::decode(&id("test:bad"), base.as_object().unwrap()).is_err()
        };
        assert!(bad(json!({"inputs":[]})) && bad(json!({"slots":{"upgrades":1,"abilities":1}})) && bad(json!({"slots":{"Bad Name":1}})) && bad(json!({"slots":{"upgrades":0}})));
        assert!(bad(json!({"level":{"min":3,"max":2}})) && bad(json!({"max_tool_size":0})) && bad(json!({"result":3})));
        assert!(bad(json!({"inputs":[{"type":"tconstruct:not_ported","tag":"tconstruct:tanks"}]})), "ingredients this port lacks reject the recipe");
    }
    #[test]
    fn recipes_load_in_id_order_with_conditions_and_report_unsupported_ones() {
        let make = |name: &str, extra: Value| { let mut v = json!({"type":"tconstruct:modifier","inputs":[{"item":"a:b"}],"tools":{"tag":"a:t"},"result":"a:r"}); v.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone()); (id(name), v) };
        let files = vec![make("test:b", json!({})), make("test:a", json!({})), make("test:off", json!({"conditions":[false]})),
            make("test:bad", json!({"inputs":[]})), (id("test:salvage"), json!({"type":"tconstruct:modifier_salvage"}))];
        let recipes = Recipes::load(&files, &Flags);
        assert_eq!((recipes.len(), recipes.problems.len()), (2, 1), "{:?}", recipes.problems);
        let in_tag = |_: &Identifier, _: &Identifier| true;
        assert_eq!(recipes.find(&id("x:tool"), &[stack("a:b", 1)], &in_tag).unwrap().id, id("test:a"));
    }
}
