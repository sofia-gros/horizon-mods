//! `library.materials.traits`: the modifiers ("traits") a material gives a tool part, by default and per kind of stats
//! (`MaterialTraits`, `MaterialTraitsManager`), loaded from `data/<namespace>/tinkering/materials/traits/<material>.json`.
use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use super::ids::{MaterialId, MaterialStatsId};
use crate::modifiers::ModifierEntry;
use crate::resource::ResourceId;

/// `MaterialTraitsManager.FOLDER`.
pub const FOLDER: &str = "tinkering/materials/traits";

fn entries(value: &Value) -> Result<Vec<ModifierEntry>, String> {
    value.as_array().ok_or("traits must be a list")?.iter().map(ModifierEntry::from_json).collect()
}

/// `MaterialTraits`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialTraits {
    pub default_traits: Vec<ModifierEntry>,
    per_stat: BTreeMap<MaterialStatsId, Vec<ModifierEntry>>,
}

impl MaterialTraits {
    /// `hasUniqueTraits`.
    pub fn has_unique_traits(&self, stat: &MaterialStatsId) -> bool {
        self.per_stat.contains_key(stat)
    }

    /// `getTraits`: those of the stat type, else the default ones.
    pub fn traits(&self, stat: &MaterialStatsId) -> &[ModifierEntry] {
        self.per_stat.get(stat).unwrap_or(&self.default_traits)
    }
}

#[derive(Default)]
struct Builder {
    default_traits: Option<Vec<ModifierEntry>>,
    per_stat: BTreeMap<MaterialStatsId, Vec<ModifierEntry>>,
}

/// The traits of every material, with the fallbacks between stat types.
#[derive(Debug, Default)]
pub struct MaterialTraitsManager {
    fallbacks: BTreeMap<MaterialStatsId, MaterialStatsId>,
    traits: HashMap<MaterialId, MaterialTraits>,
}

impl MaterialTraitsManager {
    /// `registerStatTypeFallback`: a stat type with no traits of its own uses another's (a second registration is refused).
    pub fn register_fallback(&mut self, stat: MaterialStatsId, fallback: MaterialStatsId) -> Result<(), String> {
        if self.fallbacks.contains_key(&stat) {
            return Err(format!("material stats type {stat} registered twice"));
        }
        self.fallbacks.insert(stat, fallback);
        Ok(())
    }

    pub fn default_traits(&self, material: &MaterialId) -> &[ModifierEntry] {
        self.traits.get(material).map_or(&[], |t| t.default_traits.as_slice())
    }

    pub fn has_unique_traits(&self, material: &MaterialId, stat: &MaterialStatsId) -> bool {
        self.traits.get(material).is_some_and(|t| t.has_unique_traits(stat))
    }

    pub fn traits(&self, material: &MaterialId, stat: &MaterialStatsId) -> &[ModifierEntry] {
        self.traits.get(material).map_or(&[], |t| t.traits(stat))
    }

