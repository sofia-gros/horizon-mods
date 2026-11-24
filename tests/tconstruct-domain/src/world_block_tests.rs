//! Behaviour tests for the R4-world U1/U2 slice: TinkerWorld's sticky slime
//! predicates and CongealedSlimeBlock's landing/contact rules, dispatched
//! through the registered behaviours the way the host dispatches them.
//! The W1 checks at the bottom cover the slime wood sets, slime grass, flora,
//! vines and enderbark roots as imported content instead.

use pluto_lib::{AfterFallContext, BlockBehaviors, BlockState, Direction, EntityInsideMotionContext, FallOnContext, MemoryBlockView, PathNodeType, PathNodeTypeContext, PistonBehavior};

const SLIMES: [&str; 3] = ["sky_slime", "ichor_slime", "ender_slime"];
const CONGEALED: [&str; 4] = ["earth_congealed_slime", "sky_congealed_slime", "ichor_congealed_slime", "ender_congealed_slime"];

fn state(id: &str) -> BlockState {
    BlockState::parse(id).unwrap()
}

fn slime(name: &str) -> BlockState {
    state(&format!("tconstruct:{name}"))
}

fn registered() {
    crate::world_blocks::register();
}

#[test]
fn sticky_slime_flags_and_stick_to_truth_table() {
    registered();
    for name in SLIMES {
        assert_eq!(pluto_lib::fixtures::slime_flags(&slime(name)), (Some(true), Some(true)), "{name} is a slime block and a sticky block");
    }
    // Nothing is registered for a plain block.
    assert_eq!(pluto_lib::fixtures::slime_flags(&state("minecraft:stone")), (None, None));
    assert_eq!(pluto_lib::fixtures::stick_to(&state("minecraft:stone"), &state("minecraft:stone")), None);

    // TinkerWorld's predicates: sky sticks to anything, ichor to anything but
    // itself, ender only to itself. Targets: itself, another slime type, a
    // plain block, the honey block.
    let table: [(&str, &str, [bool; 4]); 3] = [
        ("sky_slime", "ichor_slime", [true, true, true, true]),
        ("ichor_slime", "sky_slime", [false, true, true, true]),
        ("ender_slime", "sky_slime", [true, false, false, false]),
    ];
    for (name, other_slime, expected) in table {
        let this = slime(name);
        let targets = [slime(name), slime(other_slime), state("minecraft:stone"), state("minecraft:honey_block")];
        for (target, want) in targets.iter().zip(expected) {
            assert_eq!(pluto_lib::fixtures::stick_to(&this, target), Some(want), "{name} sticking to {}", target.block);
        }
    }
}

#[test]
fn congealed_slime_zeroes_fall_damage() {
    registered();
    for name in CONGEALED {
        let mut c = FallOnContext { state: slime(name), pos: [0, 64, 0], entity: 1, kind: "minecraft:player".to_string(), distance: 12.0, damage: 9.0 };
        pluto_lib::fixtures::landing_damage(&mut c);
        assert_eq!(c.damage, 0.0, "{name} deals no fall damage");
    }
    // The sticky slime blocks register no fall_on handler; damage passes through.
    let mut c = FallOnContext { state: slime("sky_slime"), pos: [0, 64, 0], entity: 1, kind: "minecraft:player".to_string(), distance: 12.0, damage: 9.0 };
    pluto_lib::fixtures::landing_damage(&mut c);
    assert_eq!(c.damage, 9.0);
}

fn after_fall(block: &str, living: bool, item: bool, suppressing_bounce: bool, vy: f64) -> AfterFallContext {
    let mut c = AfterFallContext {
        state: slime(block), pos: [0, 64, 0], entity: 1, kind: "minecraft:zombie".to_string(),
        living, item, suppressing_bounce, velocity: [0.25, vy, -0.5], fall_distance: 7.0, on_ground: true,
    };
    pluto_lib::fixtures::after_fall(&mut c);
    c
}

#[test]
fn congealed_after_fall_bounce_matches_the_original() {
    registered();
    for name in CONGEALED {
        // Living entities reflect at full strength and land without fall distance.
        let c = after_fall(name, true, false, false, -2.0);
        assert_eq!(c.velocity, [0.25, 2.0, -0.5], "{name}: living bounce");
        assert_eq!(c.fall_distance, 0.0, "{name}: living bounce clears fall distance");
        assert!(c.on_ground, "{name}: living bounce keeps on-ground");

        // Item entities reflect at 0.8 and are lifted off the ground.
        let c = after_fall(name, false, true, false, -2.0);
        assert!((c.velocity[1] - 1.6).abs() < 1e-12, "{name}: item bounce is 0.8, got {}", c.velocity[1]);
        assert_eq!(c.fall_distance, 0.0, "{name}: item bounce clears fall distance");
        assert!(!c.on_ground, "{name}: item bounce clears on-ground");

        // Anything else (the original calls out XP orbs) falls through untouched.
        let c = after_fall(name, false, false, false, -2.0);
        assert_eq!(c.velocity, [0.25, -2.0, -0.5], "{name}: other kinds do not bounce");
        assert_eq!(c.fall_distance, 7.0, "{name}: other kinds keep fall distance");

        // Suppressing the bounce (sneaking) disables it even for living entities.
        let c = after_fall(name, true, false, true, -2.0);
        assert_eq!(c.velocity, [0.25, -2.0, -0.5], "{name}: suppressed bounce does nothing");
        assert_eq!(c.fall_distance, 7.0);

        // Upward motion is not a landing.
        let c = after_fall(name, true, false, false, 1.5);
        assert_eq!(c.velocity, [0.25, 1.5, -0.5], "{name}: upward velocity untouched");
        assert_eq!(c.fall_distance, 7.0);
    }
}

