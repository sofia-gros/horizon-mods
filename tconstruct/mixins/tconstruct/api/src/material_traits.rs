//! MaterialTraitsManager/MaterialTraits, ported from SlimeKnights (MIT; see LICENSE).
//! Entries remain lazy modifier references: loading does not resolve modifier registration.
use pluto_lib::{Identifier, JsonResourceStack};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) const FOLDER: &str = "tinkering/materials/traits";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModifierEntry {
    pub id: Identifier,
    pub level: i32,
    pub optional: bool,
}
// Gson's list adapter retains null elements. Do not silently drop them or invent modifiers.
pub(crate) type TraitList = Vec<Option<ModifierEntry>>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MaterialTraits {
    pub default: TraitList,
    pub per_stat: BTreeMap<Identifier, TraitList>,
}
impl MaterialTraits {
    pub fn get(&self, stat: &Identifier) -> &[Option<ModifierEntry>] {
        self.per_stat.get(stat).unwrap_or(&self.default)
    }
    pub fn has_unique(&self, stat: &Identifier) -> bool { self.per_stat.contains_key(stat) }

    fn apply(&mut self, json: &Value) -> Result<(), String> {
        let object = json.as_object().ok_or("traits must be an object")?;
        // Gson decodes the complete document before mutating the builder.
        let default = list(object.get("default"))?;
        let mut per_stat = Vec::new();
        if let Some(value) = object.get("perStat").filter(|v| !v.is_null()) {
            for (name, value) in value.as_object().ok_or("perStat must be an object")? {
                per_stat.push((Identifier::parse(name).map_err(|e| e.to_string())?, list(Some(value))?));
            }
        }
        if let Some(default) = default { self.default = default; }
        for (stat, traits) in per_stat {
            if let Some(traits) = traits { self.per_stat.insert(stat, traits); }
            else { self.per_stat.remove(&stat); }
        }
        Ok(())
    }

    fn apply_fallbacks(&mut self, fallbacks: &BTreeMap<Identifier, Identifier>) {
        // Read only the explicit map: fallback chains must not become recursive.
        let additions: Vec<_> = fallbacks.iter().filter(|(stat, _)| !self.per_stat.contains_key(*stat))
            .filter_map(|(stat, category)| self.per_stat.get(category).map(|traits| (stat.clone(), traits.clone())))
            .collect();
        self.per_stat.extend(additions);
    }
}

fn list(value: Option<&Value>) -> Result<Option<TraitList>, String> {
    let Some(value) = value.filter(|v| !v.is_null()) else { return Ok(None); };
    value.as_array().ok_or("traits must be an array")?.iter().map(|entry| {
        if entry.is_null() { return Ok(None); }
        decode_entry(entry).map(Some)
    }).collect::<Result<TraitList, String>>().map(Some)
}

pub(crate) fn decode_entry(entry: &Value) -> Result<ModifierEntry, String> {
        let object = entry.as_object().ok_or("modifier entry must be an object")?;
        let name = object.get("name").ok_or("modifier entry needs name")?;
        // GsonHelper.convertToString accepts primitives, not arrays or objects.
        let name = match name {
            Value::String(s) => s.clone(), Value::Number(n) => n.to_string(), Value::Bool(b) => b.to_string(),
            _ => return Err("modifier name must be a primitive".into()),
        };
        let id = Identifier::parse(&name).map_err(|e| e.to_string())?;
        let level = match object.get("level").filter(|v| !v.is_null()) {
            None => 1,
            Some(Value::Number(n)) => super::materials::number_int(&n.to_string()),
            _ => return Err("modifier level must be a number".into()),
        };
        if level < 1 { return Err("modifier level must not be less than 1".into()); }
        let optional = match object.get("optional").filter(|v| !v.is_null()) {
            None => false, Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => s.eq_ignore_ascii_case("true"),
            Some(Value::Number(_)) => false,
            _ => return Err("optional must be a primitive".into()),
        };
        Ok(ModifierEntry { id, level, optional })
}

