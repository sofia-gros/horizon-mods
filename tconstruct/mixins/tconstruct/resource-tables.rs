// Shared post-processing for manually prepared and startup content roots.
pub fn apply(out: &std::path::Path, content_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        // The guide's pinned source images join the same GUI atlas in both
        // manual and startup preparation. Only its own bundled PNGs are read.
        let guide = content_path.join("assets/tconstruct/book/images");
        if guide.is_dir() {
            fn copy_images(root: &std::path::Path, dir: &std::path::Path, target: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    let ty = entry.file_type()?;
                    let path = entry.path();
                    if ty.is_dir() { copy_images(root, &path, target)?; continue; }
                    if !ty.is_file() { continue; }
                    let name = path.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
                    if !name.ends_with(".png") || !name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c)) || name.contains("..") { continue; }
                    let sprite = format!("tconstruct_book_{}", name.trim_end_matches(".png").replace(['/', '-'], "_"));
                    std::fs::copy(path, target.join(format!("{sprite}.png")))?;
                }
                Ok(())
            }
            let target = out.join("assets/minecraft/textures/gui");
            std::fs::create_dir_all(&target)?;
            copy_images(&guide, &guide, &target)?;
        }
        // Project source geode and cobalt biome modifiers into the generated bundle,
        // preserving source registration order and the vanilla base pack.
        if content_path.join("data/tconstruct/worldgen/placed_feature/sky_geode.json").is_file()
            || content_path.join("data/tconstruct/worldgen/placed_feature/cobalt_ore_small.json").is_file() {
            fn tag_members(tags: &serde_json::Value, id: &str, visited: &mut std::collections::HashSet<String>, out: &mut std::collections::HashSet<String>) -> Result<(), Box<dyn std::error::Error>> {
                if !visited.insert(id.to_owned()) { return Ok(()); }
                let raw = tags.get(id).and_then(|v| v.as_str()).ok_or_else(|| format!("missing biome tag {id}"))?;
                let value: serde_json::Value = serde_json::from_str(raw)?;
                for entry in value["values"].as_array().ok_or("invalid biome tag")? {
                    let entry = entry.as_str().or_else(|| entry["id"].as_str()).ok_or("invalid biome tag member")?;
                    if let Some(nested) = entry.strip_prefix('#') { tag_members(tags, nested, visited, out)?; }
                    else { out.insert(entry.to_owned()); }
                }
                Ok(())
            }
            let tags: serde_json::Value = serde_json::from_slice(&std::fs::read(out.join("worldgen/biome_tags.json"))?)?;
            let mut overworld = std::collections::HashSet::new();
            let mut nether = std::collections::HashSet::new();
            let mut end = std::collections::HashSet::new();
            tag_members(&tags, "minecraft:is_overworld", &mut Default::default(), &mut overworld)?;
            tag_members(&tags, "minecraft:is_nether", &mut Default::default(), &mut nether)?;
            tag_members(&tags, "minecraft:is_end", &mut Default::default(), &mut end)?;
            let mut excluded = std::collections::HashSet::new();
            for id in ["minecraft:is_ocean", "minecraft:is_deep_ocean", "minecraft:is_beach", "minecraft:is_river"] {
                tag_members(&tags, id, &mut Default::default(), &mut excluded)?;
            }
            let sky: std::collections::HashSet<_> = overworld.difference(&excluded).cloned().collect();
            end.remove("minecraft:the_end");
            let path = out.join("worldgen/biomes.json");
            let mut biomes: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
            for (name, eligible) in [("earth", &overworld), ("sky", &sky), ("ichor", &nether), ("ender", &end)] {
                if !content_path.join(format!("data/tconstruct/worldgen/placed_feature/{name}_geode.json")).is_file() { continue; }
                if eligible.is_empty() { return Err(format!("no eligible {name} geode biomes").into()); }
                let feature = format!("tconstruct:{name}_geode");
                for id in eligible {
                    let raw = biomes.get_mut(id).ok_or_else(|| format!("missing biome {id}"))?;
                    let mut biome: serde_json::Value = serde_json::from_str(raw.as_str().ok_or("invalid biome")?)?;
                    let features = biome["features"].as_array_mut().ok_or("missing biome features")?;
                    while features.len() < 3 { features.push(serde_json::json!([])); }
                    let local = features.get_mut(2).and_then(|v| v.as_array_mut()).ok_or("missing local modifications")?;
                    if !local.iter().any(|v| v.as_str() == Some(&feature)) {
                        local.push(serde_json::Value::String(feature.clone()));
                    }
                    *raw = serde_json::Value::String(serde_json::to_string(&biome)?);
                }
            }
            // TConstruct adds both ore variants to Nether underground decoration.
            for name in ["cobalt_ore_small", "cobalt_ore_large"] {
                if !content_path.join(format!("data/tconstruct/worldgen/placed_feature/{name}.json")).is_file() { continue; }
                if nether.is_empty() { return Err("no eligible cobalt ore biomes".into()); }
                let feature = format!("tconstruct:{name}");
                for id in &nether {
                    let raw = biomes.get_mut(id).ok_or_else(|| format!("missing biome {id}"))?;
                    let mut biome: serde_json::Value = serde_json::from_str(raw.as_str().ok_or("invalid biome")?)?;
                    let features = biome["features"].as_array_mut().ok_or("missing biome features")?;
                    while features.len() < 8 { features.push(serde_json::json!([])); }
                    let underground = features.get_mut(7).and_then(|v| v.as_array_mut()).ok_or("missing underground decoration")?;
                    if !underground.iter().any(|v| v.as_str() == Some(&feature)) { underground.push(serde_json::Value::String(feature.clone())); }
                    *raw = serde_json::Value::String(serde_json::to_string(&biome)?);
                }
            }
            let temp = out.join("worldgen/biomes.json.tmp");
            std::fs::write(&temp, serde_json::to_vec(&biomes)?)?;
            std::fs::rename(temp, path)?;
        }
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
        // Register the source machine IDs for persisted block entities. The
        // five machines use chest-like live inventories, but their custom IDs
        // keep machine metadata outside vanilla chest canonicalization.
        let machines = [("tconstruct:crafting_station", "tconstruct:crafting_station"), ("tconstruct:part_chest", "tconstruct:part_chest"), ("tconstruct:tinkers_chest", "tconstruct:modifier_chest"),
            ("tconstruct:tinkers_anvil", "tconstruct:anvil"), ("tconstruct:scorched_anvil", "tconstruct:anvil"),
            ("tconstruct:seared_melter", "tconstruct:melter"), ("tconstruct:seared_heater", "tconstruct:heater"),
            ("tconstruct:seared_faucet", "tconstruct:faucet"), ("tconstruct:seared_table", "tconstruct:table"),
            ("tconstruct:seared_basin", "tconstruct:basin"), ("tconstruct:smeltery_controller", "tconstruct:smeltery"),
            ("tconstruct:foundry_controller", "tconstruct:smeltery"),
            ("tconstruct:seared_fuel_tank", "tconstruct:fuel_tank"), ("tconstruct:seared_fuel_gauge", "tconstruct:fuel_tank"),
            ("tconstruct:seared_ingot_tank", "tconstruct:ingot_tank"), ("tconstruct:seared_ingot_gauge", "tconstruct:ingot_tank"),
            ("tconstruct:scorched_fuel_tank", "tconstruct:fuel_tank"), ("tconstruct:scorched_fuel_gauge", "tconstruct:fuel_tank"),
            ("tconstruct:scorched_ingot_tank", "tconstruct:ingot_tank"), ("tconstruct:scorched_ingot_gauge", "tconstruct:ingot_tank")];
        let path = out.join("registries.json");
        let mut registries: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        let entries = registries["minecraft:block_entity_type"]["entries"].as_object_mut().ok_or("missing block entity registry")?;
        let mut next = entries.values().filter_map(|value| value["protocol_id"].as_u64()).max().ok_or("empty block entity registry")? + 1;
        for (block, kind) in machines {
            if facts["defaults"][block].as_u64().is_some() && !entries.contains_key(kind) {
                entries.insert(kind.to_owned(), serde_json::json!({"protocol_id": next})); next += 1;
            }
        }
        let temp = out.join("registries.json.tmp");
        std::fs::write(&temp, serde_json::to_vec(&registries)?)?;
        std::fs::rename(temp, path)?;
        // Vanilla honey has the same 1/16 inset collision box as source
        // congealed slime. Copy only its shape, never its movement behavior.
        let shape_targets = ["earth", "sky", "ichor", "ender"].map(|kind| format!("tconstruct:{kind}_congealed_slime"));
        let targets: Vec<usize> = shape_targets.iter().filter_map(|name| facts["defaults"][name.as_str()].as_u64().map(|id| id as usize)).collect();
        if !targets.is_empty() {
            let honey = facts["defaults"]["minecraft:honey_block"].as_u64().ok_or("missing honey block")? as usize;
            for name in ["cached-collision.nbt", "worldgen/blocks/cached-collision.nbt"] {
                let path = out.join(name);
                let (mut named, _) = horizon_world::nbt::read_named(&std::fs::read(&path)?, horizon_world::nbt::Limits::tables())?;
                super::tconstruct_block_shapes::apply_congealed_shape(&mut named.tag, honey, &targets)?;
                let bytes = horizon_world::nbt::write_named(&named, horizon_world::nbt::Limits::tables())?;
                std::fs::write(path, bytes)?;
            }
        }
        // Source glow is a six-directional, non-colliding light source.
        // Use the same empty collision for every orientation in both tables.
        let glow_ids: Vec<usize> = facts["states"].as_array().ok_or("missing state list")?.iter().enumerate()
            .filter_map(|(id, row)| (row.get(0).and_then(serde_json::Value::as_str) == Some("tconstruct:glow")).then_some(id)).collect();
        if !glow_ids.is_empty() {
            let air = facts["defaults"]["minecraft:air"].as_u64().ok_or("missing air state")? as usize;
            for name in ["cached-collision.nbt", "worldgen/blocks/cached-collision.nbt"] {
                let path = out.join(name);
                let (mut named, _) = horizon_world::nbt::read_named(&std::fs::read(&path)?, horizon_world::nbt::Limits::tables())?;
                super::tconstruct_block_shapes::apply_empty_shape(&mut named.tag, air, &glow_ids)?;
                std::fs::write(path, horizon_world::nbt::write_named(&named, horizon_world::nbt::Limits::tables())?)?;
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
                super::tconstruct_block_shapes::apply_empty_shape(&mut named.tag, air, &soul_ids)?;
                std::fs::write(path, horizon_world::nbt::write_named(&named, horizon_world::nbt::Limits::tables())?)?;
            }
            let path = out.join("movement-properties.nbt");
            let (mut named, _) = horizon_world::nbt::read_named(&std::fs::read(&path)?, horizon_world::nbt::Limits::tables())?;
            let targets: Vec<(String, f32)> = soul_names.iter().map(|name|
                (format!("tconstruct:{name}"), if name.starts_with("soul_") { 0.2 } else { 0.1 })).collect();
            let targets: Vec<(&str, f32)> = targets.iter().filter(|(name, _)| facts["defaults"][name.as_str()].as_u64().is_some())
                .map(|(name, speed)| (name.as_str(), *speed)).collect();
            super::tconstruct_block_shapes::apply_soul_speed(&mut named.tag, &targets)?;
            std::fs::write(path, horizon_world::nbt::write_named(&named, horizon_world::nbt::Limits::tables())?)?;
        }
        // FaceVisibility's existing render flags have no custom pane kind.
        // Reserve (same_block, bars_tag) for these source pane definitions;
        // the connected-pane-face hook interprets the pair and restores the
        // normal iron-bars interaction for neighboring vanilla blocks.
        let path = out.join("render-definitions.json");
        let mut renders: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        let definitions = renders.as_object_mut().ok_or("missing render definitions")?;
        for (name, row) in definitions.iter_mut() {
            if !super::tconstruct_block_shapes::is_source_connected_pane(name) { continue; }
            if row["rule"].as_str() != Some("bars") || row["bars_tag"].as_bool() != Some(false) {
                return Err(format!("unexpected pane render definition: {name}").into());
            }
            row["rule"] = serde_json::Value::String("same_block".into());
            row["bars_tag"] = serde_json::Value::Bool(true);
        }
        let temp = out.join("render-definitions.json.tmp");
        std::fs::write(&temp, serde_json::to_vec(&renders)?)?;
        std::fs::rename(temp, path)?;
        let path = out.join("gameplay-facts.json");
        let mut gameplay: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        for name in ["tconstruct:part_builder", "tconstruct:tinker_station", "tconstruct:modifier_worktable"] {
            let id = facts["defaults"][name].as_u64().ok_or("missing station block state")? as usize;
            let row = gameplay["states"].as_array_mut().and_then(|rows| rows.get_mut(id)).ok_or("missing station gameplay row")?;
            row["block_entity"] = serde_json::Value::Bool(true);
            // The table has legs and a top rather than a full opaque cube.
            // Its vanilla template must not hide the ground between its legs.
            row["solid_render"] = serde_json::Value::Bool(false);
            row["can_occlude"] = serde_json::Value::Bool(false);
        }
        // Each source machine keeps its state in a block entity. All but
        // the full heater have visible gaps and must not hide neighboring faces.
        if let (Some(states), Some(rows)) = (facts["states"].as_array(), gameplay["states"].as_array_mut()) {
            for (id, state) in states.iter().enumerate() {
                let Some(name) = state.get(0).and_then(serde_json::Value::as_str) else { continue; };
                if !matches!(name, "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" | "tconstruct:seared_melter" | "tconstruct:seared_heater" | "tconstruct:seared_faucet" | "tconstruct:seared_table" | "tconstruct:seared_basin" | "tconstruct:smeltery_controller" | "tconstruct:foundry_controller" | "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge") { continue; }
                let row = rows.get_mut(id).ok_or("missing machine gameplay row")?;
                row["block_entity"] = serde_json::Value::Bool(true);
                if !matches!(name, "tconstruct:seared_heater" | "tconstruct:smeltery_controller" | "tconstruct:foundry_controller") {
                    row["solid_render"] = serde_json::Value::Bool(false);
                    row["can_occlude"] = serde_json::Value::Bool(false);
                }
            }
        }
        super::tconstruct_machine_prepare::apply(&facts, &mut gameplay, out)?;
        // Glow selection is a 1/16-thick sheet on its supporting face; its
        // collision stays empty in both collision tables above.
        if let (Some(states), Some(rows)) = (facts["states"].as_array(), gameplay["states"].as_array_mut()) {
            for (id, state) in states.iter().enumerate() {
                if state.get(0).and_then(serde_json::Value::as_str) != Some("tconstruct:glow") { continue; }
                let facing = state.pointer("/4/properties/facing").and_then(serde_json::Value::as_str).ok_or("missing glow facing")?;
                let b: [f64;6] = match facing {
                    "down" => [0.0,0.0,0.0,1.0,0.0625,1.0], "up" => [0.0,0.9375,0.0,1.0,1.0,1.0],
                    "north" => [0.0,0.0,0.0,1.0,1.0,0.0625], "south" => [0.0,0.0,0.9375,1.0,1.0,1.0],
                    "west" => [0.0,0.0,0.0,0.0625,1.0,1.0], "east" => [0.9375,0.0,0.0,1.0,1.0,1.0],
                    _ => return Err("invalid glow facing".into()),
                };
                let mut edges = Vec::new();
                for axis in 0..3 { let a = (axis+1)%3; let c = (axis+2)%3;
                    for aa in [0,1] { for cc in [0,1] {
                        let mut from = [0.0;3]; from[axis]=b[axis]; from[a]=b[a+3*aa]; from[c]=b[c+3*cc];
                        let mut to=from; to[axis]=b[axis+3];
                        edges.push([from[0],from[1],from[2],to[0],to[1],to[2]]);
                    }}
                }
                let row=rows.get_mut(id).ok_or("missing glow gameplay row")?;
                row["outline"] = serde_json::json!([b]); row["outline_edges"] = serde_json::json!(edges);
                row["full_collision"] = serde_json::json!(false); row["solid_render"] = serde_json::json!(false);
                row["can_occlude"] = serde_json::json!(false);
            }
        }
        // Source tables have legs and slime-metal blocks declare no occlusion.
        // Apply this to every state of those blocks.
        if let (Some(states), Some(rows)) = (facts["states"].as_array(), gameplay["states"].as_array_mut()) {
            for (id, state) in states.iter().enumerate() {
                if matches!(state.get(0).and_then(serde_json::Value::as_str), Some("tconstruct:crafting_station" | "tconstruct:modifier_worktable" | "tconstruct:part_chest" | "tconstruct:tinkers_chest" | "tconstruct:soulsteel_block" | "tconstruct:slimesteel_block" | "tconstruct:cinderslime_block" | "tconstruct:knightslime_block")) {
                    let row = rows.get_mut(id).ok_or("missing table gameplay row")?;
                    if matches!(state.get(0).and_then(serde_json::Value::as_str), Some("tconstruct:crafting_station" | "tconstruct:modifier_worktable" | "tconstruct:part_chest" | "tconstruct:tinkers_chest")) { row["block_entity"] = serde_json::Value::Bool(true); }
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
        for name in ["tconstruct:part_builder", "tconstruct:tinker_station", "tconstruct:tinkers_anvil", "tconstruct:scorched_anvil"] {
            let row = render.get_mut(name).and_then(serde_json::Value::as_object_mut).ok_or("missing station render definition")?;
            row.insert("can_occlude".into(), serde_json::Value::Bool(false));
        }
        for name in ["tconstruct:crafting_station", "tconstruct:modifier_worktable", "tconstruct:part_chest", "tconstruct:tinkers_chest", "tconstruct:soulsteel_block", "tconstruct:slimesteel_block", "tconstruct:cinderslime_block", "tconstruct:knightslime_block"] {
            if let Some(row) = render.get_mut(name).and_then(serde_json::Value::as_object_mut) {
                row.insert("can_occlude".into(), serde_json::Value::Bool(false));
            }
        }
        for name in ["tconstruct:seared_melter", "tconstruct:seared_faucet", "tconstruct:seared_table", "tconstruct:seared_basin"] {
            if let Some(row) = render.get_mut(name).and_then(serde_json::Value::as_object_mut) {
                row.insert("can_occlude".into(), serde_json::Value::Bool(false));
            }
        }
        let temp = out.join("render-definitions.json.tmp");
        std::fs::write(&temp, serde_json::to_vec(&render)?)?;
        std::fs::rename(temp, path)?;
        Ok(())
}

/// Derive mod tables in a private game cache, leaving the player's resources
/// and the content loader's own cache untouched.
pub fn startup(root: &mut std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::{Path, PathBuf};
    let base = root.canonicalize()?;
    let cache = std::env::current_dir()?.join(".horizon-cache/tconstruct-startup");
    let marker = cache.join("tconstruct-startup-owner");
    let mut fingerprint = std::fs::read(base.join("horizon-mod-content.stamp"))?;
    // Invalidate only our owned cache when Creative preparation semantics change.
    fingerprint.extend_from_slice(b"\0tconstruct-native-creative-v2");
    if marker.is_file() && std::fs::read(&marker)? == fingerprint {
        *root = cache;
        return Ok(());
    }
    if std::fs::symlink_metadata(&cache).is_ok() {
        if !marker.is_file() || std::fs::symlink_metadata(&cache)?.file_type().is_symlink() {
            return Err("refusing to replace an unowned TConstruct cache".into());
        }
        std::fs::remove_dir_all(&cache)?;
    }
    std::fs::create_dir_all(&cache)?;
    // An incomplete attempt has an owner marker but cannot match a real stamp.
    std::fs::write(&marker, b"incomplete")?;
    let modified = ["worldgen/biomes.json", "worldgen/blocks/cached-collision.nbt",
        "cached-collision.nbt", "movement-properties.nbt", "render-definitions.json",
        "registries.json", "gameplay-facts.json", "items.json", "creative-inventory.json", "assets/minecraft/textures/gui",
        "assets/minecraft/textures/block", "data/tconstruct/tags/block"];
    fn mirror(src: &Path, dst: &Path, rel: &Path, modified: &[&str]) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let child = rel.join(entry.file_name());
            let name = child.to_string_lossy().replace('\\', "/");
            let source = entry.path();
            let target = dst.join(entry.file_name());
            let descendants = modified.iter().any(|item| item.starts_with(&(name.clone() + "/")));
            if source.is_dir() && (descendants || modified.contains(&name.as_str())) {
                mirror(&source, &target, &child, modified)?;
            } else if modified.contains(&name.as_str()) {
                std::fs::copy(source, target)?;
            } else {
                link_or_copy(&source, &target)?;
            }
        }
        Ok(())
    }
    fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let source = entry.path();
            let target = dst.join(entry.file_name());
            if source.is_dir() { copy_dir(&source, &target)?; }
            else { std::fs::copy(source, target)?; }
        }
        Ok(())
    }
    fn link_or_copy(src: &Path, dst: &Path) -> std::io::Result<()> {
        #[cfg(unix)]
        if std::os::unix::fs::symlink(src, dst).is_ok() { return Ok(()); }
        #[cfg(windows)]
        if (if src.is_dir() { std::os::windows::fs::symlink_dir(src, dst) }
            else { std::os::windows::fs::symlink_file(src, dst) }).is_ok() { return Ok(()); }
        if src.is_dir() { copy_dir(src, dst) } else { std::fs::copy(src, dst).map(|_| ()) }
    }
    mirror(&base, &cache, Path::new(""), &modified)?;
    // Add only our own sprites, never overwrite a vanilla asset or its link.
    fn install_sprites(src: &Path, dst: &Path) -> std::io::Result<()> {
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let source = entry.path();
            let target = dst.join(entry.file_name());
            if source.is_dir() {
                if target.exists() { return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "sprite directory exists")); }
                std::fs::create_dir(&target)?;
                install_sprites(&source, &target)?;
            } else {
                if std::fs::symlink_metadata(&target).is_ok() { return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "sprite exists")); }
                std::fs::copy(source, target)?;
            }
        }
        Ok(())
    }
    let runtime = base.join("assets/tconstruct/runtime");
    install_sprites(&runtime.join("gui"), &cache.join("assets/minecraft/textures/gui"))?;
    install_sprites(&runtime.join("block"), &cache.join("assets/minecraft/textures/block"))?;
    // The same manifest drives manual preparation and the content package.
    let icons: serde_json::Value = serde_json::from_slice(&std::fs::read(runtime.join("modifier-icon-sources.json"))?)?;
    let safe_name = |value: &str| !value.is_empty() && value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    for name in icons.get("mod").and_then(serde_json::Value::as_array).ok_or("missing mod icon list")? {
        let name = name.as_str().filter(|name| safe_name(name)).ok_or("invalid mod icon name")?;
        if !runtime.join(format!("gui/tconstruct_modifier_{name}.png")).is_file() { return Err(format!("missing modifier sprite {name}").into()); }
    }
    for (name, _) in icons.get("mod_item").and_then(serde_json::Value::as_object).ok_or("missing mod item icon map")? {
        if !safe_name(name) || !runtime.join(format!("gui/tconstruct_modifier_{name}.png")).is_file() { return Err(format!("missing or invalid modifier sprite {name}").into()); }
    }
    for (name, source) in icons.get("vanilla").and_then(serde_json::Value::as_object).ok_or("missing vanilla icon map")? {
        let source = source.as_str().filter(|source| safe_name(source)).ok_or("invalid vanilla icon source")?;
        if !safe_name(name) { return Err("invalid modifier icon name".into()); }
        let dest = cache.join(format!("assets/minecraft/textures/gui/tconstruct_modifier_{name}.png"));
        if std::fs::symlink_metadata(&dest).is_ok() { return Err("modifier sprite exists".into()); }
        std::fs::copy(base.join(format!("assets/minecraft/textures/item/{source}.png")), dest)?;
    }
    // Adapt the player's prepared 26.3 data locally; no vanilla data is
    // included in either published archive.
    fn copy_json_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let source = entry.path();
            let target = dst.join(entry.file_name());
            if source.is_dir() { copy_json_tree(&source, &target)?; }
            else if source.extension().is_some_and(|ext| ext == "json") { std::fs::copy(source, target)?; }
        }
        Ok(())
    }
    if std::fs::symlink_metadata(cache.join("minecraft-original-pack")).is_ok()
        || std::fs::symlink_metadata(cache.join("tconstruct-melter-fluids.json")).is_ok() {
        return Err("source resource root already contains reserved generated paths".into());
    }
    for (source, destination) in [("recipe", "recipes"), ("tags/item", "tags/items"),
        ("tags/fluid", "tags/fluids"), ("tags/block", "tags/blocks")] {
        copy_json_tree(&base.join("data/minecraft").join(source),
            &cache.join("minecraft-original-pack/data/minecraft").join(destination))?;
    }
    let original_src = base.join("assets/tconstruct/source-packs/tconstruct-original-pack");
    let original_out = cache.join("tconstruct-original-pack");
    if std::fs::symlink_metadata(&original_out).is_ok() { return Err("original pack output exists".into()); }
    copy_json_tree(&original_src, &original_out)?;
    // Match the manual preparation's Horizon copper-nugget adaptation.
    let item_table: serde_json::Value = serde_json::from_slice(&std::fs::read(cache.join("items.json"))?)?;
    if item_table.get("minecraft:copper_nugget").is_some() {
        let tag = original_out.join("data/forge/tags/items/nuggets/copper.json");
        let text = std::fs::read_to_string(&tag)?;
        std::fs::write(tag, text.replace("tconstruct:copper_nugget", "minecraft:copper_nugget"))?;
        for (original, native) in [("copper_ingot_from_nuggets", "copper_ingot_from_nuggets"),
            ("copper_nugget_from_ingot", "copper_nugget")] {
            let path = original_out.join(format!("data/tconstruct/recipes/common/materials/{original}.json"));
            if base.join(format!("data/minecraft/recipe/{native}.json")).is_file() {
                std::fs::remove_file(path)?;
            } else {
                let text = std::fs::read_to_string(&path)?;
                std::fs::write(path, text.replace("tconstruct:copper_nugget", "minecraft:copper_nugget"))?;
            }
        }
    }
    crate::tconstruct_source_harvest_tags::prepare(&cache, &original_out, &runtime.join("forge-harvest-tags.zip"))?;
    std::fs::copy(runtime.join("tconstruct-melter-fluids.json"), cache.join("tconstruct-melter-fluids.json"))?;
    // Restore item components beyond the content loader's template and
    // max-stack declarations, using the player's own cloned item table.
    let item_path = cache.join("items.json");
    let mut items: serde_json::Value = serde_json::from_slice(&std::fs::read(&item_path)?)?;
    let equipment: Vec<String> = serde_json::from_slice(&std::fs::read(runtime.join("equipment.json"))?)?;
    for short in equipment {
        if short == "pickaxe" { continue; }
        let Some(item) = items.get_mut(format!("tconstruct:{short}")) else { continue; };
        let Some(fields) = item.as_object_mut() else { continue; };
        fields.remove("minecraft:cooking_fuel");
        let slot = [("_helmet", "head"), ("_chestplate", "chest"), ("_leggings", "legs"), ("_boots", "feet")]
            .iter().find_map(|(suffix, slot)| short.ends_with(suffix).then_some(*slot))
            .or_else(|| (short == "slime_wings").then_some("chest"));
        if let Some(slot) = slot {
            fields.insert("minecraft:equippable".into(), serde_json::json!({"slot": slot, "equip_sound": "minecraft:item.armor.equip_generic"}));
        }
    }
    for short in ["pickaxe", "flint_pickaxe"] {
        if let Some(item) = items.get_mut(format!("tconstruct:{short}")) {
            item["minecraft:max_damage"] = serde_json::json!(85);
            if let Some(rules) = item["minecraft:tool"]["rules"].as_array_mut() {
                if let Some(rule) = rules.last_mut() { rule["speed"] = serde_json::json!(3.5); }
            }
        }
    }
    for short in ["earth_slime_bottle", "sky_slime_bottle", "ichor_slime_bottle", "ender_slime_bottle"] {
        if let Some(item) = items.get_mut(format!("tconstruct:{short}")) {
            item["minecraft:max_stack_size"] = serde_json::json!(16);
            item["minecraft:use_remainder"] = serde_json::json!({"id":"minecraft:glass_bottle"});
            item["minecraft:crafting_remainder"] = serde_json::json!({"id":"minecraft:glass_bottle"});
            if short == "ichor_slime_bottle" {
                item["minecraft:food"] = serde_json::json!({"can_always_eat":true,"nutrition":0,"saturation":0.0});
                item["minecraft:consumable"] = serde_json::json!({"animation":"drink","consume_seconds":1.6,
                    "has_consume_particles":false,"sound":"minecraft:entity.generic.drink","on_consume_effects":[
                    {"type":"minecraft:apply_effects","effects":[{"id":"minecraft:levitation","duration":200},
                    {"id":"minecraft:slowness","duration":200,"amplifier":1}]}]});
            }
        }
    }
    if let Some(item) = items.get_mut("tconstruct:venom_bottle") {
        item["minecraft:food"] = serde_json::json!({"can_always_eat":true,"nutrition":0,"saturation":0.0});
        item["minecraft:consumable"] = serde_json::json!({"animation":"drink","consume_seconds":1.6,
            "has_consume_particles":false,"sound":"minecraft:entity.generic.drink","on_consume_effects":[
            {"type":"minecraft:apply_effects","effects":[{"id":"minecraft:strength","duration":600},
            {"id":"minecraft:poison","duration":250}]}]});
        item["minecraft:max_stack_size"] = serde_json::json!(16);
        item["minecraft:use_remainder"] = serde_json::json!({"id":"minecraft:glass_bottle"});
    }
    std::fs::write(item_path, serde_json::to_vec(&items)?)?;
    let creative_path = cache.join("creative-inventory.json");
    if creative_path.is_file() {
        let inventory: serde_json::Value = serde_json::from_slice(&std::fs::read(&creative_path)?)?;
        let mut groups: Vec<serde_json::Value> = serde_json::from_slice(&std::fs::read(runtime.join("creative-tabs.json"))?)?;
        enrich_creative_groups(&cache, &original_out, &mut groups)?;
        let updated = register_creative_groups(&inventory, &groups)?;
        std::fs::write(&creative_path, serde_json::to_vec(&updated)?)?;
    }
    apply(&cache, &base)?;
    std::fs::write(&marker, fingerprint)?;
    *root = PathBuf::from(cache);
    Ok(())
}

