// Adapted from pinned MIT Horizon game/persist.rs::load_mob.
// License: pluto-lib/LICENSE.horizon. Only the native finalize marker claims
// this path; ordinary saves/NBT summons keep the original loader and no reroll.
fn pluto_entity_spawn_structure(game:&mut Game<'_>,input:&Tag,mount:Option<u32>)
    ->std::ops::ControlFlow<Option<u32>> {
    use crate::pluto_entity_spawn_access as access;
    use crate::game::pluto_entity_spawn_game as bridge;
    if !matches!(input,Tag::Compound(c) if c.contains_key(&"horizon:finalize".into())) {
        return std::ops::ControlFlow::Continue(());
    }
    match access::enabled() {
        Ok(false)=>return std::ops::ControlFlow::Continue(()),
        Err(error)=>{ log::error!("PlutoLib spawn registry: {error}"); return std::ops::ControlFlow::Break(None); },
        Ok(true)=>{},
    }
    let result=(|| {
        let uuid=entity_nbt::uuid(input);
        if let Some(id)=uuid.and_then(|uuid|game.restored_vehicle(uuid)) { return Ok(Some(id)); }
        if uuid.is_some_and(|uuid|game.target_with_uuid(uuid).is_some()) { return Ok(None); }
        game.next_entity+=1;
        let mut tag=input.clone();
        Game::normalize_loaded_mob_tags(&mut tag);
        if let Tag::Compound(c)=&mut tag { c.remove(&"horizon:finalize".into()); }
        let passengers=if let Tag::Compound(c)=&mut tag { c.remove(&"Passengers".into()) } else { None };
        let (own,fresh)=sunrise_client::mobs::living_random();
        let Some(mut mob)=entity_nbt::mob_from_nbt_with_random(&tag,game.res.mob_types(),game.next_entity,Some(game.pack.assets.registry),own) else {
            return Ok(None);
        };
        if sunrise_client::mobs::camel::is_camel(mob.species()) {
            sunrise_client::mobs::camel::finish_load(&mut mob,game.world.game_time);
        }
        sunrise_client::mobs::adopt_anger_end(&mut mob,game.world.game_time);
        if mob.extra.anger>0 && mob.extra.angry_at.is_some() {
            mob.ai.target=(!mob.built_by_player()).then_some(sunrise_client::mobs::Who::Player);
        }
        mob.extra.mount=mount;
        mob.extra.passenger=mount.is_some();
        bridge::prepare(game,&mut mob,"structure")?;
        let at=[mob.position.x,mob.position.y,mob.position.z];
        let facts=game.biome_facts(at[0].floor() as i32,at[1].floor() as i32,at[2].floor() as i32);
        let difficulty=game.local_difficulty(at);
        sunrise_client::mobs::spawning::finalize(&mut mob,&facts,&difficulty);
        if mob.species()==sunrise_client::mobs::Species::Cat {
            let p=[at[0].floor() as i32,at[1].floor() as i32,at[2].floor() as i32];
            let black=game.structure_tag("minecraft:cats_spawn_as_black");
            if game.structures_at(p).iter().any(|s|black.contains(s)) { mob.extra.variant=10; mob.persistent=true; }
        }
        let pending=bridge::take(&mob)?;
        let id=mob.id;
        game.mobs.push(mob);
        game.enchant_spawn_equipment(id);
        game.halloween_spawn_head(id);
        if let Some(pending)=pending { bridge::complete(game,id,pending)?; }
        // Do not publish retained NBT/UUID bookkeeping for a failed completion.
        game.remember(id,Some(uuid.unwrap_or(fresh)),tag);
        if let Some(passengers)=passengers { game.load_passengers(id,passengers); }
        // The loader's outer caller retains native_mobs_joined ownership.
        Ok::<_,String>(Some(id))
    })();
    match result {
        Ok(id)=>std::ops::ControlFlow::Break(id),
        Err(error)=>{ log::error!("PlutoLib structure spawn failed: {error}"); std::ops::ControlFlow::Break(None) },
    }
}
