//! The slime mobs: the registered types and class queries against the original's values, and the shipped
//! data (entity loot tables, spawn biome modifiers, spawn egg declarations) in the converted shapes.
use crate::slime_entities::{TYPES, SlimeSplit, civil_from_days, finalize_spawn, head_drop_chance, sky_fall, tag_placement, teleport_destination, terracube_placement, terracube_rolls};
use pluto_lib::{AttackDamageContext, AttributeModifierOperation, CalculateFallDamageContext, CauseFallDamageContext, EntityInfo, EquipmentDropChanceContext, FallOutcome, Identifier, JumpPowerContext, LivingEntityBehaviors, MobSpawnCompletion, MobSpawnInfo, MobSpawnPreparation, MobSplitCallback, MobSplitChild, MobSplitContext, MobSplitInheritance, MobSplitRandom, SpawnPlacementContext, StackData};
use serde_json::Value;
use std::collections::VecDeque;
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
    // The terracube's predicate (the overworld minus the forge:no_default_monsters biomes) ships expanded
    // as the terracube_spawns biome tag: every overworld biome but the mushroom fields and the deep dark.
    let terracube = modifier("spawn_terracube");
    assert_eq!(terracube["type"], "neoforge:add_spawns");
    assert_eq!(terracube["biomes"], "#tconstruct:terracube_spawns");
    let spawns = content("tconstruct/tags/worldgen/biome/terracube_spawns.json");
    let values: Vec<&str> = spawns["values"].as_array().unwrap().iter().map(|value| value.as_str().unwrap()).collect();
    assert_eq!(values.len(), 54);
    assert!(values.contains(&"minecraft:beach"), "beaches keep their terracubes");
    assert!(!values.contains(&"minecraft:mushroom_fields") && !values.contains(&"minecraft:deep_dark"));
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
    // The block tags the placement predicates read are shipped data and name the crystal blocks over
    // the slime grass; the derived types' registered predicates carry the tags' current expansion in
    // TYPES, pinned here against the shipped JSONs.
    for (name, crystal) in [("earth", "earth_slime_crystal_block"), ("sky", "sky_slime_crystal_block"), ("ender", "ender_slime_crystal_block")] {
        let tag = content(&format!("tconstruct/tags/block/slime_spawn/{name}.json"));
        let values: Vec<&str> = tag["values"].as_array().unwrap().iter().map(|value| value.as_str().unwrap()).collect();
        assert!(values.contains(&format!("tconstruct:{crystal}").as_str()), "{name}");
        assert!(values.contains(&format!("#tconstruct:slimy_grass/{name}").as_str()), "{name}");
    }
    for slime in TYPES.iter().filter(|slime| slime.spawn_tag.is_some()) {
        let name = slime.name.trim_end_matches("_slime");
        let grass = content(&format!("tconstruct/tags/block/slimy_grass/{name}.json"));
        let mut expected: Vec<String> = grass["values"].as_array().unwrap().iter().map(|value| value.as_str().unwrap().to_owned()).collect();
        expected.push(format!("tconstruct:{name}_slime_crystal_block"));
        expected.push(format!("tconstruct:budding_{name}_slime_crystal"));
        expected.sort();
        let mut actual: Vec<String> = slime.spawn_blocks.iter().map(|block| block.to_string()).collect();
        actual.sort();
        assert_eq!(actual, expected, "{name}: the predicate's expansion is the shipped tag's expansion");
    }
}

// The spawn finalization, split inheritance, placement and landing behaviour, driven through the
// factored functions with scripted randomness: the draw order and thresholds are the original's.

struct ScriptedSpawn {
    info: MobSpawnInfo,
    floats: VecDeque<f32>,
    ints: VecDeque<i32>,
    pickup: Option<bool>,
    gear: std::collections::BTreeMap<String, StackData>,
    chances: Vec<(String, f32)>,
    enchants: Vec<(String, i32)>,
    attached: Vec<(String, String, bool)>,
}

