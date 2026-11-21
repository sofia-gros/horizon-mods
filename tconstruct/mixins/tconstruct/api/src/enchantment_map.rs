//! The enchantment-to-modifier map: `tinkering/enchantments_to_modifiers.json`, loaded with the
//! modifier definitions as ModifierManager loads it in the original (its apply pass reads the map
//! after the modifiers and their tags). A tool or book carrying a vanilla enchantment converts to
//! the mapped modifier through the enchantment converting worktable recipe
//! (`enchantment_converting`); the map is also what `hasEnchantment` answers for recipe lookups.
//! SlimeKnights (MIT; see LICENSE) defines the semantics; this only mirrors them.
//!
//! Two Java checks have no host counterpart here and follow the damage tag loader's precedent:
//! enchantments are not validated against a registry (PlutoLib captures no enchantment registry),
//! so a named enchantment is taken as existing, and tag membership is resolved from the shipped
//! `tconstruct:modifier_like/*` tag files themselves. The shipped tags name only other mods'
//! enchantments (every entry `required: false`), so in this game they resolve empty; the files are
//! kept byte-identical so a datapack defining one of those enchantments makes the mapping live.
use crate::materials::load_resource_tags;
use pluto_lib::{Identifier, JsonResourceStack};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The folder holding the map file (the tinkering root), declared so the reload captures it.
pub(crate) const FOLDER: &str = "tinkering";
/// The map file's name within `FOLDER`.
pub(crate) const FILE: &str = "enchantments_to_modifiers";
/// The enchantment tag folder the map's `#` keys name into, relative to `data/tconstruct/`.
pub(crate) const TAG_FOLDER: &str = "tags/enchantment/modifier_like";
/// Tag ids are `tconstruct:modifier_like/<file>`: the folder sits one level below the tag path.
const TAG_PREFIX: &str = "modifier_like/";

/// ModifierManager's enchantment map: direct enchantment-to-modifier entries, plus the
/// enchantment-tag entries in file order (a tag entry matches every enchantment in its tag).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct EnchantmentMap {
    direct: BTreeMap<Identifier, Identifier>,
    tag_entries: Vec<(Identifier, Identifier)>,
    tag_members: BTreeMap<Identifier, Vec<Identifier>>,
    /// Entries the original would log as invalid (unknown non-optional modifier or tag,
    /// malformed key or value), in file order.
    pub problems: Vec<String>,
}

/// An ID as the original parses it: a bare path takes the default namespace
/// (`tconstruct` for a ModifierId, `minecraft` for an enchantment or a tag).
fn parse_id(text: &str, default_namespace: &str) -> Result<Identifier, String> {
    if text.contains(':') { Identifier::parse(text).map_err(|error| error.to_string()) }
    else { Identifier::new(default_namespace, text).map_err(|error| error.to_string()) }
}

