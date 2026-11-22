// The status effects queued for mobs by a hit on the player (an armor's counter) go on the mobs they name at the next player tick.
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
}
