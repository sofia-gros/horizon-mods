//Pinned item/projectile sweep with an after-contact visitor. Native listeners
//run exactly once before the mutable visitor at each deduplicated block.
use super::*;

pub fn swept_inside<'a>(
    entity: i32, kind: &str,
    from: sunrise_core::geometry::Vec3, to: sunrise_core::geometry::Vec3,
    bounds: sunrise_core::geometry::Aabb,
    mut block: impl FnMut(BlockPos) -> (BlockStateId, &'a str),
    mut after: impl FnMut(BlockPos, BlockStateId),
) {
    if !MOD_BLOCKS.load(Ordering::Relaxed) || LEVEL.get().is_none() || HOOK.get().is_none() { return; }
    let bounds = bounds.deflate(sunrise_core::geometry::Vec3::new(1e-5, 1e-5, 1e-5));
    let mut visited = std::collections::HashSet::new();
    let _ = sunrise_core::geometry::sweep_intersected_blocks(from, to, bounds, 1_000_000, |pos, step| {
        if step >= 16 { return false; }
        if visited.insert(pos) {
            let (state, name) = block(pos);
            emit(Event::Inside(Contact { entity, kind, block: name, state, position: pos }));
            after(pos, state);
        }
        true
    });
}
