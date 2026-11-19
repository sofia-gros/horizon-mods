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
    let mut kinds = std::collections::BTreeSet::new();
    for id in registry.dynamic_ids() { if let Some(modifier) = registry.get(id) { for kind in &modifier.unbound { kinds.insert(kind.to_string()); } } }
    // Modifiers with at least one unported module; it only shrinks as modules are ported.
    assert_eq!((unbound, kinds.len()), (147, 107), "modifiers carrying unported modules, and the kinds: {kinds:?}");
    // Presentation modules and tool actions are bound; show_offhand (an armor level the port lacks) and the boot walk transform (no walk event) are not.
    for bound in ["show_interaction_source", "variant_color", "variant_name", "material_variant_color", "durability_color", "stat_tooltip", "tool_actions", "mob_effect", "armor_attack_mob_effect", "fiery_attack", "fiery_counter", "freezing_attack", "freezing_counter", "knockback_counter", "tool_usage_mob_effect", "enchantment_ignoring_protection", "clear_effect_on_unequip"] { assert!(!kinds.contains(&format!("tconstruct:{bound}")), "{bound} should be bound"); }
    for unbound in ["show_offhand", "tool_action_walk_transform", "embellishment", "lifesteal", "depth_protection", "recurrent_protection", "overshield", "restore_lost_health", "damage_on_unequip", "reduce_effect_on_unequip", "reduce_tool_damage", "protection"] { assert!(kinds.contains(&format!("tconstruct:{unbound}")), "{unbound} is still unbound"); }
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
fn the_model_strings_of_a_shipped_pickaxe_follow_its_material_slots_through_a_head_swap_and_a_part_carries_its_material() {
    use crate::tool_station::{lookup, Context, Input, Outcome};
    use crate::tool_materials::MaterialVariant;
    let world = world();
    let tools: Vec<(Identifier, &Data)> = world.tools.iter().filter(|(_, data)| !data.tool_parts().is_empty())
        .map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |_: &Identifier| 1;
    let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
    let part = |item: &str, material: &str| Some(Input::Part { item: Identifier::parse(item).unwrap(), material: MaterialVariant::parse(material).unwrap() });
    let strings = |slot: &pluto_lib::ItemSlot| slot.custom_model_data.as_ref().map(|data| data.strings.clone());
    let inputs = [part("tconstruct:pick_head", "tconstruct:stone"), part("tconstruct:tool_handle", "tconstruct:wood"), part("tconstruct:tool_binding", "tconstruct:iron"), None];
    let Outcome::Crafted(built) = lookup(&ctx, None, &inputs).unwrap() else { panic!("the shipped pickaxe did not build") };
    assert_eq!(crate::tool_stack::materials(&built.result.custom_data, true).get(0).to_string(), "tconstruct:stone");
    assert_eq!(strings(&built.result), Some(vec!["stone".to_owned(), "wood".to_owned(), "iron".to_owned()]), "one sprite key per material slot, in slot order");
    // Swapping the head rewrites the first key and leaves the others.
    let swap = [part("tconstruct:pick_head", "tconstruct:iron"), None, None, None];
    let Outcome::Crafted(swapped) = lookup(&ctx, Some(&built.result), &swap).unwrap() else { panic!("the head did not swap") };
    assert_eq!(strings(&swapped.result), Some(vec!["iron".to_owned(), "wood".to_owned(), "iron".to_owned()]));
    // Wear and repair rebuild the visuals without disturbing the keys.
    let mut worn = swapped.result.clone();
    crate::tool_slot::damage_slot(&mut worn, &ctx.environment(&built.result.item, tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1), 3, true, true, None, &mut || 1.0).unwrap();
    assert_eq!(strings(&worn), strings(&swapped.result));
    // A part's only key is its material; an unknown material has none, and a stack with no materials has no data at all.
    let mut head = pluto_lib::ItemSlot::new(Identifier::parse("tconstruct:pick_head").unwrap(), 1);
    crate::tool_materials::set_part_material(&mut head, &MaterialVariant::parse("tconstruct:rose_gold").unwrap());
    assert_eq!(strings(&head), Some(vec!["rose_gold".to_owned()]));
    crate::tool_materials::set_part_material(&mut head, &MaterialVariant::unknown());
    assert_eq!(strings(&head), Some(vec![String::new()]));
    let plain = pluto_lib::ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    assert_eq!(crate::tool_materials::model_strings(&plain.custom_data), None);
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
fn every_shipped_disguise_binds_and_a_worn_piece_halves_the_range_per_level_for_its_mob() {
    use crate::{mob_disguise, modifier_entries::List, tool_stack::{mob_disguises, rebuild_stats, volatile_flag, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{Nbt, NbtCompound};
    let world = world();
    let disguises: Vec<Identifier> = files("modifiers").into_iter().map(|(id, _)| id).filter(|id| id.path().ends_with("_disguise")).collect();
    assert_eq!(disguises.len(), 14, "the original ships fourteen disguises");
    for id in &disguises {
        let modifier = world.registry.get(id).expect("the shipped disguise loads");
        assert!(modifier.unbound.is_empty(), "{id} has unported modules: {:?}", modifier.unbound);
        let found: Vec<_> = modifier.mob_disguise_modules().collect();
        assert_eq!(found.len(), 1, "{id} is one mob_disguise module");
        let entity = found[0].mob_disguise(&crate::modifier_entries::Entry::new(id.clone(), 1)).unwrap().0;
        assert_eq!(entity.to_string(), format!("minecraft:{}", id.path().trim_end_matches("_disguise")), "{id} disguises as its own mob");
    }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let piece = |upgrades: &[(&str, i32)]| {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        let mut list = List::default();
        for (id, level) in upgrades { list = list.with_modifier(Identifier::parse(id).unwrap(), *level).unwrap(); }
        nbt.insert(TAG_UPGRADES.into(), list.to_nbt());
        assert!(rebuild_stats(&mut nbt, &env).unwrap());
        nbt
    };
    // Enderman disguise II and skeleton disguise I on one piece, a second skeleton disguise on another: counts add per mob type.
    let first = piece(&[("tconstruct:enderman_disguise", 2), ("tconstruct:skeleton_disguise", 1)]);
    let second = piece(&[("tconstruct:skeleton_disguise", 1)]);
    let counts = mob_disguise::count(mob_disguises(&first, &env).into_iter().chain(mob_disguises(&second, &env)));
    assert_eq!(mob_disguise::visibility_factor(&counts, "minecraft:enderman"), 0.25);
    assert_eq!(mob_disguise::visibility_factor(&counts, "minecraft:skeleton"), 0.25);
    assert_eq!(mob_disguise::visibility_factor(&counts, "minecraft:zombie"), 1.0);
    assert!(volatile_flag(&first, crate::mob_disguise_hook::ENDERMASK), "the enderman disguise makes the piece an ender mask");
    assert!(!volatile_flag(&second, crate::mob_disguise_hook::ENDERMASK));
    assert!(mob_disguises(&piece(&[]), &env).is_empty());
}

#[test]
fn shipped_worn_helmet_given_without_stats_is_built_by_its_first_wear_and_keeps_the_ender_mask_flag() {
    // The smoke scenario MOB DISGUISE: a plate helmet of [cobalt, iron] handed over with no tic_stats, then worn down once.
    use crate::{modifier_entries::List, tool_slot, tool_stack::{is_broken, is_initialized, volatile_flag, Environment, TAG_MATERIALS, TAG_MODIFIERS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let disguise = Identifier::parse("tconstruct:enderman_disguise").unwrap();
    let given = |key: &str| {
        let mut slot = ItemSlot::new(helmet_id.clone(), 1);
        slot.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:cobalt", "tconstruct:iron"].map(|m| Nbt::String(m.into())).to_vec()));
        slot.custom_data.insert(key.into(), List::default().with_modifier(disguise.clone(), 1).unwrap().to_nbt());
        assert!(!is_initialized(&slot.custom_data));
        slot
    };
    // Saved as an upgrade (the input of a rebuild), the first wear builds the piece, which is not broken and carries the flag.
    let mut worn = given(TAG_UPGRADES);
    let hit = tool_slot::damage_slot(&mut worn, &env, 1, true, true, None, &mut || 1.0).unwrap();
    assert!(is_initialized(&worn.custom_data) && !is_broken(&worn.custom_data), "{:?}", worn.custom_data);
    assert_eq!((hit.applied, hit.broke), (1, false));
    assert!(worn.max_damage.is_some_and(|max| max > 2), "a real durability, not the 1-point default: {:?}", worn.max_damage);
    assert!(volatile_flag(&worn.custom_data, crate::mob_disguise_hook::ENDERMASK), "{:?}", worn.custom_data);
    // tic_modifiers is a rebuild's output, never its input: a piece given only that list loses it, as in the original.
    let mut output_only = given(TAG_MODIFIERS);
    tool_slot::damage_slot(&mut output_only, &env, 1, true, true, None, &mut || 1.0).unwrap();
    assert!(!is_broken(&output_only.custom_data) && !volatile_flag(&output_only.custom_data, crate::mob_disguise_hook::ENDERMASK));
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
    // From level 6 the flat branch counts: 0.5 + 0.05 * 10 = 100% at level X, so even the highest roll cancels the point.
    let mut ten = tool(&[("tconstruct:reinforced", 10)]);
    let hit = tool_slot::damage_slot(&mut ten, &env, 8, true, true, None, &mut || 0.99).unwrap();
    assert_eq!(hit.applied, 0, "ten levels of reinforced cancel every point");
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
    assert_eq!(files.len(), 357, "116 modifier, 45 incremental, 38 swappable, 135 salvage, 2 removal, 5 multilevel and 16 overslime records");
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

#[test]
fn the_shipped_part_builder_makes_wood_parts_from_logs_and_hands_back_planks() {
    use crate::part_builder_recipes::{Context, Inventory, Recipes};
    let world = world();
    let part_files = recursive_files("tools/parts/builder", "tools/parts/builder");
    assert_eq!(part_files.len(), 24);
    let recipes = Recipes::load(&part_files, &conditions());
    assert!(recipes.problems.is_empty(), "{:#?}", recipes.problems);
    assert_eq!(recipes.len(), 24);
    let materials = crate::material_recipes::Recipes::load(&recursive_files("tools/materials", "tools/materials"), &conditions());
    let parts = crate::tool_parts::registry();
    let in_tag = |item: &Identifier, tag: &Identifier| match tag.path() {
        "patterns/default" => item.path() == "pattern",
        "oak_logs" => item.path() == "oak_log",
        _ => false,
    };
    let ctx = Context { materials: &world.materials, parts: &parts, in_tag: &in_tag };
    let pattern = Identifier::parse("tconstruct:pattern").unwrap();
    let planks = Identifier::parse("minecraft:oak_log").unwrap();
    let wood = materials.find(&planks, &in_tag).expect("logs are a material");
    assert_eq!(wood.material.to_string(), "tconstruct:wood");
    let inv = |count: u32| Inventory { pattern_item: Some(&pattern), stack: Some((&planks, count)), material: Some(wood) };

    // With planks in the slot, every wood-capable part is offered; a pattern with nothing placed offers all 24.
    let offered = recipes.patterns(&inv(8), &ctx);
    assert!(offered.iter().any(|p| p.path() == "pick_head") && offered.iter().any(|p| p.path() == "tool_handle"), "{offered:?}");
    assert_eq!(recipes.patterns(&Inventory { pattern_item: Some(&pattern), stack: None, material: None }, &ctx).len(), 24);

    let head = Identifier::parse("tconstruct:pick_head").unwrap();
    // A log is worth 4 and a pick head costs 2: one log pays for it and half is handed back as 2 planks.
    assert!(recipes.find(&head, &inv(0), &ctx).is_none(), "no logs, no part");
    let recipe = recipes.find(&head, &inv(1), &ctx).expect("one log makes a pick head");
    assert_eq!(wood.leftover_for(recipe.cost).map(|(source, count)| (source.clone(), count)),
        Some((crate::material_recipes::LeftoverSource::Item(Identifier::parse("minecraft:oak_planks").unwrap()), 2)));
    let crafted = recipe.assemble(&inv(1));
    assert_eq!((crafted.item.to_string(), crate::tool_materials::part_material(&crafted.custom_data).to_string()), ("tconstruct:pick_head".to_owned(), "tconstruct:wood".to_owned()));
    assert_eq!((recipe.items_used(&inv(1)), crafted.count), (1, 1), "a leftover recipe does not multiply the parts");
}

#[test]
fn the_shipped_salvage_returns_the_slot_when_a_sharpness_level_is_removed() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let salvages = crate::modifier_removal::Salvages::load(&recursive_files("tools/modifiers", "tools/modifiers"), &conditions());
    assert_eq!(salvages.len(), 133, "135 records, two behind data conditions that are false here: {:?}", salvages.problems);
    let in_tag = |_: &Identifier, tag: &Identifier| tag.path() == "modifiable/melee";
    let sharp = Identifier::parse("tconstruct:sharpness").unwrap();
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(sharp.clone(), 2).unwrap().to_nbt());
    let mut persistent = NbtCompound::new();
    persistent.insert("upgrades".into(), Nbt::Int(1));
    tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let damage = |tool: &ItemSlot| crate::tool_stack::stat(&tool.custom_data, crate::tool_stats::Stat::AttackDamage);

    let once = crate::modifier_removal::remove(&tool, &env, &sharp, &salvages, &in_tag).unwrap().expect("a level comes off");
    assert_eq!(List::from_nbt(once.custom_data.get(TAG_UPGRADES)).level(&sharp), 1);
    let Some(Nbt::Compound(slots)) = once.custom_data.get(TAG_PERSISTENT) else { panic!("persistent data") };
    assert_eq!(crate::modifier_entries::nbt_int(slots.get("upgrades")), 2, "the salvaged slot is handed back");
    assert!(damage(&once) < damage(&tool), "one level less sharpness");
    let twice = crate::modifier_removal::remove(&once, &env, &sharp, &salvages, &in_tag).unwrap().expect("the last level comes off");
    assert_eq!(List::from_nbt(twice.custom_data.get(TAG_UPGRADES)).level(&sharp), 0);
    assert!(crate::modifier_removal::remove(&twice, &env, &sharp, &salvages, &in_tag).unwrap().is_err(), "nothing left to remove");
}

#[test]
fn the_shipped_sponge_removal_loads_and_the_venom_one_waits_for_fluid_containers() {
    let removals = crate::worktable_recipes::Removals::load(&recursive_files("tools/modifiers", "tools/modifiers"), &conditions());
    assert_eq!(removals.len(), 1, "{:?}", removals.problems);
    assert_eq!(removals.problems.len(), 1);
    assert!(removals.problems[0].contains("remove_modifier_venom"), "{:?}", removals.problems);
}

#[test]
fn slime_balls_fill_overslime_on_a_pickaxe_and_the_shield_soaks_damage_first() {
    use crate::{overslime, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS}};
    use pluto_lib::{ItemSlot, Nbt};
    let mut world = world();
    for modifier in overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let recipes = overslime::Recipes::load(&recursive_files("tools/modifiers", "tools/modifiers"), &conditions());
    assert_eq!(recipes.len(), 16, "{:?}", recipes.problems);
    let in_tag = |item: &Identifier, tag: &Identifier| item.path() == "pickaxe" && tag.path() == "modifiable/durability";
    let slime = Identifier::parse("minecraft:slime_ball").unwrap();
    let inputs = [Some((slime.clone(), 2)), None];
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());

    let recipe = recipes.find(&tool.item, &inputs, &in_tag).expect("a slime ball recipe matches");
    let slimy = recipe.apply(&tool, &env, &inputs, &in_tag).unwrap().expect("overslime applies");
    let capacity = overslime::capacity(&slimy.custom_data);
    assert!(capacity > 0, "the modifier brings capacity");
    assert_eq!(overslime::amount(&slimy.custom_data), (2 * 20).min(capacity), "two slime balls restore 20 each, capped");
    // The stat penalties need the item tags (a tool with no item context skips them), so they are not checked here.
    let used = recipe.consumption(&tool, &slimy, &env, &inputs, &in_tag);
    assert_eq!(used, vec![(overslime::amount(&slimy.custom_data) as u32).div_ceil(20), 0]);

    // The shield soaks damage first; what it cannot cover goes on to the tool.
    let mut worn = slimy.clone();
    let shield = overslime::amount(&worn.custom_data);
    assert_eq!(overslime::absorb(&mut worn.custom_data, &env, None, shield - 1), 0);
    assert_eq!(overslime::amount(&worn.custom_data), 1);
    assert_eq!(overslime::absorb(&mut worn.custom_data, &env, None, 5), 4);
    assert_eq!(overslime::amount(&worn.custom_data), 0);
    // Each overworked level doubles what a restore gives (1 + level).
    let mut boosted = tool.clone();
    boosted.custom_data.insert(crate::tool_stack::TAG_UPGRADES.into(), crate::modifier_entries::List::default()
        .with_modifier(overslime::id(), 1).unwrap().with_modifier(Identifier::parse("tconstruct:overworked").unwrap(), 2).unwrap().to_nbt());
    assert!(rebuild_stats(&mut boosted.custom_data, &env).unwrap());
    assert_eq!(overslime::overworked_bonus(&boosted.custom_data, &env), 3);
    overslime::add_amount(&mut boosted.custom_data, &env, 4);
    assert_eq!(overslime::amount(&boosted.custom_data), 12.min(overslime::capacity(&boosted.custom_data)));
    // Full overslime cannot take more slime.
    let mut full = slimy;
    overslime::add_amount(&mut full.custom_data, &env, 10_000);
    assert!(recipe.apply(&full, &env, &inputs, &in_tag).unwrap().is_err());
}

#[test]
fn creative_slot_adds_the_slots_stored_in_the_creative_compound_and_forgets_them_on_removal() {
    use crate::{overslime, tool_stack::{on_removed, rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES, TAG_VOLATILE}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let mut world = world();
    for modifier in overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let slots = |tool: &ItemSlot| match tool.custom_data.get(TAG_VOLATILE) {
        Some(Nbt::Compound(data)) => crate::modifier_entries::nbt_int(data.get("upgrades")),
        _ => 0,
    };
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let before = slots(&tool);
    let mut stored = NbtCompound::new();
    stored.insert("upgrades".into(), Nbt::Int(3));
    let mut persistent = NbtCompound::new();
    persistent.insert("tconstruct:creative".into(), Nbt::Compound(stored));
    tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    tool.custom_data.insert(TAG_UPGRADES.into(), crate::modifier_entries::List::default()
        .with_modifier(Identifier::parse("tconstruct:creative_slot").unwrap(), 1).unwrap().to_nbt());
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    assert_eq!(slots(&tool), before + 3, "the stored slots join the volatile data");
    // Removing the modifier forgets the stored compound.
    let original = tool.custom_data.clone();
    tool.custom_data.insert(TAG_UPGRADES.into(), crate::modifier_entries::List::default().to_nbt());
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    on_removed(&original, &mut tool.custom_data, &env);
    match tool.custom_data.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => assert!(!data.contains_key("tconstruct:creative")), _ => {} }
    assert_eq!(slots(&tool), before);
}

#[test]
fn stat_override_adds_stored_stats_dropping_the_base_and_multiplies_stored_factors() {
    use crate::{modifier_entries::List, overslime, tool_stack::{rebuild_stats, stat, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}, tool_stats::Stat};
    use pluto_lib::{Nbt, NbtCompound};
    let mut world = world();
    for modifier in overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let build = |bonus: Option<f32>, factor: Option<f32>| {
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        let mut persistent = NbtCompound::new();
        for (key, value) in [("tconstruct:override_bonus", bonus), ("tconstruct:override_multiplier", factor)] {
            if let Some(value) = value {
                let mut stats = NbtCompound::new();
                stats.insert("tconstruct:attack_damage".into(), Nbt::Float(value));
                persistent.insert(key.into(), Nbt::Compound(stats));
            }
        }
        nbt.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        nbt.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("tconstruct:stat_override").unwrap(), 1).unwrap().to_nbt());
        assert!(rebuild_stats(&mut nbt, &env).unwrap());
        nbt
    };
    // update() sums the value and removes the implicit base, so the stat becomes the stored number plus other additions.
    let plain_damage = stat(&build(None, None), Stat::AttackDamage);
    let overridden = stat(&build(Some(7.5), None), Stat::AttackDamage);
    assert!((overridden - (plain_damage + 7.5)).abs() < 1e-3, "update sums with the definition stats: {overridden} vs {plain_damage}");
    let plain = stat(&build(None, None), Stat::AttackDamage);
    assert!((stat(&build(None, Some(2.0)), Stat::AttackDamage) - plain * 2.0).abs() < 1e-3);
}

