//! Behaviour tests for the W3 growth lane: TinkerWorld's growth mechanisms, dispatched through the
//! registered behaviours the way the host dispatches them (fixtures), with the pure conversions checked
//! directly. The rolls and floats queued on the memory world are the original's RandomSource draws, in
//! the order the Java code draws them.

use pluto_lib::{BlockBehaviors, BlockState, BonemealContext, Direction, MemoryBlockView, PistonBehavior, Placement, PlacementContext, ShapeUpdateContext, SurviveContext, TickContext};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn state(id: &str) -> BlockState {
    BlockState::parse(id).unwrap()
}

fn registered() {
    crate::world_blocks::register();
    crate::world_growth::register();
}

fn tick(view: &mut MemoryBlockView, block: &str, at: [i32; 3]) -> bool {
    let state = view.blocks.get(&at).cloned().unwrap_or_else(|| state(&format!("tconstruct:{block}")));
    let mut c = TickContext { state, pos: at, world: view };
    pluto_lib::fixtures::random_tick(&format!("tconstruct:{block}"), &mut c)
}

fn content() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data") }

fn declaration(name: &str) -> serde_json::Value {
    let path = content().join(format!("tconstruct/horizon/block/{name}.json"));
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|_| panic!("declaration {path:?}"))).unwrap()
}

#[test]
fn growth_blocks_are_registered_and_the_budding_piston_rule_survives() {
    registered();
    for name in ["earth_earth_slime_grass", "blood_vanilla_slime_grass", "earth_slime_sapling", "ichor_slime_sapling", "ender_slime_sapling",
                 "ender_slime_leaves", "sky_slime_vine", "ender_slime_vine", "budding_sky_slime_crystal"] {
        assert!(BlockBehaviors::get(&format!("tconstruct:{name}")).is_some(), "{name} registers a behaviour");
    }
    // world_growth registers after world_blocks and restates the budding blocks' piston rule.
    let budding = BlockBehaviors::get("tconstruct:budding_earth_slime_crystal").expect("budding behaviour");
    assert_eq!(budding.piston_behavior_override(), Some(PistonBehavior::Destroy));
}

#[test]
fn grass_decays_to_the_dirt_of_its_own_dirt_type() {
    registered();
    // earth foliage on sky dirt, covered by a full block: the light test fails and it decays to sky dirt.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:earth_sky_slime_grass"));
    view.blocks.insert([0, 65, 0], state("minecraft:stone"));
    assert!(tick(&mut view, "earth_sky_slime_grass", [0, 64, 0]));
    assert_eq!(view.blocks[&[0, 64, 0]], state("tconstruct:sky_slime_dirt"));

    // A full fluid above also decays it, even in the open.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:earth_sky_slime_grass"));
    view.water.insert([0, 65, 0]);
    tick(&mut view, "earth_sky_slime_grass", [0, 64, 0]);
    assert_eq!(view.blocks[&[0, 64, 0]], state("tconstruct:sky_slime_dirt"));

    // A single snow layer above is fine, and with the light above low nothing spreads either.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:earth_sky_slime_grass"));
    view.blocks.insert([0, 65, 0], state("minecraft:snow[layers=1]"));
    view.light.insert([0, 65, 0], 0);
    tick(&mut view, "earth_sky_slime_grass", [0, 64, 0]);
    assert!(view.writes.is_empty(), "no decay and no spread: {:?}", view.writes);

    // The area-loaded guard comes first: an unloaded neighbour means no tick at all.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:earth_sky_slime_grass"));
    view.blocks.insert([0, 65, 0], state("minecraft:stone"));
    view.unloaded.insert([3, 64, 0]);
    tick(&mut view, "earth_sky_slime_grass", [0, 64, 0]);
    assert!(view.writes.is_empty());
}

