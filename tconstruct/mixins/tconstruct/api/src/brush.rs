//! The brush modifier family (BrushModule, SlimeKnights, MIT; see LICENSE): a tool carrying the module brushes
//! suspicious blocks like the vanilla brush item. A right click on a block starts a 200 tick use in the BRUSH
//! pose (the per-stack use the throwing hook's registration composes for it); on the stroke ticks of the use
//! (`(duration - timeLeft + 1) % 10 == 5`) the block the view ray hits, and each of the tool's AOE transform
//! neighbours, is brushed once through PlutoLib's world brush call — the block entity's archaeology count
//! advances, its item drops out when the count completes — and the tool takes one durability damage per block
//! brushed (`BrushModule.onUsingTick`, its `damageAnimated` total). The wiring is in `brush_hook`.
//!
//! Not carried over, exactly:
//! - The release on a missed ray (`onUsingTick` calls `releaseUsingItem` when the ray no longer hits a
//!   block, and stops the use when the tool breaks): a use tick in this port cannot end the use, so the use
//!   runs to its release or its 200 ticks and the strokes simply do not happen while the ray misses or the
//!   tool is broken.
//! - The AOE highlight (`shouldHighlight`): no highlight surface exists in this port — the hook binds, as
//!   the tool action transform's does, and nothing is drawn.
//! - The client-side generic brush sound when a stroke brushes nothing: the game side plays each brushable
//!   block's own sound as its stroke lands (see the flush in PlutoLib), and a stroke on plain blocks is
//!   silent here, as it is server-side in the original.
use pluto_lib::World;

/// BrushModule.getUseDuration: the use lasts 200 ticks.
pub(crate) const USE_DURATION: i32 = 200;

/// The modifier this module belongs to, for the wear's damage cause.
pub(crate) const MODIFIER: &str = "tconstruct:brushing";

/// Whether `block` can be brushed (a `BrushableBlock`: suspicious sand and suspicious gravel —
/// the only blocks with a brushable block entity).
pub(crate) fn brushable(block: &str) -> bool {
    matches!(block, "minecraft:suspicious_sand" | "minecraft:suspicious_gravel")
}

/// BrushModule.onUsingTick's stroke phase: `(getUseDuration - timeLeft + 1) % 10 == 5`, with
/// `ticks_used` the use tick's own count (`duration - timeLeft`).
pub(crate) fn is_phase(ticks_used: u32) -> bool {
    (ticks_used + 1) % 10 == 5
}

/// The range BrushModule.calculateHitResult aims over: the block reach attribute's base
/// values, 4.5 in survival and 5 in creative (the attribute itself is not read here).
pub(crate) fn reach(creative: bool) -> f64 {
    if creative { 5.0 } else { 4.5 }
}

/// The first solid block along the view ray, walked voxel by voxel as the game's own block
/// raytrace walks it (`ProjectileUtil.getHitResultOnViewVector`'s block half): the block's
/// position, the face the ray enters through (a unit offset) and the distance to that face.
/// None when nothing solid stands within `reach`. `block_at` reads a block's id at a
/// position, None for an unloaded one (which stops nothing and is passed over as air is not:
/// an unloaded position reads as no block, so the walk goes on through it).
pub(crate) fn ray_hit(eye: [f64; 3], look: [f64; 3], reach: f64, block_at: &dyn Fn([i32; 3]) -> Option<String>) -> Option<([i32; 3], [i32; 3], f64)> {
    let solid = |pos: [i32; 3]| block_at(pos).is_some_and(|name| name != "minecraft:air");
    let mut pos = [eye[0].floor() as i32, eye[1].floor() as i32, eye[2].floor() as i32];
    if solid(pos) {
        // The eye inside a solid block: the ray starts in it, and the face it is read
        // through is the one facing the look's dominant axis.
        let axis = (0..3).max_by(|&a, &b| look[a].abs().partial_cmp(&look[b].abs()).unwrap_or(std::cmp::Ordering::Equal))?;
        let mut face = [0; 3];
        face[axis] = if look[axis] > 0.0 { -1 } else { 1 };
        return Some((pos, face, 0.0));
    }
    let step = [look[0].signum() as i32, look[1].signum() as i32, look[2].signum() as i32];
    let mut t_max = [f64::INFINITY; 3];
    let mut t_delta = [f64::INFINITY; 3];
    for axis in 0..3 {
        if step[axis] != 0 {
            t_delta[axis] = (1.0 / look[axis]).abs();
            let boundary = if step[axis] > 0 { pos[axis] as f64 + 1.0 } else { pos[axis] as f64 };
            t_max[axis] = (boundary - eye[axis]) / look[axis];
        }
    }
    loop {
        let axis = if t_max[0] <= t_max[1] && t_max[0] <= t_max[2] { 0 } else if t_max[1] <= t_max[2] { 1 } else { 2 };
        let t = t_max[axis];
        if !t.is_finite() || t > reach {
            return None;
        }
        pos[axis] += step[axis];
        t_max[axis] += t_delta[axis];
        if solid(pos) {
            let mut face = [0; 3];
            face[axis] = -step[axis];
            return Some((pos, face, t));
        }
    }
}

