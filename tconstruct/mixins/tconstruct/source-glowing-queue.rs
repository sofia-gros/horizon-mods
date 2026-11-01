// Record adjacent glows only; authoritative support is checked after the update.
fn tconstruct_glow_queue(level: &mut crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos, state: horizon_world::registry::BlockStateId) {
    if level.client_side || !level.world.loaded(pos.x, pos.z) || level.world.block(pos.x,pos.y,pos.z) == state { return; }
    for [dx,dy,dz] in [[0,-1,0],[0,1,0],[0,0,-1],[0,0,1],[-1,0,0],[1,0,0]] {
        let Some(x) = pos.x.checked_add(dx) else { continue; };
        let Some(y) = pos.y.checked_add(dy) else { continue; };
        let Some(z) = pos.z.checked_add(dz) else { continue; };
        if !level.world.loaded(x,z) { continue; }
        let neighbor = level.world.block(x,y,z);
        if &*level.facts.get(neighbor).name == "tconstruct:glow" {
            level.world.chunks.__horizon_mixins_tconstruct_glow_support_checks_mut().insert([x,y,z]);
        }
    }
}
