// Symbolic derived-type transport; no global numeric registry IDs.
use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;
use serde_json::{Value, value::{RawValue, to_raw_value}};
#[derive(Clone, Debug)]
pub struct Definition {
    pub id: String, pub factory: String, pub width: f32, pub height: f32,
    pub tracking_chunks: u32, pub velocity_updates: bool,
}
pub type Provider = fn() -> Result<Vec<Definition>, String>;
static PROVIDER: OnceLock<Provider> = OnceLock::new();
pub fn set_provider(provider: Provider) -> Result<(), String> {
    PROVIDER.set(provider).map_err(|_| "entity type provider already installed".into())
}
pub fn declarations() -> Result<Vec<Definition>, String> {
    PROVIDER.get().map_or_else(|| Ok(Vec::new()), |provider| provider())
}
pub fn cache_identity(types: &[Definition]) -> Result<Vec<u8>, String> {
    let mut sorted: Vec<_> = types.iter().collect();
    sorted.sort_by(|a,b| a.id.cmp(&b.id));
    let mut seen = HashSet::new();
    let mut bytes = b"pluto-derived-entities-v2".to_vec();
    for d in sorted {
        if !seen.insert(&d.id) || d.id.starts_with("minecraft:") || !d.factory.starts_with("minecraft:")
            || !d.width.is_finite() || d.width <= 0.0 || !d.height.is_finite() || d.height <= 0.0
            || d.tracking_chunks == 0 || d.tracking_chunks > i32::MAX as u32 { return Err("invalid derived entity declaration".into()); }
        for name in [&d.id, &d.factory] {
            bytes.extend_from_slice(&(name.len() as u64).to_le_bytes()); bytes.extend_from_slice(name.as_bytes());
        }
        bytes.extend_from_slice(&d.width.to_bits().to_le_bytes()); bytes.extend_from_slice(&d.height.to_bits().to_le_bytes());
        bytes.extend_from_slice(&d.tracking_chunks.to_le_bytes()); bytes.push(d.velocity_updates as u8);
    }
    Ok(bytes)
}

/// Preserve every original row's literal bytes, including exact float spelling.
/// Custom rows retain raw inherited fields; all factories/conflicts validate
/// before returning output. Tracking requests are explicit metadata until the
/// native tracking-policy adapter consumes them, not a claim of live syncing.
pub fn entity_table(bytes: &[u8], types: &[Definition]) -> Result<Vec<u8>, String> {
    cache_identity(types)?;
    if types.is_empty() { return Ok(bytes.to_vec()); }
    type RawMap = BTreeMap<String, Box<RawValue>>;
    let rows: RawMap = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut added = Vec::new();
    for d in types {
        if rows.contains_key(&d.id) { return Err(format!("entity {} already exists", d.id)); }
        let template = rows.get(&d.factory).ok_or_else(|| format!("missing factory {}", d.factory))?;
        let mut fields: RawMap = serde_json::from_str(template.get()).map_err(|e| e.to_string())?;
        let old_height: f64 = serde_json::from_str(fields.get("height").ok_or("factory has no height")?.get()).map_err(|e| e.to_string())?;
        let eye: f64 = serde_json::from_str(fields.get("eye_height").ok_or("factory has no eyes")?.get()).map_err(|e| e.to_string())?;
        if !old_height.is_finite() || old_height <= 0.0 || !eye.is_finite() { return Err("invalid factory dimensions".into()); }
        fields.insert("template".into(), to_raw_value(&d.factory).map_err(|e| e.to_string())?);
        fields.insert("width".into(), to_raw_value(&f64::from(d.width)).map_err(|e| e.to_string())?);
        fields.insert("height".into(), to_raw_value(&f64::from(d.height)).map_err(|e| e.to_string())?);
        fields.insert("eye_height".into(), to_raw_value(&f64::from(d.height * (eye / old_height) as f32)).map_err(|e| e.to_string())?);
        fields.insert("pluto:tracking_chunks".into(), to_raw_value(&d.tracking_chunks).map_err(|e| e.to_string())?);
        fields.insert("pluto:velocity_updates".into(), to_raw_value(&d.velocity_updates).map_err(|e| e.to_string())?);
        added.push((d.id.clone(), serde_json::to_vec(&fields).map_err(|e| e.to_string())?));
    }
    let mut out = bytes.trim_ascii_end().to_vec();
    if out.pop() != Some(b'}') { return Err("entity table is not an object".into()); }
    let mut empty = rows.is_empty();
    for (id, row) in added {
        if !std::mem::take(&mut empty) { out.push(b','); }
        out.extend(serde_json::to_vec(&id).map_err(|e| e.to_string())?); out.push(b':'); out.extend(row);
    }
    out.push(b'}'); Ok(out)
}