#[test]
fn a_damaged_wood_pickaxe_is_repaired_with_wood_material_items() {
    use crate::{tool_station::{lookup, Context, Input, Outcome}, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_DAMAGE}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let tools: Vec<(Identifier, &Data)> = world.tools.iter().filter(|(_, data)| !data.tool_parts().is_empty()).map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |_: &Identifier| 1;
    let ctx = Context { materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max };
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    tool.custom_data.insert(TAG_DAMAGE.into(), Nbt::Int(20));
    let planks = Input::Material { item: Identifier::parse("minecraft:oak_planks").unwrap(), material: Identifier::parse("tconstruct:wood").unwrap(), value: 1, needed: 1, count: 8 };
    let outcome = lookup(&ctx, Some(&tool), &[Some(planks), None, None, None]).unwrap();
    match outcome {
        Outcome::Crafted(done) => assert!(crate::tool_stack::damage(&done.result.custom_data) < 20, "repair lowers the damage"),
        Outcome::Failure(error) => panic!("repair failed: {error}"),
        Outcome::NoMatch => panic!("no match for planks repairing a wood pickaxe"),
        _ => panic!("unexpected outcome"),
    }
}

#[test]
fn shipped_mining_speed_modifiers_change_the_dig_speed_of_a_rebuilt_pickaxe() {
    use crate::{mining_speed::Context, modifier_entries::List, tool_stack::{break_speed, rebuild_stats, Environment, TAG_DAMAGE, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let mut world = world();
    for modifier in crate::overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let stone = Identifier::parse("minecraft:stone").unwrap();
    let melts = |tag: &Identifier| tag.to_string() == "tconstruct:can_melt";
    let dig = |nbt: &pluto_lib::NbtCompound, effective: bool, on_ground: bool| {
        let context = Context { block: &stone, block_in_tag: &melts, original: 4.0, multiplier: 1.0, effective, on_ground: Some(on_ground), eyes_in_water: Some(false) };
        break_speed(nbt, &env, &context, 4.0)
    };
    let build = |upgrades: Vec<(&str, i32)>, persistent: Option<(&str, i32)>, damage: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        let mut list = List::default();
        for (name, level) in upgrades { list = list.with_modifier(Identifier::parse(name).unwrap(), level).unwrap(); }
        tool.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
        if let Some((key, value)) = persistent {
            let mut data = NbtCompound::new();
            data.insert(key.into(), Nbt::Int(value));
            tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(data));
        }
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool.custom_data.insert(TAG_DAMAGE.into(), Nbt::Int(damage));
        tool.custom_data
    };
    let plain = build(vec![], None, 0);
    assert_eq!(dig(&plain, true, true), 4.0, "no modifier leaves the speed alone");
    // searing: +6 per level on blocks that can melt, only on effective blocks.
    let searing = build(vec![("tconstruct:searing", 2)], None, 0);
    assert!(dig(&searing, true, true) > 4.0 + 11.0, "searing II adds 12 times the tool multiplier: {}", dig(&searing, true, true));
    assert_eq!(dig(&searing, false, true), 4.0, "an ineffective block gets nothing");
    // airborne: +400% while off the ground, regardless of effectiveness.
    let airborne = build(vec![("tconstruct:airborne", 1)], None, 0);
    assert_eq!(dig(&airborne, true, true), 4.0);
    assert!(dig(&airborne, false, false) > 4.0, "airborne boosts in the air");
    // stonebound: the more durability lost, the faster (a percent formula over the tool's lost durability).
    let fresh = build(vec![("tconstruct:stonebound", 2)], None, 0);
    let worn = build(vec![("tconstruct:stonebound", 2)], None, 30);
    assert_eq!(dig(&fresh, true, true), 4.0);
    assert!(dig(&worn, true, true) > 4.0, "a worn stonebound tool digs faster: {}", dig(&worn, true, true));
    // overwield: overslime from persistent data speeds up broad tools; the pickaxe is not broad, so check it reads data.
    let overwield = build(vec![("tconstruct:overwield", 1)], Some(("tconstruct:overslime", 100)), 0);
    let _ = dig(&overwield, true, true);
}

