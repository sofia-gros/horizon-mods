// Original slime-tree block writes using the structure piece world interface.
// No worldgen hook calls this module until the piece and its resources exist.
use crate::registry::BlockStateId;
use crate::worldgen::structure::legacy::{PieceContext, Result};
use crate::worldgen::placement::PlacementQueryError;
use super::tconstruct_slime_tree_shape::{self as shape, Operation};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug)]
pub(crate) struct TreeConfig<'a> {
    pub base_height: i32,
    pub random_height: u32,
    pub double_height: bool,
    pub vines: bool,
    pub trunk: &'a str,
    pub trunk_properties: &'a BTreeMap<String, String>,
    pub leaves: &'a str,
    pub leaves_properties: &'a BTreeMap<String, String>,
    pub vine: Option<&'a str>,
    pub vine_properties: &'a BTreeMap<String, String>,
}

fn offset(pos: [i32; 3], delta: [i32; 3]) -> Option<[i32; 3]> {
    Some([pos[0].checked_add(delta[0])?, pos[1].checked_add(delta[1])?, pos[2].checked_add(delta[2])?])
}

fn in_tag(ctx: &PieceContext<'_>, tag: &str, state: BlockStateId) -> Result<bool> {
    let values = ctx.resources.block_tags.get(tag).ok_or(PlacementQueryError::Unavailable)?;
    Ok(values.contains(ctx.name(state)))
}

fn replaceable(ctx: &mut PieceContext<'_>, pos: [i32; 3]) -> Result<bool> {
    let block = ctx.world.block(pos)?;
    Ok(ctx.is_air(block) || in_tag(ctx, "minecraft:leaves", block)?)
}

fn required(ctx: &PieceContext<'_>, name: &str, properties: &BTreeMap<String, String>) -> Result<BlockStateId> {
    let registry = ctx.resources.registry;
    let id = registry.default_state(name).ok_or(PlacementQueryError::Unavailable)?;
    let mut state = registry.state(id).cloned().ok_or(PlacementQueryError::Unavailable)?;
    for (name, value) in properties {
        let property = state.properties.get_mut(name).ok_or(PlacementQueryError::Unavailable)?;
        *property = value.clone();
    }
    registry.id(&state).ok_or(PlacementQueryError::Unavailable)
}

pub(crate) fn with_property(ctx: &PieceContext<'_>, id: BlockStateId, key: &str, value: &str) -> Result<BlockStateId> {
    let mut state = ctx.resources.registry.state(id).cloned().ok_or(PlacementQueryError::Unavailable)?;
    *state.properties.get_mut(key).ok_or(PlacementQueryError::Unavailable)? = value.to_owned();
    ctx.resources.registry.id(&state).ok_or(PlacementQueryError::Unavailable)
}

/// Randomize the source vine's horizontal sides once per group. The upward
/// property is applied independently to the shared state at each position.
fn random_vine(ctx: &mut PieceContext<'_>, base: BlockStateId) -> Result<BlockStateId> {
    let mut state = base;
    let sides = ["north", "east", "south", "west"];
    for side in sides { state = with_property(ctx, state, side, "false")?; }
    let count = ctx.random.int(3) + 1;
    for _ in 0..count {
        let side = sides[ctx.random.int(4) as usize];
        state = with_property(ctx, state, side, "true")?;
    }
    Ok(state)
}

const NEIGHBORS: [[i32; 3]; 6] = [[0,-1,0], [0,1,0], [0,0,-1], [0,0,1], [-1,0,0], [1,0,0]];

