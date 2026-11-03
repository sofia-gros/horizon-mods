// Adapted from SlimeGrassBlock.growGrass, Copyright (c) 2022 SlimeKnights,
// MIT license: tconstruct/third_party/tconstruct/LICENSE.
use super::{BlockPos, BlockStateId, GameLevel, LevelAccess, Ticks};
use horizon_core::random::RandomSource;
use horizon_world::level::data::state_flags::FULL_COLLISION;

fn foliage(level: &GameLevel<'_>, state: BlockStateId) -> Option<&'static str> {
    let name = level.registry.state(state)?.name.as_str();
    let (kind, soil) = name
        .strip_prefix("tconstruct:")?
        .strip_suffix("_slime_grass")?
        .split_once('_')?;
    if !["earth", "sky", "ichor", "ender", "vanilla"].contains(&soil) {
        return None;
    }
    match kind {
        "earth" => Some("earth"),
        "sky" => Some("sky"),
        "ender" => Some("ender"),
        _ => None,
    }
}

pub(super) fn valid(
    level: &GameLevel<'_>,
    pos: BlockPos,
    state: BlockStateId,
    result: &mut Option<bool>,
) {
    if foliage(level, state).is_none() {
        return;
    }
    let Some(y) = pos.y.checked_add(1) else {
        *result = Some(false);
        return;
    };
    let above = level.block_state(BlockPos::new(pos.x, y, pos.z));
    *result = Some(level.behaviours.is_air(above));
}

pub(super) fn success(level: &GameLevel<'_>, state: BlockStateId, result: &mut bool) {
    if foliage(level, state).is_some() {
        *result = true;
    }
}

use super::tconstruct_slime_grass_meal_algorithm::{grow, GrowWorld};

struct Runtime<'a, 'b> {
    level: &'a mut GameLevel<'b>,
    ticks: &'a mut Ticks,
    foliage: &'static str,
}
impl GrowWorld for Runtime<'_, '_> {
    fn roll(&mut self, bound: i32) -> Option<i32> {
        self.level.world.random.next_int_bound(bound).ok()
    }
    fn full(&self, pos: BlockPos) -> bool {
        if !self.level.world.loaded(pos.x, pos.z) {
            return true;
        }
        self.level
            .behaviours
            .facts(self.level.block_state(pos))
            .flags
            & FULL_COLLISION
            != 0
    }
    fn valid_base(&self, pos: BlockPos) -> bool {
        if !self.level.world.loaded(pos.x, pos.z) {
            return false;
        }
        let block = self.level.block_state(pos);
        self.level
            .behaviours
            .tag("tconstruct:slimy_grass")
            .is_some_and(|t| t.contains(self.level.behaviours.block(block)))
    }
    fn air(&self, pos: BlockPos) -> bool {
        self.level.world.loaded(pos.x, pos.z)
            && self.level.behaviours.is_air(self.level.block_state(pos))
    }
    fn place(&mut self, pos: BlockPos, fern: bool) {
        let suffix = if fern { "fern" } else { "tall_grass" };
        let Some(state) = self
            .level
            .registry
            .default_state(&format!("tconstruct:{}_slime_{suffix}", self.foliage))
        else {
            return;
        };
        let behaviours = self.level.behaviours;
        if behaviours.can_survive(self.level, self.ticks, pos, state) {
            behaviours.set_block(self.level, self.ticks, pos, state, 3);
        }
    }
}

pub(super) fn perform(
    level: &mut GameLevel<'_>,
    ticks: &mut Ticks,
    pos: BlockPos,
    state: BlockStateId,
) {
    let Some(kind) = foliage(level, state) else {
        return;
    };
    grow(
        &mut Runtime {
            level,
            ticks,
            foliage: kind,
        },
        pos,
    );
}
