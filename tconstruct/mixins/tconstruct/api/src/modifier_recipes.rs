//! The `tconstruct:modifier` station recipe: add a modifier level to a tool for items and slots
//! (SlimeKnights ModifierRecipe/AbstractModifierRecipe, MIT; see LICENSE). Crystals, incremental,
//! swappable and the other modifier recipe types are separate recipe types and not loaded here.
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
    fn decode(value: &Value) -> Result<Self, String> {
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
    fn test(&self, stack: &(Identifier, u32), in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        stack.1 >= self.amount && self.ingredient.matches(&stack.0, in_tag)
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
}

impl Recipe {
    fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        let inputs = fields.get("inputs").and_then(Value::as_array).filter(|inputs| !inputs.is_empty()).ok_or("inputs must be a nonempty array")?
            .iter().map(Sized::decode).collect::<Result<Vec<_>, _>>()?;
        let tools = Ingredient::decode(fields.get("tools").ok_or("tools missing")?)?;
        let max_tool_size = match fields.get("max_tool_size") {
            None | Some(Value::Null) => 16,
            Some(Value::Number(n)) => { let n = crate::materials::number_int(&n.to_string()); if n >= 1 { n as u32 } else { return Err("max_tool_size must be at least 1".into()); } }
            Some(_) => return Err("max_tool_size must be a number".into()),
        };
        let result = Identifier::parse(fields.get("result").and_then(Value::as_str).ok_or("result must be an ID string")?).map_err(|e| e.to_string())?;
        let (min_level, max_level) = crate::modifiers::range(fields.get("level"))?;
        let slots = match fields.get("slots").filter(|value| !value.is_null()) {
            None => None,
            Some(Value::Object(slots)) if slots.len() == 1 => {
                let (name, count) = slots.iter().next().expect("one entry");
                let Value::Number(count) = count else { return Err("slot count must be a number".into()); };
                let count = crate::materials::number_int(&count.to_string());
                if count < 1 { return Err("slot count must be at least 1".into()); }
                if !name.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_')) { return Err(format!("invalid modifier slot name {name}")); }
                Some((name.clone(), count))
            }
            Some(_) => return Err("slots must name exactly one slot type".into()),
        };
        let check_trait_level = match fields.get("check_trait_level") { None => false, Some(Value::Bool(value)) => *value, Some(_) => return Err("check_trait_level must be boolean".into()) };
        Ok(Self { id: id.clone(), inputs, tools, max_tool_size, result, min_level, max_level, slots, check_trait_level })
    }

    /// matches(): the tool qualifies and the inputs are exactly this recipe's, with no extras.
    /// `inputs` holds each slot's item and count, None for an empty slot.
    pub fn matches(&self, tool: &Identifier, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.tools.matches(tool, in_tag) && self.assign(inputs, in_tag).is_some()
    }

    /// checkMatch: each ingredient takes the first unused matching slot (empty slots count as used),
    /// and nothing may be left over. Returns the slot taken by each ingredient.
    fn assign(&self, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<Vec<usize>> {
        let mut used: Vec<bool> = inputs.iter().map(Option::is_none).collect();
        let mut taken = Vec::with_capacity(self.inputs.len());
        for ingredient in &self.inputs {
            let slot = (0..inputs.len()).find(|&slot| !used[slot] && inputs[slot].as_ref().is_some_and(|stack| ingredient.test(stack, in_tag)))?;
            used[slot] = true;
            taken.push(slot);
        }
        used.iter().all(|used| *used).then_some(taken)
    }

    /// updateInputs: how many to take from each slot, in slot order.
    pub fn consumption(&self, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Vec<u32> {
        let mut consumed = vec![0; inputs.len()];
        if let Some(taken) = self.assign(inputs, in_tag) {
            for (slot, ingredient) in taken.into_iter().zip(&self.inputs) { consumed[slot] += ingredient.amount; }
        }
        consumed
    }

    /// getValidatedResult: the tool with this modifier added, or the failure's message key
    /// (followed by space-separated arguments).
    pub fn apply(&self, tool: &ItemSlot, env: &Environment) -> Result<Result<ItemSlot, String>, String> {
        let nbt = &tool.custom_data;
        let level_of = |tag: &str| List::from_nbt(nbt.get(tag)).level(&self.result);
        let new_level = level_of(if self.check_trait_level { TAG_MODIFIERS } else { TAG_UPGRADES }).wrapping_add(1);
        let traits = if self.check_trait_level { ".traits" } else { "" };
        if new_level < self.min_level {
            return Ok(Err(format!("recipe.tconstruct.modifier.min_level{traits} {} {}", self.result, self.min_level - 1)));
        }
        if new_level > self.max_level {
            return Ok(Err(format!("recipe.tconstruct.modifier.max_level{traits} {} {}", self.result, self.max_level)));
        }
        let free = |name: &str| {
            let slots = |tag: &str| match nbt.get(tag) { Some(Nbt::Compound(data)) => crate::modifier_entries::nbt_int(data.get(name)), _ => 0 };
            slots(TAG_PERSISTENT).wrapping_add(slots(crate::tool_stack::TAG_VOLATILE))
        };
        if let Some((name, count)) = &self.slots {
            if free(name) < *count {
                return Ok(Err(if *count == 1 { format!("recipe.tconstruct.modifier.not_enough_slot slot_type.tconstruct.{name}") }
                    else { format!("recipe.tconstruct.modifier.not_enough_slots {count} slot_type.tconstruct.{name}") }));
            }
        }
        let mut copy = tool.clone();
        if let Some((name, count)) = &self.slots {
            // ToolDataNBT.addSlots(type, -count): a total of zero removes the key.
            let mut persistent = match copy.custom_data.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
            let total = crate::modifier_entries::nbt_int(persistent.get(name)).wrapping_sub(*count);
            if total == 0 { persistent.remove(name); } else { persistent.insert(name.clone(), Nbt::Int(total)); }
            copy.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        }
        // ToolStack.addModifier(id, 1): save the new upgrade list, then rebuild.
        let upgrades = List::from_nbt(copy.custom_data.get(TAG_UPGRADES)).with_modifier(self.result.clone(), 1)?;
        copy.custom_data.insert(TAG_UPGRADES.into(), upgrades.to_nbt());
        // An environment that is not ready cannot rebuild; the recipe then must not produce a stale tool.
        if !rebuild_stats(&mut copy.custom_data, env)? { return Err("the tool could not be rebuilt".into()); }
        if let Some(error) = try_validate(&copy.custom_data, env) { return Ok(Err(error)); }
        tool_slot::sync_visuals(&mut copy, env)?;
        copy.count = tool.count.min(self.max_tool_size);
        Ok(Ok(copy))
    }
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
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:modifier") { return Ok(None); }
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
    pub fn len(&self) -> usize { self.recipes.len() }
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
        assert_eq!(r.consumption(&[stack("minecraft:stick", 5), stack("minecraft:diamond", 3)], &in_tag), vec![2, 1]);
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
        assert!(bad(json!({"inputs":[{"type":"tconstruct:no_container","tag":"tconstruct:tanks"}]})), "ingredients this port lacks reject the recipe");
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