fn inside(block: &str, position: [f64; 3], previous_position: [f64; 3], velocity: [f64; 3], server: bool, suppressing_bounce: bool) -> EntityInsideMotionContext {
    let mut c = EntityInsideMotionContext {
        state: slime(block), pos: [10, 64, 10], entity: 1, kind: "minecraft:zombie".to_string(),
        server, suppressing_bounce, position, previous_position, velocity, sync_velocity: false, step_sound: None,
    };
    pluto_lib::fixtures::inside_motion(&mut c);
    c
}

#[test]
fn congealed_inside_motion_side_bounce() {
    registered();
    for name in CONGEALED {
        // Moving +X into the block's east side: bounce along X at 0.95 of the
        // distance moved since last tick, marked for sync, step sound played.
        let c = inside(name, [10.9, 64.5, 10.5], [10.7, 64.5, 10.5], [0.0; 3], true, false);
        assert!((c.velocity[0] - 0.19).abs() < 1e-9, "{name}: +X bounce speed, got {}", c.velocity[0]);
        assert_eq!(c.velocity[2], 0.0, "{name}: no Z change");
        assert!(c.sync_velocity, "{name}: bounce marks velocity for sync");
        assert_eq!(c.step_sound, Some([1.0, 1.0]), "{name}: step sound above speed 0.1");

        // Moving -X: the bounce follows the direction's sign.
        let c = inside(name, [10.1, 64.5, 10.5], [10.3, 64.5, 10.5], [0.0; 3], true, false);
        assert!((c.velocity[0] + 0.19).abs() < 1e-9, "{name}: -X bounce speed, got {}", c.velocity[0]);

        // The larger axis wins: here Z.
        let c = inside(name, [10.5, 64.5, 10.9], [10.5, 64.5, 10.7], [0.0; 3], true, false);
        assert_eq!(c.velocity[0], 0.0, "{name}: no X change");
        assert!((c.velocity[2] - 0.19).abs() < 1e-9, "{name}: +Z bounce speed, got {}", c.velocity[2]);

        // A slow contact still bounces but plays no sound.
        let c = inside(name, [10.9, 64.5, 10.5], [10.8, 64.5, 10.5], [0.0; 3], true, false);
        assert!((c.velocity[0] - 0.095).abs() < 1e-9, "{name}: slow bounce speed, got {}", c.velocity[0]);
        assert!(c.sync_velocity);
        assert_eq!(c.step_sound, None, "{name}: no step sound at speed 0.095");

        // Exactly on the block centre the chosen axis' direction is zero, and
        // Java's signum(0) makes the bounce zero rather than full speed.
        let c = inside(name, [10.5, 64.5, 10.5], [10.5, 64.9, 10.5], [0.0, 0.0, 3.0], true, false);
        assert_eq!(c.velocity[2], 0.0, "{name}: zero direction means zero bounce");
        assert!(c.sync_velocity);
    }
}

#[test]
fn congealed_inside_motion_guards() {
    registered();
    let name = "earth_congealed_slime";
    // The y window is [-0.0625, 0.9365) of the block's own height: the top
    // edge is excluded, the bottom edge included.
    let c = inside(name, [10.9, 64.94, 10.5], [10.7, 64.94, 10.5], [0.0; 3], true, false);
    assert_eq!(c.velocity, [0.0; 3], "above the window there is no bounce");
    assert!(!c.sync_velocity);
    // Exactly 0.9365 (measured from a block at y=0 so the subtraction is
    // exact) is outside the window.
    let mut c = EntityInsideMotionContext {
        state: slime(name), pos: [10, 0, 10], entity: 1, kind: "minecraft:zombie".to_string(),
        server: true, suppressing_bounce: false, position: [10.9, 0.9365, 10.5], previous_position: [10.7, 0.9365, 10.5],
        velocity: [0.0; 3], sync_velocity: false, step_sound: None,
    };
    pluto_lib::fixtures::inside_motion(&mut c);
    assert_eq!(c.velocity, [0.0; 3], "at the top edge there is no bounce");
    assert!(!c.sync_velocity);
    let c = inside(name, [10.9, 63.9, 10.5], [10.7, 63.9, 10.5], [0.0; 3], true, false);
    assert_eq!(c.velocity, [0.0; 3], "below the window there is no bounce");
    assert!(!c.sync_velocity);
    let c = inside(name, [10.9, 63.9375, 10.5], [10.7, 63.9375, 10.5], [0.0; 3], true, false);
    assert!((c.velocity[0] - 0.19).abs() < 1e-9, "the bottom edge still bounces, got {}", c.velocity[0]);

    // No double bounce: past the halfway point on the other axis, no bounce.
    let c = inside(name, [11.2, 64.5, 11.0], [11.0, 64.5, 11.0], [0.0; 3], true, false);
    assert_eq!(c.velocity, [0.0; 3], "across offset 0.5 blocks the X bounce");
    assert!(!c.sync_velocity);
    let c = inside(name, [11.2, 64.5, 10.99], [11.0, 64.5, 10.99], [0.0; 3], true, false);
    assert!((c.velocity[0] - 0.19).abs() < 1e-9, "across offset 0.49 still bounces, got {}", c.velocity[0]);

    // Client side and suppressing entities never bounce.
    let c = inside(name, [10.9, 64.5, 10.5], [10.7, 64.5, 10.5], [0.0; 3], false, false);
    assert_eq!(c.velocity, [0.0; 3], "no client-side bounce");
    assert!(!c.sync_velocity);
    let c = inside(name, [10.9, 64.5, 10.5], [10.7, 64.5, 10.5], [0.0; 3], true, true);
    assert_eq!(c.velocity, [0.0; 3], "no suppressed bounce");
    assert!(!c.sync_velocity);
}