#[test]
fn grass_spreads_its_own_foliage_to_dirt() {
    registered();
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:earth_earth_slime_grass"));
    view.blocks.insert([1, 64, 0], state("minecraft:dirt"));
    // First attempt steps +1 x, 0 y, 0 z onto the dirt; the rest step away into air and find no dirt.
    view.rolls = vec![2, 3, 1];
    tick(&mut view, "earth_earth_slime_grass", [0, 64, 0]);
    assert_eq!(view.blocks[&[1, 64, 0]], state("tconstruct:earth_vanilla_slime_grass[snowy=false]"));

    // Water above the target stops the spread (canSpread's fluid test).
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:earth_earth_slime_grass"));
    view.blocks.insert([1, 64, 0], state("minecraft:dirt"));
    view.water.insert([1, 65, 0]);
    view.rolls = vec![2, 3, 1];
    tick(&mut view, "earth_earth_slime_grass", [0, 64, 0]);
    assert_eq!(view.blocks[&[1, 64, 0]], state("minecraft:dirt"));
}

#[test]
fn nylium_decays_on_light_alone_and_has_no_area_guard() {
    registered();
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:ichor_earth_slime_grass"));
    view.blocks.insert([0, 65, 0], state("minecraft:stone"));
    view.unloaded.insert([1, 64, 0]); // the nylium never asks, unlike the grass
    tick(&mut view, "ichor_earth_slime_grass", [0, 64, 0]);
    assert_eq!(view.blocks[&[0, 64, 0]], state("tconstruct:earth_slime_dirt"));

    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:blood_sky_slime_grass"));
    tick(&mut view, "blood_sky_slime_grass", [0, 64, 0]);
    assert!(view.writes.is_empty(), "open sky keeps nylium");
}

#[test]
fn dirt_and_grass_state_conversions_match_the_original_tables() {
    // getDirtState: the dirt of the grass's dirt type, vanilla dirt for anything that is not slime grass.
    assert_eq!(crate::world_growth::dirt_state(&state("tconstruct:blood_ichor_slime_grass")), state("tconstruct:ichor_slime_dirt"));
    assert_eq!(crate::world_growth::dirt_state(&state("tconstruct:sky_vanilla_slime_grass")), state("minecraft:dirt"));
    assert_eq!(crate::world_growth::dirt_state(&state("minecraft:grass_block")), state("minecraft:dirt"));
    // getStateFromDirt: any allDirt block takes any foliage; other blocks take none.
    assert_eq!(crate::world_growth::state_from_dirt(&state("minecraft:dirt"), "sky"), Some(state("tconstruct:sky_vanilla_slime_grass")));
    assert_eq!(crate::world_growth::state_from_dirt(&state("tconstruct:ender_slime_dirt"), "blood"), Some(state("tconstruct:blood_ender_slime_grass")));
    assert_eq!(crate::world_growth::state_from_dirt(&state("minecraft:stone"), "sky"), None);
}

#[test]
fn grass_bonemeal_plants_fern_or_tall_grass_but_never_a_sapling() {
    registered();
    let grass = state("tconstruct:earth_earth_slime_grass");
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], grass.clone());
    let check = BonemealContext { state: grass.clone(), pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::can_bonemeal("tconstruct:earth_earth_slime_grass", &check), Some(true));
    let mut c = TickContext { state: grass.clone(), pos: [0, 64, 0], world: &mut view };
    assert_eq!(pluto_lib::fixtures::bonemeal_success("tconstruct:earth_earth_slime_grass", &mut c), Some(true));

    // Roll 10 of 32: tall grass. Roll 3: fern. Roll 0: still a fern — the grass passes includeSapling=false.
    for (roll, plant) in [(10, "earth_slime_tall_grass"), (3, "earth_slime_fern"), (0, "earth_slime_fern")] {
        let mut view = MemoryBlockView::default();
        view.blocks.insert([0, 64, 0], grass.clone());
        view.rolls = vec![roll];
        let mut c = TickContext { state: grass.clone(), pos: [0, 64, 0], world: &mut view };
        pluto_lib::fixtures::on_bonemeal("tconstruct:earth_earth_slime_grass", &mut c);
        assert_eq!(view.blocks[&[0, 65, 0]], state(&format!("tconstruct:{plant}")), "roll {roll}");
    }

    // Covered grass is not a valid target.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], grass.clone());
    view.blocks.insert([0, 65, 0], state("minecraft:stone"));
    let check = BonemealContext { state: grass, pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::can_bonemeal("tconstruct:earth_earth_slime_grass", &check), Some(false));
}

