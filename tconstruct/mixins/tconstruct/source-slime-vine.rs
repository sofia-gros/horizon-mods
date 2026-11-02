//! Source Sky slime vine random growth and neighbor connection rules.
use super::*;
use horizon_core::{geometry::Direction, random::RandomSource};
const SIDES: [(Direction, &str, (i32,i32,i32)); 4] = [
    (Direction::North, "north", (0,0,-1)), (Direction::East, "east", (1,0,0)),
    (Direction::South, "south", (0,0,1)), (Direction::West, "west", (-1,0,0)),
];
fn offset(pos: BlockPos, d: (i32,i32,i32)) -> Option<BlockPos> {
    Some(BlockPos::new(pos.x.checked_add(d.0)?, pos.y.checked_add(d.1)?, pos.z.checked_add(d.2)?))
}
fn acceptable(level: &GameLevel<'_>, pos: BlockPos, direction: Direction) -> bool {
    let state = level.block_state(pos);
    level.behaviours.shape_info(state).full_faces & (1 << direction.opposite() as u8) != 0
}
fn active(state: &horizon_world::chunk::BlockState, name: &str) -> bool {
    state.properties.get(name).is_some_and(|v| v == "true")
}
fn is_leaves(level: &GameLevel<'_>, state: BlockStateId) -> Option<bool> {
    Some(level.behaviours.tag("tconstruct:slimy_leaves")?.contains(level.behaviours.block(state)))
}
fn has_attachment(level: &GameLevel<'_>, pos: BlockPos, direction: Direction, key: &str, delta: (i32,i32,i32)) -> Option<bool> {
    if acceptable(level, offset(pos, delta)?, direction) { return Some(true); }
    let above = level.block_state(offset(pos,(0,1,0))?);
    if is_leaves(level, above)? { return Some(true); }
    if direction == Direction::Up { return Some(false); }
    let state = level.registry.state(above)?;
    Some(state.name == "tconstruct:sky_slime_vine" && active(state,key) && state.properties.get("stage").is_some_and(|s| s != "end"))
}
pub(super) fn update(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, direction: Direction, new: &mut Option<BlockStateId>) {
    let Some(block) = level.registry.state(state) else { return; };
    if block.name != "tconstruct:sky_slime_vine" || direction == Direction::Down { return; }
    let mut next = block.clone();
    if active(&next,"up") {
        let Some(above) = offset(pos,(0,1,0)) else { *new=Some(state); return; };
        next.properties.insert("up".into(), acceptable(level,above,Direction::Up).to_string());
    }
    for (direction,key,delta) in SIDES {
        if active(&next,key) {
            let Some(attached) = has_attachment(level,pos,direction,key,delta) else { *new=Some(state); return; };
            next.properties.insert(key.into(),attached.to_string());
        }
    }
    if !["up","north","east","south","west"].iter().any(|s| active(&next,s)) {
        *new = level.registry.default_state("minecraft:air");
    } else { *new = level.registry.resolve(&next).ok(); }
}
pub(super) fn random(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    let Some(block) = level.registry.state(state) else { return; };
    if block.name != "tconstruct:sky_slime_vine" { return; }
    *handled = true;
    let mut next = block.clone();
    let horizontal = SIDES.iter().any(|(_,key,_)| active(&next,key));
    let behaviours = level.behaviours;
    if !horizontal {
        let Some(above) = offset(pos,(0,1,0)) else { return; };
        let Some(on_leaves) = is_leaves(level,level.block_state(above)) else { return; };
        let mut changed = false;
        for (direction,key,delta) in SIDES {
            let Some(neighbor) = offset(pos,delta) else { return; };
            if (on_leaves || acceptable(level,neighbor,direction)) && matches!(level.world.random.next_int_bound(6),Ok(0)) {
                next.properties.insert(key.into(),"true".into()); changed = true;
            }
        }
        if changed { if let Ok(next) = level.registry.resolve(&next) { behaviours.set_block(level,ticks,pos,next,3); } }
        return;
    }
    if !matches!(level.world.random.next_int_bound(4),Ok(0)) || next.properties.get("stage").is_none_or(|s| s == "end") { return; }
    let Some(below) = offset(pos,(0,-1,0)) else { return; };
    if !behaviours.is_air(level.block_state(below)) { return; }
    let floating = SIDES.iter().all(|(direction,key,delta)| {
        !active(&next,key) || offset(pos,*delta).is_none_or(|neighbor| !acceptable(level,neighbor,*direction))
    });
    if floating {
        let Some(stage) = next.properties.get("stage").cloned() else { return; };
        let mut count = 1;
        for dy in 1..3 {
            let Some(above) = offset(pos,(0,dy,0)) else { break; };
            let Some(other) = level.registry.state(level.block_state(above)) else { break; };
            if other.name != "tconstruct:sky_slime_vine" || other.properties.get("stage") != Some(&stage) { break; }
            count += 1;
        }
        if count > 2 || matches!(level.world.random.next_int_bound(2),Ok(0)) {
            let to = match stage.as_str() { "start" => "middle", "middle" => "end", _ => return };
            next.properties.insert("stage".into(),to.into());
        }
    }
    next.properties.insert("up".into(),"false".into());
    if let Ok(next) = level.registry.resolve(&next) { behaviours.set_block(level,ticks,below,next,3); }
}
