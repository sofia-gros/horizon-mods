//! Authoritative material snapshot for the Horizon port (not Forge wire compatibility).
//! Conditions are resolved on the server. Stats/traits retain captured JSON layers so
//! numeric parsing, pack merges and lazy modifier references use the production loaders.
use crate::materials::{Conditions, Definitions, Rarity};
use pluto_lib::{Identifier, JsonResourceStack};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(crate) const MAX_BYTES: usize = 4 * 1024 * 1024;
fn stacks_to_json(stacks: &[JsonResourceStack]) -> Value {
    Value::Array(stacks.iter().map(|stack| json!([stack.id.to_string(),
        stack.layers.iter().filter_map(|layer| layer.as_ref().ok()).collect::<Vec<_>>()])).collect())
}

pub(crate) fn encode(state: &Definitions, stats: &[JsonResourceStack], traits: &[JsonResourceStack]) -> Result<Vec<u8>, String> {
    let mut definitions: Vec<Value> = state.materials.values().map(|m| json!([m.id.to_string(), {
        "tier":m.tier,"sortOrder":m.sort_order,"craftable":m.craftable,"hidden":m.hidden,
        "rarity":match m.rarity { Rarity::Common=>"common", Rarity::Uncommon=>"uncommon", Rarity::Rare=>"rare", Rarity::Epic=>"epic" }
    }])).collect();
    definitions.extend(state.redirects.iter().map(|(id, target)| json!([id.to_string(), {"redirect":[{"id":target.to_string()}]}])));
    let tags: Vec<Value> = state.tags.iter().map(|(id, members)| json!([id.to_string(), [{"values":members.iter().map(ToString::to_string).collect::<Vec<_>>()}]])).collect();
    let bytes = serde_json::to_vec(&json!({"version":1,"ready":state.server_loaded,
        "definitions":definitions,"tags":tags,"stats":stacks_to_json(stats),"traits":stacks_to_json(traits)})).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES { return Err("material snapshot exceeds 4 MiB".into()); }
    Ok(bytes)
}

struct NoConditions;
impl Conditions for NoConditions {
    type Parsed = ();
    fn decode(&self, _: &Value) -> Result<(), String> { Err("network materials cannot contain conditions".into()) }
    fn test(&self, _: &()) -> Result<bool, String> { Err("network conditions are not evaluated".into()) }
}
fn entries(value: &Value) -> Result<Vec<(Identifier, Value)>, String> {
    let array = value.as_array().ok_or("snapshot section must be an array")?;
    let mut seen = BTreeSet::new();
    array.iter().map(|entry| {
        let pair = entry.as_array().filter(|pair| pair.len() == 2).ok_or("snapshot entry must be [id,value]")?;
        let id = Identifier::parse(pair[0].as_str().ok_or("snapshot id must be a string")?).map_err(|e| e.to_string())?;
        if !seen.insert(id.clone()) { return Err(format!("duplicate snapshot id {id}")); }
        Ok((id, pair[1].clone()))
    }).collect()
}
fn stacks(value: &Value) -> Result<Vec<JsonResourceStack>, String> {
    entries(value)?.into_iter().map(|(id, layers)| Ok(JsonResourceStack {
        id, layers: layers.as_array().ok_or("snapshot layers must be an array")?.iter().cloned().map(Ok).collect(),
    })).collect()
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Definitions, String> {
    if bytes.len() > MAX_BYTES { return Err("material snapshot exceeds 4 MiB".into()); }
    let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if value.get("version").and_then(Value::as_u64) != Some(1) { return Err("unsupported material snapshot version".into()); }
    let ready = value.get("ready").and_then(Value::as_bool).ok_or("snapshot needs ready flag")?;
    let section = |name: &str| value.get(name).ok_or_else(|| format!("missing snapshot section {name}"));
    // Decode every section before publication. No native effects or config queries occur here.
    let files = entries(section("definitions")?)?;
    let tags = stacks(section("tags")?)?;
    let stats = stacks(section("stats")?)?;
    let traits = stacks(section("traits")?)?;
    let mut result = Definitions::load(&files, &NoConditions);
    result.load_tags(&tags);
    if !result.problems.is_empty() { return Err(result.problems.join("; ")); }
    result.stats = crate::material_stats::load(&stats, &mut result.problems);
    result.traits = crate::material_traits::load(&traits, &mut result.problems);
    result.server_loaded = ready;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authoritative_snapshot_round_trips_without_config_and_preserves_pack_merge() {
        let id = Identifier::parse("test:wood").unwrap();
        let mut original = Definitions::load(&[(id.clone(), json!({"craftable":true,"tier":4,"sortOrder":12,"rarity":"epic"}))], &NoConditions);
        original.server_loaded = true;
        let stats = vec![JsonResourceStack { id: id.clone(), layers:vec![
            Ok(json!({"stats":{"tconstruct:head":{"durability":40,"mining_speed":1e100}}})),
            Ok(json!({"stats":{"tconstruct:head":{"melee_attack":2}}})),
        ] }];
        let traits = vec![JsonResourceStack { id:id.clone(), layers:vec![Ok(json!({"default":[{"name":"test:trait","optional":true}]}))] }];
        original.stats = crate::material_stats::load(&stats, &mut original.problems);
        original.traits = crate::material_traits::load(&traits, &mut original.problems);
        let bytes = encode(&original, &stats, &traits).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.materials, original.materials);
        assert_eq!(decoded.stats, original.stats, "float overflow retains original f32 infinity rather than JSON null");
        assert_eq!(decoded.traits, original.traits);
        assert!(decoded.server_loaded);
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        value["definitions"][0][1]["condition"] = json!(true);
        assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
        assert!(decode(&bytes[..bytes.len()-1]).is_err());
        value["version"] = json!(2);
        assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}
