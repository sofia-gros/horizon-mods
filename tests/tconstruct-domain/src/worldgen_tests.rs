//! Behaviour tests for the W4 worldgen unit: the slime tree and fungus feature codecs and generators, the
//! ender tree's leave-vine decorator and root variants, and the slime island structure and piece, all
//! dispatched through PlutoLib's worldgen registries against the content the importer shipped.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use pluto_lib::{
    BlockEditor, BlockPos, BlockState, BlockView, Direction, FeatureContext, FeatureTypeRegistry, Identifier,
    MemoryBlockView, RootPlacementContext, RootPlacementTypeRegistry, RootPlacementWorld, StructureContext,
    StructureMarkerContext, StructurePieceTypeRegistry, StructurePieceWorld, StructureTypeRegistry,
    StructureWorld, TreeDecoratorContext, TreeDecoratorTypeRegistry,
};
use serde_json::{json, Value};

fn content(relative: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tconstruct/content/data/tconstruct")
        .join(relative);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn content_exists(relative: &str) -> bool {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tconstruct/content/data/tconstruct")
        .join(relative)
        .exists()
}

fn id(path: &str) -> Identifier {
    Identifier::parse(&format!("tconstruct:{path}")).unwrap()
}

fn state(text: &str) -> BlockState {
    BlockState::parse(text).unwrap()
}

fn generate(world: &mut MemoryBlockView, feature: &Value, origin: BlockPos) -> Option<bool> {
    crate::worldgen::register();
    let kind = Identifier::parse(feature["type"].as_str().unwrap()).unwrap();
    FeatureTypeRegistry::generate(&kind, feature, &mut FeatureContext { world, origin })
        .unwrap_or_else(|error| panic!("generate {}: {error}", feature["type"]))
}

const CUSTOM_FEATURES: [&str; 9] = [
    "earth_slime_tree",
    "earth_slime_island_tree",
    "sky_slime_tree",
    "sky_slime_island_tree",
    "ender_slime_tree",
    "ender_slime_tree_tall",
    "blood_slime_fungus",
    "blood_slime_island_fungus",
    "ichor_slime_fungus",
];

const GEODES: [&str; 4] = ["earth_geode", "sky_geode", "ichor_geode", "ender_geode"];

#[test]
fn shipped_features_decode_and_host_types_stay_native() {
    crate::worldgen::register();
    // The port's own feature types decode and generate (failing cleanly on an empty world); the vanilla
    // geode and tree feature types stay with the host and report no port codec.
    for name in CUSTOM_FEATURES.into_iter().chain(GEODES) {
        let feature = content(&format!("worldgen/feature/{name}.json"));
        let mut world = MemoryBlockView::default();
        let placed = generate(&mut world, &feature, [0, 64, 0]);
        if feature["type"].as_str().unwrap().starts_with("tconstruct:") {
            assert_eq!(placed, Some(false), "{name} should decode and refuse an empty world");
        } else {
            assert_eq!(placed, None, "{name} is a host feature type");
        }
    }
    // The ender trees' custom components decode through their own registries.
    for name in ["ender_slime_tree", "ender_slime_tree_tall"] {
        let feature = content(&format!("worldgen/feature/{name}.json"));
        let mut saw_vines = false;
        for decorator in feature["decorators"].as_array().unwrap() {
            let kind = Identifier::parse(decorator["type"].as_str().unwrap()).unwrap();
            let decoded = TreeDecoratorTypeRegistry::decode(&kind, decorator).unwrap();
            if decorator["type"] == "tconstruct:leave_vines" {
                saw_vines = true;
                assert!(decoded.is_some(), "{name} leave_vines should decode");
            } else {
                assert!(decoded.is_none(), "{name} decorator {} stays native", decorator["type"]);
            }
        }
        assert!(saw_vines, "{name} should carry the leave_vines decorator");
        let root = &feature["root_placer"];
        let kind = Identifier::parse(root["type"].as_str().unwrap()).unwrap();
        let decoded = RootPlacementTypeRegistry::decode(&kind, root).unwrap();
        assert!(decoded.is_some(), "{name} root placer should decode");
        let bindings = decoded.unwrap().bindings().clone();
        assert_eq!(bindings.block_sets.len(), 3, "three root variants");
        assert_eq!(bindings.state_providers.len(), 3);
    }
}

#[test]
fn slime_tree_grows_a_trunk_and_canopy_on_slimy_soil() {
    let feature = content("worldgen/feature/earth_slime_tree.json");
    let origin = [0, 64, 0];
    let below = [0, 63, 0];
    let mut world = MemoryBlockView::default();
    world.blocks.insert(below, state("tconstruct:earth_slime_dirt"));
    world.tags.insert(below, ["tconstruct:slimy_soil".to_string(), "minecraft:dirt".to_string()].into_iter().collect());
    world.rolls = vec![2]; // height = 4 + 2
    assert_eq!(generate(&mut world, &feature, origin), Some(true));
    for i in 0..6 {
        assert_eq!(
            world.block_state([0, 64 + i, 0]).map(|placed| placed.block.to_string()),
            Some("tconstruct:greenheart_log".to_string()),
            "trunk at +{i}"
        );
    }
    // The block below the tree becomes plain dirt, and the canopy crowns the trunk.
    assert_eq!(
        world.block_state(below).map(|placed| placed.block.to_string()),
        Some("minecraft:dirt".to_string())
    );
    assert_eq!(
        world.block_state([0, 70, 0]).map(|placed| placed.block.to_string()),
        Some("tconstruct:earth_slime_leaves".to_string())
    );
    assert_eq!(
        world.block_state([3, 67, 0]).map(|placed| placed.block.to_string()),
        Some("tconstruct:earth_slime_leaves".to_string())
    );

    // Without slimy soil below, the feature refuses and writes nothing.
    let mut bare = MemoryBlockView::default();
    assert_eq!(generate(&mut bare, &feature, origin), Some(false));
    assert!(bare.writes.is_empty());
}

#[test]
fn slime_tree_with_vines_hangs_randomized_vines() {
    let feature = content("worldgen/feature/sky_slime_island_tree.json");
    let origin = [0, 64, 0];
    let mut world = MemoryBlockView::default();
    world.blocks.insert([0, 63, 0], state("tconstruct:sky_slime_dirt"));
    world.tags.insert([0, 63, 0], ["tconstruct:slimy_soil".to_string(), "minecraft:dirt".to_string()].into_iter().collect());
    // Height roll, the no-double roll, then eight randomized vines (count roll, one face roll each).
    let mut rolls = vec![0, 3];
    for face in [0, 1, 2, 3, 0, 1, 2, 3] {
        rolls.push(0);
        rolls.push(face);
    }
    world.rolls = rolls;
    assert_eq!(generate(&mut world, &feature, origin), Some(true));
    assert_eq!(
        world.block_state(origin).map(|placed| placed.block.to_string()),
        Some("tconstruct:skyroot_log".to_string())
    );
    let vines: Vec<&BlockState> = world
        .blocks
        .values()
        .filter(|placed| placed.block.to_string() == "tconstruct:sky_slime_vine")
        .collect();
    assert!(!vines.is_empty(), "the canopy should hang vines");
    assert!(vines.iter().any(|vine| vine.property("up") == Some("true")));
    assert!(vines.iter().all(|vine| ["north", "east", "south", "west"]
        .iter()
        .filter(|face| vine.property(face) == Some("true"))
        .count()
        == 1));
}

#[test]
fn slime_fungus_grows_a_stem_and_hat_on_its_ground_tag() {
    let feature = content("worldgen/feature/blood_slime_island_fungus.json");
    let origin = [0, 64, 0];
    let mut world = MemoryBlockView::default();
    world.generator_depth = Some(384);
    world.blocks.insert([0, 63, 0], state("tconstruct:blood_slime_nylium"));
    world.tags.insert([0, 63, 0], ["tconstruct:slimy_nylium".to_string()].into_iter().collect());
    // Height 4, no doubling, hat layers roll, then a radius roll per hat layer.
    world.rolls = vec![0, 1, 0, 2, 2, 2, 2, 2];
    world.floats = vec![0.5]; // not wide; later floats default to 0.0
    assert_eq!(generate(&mut world, &feature, origin), Some(true));
    // The hat may overwrite the stem's top block, as the vanilla feature does; the lower stem is safe.
    for i in 0..3 {
        assert_eq!(
            world.block_state([0, 64 + i, 0]).map(|placed| placed.block.to_string()),
            Some("tconstruct:bloodshroom_log".to_string()),
            "stem at +{i}"
        );
    }
    assert!(
        world.blocks.values().any(|placed| placed.block.to_string() == "tconstruct:blood_slime_leaves"),
        "the hat should be built from the configured leaf state"
    );

    // The wrong ground tag refuses the placement.
    let mut bare = MemoryBlockView::default();
    bare.generator_depth = Some(384);
    assert_eq!(generate(&mut bare, &feature, origin), Some(false));
    assert!(bare.writes.is_empty());
}

// ---------------------------------------------------------------------------
// Slime islands

struct FakeStructureWorld {
    center: Option<BlockPos>,
    valid: bool,
    heights: BTreeMap<(i32, i32), i32>,
    depth: i32,
    sea: i32,
    rolls: Vec<i32>,
}

impl StructureWorld for FakeStructureWorld {
    fn top_of_chunk_center(&mut self, _chunk: [i32; 2]) -> Result<Option<BlockPos>, String> {
        Ok(self.center)
    }

    fn is_valid_biome(&mut self, _pos: BlockPos) -> Result<bool, String> {
        Ok(self.valid)
    }

    fn first_occupied_height(&mut self, x: i32, z: i32) -> Result<i32, String> {
        Ok(*self.heights.get(&(x, z)).unwrap_or(&64))
    }

    fn generation_depth(&self) -> i32 {
        self.depth
    }

    fn sea_level(&self) -> i32 {
        self.sea
    }

    fn random_int(&mut self, bound: i32) -> Result<i32, String> {
        Ok(if self.rolls.is_empty() { 0 } else { self.rolls.remove(0) }.rem_euclid(bound))
    }
}

fn plan(world: &mut FakeStructureWorld, structure: &Value) -> Option<pluto_lib::StructureStart> {
    crate::worldgen::register();
    let configured = StructureTypeRegistry::decode(&id("island"), structure)
        .unwrap()
        .expect("the island structure type should be registered");
    configured
        .find_generation_point(&mut StructureContext { world, chunk: [0, 0] })
        .unwrap()
}

#[test]
fn island_structure_plans_a_sea_piece_with_drawn_choices() {
    let structure = content("worldgen/structure/earth_slime_island.json");
    let mut world = FakeStructureWorld {
        center: Some([8, 62, 8]),
        valid: true,
        heights: BTreeMap::new(),
        depth: 384,
        sea: 63,
        rolls: vec![2, 1, 2, 1],
    };
    let start = plan(&mut world, &structure).expect("a piece should be planned");
    assert_eq!(start.position, [8, 62, 8]);
    assert_eq!(start.pieces.len(), 1);
    let piece = &start.pieces[0];
    assert_eq!(piece.kind, id("island"));
    assert_eq!(piece.configuration["template"], json!("tconstruct:islands/earth/4x1x6"));
    assert_eq!(piece.configuration["origin"], json!([8, 56, 8])); // sea level 63 - 7
    assert_eq!(piece.configuration["rotation"], json!("clockwise_90"));
    assert_eq!(piece.configuration["mirror"], json!("front_back"));
    assert_eq!(piece.configuration["tree"], json!("tconstruct:sky_slime_island_tree"));

    // An invalid biome plans nothing.
    let mut invalid = FakeStructureWorld {
        center: Some([8, 62, 8]),
        valid: false,
        heights: BTreeMap::new(),
        depth: 384,
        sea: 63,
        rolls: Vec::new(),
    };
    assert!(plan(&mut invalid, &structure).is_none());
}

#[test]
fn island_structure_plans_a_sky_piece_above_the_surface() {
    let structure = content("worldgen/structure/end_slime_island.json");
    let mut heights = BTreeMap::new();
    for corner in [(7, 7), (17, 7), (7, 17), (17, 17)] {
        heights.insert(corner, 40);
    }
    let mut world = FakeStructureWorld {
        center: Some([8, 45, 8]),
        valid: true,
        heights,
        depth: 256,
        sea: 63,
        rolls: vec![0, 0, 10, 0, 5],
    };
    let start = plan(&mut world, &structure).expect("a piece should be planned");
    let piece = &start.pieces[0];
    assert_eq!(piece.configuration["template"], json!("tconstruct:islands/ender/0x1x0"));
    // Lowest corner 40 + 60 + the drawn 10, under the 236 cap.
    assert_eq!(piece.configuration["origin"], json!([8, 110, 8]));
    assert_eq!(piece.configuration["tree"], json!("tconstruct:ender_slime_tree_tall"));
    assert_eq!(piece.configuration["vines"], json!("tconstruct:ender_slime_vine"));
}

struct FakePieceWorld {
    blocks: BTreeMap<BlockPos, BlockState>,
    sturdy: BTreeSet<BlockPos>,
    survive: bool,
    feature_result: bool,
    bools: Vec<bool>,
    rolls: Vec<i32>,
    features: Vec<(String, BlockPos)>,
    writes: Vec<(BlockPos, BlockState, i32)>,
}

impl FakePieceWorld {
    fn new() -> Self {
        FakePieceWorld {
            blocks: BTreeMap::new(),
            sturdy: BTreeSet::new(),
            survive: true,
            feature_result: true,
            bools: Vec::new(),
            rolls: Vec::new(),
            features: Vec::new(),
            writes: Vec::new(),
        }
    }
}

impl StructurePieceWorld for FakePieceWorld {
    fn block_state(&mut self, pos: BlockPos) -> Result<BlockState, String> {
        Ok(self.blocks.get(&pos).cloned().unwrap_or_else(|| state("minecraft:air")))
    }

    fn set_block_state(&mut self, pos: BlockPos, placed: &BlockState, flags: i32) -> Result<bool, String> {
        self.writes.push((pos, placed.clone(), flags));
        if placed.block.to_string() == "minecraft:air" {
            self.blocks.remove(&pos);
        } else {
            self.blocks.insert(pos, placed.clone());
        }
        Ok(true)
    }

    fn face_sturdy(&mut self, pos: BlockPos, _face: Direction) -> Result<bool, String> {
        Ok(self.sturdy.contains(&pos))
    }

    fn can_survive_plant(&mut self, _placed: &BlockState, _pos: BlockPos) -> Result<bool, String> {
        Ok(self.survive)
    }

    fn random_int(&mut self, bound: i32) -> Result<i32, String> {
        Ok(if self.rolls.is_empty() { 0 } else { self.rolls.remove(0) }.rem_euclid(bound))
    }

    fn random_bool(&mut self) -> Result<bool, String> {
        Ok(if self.bools.is_empty() { false } else { self.bools.remove(0) })
    }

    fn place_feature(&mut self, feature: &Identifier, pos: BlockPos) -> Result<bool, String> {
        self.features.push((feature.to_string(), pos));
        Ok(self.feature_result)
    }
}

fn island_piece() -> pluto_lib::ConfiguredStructurePiece {
    crate::worldgen::register();
    let configuration = json!({
        "template": "tconstruct:islands/earth/4x1x6",
        "origin": [8, 56, 8],
        "rotation": "clockwise_90",
        "mirror": "front_back",
        "tree": "tconstruct:sky_slime_island_tree",
        "vines": "tconstruct:sky_slime_vine",
        "grasses": [
            {"data": "tconstruct:earth_slime_tall_grass", "weight": 7},
            {"data": "tconstruct:earth_slime_fern", "weight": 1}
        ]
    });
    StructurePieceTypeRegistry::decode(&id("island"), &configuration)
        .unwrap()
        .expect("the island piece should decode")
}

fn handle(piece: &pluto_lib::ConfiguredStructurePiece, world: &mut FakePieceWorld, state: &mut Value, marker: &str, pos: BlockPos) {
    piece
        .handle_data_marker(&mut StructureMarkerContext { world, marker, position: pos, state })
        .unwrap();
}

#[test]
fn island_piece_tree_marker_places_up_to_three_trees() {
    let piece = island_piece();
    let mut piece_state = piece.initial_state().clone();
    let mut world = FakePieceWorld::new();
    let marker = [10, 60, 10];
    for expected in 1..=3 {
        world.bools = vec![true];
        handle(&piece, &mut world, &mut piece_state, "tconstruct:slime_tree", marker);
        assert_eq!(piece_state["trees_placed"], json!(expected));
    }
    assert_eq!(world.features.len(), 3);
    assert!(world.features.iter().all(|(feature, _)| feature == "tconstruct:sky_slime_island_tree"));
    // The fourth marker is capped: no draw, no feature.
    handle(&piece, &mut world, &mut piece_state, "tconstruct:slime_tree", marker);
    assert_eq!(world.features.len(), 3);
    // A failed coin flip places nothing.
    let fresh = island_piece();
    let mut fresh_state = fresh.initial_state().clone();
    let mut world = FakePieceWorld::new();
    world.bools = vec![false];
    handle(&fresh, &mut world, &mut fresh_state, "tconstruct:slime_tree", marker);
    assert!(world.features.is_empty());
}

#[test]
fn island_piece_grass_and_vine_markers() {
    let marker = [10, 60, 10];
    // Grass: the weighted pick lands on the tall grass and it survives.
    let piece = island_piece();
    let mut piece_state = piece.initial_state().clone();
    let mut world = FakePieceWorld::new();
    world.bools = vec![true];
    world.rolls = vec![6];
    handle(&piece, &mut world, &mut piece_state, "tconstruct:slime_tall_grass", marker);
    assert_eq!(world.writes.len(), 1);
    assert_eq!(world.writes[0].0, marker);
    assert_eq!(world.writes[0].1.block.to_string(), "tconstruct:earth_slime_tall_grass");
    assert_eq!(world.writes[0].2, 2);
    // Grass that cannot survive is skipped.
    let piece = island_piece();
    let mut piece_state = piece.initial_state().clone();
    let mut world = FakePieceWorld::new();
    world.bools = vec![true];
    world.rolls = vec![6];
    world.survive = false;
    handle(&piece, &mut world, &mut piece_state, "tconstruct:slime_tall_grass", marker);
    assert!(world.writes.is_empty());

    // Vine: the north neighbour is sturdy, so the vine attaches there and grows down.
    let piece = island_piece();
    let mut piece_state = piece.initial_state().clone();
    let mut world = FakePieceWorld::new();
    world.sturdy.insert([10, 60, 9]);
    world.bools = vec![true];
    world.rolls = vec![3, 1];
    handle(&piece, &mut world, &mut piece_state, "tconstruct:slime_vine", marker);
    assert_eq!(world.writes[0].0, marker);
    assert_eq!(world.writes[0].1.block.to_string(), "tconstruct:sky_slime_vine");
    assert_eq!(world.writes[0].1.property("north"), Some("true"));
    assert_eq!(world.writes[0].2, 2);
    assert!(
        world.writes.iter().any(|(pos, placed, flags)| *pos == [10, 59, 10]
            && placed.property("up") == Some("false")
            && *flags == 3),
        "the vine should grow below the marker"
    );
}

// ---------------------------------------------------------------------------
// Ender tree components

#[test]
fn leave_vine_decorator_hangs_vines_below_leaves() {
    crate::worldgen::register();
    let feature = content("worldgen/feature/ender_slime_tree.json");
    let entry = feature["decorators"]
        .as_array()
        .unwrap()
        .iter()
        .find(|decorator| decorator["type"] == "tconstruct:leave_vines")
        .unwrap();
    let decorator = TreeDecoratorTypeRegistry::decode(&id("leave_vines"), entry)
        .unwrap()
        .expect("the decorator should decode");
    let mut world = MemoryBlockView::default();
    // South passes the 0.125 probability roll; the other directions fail theirs.
    world.floats = vec![0.1, 0.9, 0.9, 0.9];
    decorator
        .decorate(&mut TreeDecoratorContext { world: &mut world, logs: &[], leaves: &[[0, 70, 0]], roots: &[] })
        .unwrap();
    for y in [70, 69, 68, 67, 66] {
        let placed = world.block_state([0, y, 1]).expect("a hanging vine");
        assert_eq!(placed.block.to_string(), "tconstruct:ender_slime_vine");
        assert_eq!(placed.property("north"), Some("true"));
    }
    assert_eq!(
        world.block_state([0, 65, 1]).map(|placed| placed.block.to_string()),
        Some("minecraft:air".to_string()),
        "hanging stops after four extra segments"
    );
}

struct FakeRoots {
    view: MemoryBlockView,
    sets: Vec<Vec<String>>,
    providers: Vec<Value>,
}

impl BlockView for FakeRoots {
    fn block_state(&self, pos: BlockPos) -> Option<BlockState> {
        self.view.block_state(pos)
    }
    fn is_air(&self, pos: BlockPos) -> bool {
        self.view.is_air(pos)
    }
    fn is_face_sturdy(&self, pos: BlockPos, face: Direction) -> bool {
        self.view.is_face_sturdy(pos, face)
    }
    fn is_in_tag(&self, pos: BlockPos, tag: &str) -> bool {
        self.view.is_in_tag(pos, tag)
    }
    fn brightness(&self, pos: BlockPos) -> u8 {
        self.view.brightness(pos)
    }
    fn light_passes_up(&self, pos: BlockPos) -> bool {
        self.view.light_passes_up(pos)
    }
    fn is_water(&self, pos: BlockPos) -> bool {
        self.view.is_water(pos)
    }
    fn is_fluid_full(&self, pos: BlockPos) -> bool {
        self.view.is_fluid_full(pos)
    }
    fn is_area_loaded(&self, pos: BlockPos, radius: i32) -> bool {
        self.view.is_area_loaded(pos, radius)
    }
    fn is_full_block(&self, pos: BlockPos) -> bool {
        self.view.is_full_block(pos)
    }
    fn build_height(&self) -> [i32; 2] {
        self.view.build_height()
    }
}

impl BlockEditor for FakeRoots {
    fn set_block_state(&mut self, pos: BlockPos, placed: &BlockState) -> bool {
        self.view.set_block_state(pos, placed)
    }
    fn random_int(&mut self, bound: i32) -> i32 {
        self.view.random_int(bound)
    }
    fn random_float(&mut self) -> f32 {
        self.view.random_float()
    }
    fn schedule_tick(&mut self, pos: BlockPos, block: &str, delay: i32) -> bool {
        self.view.schedule_tick(pos, block, delay)
    }
    fn place_feature(&mut self, feature: &str, pos: BlockPos) -> bool {
        self.view.place_feature(feature, pos)
    }
}

impl RootPlacementWorld for FakeRoots {
    fn matches_block_set(&mut self, index: usize, pos: BlockPos) -> Result<bool, String> {
        let current = self.view.block_state(pos).ok_or("no block")?;
        Ok(self.sets[index].iter().any(|member| member == &current.block.to_string()))
    }

    fn sample_state_provider(&mut self, index: usize, _pos: BlockPos) -> Result<BlockState, String> {
        let provider = &self.providers[index];
        let text = provider["state"].as_str().ok_or("provider has no plain state")?;
        BlockState::parse(text).map_err(|error| error.to_string())
    }
}

#[test]
fn root_variants_replace_matching_root_blocks() {
    crate::worldgen::register();
    let feature = content("worldgen/feature/ender_slime_tree.json");
    let placement = RootPlacementTypeRegistry::decode(&id("extra_root_variants"), &feature["root_placer"])
        .unwrap()
        .expect("the root placer should decode");
    let bindings = placement.bindings().clone();
    let sets: Vec<Vec<String>> = bindings
        .block_sets
        .iter()
        .map(|set| set.as_array().unwrap().iter().map(|member| member.as_str().unwrap().to_string()).collect())
        .collect();
    let cases = [
        ("tconstruct:sky_congealed_slime", "tconstruct:sky_enderbark_roots"),
        ("tconstruct:ichor_congealed_slime", "tconstruct:ichor_enderbark_roots"),
        ("tconstruct:ender_congealed_slime", "tconstruct:ender_enderbark_roots"),
    ];
    for (current, expected) in cases {
        let mut world = FakeRoots { view: MemoryBlockView::default(), sets: sets.clone(), providers: bindings.state_providers.clone() };
        world.view.blocks.insert([0, 60, 0], state(current));
        let placed = placement
            .place(&mut RootPlacementContext { world: &mut world, pos: [0, 60, 0] })
            .unwrap();
        assert_eq!(placed.map(|state| state.block.to_string()), Some(expected.to_string()), "{current}");
    }
    // An unmatched block falls through to the host's native root placement.
    let mut world = FakeRoots { view: MemoryBlockView::default(), sets, providers: bindings.state_providers.clone() };
    world.view.blocks.insert([0, 60, 0], state("minecraft:stone"));
    let placed = placement.place(&mut RootPlacementContext { world: &mut world, pos: [0, 60, 0] }).unwrap();
    assert_eq!(placed, None);
}

// ---------------------------------------------------------------------------
// Content completeness

#[test]
fn worldgen_content_is_complete() {
    crate::worldgen::register();
    for name in CUSTOM_FEATURES.into_iter().chain(GEODES) {
        assert!(content_exists(&format!("worldgen/feature/{name}.json")), "feature {name}");
    }
    for name in GEODES {
        assert!(content_exists(&format!("worldgen/placed_feature/{name}.json")), "placed {name}");
        assert!(content_exists(&format!("neoforge/biome_modifier/{name}.json")), "modifier {name}");
    }
    let structures = [
        "blood_island",
        "clay_island",
        "earth_slime_island",
        "end_slime_island",
        "ocean_skyslime_island",
        "sky_slime_island",
    ];
    for name in structures {
        let structure = content(&format!("worldgen/structure/{name}.json"));
        assert_eq!(structure["type"], json!("tconstruct:island"));
        let mut world = FakeStructureWorld {
            center: Some([8, 62, 8]),
            valid: true,
            heights: BTreeMap::new(),
            depth: 384,
            sea: 63,
            rolls: Vec::new(),
        };
        assert!(plan(&mut world, &structure).is_some(), "structure {name} should plan");
    }
    for name in ["end_sky_island", "nether_ocean_island", "overworld_ocean_island", "overworld_sky_island"] {
        let set = content(&format!("worldgen/structure_set/{name}.json"));
        assert_eq!(set["placement"]["type"], json!("minecraft:random_spread"));
        assert!(!set["structures"].as_array().unwrap().is_empty());
    }
    for name in ["blood", "clay", "earthslime", "enderslime", "skyslime"] {
        assert!(content_exists(&format!("tags/worldgen/biome/islands/{name}.json")), "island tag {name}");
    }
    // All 25 island templates ship, gzip-compressed like the originals.
    let mut templates = 0;
    for material in ["blood", "dirt", "earth", "ender", "sky"] {
        for size in ["0x1x0", "2x2x4", "4x1x6", "8x1x11", "11x1x11"] {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tconstruct/content/data/tconstruct")
                .join(format!("structures/islands/{material}/{size}.nbt"));
            let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            assert!(bytes.len() > 100, "template {material}/{size} should be non-trivial");
            assert_eq!(&bytes[..2], &[0x1f, 0x8b], "template {material}/{size} should stay gzipped");
            templates += 1;
        }
    }
    assert_eq!(templates, 25);
}

#[test]
fn feature_configs_only_reference_declared_blocks() {
    fn collect(value: &Value, found: &mut BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    match (key.as_str(), inner) {
                        (_, Value::String(text)) if key == "Name" || key == "state" || key == "block" => {
                            let plain = text.split('[').next().unwrap_or(text);
                            if plain.starts_with("tconstruct:") {
                                found.insert(plain.to_string());
                            }
                        }
                        ("matches", Value::Array(members)) => {
                            for member in members {
                                if let Some(text) = member.as_str() {
                                    if text.starts_with("tconstruct:") {
                                        found.insert(text.to_string());
                                    }
                                }
                            }
                        }
                        _ => collect(inner, found),
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect(item, found);
                }
            }
            _ => {}
        }
    }
    for name in CUSTOM_FEATURES.into_iter().chain(GEODES) {
        let feature = content(&format!("worldgen/feature/{name}.json"));
        let mut found = BTreeSet::new();
        collect(&feature, &mut found);
        assert!(!found.is_empty(), "{name} should reference port blocks");
        for block in found {
            let path = block.strip_prefix("tconstruct:").unwrap();
            assert!(
                content_exists(&format!("horizon/block/{path}.json")),
                "{name} references undeclared block {block}"
            );
        }
    }
}