impl ScriptedSpawn {
    fn new(kind: &str, special_multiplier: f32, floats: &[f32], ints: &[i32]) -> Self {
        ScriptedSpawn {
            info: MobSpawnInfo { entity_type: Identifier::parse(kind).unwrap(), dimension: Identifier::parse("minecraft:overworld").unwrap(), position: [0.0, 64.0, 0.0], reason: "natural".into(), special_multiplier },
            floats: floats.iter().copied().collect(),
            ints: ints.iter().copied().collect(),
            pickup: None,
            gear: std::collections::BTreeMap::new(),
            chances: Vec::new(),
            enchants: Vec::new(),
            attached: Vec::new(),
        }
    }
}

impl MobSpawnPreparation for ScriptedSpawn {
    fn info(&self) -> &MobSpawnInfo { &self.info }
    fn next_float(&mut self) -> f32 { self.floats.pop_front().unwrap_or(0.0) }
    fn next_double(&mut self) -> f64 { 0.0 }
    fn next_int(&mut self, bound: i32) -> Result<i32, String> {
        let value = self.ints.pop_front().unwrap_or(0);
        if value < 0 || value >= bound { return Err(format!("scripted int {value} out of bound {bound}")); }
        Ok(value)
    }
    fn equipment(&self, _slot: &str) -> Result<Option<StackData>, String> { Ok(None) }
}

impl MobSpawnCompletion for ScriptedSpawn {
    fn set_equipment(&mut self, slot: &str, stack: StackData) -> Result<(), String> { self.gear.insert(slot.to_string(), stack); Ok(()) }
    fn set_can_pick_up_loot(&mut self, can: bool) -> Result<(), String> { self.pickup = Some(can); Ok(()) }
    fn set_drop_chance(&mut self, slot: &str, chance: f32) -> Result<(), String> { self.chances.push((slot.to_string(), chance)); Ok(()) }
    fn enchant_equipment(&mut self, slot: &str, level: i32) -> Result<(), String> { self.enchants.push((slot.to_string(), level)); Ok(()) }
    fn set_attached(&mut self, attachment: &str, json: &str, synced: bool) -> Result<(), String> { self.attached.push((attachment.to_string(), json.to_string(), synced)); Ok(()) }
}

#[test]
fn finalizing_a_terracube_rolls_its_helmet_and_enchantment() {
    // Pickup roll 0.4 passes at 0.55; the helmet roll 0.1 passes at 0.15; quality starts at
    // nextInt(3) = 1 and two of the three 0.25 rolls land, giving quality 3 — the iron helmet;
    // the enchantment level is 5 + 1.0 * nextInt(18) = 15.
    let mut spawn = ScriptedSpawn::new("tconstruct:terracube", 1.0, &[0.4, 0.1, 0.2, 0.3, 0.1], &[1, 10]);
    let action = finalize_spawn(&mut spawn).expect("finalization").expect("an action");
    action(&mut spawn).expect("applying the action");
    assert_eq!(spawn.pickup, Some(true));
    assert_eq!(spawn.gear["head"].item, "minecraft:iron_helmet");
    assert_eq!(spawn.enchants, [("head".to_string(), 15)]);
    assert!(spawn.attached.is_empty(), "a terracube never turns metal");
    // With no helmet roll, nothing is equipped and nothing is enchanted.
    let mut spawn = ScriptedSpawn::new("tconstruct:terracube", 1.0, &[0.6, 0.2], &[]);
    let action = finalize_spawn(&mut spawn).expect("finalization").expect("an action");
    action(&mut spawn).expect("applying the action");
    assert_eq!(spawn.pickup, Some(false), "0.6 fails the 0.55 pickup roll");
    assert!(spawn.gear.is_empty());
    assert!(spawn.enchants.is_empty());
    // Other kinds are not the callback's business.
    let mut other = ScriptedSpawn::new("minecraft:zombie", 1.0, &[], &[]);
    assert!(finalize_spawn(&mut other).expect("finalization").is_none());
}

