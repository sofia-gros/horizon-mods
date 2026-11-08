//! `library.materials.stats`: the kinds of stats a material can have (`MaterialStatType`) and their loading from
//! `data/<namespace>/tinkering/materials/stats/<material>.json` (`MaterialStatsManager`).
use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde_json::{Map, Value};

use super::ids::{MaterialId, MaterialStatsId};
use crate::resource::ResourceId;

/// `MaterialStatsManager.FOLDER`.
pub const FOLDER: &str = "tinkering/materials/stats";

/// One set of stats of a material (`IMaterialStats`). The tools phase adds the real ones (head, handle, extra, ...).
pub trait MaterialStats: Any + Send + Sync {
    fn id(&self) -> &MaterialStatsId;
    fn as_any(&self) -> &dyn Any;
}

/// Reads the stats from the JSON object of a data file.
pub type StatsLoader = Arc<dyn Fn(&Map<String, Value>) -> Result<Arc<dyn MaterialStats>, String> + Send + Sync>;

/// `MaterialStatType`: the id, the stats a material has when its data names none, how to read them and whether tools of this kind repair.
#[derive(Clone)]
pub struct MaterialStatType {
    pub id: MaterialStatsId,
    pub default_stats: Arc<dyn MaterialStats>,
    pub loader: StatsLoader,
    pub can_repair: bool,
}

/// `MaterialStatType.singleton`: stats with no fields, such as the shield core.
pub fn singleton(stats: Arc<dyn MaterialStats>) -> MaterialStatType {
    let id = stats.id().clone();
    let given = stats.clone();
    MaterialStatType { id, default_stats: stats, loader: Arc::new(move |_| Ok(given.clone())), can_repair: false }
}

/// The registry of stat types and the stats loaded for every material.
#[derive(Default)]
pub struct MaterialStatsManager {
    types: BTreeMap<MaterialStatsId, MaterialStatType>,
    stats: HashMap<MaterialId, BTreeMap<MaterialStatsId, Arc<dyn MaterialStats>>>,
}

impl MaterialStatsManager {
    /// `registerStatType`; a second registration of the same id is refused.
    pub fn register(&mut self, kind: MaterialStatType) -> Result<(), String> {
        if self.types.contains_key(&kind.id) {
            return Err(format!("material stat type {} is registered twice", kind.id));
        }
        self.types.insert(kind.id.clone(), kind);
        Ok(())
    }

    pub fn stat_type(&self, id: &MaterialStatsId) -> Option<&MaterialStatType> {
        self.types.get(id)
    }

    pub fn stat_type_ids(&self) -> impl Iterator<Item = &MaterialStatsId> {
        self.types.keys()
    }

    /// `getStatsOrNull`.
    pub fn stats(&self, material: &MaterialId, stat: &MaterialStatsId) -> Option<&Arc<dyn MaterialStats>> {
        self.stats.get(material)?.get(stat)
    }

    /// `getStatsOrDefault`: the material's own stats, else the type's defaults, else nothing for an unregistered type.
    pub fn stats_or_default(&self, material: &MaterialId, stat: &MaterialStatsId) -> Option<Arc<dyn MaterialStats>> {
        self.stats(material, stat).cloned().or_else(|| self.types.get(stat).map(|t| t.default_stats.clone()))
    }

    /// `getAllStats`.
    pub fn all_stats(&self, material: &MaterialId) -> Vec<&Arc<dyn MaterialStats>> {
        self.stats.get(material).map_or_else(Vec::new, |m| m.values().collect())
    }

