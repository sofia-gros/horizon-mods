// PlutoLib's block behaviours inside the game: the one place that answers `canSurvive`, `getStateForPlacement` and `updateShape` for
// blocks that have a behaviour registered through the API crate (block_api_v1). Other blocks are left to the game and to other hooks.
use super::*;
use horizon_core::geometry::Direction;
use crate::entry::pluto_block::{self, LevelView, Placement, PlacementAnswer, ShapeUpdate, StateInfo};

struct View<'a, 'g> {
    level: &'a GameLevel<'g>,
}

fn at(pos: [i32; 3]) -> BlockPos {
    BlockPos::new(pos[0], pos[1], pos[2])
}

fn info(level: &GameLevel<'_>, id: BlockStateId) -> Option<StateInfo> {
    let state = level.registry.state(id)?;
    Some(StateInfo {
        block: state.name.to_string(),
        properties: state.properties.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    })
}

fn resolve(level: &GameLevel<'_>, wanted: &StateInfo) -> Option<BlockStateId> {
    let base = level.registry.default_state(&wanted.block)?;
    let mut state = level.registry.state(base)?.clone();
    for (key, value) in &wanted.properties {
        state.properties.insert(key.as_str().into(), value.as_str().into());
    }
    level.registry.resolve(&state).ok()
}

impl LevelView for View<'_, '_> {
    fn state(&self, pos: [i32; 3]) -> Option<StateInfo> {
        info(self.level, self.level.block_state(at(pos)))
    }

    fn is_air(&self, pos: [i32; 3]) -> bool {
        self.level.behaviours.is_air(self.level.block_state(at(pos)))
    }

    fn face_sturdy(&self, pos: [i32; 3], face: u8) -> bool {
        face < 6 && self.level.behaviours.shape_info(self.level.block_state(at(pos))).full_faces & (1 << face) != 0
    }

    fn in_tag(&self, pos: [i32; 3], tag: &str) -> bool {
        let behaviours = self.level.behaviours;
        let block = behaviours.block(self.level.block_state(at(pos)));
        behaviours.tag(tag).is_some_and(|set| set.contains(block))
    }
}

pub(super) fn survives(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, survives: &mut Option<bool>) {
    let Some(sink) = pluto_block::sink() else { return };
    let Some(wanted) = info(level, state) else { return };
    if !(sink.has)(&wanted.block) {
        return;
    }
    if let Some(answer) = (sink.survives)(&View { level }, &wanted, [pos.x, pos.y, pos.z]) {
        *survives = Some(answer);
    }
}

pub(super) fn placement(level: &GameLevel<'_>, state: BlockStateId, context: &PlacementContext, answer: &mut Option<Option<BlockStateId>>) {
    let Some(sink) = pluto_block::sink() else { return };
    let Some(default) = info(level, state) else { return };
    if !(sink.has)(&default.block) {
        return;
    }
    let here = info(level, level.block_state(context.pos));
    let in_water = here.is_some_and(|s| s.block == "minecraft:water" || s.properties.get("waterlogged").is_some_and(|v| v == "true"));
    let request = Placement {
        state: default,
        pos: [context.pos.x, context.pos.y, context.pos.z],
        face: context.face as u8,
        yaw: context.yaw,
        pitch: context.pitch,
        sneaking: context.sneaking,
        in_water,
        item: context.item.clone(),
    };
    match (sink.placement)(&View { level }, &request) {
        PlacementAnswer::Default => {}
        PlacementAnswer::Refuse => *answer = Some(None),
        PlacementAnswer::State(chosen) => {
            if let Some(id) = resolve(level, &chosen) {
                *answer = Some(Some(id));
            }
        }
    }
}

pub(super) fn update_shape(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, direction: Direction, neighbour_pos: BlockPos, neighbour: BlockStateId, new: &mut Option<BlockStateId>) {
    let Some(sink) = pluto_block::sink() else { return };
    let Some(current) = info(level, state) else { return };
    if !(sink.has)(&current.block) {
        return;
    }
    let update = ShapeUpdate {
        state: current,
        pos: [pos.x, pos.y, pos.z],
        direction: direction as u8,
        neighbour_pos: [neighbour_pos.x, neighbour_pos.y, neighbour_pos.z],
        neighbour: info(level, neighbour),
    };
    if let Some(changed) = (sink.update_shape)(&View { level }, &update) {
        if let Some(id) = resolve(level, &changed) {
            if id != state {
                *new = Some(id);
            }
        }
    }
}
