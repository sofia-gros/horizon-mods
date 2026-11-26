// Adapted from pinned MIT Horizon game/creatures.rs::convert_to.
// License: pluto-lib/LICENSE.horizon. Only the frog conversion has a later
// finalize_converted call in this pinned caller; other conversions do not roll.
fn pluto_entity_spawn_conversion(game:&mut Game<'_>,from:&Mob,kind:&str,params:sunrise_client_lib::mobs::conversion::Params)
    ->std::ops::ControlFlow<Option<u32>> {
    use sunrise_client_lib::pluto_entity_spawn_access as access;
    use crate::game::pluto_entity_spawn_game as bridge;
    if kind!="minecraft:frog" { return std::ops::ControlFlow::Continue(()); }
    match access::enabled() {
        Ok(false)=>return std::ops::ControlFlow::Continue(()),
        Err(error)=>{ log::error!("PlutoLib spawn registry: {error}"); return std::ops::ControlFlow::Break(None); },
        Ok(true)=>{},
    }
    let Some(kind_type)=game.res.mob_types().get(kind).cloned() else { return std::ops::ControlFlow::Break(None) };
    let mut cancel=false;
    let from_kind=super::native_hooks::mob_kind(from);
    game.native_living_conversion_pre(from.id,&from_kind,kind,&mut cancel);
    if cancel {
        match game.mobs.iter_mut().find(|mob|mob.id==from.id) {
            Some(kept)=>kept.removed=false,
            None=>{ let mut kept=from.clone(); kept.removed=false; game.mobs.push(kept); },
        }
        return std::ops::ControlFlow::Break(None);
    }
    game.next_entity+=1;
    let mut mob=Mob::new(game.next_entity,kind_type,from.position,from.yaw);
    sunrise_client_lib::mobs::conversion::single(from,&mut mob,params);
    if matches!(from.species(),Species::Zombie|Species::Husk) {
        let at=[from.position.x,from.position.y,from.position.z];
        let special=game.local_difficulty(at).special();
        sunrise_client_lib::mobs::finalize::zombie_attributes(&mut mob,special,sunrise_client_lib::mobs::finalize::Reason::Conversion);
    }
    let result=(|| {
        let selected=bridge::prepare(game,&mut mob,"conversion")?;
        if selected {
            let at=[mob.position.x,mob.position.y,mob.position.z];
            let facts=game.biome_facts(at[0].floor() as i32,at[1].floor() as i32,at[2].floor() as i32);
            let difficulty=game.local_difficulty(at);
            spawning::finalize(&mut mob,&facts,&difficulty);
            mob.persistent=true;
        }
        let pending=bridge::take(&mob)?;
        let id=mob.id;
        game.mobs.push(mob);
        if let Some(pending)=pending { bridge::complete(game,id,pending)?; }
        // Publish conversion-owned relationships only after successful gear work.
        for rider in game.mobs.iter_mut() {
            if rider.extra.mount==Some(from.id) { rider.extra.mount=Some(id); }
        }
        if let Some(raw)=sunrise_client_lib::entity_nbt::conversion_tags(game.persist_raw(from.id)) {
            game.persist.raw.insert(id,raw);
        }
        let index=game.mobs.iter().position(|mob|mob.id==id).ok_or("converted mob disappeared before join")?;
        game.native_mobs_joined(index,false);
        let Some(transformed_kind)=game.mobs.iter().find(|mob|mob.id==id).map(|mob|mob.kind.id.clone()) else {
            return Ok(None);
        };
        game.native_entity_transform(from.id,&from.kind.id,id,&transformed_kind);
        game.native_living_conversion_post(from.id,&from_kind,id,&transformed_kind);
        Ok::<_,String>(Some(id))
    })();
    match result {
        Ok(id)=>std::ops::ControlFlow::Break(id),
        Err(error)=>{ log::error!("PlutoLib conversion spawn failed: {error}"); std::ops::ControlFlow::Break(None) },
    }
}
