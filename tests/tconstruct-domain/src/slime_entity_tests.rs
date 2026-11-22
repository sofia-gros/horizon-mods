//! The slime mobs: the registered types and class queries against the original's values, and the shipped
//! data (entity loot tables, spawn biome modifiers, spawn egg declarations) in the converted shapes.
use crate::slime_entities::TYPES;
use pluto_lib::{AttackDamageContext, AttributeModifierOperation, CalculateFallDamageContext, Identifier, JumpPowerContext, LivingEntityBehaviors};
use serde_json::Value;
use std::path::PathBuf;

fn id(text: &str) -> Identifier {
    Identifier::parse(text).unwrap()
}

fn content(relative: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data").join(relative);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("reading {relative}: {error}")))
        .unwrap_or_else(|error| panic!("parsing {relative}: {error}"))
}

#[test]
fn the_three_registered_slime_types_carry_the_originals_registration_values() {
    // TinkerWorld registers exactly these three, in this order; the slime islands' spawn overrides name
    // exactly these ids (the other two island overrides are the vanilla slime and magma cube).
    let names: Vec<&str> = TYPES.iter().map(|slime| slime.name).collect();
    assert_eq!(names, ["sky_slime", "ender_slime", "terracube"]);
    let tracking: Vec<u32> = TYPES.iter().map(|slime| slime.tracking).collect();
    assert_eq!(tracking, [20, 32, 8]);
    // The spawn egg colours of the registrations.
    assert_eq!(TYPES[0].egg, (0x47eff5, 0xacfff4));
    assert_eq!(TYPES[1].egg, (0x6300b0, 0xd37cff));
    assert_eq!(TYPES[2].egg, (0xafb9d6, 0xa1a7b1));
    // The travelers-plate slimes build their spawn helmet from a fixed plating material; the terracube
    // rolls vanilla armor instead, and places by its own rule rather than a block tag predicate.
    assert_eq!(TYPES[0].plating, Some("steel"));
    assert_eq!(TYPES[1].plating, Some("knightmetal"));
    assert_eq!(TYPES[2].plating, None);
    assert_eq!(TYPES[0].spawn_tag, Some("slime_spawn/sky"));
    assert_eq!(TYPES[1].spawn_tag, Some("slime_spawn/ender"));
    assert_eq!(TYPES[2].spawn_tag, None);
}

#[test]
fn registering_publishes_the_class_queries_and_the_armored_slime_modifiers() {
    crate::slime_entities::register();
    let jump = |slime: &str, size: Option<i32>, factor: f32| {
        LivingEntityBehaviors::jump_power(&JumpPowerContext { entity: 1, entity_type: id(&format!("tconstruct:{slime}")), slime_size: size, block_jump_factor: factor })
    };
    // SkySlimeEntity: sqrt(size) * factor / 2; without a slime size there is no override.
    assert_eq!(jump("sky_slime", Some(4), 1.0), Some(1.0));
    assert_eq!(jump("sky_slime", Some(1), 1.0), Some(0.5));
    assert_eq!(jump("sky_slime", Some(4), 0.5), Some(0.5));
    assert_eq!(jump("sky_slime", None, 1.0), None);
    // TerracubeEntity: 0.5 * factor, attack attribute + 2, fall damage 0. The ender slime overrides none
    // of the three queries, and neither does the sky slime override attack or fall.
    assert_eq!(jump("terracube", Some(1), 0.8), Some(0.4));
    assert_eq!(jump("ender_slime", Some(4), 1.0), None);
    let attack = |slime: &str| LivingEntityBehaviors::attack_damage(&AttackDamageContext { entity: 1, entity_type: id(&format!("tconstruct:{slime}")), attribute: 4.0 });
    assert_eq!(attack("terracube"), Some(6.0));
    assert_eq!(attack("sky_slime"), None);
    assert_eq!(attack("ender_slime"), None);
    let fall = |slime: &str| LivingEntityBehaviors::calculate_fall_damage(&CalculateFallDamageContext { entity: 1, entity_type: id(&format!("tconstruct:{slime}")), distance: 10.0, multiplier: 1.0, inherited: 7.0 });
    assert_eq!(fall("terracube"), Some(0));
    assert_eq!(fall("sky_slime"), None);
    assert_eq!(fall("ender_slime"), None);
    // ArmoredSlimeEntity's constructor modifiers, on all three types: armor, armor toughness and
    // knockback resistance, amount 3 multiply-total under the original's names.
    for slime in ["sky_slime", "ender_slime", "terracube"] {
        let modifiers = LivingEntityBehaviors::transient_attribute_modifiers(&id(&format!("tconstruct:{slime}"))).expect("registered behaviour");
        let rows: Vec<(&str, &str)> = modifiers.iter().map(|modifier| (modifier.attribute.path(), modifier.id.path())).collect();
        assert_eq!(rows, [("armor", "small_armor_bonus"), ("armor_toughness", "small_toughness_bonus"), ("knockback_resistance", "small_resistence_bonus")], "{slime}");
        assert!(modifiers.iter().all(|modifier| modifier.amount == 3.0 && modifier.operation == AttributeModifierOperation::AddMultipliedTotal), "{slime}");
    }
}

