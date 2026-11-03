// Adapted from SlimeGrassBlock.growGrass, Copyright (c) 2022 SlimeKnights,
// MIT license: tconstruct/third_party/tconstruct/LICENSE.
use super::BlockPos;
pub(super) trait GrowWorld {
    fn roll(&mut self, bound: i32) -> Option<i32>;
    fn full(&self, pos: BlockPos) -> bool;
    fn valid_base(&self, pos: BlockPos) -> bool;
    fn air(&self, pos: BlockPos) -> bool;
    fn place(&mut self, pos: BlockPos, fern: bool);
}

// Only the overworld-style grass path is implemented here; nylium spreads
// soil and can grow saplings, and must not be silently treated as this path.
pub(super) fn grow(world: &mut impl GrowWorld, origin: BlockPos) {
    let Some(y) = origin.y.checked_add(1) else {
        return;
    };
    let above = BlockPos::new(origin.x, y, origin.z);
    'attempt: for i in 0..128 {
        let mut target = above;
        for _ in 0..(i / 16) {
            let (Some(x), Some(dy), Some(scale), Some(z)) =
                (world.roll(3), world.roll(3), world.roll(3), world.roll(3))
            else {
                return;
            };
            let (Some(x), Some(y), Some(z)) = (
                target.x.checked_add(x - 1),
                target.y.checked_add((dy - 1) * scale / 2),
                target.z.checked_add(z - 1),
            ) else {
                continue 'attempt;
            };
            target = BlockPos::new(x, y, z);
            let Some(below_y) = target.y.checked_sub(1) else {
                continue 'attempt;
            };
            if world.full(target) || !world.valid_base(BlockPos::new(x, below_y, z)) {
                continue 'attempt;
            }
        }
        if world.air(target) {
            let Some(choice) = world.roll(32) else {
                return;
            };
            world.place(target, choice < 6);
        }
    }
}
