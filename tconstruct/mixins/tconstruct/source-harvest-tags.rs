// Project source harvest predicates onto available block IDs without changing
// the vanilla tags. Each source tag is written under a TConstruct-only alias.
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

type Error = Box<dyn std::error::Error>;

pub(super) fn alias(tag: &str) -> Option<String> {
    let (namespace, path) = tag.split_once(':')?;
    if namespace.is_empty() || path.is_empty() || path.split('/').any(|part| part.is_empty() || part == "." || part == "..")
        || !tag.bytes().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || b"_:./-".contains(&ch)) { return None; }
    Some(format!("tconstruct:source_effective/{namespace}/{path}"))
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.is_dir() { return Ok(()); }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() { files(&path, out)?; }
        else if path.extension().is_some_and(|ext| ext == "json") { out.push(path); }
    }
    Ok(())
}

fn merge_directory(root: &Path, tag_directory: &str, raw: &mut BTreeMap<String, Vec<Vec<u8>>>) -> Result<(), Error> {
    let data = root.join("data");
    if !data.is_dir() { return Ok(()); }
    let mut namespaces: Vec<_> = std::fs::read_dir(&data)?.filter_map(Result::ok).map(|entry| entry.path()).collect();
    namespaces.sort();
    for namespace in namespaces {
        let Some(ns) = namespace.file_name().and_then(|name| name.to_str()) else { continue; };
        let base = namespace.join("tags").join(tag_directory);
        let mut paths = Vec::new(); files(&base, &mut paths)?; paths.sort();
        for path in paths {
            let name = path.strip_prefix(&base)?.with_extension("").to_string_lossy().replace('\\', "/");
            raw.entry(format!("{ns}:{name}")).or_default().push(std::fs::read(path)?);
        }
    }
    Ok(())
}

pub(super) fn prepare(out: &Path, original: &Path, forge_archive: &Path) -> Result<usize, Error> {
    let mut tags = BTreeMap::new();
    merge_directory(out, "block", &mut tags)?;
    let mut forge = zip::ZipArchive::new(std::fs::File::open(forge_archive)?)?;
    for index in 0..forge.len() {
        use std::io::Read;
        let mut entry = forge.by_index(index)?;
        let name = entry.name().to_owned();
        let Some(path) = name.strip_prefix("data/forge/tags/blocks/").and_then(|value| value.strip_suffix(".json")) else { continue; };
        if path.split('/').any(|part| part.is_empty() || part == "." || part == "..") { return Err("invalid Forge harvest tag path".into()); }
        let mut bytes = Vec::new(); entry.read_to_end(&mut bytes)?;
        tags.entry(format!("forge:{path}")).or_default().push(bytes);
    }
    merge_directory(original, "blocks", &mut tags)?;
    let facts: serde_json::Value = serde_json::from_slice(&std::fs::read(out.join("facts.json"))?)?;
    let registry: HashSet<String> = facts.get("states").and_then(serde_json::Value::as_array).ok_or("missing block states")?.iter()
        .filter_map(|state| state.get(0).and_then(serde_json::Value::as_str).map(str::to_owned)).collect();
    let definitions = original.join("data/tconstruct/tinkering/tool_definitions");
    let mut paths = Vec::new(); files(&definitions, &mut paths)?; paths.sort();
    let mut wanted = BTreeSet::new();
    for path in paths {
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        for module in value.get("modules").and_then(serde_json::Value::as_array).into_iter().flatten() {
            if module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:is_effective")
                && module.get("predicate_type").and_then(serde_json::Value::as_str) == Some("mantle:tag") {
                wanted.insert(module.get("tag").and_then(serde_json::Value::as_str).ok_or("missing effective tag")?.to_owned());
            }
        }
    }
    for tag in &wanted {
        let alias = alias(tag).ok_or("invalid effective tag ID")?;
        let blocks = super::tconstruct_source_tags::resolve_available_blocks(tag, &tags, &registry)?;
        let relative = alias.strip_prefix("tconstruct:").ok_or("invalid alias")?;
        let path = out.join("data/tconstruct/tags/block").join(format!("{relative}.json"));
        std::fs::create_dir_all(path.parent().ok_or("invalid alias path")?)?;
        std::fs::write(path, serde_json::to_vec(&serde_json::json!({"values": blocks}))?)?;
    }
    Ok(wanted.len())
}