impl EnchantmentMap {
    /// Loads the map from its resource stack (the top layer wins, as for any data file) and the
    /// `modifier_like` tag stacks. `known` answers modifier existence, as ModifierManager checks
    /// the just-loaded registry: an entry naming an unknown modifier is dropped, silently when
    /// the value carries the `?` optional suffix, with a problem otherwise; the same applies to
    /// a `#` key naming a tag no captured file defines and to a malformed entry.
    pub(crate) fn load(stack: Option<&JsonResourceStack>, tag_stacks: &[JsonResourceStack],
                       known: &dyn Fn(&Identifier) -> bool) -> Self {
        let mut problems = Vec::new();
        // The stacks are captured by file name; the tag each file defines is modifier_like/<name>.
        let rewritten: Vec<JsonResourceStack> = tag_stacks.iter().filter_map(|stack| {
            Some(JsonResourceStack {
                id: Identifier::new(stack.id.namespace(), &format!("{TAG_PREFIX}{}", stack.id.path())).ok()?,
                layers: stack.layers.clone(),
            })
        }).collect();
        let captured: BTreeSet<Identifier> = rewritten.iter().map(|stack| stack.id.clone()).collect();
        let tag_members = load_resource_tags(&rewritten, |id| Some(id.clone()), &mut problems, "enchantment");
        let mut loaded = Self { tag_members, ..Self::default() };
        let Some(stack) = stack else { loaded.problems = problems; return loaded; };
        let value = match stack.layers.last() {
            Some(Ok(value)) => value,
            Some(Err(error)) => { problems.push(format!("{FILE}: {error}")); loaded.problems = problems; return loaded; }
            None => { loaded.problems = problems; return loaded; }
        };
        let Some(object) = value.as_object() else {
            problems.push(format!("{FILE}: the enchantment map must be a JSON object"));
            loaded.problems = problems;
            return loaded;
        };
        for (key, value) in object {
            let entry = (|| -> Result<(), String> {
                // A trailing '?' on either side marks that side optional.
                let name = key.strip_suffix('?').unwrap_or(key);
                let optional_enchantment = name.len() != key.len();
                let text = value.as_str().ok_or("the modifier must be an ID string")?;
                let modifier_text = text.strip_suffix('?').unwrap_or(text);
                let optional_modifier = modifier_text.len() != text.len();
                let modifier = parse_id(modifier_text, "tconstruct")?;
                if let Some(tag_name) = name.strip_prefix('#') {
                    let tag = parse_id(tag_name, "minecraft")?;
                    if !captured.contains(&tag) {
                        if optional_enchantment { return Ok(()); }
                        return Err(format!("unknown enchantment tag #{tag}"));
                    }
                    if !known(&modifier) {
                        if optional_modifier { return Ok(()); }
                        return Err(format!("unknown modifier {modifier}"));
                    }
                    loaded.tag_entries.push((tag, modifier));
                } else {
                    let enchantment = parse_id(name, "minecraft")?;
                    if !known(&modifier) {
                        if optional_modifier { return Ok(()); }
                        return Err(format!("unknown modifier {modifier}"));
                    }
                    loaded.direct.insert(enchantment, modifier);
                }
                Ok(())
            })();
            if let Err(error) = entry { problems.push(format!("{FILE}: invalid mapping {key}: {error}")); }
        }
        loaded.problems = problems;
        loaded
    }

    /// ModifierManager.get(enchantment): the direct mapping first, then the first tag entry
    /// whose tag holds the enchantment, in file order. None when nothing maps it.
    pub(crate) fn modifier_for(&self, enchantment: &Identifier) -> Option<&Identifier> {
        if let Some(modifier) = self.direct.get(enchantment) { return Some(modifier); }
        self.tag_entries.iter()
            .find(|(tag, _)| self.tag_members.get(tag).is_some_and(|members| members.contains(enchantment)))
            .map(|(_, modifier)| modifier)
    }

    /// ModifierManager.hasEnchantment: the modifier is the target of any mapping.
    pub(crate) fn has_enchantment(&self, modifier: &Identifier) -> bool {
        self.direct.values().any(|target| target == modifier)
            || self.tag_entries.iter().any(|(_, target)| target == modifier)
    }

    /// ModifierManager.getEquivalentEnchantments: the candidates mapping to an accepted
    /// modifier. The original streams the whole enchantment registry; this port keeps no
    /// enchantment registry, so the caller names the candidates (the enchantments a stack
    /// carries, or every mapped one).
    pub(crate) fn equivalent_enchantments(&self, accept: &dyn Fn(&Identifier) -> bool,
                                          candidates: &[Identifier]) -> Vec<Identifier> {
        candidates.iter().filter(|candidate| self.modifier_for(candidate).is_some_and(|modifier| accept(modifier))).cloned().collect()
    }

    /// Every enchantment the map can resolve, direct keys first: the candidate set for
    /// `equivalent_enchantments` when a caller has no stack to draw from.
    pub(crate) fn mapped_enchantments(&self) -> Vec<Identifier> {
        let mut all: Vec<Identifier> = self.direct.keys().cloned().collect();
        for (tag, _) in &self.tag_entries {
            if let Some(members) = self.tag_members.get(tag) {
                for member in members { if !all.contains(member) { all.push(member.clone()); } }
            }
        }
        all
    }

    /// The loaded entry counts, direct and tag: the shipped file holds 36 and 21.
    pub(crate) fn counts(&self) -> (usize, usize) { (self.direct.len(), self.tag_entries.len()) }

