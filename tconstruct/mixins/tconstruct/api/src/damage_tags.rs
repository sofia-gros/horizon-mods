//! Damage-type tags: the Tinkers tags (`tinkering/tags/damage_types`, which the protection modules and
//! `tconstruct:is_melee` read) over the vanilla tags the game version defines. The host captures no damage-type
//! tags, so the mod reads its own folder and takes the vanilla ones from the generated `damage_vanilla` table.
use crate::{damage_vanilla, materials::load_resource_tags};
use pluto_lib::{Identifier, JsonResourceStack};
use serde_json::json;
use std::collections::BTreeMap;

pub(crate) const TAG_FOLDER: &str = "tinkering/tags/damage_types";

/// Tag id to the damage types it holds.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DamageTags { members: BTreeMap<Identifier, Vec<Identifier>> }

impl DamageTags {
    /// Resolves the Tinkers tag files with the vanilla tags they reference; a tag naming an unknown required tag
    /// is dropped with a problem, as in any tag load.
    pub fn load(stacks: &[JsonResourceStack]) -> (Self, Vec<String>) {
        let mut all: Vec<JsonResourceStack> = damage_vanilla::TAGS.iter().filter_map(|(tag, members)| {
            Some(JsonResourceStack { id: Identifier::new("minecraft", tag).ok()?, layers: vec![Ok(json!({ "values": members }))] })
        }).collect();
        all.extend(stacks.iter().cloned());
        let mut problems = Vec::new();
        // A damage type is not validated against a registry here: a named type is a member.
        let members = load_resource_tags(&all, |id| Some(id.clone()), &mut problems, "damage type");
        (Self { members }, problems)
    }

    /// Whether `damage_type` is in `tag`.
    pub fn contains(&self, tag: &Identifier, damage_type: &Identifier) -> bool {
        self.members.get(tag).is_some_and(|members| members.contains(damage_type))
    }
}
