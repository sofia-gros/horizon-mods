//! Repro for the r5 station regression: drive the production wiring (the real reload listeners
//! over the shipped content, then the station block entity ticker's result refresh) the way the
//! game does, instead of hand-building a Context the way the unit tests do.
use pluto_lib::{ConfigEvents, ConfigKind, Identifier, ItemSlot, JsonResourceStack, ModConfig};
use serde_json::Value;
use std::path::PathBuf;

fn data_root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct") }

/// JsonResourceStacks for every .json under a tinkering subfolder, ids from the relative path.
fn recursive_stacks(rel: &str) -> Vec<JsonResourceStack> {
    fn walk(dir: &std::path::Path, base: &std::path::Path, out: &mut Vec<JsonResourceStack>) {
        for entry in std::fs::read_dir(dir).expect("content folder") {
            let path = entry.unwrap().path();
            if path.is_dir() { walk(&path, base, out); continue; }
            if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
            let rel = path.strip_prefix(base).unwrap().to_str().unwrap().replace('\\', "/");
            let id = Identifier::new("tconstruct", rel.trim_end_matches(".json")).unwrap();
            let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
            out.push(JsonResourceStack { id, layers: vec![Ok(value)] });
        }
    }
    let base = data_root().join("tinkering").join(rel);
    let mut out = Vec::new();
    walk(&base, &base, &mut out);
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// The captured item registry a reload sees: the mod's own items plus the vanilla staples the
/// shipped data names, and the mod's item tags (plain members only; tag references unresolved).
fn item_snapshot() -> (Vec<String>, Vec<(String, Vec<String>)>) {
    let mut entries: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(data_root().join("horizon/item")).expect("item folder") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            entries.push(format!("tconstruct:{}", path.file_stem().unwrap().to_str().unwrap()));
        }
    }
    for vanilla in ["stick", "oak_planks", "iron_ingot", "gold_ingot", "copper_ingot", "coal", "charcoal", "diamond", "emerald", "quartz", "glass", "sand", "gravel", "clay_ball", "brick", "netherite_scrap", "leather", "string", "bone", "slime_ball", "paper", "book", "stone", "cobblestone", "deepslate", "obsidian", "glowstone_dust", "redstone", "lapis_lazuli", "ender_pearl", "blaze_rod", "ghast_tear", "phantom_membrane", "feather", "flint", "wheat", "apple", "egg", "sugar", "milk_bucket", "water_bucket", "lava_bucket", "bucket", "iron_block", "gold_block", "diamond_block", "netherite_ingot", "copper_block", "amethyst_shard", "echo_shard", "disc_fragment_5", "goat_horn", "wet_sponge", "sponge", "honeycomb", "honey_bottle", "dragon_breath", "totem_of_undying", "nether_star", "heart_of_the_sea", "nautilus_shell", "scute", "armadillo_scute", "turtle_helmet", "sniffer_egg", "music_disc_13"] {
        entries.push(format!("minecraft:{vanilla}"));
    }
    entries.sort();
    entries.dedup();
    let mut tags: Vec<(String, Vec<String>)> = Vec::new();
    for entry in std::fs::read_dir(data_root().join("tags/item")).expect("item tag folder") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
        let mut members: Vec<String> = Vec::new();
        if let Some(values) = value.get("values").and_then(Value::as_array) {
            for member in values {
                match member {
                    Value::String(id) if !id.starts_with('#') => members.push(id.clone()),
                    Value::Object(map) => if let Some(id) = map.get("id").and_then(Value::as_str) { if !id.starts_with('#') { members.push(id.to_owned()); } },
                    _ => {}
                }
            }
        }
        tags.push((format!("tconstruct:{}", path.file_stem().unwrap().to_str().unwrap()), members));
    }
    tags.sort();
    // A captured registry is consistent: every tag member is itself an entry.
    for (_, members) in &tags { entries.extend(members.iter().cloned()); }
    entries.sort();
    entries.dedup();
    (entries, tags)
}

fn files_as_strings(folder: &str) -> Vec<(String, Value)> {
    crate::original_data::files(folder).into_iter().map(|(id, value)| (id.to_string(), value)).collect()
}

#[test]
fn station_previews_a_wood_pickaxe_after_a_real_reload() {
    let materials_state = crate::common::register();
    let config = ModConfig { mod_id: "tconstruct".into(), kind: ConfigKind::Common, file: "tconstruct-common.toml".into(), text: crate::config::default_text() };
    ConfigEvents::LOADING.invoke(&config);
    let (entries, tags) = item_snapshot();
    let items = Some((entries.as_slice(), tags.as_slice()));

    let material_folders = [
        (crate::materials::TAG_FOLDER.into(), recursive_stacks("tags/materials")),
        (crate::material_traits::FOLDER.into(), crate::original_data::stacks("materials/traits")),
        (crate::material_stats::FOLDER.into(), crate::original_data::stacks("materials/stats")),
    ].into_iter().collect();
    assert!(pluto_lib::fixtures::server_json_reload_folders("tconstruct:material_definitions", crate::materials::FOLDER,
        &files_as_strings("materials/definition"), items, material_folders).unwrap(), "materials reload applies");
    {
        let state = materials_state.read().unwrap();
        assert!(state.server_loaded, "materials report server_loaded after the shipped reload");
        assert!(state.problems.is_empty(), "material problems: {:?}", state.problems);
        let wood = Identifier::parse("tconstruct:wood").unwrap();
        let head = Identifier::parse("tconstruct:head").unwrap();
        assert!(state.get_stats(&wood, &head).is_some(), "wood has head stats");
    }

    let modifier_folders = [(crate::modifiers::TAG_FOLDER.into(), recursive_stacks("tags/modifiers"))].into_iter().collect();
    assert!(pluto_lib::fixtures::server_json_reload_folders("tconstruct:modifiers", crate::modifiers::FOLDER,
        &files_as_strings("modifiers"), items, modifier_folders).unwrap(), "modifiers reload applies");
    assert!(crate::common::server_modifiers().unwrap().read().unwrap().definitions_loaded, "modifiers report definitions_loaded");

    assert!(pluto_lib::fixtures::server_json_reload("tconstruct:tool_definitions", crate::tool_definitions::FOLDER,
        &files_as_strings("tool_definitions"), items).unwrap(), "tool definitions reload applies");
    {
        let definitions = crate::tool_definitions::register().read().unwrap();
        assert!(!definitions.data.is_empty(), "tool definitions loaded");
        let pickaxe = Identifier::parse("tconstruct:pickaxe").unwrap();
        assert!(definitions.data.get(&pickaxe).is_some_and(|data| !data.tool_parts().is_empty()), "the pickaxe definition has parts");
    }
    assert!(crate::common::item_tags_loaded(), "a reload captured the item registry");
    assert!(crate::station_menu::with_context(|_, _| ()).is_some(), "the station context assembles after a real reload");

    let mut items: Vec<Option<ItemSlot>> = vec![None; 6];
    for (slot, part) in [(1, "tconstruct:pick_head"), (2, "tconstruct:tool_handle"), (3, "tconstruct:tool_binding")] {
        let mut stack = ItemSlot::new(Identifier::parse(part).unwrap(), 1);
        crate::tool_materials::set_part_material(&mut stack, &crate::tool_materials::MaterialVariant::parse("tconstruct:wood").unwrap());
        items[slot] = Some(stack);
    }
    assert!(crate::station_menu::refresh_result(&mut items), "the ticker refresh changes the result slot");
    let result = items[5].as_ref().expect("a previewed tool in the result slot");
    assert_eq!(result.item.to_string(), "tconstruct:pickaxe");
}