// Registration is shared by native startup and the manual preparation path.
fn register_creative_groups(inventory: &serde_json::Value, groups: &[serde_json::Value]) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let tabs = groups.iter().map(|group| -> Result<pluto_creative_api::Tab, Box<dyn std::error::Error>> {
            let text = |key: &str| group.get(key).and_then(serde_json::Value::as_str).map(str::to_owned).ok_or_else(|| format!("missing Creative tab {key}"));
            let stacks = group.get("items").and_then(serde_json::Value::as_array).ok_or("missing Creative tab items")?;
            let mut restricted = std::collections::BTreeSet::new();
            if let Some(indices) = group.get("permissions_only") {
                for index in indices.as_array().ok_or("invalid Creative permissions")? {
                    let index = usize::try_from(index.as_u64().ok_or("invalid Creative permission index")?)?;
                    if index >= stacks.len() || !restricted.insert(index) { return Err("invalid Creative permission index".into()); }
                }
            }
            let items = stacks.iter().enumerate().map(|(index, item)| -> Result<pluto_creative_api::Entry, Box<dyn std::error::Error>> {
                Ok(pluto_creative_api::Entry { snbt: item.as_str().ok_or("invalid Creative item")?.to_owned(), searchable: true,
                    permissions_only: restricted.contains(&index), search_lines: Vec::new(), search_tags: Vec::new() })
            }).collect::<Result<Vec<_>, _>>()?;
            Ok(pluto_creative_api::Tab { id: text("id")?, title: text("title")?, icon: text("icon")?, items })
        }).collect::<Result<Vec<_>, _>>()?;
        let updated = pluto_creative_api::register_tabs(inventory, &tabs, |raw| {
            let mut stack = horizon_items::item_data::parse_stack(raw)?.ok_or("empty Creative stack")?;
            stack.count = 1;
            Ok(horizon_items::item_data::canon_stack(Some(&stack)))
        })?;
        Ok(updated)
}