#[test]
fn nylium_bonemeal_spreads_to_dirt_on_the_walk() {
    registered();
    let nylium = state("tconstruct:ichor_sky_slime_grass");
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], nylium.clone());
    view.blocks.insert([0, 65, 0], state("minecraft:stone")); // the walk starts here; nothing plants on stone
    view.blocks.insert([1, 64, 0], state("tconstruct:earth_slime_dirt"));
    // Iteration 16 takes one step: +1 x, 0 y, 0 z, landing over the dirt, which the spread converts.
    view.rolls = vec![2, 1, 0, 1];
    let mut c = TickContext { state: nylium, pos: [0, 64, 0], world: &mut view };
    pluto_lib::fixtures::on_bonemeal("tconstruct:ichor_sky_slime_grass", &mut c);
    assert_eq!(view.blocks[&[1, 64, 0]], state("tconstruct:ichor_earth_slime_grass"));
    assert_eq!(view.writes.len(), 1, "the conversion is the only write: {:?}", view.writes);
}

#[test]
fn sapling_advances_then_grows_its_tree_feature() {
    registered();
    let sapling = state("tconstruct:earth_slime_sapling");
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], sapling.clone());
    view.rolls = vec![0]; // one chance in seven, taken
    tick(&mut view, "earth_slime_sapling", [0, 64, 0]);
    assert_eq!(view.writes, vec![([0, 64, 0], state("tconstruct:earth_slime_sapling[stage=1]"), 4)]);

    // Stage 1: the position is left as air (flags 4) and the earth tree feature is placed.
    let grown = state("tconstruct:earth_slime_sapling[stage=1]");
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], grown);
    view.rolls = vec![0];
    tick(&mut view, "earth_slime_sapling", [0, 64, 0]);
    assert_eq!(view.features, vec![("tconstruct:earth_slime_tree".to_string(), [0, 64, 0])]);
    assert!(!view.blocks.contains_key(&[0, 64, 0]), "the legacy block is air");

    // Darkness stops a sapling before the chance is even drawn (the propagule below does not care).
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], sapling);
    view.light.insert([0, 65, 0], 0);
    view.rolls = vec![0];
    tick(&mut view, "earth_slime_sapling", [0, 64, 0]);
    assert!(view.writes.is_empty() && view.rolls.len() == 1, "no draw in the dark");

    // Bonemeal: always a valid target, success under 0.45, performing advances the tree.
    let mut view = MemoryBlockView::default();
    let check = BonemealContext { state: state("tconstruct:sky_slime_sapling"), pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::can_bonemeal("tconstruct:sky_slime_sapling", &check), Some(true));
    view.floats = vec![0.3];
    let mut c = TickContext { state: state("tconstruct:sky_slime_sapling"), pos: [0, 64, 0], world: &mut view };
    assert_eq!(pluto_lib::fixtures::bonemeal_success("tconstruct:sky_slime_sapling", &mut c), Some(true));
    view.floats = vec![0.6];
    let mut c = TickContext { state: state("tconstruct:sky_slime_sapling"), pos: [0, 64, 0], world: &mut view };
    assert_eq!(pluto_lib::fixtures::bonemeal_success("tconstruct:sky_slime_sapling", &mut c), Some(false));
}

#[test]
fn ender_grower_prefers_the_tall_tree_and_a_failed_feature_restores_the_sapling() {
    let mut view = MemoryBlockView::default();
    view.floats = vec![0.5];
    crate::world_growth::advance_tree(&mut view, &state("tconstruct:ender_slime_sapling[stage=1]"), [0, 64, 0], "ender");
    assert_eq!(view.features, vec![("tconstruct:ender_slime_tree_tall".to_string(), [0, 64, 0])]);

    let mut view = MemoryBlockView::default();
    view.floats = vec![0.9];
    crate::world_growth::advance_tree(&mut view, &state("tconstruct:ender_slime_sapling[stage=1]"), [0, 64, 0], "ender");
    assert_eq!(view.features, vec![("tconstruct:ender_slime_tree".to_string(), [0, 64, 0])]);

    // A waterlogged sapling leaves water behind, not air.
    let mut view = MemoryBlockView::default();
    crate::world_growth::advance_tree(&mut view, &state("tconstruct:earth_slime_sapling[stage=1,waterlogged=true]"), [0, 64, 0], "earth");
    assert_eq!(view.blocks[&[0, 64, 0]], state("minecraft:water"));
}

