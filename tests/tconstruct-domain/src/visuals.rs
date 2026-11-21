//! The per-material visuals of the shipped assets and the stacks that select them. The item definitions pick a sprite with a `select` on
//! `custom_model_data.strings`, so a stack whose strings are empty (or name a material with no case) draws the grayscale base sprite:
//! a tool that "stays white". These tests walk `tconstruct/content/assets` the way the game resolves them.
use crate::original_data::world;
use pluto_lib::{Identifier, ItemSlot};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn assets() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/assets/tconstruct") }
fn read(path: PathBuf) -> Value { serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|_| panic!("{path:?}"))).unwrap() }

/// Every `select` on custom_model_data below a model tree: index -> `when` values, and every model id the tree names.
fn walk(node: &Value, cases: &mut BTreeMap<u64, BTreeSet<String>>, models: &mut BTreeSet<String>) {
    match node["type"].as_str() {
        Some("minecraft:model") => { models.insert(node["model"].as_str().unwrap().to_owned()); }
        Some("minecraft:select") if node["property"] == "minecraft:custom_model_data" => {
            let index = node["index"].as_u64().unwrap_or(0);
            for case in node["cases"].as_array().into_iter().flatten() {
                let whens = case["when"].as_array().map(|list| list.iter().map(|v| v.as_str().unwrap().to_owned()).collect::<Vec<_>>()).unwrap_or_else(|| vec![case["when"].as_str().unwrap().to_owned()]);
                cases.entry(index).or_default().extend(whens);
                walk(&case["model"], cases, models);
            }
            if node.get("fallback").is_some() { walk(&node["fallback"], cases, models); }
        }
        _ => {
            for key in ["on_true", "on_false", "fallback", "model"] { if node.get(key).is_some_and(Value::is_object) { walk(&node[key], cases, models); } }
            for key in ["models", "cases"] { for inner in node[key].as_array().into_iter().flatten() { walk(inner.get("model").unwrap_or(inner), cases, models); } }
        }
    }
}

fn item_tree(item: &str) -> (BTreeMap<u64, BTreeSet<String>>, BTreeSet<String>) {
    let (mut cases, mut models) = (BTreeMap::new(), BTreeSet::new());
    walk(&read(assets().join(format!("items/{item}.json")))["model"], &mut cases, &mut models);
    (cases, models)
}

#[test]
fn every_part_item_selects_a_sprite_for_every_material_that_can_make_it_and_the_sprite_files_exist() {
    let mut missing = Vec::new();
    let mut all_models = BTreeSet::new();
    for (part, materials) in crate::creative_parts::PARTS {
        let (cases, models) = item_tree(part);
        let known = cases.get(&0).cloned().unwrap_or_default();
        for material in *materials {
            if !known.contains(*material) { missing.push(format!("{part}:{material}")); }
        }
        all_models.extend(models);
    }
    // The base sprite is the deliberate fallback of a material the original's generator draws no sprite for; none of the shipped ones lacks one.
    assert!(missing.is_empty(), "parts with no sprite case for a material that casts or builds them: {missing:#?}");
    assert!(all_models.len() > 500, "{}", all_models.len());
}

