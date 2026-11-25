// Adapted from pinned MIT Horizon natural_spawning.rs::finalize_spawned.
// License: pluto-lib/LICENSE.horizon. The prepared plan travels with the owned
// pending Mob while its numeric ID is still zero.
fn pluto_entity_spawn_natural(game:&Game<'_>,request:&SpawnRequest,pending:&mut Vec<Mob>)
    ->std::ops::ControlFlow<Option<Spawned>> {
    use crate::pluto_entity_spawn_access as access;
    match access::enabled() {
        Ok(false)=>return std::ops::ControlFlow::Continue(()),
        Err(error)=>{ log::error!("PlutoLib spawn registry: {error}"); return std::ops::ControlFlow::Break(None); },
        Ok(true)=>{},
    }
    let Some(kind)=game.res.mob_types().get(request.entity).cloned() else { return std::ops::ControlFlow::Break(None) };
    if !crate::game::pluto_spawn_placement_game::natural_allowed(game,request.entity,request.species,request.position) {
        return std::ops::ControlFlow::Break(None);
    }
    let [x,y,z]=request.position;
    let mut mob=Mob::new(0,kind,Vec3::new(x,y,z),request.yaw);
    if let Err(error)=crate::game::pluto_entity_spawn_game::prepare(game,&mut mob,"natural") {
        log::error!("PlutoLib natural spawn preparation failed: {error}");
        return std::ops::ControlFlow::Break(None);
    }
    let facts=game.biome_facts(x.floor() as i32,y.floor() as i32,z.floor() as i32);
    let difficulty=game.local_difficulty(request.position);
    sunrise_client::mobs::spawning::finalize(&mut mob,&facts,&difficulty);
    sunrise_client::mobs::raider::finalize(&mut mob,sunrise_client::mobs::raider::Reason::Natural);
    let bounds=mob.bounds();
    pending.push(mob);
    std::ops::ControlFlow::Break(Some(Spawned { bounds,school:true }))
}