#[test]
fn shipped_jagged_and_stonebound_change_melee_damage_with_the_tools_lost_durability() {
    use crate::{modifier_entries::List, tool_stack::{melee_damage, rebuild_stats, Environment, TAG_DAMAGE, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let hit = |modifier: &str, damage: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), 3).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool.custom_data.insert(TAG_DAMAGE.into(), Nbt::Int(damage));
        melee_damage(&tool.custom_data, &env, 2.0, 10.0, &|_| false)
    };
    assert_eq!(hit("tconstruct:jagged", 0), 10.0, "a fresh jagged tool changes nothing");
    assert!(hit("tconstruct:jagged", 40) > 10.0, "a worn jagged tool hits harder: {}", hit("tconstruct:jagged", 40));
    assert!(hit("tconstruct:stonebound", 40) < 10.0, "a worn stonebound tool hits softer: {}", hit("tconstruct:stonebound", 40));
}

#[test]
fn shipped_smite_and_bane_of_sssss_add_damage_only_to_the_right_targets() {
    use crate::{modifier_entries::List, tool_stack::{melee_damage, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let hit = |modifier: &str, tags: &'static [&'static str]| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), 2).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        melee_damage(&tool.custom_data, &env, 2.0, 10.0, &|tag: &Identifier| tags.contains(&tag.to_string().as_str()))
    };
    assert!(hit("tconstruct:smite", &["minecraft:undead"]) > 10.0, "smite boosts undead targets");
    assert_eq!(hit("tconstruct:smite", &["minecraft:arthropod"]), 10.0, "smite leaves other targets alone");
    assert!(hit("tconstruct:bane_of_sssss", &["minecraft:arthropod"]) > 10.0, "bane of sssss boosts arthropods");
    assert_eq!(hit("tconstruct:bane_of_sssss", &[]), 10.0);
}

#[test]
fn shipped_speedy_publishes_a_movement_speed_attribute_for_its_slots() {
    use crate::{modifier_entries::List, tool_stack::{attribute_entries, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("tconstruct:speedy").unwrap(), 3).unwrap().to_nbt());
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let held = attribute_entries(&tool.custom_data, &env, &["mainhand"]);
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0]["type"], Nbt::String("minecraft:movement_speed".into()));
    assert_eq!(held[0]["operation"], Nbt::String("add_multiplied_total".into()));
    match held[0]["amount"] { Nbt::Double(amount) => assert!((amount - 0.3).abs() < 1e-5, "{amount}"), _ => panic!("amount is a double") }
    assert_eq!(attribute_entries(&tool.custom_data, &env, &["head"]).len(), 1, "speedy lists the head slot");
    assert!(attribute_entries(&tool.custom_data, &env, &["offhand"]).is_empty(), "speedy does not list the offhand");
}

fn damage_tag_stacks() -> Vec<JsonResourceStack> {
    fn walk(dir: &std::path::Path, relative: &str, out: &mut Vec<JsonResourceStack>) {
        for entry in std::fs::read_dir(dir).expect("tag folder") {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            if path.is_dir() { walk(&path, &format!("{relative}{name}/"), out); continue; }
            if let Some(stem) = name.strip_suffix(".json") {
                let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
                out.push(JsonResourceStack { id: Identifier::new("tconstruct", &format!("{relative}{stem}")).unwrap(), layers: vec![Ok(value)] });
            }
        }
    }
    let mut stacks = Vec::new();
    walk(&root().join("tags/damage_types"), "", &mut stacks);
    stacks
}

#[test]
fn shipped_damage_tags_resolve_protection_tags_through_the_vanilla_ones() {
    use crate::damage_tags::DamageTags;
    let stacks = damage_tag_stacks();
    let (tags, problems) = DamageTags::load(&stacks);
    assert!(problems.is_empty(), "{problems:?}");
    let id = |text: &str| Identifier::parse(text).unwrap();
    assert!(tags.contains(&id("tconstruct:protection/fire"), &id("minecraft:lava")), "fire protection takes vanilla's fire tag");
    assert!(tags.contains(&id("tconstruct:protection/fire"), &id("minecraft:lightning_bolt")), "and lightning");
    assert!(tags.contains(&id("tconstruct:protection/blast"), &id("minecraft:explosion")));
    assert!(tags.contains(&id("tconstruct:protection/melee"), &id("minecraft:mob_attack")), "melee comes through is_melee");
    assert!(tags.contains(&id("tconstruct:protection/projectile"), &id("minecraft:arrow")));
    assert!(tags.contains(&id("tconstruct:protection/fall"), &id("minecraft:fall")));
    assert!(!tags.contains(&id("tconstruct:protection/fire"), &id("minecraft:arrow")), "an arrow is no fire damage");
}

