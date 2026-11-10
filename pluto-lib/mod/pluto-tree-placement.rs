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
pub fn place_with<W, R>(
    tree: &TreeFeature,
    world: &mut W,
    random: &mut R,
    origin: [i32; 3],
    nested: &mut NestedPlacer<'_, W, R>,
    extra: impl FnOnce(&mut W, &mut R, Positions<'_>, &mut Vec<[i32; 3]>) -> TreeResult<(), W::Error>,
) -> TreeResult<bool, W::Error>
where
    W: FeatureWorld + PlacementContext,
    R: RandomSource,
{
    let mut run = Run { world, random, sets: Sets::default(), track: true };
    if !tree.do_place(&mut run, origin)? || (run.sets.trunks.is_empty() && run.sets.foliage.is_empty()) {
        return Ok(false);
    }
    let lists = decorator::Lists::new(
        run.sets.trunks.to_vec(), run.sets.foliage.to_vec(), run.sets.roots.to_vec(),
    );
    for decorator in &tree.decorators {
        decorator.place(&mut run, &tree.blocks, &lists, nested)?;
    }
    let mut written = Vec::new();
    extra(run.world, run.random, Positions { logs: &lists.logs, leaves: &lists.leaves, roots: &lists.roots }, &mut written)?;
    for position in written { run.sets.decorations.insert(position); }
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
