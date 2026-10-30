// Source material-render metadata and sprite selection. GUI rendering uses
// the prepared overlay; dynamic resource-pack reload is not yet supported.
use std::collections::{HashMap, HashSet};
use serde_json::{Map, Value};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct RenderInfo {
    pub texture: Option<String>,
    pub fallbacks: Vec<String>,
    pub color: u32,
    pub luminosity: u8,
}

fn resource_id(id: &str) -> Result<String, &'static str> {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    let namespace = if namespace.is_empty() { "minecraft" } else { namespace };
    if path.is_empty()
        || !namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        || !path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c)) {
        return Err("invalid resource location");
    }
    Ok(format!("{namespace}:{path}"))
}

/// `MaterialItem.getMaterialId` treats malformed NBT as the unknown material.
pub(super) fn material_id(value: &str) -> Option<String> {
    let (base, variant) = value.split_once('#').unwrap_or((value, ""));
    if !variant.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c)) { return None; }
    let base = resource_id(base).ok()?;
    Some(if variant.is_empty() { base } else { format!("{base}#{variant}") })
}

fn merged(records: &HashMap<String, Value>, id: &str, stack: &mut Vec<String>) -> Result<Map<String, Value>, &'static str> {
    if stack.iter().any(|entry| entry == id) { return Err("cyclic material render parent"); }
    let object = records.get(id).ok_or("missing material render parent")?.as_object().ok_or("invalid material render object")?;
    stack.push(id.to_owned());
    let mut result = object.clone();
    if let Some(parent) = result.remove("parent") {
        let parent = resource_id(parent.as_str().ok_or("invalid material render parent")?)?;
        for (key, value) in merged(records, &parent, stack)? { result.entry(key).or_insert(value); }
    }
    stack.pop();
    Ok(result)
}

/// The upstream loader tries a variant resource first, then the base resource.
/// Values are read from the winning resource layer; callers supply that map.
pub(super) fn render_info(records: &HashMap<String, Value>, material: &str) -> Result<Option<RenderInfo>, &'static str> {
    let (base, variant) = material.split_once('#').unwrap_or((material, ""));
    let base = resource_id(base)?;
    let (_, base_path) = base.split_once(':').ok_or("invalid render ID")?;
    let nonempty = |key: &str| -> Result<bool, &'static str> {
        match records.get(key) {
            None => Ok(false),
            Some(value) => Ok(!value.as_object().ok_or("invalid material render object")?.is_empty()),
        }
    };
    // The loader treats the last path component as a variant. Therefore an
    // asset path with a slash cannot represent a base-only render entry.
    let variant_key = format!("{base}/{variant}");
    let selected = if !variant.is_empty() && !variant.contains('/') && nonempty(&variant_key)? {
        variant_key
    } else if !base_path.contains('/') && nonempty(&base)? {
        base.clone()
    } else { return Ok(None); };
    let object = merged(records, &selected, &mut Vec::new())?;
    // The default texture uses the ID of the selected render info, even when
    // its data was inherited from a parent.
    let (ns, path) = selected.split_once(':').ok_or("invalid render ID")?;
    let (base_path, render_variant) = path.rsplit_once('/').map_or((path, ""), |v| v);
    let default_texture = if render_variant.is_empty() { format!("{ns}:{base_path}") } else { format!("{ns}:{base_path}_{render_variant}") };
    let texture = match object.get("texture") {
        Some(Value::Null) => None,
        Some(Value::String(value)) => Some(resource_id(value)?),
        Some(_) => return Err("invalid material texture"),
        None => match object.get("skipUniqueTexture") {
            None => Some(default_texture),
            Some(Value::Bool(true)) => None,
            Some(Value::Bool(false)) => Some(default_texture),
            Some(_) => return Err("invalid skipUniqueTexture"),
        },
    };
    let fallbacks = match object.get("fallbacks") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(values)) => values.iter().map(|v| v.as_str().map(str::to_owned).ok_or("invalid material fallback")).collect::<Result<_, _>>()?,
        Some(_) => return Err("invalid material fallbacks"),
    };
    let color = match object.get("color") {
        None | Some(Value::Null) => u32::MAX,
        Some(Value::String(value)) if value.len() == 6 || value.len() == 8 => {
            let parsed = u32::from_str_radix(value, 16).map_err(|_| "invalid material color")?;
            if value.len() == 6 { 0xff000000 | parsed } else { parsed }
        },
        Some(_) => return Err("invalid material color"),
    };
    let luminosity = match object.get("luminosity") {
        None | Some(Value::Null) => 0,
        Some(Value::Number(n)) => n.as_u64().filter(|v| *v <= 15).ok_or("unsupported luminosity")? as u8,
        Some(_) => return Err("invalid luminosity"),
    };
    Ok(Some(RenderInfo { texture, fallbacks, color, luminosity }))
}