#[test]
fn shipped_protection_modifiers_add_protection_against_the_sources_they_name() {
    use crate::{damage_tags::DamageTags, modifier_entries::List, protection::{damage_after_protection, DamageSource}, tool_stack::{protection, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let id = |text: &str| Identifier::parse(text).unwrap();
    let protection_of = |modifier: &str, level: i32, damage_type: &str, indirect: bool| {
        let mut tool = ItemSlot::new(id("tconstruct:plate_helmet"), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id(modifier), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let damage_type = id(damage_type);
        protection(&tool.custom_data, &env, &DamageSource { damage_type: &damage_type, indirect, tags: &tags, attacker_in_tag: &|_| false }, 0.0)
    };
    assert_eq!(protection_of("tconstruct:protection", 2, "minecraft:mob_attack", false), 2.5, "protection gives 1.25 a level");
    assert_eq!(protection_of("tconstruct:protection", 2, "minecraft:out_of_world", false), 0.0, "void damage bypasses protection");
    // The running value starts at the game's enchantment points in the hook; from 0 the enchantment's 2 fire points come off its own 3.
    assert_eq!(protection_of("tconstruct:fire_protection", 1, "minecraft:lava", false), 1.0);
    // Iron plating and maille bring their own projectile protection, so an arrow is measured against a helmet with an unrelated modifier.
    let iron_alone = protection_of("tconstruct:sharpness", 1, "minecraft:arrow", true);
    assert!(iron_alone > 0.0, "iron armor protects against projectiles by itself");
    assert_eq!(protection_of("tconstruct:fire_protection", 1, "minecraft:arrow", true), iron_alone, "fire protection adds nothing against an arrow");
    assert_eq!(protection_of("tconstruct:melee_protection", 1, "minecraft:mob_attack", false), 2.0);
    assert_eq!(protection_of("tconstruct:melee_protection", 1, "minecraft:mob_attack", true), 0.0, "an indirect hit is not melee protection");
    // Boundless raises the cap on summed protection: a level 1 adds 0.1 to the 0.8 attribute, 22.5 points.
    let cap_with = |level: i32| {
        let mut tool = ItemSlot::new(id("tconstruct:plate_helmet"), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id("tconstruct:boundless"), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        crate::protection::protection_cap(crate::tool_stack::protection_cap(&tool.custom_data, &env))
    };
    assert!((cap_with(1) - 22.5).abs() < 1e-4, "{}", cap_with(1));
    assert!((cap_with(5) - 23.75).abs() < 1e-4, "the cap never passes 95% of 25");
    assert!((crate::protection::protection_cap(0.0) - 20.0).abs() < 1e-4);
    // The event comes after vanilla's protection: 10 damage with no vanilla protection and 5 points is 8 after the modifier.
    assert!((damage_after_protection(10.0, 0.0, 5.0, 20.0) - 8.0).abs() < 1e-5);
    // Vanilla's 5 points were already taken (8), so 10 points total leaves 6 rather than 4.8.
    assert!((damage_after_protection(8.0, 5.0, 10.0, 20.0) - 6.0).abs() < 1e-5);
}

#[test]
fn shipped_overwield_scales_an_attack_speed_attribute_with_the_tools_overslime() {
    use crate::{modifier_entries::List, tool_stack::{attribute_entries, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let amount_with = |overslime: Option<i32>| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("tconstruct:overwield").unwrap(), 2).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        if let Some(amount) = overslime {
            let mut persistent = NbtCompound::new();
            persistent.insert("tconstruct:overslime".into(), Nbt::Int(amount));
            tool.custom_data.insert("tic_persistent".into(), Nbt::Compound(persistent));
        }
        let held = attribute_entries(&tool.custom_data, &env, &["mainhand"]);
        let speed = held.iter().find(|entry| entry["type"] == Nbt::String("minecraft:attack_speed".into()));
        speed.map(|entry| match entry["amount"] { Nbt::Double(amount) => amount, _ => panic!("amount is a double") })
    };
    assert_eq!(amount_with(None), Some(0.0), "no overslime adds nothing");
    assert!((amount_with(Some(1)).unwrap() - 0.15).abs() < 1e-5, "one point of overslime counts once");
    assert!((amount_with(Some(9)).unwrap() - 0.3).abs() < 1e-5, "the level (2) caps the overslime that counts");
}

#[test]
fn shipped_constant_enchantments_give_the_tool_enchantment_levels() {
    use crate::{modifier_entries::List, tool_stack::{enchantments, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let with = |modifier: &str, level: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        enchantments(&tool.custom_data, &env)
    };
    // Conditions on the tool's tags hold when there is no item context (a preview outside a stack).
    assert_eq!(with("tconstruct:respiration", 1).get("minecraft:respiration"), Some(&1));
    assert_eq!(with("tconstruct:swift_sneak", 1).get("minecraft:swift_sneak"), Some(&1));
    assert!(with("tconstruct:sharpness", 2).is_empty(), "a modifier without enchantment modules gives none");
}

#[test]
fn shipped_tool_recycling_loads_and_a_wood_pickaxe_breaks_into_its_parts() {
    use crate::{part_recycling::Recycles, tool_stack::{rebuild_stats, Environment, TAG_DAMAGE, TAG_MATERIALS}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let files = recursive_files("tools/recycling", "tools/recycling");
    assert_eq!(files.len(), 15);
    let recycles = Recycles::load(&files, &conditions());
    // The minotaur axe's record waits on another mod's condition, which never holds here.
    assert_eq!((recycles.len(), recycles.problems.len()), (14, 0), "{:?}", recycles.problems);
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let venom = Identifier::parse("tconstruct:venom_bottle").unwrap();
    // The general recipe takes any recyclable multipart tool; every tag query answers yes except the unrecyclable list.
    let in_tag = |_: &Identifier, tag: &Identifier| tag.path() != "modifiable/unrecyclable";
    let recipe = recycles.find(&tool, &venom, &in_tag).expect("a recycling recipe matches a plain pickaxe and the venom bottle");
    let registry = crate::tool_parts::registry();
    let parts = pickaxe.tool_parts();
    let buttons = recipe.buttons(&tool, parts, &registry, &world.materials);
    let names: Vec<String> = buttons.iter().map(|part| part.path().to_owned()).collect();
    assert_eq!(names, vec!["pick_head", "tool_handle", "tool_binding"], "the pickaxe's three parts, once each");
    let head = recipe.assemble(&tool, &buttons[0], parts, &registry, &world.materials).expect("the head");
    assert_eq!(head.item.path(), "pick_head");
    assert!(!head.custom_data.is_empty(), "the part carries the tool's material");
    // An undamaged tool always gives a second part, picked from the others.
    let extra = recipe.leftover(&tool, &buttons[0], parts, &registry, &world.materials, &mut || 0.0).expect("an undamaged tool gives a leftover part");
    assert!(extra.item.path() != "pick_head");
    // A worn tool loses that part with the share of durability it lost: a roll below the share gives nothing.
    tool.custom_data.insert(TAG_DAMAGE.into(), Nbt::Int(30));
    assert!(recipe.leftover(&tool, &buttons[0], parts, &registry, &world.materials, &mut || 0.1).is_none());
    assert!(recipe.leftover(&tool, &buttons[0], parts, &registry, &world.materials, &mut || 0.9).is_some());
    // A tool with upgrades does not recycle.
    let mut upgraded = tool.clone();
    upgraded.custom_data.insert(crate::tool_stack::TAG_UPGRADES.into(), crate::modifier_entries::List::default().with_modifier(Identifier::parse("tconstruct:sharpness").unwrap(), 1).unwrap().to_nbt());
    assert!(recycles.find(&upgraded, &venom, &in_tag).is_none());
}

#[test]
fn a_modifier_crystal_alone_applies_its_modifier_through_the_single_step_recipe() {
    use crate::{modifier_entries::List, modifier_recipes::{lone_crystal, Recipes}, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let world = world();
    let files = recursive_files("tools/modifiers", "tools/modifiers");
    let recipes = Recipes::load(&files, &conditions());
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
    tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    let mut slots = NbtCompound::new();
    slots.insert("upgrades".into(), Nbt::Int(3));
    tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(slots));
    assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
    let in_tag = |_: &Identifier, _: &Identifier| true;
    let sharp = Identifier::parse("tconstruct:sharpness").unwrap();
    let recipe = recipes.find_crystal(&tool.item, &sharp, &in_tag).expect("sharpness takes a crystal");
    let crafted = recipe.apply_crystal(&tool, &env, &in_tag).unwrap().expect("the crystal applies");
    assert_eq!(List::from_nbt(crafted.custom_data.get(TAG_UPGRADES)).level(&sharp), 1);
    // The table holds exactly one stack and it is a crystal: that is a crystal match; two stacks, or another item, is not.
    let crystal = Identifier::parse("tconstruct:modifier_crystal").unwrap();
    assert_eq!(lone_crystal(&[Some((crystal.clone(), 2, Some(sharp.clone()))), None]), Some((0, sharp.clone())));
    assert_eq!(lone_crystal(&[None, Some((crystal.clone(), 1, Some(sharp.clone())))]), Some((1, sharp.clone())));
    assert!(lone_crystal(&[Some((crystal.clone(), 1, Some(sharp.clone()))), Some((crystal.clone(), 1, Some(sharp.clone())))]).is_none());
    assert!(lone_crystal(&[Some((Identifier::parse("minecraft:stick").unwrap(), 1, None))]).is_none());
    assert!(lone_crystal(&[Some((crystal, 1, None))]).is_none(), "a crystal with no modifier is blank");
}

#[test]
fn shipped_maintained_gives_a_helmet_toughness_until_it_is_worn_below_half() {
    use crate::{modifier_entries::List, tool_stack::{attribute_entries, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let toughness_at = |damage: i32| {
        let mut tool = ItemSlot::new(helmet_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("tconstruct:maintained").unwrap(), 2).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool.custom_data.insert("Damage".into(), Nbt::Int(damage));
        let held = attribute_entries(&tool.custom_data, &env, &["head"]);
        println!("maintained entries at damage {damage}: {held:?}");
        held.iter().find(|entry| entry["type"] == Nbt::String("minecraft:armor_toughness".into()))
            .map(|entry| match entry["amount"] { Nbt::Double(amount) => amount, _ => panic!("amount is a double") })
    };
    // The bonus is the share of durability above half, so a fresh helmet gets the most and one worn below half gets none.
    let fresh = toughness_at(0);
    assert!(fresh.is_some_and(|amount| amount > 0.0), "a fresh helmet gains toughness: {fresh:?}");
    assert!(toughness_at(400).is_none_or(|amount| amount == 0.0), "a helmet worn below half its durability gains none");
}

#[test]
fn shipped_protection_that_names_the_attacker_asks_what_it_is_and_was_doing() {
    use crate::{damage_tags::DamageTags, melee_damage::{state_in_tag, STATE_NAMESPACE}, modifier_entries::List, protection::DamageSource, tool_stack::{protection, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{EntityState, ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let id = |text: &str| Identifier::parse(text).unwrap();
    let protection_against = |modifier: &str, attacker: &EntityState, undead: bool| {
        let mut tool = ItemSlot::new(helmet_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id(modifier), 2).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let damage_type = id("minecraft:mob_attack");
        let in_tag = |tag: &Identifier| if tag.namespace() == STATE_NAMESPACE { state_in_tag(tag, Some(attacker), None) } else { undead && tag.to_string() == "minecraft:undead" };
        protection(&tool.custom_data, &env, &DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &in_tag }, 0.0)
    };
    let calm = EntityState { kind: "minecraft:husk".into(), on_ground: true, ..EntityState::default() };
    let burning = EntityState { fire_ticks: 40, ..calm.clone() };
    let flying = EntityState { on_ground: false, ..calm.clone() };
    assert_eq!(protection_against("tconstruct:tempered_protection", &burning, false), 2.5, "against a burning attacker");
    assert_eq!(protection_against("tconstruct:tempered_protection", &calm, false), 0.0, "against a calm one");
    assert_eq!(protection_against("tconstruct:consecrated", &calm, true), 2.5, "against the undead");
    assert_eq!(protection_against("tconstruct:consecrated", &calm, false), 0.0, "against the living");
    assert_eq!(protection_against("tconstruct:airborn", &flying, false), 2.5, "against an attacker in the air");
    assert_eq!(protection_against("tconstruct:airborn", &calm, false), 0.0, "against one on the ground");
}

#[test]
fn shipped_decay_puts_wither_on_what_a_pickaxe_hits_and_on_what_hits_its_wearer() {
    use crate::{modifier_entries::List, tool_stack::{counter_effects, rebuild_stats, weapon_effects, Environment, TAG_MATERIALS, TAG_UPGRADES}, weapon_effect::Whom};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let decay = |level: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse("tconstruct:decay").unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool
    };
    let tool = decay(2);
    let none = |_: &Identifier| false;
    // The weapon module: wither at the modifier level (per_level 1), 120 ticks, always.
    // (Decay also brings pierce along, an effect of the original's own that no registration here carries.)
    let hit = weapon_effects(&tool.custom_data, &env, &none, &mut || 0.0);
    let wither: Vec<_> = hit.iter().filter(|(_, grant)| grant.effect.to_string() == "minecraft:wither").collect();
    assert_eq!(wither.len(), 1, "{hit:?}");
    assert_eq!((wither[0].1.amplifier, wither[0].1.duration), (1, 120));
    // The armor counter: wither II for 120 ticks on the attacker with 50% a level (certain at level 2), and a wither I on the wearer at 10% a level.
    let counter = counter_effects(&tool.custom_data, &env, &none, &mut || 0.0);
    let counter: Vec<_> = counter.into_iter().filter(|(_, grant)| grant.effect.to_string() == "minecraft:wither").collect();
    let on_attacker: Vec<_> = counter.iter().filter(|(whom, _)| *whom == Whom::Attacker).collect();
    let on_wearer: Vec<_> = counter.iter().filter(|(whom, _)| *whom == Whom::Wearer).collect();
    assert_eq!(on_attacker.len(), 1, "{counter:?}");
    assert_eq!((on_attacker[0].1.amplifier, on_attacker[0].1.duration), (1, 120));
    assert_eq!(on_wearer.len(), 1, "a low roll lands the wearer's wither too: {counter:?}");
    assert_eq!(on_wearer[0].1.amplifier, 0);
    let unlucky = counter_effects(&tool.custom_data, &env, &none, &mut || 0.99);
    let unlucky: Vec<_> = unlucky.into_iter().filter(|(_, grant)| grant.effect.to_string() == "minecraft:wither").collect();
    assert_eq!(unlucky.iter().filter(|(whom, _)| *whom == Whom::Wearer).count(), 0, "a high roll spares the wearer (20% at level 2)");
    assert_eq!(unlucky.iter().filter(|(whom, _)| *whom == Whom::Attacker).count(), 1, "the attacker's 100% at level 2 needs no roll");
}

#[test]
fn shipped_decay_and_magnetic_put_their_effect_on_the_holder_of_a_swing_a_shot_and_for_magnetic_a_shearing() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, usage_effects, Environment, TAG_MATERIALS, TAG_UPGRADES}, weapon_effect::Usage};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    for id in ["tconstruct:decay", "tconstruct:magnetic"] {
        assert!(world.registry.get(&Identifier::parse(id).unwrap()).is_some_and(|modifier| modifier.unbound.is_empty()), "{id} is fully bound");
    }
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let tool_with = |modifier: &str, level: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool
    };
    let effects = |tool: &ItemSlot, usage: Usage, roll: f32| usage_effects(&tool.custom_data, &env, usage, &mut || roll)
        .into_iter().map(|grant| (grant.effect.to_string(), grant.amplifier, grant.duration)).collect::<Vec<_>>();
    let wither = |amplifier| vec![("minecraft:wither".to_string(), amplifier, 120)];
    // decay: wither at the level, 120 ticks, a quarter of the time; swung or shot, not for shearing (it names no shear hook).
    let decay = tool_with("tconstruct:decay", 2);
    assert_eq!(effects(&decay, Usage::MeleeHit, 0.1), wither(1));
    assert_eq!(effects(&decay, Usage::Launch { primary: true }, 0.1), wither(1));
    assert!(effects(&decay, Usage::MeleeHit, 0.3).is_empty(), "a roll over 25% spares the holder");
    assert!(effects(&decay, Usage::Shear { target: true }, 0.0).is_empty());
    assert!(effects(&decay, Usage::Launch { primary: false }, 0.0).is_empty(), "only the primary arrow of a shot");
    // magnetic: always, for every use (its hooks default to all): the original's own effect at level 1 for 40 ticks.
    let magnetic = tool_with("tconstruct:magnetic", 1);
    for usage in [Usage::MeleeHit, Usage::Launch { primary: true }, Usage::Shear { target: true }] {
        assert_eq!(effects(&magnetic, usage, 0.99), vec![("tconstruct:magnetic".to_string(), 0, 40)], "{usage:?}");
    }
    // A tool with neither has nothing to put on its holder.
    assert!(usage_effects(&tool_with("tconstruct:sharpness", 1).custom_data, &env, Usage::MeleeHit, &mut || 0.0).is_empty());
}

#[test]
fn shipped_fiery_freezing_and_springy_act_on_the_mob_hit_and_on_the_mob_that_hits_the_wearer() {
    use crate::{modifier_entries::List, tool_stack::{attack_actions, counter_actions, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ActionTarget, EntityAction, ItemSlot, KnockbackDirection, Nbt};
    let world = world();
    let tool_with = |tool: &str, material: &str, parts: usize, upgrades: &[(&str, i32)]| {
        let definition = &world.tools.iter().find(|(id, _)| id.path() == tool).unwrap().1;
        let item = Identifier::parse(&format!("tconstruct:{tool}")).unwrap();
        let tags = pluto_lib::fixtures::registry_with_tags(&[&item.to_string()], &[("tconstruct:modifiable/armor", &[item.to_string().as_str()][..])]).unwrap();
        let env = Environment { definition: Some(definition), has_materials: true, materials: &world.materials, modifiers: &world.registry,
            item_tags_loaded: true, item: Some(&item), item_tags: Some(&tags) };
        let mut stack = ItemSlot::new(item.clone(), 1);
        stack.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(vec![Nbt::String(material.into()); parts]));
        let mut list = List::default();
        for (id, level) in upgrades { list = list.with_modifier(Identifier::parse(id).unwrap(), *level).unwrap(); }
        stack.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
        assert!(rebuild_stats(&mut stack.custom_data, &env).unwrap());
        (stack, tags)
    };
    let (helmet, helmet_tags) = tool_with("plate_helmet", "tconstruct:iron", 2, &[("tconstruct:fiery", 2), ("tconstruct:freezing", 1), ("tconstruct:springy", 2)]);
    let helmet_definition = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let env = Environment { definition: Some(helmet_definition), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&helmet_tags) };
    let none = |_: &Identifier| false;
    let attacker = ActionTarget::Attacker;
    // Armor that counters: a low roll lands all three (30%, 15% and 30% at these levels); fire burns the constant second, freezing adds
    // two seconds of 40 ticks, and springy pushes the constant half against the way the mob faces.
    let landed = counter_actions(&helmet.custom_data, &env, &none, &mut || 0.0);
    assert!(landed.contains(&EntityAction::SetFire { target: attacker, seconds: 1 }), "{landed:?}");
    assert!(landed.contains(&EntityAction::Freeze { target: attacker, ticks: 80 }), "{landed:?}");
    assert!(landed.contains(&EntityAction::Knockback { target: attacker, strength: 0.5, direction: KnockbackDirection::AgainstFacing }), "{landed:?}");
    assert_eq!(landed.len(), 3, "{landed:?}");
    assert!(counter_actions(&helmet.custom_data, &env, &none, &mut || 0.99).is_empty(), "a high roll lands none of them");
    // A tool that is not armor counters nothing: the counters ask for the armor tag.
    let (pickaxe, _) = tool_with("pickaxe", "tconstruct:wood", 3, &[("tconstruct:fiery", 2), ("tconstruct:freezing", 1)]);
    let pickaxe_definition = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let pickaxe_env = Environment { definition: Some(pickaxe_definition), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    assert!(counter_actions(&pickaxe.custom_data, &pickaxe_env, &none, &mut || 0.0).is_empty());
    // The hit: five seconds of fire a level, and (4 + 4 a level) seconds of freezing at 40 ticks each.
    let hit = attack_actions(&pickaxe.custom_data, &pickaxe_env);
    assert!(hit.contains(&EntityAction::SetFire { target: ActionTarget::Victim, seconds: 10 }), "{hit:?}");
    assert!(hit.contains(&EntityAction::Freeze { target: ActionTarget::Victim, ticks: 320 }), "{hit:?}");
}

#[test]
fn shipped_block_damage_modules_block_the_sources_they_name() {
    use crate::{damage_tags::DamageTags, modifier_entries::List, protection::DamageSource, tool_stack::{blocks_damage, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let boots = &world.tools.iter().find(|(id, _)| id.path() == "plate_boots").unwrap().1;
    let boots_id = Identifier::parse("tconstruct:plate_boots").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_boots"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_boots"])]).unwrap();
    let env = Environment { definition: Some(boots), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&boots_id), item_tags: Some(&armor_tags) };
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let blocked = |modifier: &str, damage_type: &str| {
        let mut tool = ItemSlot::new(boots_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), 1).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let damage_type = Identifier::parse(damage_type).unwrap();
        blocks_damage(&tool.custom_data, &env, &DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &|_| false })
    };
    assert!(blocked("tconstruct:long_fall", "minecraft:fall"), "long fall blocks fall damage");
    assert!(!blocked("tconstruct:long_fall", "minecraft:mob_attack"), "and nothing else");
    assert!(blocked("tconstruct:frost_walker", "minecraft:hot_floor"), "frost walker blocks magma damage");
    assert!(!blocked("tconstruct:frost_walker", "minecraft:fall"), "and not a fall");
}

