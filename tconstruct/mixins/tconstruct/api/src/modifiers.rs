//! ModifierManager and composable rebuild contributions, from SlimeKnights (MIT; see LICENSE).
//! Unsupported behavior rejects a definition rather than registering an inert replacement.
use crate::{materials::Conditions, modifier_entries::Entry};
use pluto_lib::{Identifier, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::{collections::{BTreeMap, BTreeSet}, sync::{Arc, RwLock}};

pub(crate) const FOLDER: &str = "tinkering/modifiers";
pub(crate) const TAG_FOLDER: &str = "tinkering/tags/modifiers";
#[derive(Clone, Debug, PartialEq, Eq)]
enum VolatileModule {
    Flag { id: Identifier, min: i32, max: i32 },
    Rarity(i32),
}
#[derive(Clone, Debug)]
pub(crate) struct Modifier {
    pub id: Identifier,
    pub priority: i32,
    pub level_display: Identifier,
    pub tooltips: BTreeSet<String>,
    pub translation_key: Option<String>,
    volatile: Vec<VolatileModule>,
    /// Known original module types this definition declares that Tinkers has not ported. Their
    /// fields are not validated and they have no effect; callers can see exactly what is missing.
    pub unbound: Vec<Identifier>,
    empty_sentinel: bool,
}
fn id(value: &Value) -> Result<Identifier, String> {
    Identifier::parse(value.as_str().ok_or("expected identifier string")?).map_err(|e| e.to_string())
}
fn integer(value: &Value) -> Result<i32, String> {
    let Value::Number(value) = value else { return Err("expected numeric integer".into()); };
    Ok(crate::materials::number_int(&value.to_string()))
}
fn range(value: Option<&Value>) -> Result<(i32, i32), String> {
    let (min, max) = match value.filter(|v| !v.is_null()) {
        None => (1, 32767),
        Some(Value::Object(fields)) => (
            fields.get("min").map(integer).transpose()?.unwrap_or(1),
            fields.get("max").map(integer).transpose()?.unwrap_or(32767)),
        Some(value) => { let value = integer(value)?; (value, value) },
    };
    if min < 1 || max > 32767 || min > max { return Err("modifier level range must be within 1..32767".into()); }
    Ok((min, max))
}
fn tooltips(value: Option<&Value>) -> Result<BTreeSet<String>, String> {
    let values: Vec<&str> = match value.filter(|v| !v.is_null()) {
        None => vec!["tool", "tinker_station", "tool_part", "part_builder", "book"],
        Some(Value::String(name)) => match name.as_str() {
            "always" => vec!["tool", "tinker_station", "tool_part", "part_builder", "book"],
            "never" => vec![],
            "advanced" => vec!["tinker_station", "part_builder", "book"],
            "bonus_slot" => vec!["tinker_station", "tool_part", "part_builder", "book"],
            "parts_only" => vec!["tool_part", "part_builder", "book"],
            _ => return Err(format!("unsupported tooltip preset {name}")),
        },
        Some(Value::Array(values)) => values.iter().map(|value| value.as_str().ok_or("tooltip context must be a string"))
            .collect::<Result<_, _>>()?,
        _ => return Err("tooltip display must be a preset or context array".into()),
    };
    if values.iter().any(|value| !["tool", "tinker_station", "tool_part", "part_builder", "book"].contains(value)) {
        return Err("unknown tooltip context".into());
    }
    Ok(values.into_iter().map(str::to_owned).collect())
}
impl Modifier {
    pub fn decode(name: Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("modifier must be an object")?;
        let priority = fields.get("priority").filter(|v| !v.is_null()).map(integer).transpose()?.unwrap_or(i32::MIN);
        let display = match fields.get("level_display").filter(|v| !v.is_null()) {
            None => Identifier::new("tconstruct", "default").unwrap(),
            Some(Value::Object(fields)) => id(fields.get("type").ok_or("level display type missing")?)?,
            Some(value) => id(value)?,
        };
        // map_level and unique carry parameters that only the (unported) renderer reads.
        if display.namespace() != "tconstruct" || !["default", "single_level", "no_levels", "pluses", "map_level", "unique"].contains(&display.path()) {
            return Err(format!("unimplemented level display {display}"));
        }
        let deprecated_tooltips = match fields.get("tooltip_display") {
            Some(value) if !fields.contains_key("show_in_tooltips") => Some(Value::String(match value.as_str() {
                Some("always") => "always", Some("tinker_station") => "bonus_slot", Some("never") => "parts_only",
                _ => return Err("invalid deprecated tooltip display".into()),
            }.into())),
            _ => None,
        };
        let tooltips = tooltips(deprecated_tooltips.as_ref().or_else(|| fields.get("show_in_tooltips")))?;
        let translation_key = fields.get("translation_key").filter(|v| !v.is_null()).map(|value| {
            let text = match value { Value::String(s) => s.clone(), Value::Number(_) | Value::Bool(_) => value.to_string(),
                _ => return Err("translation key must be primitive".to_string()) };
            if text.encode_utf16().count() > 32767 { return Err("translation key too long".into()); }
            Ok(text)
        }).transpose()?;
        let mut volatile = Vec::new();
        let mut unbound: Vec<Identifier> = Vec::new();
        if let Some(modules) = fields.get("modules").filter(|v| !v.is_null()) {
            for module in modules.as_array().ok_or("modifier modules must be an array")? {
                let fields = module.as_object().ok_or("modifier module must be an object")?;
                let kind = id(fields.get("type").ok_or("modifier module type missing")?)?;
                let empty = kind.to_string() == "tconstruct:empty";
                let rarity = kind.to_string() == "tconstruct:rarity";
                let flag = kind.to_string() == "tconstruct:volatile_flag";
                // A module type the original never ships is a typo or a foreign module: reject it.
                if !crate::modifier_module_types::KNOWN.contains(&kind.to_string().as_str()) { return Err(format!("unknown modifier module {kind}")); }
                // Only volatile-data bindings are implemented. Anything else, and a volatile flag
                // gated on a tool predicate, is recorded as unbound instead of half-installed.
                let hooks = match fields.get("hooks").filter(|v| !v.is_null()) {
                    Some(hooks) => hooks.as_array().ok_or("hooks must be an array")?.iter().map(id).collect::<Result<Vec<_>, _>>()?,
                    None => Vec::new(),
                };
                let volatile_only = hooks.iter().all(|hook| hook.to_string() == "tconstruct:volatile_data");
                if empty && !hooks.is_empty() { return Err("unsupported modifier hook binding".into()); }
                let predicate = fields.get("tool").is_some_and(|v| !v.is_null());
                if !empty && !((rarity || flag) && volatile_only && !predicate) {
                    if !unbound.contains(&kind) { unbound.push(kind); }
                    continue;
                }
                if rarity {
                    // RarityModule is unconditional; unlike VolatileFlagModule its
                    // loader has no tool predicate or modifier-level condition.
                    let ordinal = match fields.get("rarity").and_then(Value::as_str) {
                        Some("common") => 0, Some("uncommon") => 1,
                        Some("rare") => 2, Some("epic") => 3,
                        _ => return Err("rarity must be common, uncommon, rare or epic".into()),
                    };
                    let module = VolatileModule::Rarity(ordinal);
                    if !volatile.contains(&module) { volatile.push(module); }
                } else if !empty {
                    let (min, max) = range(fields.get("modifier_level"))?;
                    let flag = VolatileModule::Flag { id: id(fields.get("flag").ok_or("flag missing")?)?, min, max };
                    if !volatile.contains(&flag) { volatile.push(flag); }
                }
            }
        }
        Ok(Self { id: name, priority: if priority == i32::MIN { 100 } else { priority },
            level_display: display, tooltips, translation_key, volatile, unbound, empty_sentinel: false })
    }
    pub fn apply_volatile(&self, entry: &Entry, data: &mut NbtCompound) {
        for module in &self.volatile {
            match module {
                VolatileModule::Flag { id, min, max } => {
                    if *min <= entry.level && entry.level <= *max { data.insert(id.to_string(), Nbt::Byte(1)); }
                }
                VolatileModule::Rarity(ordinal) => {
                    let current = crate::modifier_entries::nbt_int(data.get("tconstruct:rarity"));
                    if *ordinal > current { data.insert("tconstruct:rarity".into(), Nbt::Int(*ordinal)); }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Registry {
    static_modifiers: BTreeMap<Identifier, Arc<Modifier>>,
    expected: BTreeSet<Identifier>,
    dynamic: BTreeMap<Identifier, Arc<Modifier>>,
    pub tags: BTreeMap<Identifier, Vec<Identifier>>,
    pub reverse_tags: BTreeMap<Identifier, BTreeSet<Identifier>>,
    pub definitions_loaded: bool,
    pub generation: u64,
    pub problems: Vec<String>,
}
impl Default for Registry {
    fn default() -> Self {
        let id = Identifier::new("tconstruct", "empty").unwrap();
        let mut empty = Modifier::decode(id.clone(), &serde_json::json!({"show_in_tooltips":"never"})).unwrap();
        empty.empty_sentinel = true;
        Self { static_modifiers: [(id, Arc::new(empty))].into_iter().collect(), expected: BTreeSet::new(),
            dynamic: BTreeMap::new(), tags: BTreeMap::new(), reverse_tags: BTreeMap::new(),
            definitions_loaded: false, generation: 0, problems: Vec::new() }
    }
}
impl Registry {
    pub fn register_static(&mut self, modifier: Modifier) -> Result<(), String> {
        if self.expected.contains(&modifier.id) || self.static_modifiers.contains_key(&modifier.id) {
            return Err(format!("duplicate static/expected modifier {}", modifier.id));
        }
        self.static_modifiers.insert(modifier.id.clone(), Arc::new(modifier));
        Ok(())
    }
    pub fn expect_dynamic(&mut self, id: Identifier) -> Result<(), String> {
        if self.static_modifiers.contains_key(&id) { return Err(format!("modifier {id} is static")); }
        self.expected.insert(id); Ok(())
    }
    pub fn get(&self, id: &Identifier) -> Option<&Arc<Modifier>> {
        self.static_modifiers.get(id).or_else(|| self.dynamic.get(id))
    }
    /// Identifiers of every loaded dynamic modifier (including redirects), for audits.
    pub fn dynamic_ids(&self) -> impl Iterator<Item = &Identifier> { self.dynamic.keys() }
    pub fn contains_static(&self, id: &Identifier) -> bool { self.static_modifiers.contains_key(id) || self.expected.contains(id) }
    pub fn clear_dynamic(&mut self) {
        self.dynamic.clear(); self.tags.clear(); self.reverse_tags.clear();
        self.definitions_loaded = false; self.problems.clear();
        self.generation = self.generation.wrapping_add(1);
    }
    pub fn reload<C: Conditions>(&mut self, files: &[(Identifier, Value)], conditions: &C) {
        self.clear_dynamic();
        let mut redirects = BTreeMap::new();
        for (name, value) in files {
            let selected = (|| -> Result<Option<Identifier>, String> {
                let fields = value.as_object().ok_or("modifier must be an object")?;
                if let Some(values) = fields.get("redirects") {
                    // JsonHelper.parseList decodes the entire redirect list before testing any condition.
                    let redirects = values.as_array().ok_or("redirects must be an array")?.iter().map(|value| {
                        let fields = value.as_object().ok_or("redirect must be an object")?;
                        let target = id(fields.get("id").ok_or("redirect ID missing")?)?;
                        let condition = fields.get("condition").filter(|v| !v.is_null()).map(|value| conditions.decode(value)).transpose()?;
                        Ok((target, condition))
                    }).collect::<Result<Vec<_>, String>>()?;
                    for (target, condition) in redirects {
                        if condition.as_ref().map(|c| conditions.test(c)).transpose()?.unwrap_or(true) {
                            // Return through the selection below without decoding the unused modifier.
                            return Ok(Some(target));
                        }
                    }
                }
                if let Some(condition) = fields.get("condition") {
                    if !conditions.test(&conditions.decode(condition)?)? { return Ok(None); }
                }
                let modifier = Modifier::decode(name.clone(), value)?;
                self.dynamic.insert(name.clone(), Arc::new(modifier));
                Ok(None)
            })();
            match selected {
                Ok(Some(target)) => { redirects.insert(name.clone(), target); }
                Err(error) => self.problems.push(format!("{name}: {error}")),
                Ok(None) => {}
            }
        }
        // Resolve against concrete static/dynamic definitions before adding any redirects.
        let resolved: Vec<_> = redirects.into_iter().filter_map(|(from, to)| {
            match self.get(&to).cloned() {
                Some(modifier) => Some((from, modifier)),
                None => { self.problems.push(format!("invalid modifier redirect {from} -> {to}")); None }
            }
        }).collect();
        self.dynamic.extend(resolved);
        for id in &self.expected {
            if !self.dynamic.contains_key(id) { self.problems.push(format!("missing expected modifier {id}")); }
        }
        self.definitions_loaded = true;
    }
    pub fn load_tags(&mut self, stacks: &[pluto_lib::JsonResourceStack]) {
        let mut problems = Vec::new();
        self.tags = crate::materials::load_resource_tags(stacks, |id| {
            let modifier = self.get(id)?;
            if modifier.empty_sentinel && id.to_string() != "tconstruct:empty" { return None; }
            Some(modifier.id.clone())
        }, &mut problems, "modifier");
        self.problems.extend(problems);
        self.reverse_tags.clear();
        for (tag, members) in &self.tags {
            for member in members { self.reverse_tags.entry(member.clone()).or_default().insert(tag.clone()); }
        }
    }

    /// Resolve before merging. Optional missing references are omitted; required
    /// missing implementations error instead of being silently replaced by empty hooks.
    pub fn resolve(&self, entries: &[crate::material_traits::ModifierEntry]) -> Result<Vec<(Entry, Arc<Modifier>)>, String> {
        self.resolve_with_upgrades(&crate::modifier_entries::List::default(), entries)
    }
    pub fn resolve_with_upgrades(&self, upgrades: &crate::modifier_entries::List, traits: &[crate::material_traits::ModifierEntry]) -> Result<Vec<(Entry, Arc<Modifier>)>, String> {
        if !self.definitions_loaded { return Err("modifier definitions are not loaded".into()); }
        let mut result: Vec<(Entry, Arc<Modifier>)> = Vec::new();
        let inputs = upgrades.entries().iter().cloned().map(|entry| (entry, false)).chain(
            traits.iter().map(|entry| (Entry::new(entry.id.clone(), entry.level), entry.optional)));
        for (entry, optional) in inputs {
            let Some(modifier) = self.get(&entry.id) else {
                if optional { continue; }
                return Err(format!("required modifier {} unavailable", entry.id));
            };
            if modifier.empty_sentinel { continue; }
            if let Some((current, _)) = result.iter_mut().find(|(current, _)| current.id == entry.id) { *current = current.merge(&entry)?; }
            else { result.push((entry, modifier.clone())); }
        }
        // ModifierNBT.Builder sorts stably by negated Java int priority, not compareTo's path tie-breaker.
        result.sort_by_key(|(_, modifier)| modifier.priority.wrapping_neg());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct BooleanConditions;
    impl Conditions for BooleanConditions {
        type Parsed = bool;
        fn decode(&self, value: &Value) -> Result<bool, String> { value.as_bool().ok_or("condition must be boolean".into()) }
        fn test(&self, value: &bool) -> Result<bool, String> { Ok(*value) }
    }
    fn name(value: &str) -> Identifier { Identifier::parse(value).unwrap() }
    fn reference(value: &str, level: i32) -> crate::material_traits::ModifierEntry {
        crate::material_traits::ModifierEntry { id: name(value), level, optional: false }
    }
    #[test]
    fn static_priority_redirect_identity_and_generation_owned_resolution() {
        let mut registry = Registry::default();
        assert!(registry.resolve(&[]).is_err());
        registry.register_static(Modifier::decode(name("test:static"), &json!({"priority":150})).unwrap()).unwrap();
        assert!(registry.expect_dynamic(name("test:static")).is_err());
        registry.expect_dynamic(name("test:expected")).unwrap();
        assert!(registry.contains_static(&name("test:expected")));
        assert!(registry.get(&name("test:expected")).is_none(), "expectation is not an implemented modifier");
        assert!(registry.register_static(Modifier::decode(name("test:expected"), &json!({})).unwrap()).is_err());
        registry.reload(&[
            (name("test:static"), json!({"priority":-10})),
            (name("test:expected"), json!({"priority":200,"modules":[{"type":"tconstruct:volatile_flag","flag":"test:flag","modifier_level":{"min":2}}]})),
            (name("test:alias"), json!({"redirects":[{"id":"test:expected"}],"condition":false,"modules":"unused"})),
            (name("test:chain"), json!({"redirects":[{"id":"test:alias"}]})),
            (name("test:empty_alias"), json!({"redirects":[{"id":"tconstruct:empty"}]})),
            (name("test:invalid"), json!({"redirects":[{"id":"test:expected"},{"id":"test:static","condition":"bad"}]})),
        ], &BooleanConditions);
        assert_eq!(registry.get(&name("test:static")).unwrap().priority, 150);
        assert!(Arc::ptr_eq(registry.get(&name("test:alias")).unwrap(), registry.get(&name("test:expected")).unwrap()));
        assert!(registry.get(&name("test:chain")).is_none());
        assert!(registry.get(&name("test:invalid")).is_none(), "decode all redirects before selecting");
        let resolved = registry.resolve(&[reference("test:static",1), reference("test:alias",1), reference("test:expected",1), reference("test:alias",1)]).unwrap();
        assert_eq!(resolved.iter().map(|(entry,_)| (entry.id.to_string(),entry.level)).collect::<Vec<_>>(),
            vec![("test:alias".into(),2),("test:expected".into(),1),("test:static".into(),1)]);
        let mut data = NbtCompound::new();
        for (entry, modifier) in &resolved { modifier.apply_volatile(entry, &mut data); }
        assert_eq!(data["test:flag"], Nbt::Byte(1));
        registry.load_tags(&[
            pluto_lib::JsonResourceStack { id: name("test:members"), layers: vec![Ok(json!({"values":["test:alias","test:expected","tconstruct:empty"]}))] },
            pluto_lib::JsonResourceStack { id: name("test:invalid_empty"), layers: vec![Ok(json!({"values":["test:empty_alias"]}))] },
        ]);
        assert_eq!(registry.tags[&name("test:members")], vec![name("test:expected"), name("tconstruct:empty")]);
        assert!(!registry.tags.contains_key(&name("test:invalid_empty")), "only an explicit empty ID may reference the default object");
        assert!(registry.reverse_tags[&name("test:expected")].contains(&name("test:members")));
        assert!(!registry.reverse_tags.contains_key(&name("test:alias")), "reverse membership uses resolved modifier identity");
        registry.reload(&[], &BooleanConditions);
        assert!(registry.resolve(&[reference("test:expected",1)]).is_err());
        let mut optional = reference("test:expected",1); optional.optional = true;
        assert!(registry.resolve(&[optional]).unwrap().is_empty());
        assert_eq!(resolved[0].1.id, name("test:expected"), "owned resolved values survive a reload");
        registry.clear_dynamic();
        assert!(!registry.definitions_loaded);
        assert!(registry.get(&name("test:static")).is_some());
    }
    #[test]
    fn composable_flags_validate_full_definitions_and_level_ranges() {
        let modifier = Modifier::decode(name("test:flag"), &json!({"level_display":"tconstruct:no_levels","tooltip_display":"never",
            "modules":[{"type":"tconstruct:volatile_flag","flag":"test:flag","modifier_level":2}]})).unwrap();
        assert_eq!(modifier.level_display, name("tconstruct:no_levels"));
        assert!(!modifier.tooltips.contains("tool")); assert!(modifier.tooltips.contains("book"));
        let mut data = NbtCompound::new();
        modifier.apply_volatile(&Entry::new(name("test:flag"),1), &mut data);
        assert!(data.is_empty());
        modifier.apply_volatile(&Entry::new(name("test:flag"),2), &mut data);
        assert_eq!(data["test:flag"], Nbt::Byte(1));
        for invalid in [
            json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x"},{"type":"test:missing"}]}),
            json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x","modifier_level":0}]}),
            json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x","modifier_level":{"min":3,"max":2}}]}),
            json!({"modules":[{"type":"tconstruct:empty","hooks":["tconstruct:volatile_data"]}]}),
        ] { assert!(Modifier::decode(name("test:bad"), &invalid).is_err()); }
        // A predicate-gated flag or another hook binding is recorded as unbound, never installed.
        for gated in [
            json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x","tool":{}}]}),
            json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x","hooks":["tconstruct:projectile_launch"]}]}),
        ] {
            let modifier = Modifier::decode(name("test:gated"), &gated).unwrap();
            assert_eq!(modifier.unbound.iter().map(Identifier::to_string).collect::<Vec<_>>(), vec!["tconstruct:volatile_flag"]);
            let mut data = NbtCompound::new();
            modifier.apply_volatile(&Entry::new(name("test:gated"), 1), &mut data);
            assert!(data.is_empty(), "an unbound flag writes nothing");
        }
    }
    #[test]
    fn rarity_keeps_the_largest_ordinal_and_ignores_level_conditions() {
        let rare = Modifier::decode(name("test:rare"), &json!({"modules":[{"type":"tconstruct:rarity","rarity":"rare"}]})).unwrap();
        let common = Modifier::decode(name("test:common"), &json!({"modules":[{"type":"tconstruct:rarity","rarity":"common"}]})).unwrap();
        let mut data = NbtCompound::new();
        // Common never writes (ordinal 0 is not greater than the absent default 0).
        common.apply_volatile(&Entry::new(name("test:common"), 1), &mut data);
        assert!(data.is_empty());
        rare.apply_volatile(&Entry::new(name("test:rare"), 1), &mut data);
        assert_eq!(data["tconstruct:rarity"], Nbt::Int(2));
        let epic = Modifier::decode(name("test:epic"), &json!({"modules":[{"type":"tconstruct:rarity","rarity":"epic"}]})).unwrap();
        epic.apply_volatile(&Entry::new(name("test:epic"), 1), &mut data);
        common.apply_volatile(&Entry::new(name("test:common"), 1), &mut data);
        rare.apply_volatile(&Entry::new(name("test:rare"), 1), &mut data);
        assert_eq!(data["tconstruct:rarity"], Nbt::Int(3));
        for invalid in [json!({"type":"tconstruct:rarity"}), json!({"type":"tconstruct:rarity","rarity":"legendary"}),
            json!({"type":"tconstruct:rarity","rarity":"Rare"})] {
            assert!(Modifier::decode(name("test:bad"), &json!({"modules":[invalid]})).is_err());
        }
    }
}

pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Registry>>, String> {
    let state = Arc::new(RwLock::new(Registry::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "modifier_tags").unwrap(), TAG_FOLDER, |_| Ok(()),
    )?;
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "modifiers").unwrap(), FOLDER, move |files| {
            let context = context().and_then(|context| {
                let tags = ResourceManagerHelper::reload_json_stacks(TAG_FOLDER).ok_or("modifier tag folder was not captured")?;
                Ok((context, tags))
            });
            let mut state = output.write().map_err(|_| "modifier registry poisoned")?;
            match context {
                Ok((context, tags)) => {
                    state.reload(files, &context);
                    state.load_tags(&tags);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                },
                Err(error) => { state.clear_dynamic(); state.problems.push(error.clone()); Err(error) },
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| stopped.write().expect("modifier registry poisoned").clear_dynamic());
    Ok(state)
}