#[test]
fn fungus_bonemeal_needs_slimy_soil_and_places_its_feature() {
    registered();
    let fungus = state("tconstruct:ichor_slime_sapling");
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], fungus.clone());
    view.blocks.insert([0, 63, 0], state("tconstruct:earth_slime_dirt"));
    {
        let check = BonemealContext { state: fungus.clone(), pos: [0, 64, 0], view: &view };
        assert_eq!(pluto_lib::fixtures::can_bonemeal("tconstruct:ichor_slime_sapling", &check), Some(false), "no soil tag yet");
    }
    view.tags.insert([0, 63, 0], BTreeSet::from(["tconstruct:slimy_soil".to_string()]));
    let check = BonemealContext { state: fungus.clone(), pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::can_bonemeal("tconstruct:ichor_slime_sapling", &check), Some(true));

    view.floats = vec![0.39];
    let mut c = TickContext { state: fungus.clone(), pos: [0, 64, 0], world: &mut view };
    assert_eq!(pluto_lib::fixtures::bonemeal_success("tconstruct:ichor_slime_sapling", &mut c), Some(true));
    view.floats = vec![0.4];
    let mut c = TickContext { state: fungus.clone(), pos: [0, 64, 0], world: &mut view };
    assert_eq!(pluto_lib::fixtures::bonemeal_success("tconstruct:ichor_slime_sapling", &mut c), Some(false), "0.4 is not under 0.4");

    let mut c = TickContext { state: fungus, pos: [0, 64, 0], world: &mut view };
    pluto_lib::fixtures::on_bonemeal("tconstruct:ichor_slime_sapling", &mut c);
    assert_eq!(view.features, vec![("tconstruct:ichor_slime_fungus".to_string(), [0, 64, 0])]);
    assert!(view.writes.is_empty(), "the fungus itself is not removed");
}

#[test]
fn propagule_ages_hanging_and_grows_standing_in_the_dark() {
    registered();
    // Hanging: ages by one per tick (flags 2) until mature.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:ender_slime_sapling[hanging=true,age=2]"));
    tick(&mut view, "ender_slime_sapling", [0, 64, 0]);
    assert_eq!(view.writes, vec![([0, 64, 0], state("tconstruct:ender_slime_sapling[hanging=true,age=3]"), 2)]);
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:ender_slime_sapling[hanging=true,age=4]"));
    tick(&mut view, "ender_slime_sapling", [0, 64, 0]);
    assert!(view.writes.is_empty(), "mature hanging propagules rest");

    // Standing: advances like a sapling but with no light requirement.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:ender_slime_sapling[stage=0]"));
    view.light.insert([0, 65, 0], 0);
    view.rolls = vec![0];
    tick(&mut view, "ender_slime_sapling", [0, 64, 0]);
    assert_eq!(view.writes, vec![([0, 64, 0], state("tconstruct:ender_slime_sapling[stage=1]"), 4)]);

    // Bonemeal validity and success: hanging ones answer by age alone and never draw a float.
    let mut view = MemoryBlockView::default();
    for (age, valid) in [("3", true), ("4", false)] {
        let hanging = state(&format!("tconstruct:ender_slime_sapling[hanging=true,age={age}]"));
        let check = BonemealContext { state: hanging, pos: [0, 64, 0], view: &view };
        assert_eq!(pluto_lib::fixtures::can_bonemeal("tconstruct:ender_slime_sapling", &check), Some(valid), "age {age}");
    }
    view.floats = vec![0.9];
    let mut c = TickContext { state: state("tconstruct:ender_slime_sapling[hanging=true,age=3]"), pos: [0, 64, 0], world: &mut view };
    assert_eq!(pluto_lib::fixtures::bonemeal_success("tconstruct:ender_slime_sapling", &mut c), Some(true));
    assert_eq!(view.floats.len(), 1, "the hanging success answer draws nothing");
    // Performing on a hanging propagule ages it (flags 2).
    let mut c = TickContext { state: state("tconstruct:ender_slime_sapling[hanging=true,age=3]"), pos: [0, 64, 0], world: &mut view };
    pluto_lib::fixtures::on_bonemeal("tconstruct:ender_slime_sapling", &mut c);
    assert_eq!(view.writes, vec![([0, 64, 0], state("tconstruct:ender_slime_sapling[hanging=true,age=4]"), 2)]);
}