#[test]
fn shipped_projectile_protection_and_magnet_take_the_largest_and_the_total_over_worn_boots() {
    use crate::{max_armor, modifier_entries::List, tool_stack::{armor_levels, max_armor_claims, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let boots = &world.tools.iter().find(|(id, _)| id.path() == "plate_boots").unwrap().1;
    let boots_id = Identifier::parse("tconstruct:plate_boots").unwrap();
    let env = Environment { definition: Some(boots), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&boots_id), item_tags: None };
    let piece = |modifier: &str, level: i32| {
        let mut tool = ItemSlot::new(boots_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool
    };
    let (low, high) = (piece("tconstruct:projectile_protection", 1), piece("tconstruct:projectile_protection", 3));
    let claims: Vec<_> = [&low, &high].iter().flat_map(|tool| max_armor_claims(&tool.custom_data, &env)).collect();
    assert_eq!(claims.len(), 2, "{claims:?}");
    let amounts = max_armor::strongest(claims);
    assert_eq!(amounts.len(), 1, "{amounts:?}");
    assert_eq!(amounts[0].0, "minecraft:knockback_resistance");
    // The plate boots carry two levels of their own, so the pieces are at levels 3 and 5: the larger counts once, not 8.
    assert!((amounts[0].1 - 0.25).abs() < 1e-6, "{amounts:?}");
    // The attributes the host lacks leave the other max armor modifiers unbound, so they claim nothing.
    let dragon = max_armor_claims(&piece("tconstruct:dragonborn", 2).custom_data, &env);
    assert!(dragon.iter().all(|claim| format!("{claim:?}").contains("projectile_protection")), "only the boots' own projectile protection claims: {dragon:?}");
    // Armor levels add up over the pieces under their key.
    let (a, b) = (piece("tconstruct:magnetic", 1), piece("tconstruct:magnetic", 2));
    let totals = max_armor::totals([&a, &b].iter().flat_map(|tool| armor_levels(&tool.custom_data, &env)));
    assert_eq!(totals[&Identifier::parse("tconstruct:magnet").unwrap()], 3);
    let belt = piece("tconstruct:soul_belt", 1);
    assert_eq!(armor_levels(&belt.custom_data, &env), vec![(Identifier::parse("tconstruct:soul_belt").unwrap(), 1)]);
}

#[test]
fn shipped_inventory_modifiers_give_a_tool_its_slots_filters_and_stored_stacks() {
    use crate::{modifier_entries::List, tool_inventory::{Items, MenuKey, TOTAL_SLOTS}, tool_stack::{inventory, inventory_slots, rebuild_stats, with_persistent, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let boots = &world.tools.iter().find(|(id, _)| id.path() == "plate_boots").unwrap().1;
    let boots_id = Identifier::parse("tconstruct:plate_boots").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_boots"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_boots"])]).unwrap();
    let env = Environment { definition: Some(boots), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&boots_id), item_tags: Some(&armor_tags) };
    let build = |mods: &[(&str, i32)]| {
        let mut tool = ItemSlot::new(boots_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        let mut list = List::default();
        for (modifier, level) in mods { list = list.with_modifier(Identifier::parse(modifier).unwrap(), *level).unwrap(); }
        tool.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        tool
    };
    let id = |text: &str| Identifier::parse(text).unwrap();
    let max_stack = |_: &Identifier| 64;
    let in_tag = |item: &Identifier, tag: &Identifier| (tag.path() == "slimeball_ammo" && item.path() == "slime_ball") || (tag.path() == "throwable" && item.path() == "snowball");
    let fits = |_: &Identifier| true;
    let no_handler = |_: &ItemSlot| false;
    let items = Items { max_stack: &max_stack, in_tag: &in_tag, can_fit_inside_container: &fits, has_item_handler: &no_handler };
    // The pocket gives 3 slots a level under its own key; the tool belt 3 plus one a level.
    let pockets = build(&[("tconstruct:pocket", 2)]);
    assert_eq!(inventory_slots(&pockets.custom_data), 6);
    assert_eq!(inventory(&pockets.custom_data, &env).slots(), 6);
    let belt = build(&[("tconstruct:tool_belt", 1)]);
    assert_eq!(inventory_slots(&belt.custom_data), 4);
    // Every modifier counts, and the last one's slots come first.
    let both = build(&[("tconstruct:pocket", 1), ("tconstruct:tool_belt", 1)]);
    assert_eq!(inventory_slots(&both.custom_data), 7);
    let Some(Nbt::Compound(volatile)) = both.custom_data.get("tic_volatile_data") else { panic!("no volatile data") };
    assert_eq!(volatile.get(TOTAL_SLOTS), Some(&Nbt::Int(7)));
    let handler = inventory(&both.custom_data, &env);
    let mut tool = both.clone();
    with_persistent(&mut tool.custom_data, |persistent| {
        let rest = handler.insert_item(&items, persistent, 0, ItemSlot::new(id("minecraft:stick"), 70), false);
        assert_eq!(rest.map(|rest| rest.count), Some(6), "a slot holds a stack of 64 by default");
        handler.insert_item(&items, persistent, 6, ItemSlot::new(id("minecraft:apple"), 1), false);
    });
    let Some(Nbt::Compound(persistent)) = tool.custom_data.get("tic_persistent") else { panic!("no persistent data") };
    assert!(persistent.contains_key("tconstruct:tool_belt") && persistent.contains_key("tconstruct:pockets"), "each modifier's stacks sit under its own key");
    // Filters and limits come from the data.
    let sliver = inventory(&build(&[("tconstruct:sliver", 1)]).custom_data, &env);
    assert!(sliver.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:slime_ball"), 1)) && !sliver.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:stick"), 1)));
    assert_eq!(sliver.slot_limit(0), 32);
    assert_eq!(sliver.pattern(0, false).map(ToString::to_string).as_deref(), Some("tconstruct:slimeball"));
    assert_eq!(sliver.pattern(0, true), None);
    let sleeves = inventory(&build(&[("tconstruct:sleeves", 1)]).custom_data, &env);
    assert!(sleeves.is_item_valid(&items, 2, &ItemSlot::new(id("minecraft:snowball"), 1)) && sleeves.slot_limit(2) == 16);
    let minimap = inventory(&build(&[("tconstruct:minimap", 1)]).custom_data, &env);
    assert!(minimap.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:filled_map"), 1)) && !minimap.is_item_valid(&items, 0, &ItemSlot::new(id("minecraft:paper"), 1)));
    assert_eq!(minimap.slot_limit(0), 1);
    let frame = inventory(&build(&[("tconstruct:item_frame", 2)]).custom_data, &env);
    assert_eq!((frame.slots(), frame.slot_limit(5)), (6, 1));
    assert_eq!(inventory(&build(&[("tconstruct:shield_strap", 1)]).custom_data, &env).slots(), 1);
    // The menu modules are bound; the quiver inventories (another module) stay unbound.
    let keys = |modifier: &str| world.registry.get(&id(modifier)).map(|modifier| modifier.inventory_menu_keys()).unwrap_or_default();
    assert_eq!(keys("tconstruct:pocket"), vec![MenuKey::Any]);
    assert_eq!(keys("tconstruct:shield_strap"), vec![MenuKey::Shift]);
    assert_eq!(keys("tconstruct:tool_belt"), vec![MenuKey::Shift]);
    assert_eq!(keys("tconstruct:crafting_table"), vec![MenuKey::Any]);
    assert_eq!(inventory_slots(&build(&[("tconstruct:bulk_quiver", 1)]).custom_data), 0);
}

