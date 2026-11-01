// Connect one original tree marker to the source tree implementation.
// The owning piece supplies the already-selected tree and its shared budget.
use crate::worldgen::structure::legacy::{PieceContext, Result};
use crate::worldgen::placement::PlacementQueryError;
use horizon_core::geometry::Direction;
use super::{tconstruct_slime_island_markers::{self, TreeBudget, MAX_TREES}, tconstruct_slime_tree_world::{self, TreeConfig}};

pub(crate) fn tree_marker(
    ctx: &mut PieceContext<'_>,
    budget: &mut TreeBudget,
    selected: Option<TreeConfig<'_>>,
    pos: [i32; 3],
) -> Result<bool> {
    let Some(config) = selected else { return Ok(false); };
    // IslandPiece checks tree presence and the limit before consuming its
    // boolean draw, and spends the limit only after successful placement.
    if budget.placed() >= MAX_TREES || !ctx.random.boolean() { return Ok(false); }
    let placed = tconstruct_slime_tree_world::place(ctx, pos, config)?;
    if placed { budget.tree_marker(true, || true, || true); }
    Ok(placed)
}

/// Source grass markers select a weighted block only after their boolean draw.
pub(crate) fn grass_marker(ctx: &mut PieceContext<'_>, choices: &[(String, u32)], pos: [i32; 3]) -> Result<bool> {
    if !ctx.random.boolean() || choices.is_empty() { return Ok(false); }
    let total = choices.iter().try_fold(0u32, |sum, (_, weight)| sum.checked_add(*weight)).ok_or(PlacementQueryError::Unavailable)?;
    let bound = i32::try_from(total).ok().filter(|v| *v > 0).ok_or(PlacementQueryError::Unavailable)?;
    let draw = ctx.random.int(bound) as u32;
    let weights: Vec<_> = choices.iter().map(|(_, weight)| *weight).collect();
    let index = tconstruct_slime_island_markers::weighted_index(&weights, draw).ok_or(PlacementQueryError::Unavailable)?;
    let state = ctx.resources.registry.default_state(&choices[index].0).ok_or(PlacementQueryError::Unavailable)?;
    let Some(y) = pos[1].checked_sub(1) else { return Ok(false); };
    let soil = ctx.world.block([pos[0], y, pos[2]])?;
    let allowed = ctx.resources.block_tags.get("tconstruct:slimy_soil").ok_or(PlacementQueryError::Unavailable)?;
    if !allowed.contains(ctx.name(soil)) { return Ok(false); }
    ctx.world.set(pos, state, 2)?;
    Ok(true)
}

const HORIZONTAL: [(Direction, &str, [i32; 3]); 4] = [
    (Direction::North, "north", [0,0,-1]), (Direction::South, "south", [0,0,1]),
    (Direction::West, "west", [-1,0,0]), (Direction::East, "east", [1,0,0]),
];

fn shifted(pos: [i32; 3], delta: [i32; 3]) -> Option<[i32; 3]> {
    Some([pos[0].checked_add(delta[0])?, pos[1].checked_add(delta[1])?, pos[2].checked_add(delta[2])?])
}

/// Place and initially grow the original slime vine at a template marker.
pub(crate) fn vine_marker(ctx: &mut PieceContext<'_>, vine: Option<&str>, pos: [i32; 3]) -> Result<bool> {
    let Some(name) = vine else { return Ok(false); };
    if !ctx.random.boolean() { return Ok(false); }
    let base = ctx.resources.registry.default_state(name).ok_or(PlacementQueryError::Unavailable)?;
    let mut placed = false;
    for (direction, property) in [(Direction::Up, "up"), (Direction::North, "north"),
        (Direction::South, "south"), (Direction::West, "west"), (Direction::East, "east")] {
        if ctx.world.face_full(pos, direction)? {
            let state = tconstruct_slime_tree_world::with_property(ctx, base, property, "true")?;
            ctx.world.set(pos, state, 2)?;
            placed = true;
        }
    }
    let grow = ctx.random.int(8);
    let mut current = pos;
    for _ in 0..=grow {
        let state = ctx.world.block(current)?;
        if ctx.name(state) != name { break; }
        let stage = ctx.property(state, "stage").ok_or(PlacementQueryError::Unavailable)?.to_owned();
        let has_sides = HORIZONTAL.iter().any(|(_, side, _)| ctx.property(state, side) == Some("true"));
        if !has_sides || stage == "end" { break; }
        let Some(below) = shifted(current, [0,-1,0]) else { break; };
        let below_state = ctx.world.block(below)?;
        if !ctx.is_air(below_state) { break; }
        let mut free_floating = true;
        for (direction, side, _) in HORIZONTAL {
            if ctx.property(state, side) == Some("true") && ctx.world.face_full(current, direction)? {
                free_floating = false;
                break;
            }
        }
        let mut next = state;
        if free_floating {
            let mut same_above = 1;
            for dy in 1..3 {
                let Some(above) = shifted(current, [0,dy,0]) else { break; };
                let other = ctx.world.block(above)?;
                if ctx.name(other) != name || ctx.property(other, "stage") != Some(stage.as_str()) { break; }
                same_above += 1;
            }
            if same_above > 2 || ctx.random.int(2) == 0 {
                let next_stage = match stage.as_str() { "start" => "middle", "middle" => "end", _ => return Err(PlacementQueryError::Unavailable) };
                next = tconstruct_slime_tree_world::with_property(ctx, next, "stage", next_stage)?;
            }
        }
        next = tconstruct_slime_tree_world::with_property(ctx, next, "up", "false")?;
        ctx.world.set(below, next, 3)?;
        current = below;
    }
    Ok(placed)
}