#[test]
fn propagule_survival_and_placement_match_the_original() {
    registered();
    // Hanging: slimy leaves above or nothing.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 65, 0], state("tconstruct:ender_slime_leaves"));
    view.tags.insert([0, 65, 0], BTreeSet::from(["tconstruct:slimy_leaves".to_string()]));
    let hanging = state("tconstruct:ender_slime_sapling[hanging=true,age=4]");
    let c = SurviveContext { state: hanging.clone(), pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::survives("tconstruct:ender_slime_sapling", &c), Some(true));
    view.tags.clear();
    let c = SurviveContext { state: hanging, pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::survives("tconstruct:ender_slime_sapling", &c), Some(false));

    // Standing: the sapling ground set — slime dirt and slime grass hold it, stone does not.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 63, 0], state("tconstruct:sky_slime_dirt"));
    let standing = state("tconstruct:ender_slime_sapling[age=4]");
    let c = SurviveContext { state: standing.clone(), pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::survives("tconstruct:ender_slime_sapling", &c), Some(true));
    view.blocks.insert([0, 63, 0], state("tconstruct:earth_earth_slime_grass"));
    let c = SurviveContext { state: standing.clone(), pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::survives("tconstruct:ender_slime_sapling", &c), Some(true));
    view.blocks.insert([0, 63, 0], state("minecraft:stone"));
    let c = SurviveContext { state: standing, pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::survives("tconstruct:ender_slime_sapling", &c), Some(false));

    // Placement: mature, and waterlogged when placed in water.
    let view = MemoryBlockView::default();
    let c = PlacementContext { state: state("tconstruct:ender_slime_sapling"), pos: [0, 64, 0], face: Direction::Up, yaw: 0.0, pitch: 0.0,
                               sneaking: false, in_water: true, item: "tconstruct:ender_slime_sapling".to_string(), replace_clicked: false, view: &view };
    assert_eq!(pluto_lib::fixtures::placement("tconstruct:ender_slime_sapling", &c),
               Placement::State(state("tconstruct:ender_slime_sapling[age=4,waterlogged=true]")));
}

#[test]
fn propagule_leaves_bonemeal_places_a_hanging_propagule_below() {
    registered();
    let leaves = state("tconstruct:ender_slime_leaves");
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], leaves.clone());
    let check = BonemealContext { state: leaves.clone(), pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::can_bonemeal("tconstruct:ender_slime_leaves", &check), Some(true));
    let mut c = TickContext { state: leaves, pos: [0, 64, 0], world: &mut view };
    pluto_lib::fixtures::on_bonemeal("tconstruct:ender_slime_leaves", &mut c);
    assert_eq!(view.writes, vec![([0, 63, 0], state("tconstruct:ender_slime_sapling[age=0,hanging=true]"), 2)]);

    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 63, 0], state("minecraft:stone"));
    let check = BonemealContext { state: state("tconstruct:ender_slime_leaves"), pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::can_bonemeal("tconstruct:ender_slime_leaves", &check), Some(false));
}

