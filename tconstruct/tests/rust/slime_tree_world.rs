//! A small in-memory PieceWorld exercises source tree writes without a game.
pub use horizon_world::{nbt, registry, worldgen};
#[path = "../../mixins/tconstruct/source-slime-tree-shape.rs"] mod tconstruct_slime_tree_shape;
#[path = "../../mixins/tconstruct/source-slime-tree-world.rs"] mod tconstruct_slime_tree_world;
#[path = "../../mixins/tconstruct/source-slime-island-markers.rs"] mod tconstruct_slime_island_markers;
#[path = "../../mixins/tconstruct/source-slime-island-marker-world.rs"] mod tconstruct_slime_island_marker_world;
#[path = "../../mixins/tconstruct/source-slime-tree-config.rs"] mod tconstruct_slime_tree_config;
mod tconstruct_island_slime_fungus {
    pub(crate) fn place(_: &mut horizon_world::worldgen::structure::legacy::PieceContext<'_>, _: &[bool], _: i32, _: [i32;3]) -> horizon_world::worldgen::structure::legacy::Result<bool> { Ok(false) }
}
#[path = "../../mixins/tconstruct/source-slime-island-piece.rs"] mod tconstruct_slime_island_piece;

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_core::{position::ChunkPos, random::LegacyRandom};
    use horizon_world::{heightmap::HeightmapType, nbt::Tag, registry::{BlockRegistry, BlockStateId},
        worldgen::{anchor::WorldGenerationContext, feature::FeatureWorld,
                   placement::{PlacementContext, PlacementQueryError},
                   structure::{legacy::{LegacyPiece, PieceContext, PieceRandom}, processor::ProcessorRandom, BoundingBox, Mirror, Rotation, StateTransforms, StructureResources}}};
    use std::{collections::BTreeMap, path::Path};

    struct World { blocks: BTreeMap<[i32; 3], BlockStateId> }
    #[derive(Default)]
    struct FixedRandom { booleans: u32 }
    impl ProcessorRandom for FixedRandom {
        fn float(&mut self) -> f32 { 0.0 }
        fn int(&mut self, _: i32) -> i32 { 0 }
        fn long(&mut self) -> i64 { 0 }
    }
    impl PieceRandom for FixedRandom {
        fn boolean(&mut self) -> bool { self.booleans += 1; true }
        fn double(&mut self) -> f64 { 0.0 }
        fn as_any(&mut self) -> &mut dyn std::any::Any { self }
    }
    impl FeatureWorld for World {
        type Error = PlacementQueryError;
        fn block(&mut self, p: [i32; 3]) -> Result<BlockStateId, Self::Error> { Ok(*self.blocks.get(&p).unwrap_or(&BlockStateId(0))) }
        fn set_block(&mut self, p: [i32; 3], state: BlockStateId, _: i32) -> Result<bool, Self::Error> { self.blocks.insert(p, state); Ok(true) }
    }
    impl PlacementContext for World {
        fn generation(&self) -> WorldGenerationContext { WorldGenerationContext::new(-64, 384, 63) }
        fn min_y(&mut self) -> Result<i32, PlacementQueryError> { Ok(-64) }
        fn max_y(&mut self) -> Result<i32, PlacementQueryError> { Ok(320) }
        fn height(&mut self, _: HeightmapType, _: i32, _: i32) -> Result<i32, PlacementQueryError> { Ok(64) }
        fn block_state(&mut self, p: [i32; 3]) -> Result<BlockStateId, PlacementQueryError> { self.block(p) }
        fn attachment_at(&mut self, p: [i32; 3], _: horizon_core::geometry::Direction) -> Result<bool, PlacementQueryError> {
            Ok(self.block(p)? != BlockStateId(0))
        }
    }

