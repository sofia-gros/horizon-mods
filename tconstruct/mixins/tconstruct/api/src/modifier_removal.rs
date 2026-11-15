//! Modifier salvage data and the removal of one modifier level from a tool, from SlimeKnights
//! ModifierSalvage and ModifierRemovalRecipe.getResult (MIT; see LICENSE). The worktable block, its
//! modifier buttons and the removal recipes' input matching are separate.
use crate::{
    material_recipes::Ingredient,
    materials::Conditions,
    modifier_entries::List,
    tool_slot,
    tool_stack::{rebuild_stats, try_validate, Environment, TAG_PERSISTENT, TAG_UPGRADES},
};
use pluto_lib::{Identifier, ItemSlot, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

/// Salvage records live beside the modifier recipes.
pub(crate) const FOLDER: &str = crate::modifier_recipes::FOLDER;

/// ModifierSalvage: the slots handed back when this modifier is removed from a matching tool.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Salvage {
    pub id: Identifier,
    pub tools: Ingredient,
    pub modifier: Identifier,
    pub min_level: i32,
    pub max_level: i32,
    pub slot: (String, i32),
}

impl Salvage {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let tools = Ingredient::decode(fields.get("tools").ok_or("tools missing")?)?;
        let modifier = Identifier::parse(fields.get("modifier").and_then(Value::as_str).ok_or("modifier must be an ID string")?).map_err(|e| e.to_string())?;
        let (min_level, max_level) = crate::modifiers::range(fields.get("level"))?;
        let slot = match fields.get("slots") {
            Some(Value::Object(slots)) if slots.len() == 1 => {
                let (name, count) = slots.iter().next().expect("one entry");
                let Value::Number(count) = count else { return Err("slot count must be a number".into()); };
                let count = crate::materials::number_int(&count.to_string());
                if count < 1 { return Err("slot count must be at least 1".into()); }
                if !name.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_')) { return Err(format!("invalid modifier slot name {name}")); }
                (name.clone(), count)
            }
            _ => return Err("slots must name exactly one slot type".into()),
        };
        Ok(Self { id: id.clone(), tools, modifier, min_level, max_level, slot })
    }
    /// matches: the level is in range and the tool qualifies.
    pub fn matches(&self, tool: &Identifier, level: i32, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.min_level <= level && level <= self.max_level && self.tools.matches(tool, in_tag)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Salvages { salvages: Vec<Salvage>, pub problems: Vec<String>, pub loaded: bool }

impl Salvages {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Salvage>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:modifier_salvage") { return Ok(None); }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                }
                Salvage::decode(id, value).map(Some)
            })();
            match outcome {
                Ok(Some(salvage)) => result.salvages.push(salvage),
                Ok(None) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }
    pub fn len(&self) -> usize { self.salvages.len() }
    /// ModifierRecipeLookup.getSalvage: the first salvage for this modifier, level and tool.
    pub fn find(&self, tool: &Identifier, modifier: &Identifier, level: i32, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Salvage> {
        self.salvages.iter().find(|salvage| &salvage.modifier == modifier && salvage.matches(tool, level, in_tag))
    }
}

/// Removes one level of `modifier` from the tool's upgrades (ModifierRemovalRecipe.getResult), giving
/// back the salvage's slots. `Ok(Err(key))` is a failure message; `Err` an unusable environment.
pub(crate) fn remove(tool: &ItemSlot, env: &Environment, modifier: &Identifier, salvages: &Salvages,
    in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Result<Result<ItemSlot, String>, String> {
    let upgrades = List::from_nbt(tool.custom_data.get(TAG_UPGRADES));
    let level = upgrades.level(modifier);
    if level <= 0 { return Ok(Err("recipe.tconstruct.remove_modifier.no_modifiers".into())); }
    let mut copy = tool.clone();
    if let Some(salvage) = salvages.find(&tool.item, modifier, level, in_tag) {
        // ToolDataNBT.addSlots(type, count): a total of zero removes the key.
        let mut persistent = match copy.custom_data.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
        let total = crate::modifier_entries::nbt_int(persistent.get(&salvage.slot.0)).wrapping_add(salvage.slot.1);
        if total == 0 { persistent.remove(&salvage.slot.0); } else { persistent.insert(salvage.slot.0.clone(), Nbt::Int(total)); }
        copy.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    }
    let reduced = upgrades.without_modifier(modifier, 1)?;
    copy.custom_data.insert(TAG_UPGRADES.into(), reduced.to_nbt());
    if !rebuild_stats(&mut copy.custom_data, env)? { return Err("the tool could not be rebuilt".into()); }
    if let Some(error) = try_validate(&copy.custom_data, env) { return Ok(Err(error)); }
    tool_slot::sync_visuals(&mut copy, env)?;
    Ok(Ok(copy))
}

/// Reload listener, shaped like the other recipe listeners.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Salvages>>, String> {
    let state = Arc::new(RwLock::new(Salvages::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "modifier_salvage").unwrap(), FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "modifier salvage poisoned")?;
            match context {
                Ok(context) => {
                    *state = Salvages::load(files, &context);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { *state = Salvages { problems: vec![error.clone()], ..Salvages::default() }; Err(error) }
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("modifier salvage poisoned") = Salvages::default());
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
    fn salvage(extra: Value) -> (Identifier, Value) {
        let mut base = json!({"type":"tconstruct:modifier_salvage","modifier":"test:grow","slots":{"abilities":1},"tools":{"tag":"test:tools"}});
        base.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        (id("test:s"), base)
    }
    #[test]
    fn salvage_matches_by_modifier_level_and_tool_and_reports_bad_records() {
        let in_tag = |item: &Identifier, tag: &Identifier| tag.path() == "tools" && item.path() == "pick";
        let files = vec![salvage(json!({"level":{"min":2,"max":3}})), (id("test:t"), salvage(json!({"slots":{"upgrades":0}})).1),
            (id("test:off"), salvage(json!({"conditions":[false]})).1), (id("test:other"), json!({"type":"tconstruct:modifier"}))];
        let salvages = Salvages::load(&files, &Flags);
        assert_eq!((salvages.len(), salvages.problems.len()), (1, 1), "{:?}", salvages.problems);
        let (pick, grow) = (id("test:pick"), id("test:grow"));
        assert!(salvages.find(&pick, &grow, 2, &in_tag).is_some());
        assert!(salvages.find(&pick, &grow, 1, &in_tag).is_none(), "below the level range");
        assert!(salvages.find(&id("test:sword"), &grow, 2, &in_tag).is_none(), "the tool must qualify");
        assert!(salvages.find(&pick, &id("test:other"), 2, &in_tag).is_none());
    }
}
