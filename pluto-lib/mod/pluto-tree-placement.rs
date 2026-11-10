// PlutoLib-owned orchestration around the unchanged native tree algorithms.
// No generic function injection, borrowed TLS/global world, or post-chunk scan.
use super::*;

/// Position order comes from native PosSet iteration, stably sorted by height.
pub struct Positions<'a> {
    pub logs: &'a [[i32; 3]],
    pub leaves: &'a [[i32; 3]],
    pub roots: &'a [[i32; 3]],
}

/// Run native roots/trunk/foliage and native decorators, then the supplied extra
/// decorators before native finalization. The caller uses the same world/random
/// borrows and records every successful extra write in `written`. The caller
/// must decode all configuration before this method starts. The extra callback
/// is appended after native decorators; it does not reorder/interleave them.
///
/// The concrete live adapter invokes this entry; its oak fixture checks extra
/// vines, final leaf distances, edge updates and callback failure ownership.
/// Errors preserve earlier writes, just like native trees.
pub fn place_with<W: FeatureWorld + PlacementContext, R: RandomSource>(
    tree: &TreeFeature, world: &mut W, random: &mut R, origin: [i32; 3], nested: &mut NestedPlacer<'_, W, R>,
    extra: impl FnOnce(&mut W, &mut R, Positions<'_>, &mut Vec<[i32; 3]>) -> TreeResult<(), W::Error>,
) -> TreeResult<bool, W::Error> {
    place_with_root_override(tree, world, random, origin, nested, None, extra)
}

pub type RootOverride<'a, W, R> = dyn FnMut(&mut W, &mut R, [i32; 3], &mut Vec<[i32; 3]>) -> TreeResult<Option<BlockStateId>, <W as FeatureWorld>::Error> + 'a;

pub fn place_with_root_override<W: FeatureWorld + PlacementContext, R: RandomSource>(
    tree: &TreeFeature, world: &mut W, random: &mut R, origin: [i32; 3], nested: &mut NestedPlacer<'_, W, R>,
    roots: Option<&mut RootOverride<'_, W, R>>,
    extra: impl FnOnce(&mut W, &mut R, Positions<'_>, &mut Vec<[i32; 3]>) -> TreeResult<(), W::Error>,
) -> TreeResult<bool, W::Error> {
    use crate::level::pluto_block_access::TreeStep;
    let mut plan: Vec<_> = (0..tree.decorators.len()).map(TreeStep::Native).collect();
    plan.push(TreeStep::Custom(0));
    let mut extra = Some(extra);
    place_with_plan(tree, world, random, origin, nested, roots, Some(&plan),
        |_, world, random, positions, written| extra.take().expect("single appended callback")(world, random, positions, written))
}

/// All native decorators appear exactly once, with custom steps between them.
/// Invalid plans are rejected before native placement consumes RNG or writes.
pub fn place_with_plan<W: FeatureWorld + PlacementContext, R: RandomSource>(
    tree: &TreeFeature, world: &mut W, random: &mut R, origin: [i32; 3], nested: &mut NestedPlacer<'_, W, R>,
    roots: Option<&mut RootOverride<'_, W, R>>, plan: Option<&[crate::level::pluto_block_access::TreeStep]>,
    mut extra: impl FnMut(usize, &mut W, &mut R, Positions<'_>, &mut Vec<[i32; 3]>) -> TreeResult<(), W::Error>,
) -> TreeResult<bool, W::Error> {
    use crate::level::pluto_block_access::{TreeStep, valid_tree_plan};
    let mut appended: Vec<_> = (0..tree.decorators.len()).map(TreeStep::Native).collect();
    appended.push(TreeStep::Custom(0));
    let plan = plan.unwrap_or(&appended);
    if !valid_tree_plan(plan, tree.decorators.len()) { return Ok(false) }
    let mut run = Run { world, random, sets: Sets::default(), track: true };
    let placed = match roots {
        Some(roots) => do_place_with_roots(tree, &mut run, origin, roots)?,
        None => tree.do_place(&mut run, origin)?,
    };
    if !placed || (run.sets.trunks.is_empty() && run.sets.foliage.is_empty()) {
        return Ok(false);
    }
    let lists = decorator::Lists::new(
        run.sets.trunks.to_vec(), run.sets.foliage.to_vec(), run.sets.roots.to_vec(),
    );
    for step in plan {
        match *step {
            TreeStep::Native(index) => tree.decorators[index].place(&mut run, &tree.blocks, &lists, nested)?,
            TreeStep::Custom(index) => {
                let mut written = Vec::new();
                extra(index, run.world, run.random, Positions { logs: &lists.logs, leaves: &lists.leaves, roots: &lists.roots }, &mut written)?;
                for position in written { run.sets.decorations.insert(position); }
            }
        }
    }
    let mut bounds: Option<([i32; 3], [i32; 3])> = None;
    for set in [&run.sets.roots, &run.sets.trunks, &run.sets.foliage, &run.sets.decorations] {
        for p in set.to_vec() {
            bounds = Some(match bounds {
                None => (p, p),
                Some((lo, hi)) => (
                    std::array::from_fn(|i| lo[i].min(p[i])),
                    std::array::from_fn(|i| hi[i].max(p[i])),
                ),
            });
        }
    }
    let Some((low, high)) = bounds else { return Ok(false) };
    let shape = update_leaves(&mut run, &tree.blocks, low, high)?;
    update_shape_at_edge(run.world, &shape, low, 3)?;
    Ok(true)
}

// Same native tree orchestration, with only RootPlacer.placeRoot overridden.
// Native simulation, clipping, RNG order, trunk and foliage remain unchanged.
fn do_place_with_roots<W: FeatureWorld + PlacementContext, R: RandomSource>(
    tree: &TreeFeature, run: &mut Run<'_, W, R>, origin: [i32; 3], callback: &mut RootOverride<'_, W, R>,
) -> TreeResult<bool, W::Error> {
    let tree_height = tree.trunk_placer.tree_height(run.random)?;
    let foliage_height = tree.foliage_placer.foliage_height(run.random, tree_height)?;
    let trunk_height = tree_height.wrapping_sub(foliage_height);
    let leaf_radius = tree.foliage_placer.foliage_radius(run.random, trunk_height)?;
    let trunk_origin = match &tree.root_placer {
        Some(root) => tb::offset(origin, 0, root.trunk_offset_y.sample(run.random)?, 0), None => origin,
    };
    let min_y = origin[1].min(trunk_origin[1]);
    let max_y = origin[1].max(trunk_origin[1]).wrapping_add(tree_height).wrapping_add(1);
    let world_min = run.world.min_y().map_err(FeatureExecutionError::Query)?;
    let world_max = run.world.max_y().map_err(FeatureExecutionError::Query)?;
    if min_y < world_min.wrapping_add(1) || max_y > world_max.wrapping_add(1) { return Ok(false) }
    let clipped = tree.max_free_tree_height(run, tree_height, trunk_origin)?;
    if clipped < tree_height && tree.min_clipped_height.is_none_or(|min| clipped < min) { return Ok(false) }
    if let Some(root) = &tree.root_placer {
        let mut column = origin;
        while column[1] < trunk_origin[1] {
            if !root.can_place_root(run, tree, column)? { return Ok(false) }
            column[1] = column[1].wrapping_add(1);
        }
        let mut positions = vec![tb::offset(trunk_origin, 0, -1, 0)];
        for direction in tb::HORIZONTAL {
            let start = tb::relative(trunk_origin, direction, 1);
            let mut branch = Vec::new();
            if !root.simulate(run, tree, start, direction, trunk_origin, &mut branch, 0)? { return Ok(false) }
            positions.extend(branch);
            positions.push(start);
        }
        for pos in positions {
            let mut written = Vec::new();
            let replacement = callback(run.world, run.random, pos, &mut written)?;
            for pos in written { run.sets.roots.insert(pos); }
            match replacement {
                Some(state) => {
                    let state = RootPlacer::waterlogged(tree, run.world, pos, state)?;
                    run.set(Layer::Root, pos, state)?;
                }
                None => root.place_root(run, tree, pos)?,
            }
        }
    }
    let attachments = tree.trunk_placer.place_trunk(run, tree, clipped, trunk_origin)?;
    for attachment in attachments {
        tree.foliage_placer.create_foliage(run, tree, clipped, attachment, foliage_height, leaf_radius)?;
    }
    Ok(true)
}
