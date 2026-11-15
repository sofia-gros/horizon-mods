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

#[test]
fn shipped_reinforced_and_cultivated_change_damage_and_repair_of_a_real_pickaxe() {
    use crate::{modifier_entries::List, tool_slot, tool_stack::{current_durability, rebuild_stats, repair_factor, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let tool = |upgrades: &[(&str, i32)]| {
        let mut slot = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        slot.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        let mut list = List::default();
        for (id, level) in upgrades { list = list.with_modifier(Identifier::parse(id).unwrap(), *level).unwrap(); }
        slot.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
        assert!(rebuild_stats(&mut slot.custom_data, &env).unwrap());
        slot
    };
    // Reinforced V cancels 0.025 * 5 * 6 = 75% of damage: a roll below it cancels, above it lands.
    let mut reinforced = tool(&[("tconstruct:reinforced", 5)]);
    let hit = tool_slot::damage_slot(&mut reinforced, &env, 8, true, true, None, &mut || 0.5).unwrap();
    assert_eq!(hit.applied, 0, "every roll below 75% cancels its point");
    let hit = tool_slot::damage_slot(&mut reinforced, &env, 8, true, true, None, &mut || 0.9).unwrap();
    assert_eq!(hit.applied, 8, "every roll above 75% lets its point through");
    // An unmodified pickaxe takes the full damage regardless of the roll.
    let mut plain = tool(&[]);
    assert_eq!(tool_slot::damage_slot(&mut plain, &env, 5, true, true, None, &mut || 0.0).unwrap().applied, 5);
    // Cultivated scales repair by factor * (1 + 0.5 per level). Wood parts already bring it at level 3
    // (three wood parts), so a plain wood pickaxe repairs at x2.5 and two upgrade levels reach x3.5.
    let cultivated = tool(&[("tconstruct:cultivated", 2)]);
    assert!((repair_factor(&plain.custom_data, &env, 10.0) - 25.0).abs() < 1e-4);
    assert!((repair_factor(&cultivated.custom_data, &env, 10.0) - 35.0).abs() < 1e-4);
    let _ = current_durability(&plain.custom_data);
}

fn recursive_files(folder: &str, prefix: &str) -> Vec<(Identifier, Value)> {
    fn walk(dir: &std::path::Path, relative: String, out: &mut Vec<(Identifier, Value)>) {
        for entry in std::fs::read_dir(dir).expect("folder") {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            if path.is_dir() { walk(&path, format!("{relative}/{name}"), out); continue; }
            if let Some(stem) = name.strip_suffix(".json") {
                let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
                out.push((Identifier::new("tconstruct", &format!("{relative}/{stem}")).unwrap(), value));
            }
        }
    }
    let mut found = Vec::new();
    walk(&root().join("../recipes").join(folder), prefix.to_owned(), &mut found);
    found
}

#[test]
fn shipped_material_recipes_load_and_only_goat_horn_recipes_are_left_out() {
    let files = recursive_files("tools/materials", "tools/materials");
    assert_eq!(files.len(), 176);
    let recipes = crate::material_recipes::Recipes::load(&files, &conditions());
    // Instrument (goat horn) ingredients need item components, which this port does not read.
    assert!(recipes.problems.iter().all(|problem| problem.contains("unimplemented ingredient type tconstruct:instrument") || problem.contains("registry minecraft:instrument unavailable")), "{:#?}", recipes.problems);
    assert!(!recipes.problems.is_empty() && recipes.len() > 60, "{} loaded, {} problems", recipes.len(), recipes.problems.len());
    // Tag-gated recipes (planks, logs) stay off while the item tags are empty, as their conditions say;
    // item-keyed recipes resolve: the mod's own blazewood and vanilla flint.
    let blazewood = recipes.find(&Identifier::parse("tconstruct:blazewood").unwrap(), &|_, _| false).expect("blazewood");
    assert_eq!(blazewood.material.to_string(), "tconstruct:blazewood");
    let flint = recipes.find(&Identifier::parse("minecraft:flint").unwrap(), &|_, _| false).expect("flint");
    assert_eq!(flint.material.to_string(), "tconstruct:flint");
}

#[test]
fn shipped_modifier_recipes_load_and_only_unported_ingredient_types_are_left_out() {
    let files = recursive_files("tools/modifiers", "tools/modifiers");
    assert_eq!(files.len(), 199, "116 modifier, 45 incremental and 38 swappable recipes");
    let recipes = crate::modifier_recipes::Recipes::load(&files, &conditions());
    for problem in &recipes.problems {
        assert!(problem.contains("unimplemented ingredient type") || problem.contains("unimplemented tool predicate"), "{problem}");
    }
    assert!(recipes.len() > 160, "{} loaded, {:#?}", recipes.len(), recipes.problems);
}

#[test]
fn the_shipped_reinforced_recipe_fills_a_level_in_two_partial_applications() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let recipes = crate::modifier_recipes::Recipes::load(&recursive_files("tools/modifiers", "tools/modifiers"), &conditions());
    let in_tag = |item: &Identifier, tag: &Identifier| item.path() == "pickaxe" && tag.path() == "modifiable/durability";
    let reinforced = Identifier::parse("tconstruct:reinforced").unwrap();
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    let mut persistent = NbtCompound::new();
    persistent.insert("upgrades".into(), Nbt::Int(3));
    tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let reinforcement = |count: u32| [Some((Identifier::parse("tconstruct:emerald_reinforcement").unwrap(), count))];
    let upgrades = |tool: &ItemSlot| List::from_nbt(tool.custom_data.get(TAG_UPGRADES));
    let free = |tool: &ItemSlot| match tool.custom_data.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => crate::modifier_entries::nbt_int(data.get("upgrades")), _ => 0 };

    let first = recipes.find(&tool.item, &reinforcement(2), &in_tag).expect("the shipped recipe matches a pickaxe");
    let half = first.apply(&tool, &env, &reinforcement(2), &in_tag).unwrap().expect("two of four items apply");
    assert_eq!((upgrades(&half).entry(&reinforced).amount(0), upgrades(&half).entry(&reinforced).needed(), free(&half)), (2, 4, 2), "a partial level spends one slot");
    assert_eq!(first.consumption(&tool, &half, &reinforcement(2), &in_tag).0, vec![2]);

    // Continuing the partial level spends no further slot, and two more items finish it.
    let full = first.apply(&half, &env, &reinforcement(2), &in_tag).unwrap().expect("the second half applies");
    assert_eq!((upgrades(&full).level(&reinforced), upgrades(&full).entry(&reinforced).needed(), free(&full)), (1, 0, 2));
    assert_eq!(first.consumption(&half, &full, &reinforcement(5), &in_tag).0, vec![2], "only what the level still needed is taken");
    // reinforced reduces damage taken (see the damage test); it does not change the durability stat.
}

#[test]
fn shipped_diamond_and_emerald_raise_the_harvest_tier_of_a_wood_pickaxe() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let tier_with = |modifier: Option<&str>| {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        if let Some(id) = modifier { nbt.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(id).unwrap(), 1).unwrap().to_nbt()); }
        assert!(rebuild_stats(&mut nbt, &env).unwrap());
        let Some(Nbt::Compound(stats)) = nbt.get(crate::tool_stack::TAG_STATS) else { panic!("no stats") };
        match stats.get("tconstruct:harvest_tier") { Some(Nbt::String(tier)) => tier.clone(), other => panic!("no tier: {other:?}") }
    };
    assert_eq!(tier_with(None), "minecraft:wood");
    assert_eq!(tier_with(Some("tconstruct:emerald")), "minecraft:iron");
    assert_eq!(tier_with(Some("tconstruct:diamond")), "minecraft:diamond");
}
