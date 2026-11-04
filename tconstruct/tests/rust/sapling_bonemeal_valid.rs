use std::collections::HashMap;
#[derive(Clone, Copy)] struct BlockPos { x: i32, z: i32 }
type BlockStateId = u8;
struct Block { properties: HashMap<String, String> }
struct Registry { block: Block }
impl Registry { fn state(&self, _: BlockStateId) -> Option<&Block> { Some(&self.block) } }
struct GameLevel<'a> { registry: Registry, loaded: bool, _marker: std::marker::PhantomData<&'a ()> }
mod tconstruct_slime_tree_shape { include!("../../mixins/tconstruct/source-slime-tree-shape.rs"); }
mod tconstruct_sapling_tree {
    use super::*;
    pub(super) fn sapling(_: &GameLevel<'_>, state: BlockStateId) -> bool { state == 1 }
    pub(super) fn growth_chunks_loaded(level: &GameLevel<'_>, _: BlockPos) -> bool { level.loaded }
}
mod adapter { use super::*; include!("../../mixins/tconstruct/source-sapling-bonemeal-valid.rs");
    pub(super) fn invoke(level: &GameLevel<'_>, state: BlockStateId) -> Option<bool> { let mut result = None; tconstruct_sapling_bonemeal_valid(level, BlockPos { x: 0, z: 0 }, state, &mut result); result }
}
#[test] fn stage_one_refuses_unloaded_and_stage_zero_stays_usable() {
    let mut level = GameLevel { registry: Registry { block: Block { properties: [("stage".into(), "1".into())].into() } }, loaded: false, _marker: std::marker::PhantomData };
    assert_eq!(adapter::invoke(&level, 1), Some(false));
    level.registry.block.properties.insert("stage".into(), "0".into()); assert_eq!(adapter::invoke(&level, 1), Some(true));
    level.registry.block.properties.insert("stage".into(), "1".into()); level.loaded = true; assert_eq!(adapter::invoke(&level, 1), Some(true));
    assert_eq!(adapter::invoke(&level, 2), None);
}
