//! T1: the twelve armor pieces and the slime wings build as tools.
//!
//! In the original the three armor families are acquired three different ways, and the port keeps those paths:
//! - plate armor is built at the tinker station (`tool_building` recipes, slot layout `tconstruct:plate_armor`);
//! - the slimesuit pieces and the slime wings are cast (`basin_tool_casting`: the cast part or item takes one
//!   material slot, the poured slime material takes the other; the wings' elytra is consumed for the fluid's
//!   material alone) — the basin builds themselves are proven in `casting_loop`;
//! - travelers armor is crafted on a crafting table (`crafting_shaped_materials` recipes: plating and cuirass
//!   materials laid out with string). That recipe type is NOT ported — it also drives the material anvils and
//!   the fake ingot's block recipe, and the fake ingot item is not declared — so travelers pieces currently
//!   have no in-game acquisition path. What is proven here is that their definitions build correct stacks
//!   from their materials (the original's default materials, rose gold and leather); the crafting recipe
//!   port is the remaining gap, recorded here rather than faked with a station recipe the original lacks.
//!
//! Two reductions on the wear side, both recorded rather than faked:
//! - the slime helmet is the original's `SlimeskullItem`: worn, it renders the skull it was cast from. The
//!   port has no skull-rendering path (`armor_worn` answers no layers for it), so its item declaration keeps
//!   the plain leather helmet template, which is what makes it equippable on the head at all.
//! - the travelers pieces' `statless_part_repair` module (the cuirass slot repaired with leather) is one of
//!   the definition module types the port records but does not bind; station repair of travelers armor uses
//!   the generic parts repair instead.

use crate::tool_materials::MaterialVariant;
use pluto_lib::Identifier;
use serde_json::Value;
use std::path::PathBuf;

fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

fn variant(name: &str) -> MaterialVariant { MaterialVariant::parse(&format!("tconstruct:{name}")).unwrap() }

fn world_with_overslime() -> crate::original_data::World {
    // The slime materials' trait is the overslime code modifier; the production registry registers it, the
    // test world leaves code modifiers to the tests that need them (as in casting_loop).
    let mut world = crate::original_data::world();
    for modifier in crate::overslime::code_modifiers().unwrap() { world.registry.register_static(modifier).unwrap(); }
    world
}

fn definition<'w>(world: &'w crate::original_data::World, name: &str) -> &'w crate::tool_definitions::Data {
    &world.tools.iter().find(|(tool, _)| tool.path() == name).unwrap_or_else(|| panic!("{name} definition")).1
}

/// `ToolBuildHandler.buildToolFromMaterials`: a built stack's data for a definition and its materials, in slot order.
fn build(world: &crate::original_data::World, name: &str, materials: &[&str]) -> pluto_lib::NbtCompound {
    let item = id(&format!("tconstruct:{name}"));
    let definition_name = crate::tool_definitions::definition_id_for_item(&item);
    let variants: Vec<MaterialVariant> = materials.iter().map(|material| variant(material)).collect();
    let env = crate::tool_stack::Environment {
        definition: Some(definition(world, definition_name.path())), has_materials: true, materials: &world.materials,
        modifiers: &world.registry, item_tags_loaded: true, item: Some(&item), item_tags: None,
    };
    let list = crate::tool_materials::MaterialList::from_variants(variants);
    crate::tool_build::create_tool(&env, &list, false).unwrap_or_else(|error| panic!("{name} builds: {error}"))
}

fn station(world: &crate::original_data::World, inputs: &[(&str, &str)]) -> crate::tool_station::Outcome {
    let tools: Vec<(Identifier, &crate::tool_definitions::Data)> =
        world.tools.iter().map(|(tool, data)| (tool.clone(), data)).collect();
    let item_max = |_: &Identifier| 1;
    let ctx = crate::tool_station::Context {
        materials: &world.materials, modifiers: &world.registry,
        item_tags_loaded: true, item_tags: None, tools: &tools, item_max: &item_max,
    };
    let mut slots: Vec<Option<crate::tool_station::Input>> = inputs.iter().map(|(part, material)| {
        Some(crate::tool_station::Input::Part { item: id(&format!("tconstruct:{part}")), material: variant(material) })
    }).collect();
    while slots.len() < 6 { slots.push(None); }
    crate::tool_station::lookup(&ctx, None, &slots).unwrap()
}