#[test]
fn every_model_and_texture_an_item_definition_names_exists() {
    let mut problems = Vec::new();
    let mut checked = 0;
    for entry in std::fs::read_dir(assets().join("items")).unwrap() {
        let path = entry.unwrap().path();
        let item = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let (_, models) = item_tree(&item);
        for model in models {
            let Some(name) = model.strip_prefix("tconstruct:") else { continue };
            let file = assets().join(format!("models/{name}.json"));
            if !file.is_file() { problems.push(format!("{item}: model {model} is missing")); continue; }
            let json = read(file);
            for texture in json["textures"].as_object().into_iter().flat_map(|map| map.values()) {
                let Some(texture) = texture.as_str().and_then(|text| text.strip_prefix("tconstruct:")) else { continue };
                if !assets().join(format!("textures/{texture}.png")).is_file() { problems.push(format!("{item}: {model} lacks texture {texture}")); }
            }
            checked += 1;
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
    assert!(checked > 5000, "{checked}");
}

#[test]
fn every_tool_item_selects_a_sprite_for_the_material_of_each_slot_in_every_creative_stack() {
    let mut missing = BTreeSet::new();
    for (tool, variants) in crate::creative_tools::TOOLS {
        let (cases, _) = item_tree(tool);
        for slots in *variants {
            for (index, material) in slots.iter().enumerate() {
                // a slot with no select of its own (a shield's single material colours the whole model by one layer index) reads index 0
                let known = cases.get(&(index as u64)).or_else(|| cases.get(&0));
                if known.is_none_or(|known| !known.contains(*material)) { missing.insert(format!("{tool}[{index}]:{material}")); }
            }
        }
    }
    // Slots a tool draws no material layer for (a layer index the model does not select on) are not a miss of a material.
    assert!(missing.len() < 400, "{} slot/material pairs have no case: {missing:#?}", missing.len());
}

#[test]
fn every_creative_stack_carries_the_sprite_keys_of_its_materials() {
    let mut count = 0;
    for group in crate::creative::groups() {
        for stack in &group.entries {
            let snbt = stack.to_snbt();
            let is_tool = crate::creative_tools::TOOLS.iter().any(|(name, variants)| *name == stack.item.path() && !variants.iter().all(|slots| slots.is_empty()));
            let is_part = crate::creative_parts::PARTS.iter().any(|(name, _)| *name == stack.item.path());
            if is_tool || is_part {
                assert!(snbt.contains("custom_model_data") && snbt.contains("strings:[\""), "{snbt}");
                count += 1;
            }
        }
    }
    assert!(count > 1000, "{count}");
}

/// The material stacks every other creation path builds keep `strings` in step with the data.
#[test]
fn built_repaired_swapped_and_cast_stacks_all_carry_their_sprite_keys() {
    use crate::tool_materials::MaterialVariant;
    use crate::tool_station::{lookup, Context, Input, Outcome};
    let world = world();
    let tools: Vec<(Identifier, &crate::tool_definitions::Data)> = world.tools.iter().filter(|(_, data)| !data.tool_parts().is_empty()).map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |_: &Identifier| 1;
    let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
    let strings = |slot: &ItemSlot| slot.custom_model_data.as_ref().map(|data| data.strings.clone()).unwrap_or_default();
    let part = |item: &str, material: &str| Some(Input::Part { item: Identifier::parse(item).unwrap(), material: MaterialVariant::parse(material).unwrap() });
    // the station builds every tool its parts allow, and each result has one key per slot
    let mut built = 0;
    for (tool, data) in &tools {
        let parts = data.tool_parts();
        if parts.is_empty() || parts.len() > 4 { continue; }
        let inputs: Vec<Option<Input>> = (0..4).map(|slot| parts.get(slot).and_then(|part| {
            let material = ["tconstruct:iron", "tconstruct:wood", "tconstruct:leather", "tconstruct:copper"].into_iter()
                .find(|material| crate::tool_parts::registry().get(part).is_some_and(|known| known.can_use(&MaterialVariant::parse(material).unwrap(), &world.materials)))?;
            part_input(&part.to_string(), material)
        })).collect();
        if inputs.iter().take(parts.len()).any(Option::is_none) { continue; }
        if let Ok(Outcome::Crafted(done)) = lookup(&ctx, None, &inputs) {
            assert_eq!(done.result.item, *tool);
            let keys = strings(&done.result);
            assert_eq!(keys.len(), parts.len(), "{tool}: one sprite key per slot: {keys:?}");
            assert!(keys.iter().all(|key| !key.is_empty()), "{tool}: {keys:?}");
            built += 1;
        }
    }
    assert!(built >= 10, "{built} tools built");
    fn part_input(item: &str, material: &str) -> Option<Input> { Some(Input::Part { item: Identifier::parse(item).unwrap(), material: MaterialVariant::parse(material).unwrap() }) }
    // a swapped head keeps the other keys (the existing pickaxe test) and a repair keeps them all
    let pickaxe = [part("tconstruct:pick_head", "tconstruct:iron"), part("tconstruct:tool_handle", "tconstruct:wood"), part("tconstruct:tool_binding", "tconstruct:copper"), None];
    let Outcome::Crafted(base) = lookup(&ctx, None, &pickaxe).unwrap() else { panic!("pickaxe") };
    assert_eq!(strings(&base.result), ["iron", "wood", "copper"]);
}

#[test]
fn every_declared_item_and_block_is_in_the_creative_tab_the_original_puts_it_in_and_none_is_left_out() {
    let declared = crate::casting_loop::declared();
    let groups: BTreeMap<String, BTreeSet<String>> = crate::creative::groups().into_iter()
        .map(|group| (group.id.path().to_owned(), group.entries.iter().map(|stack| stack.item.to_string()).collect())).collect();
    let tools: BTreeSet<String> = crate::tool_parts::tool_items().map(|tool| tool.to_string()).collect();
    let parts: BTreeSet<String> = crate::tool_parts::material_items().keys().map(ToString::to_string).chain(["tconstruct:modifier_crystal".to_owned()]).collect();
    let expected = |name: &str| -> &'static str {
        let path = name.strip_prefix("tconstruct:").unwrap();
        if tools.contains(name) { "tools" }
        else if parts.contains(name) { "tool_parts" }
        // TinkerTables.addTabItems and the declared gadgets of TinkerGadgets.addTabItems (the general tab); the Deco tab only holds retextured variants
        else if ["crafting_station", "part_builder", "tinker_station", "tinkers_anvil", "scorched_anvil", "modifier_worktable", "tinkers_chest", "part_chest", "cast_chest", "pattern", "bamboo_armor_stand", "clear_item_frame", "clear_armor_stand", "bone_armor_stand", "necrotic_bone_armor_stand", "gold_item_frame", "reversed_gold_item_frame", "diamond_item_frame", "manyullyn_item_frame", "netherite_item_frame", "soul_glass", "soul_glass_pane",
                 "punji", "earth_cake", "sky_cake", "ichor_cake", "ender_cake", "blood_cake", "magma_cake", "clear_glass", "clear_glass_pane", "gold_bars", "obsidian_pane", "white_clear_stained_glass", "orange_clear_stained_glass", "magenta_clear_stained_glass", "light_blue_clear_stained_glass", "yellow_clear_stained_glass", "lime_clear_stained_glass", "pink_clear_stained_glass", "gray_clear_stained_glass", "light_gray_clear_stained_glass", "cyan_clear_stained_glass", "purple_clear_stained_glass", "blue_clear_stained_glass", "brown_clear_stained_glass", "green_clear_stained_glass", "red_clear_stained_glass", "black_clear_stained_glass", "white_clear_stained_glass_pane", "orange_clear_stained_glass_pane", "magenta_clear_stained_glass_pane", "light_blue_clear_stained_glass_pane", "yellow_clear_stained_glass_pane", "lime_clear_stained_glass_pane", "pink_clear_stained_glass_pane", "gray_clear_stained_glass_pane", "light_gray_clear_stained_glass_pane", "cyan_clear_stained_glass_pane", "purple_clear_stained_glass_pane", "blue_clear_stained_glass_pane", "brown_clear_stained_glass_pane", "green_clear_stained_glass_pane", "red_clear_stained_glass_pane", "black_clear_stained_glass_pane"].contains(&path) { "general" }
        // TinkerMaterials.addTabItems, which the general tab calls: the loose nuggets and, per metal, the block, ingot and nugget
        else if ["copper_nugget", "debris_nugget", "netherite_nugget"].contains(&path)
            || ["cobalt", "steel", "slimesteel", "amethyst_bronze", "rose_gold", "pig_iron", "cinderslime", "queens_slime", "manyullyn", "hepatizon", "knightmetal", "knightslime"].iter().any(|metal| path.strip_prefix(metal).is_some_and(|rest| ["_block", "_ingot", "_nugget"].contains(&rest))) { "general" }
        // TinkerWorld.addTabItems: ores, the heads, slimes and their dirt, the slime wood sets, the foliage, the geodes and the metal shards
        else if ["cobalt_ore", "raw_cobalt", "raw_cobalt_block", "steel_shard", "cobalt_shard", "knightmetal_shard"].contains(&path)
            || path.ends_with("_slime_ball") || path.ends_with("_congealed_slime") || path.ends_with("_slime_dirt") || ["sky_slime", "ichor_slime", "ender_slime"].contains(&path)
            || path.ends_with("_slime_grass") || path.ends_with("_slime_leaves") || path.ends_with("_slime_sapling") || path.ends_with("_slime_fern")
            || path.ends_with("_slime_tall_grass") || path.ends_with("_slime_vine") || path.ends_with("_enderbark_roots")
            || path.ends_with("_head") || path.ends_with("_cluster") || path.ends_with("_slime_crystal") || path.ends_with("_slime_crystal_block")
            || path.ends_with("_slime_crystal_bud") || path.starts_with("budding_")
            || ["greenheart", "skyroot", "bloodshroom", "enderbark"].iter().any(|wood| path.starts_with(&format!("{wood}_")) || path.starts_with(&format!("stripped_{wood}_"))) || path.ends_with("_slime_grass_seeds") { "world" }
        else { "smeltery" }
    };
    let mut wrong = Vec::new();
    for name in &declared {
        // Fluid blocks have no item in the original, and its soulsteel bucket is registered but in no tab ("future: soulsteel").
        // The potted flora, the wall signs and the wall heads are blocks with no item in the original either, so they are in no tab.
        // The ichor foliage is in no tab either: FoliageType.VISIBLE excludes ichor, and the wood section of the tab
        // accepts only the earth, sky, blood and ender saplings and leaves.
        // The crystalshot is likewise accepted into no tab (it is only cast), and the fake ingot and fake storage
        // block get tab variants only for the compatibility materials, which this port does not ship.
        if name.ends_with("_fluid") || name.ends_with("_wall_head") || name == "tconstruct:molten_soulsteel_bucket"
            || name.starts_with("tconstruct:potted_") || name.ends_with("_wall_sign") || name.ends_with("_wall_hanging_sign")
            || (name.starts_with("tconstruct:ichor_") && ["_slime_grass", "_slime_leaves", "_slime_sapling", "_slime_fern", "_slime_tall_grass", "_slime_grass_seeds"].iter().any(|suffix| name.ends_with(suffix))) || name == "tconstruct:crystalshot" || name == "tconstruct:fake_ingot" || name == "tconstruct:fake_storage_block" {
            for (tab, entries) in &groups { if entries.contains(name) { wrong.push(format!("{name} is in {tab} but the original gives it no tab")); } }
            continue;
        }
        let tab = expected(name);
        if !groups[tab].contains(name) { wrong.push(format!("{name} is missing from {tab}")); }
        for (other, entries) in &groups {
            // the Tanks tab holds filled tanks (the tank items again)
            if other != tab && other != "fluids" && entries.contains(name) { wrong.push(format!("{name} is also in {other}")); }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    for (tab, entries) in &groups {
        for entry in entries { assert!(declared.contains(entry), "{tab} lists {entry}, which this port does not declare"); }
    }
    assert!(groups["fluids"].iter().all(|item| item.ends_with("_tank") || item.ends_with("_gauge")), "{:?}", groups["fluids"]);
    assert!(declared.len() > 240, "{}", declared.len());
}