fn take_tconstruct_creative_groups(inventory: &mut serde_json::Value) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let tabs = inventory.get_mut("tabs").and_then(serde_json::Value::as_array_mut).ok_or("missing Creative tabs")?;
    let mut groups = Vec::new();
    tabs.retain(|tab| {
        if matches!(tab.get("id").and_then(serde_json::Value::as_str), Some("tconstruct:tools" | "tconstruct:blocks")) {
            groups.push(tab.clone());
            false
        } else { true }
    });
    Ok(groups)
}

/// Apply source material stats to the generated Creative tool variants.
pub fn creative(out: &std::path::Path, original_pack: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = out.join("creative-inventory.json");
    if !path.is_file() { return Ok(()); }
    let mut inventory: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    let mut groups = take_tconstruct_creative_groups(&mut inventory)?;
    if groups.is_empty() { return Ok(()); }
    enrich_creative_groups(out, original_pack, &mut groups)?;
    let inventory = register_creative_groups(&inventory, &groups)?;
    // The prepared overlay may link to the original file; replace the link.
    let temp = out.join("creative-inventory.json.tmp");
    std::fs::write(&temp, serde_json::to_vec(&inventory)?)?;
    std::fs::remove_file(&path)?;
    std::fs::rename(temp, path)?;
    Ok(())
}

