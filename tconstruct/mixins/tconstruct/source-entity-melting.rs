// Source entity recipes and layered registries. The runtime initially exposes
// only the Blaze bootstrap; unsupported matches never become a guessed fallback.
use std::collections::{BTreeMap, HashMap, HashSet};
use serde_json::Value;
use super::tconstruct_source_tags as tags;

type RawTags = BTreeMap<String, Vec<Vec<u8>>>;
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Snapshot {
    pub entities: RawTags,
    pub known_entities: HashSet<String>,
    pub damage: Value,
    pub damage_tags: Value,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct Recipe { pub fluid: String, pub amount: u32, pub damage: u32 }
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Prepared {
    pub recipe: Option<Recipe>,
    pub damage: Value,
    pub damage_tags: Value,
}


fn canonical(id: &str) -> String {
    if id.contains(':') { id.into() } else { format!("minecraft:{id}") }
}
fn entity_matches(value: &Value, entity: &str, snapshot: &Snapshot) -> Result<bool, ()> {
    if let Some(values) = value.as_array() {
        let mut found = false;
        for value in values { found |= entity_matches(value, entity, snapshot)?; }
        return Ok(found);
    }
    let obj = value.as_object().ok_or(())?;
    if obj.len() != 1 { return Err(()); }
    if let Some(value) = obj.get("type") { return Ok(canonical(value.as_str().ok_or(())?) == entity); }
    if let Some(values) = obj.get("types") {
        return values.as_array().ok_or(())?.iter().try_fold(false, |found, value|
            Ok(found || canonical(value.as_str().ok_or(())?) == entity));
    }
    let id = canonical(obj.get("tag").and_then(Value::as_str).ok_or(())?);
    match tags::resolve_tag(&id, &snapshot.entities, &snapshot.known_entities) {
        Ok(values) => Ok(values.iter().any(|id| id == entity)),
        Err(_) if tags::definitely_absent(&id, entity, &snapshot.entities).unwrap_or(false) => Ok(false),
        Err(_) => Err(()),
    }
}

pub(super) fn recipe(
    recipes: &BTreeMap<String, Vec<u8>>, snapshot: &Snapshot, entity: &str,
    fluids: &HashSet<String>, fluid_tags: &HashMap<String, HashSet<String>>,
) -> Result<Option<Recipe>, ()> {
    if !snapshot.known_entities.contains(entity) { return Err(()); }
    let blacklist = tags::resolve_tag("tconstruct:melting/blacklist", &snapshot.entities, &snapshot.known_entities).map_err(|_| ())?;
    if blacklist.iter().any(|id| id == entity) { return Ok(None); }
    let mut selected = None;
    for bytes in recipes.values() {
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else { continue; };
        if value.get("type").and_then(Value::as_str) != Some("tconstruct:entity_melting") { continue; }
        if !entity_matches(value.get("entity").ok_or(())?, entity, snapshot)? { continue; }
        // The source caches its first matching recipe. Until host ordering is
        // proven, ambiguous matches fail closed instead of choosing a new order.
        if selected.is_some() { return Err(()); }
        let object = value.as_object().ok_or(())?;
        if object.keys().any(|key| !["type", "entity", "result", "damage"].contains(&key.as_str())) { return Err(()); }
        let damage = match value.get("damage") { Some(value) => value.as_u64().ok_or(())?, None => 2 };
        if damage == 0 || damage > i32::MAX as u64 { return Err(()); }
        let output = super::tconstruct_source_melting_recipes::FluidSpec::parse(value.get("result").ok_or(())?).ok_or(())?;
        let fluid = if output.tagged {
            let values = fluid_tags.get(&output.id).ok_or(())?;
            if values.len() != 1 { return Err(()); }
            values.iter().next().ok_or(())?.clone()
        } else { output.id };
        if !fluids.contains(&fluid) { return Err(()); }
        selected = Some(Recipe { fluid, amount: output.amount, damage: damage as u32 });
    }
    Ok(selected)
}

pub(super) fn merge_pack(
    entities: &mut RawTags, damage_tags: &mut RawTags, definitions: &mut BTreeMap<String, Value>,
    files: impl IntoIterator<Item=String>, read: impl Fn(&str)->Option<Vec<u8>>,
) -> Result<(), String> {
    for file in files {
        let Some((namespace, path)) = file.strip_prefix("data/").and_then(|p| p.split_once('/')) else { continue; };
        let Some(path) = path.strip_suffix(".json") else { continue; };
        if namespace.is_empty() || path.split('/').any(|p| p.is_empty() || p == "." || p == "..") { continue; }
        if !["tags/entity_types/", "tags/entity_type/", "tags/damage_type/", "damage_type/"].iter().any(|prefix| path.starts_with(prefix)) { continue; }
        let Some(bytes) = read(&file) else { return Err(format!("unreadable entity-melting resource {file}")); };
        if let Some(id) = path.strip_prefix("tags/entity_types/").or_else(|| path.strip_prefix("tags/entity_type/")) {
            entities.entry(format!("{namespace}:{id}")).or_default().push(bytes);
        } else if let Some(id) = path.strip_prefix("tags/damage_type/") {
            damage_tags.entry(format!("{namespace}:{id}")).or_default().push(bytes);
        } else if let Some(id) = path.strip_prefix("damage_type/") {
            definitions.insert(format!("{namespace}:{id}"), serde_json::from_slice(&bytes).map_err(|error| error.to_string())?);
        }
    }
    Ok(())
}

pub(super) fn compile_damage(definitions: &BTreeMap<String, Value>, raw: &RawTags) -> Result<(Value, Value), String> {
    let known: HashSet<_> = definitions.keys().cloned().collect();
    let mut resolved = serde_json::Map::new();
    for id in raw.keys() {
        let values = tags::resolve_tag(id, raw, &known)?;
        resolved.insert(id.clone(), Value::String(serde_json::json!({"values":values}).to_string()));
    }
    let mut types = Vec::new();
    for (id, value) in definitions {
        let exhaustion = value.get("exhaustion").and_then(Value::as_f64).ok_or_else(|| format!("invalid exhaustion {id}"))? as f32;
        if !exhaustion.is_finite() || exhaustion < 0.0 { return Err(format!("invalid exhaustion {id}")); }
        types.push(serde_json::json!({"id":id,"message_id":value.get("message_id").ok_or("missing message_id")?,
            "exhaustion":format!("{:08x}",exhaustion.to_bits()),"scaling":value.get("scaling").ok_or("missing scaling")?,
            "effects":value.get("effects").cloned().unwrap_or(Value::String("hurt".into())),
            "death_message_type":value.get("death_message_type").cloned().unwrap_or(Value::String("default".into()))}));
    }
    Ok((serde_json::json!({"data_version":horizon_world::DATA_VERSION,"types":types}), resolved.into()))
}

// A native snapshot must agree with the actual loot/advancement tables.
// Otherwise unsupported tag operations could bypass one predicate but not
// another; disable only the affected source instead of guessing.
pub(super) fn matching_damage_tags(catalog: &horizon_world::damage_source::Catalog,
    name: &str, actual: &HashMap<String, HashSet<String>>) -> bool {
    let Some(id) = catalog.id(name) else { return false; };
    catalog.tags().chain(actual.keys().map(String::as_str)).all(|tag|
        actual.get(tag).is_some_and(|values| values.contains(name)) == catalog.in_tag(id, tag))
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn actual_gameplay_damage_tag_disagreement_fails_closed() {
        let definitions = [("tconstruct:smeltery_magic".into(), serde_json::json!({
            "message_id":"tconstruct.smeltery_magic","exhaustion":0.1,"scaling":"never"}))].into_iter().collect();
        let raw = [("minecraft:witch_resistant_to".into(), vec![br#"{"values":["tconstruct:smeltery_magic"]}"#.to_vec()])].into_iter().collect();
        let (damage, tags) = compile_damage(&definitions, &raw).unwrap();
        let catalog = horizon_world::damage_source::Catalog::from_json(&damage, &tags).unwrap();
        let mut actual = HashMap::new();
        assert!(!matching_damage_tags(&catalog, "tconstruct:smeltery_magic", &actual));
        actual.insert("minecraft:witch_resistant_to".into(), ["tconstruct:smeltery_magic".into()].into_iter().collect());
        assert!(matching_damage_tags(&catalog, "tconstruct:smeltery_magic", &actual));
        actual.insert("minecraft:bypasses_armor".into(), ["tconstruct:smeltery_magic".into()].into_iter().collect());
        assert!(!matching_damage_tags(&catalog, "tconstruct:smeltery_magic", &actual));
    }
    fn setup() -> (Snapshot, BTreeMap<String, Vec<u8>>, HashSet<String>) {
        let snapshot = Snapshot {
            known_entities: ["minecraft:blaze".into()].into_iter().collect(),
            entities: [("forge:blazes".into(), vec![br#"{"values":["minecraft:blaze"]}"#.to_vec()]),
                ("tconstruct:melting/blacklist".into(), vec![br#"{"values":[]}"#.to_vec()])].into_iter().collect(),
            ..Default::default()
        };
        let recipes = [("tconstruct:smeltery/entity_melting/blaze".into(), br#"{"type":"tconstruct:entity_melting","damage":2,"entity":{"tag":"forge:blazes"},"result":{"amount":20,"fluid":"tconstruct:blazing_blood"}}"#.to_vec())].into_iter().collect();
        (snapshot, recipes, ["tconstruct:blazing_blood".into()].into_iter().collect())
    }
    #[test] fn source_blaze_recipe_and_world_replacement() {
        let (mut snapshot, recipes, fluids) = setup();
        assert_eq!(recipe(&recipes, &snapshot, "minecraft:blaze", &fluids, &HashMap::new()).unwrap(),
            Some(Recipe { fluid:"tconstruct:blazing_blood".into(), amount:20, damage:2 }));
        snapshot.entities.get_mut("forge:blazes").unwrap().push(br#"{"replace":true,"values":[]}"#.to_vec());
        assert_eq!(recipe(&recipes, &snapshot, "minecraft:blaze", &fluids, &HashMap::new()), Ok(None));
    }
    #[test] fn blacklists_removals_unknown_tags_and_ambiguous_recipes_never_fallback() {
        let (mut snapshot, mut recipes, fluids) = setup();
        snapshot.entities.get_mut("tconstruct:melting/blacklist").unwrap().push(br#"{"values":["minecraft:blaze"]}"#.to_vec());
        assert_eq!(recipe(&recipes, &snapshot, "minecraft:blaze", &fluids, &HashMap::new()), Ok(None));
        snapshot.entities.get_mut("tconstruct:melting/blacklist").unwrap().push(br#"{"values":[],"remove":["minecraft:blaze"]}"#.to_vec());
        assert!(recipe(&recipes, &snapshot, "minecraft:blaze", &fluids, &HashMap::new()).unwrap().is_some());
        recipes.insert("example:conflict".into(), recipes.values().next().unwrap().clone());
        assert!(recipe(&recipes, &snapshot, "minecraft:blaze", &fluids, &HashMap::new()).is_err());
        recipes.remove("example:conflict");
        snapshot.entities.get_mut("forge:blazes").unwrap().push(br#"{"values":["example:unknown"]}"#.to_vec());
        assert!(recipe(&recipes, &snapshot, "minecraft:blaze", &fluids, &HashMap::new()).is_err());
    }
    #[test] fn unsupported_output_and_absent_fluids_do_not_damage() {
        let (snapshot, mut recipes, fluids) = setup();
        assert!(recipe(&recipes, &snapshot, "minecraft:blaze", &HashSet::new(), &HashMap::new()).is_err());
        let bytes = recipes.values_mut().next().unwrap();
        let mut value: Value = serde_json::from_slice(bytes).unwrap();
        value["result"]["nbt"] = serde_json::json!({"Potion":"minecraft:water"});
        *bytes = serde_json::to_vec(&value).unwrap();
        assert!(recipe(&recipes, &snapshot, "minecraft:blaze", &fluids, &HashMap::new()).is_err());
    }
    #[test] fn layered_damage_tags_preserve_source_identity_and_overrides() {
        let mut definitions = BTreeMap::new(); let mut damage_tags = BTreeMap::new(); let mut entities = BTreeMap::new();
        let files: BTreeMap<String, Vec<u8>> = [
            ("data/tconstruct/damage_type/smeltery_magic.json".into(), br#"{"effects":"burning","exhaustion":0.1,"message_id":"tconstruct.smeltery_magic","scaling":"never"}"#.to_vec()),
            ("data/minecraft/tags/damage_type/witch_resistant_to.json".into(), br#"{"values":["tconstruct:smeltery_magic"]}"#.to_vec()),
        ].into_iter().collect();
        merge_pack(&mut entities, &mut damage_tags, &mut definitions, files.keys().cloned(), |name| files.get(name).cloned()).unwrap();
        let (data, tags) = compile_damage(&definitions, &damage_tags).unwrap();
        let catalog = horizon_world::damage_source::Catalog::from_json(&data, &tags).unwrap();
        let id = catalog.id("tconstruct:smeltery_magic").unwrap();
        assert!(catalog.in_tag(id, "minecraft:witch_resistant_to"));
        assert!(!catalog.in_tag(id, "minecraft:bypasses_armor"));
        damage_tags.get_mut("minecraft:witch_resistant_to").unwrap().push(br#"{"replace":true,"values":[]}"#.to_vec());
        let (data, tags) = compile_damage(&definitions, &damage_tags).unwrap();
        let catalog = horizon_world::damage_source::Catalog::from_json(&data, &tags).unwrap();
        assert!(!catalog.in_tag(catalog.id("tconstruct:smeltery_magic").unwrap(), "minecraft:witch_resistant_to"));
    }
}
