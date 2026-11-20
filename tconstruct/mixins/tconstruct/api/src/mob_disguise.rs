//! MobDisguiseModule, from SlimeKnights (MIT; see LICENSE): armor that makes its wearer look like another kind of mob from afar. Each worn
//! piece adds its modifier level to the wearer's count for the mob type it names, and a mob of that type sees the wearer from half as far
//! for each level in the count (`ToolEvents.livingVisibility`: `modifyVisibility(1 / 2^count)`). It never stops a mob that already targets
//! the wearer; only the range it notices them from shrinks.
use crate::modifier_entries::Entry;
use pluto_lib::Identifier;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MobDisguise { entity: Identifier }

impl MobDisguise {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let entity = fields.get("entity").and_then(Value::as_str).ok_or("entity missing")?;
        Ok(Self { entity: Identifier::parse(entity).map_err(|e| e.to_string())? })
    }

    /// The mob type this piece disguises its wearer as, with the count it adds: the modifier's level.
    pub(crate) fn disguise(&self, entry: &Entry) -> (Identifier, i32) { (self.entity.clone(), entry.level) }
}

/// The wearer's count for each mob type, from the disguises of every worn piece (the multiset of `MobDisguiseModule.DISGUISES`).
pub(crate) fn count(disguises: impl IntoIterator<Item = (Identifier, i32)>) -> BTreeMap<String, i32> {
    let mut counts = BTreeMap::new();
    for (entity, level) in disguises { let total = counts.entry(entity.to_string()).or_insert(0_i32); *total = total.saturating_add(level); }
    counts
}

/// The factor on how far a mob of type `kind` sees the wearer: 1 without a disguise for it, then half for each level counted.
pub(crate) fn visibility_factor(counts: &BTreeMap<String, i32>, kind: &str) -> f64 {
    match counts.get(kind) {
        Some(&count) if count > 0 => 1.0 / 2.0_f64.powi(count),
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    #[test]
    fn a_disguise_names_its_mob_and_adds_the_modifier_level() {
        let spider = MobDisguise::decode(json!({"entity":"minecraft:spider"}).as_object().unwrap()).unwrap();
        assert_eq!(spider.disguise(&Entry::new(id("tconstruct:spider_disguise"), 2)), (id("minecraft:spider"), 2));
        assert!(MobDisguise::decode(json!({}).as_object().unwrap()).is_err(), "the entity is required");
        assert!(MobDisguise::decode(json!({"entity":"not an id"}).as_object().unwrap()).is_err());
    }

    #[test]
    fn each_counted_level_halves_the_range_only_for_that_mob() {
        // Two worn pieces for skeletons (levels 1 and 2) and one for zombies: the counts add per type.
        let counts = count([(id("minecraft:skeleton"), 1), (id("minecraft:zombie"), 1), (id("minecraft:skeleton"), 2)]);
        assert_eq!(visibility_factor(&counts, "minecraft:skeleton"), 0.125);
        assert_eq!(visibility_factor(&counts, "minecraft:zombie"), 0.5);
        assert_eq!(visibility_factor(&counts, "minecraft:creeper"), 1.0, "a mob with no disguise sees as usual");
        assert_eq!(visibility_factor(&count([]), "minecraft:skeleton"), 1.0);
        let huge = count([(id("minecraft:skeleton"), i32::MAX), (id("minecraft:skeleton"), 5)]);
        assert_eq!(visibility_factor(&huge, "minecraft:skeleton"), 0.0, "the count saturates instead of wrapping around to a negative");
    }
}