/// The source updates the DISTANCE property outward from written trunk blocks.
fn update_distances(ctx: &mut PieceContext<'_>, logs: &BTreeSet<[i32; 3]>) -> Result<()> {
    let mut current = BTreeSet::new();
    for log in logs {
        for delta in NEIGHBORS {
            let Some(pos) = offset(*log, delta) else { continue; };
            if logs.contains(&pos) { continue; }
            let state = ctx.world.block(pos)?;
            if ctx.property(state, "distance").is_some() {
                current.insert(pos);
                ctx.world.set(pos, ctx.with_property(state, "distance", "1"), 19)?;
            }
        }
    }
    for distance in 1..6 {
        let mut next = BTreeSet::new();
        for pos in &current {
            for delta in NEIGHBORS {
                let Some(neighbor) = offset(*pos, delta) else { continue; };
                if current.contains(&neighbor) || next.contains(&neighbor) { continue; }
                let state = ctx.world.block(neighbor)?;
                let Some(value) = ctx.property(state, "distance").and_then(|v| v.parse::<i32>().ok()) else { continue; };
                if value > distance + 1 {
                    let state = ctx.with_property(state, "distance", &(distance + 1).to_string());
                    ctx.world.set(neighbor, state, 19)?;
                    next.insert(neighbor);
                }
            }
        }
        current = next;
    }
    Ok(())
}

pub(crate) fn place(ctx: &mut PieceContext<'_>, origin: [i32; 3], config: TreeConfig<'_>) -> Result<bool> {
    for tag in ["tconstruct:slimy_soil", "minecraft:dirt", "minecraft:leaves"] {
        if !ctx.resources.block_tags.contains_key(tag) { return Err(PlacementQueryError::Unavailable); }
    }
    let range = i32::try_from(config.random_height).ok().filter(|v| *v > 0).ok_or(PlacementQueryError::Unavailable)?;
    let draw = ctx.random.int(range) as u32;
    let double = config.double_height.then(|| ctx.random.int(10) as u32);
    let height = shape::height(config.base_height, config.random_height, draw, double).ok_or(PlacementQueryError::Unavailable)?;
    let min_y = ctx.world.min_y()?;
    let max_y = ctx.world.max_y()?;
    let Some(below) = offset(origin, [0,-1,0]) else { return Ok(false); };
    let soil = ctx.world.block(below)?;
    if !shape::valid_site(origin[1], height, min_y, max_y, in_tag(ctx, "tconstruct:slimy_soil", soil)?) { return Ok(false); }

    // Resolve all required states before modifying the world.
    let trunk = required(ctx, config.trunk, config.trunk_properties)?;
    let leaves = required(ctx, config.leaves, config.leaves_properties)?;
    let empty = BTreeMap::new();
    let air = required(ctx, "minecraft:air", &empty)?;
    let vine = if config.vines { Some(required(ctx, config.vine.ok_or(PlacementQueryError::Unavailable)?, config.vine_properties)?) } else { None };
    if in_tag(ctx, "minecraft:dirt", soil)? {
        let dirt = required(ctx, "minecraft:dirt", &empty)?;
        ctx.world.set(below, dirt, 2)?;
    }

    let mut logs = BTreeSet::new();
    for dy in 0..height {
        let Some(pos) = offset(origin, [0,dy,0]) else { return Err(PlacementQueryError::Unavailable); };
        if replaceable(ctx, pos)? { ctx.world.set(pos, trunk, 2)?; logs.insert(pos); }
    }
    let top = offset(origin, [0,height,0]).ok_or(PlacementQueryError::Unavailable)?;
    let mut vines = BTreeMap::new();
    for op in shape::canopy(config.vines) {
        let (delta, state) = match op {
            Operation::Leaf(delta) => (delta, leaves),
            Operation::Air(delta) => (delta, air),
            Operation::Vine { offset, group, upward } => {
                let state = if let Some(value) = vines.get(&group) { *value } else {
                    let state = random_vine(ctx, vine.ok_or(PlacementQueryError::Unavailable)?)?;
                    vines.insert(group, state);
                    state
                };
                (offset, with_property(ctx, state, "up", if upward { "true" } else { "false" })?)
            }
        };
        let Some(pos) = offset(top, delta) else { return Err(PlacementQueryError::Unavailable); };
        if replaceable(ctx, pos)? { ctx.world.set(pos, state, 2)?; }
    }
    if logs.is_empty() { return Ok(false); }
    update_distances(ctx, &logs)?;
    // The pinned source leaves its edge-update voxel shape empty (its fill
    // calls are commented out), so there are no shape edges to update here.
    Ok(true)
}