/// Pick the same unique/fallback/base sprite and tint as MaterialRenderInfo.
pub(super) fn select_sprite(base: &str, info: &RenderInfo, available: &HashSet<String>) -> Result<(String, u32, u8), &'static str> {
    select_sprite_by(base, info, |id| available.contains(id))
}

pub(super) fn select_sprite_by(base: &str, info: &RenderInfo, available: impl Fn(&str) -> bool) -> Result<(String, u32, u8), &'static str> {
    let base = resource_id(base)?;
    if let Some(texture) = &info.texture {
        let (ns, path) = texture.split_once(':').ok_or("invalid texture")?;
        let suffix = if ns == "minecraft" { path.to_owned() } else { format!("{ns}_{path}") };
        let sprite = resource_id(&format!("{base}_{suffix}"))?;
        if available(&sprite) { return Ok((sprite, u32::MAX, info.luminosity)); }
    }
    for fallback in &info.fallbacks {
        let sprite = resource_id(&format!("{base}_{fallback}"))?;
        if available(&sprite) { return Ok((sprite, info.color, info.luminosity)); }
    }
    Ok((base, info.color, info.luminosity))
}

/// MaterialBlockModel.BakedSingleMaterial keeps its original model for
/// unknown material or missing render info; otherwise it retextures `all`.
/// None means the original model must be used unchanged.
pub(super) fn storage_sprite_by(
    records: &HashMap<String, Value>,
    material: &str,
    available: impl Fn(&str) -> bool,
) -> Result<Option<(String, u32, u8)>, &'static str> {
    let material = material_id(material).ok_or("invalid material ID")?;
    if material == "tconstruct:unknown" { return Ok(None); }
    let Some(info) = render_info(records, &material)? else { return Ok(None); };
    select_sprite_by("tconstruct:block/storage/fallback", &info, available).map(Some)
}

pub(super) type PartModels = HashMap<String, (String, [f32; 2])>;