#[test]
fn congealed_path_node_is_sticky_honey_and_all_seven_blocks_register() {
    registered();
    let view = MemoryBlockView::default();
    for name in CONGEALED {
        let c = PathNodeTypeContext { state: slime(name), pos: [0, 64, 0], view: &view, entity: None, kind: None };
        assert_eq!(pluto_lib::fixtures::path_node(&c), Some(PathNodeType::StickyHoney), "{name} paths as sticky honey");
    }
    // The sticky slime blocks register no path handler.
    let c = PathNodeTypeContext { state: slime("sky_slime"), pos: [0, 64, 0], view: &view, entity: None, kind: None };
    assert_eq!(pluto_lib::fixtures::path_node(&c), None);
    // Registration of the seven blocks is proven by dispatch: slime flags and
    // the stick table cover the three slimes; landing damage, the after-fall
    // and inside-motion fixtures (which unwrap their handlers) and the path
    // node cover the four congealed blocks.
}

// The slime wood sets, slime grass, flora, vines and enderbark roots of TinkerWorld, as imported content: the declarations
// carry the original's properties, the world tags that named them resolve to declared blocks, and the world tab lists the
// item-bearing ones in the original's order.

use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;

const W1_WOODS: [&str; 4] = ["greenheart", "skyroot", "bloodshroom", "enderbark"];
const W1_FOLIAGE: [&str; 5] = ["earth", "sky", "ichor", "ender", "blood"];
const W1_DIRTS: [&str; 5] = ["vanilla", "earth", "sky", "ichor", "ender"];
const W1_SLIMES: [&str; 4] = ["earth", "sky", "ichor", "ender"];

fn w1_content() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data") }

fn w1_declaration(name: &str) -> Value {
    let path = w1_content().join(format!("tconstruct/horizon/block/{name}.json"));
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|_| panic!("declaration {path:?}"))).unwrap()
}

fn w1_names() -> Vec<String> {
    let mut names = Vec::new();
    for wood in W1_WOODS {
        for part in ["planks", "planks_slab", "planks_stairs", "fence", "fence_gate", "door", "trapdoor", "pressure_plate", "button",
                     "sign", "wall_sign", "hanging_sign", "wall_hanging_sign", "log", "wood"] {
            names.push(format!("{wood}_{part}"));
        }
        names.push(format!("stripped_{wood}_log"));
        names.push(format!("stripped_{wood}_wood"));
    }
    for foliage in W1_FOLIAGE {
        for dirt in W1_DIRTS { names.push(format!("{foliage}_{dirt}_slime_grass")); }
        for part in ["slime_sapling", "slime_leaves", "slime_fern", "slime_tall_grass"] { names.push(format!("{foliage}_{part}")); }
        names.push(format!("potted_{foliage}_slime_fern"));
        names.push(format!("potted_{foliage}_slime_sapling"));
    }
    names.push("sky_slime_vine".to_owned());
    names.push("ender_slime_vine".to_owned());
    names.push("enderbark_roots".to_owned());
    for slime in W1_SLIMES { names.push(format!("{slime}_enderbark_roots")); }
    names
}

