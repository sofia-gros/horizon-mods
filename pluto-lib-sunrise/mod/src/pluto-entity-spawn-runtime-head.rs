// Adapted from pinned MIT Horizon game/creatures.rs::spawn_mob_internal.
// License: pluto-lib/LICENSE.horizon. Only selected owned completion work defers
// native join until inherited enchantment, seasonal and jockey/class tails end.
fn pluto_entity_spawn_runtime(game:&mut Game<'_>,kind:&str,at:[f64;3],persistent:bool,scripted:bool,natural:bool,finalize:bool,yaw:SpawnYaw)
    ->std::ops::ControlFlow<Option<u32>> {
    // Construction without finalization belongs to the native conversion path.
    if !finalize { return std::ops::ControlFlow::Continue(()); }
    use sunrise_client_lib::pluto_entity_spawn_access as access;
    use crate::game::pluto_entity_spawn_game as bridge;
    match access::enabled() {
        Ok(false)=>return std::ops::ControlFlow::Continue(()),
        Err(error)=>{ log::error!("PlutoLib spawn registry: {error}"); return std::ops::ControlFlow::Break(None); },
        Ok(true)=>{},
    }
    let kind=if kind.contains(':') {kind.to_owned()}else{format!("minecraft:{kind}")};
    let Some(t)=game.res.mob_types().get(&kind).cloned() else { return std::ops::ControlFlow::Break(None) };
    game.next_entity+=1;
    let yaw=match yaw {
        SpawnYaw::Level=>game.world.random.next_float()*360.0,
        SpawnYaw::Created=>sunrise_client_render::camera::wrap_degrees(game.world.random.next_float()*360.0),
        SpawnYaw::Fixed(yaw)=>yaw,
    };
    let (own,uuid)=sunrise_client_lib::mobs::living_random();
    game.persist.uuids.insert(game.next_entity,uuid);
    let mut mob=Mob::with_random(game.next_entity,t,Vec3::new(at[0],at[1],at[2]),yaw,own);
    mob.persistent=persistent;
    let result=(|| {
        bridge::prepare(game,&mut mob,if natural {"natural"}else{"runtime"})?;
        let facts=game.biome_facts(at[0].floor() as i32,at[1].floor() as i32,at[2].floor() as i32);
        let difficulty=game.local_difficulty(at);
        spawning::finalize(&mut mob,&facts,&difficulty);
        if !scripted {
            let reason=if persistent {sunrise_client_lib::mobs::raider::Reason::Other}else{sunrise_client_lib::mobs::raider::Reason::Natural};
            sunrise_client_lib::mobs::raider::finalize(&mut mob,reason);
        }
        let pending=bridge::take(&mob)?;
        let deferred=pending.is_some();
        let id=mob.id;
        game.mobs.push(mob);
        if !deferred {
            game.native_mobs_joined(game.mobs.len()-1,false);
            if !game.mobs.iter().any(|m|m.id==id) { return Ok(None); }
        }
        game.enchant_spawn_equipment(id);
        game.halloween_spawn_head(id);
        if !persistent && !scripted { game.spawn_jockeys(id,natural); }
        if let Some(pending)=pending {
            bridge::complete(game,id,pending)?;
            let index=game.mobs.iter().position(|m|m.id==id).ok_or("completed spawn disappeared before join")?;
            game.native_mobs_joined(index,false);
            if !game.mobs.iter().any(|m|m.id==id) { return Ok(None); }
        }
        Ok::<_,String>(Some(id))
    })();
    match result {
        Ok(id)=>std::ops::ControlFlow::Break(id),
        Err(error)=>{ log::error!("PlutoLib runtime mob spawn failed: {error}"); std::ops::ControlFlow::Break(None) },
    }
}
