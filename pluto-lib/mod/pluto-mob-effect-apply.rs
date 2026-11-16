// After a player's weapon hit has been offered to the mods, the status effects they queued go on the mobs they name.
fn pluto_mob_effect_apply(game:&mut crate::game::Game<'_>) {
    for (entity,effect,ticks,amplifier) in crate::entry::pluto_mob_effect_access::take() {
        if ticks <= 0 || amplifier < 0 {continue;}
        let effect=crate::entry::pluto_mob_effect_access::interned(&effect);
        if let Some(mob)=game.mobs.iter_mut().find(|mob| mob.id==entity) {mob.add_effect(effect,ticks,amplifier);}
    }
}