    pub(crate) fn is_empty(&self) -> bool { self.direct.is_empty() && self.tag_entries.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stack(id: &str, value: Value) -> JsonResourceStack {
        JsonResourceStack { id: Identifier::parse(id).unwrap(), layers: vec![Ok(value)] }
    }
    fn known(ids: &[&str]) -> impl Fn(&Identifier) -> bool {
        let ids: Vec<Identifier> = ids.iter().map(|id| Identifier::parse(id).unwrap()).collect();
        move |id: &Identifier| ids.contains(id)
    }
    fn load(value: Value, tags: &[JsonResourceStack]) -> EnchantmentMap {
        EnchantmentMap::load(Some(&stack("tconstruct:enchantments_to_modifiers", value)), tags,
                             &known(&["tconstruct:haste", "tconstruct:luck", "tconstruct:fiery"]))
    }

    #[test]
    fn direct_entries_win_over_tags_and_tags_match_in_file_order() {
        let tags = vec![
            stack("tconstruct:luck", json!({"values": ["minecraft:fortune", "test:from_tag"]})),
            stack("tconstruct:fiery", json!({"values": ["test:other_tag"]})),
        ];
        let map = load(json!({
            "minecraft:efficiency": "tconstruct:haste",
            "minecraft:fortune": "tconstruct:luck",
            "#tconstruct:modifier_like/luck": "tconstruct:luck",
            "#tconstruct:modifier_like/fiery": "tconstruct:fiery",
        }), &tags);
        assert!(map.problems.is_empty(), "{:?}", map.problems);
        let id = |text: &str| Identifier::parse(text).unwrap();
        assert_eq!(map.modifier_for(&id("minecraft:efficiency")), Some(&id("tconstruct:haste")));
        // Fortune is in the luck tag too, but its direct entry is consulted first.
        assert_eq!(map.modifier_for(&id("minecraft:fortune")), Some(&id("tconstruct:luck")));
        // A tag-only enchantment resolves through its tag's entry.
        assert_eq!(map.modifier_for(&id("test:from_tag")), Some(&id("tconstruct:luck")));
        assert_eq!(map.modifier_for(&id("test:other_tag")), Some(&id("tconstruct:fiery")));
        assert_eq!(map.modifier_for(&id("minecraft:sharpness")), None);
        assert!(map.has_enchantment(&id("tconstruct:fiery")));
        assert!(map.has_enchantment(&id("tconstruct:haste")));
        assert!(!map.has_enchantment(&id("tconstruct:overslime")));
        assert_eq!(map.equivalent_enchantments(&|m| *m == id("tconstruct:luck"), &[id("minecraft:fortune"), id("test:from_tag"), id("minecraft:efficiency")]),
                   vec![id("minecraft:fortune"), id("test:from_tag")]);
        assert_eq!(map.counts(), (2, 2));
    }

    #[test]
    fn unknown_and_malformed_entries_are_dropped_as_the_original_logs_them() {
        let map = load(json!({
            "minecraft:efficiency": "tconstruct:haste",
            "minecraft:power": "tconstruct:missing",
            "minecraft:punch?": "tconstruct:also_missing?",
            "#tconstruct:modifier_like/absent": "tconstruct:luck",
            "#tconstruct:modifier_like/absent_too?": "tconstruct:luck",
            "minecraft:flame": 3,
            "not an id": "tconstruct:luck",
        }), &[]);
        // Only the valid entry survives; the two silent optionals leave no problem.
        assert_eq!(map.counts(), (1, 0));
        assert_eq!(map.problems.len(), 4, "{:?}", map.problems);
        assert!(map.problems.iter().any(|p| p.contains("unknown modifier tconstruct:missing")));
        assert!(map.problems.iter().any(|p| p.contains("unknown enchantment tag")));
        assert!(map.problems.iter().any(|p| p.contains("must be an ID string")));
    }

    #[test]
    fn a_missing_file_is_an_empty_map_and_bare_paths_take_the_default_namespaces() {
        let empty = EnchantmentMap::load(None, &[], &known(&[]));
        assert!(empty.is_empty() && empty.problems.is_empty());
        let map = load(json!({ "efficiency": "haste" }), &[]);
        assert_eq!(map.modifier_for(&Identifier::parse("minecraft:efficiency").unwrap()),
                   Some(&Identifier::parse("tconstruct:haste").unwrap()));
    }
}