#[test]
fn finalizing_a_sky_slime_records_its_metal_flag() {
    // The metal roll (0.15, unscaled by difficulty) lands at 0.1; the helmet roll fails at 0.9, so
    // the flag is the whole result and no tool needs building.
    let mut spawn = ScriptedSpawn::new("tconstruct:sky_slime", 1.0, &[0.4, 0.1, 0.9], &[]);
    let action = finalize_spawn(&mut spawn).expect("finalization").expect("an action");
    action(&mut spawn).expect("applying the action");
    assert_eq!(spawn.pickup, Some(true));
    assert_eq!(spawn.attached, [("tconstruct:metal".to_string(), "true".to_string(), true)]);
}

#[test]
fn terracube_rolls_map_quality_to_the_vanilla_helmets() {
    // Quality 5 (nextInt(3) = 2, all three increments) is the turtle helmet; quality 0 is leather.
    let mut view = ScriptedSpawn::new("tconstruct:terracube", 1.0, &[0.0, 0.0, 0.0, 0.0], &[2, 0]);
    let (stack, level) = terracube_rolls(&mut view, 1.0).expect("a helmet roll");
    assert_eq!(stack.item, "minecraft:turtle_helmet");
    assert_eq!(level, 5);
    let mut view = ScriptedSpawn::new("tconstruct:terracube", 1.0, &[0.0, 0.9, 0.9, 0.9], &[0, 17]);
    let (stack, level) = terracube_rolls(&mut view, 1.0).expect("a helmet roll");
    assert_eq!(stack.item, "minecraft:leather_helmet");
    assert_eq!(level, 22, "5 + 17 at multiplier 1");
}

#[test]
fn the_sky_slimes_bounce_and_the_head_cap_follow_the_original() {
    let fall = |distance: f64, velocity: [f64; 3]| CauseFallDamageContext {
        entity: 1,
        entity_type: Identifier::parse("tconstruct:sky_slime").unwrap(),
        distance, multiplier: 1.0, velocity, suppressing_bounce: false,
    };
    match sky_fall(&fall(10.0, [1.9, -2.0, 0.95])) {
        Some(FallOutcome::Bounce { velocity, sound }) => {
            for (actual, expected) in velocity.iter().zip([2.0, 1.8, 1.0]) {
                assert!((actual - expected).abs() < 1e-9, "x and z divide by 0.95, y flips at 0.9: {velocity:?}");
            }
            assert_eq!(sound.map(|sound| sound.to_string()), Some("tconstruct:slimy_bounce".to_string()));
        }
        other => panic!("expected a bounce, got {other:?}"),
    }
    assert!(matches!(sky_fall(&fall(2.0, [0.0, -1.0, 0.0])), Some(FallOutcome::Cancel)), "a short fall is cancelled outright");
    assert!(sky_fall(&CauseFallDamageContext { suppressing_bounce: true, ..fall(10.0, [0.0, -2.0, 0.0]) }).is_none(), "the suppressing branch is declined for mobs");
    // The head drop cap: above 0.25 with size over 1 caps to 0.25; at size 1 the chance stands.
    let chance = |size: Option<i32>, chance: f32| EquipmentDropChanceContext {
        entity_type: Identifier::parse("tconstruct:sky_slime").unwrap(), slot: "head".into(), slime_size: size, chance, looting: 0,
    };
    assert_eq!(head_drop_chance(&chance(Some(4), 2.0)), Some(0.25));
    assert_eq!(head_drop_chance(&chance(Some(1), 2.0)), None);
    assert_eq!(head_drop_chance(&chance(Some(4), 0.1)), None);
    // The cap is on the recorded chance alone: the killer's Looting neither
    // raises nor lowers the class chance the callback returns. The game
    // stores that chance unadjusted and the host's equipment_drops effect
    // adds looting*0.01 exactly once, so a Looting III kill rolls against
    // 0.25 + 3*0.01 = 0.28 (issue #1163; folding looting into the stored
    // chance as well rolled at 0.31).
    let looting_chance = |size: Option<i32>, chance: f32, looting: i32| EquipmentDropChanceContext {
        entity_type: Identifier::parse("tconstruct:sky_slime").unwrap(), slot: "head".into(), slime_size: size, chance, looting,
    };
    let capped = head_drop_chance(&looting_chance(Some(4), 2.0, 3)).expect("the cap applies under Looting III");
    assert_eq!(capped, 0.25);
    assert!((capped + 3.0 * 0.01 - 0.28).abs() < 1e-6, "canon effective chance is 0.28, got {}", capped + 3.0 * 0.01);
}