#[test]
fn vine_top_piece_gains_sides_against_support_or_under_leaves() {
    registered();
    // A stone to the north: only the north side can be gained, on its one-in-six draw.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:sky_slime_vine[stage=start]"));
    view.blocks.insert([0, 64, -1], state("minecraft:stone"));
    view.rolls = vec![0];
    tick(&mut view, "sky_slime_vine", [0, 64, 0]);
    assert_eq!(view.blocks[&[0, 64, 0]], state("tconstruct:sky_slime_vine[north=true,stage=start]"));
    assert_eq!(view.rolls.len(), 0, "one draw for the one eligible side");

    // Under slimy leaves every side is eligible: north and south take their draws, east and west refuse.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:ender_slime_vine[stage=start]"));
    view.blocks.insert([0, 65, 0], state("tconstruct:ender_slime_leaves"));
    view.tags.insert([0, 65, 0], BTreeSet::from(["tconstruct:slimy_leaves".to_string()]));
    view.rolls = vec![0, 1, 0, 1];
    tick(&mut view, "ender_slime_vine", [0, 64, 0]);
    assert_eq!(view.blocks[&[0, 64, 0]], state("tconstruct:ender_slime_vine[north=true,south=true,stage=start]"));
}

#[test]
fn vine_grows_down_keeping_its_sides_and_advancing_when_free() {
    registered();
    // Supported on the north: the piece below copies the state, minus the up face, at the same stage.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:sky_slime_vine[north=true,stage=start]"));
    view.blocks.insert([0, 64, -1], state("minecraft:stone"));
    view.rolls = vec![0]; // the one-in-four growth draw
    tick(&mut view, "sky_slime_vine", [0, 64, 0]);
    assert_eq!(view.blocks[&[0, 63, 0]], state("tconstruct:sky_slime_vine[north=true,stage=start,up=false]"));

    // Free floating with two of its own stage above: the new piece advances a stage with no coin flip.
    let mut view = MemoryBlockView::default();
    for y in [64, 65, 66] {
        view.blocks.insert([0, y, 0], state("tconstruct:sky_slime_vine[north=true,stage=start]"));
    }
    view.rolls = vec![0];
    tick(&mut view, "sky_slime_vine", [0, 64, 0]);
    assert_eq!(view.blocks[&[0, 63, 0]], state("tconstruct:sky_slime_vine[north=true,stage=middle,up=false]"));

    // An ended vine does not grow.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:sky_slime_vine[north=true,stage=end]"));
    view.blocks.insert([0, 64, -1], state("minecraft:stone"));
    view.rolls = vec![0];
    tick(&mut view, "sky_slime_vine", [0, 64, 0]);
    assert!(view.writes.is_empty());
}

#[test]
fn vine_survival_shape_and_placement_follow_its_connections() {
    registered();
    let vine = state("tconstruct:sky_slime_vine[north=true,stage=start]");
    // Survival: the north side holds against stone, and is lost against air.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, -1], state("minecraft:stone"));
    let c = SurviveContext { state: vine.clone(), pos: [0, 64, 0], view: &view };
    assert_eq!(pluto_lib::fixtures::survives("tconstruct:sky_slime_vine", &c), Some(true));
    let empty = MemoryBlockView::default();
    let c = SurviveContext { state: vine.clone(), pos: [0, 64, 0], view: &empty };
    assert_eq!(pluto_lib::fixtures::survives("tconstruct:sky_slime_vine", &c), Some(false));

    // A side update with the support gone removes the vine; a downward update leaves it untouched.
    let c = ShapeUpdateContext::new(vine.clone(), [0, 64, 0], Direction::North, [0, 64, -1], Some(state("minecraft:air")), &empty);
    assert_eq!(pluto_lib::fixtures::shape_update("tconstruct:sky_slime_vine", &c), Some(state("minecraft:air")));
    let c = ShapeUpdateContext::new(vine.clone(), [0, 64, 0], Direction::Down, [0, 63, 0], None, &empty);
    assert_eq!(pluto_lib::fixtures::shape_update("tconstruct:sky_slime_vine", &c), Some(vine.clone()));

    // Placement against a stone to the south takes the south face; with no support it refuses, and into
    // an existing vine it adds the new face to the carried ones.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 1], state("minecraft:stone"));
    let c = PlacementContext { state: state("tconstruct:sky_slime_vine[stage=start]"), pos: [0, 64, 0], face: Direction::North, yaw: 0.0, pitch: 0.0,
                               sneaking: false, in_water: false, item: "tconstruct:sky_slime_vine".to_string(), replace_clicked: false, view: &view };
    assert_eq!(pluto_lib::fixtures::placement("tconstruct:sky_slime_vine", &c),
               Placement::State(state("tconstruct:sky_slime_vine[south=true,stage=start]")));
    let c = PlacementContext { state: state("tconstruct:sky_slime_vine[stage=start]"), pos: [0, 64, 0], face: Direction::North, yaw: 0.0, pitch: 0.0,
                               sneaking: false, in_water: false, item: "tconstruct:sky_slime_vine".to_string(), replace_clicked: false, view: &empty };
    assert_eq!(pluto_lib::fixtures::placement("tconstruct:sky_slime_vine", &c), Placement::Refuse);
    view.blocks.insert([0, 64, 0], vine.clone());
    let c = PlacementContext { state: state("tconstruct:sky_slime_vine[stage=start]"), pos: [0, 64, 0], face: Direction::North, yaw: 0.0, pitch: 0.0,
                               sneaking: false, in_water: false, item: "tconstruct:sky_slime_vine".to_string(), replace_clicked: false, view: &view };
    assert_eq!(pluto_lib::fixtures::placement("tconstruct:sky_slime_vine", &c),
               Placement::State(state("tconstruct:sky_slime_vine[north=true,south=true,stage=start]")));
}