#[test]
fn slime_wood_grass_flora_vines_and_roots_are_declared_with_the_original_properties() {
    let names = w1_names();
    assert_eq!(names.len(), 130);
    let declared = crate::casting_loop::declared();
    for name in &names {
        assert!(declared.contains(&format!("tconstruct:{name}")), "{name} is declared");
        w1_declaration(name);
    }
    // createSlimewood sets no strength: the default variant takes the planks colour and the slime sound with no tool
    // requirement, the log variant the bark colour, the wood sound and the tool requirement. Map colours are the host's ids:
    // COLOR_LIGHT_GREEN 19, COLOR_GREEN 27, TERRACOTTA_CYAN 45, COLOR_RED 28, COLOR_BLACK 29.
    let planks = w1_declaration("greenheart_planks");
    assert_eq!(planks["template"].as_str(), Some("minecraft:oak_planks"));
    assert_eq!(planks["properties"]["map_color"].as_i64(), Some(19));
    assert_eq!(planks["properties"]["sounds"].as_str(), Some("minecraft:slime_block"));
    assert_eq!(planks["properties"]["requires_tool"].as_bool(), Some(false));
    assert_eq!(planks["properties"]["hardness"].as_f64(), Some(0.0));
    let log = w1_declaration("greenheart_log");
    assert_eq!(log["template"].as_str(), Some("minecraft:oak_log"));
    assert_eq!(log["properties"]["map_color"].as_i64(), Some(27));
    assert_eq!(log["properties"]["sounds"].as_str(), Some("minecraft:oak_log"));
    assert_eq!(log["properties"]["requires_tool"].as_bool(), Some(true));
    assert_eq!(w1_declaration("skyroot_log")["properties"]["map_color"].as_i64(), Some(45));
    assert_eq!(w1_declaration("bloodshroom_planks")["properties"]["map_color"].as_i64(), Some(28));
    assert_eq!(w1_declaration("enderbark_wood")["properties"]["map_color"].as_i64(), Some(29));
    // The wall signs and the potted flora have no item in the original.
    assert_eq!(w1_declaration("greenheart_wall_sign")["item"].as_bool(), Some(false));
    assert_eq!(w1_declaration("greenheart_wall_hanging_sign")["item"].as_bool(), Some(false));
    assert_eq!(w1_declaration("potted_earth_slime_fern")["item"].as_bool(), Some(false));
    assert_eq!(w1_declaration("greenheart_sign")["item"].as_bool(), Some(true));
    // slimeGrassProps: strength 2.0 with the tool requirement; the overworld foliage is the grass block (SnowyDirtBlock),
    // the nether foliage nylium, a plain block.
    let grass = w1_declaration("earth_earth_slime_grass");
    assert_eq!(grass["template"].as_str(), Some("minecraft:grass_block"));
    assert_eq!(grass["properties"]["hardness"].as_f64(), Some(2.0));
    assert_eq!(grass["properties"]["requires_tool"].as_bool(), Some(true));
    assert_eq!(grass["properties"]["map_color"].as_i64(), Some(1));
    let nylium = w1_declaration("ichor_sky_slime_grass");
    assert_eq!(nylium["template"].as_str(), Some("minecraft:dirt"));
    assert_eq!(nylium["properties"]["map_color"].as_i64(), Some(15));
    // The leaves: the overworld ones are leaves (strength 1.0, no occlusion), the nether ones plain wart blocks (strength 1.5).
    let leaves = w1_declaration("earth_slime_leaves");
    assert_eq!(leaves["template"].as_str(), Some("minecraft:oak_leaves"));
    assert_eq!(leaves["properties"]["hardness"].as_f64(), Some(1.0));
    assert_eq!(leaves["properties"]["occludes"].as_bool(), Some(false));
    let wart = w1_declaration("blood_slime_leaves");
    assert_eq!(wart["template"].as_str(), Some("minecraft:dirt"));
    assert_eq!(wart["properties"]["hardness"].as_f64(), Some(1.5));
    assert_eq!(wart["properties"]["sounds"].as_str(), Some("minecraft:nether_wart_block"));
    // The ender sapling is the propagule and the ichor one a fungus; the vine carries its growth stage state at strength 0.75.
    assert_eq!(w1_declaration("ender_slime_sapling")["template"].as_str(), Some("minecraft:mangrove_propagule"));
    assert_eq!(w1_declaration("ichor_slime_sapling")["template"].as_str(), Some("minecraft:crimson_fungus"));
    let vine = w1_declaration("sky_slime_vine");
    assert_eq!(vine["properties"]["hardness"].as_f64(), Some(0.75));
    assert_eq!(vine["state_properties"]["stage"]["values"], serde_json::json!(["start", "middle", "end"]));
    // The roots: strength 0.7; the ichor ones glow at the slime light of 10.
    assert_eq!(w1_declaration("enderbark_roots")["template"].as_str(), Some("minecraft:mangrove_roots"));
    assert_eq!(w1_declaration("enderbark_roots")["properties"]["hardness"].as_f64(), Some(0.7));
    assert_eq!(w1_declaration("ichor_enderbark_roots")["properties"]["light"].as_i64(), Some(10));
}