#[test]
fn the_thirteen_definitions_carry_the_original_stat_slots_parts_and_scales() {
    let world = crate::original_data::world();
    // (tool, parts in slot order, (stat type, scale) in slot order), from the original's tool definitions.
    let expected: &[(&str, &[&str], &[(&str, f32)])] = &[
        ("plate_helmet", &["helmet_plating", "maille"], &[("plating_helmet", 1.0), ("maille", 1.0)]),
        ("plate_chestplate", &["chestplate_plating", "maille"], &[("plating_chestplate", 1.0), ("maille", 1.0)]),
        ("plate_leggings", &["leggings_plating", "maille"], &[("plating_leggings", 1.0), ("maille", 1.0)]),
        ("plate_boots", &["boots_plating", "maille"], &[("plating_boots", 1.0), ("maille", 1.0)]),
        ("slime_helmet", &[], &[("skull", 1.0), ("slime", 1.1)]),
        ("slime_chestplate", &["ribcage"], &[("ribcage", 1.0), ("slime", 1.6)]),
        ("slime_leggings", &["shell"], &[("shell", 1.0), ("slime", 1.5)]),
        ("slime_boots", &["laces"], &[("laces", 1.0), ("slime", 1.3)]),
        ("travelers_helmet", &[], &[("plating_helmet", 0.75), ("cuirass", 1.0)]),
        ("travelers_chestplate", &[], &[("plating_chestplate", 0.75), ("cuirass", 1.0)]),
        ("travelers_leggings", &[], &[("plating_leggings", 0.75), ("cuirass", 1.0)]),
        ("travelers_boots", &[], &[("plating_boots", 0.75), ("cuirass", 1.0)]),
        ("slime_wings", &[], &[("slime", 1.6)]),
    ];
    for (name, parts, stats) in expected {
        let data = definition(&world, name);
        let actual_parts: Vec<String> = data.tool_parts().iter().map(|part| part.path().to_owned()).collect();
        assert_eq!(actual_parts, parts.iter().map(|part| part.to_string()).collect::<Vec<_>>(), "{name} parts");
        let actual_stats: Vec<(String, f32)> = data.material_stat_types().unwrap().iter()
            .map(|(stat, scale)| (stat.path().to_owned(), *scale)).collect();
        assert_eq!(actual_stats, stats.iter().map(|(stat, scale)| (stat.to_string(), *scale)).collect::<Vec<_>>(), "{name} stat slots");
    }
}

#[test]
fn plate_armor_builds_at_the_tinker_station_with_the_platings_stats() {
    let world = crate::original_data::world();
    // Iron plate armor: the plating stat carries durability, armor and toughness; the maille stat is statless.
    // (durability, armor) per piece from iron's shipped plating stats.
    for (name, plating, durability, armor) in [
        ("plate_helmet", "helmet_plating", 165, 2.0),
        ("plate_chestplate", "chestplate_plating", 240, 5.0),
        ("plate_leggings", "leggings_plating", 225, 4.0),
        ("plate_boots", "boots_plating", 195, 2.0),
    ] {
        let crate::tool_station::Outcome::Crafted(crafted) = station(&world, &[(plating, "iron"), ("maille", "iron")])
            else { panic!("{name} builds at the station") };
        assert_eq!(crafted.result.item, id(&format!("tconstruct:{name}")));
        let nbt = &crafted.result.custom_data;
        assert!(crate::tool_stack::is_initialized(nbt), "{name} is a built tool");
        assert_eq!(crate::tool_stack::durability(nbt), durability, "{name} durability");
        assert_eq!(crate::tool_stack::stat(nbt, crate::tool_stats::Stat::Armor), armor, "{name} armor");
        assert_eq!(crate::tool_stack::stat(nbt, crate::tool_stats::Stat::ArmorToughness), 0.0, "{name} toughness");
        let materials: Vec<String> = crate::tool_stack::materials(nbt, true).as_slice().iter().map(ToString::to_string).collect();
        assert_eq!(materials, ["tconstruct:iron", "tconstruct:iron"], "{name} materials");
        assert_eq!(crafted.result.max_damage.map(|value| value as i32), Some(durability), "{name} max damage");
    }
}