fn enrich_creative_groups(out: &std::path::Path, original_pack: &std::path::Path, tabs: &mut [serde_json::Value]) -> Result<(), Box<dyn std::error::Error>> {
    let path = horizon_protocol::registry::Builtin::find(out).ok_or("missing built-in registry report")?;
    let builtin = horizon_protocol::registry::Builtin::load(&path)?;
    let block_ids = builtin.get("minecraft:block").ok_or("missing block registry")?;
    let items: serde_json::Value = serde_json::from_slice(&std::fs::read(out.join("items.json"))?)?;
        // Give supported Creative tool variants the same source-derived base
        // stats and mining rules as a tool assembled in the station. Other tool
        // categories keep their material identities until their stats are ported.
        // Resolve the same source item tags used by station assembly, against
        // the actual generated registry rather than guessing tool categories.
        fn collect_tag_files(root: &std::path::Path, dir: &std::path::Path, files: &mut Vec<String>) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() { collect_tag_files(root, &path, files)?; }
                else if let Ok(relative) = path.strip_prefix(root) {
                    files.push(relative.to_string_lossy().replace('\\', "/"));
                }
            }
            Ok(())
        }
        let mut tag_files = Vec::new();
        collect_tag_files(&original_pack, &original_pack.join("data"), &mut tag_files)?;
        let mut raw_tags = std::collections::BTreeMap::new();
        crate::tconstruct_source_tags::merge_pack_tags(&mut raw_tags, tag_files, |path| std::fs::read(original_pack.join(path)).ok());
        let registry: std::collections::HashSet<String> = items.as_object().ok_or("invalid item registry")?.keys().cloned().collect();
        let harvest = crate::tconstruct_source_tags::resolve_tag("tconstruct:modifiable/harvest", &raw_tags, &registry)?;
        let melee = crate::tconstruct_source_tags::resolve_tag("tconstruct:modifiable/melee", &raw_tags, &registry)?;
        let melee_weapon = crate::tconstruct_source_tags::resolve_tag("tconstruct:modifiable/melee/weapon", &raw_tags, &registry)?;
        let armor = crate::tconstruct_source_tags::resolve_tag("tconstruct:modifiable/armor", &raw_tags, &registry)?;
        let ranged = crate::tconstruct_source_tags::resolve_tag("tconstruct:modifiable/ranged", &raw_tags, &registry)?;
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
                                let traits = crate::tconstruct_source_material_traits::ordered_levels_from_definition(&definition, &ids, |id| {
                                    let (namespace, path) = id.split_once(':')?;
                                    std::fs::read(original_pack.join(format!("data/{namespace}/{path}.json"))).ok().map(|bytes| vec![bytes])
                                });
                                if let (Some((traits, order)), Some(stats)) = (traits, crate::tconstruct_source_station_stats::base_stats(&definition, &maps)) {
                                    if crate::tconstruct_source_material_effects::apply_stat_traits(&mut stack, &stats, &traits).is_none() { continue; }
                                    if crate::tconstruct_source_upgrade_rules::apply_stats(&mut stack, &stats, &traits, melee.iter().any(|id| id == item), melee_weapon.iter().any(|id| id == item)).is_none() { continue; }
                                    if crate::tconstruct_source_mining_stack::apply(&mut stack, &stats, &traits, &order, harvest.iter().any(|id| id == item)).is_none() { continue; }
                                    if crate::tconstruct_source_overslime::rebuild(&mut stack, &stats, &traits, melee.iter().any(|id| id == item), armor.iter().any(|id| id == item), ranged.iter().any(|id| id == item), harvest.iter().any(|id| id == item)).is_none() { continue; }
                                }
                                crate::tconstruct_source_tool_normalize::finalize_tool(&mut stack, |id| block_ids.id(id))?;
                                *value = serde_json::Value::String(crate::snbt::to_snbt(&horizon_items::item_nbt::item_to_nbt(&stack)));
                            }
                        }
                    }
                }

    Ok(())
}