    /// `parse` and `finishLoad`: `files` in load order as (material id, JSON). Default traits of a later file replace the earlier ones
    /// when it has some; a per-stat list replaces that stat's, and a null resets the stat to the default.
    pub fn load(&mut self, files: &[(ResourceId, Value)]) -> Vec<String> {
        let mut problems = Vec::new();
        let mut builders: BTreeMap<ResourceId, Builder> = BTreeMap::new();
        for (material, json) in files {
            let builder = builders.entry(material.clone()).or_default();
            if let Some(default) = json.get("default").filter(|v| !v.is_null()) {
                match entries(default) {
                    Ok(list) => builder.default_traits = Some(list),
                    Err(error) => {
                        problems.push(format!("Bad default traits on material {material}: {error}"));
                        continue;
                    }
                }
            }
            let Some(per_stat) = json.get("perStat").and_then(Value::as_object) else { continue };
            for (key, value) in per_stat {
                let Some(stat) = ResourceId::try_parse(key).map(MaterialStatsId::from_resource) else {
                    problems.push(format!("Bad stat type '{key}' in the traits of {material}"));
                    continue;
                };
                if value.is_null() {
                    builder.per_stat.remove(&stat);
                    continue;
                }
                match entries(value) {
                    Ok(list) => {
                        builder.per_stat.insert(stat, list);
                    }
                    Err(error) => problems.push(format!("Bad traits for {key} on material {material}: {error}")),
                }
            }
        }
        self.traits.clear();
        for (material, builder) in builders {
            let mut per_stat = builder.per_stat.clone();
            if !builder.per_stat.is_empty() {
                // fallbacks are added now so lookups stay a single hit
                for (stat, fallback) in &self.fallbacks {
                    if !builder.per_stat.contains_key(stat) {
                        if let Some(list) = builder.per_stat.get(fallback) {
                            per_stat.insert(stat.clone(), list.clone());
                        }
                    }
                }
            }
            self.traits.insert(MaterialId::from_resource(material), MaterialTraits { default_traits: builder.default_traits.unwrap_or_default(), per_stat });
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rid(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    fn stat(text: &str) -> MaterialStatsId {
        MaterialStatsId::try_parse(text).unwrap()
    }

    fn wood() -> MaterialId {
        MaterialId::try_parse("tconstruct:wood").unwrap()
    }

    #[test]
    fn traits_read_the_data_files_shape() {
        let mut manager = MaterialTraitsManager::default();
        let problems = manager.load(&[(
            rid("tconstruct:wood"),
            json!({"default": [{"level": 1, "name": "tconstruct:cultivated"}], "perStat": {"tconstruct:ammo": [{"level": 2, "name": "tconstruct:economical"}]}}),
        )]);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(manager.default_traits(&wood())[0].modifier.to_string(), "tconstruct:cultivated");
        let ammo = manager.traits(&wood(), &stat("tconstruct:ammo"));
        assert_eq!((ammo[0].modifier.to_string().as_str(), ammo[0].level), ("tconstruct:economical", 2));
        assert!(manager.has_unique_traits(&wood(), &stat("tconstruct:ammo")));
        assert!(!manager.has_unique_traits(&wood(), &stat("tconstruct:head")));
        assert_eq!(manager.traits(&wood(), &stat("tconstruct:head")), manager.default_traits(&wood()), "other stat types use the default");
    }

    #[test]
    fn a_fallback_stat_type_borrows_the_traits_of_another() {
        let mut manager = MaterialTraitsManager::default();
        manager.register_fallback(stat("tconstruct:grip"), stat("tconstruct:handle")).unwrap();
        assert!(manager.register_fallback(stat("tconstruct:grip"), stat("tconstruct:binding")).is_err());
        manager.load(&[(rid("tconstruct:wood"), json!({"perStat": {"tconstruct:handle": [{"name": "tconstruct:sturdy"}]}}))]);
        assert!(manager.has_unique_traits(&wood(), &stat("tconstruct:grip")));
        assert_eq!(manager.traits(&wood(), &stat("tconstruct:grip"))[0].modifier.to_string(), "tconstruct:sturdy");
    }

    #[test]
    fn a_null_resets_a_stat_and_a_later_default_replaces_the_earlier() {
        let mut manager = MaterialTraitsManager::default();
        manager.load(&[
            (rid("tconstruct:wood"), json!({"default": [{"name": "a:one"}], "perStat": {"tconstruct:ammo": [{"name": "a:two"}]}})),
            (rid("tconstruct:wood"), json!({"default": [{"name": "a:three"}], "perStat": {"tconstruct:ammo": null}})),
        ]);
        assert_eq!(manager.default_traits(&wood())[0].modifier.to_string(), "a:three");
        assert!(!manager.has_unique_traits(&wood(), &stat("tconstruct:ammo")));
    }

    #[test]
    fn broken_entries_are_reported() {
        let mut manager = MaterialTraitsManager::default();
        // a file with a broken default list is dropped whole, as the original's loader drops a file that throws
        let problems = manager.load(&[(rid("tconstruct:wood"), json!({"default": [{"level": 1}], "perStat": {"tconstruct:ammo": [{"name": "a:b"}]}}))]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(!manager.has_unique_traits(&wood(), &stat("tconstruct:ammo")));
        let problems = manager.load(&[(rid("tconstruct:wood"), json!({"perStat": {"tconstruct:ammo": [{"name": "a:b", "level": 0}]}}))]);
        assert_eq!(problems.len(), 1, "{problems:?}");
    }
}
