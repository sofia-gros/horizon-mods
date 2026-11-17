// The status effects queued for mobs by a hit on the player (an armor's counter) go on the mobs they name at the next player tick.
fn pluto_mob_effect_apply_tick(game:&mut crate::game::Game<'_>) {
    for (entity,effect,ticks,amplifier) in crate::entry::pluto_mob_effect_access::take() {
        if ticks <= 0 || amplifier < 0 {continue;}
        let Some(effect)=crate::entry::pluto_mob_effect_access::interned(&effect) else {continue};
        if let Some(mob)=game.mobs.iter_mut().find(|mob| mob.id==entity) {mob.add_effect(effect,ticks,amplifier);}
    }
}