/// DefaultAttributes suppliers are a separate catalog from entity-types.json.
/// Retain raw definition/supplier values (including bit strings and unknown
/// fields); validate the whole batch before returning any output to the writer.
fn attribute_table(bytes:&[u8],types:&[Definition])->Result<Vec<u8>,String> {
    cache_identity(types)?;
    if types.is_empty() {return Ok(bytes.to_vec());}
    type RawMap=BTreeMap<String,Box<RawValue>>;
    let mut root:RawMap=serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
    let mut suppliers:RawMap=serde_json::from_str(root.get("entities").ok_or("missing attribute suppliers")?.get())
        .map_err(|e|e.to_string())?;
    for d in types {
        if suppliers.contains_key(&d.id) {return Err(format!("attribute supplier {} already exists",d.id));}
        let inherited=suppliers.get(&d.factory).ok_or_else(||format!("missing attribute supplier {}",d.factory))?;
        // Native Catalog expects an object, never a missing/null supplier.
        let _:RawMap=serde_json::from_str(inherited.get()).map_err(|e|e.to_string())?;
        suppliers.insert(d.id.clone(),inherited.clone());
    }
    root.insert("entities".into(),to_raw_value(&suppliers).map_err(|e|e.to_string())?);
    serde_json::to_vec(&root).map_err(|e|e.to_string())
}
/// The entity-table writer prepares BOTH catalogs before publishing either.
/// A missing/invalid supplier cannot yield a successful entity-only prefix.
pub fn entity_catalogs(entity_bytes:&[u8],attribute_bytes:&[u8],types:&[Definition])->Result<(Vec<u8>,Vec<u8>),String> {
    let entities=entity_table(entity_bytes,types)?;
    if types.is_empty() {return Ok((entities,attribute_bytes.to_vec()));}
    type RawMap=BTreeMap<String,Box<RawValue>>;
    let rows:RawMap=serde_json::from_slice(entity_bytes).map_err(|e|e.to_string())?;
    let mut living=Vec::new();
    for d in types {
        let row:RawMap=serde_json::from_str(rows[&d.factory].get()).map_err(|e|e.to_string())?;
        // Native export has `attributes` only for DefaultAttributes suppliers.
        // Do not invent a supplier or reject a legitimate nonliving factory.
        if let Some(attributes)=row.get("attributes") {
            let _:RawMap=serde_json::from_str(attributes.get()).map_err(|e|e.to_string())?;
            living.push(d.clone());
        }
    }
    let attributes=attribute_table(attribute_bytes,&living)?;
    Ok((entities,attributes))
}