#[test]
fn the_shipped_entity_loot_tables_carry_the_originals_drops() {
    for (name, ball, shard) in [("sky_slime", "sky_slime_ball", "steel_shard"), ("ender_slime", "ender_slime_ball", "knightmetal_shard")] {
        let table = content(&format!("tconstruct/loot_table/entities/{name}.json"));
        assert_eq!(table["type"], "minecraft:entity", "{name}");
        assert_eq!(table["random_sequence"], format!("tconstruct:entities/{name}"), "{name}");
        let pools = table["pools"].as_array().unwrap();
        assert_eq!(pools.len(), 2, "{name}: the plain pool and the frog pool");
        // Pool one drops only for a size 1 slime not killed by a frog: the ball, or the shard when metal.
        let plain = &pools[0];
        assert_eq!(plain["condition"]["type"], "minecraft:all_of", "{name}");
        assert_eq!(plain["condition"]["terms"][0]["term"]["predicate"]["source_entity"]["type"], "minecraft:frog", "{name}: not a frog");
        assert_eq!(plain["condition"]["terms"][1]["type"], "minecraft:entity_properties", "{name}");
        assert_eq!(plain["condition"]["terms"][1]["predicate"]["type_specific"]["size"], 1, "{name}");
        let entries = plain["entries"].as_array().unwrap();
        assert_eq!(entries[0]["name"], format!("tconstruct:{ball}"), "{name}");
        assert_eq!(entries[0]["condition"]["term"]["predicate"]["nbt"], "{metal:1b}", "{name}: the ball is the non-metal drop");
        assert_eq!(entries[1]["name"], format!("tconstruct:{shard}"), "{name}");
        assert_eq!(entries[1]["condition"]["predicate"]["nbt"], "{metal:1b}", "{name}: the shard is the metal drop");
        let functions: Vec<&str> = entries[0]["modifier"].as_array().unwrap().iter().map(|function| function["type"].as_str().unwrap()).collect();
        assert_eq!(functions, ["minecraft:set_count", "minecraft:looting_enchant"], "{name}");
        assert_eq!(entries[0]["modifier"][0]["count"], serde_json::json!({"type": "minecraft:uniform", "min": 0, "max": 2}), "{name}");
        // Pool two is the frog pool: exactly one of the same two, chosen by the same metal condition.
        let frog = &pools[1];
        assert_eq!(frog["condition"]["type"], "minecraft:all_of", "{name}");
        assert_eq!(frog["condition"]["terms"][0]["type"], "minecraft:damage_source_properties", "{name}");
        assert_eq!(frog["condition"]["terms"][0]["predicate"]["source_entity"]["type"], "minecraft:frog", "{name}");
        assert_eq!(frog["entries"][0]["modifier"][0]["count"], 1, "{name}");
    }
    let table = content("tconstruct/loot_table/entities/terracube.json");
    assert_eq!(table["type"], "minecraft:entity");
    assert_eq!(table["random_sequence"], "tconstruct:entities/terracube");
    let pools = table["pools"].as_array().unwrap();
    assert_eq!(pools.len(), 1);
    // The whole pool smelts for a burning cube.
    assert_eq!(pools[0]["modifier"][0]["type"], "minecraft:furnace_smelt");
    assert_eq!(pools[0]["modifier"][0]["condition"]["predicate"]["flags"]["is_on_fire"], true);
    let entries = pools[0]["entries"].as_array().unwrap();
    // Clay from a cube of size 2 or more not killed by a frog, at the original's -2..1 count.
    assert_eq!(entries[0]["name"], "minecraft:clay_ball");
    assert_eq!(entries[0]["modifier"][0]["count"], serde_json::json!({"type": "minecraft:uniform", "min": -2, "max": 1}));
    let terms = entries[0]["condition"]["terms"].as_array().unwrap();
    assert_eq!(terms[0]["term"]["predicate"]["source_entity"]["type"], "minecraft:frog");
    assert_eq!(terms[1]["predicate"]["type_specific"]["size"], serde_json::json!({"min": 2}));
    // A frog gets the seared lamp instead.
    assert_eq!(entries[1]["name"], "tconstruct:seared_lamp");
    assert_eq!(entries[1]["condition"]["predicate"]["source_entity"]["type"], "minecraft:frog");
}