#[test]
fn budding_crystals_grow_buds_outward_and_upward_in_size() {
    registered();
    // One in five, direction up into air: a small bud, facing up, dry.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:budding_earth_slime_crystal"));
    view.rolls = vec![0, 1];
    tick(&mut view, "budding_earth_slime_crystal", [0, 64, 0]);
    assert_eq!(view.blocks[&[0, 65, 0]], state("tconstruct:small_earth_slime_crystal_bud[facing=up,waterlogged=false]"));

    // Four in five: nothing happens and no direction is drawn.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:budding_earth_slime_crystal"));
    view.rolls = vec![1];
    tick(&mut view, "budding_earth_slime_crystal", [0, 64, 0]);
    assert!(view.writes.is_empty() && view.rolls.is_empty());

    // An existing bud grows one size in place, keeping the direction it faces.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:budding_sky_slime_crystal"));
    view.blocks.insert([0, 64, -1], state("tconstruct:medium_sky_slime_crystal_bud[facing=north,waterlogged=false]"));
    view.rolls = vec![0, 2]; // north
    tick(&mut view, "budding_sky_slime_crystal", [0, 64, 0]);
    assert_eq!(view.blocks[&[0, 64, -1]], state("tconstruct:large_sky_slime_crystal_bud[facing=north,waterlogged=false]"));

    // A large bud becomes the cluster; a cluster grows no further.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:budding_ichor_slime_crystal"));
    view.blocks.insert([1, 64, 0], state("tconstruct:large_ichor_slime_crystal_bud[facing=east,waterlogged=false]"));
    view.rolls = vec![0, 5]; // east
    tick(&mut view, "budding_ichor_slime_crystal", [0, 64, 0]);
    assert_eq!(view.blocks[&[1, 64, 0]], state("tconstruct:ichor_slime_crystal_cluster[facing=east,waterlogged=false]"));
    view.blocks.insert([0, 65, 0], state("tconstruct:ichor_slime_crystal_cluster[facing=up,waterlogged=false]"));
    view.rolls = vec![0, 1];
    tick(&mut view, "budding_ichor_slime_crystal", [0, 64, 0]);
    assert_eq!(view.writes.len(), 1, "only the cluster conversion was written");

    // Into water: the bud keeps the water.
    let mut view = MemoryBlockView::default();
    view.blocks.insert([0, 64, 0], state("tconstruct:budding_ender_slime_crystal"));
    view.blocks.insert([0, 63, 0], state("minecraft:water"));
    view.water.insert([0, 63, 0]);
    view.rolls = vec![0, 0]; // down
    tick(&mut view, "budding_ender_slime_crystal", [0, 64, 0]);
    assert_eq!(view.blocks[&[0, 63, 0]], state("tconstruct:small_ender_slime_crystal_bud[facing=down,waterlogged=true]"));
}

