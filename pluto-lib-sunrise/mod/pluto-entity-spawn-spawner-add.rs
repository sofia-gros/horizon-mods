// Adapted from pinned MIT Horizon game/spawner.rs::add_planned.
// License: pluto-lib/LICENSE.horizon. The native provisional-ID result map is
// unchanged; completion work itself stays in each owned Mob, never in that map.
fn pluto_entity_spawn_spawner_add(game:&mut Game<'_>,planned:Vec<Planned>)->HashMap<u32,u32> {
    use crate::game::pluto_entity_spawn_game as bridge;
    let mut ids=HashMap::new();
    for (i,plan) in planned.into_iter().enumerate() {
        let provisional=u32::MAX-i as u32;
        let id=match plan {
            Planned::Finalized(mut mob,equipment)=>{
                game.next_entity+=1;
                mob.id=game.next_entity;
                let id=mob.id;
                let prepared=match bridge::take(&mob) {
                    Ok(prepared)=>prepared,
                    Err(error)=>{ log::error!("PlutoLib spawner state: {error}"); continue; },
                };
                let deferred=prepared.is_some();
                game.mobs.push(mob);
                if !deferred { game.native_mobs_joined(game.mobs.len()-1,false); }
                game.enchant_spawn_equipment(id);
                game.halloween_spawn_head(id);
                game.spawn_jockeys(id,false);
                if let Some(prepared)=prepared {
                    if let Err(error)=bridge::complete(game,id,prepared) {
                        log::error!("PlutoLib spawner completion failed: {error}");
                        continue;
                    }
                    if let Some(index)=game.mobs.iter().position(|m|m.id==id) {
                        game.native_mobs_joined(index,false);
                    }
                    if !game.mobs.iter().any(|m|m.id==id) { continue; }
                }
                Some((id,equipment))
            },
            Planned::Loaded(tag,equipment)=>game.load_mob(tag,None).map(|id|(id,equipment)),
        };
        if let Some((id,equipment))=id {
            // Explicit spawn-data equipment is applied AFTER finalizeSpawn;
            // this intentionally can replace a callback's generated gear.
            if let Some(table)=equipment { game.equip_from_table(id,&table); }
            ids.insert(provisional,id);
        }
    }
    ids
}
