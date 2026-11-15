// Adapted from pinned MIT Horizon natural_spawning.rs::add_spawned.
// License: pluto-lib/LICENSE.horizon. OVERWRITE consumes the owned pending Vec
// without cloning Mob or moving an argument needed by a HEAD continuation.
fn pluto_entity_spawn_add(game:&mut Game<'_>,pending:Vec<Mob>,persistent:bool) {
    use crate::game::pluto_entity_spawn_game as bridge;
    for mut mob in pending {
        game.next_entity+=1;
        mob.id=game.next_entity;
        mob.persistent=persistent;
        let id=mob.id;
        let prepared=match bridge::take(&mob) {
            Ok(prepared)=>prepared,
            Err(error)=>{ log::error!("PlutoLib pending spawn state: {error}"); continue; },
        };
        let deferred=prepared.is_some();
        game.mobs.push(mob);
        if !deferred { game.native_mobs_joined(game.mobs.len()-1,true); }
        game.enchant_spawn_equipment(id);
        game.halloween_spawn_head(id);
        if !persistent { game.spawn_jockeys(id,true); }
        if let Some(prepared)=prepared {
            if let Err(error)=bridge::complete(game,id,prepared) {
                log::error!("PlutoLib pending spawn completion failed: {error}");
                continue;
            }
            if let Some(index)=game.mobs.iter().position(|m|m.id==id) {
                game.native_mobs_joined(index,true);
            }
        }
    }
}