fn w1_tag(path: &str, into: &mut BTreeSet<String>) {
    let file = w1_content().join(format!("tconstruct/tags/block/{path}.json"));
    let data: Value = serde_json::from_slice(&std::fs::read(&file).unwrap_or_else(|_| panic!("tag {file:?}"))).unwrap();
    for value in data["values"].as_array().unwrap() {
        let value = value.as_str().map(str::to_owned).unwrap_or_else(|| value["id"].as_str().unwrap().to_owned());
        if let Some(reference) = value.strip_prefix("#tconstruct:") { w1_tag(reference, into); } else { into.insert(value); }
    }
}
const CLUSTERS: [&str; 19] = [
    "earth_slime_crystal_cluster", "small_earth_slime_crystal_bud", "medium_earth_slime_crystal_bud", "large_earth_slime_crystal_bud",
    "sky_slime_crystal_cluster", "small_sky_slime_crystal_bud", "medium_sky_slime_crystal_bud", "large_sky_slime_crystal_bud",
    "ichor_slime_crystal_cluster", "small_ichor_slime_crystal_bud", "medium_ichor_slime_crystal_bud", "large_ichor_slime_crystal_bud",
    "ender_slime_crystal_cluster", "small_ender_slime_crystal_bud", "medium_ender_slime_crystal_bud", "large_ender_slime_crystal_bud",
    "steel_cluster", "cobalt_cluster", "knightmetal_cluster",
];
const BUDDING: [&str; 4] = ["budding_earth_slime_crystal", "budding_sky_slime_crystal", "budding_ichor_slime_crystal", "budding_ender_slime_crystal"];
const HEADS: [&str; 9] = ["blaze_head", "enderman_head", "stray_head", "husk_head", "drowned_head", "spider_head", "cave_spider_head", "piglin_brute_head", "zombified_piglin_head"];
const WALL_HEADS: [&str; 9] = ["blaze_wall_head", "enderman_wall_head", "stray_wall_head", "husk_wall_head", "drowned_wall_head", "spider_wall_head", "cave_spider_wall_head", "piglin_brute_wall_head", "zombified_piglin_wall_head"];

#[test]
fn clusters_point_along_the_clicked_face_and_keep_their_water() {
    for (face, name) in [(Direction::Down, "down"), (Direction::Up, "up"), (Direction::North, "north"),
                         (Direction::South, "south"), (Direction::West, "west"), (Direction::East, "east")] {
        let placed = crate::world_blocks::cluster_placement(slime("steel_cluster"), face, false);
        assert_eq!(placed.property("facing"), Some(name), "{name}");
        assert_eq!(placed.property("waterlogged"), Some("false"), "{name} placed dry");
        let wet = crate::world_blocks::cluster_placement(slime("steel_cluster"), face, true);
        assert_eq!(wet.property("waterlogged"), Some("true"), "{name} placed in water");
    }
}

#[test]
fn clusters_survive_only_behind_a_sturdy_block() {
    let up = crate::world_blocks::cluster_placement(slime("steel_cluster"), Direction::Up, false);
    let mut view = MemoryBlockView::default();
    assert!(!crate::world_blocks::cluster_survives(&up, [10, 64, 10], &view), "no support below");
    view.blocks.insert([10, 63, 10], state("minecraft:stone"));
    assert!(crate::world_blocks::cluster_survives(&up, [10, 64, 10], &view), "the stone below supports an up cluster");
    let north = crate::world_blocks::cluster_placement(slime("steel_cluster"), Direction::North, false);
    assert!(!crate::world_blocks::cluster_survives(&north, [10, 64, 10], &view), "the stone below does not support a north cluster");
    view.blocks.insert([10, 64, 11], state("minecraft:stone"));
    assert!(crate::world_blocks::cluster_survives(&north, [10, 64, 10], &view), "the stone behind supports a north cluster");
}

#[test]
fn heads_take_the_placers_yaw_segment() {
    for (yaw, segment) in [(0.0, "0"), (11.0, "0"), (12.0, "1"), (90.0, "4"), (180.0, "8"), (270.0, "12"), (-90.0, "12"), (360.0, "0")] {
        let placed = crate::world_blocks::head_placement(slime("blaze_head"), yaw);
        assert_eq!(placed.property("rotation"), Some(segment), "yaw {yaw}");
    }
}

#[test]
fn the_world_tags_that_named_the_new_blocks_resolve_to_declared_blocks() {
    let declared = crate::casting_loop::declared();
    let mut soil = BTreeSet::new();
    w1_tag("slimy_soil", &mut soil);
    // The five grasses of every foliage type, the four slime dirts and the four slimy enderbark roots.
    assert_eq!(soil.len(), 33, "{soil:?}");
    for entry in &soil { assert!(declared.contains(entry), "slimy_soil names {entry}, which is not declared"); }
    for name in ["slimy_grass", "slimy_nylium", "slimy_leaves", "slimy_logs", "slimy_planks", "slimy_saplings", "slimy_vines", "enderbark/roots"] {
        let mut entries = BTreeSet::new();
        w1_tag(name, &mut entries);
        assert!(!entries.is_empty(), "{name}");
        for entry in &entries { assert!(declared.contains(entry), "{name} names {entry}, which is not declared"); }
    }
}