#[test]
fn casting_only_and_crafting_only_tools_have_no_station_building() {
    let world = world_with_overslime();
    // The slimesuit pieces and the plate shield take their second material from a cast fluid in the original
    // and have no building recipe; the station used to invent one and hand back a tool with the fluid's slot
    // unfilled (durability 1). A lone cast part must now match nothing.
    for (part, material) in [("ribcage", "bone"), ("shell", "dragon_scale"), ("laces", "cheese"), ("shield_core", "iron")] {
        assert!(matches!(station(&world, &[(part, material)]), crate::tool_station::Outcome::NoMatch),
            "a lone {part} builds nothing at the station");
    }
    // Travelers armor is crafted, not station-built, in the original: a plating part alone matches nothing.
    assert!(matches!(station(&world, &[("helmet_plating", "rose_gold")]), crate::tool_station::Outcome::NoMatch),
        "travelers armor has no station recipe");
    // Control: an ordinary tool still builds.
    let crate::tool_station::Outcome::Crafted(crafted) =
        station(&world, &[("pick_head", "iron"), ("tool_handle", "wood"), ("tool_binding", "string")])
        else { panic!("the pickaxe still builds") };
    assert_eq!(crafted.result.item, id("tconstruct:pickaxe"));
}

#[test]
fn slimesuit_pieces_and_wings_build_from_their_casting_materials() {
    let world = world_with_overslime();
    // The materials each piece's casting recipe produces, in slot order: the cast part's material first, the
    // poured slime material second (the wings take the fluid's material alone). The ribcage, shell and laces
    // stats carry repair amounts only, and the skull stat contributes armor alone, so durability is the slime
    // stat's, scaled (the wings add their base 222).
    for (name, materials, durability) in [
        ("slimy_chestplate", &["bone", "earthslime"][..], 80),
        ("slime_leggings", &["dragon_scale", "earthslime"][..], 75),
        ("slime_boots", &["cheese", "earthslime"][..], 65),
        ("slime_helmet", &["bone", "enderslime"][..], 110),
        ("slime_wings", &["enderslime"][..], 382),
    ] {
        let nbt = build(&world, name, materials);
        assert!(crate::tool_stack::is_initialized(&nbt), "{name} is a built tool");
        assert_eq!(crate::tool_stack::durability(&nbt), durability, "{name} durability");
        let actual: Vec<String> = crate::tool_stack::materials(&nbt, true).as_slice().iter().map(ToString::to_string).collect();
        assert_eq!(actual, materials.iter().map(|material| format!("tconstruct:{material}")).collect::<Vec<_>>(), "{name} materials");
    }
}

#[test]
fn travelers_armor_builds_from_its_default_materials() {
    let world = crate::original_data::world();
    // The original's default materials (rose gold plating, leather cuirass). Durability is the plating's,
    // scaled by the definition's 0.75 stat scale and its 0.75 durability multiplier, truncated; armor is the
    // plating's scaled by 0.75; the cuirass stat is statless and rose gold gives no toughness.
    for (name, durability, armor) in [
        ("travelers_helmet", 55, 1.5),
        ("travelers_chestplate", 81, 3.75),
        ("travelers_leggings", 75, 2.25),
        ("travelers_boots", 65, 0.75),
    ] {
        let nbt = build(&world, name, &["rose_gold", "leather"]);
        assert!(crate::tool_stack::is_initialized(&nbt), "{name} is a built tool");
        assert_eq!(crate::tool_stack::durability(&nbt), durability, "{name} durability");
        assert_eq!(crate::tool_stack::stat(&nbt, crate::tool_stats::Stat::Armor), armor, "{name} armor");
        assert_eq!(crate::tool_stack::stat(&nbt, crate::tool_stats::Stat::ArmorToughness), 0.0, "{name} toughness");
        let actual: Vec<String> = crate::tool_stack::materials(&nbt, true).as_slice().iter().map(ToString::to_string).collect();
        assert_eq!(actual, ["tconstruct:rose_gold", "tconstruct:leather"], "{name} materials");
    }
}