struct ScriptedRandom { floats: VecDeque<f32>, ints: VecDeque<i32> }
impl MobSplitRandom for ScriptedRandom {
    fn next_float(&mut self) -> f32 { self.floats.pop_front().unwrap_or(0.0) }
    fn next_int(&mut self, bound: i32) -> Result<i32, String> {
        let value = self.ints.pop_front().unwrap_or(0);
        if value < 0 || value >= bound { return Err(format!("scripted int {value} out of bound {bound}")); }
        Ok(value)
    }
}

#[derive(Default)]
struct FakeChildState {
    gear: std::collections::BTreeMap<String, StackData>,
    name: Option<Option<String>>,
    no_ai: Option<bool>,
    invulnerable: Option<bool>,
}
struct FakeChild { id: u32, state: std::rc::Rc<std::cell::RefCell<FakeChildState>> }
impl MobSplitChild for FakeChild {
    fn info(&self) -> EntityInfo { EntityInfo { id: self.id, kind: "tconstruct:sky_slime".into(), dimension: "minecraft:overworld".into() } }
    fn set_equipment(&mut self, slot: &str, stack: StackData) -> Result<(), String> { self.state.borrow_mut().gear.insert(slot.to_string(), stack); Ok(()) }
    fn set_custom_name(&mut self, name: Option<String>) -> Result<(), String> { self.state.borrow_mut().name = Some(name); Ok(()) }
    fn set_no_ai(&mut self, no_ai: bool) -> Result<(), String> { self.state.borrow_mut().no_ai = Some(no_ai); Ok(()) }
    fn set_invulnerable(&mut self, invulnerable: bool) -> Result<(), String> { self.state.borrow_mut().invulnerable = Some(invulnerable); Ok(()) }
}

#[test]
fn split_children_inherit_the_head_stack_by_the_originals_draws() {
    let helmet = StackData { item: "minecraft:iron_helmet".into(), count: 1, ..StackData::default() };
    let run = |chance: f32, ints: &[i32], floats: &[f32]| {
        let mut random = ScriptedRandom { floats: floats.iter().copied().collect(), ints: ints.iter().copied().collect() };
        let states: Vec<std::rc::Rc<std::cell::RefCell<FakeChildState>>> = (0..3).map(|_| Default::default()).collect();
        let mut children: Vec<Box<dyn MobSplitChild>> = states.iter().enumerate().map(|(id, state)| Box::new(FakeChild { id: id as u32, state: state.clone() }) as Box<dyn MobSplitChild>).collect();
        let context = MobSplitContext { parent: EntityInfo { id: 9, kind: "tconstruct:sky_slime".into(), dimension: "minecraft:overworld".into() }, position: [0.0; 3], size: 4, child_size: 2 };
        let mut view = MobSplitInheritance {
            context: &context,
            head: Some(helmet.clone()), head_drop_chance: chance,
            custom_name: Some("Bob".into()), no_ai: true, invulnerable: false,
            random: &mut random, children: &mut children,
        };
        SlimeSplit.inherit(&mut view).expect("inheritance");
        states
    };
    // Drop chance 2.0: only the drawn child (index 1) receives the head stack, no copies.
    let children = run(2.0, &[1], &[]);
    let worn: Vec<bool> = children.iter().map(|child| child.borrow().gear.contains_key("head")).collect();
    assert_eq!(worn, [false, true, false]);
    // Drop chance below 1: the other children copy at 0.25 — the first copies at 0.1, the second not at 0.5.
    let children = run(0.5, &[1], &[0.1, 0.5]);
    let worn: Vec<bool> = children.iter().map(|child| child.borrow().gear.contains_key("head")).collect();
    assert_eq!(worn, [true, true, false]);
    // Every child inherits the name, no-AI and invulnerability flags.
    let children = run(2.0, &[0], &[]);
    assert!(children.iter().all(|child| { let state = child.borrow(); state.name == Some(Some("Bob".to_string())) && state.no_ai == Some(true) && state.invulnerable == Some(false) }));
}

