// Forge 1.20.1 tag file merge, before registry-dependent resolution.
// A tag's files are supplied in increasing pack priority. This module is not
// yet connected to gameplay or world pack enumeration.
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TagEntry {
    pub id: String,
    pub required: bool,
    pub remove: bool,
}

/// Invalid tag files are ignored as a whole, as in Forge's TagLoader.
pub(super) fn merge_tag_files(files: &[Vec<u8>]) -> Vec<TagEntry> {
    let mut merged = Vec::new();
    for bytes in files {
        let Some((replace, entries)) = parse_file(bytes) else { continue; };
        if replace { merged.clear(); }
        merged.extend(entries);
    }
    merged
}

fn parse_file(bytes: &[u8]) -> Option<(bool, Vec<TagEntry>)> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let obj = value.as_object()?;
    let replace = match obj.get("replace") {
        None => false,
        Some(value) => value.as_bool()?,
    };
    let mut entries = Vec::new();
    for (key, remove) in [("values", false), ("remove", true)] {
        let values = match obj.get(key) {
            None if remove => continue,
            value => value?.as_array()?,
        };
        for value in values {
            let (id, required) = if let Some(id) = value.as_str() {
                (id, true)
            } else {
                let item = value.as_object()?;
                let id = item.get("id")?.as_str()?;
                let required = match item.get("required") {
                    Some(required) => required.as_bool()?,
                    None => true,
                };
                (id, required)
            };
            if !valid_id(id.strip_prefix('#').unwrap_or(id)) { return None; }
            entries.push(TagEntry { id: id.to_owned(), required, remove });
        }
    }
    Some((replace, entries))
}

fn valid_id(id: &str) -> bool {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    !namespace.is_empty() && !path.is_empty()
        && namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        && path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
}

/// Prove that an item cannot be in a tag even when unrelated required entries
/// are not registered yet. Traverse the merged raw tag graph conservatively:
/// removals cannot add an item, and any reachable matching item is unknown,
/// not evidence of membership (the full tag might fail to load).
pub(super) fn definitely_absent(
    tag: &str, item: &str, tags: &BTreeMap<String, Vec<Vec<u8>>>,
) -> Result<bool, String> {
    fn canonical(id: &str) -> String {
        if id.contains(':') { id.to_owned() } else { format!("minecraft:{id}") }
    }
    fn visit(tag: &str, item: &str, tags: &BTreeMap<String, Vec<Vec<u8>>>, visiting: &mut Vec<String>) -> Result<bool, String> {
        if visiting.iter().any(|id| id == tag) { return Err(format!("cyclic tag {tag}")); }
        let Some(files) = tags.get(tag) else { return Ok(true); };
        visiting.push(tag.to_owned());
        for entry in merge_tag_files(files).into_iter().filter(|entry| !entry.remove) {
            let absent = if let Some(nested) = entry.id.strip_prefix('#') {
                visit(&canonical(nested), item, tags, visiting)?
            } else { canonical(&entry.id) != item };
            if !absent { visiting.pop(); return Ok(false); }
        }
        visiting.pop();
        Ok(true)
    }
    visit(&canonical(tag), &canonical(item), tags, &mut Vec::new())
}

/// Reconstruct the ordered merge inputs by namespaced tag ID. Values are
/// raw files so `replace` and Forge `remove` retain their original ordering.
pub fn merge_pack_tags(
    tags: &mut BTreeMap<String, Vec<Vec<u8>>>,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) {
    for file in files {
        let Some(rest) = file.strip_prefix("data/") else { continue; };
        let Some((namespace, path)) = rest.split_once('/') else { continue; };
        let Some(path) = path.strip_prefix("tags/items/").and_then(|p| p.strip_suffix(".json")) else { continue; };
        if !valid_id(&format!("{namespace}:{path}")) || path.split('/').any(|p| p.is_empty() || p == "." || p == "..") { continue; }
        if let Some(bytes) = read(&file) { tags.entry(format!("{namespace}:{path}")).or_default().push(bytes); }
    }
}

