fn prepare_tconstruct_resources() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_none_or(|arg| arg != "--tconstruct-prepare") { return; }
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        if args.len() != 5 { return Err("usage: --tconstruct-prepare BASE CONTENT OUTPUT".into()); }
        let base = std::path::PathBuf::from(&args[2]).canonicalize()?;
        let content_path = std::path::PathBuf::from(&args[3]).canonicalize()?;
        let requested = std::path::PathBuf::from(&args[4]);
        let parent = requested.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
        let name = requested.file_name().ok_or("output needs a final directory name")?;
        let out = parent.canonicalize()?.join(name);
        if std::fs::symlink_metadata(&out).is_ok() || out.starts_with(&base) || base.starts_with(&out) || out.starts_with(&content_path) || content_path.starts_with(&out) {
            return Err("output must not exist and must be separate from both inputs".into());
        }
        let manifest = horizon_mods::manifest::Manifest::parse("id = 'tconstruct'\nname = 'TConstruct assets'\nversion = '0.1.0'\napi = '0.2'\ncapabilities = ['registries']\n")?;
        let content = horizon_mods::content::Content::read(&content_path, &manifest)?;
        let content_mod = crate::mod_content::ContentMod { id: "tconstruct".into(), content };
        let warnings = crate::mod_content::build(&base, &[content_mod], &out)?;
        // Preserve the original 1.20.1 pack as its own layer: Horizon's
        // flattened content builder is not Forge's ordered tag-file loader.
        fn copy_pack_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
            std::fs::create_dir_all(dst)?;
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                let to = dst.join(entry.file_name());
                if kind.is_dir() { copy_pack_dir(&entry.path(), &to)?; }
                else if kind.is_file() { std::fs::copy(entry.path(), to)?; }
                else { return Err("source pack contains a non-file entry".into()); }
            }
            Ok(())
        }
        let original_pack = out.join("tconstruct-original-pack");
        if std::fs::symlink_metadata(&original_pack).is_ok() { return Err("original pack output already exists".into()); }
        copy_pack_dir(&content_path.join("reference/tconstruct-tags/data"), &original_pack.join("data"))?;
        fn unpack_pack(src: &std::path::Path, dst: &std::path::Path, prefixes: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
            let mut archive = zip::ZipArchive::new(std::fs::File::open(src)?)?;
            for index in 0..archive.len() {
                let mut entry = archive.by_index(index)?;
                let path = entry.enclosed_name().ok_or("invalid original data path")?;
                if entry.is_dir() || !prefixes.iter().any(|prefix| path.starts_with(prefix)) || path.extension().is_none_or(|ext| ext != "json") {
                    return Err("invalid original data archive entry".into());
                }
                let target = dst.join(path);
                if target.exists() { return Err("duplicate original data path".into()); }
                std::fs::create_dir_all(target.parent().ok_or("missing data parent")?)?;
                std::io::copy(&mut entry, &mut std::fs::File::create(target)?)?;
            }
            Ok(())
        }
        unpack_pack(&content_path.join("reference/tconstruct-recipes.zip"), &original_pack, &["data/tconstruct/recipes"])?;
        unpack_pack(&content_path.join("reference/tconstruct-materials.zip"), &original_pack, &["data/tconstruct/tinkering/materials", "data/tconstruct/tinkering/tags/materials"])?;
        unpack_pack(&content_path.join("reference/tconstruct-tool-definitions.zip"), &original_pack, &["data/tconstruct/tinkering/tool_definitions"])?;
        unpack_pack(&content_path.join("reference/tconstruct-fluid-tags.zip"), &original_pack, &["data/c/tags/fluids", "data/forge/tags/fluids", "data/mantle/tags/fluids", "data/tconstruct/tags/fluids"])?;
        for warning in warnings { eprintln!("{warning}"); }
        // The vanilla table templates do not declare block entities. Our
        // containers must be visited by Horizon's normal removal/save path.
        let facts: serde_json::Value = serde_json::from_slice(&std::fs::read(out.join("facts.json"))?)?;
        // Register the source block entity type only when the content pack
        // actually registers the corresponding block. Its wire ID remains
        // local to this generated registry, never a guessed constant.
        if facts["defaults"]["tconstruct:fake_storage_block"].as_u64().is_some() {
            let path = out.join("registries.json");
            let mut registries: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
            let entries = registries["minecraft:block_entity_type"]["entries"].as_object_mut().ok_or("missing block entity registry")?;
            if !entries.contains_key("tconstruct:material_block") {
                let next = entries.values().filter_map(|v| v["protocol_id"].as_u64()).max().ok_or("empty block entity registry")? + 1;
                entries.insert("tconstruct:material_block".to_owned(), serde_json::json!({"protocol_id": next}));
            }
            let temp = out.join("registries.json.tmp");
            std::fs::write(&temp, serde_json::to_vec(&registries)?)?;
            std::fs::rename(temp, path)?;
        }
        let path = out.join("gameplay-facts.json");
        let mut gameplay: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        for name in ["tconstruct:part_builder", "tconstruct:tinker_station"] {
            let id = facts["defaults"][name].as_u64().ok_or("missing station block state")? as usize;
            let row = gameplay["states"].as_array_mut().and_then(|rows| rows.get_mut(id)).ok_or("missing station gameplay row")?;
            row["block_entity"] = serde_json::Value::Bool(true);
        }
        if let Some(id) = facts["defaults"]["tconstruct:fake_storage_block"].as_u64() {
            let row = gameplay["states"].as_array_mut().and_then(|rows| rows.get_mut(id as usize)).ok_or("missing material block gameplay row")?;
            row["block_entity"] = serde_json::Value::Bool(true);
        }
        let temp = out.join("gameplay-facts.json.tmp");
        std::fs::write(&temp, serde_json::to_vec(&gameplay)?)?;
        std::fs::rename(temp, path)?;
        Ok(())
    })();
    match result {
        Ok(()) => std::process::exit(0),
        Err(error) => { eprintln!("tconstruct resource preparation: {error}"); std::process::exit(1); }
    }
}
