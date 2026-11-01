//! A small in-memory PieceWorld exercises source tree writes without a game.
pub use horizon_world::{registry, worldgen};
#[path = "../../mixins/tconstruct/source-slime-tree-shape.rs"] mod tconstruct_slime_tree_shape;
#[path = "../../mixins/tconstruct/source-slime-tree-world.rs"] mod tconstruct_slime_tree_world;
#[path = "../../mixins/tconstruct/source-slime-island-markers.rs"] mod tconstruct_slime_island_markers;
#[path = "../../mixins/tconstruct/source-slime-island-marker-world.rs"] mod tconstruct_slime_island_marker_world;
#[path = "../../mixins/tconstruct/source-slime-tree-config.rs"] mod tconstruct_slime_tree_config;

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_core::{position::ChunkPos, random::LegacyRandom};
    use horizon_world::{heightmap::HeightmapType, nbt::Tag, registry::{BlockRegistry, BlockStateId},
        worldgen::{anchor::WorldGenerationContext, feature::FeatureWorld,
                   placement::{PlacementContext, PlacementQueryError},
                   structure::{legacy::{PieceContext, PieceRandom}, processor::ProcessorRandom, BoundingBox, StateTransforms, StructureResources}}};
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
    }

    fn empty_properties() -> &'static BTreeMap<String, String> {
        static EMPTY: std::sync::OnceLock<BTreeMap<String, String>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(BTreeMap::new)
    }

    fn resources() -> StructureResources {
        let mut states = vec![Tag::String("minecraft:air".into()), Tag::String("minecraft:dirt".into()),
            Tag::String("tconstruct:earth_slime_dirt".into()), Tag::String("tconstruct:greenheart_log".into())];
        for distance in 1..=7 {
            states.push(Tag::compound([("id", Tag::String("tconstruct:earth_slime_leaves".into())),
                ("properties", Tag::compound([("distance", Tag::String(distance.to_string().as_str().into()))]))]));
        }
        let defaults = Tag::compound([("minecraft:air", Tag::Int(0)), ("minecraft:dirt", Tag::Int(1)),
            ("tconstruct:earth_slime_dirt", Tag::Int(2)), ("tconstruct:greenheart_log", Tag::Int(3)),
            ("tconstruct:earth_slime_leaves", Tag::Int(10))]);
        let registry = Tag::compound([("DataVersion", Tag::Int(horizon_world::DATA_VERSION)), ("states", Tag::List(states)), ("defaults", defaults)]);
        let registry = Box::leak(Box::new(BlockRegistry::from_nbt(&registry).unwrap()));
        let tags = BTreeMap::from([
            ("tconstruct:slimy_soil".into(), vec!["tconstruct:earth_slime_dirt".into()]),
            ("minecraft:dirt".into(), vec!["minecraft:dirt".into()]),
            ("minecraft:leaves".into(), vec!["tconstruct:earth_slime_leaves".into()]),
        ]);
        StructureResources::new(registry, StateTransforms::default(), Path::new("/nonexistent"), BTreeMap::new(), &tags)
    }

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
}
