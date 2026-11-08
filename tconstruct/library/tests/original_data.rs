//! Loads the original's generated data (`src/generated/resources/data/tconstruct/tinkering`) with the library, when the original's source is
//! checked out under `.local/tconstruct-source` (as the owner's machine has it); otherwise the test does nothing.
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tconstruct_library::condition::EmptyContext;
use tconstruct_library::materials::manager::MaterialManager;
use tconstruct_library::materials::traits::MaterialTraitsManager;
use tconstruct_library::materials::MaterialId;
use tconstruct_library::resource::ResourceId;

fn data_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/tconstruct-source/src/generated/resources/data/tconstruct/tinkering");
    root.is_dir().then_some(root)
}

/// Every `.json` under `dir`, as (`tconstruct:<relative path without extension>`, parsed).
fn read_all(dir: &Path) -> Vec<(ResourceId, Value)> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(folder) = stack.pop() {
        let Ok(entries) = fs::read_dir(&folder) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                let relative = path.strip_prefix(dir).unwrap().with_extension("");
                let name = relative.to_string_lossy().replace('\\', "/");
                let json: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                found.push((ResourceId::try_new("tconstruct", &name).unwrap(), json));
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[test]
fn the_originals_materials_load_without_problems() {
    let Some(root) = data_root() else { return };
    let definitions = read_all(&root.join("materials/definition"));
    let tags = read_all(&root.join("tags/materials"));
    assert!(definitions.len() > 100, "{} definition files", definitions.len());
    let (manager, problems) = MaterialManager::load(&definitions, &tags, &EmptyContext);
    assert!(problems.is_empty(), "{problems:?}");
    let flint = manager.material(&MaterialId::try_parse("tconstruct:flint").unwrap()).unwrap();
    assert_eq!((flint.tier, flint.craftable), (1, true));
    // nothing is installed beside the game and no tag has members, so the integration materials stay out
    assert!(manager.material(&MaterialId::try_parse("tconstruct:aluminum").unwrap()).is_none());
    assert!(manager.visible_materials().windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn the_originals_traits_load_without_problems() {
    let Some(root) = data_root() else { return };
    let files = read_all(&root.join("materials/traits"));
    assert!(files.len() > 90, "{} trait files", files.len());
    let mut manager = MaterialTraitsManager::default();
    let problems = manager.load(&files);
    assert!(problems.is_empty(), "{problems:?}");
    let wood = MaterialId::try_parse("tconstruct:wood").unwrap();
    assert_eq!(manager.default_traits(&wood)[0].modifier.to_string(), "tconstruct:cultivated");
}

#[test]
fn the_originals_stats_load_with_every_registered_type() {
    let Some(root) = data_root() else { return };
    let files = read_all(&root.join("materials/stats"));
    assert!(files.len() > 90, "{} stat files", files.len());
    let mut registry = tconstruct_library::materials::MaterialRegistry::with_tinkers_stats();
    let problems = registry.stats.load(&files);
    assert!(problems.is_empty(), "{:?}", &problems[..problems.len().min(5)]);
    let flint = MaterialId::try_parse("tconstruct:flint").unwrap();
    let head = tconstruct_library::materials::MaterialStatsId::try_parse("tconstruct:head").unwrap();
    assert!(registry.material_stats(&flint, &head).is_some());
}

#[test]
fn the_originals_modifiers_load_without_problems() {
    use tconstruct_library::modifiers::{ModifierManager, ModuleRegistry};
    let Some(root) = data_root() else { return };
    let files = read_all(&root.join("modifiers"));
    let tags = read_all(&root.join("tags/modifiers"));
    assert!(files.len() > 300, "{} modifier files", files.len());
    let enchantments = fs::read(root.join("enchantments_to_modifiers.json")).map(|b| serde_json::from_slice::<Value>(&b).unwrap()).into_iter().collect::<Vec<_>>();
    let (manager, problems) = ModifierManager::load(&files, &tags, &enchantments, &ModuleRegistry::with_original_types(), &EmptyContext);
    assert!(problems.is_empty(), "{:?}", &problems[..problems.len().min(8)]);
    assert!(manager.ids().len() > 280, "{} modifiers", manager.ids().len());
    // nothing is carried yet: every module of every modifier is still raw data
    assert!(manager.uncarried_modules() > 400, "{} modules", manager.uncarried_modules());
}

#[test]
fn the_originals_tool_definitions_load_without_problems() {
    use tconstruct_library::tools::definition::{ToolDefinitionData, ToolModuleRegistry};
    use tconstruct_library::tools::stat::{HarvestTiers, ModifierStatsBuilder, StatValue, ToolStatId, ToolStats};
    let Some(root) = data_root() else { return };
    let files = read_all(&root.join("tool_definitions"));
    assert!(files.len() >= 45, "{} tool definitions", files.len());
    let registry = ToolModuleRegistry::with_original_types();
    let stats = ToolStats::tinkers(HarvestTiers::vanilla());
    let mut uncarried = 0;
    for (id, json) in &files {
        let data = ToolDefinitionData::from_json(json, &registry, &stats).unwrap_or_else(|e| panic!("{id}: {e}"));
        uncarried += data.uncarried_modules();
    }
    assert!(uncarried > 100, "{uncarried} modules are still raw data");
    // the pickaxe: attack damage 0.5 and speed 1.2 set outright, 1 ability and 3 upgrade slots, the pierce trait, three parts
    let pickaxe = files.iter().find(|(id, _)| id.path() == "pickaxe").unwrap();
    let data = ToolDefinitionData::from_json(&pickaxe.1, &registry, &stats).unwrap();
    let mut builder = ModifierStatsBuilder::new(&stats);
    data.add_tool_stats(&mut builder);
    assert_eq!(builder.get(&ToolStatId::tconstruct("attack_damage")), StatValue::Float(0.5));
    assert_eq!(builder.get(&ToolStatId::tconstruct("attack_speed")), StatValue::Float(1.2));
    assert_eq!((data.slots()["upgrades"], data.slots()["abilities"]), (3, 1));
    assert_eq!(data.traits()[0].modifier.to_string(), "tconstruct:pierce");
    assert_eq!(data.parts().len(), 3);
    assert!(data.can_perform("pickaxe_dig"));
}

#[test]
fn every_part_of_the_originals_tools_has_a_stat_type_and_a_wood_pickaxe_has_the_originals_numbers() {
    use tconstruct_library::tools::definition::{part_stat_type, ToolDefinitionData, ToolModuleRegistry};
    use tconstruct_library::tools::stat::{HarvestTiers, ModifierStatsBuilder, StatValue, ToolStatId, ToolStats};
    let Some(root) = data_root() else { return };
    let defs = read_all(&root.join("tool_definitions"));
    let registry = ToolModuleRegistry::with_original_types();
    let stats = ToolStats::tinkers(HarvestTiers::vanilla());
    for (id, json) in &defs {
        let data = ToolDefinitionData::from_json(json, &registry, &stats).unwrap();
        for part in data.parts() {
            assert!(part_stat_type(&part.item).is_some(), "{id}: no stat type for part {}", part.item);
        }
    }
    // wood pickaxe: head 60 durability, +0 handle percent; the pickaxe's own base: attack damage 0.5 and speed 1.2
    let mut materials = tconstruct_library::materials::MaterialRegistry::with_tinkers_stats();
    let problems = materials.stats.load(&read_all(&root.join("materials/stats")));
    assert!(problems.is_empty(), "{problems:?}");
    let wood = MaterialId::try_parse("tconstruct:wood").unwrap();
    let pickaxe = defs.iter().find(|(id, _)| id.path() == "pickaxe").unwrap();
    let data = ToolDefinitionData::from_json(&pickaxe.1, &registry, &stats).unwrap();
    let mut builder = ModifierStatsBuilder::new(&stats);
    data.add_material_stats(&mut builder, &[wood.clone(), wood.clone(), wood], &materials);
    data.add_tool_stats(&mut builder);
    assert_eq!(builder.get(&ToolStatId::tconstruct("durability")), StatValue::Float(60.0));
    assert_eq!(builder.get(&ToolStatId::tconstruct("mining_speed")), StatValue::Float(2.0));
}