/// Read the already-resolved resource overlay. Runtime resource-pack reload
/// and arbitrary external material-model registrations are not yet supported.
pub(super) fn load_render_assets(root: &std::path::Path) -> Result<(HashMap<String, Value>, PartModels), String> {
    fn visit(dir: &std::path::Path, prefix: &str, namespace: &str, out: &mut HashMap<String, Value>) -> Result<(), String> {
        if !dir.is_dir() { return Ok(()); }
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let name = entry.file_name().into_string().map_err(|_| "invalid render asset name")?;
            if path.is_dir() { visit(&path, &format!("{prefix}{name}/"), namespace, out)?; }
            else if let Some(name) = name.strip_suffix(".json") {
                let key = format!("{namespace}:{prefix}{name}");
                let value = serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                out.insert(key, value);
            }
        }
        Ok(())
    }
    let assets = root.join("assets");
    let mut records = HashMap::new();
    for namespace in std::fs::read_dir(&assets).map_err(|e| e.to_string())? {
        let namespace = namespace.map_err(|e| e.to_string())?;
        let name = namespace.file_name().into_string().map_err(|_| "invalid asset namespace")?;
        visit(&namespace.path().join("tinkering/materials"), "", &name, &mut records)?;
    }
    let model_dir = assets.join("tconstruct/tinkering/source_part_models");
    let mut models = HashMap::new();
    for file in std::fs::read_dir(model_dir).map_err(|e| e.to_string())? {
        let path = file.map_err(|e| e.to_string())?.path();
        if path.extension().is_none_or(|v| v != "json") { continue; }
        let name = path.file_stem().and_then(|v| v.to_str()).ok_or("invalid part model name")?;
        let value: Value = serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let texture = value.get("textures").and_then(|v| v.get("texture")).and_then(Value::as_str).ok_or("missing part texture")?;
        let offset = match value.get("offset") {
            None => [0.0, 0.0],
            Some(Value::Array(v)) if v.len() == 2 => [v[0].as_f64().ok_or("invalid part offset")? as f32, v[1].as_f64().ok_or("invalid part offset")? as f32],
            _ => return Err("invalid part offset".into()),
        };
        models.insert(format!("tconstruct:{name}"), (resource_id(texture).map_err(str::to_owned)?, offset));
    }
    Ok((records, models))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parent_variant_and_unique_then_tinted_fallback() {
        let records = [
            ("tconstruct:wood".into(), serde_json::json!({"color":"123456","fallbacks":["wood","stick"]})),
            ("tconstruct:wood/dark".into(), serde_json::json!({"parent":"tconstruct:wood","luminosity":3})),
        ].into_iter().collect();
        let info = render_info(&records, "tconstruct:wood#dark").unwrap().unwrap();
        assert_eq!(info.texture.as_deref(), Some("tconstruct:wood_dark"));
        assert_eq!(info.color, 0xff123456);
        let unique = ["tconstruct:item/head_tconstruct_wood_dark".into()].into_iter().collect();
        assert_eq!(select_sprite("tconstruct:item/head", &info, &unique).unwrap(), ("tconstruct:item/head_tconstruct_wood_dark".into(), u32::MAX, 3));
        let fallback = ["tconstruct:item/head_wood".into()].into_iter().collect();
        assert_eq!(select_sprite("tconstruct:item/head", &info, &fallback).unwrap(), ("tconstruct:item/head_wood".into(), 0xff123456, 3));
        let base = render_info(&records, "tconstruct:wood#missing").unwrap().unwrap();
        assert_eq!(base.texture.as_deref(), Some("tconstruct:wood"));
        let mut with_empty = records.clone();
        with_empty.insert("tconstruct:wood/empty".into(), serde_json::json!({}));
        assert_eq!(render_info(&with_empty, "tconstruct:wood#empty").unwrap().unwrap().texture.as_deref(), Some("tconstruct:wood"));
        assert_eq!(render_info(&records, "tconstruct:wood/dark"), Ok(None));
    }

    #[test]
    fn texture_null_and_parent_cycle_are_explicit() {
        let records = [
            ("test:a".into(), serde_json::json!({"texture":null,"color":"ffffff"})),
            ("test:b".into(), serde_json::json!({"parent":"test:c"})),
            ("test:c".into(), serde_json::json!({"parent":"test:b"})),
        ].into_iter().collect();
        assert_eq!(render_info(&records, "test:a").unwrap().unwrap().texture, None);
        assert_eq!(render_info(&records, "test:b"), Err("cyclic material render parent"));
        assert_eq!(material_id("tconstruct:wood#Bad"), None);
        assert_eq!(material_id("wood#oak").as_deref(), Some("minecraft:wood#oak"));
        assert_eq!(material_id(":wood#oak").as_deref(), Some("minecraft:wood#oak"));
    }

    #[test]
    fn pinned_render_data_selects_original_part_sprites() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors()
            .map(|p| p.join("content/reference/tconstruct-part-visuals.zip"))
            .find(|p| p.is_file()).expect("pinned part visuals");
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let mut records = HashMap::new();
        let mut available = HashSet::new();
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).unwrap();
            let name = entry.name().to_owned();
            if let Some(path) = name.strip_prefix("assets/tconstruct/tinkering/materials/").and_then(|s| s.strip_suffix(".json")) {
                let mut bytes = Vec::new(); entry.read_to_end(&mut bytes).unwrap();
                records.insert(format!("tconstruct:{path}"), serde_json::from_slice(&bytes).unwrap());
            } else if let Some(path) = name.strip_prefix("assets/tconstruct/textures/").and_then(|s| s.strip_suffix(".png")) {
                available.insert(format!("tconstruct:{path}"));
            }
        }
        assert_eq!(records.len(), 145);
        for id in records.keys() {
            let (ns, path) = id.split_once(':').unwrap();
            let material = match path.rsplit_once('/') { Some((base, variant)) => format!("{ns}:{base}#{variant}"), None => id.clone() };
            render_info(&records, &material).unwrap_or_else(|e| panic!("{id}: {e}"));
        }
        for (material, suffix) in [("tconstruct:flint", "tconstruct_flint"), ("tconstruct:wood", "tconstruct_wood"), ("tconstruct:copper#oxidized", "tconstruct_copper_oxidized")] {
            let info = render_info(&records, material).unwrap().unwrap();
            assert_eq!(select_sprite("tconstruct:item/tool/pickaxe/head", &info, &available).unwrap().0,
                format!("tconstruct:item/tool/pickaxe/head_{suffix}"));
        }
        let root = path.ancestors().find(|p| p.join("content/reference/tconstruct-storage-visuals").is_dir()).unwrap();
        let textures = root.join("content/reference/tconstruct-storage-visuals/assets/tconstruct/textures/block/storage");
        let exists = |id: &str| id.strip_prefix("tconstruct:block/storage/")
            .is_some_and(|name| textures.join(format!("{name}.png")).is_file());
        assert_eq!(storage_sprite_by(&records, "tconstruct:unknown", exists), Ok(None));
        assert_eq!(storage_sprite_by(&records, "tconstruct:not_a_material", exists), Ok(None));
        let bronze = storage_sprite_by(&records, "tconstruct:bronze", exists).unwrap().unwrap();
        assert_eq!(bronze.0, "tconstruct:block/storage/fallback_tconstruct_bronze");
        assert_eq!(bronze.1, u32::MAX);
        let info = render_info(&records, "tconstruct:wood").unwrap().unwrap();
        assert_eq!(storage_sprite_by(&records, "tconstruct:wood", |_| false).unwrap(),
            Some(("tconstruct:block/storage/fallback".into(), info.color, info.luminosity)));
    }
}