pub(crate) fn fallbacks() -> BTreeMap<Identifier, Identifier> {
    // MaterialRegistry constructor; ribcage and repair_kit deliberately have no fallback.
    [
        ("melee_harvest", &["head", "handle", "binding"][..]),
        ("ranged", &["limb", "grip", "bowstring"][..]),
        ("armor", &["plating_helmet", "plating_chestplate", "plating_leggings", "plating_boots", "plating_shield",
            "cuirass", "maille", "shield_core", "slime", "skull", "shell", "laces"][..]),
        ("ammo", &["arrow_head", "arrow_shaft", "fletching"][..]),
    ].into_iter().flat_map(|(category, stats)| stats.iter().map(move |stat|
        (Identifier::new("tconstruct", *stat).expect("constant stat ID"),
         Identifier::new("tconstruct", category).expect("constant category ID")))).collect()
}

pub(crate) fn load(stacks: &[JsonResourceStack], problems: &mut Vec<String>) -> BTreeMap<Identifier, MaterialTraits> {
    let mut loaded = BTreeMap::<Identifier, MaterialTraits>::new();
    for stack in stacks {
        for (layer, value) in stack.layers.iter().enumerate() {
            let result = match value {
                Ok(value) if !value.is_null() => loaded.entry(stack.id.clone()).or_default().apply(value),
                Ok(_) => Err("null or empty traits document".into()),
                Err(error) => Err(error.clone()),
            };
            if let Err(error) = result {
                problems.push(format!("Could not read material traits {} layer {layer}: {error}", stack.id));
            }
        }
    }
    let fallbacks = fallbacks();
    for traits in loaded.values_mut() { traits.apply_fallbacks(&fallbacks); }
    loaded
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn id(name: &str) -> Identifier { Identifier::parse(name).unwrap() }
    #[test]
    fn layered_replacement_null_reset_and_invalid_layer_atomicity() {
        let mut errors = Vec::new();
        let loaded = load(&[JsonResourceStack { id: id("test:wood"), layers: vec![
            Ok(json!({"default":[{"name":"test:first"}], "perStat":{
                "tconstruct:melee_harvest":[{"name":"test:category","level":2}],
                "tconstruct:head":[], "tconstruct:handle":[{"name":"test:explicit"}]}})),
            Ok(json!({"default":[{"name":"test:replacement","optional":true}], "perStat":{"tconstruct:handle":null}})),
            Ok(json!({"default":[], "perStat":{"tconstruct:head":[{"name":"test:bad","level":0}]}})),
            Ok(json!({"default":null})),
        ]}], &mut errors);
        let traits = &loaded[&id("test:wood")];
        assert_eq!(errors.len(), 1);
        assert_eq!(traits.default[0].as_ref().unwrap().id, id("test:replacement"));
        assert!(traits.default[0].as_ref().unwrap().optional);
        assert!(traits.get(&id("tconstruct:head")).is_empty());
        assert!(traits.has_unique(&id("tconstruct:head")));
        assert_eq!(traits.get(&id("tconstruct:handle"))[0].as_ref().unwrap().level, 2);
        assert_eq!(traits.get(&id("tconstruct:ribcage")), traits.default);
        assert!(!traits.has_unique(&id("tconstruct:ribcage")));
        assert!(load(&[], &mut errors).is_empty());
    }
    #[test]
    fn fallbacks_are_one_hop_and_entries_remain_lazy() {
        let mut traits = MaterialTraits::default();
        traits.apply(&json!({"default":[null,{"name":"test:unregistered","level":1.9}],
            "perStat":{"test:c":[{"name":"test:category"}]}})).unwrap();
        traits.apply_fallbacks(&[(id("test:a"),id("test:b")),(id("test:b"),id("test:c"))].into_iter().collect());
        assert!(!traits.has_unique(&id("test:a")));
        assert!(traits.has_unique(&id("test:b")));
        assert!(traits.default[0].is_none());
        assert_eq!(traits.default[1].as_ref().unwrap().level, 1);
        assert!(traits.apply(&json!({"default":[{"name":"test:x","level":"2"}]})).is_err());
    }
}