/// Mantle 1.20's generated water tag, absent from the retained TConstruct
/// and Forge source archives. Later datapack layers may still replace it.
pub(super) fn merge_mantle_fluid_tags(tags: &mut BTreeMap<String, Vec<Vec<u8>>>) {
    tags.entry("mantle:water".into()).or_default().push(
        br#"{"values":["minecraft:water","minecraft:flowing_water"]}"#.to_vec());
}

pub(super) fn merge_pack_fluid_tags(
    tags: &mut BTreeMap<String, Vec<Vec<u8>>>,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) {
    for file in files {
        let Some(rest) = file.strip_prefix("data/") else { continue; };
        let Some((namespace, path)) = rest.split_once('/') else { continue; };
        let Some(path) = path.strip_prefix("tags/fluids/").and_then(|p| p.strip_suffix(".json")) else { continue; };
        if !valid_id(&format!("{namespace}:{path}")) || path.split('/').any(|p| p.is_empty() || p == "." || p == "..") { continue; }
        if let Some(bytes) = read(&file) { tags.entry(format!("{namespace}:{path}")).or_default().push(bytes); }
    }
}

pub(super) fn merge_pack_block_tags(
    tags: &mut BTreeMap<String, Vec<Vec<u8>>>,
    files: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> Option<Vec<u8>>,
) {
    for file in files {
        let Some(rest) = file.strip_prefix("data/") else { continue; };
        let Some((namespace, path)) = rest.split_once('/') else { continue; };
        let Some(path) = path.strip_prefix("tags/blocks/").or_else(|| path.strip_prefix("tags/block/"))
            .and_then(|p| p.strip_suffix(".json")) else { continue; };
        if !valid_id(&format!("{namespace}:{path}")) || path.split('/').any(|p| p.is_empty() || p == "." || p == "..") { continue; }
        if let Some(bytes) = read(&file) { tags.entry(format!("{namespace}:{path}")).or_default().push(bytes); }
    }
}

/// Bridge the source 1.20 spelling to the vanilla 26.3 block tag when the
/// original vanilla tag is absent. Insert at vanilla priority so later
/// datapacks can extend or replace the legacy tag normally.
pub(super) fn bridge_vanilla_mud_tag(tags: &mut BTreeMap<String, Vec<Vec<u8>>>) {
    if !tags.contains_key("minecraft:convertable_to_mud") && tags.contains_key("minecraft:convertible_to_mud") {
        tags.insert("minecraft:convertable_to_mud".into(), vec![br##"{"values":["#minecraft:convertible_to_mud"]}"##.to_vec()]);
    }
}

/// Project a source block tag onto the actual block registry. Unregistered
/// source block IDs cannot be encountered in this world and are ignored;
/// unresolved required nested tags and cycles remain errors.
pub(super) fn resolve_available_blocks(
    id: &str,
    tags: &BTreeMap<String, Vec<Vec<u8>>>,
    registry: &std::collections::HashSet<String>,
) -> Result<Vec<String>, String> {
    fn canonical(id: &str) -> String {
        if id.contains(':') { id.to_owned() } else { format!("minecraft:{id}") }
    }
    fn resolve(id: &str, tags: &BTreeMap<String, Vec<Vec<u8>>>, registry: &std::collections::HashSet<String>, visiting: &mut Vec<String>) -> Result<Vec<String>, String> {
        if visiting.iter().any(|value| value == id) { return Err(format!("cyclic block tag {id}")); }
        let files = tags.get(id).ok_or_else(|| format!("missing block tag {id}"))?;
        if !files.iter().any(|file| parse_file(file).is_some()) { return Err(format!("invalid block tag {id}")); }
        visiting.push(id.to_owned());
        let mut result = Vec::new();
        for entry in merge_tag_files(files) {
            let values = if let Some(tag) = entry.id.strip_prefix('#') {
                match resolve(&canonical(tag), tags, registry, visiting) {
                    Ok(values) => values,
                    Err(error) if error.starts_with("cyclic block tag ") => { visiting.pop(); return Err(error); }
                    Err(_) if entry.remove || !entry.required => continue,
                    Err(error) => { visiting.pop(); return Err(error); }
                }
            } else {
                let block = canonical(&entry.id);
                if registry.contains(&block) { vec![block] } else { Vec::new() }
            };
            for value in values {
                if entry.remove { result.retain(|existing| existing != &value); }
                else if !result.contains(&value) { result.push(value); }
            }
        }
        visiting.pop();
        Ok(result)
    }
    resolve(&canonical(id), tags, registry, &mut Vec::new())
}

/// Resolve an acyclic tag against the current registry. Missing required
/// entries fail the tag; missing removals are optional in Forge. Cycles are
/// explicitly unsupported for now and must not be treated as an empty tag.
#[allow(dead_code)]
pub fn resolve_tag(
    id: &str,
    tags: &BTreeMap<String, Vec<Vec<u8>>>,
    registry: &std::collections::HashSet<String>,
) -> Result<Vec<String>, String> {
    resolve_tag_identities(id, tags, |id| registry.contains(id).then(|| id.to_owned()))
}

/// Resolve values to registry object identities before linked-set insertion or
/// removal. Aliases must compare as the same object, including inside nested
/// tags. The caller owns registry completeness and missing-value semantics.
pub(super) fn resolve_tag_identities(
    id: &str,
    tags: &BTreeMap<String, Vec<Vec<u8>>>,
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<Vec<String>, String> {
    fn canonical(id: &str) -> String {
        if id.contains(':') { id.to_owned() } else { format!("minecraft:{id}") }
    }
    fn resolve(
        id: &str,
        tags: &BTreeMap<String, Vec<Vec<u8>>>,
        lookup: &impl Fn(&str) -> Option<String>,
        visiting: &mut Vec<String>,
    ) -> Result<Vec<String>, String> {
        if visiting.iter().any(|v| v == id) { return Err(format!("cyclic tag {id}")); }
        let files = tags.get(id).ok_or_else(|| format!("missing tag {id}"))?;
        if !files.iter().any(|file| parse_file(file).is_some()) { return Err(format!("invalid tag {id}")); }
        visiting.push(id.to_owned());
        let mut result: Vec<String> = Vec::new();
        for entry in merge_tag_files(files) {
            let found = if let Some(tag) = entry.id.strip_prefix('#') {
                resolve(&canonical(tag), tags, lookup, visiting)
            } else {
                let item = canonical(&entry.id);
                lookup(&item).map(|identity| vec![identity]).ok_or_else(|| format!("missing item {item}"))
            };
            let values = match found {
                Ok(values) => values,
                // A cyclic dependency is not safely resolved by treating it as optional.
                Err(error) if error.starts_with("cyclic tag ") => { visiting.pop(); return Err(error); },
                Err(_) if entry.remove || !entry.required => continue,
                Err(error) => { visiting.pop(); return Err(error); },
            };
            for value in values {
                if entry.remove { result.retain(|v| v != &value); }
                else if !result.contains(&value) { result.push(value); }
            }
        }
        visiting.pop();
        Ok(result)
    }
    resolve(&canonical(id), tags, &lookup, &mut Vec::new())
}

/// Resolve item tags against one world's actual item registry. Like the
/// source loader, omit tags with unresolved required entries. Cycles remain
/// an explicit error until the original dependency sorter is implemented.
#[allow(dead_code)]
pub(super) fn resolve_registry(
    tags: &BTreeMap<String, Vec<Vec<u8>>>,
    registry: &std::collections::HashSet<String>,
) -> Result<std::collections::HashMap<String, std::collections::HashSet<String>>, String> {
    let mut resolved = std::collections::HashMap::new();
    for id in tags.keys() {
        match resolve_tag(id, tags, registry) {
            Ok(values) => { resolved.insert(id.clone(), values.into_iter().collect()); },
            Err(error) if error.starts_with("cyclic tag ") => return Err(error),
            Err(_) => {},
        }
    }
    Ok(resolved)
}

/// Query a fully resolved world snapshot. Missing or failed tags must not be
/// guessed from the built-in registry, since datapacks may replace their values.
pub(super) fn resolved_member(
    tags: Option<&std::collections::HashMap<String, std::collections::HashSet<String>>>,
    tag: &str, id: &str,
) -> Option<bool> {
    Some(tags?.get(tag)?.contains(id))
}

/// Resolve one tag independently of errors in unrelated tag graphs.
pub(super) fn local_tag_member(
    tags: &BTreeMap<String, Vec<Vec<u8>>>,
    registry: &std::collections::HashSet<String>,
    tag: &str, id: &str,
) -> Option<bool> {
    // An absent or wholly malformed root tag is not a reliable negative.
    let files = tags.get(tag)?;
    if !files.iter().any(|file| parse_file(file).is_some()) { return None; }
    // Prove negative membership without requiring unrelated registry entries.
    // A reachable matching entry is only a possibility, so resolve normally.
    if definitely_absent(tag, id, tags).ok()? { return Some(false); }
    Some(resolve_tag(tag, tags, registry).ok()?.iter().any(|value| value == id))
}

/// MaterialManager uses the same TagLoader, but with material identifiers
/// rather than items and the tinkering/tags/materials resource directory.
#[allow(dead_code)]
pub(super) fn resolve_material_registry(
    files: &BTreeMap<String, Vec<Vec<u8>>>,
    materials: &std::collections::HashSet<String>,
) -> Result<std::collections::HashMap<String, std::collections::HashSet<String>>, String> {
    let mut tags = BTreeMap::new();
    for (id, layers) in files {
        let Some((namespace, path)) = id.split_once(':') else { return Err(format!("invalid material tag {id}")); };
        let Some(path) = path.strip_prefix("tinkering/tags/materials/") else { continue; };
        tags.insert(format!("{namespace}:{path}"), layers.clone());
    }
    resolve_registry(&tags, materials)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolved_water_membership_is_world_scoped_and_unknown_fails_closed() {
        use std::collections::{HashMap, HashSet};
        let first = HashMap::from([("minecraft:water".to_owned(),
            HashSet::from(["minecraft:water".to_owned(), "minecraft:flowing_water".to_owned()]))]);
        let second = HashMap::from([("minecraft:water".to_owned(),
            HashSet::from(["minecraft:lava".to_owned()]))]);
        assert_eq!(resolved_member(Some(&first), "minecraft:water", "minecraft:water"), Some(true));
        assert_eq!(resolved_member(Some(&first), "minecraft:water", "minecraft:lava"), Some(false));
        assert_eq!(resolved_member(Some(&second), "minecraft:water", "minecraft:water"), Some(false));
        assert_eq!(resolved_member(Some(&second), "minecraft:water", "minecraft:lava"), Some(true));
        assert_eq!(resolved_member(None, "minecraft:water", "minecraft:water"), None);
        assert_eq!(resolved_member(Some(&HashMap::new()), "minecraft:water", "minecraft:water"), None);

        let raw = std::collections::BTreeMap::from([("minecraft:water".to_owned(), vec![
            br#"{"values":["minecraft:water","minecraft:flowing_water"]}"#.to_vec(),
            br#"{"replace":true,"values":["minecraft:lava"]}"#.to_vec(),
        ])]);
        let registry = HashSet::from(["minecraft:water".to_owned(),
            "minecraft:flowing_water".to_owned(), "minecraft:lava".to_owned()]);
        let resolved = resolve_registry(&raw, &registry).unwrap();
        assert_eq!(resolved_member(Some(&resolved), "minecraft:water", "minecraft:water"), Some(false));
        assert_eq!(resolved_member(Some(&resolved), "minecraft:water", "minecraft:lava"), Some(true));

        let mut raw_with_unrelated_cycle = raw.clone();
        raw_with_unrelated_cycle.insert("test:cycle".to_owned(), vec![br##"{"values":["#test:cycle"]}"##.to_vec()]);
        let mut registry_with_empty = registry.clone();
        registry_with_empty.insert("minecraft:empty".to_owned());
        assert!(resolve_registry(&raw_with_unrelated_cycle, &registry_with_empty).is_err());
        // The adapter's fast path has no global map after the unrelated
        // cycle; its local fallback must still permit an air target.
        let air_result = resolved_member(None, "minecraft:water", "minecraft:empty")
            .or_else(|| local_tag_member(&raw_with_unrelated_cycle, &registry_with_empty, "minecraft:water", "minecraft:empty"));
        assert_eq!(air_result, Some(false));
        assert_eq!(local_tag_member(&raw_with_unrelated_cycle, &registry_with_empty, "minecraft:water", "minecraft:lava"), Some(true));
        let mut bad_water = raw.clone();
        bad_water.insert("minecraft:water".to_owned(), vec![br##"{"values":["#minecraft:water"]}"##.to_vec()]);
        assert_eq!(local_tag_member(&bad_water, &registry_with_empty, "minecraft:water", "minecraft:empty"), None);
        assert_eq!(local_tag_member(&std::collections::BTreeMap::new(), &registry_with_empty, "minecraft:water", "minecraft:empty"), None);
        let malformed = std::collections::BTreeMap::from([("minecraft:water".to_owned(), vec![b"not-json".to_vec()])]);
        assert_eq!(local_tag_member(&malformed, &registry_with_empty, "minecraft:water", "minecraft:empty"), None);
        let unknown_required = std::collections::BTreeMap::from([("minecraft:water".to_owned(), vec![
            br#"{"values":["minecraft:water","example:unregistered_water"]}"#.to_vec()
        ])]);
        assert!(resolve_tag("minecraft:water", &unknown_required, &registry_with_empty).is_err());
        // Air and other proven non-members still permit propagation, while
        // a reachable water entry is not guessed from an invalid tag.
        assert_eq!(local_tag_member(&unknown_required, &registry_with_empty, "minecraft:water", "minecraft:empty"), Some(false));
        assert_eq!(local_tag_member(&unknown_required, &registry_with_empty, "minecraft:water", "minecraft:lava"), Some(false));
        assert_eq!(local_tag_member(&unknown_required, &registry_with_empty, "minecraft:water", "minecraft:water"), None);
    }

    #[test]
    fn missing_platform_does_not_block_unrelated_basin_input() {
        let tags = BTreeMap::from([("tconstruct:casts/empty/basin".into(),
            vec![br#"{"values":["tconstruct:gold_platform"]}"#.to_vec()])]);
        let registry = ["tconstruct:seared_lantern".to_owned()].into_iter().collect();
        let resolved = resolve_registry(&tags, &registry).unwrap();
        assert!(!resolved.contains_key("tconstruct:casts/empty/basin"));
        assert_eq!(definitely_absent("tconstruct:casts/empty/basin", "tconstruct:seared_lantern", &tags), Ok(true));
        assert_eq!(definitely_absent("tconstruct:casts/empty/basin", "tconstruct:gold_platform", &tags), Ok(false));
    }

    #[test]
    fn source_mud_tag_uses_vanilla_layer_and_respects_replacement() {
        let mut tags = BTreeMap::new();
        let path = "data/minecraft/tags/block/convertible_to_mud.json";
        merge_pack_block_tags(&mut tags, [path.to_owned()], |_| Some(br#"{"values":["minecraft:dirt","minecraft:coarse_dirt","minecraft:rooted_dirt"]}"#.to_vec()));
        bridge_vanilla_mud_tag(&mut tags);
        let registry = ["minecraft:dirt", "minecraft:coarse_dirt", "minecraft:rooted_dirt", "minecraft:stone"].into_iter().map(str::to_owned).collect();
        let values = resolve_tag("minecraft:convertable_to_mud", &tags, &registry).unwrap();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&"minecraft:dirt".to_owned()));
        tags.get_mut("minecraft:convertable_to_mud").unwrap().push(br#"{"replace":true,"values":["minecraft:stone"]}"#.to_vec());
        assert_eq!(resolve_tag("minecraft:convertable_to_mud", &tags, &registry).unwrap(), vec!["minecraft:stone".to_owned()]);
    }

    #[test]
    fn source_mud_tag_does_not_override_existing_legacy_layer() {
        let mut tags = BTreeMap::new();
        tags.insert("minecraft:convertible_to_mud".into(), vec![br#"{"values":["minecraft:dirt"]}"#.to_vec()]);
        tags.insert("minecraft:convertable_to_mud".into(), vec![br#"{"values":["minecraft:stone"]}"#.to_vec()]);
        bridge_vanilla_mud_tag(&mut tags);
        let registry = ["minecraft:dirt", "minecraft:stone"].into_iter().map(str::to_owned).collect();
        assert_eq!(resolve_tag("minecraft:convertable_to_mud", &tags, &registry).unwrap(), vec!["minecraft:stone".to_owned()]);
    }
    #[test]
    fn absent_member_is_provable_without_unrelated_registered_casts() {
        let mut tags = BTreeMap::new();
        tags.insert("tconstruct:patterns/reusable".into(), vec![br##"{"values":["#tconstruct:casts/gold","minecraft:elytra"]}"##.to_vec()]);
        tags.insert("tconstruct:casts/gold".into(), vec![br#"{"values":["tconstruct:ingot_cast","tconstruct:pick_head_cast"]}"#.to_vec()]);
        assert_eq!(definitely_absent("tconstruct:patterns/reusable", "tconstruct:pattern", &tags), Ok(true));
        assert_eq!(definitely_absent("tconstruct:patterns/reusable", "tconstruct:pick_head_cast", &tags), Ok(false));
        tags.get_mut("tconstruct:casts/gold").unwrap().push(br#"{"replace":true,"values":["tconstruct:pattern"]}"#.to_vec());
        assert_eq!(definitely_absent("tconstruct:patterns/reusable", "tconstruct:pattern", &tags), Ok(false));
        tags.insert("tconstruct:casts/gold".into(), vec![br##"{"values":["#tconstruct:patterns/reusable"]}"##.to_vec()]);
        assert!(definitely_absent("tconstruct:patterns/reusable", "tconstruct:pattern", &tags).is_err());
    }
    #[test]
    fn material_tags_resolve_against_materials_in_pack_order() {
        let mut files = BTreeMap::new();
        files.insert("tconstruct:tinkering/tags/materials/compatibility_metals".into(), vec![
            br#"{"values":["tconstruct:wood"]}"#.to_vec(),
            br#"{"replace":true,"values":["tconstruct:flint",{"id":"tconstruct:missing","required":false}]}"#.to_vec(),
        ]);
        let materials = ["tconstruct:wood".into(), "tconstruct:flint".into()].into_iter().collect();
        let tags = resolve_material_registry(&files, &materials).unwrap();
        assert_eq!(tags["tconstruct:compatibility_metals"], ["tconstruct:flint".into()].into_iter().collect());
    }

    #[test]
    fn fluid_tags_preserve_pack_layers_without_reading_item_tags() {
        let mut tags = BTreeMap::new();
        let fluid = "data/forge/tags/fluids/creosote.json";
        merge_pack_fluid_tags(&mut tags, [fluid.into(), "data/forge/tags/items/creosote.json".into()], |_| Some(b"base".to_vec()));
        merge_pack_fluid_tags(&mut tags, [fluid.into()], |_| Some(b"override".to_vec()));
        assert_eq!(tags["forge:creosote"], [b"base".to_vec(), b"override".to_vec()]);
        assert_eq!(tags.len(), 1);
    }
    #[test]
    fn effective_blocks_project_missing_ids_and_honor_nested_removal() {
        let mut tags = BTreeMap::new();
        tags.insert("minecraft:hoe".into(), vec![br#"{"values":["minecraft:pumpkin","minecraft:wheat","tconstruct:unported"]}"#.to_vec()]);
        tags.insert("tconstruct:scythe".into(), vec![br##"{"values":["#minecraft:hoe"],"remove":["minecraft:pumpkin"]}"##.to_vec()]);
        let registry = std::collections::HashSet::from(["minecraft:pumpkin".into(), "minecraft:wheat".into()]);
        assert_eq!(resolve_available_blocks("tconstruct:scythe", &tags, &registry).unwrap(), ["minecraft:wheat"]);
        assert!(resolve_available_blocks("tconstruct:missing", &tags, &registry).is_err());
    }

    #[test]
    fn block_tags_preserve_pack_layers_without_reading_item_tags() {
        let mut tags = BTreeMap::new();
        let block = "data/tconstruct/tags/blocks/smeltery/wall.json";
        merge_pack_block_tags(&mut tags, [block.into(), "data/tconstruct/tags/items/smeltery/wall.json".into()], |_| Some(b"base".to_vec()));
        merge_pack_block_tags(&mut tags, [block.into()], |_| Some(b"override".to_vec()));
        assert_eq!(tags["tconstruct:smeltery/wall"], [b"base".to_vec(), b"override".to_vec()]);
        assert_eq!(tags.len(), 1);
    }
    #[test]
    fn mantle_water_is_source_defined_and_world_packs_can_replace_it() {
        let mut raw = BTreeMap::new();
        merge_mantle_fluid_tags(&mut raw);
        let registry = ["minecraft:water".to_owned(), "minecraft:flowing_water".to_owned(), "minecraft:lava".to_owned()].into_iter().collect();
        assert_eq!(resolve_tag("mantle:water", &raw, &registry).unwrap(), ["minecraft:water", "minecraft:flowing_water"]);
        raw.get_mut("mantle:water").unwrap().push(br#"{"replace":true,"values":["minecraft:lava"]}"#.to_vec());
        assert_eq!(resolve_tag("mantle:water", &raw, &registry).unwrap(), ["minecraft:lava"]);
    }
    #[test]
    fn pack_priority_replace_and_forge_removal() {
        let files = vec![br##"{"values":["minecraft:stone",{"id":"#test:optional","required":false}]}"##.to_vec(),
                         br##"{"replace":true,"values":["test:wood"],"remove":["#test:old"]}"##.to_vec()];
        assert_eq!(merge_tag_files(&files), vec![TagEntry { id:"test:wood".into(), required:true, remove:false }, TagEntry { id:"#test:old".into(), required:true, remove:true }]);
    }
    #[test]
    fn invalid_files_do_not_erase_lower_priority() {
        let files = vec![br#"{"values":["test:keep"]}"#.to_vec(), br#"{"replace":true,"values":[42]}"#.to_vec()];
        assert_eq!(merge_tag_files(&files), vec![TagEntry { id:"test:keep".into(), required:true, remove:false }]);
    }
    #[test]
    fn required_optional_nested_and_removals() {
        let mut tags = BTreeMap::new();
        tags.insert("test:base".into(), vec![br#"{"values":["test:a","test:b"]}"#.to_vec()]);
        tags.insert("test:result".into(), vec![br##"{"values":["#test:base",{"id":"test:missing","required":false},"test:c"],"remove":["test:b","test:missing"]}"##.to_vec()]);
        let registry = ["test:a", "test:b", "test:c"].into_iter().map(str::to_owned).collect();
        assert_eq!(resolve_tag("test:result", &tags, &registry).unwrap(), ["test:a", "test:c"]);
        tags.insert("test:bad".into(), vec![br#"{"values":["test:missing"]}"#.to_vec()]);
        assert!(resolve_tag("test:bad", &tags, &registry).is_err());
    }
    #[test]
    fn cyclic_tags_are_not_silently_accepted() {
        let mut tags = BTreeMap::new();
        tags.insert("test:cycle".into(), vec![br##"{"values":[{"id":"#test:cycle","required":false}]}"##.to_vec()]);
        assert!(resolve_tag("test:cycle", &tags, &Default::default()).unwrap_err().starts_with("cyclic tag"));
    }
    #[test]
    fn registry_omits_missing_required_tags_and_rejects_cycles() {
        let mut tags = BTreeMap::new();
        tags.insert("test:valid".into(), vec![br#"{"values":["test:item",{"id":"test:optional","required":false}]}"#.to_vec()]);
        tags.insert("test:invalid".into(), vec![br#"{"values":["test:missing"]}"#.to_vec()]);
        let registry = ["test:item".to_owned()].into_iter().collect();
        let resolved = resolve_registry(&tags, &registry).unwrap();
        assert!(resolved["test:valid"].contains("test:item"));
        assert!(!resolved.contains_key("test:invalid"));
        tags.insert("test:cycle".into(), vec![br##"{"values":["#test:cycle"]}"##.to_vec()]);
        assert!(resolve_registry(&tags, &registry).unwrap_err().starts_with("cyclic tag"));
    }

    #[test]
    fn raw_tag_files_preserve_pack_order_and_ignore_other_kinds() {
        let mut tags = BTreeMap::new();
        let path = "data/test/tags/items/a.json";
        merge_pack_tags(&mut tags, [path.into(), "data/test/tags/blocks/a.json".into()], |_| Some(b"low".to_vec()));
        merge_pack_tags(&mut tags, [path.into()], |_| Some(b"high".to_vec()));
        assert_eq!(tags["test:a"], [b"low".to_vec(), b"high".to_vec()]);
        assert_eq!(tags.len(), 1);
    }
}

/// Restore the source music-disc tag when the host data version no longer
/// exposes it. Explicit pack definitions retain complete precedence.
pub(super) fn fallback_music_discs(
    raw: &BTreeMap<String, Vec<Vec<u8>>>,
    resolved: &mut std::collections::HashMap<String, std::collections::HashSet<String>>,
    playable: impl IntoIterator<Item = String>,
) {
    if !raw.contains_key("minecraft:music_discs") {
        resolved.insert("minecraft:music_discs".into(), playable.into_iter().collect());
    }
}

#[cfg(test)]
mod music_disc_fallback_tests {
    use super::*;
    #[test]
    fn missing_tag_uses_host_playable_items_but_explicit_tag_wins() {
        let mut resolved = std::collections::HashMap::new();
        fallback_music_discs(&BTreeMap::new(), &mut resolved, ["minecraft:music_disc_13".to_owned()]);
        assert!(resolved["minecraft:music_discs"].contains("minecraft:music_disc_13"));
        let raw = BTreeMap::from([("minecraft:music_discs".to_owned(), vec![br#"{"replace":true,"values":[]}"#.to_vec()])]);
        resolved.insert("minecraft:music_discs".into(), Default::default());
        fallback_music_discs(&raw, &mut resolved, ["minecraft:music_disc_13".to_owned()]);
        assert!(resolved["minecraft:music_discs"].is_empty());
    }
}

/// The source Forge head tag names heads that this port has not registered.
/// Project that tag onto available items while keeping pack replace/removal
/// and required nested-tag behavior. Existing fully resolved tags win.
pub(super) fn fallback_available_heads(
    raw: &BTreeMap<String, Vec<Vec<u8>>>,
    registry: &std::collections::HashSet<String>,
    resolved: &mut std::collections::HashMap<String, std::collections::HashSet<String>>,
) {
    if !resolved.contains_key("forge:heads") && raw.contains_key("forge:heads") {
        let mut projected = raw.clone();
        if let Some(layers) = projected.get_mut("forge:heads") {
            for layer in layers {
                let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(layer) else { continue; };
                if let Some(entries) = value.get_mut("values").and_then(serde_json::Value::as_array_mut) {
                    entries.retain(|entry| {
                        let id = entry.as_str().or_else(|| entry.get("id").and_then(serde_json::Value::as_str));
                        !id.is_some_and(|id| id.starts_with("tconstruct:") && !registry.contains(id))
                    });
                }
                if let Ok(bytes) = serde_json::to_vec(&value) { *layer = bytes; }
            }
        }
        if let Ok(heads) = resolve_tag("forge:heads", &projected, registry) {
            resolved.insert("forge:heads".into(), heads.into_iter().collect());
        }
    }
}

#[cfg(test)]
mod available_head_tests {
    use super::*;
    #[test]
    fn unavailable_source_heads_do_not_hide_registered_vanilla_heads() {
        let raw = BTreeMap::from([("forge:heads".to_owned(), vec![br#"{"values":["minecraft:skeleton_skull","tconstruct:unregistered_head"]}"#.to_vec()])]);
        let registry = ["minecraft:skeleton_skull".to_owned()].into_iter().collect();
        let mut result = resolve_registry(&raw, &registry).unwrap();
        assert!(!result.contains_key("forge:heads"));
        fallback_available_heads(&raw, &registry, &mut result);
        assert_eq!(result["forge:heads"], registry);
        let raw = BTreeMap::from([("forge:heads".to_owned(), vec![br#"{"values":["minecraft:missing_required_head"]}"#.to_vec()])]);
        let mut result = resolve_registry(&raw, &registry).unwrap();
        fallback_available_heads(&raw, &registry, &mut result);
        assert!(!result.contains_key("forge:heads"));
    }
}

#[cfg(test)]
mod tinted_glass_registry_tests {
    use super::*;
    use std::collections::{BTreeMap, HashSet};
    #[test]
    fn registered_source_glass_retains_forge_and_pack_semantics() {
        let mut tags = BTreeMap::from([("forge:glass/tinted".to_owned(), vec![
            br#"{"values":["minecraft:tinted_glass"]}"#.to_vec(),
            br#"{"values":["tconstruct:clear_tinted_glass"]}"#.to_vec(),
        ])]);
        let mut registry: HashSet<String> = ["minecraft:tinted_glass".to_owned()].into_iter().collect();
        assert!(!resolve_registry(&tags, &registry).unwrap().contains_key("forge:glass/tinted"));
        registry.insert("tconstruct:clear_tinted_glass".into());
        assert_eq!(resolve_registry(&tags, &registry).unwrap()["forge:glass/tinted"], registry);
        tags.get_mut("forge:glass/tinted").unwrap().push(br#"{"replace":true,"values":["example:missing"]}"#.to_vec());
        assert!(!resolve_registry(&tags, &registry).unwrap().contains_key("forge:glass/tinted"));
    }
}
