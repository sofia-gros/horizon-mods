// After a player's weapon hit has been offered to the mods, the status effects they queued go on the mobs they name.
fn pluto_mob_effect_apply(value:(),game:&mut crate::game::Game<'_>) {
    let _=value;
    for (entity,effect,ticks,amplifier) in sunrise_client_lib::pluto_mob_effect_access::take() {
        if ticks <= 0 || amplifier < 0 {continue;}
        let Some(effect)=sunrise_client_lib::pluto_mob_effect_access::interned(&effect) else {continue};
        if let Some(mob)=game.mobs.iter_mut().find(|mob| mob.id==entity) {mob.add_effect(effect,ticks,amplifier);}
    }
    // What entity-inside callbacks did to entities lands here too: a mob's queued effect and fire are
    // applied to the mob they name, and a hurt goes through the generalized hurt reach
    // (pluto_entity_hurt_game::hurt_any): living entities through the game's own hurt call, with the
    // game's armour, invulnerability time, death and damage hooks, the rest through the game's damage
    // hurt (a hurt that reaches nothing is traced, PLUTO_TRACE_INSIDE).
    for (entity,action) in sunrise_client_lib::pluto_contact_access::take() {
        match action {
            sunrise_client_lib::pluto_contact_access::Contact::Effect(effect,ticks,amplifier) => {
                if ticks <= 0 || amplifier < 0 {continue;}
                let Some(effect)=sunrise_client_lib::pluto_contact_access::interned(&effect) else {continue};
                if let Some(mob)=game.mobs.iter_mut().find(|mob| mob.id==entity) {mob.add_effect(effect,ticks,amplifier);}
            }
            sunrise_client_lib::pluto_contact_access::Contact::Fire(seconds) => {
                if let Some(mob)=game.mobs.iter_mut().find(|mob| mob.id==entity) {
                    let ticks=seconds.saturating_mul(20);
                    if mob.fire_ticks<ticks {mob.fire_ticks=ticks;}
                }
            }
            sunrise_client_lib::pluto_contact_access::Contact::Hurt(damage_type,amount) => {
                if !crate::game::pluto_entity_hurt_game::hurt_any(game,entity,amount,&damage_type)
                    && std::env::var_os("PLUTO_TRACE_INSIDE").is_some() {eprintln!("pluto_lib: contact hurt {amount} ({damage_type}) for entity {entity} did not land: not an entity of this level that takes damage");}
            }
        }
    }
}