#[test]
fn the_world_tab_lists_the_wood_and_foliage_in_the_original_order() {
    let groups = crate::creative::groups();
    let world = groups.iter().find(|group| group.id.path() == "world").expect("world tab");
    let entries: Vec<String> = world.entries.iter().map(|stack| stack.item.path().to_owned()).collect();
    let at = |name: &str| entries.iter().position(|entry| entry == name).unwrap_or_else(|| panic!("{name} in the world tab"));
    // Each wood set, then its sapling and leaves; the enderbark roots follow the enderbark set.
    assert!(at("greenheart_planks") < at("greenheart_sign"));
    assert!(at("greenheart_hanging_sign") < at("earth_slime_sapling") && at("earth_slime_sapling") < at("earth_slime_leaves"));
    assert!(at("earth_slime_leaves") < at("skyroot_planks"));
    assert!(at("ender_slime_leaves") < at("enderbark_roots") && at("enderbark_roots") < at("earth_enderbark_roots"));
    // Dirt, then each visible foliage type's tall grass, fern, vine and five grasses; the shards close the tab.
    assert!(at("ender_enderbark_roots") < at("earth_slime_dirt") && at("earth_slime_dirt") < at("earth_slime_tall_grass"));
    assert!(at("sky_slime_fern") < at("sky_slime_vine") && at("sky_slime_vine") < at("sky_vanilla_slime_grass"));
    assert!(at("ender_ender_slime_grass") < at("steel_shard"));
    // Every item-bearing W1 block is listed; the wall signs and the potted flora have no item and are not, and neither is
    // the ichor foliage (FoliageType.VISIBLE excludes ichor, so the original's tab never lists it).
    let listed: BTreeSet<&String> = entries.iter().collect();
    for name in w1_names() {
        let ichor_foliage = name.starts_with("ichor_")
            && ["_slime_grass", "_slime_leaves", "_slime_sapling", "_slime_fern", "_slime_tall_grass"].iter().any(|suffix| name.ends_with(suffix));
        let itemless = name.starts_with("potted_") || name.ends_with("_wall_sign") || name.ends_with("_wall_hanging_sign") || ichor_foliage;
        assert_eq!(listed.contains(&name), !itemless, "{name} in the world tab");
    }
}

#[test]
fn wall_heads_pick_the_standing_head() {
    for (wall, head) in WALL_HEADS.iter().zip(HEADS) {
        let pick = crate::world_blocks::wall_head_pick(&slime(wall)).expect("a wall head picks");
        assert_eq!(pick.item.to_string(), format!("tconstruct:{head}"), "{wall}");
        assert_eq!(pick.count, 1, "{wall}");
    }
    assert!(crate::world_blocks::wall_head_pick(&slime("blaze_head")).is_none(), "a standing head is not a wall head");
}

#[test]
fn geode_and_head_piston_rules_and_registration() {
    registered();
    for name in CLUSTERS.iter().chain(&BUDDING).chain(&HEADS) {
        let behaviour = BlockBehaviors::get(&format!("tconstruct:{name}")).unwrap_or_else(|| panic!("{name} registers a behaviour"));
        assert_eq!(behaviour.piston_behavior_override(), Some(PistonBehavior::Destroy), "{name} is destroyed by pistons");
    }
    // The wall heads register only their pick: a piston moves them normally, as the original leaves them.
    for name in WALL_HEADS {
        let behaviour = BlockBehaviors::get(&format!("tconstruct:{name}")).unwrap_or_else(|| panic!("{name} registers a behaviour"));
        assert_eq!(behaviour.piston_behavior_override(), None, "{name} keeps the normal piston behaviour");
    }
}

#[test]
fn crystal_chime_sounds_match_the_originals_blocks() {
    let chime = crate::world_blocks::chime_sounds;
    // Every geode block chimes with its geode's chime: the crystal block and the budding block over the
    // amethyst block hit sound, the cluster and the three buds over the amethyst cluster hit sound.
    for (geode, sound) in [
        ("earth", "tconstruct:block.earth_crystal.chime"),
        ("sky", "tconstruct:block.sky_crystal.chime"),
        ("ichor", "tconstruct:block.ichor_crystal.chime"),
        ("ender", "tconstruct:block.ender_crystal.chime"),
    ] {
        assert_eq!(chime(&format!("{geode}_slime_crystal_block")), Some(("minecraft:block.amethyst_block.hit", sound)), "{geode} crystal block");
        assert_eq!(chime(&format!("budding_{geode}_slime_crystal")), Some(("minecraft:block.amethyst_block.hit", sound)), "{geode} budding block");
        for name in [
            format!("{geode}_slime_crystal_cluster"),
            format!("small_{geode}_slime_crystal_bud"),
            format!("medium_{geode}_slime_crystal_bud"),
            format!("large_{geode}_slime_crystal_bud"),
        ] {
            assert_eq!(chime(&name), Some(("minecraft:block.amethyst_cluster.hit", sound)), "{name}");
        }
    }
    // The metal clusters chime with another geode's chime, over their own sound types.
    assert_eq!(chime("steel_cluster"), Some(("minecraft:block.metal.hit", "tconstruct:block.sky_crystal.chime")));
    assert_eq!(chime("cobalt_cluster"), Some(("minecraft:block.netherite_block.hit", "tconstruct:block.ichor_crystal.chime")));
    assert_eq!(chime("knightmetal_cluster"), Some(("minecraft:block.netherite_block.hit", "tconstruct:block.ender_crystal.chime")));
    for other in ["earth_slime_crystal", "sky_slime", "earth_congealed_slime", "stone", "budding_earth_slime", "earth_slime_crystal_shard"] {
        assert_eq!(chime(other), None, "{other} does not chime");
    }
}

