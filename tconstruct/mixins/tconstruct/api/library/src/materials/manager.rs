//! `library.materials.definition.MaterialManager`: the materials of `data/<namespace>/tinkering/materials/definition/*.json`, the
//! redirects among them and the material tags.
use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use super::ids::MaterialId;
use super::material::{Material, Rarity};
use crate::condition::{self, ConditionContext};
use crate::resource::ResourceId;

/// `MaterialManager.FOLDER`.
pub const FOLDER: &str = "tinkering/materials/definition";
/// `MaterialManager.TAG_FOLDER`.
pub const TAG_FOLDER: &str = "tinkering/tags/materials";

/// The loaded materials, in the original's three maps.
#[derive(Debug, Default)]
pub struct MaterialManager {
    materials: HashMap<MaterialId, Material>,
    redirects: HashMap<MaterialId, MaterialId>,
    sorted: Vec<MaterialId>,
    tags: BTreeMap<ResourceId, Vec<MaterialId>>,
}

/// What loading found wrong; the original logs each and skips the file.
pub type Problems = Vec<String>;

impl MaterialManager {
    /// `apply`: `files` are the definition files as (id, JSON), the id being `namespace:name` of the file; `tag_files` the tag files.
    pub fn load(files: &[(ResourceId, Value)], tag_files: &[(ResourceId, Value)], context: &dyn ConditionContext) -> (Self, Problems) {
        let mut problems = Problems::new();
        let mut materials = HashMap::new();
        let mut redirects = HashMap::new();
        for (id, json) in files {
            if !json.is_object() {
                continue;
            }
            match load_material(id, json, &mut redirects, context) {
                Ok(Some(material)) => {
                    materials.insert(material.id.clone(), material);
                }
                Ok(None) => {}
                Err(error) => problems.push(format!("Could not deserialize material {id}: {error}")),
            }
        }
        // a redirect to a material that does not exist is dropped
        redirects.retain(|from: &MaterialId, to: &mut MaterialId| {
            let keep = materials.contains_key(to);
            if !keep {
                problems.push(format!("Invalid material redirect {from} as material {to} does not exist"));
            }
            keep
        });
        let mut manager = Self { materials, redirects, sorted: Vec::new(), tags: BTreeMap::new() };
        manager.resort();
        let (tags, tag_problems) = crate::tags::load(tag_files, &|id| manager.materials.contains_key(&MaterialId::from_resource(id.clone())));
        problems.extend(tag_problems);
        manager.tags = tags.into_iter().map(|(tag, members)| (tag, members.into_iter().map(MaterialId::from_resource).collect())).collect();
        (manager, problems)
    }

    fn resort(&mut self) {
        let mut visible: Vec<&Material> = self.materials.values().filter(|m| !m.hidden).collect();
        visible.sort();
        self.sorted = visible.into_iter().map(|m| m.id.clone()).collect();
    }

    /// `getVisibleMaterials`: not hidden, sorted.
    pub fn visible_materials(&self) -> Vec<&Material> {
        self.sorted.iter().filter_map(|id| self.materials.get(id)).collect()
    }

    pub fn all_materials(&self) -> impl Iterator<Item = &Material> {
        self.materials.values()
    }

    pub fn material(&self, id: &MaterialId) -> Option<&Material> {
        self.materials.get(id)
    }

    /// `resolveRedirect`: the material a redirect points to, or the id itself.
    pub fn resolve_redirect<'a>(&'a self, id: &'a MaterialId) -> &'a MaterialId {
        self.redirects.get(id).unwrap_or(id)
    }

    /// `getValues`: the members of a material tag, empty for an unknown tag.
    pub fn tag(&self, tag: &ResourceId) -> &[MaterialId] {
        self.tags.get(tag).map_or(&[], Vec::as_slice)
    }

    /// `isIn`.
    pub fn is_in(&self, id: &MaterialId, tag: &ResourceId) -> bool {
        self.tag(tag).contains(id)
    }

    /// `getTagKeys`: the tags a material is in.
    pub fn tags_of(&self, id: &MaterialId) -> Vec<&ResourceId> {
        self.tags.iter().filter(|(_, members)| members.contains(id)).map(|(tag, _)| tag).collect()
    }
}