#[test]
fn seed_items_convert_vines_first_then_dirt() {
    for (item, foliage) in [("earth", "earth"), ("sky", "sky"), ("ichor", "ichor"), ("ender", "ender"), ("blood", "blood")] {
        assert_eq!(crate::world_growth::seed_foliage(&format!("tconstruct:{item}_slime_grass_seeds")), Some(foliage));
    }
    assert_eq!(crate::world_growth::seed_foliage("tconstruct:pattern"), None);

    // Dirt takes the seed's foliage on the clicked dirt type, with the slime place sound.
    let (placed, sound) = crate::world_growth::seed_conversion(&state("minecraft:dirt"), "earth").expect("dirt converts");
    assert_eq!(placed, state("tconstruct:earth_vanilla_slime_grass"));
    assert_eq!(sound, "minecraft:block.slime_block.place");
    let (placed, _) = crate::world_growth::seed_conversion(&state("tconstruct:sky_slime_dirt"), "blood").expect("slime dirt converts");
    assert_eq!(placed, state("tconstruct:blood_sky_slime_grass"));
    assert_eq!(crate::world_growth::seed_conversion(&state("minecraft:stone"), "sky"), None);

    // A vanilla vine becomes the seed's slime vine at the start stage, copying its true faces; the earth
    // seeds have no vine, so a vine click falls through to the dirt check and passes.
    let vine = state("minecraft:vine[north=true,up=true]");
    let (placed, sound) = crate::world_growth::seed_conversion(&vine, "sky").expect("vine converts");
    assert_eq!(placed, state("tconstruct:sky_slime_vine[north=true,stage=start,up=true]"));
    assert_eq!(sound, "minecraft:block.grass.place");
    assert_eq!(crate::world_growth::seed_conversion(&vine, "earth"), None);
    let (placed, _) = crate::world_growth::seed_conversion(&vine, "ender").expect("ender vine converts");
    assert_eq!(placed, state("tconstruct:ender_slime_vine[north=true,stage=start,up=true]"));
}

#[test]
fn growth_declarations_carry_the_original_random_ticks() {
    // randomTicks() in the original's registrations: the grass (both kinds), the saplings, the propagule
    // and the vines declare it; the fungi do not.
    for name in ["earth_earth_slime_grass", "ichor_sky_slime_grass", "earth_slime_sapling", "sky_slime_sapling", "ender_slime_sapling", "sky_slime_vine", "ender_slime_vine"] {
        assert_eq!(declaration(name)["properties"]["random_ticks"].as_bool(), Some(true), "{name}");
    }
    for name in ["ichor_slime_sapling", "blood_slime_sapling"] {
        assert!(declaration(name)["properties"].get("random_ticks").is_none(), "{name} declares no random ticks");
    }
    // The seed items are declared at last.
    let declared = crate::casting_loop::declared();
    for foliage in ["earth", "sky", "ichor", "ender", "blood"] {
        assert!(declared.contains(&format!("tconstruct:{foliage}_slime_grass_seeds")), "{foliage} seeds declared");
    }
}

#[test]
fn slime_dirt_sustains_through_the_supports_vegetation_tag() {
    // SlimeDirtBlock.canSustainPlant has no PlutoLib callback (see world_growth's module note); the port's
    // stand-in is the host's vegetation support tag, and it must list exactly the four slime dirts, which
    // are the original's SLIME/PLAINS sustainers.
    let path = content().join("minecraft/tags/block/supports_vegetation.json");
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|_| panic!("tag {path:?}"))).unwrap();
    let values: BTreeSet<&str> = data["values"].as_array().unwrap().iter().map(|value| value.as_str().unwrap()).collect();
    assert_eq!(values, BTreeSet::from(["tconstruct:earth_slime_dirt", "tconstruct:sky_slime_dirt", "tconstruct:ichor_slime_dirt", "tconstruct:ender_slime_dirt"]));
}
