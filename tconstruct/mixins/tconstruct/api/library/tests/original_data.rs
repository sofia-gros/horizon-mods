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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../../.local/tconstruct-source/src/generated/resources/data/tconstruct/tinkering");
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

#[test]
fn the_originals_modifiers_load_with_the_stat_trait_and_volatile_modules_carried() {
    use std::sync::Arc;
    use tconstruct_library::modifiers::{ModifierManager, ModuleRegistry};
    use tconstruct_library::tools::stat::{HarvestTiers, ToolStats};
    let Some(root) = data_root() else { return };
    let files = read_all(&root.join("modifiers"));
    let tags = read_all(&root.join("tags/modifiers"));
    let mut modules = ModuleRegistry::with_original_types();
    modules.register_stat_modules(Arc::new(ToolStats::tinkers(HarvestTiers::vanilla())));
    let (before, _) = ModifierManager::load(&files, &tags, &[], &ModuleRegistry::with_original_types(), &EmptyContext);
    let (after, problems) = ModifierManager::load(&files, &tags, &[], &modules, &EmptyContext);
    assert!(problems.is_empty(), "{:?}", &problems[..problems.len().min(8)]);
    // the modules of the carried types are carried now (modifiers gated on other mods are not loaded, so count from the manager)
    let carried_now: usize = after.ids().iter().filter_map(|id| after.get(id)).flat_map(|m| m.modules.iter()).filter(|m| m.carried()).count();
    assert!(carried_now > 150, "{carried_now} carried modules");
    assert_eq!(before.uncarried_modules() - after.uncarried_modules(), carried_now);
}

#[test]
fn a_whole_wood_pickaxe_is_rebuilt_from_the_originals_data() {
    use std::sync::Arc;
    use tconstruct_library::modifiers::stat_modules::ToolContext;
    use tconstruct_library::modifiers::{ModifierManager, ModuleRegistry};
    use tconstruct_library::tools::definition::{ToolDefinitionData, ToolModuleRegistry};
    use tconstruct_library::tools::stack::rebuild_stats;
    use tconstruct_library::tools::stat::{HarvestTiers, StatValue, ToolStatId, ToolStats};
    struct Pickaxe;
    impl ToolContext for Pickaxe {
        fn item_in_tag(&self, tag: &str) -> bool {
            ["tconstruct:modifiable/harvest", "tconstruct:modifiable/durability", "tconstruct:modifiable/melee"].contains(&tag)
        }
    }
    let Some(root) = data_root() else { return };
    let stats = Arc::new(ToolStats::tinkers(HarvestTiers::vanilla()));
    let mut materials = tconstruct_library::materials::MaterialRegistry::with_tinkers_stats();
    assert!(materials.stats.load(&read_all(&root.join("materials/stats"))).is_empty());
    assert!(materials.traits.load(&read_all(&root.join("materials/traits"))).is_empty());
    let mut modules = ModuleRegistry::with_original_types();
    modules.register_stat_modules(stats.clone());
    let (modifiers, problems) = ModifierManager::load(&read_all(&root.join("modifiers")), &[], &[], &modules, &EmptyContext);
    assert!(problems.is_empty(), "{problems:?}");
    let defs = read_all(&root.join("tool_definitions"));
    let pickaxe = defs.iter().find(|(id, _)| id.path() == "pickaxe").unwrap();
    let definition = ToolDefinitionData::from_json(&pickaxe.1, &ToolModuleRegistry::with_original_types(), &stats).unwrap();
    let wood = MaterialId::try_parse("tconstruct:wood").unwrap();
    let rebuilt = rebuild_stats(&definition, &[wood.clone(), wood.clone(), wood], &[], &materials, &modifiers, &stats, &Pickaxe);
    let names: Vec<String> = rebuilt.modifiers.iter().map(|e| format!("{}x{}", e.modifier, e.level)).collect();
    // the pickaxe's own pierce trait and the wood's default trait on each of the three parts
    assert!(names.contains(&"tconstruct:piercex1".to_owned()), "{names:?}");
    assert!(names.contains(&"tconstruct:cultivatedx3".to_owned()), "{names:?}");
    assert_eq!(rebuilt.stats[&ToolStatId::tconstruct("durability")], StatValue::Float(60.0));
    assert_eq!(rebuilt.slots["upgrades"], 3);
}

#[test]
fn every_formula_in_the_originals_modifier_data_is_well_formed() {
    use tconstruct_library::formula::PostFixFormula;
    fn walk(value: &Value, found: &mut Vec<Vec<Value>>) {
        match value {
            Value::Object(object) => {
                if let Some(Value::Array(formula)) = object.get("formula") {
                    found.push(formula.clone());
                }
                object.values().for_each(|v| walk(v, found));
            }
            Value::Array(list) => list.iter().for_each(|v| walk(v, found)),
            _ => {}
        }
    }
    let Some(root) = data_root() else { return };
    let mut formulas = Vec::new();
    for (_, json) in read_all(&root.join("modifiers")) {
        walk(&json, &mut formulas);
    }
    assert!(formulas.len() > 20, "{} formulas", formulas.len());
    for tokens in &formulas {
        // the variables a formula may name are those it uses; the stack and the operators are what is checked
        let names: Vec<String> = tokens.iter().filter_map(|t| t.as_str()).filter_map(|t| t.strip_prefix('$')).map(str::to_owned).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        PostFixFormula::parse(tokens, &names).unwrap_or_else(|e| panic!("{tokens:?}: {e}"));
    }
}

