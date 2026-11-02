//! Source SlimeLeavesBlock distance rules for the installed overworld leaves.
use super::*;

fn is_slime_leaf(name: &str) -> bool {
    matches!(name, "tconstruct:earth_slime_leaves" | "tconstruct:sky_slime_leaves" | "tconstruct:ender_slime_leaves")
}

fn distance(level: &GameLevel<'_>, state: BlockStateId) -> Option<i32> {
    let tag = level.behaviours.tag("tconstruct:slimy_logs")?;
    if tag.contains(level.behaviours.block(state)) { return Some(0); }
    let block = level.registry.state(state)?;
    if is_slime_leaf(&block.name) {
        return block.properties.get("distance")?.parse::<i32>().ok().filter(|value| (1..=7).contains(value));
    }
    Some(7)
}

pub(super) fn update_shape(level: &GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos,
    state: BlockStateId, neighbor: BlockStateId, new: &mut Option<BlockStateId>) {
    let Some(block) = level.registry.state(state) else { return; };
    if !is_slime_leaf(&block.name) { return; }
    // Do not substitute vanilla's broader leaf/log rules if source tags are
    // unavailable; leave the state intact until they can be resolved.
    *new = Some(state);
    let Some(other) = distance(level, neighbor) else { return; };
    let Some(current) = block.properties.get("distance").and_then(|v| v.parse::<i32>().ok()) else { return; };
    if other + 1 != 1 || current != 1 {
        ticks.schedule_block(level.game_time(), pos, level.behaviours.block(state), 1,
            horizon_core::ticks::TickPriority::Normal);
    }
}

fn updated_state(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, placed: bool) -> Option<BlockStateId> {
    let block = level.registry.state(state)?;
    if !is_slime_leaf(&block.name) { return None; }
    let mut next = block.clone();
    let mut value = 7;
    for (x, y, z) in [(0,-1,0), (0,1,0), (0,0,-1), (0,0,1), (-1,0,0), (1,0,0)] {
        let x = pos.x.checked_add(x)?;
        let y = pos.y.checked_add(y)?;
        let z = pos.z.checked_add(z)?;
        value = value.min(distance(level, level.block_state(BlockPos::new(x,y,z)))? + 1);
        if value == 1 { break; }
    }
    *next.properties.get_mut("distance")? = value.to_string();
    if placed { *next.properties.get_mut("persistent")? = "true".into(); }
    level.registry.resolve(&next).ok()
}

pub(super) fn placement(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId,
    answer: &mut Option<Option<BlockStateId>>) {
    let Some(block) = level.registry.state(state) else { return; };
    if !is_slime_leaf(&block.name) { return; }
    // Refuse placement if source state/tag data is missing, rather than
    // constructing a leaf with the host's different distance rules.
    *answer = Some(updated_state(level, pos, state, true));
}

pub(super) fn tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos,
    state: BlockStateId, handled: &mut bool) {
    let Some(block) = level.registry.state(state) else { return; };
    if !is_slime_leaf(&block.name) { return; }
    *handled = true;
    let Some(next) = updated_state(level, pos, state, false) else { return; };
    let behaviours = level.behaviours;
    behaviours.set_block(level, ticks, pos, next, 3);
}