#[test]
fn the_shipped_spawn_modifiers_spawn_the_slimes_where_the_original_spawns_them() {
    let modifier = |name: &str| content(&format!("tconstruct/neoforge/biome_modifier/{name}.json"));
    let overworld = modifier("spawn_overworld_slime");
    assert_eq!(overworld["type"], "neoforge:add_spawns");
    assert_eq!(overworld["biomes"], "#minecraft:is_overworld");
    assert_eq!(overworld["spawners"], serde_json::json!([{"type": "tconstruct:sky_slime", "weight": 100, "minCount": 2, "maxCount": 4}]));
    // The terracube's predicate is widened to the overworld tag (the module documentation records why).
    let terracube = modifier("spawn_terracube");
    assert_eq!(terracube["type"], "neoforge:add_spawns");
    assert_eq!(terracube["biomes"], "#minecraft:is_overworld");
    assert_eq!(terracube["spawners"], serde_json::json!([{"type": "tconstruct:terracube", "weight": 10, "minCount": 2, "maxCount": 4}]));
    let end = modifier("spawn_end_slime");
    assert_eq!(end["type"], "neoforge:add_spawns");
    assert_eq!(end["biomes"], "#minecraft:is_end");
    assert_eq!(end["spawners"], serde_json::json!([{"type": "tconstruct:ender_slime", "weight": 10, "minCount": 2, "maxCount": 4}]));
}

#[test]
fn the_spawn_eggs_are_declared_for_the_registered_types_and_listed_in_the_world_tab() {
    for slime in ["sky_slime", "ender_slime", "terracube"] {
        let declaration = content(&format!("tconstruct/horizon/item/{slime}_spawn_egg.json"));
        assert_eq!(declaration["template"], "minecraft:slime_spawn_egg", "{slime}");
        assert_eq!(declaration["components"]["minecraft:entity_data"]["id"], format!("tconstruct:{slime}"), "{slime}");
    }
    let tab: Vec<String> = crate::creative::world_group().entries.iter().map(|stack| stack.item.to_string()).collect();
    for egg in ["tconstruct:sky_slime_spawn_egg", "tconstruct:ender_slime_spawn_egg", "tconstruct:terracube_spawn_egg"] {
        assert!(tab.iter().any(|item| item == egg), "{egg} is listed in the world tab");
    }
}

#[test]
fn the_spawn_placement_block_tags_ship_with_the_predicates_inputs() {
    // The placement predicates themselves are not registered (no surface; see slime_entities), but the
    // block tags they read are shipped data and name the crystal blocks over the slime grass.
    for (name, crystal) in [("earth", "earth_slime_crystal_block"), ("sky", "sky_slime_crystal_block"), ("ender", "ender_slime_crystal_block")] {
        let tag = content(&format!("tconstruct/tags/block/slime_spawn/{name}.json"));
        let values: Vec<&str> = tag["values"].as_array().unwrap().iter().map(|value| value.as_str().unwrap()).collect();
        assert!(values.contains(&format!("tconstruct:{crystal}").as_str()), "{name}");
        assert!(values.contains(&format!("#tconstruct:slimy_grass/{name}").as_str()), "{name}");
    }
}
