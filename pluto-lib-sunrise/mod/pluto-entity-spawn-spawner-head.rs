// Adapted from pinned MIT Horizon game/spawner.rs::plan_spawner_mob.
// License: pluto-lib/LICENSE.horizon. NBT-loaded mobs deliberately continue
// through the original non-finalizing branch without selection or RNG draws.
fn pluto_entity_spawn_spawner(game:&Game<'_>,level:&super::natural_spawning::GameLevel<'_,'_>,request:&SpawnerSpawn<'_>,planned:&mut Vec<Planned>)
    ->std::ops::ControlFlow<Option<SpawnerAdded>> {
    use crate::entry::pluto_entity_spawn_access as access;
    use crate::game::pluto_entity_spawn_game as bridge;
    if !request.finalize { return std::ops::ControlFlow::Continue(()); }
    match access::enabled() {
        Ok(false)=>return std::ops::ControlFlow::Continue(()),
        Err(error)=>{ log::error!("PlutoLib spawn registry: {error}"); return std::ops::ControlFlow::Break(None); },
        Ok(true)=>{},
    }
    let Some(kind)=game.res.mob_types().get(request.id).cloned() else { return std::ops::ControlFlow::Break(None) };
    let [x,y,z]=request.position;
    let mut mob=Mob::new(0,kind,Vec3::new(x,y,z),request.yaw);
    if let Err(error)=bridge::prepare(game,&mut mob,if request.trial {"trial_spawner"}else{"spawner"}) {
        log::error!("PlutoLib spawner preparation failed: {error}");
        return std::ops::ControlFlow::Break(None);
    }
    let facts=game.biome_facts(x.floor() as i32,y.floor() as i32,z.floor() as i32);
    let difficulty=game.local_difficulty(request.position);
    spawning::finalize(&mut mob,&facts,&difficulty);
    mob.persistent=request.trial;
    sunrise_client::mobs::raider::finalize(&mut mob,sunrise_client::mobs::raider::Reason::Other);
    if mob.species()==sunrise_client::mobs::Species::Shulker {
        mob.yaw=0.0; mob.previous_yaw=0.0;
        mob.body_yaw=0.0; mob.previous_body_yaw=0.0;
        mob.head_yaw=0.0; mob.previous_head_yaw=0.0;
    }
    let bounds=mob.bounds();
    planned.push(Planned::Finalized(mob,request.equipment.cloned()));
    level.pending.borrow_mut().push(bounds);
    level.pending_ids.borrow_mut().push((request.id.to_owned(),bounds));
    std::ops::ControlFlow::Break(Some(SpawnerAdded { id:u32::MAX-(planned.len() as u32-1),bounds,mob:true }))
}
