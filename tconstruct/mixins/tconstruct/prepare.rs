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
        // The melter stores fluid IDs in tanks, without registering those
        // fluids as placeable vanilla world fluids.
        std::fs::copy(content_path.join("reference/tconstruct-melter-fluids.json"), out.join("tconstruct-melter-fluids.json"))?;
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
        unpack_pack(&content_path.join("reference/tconstruct-station-layouts.zip"), &original_pack, &["data/tconstruct/tinkering/station_layouts"])?;
        unpack_pack(&content_path.join("reference/tconstruct-fluid-tags.zip"), &original_pack, &["data/c/tags/fluids", "data/forge/tags/fluids", "data/mantle/tags/fluids", "data/tconstruct/tags/fluids"])?;
        unpack_pack(&content_path.join("reference/tconstruct-block-tags.zip"), &original_pack, &["data/ceramics/tags/blocks", "data/create/tags/blocks", "data/forge/tags/blocks", "data/mantle/tags/blocks", "data/minecraft/tags/blocks", "data/tconstruct/tags/blocks"])?;
        // Horizon 26.3 already has the copper nugget and its ordinary
        // crafting recipes. Adapt source-pack references to the native item.
        let items: serde_json::Value = serde_json::from_slice(&std::fs::read(out.join("items.json"))?)?;
        if items.get("minecraft:copper_nugget").is_some() {
            let tag = original_pack.join("data/forge/tags/items/nuggets/copper.json");
            let source = std::fs::read_to_string(&tag)?;
            std::fs::write(tag, source.replace("tconstruct:copper_nugget", "minecraft:copper_nugget"))?;
            for (original, native) in [
                ("copper_ingot_from_nuggets", "copper_ingot_from_nuggets"),
                ("copper_nugget_from_ingot", "copper_nugget"),
            ] {
                let path = original_pack.join(format!("data/tconstruct/recipes/common/materials/{original}.json"));
                if base.join(format!("data/minecraft/recipe/{native}.json")).is_file() {
                    std::fs::remove_file(path)?;
                } else {
                    let source = std::fs::read_to_string(&path)?;
                    std::fs::write(path, source.replace("tconstruct:copper_nugget", "minecraft:copper_nugget"))?;
                }
            }
        }
        // Project original harvest predicates onto installed blocks under
        // TConstruct-only aliases; native vanilla tags stay untouched.
        crate::tconstruct_source_harvest_tags::prepare(&out, &original_pack, &content_path.join("reference/forge-47.4.0-harvest-tags.zip"))?;
        horizon_client::tconstruct_resource_tables::creative(&out, &original_pack)?;
        for warning in warnings { eprintln!("{warning}"); }
        horizon_client::tconstruct_resource_tables::apply(&out, &content_path)?;
        Ok(())
    })();
    match result {
        Ok(()) => std::process::exit(0),
        Err(error) => { eprintln!("tconstruct resource preparation: {error}"); std::process::exit(1); }
    }
}