    /// `parse` and `finishLoad`: `files` are the stat files in load order (earlier packs first) as (material id, JSON); later files
    /// merge field by field over earlier ones, and a null stat removes it. The merged objects are read with the registered types;
    /// an unregistered type is skipped (quietly if the data says `optional`), a broken one is reported and skipped.
    pub fn load(&mut self, files: &[(ResourceId, Value)]) -> Vec<String> {
        let mut problems = Vec::new();
        let mut merged: BTreeMap<ResourceId, BTreeMap<ResourceId, Map<String, Value>>> = BTreeMap::new();
        for (material, json) in files {
            let Some(stats) = json.get("stats").and_then(Value::as_object) else { continue };
            let builder = merged.entry(material.clone()).or_default();
            for (key, value) in stats {
                let Some(key) = ResourceId::try_parse(key) else {
                    problems.push(format!("Bad stat type '{key}' on material {material}"));
                    continue;
                };
                match value {
                    Value::Null => {
                        builder.remove(&key);
                    }
                    Value::Object(object) => {
                        let existing = builder.entry(key).or_default();
                        for (field, field_value) in object {
                            existing.insert(field.clone(), field_value.clone());
                        }
                    }
                    _ => problems.push(format!("Stats {key} of material {material} must be an object")),
                }
            }
        }
        self.stats.clear();
        for (material, per_type) in merged {
            let mut loaded = BTreeMap::new();
            for (key, object) in per_type {
                let id = MaterialStatsId::from_resource(key);
                let Some(kind) = self.types.get(&id) else {
                    if !object.get("optional").and_then(Value::as_bool).unwrap_or(false) {
                        problems.push(format!("Skipping unregistered material stat type '{id}' for material '{material}'"));
                    }
                    continue;
                };
                match (kind.loader)(&object) {
                    Ok(stats) => {
                        loaded.insert(id, stats);
                    }
                    Err(error) => problems.push(format!("Failed to parse material stats {id} on material '{material}': {error}")),
                }
            }
            self.stats.insert(MaterialId::from_resource(material), loaded);
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Head {
        id: MaterialStatsId,
        durability: i64,
        mining_speed: f64,
    }

    impl MaterialStats for Head {
        fn id(&self) -> &MaterialStatsId {
            &self.id
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn head_id() -> MaterialStatsId {
        MaterialStatsId::try_parse("tconstruct:head").unwrap()
    }

    fn head_type() -> MaterialStatType {
        let id = head_id();
        let loader_id = id.clone();
        MaterialStatType {
            id: id.clone(),
            default_stats: Arc::new(Head { id: id.clone(), durability: 1, mining_speed: 1.0 }),
            loader: Arc::new(move |object| {
                let durability = object.get("durability").and_then(Value::as_i64).ok_or("durability is required")?;
                let mining_speed = object.get("mining_speed").and_then(Value::as_f64).unwrap_or(1.0);
                Ok(Arc::new(Head { id: loader_id.clone(), durability, mining_speed }) as Arc<dyn MaterialStats>)
            }),
            can_repair: true,
        }
    }

    fn rid(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    #[test]
    fn stats_load_and_later_files_merge_field_by_field() {
        let mut manager = MaterialStatsManager::default();
        manager.register(head_type()).unwrap();
        let problems = manager.load(&[
            (rid("tconstruct:wood"), json!({"stats": {"tconstruct:head": {"durability": 60, "mining_speed": 2.0}}})),
            (rid("tconstruct:wood"), json!({"stats": {"tconstruct:head": {"durability": 70}}})),
        ]);
        assert!(problems.is_empty(), "{problems:?}");
        let wood = MaterialId::try_parse("tconstruct:wood").unwrap();
        let stats = manager.stats(&wood, &head_id()).unwrap();
        let head = stats.as_any().downcast_ref::<Head>().unwrap();
        assert_eq!((head.durability, head.mining_speed), (70, 2.0));
    }

    #[test]
    fn a_null_removes_a_stat_and_the_default_fills_in() {
        let mut manager = MaterialStatsManager::default();
        manager.register(head_type()).unwrap();
        manager.load(&[
            (rid("tconstruct:wood"), json!({"stats": {"tconstruct:head": {"durability": 60}}})),
            (rid("tconstruct:wood"), json!({"stats": {"tconstruct:head": null}})),
        ]);
        let wood = MaterialId::try_parse("tconstruct:wood").unwrap();
        assert!(manager.stats(&wood, &head_id()).is_none());
        let fallback = manager.stats_or_default(&wood, &head_id()).unwrap();
        assert_eq!(fallback.as_any().downcast_ref::<Head>().unwrap().durability, 1);
        assert!(manager.stats_or_default(&wood, &MaterialStatsId::try_parse("tconstruct:unknown").unwrap()).is_none());
    }

    #[test]
    fn unregistered_and_broken_stats_are_skipped() {
        let mut manager = MaterialStatsManager::default();
        manager.register(head_type()).unwrap();
        let problems = manager.load(&[(
            rid("tconstruct:wood"),
            json!({"stats": {"tconstruct:head": {}, "tconstruct:grip": {}, "tconstruct:limb": {"optional": true}}}),
        )]);
        assert_eq!(problems.len(), 2, "the broken head and the unregistered grip, not the optional limb: {problems:?}");
        assert!(manager.all_stats(&MaterialId::try_parse("tconstruct:wood").unwrap()).is_empty());
    }

    #[test]
    fn a_stat_type_cannot_be_registered_twice() {
        let mut manager = MaterialStatsManager::default();
        manager.register(head_type()).unwrap();
        assert!(manager.register(head_type()).is_err());
    }
}
