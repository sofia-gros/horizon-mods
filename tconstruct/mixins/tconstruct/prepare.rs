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
        unpack_pack(&content_path.join("reference/tconstruct-station-layouts.zip"), &original_pack, &["data/tconstruct/tinkering/station_layouts"])?;
        unpack_pack(&content_path.join("reference/tconstruct-fluid-tags.zip"), &original_pack, &["data/c/tags/fluids", "data/forge/tags/fluids", "data/mantle/tags/fluids", "data/tconstruct/tags/fluids"])?;
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
        // Give supported Creative tool variants the same source-derived base
        // stats and mining rules as a tool assembled in the station. Other tool
        // categories keep their material identities until their stats are ported.
        let creative_path = out.join("creative-inventory.json");
        if creative_path.is_file() {
            let mut creative: serde_json::Value = serde_json::from_slice(&std::fs::read(&creative_path)?)?;
            if let Some(tabs) = creative.get_mut("tabs").and_then(serde_json::Value::as_array_mut) {
                for tab in tabs.iter_mut().filter(|tab| tab.get("id").and_then(serde_json::Value::as_str) == Some("tconstruct:tools")) {
                    if let Some(stacks) = tab.get_mut("items").and_then(serde_json::Value::as_array_mut) {
                        for value in stacks {
                            let Some(raw) = value.as_str() else { continue; };
                            let Ok(raw) = serde_json::from_str::<serde_json::Value>(raw) else { continue; };
                            let Some(item) = raw.get("id").and_then(serde_json::Value::as_str) else { continue; };
                            let Some(short) = item.strip_prefix("tconstruct:") else { continue; };
                            let Some(ids) = raw.pointer("/components/minecraft:custom_data/tic_materials").and_then(serde_json::Value::as_array) else { continue; };
                            let Some(ids) = ids.iter().map(|id| id.as_str().map(str::to_owned)).collect::<Option<Vec<_>>>() else { continue; };
                            let Ok(definition) = std::fs::read(original_pack.join(format!("data/tconstruct/tinkering/tool_definitions/{short}.json"))) else { continue; };
                            let mut maps = Vec::new();
                            for id in &ids {
                                let Some(name) = id.strip_prefix("tconstruct:") else { maps.clear(); break; };
                                let Ok(bytes) = std::fs::read(original_pack.join(format!("data/tconstruct/tinkering/materials/stats/{name}.json"))) else { maps.clear(); break; };
                                let Ok(merged) = crate::tconstruct_tx::source_merge_material_stats(&[bytes]) else { maps.clear(); break; };
                                maps.push(merged.into_iter().map(|(key, fields)| (key, serde_json::Value::Object(fields))).collect());
                            }
                            if maps.len() != ids.len() { continue; }
                            if let Some(mut stack) = crate::tconstruct_source_station_world::assemble_with_stats(&definition, item, &ids, &maps, None) {
                                let traits = crate::tconstruct_source_material_traits::levels_from_definition(&definition, &ids, |id| {
                                    let (namespace, path) = id.split_once(':')?;
                                    std::fs::read(original_pack.join(format!("data/{namespace}/{path}.json"))).ok().map(|bytes| vec![bytes])
                                });
                                if let (Some(traits), Some(stats)) = (traits, crate::tconstruct_source_station_stats::base_stats(&definition, &maps)) {
                                    if crate::tconstruct_source_material_effects::apply_stat_traits(&mut stack, &stats, &traits).is_none() { continue; }
                                }
                                *value = serde_json::Value::String(crate::snbt::to_snbt(&horizon_items::item_nbt::item_to_nbt(&stack)));
                            }
                        }
                    }
                }
            }
            // The prepared output initially links to the input overlay. Replace
            // the link itself so the original creative data stays untouched.
            let temp = out.join("creative-inventory.json.tmp");
            std::fs::write(&temp, serde_json::to_vec(&creative)?)?;
            std::fs::remove_file(&creative_path)?;
            std::fs::rename(temp, creative_path)?;
        }
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
        // Vanilla honey has the same 1/16 inset collision box as source
        // congealed slime. Copy only its shape, never its movement behavior.
        let shape_targets = ["earth", "sky", "ichor", "ender"].map(|kind| format!("tconstruct:{kind}_congealed_slime"));
        let targets: Vec<usize> = shape_targets.iter().filter_map(|name| facts["defaults"][name.as_str()].as_u64().map(|id| id as usize)).collect();
        if !targets.is_empty() {
            let honey = facts["defaults"]["minecraft:honey_block"].as_u64().ok_or("missing honey block")? as usize;
            for name in ["cached-collision.nbt", "worldgen/blocks/cached-collision.nbt"] {
                let path = out.join(name);
                let (mut named, _) = horizon_world::nbt::read_named(&std::fs::read(&path)?, horizon_world::nbt::Limits::tables())?;
                tconstruct_block_shapes::apply_congealed_shape(&mut named.tag, honey, &targets)?;
                let bytes = horizon_world::nbt::write_named(&named, horizon_world::nbt::Limits::tables())?;
                std::fs::write(path, bytes)?;
            }
        }
        // Source soul glass has no collision and slows entities while inside.
        let soul_names = ["soul_glass", "soul_glass_pane", "seared_soul_glass", "seared_soul_glass_pane", "scorched_soul_glass", "scorched_soul_glass_pane"];
        let soul_ids: Vec<usize> = facts["states"].as_array().ok_or("missing state list")?.iter().enumerate()
            .filter_map(|(id, row)| row.get(0).and_then(serde_json::Value::as_str)
                .and_then(|name| name.strip_prefix("tconstruct:"))
                .filter(|name| soul_names.contains(name)).map(|_| id)).collect();
        if !soul_ids.is_empty() {
            let air = facts["defaults"]["minecraft:air"].as_u64().ok_or("missing air state")? as usize;
            for name in ["cached-collision.nbt", "worldgen/blocks/cached-collision.nbt"] {
                let path = out.join(name);
                let (mut named, _) = horizon_world::nbt::read_named(&std::fs::read(&path)?, horizon_world::nbt::Limits::tables())?;
                tconstruct_block_shapes::apply_empty_shape(&mut named.tag, air, &soul_ids)?;
                std::fs::write(path, horizon_world::nbt::write_named(&named, horizon_world::nbt::Limits::tables())?)?;
            }
            let path = out.join("movement-properties.nbt");
            let (mut named, _) = horizon_world::nbt::read_named(&std::fs::read(&path)?, horizon_world::nbt::Limits::tables())?;
            let targets: Vec<(String, f32)> = soul_names.iter().map(|name|
                (format!("tconstruct:{name}"), if name.starts_with("soul_") { 0.2 } else { 0.1 })).collect();
            let targets: Vec<(&str, f32)> = targets.iter().filter(|(name, _)| facts["defaults"][name.as_str()].as_u64().is_some())
                .map(|(name, speed)| (name.as_str(), *speed)).collect();
            tconstruct_block_shapes::apply_soul_speed(&mut named.tag, &targets)?;
            std::fs::write(path, horizon_world::nbt::write_named(&named, horizon_world::nbt::Limits::tables())?)?;
        }
        let path = out.join("gameplay-facts.json");
        let mut gameplay: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        for name in ["tconstruct:part_builder", "tconstruct:tinker_station"] {
            let id = facts["defaults"][name].as_u64().ok_or("missing station block state")? as usize;
            let row = gameplay["states"].as_array_mut().and_then(|rows| rows.get_mut(id)).ok_or("missing station gameplay row")?;
            row["block_entity"] = serde_json::Value::Bool(true);
            // The table has legs and a top rather than a full opaque cube.
            // Its vanilla template must not hide the ground between its legs.
            row["solid_render"] = serde_json::Value::Bool(false);
            row["can_occlude"] = serde_json::Value::Bool(false);
        }
        // Source tables have legs and slime-metal blocks declare no occlusion.
        // Apply this to every state of those blocks.
        if let (Some(states), Some(rows)) = (facts["states"].as_array(), gameplay["states"].as_array_mut()) {
            for (id, state) in states.iter().enumerate() {
                if matches!(state.get(0).and_then(serde_json::Value::as_str), Some("tconstruct:crafting_station" | "tconstruct:modifier_worktable" | "tconstruct:soulsteel_block" | "tconstruct:slimesteel_block" | "tconstruct:cinderslime_block" | "tconstruct:knightslime_block")) {
                    let row = rows.get_mut(id).ok_or("missing table gameplay row")?;
                    row["solid_render"] = serde_json::Value::Bool(false);
                    row["can_occlude"] = serde_json::Value::Bool(false);
                }
            }
        }
        if let Some(id) = facts["defaults"]["tconstruct:fake_storage_block"].as_u64() {
            let row = gameplay["states"].as_array_mut().and_then(|rows| rows.get_mut(id as usize)).ok_or("missing material block gameplay row")?;
            row["block_entity"] = serde_json::Value::Bool(true);
        }
        let temp = out.join("gameplay-facts.json.tmp");
        std::fs::write(&temp, serde_json::to_vec(&gameplay)?)?;
        std::fs::rename(temp, path)?;
        let path = out.join("render-definitions.json");
        let mut render: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        for name in ["tconstruct:part_builder", "tconstruct:tinker_station"] {
            let row = render.get_mut(name).and_then(serde_json::Value::as_object_mut).ok_or("missing station render definition")?;
            row.insert("can_occlude".into(), serde_json::Value::Bool(false));
        }
        for name in ["tconstruct:crafting_station", "tconstruct:modifier_worktable", "tconstruct:soulsteel_block", "tconstruct:slimesteel_block", "tconstruct:cinderslime_block", "tconstruct:knightslime_block"] {
            if let Some(row) = render.get_mut(name).and_then(serde_json::Value::as_object_mut) {
                row.insert("can_occlude".into(), serde_json::Value::Bool(false));
            }
        }
        let temp = out.join("render-definitions.json.tmp");
        std::fs::write(&temp, serde_json::to_vec(&render)?)?;
        std::fs::rename(temp, path)?;
        Ok(())
    })();
    match result {
        Ok(()) => std::process::exit(0),
        Err(error) => { eprintln!("tconstruct resource preparation: {error}"); std::process::exit(1); }
    }
}