#[test]
fn the_thirteen_are_listed_in_the_creative_tools_tab_in_the_original_order() {
    let group = crate::creative::tools_group();
    // TinkerTools.addTabItems order within the armor section: travelers, then plate, then the slimesuit,
    // with the wings last in the whole tab; every listed stack carries one material per stat slot.
    let order = ["travelers_helmet", "travelers_chestplate", "travelers_leggings", "travelers_boots",
        "plate_helmet", "plate_chestplate", "plate_leggings", "plate_boots",
        "slime_helmet", "slimy_chestplate", "slime_leggings", "slime_boots", "slime_wings"];
    let slots = |name: &str| if name == "slime_wings" { 1 } else { 2 };
    let mut first_at: Vec<usize> = Vec::new();
    for name in order {
        let entries: Vec<usize> = group.entries.iter().enumerate()
            .filter(|(_, stack)| stack.item.path() == name).map(|(index, _)| index).collect();
        assert!(!entries.is_empty(), "{name} is in the tools tab");
        for index in &entries {
            let snbt = group.entries[*index].to_snbt();
            let list = snbt.split("tic_materials:[").nth(1).and_then(|rest| rest.split(']').next())
                .unwrap_or_else(|| panic!("{name} tab stack carries its materials: {snbt}"));
            assert_eq!(list.matches('"').count() / 2, slots(name), "{name} tab stack material slots");
        }
        first_at.push(entries[0]);
    }
    assert!(first_at.windows(2).all(|pair| pair[0] < pair[1]), "the armor section keeps the original order");
    assert_eq!(group.entries.last().unwrap().item.path(), "slime_wings", "the wings close the tab");
}

fn content_json(relative: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct").join(relative);
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|_| panic!("{relative}"))).unwrap()
}

#[test]
fn armor_items_declare_their_equipment_and_the_armor_tags_name_them() {
    // The port's armor/equipment path: each piece's item declaration puts it in its slot with its family's
    // worn asset (the slime helmet is the documented exception in this file's header).
    for (name, template, slot, asset) in [
        ("plate_helmet", "minecraft:iron_helmet", "head", "tconstruct:plate"),
        ("plate_chestplate", "minecraft:iron_chestplate", "chest", "tconstruct:plate"),
        ("plate_leggings", "minecraft:iron_leggings", "legs", "tconstruct:plate"),
        ("plate_boots", "minecraft:iron_boots", "feet", "tconstruct:plate"),
        ("travelers_helmet", "minecraft:leather_helmet", "head", "tconstruct:travelers"),
        ("travelers_chestplate", "minecraft:leather_chestplate", "chest", "tconstruct:travelers"),
        ("travelers_leggings", "minecraft:leather_leggings", "legs", "tconstruct:travelers"),
        ("travelers_boots", "minecraft:leather_boots", "feet", "tconstruct:travelers"),
        ("slimy_chestplate", "minecraft:leather_chestplate", "chest", "tconstruct:slime"),
        ("slime_leggings", "minecraft:leather_leggings", "legs", "tconstruct:slime"),
        ("slime_boots", "minecraft:leather_boots", "feet", "tconstruct:slime"),
        ("slime_wings", "minecraft:elytra", "chest", "tconstruct:slime_wings"),
    ] {
        let item = content_json(&format!("horizon/item/{name}.json"));
        assert_eq!(item["template"], template, "{name} template");
        assert_eq!(item["max_stack_size"], 1, "{name} is unstackable");
        let equippable = &item["components"]["minecraft:equippable"];
        assert_eq!(equippable["slot"], slot, "{name} equipment slot");
        assert_eq!(equippable["asset_id"], asset, "{name} worn asset");
    }
    let helmet = content_json("horizon/item/slime_helmet.json");
    assert_eq!(helmet["template"], "minecraft:leather_helmet", "the slime helmet keeps the leather helmet template");
    assert_eq!(helmet["max_stack_size"], 1, "the slime helmet is unstackable");
    // The armor attribute path keys off these tags; they must name the port's items (the slime
    // chestplate's item is `slimy_chestplate`; only its tool definition is `slime_chestplate`).
    for (tag, members) in [
        ("helmets", &["travelers_helmet", "plate_helmet", "slime_helmet"][..]),
        ("chestplate", &["travelers_chestplate", "plate_chestplate", "slimy_chestplate", "slime_wings"][..]),
        ("leggings", &["travelers_leggings", "plate_leggings", "slime_leggings"][..]),
        ("boots", &["travelers_boots", "plate_boots", "slime_boots"][..]),
    ] {
        let values = content_json(&format!("tags/item/modifiable/armor/{tag}.json"));
        let named: Vec<String> = values["values"].as_array().unwrap().iter().map(|entry| {
            entry.as_str().map(str::to_owned).unwrap_or_else(|| entry["id"].as_str().unwrap().to_owned())
        }).collect();
        for member in members {
            assert!(named.contains(&format!("tconstruct:{member}")), "{tag} tag names {member}");
        }
    }
}