#[test]
fn the_placement_predicates_follow_the_originals_rules() {
    let context = |peaceful: bool, reason: &str, below: &str, fluid_at: &str, fluid_below: &str, valid: bool, dark: bool| SpawnPlacementContext {
        entity_type: Identifier::parse("tconstruct:sky_slime").unwrap(),
        dimension: Identifier::parse("minecraft:overworld").unwrap(),
        position: [0, 64, 0], reason: reason.into(), peaceful,
        block_below: Identifier::parse(below).unwrap(),
        fluid_at: Identifier::parse(fluid_at).unwrap(),
        fluid_below: Identifier::parse(fluid_below).unwrap(),
        block_below_valid_spawn: valid, dark_enough: dark,
    };
    let sky = TYPES.iter().find(|slime| slime.name == "sky_slime").unwrap();
    assert!(!tag_placement(&context(true, "natural", "tconstruct:sky_sky_slime_grass", "minecraft:empty", "minecraft:empty", true, true), sky.spawn_blocks), "never on peaceful");
    assert!(tag_placement(&context(false, "spawner", "minecraft:stone", "minecraft:empty", "minecraft:empty", false, false), sky.spawn_blocks), "always from a spawner");
    assert!(tag_placement(&context(false, "natural", "tconstruct:sky_sky_slime_grass", "minecraft:empty", "minecraft:empty", false, false), sky.spawn_blocks), "on its slime grass");
    assert!(!tag_placement(&context(false, "natural", "minecraft:grass_block", "minecraft:empty", "minecraft:empty", true, true), sky.spawn_blocks), "not on plain grass");
    assert!(terracube_placement(&context(false, "natural", "minecraft:stone", "minecraft:water", "minecraft:water", false, false)), "in water");
    assert!(terracube_placement(&context(false, "natural", "minecraft:stone", "minecraft:empty", "minecraft:empty", true, true)), "a valid dark spot");
    assert!(!terracube_placement(&context(false, "natural", "minecraft:stone", "minecraft:empty", "minecraft:empty", true, false)), "not in the light");
    assert!(!terracube_placement(&context(true, "spawner", "minecraft:stone", "minecraft:water", "minecraft:water", true, true)), "never on peaceful, even from a spawner");
}

#[test]
fn teleport_destinations_and_the_civil_calendar_match_their_sources() {
    assert_eq!(teleport_destination([100.0, 70.0, -50.0], "minecraft:overworld", 0.5, 0, 0.5), [100.0, 70.0, -50.0]);
    assert_eq!(teleport_destination([0.0, 70.0, 0.0], "minecraft:overworld", 0.0, 300, 1.0), [-8.0, 319.0, 8.0], "the vertical clamps to the build range");
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(19723), (2024, 1, 1));
    assert_eq!(civil_from_days(20089), (2025, 1, 1));
}