#[test]
fn chime_pitch_spans_the_originals_range() {
    let pitch = crate::world_blocks::chime_pitch;
    assert_eq!(pitch(0.0), 0.5);
    assert!((pitch(0.5) - 1.1).abs() < 1e-6);
    assert!(pitch(0.999) < 1.7, "a draw below 1 stays below 0.5 + 1.2");
}

#[test]
fn a_projectile_hit_on_a_crystal_plays_its_hit_sound_then_its_chime() {
    use pluto_lib::{MemoryWorldOps, NbtCompound, ProjectileImpact, with_world};
    let hit = |entity: Option<u32>| ProjectileImpact {
        dimension: "minecraft:overworld".into(),
        projectile: 7,
        kind: "minecraft:arrow".into(),
        owner: 1,
        entity,
        entity_kind: String::new(),
        entity_effects: Vec::new(),
        block: [4, 70, 4],
        face: [0, 1, 0],
        at: [4.5, 71.0, 4.5],
        motion: [0.0, 0.0, 0.0],
        data: NbtCompound::new(),
        piercing_ignored: Vec::new(),
        discard: std::cell::Cell::new(false),
        motion_after: std::cell::RefCell::new(None),
        ignore_entities: std::cell::RefCell::new(Vec::new()),
        actions: std::cell::RefCell::new(Vec::new()),
    };
    let mut ops = MemoryWorldOps::default();
    ops.blocks.insert(("minecraft:overworld".into(), [4, 70, 4]), "tconstruct:earth_slime_crystal_cluster".into());
    with_world(&mut ops, || {
        assert!(crate::world_blocks::chime_on_impact(&hit(None)), "the hit itself is never cancelled");
        assert!(crate::world_blocks::chime_on_impact(&hit(Some(9))), "an entity hit chimes nothing");
    });
    assert_eq!(
        ops.sounds,
        vec![
            ("minecraft:block.amethyst_cluster.hit".to_string(), [4.5, 70.5, 4.5]),
            ("tconstruct:block.earth_crystal.chime".to_string(), [4.5, 70.5, 4.5]),
        ],
        "the hit sound plays first, then the chime, both at the block's centre"
    );
}

#[test]
fn head_drop_matches_the_originals_creeper_rule() {
    use crate::world_blocks::head_drop_for;
    // TinkerHeadType.fromEntityType: exactly the nine entity types have heads.
    for (name, kind) in crate::world_blocks::HEAD_TYPES {
        assert_eq!(head_drop_for(Some("minecraft:creeper"), kind), Some(name), "{kind}");
    }
    // A mob with a vanilla head, the creeper itself and a player drop no Tinkers head.
    for kind in ["minecraft:zombie", "minecraft:skeleton", "minecraft:creeper", "minecraft:player"] {
        assert_eq!(head_drop_for(Some("minecraft:creeper"), kind), None, "{kind}");
    }
    // Only a creeper's kill counts: not a player's, not a fall, not an unknown owner.
    for killer in [Some("minecraft:player"), Some("minecraft:skeleton"), None] {
        assert_eq!(head_drop_for(killer, "minecraft:husk"), None, "{killer:?}");
    }
}

#[test]
fn a_charged_creeper_kill_claims_the_skull_and_appends_the_head() {
    use pluto_lib::{EntityInfo, LivingDropInfo, LivingEntityDrops, StackData};
    struct Drops {
        info: LivingDropInfo,
        eligible: bool,
        claimed: bool,
        stacks: Vec<StackData>,
    }
    impl LivingEntityDrops for Drops {
        fn info(&self) -> &LivingDropInfo { &self.info }
        fn can_drop_head(&self) -> Result<bool, String> { Ok(self.eligible) }
        fn claim_head_drop(&mut self) -> Result<bool, String> {
            self.claimed = true;
            Ok(std::mem::replace(&mut self.eligible, false))
        }
        fn append_stack(&mut self, stack: StackData) -> Result<(), String> {
            self.stacks.push(stack);
            Ok(())
        }
    }
    let drops = |killer: Option<&str>, kind: &str, eligible: bool| Drops {
        info: LivingDropInfo {
            entity: EntityInfo { id: 5, kind: kind.into(), dimension: "minecraft:overworld".into() },
            position: [0.0; 3],
            damage_type: Some("minecraft:explosion".into()),
            killer_type: killer.map(str::to_owned),
        },
        eligible,
        claimed: false,
        stacks: Vec::new(),
    };
    // The config is not loaded in this process, so head drops read at the defaults: every head drops.
    let mut husk = drops(Some("minecraft:creeper"), "minecraft:husk", true);
    crate::world_blocks::creeper_head_drop(&mut husk).unwrap();
    assert!(husk.claimed, "the creeper's skull is claimed before the drop");
    assert_eq!(husk.stacks.len(), 1);
    assert_eq!(husk.stacks[0].item, "tconstruct:husk_head");
    assert_eq!(husk.stacks[0].count, 1);
    // A creeper that already spent its skull drops nothing, and nothing is claimed.
    let mut spent = drops(Some("minecraft:creeper"), "minecraft:husk", false);
    crate::world_blocks::creeper_head_drop(&mut spent).unwrap();
    assert!(spent.stacks.is_empty());
    assert!(!spent.claimed);
    // Other killers and headless types neither claim nor drop.
    for (killer, kind) in [(Some("minecraft:player"), "minecraft:husk"), (Some("minecraft:creeper"), "minecraft:zombie"), (None, "minecraft:husk")] {
        let mut other = drops(killer, kind, true);
        crate::world_blocks::creeper_head_drop(&mut other).unwrap();
        assert!(other.stacks.is_empty(), "{killer:?} killing {kind}");
        assert!(!other.claimed, "{killer:?} killing {kind}");
    }
}