    fn empty_properties() -> &'static BTreeMap<String, String> {
        static EMPTY: std::sync::OnceLock<BTreeMap<String, String>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(BTreeMap::new)
    }

    fn resources_at(dir: &Path) -> StructureResources {
        let mut states = vec![Tag::String("minecraft:air".into()), Tag::String("minecraft:dirt".into()),
            Tag::String("tconstruct:earth_slime_dirt".into()), Tag::String("tconstruct:greenheart_log".into())];
        for distance in 1..=7 {
            states.push(Tag::compound([("id", Tag::String("tconstruct:earth_slime_leaves".into())),
                ("properties", Tag::compound([("distance", Tag::String(distance.to_string().as_str().into()))]))]));
        }
        states.push(Tag::String("tconstruct:earth_slime_tall_grass".into()));
        states.push(Tag::String("tconstruct:earth_slime_fern".into()));
        for stage in ["start", "middle", "end"] {
            for bits in 0..32 {
                let flag = |bit| if bits & bit != 0 { "true" } else { "false" };
                states.push(Tag::compound([("id", Tag::String("tconstruct:sky_slime_vine".into())),
                    ("properties", Tag::compound([
                        ("stage", Tag::String(stage.into())), ("up", Tag::String(flag(1).into())),
                        ("north", Tag::String(flag(2).into())), ("south", Tag::String(flag(4).into())),
                        ("west", Tag::String(flag(8).into())), ("east", Tag::String(flag(16).into())),
                    ]))]));
            }
        }
        states.push(Tag::String("minecraft:short_grass".into()));
        let defaults = Tag::compound([("minecraft:air", Tag::Int(0)), ("minecraft:dirt", Tag::Int(1)),
            ("tconstruct:earth_slime_dirt", Tag::Int(2)), ("tconstruct:greenheart_log", Tag::Int(3)),
            ("tconstruct:earth_slime_leaves", Tag::Int(10)),
            ("tconstruct:earth_slime_tall_grass", Tag::Int(11)), ("tconstruct:earth_slime_fern", Tag::Int(12)),
            ("tconstruct:sky_slime_vine", Tag::Int(13)), ("minecraft:short_grass", Tag::Int(109))]);
        let registry = Tag::compound([("DataVersion", Tag::Int(horizon_world::DATA_VERSION)), ("states", Tag::List(states)), ("defaults", defaults)]);
        let registry = Box::leak(Box::new(BlockRegistry::from_nbt(&registry).unwrap()));
        let tags = BTreeMap::from([
            ("tconstruct:slimy_soil".into(), vec!["tconstruct:earth_slime_dirt".into()]),
            ("minecraft:dirt".into(), vec!["minecraft:dirt".into()]),
            ("minecraft:leaves".into(), vec!["tconstruct:earth_slime_leaves".into()]),
        ]);
        StructureResources::new(registry, StateTransforms::default(), dir, BTreeMap::new(), &tags)
    }

    fn resources() -> StructureResources { resources_at(Path::new("/nonexistent")) }

    #[test]
    fn source_tree_writes_logs_canopy_and_leaf_distances_on_slimy_soil() {
        let resources = resources();
        let mut world = World { blocks: BTreeMap::from([([0, 63, 0], BlockStateId(2))]) };
        let mut random = LegacyRandom::new(0);
        let mut ctx = PieceContext { world: &mut world, resources: &resources, random: &mut random,
            chunk_box: BoundingBox::new([-16, -64, -16], [31, 319, 31]), chunk: ChunkPos::new(0, 0), reference: [0, 64, 0] };
        let cfg = tconstruct_slime_tree_world::TreeConfig { base_height: 4, random_height: 3, double_height: false,
            vines: false, trunk: "tconstruct:greenheart_log", trunk_properties: empty_properties(), leaves: "tconstruct:earth_slime_leaves", leaves_properties: empty_properties(), vine: None, vine_properties: empty_properties() };
        assert!(tconstruct_slime_tree_world::place(&mut ctx, [0, 64, 0], cfg).unwrap());
        let logs = world.blocks.values().filter(|id| **id == BlockStateId(3)).count();
        let leaves: Vec<_> = world.blocks.values().filter(|id| (4..=10).contains(&id.0)).collect();
        assert!((4..=6).contains(&logs));
        assert!(!leaves.is_empty());
        assert!(leaves.iter().any(|id| id.0 == 4)); // distance 1
    }

    #[test]
    fn non_slimy_soil_rejects_tree_without_writes() {
        let resources = resources();
        let mut world = World { blocks: BTreeMap::from([([0, 63, 0], BlockStateId(1))]) };
        let mut random = LegacyRandom::new(0);
        let mut ctx = PieceContext { world: &mut world, resources: &resources, random: &mut random,
            chunk_box: BoundingBox::new([-16, -64, -16], [31, 319, 31]), chunk: ChunkPos::new(0, 0), reference: [0, 64, 0] };
        let cfg = tconstruct_slime_tree_world::TreeConfig { base_height: 4, random_height: 3, double_height: false,
            vines: false, trunk: "tconstruct:greenheart_log", trunk_properties: empty_properties(), leaves: "tconstruct:earth_slime_leaves", leaves_properties: empty_properties(), vine: None, vine_properties: empty_properties() };
        assert!(!tconstruct_slime_tree_world::place(&mut ctx, [0, 64, 0], cfg).unwrap());
        assert_eq!(world.blocks.len(), 1);
    }

    #[test]
    fn missing_tree_state_is_reported_before_any_writes() {
        let resources = resources();
        let mut world = World { blocks: BTreeMap::from([([0, 63, 0], BlockStateId(2))]) };
        let mut random = LegacyRandom::new(0);
        let mut ctx = PieceContext { world: &mut world, resources: &resources, random: &mut random,
            chunk_box: BoundingBox::new([-16, -64, -16], [31, 319, 31]), chunk: ChunkPos::new(0, 0), reference: [0, 64, 0] };
        let cfg = tconstruct_slime_tree_world::TreeConfig { base_height: 4, random_height: 3, double_height: false,
            vines: false, trunk: "tconstruct:missing_log", trunk_properties: empty_properties(), leaves: "tconstruct:earth_slime_leaves", leaves_properties: empty_properties(), vine: None, vine_properties: empty_properties() };
        assert!(matches!(tconstruct_slime_tree_world::place(&mut ctx, [0, 64, 0], cfg), Err(PlacementQueryError::Unavailable)));
        assert_eq!(world.blocks.len(), 1);
    }

    #[test]
    fn missing_provider_property_is_reported_before_any_writes() {
        let resources = resources();
        let mut world = World { blocks: BTreeMap::from([([0, 63, 0], BlockStateId(2))]) };
        let mut random = LegacyRandom::new(0);
        let mut ctx = PieceContext { world: &mut world, resources: &resources, random: &mut random,
            chunk_box: BoundingBox::new([-16, -64, -16], [31, 319, 31]), chunk: ChunkPos::new(0, 0), reference: [0, 64, 0] };
        let props = BTreeMap::from([("stage".to_owned(), "middle".to_owned())]);
        let cfg = tconstruct_slime_tree_world::TreeConfig { base_height: 4, random_height: 3, double_height: false,
            vines: false, trunk: "tconstruct:greenheart_log", trunk_properties: &props,
            leaves: "tconstruct:earth_slime_leaves", leaves_properties: empty_properties(), vine: None, vine_properties: empty_properties() };
        assert!(matches!(tconstruct_slime_tree_world::place(&mut ctx, [0, 64, 0], cfg), Err(PlacementQueryError::Unavailable)));
        assert_eq!(world.blocks.len(), 1);
    }

    #[test]
    fn tree_markers_place_real_trees_until_shared_budget_is_full() {
        let resources = resources();
        let mut world = World { blocks: (0..4).map(|i| ([i * 20, 63, 0], BlockStateId(2))).collect() };
        let mut random = FixedRandom::default();
        let mut budget = tconstruct_slime_island_markers::TreeBudget::default();
        let mut ctx = PieceContext { world: &mut world, resources: &resources, random: &mut random,
            chunk_box: BoundingBox::new([-16, -64, -16], [100, 319, 31]), chunk: ChunkPos::new(0, 0), reference: [0, 64, 0] };
        let cfg = tconstruct_slime_tree_world::TreeConfig { base_height: 4, random_height: 3, double_height: false,
            vines: false, trunk: "tconstruct:greenheart_log", trunk_properties: empty_properties(), leaves: "tconstruct:earth_slime_leaves", leaves_properties: empty_properties(), vine: None, vine_properties: empty_properties() };
        assert!(!tconstruct_slime_island_marker_world::tree_marker(&mut ctx, &mut budget, None, [0, 64, 0]).unwrap());
        for i in 0..3 { assert!(tconstruct_slime_island_marker_world::tree_marker(&mut ctx, &mut budget, Some(cfg), [i * 20, 64, 0]).unwrap()); }
        assert!(!tconstruct_slime_island_marker_world::tree_marker(&mut ctx, &mut budget, Some(cfg), [60, 64, 0]).unwrap());
        assert_eq!(budget.placed(), 3);
        assert_eq!(random.booleans, 3);
        assert_eq!(world.blocks.values().filter(|id| **id == BlockStateId(3)).count(), 12);
    }

    #[test]
    fn grass_marker_uses_weighted_source_plant_and_requires_slimy_soil() {
        let resources = resources();
        let mut world = World { blocks: BTreeMap::from([([0, 63, 0], BlockStateId(2)), ([2, 63, 0], BlockStateId(1))]) };
        let mut random = FixedRandom::default();
        let mut ctx = PieceContext { world: &mut world, resources: &resources, random: &mut random,
            chunk_box: BoundingBox::new([-16, -64, -16], [31, 319, 31]), chunk: ChunkPos::new(0, 0), reference: [0, 64, 0] };
        let choices = vec![("tconstruct:earth_slime_tall_grass".to_owned(), 7), ("tconstruct:earth_slime_fern".to_owned(), 1)];
        assert!(tconstruct_slime_island_marker_world::grass_marker(&mut ctx, &choices, [0, 64, 0]).unwrap());
        assert!(!tconstruct_slime_island_marker_world::grass_marker(&mut ctx, &choices, [2, 64, 0]).unwrap());
        assert_eq!(world.blocks.get(&[0, 64, 0]), Some(&BlockStateId(11)));
        assert!(!world.blocks.contains_key(&[2, 64, 0]));
    }

    #[test]
    fn clay_island_grass_uses_vanilla_dirt_instead_of_slimy_soil() {
        let resources = resources();
        let mut world = World { blocks: BTreeMap::from([([0, 63, 0], BlockStateId(1)), ([2, 63, 0], BlockStateId(2))]) };
        let mut random = FixedRandom::default();
        let mut ctx = PieceContext { world: &mut world, resources: &resources, random: &mut random,
            chunk_box: BoundingBox::new([-16, -64, -16], [31, 319, 31]), chunk: ChunkPos::new(0, 0), reference: [0, 64, 0] };
        let choices = vec![("minecraft:short_grass".to_owned(), 7)];
        tconstruct_slime_island_marker_world::validate(&ctx, None, &choices, None).unwrap();
        assert!(tconstruct_slime_island_marker_world::grass_marker(&mut ctx, &choices, [0, 64, 0]).unwrap());
        assert!(!tconstruct_slime_island_marker_world::grass_marker(&mut ctx, &choices, [2, 64, 0]).unwrap());
        assert_eq!(world.blocks.get(&[0, 64, 0]), Some(&BlockStateId(109)));
    }

    #[test]
    fn vine_marker_attaches_and_grows_down_when_supported() {
        let resources = resources();
        let mut world = World { blocks: BTreeMap::from([([1, 64, 0], BlockStateId(1))]) };
        let mut random = FixedRandom::default();
        let mut ctx = PieceContext { world: &mut world, resources: &resources, random: &mut random,
            chunk_box: BoundingBox::new([-16, -64, -16], [31, 319, 31]), chunk: ChunkPos::new(0, 0), reference: [0, 64, 0] };
        assert!(tconstruct_slime_island_marker_world::vine_marker(&mut ctx, Some("tconstruct:sky_slime_vine"), [0, 64, 0]).unwrap());
        for pos in [[0, 64, 0], [0, 63, 0]] {
            let state = *world.blocks.get(&pos).unwrap();
            let block = resources.registry.state(state).unwrap();
            assert_eq!(block.name, "tconstruct:sky_slime_vine");
            assert_eq!(block.properties.get("east").map(String::as_str), Some("true"));
            assert_eq!(block.properties.get("stage").map(String::as_str), Some("start"));
        }
    }

    #[test]
    fn island_piece_round_trips_marker_state_rotation_and_mirror() {
        let dir = std::env::temp_dir().join(format!("tconstruct-island-piece-{}", std::process::id()));
        let fixture_dir = dir.join("tconstruct/test");
        std::fs::create_dir_all(&fixture_dir).unwrap();
        std::fs::write(fixture_dir.join("island.nbt"), include_bytes!("../fixtures/slime-island-empty.nbt")).unwrap();
        let resources = resources_at(&dir);
        let source: serde_json::Value = serde_json::from_slice(&std::fs::read(
            "tconstruct/content/reference/slime-islands/data/tconstruct/worldgen/configured_feature/earth_slime_island_tree.json").unwrap()).unwrap();
        let tree = tconstruct_slime_tree_config::SlimeTreeSpec::from_json("tconstruct:earth_slime_island_tree", &source).unwrap();
        let markers = tconstruct_slime_island_piece::MarkerConfig { structure: "tconstruct:earth_slime_island".into(),
            tree: Some(tree), deferred_tree: None, grasses: vec![("tconstruct:earth_slime_tall_grass".into(), 7), ("tconstruct:earth_slime_fern".into(), 1)], vine: None };
        let piece = tconstruct_slime_island_piece::IslandPiece::new(&resources, "tconstruct:test/island", [8, 80, 24],
            Rotation::Clockwise90, Mirror::FrontBack, markers, 2, None, 384).unwrap();
        let saved = piece.save().unwrap();
        let read_str = |key: &str| match saved.get(&key.into()).unwrap() { Tag::String(s) => s.to_string_lossy(), _ => panic!("expected string") };
        let read_int = |key: &str| match saved.get(&key.into()).unwrap() { Tag::Int(v) => *v, _ => panic!("expected int") };
        let value = serde_json::json!({"Rot":read_str("Rot"), "Mi":read_str("Mi"), "Template":read_str("Template"),
            "TCMarkers":read_str("TCMarkers"), "NumberOfTreesPlaced":read_int("NumberOfTreesPlaced"),
            "TPX":read_int("TPX"), "TPY":read_int("TPY"), "TPZ":read_int("TPZ")});
        let restored = tconstruct_slime_island_piece::IslandPiece::from_saved(&resources, &value, None, 384).unwrap();
        assert_eq!(restored.save().unwrap(), saved);
        let mut invalid = value;
        invalid["NumberOfTreesPlaced"] = serde_json::json!(4);
        assert!(tconstruct_slime_island_piece::IslandPiece::from_saved(&resources, &invalid, None, 384).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
