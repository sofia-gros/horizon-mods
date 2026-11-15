//! The original mod's generated data, as copied into the content root, through the production
//! decoders. This is the check that decoders accept what the original actually ships; synthetic
//! fixtures elsewhere only cover the shapes their authors thought of.
use crate::{materials::Definitions as Materials, modifiers::Registry, tool_definitions::Data};
use pluto_lib::{Identifier, JsonResourceStack};
use serde_json::Value;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct/tinkering")
}
fn files(folder: &str) -> Vec<(Identifier, Value)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root().join(folder)).expect("content folder") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
        found.push((Identifier::new("tconstruct", &name).unwrap(), value));
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}
fn stacks(folder: &str) -> Vec<JsonResourceStack> {
    files(folder).into_iter().map(|(id, value)| JsonResourceStack { id, layers: vec![Ok(value)] }).collect()
}
/// The production condition tree over an item registry with no external ingot tags and default config,
/// the vanilla-only world the other-mod materials were excluded for.
fn conditions() -> crate::conditions::Context {
    crate::conditions::Context { items: Some(pluto_lib::fixtures::registry(&[]).unwrap()), config: crate::config::parse("").unwrap() }
}

#[test]
fn every_shipped_tool_definition_decodes() {
    let definitions = files("tool_definitions");
    assert_eq!(definitions.len(), 44, "45 original definitions minus the other-mod minotaur axe");
    let failures: Vec<String> = definitions.iter()
        .filter_map(|(id, value)| Data::decode(value).err().map(|error| format!("{id}: {error}"))).collect();
    assert!(failures.is_empty(), "definitions that do not decode: {failures:#?}");
}

#[test]
fn every_shipped_material_loads_with_stats_and_traits_without_problems() {
    let mut registry = Materials::default();
    let definitions = files("materials/definition");
    registry = Materials::load(&definitions, &conditions());
    assert!(registry.problems.is_empty(), "material definition problems: {:#?}", registry.problems);
    assert_eq!(registry.materials.len() + registry.redirects.len(), definitions.len());
    let mut problems = Vec::new();
    registry.stats = crate::material_stats::load(&stacks("materials/stats"), &mut problems);
    registry.traits = crate::material_traits::load(&stacks("materials/traits"), &mut problems);
    assert!(problems.is_empty(), "material stats/traits problems: {problems:#?}");
    assert_eq!(registry.stats.len(), registry.materials.len(), "every concrete material ships stats");
}

#[test]
fn shipped_modifiers_report_exactly_the_modules_that_are_not_ported() {
    let mut registry = Registry::default();
    registry.reload(&files("modifiers"), &conditions());
    assert!(registry.problems.is_empty(), "modifiers that do not decode: {:#?}", registry.problems);
    let unbound = (0..).zip(registry.dynamic_ids()).filter(|(_, id)| registry.get(id).is_some_and(|modifier| !modifier.unbound.is_empty())).count();
    // The baseline of modifiers with at least one unported module; it only shrinks as modules are ported.
    assert!(unbound <= 300, "{unbound} modifiers carry unported modules");
}

/// The whole loaded original, as one world for end-to-end checks.
struct World { materials: Materials, registry: Registry, tools: Vec<(Identifier, Data)> }
fn world() -> World {
    let mut materials = Materials::load(&files("materials/definition"), &conditions());
    let mut problems = Vec::new();
    materials.stats = crate::material_stats::load(&stacks("materials/stats"), &mut problems);
    materials.traits = crate::material_traits::load(&stacks("materials/traits"), &mut problems);
    assert!(problems.is_empty(), "{problems:?}");
    materials.server_loaded = true;
    let mut registry = Registry::default();
    registry.reload(&files("modifiers"), &conditions());
    let tools = files("tool_definitions").into_iter().map(|(id, value)| (id, Data::decode(&value).expect("shipped definition"))).collect();
    World { materials, registry, tools }
}

#[test]
fn a_pickaxe_of_shipped_wood_builds_repairs_and_breaks_through_the_station_dispatch() {
    use crate::tool_station::{lookup, Context, Input, Outcome};
    use crate::tool_materials::MaterialVariant;
    let world = world();
    let tools: Vec<(Identifier, &Data)> = world.tools.iter().filter(|(_, data)| !data.tool_parts().is_empty())
        .map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |_: &Identifier| 1;
    let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
    let wood = MaterialVariant::parse("tconstruct:wood").unwrap();
    let part = |item: &str| Some(Input::Part { item: Identifier::parse(item).unwrap(), material: wood.clone() });
    let inputs = [part("tconstruct:pick_head"), part("tconstruct:tool_handle"), part("tconstruct:tool_binding"), None];
    let Outcome::Crafted(built) = lookup(&ctx, None, &inputs).unwrap() else { panic!("the shipped pickaxe did not build") };
    assert_eq!(built.result.item.to_string(), "tconstruct:pickaxe");
    let data = &built.result.custom_data;
    assert!(crate::tool_stack::durability(data) > 1, "wood parts give the tool real durability");
    assert!(crate::tool_stack::stat(data, crate::tool_stats::Stat::MiningSpeed) > 1.0);
    assert_eq!(crate::tool_stack::materials(data, true).as_slice().len(), 3);
    assert!(built.result.max_damage.is_some() && built.result.tool.is_some(), "the slot publishes its bar and mining rules");
    // The shipped definition also lists what it declares but Tinkers has not ported.
    let (_, pickaxe) = tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap();
    assert!(pickaxe.unbound_modules().iter().any(|module| module.path() == "box_aoe"));
}

#[test]
fn shipped_stat_boost_modifiers_change_a_rebuilt_pickaxe() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, stat, Environment, TAG_UPGRADES, TAG_MATERIALS}, tool_stats::Stat};
    use pluto_lib::{Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let build = |upgrades: Option<List>| {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        if let Some(list) = upgrades { nbt.insert(TAG_UPGRADES.into(), list.to_nbt()); }
        assert!(rebuild_stats(&mut nbt, &env).unwrap());
        nbt
    };
    let plain = build(None);
    let sharp = build(Some(List::default().with_modifier(Identifier::parse("tconstruct:sharpness").unwrap(), 3).unwrap()));
    // sharpness: +0.75 attack damage per level, three levels.
    let gained = stat(&sharp, Stat::AttackDamage) - stat(&plain, Stat::AttackDamage);
    assert!((gained - 2.25).abs() < 1e-4, "sharpness III added {gained}");
    // ductile: +10% durability per level (multiply_base), so durability grows.
    let ductile = build(Some(List::default().with_modifier(Identifier::parse("tconstruct:ductile").unwrap(), 2).unwrap()));
    assert!(crate::tool_stack::durability(&ductile) > crate::tool_stack::durability(&plain));
    // The modifiers themselves are recorded in the tool's modifier list.
    let Nbt::List(listed) = &sharp["tic_modifiers"] else { panic!() };
    assert!(listed.iter().any(|entry| matches!(entry, Nbt::Compound(c) if c.get("name") == Some(&Nbt::String("tconstruct:sharpness".into())))));
}
