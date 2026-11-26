// Adapted from pinned MIT Horizon game/raids.rs::spawn_event_mob.
// License: pluto-lib/LICENSE.horizon. Patrol leadership/target setup precedes
// finalizeSpawn; raid wave buffs in the caller still follow its return.
fn pluto_entity_spawn_raid(game:&mut Game<'_>,kind:&str,at:[f64;3],leader:bool,reason:raider::Reason)
    ->std::ops::ControlFlow<Option<u32>> {
    use crate::pluto_entity_spawn_access as access;
    use crate::game::pluto_entity_spawn_game as bridge;
    match access::enabled() {
        Ok(false)=>return std::ops::ControlFlow::Continue(()),
        Err(error)=>{ log::error!("PlutoLib spawn registry: {error}"); return std::ops::ControlFlow::Break(None); },
        Ok(true)=>{},
    }
    let Some(kind)=game.res.mob_types().get(kind).cloned() else { return std::ops::ControlFlow::Break(None) };
    game.next_entity+=1;
    let mut mob=Mob::new(game.next_entity,kind,Vec3::new(at[0],at[1],at[2]),0.0);
    if leader {
        raider::set_patrol_leader(&mut mob,true);
        if reason==raider::Reason::Patrol { raider::find_patrol_target_from(&mut mob,[0,0,0]); }
    }
    let result=(|| {
        bridge::prepare(game,&mut mob,if reason==raider::Reason::Patrol {"patrol"}else{"event"})?;
        let facts=game.biome_facts(at[0].floor() as i32,at[1].floor() as i32,at[2].floor() as i32);
        let difficulty=game.local_difficulty(at);
        spawning::finalize(&mut mob,&facts,&difficulty);
        raider::finalize(&mut mob,reason);
        let pending=bridge::take(&mob)?;
        let deferred=pending.is_some();
        let id=mob.id;
        game.mobs.push(mob);
        if !deferred { game.native_mobs_joined(game.mobs.len()-1,false); }
        game.enchant_spawn_equipment(id);
        game.halloween_spawn_head(id);
        if let Some(pending)=pending {
            bridge::complete(game,id,pending)?;
            let index=game.mobs.iter().position(|m|m.id==id).ok_or("completed event spawn disappeared before join")?;
            game.native_mobs_joined(index,false);
            if !game.mobs.iter().any(|m|m.id==id) { return Ok(None); }
        }
        Ok::<_,String>(Some(id))
    })();
    match result {
        Ok(id)=>std::ops::ControlFlow::Break(id),
        Err(error)=>{ log::error!("PlutoLib event spawn failed: {error}"); std::ops::ControlFlow::Break(None) },
    }
}