#[test]
fn skull_models_match_the_originals_layers_textures_and_geometry() {
    use pluto_lib::SkullModelAnimation;
    let models = &crate::world_blocks::HEAD_MODELS;
    assert_eq!(models.len(), 9);
    // SkullModelHelper.HEAD_LAYERS: tconstruct:<type>_head#main, and the item is tconstruct:<type>_head.
    for model in models {
        assert_eq!(model.layer_id(), format!("tconstruct:{}_head#main", model.name), "{}", model.name);
        assert_eq!(model.item(), format!("{}_head", model.name), "{}", model.name);
    }
    // The registration paths are the same items, in the same order.
    for (model, path) in models.iter().zip(crate::world_blocks::HEAD_ITEM_PATHS) {
        assert_eq!(model.item(), path, "{}", model.name);
    }
    let by_name = |name: &str| models.iter().find(|model| model.name == name).unwrap();
    // SKIN_BY_TYPE: three heads read Tinkers textures, the rest vanilla ones.
    assert_eq!(by_name("enderman").texture, "tconstruct:entity/skull/enderman");
    assert_eq!(by_name("stray").texture, "tconstruct:entity/skull/stray");
    assert_eq!(by_name("drowned").texture, "tconstruct:entity/skull/drowned");
    assert_eq!(by_name("blaze").texture, "minecraft:entity/blaze");
    assert_eq!(by_name("husk").texture, "minecraft:entity/zombie/husk");
    // Only the piglin heads animate as piglins.
    assert_eq!(by_name("piglin_brute").animation, SkullModelAnimation::Piglin);
    assert_eq!(by_name("zombified_piglin").animation, SkullModelAnimation::Piglin);
    assert_eq!(by_name("blaze").animation, SkullModelAnimation::None);
    // Blaze: SkullModel.createMobHeadLayer — one 8x8x8 head cube on a 64x64 texture.
    let blaze = by_name("blaze").layer_definition();
    assert_eq!(blaze.texture_size, [64, 64]);
    assert_eq!(blaze.parts.len(), 1);
    assert_eq!(blaze.parts[0].0, "head");
    assert_eq!(blaze.parts[0].1.cubes.len(), 1);
    assert_eq!(blaze.parts[0].1.cubes[0].origin, [-4.0, -8.0, -4.0]);
    assert_eq!(blaze.parts[0].1.cubes[0].dimensions, [8.0, 8.0, 8.0]);
    // Enderman: createHeadLayer(0, 0, 32, 16).
    assert_eq!(by_name("enderman").layer_definition().texture_size, [32, 16]);
    // Stray: createHeadHatLayer(0, 16, 32, 32) — a hat cube grown by 0.25 reading at (0, 16).
    let stray = by_name("stray").layer_definition();
    assert_eq!(stray.texture_size, [32, 32]);
    let head = &stray.parts[0].1;
    assert_eq!(head.children.len(), 1);
    assert_eq!(head.children[0].0, "hat");
    assert_eq!(head.children[0].1.cubes[0].texture_offset, [0, 16]);
    assert_eq!(head.children[0].1.cubes[0].deformation, [0.25; 3]);
    // Spider: createHeadLayer(32, 4, 64, 32).
    let spider = by_name("spider").layer_definition();
    assert_eq!(spider.texture_size, [64, 32]);
    assert_eq!(spider.parts[0].1.cubes[0].texture_offset, [32, 4]);
    // Piglin: the 10-wide head with snout and nostrils, and two posed ears, the right one mirrored.
    let piglin = by_name("piglin_brute").layer_definition();
    assert_eq!(piglin.texture_size, [64, 64]);
    let head = &piglin.parts[0].1;
    assert_eq!(head.cubes.len(), 4);
    assert_eq!(head.cubes[0].dimensions, [10.0, 8.0, 8.0]);
    assert_eq!(head.cubes[1].texture_offset, [31, 1]);
    assert!(head.cubes[3].mirror, "the left nostril is mirrored");
    assert_eq!(head.children.len(), 2);
    assert_eq!(head.children[0].0, "left_ear");
    assert_eq!(head.children[0].1.pose.translation, [4.5, -6.0, 0.0]);
    assert_eq!(head.children[0].1.pose.rotation, [0.0, 0.0, -0.3491]);
    assert_eq!(head.children[1].0, "right_ear");
    assert_eq!(head.children[1].1.pose.translation, [-4.5, -6.0, 0.0]);
    assert!(head.children[1].1.cubes[0].mirror, "the right ear is mirrored");
}