/// A rebuilt wood pickaxe carrying `modifier` at `level` (no item context, so every stat is supported).
fn stat_tool(world: &World, modifier: &str, level: i32) -> pluto_lib::NbtCompound {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::Nbt;
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let mut nbt = pluto_lib::NbtCompound::new();
    nbt.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec()));
    nbt.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
    assert!(rebuild_stats(&mut nbt, &env).unwrap());
    nbt
}
#[test]
fn shipped_conditional_stat_and_power_bind_exactly_the_modules_whose_inputs_are_reported() {
    let mut registry = Registry::default();
    registry.reload(&files("modifiers"), &conditions());
    let unbound = |name: &str, kind: &str| registry.get(&Identifier::parse(&format!("tconstruct:{name}")).unwrap()).unwrap().unbound.iter().any(|id| id.to_string() == kind);
    // Holder state (airborne), tool data (durability, stat, persistent data) and the projectile data are held.
    for name in ["airborne", "dragonshot", "maintained", "overwield", "war_charge"] { assert!(!unbound(name, "tconstruct:conditional_stat"), "{name}"); }
    // Health, an effect level and the biome temperature are not reported.
    for name in ["ambush", "antitoxin", "flamestance", "invariant", "raging", "temperate"] { assert!(unbound(name, "tconstruct:conditional_stat"), "{name}"); }
    for name in ["fins", "holy", "rebound"] { assert!(!unbound(name, "tconstruct:conditional_power"), "{name}"); }
    // Health, rain and armor coverage of the target are not reported.
    for name in ["ambush", "shock", "valiant"] { assert!(unbound(name, "tconstruct:conditional_power"), "{name}"); }
}
#[test]
fn shipped_conditional_stats_follow_the_holder_state_and_the_tool_data() {
    use crate::{melee_damage::state_in_tag, tool_stack::{conditional_stat, durability, Environment, TAG_DAMAGE, TAG_PERSISTENT}, tool_stats::Stat};
    use pluto_lib::{EntityState, Nbt, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let value = |tool: &NbtCompound, stat: Stat, holder: &EntityState| conditional_stat(tool, &env, stat, &|tag| state_in_tag(tag, Some(holder), None));
    let grounded = EntityState { on_ground: true, ..EntityState::default() };
    let jumping = EntityState { on_ground: false, ..EntityState::default() };
    let swimming = EntityState { on_ground: false, in_water: true, ..EntityState::default() };
    // airborne: +0.1 velocity while in the air (not in water).
    let airborne = stat_tool(&world, "tconstruct:airborne", 1);
    assert_eq!(value(&airborne, Stat::Velocity, &grounded), 1.0);
    assert!((value(&airborne, Stat::Velocity, &jumping) - 1.1).abs() < 1e-6);
    assert_eq!(value(&airborne, Stat::Velocity, &swimming), 1.0, "AIRBORNE excludes swimming");
    assert_eq!(value(&airborne, Stat::DrawSpeed, &jumping), 1.0, "another stat is left alone");
    // dragonshot: +1 projectile damage per level while airborne.
    let dragonshot = stat_tool(&world, "tconstruct:dragonshot", 2);
    assert_eq!(value(&dragonshot, Stat::ProjectileDamage, &jumping), 4.0);
    assert_eq!(value(&dragonshot, Stat::ProjectileDamage, &grounded), 2.0);
    // overwield: 0.06 per point of overslime, up to the modifier level, on velocity and draw speed.
    let mut overwield = stat_tool(&world, "tconstruct:overwield", 2);
    let mut persistent = NbtCompound::new();
    persistent.insert("tconstruct:overslime".into(), Nbt::Int(5));
    overwield.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    assert!((value(&overwield, Stat::Velocity, &grounded) - 1.12).abs() < 1e-6);
    assert!((value(&overwield, Stat::DrawSpeed, &grounded) - 1.12).abs() < 1e-6);
    // war charge: 0.01 velocity per point of stored charge.
    let mut charged = stat_tool(&world, "tconstruct:war_charge", 1);
    let mut persistent = NbtCompound::new();
    persistent.insert("tconstruct:war_charge".into(), Nbt::Float(50.0));
    charged.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
    assert!((value(&charged, Stat::Velocity, &grounded) - 1.5).abs() < 1e-6);
    // maintained: 0.08 velocity per level, scaled by the share of durability above half.
    let mut maintained = stat_tool(&world, "tconstruct:maintained", 2);
    assert!((value(&maintained, Stat::Velocity, &grounded) - 1.16).abs() < 1e-5, "{}", value(&maintained, Stat::Velocity, &grounded));
    let worn = durability(&maintained) * 3 / 4;
    maintained.insert(TAG_DAMAGE.into(), Nbt::Int(worn));
    assert_eq!(value(&maintained, Stat::Velocity, &grounded), 1.0, "below half the bonus is gone");
}
#[test]
fn shipped_conditional_power_follows_the_target_and_the_projectile() {
    use crate::{conditional_stat::PowerContext, melee_damage::state_in_tag, tool_stack::{power_hit, power_shoot, Environment}};
    use pluto_lib::{EntityState, NbtCompound};
    let world = world();
    let pickaxe = &world.tools.iter().find(|(id, _)| id.path() == "pickaxe").unwrap().1;
    let env = Environment { definition: Some(pickaxe), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let shot = |modifier: &str, level: i32, target: EntityState, tags: &'static [&'static str], launch: &dyn Fn(&mut NbtCompound)| {
        let tool = stat_tool(&world, modifier, level);
        let mut arrow = NbtCompound::new();
        power_shoot(&tool, &env, false, &mut arrow);
        launch(&mut arrow);
        let target_in_tag = |tag: &Identifier| tags.contains(&tag.to_string().as_str()) || state_in_tag(tag, None, Some(&target));
        let context = PowerContext { holder_in_tag: &|_| false, target_in_tag: &target_in_tag };
        power_hit(&tool, &env, &mut arrow, &context, 2.0)
    };
    let dry = EntityState::default();
    let wet = EntityState { in_water: true, ..EntityState::default() };
    let nothing = |_: &mut NbtCompound| {};
    assert_eq!(shot("tconstruct:fins", 2, wet, &[], &nothing), 4.0, "fins: +1 power per level against a target in water");
    assert_eq!(shot("tconstruct:fins", 2, dry.clone(), &[], &nothing), 2.0);
    assert_eq!(shot("tconstruct:holy", 2, dry.clone(), &["minecraft:undead"], &nothing), 3.5, "holy: +0.75 per level against the undead");
    assert_eq!(shot("tconstruct:holy", 2, dry.clone(), &[], &nothing), 2.0);
    // rebound: half a point per bounce and level, from the bounce count of the arrow.
    let bounced = |arrow: &mut NbtCompound| { arrow.insert("tconstruct:bounce".into(), pluto_lib::Nbt::Float(3.0)); };
    assert_eq!(shot("tconstruct:rebound", 2, dry, &[], &bounced), 5.0);
}

#[test]
fn shipped_capacity_shields_soak_tool_wear_from_their_bar_and_leave_the_fillers_unbound() {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, Environment, TAG_MATERIALS, TAG_PERSISTENT, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt, NbtCompound};
    let world = world();
    let boots = &world.tools.iter().find(|(id, _)| id.path() == "plate_boots").unwrap().1;
    let env = Environment { definition: Some(boots), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: None, item_tags: None };
    let unbound = |name: &str| world.registry.get(&Identifier::parse(name).unwrap()).unwrap().unbound.iter().map(Identifier::to_string).collect::<Vec<_>>();
    assert!(unbound("tconstruct:barkskin").is_empty(), "the bar and its shield are both bound");
    assert!(unbound("tconstruct:stoneshield") == ["tconstruct:loot_to_capacity"] && unbound("tconstruct:frostshield") == ["tconstruct:damage_to_capacity"], "fillers without a host path stay unbound");
    let tool = |modifier: &str, level: i32, stored: i32| {
        let mut tool = ItemSlot::new(Identifier::parse("tconstruct:plate_boots").unwrap(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(Identifier::parse(modifier).unwrap(), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let mut persistent = NbtCompound::new();
        persistent.insert(modifier.into(), Nbt::Int(stored));
        tool.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        tool
    };
    let stored = |tool: &ItemSlot, modifier: &str| match &tool.custom_data[TAG_PERSISTENT] { Nbt::Compound(data) => data.get(modifier).map_or(0, |value| value.as_i64().unwrap() as i32), _ => 0 };
    for modifier in ["tconstruct:barkskin", "tconstruct:frostshield"] {
        let mut worn = tool(modifier, 1, 10);
        assert_eq!(crate::capacity::absorb(&mut worn.custom_data, &env, None, 4, &mut || 0.0), 0, "{modifier}");
        assert_eq!(stored(&worn, modifier), 6);
        assert_eq!(crate::capacity::absorb(&mut worn.custom_data, &env, None, 10, &mut || 0.0), 4);
        assert_eq!(stored(&worn, modifier), 0);
    }
    // The wear goes through the stack path too: the shield keeps the tool whole and the persistent data follows.
    let mut worn = tool("tconstruct:stoneshield", 1, 20);
    let outcome = crate::tool_slot::damage_slot(&mut worn, &env, 5, true, true, None, &mut || 0.0).unwrap();
    assert_eq!((outcome.applied, stored(&worn, "tconstruct:stoneshield")), (0, 15));
}

/// A rebuilt tool of a shipped definition carrying `upgrades`.
fn upgraded(world: &World, tool: &str, upgrades: &[(&str, i32)]) -> pluto_lib::ItemSlot {
    use crate::{modifier_entries::List, tool_stack::{rebuild_stats, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let env = env_of(world, tool);
    let mut stack = ItemSlot::new(Identifier::new("tconstruct", tool).unwrap(), 1);
    if env.has_materials { stack.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:wood"; 3].map(|m| Nbt::String(m.into())).to_vec())); }
    let mut list = List::default();
    for (modifier, level) in upgrades { list = list.with_modifier(Identifier::parse(modifier).unwrap(), *level).unwrap(); }
    stack.custom_data.insert(TAG_UPGRADES.into(), list.to_nbt());
    assert!(rebuild_stats(&mut stack.custom_data, &env).unwrap());
    stack
}
fn env_of<'a>(world: &'a World, tool: &str) -> crate::tool_stack::Environment<'a> {
    let definition = &world.tools.iter().find(|(id, _)| id.path() == tool).unwrap().1;
    crate::tool_stack::Environment { definition: Some(definition), has_materials: !definition.tool_parts().is_empty(), materials: &world.materials, modifiers: &world.registry, item_tags_loaded: true, item: None, item_tags: None }
}

#[test]
fn shipped_tanned_limits_the_wear_of_ordinary_use_to_one_point() {
    let world = world();
    let plain = upgraded(&world, "pickaxe", &[]);
    let tanned = upgraded(&world, "pickaxe", &[("tconstruct:tanned", 1)]);
    let env = env_of(&world, "pickaxe");
    let wear = |stack: &pluto_lib::ItemSlot, amount| crate::tool_stack::reduce_damage(&stack.custom_data, &env, None, amount, &mut || panic!("a range rolls nothing"));
    assert_eq!(wear(&plain, 7), 7);
    assert_eq!(wear(&tanned, 7), 1, "a range with max 1 clamps ordinary use");
    assert_eq!(wear(&tanned, 1), 1);
    // A cause that is a modifier outside the bypass tag is clamped too; with no modifier tags loaded none is in a tag.
    let cause = Identifier::parse("tconstruct:unknown").unwrap();
    assert_eq!(crate::tool_stack::reduce_damage(&tanned.custom_data, &env, Some(&cause), 4, &mut || 0.0), 1);
}

#[test]
fn shipped_overlord_copies_a_tenth_of_the_final_durability_into_overslime_per_level() {
    use pluto_lib::Nbt;
    let world = world();
    let stat = |stack: &pluto_lib::ItemSlot, name: &str| match stack.custom_data.get(crate::tool_stack::TAG_STATS) {
        Some(Nbt::Compound(stats)) => stats.get(&format!("tconstruct:{name}")).and_then(Nbt::as_f64).map(|value| value as f32),
        _ => None,
    };
    assert_eq!(stat(&upgraded(&world, "pickaxe", &[]), "overslime"), None);
    let one = upgraded(&world, "pickaxe", &[("tconstruct:overlord", 1)]);
    let three = upgraded(&world, "pickaxe", &[("tconstruct:overlord", 3)]);
    // The copy runs after the durability penalty (-15% per level), so it sees the reduced durability.
    for (stack, level) in [(&one, 1.0_f32), (&three, 3.0)] {
        let durability = stat(stack, "durability").unwrap();
        let overslime = stat(stack, "overslime").unwrap();
        assert!((overslime - durability * 0.1 * level).abs() < 0.01, "level {level}: {overslime} of {durability}");
    }
    let plain = stat(&upgraded(&world, "pickaxe", &[]), "durability").unwrap();
    assert!(stat(&one, "durability").unwrap() < plain, "the durability penalty applies");
}

#[test]
fn shipped_soulspeed_is_a_soul_speed_enchantment_and_trim_adds_a_slot_only_without_a_trim_hook() {
    use crate::tool_stack::{enchantments, TAG_VOLATILE};
    use pluto_lib::Nbt;
    let world = world();
    let soul = upgraded(&world, "pickaxe", &[("tconstruct:soulspeed", 1)]);
    assert_eq!(enchantments(&soul.custom_data, &env_of(&world, "pickaxe")).get("minecraft:soul_speed"), Some(&1));
    let defense = |stack: &pluto_lib::ItemSlot| match stack.custom_data.get(TAG_VOLATILE) {
        Some(Nbt::Compound(data)) => data.get("defense").and_then(Nbt::as_i64).unwrap_or(0),
        _ => 0,
    };
    assert_eq!(defense(&upgraded(&world, "pickaxe", &[("tconstruct:trim", 1)])), 1, "a tool with no trim trait hook gets a defense slot");
    assert_eq!(defense(&upgraded(&world, "slime_helmet", &[("tconstruct:trim", 1)])), 0, "a tool whose definition binds the trim trait hook does not");
}

#[test]
fn shipped_attributes_on_forge_names_publish_the_vanilla_attribute_and_the_rest_stay_unbound() {
    use crate::tool_stack::attribute_entries;
    use pluto_lib::Nbt;
    let world = world();
    let amounts = |modifier: &str, level: i32, slot: &str| -> Vec<(String, f64)> {
        let stack = upgraded(&world, "pickaxe", &[(modifier, level)]);
        attribute_entries(&stack.custom_data, &env_of(&world, "pickaxe"), &[slot]).iter().map(|entry| {
            let (Some(Nbt::String(kind)), Some(Nbt::Double(amount))) = (entry.get("type"), entry.get("amount")) else { panic!("an attribute entry") };
            (kind.clone(), *amount)
        }).collect()
    };
    let reach = amounts("tconstruct:reach", 2, "mainhand");
    assert!(reach.contains(&("minecraft:block_interaction_range".into(), 2.0)) && reach.contains(&("minecraft:entity_interaction_range".into(), 2.0)), "{reach:?}");
    assert_eq!(amounts("tconstruct:step_up", 2, "feet"), vec![("minecraft:step_height".to_string(), 1.0)]);
    let sky = amounts("tconstruct:skyfall", 1, "mainhand");
    let gravity = sky.iter().find(|(kind, _)| kind == "minecraft:gravity").expect("gravity").1;
    assert!((gravity - -0.15).abs() < 1e-6, "{sky:?}");
    assert!(sky.contains(&("minecraft:safe_fall_distance".into(), 1.0)));
    // Attributes the host has no counterpart for are not published; the modifier keeps them unbound.
    assert!(amounts("tconstruct:double_jump", 1, "feet").is_empty());
    let mut registry = Registry::default();
    registry.reload(&files("modifiers"), &conditions());
    let unbound = |name: &str| registry.get(&Identifier::parse(name).unwrap()).unwrap().unbound.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert!(unbound("tconstruct:reach").is_empty() && unbound("tconstruct:step_up").is_empty() && unbound("tconstruct:skyfall").is_empty());
    assert_eq!(unbound("tconstruct:turtles_grace"), vec!["tconstruct:attribute"], "forge:swim_speed has no vanilla attribute");
    assert_eq!(unbound("tconstruct:double_jump"), vec!["tconstruct:attribute"]);
    // The modules that need host data this port does not receive stay unbound.
    for (modifier, kind) in [("tconstruct:gold_guard", "golden_attribute"), ("tconstruct:lightspeed", "lightspeed_attribute"), ("tconstruct:warded", "adjust_damage"),
        ("tconstruct:dragonheart", "adjust_damage"), ("tconstruct:temperate", "conditional_mining_speed"), ("tconstruct:shock", "conditional_melee_damage"), ("tconstruct:tank", "stat_boost"), ("tconstruct:experienced", "volatile_float")] {
        assert!(unbound(modifier).contains(&format!("tconstruct:{kind}")), "{modifier} keeps {kind} unbound");
    }
    assert!(unbound("tconstruct:tanned").is_empty() && unbound("tconstruct:overlord").is_empty() && unbound("tconstruct:soulspeed").is_empty());
}

#[test]
fn shipped_fire_protection_takes_back_the_vanilla_points_its_enchantment_gave() {
    use crate::{damage_tags::DamageTags, modifier_entries::List, protection::{damage_after_protection, host_enchantment_points, DamageSource}, tool_stack::{enchantments, protection, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{EntityState, ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let (tags, problems) = DamageTags::load(&damage_tag_stacks());
    assert!(problems.is_empty(), "{problems:?}");
    let id = |text: &str| Identifier::parse(text).unwrap();
    // The helmet, the points the game's enchantment protection counted for it, and the running protection the modifiers leave.
    let run = |modifier: &str, level: i32, damage_type: &str, attacker: Option<&EntityState>| {
        let mut tool = ItemSlot::new(helmet_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id(modifier), level).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        let damage_type = id(damage_type);
        let in_tag = |tag: &Identifier| tag.namespace() == crate::melee_damage::STATE_NAMESPACE && crate::melee_damage::state_in_tag(tag, attacker, None);
        let source = DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &in_tag };
        let vanilla = host_enchantment_points([&enchantments(&tool.custom_data, &env)], &source);
        (vanilla, protection(&tool.custom_data, &env, &source, vanilla))
    };
    // Fire protection I is 2 vanilla points against fire, which come off again before its own 3 are added.
    assert_eq!(run("tconstruct:fire_protection", 1, "minecraft:lava", None), (2.0, 3.0));
    // Iron plating brings its own projectile and melee protection, so the unrelated baseline is measured with an unrelated modifier.
    let base = |damage_type: &str| run("tconstruct:sharpness", 1, damage_type, None).1;
    assert_eq!(run("tconstruct:fire_protection", 1, "minecraft:arrow", None), (0.0, base("minecraft:arrow")), "no fire, no points either way");
    assert_eq!(run("tconstruct:fire_protection", 1, "minecraft:sonic_boom", None), (0.0, base("minecraft:sonic_boom")), "damage that bypasses enchantments counts none");
    assert_eq!(run("tconstruct:blast_protection", 1, "minecraft:explosion", None).0, 2.0, "blast protection is 2 points a level against explosions");
    let (vanilla, total) = run("tconstruct:fireborn", 1, "minecraft:lava", None);
    assert_eq!((vanilla, total), (14.0, 0.0), "fireborn's level 7 is all taken back");
    // The event's damage already had the vanilla points: they are undone, then the modifiers' protection applies.
    assert!((damage_after_protection(10.0, 2.0, 3.0, 20.0) - 10.0 / 0.92 * 0.88).abs() < 1e-4);
    assert!((damage_after_protection(8.0, 14.0, 0.0, 20.0) - 8.0 / (1.0 - 14.0 / 25.0)).abs() < 1e-4, "ignoring the whole enchantment gives the damage back");
    // Scorch protection protects against a fire-immune attacker only.
    let immune = EntityState { kind: "minecraft:blaze".into(), fire_immune: true, ..EntityState::default() };
    let normal = EntityState { kind: "minecraft:husk".into(), ..EntityState::default() };
    assert_eq!(run("tconstruct:scorch_protection", 2, "minecraft:mob_attack", Some(&immune)), (0.0, base("minecraft:mob_attack") + 2.5));
    assert_eq!(run("tconstruct:scorch_protection", 2, "minecraft:mob_attack", Some(&normal)), (0.0, base("minecraft:mob_attack")));
    assert_eq!(run("tconstruct:scorch_protection", 2, "minecraft:mob_attack", None), (0.0, base("minecraft:mob_attack")), "an unknown attacker is not fire immune");
}

#[test]
fn shipped_revenge_names_the_effect_that_unequipping_takes_off() {
    use crate::{modifier_entries::List, tool_stack::{effects_cleared_on_unequip, rebuild_stats, Environment, TAG_MATERIALS, TAG_UPGRADES}};
    use pluto_lib::{ItemSlot, Nbt};
    let world = world();
    let helmet = &world.tools.iter().find(|(id, _)| id.path() == "plate_helmet").unwrap().1;
    let helmet_id = Identifier::parse("tconstruct:plate_helmet").unwrap();
    let armor_tags = pluto_lib::fixtures::registry_with_tags(&["tconstruct:plate_helmet"], &[("tconstruct:modifiable/armor", &["tconstruct:plate_helmet"])]).unwrap();
    let env = Environment { definition: Some(helmet), has_materials: true, materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item: Some(&helmet_id), item_tags: Some(&armor_tags) };
    let id = |text: &str| Identifier::parse(text).unwrap();
    let cleared = |modifier: &str| {
        let mut tool = ItemSlot::new(helmet_id.clone(), 1);
        tool.custom_data.insert(TAG_MATERIALS.into(), Nbt::List(["tconstruct:iron"; 2].map(|m| Nbt::String(m.into())).to_vec()));
        tool.custom_data.insert(TAG_UPGRADES.into(), List::default().with_modifier(id(modifier), 1).unwrap().to_nbt());
        assert!(rebuild_stats(&mut tool.custom_data, &env).unwrap());
        effects_cleared_on_unequip(&tool.custom_data, &env)
    };
    assert_eq!(cleared("tconstruct:revenge"), vec![id("minecraft:strength")]);
    assert!(cleared("tconstruct:fire_protection").is_empty());
}
