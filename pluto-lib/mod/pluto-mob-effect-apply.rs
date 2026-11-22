// After a player's weapon hit has been offered to the mods, the status effects they queued go on the mobs they name.
fn pluto_mob_effect_apply(value:(),game:&mut crate::game::Game<'_>) {
    let _=value;
    for (entity,effect,ticks,amplifier) in crate::entry::pluto_mob_effect_access::take() {
        if ticks <= 0 || amplifier < 0 {continue;}
        let Some(effect)=crate::entry::pluto_mob_effect_access::interned(&effect) else {continue};
        if let Some(mob)=game.mobs.iter_mut().find(|mob| mob.id==entity) {mob.add_effect(effect,ticks,amplifier);}
    }
    // What entity-inside callbacks did to entities lands here too: a mob's queued effect and fire are
    // applied; a hurt has no reach at this point (no mob hurt in this context, and a player is not
    // reachable by id), so it is dropped and traced (PLUTO_TRACE_INSIDE).
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
                if std::env::var_os("PLUTO_TRACE_INSIDE").is_some() {eprintln!("pluto_lib: contact hurt {amount} ({damage_type}) for entity {entity} dropped: no hurt reach where contact actions land");}
            }
        }
    }
}