#[test]
fn every_item_tag_a_tool_stat_names_exists_in_the_content() {
    use tconstruct_library::tools::stat::{HarvestTiers, ToolStats, ToolType};
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../content/data/tconstruct/tags/item");
    if !content.is_dir() {
        return;
    }
    let stats = ToolStats::tinkers(HarvestTiers::vanilla());
    let mut tags: Vec<String> = stats.all().flat_map(|s| s.item_tags.clone()).collect();
    tags.extend([ToolType::Melee, ToolType::Harvest, ToolType::Ranged, ToolType::Launcher, ToolType::Armor, ToolType::MeleeUnarmed].map(|t| t.tag().to_owned()));
    for tag in tags {
        let path = tag.strip_prefix("tconstruct:").unwrap_or_else(|| panic!("{tag} is not a tconstruct tag"));
        assert!(content.join(format!("{path}.json")).is_file(), "no content file for the item tag {tag}");
    }
}

#[test]
fn every_original_tool_can_be_rebuilt_from_wood_and_from_iron() {
    use std::sync::Arc;
    use tconstruct_library::modifiers::stat_modules::ToolContext;
    use tconstruct_library::modifiers::{ModifierManager, ModuleRegistry};
    use tconstruct_library::tools::definition::{ToolDefinitionData, ToolModuleRegistry};
    use tconstruct_library::tools::stack::rebuild_stats;
    use tconstruct_library::tools::stat::{HarvestTiers, ToolStatId, ToolStats};
    struct Anything;
    impl ToolContext for Anything {
        fn item_in_tag(&self, _: &str) -> bool {
            true
        }
    }
    let Some(root) = data_root() else { return };
    let stats = Arc::new(ToolStats::tinkers(HarvestTiers::vanilla()));
    let mut materials = tconstruct_library::materials::MaterialRegistry::with_tinkers_stats();
    assert!(materials.stats.load(&read_all(&root.join("materials/stats"))).is_empty());
    assert!(materials.traits.load(&read_all(&root.join("materials/traits"))).is_empty());
    let mut modules = ModuleRegistry::with_original_types();
    modules.register_stat_modules(stats.clone());
    let (modifiers, _) = ModifierManager::load(&read_all(&root.join("modifiers")), &[], &[], &modules, &EmptyContext);
    let registry = ToolModuleRegistry::with_original_types();
    let durability = ToolStatId::tconstruct("durability");
    let mut checked = 0;
    for (id, json) in read_all(&root.join("tool_definitions")) {
        let definition = ToolDefinitionData::from_json(&json, &registry, &stats).unwrap();
        for material in ["wood", "iron"] {
            let material = MaterialId::try_parse(&format!("tconstruct:{material}")).unwrap();
            let materials_list = vec![material; definition.parts().len()];
            let rebuilt = rebuild_stats(&definition, &materials_list, &[], &materials, &modifiers, &stats, &Anything);
            // a tool with parts has a durability from them, unless it is a fixed-stat tool without part stats
            let has_durable_part = definition.parts().iter().filter_map(|p| tconstruct_library::tools::definition::part_stat_type(&p.item)).any(|k| {
                let name = k.resource().path();
                name == "head" || name == "limb"
            });
            if has_durable_part && rebuilt.stats.get(&durability).is_none() {
                panic!("{id} made of {materials_list:?} has no durability");
            }
        }
        checked += 1;
    }
    assert!(checked >= 45, "{checked} tools");
}

#[test]
fn the_originals_alloy_recipes_all_load_and_the_ones_without_mod_gates_stay() {
    let Some(root) = data_root() else { return };
    let recipes = read_all(&root.join("../recipes/smeltery/alloys"));
    assert_eq!(recipes.len(), 24);
    let mut kept = Vec::new();
    for (id, json) in &recipes {
        match tconstruct_library::smeltery::alloy::AlloyRecipe::from_json(id.clone(), json, &EmptyContext) {
            Ok(Some(recipe)) => kept.push(recipe.id.path().to_owned()),
            Ok(None) => {}
            Err(error) => panic!("{id}: {error}"),
        }
    }
    // without mod integration, the recipes gated on a mod's ingot tag are skipped; those on vanilla fluids stay
    assert!(kept.iter().any(|k| k.ends_with("molten_obsidian")), "{kept:?}");
    assert!(kept.len() < recipes.len());
}

#[test]
fn the_originals_plain_melting_recipes_load() {
    let Some(root) = data_root() else { return };
    let recipes = read_all(&root.join("../recipes/smeltery/melting"));
    let (mut plain, mut loaded) = (0, 0);
    for (id, json) in &recipes {
        if json.get("type").and_then(Value::as_str) != Some("tconstruct:melting") {
            continue; // damagable, ore and conditional recipes are not ported yet
        }
        match tconstruct_library::smeltery::melting::MeltingRecipe::from_json(id.clone(), json, &EmptyContext) {
            Ok(Some(_)) => loaded += 1,
            Ok(None) => {}
            Err(error) if error.contains("unknown condition") || error.contains("unsupported ingredient type") => {}
            Err(error) => panic!("{id}: {error}"),
        }
        plain += 1;
    }
    assert!(plain > 400, "{plain}");
    assert!(loaded > 100, "{loaded}");
}
