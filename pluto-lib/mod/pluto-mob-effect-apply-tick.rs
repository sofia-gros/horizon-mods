// The status effects queued for mobs by a hit on the player (an armor's counter) go on the mobs they name at the next player tick.
// The drops a mod's damage callback asked for at an entity land here too: each stack is spawned at the entity it was
// dropped at (a mob, the ender dragon, or the player), with the game's own pickup delay, as `LivingEntity.spawnAtLocation`
// drops it. A target that has left the world drops nothing.
fn pluto_mob_effect_apply_tick(game:&mut crate::game::Game<'_>) {
    for (entity,effect,ticks,amplifier) in crate::entry::pluto_mob_effect_access::take() {
        if ticks <= 0 || amplifier < 0 {continue;}
        let Some(effect)=crate::entry::pluto_mob_effect_access::interned(&effect) else {continue};
        if let Some(mob)=game.mobs.iter_mut().find(|mob| mob.id==entity) {mob.add_effect(effect,ticks,amplifier);}
    }
    // What entity-inside callbacks did to entities lands here too: a mob's queued effect and fire are
    // applied to the mob they name, and a hurt goes through the game's own hurt call, which reaches
    // a mob of this level or the player in place by id, with the game's armour, invulnerability
    // time, death and damage hooks (a hurt that reaches nothing is traced, PLUTO_TRACE_INSIDE).
    for (entity,action) in crate::entry::pluto_contact_access::take() {
        match action {
            crate::entry::pluto_contact_access::Contact::Effect(effect,ticks,amplifier) => {
                if ticks <= 0 || amplifier < 0 {continue;}
                let Some(effect)=crate::entry::pluto_contact_access::interned(&effect) else {continue};
                if let Some(mob)=game.mobs.iter_mut().find(|mob| mob.id==entity) {mob.add_effect(effect,ticks,amplifier);}
            }
            crate::entry::pluto_contact_access::Contact::Fire(seconds) => {
                if let Some(mob)=game.mobs.iter_mut().find(|mob| mob.id==entity) {
                    let ticks=seconds.saturating_mul(20);
                    if mob.fire_ticks<ticks {mob.fire_ticks=ticks;}
                }
            }
            crate::entry::pluto_contact_access::Contact::Hurt(damage_type,amount) => {
                if !game.native_hurt_entity(game.dim.id(),entity,amount,&damage_type,None)
                    && std::env::var_os("PLUTO_TRACE_INSIDE").is_some() {eprintln!("pluto_lib: contact hurt {amount} ({damage_type}) for entity {entity} did not land: not a living entity of this level");}
            }
        }
    }
    for (entity,stack) in crate::entry::pluto_damage_facts_access::take_drops() {
        let position=if let Some(mob)=game.mobs.iter().find(|mob| mob.id==entity) {
            [mob.position.x,mob.position.y,mob.position.z]
        } else if let Some(dragon)=game.dragon.as_ref().filter(|dragon| dragon.id==entity) {
            [dragon.position.x,dragon.position.y,dragon.position.z]
        } else if let Some(player)=game.player.as_ref().filter(|player| player.entity_id==entity as i32) {
            [player.position.x,player.position.y,player.position.z]
        } else {continue;};
        game.spawn_item_own(stack,position,10);
    }
}