pub fn registry_table(bytes: &[u8], types: &[Definition]) -> Result<Vec<u8>, String> {
    cache_identity(types)?;
    if types.is_empty() { return Ok(bytes.to_vec()); }
    let mut doc: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let entries = doc.pointer_mut("/minecraft:entity_type/entries").and_then(Value::as_object_mut).ok_or("missing entity registry entries")?;
    let mut next = entries.values().map(|v| v.get("protocol_id").and_then(Value::as_u64).ok_or("invalid entity protocol ID"))
        .collect::<Result<Vec<_>,_>>()?.into_iter().max().map_or(Ok(0), |n| n.checked_add(1).ok_or("entity protocol ID overflow"))?;
    let mut sorted: Vec<_> = types.iter().collect(); sorted.sort_by(|a,b|a.id.cmp(&b.id));
    for d in sorted {
        if entries.contains_key(&d.id) { return Err(format!("entity registry already contains {}",d.id)); }
        let id = u32::try_from(next).map_err(|_| "entity protocol ID overflow")?;
        entries.insert(d.id.clone(), serde_json::json!({"protocol_id":id})); next += 1;
    }
    serde_json::to_vec(&doc).map_err(|e| e.to_string())
}
pub fn command_table(bytes: &[u8], types: &[Definition]) -> Result<Vec<u8>, String> {
    cache_identity(types)?;
    if types.is_empty() { return Ok(bytes.to_vec()); }
    let mut doc: Value = serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
    let mut names: Vec<_> = types.iter().map(|d|d.id.clone()).collect(); names.sort();
    for path in ["/entity_types", "/summonable", "/by_registry/minecraft:entity_type/ids"] {
        let ids = doc.pointer_mut(path).and_then(Value::as_array_mut).ok_or("missing entity command list")?;
        for name in &names { if !ids.iter().any(|v|v.as_str()==Some(name)) { ids.push(name.clone().into()); } }
    }
    serde_json::to_vec(&doc).map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definition(id:&str)->Definition { Definition { id:id.into(), factory:"minecraft:slime".into(),
        width:2.04_f32*0.255_f32, height:2.04_f32*0.255_f32, tracking_chunks:20, velocity_updates:true } }
    #[test]
    fn derived_rows_preserve_literal_floats_and_publish_matching_network_and_command_ids() {
        let bytes=br#"{"minecraft:slime":{"width":0.52,"height":0.52,"eye_height":0.325,"category":"monster","attributes":{"minecraft:jump_strength":0.41999998688697815}}}"#;
        let types=[definition("example:sky"),definition("example:ender")];
        let attributes=br#"{"data_version":5023,"attributes":[{"id":"minecraft:armor","default":"0000000000000000","min":"0000000000000000","max":"403e000000000000","syncable":true},{"id":"minecraft:jump_strength","default":"3fdae147a0000000","min":"0000000000000000","max":"4040000000000000","syncable":true}],"entities":{"minecraft:slime":{"minecraft:armor":"0000000000000000","minecraft:jump_strength":"3fdae147a0000000"}},"unknown":{"precise":0.41999998688697815}}"#;
        let (out,catalog)=entity_catalogs(bytes,attributes,&types).unwrap();
        type RawMap=BTreeMap<String,Box<RawValue>>;
        let original:RawMap=serde_json::from_slice(attributes).unwrap();
        let inherited:RawMap=serde_json::from_slice(&catalog).unwrap();
        for key in ["data_version","attributes","unknown"] {assert_eq!(inherited[key].get(),original[key].get());}
        let suppliers:RawMap=serde_json::from_str(inherited["entities"].get()).unwrap();
        for kind in ["minecraft:slime","example:sky","example:ender"] {
            assert_eq!(suppliers[kind].get(),r#"{"minecraft:armor":"0000000000000000","minecraft:jump_strength":"3fdae147a0000000"}"#);
        }
        // Returned buffers are independent; replacing one derived default does
        // not mutate the template, sibling supplier or the original bytes.
        let mut changed:Value=serde_json::from_slice(&catalog).unwrap();
        changed["entities"]["example:sky"]["minecraft:armor"]="3ff0000000000000".into();
        assert_eq!(changed["entities"]["example:ender"],changed["entities"]["minecraft:slime"]);
        assert!(out.starts_with(&bytes[..bytes.len()-1]));
        let doc:Value=serde_json::from_slice(&out).unwrap();
        assert_eq!(doc["example:sky"]["template"],"minecraft:slime");
        assert_eq!(doc["example:sky"]["width"],f64::from(2.04_f32*0.255_f32));
        assert_eq!(doc["example:sky"]["attributes"],doc["minecraft:slime"]["attributes"]);
        let registry=br#"{"minecraft:entity_type":{"entries":{"minecraft:slime":{"protocol_id":9}}},"sentinel":true}"#;
        let out=registry_table(registry,&types).unwrap(); let doc:Value=serde_json::from_slice(&out).unwrap();
        assert_eq!(doc["minecraft:entity_type"]["entries"]["example:ender"]["protocol_id"],10);
        assert_eq!(doc["minecraft:entity_type"]["entries"]["example:sky"]["protocol_id"],11);
        assert_eq!(doc["sentinel"],true);
        let commands=br#"{"entity_types":["minecraft:slime"],"summonable":["minecraft:slime"],"by_registry":{"minecraft:entity_type":{"ids":["minecraft:slime"]}}}"#;
        let out=command_table(commands,&types).unwrap();let doc:Value=serde_json::from_slice(&out).unwrap();
        assert_eq!(doc["summonable"],serde_json::json!(["minecraft:slime","example:ender","example:sky"]));
        let mut reversed=types.to_vec();reversed.reverse();assert_eq!(cache_identity(&types).unwrap(),cache_identity(&reversed).unwrap());
    }
    #[test]
    fn invalid_factory_conflicts_and_later_errors_do_not_return_partial_output() {
        assert!(entity_table(b"{}",&[definition("example:missing")]).is_err());
        let mut bad=definition("example:bad");bad.width=f32::NAN;
        assert!(cache_identity(&[bad]).is_err());
        assert!(cache_identity(&[definition("example:a"),definition("example:a")]).is_err());
        assert!(command_table(b"{}",&[definition("example:a")]).is_err());
        assert_eq!(entity_table(b"unchanged",&[]).unwrap(),b"unchanged");
        assert_eq!(attribute_table(b"unchanged",&[]).unwrap(),b"unchanged");
        let nonliving=br#"{"minecraft:item":{"width":0.25,"height":0.25,"eye_height":0.2125}}"#;
        let catalog=br#"{"data_version":5023,"attributes":[],"entities":{}}"#;
        let (rows,attributes)=entity_catalogs(nonliving,catalog,&[Definition {factory:"minecraft:item".into(),..definition("example:item")}]).unwrap();
        assert_eq!(attributes,catalog,"nonliving factories do not acquire fabricated suppliers");
        assert!(serde_json::from_slice::<Value>(&rows).unwrap().get("example:item").is_some());
        let entity=br#"{"minecraft:slime":{"width":0.52,"height":0.52,"eye_height":0.325,"attributes":{}}}"#;
        for invalid in [br#"{}"#.as_slice(),br#"{"entities":{}}"#,br#"{"entities":{"minecraft:slime":null}}"#,
            br#"{"entities":{"minecraft:slime":{},"example:a":{}}}"#] {
            assert!(entity_catalogs(entity,invalid,&[definition("example:a")]).is_err());
        }
        assert!(entity_catalogs(entity,br#"{"entities":{"minecraft:slime":{}}}"#,
            &[definition("example:a"),Definition {factory:"minecraft:missing".into(),..definition("example:b")}]).is_err());
    }
}
