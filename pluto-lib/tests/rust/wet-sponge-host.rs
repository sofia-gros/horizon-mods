use horizon_core::{position::BlockPos, random::LegacyRandom};
use horizon_world::{
    level::{self, flags, Behaviours, Dimension, LevelAccess, LevelEvent, Ticks},
    registry::BlockStateId,
};
use std::{collections::HashMap, path::PathBuf};
#[path = "../../mixins/pluto_items_b2/placement-commit.rs"]
mod placement_commit;
struct World {
    blocks: HashMap<BlockPos, BlockStateId>,
    air: BlockStateId,
    random: LegacyRandom,
}
impl LevelAccess for World {
    fn block_state(&self, p: BlockPos) -> BlockStateId {
        self.blocks.get(&p).copied().unwrap_or(self.air)
    }
    fn set_raw(&mut self, p: BlockPos, s: BlockStateId) -> Option<BlockStateId> {
        Some(self.blocks.insert(p, s).unwrap_or(self.air))
    }
    fn is_loaded(&self, _: BlockPos) -> bool {
        true
    }
    fn random(&mut self) -> &mut LegacyRandom {
        &mut self.random
    }
    fn game_time(&self) -> i64 {
        0
    }
    fn emit(&mut self, _: LevelEvent) {}
    fn dimension(&self) -> Dimension {
        Dimension::Nether
    }
}
#[test]
fn real_host_wet_sponge_write_charges_even_if_on_place_changes_state() {
    let root = PathBuf::from(
        std::env::var_os("HORIZON_RESOURCES")
            .expect("set HORIZON_RESOURCES to the pinned host resources"),
    );
    let registry = level::data::load_registry(&root).unwrap();
    let behaviours = Behaviours::load(&registry, &root).unwrap();
    let wet = registry.default_state("minecraft:wet_sponge").unwrap();
    let dry = registry.default_state("minecraft:sponge").unwrap();
    let mut world = World {
        blocks: HashMap::new(),
        air: registry.default_state("minecraft:air").unwrap(),
        random: LegacyRandom::new(42),
    };
    let mut ticks = Ticks::new(42);
    let pos = BlockPos::new(0, 64, 0);
    assert!(behaviours.set_block(&mut world, &mut ticks, pos, wet, flags::UPDATE_ALL));
    assert_eq!(world.block_state(pos), dry);
    let mut cost = 0;
    let outcome = placement_commit::finish(
        &mut cost,
        |c| {
            *c += 1;
            true
        },
        |_| true,
        |_| world.block_state(pos) == wet,
    );
    assert_eq!(outcome, (true, true, false));
    assert_eq!(cost, 1);
}