/// One stroke of the use (BrushModule.onUsingTick's brushBlock loop): the aimed block and
/// each AOE neighbour, in that order, is brushed when it is brushable — through the world
/// brush call, which the game applies at its next seam — and the wear the stroke costs is
/// the count of blocks brushed, one point each, as the original's `damageAnimated` total
/// counts them. A block that is not brushable is never called on, as the original calls
/// `brush` only on a brushable block entity.
pub(crate) fn stroke(world: &World, hit: [i32; 3], face: [i32; 3], neighbours: &[[i32; 3]]) -> i32 {
    let mut damage = 0;
    for pos in std::iter::once(hit).chain(neighbours.iter().copied()) {
        let brushable_here = world.get_block_state(pos).is_some_and(|state| brushable(&state.block.to_string()));
        if brushable_here && world.brush_block(pos, face) {
            damage += 1;
        }
    }
    damage
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{MemoryWorldOps, with_world};
    use std::collections::HashMap;

    #[test]
    fn strokes_land_only_on_the_phase_ticks() {
        let phases: Vec<u32> = (0..30).filter(|ticks| is_phase(*ticks)).collect();
        assert_eq!(phases, [4, 14, 24], "(ticks_used + 1) % 10 == 5, as (200 - timeLeft + 1) % 10 == 5 counts it");
        assert!(!is_phase(0) && !is_phase(5) && !is_phase(199));
        assert!(is_phase(194), "the last stroke of the 200 ticks");
    }

    #[test]
    fn the_ray_walks_to_the_first_solid_block_and_names_the_face_it_enters() {
        let mut blocks: HashMap<[i32; 3], String> = HashMap::new();
        blocks.insert([0, 65, -3], "minecraft:suspicious_sand".to_owned());
        blocks.insert([0, 65, -5], "minecraft:stone".to_owned());
        blocks.insert([2, 64, 0], "minecraft:suspicious_gravel".to_owned());
        let at = |pos: [i32; 3]| blocks.get(&pos).cloned();
        // North, at head height: the sand three blocks ahead, entered through its south face.
        let hit = ray_hit([0.5, 65.5, 0.5], [0.0, 0.0, -1.0], 4.5, &at).expect("the sand is hit");
        assert_eq!(hit.0, [0, 65, -3]);
        assert_eq!(hit.1, [0, 0, 1]);
        assert!((hit.2 - 2.5).abs() < 1e-9, "the face is two and a half blocks from the eye: {}", hit.2);
        // Down onto the gravel: entered through its top face.
        let hit = ray_hit([2.5, 66.5, 0.5], [0.0, -1.0, 0.0], 4.5, &at).expect("the gravel is hit");
        assert_eq!((hit.0, hit.1), ([2, 64, 0], [0, 1, 0]));
        // Past the reach nothing is hit, and an empty view hits nothing (the use's strokes
        // simply do not happen while the ray misses, as a released use's would not).
        assert_eq!(ray_hit([0.5, 65.5, 0.5], [0.0, 0.0, -1.0], 2.0, &at), None, "the sand is past a reach of 2");
        assert_eq!(ray_hit([0.5, 65.5, 0.5], [1.0, 0.0, 0.0], 4.5, &at), None);
        assert_eq!(ray_hit([0.5, 65.5, 0.5], [0.0, 0.0, -1.0], 4.5, &|_| None), None, "an unloaded view hits nothing");
    }

    #[test]
    fn a_stroke_brushes_the_hit_and_its_brushable_neighbours_and_counts_the_wear() {
        let world = World::new("minecraft:overworld");
        let mut ops = MemoryWorldOps::default();
        ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, 0]), "minecraft:suspicious_sand".to_owned());
        ops.blocks.insert(("minecraft:overworld".to_owned(), [1, 64, 0]), "minecraft:suspicious_gravel".to_owned());
        ops.blocks.insert(("minecraft:overworld".to_owned(), [0, 64, 1]), "minecraft:stone".to_owned());
        let damage = with_world(&mut ops, || stroke(&world, [0, 64, 0], [0, 1, 0], &[[1, 64, 0], [0, 64, 1], [5, 64, 5]]));
        assert_eq!(damage, 2, "one point per block brushed: the sand and the gravel, not the stone or the air");
        let brushed = &ops.brushes;
        assert_eq!(brushed.len(), 2);
        assert_eq!(brushed[0].pos, [0, 64, 0], "the aimed block is brushed first");
        assert_eq!(brushed[1].pos, [1, 64, 0]);
        assert!(brushed.iter().all(|record| record.face == [0, 1, 0] && record.dimension == "minecraft:overworld"),
            "every neighbour is brushed through the aimed block's face, as Util.offset keeps it");
        // Outside a world scope nothing is brushed and nothing is worn.
        assert_eq!(stroke(&world, [0, 64, 0], [0, 1, 0], &[]), 0);
    }
}