fn load_material(file: &ResourceId, json: &Value, redirects: &mut HashMap<MaterialId, MaterialId>, context: &dyn ConditionContext) -> Result<Option<Material>, String> {
    let id = MaterialId::from_resource(file.clone());
    // a redirect that holds ends the loading of this file; it is read first so a material can redirect and fall back to a conditional one
    if let Some(list) = json.get("redirect") {
        for redirect in list.as_array().ok_or("'redirect' must be a list")? {
            let holds = match redirect.get("condition") {
                Some(condition) => condition::test(condition, context)?,
                None => true,
            };
            if holds {
                let target = redirect.get("id").and_then(Value::as_str).and_then(ResourceId::try_parse).ok_or("a redirect needs an 'id'")?;
                redirects.insert(id, MaterialId::from_resource(target));
                return Ok(None);
            }
        }
    }
    if let Some(condition) = json.get("condition") {
        if !condition::test(condition, context)? {
            return Ok(None);
        }
    }
    let craftable = match json.get("craftable") {
        Some(value) => value.as_bool().ok_or("'craftable' must be true or false")?,
        None => return Err("a material needs crafting information ('craftable')".to_owned()),
    };
    let hidden = json.get("hidden").and_then(Value::as_bool).unwrap_or(false);
    let tier = json.get("tier").and_then(Value::as_i64).unwrap_or(0) as i32;
    let rarity = match json.get("rarity") {
        Some(value) => value.as_str().and_then(Rarity::from_name).ok_or("'rarity' is not a rarity")?,
        None => Rarity::for_tier(tier),
    };
    let sort_order = json.get("sortOrder").and_then(Value::as_i64).unwrap_or(100) as i32;
    Ok(Some(Material { id, tier, sort_order, rarity, craftable, hidden }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::EmptyContext;
    use serde_json::json;

    fn id(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    fn load(files: Vec<(&str, Value)>, tags: Vec<(&str, Value)>) -> (MaterialManager, Problems) {
        let files: Vec<_> = files.into_iter().map(|(n, v)| (id(n), v)).collect();
        let tags: Vec<_> = tags.into_iter().map(|(n, v)| (id(n), v)).collect();
        MaterialManager::load(&files, &tags, &EmptyContext)
    }

    #[test]
    fn a_material_loads_with_the_originals_defaults() {
        let (manager, problems) = load(vec![("tconstruct:wood", json!({"craftable": true, "hidden": false, "sortOrder": 0, "tier": 0})), ("tconstruct:netherite", json!({"craftable": false, "tier": 5}))], vec![]);
        assert!(problems.is_empty(), "{problems:?}");
        let netherite = manager.material(&MaterialId::try_parse("tconstruct:netherite").unwrap()).unwrap();
        assert_eq!((netherite.tier, netherite.sort_order, netherite.rarity, netherite.craftable, netherite.hidden), (5, 100, Rarity::Epic, false, false));
    }

    #[test]
    fn a_material_without_crafting_information_is_skipped_with_a_problem() {
        let (manager, problems) = load(vec![("tconstruct:odd", json!({"tier": 1}))], vec![]);
        assert!(manager.all_materials().next().is_none());
        assert_eq!(problems.len(), 1);
    }

    #[test]
    fn conditions_and_redirects_decide_what_loads() {
        let gated = json!({"condition": {"type": "forge:mod_loaded", "modid": "other"}, "craftable": true, "tier": 2});
        let alias = json!({"redirect": [{"id": "tconstruct:wood", "condition": {"type": "forge:true"}}]});
        let broken = json!({"redirect": [{"id": "tconstruct:missing"}]});
        let (manager, problems) = load(vec![("tconstruct:wood", json!({"craftable": true})), ("tconstruct:gated", gated), ("tconstruct:alias", alias), ("tconstruct:broken", broken)], vec![]);
        assert!(manager.material(&MaterialId::try_parse("tconstruct:gated").unwrap()).is_none(), "no other mods are installed");
        let alias = MaterialId::try_parse("tconstruct:alias").unwrap();
        assert_eq!(manager.resolve_redirect(&alias).to_string(), "tconstruct:wood");
        let broken = MaterialId::try_parse("tconstruct:broken").unwrap();
        assert_eq!(manager.resolve_redirect(&broken), &broken, "a redirect to nothing is dropped");
        assert_eq!(problems.len(), 1);
    }

    #[test]
    fn visible_materials_are_sorted_and_hidden_ones_left_out() {
        let (manager, _) = load(
            vec![("tconstruct:b", json!({"craftable": true, "tier": 1, "sortOrder": 2})), ("tconstruct:a", json!({"craftable": true, "tier": 1, "sortOrder": 1})), ("tconstruct:h", json!({"craftable": true, "hidden": true})), ("tconstruct:t0", json!({"craftable": true, "tier": 0}))],
            vec![],
        );
        let names: Vec<&str> = manager.visible_materials().iter().map(|m| m.id.path()).collect();
        assert_eq!(names, ["t0", "a", "b"]);
    }

    #[test]
    fn tags_collect_materials_through_nested_tags() {
        let (manager, problems) = load(
            vec![("tconstruct:a", json!({"craftable": true})), ("tconstruct:b", json!({"craftable": true})), ("tconstruct:c", json!({"craftable": true}))],
            vec![
                ("tconstruct:inner", json!({"values": ["tconstruct:b", "tconstruct:c"]})),
                ("tconstruct:outer", json!({"values": ["tconstruct:a", "#tconstruct:inner", "tconstruct:a", {"id": "tconstruct:later", "required": false}]})),
            ],
        );
        assert!(problems.is_empty(), "{problems:?}");
        let outer = manager.tag(&id("tconstruct:outer"));
        assert_eq!(outer.iter().map(ToString::to_string).collect::<Vec<_>>(), ["tconstruct:a", "tconstruct:b", "tconstruct:c"]);
        assert!(manager.is_in(&MaterialId::try_parse("tconstruct:b").unwrap(), &id("tconstruct:inner")));
        assert_eq!(manager.tags_of(&MaterialId::try_parse("tconstruct:b").unwrap()).len(), 2);
        assert!(manager.tag(&id("tconstruct:none")).is_empty());
    }

    #[test]
    fn a_tag_that_includes_itself_or_names_a_missing_material_is_reported() {
        let (_, problems) = load(vec![("tconstruct:a", json!({"craftable": true}))], vec![("tconstruct:loop", json!({"values": ["#tconstruct:loop", "tconstruct:nothing"]}))]);
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    #[test]
    fn replace_discards_the_earlier_files_of_a_tag() {
        let (manager, _) = load(
            vec![("tconstruct:a", json!({"craftable": true})), ("tconstruct:b", json!({"craftable": true}))],
            vec![("tconstruct:t", json!({"values": ["tconstruct:a"]})), ("tconstruct:t", json!({"replace": true, "values": ["tconstruct:b"]}))],
        );
        assert_eq!(manager.tag(&id("tconstruct:t")).iter().map(ToString::to_string).collect::<Vec<_>>(), ["tconstruct:b"]);
    }
}
